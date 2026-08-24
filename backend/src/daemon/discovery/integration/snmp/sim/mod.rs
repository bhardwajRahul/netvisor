//! The simulated SNMP devices, defined once as typed values.
//!
//! Every device in `tools/snmp/` lives here and nowhere else. The deployment generates its data
//! files and agent configs from these definitions rather than carrying a second copy, and each
//! device's regression test drives the real collection path against it — so a fixture that stops
//! exercising the defect it exists for fails a test rather than passing quietly.
//!
//! Compiled under `cfg(test)` and behind the `snmp-sim` feature, which the fixture generator
//! enables; the shipped `server` and `daemon` binaries carry none of it.

pub mod devices;
pub mod emit;
pub mod harness;
pub mod lldp;
pub mod mibs;
pub mod tables;
pub mod transport;
pub mod wire;

use std::net::Ipv4Addr;

use transport::{Handler, Registration, SimAgent};
use wire::{DataFile, Ordering};

use crate::daemon::discovery::integration::snmp::oids::{
    arp, bridge, cdp, entity, if_mib, ip_mib, lldp as lldp_oids, oid_parts,
};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

/// Why a device is in the lab.
///
/// Required rather than optional: a device with no established defect has to say so. "A fixture
/// whose purpose is not written down is one nobody dares change and nobody can verify" was a
/// review instruction in `SNMP-TEST-ENV.md`; here it is not expressible.
#[derive(Debug, Clone, Copy)]
pub enum Purpose {
    /// Reproduces a specific reported defect. `issue` names the GitHub issue or the report it
    /// came from; `defect` says what goes wrong when the guard is absent.
    Regression {
        issue: &'static str,
        defect: &'static str,
    },
    /// No defect of its own: a far end other devices resolve against, or a healthy baseline that
    /// demonstrates a check staying quiet.
    Control { role: &'static str },
}

/// Every MIB a device serves.
#[derive(Debug, Clone, Default)]
pub struct Tables {
    pub if_table: Option<tables::IfTable>,
    pub lldp: Option<lldp::LldpTable>,
    pub bridge: mibs::BridgeTable,
    pub arp: mibs::ArpTable,
    pub ip_addr: mibs::IpAddrTable,
    pub entity: mibs::EntityTable,
    pub cdp: mibs::CdpTable,
    /// Alternative LLDP files written alongside the active one and swapped in by hand. Only
    /// `switch-flaky-01` has these; each drives a different discard counter.
    pub lldp_variants: Vec<(&'static str, lldp::LldpTable)>,
    /// A second bridge table served in a named SNMP context, through a proxied back-end agent.
    /// Only `switch-cisco-01`, which is what GH #686 is about.
    pub context_bridge: Option<mibs::BridgeTable>,
}

/// A simulated device: everything the deployment and the tests need, in one value.
#[derive(Debug, Clone)]
pub struct SimDevice {
    /// `sysName`, and the prefix of every file this device serves.
    pub name: &'static str,
    pub ip: Ipv4Addr,
    pub purpose: Purpose,
    pub credential: CredentialType,
    pub system: SystemInfo,
    pub tables: Tables,
    /// Which `pass` handler serves the ARP table. Two devices deliberately misbehave here.
    pub arp_handler: Handler,
    /// Subtrees this device registers against an empty file, to stop net-snmp answering them.
    ///
    /// `ifTable`/`ifXTable` are suppressed by the `-I` flag every unit carries, but `ipAddrTable`
    /// and `ipNetToMediaTable` cannot be — without these overrides a device that is supposed to
    /// serve nothing would report the host's own addresses and ARP cache and would not be mute.
    pub suppresses: Vec<&'static str>,
}

/// The file suffixes, kept here so the deployment and the registrations cannot disagree about
/// what a device's files are called.
const IFTABLE: &str = "iftable";
const LLDP: &str = "lldp";
const BRIDGE: &str = "bridge";
const ARP: &str = "arp";
const IPADDR: &str = "ipaddr";
const ENTITY: &str = "entity";
const CDP: &str = "cdp";
pub(crate) const VLAN20: &str = "vlan20";
const EMPTY: &str = "empty";

impl SimDevice {
    fn file(&self, suffix: &str, ordering: Ordering, rows: Vec<wire::Row>) -> DataFile {
        DataFile::new(format!("{}-{}", self.name, suffix), ordering, rows)
    }

