//! DNS-SD message construction and response correlation.
//!
//! Deliberately free of sockets so the part with actual logic — reassembling PTR, SRV, TXT and
//! A/AAAA records scattered across several packets into one host — can be tested against fixture
//! bytes.
//!
//! The wire format is ordinary DNS, so `hickory_resolver::proto` does all of the encoding and
//! decoding. mDNS differs from unicast DNS in transport and authority, not in syntax: the query
//! goes to a multicast group and the device itself answers, rather than a configured resolver
//! answering out of a zone somebody maintains.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::net::IpAddr;

use anyhow::{Error, Result};
use hickory_resolver::proto::op::{Message, MessageType, OpCode, Query};
use hickory_resolver::proto::rr::{Name, RData, RecordType};
use hickory_resolver::proto::serialize::binary::BinDecodable;

use super::types::DnsSdHost;

/// The DNS-SD service-type enumeration query. Its answers are service types present on the link,
/// not instances — which is why [`Accumulator`] treats records under this owner separately.
pub const SERVICE_ENUMERATION: &str = "_services._dns-sd._udp.local.";

/// TXT keys that carry a human-chosen name, in preference order.
///
/// `fn` is Chromecast and Google Home's "friendly name"; `nm` appears on HomeKit and some Sonos
/// firmwares. Both are what the owner typed during setup, which is worth more than the instance
/// label a vendor generated.
const FRIENDLY_NAME_KEYS: [&str; 2] = ["fn", "nm"];

/// Build one query carrying a `PTR` question per name.
///
/// A single message with several questions is legal and is what a browse should send: one packet
/// on the wire instead of one per service type, and responders answer the questions they know.
pub fn build_query(names: &[Name]) -> Result<Vec<u8>> {
    // ID 0: mDNS matches responses by question, not by transaction, because any listener on the
    // group may answer and several may answer the same question.
    let mut message = Message::new(0, MessageType::Query, OpCode::Query);
    for name in names {
        message.add_query(Query::query(name.clone(), RecordType::PTR));
    }
    message.to_vec().map_err(Error::from)
}

/// Reassembles the records of a browse into hosts.
///
/// Records arrive spread across packets and sections — a PTR naming an instance, an SRV pointing
/// that instance at a hostname, TXT describing it, and an A binding the hostname to an address.
/// None of those four is individually useful, so they are accumulated by their join keys and
/// resolved once at the end.
#[derive(Debug, Default)]
pub struct Accumulator {
    /// Service types seen in the `_services._dns-sd._udp.local` enumeration, so a second burst can
    /// ask about types we did not think to name.
    advertised_types: BTreeSet<Name>,
    /// Instance name → the service type whose PTR named it.
    instance_service: BTreeMap<Name, BTreeSet<String>>,
    /// Instance name → the hostname its SRV points at.
    instance_target: BTreeMap<Name, Name>,
    /// Instance name → its TXT key/value pairs.
    instance_txt: BTreeMap<Name, BTreeMap<String, String>>,
    /// Hostname → the addresses its A/AAAA records bind.
    host_addresses: BTreeMap<Name, Vec<IpAddr>>,
}

impl Accumulator {
    /// Fold one received packet in. A packet that does not parse is skipped: the group carries
    /// traffic from every mDNS speaker on the link, not only answers to us.
    pub fn absorb(&mut self, packet: &[u8]) {
        let Ok(message) = Message::from_bytes(packet) else {
            return;
        };

        for record in message.all_sections() {
            let owner = record.name.clone();
            match &record.data {
                RData::PTR(ptr) => {
                    let target = ptr.0.clone();
                    if is_service_enumeration(&owner) {
                        self.advertised_types.insert(target);
                    } else {
                        self.instance_service
                            .entry(target)
                            .or_default()
                            .insert(service_type_label(&owner));
                    }
                }
                RData::SRV(srv) => {
                    self.instance_target.insert(owner, srv.target.clone());
                }
                RData::TXT(txt) => {
                    let entry = self.instance_txt.entry(owner).or_default();
                    for pair in txt.txt_data.iter() {
                        if let Some((key, value)) = parse_txt_pair(pair) {
                            entry.insert(key, value);
                        }
                    }
                }
                RData::A(a) => {
                    self.host_addresses
                        .entry(owner)
                        .or_default()
                        .push(IpAddr::V4(a.0));
                }
                RData::AAAA(aaaa) => {
                    self.host_addresses
                        .entry(owner)
                        .or_default()
                        .push(IpAddr::V6(aaaa.0));
                }
                _ => {}
            }
        }
    }

