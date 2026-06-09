use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use strum_macros::EnumDiscriminants;
use uuid::Uuid;

use crate::server::shared::events::types::EventLogLevel;

/// Compact summary of a host for digest rendering. Keeps the payload small
/// enough to round-trip through the event bus without serializing entire
/// entity rows.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostSummary {
    pub id: Uuid,
    /// Hostname when discovery resolved one (SNMP `sysName.0`, reverse DNS,
    /// or user-assigned); falls back to the synthetic `name`.
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    /// e.g. `"443/tcp"` or `"Http (ID: ...)"`. Source: `Port`'s Display impl.
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpAddressSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BindingSummary {
    pub id: Uuid,
    pub host_id: Uuid,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubnetSummary {
    pub id: Uuid,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VlanSummary {
    pub id: Uuid,
    pub vlan_number: u16,
    pub name: String,
}

/// Per-host child-entity changes detected over the session window. Built only
/// for hosts that exist before the session and were refreshed during it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostChildChanges {
    pub host: HostSummary,
    pub ports_added: Vec<PortSummary>,
    pub ports_removed: Vec<PortSummary>,
    pub services_added: Vec<ServiceSummary>,
    pub services_removed: Vec<ServiceSummary>,
    pub ip_addresses_added: Vec<IpAddressSummary>,
    pub ip_addresses_removed: Vec<IpAddressSummary>,
    pub interfaces_added: Vec<InterfaceSummary>,
    pub interfaces_removed: Vec<InterfaceSummary>,
    pub bindings_added: Vec<BindingSummary>,
    pub bindings_removed: Vec<BindingSummary>,
}

impl HostChildChanges {
    pub fn is_empty(&self) -> bool {
        self.ports_added.is_empty()
            && self.ports_removed.is_empty()
            && self.services_added.is_empty()
            && self.services_removed.is_empty()
            && self.ip_addresses_added.is_empty()
            && self.ip_addresses_removed.is_empty()
            && self.interfaces_added.is_empty()
            && self.interfaces_removed.is_empty()
            && self.bindings_added.is_empty()
            && self.bindings_removed.is_empty()
    }
}

/// Lightweight recipient identity. We carry just what the email subscriber
/// needs to dispatch — id (for the per-user discovery_digest gate), email,
/// and the in-memory `EmailSettings` snapshot so the gate is decided without
/// re-fetching the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DigestRecipient {
    pub user_id: Uuid,
    #[serde(with = "email_address_serde")]
    pub email: EmailAddress,
    pub discovery_digest_enabled: bool,
}

mod email_address_serde {
    use email_address::EmailAddress;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};
    use std::str::FromStr;

    pub fn serialize<S: Serializer>(addr: &EmailAddress, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(addr.as_ref())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<EmailAddress, D::Error> {
        let s = String::deserialize(de)?;
        EmailAddress::from_str(&s).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveryDigestPayload {
    pub session_id: Uuid,
    pub network_id: Uuid,
    pub network_name: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,

    pub subnets_scanned: Vec<SubnetSummary>,
    pub hosts_added: Vec<HostSummary>,
    pub hosts_vanished: Vec<HostSummary>,
    pub hosts_with_child_changes: Vec<HostChildChanges>,
    pub vlans_added: Vec<VlanSummary>,
    pub vlans_removed: Vec<VlanSummary>,

    pub recipients: Vec<DigestRecipient>,
}

impl DiscoveryDigestPayload {
    /// Whether the digest has any user-visible deltas worth emailing about.
    /// Subnets-scanned alone is metadata, not a change — empty digests are
    /// suppressed regardless of recipient settings.
    pub fn has_changes(&self) -> bool {
        !self.hosts_added.is_empty()
            || !self.hosts_vanished.is_empty()
            || !self.hosts_with_child_changes.is_empty()
            || !self.vlans_added.is_empty()
            || !self.vlans_removed.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::Display, EnumDiscriminants)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(Hash, EnumIter, strum::Display, Serialize, Deserialize))]
pub enum DiscoveryDigestOperation {
    Computed {
        payload: Box<DiscoveryDigestPayload>,
    },
}

/// Org scope mirrors the existing `OrgScope` used by Billing/Onboarding —
/// digests are per-org-keyed for routing; the network_id rides on the payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DiscoveryDigestScope {
    pub organization_id: Uuid,
    pub network_id: Uuid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DiscoveryDigestFlags {
    pub suppress_logs: bool,
}

impl crate::server::shared::events::traits::Operation for DiscoveryDigestOperation {
    type Scope = DiscoveryDigestScope;
    type Flags = DiscoveryDigestFlags;
    type Filter = crate::server::shared::events::traits::EventFilter<DiscoveryDigestOperation>;

    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }
}