    fn ethernet_if_indexes(&self) -> Vec<i32> {
        self.tables
            .if_table
            .as_ref()
            .map(tables::IfTable::ethernet_if_indexes)
            .unwrap_or_default()
    }

    /// The `pass` data files this device serves, in a fixed order that the registrations index
    /// into.
    pub fn data_files(&self) -> Vec<DataFile> {
        let mut files = Vec::new();
        if let Some(if_table) = &self.tables.if_table {
            files.push(self.file(IFTABLE, Ordering::Ascending, if_table.wire_rows()));
        }
        if let Some(table) = &self.tables.lldp {
            // A device with swappable variants serves an `-active` slot rather than a fixed file:
            // the handler re-reads its file per request, so copying a variant over it takes effect
            // with no snmpd restart.
            let suffix = if self.tables.lldp_variants.is_empty() {
                LLDP.to_string()
            } else {
                format!("{LLDP}-active")
            };
            files.push(self.file(&suffix, Ordering::Ascending, table.wire_rows()));
        }
        if !self.tables.bridge.is_empty() {
            let rows = self.tables.bridge.wire_rows(&self.ethernet_if_indexes());
            files.push(self.file(BRIDGE, Ordering::Ascending, rows));
        }
        if !self.tables.arp.is_empty() {
            // The one device whose table is served out of ascending order keeps the order it was
            // written in; everything else is sorted.
            let ordering = match self.arp_handler {
                Handler::Positional => Ordering::Positional,
                _ => Ordering::Ascending,
            };
            files.push(self.file(ARP, ordering, self.tables.arp.wire_rows()));
        }
        if !self.tables.ip_addr.is_empty() {
            files.push(self.file(IPADDR, Ordering::Ascending, self.tables.ip_addr.wire_rows()));
        }
        if !self.tables.entity.is_empty() {
            files.push(self.file(ENTITY, Ordering::Ascending, self.tables.entity.wire_rows()));
        }
        if !self.tables.cdp.is_empty() {
            files.push(self.file(CDP, Ordering::Ascending, self.tables.cdp.wire_rows()));
        }
        if !self.suppresses.is_empty() {
            files.push(self.file(EMPTY, Ordering::Ascending, Vec::new()));
        }
        files
    }

    /// The file the context back end serves.
    ///
    /// `pass` takes no context argument — it registers into the default context and nothing else —
    /// so a handler cannot be scoped to a context directly. A second snmpd on loopback holds this
    /// table and the front agent reaches it with `proxy -Cn vlan-20`. It is genuinely two agents,
    /// which is why this is not one of [`Self::data_files`].
    pub fn context_files(&self) -> Vec<DataFile> {
        self.tables
            .context_bridge
            .iter()
            .map(|context| {
                self.file(
                    VLAN20,
                    Ordering::Ascending,
                    context.wire_rows(&self.ethernet_if_indexes()),
                )
            })
            .collect()
    }

    /// The back end's own registration: the whole bridge MIB, from its own file.
    pub fn context_registrations(&self) -> Vec<Registration> {
        self.context_files()
            .iter()
            .enumerate()
            .map(|(file, _)| Registration {
                subtree: oid_parts(bridge::BRIDGE_MIB),
                file,
                handler: Handler::Normal,
            })
            .collect()
    }

    /// The swappable LLDP variants, written beside the active file but served by nobody until
    /// copied over it. Deliberately not in [`Self::data_files`]: they are not part of the
    /// device's normal answer.
    pub fn variant_files(&self) -> Vec<DataFile> {
        self.tables
            .lldp_variants
            .iter()
            .map(|(name, table)| {
                DataFile::new(
                    format!("{}-lldp-{}", self.name, name),
                    Ordering::Ascending,
                    table.wire_rows(),
                )
            })
            .collect()
    }

