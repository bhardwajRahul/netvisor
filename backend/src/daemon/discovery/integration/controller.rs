//! What a controller knows about a device or client it manages.
//!
//! UniFi and HPE Instant On both learn an administrator's deliberate name for every device they
//! adopt and every client they see — the name that is stable when the management IP is a DHCP
//! lease. Before this type each integration mapped that name into `sys_name` and left the host's
//! display `name` empty, so the topology was labelled with an IP (GH #680).
//!
//! This is the one place a controller integration expresses identity. [`ControllerIdentity`] has
//! no `Default` impl on purpose: a new integration cannot `..Default::default()` past the name
//! field, so "we forgot to supply the name" is a compile error rather than a blank label. And
//! because both entry points below route the name through
//! [`HostBase::apply_name`](crate::server::hosts::r#impl::base::HostBase::apply_name), no
//! integration ever writes precedence logic of its own.

use std::net::IpAddr;

use uuid::Uuid;

use crate::daemon::discovery::integration::IntegrationContext;
use crate::daemon::discovery::service::ops::HostData;
use crate::server::hosts::r#impl::{
    base::{Host, HostBase},
    name::HostName,
};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
use crate::server::shared::types::entities::EntitySource;
use crate::server::snmp::resolution::lldp::canonical_mac;
use crate::server::subnets::r#impl::base::Subnet;

/// The identity fields a controller reports for one device or client.
///
/// Every field is `Option` because controllers differ in what they hold, but every field must be
/// written at the construction site — that is what makes the omission visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerIdentity {
    /// The name a person assigned in the controller ("Core Switch", "Meeting Room AP").
    /// `None` only when the controller genuinely holds none.
    pub name: Option<String>,
    /// A hostname the controller observed rather than one a person chose — a DHCP client's
    /// advertised hostname, typically. Ranks below `name`.
    pub hostname: Option<String>,
    /// LLDP chassis ID, canonicalised the same way the SNMP daemon writes it.
    pub chassis_id: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub sys_descr: Option<String>,
}

impl ControllerIdentity {
    /// Mint a host for a device or client the controller reports but the sweep never scanned.
    ///
    /// The server deduplicates on IP and MAC, so this merges into the host another discovery
    /// path already found when there is one.
    pub fn into_host(self, network_id: Uuid) -> Host {
        let Self {
            name,
            hostname,
            chassis_id,
            manufacturer,
            model,
            serial_number,
            sys_descr,
        } = self.normalized();

        let mut host = Host::new(HostBase {
            network_id,
            source: EntitySource::Discovery,
            // The controller's name is also what the device advertises as LLDP sysName, and
            // neighbour resolution matches `interfaces.lldp_sys_name` against this column.
            sys_name: name.clone(),
            hostname: hostname.clone(),
            chassis_id,
            manufacturer,
            model,
            serial_number,
            sys_descr,
            ..Default::default()
        });

        Self::apply_names(&mut host.base, name, hostname);
        host
    }

    /// Fold this identity into the host currently being scanned.
    ///
    /// Every field except the name stays first-write-wins, so a prior SNMP pass in the same scan
    /// keeps its values — SNMP reads the device directly and is the better source when both are
    /// present. The name instead goes through the ladder, where a controller's name outranks
    /// anything the scan itself could derive and loses only to a name a person typed.
    pub fn enrich(&self, host_data: &mut HostData) {
        let Self {
            name,
            hostname,
            chassis_id,
            manufacturer,
            model,
            serial_number,
            sys_descr,
        } = self.clone().normalized();

        if let Some(hostname) = hostname {
            host_data.with_hostname_fallback(hostname);
        }
        if let Some(name) = name {
            host_data.with_sys_name(name.clone());
            if let Some(candidate) = HostName::from_integration(name) {
                host_data.apply_name(candidate);
            }
        }
        if let Some(chassis_id) = chassis_id {
            host_data.with_chassis_id(chassis_id);
        }
        if let Some(manufacturer) = manufacturer {
            host_data.with_manufacturer(manufacturer);
        }
        if let Some(model) = model {
            host_data.with_model(model);
        }
        if let Some(serial_number) = serial_number {
            host_data.with_serial_number(serial_number);
        }
        if let Some(sys_descr) = sys_descr {
            host_data.with_sys_descr(sys_descr);
        }
    }

    fn apply_names(base: &mut HostBase, name: Option<String>, hostname: Option<String>) {
        if let Some(candidate) = hostname.and_then(HostName::from_hostname) {
            base.apply_name(candidate);
        }
        if let Some(candidate) = name.and_then(HostName::from_integration) {
            base.apply_name(candidate);
        }
    }

    /// Controllers routinely return `""` for a field a user never filled in. An empty string is
    /// absence, not a value, and must not displace something real.
    fn normalized(self) -> Self {
        fn blank_to_none(v: Option<String>) -> Option<String> {
            v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }
        Self {
            name: blank_to_none(self.name),
            hostname: blank_to_none(self.hostname),
            chassis_id: blank_to_none(self.chassis_id),
            manufacturer: blank_to_none(self.manufacturer),
            model: blank_to_none(self.model),
            serial_number: blank_to_none(self.serial_number),
            sys_descr: blank_to_none(self.sys_descr),
        }
    }
}

