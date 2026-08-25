//! What a simulated device puts on the wire.
//!
//! net-snmp's `pass` protocol is line-based: the handler prints the OID, a type token and the
//! value as three lines, and a data file is one `<oid> <type> <value>` line per instance. This
//! module owns that encoding and the ordering rule that goes with it, so no device definition
//! writes either by hand.

use std::net::Ipv4Addr;

use mac_address::MacAddress;

use crate::daemon::discovery::integration::snmp::oids::oid_parts;

/// A value in the `pass` protocol's vocabulary.
///
/// Deliberately not [`snmp2::Value`]: that type borrows its octet strings and carries no notion of
/// the type *token* a `pass` handler prints, which is the half that goes wrong. `Octets` and `Str`
/// are the same ASN.1 shape and different tokens — `octet` emits raw bytes, `string` emits text —
/// and a MAC written as the second arrives as 17 ASCII bytes where six raw octets belong.
///
/// Construct MAC-valued columns through [`Self::mac`] rather than either variant directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassValue {
    Integer(i64),
    /// A `Gauge32`, held as `u64` and rendered verbatim.
    ///
    /// Wider than the MIB's type on purpose: at least one fixture publishes an `ifSpeed` above
    /// 2^32 (a 10 Gbit/s port writing 10000000000 into a 32-bit column), which is out of spec but
    /// is what the device in the lab has always served. Narrowing it here would silently rewrite
    /// a fixture rather than reporting it.
    Gauge(u64),
    Counter64(u64),
    TimeTicks(u32),
    /// Text. Never a MAC — see [`Self::mac`].
    Str(String),
    /// Raw bytes, emitted as space-separated hex.
    Octets(Vec<u8>),
    IpAddress(Ipv4Addr),
    ObjectId(String),
}

impl PassValue {
    /// A MAC-valued column, in the encoding the modelled firmware actually sends.
    ///
    /// The only way to put a MAC in a fixture. `MacEncoding::Octets` — the default and what
    /// conforming agents send — yields raw bytes; the ASCII forms yield text and have to be asked
    /// for by name, which is what makes the trap visible instead of silent.
    pub fn mac(mac: &MacAddress, encoding: MacEncoding) -> Self {
        match encoding {
            MacEncoding::Octets => Self::Octets(encoding.encode(mac)),
            _ => Self::Str(String::from_utf8_lossy(&encoding.encode(mac)).into_owned()),
        }
    }

    /// The token a `pass` handler prints on its second line.
    pub fn type_token(&self) -> &'static str {
        match self {
            Self::Integer(_) => "integer",
            Self::Gauge(_) => "gauge",
            Self::Counter64(_) => "counter64",
            Self::TimeTicks(_) => "timeticks",
            Self::Str(_) => "string",
            Self::Octets(_) => "octet",
            Self::IpAddress(_) => "ipaddress",
            Self::ObjectId(_) => "objectid",
        }
    }

    /// The third line: the value as `pass` expects to read it back.
    pub fn render(&self) -> String {
        match self {
            Self::Integer(n) => n.to_string(),
            Self::Gauge(n) => n.to_string(),
            Self::Counter64(n) => n.to_string(),
            Self::TimeTicks(n) => n.to_string(),
            Self::Str(s) => s.clone(),
            Self::Octets(bytes) => bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" "),
            Self::IpAddress(ip) => ip.to_string(),
            Self::ObjectId(oid) => oid.clone(),
        }
    }

    /// The same value as the transport hands to the walk layer.
    ///
    /// Borrows from `self`, exactly as a real session's varbinds borrow its response buffer, so a
    /// device-driven test exercises the same lifetimes production does.
    pub fn as_snmp(&self) -> snmp2::Value<'_> {
        match self {
            Self::Integer(n) => snmp2::Value::Integer(*n),
            // Saturating, because a real agent cannot put more than 32 bits on the wire either.
            Self::Gauge(n) => snmp2::Value::Unsigned32(u32::try_from(*n).unwrap_or(u32::MAX)),
            Self::Counter64(n) => snmp2::Value::Counter64(*n),
            Self::TimeTicks(n) => snmp2::Value::Timeticks(*n),
            Self::Str(s) => snmp2::Value::OctetString(s.as_bytes()),
            Self::Octets(bytes) => snmp2::Value::OctetString(bytes),
            Self::IpAddress(ip) => snmp2::Value::IpAddress(ip.octets()),
            // `pass` prints an OID as text and snmpd re-parses it; the walk layer only ever reads
            // these through `value_to_string`, which renders an OID back to the same dotted form.
            Self::ObjectId(oid) => snmp2::Value::OctetString(oid.as_bytes()),
        }
    }
}

pub use crate::server::lldp::MacEncoding;

/// One instance: a fully-qualified OID and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub oid: Vec<u64>,
    pub value: PassValue,
}

impl Row {
    /// A row at `base` (an OID constant from [`super::super::oids`]) extended by `suffix`.
    ///
    /// Takes the base as a `&str` constant rather than a literal: every call site names a
    /// constant, so there is no path by which a fixture invents an OID.
    pub fn at(base: &str, suffix: &[u64], value: PassValue) -> Self {
        let mut oid = oid_parts(base);
        oid.extend_from_slice(suffix);
        Self { oid, value }
    }

    /// A scalar instance, served at `base` exactly as given.
    ///
    /// Nothing is appended: the scalar OID constants carry their own trailing `.0`. A constant
    /// without one is served at the object OID, where a walk of the table finds no instance and
    /// the value silently does not exist.
    pub fn scalar(base: &str, value: PassValue) -> Self {
        let oid = oid_parts(base);
        Self { oid, value }
    }

