//! Payment-method & portal sessions, subscription status, plan changes, and invoice-payment failure handling.
use super::*;

impl BillingService {
    /// Create a checkout session in setup mode to collect payment method
    pub async fn create_setup_payment_method_session(
        &self,
        organization_id: Uuid,
        success_url: String,
        cancel_url: String,
        authentication: AuthenticatedEntity,
    ) -> Result<CheckoutSession, Error> {
        let customer_id = self
            .get_or_create_customer(organization_id, authentication)
            .await?;

        let session = CreateCheckoutSession::new()
            .customer(customer_id)
            .success_url(success_url)
            .cancel_url(cancel_url)
            .mode(CheckoutSessionMode::Setup)
            .currency(stripe_types::Currency::USD)
            .metadata([("organization_id".to_string(), organization_id.to_string())])
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        tracing::info!(
            organization_id = %organization_id,
            session_id = %session.id,
            "Setup payment method session created"
        );

        Ok(session)
    }

    pub async fn create_portal_session(
        &self,
        organization_id: Uuid,
        return_url: String,
    ) -> Result<String, Error> {
        // Get customer ID
        let organization = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow!("Organization not found"))?;

        let customer_id = organization
            .base
            .stripe_customer_id
            .ok_or_else(|| anyhow!("No Stripe customer ID"))?;

        let session = CreateBillingPortalSession::new(CustomerId::from(customer_id.clone()))
            .return_url(return_url)
            .send(&self.stripe)
            .await?;

        tracing::info!(
            organization_id = %organization_id,
            customer_id = %customer_id,
            "Created billing portal session"
        );

