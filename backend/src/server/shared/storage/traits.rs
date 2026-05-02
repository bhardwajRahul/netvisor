use std::net::IpAddr;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::dependencies::r#impl::base::Dependency;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::entity_metadata::EntityCategory;
use crate::server::shared::events::types::{BillingOperation, OnboardingOperationDiscriminants};
use crate::server::shares::r#impl::base::ShareOptions;
use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::virtualization::SubnetVirtualization;
use crate::server::tags::r#impl::base::Tag;
use crate::server::topology::types::views::TopologyView;
use crate::server::vlans::r#impl::base::Vlan;
use crate::server::{
    billing::types::base::BillingPlan,
    daemons::r#impl::{api::DaemonCapabilities, base::DaemonMode},
    discovery::r#impl::types::{DiscoveryType, RunType},
    hosts::r#impl::{base::Host, virtualization::HostVirtualization},
    interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress,
    organizations::r#impl::base::PlanLimitNotifications,
    ports::r#impl::base::Port,
    services::r#impl::{definitions::ServiceDefinition, virtualization::ServiceVirtualization},
    shared::{storage::filter::StorableFilter, types::entities::EntitySource},
    topology::types::{
        base::TopologyOptions,
        edges::{Edge, EdgeStyle},
        nodes::Node,
    },
    users::r#impl::permissions::UserOrgPermissions,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cidr::IpCidr;
use email_address::EmailAddress;
use mac_address::MacAddress;
use sqlx::postgres::PgRow;
use stripe_billing::SubscriptionStatus;
use uuid::Uuid;

/// Result of a paginated query, containing items and total count.
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    /// The items for the current page
    pub items: Vec<T>,
    /// Total count of items matching the filter (ignoring limit/offset)
    pub total_count: u64,
}

#[async_trait]
pub trait Storage<T: Storable>: Send + Sync {
    async fn create(&self, entity: &T) -> Result<T, anyhow::Error>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<T>, anyhow::Error>;
    async fn get_all(&self, filter: StorableFilter<T>) -> Result<Vec<T>, anyhow::Error>;
    async fn get_all_ordered(
        &self,
        filter: StorableFilter<T>,
        order_by: &str,
    ) -> Result<Vec<T>, anyhow::Error>;
    /// Get entities with pagination, returning items and total count.
    /// The filter's limit/offset are applied to the query.
    async fn get_paginated(
        &self,
        filter: StorableFilter<T>,
        order_by: &str,
    ) -> Result<PaginatedResult<T>, anyhow::Error>;
    async fn get_one(&self, filter: StorableFilter<T>) -> Result<Option<T>, anyhow::Error>;
    async fn update(&self, entity: &mut T) -> Result<T, anyhow::Error>;
    async fn delete(&self, id: &Uuid) -> Result<(), anyhow::Error>;
    async fn create_many(&self, entities: &[T]) -> Result<Vec<T>, anyhow::Error>;
    async fn update_many(&self, entities: &[T]) -> Result<Vec<T>, anyhow::Error>;
    async fn delete_many(&self, ids: &[Uuid]) -> Result<usize, anyhow::Error>;
    async fn delete_by_filter(&self, filter: StorableFilter<T>) -> Result<usize, anyhow::Error>;
}

/// Base trait for anything stored in the database, including junction tables.
/// Provides the minimal interface needed for storage operations.
pub trait Storable: Sized + Clone + Send + Sync + 'static + Default {
    type BaseData;

    fn new(base: Self::BaseData) -> Self;
    fn get_base(&self) -> Self::BaseData;

    /// Database table name
    fn table_name() -> &'static str;

    /// Serialization for database storage
    /// Returns (column_names, bind_values)
    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error>;

    /// Deserialization from database
    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error>;
}

/// Extended trait for user-facing domain entities (excludes junction tables).
/// Provides entity metadata, tenant scoping, timestamps, and tagging support.
pub trait Entity: Storable {
    /// Primary key
    fn id(&self) -> Uuid;
    fn created_at(&self) -> DateTime<Utc>;
    fn set_id(&mut self, id: Uuid);
    fn set_created_at(&mut self, time: DateTime<Utc>);

    /// Entity type discriminant for the entity enum
    fn entity_type() -> EntityDiscriminants;

    /// CSV row type for export. Must be Serialize.
    /// The csv crate derives headers automatically from field names.
    type CsvRow: serde::Serialize;

    /// Converts this entity to a CSV row struct.
    fn to_csv_row(&self) -> Self::CsvRow;

