use serde::Deserialize;
use serde::de::DeserializeOwned;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::server::shared::storage::filter::EntityFilter;

// ============================================================================
// Pagination Parameters
// ============================================================================

/// Pagination parameters that can be composed into filter queries.
///
/// Default behavior:
/// - `limit`: 50 (returns up to 50 results)
/// - `offset`: 0 (starts from the beginning)
/// - `limit=0`: Returns all results (unlimited)
/// - Maximum `limit`: 1000 (to prevent abuse)
#[derive(Deserialize, Default, Debug, Clone, IntoParams, utoipa::ToSchema)]
pub struct PaginationParams {
    /// Maximum number of results to return. Default: 50. Use 0 for unlimited.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl PaginationParams {
    /// Default limit when not specified
    pub const DEFAULT_LIMIT: u32 = 50;
    /// Maximum allowed limit (0 means unlimited)
    pub const MAX_LIMIT: u32 = 1000;

    /// Get the effective limit, applying defaults and caps.
    /// Returns None if unlimited (limit=0).
    pub fn effective_limit(&self) -> Option<u32> {
        match self.limit {
            Some(0) => None, // Unlimited
            Some(n) => Some(n.min(Self::MAX_LIMIT)),
            None => Some(Self::DEFAULT_LIMIT),
        }
    }

    /// Get the effective offset, defaulting to 0.
    pub fn effective_offset(&self) -> u32 {
        self.offset.unwrap_or(0)
    }

    /// Apply pagination to an EntityFilter.
    pub fn apply_to_filter(&self, filter: EntityFilter) -> EntityFilter {
        let filter = if let Some(limit) = self.effective_limit() {
            filter.limit(limit)
        } else {
            filter
        };
        filter.offset(self.effective_offset())
    }
}

// ============================================================================
// Filter Query Extractor Trait
// ============================================================================

/// Trait for query structs that filter entities by network or organization.
pub trait FilterQueryExtractor: DeserializeOwned + Send + Sync + Default {
    /// Apply query parameters to the filter, respecting user's access permissions.
    fn apply_to_filter(
        &self,
        filter: EntityFilter,
        user_network_ids: &[Uuid],
        user_organization_id: Uuid,
    ) -> EntityFilter;

    /// Get pagination parameters from the query.
    fn pagination(&self) -> PaginationParams;
}

// ============================================================================
// Standard filter query types for CrudHandlers
// ============================================================================

/// Filter query for entities keyed by network_id.
/// Allows filtering to a specific network the user has access to.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct NetworkFilterQuery {
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,
}

