//! A simulated device as an SNMP transport.
//!
//! This emulates *snmpd driving a `pass` handler*, not a sorted map. That distinction is the whole
//! reason a device test proves anything:
//!
//! - A GETBULK of `n` is `n` successive handler GETNEXTs, each starting from the previous answer,
//!   because that is how snmpd assembles a bulk response over a `pass` script that answers one
//!   varbind at a time. A model that returned a slice of a sorted table would make every page
//!   perfect and could never reproduce a walk that ends early.
//! - A request is routed to exactly one registration — the `pass` line whose subtree covers it —
//!   and that registration's answer is bounded to its own subtree. Falling off the end moves to
//!   the next registration, which is what makes a device serving nothing at a subtree report the
//!   next one up rather than inventing an answer.
//! - The four handlers (`Normal`, `Positional`, `Stuck`, `Slow`) are the four shell scripts in
//!   `tools/snmp/lxc/setup.sh`, and three of them are meant to misbehave.
//! - Registrations are disjoint, and a request goes to the first one covering it. net-snmp serves
//!   only the broadest of a set of overlapping `pass` lines and ignores the rest, so serving one
//!   sub-table of a MIB differently means carving the MIB up, not nesting inside it.

use anyhow::Result;

use super::wire::{DataFile, Row};
use crate::daemon::discovery::integration::snmp::queries::{SnmpWalkTransport, Varbinds, WalkPage};

/// Which `pass` handler serves a registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Handler {
    /// `snmp-pass-handler.sh` — answers GETNEXT with the first line numerically greater than the
    /// request. Every device but two.
    #[default]
    Normal,
    /// `snmp-pass-handler-unsorted.sh` — answers with the line that physically *follows* the one
    /// asked for, in file order. Firmware that stores a table unsorted and iterates it
    /// positionally does this, and it is why `snmpwalk` stops at "OID not increasing" where
    /// `snmpbulkwalk -Cc` reads the table in full (GH #674). The normal handler cannot reproduce
    /// it: it can only ever produce an ascending sequence, so a shuffled file would simply end the
    /// walk early.
    Positional,
    /// `snmp-pass-handler-stuck.sh` — answers every GETNEXT with the first line, whatever was
    /// asked. The non-advancing agent the walk's retry-then-stop guard was written for.
    Stuck,
    /// `snmp-pass-handler-slow.sh` — the normal handler with a `sleep` in front of it. Answers a
    /// GETNEXT correctly and cannot assemble a GETBULK inside the client's timeout (GH #668).
    ///
    /// This is one device behaviour, not two: a getbulk routed here fails, a getnext answers.
    ///
    /// **The deployed twin does not yet reproduce it, and the model is ahead of the lab here.**
    /// `snmp-pass-handler-slow.sh` sleeps per invocation, and snmpd drives `pass` serially — three
    /// calls measured at 9.03s for a 3s sleep, strictly additive. A getbulk of 20 therefore blocks
    /// the agent for `20 * sleep` while the client gives up after [`SNMP_TIMEOUT`], so the getnext
    /// the walk sends next is still queued behind the bulk it was meant to escape and times out
    /// too. Any sleep large enough to fail the bulk is large enough to fail the getnext behind it.
    /// Slowness per varbind cannot express "will not serve bulk, will serve getnext" on a serial
    /// agent; a mechanism that refuses the bulk without consuming agent time can. Until one
    /// exists, this variant is exercised by the unit tests only and `switch-slowbulk-01` will not
    /// reproduce GH #668 on the VM.
    ///
    /// Distinct from [`SimAgent::without_getbulk`], which is SNMPv1 answering "no such request
    /// type". That is a clean refusal the walk has always handled by falling back. This is
    /// silence, which until GH #668 it could not: `shrink_page` needed five halvings to reach
    /// getnext and `MAX_TRANSPORT_RETRIES` allowed two, so the column was abandoned having never
    /// asked the question the device would have answered.
    ///
    /// [`SNMP_TIMEOUT`]: crate::daemon::discovery::integration::snmp::session::SNMP_TIMEOUT
    Slow,
}