    /// Service types the link advertised that `asked` did not cover, so a second burst can ask
    /// about them. This is what lets a browse find a device nobody anticipated.
    pub fn unasked_types(&self, asked: &[Name]) -> Vec<Name> {
        self.advertised_types
            .iter()
            .filter(|name| !asked.contains(name))
            .cloned()
            .collect()
    }

    /// Resolve the accumulated records into one entry per address.
    ///
    /// An instance whose SRV or A record never arrived resolves to nothing — there is no address
    /// to attach it to, and inventing one from a partial response would be worse than silence.
    pub fn resolve(self) -> HashMap<IpAddr, DnsSdHost> {
        let mut hosts: HashMap<IpAddr, DnsSdHost> = HashMap::new();

        for (instance, services) in &self.instance_service {
            let Some(target) = self.instance_target.get(instance) else {
                continue;
            };
            let Some(addresses) = self.host_addresses.get(target) else {
                continue;
            };
            let txt = self.instance_txt.get(instance);

            for address in addresses {
                let host = hosts.entry(*address).or_default();
                host.hostname
                    .get_or_insert_with(|| trim_root(&target.to_utf8()));
                host.services.extend(services.iter().cloned());
                if let Some(txt) = txt {
                    host.txt
                        .extend(txt.iter().map(|(k, v)| (k.clone(), v.clone())));
                }
                // TXT outranks the instance label because it is what the owner typed, not what the
                // vendor generated.
                //
                // The fallback to the instance label is under investigation: a live scan named an
                // iPhone "Spotify Group Session [612Gq]", because `_spotify-connect._tcp` names a
                // *session* rather than the device and sorted first. TEMPORARY diagnostic, to
                // establish which service type and which path supplies each name on real hardware
                // before the rule is narrowed — REMOVE once that is known.
                let from_txt = txt.and_then(friendly_name_from_txt);
                let name = from_txt.clone().or_else(|| first_label(instance));
                if let Some(name) = name {
                    tracing::debug!(
                        address = %address,
                        instance = %instance,
                        services = %services.iter().cloned().collect::<Vec<_>>().join(","),
                        source = if from_txt.is_some() { "txt" } else { "instance-label" },
                        candidate = %name,
                        accepted = host.instance_name.is_none(),
                        "mDNS name candidate"
                    );
                    host.instance_name.get_or_insert(name);
                }
            }
        }

        hosts.retain(|_, host| !host.is_empty());
        hosts
    }
}

/// Whether a PTR's owner is the service-type enumeration rather than a concrete service.
fn is_service_enumeration(owner: &Name) -> bool {
    owner.to_utf8().eq_ignore_ascii_case(SERVICE_ENUMERATION)
}

/// The wire form of a service type, without the trailing `.local.` — `_googlecast._tcp`.
///
/// Trimmed because that is how a service definition names it, and carrying the domain into the
/// match would make every pattern repeat it.
fn service_type_label(owner: &Name) -> String {
    let name = trim_root(&owner.to_utf8());
    name.strip_suffix(".local")
        .unwrap_or(&name)
        .to_ascii_lowercase()
}

/// The leftmost label of a name, which for a DNS-SD instance is its human-facing part.
fn first_label(name: &Name) -> Option<String> {
    let label = name.iter().next()?;
    let label = String::from_utf8_lossy(label).trim().to_string();
    (!label.is_empty()).then_some(label)
}

/// Split a TXT `key=value` entry. Keys are lowercased because the specification makes them
/// case-insensitive and vendors disagree in practice.
fn parse_txt_pair(pair: &[u8]) -> Option<(String, String)> {
    let text = String::from_utf8_lossy(pair);
    let (key, value) = text.split_once('=')?;
    let key = key.trim().to_ascii_lowercase();
    (!key.is_empty()).then(|| (key, value.trim().to_string()))
}

