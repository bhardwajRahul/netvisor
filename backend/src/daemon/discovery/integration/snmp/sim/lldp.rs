//! LLDP, local and remote.
//!
//! Identifiers are [`LldpChassisId`] / [`LldpPortId`], so the subtype is the variant and a fixture
//! cannot advertise subtype 4 carrying something that is not an address. What the enums do not
//! carry — because they are post-parse types — is how the bytes leave the agent, and that is
//! [`Advertised`].
//!
//! **The encoding here is not a detail to normalise.** Unlike `ifPhysAddress`, where six raw
//! octets are the only correct form and text is silently dropped, `parse_mac_id` accepts *both*
//! for an LLDP identifier — deliberately, because real firmware sends both. The lab covers both
//! branches on purpose: `switch-dlink-01` sends raw octets (the only end-to-end coverage of that
//! branch), `switch-tplink-01` sends uppercase ASCII, and `switch-exos-01`'s own chassis id is
//! left *abbreviated* (`0:4:96:1:e0:0`) as the standing guard on the unpadded form. Changing any
//! of those removes coverage rather than fixing anything.

use super::wire::{MacEncoding, PassValue, Row};
use crate::daemon::discovery::integration::snmp::oids::lldp;
use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};

/// An identifier together with how the agent puts it on the wire.
///
/// The encoding is only consulted for the MAC-valued variants; everything else is text by
/// definition. Constructing one forces the choice to be made explicitly at the call site, which is
/// the point.
#[derive(Debug, Clone)]
pub struct Advertised<T> {
    pub id: T,
    pub encoding: MacEncoding,
}

impl<T> Advertised<T> {
    /// Six raw octets — what a conforming agent sends.
    pub fn octets(id: T) -> Self {
        Self {
            id,
            encoding: MacEncoding::Octets,
        }
    }

    /// The identifier as text. Legitimate for LLDP — `parse_mac_id` accepts it — and named so the
    /// choice is visible in the device definition rather than implied by a quoted string.
    pub fn text(id: T, encoding: MacEncoding) -> Self {
        Self { id, encoding }
    }
}

/// How a firmware indexes `lldpRemEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeMark {
    /// `lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex`, as the MIB describes.
    At(u32),
    /// The time mark omitted, so every row arrives one sub-id short (GH #668, TP-Link
    /// TL-SX3016F). A parser requiring three sub-ids built no record at all, nothing reached the
    /// discard counters, and the walk still reported itself complete.
    Omitted,
}

/// A chassis column served wrongly, on purpose.
///
/// The one place a fixture may contradict itself, because these are the shapes real firmware
/// produces and each drives a different per-cause counter and a different piece of operator
/// advice (GH #668). Naming them is what keeps "deliberate defect" apart from "mistake".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChassisDefect {
    /// The row appears in no chassis column at all — a ghost row. Indistinguishable from a
    /// chassis column that never had those positions, which is why it reads as `GhostRows`.
    NoChassisColumns,
    /// `lldpRemChassisId` present, its subtype absent — an incomplete record.
    NoSubtype,
    /// The subtype served as an OCTET STRING where an INTEGER belongs, carrying the text the
    /// device actually sends — `macAddress` rather than `4`. Reads as a *complete* walk, with no
    /// truncation signal anywhere, so before the per-cause counters the only evidence was the
    /// record silently going missing.
    SubtypeWrongType(&'static str),
}

/// A neighbour as the agent advertises it.
#[derive(Debug, Clone)]
pub struct RemoteNeighbour {
    pub time_mark: TimeMark,
    pub local_port: u32,
    pub index: u32,
    pub chassis: Option<Advertised<LldpChassisId>>,
    pub port: Option<Advertised<LldpPortId>>,
    pub port_desc: Option<String>,
    pub sys_name: Option<String>,
    pub sys_desc: Option<String>,
    /// Set only where the malformed shape is the point.
    pub defect: Option<ChassisDefect>,
}

impl RemoteNeighbour {
    /// A well-formed neighbour with a time mark of 0, which is what most of the lab serves.
    pub fn new(
        local_port: u32,
        chassis: Advertised<LldpChassisId>,
        port: Advertised<LldpPortId>,
    ) -> Self {
        Self {
            time_mark: TimeMark::At(0),
            local_port,
            index: 1,
            chassis: Some(chassis),
            port: Some(port),
            port_desc: None,
            sys_name: None,
            sys_desc: None,
            defect: None,
        }
    }

