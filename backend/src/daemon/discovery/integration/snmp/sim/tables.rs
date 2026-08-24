//! MIB tables, built from the daemon's own collection types.
//!
//! A fixture row is the type the collection produces, plus only the wire facts that type
//! deliberately drops — how a MAC is encoded, and which ifXTable columns the firmware implements.
//! Reusing the collection types is what keeps a fixture and the thing it is a fixture *for* from
//! describing an interface differently.

use mac_address::MacAddress;

use super::wire::{MacEncoding, PassValue, Row};
use crate::daemon::discovery::integration::snmp::oids::if_mib;
use crate::daemon::discovery::integration::snmp::types::IfTableEntry;
use crate::server::interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, if_type};

/// One interface as a fixture serves it.
///
/// [`IfTableEntry`] carries every column. `serves_high_speed` is the one thing it cannot: the
/// collection folds `ifHighSpeed` into `if_speed`, but whether the agent answers that ifXTable
/// column at all varies by firmware and is load-bearing here — ten of the lab's devices implement
/// it and eleven do not.
#[derive(Debug, Clone)]
pub struct IfRow {
    pub entry: IfTableEntry,
    pub serves_high_speed: Option<HighSpeed>,
    /// The device answers `ifPhysAddress` with an *empty* value.
    ///
    /// Distinct from not serving the column at all, which is why it is not just
    /// `if_phys_address: None`. A loopback has no hardware address and says so by returning a
    /// zero-length OCTET STRING; the row exists in the walk and carries nothing.
    pub serves_empty_phys_address: bool,
}

impl IfRow {
    /// A physical ethernet port, admin and oper up — what most rows are.
    pub fn port(if_index: i32, if_descr: &str, phys_address: Option<MacAddress>) -> Self {
        Self {
            entry: IfTableEntry {
                if_index,
                if_descr: Some(if_descr.to_string()),
                if_type: Some(if_type::ETHERNET_CSMA_CD),
                if_speed: Some(1_000_000_000),
                if_phys_address: phys_address,
                if_admin_status: Some(IfAdminStatus::Up.into()),
                if_oper_status: Some(IfOperStatus::Up.into()),
                ..Default::default()
            },
            serves_high_speed: None,
            serves_empty_phys_address: false,
        }
    }

    /// A non-physical row — a VLAN interface, a loopback. `if_type` decides whether the L2 view
    /// and the MAC-uniqueness rules count it, so it is named rather than defaulted.
    pub fn virtual_if(if_index: i32, if_descr: &str, if_type: i32) -> Self {
        let mut row = Self::port(if_index, if_descr, None);
        row.entry.if_type = Some(if_type);
        row.entry.if_speed = Some(0);
        row
    }

    pub fn name(mut self, if_name: &str) -> Self {
        self.entry.if_name = Some(if_name.to_string());
        self
    }

    pub fn alias(mut self, if_alias: &str) -> Self {
        self.entry.if_alias = Some(if_alias.to_string());
        self
    }

    pub fn mac(mut self, mac: MacAddress) -> Self {
        self.entry.if_phys_address = Some(mac);
        self
    }

    /// Serve `ifPhysAddress` with nothing in it — what an interface with no hardware address
    /// reports. The column is present in the walk and holds no address.
    pub fn no_hardware_address(mut self) -> Self {
        self.serves_empty_phys_address = true;
        self
    }

    /// Admin and oper status down. A port that is present and not passing traffic.
    pub fn down(mut self) -> Self {
        self.entry.if_admin_status = Some(IfAdminStatus::Down.into());
        self.entry.if_oper_status = Some(IfOperStatus::Down.into());
        self
    }

    /// `ifMtu`. Served by the two devices that publish it and by nothing else, so it is asked
    /// for rather than defaulted.
    pub fn mtu(mut self, mtu: i32) -> Self {
        self.entry.if_mtu = Some(mtu);
        self
    }

    pub fn speed(mut self, bits_per_second: u64) -> Self {
        self.entry.if_speed = Some(bits_per_second);
        self
    }

    /// Serve ifXTable's `ifHighSpeed`, derived from `ifSpeed`.
    ///
    /// The value is not stored: it is `ifSpeed` in Mbit/s, which is what the MIB defines it as, so
    /// the two cannot drift apart the way a hand-written pair can. True of every port in the lab
    /// but two — see [`Self::high_speed_mbps`].
    pub fn high_speed(mut self) -> Self {
        self.serves_high_speed = Some(HighSpeed::FromSpeed);
        self
    }

    /// Serve an `ifHighSpeed` the device's own `ifSpeed` does not imply.
    ///
    /// A Wi-Fi radio reports `0` in the 32-bit column and its real rate here, so on
    /// `ap-wireless-01` the two genuinely disagree. Naming the value is how a real exception is
    /// told apart from a stale hand-written pair.
    pub fn high_speed_mbps(mut self, mbps: u32) -> Self {
        self.serves_high_speed = Some(HighSpeed::Mbps(mbps));
        self
    }