    fn render(&self) -> String {
        let oid = self
            .oid
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
            .join(".");
        format!(
            ".{} {} {}",
            oid,
            self.value.type_token(),
            self.value.render()
        )
    }
}

/// How a data file's lines are ordered.
///
/// The GETNEXT handler answers with the first line *numerically greater* than the request, so a
/// file out of ascending order silently ends a walk early. That was a hand-maintained property and
/// a review item; here it is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ordering {
    /// Sorted by OID before rendering. Every device but one.
    #[default]
    Ascending,
    /// Rendered in the order the definition lists, for the device that stores a table unsorted and
    /// iterates it positionally (GH #674). Served by `snmp-pass-handler-unsorted.sh`, which walks
    /// its file in file order — the normal handler could only ever produce an ascending sequence,
    /// so a shuffled file would end the walk early rather than reproduce the defect.
    Positional,
}

/// One `pass` data file: the rows a device serves from a single file, in a known order.
#[derive(Debug, Clone)]
pub struct DataFile {
    /// Basename without extension, e.g. `switch-core-01-iftable`.
    pub name: String,
    pub ordering: Ordering,
    rows: Vec<Row>,
}

impl DataFile {
    pub fn new(name: impl Into<String>, ordering: Ordering, rows: Vec<Row>) -> Self {
        Self {
            name: name.into(),
            ordering,
            rows,
        }
    }

    /// The rows in the order the file serves them.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = self.rows.clone();
        if self.ordering == Ordering::Ascending {
            rows.sort_by(|a, b| a.oid.cmp(&b.oid));
        }
        rows
    }

    /// The file's contents, one instance per line.
    pub fn render(&self) -> String {
        let rows = self.rows();
        if rows.is_empty() {
            return String::new();
        }
        let mut out: String = rows
            .iter()
            .map(|row| row.render())
            .collect::<Vec<_>>()
            .join("\n");
        out.push('\n');
        out
    }

    /// Whether this file has no rows.
    ///
    /// An empty file is meaningful, not a mistake: a device that answers a subtree with nothing is
    /// how `switch-mute-01` stops net-snmp answering from the host's own state. Callers that skip
    /// empty files must not skip that one.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::snmp::oids::if_mib;

    fn mac() -> MacAddress {
        "00:ad:24:af:4e:00".parse().unwrap()
    }

    /// The encoding decides the token, and the token is what the daemon's reader dispatches on.
    /// `octet` is six bytes rendered as hex pairs; `string` is seventeen characters of text.
    #[test]
    fn a_mac_column_renders_as_its_encoding_says() {
        let octets = PassValue::mac(&mac(), MacEncoding::Octets);
        assert_eq!(octets.type_token(), "octet");
        assert_eq!(octets.render(), "00 ad 24 af 4e 00");

        let text = PassValue::mac(&mac(), MacEncoding::AsciiLower);
        assert_eq!(text.type_token(), "string");
        assert_eq!(text.render(), "00:ad:24:af:4e:00");
    }

    /// The ordering the GETNEXT handler needs is derived, not authored: rows given out of order
    /// come back ascending, so a mis-sorted definition cannot truncate a walk.
    #[test]
    fn an_ascending_file_sorts_rows_the_definition_listed_out_of_order() {
        let file = DataFile::new(
            "unsorted-input",
            Ordering::Ascending,
            vec![
                Row::at(if_mib::columns::IF_INDEX, &[10], PassValue::Integer(10)),
                Row::at(if_mib::columns::IF_INDEX, &[2], PassValue::Integer(2)),
                Row::at(if_mib::columns::IF_INDEX, &[1], PassValue::Integer(1)),
            ],
        );

        let indexes: Vec<u64> = file
            .rows()
            .iter()
            .map(|row| *row.oid.last().unwrap())
            .collect();
        assert_eq!(indexes, vec![1, 2, 10]);
    }

    /// Numeric, not lexicographic — `.10` sorts above `.2`, which a string sort gets backwards and
    /// which is exactly the mis-sort that ends a walk at the first two-digit index.
    #[test]
    fn ordering_is_numeric_rather_than_textual() {
        let file = DataFile::new(
            "numeric",
            Ordering::Ascending,
            vec![
                Row::at(if_mib::columns::IF_INDEX, &[9], PassValue::Integer(9)),
                Row::at(if_mib::columns::IF_INDEX, &[49153], PassValue::Integer(1)),
                Row::at(if_mib::columns::IF_INDEX, &[10], PassValue::Integer(10)),
            ],
        );
        let rendered = file.render();
        let lines: Vec<&str> = rendered.lines().collect();
        assert!(lines[0].ends_with(".9 integer 9"), "{:?}", lines);
        assert!(lines[1].ends_with(".10 integer 10"), "{:?}", lines);
        assert!(lines[2].ends_with(".49153 integer 1"), "{:?}", lines);
    }

    /// The one device that must not be sorted keeps the order it was written in.
    #[test]
    fn a_positional_file_keeps_the_order_it_was_given() {
        let file = DataFile::new(
            "positional",
            Ordering::Positional,
            vec![
                Row::at(if_mib::columns::IF_INDEX, &[44], PassValue::Integer(44)),
                Row::at(if_mib::columns::IF_INDEX, &[1], PassValue::Integer(1)),
            ],
        );
        let indexes: Vec<u64> = file
            .rows()
            .iter()
            .map(|row| *row.oid.last().unwrap())
            .collect();
        assert_eq!(indexes, vec![44, 1]);
    }
}
