//! Organizations subscriber for OnboardingOperation and BillingOperation events.
//!
//! Onboarding: persists milestone discriminants onto `organizations.onboarding`
//! so the UI checklist renders without re-deriving from the event log.
//!
//! Billing: updates flag columns (`last_paused_at`, `trial_extended_used`,
//! `last_downgrade_at`, `last_downgrade_from_plan`) on the variants that drive
//! Phase 5 eligibility gates and the downgrade banner; mirrors
//! `BillingOperation::implied_status()` onto `organizations.plan_status` so
//! every billing event keeps the canonical status column in sync; and writes
//! `organizations.plan` + `trial_end_date` from the variants that establish
//! or change the current plan (`CheckoutCompleted`, `TrialStarted`,
//! `PlanChanged`, `TrialExtended`).

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
            types::{BillingOperation, OnboardingOperation},
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

        tracing::info!(
            event_count = events.len(),
            "OrganizationService onboarding subscriber: handle entry"
        );

        for event in events {
            let onboarding_step = event.operation.discriminant();
            tracing::info!(
                org_id = %event.scope.organization_id,
                onboarding_step = ?onboarding_step,
                "OrganizationService onboarding subscriber: processing event"
            );
            if let Some(mut organization) = self.get_by_id(&event.scope.organization_id).await? {
                let not_onboarded = organization.not_onboarded(&onboarding_step);
                tracing::info!(
                    org_id = %event.scope.organization_id,
                    onboarding_step = ?onboarding_step,
                    not_onboarded = not_onboarded,
                    "OrganizationService onboarding subscriber: not_onboarded check"
                );
                if not_onboarded {
                    organization.base.onboarding.push(onboarding_step);
                    self.update(&mut organization, AuthenticatedEntity::System)
                        .await?;
                    tracing::info!(
                        org_id = %event.scope.organization_id,
                        onboarding_step = ?onboarding_step,
                        "OrganizationService onboarding subscriber: pushed + persisted"
                    );
                }
            } else {
                tracing::warn!(
                    org_id = %event.scope.organization_id,
                    onboarding_step = ?onboarding_step,
                    "OrganizationService onboarding subscriber: org not found"
                );
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
        // Wildcard so every billing event participates in the plan_status
        // mirror below; per-variant flag updates still gate themselves.
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        // Diagnostic tracing: surface every step of the plan_status mirror so
        // when an event doesn't flip plan_status we can tell from logs whether
        // the subscriber ran, what the prior status was, what implied_status
        // returned, whether the comparison fired, and whether the update
        // succeeded. Targeted at the Portal-cancel `pending_cancellation`
        // mirror bug under investigation (2026-06-15); kept in long-term as
        // these traces are cheap and the plan_status mirror is critical.
        let batch_ops: Vec<String> = events
            .iter()
            .map(|e| e.operation.discriminant().to_string())
            .collect();
        let batch_orgs: Vec<uuid::Uuid> = events.iter().map(|e| e.scope.organization_id).collect();
        tracing::info!(
            event_count = events.len(),
            ops = ?batch_ops,
            org_ids = ?batch_orgs,
            "OrganizationService<BillingOperation>::handle batch arrived",
        );

        for event in events {
            let org_id = event.scope.organization_id;
            let op_disc = event.operation.discriminant().to_string();
            let Some(mut organization) = self.get_by_id(&org_id).await? else {
                tracing::warn!(
                    %org_id,
                    op = %op_disc,
                    "OrganizationService<BillingOperation>::handle org not found, skipping",
                );
                continue;
            };

            let prior_status = organization.base.plan_status;
            let implied = event.operation.implied_status();
            tracing::info!(
                %org_id,
                op = %op_disc,
                prior_plan_status = ?prior_status,
                implied_status = ?implied,
                "OrganizationService<BillingOperation>::handle per-event",
            );

            let mut changed = false;
            match &event.operation {
                BillingOperation::Paused { .. } => {
                    organization.base.last_paused_at = Some(event.timestamp);
                    changed = true;
                }
                BillingOperation::CheckoutCompleted { plan, .. } => {
                    if organization.base.plan.as_ref() != Some(plan) {
                        organization.base.plan = Some(*plan);
                        changed = true;
                    }
                }
                BillingOperation::TrialStarted {
                    plan, trial_end, ..
                } => {
                    if organization.base.plan.as_ref() != Some(plan) {
                        organization.base.plan = Some(*plan);
                        changed = true;
                    }
                    if organization.base.trial_end_date != Some(*trial_end) {
                        organization.base.trial_end_date = Some(*trial_end);
                        changed = true;
                    }
                }
                BillingOperation::TrialExtended { new_trial_end, .. } => {
                    if !organization.base.trial_extended_used {
                        organization.base.trial_extended_used = true;
                        changed = true;
                    }
                    if organization.base.trial_end_date != Some(*new_trial_end) {
                        organization.base.trial_end_date = Some(*new_trial_end);
                        changed = true;
                    }
                }
                BillingOperation::PlanChanged {
                    from,
                    to,
                    is_downgrade,
                    ..
                } => {
                    if organization.base.plan.as_ref() != Some(to) {
                        organization.base.plan = Some(*to);
                        changed = true;
                    }
                    if *is_downgrade {
                        organization.base.last_downgrade_at = Some(event.timestamp);
                        organization.base.last_downgrade_from_plan = Some(*from);
                        changed = true;
                    }
                }
                BillingOperation::SubscriptionCancelled { plan, .. } => {
                    organization.base.last_downgrade_at = Some(event.timestamp);
                    organization.base.last_downgrade_from_plan = Some(*plan);
                    changed = true;
                }
                _ => {}
            }

            // Mirror the canonical PlanStatus implied by every billing
            // operation onto `plan_status`. Single source of truth via
            // `BillingOperation::implied_status()`; downstream consumers
            // (auth gates, BillingTab pills, Brevo sync) read the typed
            // enum, set in one place.
            if let Some(status) = implied {
                let new_status = Some(status);
                if organization.base.plan_status != new_status {
                    tracing::info!(
                        %org_id,
                        op = %op_disc,
                        from = ?organization.base.plan_status,
                        to = ?new_status,
                        "OrganizationService<BillingOperation>::handle mirror writing plan_status",
                    );
                    organization.base.plan_status = new_status;
                    changed = true;
                } else {
                    tracing::info!(
                        %org_id,
                        op = %op_disc,
                        plan_status = ?new_status,
                        "OrganizationService<BillingOperation>::handle mirror noop (plan_status already matches)",
                    );
                }
            }

            if changed {
                tracing::info!(
                    %org_id,
                    op = %op_disc,
                    "OrganizationService<BillingOperation>::handle calling update",
                );
                match self
                    .update(&mut organization, AuthenticatedEntity::System)
                    .await
                {
                    Ok(_) => {
                        tracing::info!(
                            %org_id,
                            op = %op_disc,
                            persisted_plan_status = ?organization.base.plan_status,
                            "OrganizationService<BillingOperation>::handle update Ok",
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            %org_id,
                            op = %op_disc,
                            error = %e,
                            "OrganizationService<BillingOperation>::handle update Err",
                        );
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    OrganizationService,
    BillingOperation,
>());