/// A client a controller reports — a device on the network it can see but has not adopted.
///
/// Clients are mapped separately from adopted devices because that is all a controller knows
/// about them: an address, a MAC, and whatever name or DHCP hostname it has. They still become
/// hosts. The server deduplicates on IP and MAC, so a client the sweep also found is the same
/// host with a better name, and one it could not reach (a different VLAN, no ARP entry) becomes
/// a host the controller is the only witness for.
pub struct MappedClient {
    pub identity: ControllerIdentity,
    pub ip_address: IPAddress,
    pub ip: IpAddr,
}

impl MappedClient {
    /// Place a reported client on a known subnet, or skip it.
    ///
    /// `None` when the address is missing, unparseable, or outside every known subnet: host
    /// deduplication is IP-based, so a host created without a placeable address would mint a
    /// fresh duplicate on every scan instead of updating the one already there.
    pub fn new(
        identity: ControllerIdentity,
        ip: Option<&str>,
        mac: Option<&str>,
        network_id: Uuid,
        subnets: &[Subnet],
    ) -> Option<Self> {
        let ip: IpAddr = ip?.trim().parse().ok()?;
        let subnet = subnets.iter().find(|s| s.base.cidr.contains(&ip))?;
        let mac = mac.and_then(canonical_mac);

        Some(Self {
            identity,
            ip_address: IPAddress::new(IPAddressBase {
                network_id,
                host_id: Uuid::nil(), // server assigns
                subnet_id: subnet.id,
                ip_address: ip,
                mac_address: mac.as_deref().and_then(|m| m.parse().ok()),
                name: None,
                position: 0,
            }),
            ip,
        })
    }
}

/// Submit each reported client as a host, and report how many were created.
///
/// Shared rather than per-integration so that a controller integration only has to answer "what
/// does the controller call this thing, and where is it?" — everything downstream of that,
/// including the naming ladder, is decided here.
pub async fn create_client_hosts(
    ctx: &IntegrationContext<'_>,
    clients: Vec<MappedClient>,
) -> usize {
    let Ok(network_id) = ctx.ops.network_id().await else {
        return 0;
    };

    let mut created = 0usize;
    for client in clients {
        if ctx.cancel.is_cancelled() {
            break;
        }
        let MappedClient {
            identity,
            ip_address,
            ip: _,
        } = client;

        let result = ctx
            .ops
            .create_host(
                identity.into_host(network_id),
                vec![ip_address],
                vec![],
                vec![],
                vec![],
                vec![],
                // A controller sees a client's address and name, never its interfaces. Claiming
                // an authoritative empty ifTable here would tear down interfaces an SNMP walk of
                // the same host collected.
                false,
                InterfaceDataComplete::default(),
                ctx.cancel,
            )
            .await;

        match result {
            Ok(_) => created += 1,
            Err(e) => tracing::debug!(error = %e, "Failed to create controller-reported client"),
        }
    }
    created
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::name::HostNameSource;

    fn identity(name: Option<&str>, hostname: Option<&str>) -> ControllerIdentity {
        ControllerIdentity {
            name: name.map(str::to_string),
            hostname: hostname.map(str::to_string),
            chassis_id: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            sys_descr: None,
        }
    }

    #[test]
    fn controller_name_becomes_the_display_name() {
        let host = identity(Some("Core Switch"), None).into_host(Uuid::new_v4());
        assert_eq!(host.base.name, "Core Switch");
        assert_eq!(host.base.name_source, HostNameSource::Integration);
        // Still mirrored into sys_name, which is what LLDP neighbour resolution matches on.
        assert_eq!(host.base.sys_name.as_deref(), Some("Core Switch"));
    }

    #[test]
    fn a_controller_known_hostname_names_a_client_that_has_no_assigned_name() {
        let host = identity(None, Some("marys-laptop")).into_host(Uuid::new_v4());
        assert_eq!(host.base.name, "marys-laptop");
        assert_eq!(host.base.name_source, HostNameSource::Hostname);
    }

    #[test]
    fn an_assigned_name_outranks_a_hostname_for_the_same_device() {
        let host = identity(Some("Reception iPad"), Some("ipad-1a2b")).into_host(Uuid::new_v4());
        assert_eq!(host.base.name, "Reception iPad");
        assert_eq!(host.base.name_source, HostNameSource::Integration);
        assert_eq!(host.base.hostname.as_deref(), Some("ipad-1a2b"));
    }

    #[test]
    fn a_blank_controller_name_leaves_the_host_unnamed_rather_than_naming_it_empty() {
        let host = identity(Some("   "), None).into_host(Uuid::new_v4());
        assert_eq!(host.base.name, "");
        assert_eq!(host.base.name_source, HostNameSource::Unspecified);
        assert_eq!(host.base.sys_name, None);
    }
}