        Ok(session.url)
    }

    /// Whether an org still has an active paid Stripe subscription that will
    /// keep billing them. Used to block destructive actions (e.g. org delete)
    /// that would leave Stripe charging an unreachable customer.
    ///
    /// Returns `false` for:
    /// - Free / self-hosted (Community + CommercialSelfHosted) plans (no Stripe subscription)
    /// - Pending-cancellation, paused, or cancelled status
    /// - Orgs with no subscription history at all
    pub async fn has_active_paid_subscription(&self, organization_id: Uuid) -> Result<bool, Error> {
        let Some(org) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
        else {
            return Ok(false);
        };
        let plan = org
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        if plan.is_free() || plan.is_self_hosted() {
            return Ok(false);
        }
        Ok(matches!(
            org.base.plan_status,
            Some(PlanStatus::Active) | Some(PlanStatus::Trialing) | Some(PlanStatus::PastDue)
        ))
    }

    /// Schedule a downgrade to Free at the end of the billing cycle.
    ///
    /// Sets `cancel_at = MaxPeriodEnd` on the active subscription (Stripe
    /// computes the period-end and writes it to `cancel_at`). Stripe keeps the
    /// subscription active until the period ends, then fires `customer.subscription.deleted`
    /// which triggers auto-Free creation via `handle_subscription_deleted`.
    pub async fn schedule_downgrade(
        &self,
        organization_id: Uuid,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;
        let customer_id = organization
            .base
            .stripe_customer_id
            .ok_or_else(|| anyhow!("No Stripe customer ID"))?;

        let subs = ListSubscription::new()
            .customer(CustomerId::from(customer_id))
            .send(&self.stripe)
            .await?;

        if let Some(sub) = subs.data.iter().find(|s| {
            matches!(
                s.status,
                SubscriptionStatus::Active | SubscriptionStatus::Trialing
            )
        }) {
            let is_trialing = sub.status == SubscriptionStatus::Trialing;

            UpdateSubscription::new(&sub.id)
                .cancel_at(UpdateSubscriptionCancelAt::MaxPeriodEnd)
                .send(&self.stripe)
                .await?;

            tracing::info!(
                organization_id = %organization_id,
                subscription_id = %sub.id,
                is_trialing,
                "Scheduled downgrade to Free at period end"
            );

            if is_trialing {
                Ok("Your plan will change to Free when your trial ends.".to_string())
            } else {
                Ok("Your plan will change to Free at the end of your billing cycle.".to_string())
            }
        } else {
            Err(anyhow!("No active subscription found"))
        }
    }

    /// Preview what would change when switching to a different plan
    pub async fn preview_plan_change(
        &self,
        organization_id: Uuid,
        target_plan: BillingPlan,
    ) -> Result<ChangePlanPreview, Error> {
        let org_filter = StorableFilter::<Network>::new_from_org_id(&organization_id);
        let networks = self.network_service.get_all(org_filter.clone()).await?;
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();

        let host_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids);
        let host_count = self.host_service.get_all(host_filter).await?.len() as u64;

        let user_filter =
            StorableFilter::<crate::server::users::r#impl::base::User>::new_from_org_id(
                &organization_id,
            );
        let seat_count = self.user_service.get_all(user_filter).await?.len() as u64;

        let target_config = target_plan.config();

        let excess_hosts = target_config
            .included_hosts
            .map(|limit| host_count.saturating_sub(limit))
            .unwrap_or(0);

        let excess_networks = target_config
            .included_networks
            .map(|limit| (networks.len() as u64).saturating_sub(limit))
            .unwrap_or(0);

        let excess_seats = target_config
            .included_seats
            .map(|limit| seat_count.saturating_sub(limit))
            .unwrap_or(0);

        Ok(ChangePlanPreview {
            excess_hosts,
            excess_networks,
            excess_seats,
        })
    }

    /// Change the organization's billing plan
    ///
    /// Updates the Stripe subscription to the target plan's price.
    /// The webhook handles setting the plan in our database.
    pub async fn change_plan(
        &self,
        organization_id: Uuid,
        target_plan: BillingPlan,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow!("Organization not found"))?;

        let customer_id = organization
            .base
            .stripe_customer_id
            .clone()
            .ok_or_else(|| anyhow!("No Stripe customer ID"))?;

        let base_price = self
            .get_price_from_lookup_key(target_plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find price for target plan"))?;

        let org_subscriptions = ListSubscription::new()
            .customer(CustomerId::from(customer_id))
            .send(&self.stripe)
            .await?;

        if let Some(sub) = org_subscriptions.data.iter().find(|s| {
            matches!(
                s.status,
                SubscriptionStatus::Active | SubscriptionStatus::Trialing
            )
        }) {
            // Find the base price item to replace
            let base_item = sub
                .items
                .data
                .first()
                .ok_or_else(|| anyhow!("No subscription items found"))?;

            let proration = if sub.status == SubscriptionStatus::Trialing {
                UpdateSubscriptionProrationBehavior::None
            } else {
                UpdateSubscriptionProrationBehavior::AlwaysInvoice
            };

            UpdateSubscription::new(&sub.id)
                .items(vec![UpdateSubscriptionItems {
                    id: Some(base_item.id.to_string()),
                    price: Some(base_price.id.to_string()),
                    quantity: Some(1),
                    ..Default::default()
                }])
                .metadata([
                    ("plan".to_string(), serde_json::to_string(&target_plan)?),
                    ("organization_id".to_string(), organization_id.to_string()),
                ])
                .proration_behavior(proration)
                // Clear any pending cancellation. We standardize on `cancel_at`
                // everywhere we SET or READ scheduled-cancellation state, but
                // async-stripe-billing's UpdateSubscription has no way to send
                // `cancel_at: null` (Option::is_none is skip-serialized), so the
                // documented canonical clear `cancel_at_period_end(false)` is
                // the only SDK-supported path. Stripe interprets it as "clear
                // all scheduled cancellation state" regardless of how cancel_at
                // was originally set.
                .cancel_at_period_end(false)
                .send(&self.stripe)
                .await?;

            let is_trialing = sub.status == SubscriptionStatus::Trialing;

            tracing::info!(
                organization_id = %organization_id,
                target_plan = %target_plan.name(),
                is_trialing,
                "Plan changed via subscription update"
            );

            if is_trialing {
                Ok(format!(
                    "Plan changed to {}. Your trial continues.",
                    target_plan.name()
                ))
            } else {
                Ok(format!("Plan changed to {}", target_plan.name()))
            }
        } else {
            Err(anyhow!("No active subscription found to modify"))
        }
    }

    pub(crate) async fn get_org_from_invoice(
        &self,
        invoice: &stripe_billing::Invoice,
    ) -> Result<Option<Organization>, Error> {
        let Some(customer) = invoice.customer.as_ref() else {
            return Ok(None);
        };
        let customer_id = customer.id().to_string();
        let filter = StorableFilter::<Organization>::new_with_stripe_customer_id(&customer_id);
        self.organization_service.get_one(filter).await
    }

    pub(crate) async fn handle_invoice_payment_failed(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::debug!("No org found for invoice.payment_failed — ignoring");
            return Ok(());
        };

        // Skip for Free plan orgs — legacy $0 subscriptions may still generate invoices
        if organization.base.plan.as_ref().is_none_or(|p| p.is_free()) {
            tracing::info!(organization_id = %organization.id, "Skipping payment_failed — Free plan (legacy subscription)");
            return Ok(());
        }

        // Skip for orgs without a payment method — trial auto-cancel flow
        if !organization.base.has_payment_method {
            tracing::info!(organization_id = %organization.id, "Skipping payment_failed — no payment method (trial auto-cancel)");
            return Ok(());
        }

        tracing::info!(
            organization_id = %organization.id,
            attempt_count = invoice.attempt_count,
            "Invoice payment failed"
        );

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentFailed {
                    invoice_id: invoice
                        .id
                        .as_ref()
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                    amount_cents: invoice.amount_due,
                    plan: organization.base.plan.unwrap_or_else(get_free_plan),
                    attempt_count: invoice.attempt_count as u32,
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    pub(crate) async fn handle_invoice_payment_action_required(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::debug!("No org found for invoice.payment_action_required — ignoring");
            return Ok(());
        };

        // Skip for Free plan orgs — legacy $0 subscriptions may still generate invoices
        if organization.base.plan.as_ref().is_none_or(|p| p.is_free()) {
            tracing::info!(organization_id = %organization.id, "Skipping payment_action_required — Free plan (legacy subscription)");
            return Ok(());
        }

        // Skip for orgs without a payment method — trial auto-cancel flow
        if !organization.base.has_payment_method {
            tracing::info!(organization_id = %organization.id, "Skipping payment_action_required — no payment method (trial auto-cancel)");
            return Ok(());
        }

        tracing::info!(
            organization_id = %organization.id,
            "Invoice payment action required (3D Secure / SCA)"
        );

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentActionRequired {
                    invoice_id: invoice
                        .id
                        .as_ref()
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                    hosted_invoice_url: invoice.hosted_invoice_url.clone(),
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    /// Find the active / trialing / paused subscription for an org. Used by
    /// pause/resume/extend-trial/cancel — all operate on the org's current
    /// Stripe subscription regardless of state.
    pub(crate) async fn find_current_subscription(
        &self,
        organization: &Organization,
    ) -> Result<Subscription, Error> {
        let customer_id = organization
            .base
            .stripe_customer_id
            .clone()
            .ok_or_else(|| anyhow!("No Stripe customer ID"))?;

        let subs = ListSubscription::new()
            .customer(CustomerId::from(customer_id))
            .send(&self.stripe)
            .await?;

        subs.data
            .into_iter()
            .find(|s| {
                matches!(
                    s.status,
                    SubscriptionStatus::Active
                        | SubscriptionStatus::Trialing
                        | SubscriptionStatus::Paused
                        | SubscriptionStatus::PastDue
                )
            })
            .ok_or_else(|| anyhow!("No active subscription found"))
    }
}