impl Handler {
    /// One handler invocation: what the script prints for `GETNEXT from`, or nothing.
    fn getnext<'a>(self, file: &'a ServedFile, from: &[u64]) -> Option<&'a Row> {
        let rows = &file.rows;
        match self {
            // `Slow` differs from `Normal` only in how long it takes, which is a property of the
            // request type rather than of the answer — see `SimAgent::walk_getbulk`.
            Self::Normal | Self::Slow => rows.iter().find(|row| row.oid.as_slice() > from),
            Self::Stuck => rows.first(),
            Self::Positional => {
                // The line after the requested one, in file order. A request naming no line of its
                // own — a bare column or table prefix — is answered with the first line under it,
                // again in file order, which is where the shuffle first shows.
                if let Some(pos) = rows.iter().position(|row| row.oid.as_slice() == from) {
                    return rows.get(pos + 1);
                }
                rows.iter()
                    .find(|row| row.oid.len() > from.len() && row.oid.starts_with(from))
                    .or_else(|| rows.iter().find(|row| row.oid.as_slice() > from))
            }
        }
    }
}

/// A `pass` line: one subtree, served from one file by one handler.
#[derive(Debug, Clone)]
pub struct Registration {
    pub subtree: Vec<u64>,
    pub file: usize,
    pub handler: Handler,
}

struct ServedFile {
    rows: Vec<Row>,
}

/// A device answering SNMP the way its deployed agent does.
pub struct SimAgent {
    files: Vec<ServedFile>,
    /// Ascending by subtree, which is the order snmpd walks its registrations in.
    registrations: Vec<Registration>,
    /// The agent has no getbulk, because SNMPv1 has none.
    ///
    /// Not a knob: it follows from the device's credential. A v1 agent answers a getbulk with an
    /// error, and the walk falls back to getnext — one varbind per round trip, for every column of
    /// every table. That fallback is most of what GH #557 is about, and a transport that always
    /// answered getbulk could not exercise it.
    bulk_unsupported: bool,
    /// Set once a walk has fallen back to getnext, and read by every walk after it.
    ///
    /// One `SimAgent` serves a whole collection, exactly as one session serves one host, so this
    /// is what lets a device test observe the per-host half of the GH #668 fix — a device costing
    /// the scan one column's worth of timeouts rather than one per column — and not only the
    /// per-column half.
    getbulk_unusable: bool,
}

impl SimAgent {
    pub fn new(files: &[DataFile], registrations: Vec<Registration>) -> Self {
        let files = files
            .iter()
            .map(|file| ServedFile {
                // `rows()` applies the file's ordering, so a `Positional` file keeps the order it
                // was written in and every other file is sorted — exactly what lands on the VM.
                rows: file.rows(),
            })
            .collect();
        let mut registrations = registrations;
        registrations.sort_by(|a, b| a.subtree.cmp(&b.subtree));
        Self {
            files,
            registrations,
            bulk_unsupported: false,
            getbulk_unusable: false,
        }
    }

    /// An agent with no getbulk, as SNMPv1 has none.
    pub fn without_getbulk(mut self) -> Self {
        self.bulk_unsupported = true;
        self
    }

    fn covers(registration: &Registration, oid: &[u64]) -> bool {
        oid.starts_with(&registration.subtree)
    }

    /// Route one GETNEXT the way snmpd does, then bound the answer to the registration that gave
    /// it. An answer outside its own subtree is not returned — net-snmp treats that registration
    /// as exhausted and moves on, which is what stops one `pass` file leaking rows into a walk of
    /// a subtree it does not serve.
    fn answer(&self, from: &[u64]) -> Option<&Row> {
        self.routed(from).map(|(_, row)| row)
    }