    /// Oper status down while admin stays up: a port that is enabled with nothing plugged in.
    pub fn oper_down(mut self) -> Self {
        self.entry.if_oper_status = Some(IfOperStatus::Down.into());
        self
    }

    fn high_speed_value(&self) -> Option<u32> {
        match self.serves_high_speed? {
            HighSpeed::FromSpeed => Some(
                u32::try_from(self.entry.if_speed.unwrap_or(0) / 1_000_000).unwrap_or(u32::MAX),
            ),
            HighSpeed::Mbps(mbps) => Some(mbps),
        }
    }
}

/// Where a row's `ifHighSpeed` comes from.
#[derive(Debug, Clone, Copy)]
pub enum HighSpeed {
    /// `ifSpeed` in Mbit/s, which is the MIB's own definition of the column.
    FromSpeed,
    /// A rate the device's `ifSpeed` does not imply, because the firmware reports `0` there.
    Mbps(u32),
}

/// What a device claims about itself, as opposed to what it serves.
#[derive(Debug, Clone, Copy, Default)]
pub enum IfNumber {
    /// Counted from the rows the table holds. Every device but one, so editing an ifTable cannot
    /// leave a stale count behind and turn every scan of that device into a false warning.
    #[default]
    Derived,
    /// A device that misreports itself on purpose, so the daemon's count check can be watched
    /// firing rather than only staying quiet. `switch-dell-01` declares 52 and serves 23 (GH #685).
    Declares(i32),
}

/// A device's ifTable and ifXTable.
#[derive(Debug, Clone)]
pub struct IfTable {
    pub rows: Vec<IfRow>,
    /// How every `ifPhysAddress` on this device goes on the wire. One setting per device because
    /// firmware does not vary it per port; [`MacEncoding::Octets`] is what a conforming agent
    /// sends and what a fixture gets unless it says otherwise.
    pub mac_encoding: MacEncoding,
    pub declares: IfNumber,
}

impl IfTable {
    pub fn new(rows: Vec<IfRow>) -> Self {
        Self {
            rows,
            mac_encoding: MacEncoding::Octets,
            declares: IfNumber::Derived,
        }
    }

    pub fn declaring(mut self, declares: IfNumber) -> Self {
        self.declares = declares;
        self
    }

    /// `ifNumber.0` — what the device tells a collection to expect.
    pub fn declared_count(&self) -> i32 {
        match self.declares {
            IfNumber::Derived => i32::try_from(self.rows.len()).unwrap_or(i32::MAX),
            IfNumber::Declares(count) => count,
        }
    }

    /// Whether any row implements an ifXTable column, which is what decides if the device
    /// registers that subtree at all. `switch-tplink-01` implements none: its ports are known only
    /// by `ifDescr`, which is what its neighbour port ids have to be matched against.
    pub fn serves_if_x_table(&self) -> bool {
        self.rows.iter().any(|row| {
            row.entry.if_name.is_some()
                || row.entry.if_alias.is_some()
                || row.serves_high_speed.is_some()
        })
    }

    /// The bridge ports an unconfigured managed switch reports: its own ethernet interfaces, in
    /// ifIndex order. VLAN and loopback rows are not bridge ports.
    pub fn ethernet_if_indexes(&self) -> Vec<i32> {
        let mut indexes: Vec<i32> = self
            .rows
            .iter()
            .filter(|row| row.entry.if_type == Some(if_type::ETHERNET_CSMA_CD))
            .map(|row| row.entry.if_index)
            .collect();
        indexes.sort_unstable();
        indexes
    }