    pub fn time_mark(mut self, time_mark: TimeMark) -> Self {
        self.time_mark = time_mark;
        self
    }

    pub fn index(mut self, index: u32) -> Self {
        self.index = index;
        self
    }

    pub fn port_desc(mut self, desc: &str) -> Self {
        self.port_desc = Some(desc.to_string());
        self
    }

    pub fn sys_name(mut self, name: &str) -> Self {
        self.sys_name = Some(name.to_string());
        self
    }

    pub fn sys_desc(mut self, desc: &str) -> Self {
        self.sys_desc = Some(desc.to_string());
        self
    }

    pub fn defect(mut self, defect: ChassisDefect) -> Self {
        self.defect = Some(defect);
        self
    }

    /// The index sub-ids this row is keyed by — three normally, two where the firmware omits the
    /// time mark.
    fn suffix(&self) -> Vec<u64> {
        match self.time_mark {
            TimeMark::At(mark) => vec![mark as u64, self.local_port as u64, self.index as u64],
            TimeMark::Omitted => vec![self.local_port as u64, self.index as u64],
        }
    }

    fn wire_rows(&self) -> Vec<Row> {
        let suffix = self.suffix();
        let mut rows = Vec::new();

        if let Some(chassis) = &self.chassis {
            let (subtype, value) = chassis.id.to_snmp(chassis.encoding);
            match self.defect {
                // Lists the row in no chassis column at all.
                Some(ChassisDefect::NoChassisColumns) => {}
                Some(ChassisDefect::NoSubtype) => {
                    rows.push(Row::at(
                        lldp::remote::entry::LLDP_REM_CHASSIS_ID,
                        &suffix,
                        chassis_value(&chassis.id, value),
                    ));
                }
                Some(ChassisDefect::SubtypeWrongType(text)) => {
                    rows.push(Row::at(
                        lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
                        &suffix,
                        PassValue::Str(text.to_string()),
                    ));
                    rows.push(Row::at(
                        lldp::remote::entry::LLDP_REM_CHASSIS_ID,
                        &suffix,
                        chassis_value(&chassis.id, value),
                    ));
                }
                None => {
                    rows.push(Row::at(
                        lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
                        &suffix,
                        PassValue::Integer(subtype as i64),
                    ));
                    rows.push(Row::at(
                        lldp::remote::entry::LLDP_REM_CHASSIS_ID,
                        &suffix,
                        chassis_value(&chassis.id, value),
                    ));
                }
            }
        }

        if let Some(port) = &self.port {
            let (subtype, value) = port.id.to_snmp(port.encoding);
            rows.push(Row::at(
                lldp::remote::entry::LLDP_REM_PORT_ID_SUBTYPE,
                &suffix,
                PassValue::Integer(subtype as i64),
            ));
            rows.push(Row::at(
                lldp::remote::entry::LLDP_REM_PORT_ID,
                &suffix,
                port_value(&port.id, value),
            ));
        }
        for (base, text) in [
            (lldp::remote::entry::LLDP_REM_PORT_DESC, &self.port_desc),
            (lldp::remote::entry::LLDP_REM_SYS_NAME, &self.sys_name),
            (lldp::remote::entry::LLDP_REM_SYS_DESC, &self.sys_desc),
        ] {
            if let Some(text) = text {
                rows.push(Row::at(base, &suffix, PassValue::Str(text.clone())));
            }
        }
        rows
    }
}

/// An identifier's wire bytes as the right `pass` value: raw octets stay `octet`, everything else
/// is text. A MAC asked for in an ASCII encoding has already become text in `to_snmp`.
fn chassis_value(id: &LldpChassisId, bytes: Vec<u8>) -> PassValue {
    match id {
        LldpChassisId::MacAddress(_) | LldpChassisId::NetworkAddress(_) if bytes.len() <= 17 => {
            octet_or_text(bytes)
        }
        _ => PassValue::Str(String::from_utf8_lossy(&bytes).into_owned()),
    }
}