    /// The registration that answers `from`, alongside its row.
    ///
    /// Split out of [`Self::answer`] because a request's *cost* belongs to whichever `pass` line
    /// serves it, not to the device: one slow registration on a switch whose other tables answer
    /// instantly is the shape GH #668 arrived in, and a whole-agent flag could not express it.
    ///
    /// First match, and it may stay first match because registrations are disjoint —
    /// [`super::SimDevice::registrations`] builds them that way and
    /// `no_registration_is_nested_inside_another` holds every device to it. This deliberately does
    /// *not* prefer the longest match: net-snmp's `pass` serves only the broadest of a set of
    /// overlapping lines and silently ignores the rest, so a model that resolved nesting would be
    /// more capable than the agent it stands in for, and a fixture that worked here would go quiet
    /// on the VM. That is not hypothetical — it is what a nested LLDP registration did.
    fn routed(&self, from: &[u64]) -> Option<(&Registration, &Row)> {
        let start = self
            .registrations
            .iter()
            .position(|reg| Self::covers(reg, from))
            .or_else(|| {
                self.registrations
                    .iter()
                    .position(|reg| reg.subtree.as_slice() > from)
            })?;

        for registration in &self.registrations[start..] {
            let ask: &[u64] = if Self::covers(registration, from) {
                from
            } else {
                &registration.subtree
            };
            if let Some(row) = registration
                .handler
                .getnext(&self.files[registration.file], ask)
                .filter(|row| Self::covers(registration, &row.oid))
            {
                return Some((registration, row));
            }
        }
        None
    }

