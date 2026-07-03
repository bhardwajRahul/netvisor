//! Forward-compatibility contract for daemon-consumed server responses.
//!
//! The daemon and server build from this same repo, so a daemon deployed on a
//! customer network is frequently an *older build* talking to a *newer* cloud
//! server. Two production `discovery_failed` incidents came from exactly this:
//! an old daemon could not deserialize a `Loopback` subnet type, nor a
//! `Discovery` entity source after the server dropped its `metadata` field.
//!
//! The systematic guard, rather than hardening one enum at a time:
//!
//! 1. [`DaemonResponse`] is required by every `ApiClient` deserialize method, so
//!    a newly-added daemon-consumed type *won't compile* until it opts into the
//!    contract here.
//! 2. Each type's [`DaemonResponse::skewed`] builds a payload simulating a newer
//!    server. Structs **exhaustively destructure** their own fields (no `..`), so
//!    adding a field is a compile error until someone classifies it; enums build
//!    skew from their own serde representation (no JSON tag-guessing).
//! 3. Every registered type is collected via `inventory` (same mechanism as
//!    `ServiceDefinition` / `Subscriber`) and exercised by one auto-running test,
//!    so coverage is the mechanism's job, not the author's.
//!
//! A `skewed()` impl injects an unknown variant only where degradation is the
//! intended contract (the field/enum is hardened with `#[serde(other)]` or a
//! lenient `Option` deserializer). Where an unknown value should legitimately be
//! rejected — e.g. a `DiscoveryType` work item the daemon cannot execute — the
//! skew keeps a valid discriminant and only adds an unknown *field*.

use semver::Version;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::server::daemons::r#impl::api::{
    DaemonDiscoveryRequest, DaemonRegistrationResponse, ServerCapabilities,
};
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::hosts::r#impl::api::HostResponse;
use crate::server::hosts::r#impl::virtualization::HostVirtualization;
use crate::server::services::r#impl::virtualization::ServiceVirtualization;
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::entities::EntitySource;
use crate::server::shared::types::examples;
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::subnets::r#impl::virtualization::SubnetVirtualization;
use crate::server::vlans::handlers::{VlanDiscoveryResponse, VlanDiscoveryResponseItem};

/// Sentinel string standing in for an enum variant or field a *newer* server
/// might emit that this build doesn't know about. Defined once; impls reference
/// it via [`DaemonResponse::SENTINEL`] instead of hard-coding their own.
const SENTINEL: &str = "__variant_from_a_newer_server__";

/// Implemented by every type the daemon deserializes from a server response.
/// Required by `ApiClient::execute`/`get`/`post`, so the boundary is compile-
/// enforced: an unmarked type cannot be deserialized in the daemon.
pub trait DaemonResponse: Serialize + DeserializeOwned {
    /// Shared sentinel for skew payloads. Override is never needed.
    const SENTINEL: &'static str = SENTINEL;

    /// A representative payload mutated to look like it came from a newer
    /// server. Deserializing it must **succeed** (degrade), not error.
    fn skewed() -> Value;
}

/// Add an unknown top-level field, simulating a field a newer server added.
fn with_unknown_field(mut v: Value) -> Value {
    if let Value::Object(map) = &mut v {
        map.insert(SENTINEL.to_string(), json!(true));
    }
    v
}

// ---------------------------------------------------------------------------
// Composing impls — these let the real call-site types (e.g. `Vec<Subnet>`,
// `(Option<DaemonDiscoveryRequest>, bool)`, `ApiResponse<VlanDiscoveryResponse>`)
// resolve `DaemonResponse` from their element types.
// ---------------------------------------------------------------------------

impl DaemonResponse for bool {
    fn skewed() -> Value {
        json!(false)
    }
}

impl<T: DaemonResponse> DaemonResponse for Vec<T> {
    fn skewed() -> Value {
        json!([T::skewed()])
    }
}

impl<T: DaemonResponse> DaemonResponse for Option<T> {
    fn skewed() -> Value {
        T::skewed()
    }
}

impl<A: DaemonResponse, B: DaemonResponse> DaemonResponse for (A, B) {
    fn skewed() -> Value {
        json!([A::skewed(), B::skewed()])
    }
}