    /// The `pass` registrations this device's snmpd config carries.
    ///
    /// Derived from the tables the device actually holds, which is what retires the deploy
    /// script's section 5c: a config cannot name a data file nobody wrote, a data file cannot go
    /// unserved, and a device serving an ifTable cannot forget to register `ifNumber`, because
    /// none of those is written down separately any more.
    pub fn registrations(&self) -> Vec<Registration> {
        let files = self.data_files();
        let index = |suffix: &str| {
            let wanted = format!("{}-{}", self.name, suffix);
            files.iter().position(|f| f.name == wanted)
        };
        let mut registrations = Vec::new();

        if let Some(file) = index(IFTABLE) {
            // `mibII/interfaces` owns this scalar, so without its own registration the device
            // answers its interface count from the host's kernel state — a contradiction reported
            // on every scan for a fault it does not have.
            registrations.push(Registration {
                subtree: oid_parts(if_mib::IF_NUMBER_OBJECT),
                file,
                handler: Handler::Normal,
            });
            registrations.push(Registration {
                subtree: oid_parts(if_mib::IF_TABLE),
                file,
                handler: Handler::Normal,
            });
            if self
                .tables
                .if_table
                .as_ref()
                .is_some_and(tables::IfTable::serves_if_x_table)
            {
                registrations.push(Registration {
                    subtree: oid_parts(if_mib::if_x_table::IF_X_TABLE),
                    file,
                    handler: Handler::Normal,
                });
            }
        }
        let lldp_file = if self.tables.lldp_variants.is_empty() {
            LLDP.to_string()
        } else {
            format!("{LLDP}-active")
        };
        for (suffix, subtree) in [
            (lldp_file.as_str(), lldp_oids::LLDP_MIB),
            (BRIDGE, bridge::BRIDGE_MIB),
            (ENTITY, entity::ENTITY_MIB),
            (CDP, cdp::CDP_MIB),
        ] {
            if let Some(file) = index(suffix) {
                registrations.push(Registration {
                    subtree: oid_parts(subtree),
                    file,
                    handler: Handler::Normal,
                });
            }
        }
        if let Some(file) = index(IPADDR) {
            // Column by column, not as one subtree. net-snmp's own IP module owns `ipAddrTable`,
            // and a whole-table `pass` loses the duplicate registration: the agent then quietly
            // answers from the VM's addresses instead of the fixture's, which is a device that
            // looks healthy and is not the device under test (GH #663).
            for column in [
                ip_mib::ip_addr_entry::IP_AD_ENT_ADDR,
                ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
                ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
            ] {
                registrations.push(Registration {
                    subtree: oid_parts(column),
                    file,
                    handler: Handler::Normal,
                });
            }
        }
        if let Some(file) = index(EMPTY) {
            for subtree in &self.suppresses {
                registrations.push(Registration {
                    subtree: oid_parts(subtree),
                    file,
                    handler: Handler::Normal,
                });
            }
        }
        if let Some(file) = index(ARP) {
            // Registered column by column rather than as one subtree, because the two misbehaving
            // handlers answer per column and a whole-table registration would let one column's
            // answer stand in for another's.
            for column in [
                arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
                arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
                arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS,
                arp::entry::IP_NET_TO_MEDIA_TYPE,
            ] {
                registrations.push(Registration {
                    subtree: oid_parts(column),
                    file,
                    handler: self.arp_handler,
                });
            }
        }
        registrations
    }

    /// Registrations for the LLDP subtree alone, against a single file.
    ///
    /// For serving one of the swappable variants: the deployed device does this by copying a
    /// variant over `-lldp-active.txt`, and a test does it by handing the variant's rows to an
    /// agent that serves only that subtree.
    pub fn registrations_for_lldp_only(&self) -> Vec<Registration> {
        vec![Registration {
            subtree: oid_parts(lldp_oids::LLDP_MIB),
            file: 0,
            handler: Handler::Normal,
        }]
    }

    /// This device answering SNMP, for driving the real collection path against.
    ///
    /// Whether it offers getbulk follows from its credential rather than being set per device:
    /// SNMPv1 has no getbulk, so the v1 agent forces every column through getnext.
    pub fn agent(&self) -> SimAgent {
        let agent = SimAgent::new(&self.data_files(), self.registrations());
        match self.credential {
            CredentialType::SnmpV1 { .. } => agent.without_getbulk(),
            _ => agent,
        }
    }

