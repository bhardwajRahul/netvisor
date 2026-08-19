use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::hosts::r#impl::name::{HostName, HostNameSource};
use crate::server::hosts::r#impl::virtualization::HostVirtualization;
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::api::deserialize_empty_string_as_none;
use crate::server::shared::types::entities::EntitySource;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Base data for a Host entity (stored in database).
/// Child entities (ip_addresses, ports, services) are stored in their own tables
/// and queried by `host_id`. They are NOT stored on the host.
#[derive(Debug, Clone, Serialize, Validate, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct HostBase {
    /// Human-facing name for the host.
    ///
    /// Always the best name known. Which rung of the naming ladder it came from is recorded
    /// separately in `name_source`, and only `apply_name` may write either.
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    /// Which rung of the naming ladder produced `name`: an address, a detected service, a
    /// hostname, an integration's device name, or a person typing it. Read-only over the API.
    #[serde(default)]
    #[schema(read_only)]
    pub name_source: HostNameSource,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// Hostname as resolved or reported by the host.
    #[schema(required)]
    pub hostname: Option<String>,
    /// Free-text notes about the host.
    #[validate(length(min = 0, max = 500))]
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    #[schema(required)]
    pub description: Option<String>,
    /// How this host came to be known — discovered, imported, or created by hand.
    #[schema(read_only)]
    pub source: EntitySource,
    /// How the host is virtualized, when it is a VM or container guest.
    #[schema(required)]
    pub virtualization_metadata: Option<HostVirtualization>,
    /// The service doing the virtualizing — the hypervisor this VM runs on.
    ///
    /// Its own column with a foreign key rather than a field inside
    /// [`HostVirtualization`]: a reference that no longer resolves now fails the write instead of
    /// surviving as a value nothing matches, and `ON DELETE SET NULL` clears it when the
    /// hypervisor service goes away (GH #650).
    #[schema(required)]
    pub virtualization_service_id: Option<Uuid>,
    /// Whether the host is hidden from topology views.
    pub hidden: bool,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    // SNMP System MIB fields
    /// SNMP sysDescr.0 - full system description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_descr: Option<String>,
    /// SNMP sysObjectID.0 - vendor OID for device identification
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_object_id: Option<String>,
    /// SNMP sysLocation.0 - physical location
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_location: Option<String>,
    /// SNMP sysContact.0 - admin contact info
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_contact: Option<String>,
    /// URL for device management interface (manual or discovered)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(format = "uri")]
    pub management_url: Option<String>,
    /// LLDP lldpLocChassisId - globally unique device identifier for deduplication
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis_id: Option<String>,
    /// SNMP sysName.0 - administratively-assigned hostname
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sys_name: Option<String>,
    /// ENTITY-MIB entPhysicalMfgName - hardware manufacturer
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    /// ENTITY-MIB entPhysicalModelName - hardware model
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ENTITY-MIB entPhysicalSerialNum - hardware serial number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    /// Credential assignments for this host (hydrated from junction table).
    #[serde(default)]
    #[schema(required)]
    pub credential_assignments: Vec<CredentialAssignment>,
}

impl Default for HostBase {
    fn default() -> Self {
        Self {
            name: String::new(),
            name_source: HostNameSource::default(),
            network_id: Uuid::nil(),
            hostname: None,
            description: None,
            source: EntitySource::Unknown,
            virtualization_metadata: None,
            virtualization_service_id: None,
            hidden: false,
            tags: Vec::new(),
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments: Vec::new(),
        }
    }
}

impl HostBase {
    /// Assign `name`/`name_source` if `candidate` is at least as authoritative as what is stored.
    /// Returns whether anything changed.
    ///
    /// **This is the only place either field is written.** The ordering lives entirely in
    /// [`HostNameSource`]'s derived `Ord`, so there is no per-call-site precedence to keep in
    /// sync — a caller only has to say where its name came from.
    ///
    /// Equal rank wins, which is what makes a re-sync idempotent in the useful direction: a
    /// controller rename propagates on the next discovery, while a lower rung (reverse DNS, a
    /// detected service, an IP) never displaces it, and nothing displaces
    /// [`HostNameSource::Manual`].
    pub fn apply_name(&mut self, candidate: HostName) -> bool {
        if candidate.source() < self.name_source {
            return false;
        }
        let (value, source) = candidate.into_parts();
        if self.name == value && self.name_source == source {
            return false;
        }
        self.name = value;
        self.name_source = source;
        true
    }