    /// Singular name for error messages (e.g., "host")
    /// Use the constant in const contexts, use the method at runtime.
    const ENTITY_NAME_SINGULAR: &'static str;

    /// Plural name for API paths and collections (e.g., "hosts")
    /// Use the constant in const contexts, use the method at runtime.
    const ENTITY_NAME_PLURAL: &'static str;

    /// Description for API documentation and database schema docs.
    /// Should be 1-3 sentences explaining the entity's purpose.
    const ENTITY_DESCRIPTION: &'static str;

    /// Category for documentation grouping.
    fn entity_category() -> EntityCategory;

    /// Singular name for error messages (e.g., "host")
    fn entity_name_singular() -> &'static str {
        Self::ENTITY_NAME_SINGULAR
    }

    /// Plural name for API paths and collections (e.g., "hosts")
    fn entity_name_plural() -> &'static str {
        Self::ENTITY_NAME_PLURAL
    }

    /// Tenant scoping - network context
    fn network_id(&self) -> Option<Uuid>;

    /// Tenant scoping - organization context
    fn organization_id(&self) -> Option<Uuid>;

    /// Whether entities of this type are scoped to a network
    fn is_network_keyed() -> bool {
        Self::default().network_id().is_some()
    }

    /// Whether entities of this type are scoped to an organization
    fn is_organization_keyed() -> bool {
        Self::default().organization_id().is_some()
    }

    /// Last modification timestamp
    fn updated_at(&self) -> DateTime<Utc>;
    fn set_updated_at(&mut self, time: DateTime<Utc>);

    /// Whether this entity type supports tagging.
    /// Default implementation delegates to `EntityDiscriminants::is_taggable`.
    fn is_taggable() -> bool {
        Self::entity_type().is_taggable()
    }

    /// Get the tags field from the entity for validation.
    /// Override for entities with a tags field.
    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        None
    }

    /// Set the tags field on the entity.
    /// Override for entities with a tags field.
    fn set_tags(&mut self, _tags: Vec<Uuid>) {
        // Default: no-op
    }

    /// Set the source field on the entity.
    /// Override for entities with a source field.
    fn set_source(&mut self, _source: EntitySource) {
        // Default: no-op
    }

    /// Preserve entity-specific immutable fields from the existing entity.
    /// Override for entities that have additional read-only fields beyond id/created_at.
    fn preserve_immutable_fields(&mut self, _existing: &Self) {
        // Default: no-op
    }
}

/// Helper type for SQL values
#[derive(Clone, strum_macros::EnumDiscriminants)]
#[strum_discriminants(derive(strum_macros::EnumIter))]
pub enum SqlValue {
    Uuid(Uuid),
    OptionalUuid(Option<Uuid>),
    String(String),
    OptionalString(Option<String>),
    I32(i32),
    OptionalI64(Option<i64>),
    U16(u16),
    Bool(bool),
    Email(EmailAddress),
    Timestamp(DateTime<Utc>),
    OptionTimestamp(Option<DateTime<Utc>>),
    UuidArray(Vec<Uuid>),
    IpCidr(IpCidr),
    IpAddr(IpAddr),
    OptionalIpAddr(Option<IpAddr>),
    EntitySource(EntitySource),
    EntityDiscriminant(EntityDiscriminants),
    ServiceDefinition(Box<dyn ServiceDefinition>),
    OptionalServiceVirtualization(Option<ServiceVirtualization>),
    OptionalHostVirtualization(Option<HostVirtualization>),
    OptionalSubnetVirtualization(Option<SubnetVirtualization>),
    Ports(Vec<Port>),
    IPAddresses(Vec<IPAddress>),
    RunType(RunType),
    DiscoveryType(DiscoveryType),
    DaemonCapabilities(DaemonCapabilities),
    UserOrgPermissions(UserOrgPermissions),
    OptionBillingPlan(Option<BillingPlan>),
    OptionBillingPlanStatus(Option<SubscriptionStatus>),
    BillingOperation(BillingOperation),
    AuthenticatedEntity(AuthenticatedEntity),
    EdgeStyle(EdgeStyle),
    DaemonMode(DaemonMode),
    Nodes(Vec<Node>),
    Edges(Vec<Edge>),
    TopologyOptions(TopologyOptions),
    Hosts(Vec<Host>),
    Subnets(Vec<Subnet>),
    Services(Vec<Service>),
    Bindings(Vec<Binding>),
    Dependencies(Vec<Dependency>),
    OnboardingOperation(Vec<OnboardingOperationDiscriminants>),
    StringArray(Vec<String>),
    OptionalStringArray(Option<Vec<String>>),
    OptionalLldpChassisId(Option<LldpChassisId>),
    OptionalLldpPortId(Option<LldpPortId>),
    OptionalFdbMacs(Option<Vec<String>>),
    OptionVecU16(Option<Vec<u16>>),
    OptionVecUuid(Option<Vec<Uuid>>),
    ShareOptions(ShareOptions),
    EnabledViews(Option<Vec<TopologyView>>),
    CredentialType(CredentialType),
    MacAddress(MacAddress),
    OptionalMacAddress(Option<MacAddress>),
    Interfaces(Vec<Interface>),
    Tags(Vec<Tag>),
    Vlans(Vec<Vlan>),
    PlanLimitNotifications(PlanLimitNotifications),
    OptionalIpAddrArray(Option<Vec<IpAddr>>),
    OptionalUuidVec(Option<Vec<Uuid>>),
}