impl<T: DaemonResponse> DaemonResponse for ApiResponse<T> {
    fn skewed() -> Value {
        with_unknown_field(json!({
            "success": true,
            "data": T::skewed(),
            "meta": { "api_version": 1, "server_version": "0.0.0" },
        }))
    }
}

// ---------------------------------------------------------------------------
// Enum skew — each enum builds skew from its OWN serde representation. No JSON
// tag-guessing: a unit enum yields a bare string, an internally/adjacently
// tagged enum yields its tag set to the sentinel.
// ---------------------------------------------------------------------------

impl DaemonResponse for SubnetType {
    // Plain unit enum; absorbed by `#[serde(other)] Unknown`.
    fn skewed() -> Value {
        json!(Self::SENTINEL)
    }
}

impl DaemonResponse for EntitySource {
    // Internally tagged (`tag = "type"`); absorbed by `#[serde(other)] Unknown`.
    fn skewed() -> Value {
        json!({ "type": Self::SENTINEL })
    }
}

impl DaemonResponse for SubnetVirtualization {
    // Internally tagged, single variant; tolerated via the lenient `Option`
    // deserializer on the field that holds it (degrades to `None`).
    fn skewed() -> Value {
        json!({ "type": Self::SENTINEL })
    }
}

impl DaemonResponse for HostVirtualization {
    // Adjacently tagged; tolerated via the lenient `Option` field deserializer.
    fn skewed() -> Value {
        json!({ "type": Self::SENTINEL })
    }
}

impl DaemonResponse for ServiceVirtualization {
    // Adjacently tagged; tolerated via the lenient `Option` field deserializer.
    fn skewed() -> Value {
        json!({ "type": Self::SENTINEL })
    }
}

// ---------------------------------------------------------------------------
// Leaf response types. Each exhaustively destructures its own fields (no `..`)
// as a compile-time completeness guard, then skews the enum-bearing fields.
// ---------------------------------------------------------------------------

impl DaemonResponse for Subnet {
    fn skewed() -> Value {
        let instance = Subnet::default();
        // Compile guard: a new field breaks this until it is classified.
        let Subnet {
            id: _,
            created_at: _,
            updated_at: _,
            valid_from: _,
            valid_to: _,
            lineage_id: _,
            last_seen_at: _,
            last_discovery_id: _,
            first_discovery_id: _,
            base:
                SubnetBase {
                    cidr: _,
                    network_id: _,
                    name: _,
                    description: _,
                    subnet_type: _,
                    virtualization: _,
                    source: _,
                    tags: _,
                },
        } = &instance;

        let mut v = serde_json::to_value(&instance).expect("Subnet serializes");
        // `base` is `#[serde(flatten)]`, so these sit at the top level.
        v["subnet_type"] = SubnetType::skewed();
        v["source"] = EntitySource::skewed();
        v["virtualization"] = SubnetVirtualization::skewed();
        with_unknown_field(v)
    }
}

impl DaemonResponse for HostResponse {
    fn skewed() -> Value {
        let instance = examples::host_response();
        // Compile guard.
        let HostResponse {
            id: _,
            created_at: _,
            updated_at: _,
            name: _,
            network_id: _,
            hostname: _,
            description: _,
            source: _,
            virtualization: _,
            hidden: _,
            tags: _,
            sys_descr: _,
            sys_object_id: _,
            sys_location: _,
            sys_contact: _,
            management_url: _,
            chassis_id: _,
            credential_assignments: _,
            ip_addresses: _,
            ports: _,
            services: _,
            interfaces: _,
        } = &instance;

        let mut v = serde_json::to_value(&instance).expect("HostResponse serializes");
        v["source"] = EntitySource::skewed();
        v["virtualization"] = HostVirtualization::skewed();
        // The example carries one service; skew its tolerant enums too so the
        // child tree is exercised. `ServiceBase` is `#[serde(flatten)]`.
        if let Some(service) = v["services"].get_mut(0) {
            service["source"] = EntitySource::skewed();
            service["virtualization"] = ServiceVirtualization::skewed();
        }
        with_unknown_field(v)
    }
}

impl DaemonResponse for ServerCapabilities {
    fn skewed() -> Value {
        let instance = ServerCapabilities {
            server_version: Version::new(0, 0, 0),
            minimum_daemon_version: Version::new(0, 0, 0),
            deprecation_warnings: Vec::new(),
        };
        // Compile guard.
        let ServerCapabilities {
            server_version: _,
            minimum_daemon_version: _,
            deprecation_warnings: _,
        } = &instance;
        with_unknown_field(serde_json::to_value(&instance).expect("ServerCapabilities serializes"))
    }
}