    /// Lower the recorded provenance to `ceiling` if it claims more, keeping the name itself.
    /// Returns whether anything changed.
    ///
    /// The server applies this to daemon payloads. It is the only other writer of `name_source`,
    /// and it can only ever move it down the ladder.
    pub fn clamp_name_source(&mut self, ceiling: HostNameSource) -> bool {
        if self.name_source <= ceiling {
            return false;
        }
        self.name_source = ceiling;
        true
    }

    /// The stored name as a candidate, for merging this host's name into another one.
    /// `None` when the host has no name.
    pub(in crate::server::hosts) fn name_candidate(&self) -> Option<HostName> {
        HostName::from_parts(self.name.clone(), self.name_source)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::host)]
pub struct Host {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// SCD2: when this row version became live. Equal to `created_at` for
    /// rows that have never ridden a snapshot; advanced to the snapshot's
    /// `taken_at` for live rows after a network snapshot fires.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    /// SCD2: when this row was closed by a snapshot. NULL = currently live.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    /// Lineage pointer on closed historical rows back to the live row whose
    /// state they capture. NULL on live rows.
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    /// Last successful natural-key match by daemon discovery against this
    /// live row. Refreshed every scan, regardless of field changes.
    #[serde(default)]
    #[schema(read_only)]
    pub last_seen_at: DateTime<Utc>,
    /// Discovery (historical row) that last touched this entity. Populated
    /// post-terminal by the per-entity-service subscriber on
    /// `DiscoveryProcessed`. NULL until the first successful discovery
    /// session terminates after this row was created.
    #[serde(default)]
    #[schema(read_only)]
    pub last_discovery_id: Option<Uuid>,
    /// Discovery (historical row) that first observed this entity. Set once
    /// (post-terminal); immutable thereafter via the `IS NULL` guard in
    /// `update_discovery_fks`.
    #[serde(default)]
    #[schema(read_only)]
    pub first_discovery_id: Option<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: HostBase,
}

impl Hash for Host {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Host {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.base.name, self.id)
    }
}

impl Host {
    pub fn new(base: HostBase) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base,
        }
    }
}

impl ChangeTriggersTopologyStaleness<Host> for Host {
    fn triggers_staleness(&self, other: Option<Host>) -> bool {
        if let Some(other_host) = other {
            self.base.hostname != other_host.base.hostname
                || self.base.virtualization_metadata != other_host.base.virtualization_metadata
                || self.base.virtualization_service_id != other_host.base.virtualization_service_id
                || self.base.hidden != other_host.base.hidden
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `apply_name`'s return value is what `upsert_host` uses to decide whether the host actually
    /// changed, and an Updated event (and a topology rebuild) rides on that. A re-sync that
    /// reports the same name must be silent, not a no-op write that still looks like a change.
    #[test]
    fn reapplying_an_unchanged_name_reports_no_change() {
        let mut base = HostBase::default();
        assert!(base.apply_name(HostName::from_integration("Core Switch").unwrap()));
        assert!(!base.apply_name(HostName::from_integration("Core Switch").unwrap()));
        assert!(base.apply_name(HostName::from_integration("Core Switch 2").unwrap()));
    }

    /// The same value arriving from a *better* source is still a change worth recording: the name
    /// reads the same, but the host is now protected from the rungs in between.
    #[test]
    fn the_same_name_from_a_higher_rung_is_recorded() {
        let mut base = HostBase::default();
        base.apply_name(HostName::from_hostname("switch.lan").unwrap());
        assert!(base.apply_name(HostName::from_integration("switch.lan").unwrap()));
        assert_eq!(base.name_source, HostNameSource::Integration);
    }
}