// ============================================================================
// DB-backed enum catalog for backward-compat tests
// ============================================================================
//
// `SqlValue` is the complete typed catalog of everything the storage layer
// writes to the DB. By walking every `SqlValue` variant and contributing the
// variant names of each DB-backed Rust enum it transitively reaches, we build
// a baseline that lets us detect:
//
//   - removed/renamed variants that break an upgraded binary reading old rows
//     (forward-compat — Test A in tests.rs)
//   - added variants that break an old binary reading new rows during deploy
//     coexistence (backward-compat — Test B in tests.rs)
//
// Two compile-time gates ensure the catalog can't silently drift:
//   1. The `match` in `SqlValue::contribute_db_enum_variants` is exhaustive —
//      adding a new `SqlValue` variant fails the build.
//   2. Each non-primitive arm dispatches via the `DbEnumContributor` trait —
//      wrapping a type that doesn't implement the trait fails the build.

/// A type whose DB-persisted variant names (if any) should flow into the
/// backward-compat baseline fixture. Empty impls are legal — not every type
/// wrapped by `SqlValue` has DB-backed enums.
pub trait DbEnumContributor {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>);
}

/// Bare type name from `std::any::type_name`, stripping the module path.
/// Used as the fixture key so renames refactor automatically.
pub fn db_enum_key_for<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or("?")
}

/// Empty impls for types that wrap no DB-backed enums (primitives, foreign
/// types, composite structs whose enum fields are covered elsewhere).
macro_rules! impl_db_enum_contributor_empty {
    ($($t:ty),* $(,)?) => {
        $(
            impl DbEnumContributor for $t {
                fn contribute(_: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {}
            }
        )*
    };
}

/// Populating impls for DB-backed enums. Uses `strum::VariantNames` to obtain
/// the list of variant identifiers at compile time — no instance construction
/// needed, so enums with non-`Default` payload fields work here too.
///
/// Gotcha: `strum::VariantNames` returns Rust identifiers. If a variant carries
/// `#[serde(rename = "...")]`, the DB-persisted tag differs from the Rust name;
/// the baseline will need manual extension to include the renamed form.
macro_rules! impl_db_enum_contributor_via_variant_names {
    ($($t:ty),* $(,)?) => {
        $(
            impl DbEnumContributor for $t {
                fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
                    let variants: Vec<String> = <$t as ::strum::VariantNames>::VARIANTS
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                    out.insert(db_enum_key_for::<$t>(), variants);
                }
            }
        )*
    };
}

// Primitives and foreign types wrapped in SqlValue — no DB-backed enums.
impl_db_enum_contributor_empty!(
    Uuid,
    String,
    i32,
    i64,
    u16,
    u8,
    bool,
    EmailAddress,
    DateTime<Utc>,
    IpCidr,
    IpAddr,
    MacAddress,
);

// Generic wrapper pass-through — inner type's contribution flows up.
impl<T: DbEnumContributor> DbEnumContributor for Option<T> {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        T::contribute(out);
    }
}
impl<T: DbEnumContributor> DbEnumContributor for Vec<T> {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        T::contribute(out);
    }
}

