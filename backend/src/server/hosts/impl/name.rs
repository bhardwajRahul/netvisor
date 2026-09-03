//! Host naming: the name of a host, inseparable from the evidence that produced it.
//!
//! Before this module the question "did a person type this name, or did we derive it?" was
//! answered by inspecting the string — `name.parse::<IpAddr>().is_ok()`. That could recognise
//! exactly one derived shape, so a name derived from a detected service was indistinguishable
//! from a hand-typed one and froze forever, and a name supplied by a controller had nowhere to
//! sit in the ordering at all (GH #680).
//!
//! The first fix carried the rung in a second `HostBase` field beside `name`. Two fields that
//! must move together is a standing invitation to move only one: three construction sites
//! assigned `name` directly and let the rung default, and one of them shipped a host labelled
//! with an address it no longer held. The second fix made the rung the enum variant, which fixed
//! that but gave names a private eight-rung ladder no other field could use.
//!
//! Now the ladder is [`AttributeSource`], shared with every other provenanced value, and a name is
//! an [`Attributed`] like any of them. What remains here is the part that is genuinely about names:
//! the typed `Ip` payload, and the source-specific constructors that keep a value and its rung from
//! being paired wrongly.

use std::borrow::Cow;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
use utoipa::openapi::{RefOr, Schema};

use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};

/// A host's name, keeping the typed address for the one rung derived from one.
///
/// `Ip` is a variant rather than a plain string so a name derived from an address cannot hold a
/// non-address. Splitting the value from its source loses the coupling *between* them — which is
/// what the source-specific constructors on [`HostName`] below exist to restore.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HostNameValue {
    /// The host's own address, rendered.
    Ip(IpAddr),
    /// Any other name: a hostname, a service name, a label from a controller, a typed one.
    Text(String),
}

impl HostNameValue {
    /// The name itself. Borrowed for [`Self::Text`]; [`Self::Ip`] formats its address on demand.
    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            Self::Ip(ip) => Cow::Owned(ip.to_string()),
            Self::Text(value) => Cow::Borrowed(value),
        }
    }
}

impl std::fmt::Display for HostNameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_str())
    }
}

impl crate::server::shared::attribution::AttributeValue for HostNameValue {
    const VALUE_KEY: &'static str = "name";
    const SOURCE_KEY: &'static str = "name_source";
    const SCHEMA_NAME: &'static str = "HostName";
    /// `hosts.name` is `TEXT NOT NULL` and every consumer treats it as a bare string; an unnamed
    /// host has an empty one rather than none.
    const VALUE_REQUIRED: bool = true;
    /// A host can be renamed, and a rename at the same rung from the same source has to propagate
    /// — that is how a controller rename reaches Scanopy on the next sync.
    const REFRESHABLE: bool = true;

    fn is_blank(&self) -> bool {
        self.as_str().trim().is_empty()
    }
}

impl utoipa::PartialSchema for HostNameValue {
    fn schema() -> RefOr<Schema> {
        RefOr::T(Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::new(Type::String))
                .description(Some("Human-facing name for the host."))
                .build(),
        ))
    }
}

/// A host's name together with the source that produced it.
pub type HostName = Attributed<HostNameValue>;

/// Source-specific constructors.
///
/// These exist so `HostName::new(HostNameValue::Text("…"), AttributeSource::OwnAddress)` — which
/// the generic carrier makes spellable — has no path anyone would reach for. Weaker than the
/// type-level guarantee the fused enum gave, stronger than a convention.
pub trait HostNameSources {
    /// The absence of a name. Not a rung: an unnamed host is a host whose name we have no source
    /// for, which is exactly what `Unspecified` means.
    fn unnamed() -> Self;
    /// A name from a daemon predating provenance: real, but with no source we can name.
    fn unattributed(name: String) -> Self;
    /// The host's own address, standing in for a name it does not have.
    fn from_ip(ip: IpAddr) -> Self;
    /// Reverse DNS: a known speaker that is not the subject.
    fn from_hostname(hostname: String) -> Self;
    /// A hostname a controller observed for a device it manages — a DHCP client's advertised
    /// name, typically. The same rung as reverse DNS, and for the same reason: somebody else
    /// telling us about the subject. Distinct from [`Self::from_controller`], which is a name a
    /// person deliberately assigned.
    fn from_controller_hostname(hostname: String, probe: ClientProbe) -> Self;
    /// Named after the best non-generic service detected on it.
    fn from_service(service: String) -> Self;
    /// An mDNS instance label, typed by a person during device setup.
    fn from_dns_sd(label: String) -> Self;
    /// A name a person assigned in a controller, read back over that controller's API.
    fn from_controller(name: String, probe: ClientProbe) -> Self;
    /// `sysName`, read from the device itself.
    fn from_sys_name(name: String) -> Self;
    /// A name a person typed into Scanopy. Only the server can assert this.
    fn manual(name: String) -> Self;
}

