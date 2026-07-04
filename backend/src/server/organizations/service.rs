use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_commercial_self_hosted_plan;
use crate::server::billing::types::base::BillingPlan;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::EventBusService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::tags::entity_tags::EntityTagService;
use crate::server::{
    organizations::r#impl::base::Organization,
    shared::{services::traits::CrudService, storage::generic::GenericPostgresStorage},
};
use anyhow::Error;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

pub struct OrganizationService {
    storage: Arc<GenericPostgresStorage<Organization>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<Organization> for OrganizationService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &Organization) -> Option<Uuid> {
        None
    }
    fn get_organization_id(&self, entity: &Organization) -> Option<Uuid> {
        Some(entity.id)
    }
}

#[async_trait]
impl CrudService<Organization> for OrganizationService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Organization>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl OrganizationService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Organization>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self { storage, event_bus }
    }

    /// Reconcile self-hosted org plans to the entitlement implied by a present,
    /// valid commercial license: upgrade every org still on the Community plan to
    /// CommercialSelfHosted. The caller gates this on a self-hosted deployment
    /// (no Stripe secret) with `LicenseStatus::Valid`, so this walks all orgs and
    /// upgrades only the Community ones.
    ///
    /// Idempotent — orgs already on CommercialSelfHosted (or any other plan) are
    /// skipped, so re-running on every boot is a no-op once reconciled. The plan
    /// write goes through the `LicenseReconciled` billing event → this service's
    /// own `Subscriber<BillingOperation>` impl (the sole writer of
    /// `organizations.plan`); we never write the row here. Best-effort: a per-org
    /// publish failure is logged, not fatal. Returns the number of orgs upgraded.
    pub async fn reconcile_self_hosted_license_plans(&self) -> Result<u64, Error> {
        let orgs = self.get_all(StorableFilter::<Organization>::new()).await?;

        let target = get_commercial_self_hosted_plan();
        let mut upgraded = 0u64;
        for org in orgs {
            // `None` resolves to the build default; on a self-hosted commercial
            // build that is CommercialSelfHosted, which is correctly skipped.
            let current = org.base.plan.unwrap_or_default();
            if !matches!(current, BillingPlan::Community(_)) {
                continue;
            }

            if let Err(e) = self
                .event_bus()
                .publish(Event::new(
                    OrgScope {
                        organization_id: org.id,
                    },
                    BillingOperation::LicenseReconciled {
                        from: current,
                        to: target,
                    },
                    AuthenticatedEntity::System,
                ))
                .await
            {
                tracing::warn!(
                    organization_id = %org.id,
                    error = %e,
                    "Failed to publish license reconciliation for org",
                );
                continue;
            }
            upgraded += 1;
        }

        if upgraded > 0 {
            tracing::info!(
                count = upgraded,
                "Reconciled self-hosted org plan(s) to CommercialSelfHosted from license entitlement",
            );
        }

        Ok(upgraded)
    }
}