// Trait object: no enumerable variants. Dynamic dispatch through
// ServiceDefinition covers service metadata (Docker, nginx, etc.), not
// DB-persisted discriminants — out of scope for this catalog.
impl DbEnumContributor for Box<dyn ServiceDefinition> {
    fn contribute(_: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {}
}

// Composite structs wrapped in SqlValue. Their enum fields (if any) are
// already reachable directly through other SqlValue variants, so these
// contribute nothing themselves. If a composite gains a nested enum that's
// NOT reachable elsewhere, replace the empty impl with one that delegates
// to the nested enum's `contribute`.
impl_db_enum_contributor_empty!(
    DaemonCapabilities,
    UserOrgPermissions,
    TopologyOptions,
    ShareOptions,
    PlanLimitNotifications,
    Port,
    IPAddress,
    Host,
    Subnet,
    Service,
    Binding,
    Dependency,
    Interface,
    Tag,
    Vlan,
    Node,
    Edge,
);

// DB-backed enums. Each gets a variant-names contribution. Requires
// `#[derive(strum::VariantNames)]` on the enum definition.
//
// Note: `SubscriptionStatus` is a foreign type from the `stripe_billing`
// crate — we can't add derives to it. Treated as empty below (Stripe SDK
// version bumps are explicit and coordinated with server deploys, so the
// coexistence-window risk is negligible in practice).
impl_db_enum_contributor_via_variant_names!(
    EntitySource,
    HostVirtualization,
    ServiceVirtualization,
    SubnetVirtualization,
    RunType,
    DiscoveryType,
    BillingPlan,
    EdgeStyle,
    DaemonMode,
    CredentialType,
    LldpChassisId,
    LldpPortId,
    TopologyView,
    crate::server::billing::types::base::CancelReason,
    crate::server::billing::types::base::SaveOffer,
    crate::server::billing::types::base::LimitType,
    crate::server::billing::types::base::LimitSource,
    AuthenticatedEntity,
    OnboardingOperationDiscriminants,
);

// BillingOperation is a typed sum with payload-bearing variants. Its
// `Discriminants` enum carries the variant names; we contribute via that.
impl DbEnumContributor for BillingOperation {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        // BillingOperation itself doesn't impl VariantNames (struct variants
        // confuse strum::VariantNames), but the Discriminants enum does not
        // either by default — list variant names explicitly to keep the
        // backward-compat baseline accurate.
        let variants = vec![
            "CheckoutStarted".to_string(),
            "CheckoutCompleted".to_string(),
            "TrialStarted".to_string(),
            "TrialWillEnd".to_string(),
            "TrialEnded".to_string(),
            "PlanChanged".to_string(),
            "SubscriptionCancelled".to_string(),
            "PaymentFailed".to_string(),
            "PaymentActionRequired".to_string(),
            "PaymentRecovered".to_string(),
            "FeatureLimitHit".to_string(),
            "Paused".to_string(),
            "Resumed".to_string(),
            "TrialExtended".to_string(),
            "CancellationInitiated".to_string(),
            "PaymentMethodAdded".to_string(),
            "PaymentMethodRemoved".to_string(),
        ];
        out.insert(db_enum_key_for::<Self>(), variants);
    }
}

// SubscriptionStatus: foreign type from stripe_billing. Empty impl.
impl DbEnumContributor for SubscriptionStatus {
    fn contribute(_: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {}
}

// EntityDiscriminants: auto-generated from Entity via EnumDiscriminants. The
// derive list includes VariantNames (see backend/src/server/shared/entities.rs).
impl DbEnumContributor for EntityDiscriminants {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        let variants: Vec<String> = <Self as ::strum::VariantNames>::VARIANTS
            .iter()
            .map(|s| s.to_string())
            .collect();
        out.insert(db_enum_key_for::<Self>(), variants);
    }
}

impl SqlValue {
    /// For each `SqlValue` variant, contribute the DB-backed enum variant
    /// names reachable through its wrapped type. Exhaustive match on
    /// `SqlValueDiscriminants` forces every variant to be covered.
    fn dispatch_kind(
        kind: SqlValueDiscriminants,
        out: &mut std::collections::BTreeMap<&'static str, Vec<String>>,
    ) {
        use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};
        use ShareOptions;
        use SubnetVirtualization;
        use TopologyView;
        use Vlan;