impl DaemonResponse for DaemonRegistrationResponse {
    fn skewed() -> Value {
        let instance = DaemonRegistrationResponse {
            daemon: examples::daemon(),
            host_id: Uuid::nil(),
            server_capabilities: None,
        };
        // Compile guard.
        let DaemonRegistrationResponse {
            daemon: _,
            host_id: _,
            server_capabilities: _,
        } = &instance;
        with_unknown_field(
            serde_json::to_value(&instance).expect("DaemonRegistrationResponse serializes"),
        )
    }
}

impl DaemonResponse for DaemonDiscoveryRequest {
    fn skewed() -> Value {
        let instance = DaemonDiscoveryRequest {
            session_id: Uuid::nil(),
            discovery_type: DiscoveryType::default(),
            credential_mappings: Vec::new(),
            discovery_id: Uuid::nil(),
        };
        // Compile guard.
        let DaemonDiscoveryRequest {
            session_id: _,
            discovery_type: _,
            credential_mappings: _,
            discovery_id: _,
        } = &instance;
        // `discovery_type` is intentionally NOT skewed: an unknown discovery
        // kind is not actionable by the daemon and should be rejected, not
        // silently degraded. We only verify unknown-field tolerance here.
        with_unknown_field(
            serde_json::to_value(&instance).expect("DaemonDiscoveryRequest serializes"),
        )
    }
}

impl DaemonResponse for VlanDiscoveryResponse {
    fn skewed() -> Value {
        let instance = VlanDiscoveryResponse {
            vlans: vec![VlanDiscoveryResponseItem {
                vlan_number: 1,
                id: Uuid::nil(),
            }],
        };
        // Compile guard.
        let VlanDiscoveryResponse { vlans: _ } = &instance;
        with_unknown_field(
            serde_json::to_value(&instance).expect("VlanDiscoveryResponse serializes"),
        )
    }
}

// ---------------------------------------------------------------------------
// Auto-registration + auto-run test (mirrors `SubscriberRegistration`).
// ---------------------------------------------------------------------------

/// A forward-compat check for one registered `DaemonResponse` type, collected
/// via `inventory` so the test below need not maintain a list.
pub struct DaemonResponseCheck {
    check: fn(),
}

impl DaemonResponseCheck {
    pub const fn new<T: DaemonResponse>() -> Self {
        Self {
            check: || {
                if let Err(e) = serde_json::from_value::<T>(T::skewed()) {
                    panic!(
                        "{} rejects a newer-server payload (not forward-compatible): {e}",
                        std::any::type_name::<T>()
                    );
                }
            },
        }
    }

    /// Run the forward-compat assertion for this type (panics if it fails).
    pub fn run(&self) {
        (self.check)()
    }
}

inventory::collect!(DaemonResponseCheck);

// One submission per type the daemon actually deserializes (the `ApiClient`
// call sites). Adding a new daemon-consumed type → the `DaemonResponse` bound
// forces an impl, and a line here enrolls it in the auto-run test.
inventory::submit!(DaemonResponseCheck::new::<Vec<Subnet>>());
inventory::submit!(DaemonResponseCheck::new::<Subnet>());
inventory::submit!(DaemonResponseCheck::new::<HostResponse>());
inventory::submit!(DaemonResponseCheck::new::<DaemonRegistrationResponse>());
inventory::submit!(DaemonResponseCheck::new::<ServerCapabilities>());
inventory::submit!(DaemonResponseCheck::new::<(
    Option<DaemonDiscoveryRequest>,
    bool
)>());
inventory::submit!(DaemonResponseCheck::new::<VlanDiscoveryResponse>());

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registered daemon-consumed response type must tolerate a payload
    /// from a newer server. Adding a type to the registry above runs it here
    /// automatically — no list to maintain.
    #[test]
    fn registered_daemon_responses_are_forward_compatible() {
        let mut count = 0;
        for check in inventory::iter::<DaemonResponseCheck> {
            check.run();
            count += 1;
        }
        assert!(
            count > 0,
            "no DaemonResponse types registered — inventory wiring is broken"
        );
    }
}