    /// A GETBULK page: `max_repetitions` chained GETNEXTs, as snmpd assembles one over a handler
    /// that answers a single varbind per invocation.
    ///
    /// The chaining is what lets a misbehaving handler show itself. A `Stuck` handler yields the
    /// same row `n` times; a `Positional` one yields a page whose last row can sort *below* its
    /// first, which is the exact moment a strictly-ascending walk gives up.
    fn page(&self, from: &[u64], max_repetitions: u32) -> Varbinds<'_> {
        let mut page = Vec::new();
        let mut cursor = from.to_vec();
        for _ in 0..max_repetitions.max(1) {
            match self.answer(&cursor) {
                Some(row) => {
                    page.push((row.oid.clone(), row.value.as_snmp()));
                    cursor = row.oid.clone();
                }
                None => break,
            }
        }
        // Past the last row a real agent says so rather than answering with nothing: an empty
        // response is abnormal and the walk rightly reads it as truncation.
        if page.is_empty() {
            return vec![(from.to_vec(), snmp2::Value::EndOfMibView)];
        }
        page
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for SimAgent {
    fn getbulk_unusable(&self) -> bool {
        self.getbulk_unusable
    }

    fn note_getbulk_unusable(&mut self) {
        self.getbulk_unusable = true;
    }

    async fn walk_getbulk<'a>(
        &'a mut self,
        from: &[u64],
        max_repetitions: u32,
    ) -> Result<WalkPage<'a>> {
        if self.bulk_unsupported {
            return Ok(WalkPage::BulkUnsupported);
        }
        // A bulk page over a `Handler::Slow` registration costs `max_repetitions` sleeps and the
        // client gives up first, whatever it asked for — so this is an `Err`, the same shape a
        // real timeout takes, and not an empty page or a refusal. `walk_getnext` below is
        // deliberately not guarded: one invocation is what the handler *can* serve in time, and
        // that asymmetry is the whole defect.
        if self
            .routed(from)
            .is_some_and(|(reg, _)| reg.handler == Handler::Slow)
        {
            return Err(anyhow::anyhow!("getbulk timed out"));
        }
        Ok(WalkPage::Varbinds(self.page(from, max_repetitions)))
    }

    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        Ok(self.page(from, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::snmp::oids::{arp, if_mib, oid_parts};
    use crate::daemon::discovery::integration::snmp::sim::wire::{Ordering, PassValue};

    fn arp_file(order: Ordering, indexes: &[u64]) -> DataFile {
        DataFile::new(
            "arp",
            order,
            indexes
                .iter()
                .map(|i| {
                    Row::at(
                        arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
                        &[1, 10, 20, 30, *i],
                        PassValue::Integer(1),
                    )
                })
                .collect(),
        )
    }

    fn agent(file: DataFile, base: &str, handler: Handler) -> SimAgent {
        SimAgent::new(
            &[file],
            vec![Registration {
                subtree: oid_parts(base),
                file: 0,
                handler,
            }],
        )
    }

    /// A page is chained GETNEXTs, so a well-behaved agent walks forward one row at a time.
    #[test]
    fn a_page_advances_one_row_per_repetition() {
        let agent = agent(
            arp_file(Ordering::Ascending, &[1, 2, 3, 4, 5]),
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Handler::Normal,
        );
        let page = agent.page(&oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX), 3);
        let last: Vec<u64> = page.iter().map(|(oid, _)| *oid.last().unwrap()).collect();
        assert_eq!(last, vec![1, 2, 3]);
    }

    /// GH #674, and the property the whole `.246` fixture rests on: served positionally, a page
    /// can end *lower* than it began. A sorted-map model cannot produce this, which is why the
    /// transport emulates the handler rather than the table.
    #[test]
    fn a_positional_agent_hands_back_a_page_that_ends_below_where_it_started() {
        // Evens then odds — the ordering the real fixture uses, so the second page ends lower
        // than the first.
        let agent = agent(
            arp_file(Ordering::Positional, &[2, 4, 6, 8, 1, 3, 5]),
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Handler::Positional,
        );
        let page = agent.page(&oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX), 6);
        let last: Vec<u64> = page.iter().map(|(oid, _)| *oid.last().unwrap()).collect();

        assert_eq!(last, vec![2, 4, 6, 8, 1, 3]);
        // The signature of the defect: somewhere in the page a row sorts *below* the one before
        // it. That is the moment a client insisting every step ascend gives up ("OID not
        // increasing") while `snmpbulkwalk -Cc` reads the table in full.
        assert!(
            last.windows(2).any(|pair| pair[1] < pair[0]),
            "a positional agent must hand back a descent: {last:?}"
        );
    }

    /// The non-advancing agent: every repetition is the same row, which is what the walk's
    /// retry-then-stop guard exists to survive.
    #[test]
    fn a_stuck_agent_answers_every_repetition_with_the_same_row() {
        let agent = agent(
            arp_file(Ordering::Ascending, &[1, 2, 3]),
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Handler::Stuck,
        );
        let page = agent.page(&oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX), 4);
        let last: Vec<u64> = page.iter().map(|(oid, _)| *oid.last().unwrap()).collect();
        assert_eq!(last, vec![1, 1, 1, 1]);
    }

    /// The broadest of two overlapping registrations answers, and the nested one is dead config.
    ///
    /// This is net-snmp, not a modelling choice. A `pass` line nested inside another is not served
    /// at all: an LLDP MIB registered at its root alongside `…1.3.7` and `…1.4` answered every
    /// request from the root line, so the sub-table handlers never ran and the rows moved into the
    /// sub-table's own file went unserved — the device reported `No Such Instance` for a table it
    /// held. Pinned here so the model cannot quietly become more capable than the agent, and
    /// enforced for real by `no_registration_is_nested_inside_another`, which stops any device
    /// producing this config in the first place.
    #[test]
    fn a_registration_nested_inside_another_is_never_reached() {
        let file = arp_file(Ordering::Ascending, &[1, 2, 3]);
        let inner = oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX);
        let outer = inner[..inner.len() - 1].to_vec();
        let agent = SimAgent::new(
            &[file],
            vec![
                Registration {
                    subtree: outer,
                    file: 0,
                    handler: Handler::Normal,
                },
                Registration {
                    subtree: inner.clone(),
                    file: 0,
                    handler: Handler::Stuck,
                },
            ],
        );

        // `Stuck` would answer every request with the first row. Advancing normally is the outer
        // `Normal` line taking the request, which is what the VM does.
        let page = agent.page(&inner, 3);
        let last: Vec<u64> = page.iter().map(|(oid, _)| *oid.last().unwrap()).collect();
        assert_eq!(last, vec![1, 2, 3]);
    }

    /// The asymmetry GH #668 turns on: the same registration answers a getnext and cannot answer
    /// a getbulk of any size. A whole-agent flag could say "no getbulk"; only a per-registration
    /// one can say "this table is slow and the rest of the device is fine".
    #[tokio::test]
    async fn a_slow_registration_answers_getnext_and_times_out_every_getbulk() {
        let mut agent = agent(
            arp_file(Ordering::Ascending, &[1, 2, 3]),
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Handler::Slow,
        );
        let base = oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX);

        // Every page size the walk shrinks through, including the smallest it ever asks for.
        for max_repetitions in [20, 10, 5, 2, 1] {
            assert!(
                agent.walk_getbulk(&base, max_repetitions).await.is_err(),
                "a bulk page of {max_repetitions} costs that many sleeps and cannot arrive in time"
            );
        }

        let answered = agent.walk_getnext(&base).await.expect("getnext answers");
        assert_eq!(*answered[0].0.last().unwrap(), 1);
    }

    /// One slow `pass` line must not make the whole device slow. The reporter's switch served its
    /// ifTable and ARP table perfectly while the LLDP subtree went silent.
    #[tokio::test]
    async fn a_slow_registration_does_not_slow_its_neighbours() {
        let mut agent = SimAgent::new(
            &[
                arp_file(Ordering::Ascending, &[1, 2]),
                DataFile::new(
                    "iftable",
                    Ordering::Ascending,
                    vec![Row::at(
                        if_mib::columns::IF_INDEX,
                        &[1],
                        PassValue::Integer(1),
                    )],
                ),
            ],
            vec![
                Registration {
                    subtree: oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX),
                    file: 0,
                    handler: Handler::Slow,
                },
                Registration {
                    subtree: oid_parts(if_mib::IF_TABLE),
                    file: 1,
                    handler: Handler::Normal,
                },
            ],
        );

        assert!(
            agent
                .walk_getbulk(&oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX), 10)
                .await
                .is_err()
        );
        assert!(matches!(
            agent.walk_getbulk(&oid_parts(if_mib::IF_TABLE), 10).await,
            Ok(WalkPage::Varbinds(_))
        ));
    }

    /// One file serving several subtrees must not leak rows across them: a walk of `ifTable` ends
    /// at the end of `ifTable`, even though the same file also holds the ifXTable rows that sort
    /// immediately above it.
    #[test]
    fn a_registration_does_not_answer_outside_its_own_subtree() {
        let file = DataFile::new(
            "iftable",
            Ordering::Ascending,
            vec![
                Row::at(if_mib::columns::IF_INDEX, &[1], PassValue::Integer(1)),
                Row::at(
                    if_mib::if_x_table::IF_NAME,
                    &[1],
                    PassValue::Str("Gi0/1".into()),
                ),
            ],
        );
        let agent = SimAgent::new(
            &[file],
            vec![Registration {
                subtree: oid_parts(if_mib::IF_TABLE),
                file: 0,
                handler: Handler::Normal,
            }],
        );

        // Walking off the end of ifTable finds nothing, rather than handing back the ifXTable row
        // that physically follows it in the same file.
        let page = agent.page(&oid_parts(if_mib::columns::IF_INDEX), 5);
        assert_eq!(page.len(), 1);
        assert_eq!(*page[0].0.last().unwrap(), 1);

        let past = agent.page(&oid_parts(if_mib::if_x_table::IF_NAME), 1);
        assert!(matches!(past[0].1, snmp2::Value::EndOfMibView));
    }
}
