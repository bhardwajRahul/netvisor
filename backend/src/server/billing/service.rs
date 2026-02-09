use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::YEARLY_DISCOUNT;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::billing::plans::get_free_plan;
use crate::server::billing::types::api::ChangePlanPreview;
use crate::server::billing::types::base::BillingPlan;
use crate::server::billing::types::features::Feature;
use crate::server::daemons::r#impl::base::DaemonMode;
use crate::server::daemons::service::DaemonService;
use crate::server::discovery::service::DiscoveryService;
use crate::server::email::traits::EmailService;
use crate::server::hosts::r#impl::base::Host;
use crate::server::hosts::service::HostService;
use crate::server::invites::service::InviteService;
use crate::server::networks::r#impl::Network;
use crate::server::networks::service::NetworkService;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::types::TelemetryEvent;
use crate::server::shared::events::types::TelemetryOperation;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::shares::service::ShareService;
use crate::server::users::service::UserService;
use anyhow::Error;
use anyhow::anyhow;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::sync::OnceLock;
use stripe::Client;
use stripe_billing::billing_portal_session::CreateBillingPortalSession;
use stripe_billing::subscription::CreateSubscription;
use stripe_billing::subscription::CreateSubscriptionItems;
use stripe_billing::subscription::ListSubscription;
use stripe_billing::subscription::UpdateSubscription;
use stripe_billing::subscription::UpdateSubscriptionItems;
use stripe_billing::subscription::UpdateSubscriptionProrationBehavior;
use stripe_billing::{Subscription, SubscriptionStatus};
use stripe_checkout::checkout_session::CreateCheckoutSessionCustomerUpdate;
use stripe_checkout::checkout_session::CreateCheckoutSessionCustomerUpdateAddress;
use stripe_checkout::checkout_session::CreateCheckoutSessionCustomerUpdateName;
use stripe_checkout::checkout_session::CreateCheckoutSessionPaymentMethodCollection;
use stripe_checkout::checkout_session::CreateCheckoutSessionSubscriptionData;
use stripe_checkout::checkout_session::{
    CreateCheckoutSession, CreateCheckoutSessionLineItems, CreateCheckoutSessionTaxIdCollection,
};
use stripe_checkout::{
    CheckoutSession, CheckoutSessionBillingAddressCollection, CheckoutSessionMode,
};
use stripe_core::customer::CreateCustomer;
use stripe_core::{CustomerId, EventType};
use stripe_product::Price;
use stripe_product::price::CreatePriceRecurring;
use stripe_product::price::SearchPrice;
use stripe_product::price::{CreatePrice, CreatePriceRecurringUsageType};
use stripe_product::product::Features;
use stripe_product::product::{CreateProduct, RetrieveProduct};
use stripe_webhook::{EventObject, Webhook};
use uuid::Uuid;
pub struct BillingService {
    pub stripe: stripe::Client,
    pub webhook_secret: String,
    pub organization_service: Arc<OrganizationService>,
    pub invite_service: Arc<InviteService>,
    pub user_service: Arc<UserService>,
    pub network_service: Arc<NetworkService>,
    pub host_service: Arc<HostService>,
    pub daemon_service: Arc<DaemonService>,
    pub discovery_service: Arc<DiscoveryService>,
    pub share_service: Arc<ShareService>,
    pub email_service: Option<Arc<EmailService>>,
    pub plans: OnceLock<Vec<BillingPlan>>,
    pub event_bus: Arc<EventBus>,
}

const SEAT_PRODUCT_ID: &str = "extra_seats";
const SEAT_PRODUCT_NAME: &str = "Extra Seats";
const NETWORK_PRODUCT_ID: &str = "extra_networks";
const NETWORK_PRODUCT_NAME: &str = "Extra Networks";

pub struct BillingServiceParams {
    pub stripe_secret: String,
    pub webhook_secret: String,
    pub organization_service: Arc<OrganizationService>,
    pub invite_service: Arc<InviteService>,
    pub user_service: Arc<UserService>,
    pub network_service: Arc<NetworkService>,
    pub host_service: Arc<HostService>,
    pub daemon_service: Arc<DaemonService>,
    pub discovery_service: Arc<DiscoveryService>,
    pub share_service: Arc<ShareService>,
    pub email_service: Option<Arc<EmailService>>,
    pub event_bus: Arc<EventBus>,
}

impl BillingService {
    pub fn new(params: BillingServiceParams) -> Self {
        let BillingServiceParams {
            stripe_secret,
            webhook_secret,
            organization_service,
            invite_service,
            user_service,
            network_service,
            host_service,
            daemon_service,
            discovery_service,
            share_service,
            email_service,
            event_bus,
        } = params;

        Self {
            stripe: Client::new(stripe_secret),
            webhook_secret,
            organization_service,
            invite_service,
            network_service,
            host_service,
            daemon_service,
            discovery_service,
            share_service,
            email_service,
            user_service,
            plans: OnceLock::new(),
            event_bus,
        }
    }

    pub fn get_plans(&self) -> Vec<BillingPlan> {
        self.plans.get().map(|v| v.to_vec()).unwrap_or_default()
    }