        match kind {
            SqlValueDiscriminants::Uuid
            | SqlValueDiscriminants::OptionalUuid
            | SqlValueDiscriminants::UuidArray
            | SqlValueDiscriminants::OptionVecUuid
            | SqlValueDiscriminants::OptionalUuidVec => Uuid::contribute(out),
            SqlValueDiscriminants::String
            | SqlValueDiscriminants::OptionalString
            | SqlValueDiscriminants::StringArray
            | SqlValueDiscriminants::OptionalStringArray
            | SqlValueDiscriminants::OptionalFdbMacs => String::contribute(out),
            SqlValueDiscriminants::I32 => i32::contribute(out),
            SqlValueDiscriminants::OptionalI64 => i64::contribute(out),
            SqlValueDiscriminants::U16 | SqlValueDiscriminants::OptionVecU16 => {
                u16::contribute(out)
            }
            SqlValueDiscriminants::Bool => bool::contribute(out),
            SqlValueDiscriminants::Email => EmailAddress::contribute(out),
            SqlValueDiscriminants::Timestamp | SqlValueDiscriminants::OptionTimestamp => {
                <DateTime<Utc>>::contribute(out)
            }
            SqlValueDiscriminants::IpCidr => IpCidr::contribute(out),
            SqlValueDiscriminants::IpAddr
            | SqlValueDiscriminants::OptionalIpAddr
            | SqlValueDiscriminants::OptionalIpAddrArray => IpAddr::contribute(out),
            SqlValueDiscriminants::MacAddress | SqlValueDiscriminants::OptionalMacAddress => {
                MacAddress::contribute(out)
            }
            SqlValueDiscriminants::EntitySource => EntitySource::contribute(out),
            SqlValueDiscriminants::EntityDiscriminant => EntityDiscriminants::contribute(out),
            SqlValueDiscriminants::ServiceDefinition => {
                <Box<dyn ServiceDefinition>>::contribute(out)
            }
            SqlValueDiscriminants::OptionalServiceVirtualization => {
                ServiceVirtualization::contribute(out)
            }
            SqlValueDiscriminants::OptionalHostVirtualization => {
                HostVirtualization::contribute(out)
            }
            SqlValueDiscriminants::OptionalSubnetVirtualization => {
                SubnetVirtualization::contribute(out)
            }
            SqlValueDiscriminants::Ports => Port::contribute(out),
            SqlValueDiscriminants::IPAddresses => IPAddress::contribute(out),
            SqlValueDiscriminants::RunType => RunType::contribute(out),
            SqlValueDiscriminants::DiscoveryType => DiscoveryType::contribute(out),
            SqlValueDiscriminants::DaemonCapabilities => DaemonCapabilities::contribute(out),
            SqlValueDiscriminants::UserOrgPermissions => UserOrgPermissions::contribute(out),
            SqlValueDiscriminants::OptionBillingPlan => BillingPlan::contribute(out),
            SqlValueDiscriminants::OptionBillingPlanStatus => SubscriptionStatus::contribute(out),
            SqlValueDiscriminants::BillingOperation => BillingOperation::contribute(out),
            SqlValueDiscriminants::AuthenticatedEntity => AuthenticatedEntity::contribute(out),
            SqlValueDiscriminants::EdgeStyle => EdgeStyle::contribute(out),
            SqlValueDiscriminants::DaemonMode => DaemonMode::contribute(out),
            SqlValueDiscriminants::Nodes => Node::contribute(out),
            SqlValueDiscriminants::Edges => Edge::contribute(out),
            SqlValueDiscriminants::TopologyOptions => TopologyOptions::contribute(out),
            SqlValueDiscriminants::Hosts => Host::contribute(out),
            SqlValueDiscriminants::Subnets => Subnet::contribute(out),
            SqlValueDiscriminants::Services => Service::contribute(out),
            SqlValueDiscriminants::Bindings => Binding::contribute(out),
            SqlValueDiscriminants::Dependencies => Dependency::contribute(out),
            SqlValueDiscriminants::OnboardingOperation => {
                OnboardingOperationDiscriminants::contribute(out)
            }
            SqlValueDiscriminants::OptionalLldpChassisId => LldpChassisId::contribute(out),
            SqlValueDiscriminants::OptionalLldpPortId => LldpPortId::contribute(out),
            SqlValueDiscriminants::ShareOptions => ShareOptions::contribute(out),
            SqlValueDiscriminants::EnabledViews => TopologyView::contribute(out),
            SqlValueDiscriminants::CredentialType => CredentialType::contribute(out),
            SqlValueDiscriminants::Interfaces => Interface::contribute(out),
            SqlValueDiscriminants::Tags => Tag::contribute(out),
            SqlValueDiscriminants::Vlans => Vlan::contribute(out),
            SqlValueDiscriminants::PlanLimitNotifications => {
                PlanLimitNotifications::contribute(out)
            }
        }
    }

    /// Produces the DB-backed enum baseline for the current binary: every
    /// DB-backed Rust enum reachable through any `SqlValue` variant, with its
    /// known variant names.
    pub fn collect_all_db_enum_variants() -> std::collections::BTreeMap<&'static str, Vec<String>> {
        use strum::IntoEnumIterator;
        let mut out = std::collections::BTreeMap::new();
        for kind in SqlValueDiscriminants::iter() {
            Self::dispatch_kind(kind, &mut out);
        }
        out
    }
}