/// The friendliest name a TXT record offers, if any.
fn friendly_name_from_txt(txt: &BTreeMap<String, String>) -> Option<String> {
    FRIENDLY_NAME_KEYS
        .iter()
        .find_map(|key| txt.get(*key))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Drop the root label's trailing dot, which is correct on the wire and noise in a UI.
fn trim_root(name: &str) -> String {
    name.trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_resolver::proto::rr::rdata::{A, PTR, SRV, TXT};
    use hickory_resolver::proto::rr::{DNSClass, Record};
    use hickory_resolver::proto::serialize::binary::BinEncodable;
    use std::net::Ipv4Addr;

    fn name(s: &str) -> Name {
        Name::from_utf8(s).unwrap()
    }

    /// Build a name from raw labels.
    ///
    /// DNS-SD instance labels routinely contain spaces — "Living Room._googlecast._tcp.local" is
    /// what a Chromecast actually announces. That is legal on the wire, where a label is
    /// length-prefixed bytes, but not in the *text* form `Name::from_utf8` parses, which requires
    /// the space escaped as `\032`. Fixtures build from labels so they carry what a device sends
    /// rather than what a zone file would write.
    fn instance(label: &str, service: &str) -> Name {
        let mut labels: Vec<&[u8]> = vec![label.as_bytes()];
        labels.extend(
            service
                .trim_end_matches('.')
                .split('.')
                .map(|part| part.as_bytes()),
        );
        let mut name = Name::from_labels(labels).unwrap();
        name.set_fqdn(true);
        name
    }

    fn record_for(owner: Name, data: RData) -> Record {
        let mut record = Record::from_rdata(owner, 120, data);
        record.dns_class = DNSClass::IN;
        record
    }

    fn record(owner: &str, data: RData) -> Record {
        record_for(name(owner), data)
    }

    /// A complete Chromecast announcement as it arrives on the wire: the four record types spread
    /// across the answer and additional sections, which only mean something together.
    fn chromecast_response() -> Vec<u8> {
        let mut message = Message::response(0, hickory_resolver::proto::op::OpCode::Query);
        message.add_answer(record(
            "_googlecast._tcp.local.",
            RData::PTR(PTR(instance("Living Room", "_googlecast._tcp.local."))),
        ));
        message.add_additional(record_for(
            instance("Living Room", "_googlecast._tcp.local."),
            RData::SRV(SRV::new(0, 0, 8009, name("chromecast-a1b2c3.local."))),
        ));
        message.add_additional(record_for(
            instance("Living Room", "_googlecast._tcp.local."),
            RData::TXT(TXT::new(vec![
                "md=Chromecast Ultra".to_string(),
                "fn=Living Room TV".to_string(),
            ])),
        ));
        message.add_additional(record(
            "chromecast-a1b2c3.local.",
            RData::A(A(Ipv4Addr::new(192, 168, 1, 50))),
        ));
        message.to_bytes().unwrap()
    }

    /// The whole point of the accumulator: four records, three owner names, one host.
    #[test]
    fn the_four_record_types_reassemble_into_one_host() {
        let mut accumulator = Accumulator::default();
        accumulator.absorb(&chromecast_response());

        let hosts = accumulator.resolve();
        let host = hosts
            .get(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)))
            .expect("the A record's address carries the host");

        assert_eq!(host.hostname.as_deref(), Some("chromecast-a1b2c3.local"));
        assert!(host.services.contains("_googlecast._tcp"));
        assert_eq!(
            host.txt.get("md").map(String::as_str),
            Some("Chromecast Ultra")
        );
    }

    /// The name a person typed beats the label a vendor generated. This is most of the
    /// user-visible value of the feature — "Living Room TV" rather than "chromecast-a1b2c3".
    #[test]
    fn a_txt_friendly_name_outranks_the_instance_label() {
        let mut accumulator = Accumulator::default();
        accumulator.absorb(&chromecast_response());

        let hosts = accumulator.resolve();
        let host = &hosts[&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50))];

        assert_eq!(host.instance_name.as_deref(), Some("Living Room TV"));
    }

    /// With no TXT name to prefer, the instance label is the fallback rather than nothing.
    #[test]
    fn the_instance_label_is_used_when_txt_offers_no_name() {
        let mut message = Message::response(0, hickory_resolver::proto::op::OpCode::Query);
        message.add_answer(record(
            "_ipp._tcp.local.",
            RData::PTR(PTR(instance("Office Printer", "_ipp._tcp.local."))),
        ));
        message.add_additional(record_for(
            instance("Office Printer", "_ipp._tcp.local."),
            RData::SRV(SRV::new(0, 0, 631, name("printer.local."))),
        ));
        message.add_additional(record(
            "printer.local.",
            RData::A(A(Ipv4Addr::new(192, 168, 1, 60))),
        ));

        let mut accumulator = Accumulator::default();
        accumulator.absorb(&message.to_bytes().unwrap());

        let hosts = accumulator.resolve();
        let host = &hosts[&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 60))];
        assert_eq!(host.instance_name.as_deref(), Some("Office Printer"));
        assert!(host.services.contains("_ipp._tcp"));
    }

    /// An instance whose SRV or A never arrived has no address to attach to. Recording it against
    /// a guessed address would invent a host; dropping it is the honest outcome.
    #[test]
    fn an_instance_with_no_address_resolves_to_nothing() {
        let mut message = Message::response(0, hickory_resolver::proto::op::OpCode::Query);
        message.add_answer(record(
            "_googlecast._tcp.local.",
            RData::PTR(PTR(name("Orphan._googlecast._tcp.local."))),
        ));

        let mut accumulator = Accumulator::default();
        accumulator.absorb(&message.to_bytes().unwrap());

        assert!(accumulator.resolve().is_empty());
    }

    /// One device answering for several service types is still one host, with both types on it.
    #[test]
    fn a_device_answering_two_service_types_stays_one_host() {
        let mut message = Message::response(0, hickory_resolver::proto::op::OpCode::Query);
        for service in ["_airplay._tcp.local.", "_raop._tcp.local."] {
            message.add_answer(record(
                service,
                RData::PTR(PTR(instance("Apple TV", service))),
            ));
            message.add_additional(record_for(
                instance("Apple TV", service),
                RData::SRV(SRV::new(0, 0, 7000, name("appletv.local."))),
            ));
        }
        message.add_additional(record(
            "appletv.local.",
            RData::A(A(Ipv4Addr::new(192, 168, 1, 70))),
        ));

        let mut accumulator = Accumulator::default();
        accumulator.absorb(&message.to_bytes().unwrap());

        let hosts = accumulator.resolve();
        assert_eq!(hosts.len(), 1, "one device must not become two hosts");
        let host = &hosts[&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 70))];
        assert!(host.services.contains("_airplay._tcp"));
        assert!(host.services.contains("_raop._tcp"));
    }

    /// The enumeration query answers with service *types*, not instances. Treating them as
    /// instances would fabricate a host per service type on the link.
    #[test]
    fn the_service_enumeration_yields_types_to_ask_about_not_hosts() {
        let mut message = Message::response(0, hickory_resolver::proto::op::OpCode::Query);
        message.add_answer(record(
            SERVICE_ENUMERATION,
            RData::PTR(PTR(name("_hap._tcp.local."))),
        ));

        let mut accumulator = Accumulator::default();
        accumulator.absorb(&message.to_bytes().unwrap());

        assert_eq!(
            accumulator.unasked_types(&[]),
            vec![name("_hap._tcp.local.")]
        );
        assert!(
            accumulator
                .unasked_types(&[name("_hap._tcp.local.")])
                .is_empty(),
            "a type we already asked about is not re-asked"
        );
        assert!(
            accumulator.resolve().is_empty(),
            "a service type is not a host"
        );
    }

    /// The group carries every mDNS speaker's traffic, including packets that are not answers to
    /// us and bytes that are not DNS at all.
    #[test]
    fn a_packet_that_does_not_parse_is_skipped() {
        let mut accumulator = Accumulator::default();
        accumulator.absorb(&[]);
        accumulator.absorb(&[0xff; 12]);
        assert!(accumulator.resolve().is_empty());
    }

    /// Round-trips the query builder through the parser, so a change to either that breaks the
    /// pairing is caught here rather than by a silent empty browse.
    #[test]
    fn a_built_query_carries_a_ptr_question_per_name() {
        let names = [name("_googlecast._tcp.local."), name("_hap._tcp.local.")];
        let bytes = build_query(&names).unwrap();

        let parsed = Message::from_bytes(&bytes).unwrap();
        let asked: Vec<Name> = parsed.queries.iter().map(|q| q.name.clone()).collect();

        assert_eq!(asked, names);
        assert!(
            parsed
                .queries
                .iter()
                .all(|q| q.query_type == RecordType::PTR),
            "DNS-SD browses by PTR"
        );
    }
}