    pub async fn get_organization(&self, organization_id: Uuid) -> Result<Organization, Error> {
        self.organization_service
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow!("Organization {} not found", organization_id))
    }

    pub async fn get_price_from_lookup_key(
        &self,
        lookup_key: String,
    ) -> Result<Option<Price>, Error> {
        let price = SearchPrice::new(format!("lookup_key: \"{}\"", lookup_key))
            .limit(1)
            .send(&self.stripe)
            .await?
            .data
            .first()
            .cloned();

        Ok(price)
    }

    pub async fn initialize_products(&self, plans: Vec<BillingPlan>) -> Result<(), Error> {
        let mut created_plans = Vec::new();

        tracing::info!(
            plan_count = plans.len(),
            "Initializing Stripe products and prices"
        );

        // Create seat and network products
        let seat_product = match RetrieveProduct::new(SEAT_PRODUCT_ID)
            .send(&self.stripe)
            .await
        {
            Ok(p) => {
                tracing::info!("Product {} already exists", p.id);
                p
            }
            Err(_) => {
                // Create product
                let create_product = CreateProduct::new(SEAT_PRODUCT_NAME)
                    .id(SEAT_PRODUCT_ID)
                    .description("Additional seats over what's included in the base plan");

                let product = create_product.send(&self.stripe).await?;

                tracing::info!("Created product: {}", SEAT_PRODUCT_NAME);
                product
            }
        };

        let network_product = match RetrieveProduct::new(NETWORK_PRODUCT_ID)
            .send(&self.stripe)
            .await
        {
            Ok(p) => {
                tracing::info!("Product {} already exists", p.id);
                p
            }
            Err(_) => {
                // Create product
                let create_product = CreateProduct::new(NETWORK_PRODUCT_NAME)
                    .id(NETWORK_PRODUCT_ID)
                    .description("Additional networks over what's included in the base plan");

                let product = create_product.send(&self.stripe).await?;

                tracing::info!("Created product: {}", NETWORK_PRODUCT_NAME);
                product
            }
        };

        for plan in plans {
            // Skip self-hosted/contact-only plans — they don't need Stripe products
            if matches!(
                plan,
                BillingPlan::Community(_)
                    | BillingPlan::CommercialSelfHosted(_)
                    | BillingPlan::Enterprise(_)
                    | BillingPlan::Demo(_)
            ) {
                continue;
            }

            // Check if product exists, create if not
            let product_id = plan.stripe_product_id();
            let product = match RetrieveProduct::new(product_id.clone())
                .send(&self.stripe)
                .await
            {
                Ok(p) => {
                    tracing::info!("Product {} already exists", p.id);
                    p
                }
                Err(_) => {
                    let features: Vec<Feature> = plan.features().into();

                    let features: Vec<Features> =
                        features.iter().map(|f| Features::new(f.name())).collect();

                    // Create product
                    let create_product = CreateProduct::new(plan.name())
                        .id(product_id)
                        .marketing_features(features)
                        .description(plan.description());

                    let product = create_product.send(&self.stripe).await?;

                    tracing::info!("Created product: {}", plan.name());
                    product
                }
            };

            // Create base price
            match self
                .get_price_from_lookup_key(plan.stripe_base_price_lookup_key())
                .await?
            {
                Some(p) => {
                    tracing::info!("Price {} already exists", p.id);
                }
                None => {
                    // Create price
                    let create_base_price = CreatePrice::new(stripe_types::Currency::USD)
                        .lookup_key(plan.stripe_base_price_lookup_key())
                        .product(product.id.clone())
                        .unit_amount(plan.config().base_cents)
                        .recurring(CreatePriceRecurring {
                            interval: plan.config().rate.stripe_recurring_interval(),
                            interval_count: Some(1),
                            trial_period_days: Some(plan.config().trial_days),
                            meter: None,
                            usage_type: Some(CreatePriceRecurringUsageType::Licensed),
                        });

                    let price = create_base_price.send(&self.stripe).await?;

                    tracing::info!("Created price: {}", price.id);
                }
            };

            // Create seat prices
            if let (Some(seat_lookup_key), Some(seat_cents)) = (
                plan.stripe_seat_addon_price_lookup_key(),
                plan.config().seat_cents,
            ) {
                // Create seat addon price
                match self
                    .get_price_from_lookup_key(seat_lookup_key.clone())
                    .await?
                {
                    Some(p) => {
                        tracing::info!("Price {} already exists", p.id);
                    }
                    None => {
                        // Create price
                        let create_seat_price = CreatePrice::new(stripe_types::Currency::USD)
                            .lookup_key(seat_lookup_key)
                            .product(seat_product.id.clone())
                            .unit_amount(seat_cents)
                            .recurring(CreatePriceRecurring {
                                interval: plan.config().rate.stripe_recurring_interval(),
                                interval_count: Some(1),
                                trial_period_days: Some(plan.config().trial_days),
                                meter: None,
                                usage_type: Some(CreatePriceRecurringUsageType::Licensed),
                            });

                        let price = create_seat_price.send(&self.stripe).await?;

                        tracing::info!("Created price: {}", price.id);
                    }
                };
            }

            // Create network prices
            if let (Some(network_lookup_key), Some(network_cents)) = (
                plan.stripe_network_addon_price_lookup_key(),
                plan.config().network_cents,
            ) {
                // Create network addon price
                match self
                    .get_price_from_lookup_key(network_lookup_key.clone())
                    .await?
                {
                    Some(p) => {
                        tracing::info!("Price {} already exists", p.id);
                    }
                    None => {
                        // Create price
                        let create_network_price = CreatePrice::new(stripe_types::Currency::USD)
                            .lookup_key(network_lookup_key)
                            .product(network_product.id.clone())
                            .unit_amount(network_cents)
                            .recurring(CreatePriceRecurring {
                                interval: plan.config().rate.stripe_recurring_interval(),
                                interval_count: Some(1),
                                trial_period_days: Some(plan.config().trial_days),
                                meter: None,
                                usage_type: Some(CreatePriceRecurringUsageType::Licensed),
                            });

                        let price = create_network_price.send(&self.stripe).await?;

                        tracing::info!("Created price: {}", price.id);
                    }
                };
            }

            created_plans.push(plan)
        }

        created_plans.push(get_enterprise_plan());
        created_plans.push(get_enterprise_plan().to_yearly(YEARLY_DISCOUNT));

        let _ = self.plans.set(created_plans.clone());

        tracing::info!(
            initialized_plans = created_plans.len(),
            "Successfully initialized all Stripe products"
        );

        Ok(())
    }

    /// Create checkout session for upgrading
    pub async fn create_checkout_session(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        success_url: String,
        cancel_url: String,
        authentication: AuthenticatedEntity,
    ) -> Result<CheckoutSession, Error> {
        // Clone authentication for event publishing later
        let auth_for_event = authentication.clone();

        // Check if this is a returning customer (has had a non-Free paid plan or has trialed before)
        let is_returning_customer = if let Some(organization) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
        {
            let has_non_free_plan = organization
                .base
                .plan
                .as_ref()
                .is_some_and(|p| !p.is_free());
            let has_trialed = organization.base.trial_end_date.is_some();
            Ok(has_non_free_plan || has_trialed)
        } else {
            Err(anyhow!(
                "Could not find an organization with id {}",
                organization_id
            ))
        }?;

        // Get or create Stripe customer
        let (_, customer_id) = self
            .get_or_create_customer(organization_id, authentication)
            .await?;

        let base_price = self
            .get_price_from_lookup_key(plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find base price for selected plan"))?;

        // Only apply trial if plan has trial days AND customer is new (not returning)
        let trial_days = if is_returning_customer || plan.config().trial_days == 0 {
            None
        } else {
            Some(plan.config().trial_days)
        };

        // Allow trial or $0 plans without requiring credit card
        let payment_method_collection = if trial_days.is_some() || plan.config().base_cents == 0 {
            CreateCheckoutSessionPaymentMethodCollection::IfRequired
        } else {
            CreateCheckoutSessionPaymentMethodCollection::Always
        };

        let create_checkout_session = CreateCheckoutSession::new()
            .customer(customer_id)
            .success_url(success_url)
            .cancel_url(cancel_url)
            .mode(CheckoutSessionMode::Subscription)
            .payment_method_collection(payment_method_collection)
            .billing_address_collection(CheckoutSessionBillingAddressCollection::Auto)
            .customer_update(CreateCheckoutSessionCustomerUpdate {
                name: Some(CreateCheckoutSessionCustomerUpdateName::Auto),
                address: if plan.is_commercial() {
                    Some(CreateCheckoutSessionCustomerUpdateAddress::Auto)
                } else {
                    None
                },
                shipping: None,
            })
            .tax_id_collection(CreateCheckoutSessionTaxIdCollection::new(
                plan.is_commercial(),
            ))
            .line_items(vec![CreateCheckoutSessionLineItems {
                price: Some(base_price.id.to_string()),
                quantity: Some(1),
                adjustable_quantity: None,
                price_data: None,
                tax_rates: None,
                dynamic_tax_rates: None,
            }])
            .metadata([("organization_id".to_string(), organization_id.to_string())])
            .subscription_data(CreateCheckoutSessionSubscriptionData {
                trial_period_days: trial_days,
                metadata: Some(
                    [
                        ("organization_id".to_string(), organization_id.to_string()),
                        ("plan".to_string(), serde_json::to_string(&plan)?),
                    ]
                    .into(),
                ),
                ..Default::default()
            });

        let session = create_checkout_session
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        tracing::info!(
            organization_id = %organization_id,
            plan = %plan.name(),
            session_id = %session.id,
            "Checkout session created successfully"
        );

        // Publish checkout_started event for email automation
        self.event_bus
            .publish_telemetry(TelemetryEvent::new(
                Uuid::new_v4(),
                organization_id,
                TelemetryOperation::CheckoutStarted,
                Utc::now(),
                auth_for_event,
                json!({
                    "checkout_status": "pending",
                    "plan_name": plan.name(),
                    "is_commercial": plan.is_commercial(),
                    "has_trial": plan.config().trial_days > 0,
                    "org_id": organization_id.to_string(),
                }),
            ))
            .await?;

        Ok(session)
    }

    pub async fn update_addon_prices(
        &self,
        organization: Organization,
        network_count: u64,
        seat_count: u64,
    ) -> Result<(), Error> {
        tracing::info!(
            organization_id = %organization.id,
            network_count = %network_count,
            seat_count = %seat_count,
            "Updating addon prices"
        );

        let plan = organization.base.plan.ok_or_else(|| {
            anyhow!(
                "Organization {} doesn't have a billing plan",
                organization.base.name
            )
        })?;
        let customer_id = organization.base.stripe_customer_id.ok_or_else(|| {
            anyhow!(
                "Organization {} doesn't have a Stripe customer ID",
                organization.base.name
            )
        })?;

        let extra_networks = if let Some(included_networks) = plan.config().included_networks {
            network_count.saturating_sub(included_networks)
        } else {
            0
        };

        let extra_seats = if let Some(included_seats) = plan.config().included_seats {
            seat_count.saturating_sub(included_seats)
        } else {
            0
        };

        // Query all subscriptions and filter for Active or Trialing status
        // This ensures trial subscriptions are included when syncing addon quantities
        let org_subscriptions = ListSubscription::new()
            .customer(customer_id)
            .send(&self.stripe)
            .await?;

        let subscription = org_subscriptions
            .data
            .iter()
            .find(|s| {
                matches!(
                    s.status,
                    SubscriptionStatus::Active | SubscriptionStatus::Trialing
                )
            })
            .ok_or_else(|| anyhow!("No active or trialing subscription found"))?;

        // Build items array - need to update quantities on existing items
        let mut items_to_update = vec![];

        // Track what we found
        let mut found_seat_item = false;
        let mut found_network_item = false;

        // Find existing subscription items by price lookup key
        for item in &subscription.items.data {
            let price_id = &item.price.id;

            // Check if this is a seat addon item
            if let Some(seat_lookup) = plan.stripe_seat_addon_price_lookup_key()
                && let Some(seat_price) = self.get_price_from_lookup_key(seat_lookup).await?
                && price_id == &seat_price.id
            {
                found_seat_item = true;
                items_to_update.push(UpdateSubscriptionItems {
                    id: Some(item.id.to_string()),
                    price: Some(price_id.to_string()),
                    quantity: Some(extra_seats),
                    deleted: if extra_seats == 0 { Some(true) } else { None },
                    ..Default::default()
                });
                continue;
            }

            // Check if this is a network addon item
            if let Some(network_lookup) = plan.stripe_network_addon_price_lookup_key()
                && let Some(network_price) = self.get_price_from_lookup_key(network_lookup).await?
                && price_id == &network_price.id
            {
                found_network_item = true;
                items_to_update.push(UpdateSubscriptionItems {
                    id: Some(item.id.to_string()),
                    price: Some(price_id.to_string()),
                    quantity: Some(extra_networks),
                    deleted: if extra_networks == 0 {
                        Some(true)
                    } else {
                        None
                    },
                    ..Default::default()
                });
                continue;
            }
        }

        // Add new seat item if needed
        if !found_seat_item
            && extra_seats > 0
            && let Some(seat_lookup) = plan.stripe_seat_addon_price_lookup_key()
            && let Some(seat_price) = self.get_price_from_lookup_key(seat_lookup).await?
        {
            items_to_update.push(UpdateSubscriptionItems {
                price: Some(seat_price.id.to_string()),
                quantity: Some(extra_seats),
                ..Default::default()
            });
        }

        // Add new network item if needed
        if !found_network_item
            && extra_networks > 0
            && let Some(network_lookup) = plan.stripe_network_addon_price_lookup_key()
            && let Some(network_price) = self.get_price_from_lookup_key(network_lookup).await?
        {
            items_to_update.push(UpdateSubscriptionItems {
                price: Some(network_price.id.to_string()),
                quantity: Some(extra_networks),
                ..Default::default()
            });
        }

        // Update the subscription if there are changes
        if !items_to_update.is_empty() {
            UpdateSubscription::new(&subscription.id)
                .items(items_to_update)
                .proration_behavior(UpdateSubscriptionProrationBehavior::CreateProrations)
                .send(&self.stripe)
                .await?;

            tracing::info!(
                organization_id = %organization.id,
                subscription_id = %subscription.id,
                extra_seats = ?extra_seats,
                extra_networks = ?extra_networks,
                "Updated subscription addon quantities"
            );
        }

        Ok(())
    }

    /// Get existing customer or create new one
    async fn get_or_create_customer(
        &self,
        organization_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(Organization, CustomerId), Error> {
        // Check if org already has stripe_customer_id
        let mut organization = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow!("Organization {} doesn't exist.", organization_id))?;

        if let Some(customer_id) = organization.base.stripe_customer_id.clone() {
            return Ok((organization, CustomerId::from(customer_id.to_owned())));
        }

        let organization_owners = self
            .user_service
            .get_organization_owners(&organization_id)
            .await?;

        let first_owner = organization_owners
            .first()
            .ok_or_else(|| anyhow!("Organization {} doesn't have an owner.", organization_id))?;

        // Create new customer
        let create_customer = CreateCustomer::new()
            .metadata([("organization_id".to_string(), organization_id.to_string())])
            .email(first_owner.base.email.clone());

        let customer = create_customer.send(&self.stripe).await?;

        tracing::info!(
            organization_id = %organization_id,
            customer_id = %customer.id,
            customer_email = %first_owner.base.email,
            "Created new Stripe customer"
        );

        organization.base.stripe_customer_id = Some(customer.id.to_string());

        self.organization_service
            .update(&mut organization, authentication)
            .await?;

        Ok((organization, customer.id))
    }

    /// Handle webhook events
    pub async fn handle_webhook(&self, payload: &str, signature: &str) -> Result<(), Error> {
        let event = Webhook::construct_event(payload, signature, &self.webhook_secret)?;

        tracing::debug!(
            event_type = ?event.type_,
            event_id = %event.id,
            "Received Stripe webhook"
        );

        match event.type_ {
            EventType::CustomerSubscriptionCreated | EventType::CustomerSubscriptionUpdated => {
                let sub = match event.data.object {
                    EventObject::CustomerSubscriptionCreated(sub) => Some(sub),
                    EventObject::CustomerSubscriptionUpdated(sub) => Some(sub),
                    _ => None,
                };

                if let Some(sub) = sub {
                    self.handle_subscription_update(sub).await?;
                }
            }
            EventType::CustomerSubscriptionTrialWillEnd => {
                if let EventObject::CustomerSubscriptionTrialWillEnd(sub) = event.data.object {
                    self.handle_trial_will_end(sub).await?;
                }
            }
            EventType::CustomerSubscriptionPaused | EventType::CustomerSubscriptionDeleted => {
                if let EventObject::CustomerSubscriptionDeleted(sub) = event.data.object {
                    self.handle_subscription_deleted(sub).await?;
                }
            }
            EventType::CheckoutSessionCompleted => {
                if let EventObject::CheckoutSessionCompleted(session) = event.data.object {
                    self.handle_checkout_completed(session).await?;
                }
            }
            _ => {
                tracing::debug!(
                    event_type = ?event.type_,
                    "Unhandled webhook event type"
                );
            }
        }

        Ok(())
    }

    async fn handle_subscription_update(&self, sub: Subscription) -> Result<(), Error> {
        tracing::debug!(
            subscription_id = %sub.id,
            subscription_status = ?sub.status,
            metadata = ?sub.metadata,
            "Processing subscription update"
        );

        let org_id = sub
            .metadata
            .get("organization_id")
            .ok_or_else(|| anyhow!("No organization_id in subscription metadata"))?;

        let plan_str = sub
            .metadata
            .get("plan")
            .ok_or_else(|| anyhow!("No plan in subscription metadata"))?;

        let plan: BillingPlan = serde_json::from_str(plan_str)?;

        tracing::info!(
            organization_id = %org_id,
            plan = %plan.name(),
            subscription_status = ?sub.status,
            subscription_id = %sub.id,
            "Subscription updated"
        );

        let org_id = Uuid::parse_str(org_id)?;

        let mut organization = match self.organization_service.get_by_id(&org_id).await? {
            Some(org) => org,
            None => {
                // Organization was deleted - acknowledge webhook to stop retries
                tracing::warn!(
                    stripe_customer_id = %sub.customer.id(),
                    "Received subscription update for deleted organization, ignoring"
                );
                return Ok(());
            }
        };

        let owners = self
            .user_service
            .get_organization_owners(&organization.id)
            .await?;

        // First time signing up for a plan
        if let Some(owner) = owners.first()
            && organization.base.plan.is_none()
            && organization.not_onboarded(&TelemetryOperation::CommercialPlanSelected)
            && organization.not_onboarded(&TelemetryOperation::PersonalPlanSelected)
        {
            let authentication: AuthenticatedEntity = owner.clone().into();
            self.event_bus
                .publish_telemetry(TelemetryEvent {
                    id: Uuid::new_v4(),
                    organization_id: organization.id,
                    operation: TelemetryOperation::PlanSelected,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({
                        "is_onboarding_step": true,
                        "plan": plan.to_string(),
                        "is_commercial": plan.is_commercial()
                    }),

                    authentication,
                })
                .await?;
        }

        // Publish billing lifecycle events for email automation
        if let Some(owner) = owners.first() {
            let authentication: AuthenticatedEntity = owner.clone().into();
            let is_trialing = sub.status == SubscriptionStatus::Trialing;
            let trial_end_date = sub.trial_end.map(|t| t.to_string());

            // Checkout completed (first subscription creation)
            if organization.base.plan.is_none() {
                let plan_config = plan.config();
                self.event_bus
                    .publish_telemetry(TelemetryEvent::new(
                        Uuid::new_v4(),
                        organization.id,
                        TelemetryOperation::CheckoutCompleted,
                        Utc::now(),
                        authentication.clone(),
                        json!({
                            "checkout_status": "completed",
                            "plan_name": plan.name(),
                            "is_commercial": plan.is_commercial(),
                            "has_trial": is_trialing,
                            "org_id": organization.id.to_string(),
                            "included_networks": plan_config.included_networks,
                            "included_seats": plan_config.included_seats,
                        }),
                    ))
                    .await?;

                // Trial started (if subscription is in trialing state)
                if is_trialing {
                    let trial_days = plan.config().trial_days;
                    self.event_bus
                        .publish_telemetry(TelemetryEvent::new(
                            Uuid::new_v4(),
                            organization.id,
                            TelemetryOperation::TrialStarted,
                            Utc::now(),
                            authentication.clone(),
                            json!({
                                "trial_status": "active",
                                "plan_name": plan.name(),
                                "is_commercial": plan.is_commercial(),
                                "trial_end_date": trial_end_date,
                                "trial_days": trial_days,
                                "org_id": organization.id.to_string(),
                            }),
                        ))
                        .await?;

                    if let Some(ref email_service) = self.email_service
                        && let Err(e) = email_service
                            .send_trial_started_email(
                                owner.base.email.clone(),
                                plan.name(),
                                trial_days,
                            )
                            .await
                    {
                        tracing::warn!(error = %e, "Failed to send trial_started email");
                    }
                }
            }

            // Trial ended (transition from trialing to active or canceled)
            if let Some(old_status) = &organization.base.plan_status {
                let was_trialing = old_status == "trialing";
                let now_active = sub.status == SubscriptionStatus::Active;
                if was_trialing && now_active {
                    self.event_bus
                        .publish_telemetry(TelemetryEvent::new(
                            Uuid::new_v4(),
                            organization.id,
                            TelemetryOperation::TrialEnded,
                            Utc::now(),
                            authentication,
                            json!({
                                "trial_status": "ended",
                                "converted": true,
                                "plan_name": plan.name(),
                                "org_id": organization.id.to_string(),
                            }),
                        ))
                        .await?;
                }
            }
        }

        // Detect plan changes for notification (capture old plan before overwriting)
        let old_plan_name = organization
            .base
            .plan
            .as_ref()
            .map(|p| p.name().to_string());

        // Enforce all plan restrictions (network/host trimming, daemon standby, discovery conversion)
        // Note: We don't delete shares when embeds feature is removed —
        // embed access is gated at the handler level instead.
        self.enforce_plan_restrictions(&org_id, &plan).await?;

        organization.base.plan = Some(plan);

        // Free plan has no trial — always active, but preserve trial_end_date
        // to prevent trial abuse (is_returning_customer check uses trial_end_date)
        if plan.is_free() {
            organization.base.plan_status = Some("active".to_string());
        } else {
            organization.base.plan_status = Some(sub.status.to_string());
            organization.base.trial_end_date = sub
                .trial_end
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        }

        // Sync payment method status from Stripe (source of truth)
        organization.base.has_payment_method = sub.default_payment_method.is_some();

        self.organization_service
            .update(&mut organization, AuthenticatedEntity::System)
            .await?;

        // Publish PlanChanged event if plan type actually changed (covers upgrades, downgrades, tier switches)
        if let Some(ref old_name) = old_plan_name {
            let new_name = plan.name();
            if old_name != new_name
                && let Some(owner) = owners.first()
            {
                self.event_bus
                    .publish_telemetry(TelemetryEvent::new(
                        Uuid::new_v4(),
                        org_id,
                        TelemetryOperation::PlanChanged,
                        Utc::now(),
                        owner.clone().into(),
                        json!({
                            "old_plan": old_name,
                            "new_plan": new_name,
                            "is_downgrade": plan.is_free(),
                            "org_id": org_id.to_string(),
                        }),
                    ))
                    .await?;

                if let Some(ref email_service) = self.email_service
                    && let Err(e) = email_service
                        .send_plan_changed_email(owner.base.email.clone(), new_name)
                        .await
                {
                    tracing::warn!(error = %e, "Failed to send plan_changed email");
                }
            }
        }

        tracing::info!(
            "Updated organization {} subscription status to {}",
            org_id,
            sub.status
        );
        Ok(())
    }

    /// Handle trial_will_end webhook (3 days before trial expiry)
    async fn handle_trial_will_end(&self, sub: Subscription) -> Result<(), Error> {
        let org_id = sub
            .metadata
            .get("organization_id")
            .ok_or_else(|| anyhow!("No organization_id in subscription metadata"))?;
        let org_id = Uuid::parse_str(org_id)?;

        let plan_str = sub
            .metadata
            .get("plan")
            .ok_or_else(|| anyhow!("No plan in subscription metadata"))?;
        let plan: BillingPlan = serde_json::from_str(plan_str)?;

        let organization = self
            .organization_service
            .get_by_id(&org_id)
            .await?
            .ok_or_else(|| anyhow!("Organization not found for trial_will_end"))?;

        tracing::info!(
            organization_id = %org_id,
            has_payment_method = organization.base.has_payment_method,
            "Trial ending soon"
        );

        // Publish TrialWillEnd event for email automation
        let owners = self
            .user_service
            .get_organization_owners(&organization.id)
            .await?;

        if let Some(owner) = owners.first() {
            self.event_bus
                .publish_telemetry(TelemetryEvent::new(
                    Uuid::new_v4(),
                    org_id,
                    TelemetryOperation::TrialWillEnd,
                    Utc::now(),
                    owner.clone().into(),
                    json!({
                        "trial_status": "ending_soon",
                        "plan_name": plan.name(),
                        "org_id": org_id.to_string(),
                        "has_payment_method": organization.base.has_payment_method,
                    }),
                ))
                .await?;

            if let Some(ref email_service) = self.email_service
                && let Err(e) = email_service
                    .send_trial_ending_email(owner.base.email.clone(), plan.name())
                    .await
            {
                tracing::warn!(error = %e, "Failed to send trial_ending email");
            }
        }

        Ok(())
    }

    /// Handle checkout.session.completed — detect setup mode to mark payment method added
    async fn handle_checkout_completed(
        &self,
        session: stripe_checkout::CheckoutSession,
    ) -> Result<(), Error> {
        // Only handle setup mode completions (payment method collection)
        if session.mode != CheckoutSessionMode::Setup {
            return Ok(());
        }

        let metadata = session
            .metadata
            .as_ref()
            .ok_or_else(|| anyhow!("No metadata in setup session"))?;
        let org_id = metadata
            .get("organization_id")
            .ok_or_else(|| anyhow!("No organization_id in setup session metadata"))?;
        let org_id = Uuid::parse_str(org_id)?;

        let mut organization = self
            .organization_service
            .get_by_id(&org_id)
            .await?
            .ok_or_else(|| anyhow!("Organization not found for checkout completed"))?;

        organization.base.has_payment_method = true;

        self.organization_service
            .update(&mut organization, AuthenticatedEntity::System)
            .await?;

        tracing::info!(
            organization_id = %org_id,
            "Payment method added via setup checkout"
        );

        Ok(())
    }

    async fn handle_subscription_deleted(&self, sub: Subscription) -> Result<(), Error> {
        let org_id = sub
            .metadata
            .get("organization_id")
            .ok_or_else(|| anyhow!("No organization_id in subscription metadata"))?;
        let org_id = Uuid::parse_str(org_id)?;

        let organization = self
            .organization_service
            .get_by_id(&org_id)
            .await?
            .ok_or_else(|| anyhow!("Could not find organization to update subscriptions status"))?;

        // Publish subscription_cancelled event for email automation (before clearing plan)
        let owners = self
            .user_service
            .get_organization_owners(&organization.id)
            .await?;

        if let Some(owner) = owners.first() {
            let authentication: AuthenticatedEntity = owner.clone().into();
            let plan_name = organization
                .base
                .plan
                .as_ref()
                .map(|p| p.name().to_string())
                .unwrap_or_default();
            let was_trialing = organization
                .base
                .plan_status
                .as_ref()
                .map(|s| s == "trialing")
                .unwrap_or(false);

            self.event_bus
                .publish_telemetry(TelemetryEvent::new(
                    Uuid::new_v4(),
                    organization.id,
                    TelemetryOperation::SubscriptionCancelled,
                    Utc::now(),
                    authentication.clone(),
                    json!({
                        "subscription_status": "cancelled",
                        "plan_name": plan_name,
                        "org_id": organization.id.to_string(),
                    }),
                ))
                .await?;

            if let Some(ref email_service) = self.email_service
                && let Err(e) = email_service
                    .send_subscription_cancelled_email(owner.base.email.clone())
                    .await
            {
                tracing::warn!(error = %e, "Failed to send subscription_cancelled email");
            }

            // If trial was cancelled (not converted), send trial_ended event
            if was_trialing {
                self.event_bus
                    .publish_telemetry(TelemetryEvent::new(
                        Uuid::new_v4(),
                        organization.id,
                        TelemetryOperation::TrialEnded,
                        Utc::now(),
                        authentication,
                        json!({
                            "trial_status": "ended",
                            "converted": false,
                            "plan_name": plan_name,
                            "org_id": organization.id.to_string(),
                        }),
                    ))
                    .await?;

                if let Some(ref email_service) = self.email_service
                    && let Err(e) = email_service
                        .send_trial_expired_email(owner.base.email.clone(), &plan_name)
                        .await
                {
                    tracing::warn!(error = %e, "Failed to send trial_expired email");
                }
            }
        }

        self.invite_service
            .revoke_org_invites(&organization.id)
            .await?;

        // Create a Free subscription — the webhook will set the plan via handle_subscription_update
        let free_plan = get_free_plan();
        let free_price = self
            .get_price_from_lookup_key(free_plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find price for Free plan"))?;

        let customer_id = organization
            .base
            .stripe_customer_id
            .clone()
            .ok_or_else(|| anyhow!("No Stripe customer ID for organization"))?;

        CreateSubscription::new(customer_id)
            .items(vec![CreateSubscriptionItems {
                price: Some(free_price.id.to_string()),
                quantity: Some(1),
                ..Default::default()
            }])
            .metadata([
                ("plan".to_string(), serde_json::to_string(&free_plan)?),
                ("organization_id".to_string(), org_id.to_string()),
            ])
            .send(&self.stripe)
            .await?;

        tracing::info!(
            organization_id = %org_id,
            subscription_id = %sub.id,
            "Subscription canceled, created Free subscription, invites revoked"
        );
        Ok(())
    }

    /// Create a checkout session in setup mode to collect payment method
    pub async fn create_setup_payment_method_session(
        &self,
        organization_id: Uuid,
        success_url: String,
        cancel_url: String,
        authentication: AuthenticatedEntity,
    ) -> Result<CheckoutSession, Error> {
        let (_, customer_id) = self
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
                .proration_behavior(UpdateSubscriptionProrationBehavior::CreateProrations)
                .send(&self.stripe)
                .await?;

            tracing::info!(
                organization_id = %organization_id,
                target_plan = %target_plan.name(),
                "Plan changed via subscription update"
            );

            Ok(format!("Plan changed to {}", target_plan.name()))
        } else {
            Err(anyhow!("No active subscription found to modify"))
        }
    }

    /// Enforce all plan restrictions for an organization.
    ///
    /// Centralizes plan change side effects:
    /// 1. Trims excess networks if plan limits them without overage pricing
    /// 2. Sets DaemonPoll daemons to standby if plan doesn't support daemon_poll
    /// 3. Converts scheduled discoveries to ad-hoc if plan doesn't support scheduled_discovery
    /// 4. Trims excess hosts if plan limits them without overage pricing (preserves daemon hosts)
    async fn enforce_plan_restrictions(
        &self,
        organization_id: &Uuid,
        plan: &BillingPlan,
    ) -> Result<(), Error> {
        use crate::server::discovery::r#impl::types::RunType;
        let config = plan.config();
        let features = plan.features();

        // 1. Trim excess networks if plan limits them without overage pricing
        let org_filter = StorableFilter::<Network>::new_from_org_id(organization_id);
        let networks = self.network_service.get_all(org_filter).await?;

        if let Some(included_networks) = config.included_networks
            && config.network_cents.is_none()
        {
            let limit: usize = included_networks.try_into().unwrap_or(3);
            for network in networks.iter().skip(limit) {
                self.network_service
                    .delete(&network.id, AuthenticatedEntity::System)
                    .await?;
                tracing::info!(
                    organization_id = %organization_id,
                    network_id = %network.id,
                    "Deleted network exceeding plan limit"
                );
            }
        }

        // Recalculate network IDs (only remaining networks after trimming)
        let network_ids: Vec<Uuid> = if let Some(included_networks) = config.included_networks
            && config.network_cents.is_none()
        {
            let limit: usize = included_networks.try_into().unwrap_or(3);
            networks.iter().take(limit).map(|n| n.id).collect()
        } else {
            networks.iter().map(|n| n.id).collect()
        };

        // 2. Set standby on DaemonPoll daemons if plan doesn't support it
        let daemon_filter =
            StorableFilter::<crate::server::daemons::r#impl::base::Daemon>::new_from_network_ids(
                &network_ids,
            );
        let daemons = self.daemon_service.get_all(daemon_filter).await?;
        let daemon_host_ids: Vec<Uuid> = daemons.iter().map(|d| d.base.host_id).collect();

        if !features.daemon_poll {
            for mut daemon in daemons {
                if daemon.base.mode == DaemonMode::DaemonPoll && !daemon.base.standby {
                    daemon.base.standby = true;
                    self.daemon_service
                        .update(&mut daemon, AuthenticatedEntity::System)
                        .await?;
                    tracing::info!(
                        daemon_id = %daemon.id,
                        "Set daemon to standby (plan lacks daemon_poll)"
                    );
                }
            }
        } else {
            // Plan supports daemon_poll — clear standby on any daemons that were previously restricted
            for mut daemon in daemons {
                if daemon.base.standby {
                    daemon.base.standby = false;
                    self.daemon_service
                        .update(&mut daemon, AuthenticatedEntity::System)
                        .await?;
                    tracing::info!(
                        daemon_id = %daemon.id,
                        "Cleared daemon standby (plan supports daemon_poll)"
                    );
                }
            }
        }

        // 3. Convert scheduled discoveries to ad-hoc if plan doesn't support it
        if !features.scheduled_discovery {
            let discovery_filter = StorableFilter::<
                crate::server::discovery::r#impl::base::Discovery,
            >::new_from_network_ids(&network_ids);
            let discoveries = self.discovery_service.get_all(discovery_filter).await?;
            for mut discovery in discoveries {
                if let RunType::Scheduled { last_run, .. } = discovery.base.run_type {
                    discovery.base.run_type = RunType::AdHoc { last_run };
                    self.discovery_service
                        .update(&mut discovery, AuthenticatedEntity::System)
                        .await?;
                    tracing::info!(
                        discovery_id = %discovery.id,
                        "Converted scheduled discovery to ad-hoc (plan lacks scheduled_discovery)"
                    );
                }
            }
        }

        // 4. Trim excess hosts if plan limits them without overage pricing
        if let Some(included_hosts) = config.included_hosts
            && config.host_cents.is_none()
        {
            let host_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids);
            let mut hosts = self.host_service.get_all(host_filter).await?;
            if hosts.len() as u64 > included_hosts {
                hosts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                let mut kept = 0u64;
                let mut to_delete = Vec::new();
                for host in &hosts {
                    let is_daemon_host = daemon_host_ids.contains(&host.id);
                    if kept < included_hosts || is_daemon_host {
                        if !is_daemon_host {
                            kept += 1;
                        }
                    } else {
                        to_delete.push(host.id);
                    }
                }
                for host_id in &to_delete {
                    self.host_service
                        .delete(host_id, AuthenticatedEntity::System)
                        .await?;
                }
                if !to_delete.is_empty() {
                    tracing::info!(
                        organization_id = %organization_id,
                        deleted_hosts = to_delete.len(),
                        "Trimmed excess hosts to plan limit"
                    );
                }
            }
        }

        Ok(())
    }
}