fn port_value(id: &LldpPortId, bytes: Vec<u8>) -> PassValue {
    match id {
        LldpPortId::MacAddress(_) | LldpPortId::NetworkAddress(_) => octet_or_text(bytes),
        _ => PassValue::Str(String::from_utf8_lossy(&bytes).into_owned()),
    }
}

/// Six bytes is a raw address; anything longer is the ASCII rendering of one.
fn octet_or_text(bytes: Vec<u8>) -> PassValue {
    if bytes.len() == 6 {
        PassValue::Octets(bytes)
    } else {
        PassValue::Str(String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// One row of `lldpLocPortTable`, keyed by `lldpLocPortNum`.
///
/// That key is a separate namespace from `ifIndex` on some firmware — ExtremeXOS numbers 1..N
/// against ifIndex 1001+, and the Dell OS10 runs 4 and 555-570 against ifIndex values in the
/// millions — which is why the daemon walks this table at all.
#[derive(Debug, Clone)]
pub struct LocalPort {
    pub num: u32,
    pub id: Advertised<LldpPortId>,
    pub desc: Option<String>,
}

impl LocalPort {
    pub fn new(num: u32, id: Advertised<LldpPortId>) -> Self {
        Self {
            num,
            id,
            desc: None,
        }
    }

    pub fn desc(mut self, desc: &str) -> Self {
        self.desc = Some(desc.to_string());
        self
    }
}

/// A device's whole LLDP MIB: who it says it is, its local ports, and its neighbours.
#[derive(Debug, Clone)]
pub struct LldpTable {
    pub chassis: Advertised<LldpChassisId>,
    pub sys_name: String,
    pub sys_desc: Option<String>,
    pub local_ports: Vec<LocalPort>,
    pub neighbours: Vec<RemoteNeighbour>,
}

impl LldpTable {
    pub fn new(chassis: Advertised<LldpChassisId>, sys_name: &str) -> Self {
        Self {
            chassis,
            sys_name: sys_name.to_string(),
            sys_desc: None,
            local_ports: Vec::new(),
            neighbours: Vec::new(),
        }
    }

    pub fn sys_desc(mut self, desc: &str) -> Self {
        self.sys_desc = Some(desc.to_string());
        self
    }

    pub fn local_ports(mut self, ports: Vec<LocalPort>) -> Self {
        self.local_ports = ports;
        self
    }

    pub fn neighbours(mut self, neighbours: Vec<RemoteNeighbour>) -> Self {
        self.neighbours = neighbours;
        self
    }

    pub fn wire_rows(&self) -> Vec<Row> {
        let (subtype, value) = self.chassis.id.to_snmp(self.chassis.encoding);
        let mut rows = vec![
            Row::scalar(
                lldp::local::LLDP_LOC_CHASSIS_ID_SUBTYPE,
                PassValue::Integer(subtype as i64),
            ),
            Row::scalar(
                lldp::local::LLDP_LOC_CHASSIS_ID,
                chassis_value(&self.chassis.id, value),
            ),
            Row::scalar(
                lldp::local::LLDP_LOC_SYS_NAME,
                PassValue::Str(self.sys_name.clone()),
            ),
        ];
        if let Some(desc) = &self.sys_desc {
            rows.push(Row::scalar(
                lldp::local::LLDP_LOC_SYS_DESC,
                PassValue::Str(desc.clone()),
            ));
        }

        for port in &self.local_ports {
            let suffix = [port.num as u64];
            let (subtype, value) = port.id.id.to_snmp(port.id.encoding);
            rows.push(Row::at(
                lldp::local::LLDP_LOC_PORT_ID_SUBTYPE,
                &suffix,
                PassValue::Integer(subtype as i64),
            ));
            rows.push(Row::at(
                lldp::local::LLDP_LOC_PORT_ID,
                &suffix,
                port_value(&port.id.id, value),
            ));
            if let Some(desc) = &port.desc {
                rows.push(Row::at(
                    lldp::local::LLDP_LOC_PORT_DESC,
                    &suffix,
                    PassValue::Str(desc.clone()),
                ));
            }
        }

        rows.extend(self.neighbours.iter().flat_map(RemoteNeighbour::wire_rows));
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chassis() -> Advertised<LldpChassisId> {
        Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()))
    }

    fn port() -> Advertised<LldpPortId> {
        Advertised::octets(LldpPortId::InterfaceName("Gi0/1".into()))
    }

    fn suffixes(rows: &[Row], column: &str) -> Vec<Vec<u64>> {
        let base_len = crate::daemon::discovery::integration::snmp::oids::oid_parts(column).len();
        rows.iter()
            .filter(|row| {
                row.oid.len() > base_len
                    && row.oid[..base_len]
                        == crate::daemon::discovery::integration::snmp::oids::oid_parts(column)[..]
            })
            .map(|row| row.oid[base_len..].to_vec())
            .collect()
    }

    /// The MIB's three-part index. Every device but one.
    #[test]
    fn a_neighbour_is_keyed_by_time_mark_port_and_index() {
        let rows = RemoteNeighbour::new(2, chassis(), port())
            .time_mark(TimeMark::At(31577700))
            .index(3)
            .wire_rows();
        assert_eq!(
            suffixes(&rows, lldp::remote::entry::LLDP_REM_CHASSIS_ID),
            vec![vec![31577700, 2, 3]]
        );
    }

    /// GH #668: firmware that omits `lldpRemTimeMark` indexes on the remaining two sub-ids, so
    /// every row arrives one shorter. The shape that made a sixteen-port switch vanish without
    /// raising a warning of any kind.
    #[test]
    fn a_neighbour_indexed_without_a_time_mark_is_one_sub_id_shorter() {
        let rows = RemoteNeighbour::new(1, chassis(), port())
            .time_mark(TimeMark::Omitted)
            .wire_rows();
        assert_eq!(
            suffixes(&rows, lldp::remote::entry::LLDP_REM_CHASSIS_ID),
            vec![vec![1, 1]]
        );
    }

    /// A ghost row lists itself in no chassis column, which is what makes it indistinguishable
    /// from a column that never held those positions.
    #[test]
    fn a_ghost_row_appears_in_neither_chassis_column() {
        let rows = RemoteNeighbour::new(2, chassis(), port())
            .defect(ChassisDefect::NoChassisColumns)
            .sys_name("switch-core-01")
            .wire_rows();

        assert!(suffixes(&rows, lldp::remote::entry::LLDP_REM_CHASSIS_ID).is_empty());
        assert!(suffixes(&rows, lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE).is_empty());
        // ...while the rest of the row is served, which is what makes it a *row* rather than an
        // absence.
        assert_eq!(
            suffixes(&rows, lldp::remote::entry::LLDP_REM_SYS_NAME),
            vec![vec![0, 2, 1]]
        );
    }

    /// The subtype served as text where an integer belongs. It reads as a complete walk, which is
    /// why it needed its own counter to become visible at all.
    #[test]
    fn a_wrong_typed_subtype_is_served_as_a_string() {
        let rows = RemoteNeighbour::new(1, chassis(), port())
            .defect(ChassisDefect::SubtypeWrongType("macAddress"))
            .wire_rows();
        let subtype = rows
            .iter()
            .find(|row| {
                row.oid.starts_with(
                    &crate::daemon::discovery::integration::snmp::oids::oid_parts(
                        lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
                    ),
                )
            })
            .expect("the subtype column is served — that is what makes the walk look complete");
        assert_eq!(subtype.value.type_token(), "string");
    }

    /// Both encodings are legitimate here and both are in the lab on purpose. The model must be
    /// able to express either without one being the accident.
    #[test]
    fn a_chassis_id_can_be_advertised_as_octets_or_as_text() {
        let id = LldpChassisId::MacAddress("00:ad:24:af:4e:00".into());

        let raw = LldpTable::new(Advertised::octets(id.clone()), "switch-dlink-01").wire_rows();
        let raw_value = &raw[1].value;
        assert_eq!(raw_value.type_token(), "octet");
        assert_eq!(raw_value.render(), "00 ad 24 af 4e 00");

        let text = LldpTable::new(
            Advertised::text(id, MacEncoding::AsciiUpper),
            "switch-tplink-01",
        )
        .wire_rows();
        let text_value = &text[1].value;
        assert_eq!(text_value.type_token(), "string");
        assert_eq!(text_value.render(), "00:AD:24:AF:4E:00");
    }
}