impl HostNameSources for HostName {
    fn unnamed() -> Self {
        Self::new(
            HostNameValue::Text(String::new()),
            AttributeSource::Unspecified,
        )
    }

    fn unattributed(name: String) -> Self {
        Self::new(HostNameValue::Text(name), AttributeSource::Unspecified)
    }

    fn from_ip(ip: IpAddr) -> Self {
        Self::new(HostNameValue::Ip(ip), AttributeSource::OwnAddress)
    }

    fn from_hostname(hostname: String) -> Self {
        Self::new(HostNameValue::Text(hostname), AttributeSource::ReverseDns)
    }

    fn from_controller_hostname(hostname: String, probe: ClientProbe) -> Self {
        Self::new(HostNameValue::Text(hostname), AttributeSource::Probe(probe))
    }

    fn from_service(service: String) -> Self {
        Self::new(HostNameValue::Text(service), AttributeSource::ServiceMatch)
    }

    fn from_dns_sd(label: String) -> Self {
        Self::new(
            HostNameValue::Text(label),
            AttributeSource::DnsSdInstanceName,
        )
    }

    fn from_controller(name: String, probe: ClientProbe) -> Self {
        Self::new(HostNameValue::Text(name), AttributeSource::Authored(probe))
    }

    fn from_sys_name(name: String) -> Self {
        Self::new(
            HostNameValue::Text(name),
            AttributeSource::Probe(ClientProbe::Snmp),
        )
    }

    fn manual(name: String) -> Self {
        Self::new(HostNameValue::Text(name), AttributeSource::Manual)
    }
}

/// Rebuild from a stored or received `(name, name_source)` pair.
///
/// A blank value collapses to [`HostNameSources::unnamed`] whatever rung was claimed for it — a
/// rung with no name means nothing. An `OwnAddress` rung whose value is not actually an address
/// degrades to `Unspecified` rather than asserting something false: that is a live case, not a
/// hypothetical, because the backfill classifies by regex and
/// `^[0-9]{1,3}(\.[0-9]{1,3}){3}$` happily matches `999.999.999.999`. Such a row self-heals the
/// next time it is written.
pub fn host_name_from_parts(value: String, source: AttributeSource) -> HostName {
    if value.trim().is_empty() {
        return HostName::unnamed();
    }
    if source == AttributeSource::OwnAddress {
        return match value.parse::<IpAddr>() {
            Ok(ip) => HostName::from_ip(ip),
            Err(_) => HostName::new(HostNameValue::Text(value), AttributeSource::Unspecified),
        };
    }
    HostName::new(HostNameValue::Text(value), source)
}