    /// Every instance this table serves, `ifNumber` included.
    pub fn wire_rows(&self) -> Vec<Row> {
        let mut rows = vec![Row::scalar(
            if_mib::IF_NUMBER,
            PassValue::Integer(self.declared_count() as i64),
        )];

        for row in &self.rows {
            let idx = &[row.entry.if_index as u64][..];
            let e = &row.entry;
            rows.push(Row::at(
                if_mib::columns::IF_INDEX,
                idx,
                PassValue::Integer(e.if_index as i64),
            ));
            if let Some(descr) = &e.if_descr {
                rows.push(Row::at(
                    if_mib::columns::IF_DESCR,
                    idx,
                    PassValue::Str(descr.clone()),
                ));
            }
            if let Some(t) = e.if_type {
                rows.push(Row::at(
                    if_mib::columns::IF_TYPE,
                    idx,
                    PassValue::Integer(t as i64),
                ));
            }
            if let Some(mtu) = e.if_mtu {
                rows.push(Row::at(
                    if_mib::columns::IF_MTU,
                    idx,
                    PassValue::Integer(mtu as i64),
                ));
            }
            if let Some(speed) = e.if_speed {
                rows.push(Row::at(
                    if_mib::columns::IF_SPEED,
                    idx,
                    PassValue::Gauge(speed),
                ));
            }
            if let Some(mac) = &e.if_phys_address {
                rows.push(Row::at(
                    if_mib::columns::IF_PHYS_ADDRESS,
                    idx,
                    PassValue::mac(mac, self.mac_encoding),
                ));
            } else if row.serves_empty_phys_address {
                rows.push(Row::at(
                    if_mib::columns::IF_PHYS_ADDRESS,
                    idx,
                    PassValue::Str(String::new()),
                ));
            }
            if let Some(status) = e.if_admin_status {
                rows.push(Row::at(
                    if_mib::columns::IF_ADMIN_STATUS,
                    idx,
                    PassValue::Integer(status as i64),
                ));
            }
            if let Some(status) = e.if_oper_status {
                rows.push(Row::at(
                    if_mib::columns::IF_OPER_STATUS,
                    idx,
                    PassValue::Integer(status as i64),
                ));
            }
            if let Some(name) = &e.if_name {
                rows.push(Row::at(
                    if_mib::if_x_table::IF_NAME,
                    idx,
                    PassValue::Str(name.clone()),
                ));
            }
            if let Some(high_speed) = row.high_speed_value() {
                rows.push(Row::at(
                    if_mib::if_x_table::IF_HIGH_SPEED,
                    idx,
                    PassValue::Gauge(high_speed as u64),
                ));
            }
            if let Some(alias) = &e.if_alias {
                rows.push(Row::at(
                    if_mib::if_x_table::IF_ALIAS,
                    idx,
                    PassValue::Str(alias.clone()),
                ));
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(last: u8) -> MacAddress {
        MacAddress::new([0x00, 0x1a, 0x2b, 0x00, 0x10, last])
    }

    /// The count follows the rows. Adding a port is enough; there is no second figure to update,
    /// which is what the old "do not hand-write ifNumber" instruction was compensating for.
    #[test]
    fn adding_a_port_moves_the_declared_count_with_it() {
        let mut table = IfTable::new(vec![
            IfRow::port(1, "GigabitEthernet0/1", Some(mac(1))),
            IfRow::port(2, "GigabitEthernet0/2", Some(mac(2))),
        ]);
        assert_eq!(table.declared_count(), 2);

        table
            .rows
            .push(IfRow::port(3, "GigabitEthernet0/3", Some(mac(3))));
        assert_eq!(table.declared_count(), 3);
    }

    /// The one device that must disagree with itself says so in one place (GH #685).
    #[test]
    fn a_device_can_declare_more_interfaces_than_it_serves() {
        let table = IfTable::new(vec![IfRow::port(1, "ethernet1/1/1", None)])
            .declaring(IfNumber::Declares(52));
        assert_eq!(table.declared_count(), 52);
        assert_eq!(table.rows.len(), 1);
    }

    /// A device implementing no ifXTable column does not register the subtree — the shape
    /// `switch-tplink-01` needs, where ports are known only by `ifDescr`.
    #[test]
    fn a_table_with_no_if_x_columns_does_not_serve_the_subtree() {
        let bare = IfTable::new(vec![IfRow::port(1, "ten-gigabitEthernet 1/0/1", None)]);
        assert!(!bare.serves_if_x_table());

        let named = IfTable::new(vec![
            IfRow::port(1, "GigabitEthernet0/1", None).name("Gi0/1"),
        ]);
        assert!(named.serves_if_x_table());
    }

    /// `ifHighSpeed` is `ifSpeed` in Mbit/s, derived at emission. Changing the speed moves both,
    /// so the pair cannot go stale against each other.
    #[test]
    fn high_speed_follows_the_speed_it_is_derived_from() {
        let row = IfRow::port(1, "Gi0/1", None)
            .speed(10_000_000_000)
            .high_speed();
        assert_eq!(row.high_speed_value(), Some(10_000));

        // ...and a radio whose 32-bit column reads 0 states its rate instead of deriving one.
        let radio = IfRow::port(2, "ath0", None).speed(0).high_speed_mbps(867);
        assert_eq!(radio.high_speed_value(), Some(867));
    }

    /// Bridge ports are the ethernet rows only: a VLAN interface is not a port a cable lands on.
    #[test]
    fn bridge_ports_exclude_virtual_interfaces() {
        let table = IfTable::new(vec![
            IfRow::port(1, "GigabitEthernet0/1", Some(mac(1))),
            IfRow::port(2, "GigabitEthernet0/2", Some(mac(2))),
            IfRow::virtual_if(4, "Vlan10", if_type::PROP_VIRTUAL),
            IfRow::virtual_if(5, "lo", if_type::SOFTWARE_LOOPBACK),
        ]);
        assert_eq!(table.ethernet_if_indexes(), vec![1, 2]);
    }

    /// A MAC reaches the wire as octets without anyone asking, and only becomes text if a device
    /// says so. This is the trap the model exists to close.
    #[test]
    fn port_macs_are_octets_unless_the_device_says_otherwise() {
        let table = IfTable::new(vec![IfRow::port(1, "Gi0/1", Some(mac(1)))]);
        let phys = table
            .wire_rows()
            .into_iter()
            .find(|row| row.value.type_token() == "octet")
            .expect("a port MAC should be emitted as octets");
        assert_eq!(phys.value.render(), "00 1a 2b 00 10 01");
    }
}