    /// The context back end answering SNMP, for the one device that has one.
    pub fn context_agent(&self) -> Option<SimAgent> {
        let files = self.context_files();
        if files.is_empty() {
            return None;
        }
        Some(SimAgent::new(&files, self.context_registrations()))
    }

    /// `ifNumber.0` as this device publishes it, or `None` where it serves no ifTable.
    pub fn declared_if_number(&self) -> Option<i32> {
        self.tables
            .if_table
            .as_ref()
            .map(tables::IfTable::declared_count)
    }
}

/// Every device in the lab, in address order.
pub fn lab() -> Vec<SimDevice> {
    devices::all()
}

/// One device by name, for a test that wants to name what it is driving.
pub fn device(name: &str) -> SimDevice {
    lab()
        .into_iter()
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("no simulated device named {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lab is the lab: 22 devices on 192.168.7.230-251, each on its own address.
    ///
    /// A property rather than a list — it fails when two devices collide or one goes missing,
    /// and not when a device is legitimately renamed.
    #[test]
    fn every_device_has_its_own_address() {
        let lab = lab();
        assert_eq!(lab.len(), 22);

        let mut addresses: Vec<Ipv4Addr> = lab.iter().map(|d| d.ip).collect();
        addresses.sort();
        addresses.dedup();
        assert_eq!(addresses.len(), 22, "two devices share an address");

        let mut names: Vec<&str> = lab.iter().map(|d| d.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 22, "two devices share a name");
    }

    /// Every file a device serves is served by a registration, and every registration names a
    /// file the device wrote. This was four `grep`s run at deploy time; here it cannot be false,
    /// and the test says so rather than the deploy script discovering it.
    #[test]
    fn every_served_file_has_a_registration_and_vice_versa() {
        for device in lab() {
            for (files, registrations) in [
                (device.data_files(), device.registrations()),
                (device.context_files(), device.context_registrations()),
            ] {
                for registration in &registrations {
                    assert!(
                        registration.file < files.len(),
                        "{}: a registration names no file",
                        device.name
                    );
                }
                for (index, file) in files.iter().enumerate() {
                    assert!(
                        registrations.iter().any(|r| r.file == index),
                        "{}: {} is written but nothing serves it",
                        device.name,
                        file.name
                    );
                }
            }
        }
    }

    /// Every device serving an ifTable registers `ifNumber`. Otherwise the scalar sits in the data
    /// file unserved while `mibII/interfaces` answers from the VM's own kernel state, and the
    /// device reports a count contradiction on every scan for a fault it does not have.
    #[test]
    fn a_device_serving_an_if_table_registers_its_own_count() {
        let if_number = oid_parts(if_mib::IF_NUMBER_OBJECT);
        for device in lab() {
            if device.tables.if_table.is_none() {
                continue;
            }
            assert!(
                device
                    .registrations()
                    .iter()
                    .any(|r| r.subtree == if_number),
                "{} serves an ifTable but does not register ifNumber",
                device.name
            );
        }
    }

    /// Every data file is in strictly ascending OID order, except the one device that is unsorted
    /// on purpose. A mis-sort silently truncates a walk, and this is the property that used to be
    /// a review instruction.
    #[test]
    fn every_file_ascends_except_the_one_that_must_not() {
        for device in lab() {
            for file in device
                .data_files()
                .iter()
                .chain(device.variant_files().iter())
                .chain(device.context_files().iter())
            {
                let rows = file.rows();
                let ascending = rows.windows(2).all(|pair| pair[0].oid < pair[1].oid);
                match file.ordering {
                    Ordering::Ascending => {
                        assert!(ascending, "{} is not in ascending OID order", file.name)
                    }
                    Ordering::Positional => assert!(
                        !ascending,
                        "{} is served positionally but happens to be sorted, so it cannot \
                         reproduce the defect it exists for",
                        file.name
                    ),
                }
            }
        }
    }
}