/// The flattened read path for `HostBase.name`.
///
/// `optional` with a fallback rather than `required`, because a payload with no name at all is a
/// real case — a daemon reporting a host it has only an address for — and it has always read as
/// unnamed rather than as an error.
pub fn deserialize_host_name<'de, D>(deserializer: D) -> Result<HostName, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(
        crate::server::shared::attribution::optional::<D, HostNameValue>(deserializer)?
            .unwrap_or_else(HostName::unnamed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A daemon predating provenance sends a bare name. It is real but unattributable, so it enters
    /// at the bottom and cannot displace anything whose source we know.
    fn unattributed(value: &str) -> HostName {
        HostName::new(
            HostNameValue::Text(value.to_string()),
            AttributeSource::Unspecified,
        )
    }

    /// A rung with no name means nothing, whatever it claims — so a blank value collapses to
    /// unnamed rather than occupying the rung it was labelled with.
    #[test]
    fn a_rung_without_a_name_collapses_to_unnamed() {
        let rebuilt = host_name_from_parts(
            String::new(),
            AttributeSource::Authored(ClientProbe::UnifiController),
        );

        assert!(rebuilt.is_blank());
        assert_eq!(rebuilt.source(), AttributeSource::Unspecified);
    }

    /// The backfill classifies by regex, and `^[0-9]{1,3}(\.[0-9]{1,3}){3}$` matches
    /// `999.999.999.999`. A row like that must degrade rather than claim to hold an address, and it
    /// self-heals the next time anything writes it.
    #[test]
    fn an_address_rung_whose_value_is_not_an_address_degrades_instead_of_lying() {
        let bogus =
            host_name_from_parts("999.999.999.999".to_string(), AttributeSource::OwnAddress);
        assert_eq!(bogus.source(), AttributeSource::Unspecified);
        assert_eq!(bogus.value().as_str(), "999.999.999.999");

        let real = host_name_from_parts("192.168.1.20".to_string(), AttributeSource::OwnAddress);
        assert_eq!(real.source(), AttributeSource::OwnAddress);
        assert!(matches!(real.value(), HostNameValue::Ip(_)));
    }

    /// Clamping lowers an overreaching claim and keeps the value, which is what lets the server
    /// refuse a daemon's `Manual` without throwing the name away.
    #[test]
    fn clamping_lowers_the_rung_and_keeps_the_value() {
        let clamped =
            HostName::manual("typed".to_string()).clamped_to(AttributeSource::Unspecified);
        assert_eq!(clamped.source(), AttributeSource::Unspecified);
        assert_eq!(clamped.value().as_str(), "typed");
    }

    /// A ceiling never raises a rung: clamping something already below it is a no-op. This is why
    /// the server's guard names `Manual` specifically rather than clamping by rank — a rank
    /// ceiling at the floor would strip the provenance off every inbound name.
    #[test]
    fn clamping_leaves_a_rung_below_the_ceiling_alone() {
        let untouched = HostName::from_hostname("switch.lan".to_string())
            .clamped_to(AttributeSource::Authored(ClientProbe::UnifiController));
        assert_eq!(untouched.source(), AttributeSource::ReverseDns);
    }

    /// The rungs a name can occupy, in the order the naming ladder used to hard-code. Asserted as
    /// an ordering rather than per-rung values: what matters is that an operator's deliberate name
    /// still outranks anything a scan derives, which is the property the shipped ladder had.
    #[test]
    fn the_naming_ladder_survives_the_generalisation() {
        let ip = HostName::from_ip("192.168.1.20".parse().unwrap());
        let service = HostName::from_service("SSH".to_string());
        let reverse_dns = HostName::from_hostname("nas.lan".to_string());
        let sys_name = HostName::from_sys_name("core-sw-1".to_string());
        let dns_sd = HostName::from_dns_sd("Living Room TV".to_string());
        let controller =
            HostName::from_controller("Core Switch".to_string(), ClientProbe::UnifiController);
        let manual = HostName::manual("Rack 3 Top".to_string());

        // An address and a detected service are both derivations, and neither beats a real name.
        assert_eq!(ip.rank(), service.rank());
        assert!(ip.rank() < reverse_dns.rank());
        // Reverse DNS and `sysName` used to share one rung; they separate correctly here, because
        // we asked the device for one of them and a third party for the other.
        assert!(reverse_dns.rank() < sys_name.rank());
        // Both of these are names a person chose, so both outrank everything a machine emitted.
        assert!(sys_name.rank() < dns_sd.rank());
        assert!(dns_sd.rank() < controller.rank());
        assert!(controller.rank() < manual.rank());
    }

    /// An unattributable name is displaced by anything that knows where it came from, and displaces
    /// nothing itself.
    #[test]
    fn an_unattributed_name_yields_to_anything_attributed() {
        assert!(
            unattributed("nas.lan").rank() < HostName::from_ip("10.0.0.2".parse().unwrap()).rank()
        );
    }
}
