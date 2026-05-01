//! Organizations subscriber for OnboardingOperation and BillingOperation events.
//!
//! Onboarding: persists milestone discriminants onto `organizations.onboarding`
//! so the UI checklist renders without re-deriving from the event log.
//!
//! Billing: updates flag columns (`last_paused_at`, `trial_extended_used`,
//! `last_downgrade_at`, `last_downgrade_from_plan`) on the four billing
//! variants that drive Phase 5 eligibility gates and the downgrade banner.

use anyhow::Error;
use async_trait::async_trait;

use strum::IntoDiscriminant;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    organizations::service::OrganizationService,
    shared::{
        events::{
            registry::SubscriberRegistration,
            traits::{Event, EventFilter, Subscriber},
            types::{BillingOperation, BillingOperationDiscriminants, OnboardingOperation},
        },
        services::traits::CrudService,
    },
};

#[async_trait]
impl Subscriber<OnboardingOperation> for OrganizationService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            if let Some(mut organization) = self.get_by_id(&event.scope.organization_id).await? {
                let onboarding_step = event.operation.discriminant();
                if organization.not_onboarded(&onboarding_step) {
                    organization.base.onboarding.push(onboarding_step);
                    self.update(&mut organization, AuthenticatedEntity::System)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    OrganizationService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<BillingOperation> for OrganizationService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::ops(vec![
            BillingOperationDiscriminants::Paused,
            BillingOperationDiscriminants::TrialExtended,
            BillingOperationDiscriminants::PlanChanged,
            BillingOperationDiscriminants::SubscriptionCancelled,
        ])
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            let org_id = event.scope.organization_id;
            let Some(mut organization) = self.get_by_id(&org_id).await? else {
                continue;
            };

            let mut changed = false;
            match &event.operation {
                BillingOperation::Paused { .. } => {
                    organization.base.last_paused_at = Some(event.timestamp);
                    changed = true;
                }
                BillingOperation::TrialExtended { .. } => {
                    if !organization.base.trial_extended_used {
                        organization.base.trial_extended_used = true;
                        changed = true;
                    }
                }
                BillingOperation::PlanChanged {
                    from,
                    is_downgrade: true,
                    ..
                } => {
                    organization.base.last_downgrade_at = Some(event.timestamp);
                    organization.base.last_downgrade_from_plan = Some(*from);
                    changed = true;
                }
                BillingOperation::SubscriptionCancelled { plan, .. } => {
                    organization.base.last_downgrade_at = Some(event.timestamp);
                    organization.base.last_downgrade_from_plan = Some(*plan);
                    changed = true;
                }
                _ => {}
            }

            if changed {
                self.update(&mut organization, AuthenticatedEntity::System)
                    .await?;
            }
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    OrganizationService,
    BillingOperation,
>());
