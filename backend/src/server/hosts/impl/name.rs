//! Host naming: which rung of the ladder produced a host's display name, and the only
//! value that may be assigned to it.
//!
//! Before this module the question "did a person type this name, or did we derive it?" was
//! answered by inspecting the string — `name.parse::<IpAddr>().is_ok()`. That could recognise
//! exactly one derived shape, so a name derived from a detected service was indistinguishable
//! from a hand-typed one and froze forever, and a name supplied by a controller had nowhere to
//! sit in the ordering at all (GH #680).

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use strum::{Display, EnumString, VariantNames};
use utoipa::ToSchema;

/// Which rung of the naming ladder produced a host's display name, weakest first.
///
/// Declaration order is the precedence order — `Ord` is derived from it, and that derive is the
/// whole enforcement mechanism: a rung inserted at its rank propagates to every comparison, so
/// there is no per-call-site precedence to keep in sync.
///
/// Persisted as bare text (`Manual`, `Integration`, …) via `Display`/`FromStr`.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    VariantNames,
    ToSchema,
)]
pub enum HostNameSource {
    /// Provenance unknown: a row predating this column, or a payload from a daemon that predates
    /// it. Ranked lowest so such a name never displaces one whose provenance we do know.
    #[default]
    Unspecified,
    /// The host's own IP address, used because nothing better was known.
    Ip,
    /// The name of the best non-generic service detected on the host.
    DetectedService,
    /// Reverse DNS, a hostname the host reported, or SNMP sysName.
    Hostname,
    /// A name a person assigned in a controller (UniFi, HPE Instant On, …) and that the
    /// integration read back out. Deliberate, and stable across DHCP lease changes.
    Integration,
    /// A name a person typed into Scanopy. Nothing outranks it, and discovery cannot reach it:
    /// [`HostName::manual`] is private to the hosts module and the server clamps the rank a
    /// daemon payload may claim.
    Manual,
}

/// A candidate host name, inseparable from the evidence that produced it.
///
/// The fields are private and there is no `From<String>`: the only way to obtain one is to call
/// a constructor that names the evidence. That is what stops a caller from supplying a name
/// without declaring where it came from, which is how the old code lost the distinction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostName {
    value: String,
    source: HostNameSource,
}

impl HostName {
    /// A name a person assigned in a controller that an integration manages.
    ///
    /// `None` when the controller holds no name (or only whitespace) — an absent name must not
    /// displace a worse-but-present one.
    pub fn from_integration(value: impl Into<String>) -> Option<Self> {
        Self::non_blank(value, HostNameSource::Integration)
    }

    /// Reverse DNS, a hostname the host reported, or SNMP sysName.
    pub fn from_hostname(value: impl Into<String>) -> Option<Self> {
        Self::non_blank(value, HostNameSource::Hostname)
    }

    /// The best non-generic service detected on the host.
    pub fn from_service(value: impl Into<String>) -> Option<Self> {
        Self::non_blank(value, HostNameSource::DetectedService)
    }

    /// The host's IP address — the bottom of the ladder, and never blank.
    pub fn from_ip(ip: IpAddr) -> Self {
        Self {
            value: ip.to_string(),
            source: HostNameSource::Ip,
        }
    }

    /// A name a person typed into Scanopy.
    ///
    /// `pub(in crate::server::hosts)` on purpose: daemon and integration code cannot call it, so
    /// no discovery path can mint a name that outranks a user's.
    pub(in crate::server::hosts) fn manual(value: impl Into<String>) -> Option<Self> {
        Self::non_blank(value, HostNameSource::Manual)
    }

    /// Rebuild a candidate from a stored or received `(name, name_source)` pair, for the server's
    /// merge of an incoming discovery payload against what is already stored.
    pub(in crate::server::hosts) fn from_parts(
        value: impl Into<String>,
        source: HostNameSource,
    ) -> Option<Self> {
        Self::non_blank(value, source)
    }

    fn non_blank(value: impl Into<String>, source: HostNameSource) -> Option<Self> {
        let value = value.into();
        (!value.trim().is_empty()).then_some(Self { value, source })
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn source(&self) -> HostNameSource {
        self.source
    }

    pub(in crate::server::hosts) fn into_parts(self) -> (String, HostNameSource) {
        (self.value, self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_names_produce_no_candidate() {
        assert!(HostName::from_integration("   ").is_none());
        assert!(HostName::from_hostname("").is_none());
        assert!(HostName::from_integration("Core Switch").is_some());
    }
}