impl FilterQueryExtractor for NetworkFilterQuery {
    fn apply_to_filter(
        &self,
        filter: EntityFilter,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> EntityFilter {
        match self.network_id {
            Some(id) if user_network_ids.contains(&id) => filter.network_ids(&[id]),
            Some(_) => filter.network_ids(&[]), // User doesn't have access - return empty
            None => filter.network_ids(user_network_ids),
        }
    }

    fn pagination(&self) -> PaginationParams {
        self.pagination.clone().unwrap_or_default()
    }
}

/// Empty filter query for entities that are scoped to org (or are the org itself) and don't support further filtering by query param
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct NoFilterQuery {
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,
}

impl FilterQueryExtractor for NoFilterQuery {
    fn apply_to_filter(
        &self,
        filter: EntityFilter,
        _user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> EntityFilter {
        // Don't apply additional filters (network_id / org_id permissioning is taken care of in handler)
        filter
    }

    fn pagination(&self) -> PaginationParams {
        self.pagination.clone().unwrap_or_default()
    }
}

/// Query for filtering by group_id (used by GroupBinding).
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct GroupIdQuery {
    /// Filter by group ID
    pub group_id: Uuid,
    /// Filter by network ID
    pub network_id: Uuid,
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,
}

// ============================================================================
// Combined query types for child entities with network filtering
// ============================================================================

/// Query for filtering ports by host_id and/or network_id.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct HostChildQuery {
    /// Filter by host ID
    pub host_id: Option<Uuid>,
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,
}

impl FilterQueryExtractor for HostChildQuery {
    fn apply_to_filter(
        &self,
        filter: EntityFilter,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> EntityFilter {
        let filter = match self.network_id {
            Some(id) if user_network_ids.contains(&id) => filter.network_ids(&[id]),
            Some(_) => filter.network_ids(&[]),
            None => filter.network_ids(user_network_ids),
        };
        match self.host_id {
            Some(id) => filter.host_id(&id),
            None => filter,
        }
    }

    fn pagination(&self) -> PaginationParams {
        self.pagination.clone().unwrap_or_default()
    }
}

/// Query for filtering bindings by service_id and/or network_id.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct BindingQuery {
    /// Filter by service ID
    pub service_id: Option<Uuid>,
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Filter by port ID
    pub port_id: Option<Uuid>,
    /// Filter by interface ID
    pub interface_id: Option<Uuid>,
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,
}

impl FilterQueryExtractor for BindingQuery {
    fn apply_to_filter(
        &self,
        filter: EntityFilter,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> EntityFilter {
        let mut filter = match self.network_id {
            Some(id) if user_network_ids.contains(&id) => filter.network_ids(&[id]),
            Some(_) => filter.network_ids(&[]),
            None => filter.network_ids(user_network_ids),
        };
        filter = match self.service_id {
            Some(id) => filter.service_id(&id),
            None => filter,
        };
        filter = match self.port_id {
            Some(id) => filter.uuid_column("port_id", &id),
            None => filter,
        };
        filter = match self.interface_id {
            Some(id) => filter.uuid_column("interface_id", &id),
            None => filter,
        };

        filter
    }

    fn pagination(&self) -> PaginationParams {
        self.pagination.clone().unwrap_or_default()
    }
}

/// Query for filtering interfaces by host_id, subnet_id, and/or network_id.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct InterfaceQuery {
    /// Filter by host ID
    pub host_id: Option<Uuid>,
    /// Filter by subnet ID
    pub subnet_id: Option<Uuid>,
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,
}

impl FilterQueryExtractor for InterfaceQuery {
    fn apply_to_filter(
        &self,
        filter: EntityFilter,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> EntityFilter {
        let mut filter = match self.network_id {
            Some(id) if user_network_ids.contains(&id) => filter.network_ids(&[id]),
            Some(_) => filter.network_ids(&[]),
            None => filter.network_ids(user_network_ids),
        };
        filter = match self.subnet_id {
            Some(id) => filter.subnet_id(&id),
            None => filter,
        };
        filter = match self.host_id {
            Some(id) => filter.host_id(&id),
            None => filter,
        };

        filter
    }

    fn pagination(&self) -> PaginationParams {
        self.pagination.clone().unwrap_or_default()
    }
}

/// Query for filtering discoveries by network_id or daemon_id
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct DiscoveryQuery {
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Filter by daemon ID
    pub daemon_id: Option<Uuid>,
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,
}

impl FilterQueryExtractor for DiscoveryQuery {
    fn apply_to_filter(
        &self,
        filter: EntityFilter,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> EntityFilter {
        let mut filter = match self.network_id {
            Some(id) if user_network_ids.contains(&id) => filter.network_ids(&[id]),
            Some(_) => filter.network_ids(&[]),
            None => filter.network_ids(user_network_ids),
        };
        filter = match self.daemon_id {
            Some(id) => filter.uuid_column("daemon_id", &id),
            None => filter,
        };

        filter
    }

    fn pagination(&self) -> PaginationParams {
        self.pagination.clone().unwrap_or_default()
    }
}

/// Query for filtering shares by network_id or topology_id
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct SharesQuery {
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Filter by topology ID
    pub topology_id: Option<Uuid>,
    /// Pagination parameters
    pub pagination: Option<PaginationParams>,
}

impl FilterQueryExtractor for SharesQuery {
    fn apply_to_filter(
        &self,
        filter: EntityFilter,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> EntityFilter {
        let mut filter = match self.network_id {
            Some(id) if user_network_ids.contains(&id) => filter.network_ids(&[id]),
            Some(_) => filter.network_ids(&[]),
            None => filter.network_ids(user_network_ids),
        };
        filter = match self.topology_id {
            Some(id) => filter.uuid_column("topology_id", &id),
            None => filter,
        };

        filter
    }

    fn pagination(&self) -> PaginationParams {
        self.pagination.clone().unwrap_or_default()
    }
}
