use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::YEARLY_DISCOUNT;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::billing::plans::get_free_plan;
use crate::server::billing::types::api::ChangePlanPreview;
use crate::server::billing::types::base::{BillingInvoice, BillingPlan};
use crate::server::billing::types::features::Feature;
use crate::server::hosts::r#impl::base::Host;
use crate::server::hosts::service::HostService;
use crate::server::networks::r#impl::Network;
use crate::server::networks::service::NetworkService;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::{
    BillingOperation, OnboardingOperation, OnboardingOperationDiscriminants,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::users::service::UserService;
use anyhow::Error;
use anyhow::anyhow;
use chrono::Utc;
use std::sync::Arc;
use std::sync::OnceLock;
use stripe::Client;
use stripe_billing::CancellationDetailsFeedback;
use stripe_billing::billing_portal_session::CreateBillingPortalSession;
use stripe_billing::subscription::CancelSubscription;
use stripe_billing::subscription::CreateSubscription;
use stripe_billing::subscription::CreateSubscriptionItems;
use stripe_billing::subscription::CreateSubscriptionTrialSettings;
use stripe_billing::subscription::CreateSubscriptionTrialSettingsEndBehavior;
use stripe_billing::subscription::CreateSubscriptionTrialSettingsEndBehaviorMissingPaymentMethod;
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
    CheckoutSessionPaymentMethodCollection,
};
use stripe_core::customer::CreateCustomer;
use stripe_core::customer::ListPaymentMethodsCustomer;
use stripe_core::customer::UpdateCustomer;
use stripe_core::customer::UpdateCustomerInvoiceSettings;
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
    pub user_service: Arc<UserService>,
    pub network_service: Arc<NetworkService>,
    pub host_service: Arc<HostService>,
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
    pub user_service: Arc<UserService>,
    pub network_service: Arc<NetworkService>,
    pub host_service: Arc<HostService>,
    pub event_bus: Arc<EventBus>,
}

impl BillingService {
    pub fn new(params: BillingServiceParams) -> Self {
        let BillingServiceParams {
            stripe_secret,
            webhook_secret,
            organization_service,
            user_service,
            network_service,
            host_service,
            event_bus,
        } = params;

        Self {
            stripe: Client::new(stripe_secret),
            webhook_secret,
            organization_service,
            network_service,
            host_service,
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
                            trial_period_days: None,
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
                                trial_period_days: None,
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
                                trial_period_days: None,
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
            has_non_free_plan || has_trialed
        } else {
            return Err(anyhow!(
                "Could not find an organization with id {}",
                organization_id
            ));
        };

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
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutStarted {
                    plan,
                    has_trial: plan.config().trial_days > 0,
                },
                auth_for_event,
            ))
            .await?;

        Ok(session)
    }

    /// Activate the Free plan directly without Stripe.
    /// Plan/status is now derived from the subscriptions ledger via the
    /// CheckoutCompleted billing event below.
    pub async fn activate_free_plan(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;

        // Publish PlanSelected onboarding event
        if organization.not_onboarded(&OnboardingOperationDiscriminants::PlanSelected) {
            self.event_bus
                .publish(Event::new(
                    OrgScope { organization_id },
                    OnboardingOperation::PlanSelected { plan },
                    authentication.clone(),
                ))
                .await?;
        }

        // Publish CheckoutCompleted billing event
        let plan_config = plan.config();
        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutCompleted {
                    plan,
                    included_networks: plan_config.included_networks,
                    included_seats: plan_config.included_seats,
                },
                authentication,
            ))
            .await?;

        tracing::info!(
            organization_id = %organization_id,
            plan = %plan.name(),
            "Free plan activated directly (no Stripe)"
        );

        Ok("Free plan activated!".to_string())
    }

    /// Create a trial subscription directly via the Stripe API, skipping Checkout
    pub async fn create_trial_subscription(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        // Guard: prevent trial reuse — org can only trial once.
        // `trial_end_date` is set when a trial starts and never cleared.
        let has_ever_trialed = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            .and_then(|o| o.base.trial_end_date)
            .is_some();
        if has_ever_trialed {
            return Err(anyhow!(
                "Organization {} has already used a trial",
                organization_id
            ));
        }

        let auth_for_event = authentication.clone();

        let (_, customer_id) = self
            .get_or_create_customer(organization_id, authentication)
            .await?;

        let base_price = self
            .get_price_from_lookup_key(plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find base price for selected plan"))?;

        let subscription = CreateSubscription::new(customer_id)
            .items(vec![CreateSubscriptionItems {
                price: Some(base_price.id.to_string()),
                quantity: Some(1),
                ..Default::default()
            }])
            .trial_period_days(plan.config().trial_days)
            .trial_settings(CreateSubscriptionTrialSettings::new(
                CreateSubscriptionTrialSettingsEndBehavior::new(
                    CreateSubscriptionTrialSettingsEndBehaviorMissingPaymentMethod::Cancel,
                ),
            ))
            .metadata([
                ("organization_id".to_string(), organization_id.to_string()),
                ("plan".to_string(), serde_json::to_string(&plan)?),
            ])
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        tracing::info!(
            organization_id = %organization_id,
            plan = %plan.name(),
            subscription_id = %subscription.id,
            trial_days = plan.config().trial_days,
            "Trial subscription created directly (skipped checkout)"
        );

        // Publish checkout_started event for email automation
        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutStarted {
                    plan,
                    has_trial: true,
                },
                auth_for_event,
            ))
            .await?;

        Ok(format!("Your {} trial has started!", plan.name()))
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

        let plan = organization
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        if plan.is_free() {
            // Free plan has no addons to update.
            return Ok(());
        }
        let customer_id = organization
            .base
            .stripe_customer_id
            .clone()
            .ok_or_else(|| {
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
                let sub = match event.data.object {
                    EventObject::CustomerSubscriptionDeleted(sub) => Some(sub),
                    EventObject::CustomerSubscriptionPaused(sub) => Some(sub),
                    _ => None,
                };
                if let Some(sub) = sub {
                    self.handle_subscription_deleted(sub).await?;
                }
            }
            EventType::CheckoutSessionCompleted => {
                if let EventObject::CheckoutSessionCompleted(session) = event.data.object {
                    self.handle_checkout_completed(session).await?;
                }
            }
            EventType::PaymentMethodAttached => {
                if let EventObject::PaymentMethodAttached(pm) = event.data.object
                    && let Some(customer) = pm.customer.as_ref()
                {
                    self.handle_payment_method_attached(
                        customer.id().to_string(),
                        pm.id.to_string(),
                    )
                    .await?;
                }
            }
            EventType::PaymentMethodDetached => {
                // The PaymentMethod.customer field is null after detachment —
                // extract the previous customer ID from the raw event payload.
                if let EventObject::PaymentMethodDetached(_) = event.data.object {
                    let raw: serde_json::Value = serde_json::from_str(payload)?;
                    if let Some(customer_id) = raw
                        .get("data")
                        .and_then(|d| d.get("previous_attributes"))
                        .and_then(|pa| pa.get("customer"))
                        .and_then(|c| c.as_str())
                    {
                        self.handle_payment_method_detached(customer_id.to_string())
                            .await?;
                    }
                }
            }
            EventType::InvoicePaymentFailed => {
                if let EventObject::InvoicePaymentFailed(invoice) = event.data.object {
                    self.handle_invoice_payment_failed(invoice).await?;
                }
            }
            EventType::InvoicePaymentActionRequired => {
                if let EventObject::InvoicePaymentActionRequired(invoice) = event.data.object {
                    self.handle_invoice_payment_action_required(invoice).await?;
                }
            }
            EventType::InvoicePaid => {
                if let EventObject::InvoicePaid(invoice) = event.data.object {
                    self.handle_invoice_paid(invoice).await?;
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

        let organization = match self.organization_service.get_by_id(&org_id).await? {
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

        // Snapshot pre-webhook state from the org row so we can detect
        // transitions (None plan, was-trialing, etc.) before applying
        // webhook updates.
        let prior_plan = organization
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        let prior_status_str = organization.base.plan_status.clone();
        let prior_was_free = prior_plan.is_free();
        let prior_was_trialing = prior_status_str.as_deref() == Some("trialing");

        // Pending cancellation — user keeps current plan until period ends
        if sub.cancel_at_period_end {
            if let Some(owner) = owners.first() {
                let authentication: AuthenticatedEntity = owner.clone().into();
                let planned_period_end = sub
                    .ended_at
                    .or(sub.canceled_at)
                    .or(sub.cancel_at)
                    .and_then(|t| chrono::DateTime::<Utc>::from_timestamp(t, 0))
                    .unwrap_or_else(Utc::now);
                self.event_bus
                    .publish(Event::new(
                        OrgScope {
                            organization_id: organization.id,
                        },
                        BillingOperation::CancellationInitiated {
                            reason_code: crate::server::billing::types::base::CancelReason::Other,
                            stripe_feedback: None,
                            comment: None,
                            save_offer_shown: vec![],
                            save_offer_redeemed: None,
                            planned_period_end,
                        },
                        authentication,
                    ))
                    .await?;
            }
            tracing::info!(
                organization_id = %org_id,
                "Subscription marked as pending cancellation"
            );
            return Ok(());
        }

        // First time signing up for a plan
        if let Some(owner) = owners.first()
            && (prior_plan.is_free() || prior_status_str.is_none())
            && organization.not_onboarded(&OnboardingOperationDiscriminants::PlanSelected)
        {
            let authentication: AuthenticatedEntity = owner.clone().into();
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: organization.id,
                    },
                    OnboardingOperation::PlanSelected { plan },
                    authentication,
                ))
                .await?;
        }

        // Publish billing lifecycle events for email automation
        if let Some(owner) = owners.first() {
            let authentication: AuthenticatedEntity = owner.clone().into();
            let is_trialing = sub.status == SubscriptionStatus::Trialing;

            // Checkout completed (first subscription creation, or upgrade from Free)
            if prior_status_str.is_none() || prior_was_free {
                let plan_config = plan.config();
                self.event_bus
                    .publish(Event::new(
                        OrgScope {
                            organization_id: organization.id,
                        },
                        BillingOperation::CheckoutCompleted {
                            plan,
                            included_networks: plan_config.included_networks,
                            included_seats: plan_config.included_seats,
                        },
                        authentication.clone(),
                    ))
                    .await?;

                // Trial started (if subscription is in trialing state)
                if is_trialing {
                    let trial_days = plan.config().trial_days;
                    let trial_end_dt = sub
                        .trial_end
                        .and_then(|t| chrono::DateTime::<Utc>::from_timestamp(t, 0))
                        .unwrap_or_else(Utc::now);
                    self.event_bus
                        .publish(Event::new(
                            OrgScope {
                                organization_id: organization.id,
                            },
                            BillingOperation::TrialStarted {
                                plan,
                                trial_end: trial_end_dt,
                                trial_days,
                            },
                            authentication.clone(),
                        ))
                        .await?;
                }
            }

            // Trial ended (transition from trialing to active)
            if prior_was_trialing && sub.status == SubscriptionStatus::Active {
                self.event_bus
                    .publish(Event::new(
                        OrgScope {
                            organization_id: organization.id,
                        },
                        BillingOperation::TrialEnded {
                            plan,
                            converted: true,
                        },
                        authentication,
                    ))
                    .await?;
            }
        }

        // Detect plan changes — emit PlanChanged so the ledger reflects the
        // new plan as derived state. Skip if plan hasn't changed.

        // Cancel duplicate subscriptions — when Stripe Checkout creates a new subscription
        // for an existing customer, the old subscription still exists. Clean it up.
        if let Some(customer_id) = &organization.base.stripe_customer_id {
            let all_subs = ListSubscription::new()
                .customer(CustomerId::from(customer_id.clone()))
                .send(&self.stripe)
                .await?;

            let old_subs: Vec<_> = all_subs
                .data
                .iter()
                .filter(|s| {
                    s.id != sub.id
                        && matches!(
                            s.status,
                            SubscriptionStatus::Active | SubscriptionStatus::Trialing
                        )
                })
                .collect();

            for old_sub in old_subs {
                CancelSubscription::new(&old_sub.id)
                    .send(&self.stripe)
                    .await?;
                tracing::info!(
                    old_subscription = %old_sub.id,
                    new_subscription = %sub.id,
                    "Cancelled duplicate subscription during upgrade"
                );
            }
        }

        // Publish PlanChanged event if plan type actually changed (covers upgrades, downgrades, tier switches).
        // Only emit if the prior state had a real subscription history (not the
        // synthetic Free default returned when no events exist).
        if prior_status_str.is_some()
            && prior_plan.name() != plan.name()
            && let Some(owner) = owners.first()
        {
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::PlanChanged {
                        from: prior_plan,
                        to: plan,
                        is_downgrade: plan.is_free(),
                    },
                    owner.clone().into(),
                ))
                .await?;
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
        // Skip email if subscription is already marked for cancellation (e.g., user switched to Free)
        if sub.cancel_at_period_end {
            tracing::info!(
                "Trial ending soon but subscription is pending cancellation, skipping email"
            );
            return Ok(());
        }

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
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::TrialWillEnd {
                        plan,
                        has_payment_method: organization.base.has_payment_method,
                    },
                    owner.clone().into(),
                ))
                .await?;
        }

        Ok(())
    }

    /// Handle checkout.session.completed — mark payment method as collected.
    ///
    /// Only sets has_payment_method when payment was actually collected (setup mode,
    /// or subscription mode with payment_method_collection = Always). Trial checkouts
    /// use IfRequired and don't collect payment upfront.
    async fn handle_checkout_completed(
        &self,
        session: stripe_checkout::CheckoutSession,
    ) -> Result<(), Error> {
        // Only handle setup and subscription modes (not one-time payments)
        if session.mode != CheckoutSessionMode::Setup
            && session.mode != CheckoutSessionMode::Subscription
        {
            return Ok(());
        }

        // Trial checkouts use IfRequired — no payment method is collected
        let collected_payment = session.payment_method_collection
            != Some(CheckoutSessionPaymentMethodCollection::IfRequired);

        if !collected_payment {
            tracing::debug!(
                mode = ?session.mode,
                "Checkout completed without payment collection (trial) — skipping has_payment_method"
            );
            return Ok(());
        }

        let metadata = session
            .metadata
            .as_ref()
            .ok_or_else(|| anyhow!("No metadata in checkout session"))?;
        let org_id = metadata
            .get("organization_id")
            .ok_or_else(|| anyhow!("No organization_id in checkout session metadata"))?;
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
            mode = ?session.mode,
            "Payment method confirmed via checkout"
        );

        Ok(())
    }

    async fn handle_payment_method_attached(
        &self,
        customer_id: String,
        payment_method_id: String,
    ) -> Result<(), Error> {
        let filter = StorableFilter::<Organization>::new_with_stripe_customer_id(&customer_id);
        let Some(mut organization) = self.organization_service.get_one(filter).await? else {
            tracing::debug!(
                stripe_customer_id = %customer_id,
                "No organization found for payment_method.attached — ignoring"
            );
            return Ok(());
        };

        organization.base.has_payment_method = true;
        self.organization_service
            .update(&mut organization, AuthenticatedEntity::System)
            .await?;

        // Set as default payment method for future invoices so Stripe can
        // charge it when the trial ends or the next billing cycle occurs
        let mut invoice_settings = UpdateCustomerInvoiceSettings::new();
        invoice_settings.default_payment_method = Some(payment_method_id);
        UpdateCustomer::new(CustomerId::from(customer_id))
            .invoice_settings(invoice_settings)
            .send(&self.stripe)
            .await?;

        tracing::info!(
            organization_id = %organization.id,
            "Payment method attached — has_payment_method set to true, default invoice payment method updated"
        );

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentMethodAdded,
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    async fn handle_payment_method_detached(&self, customer_id: String) -> Result<(), Error> {
        let filter = StorableFilter::<Organization>::new_with_stripe_customer_id(&customer_id);
        let Some(mut organization) = self.organization_service.get_one(filter).await? else {
            tracing::debug!(
                stripe_customer_id = %customer_id,
                "No organization found for payment_method.detached — ignoring"
            );
            return Ok(());
        };

        // Check if the customer still has any payment methods remaining
        let remaining = ListPaymentMethodsCustomer::new(CustomerId::from(customer_id.clone()))
            .send(&self.stripe)
            .await?;

        if remaining.data.is_empty() {
            organization.base.has_payment_method = false;
            self.organization_service
                .update(&mut organization, AuthenticatedEntity::System)
                .await?;

            tracing::info!(
                organization_id = %organization.id,
                "Last payment method detached — has_payment_method set to false"
            );
        } else {
            tracing::info!(
                organization_id = %organization.id,
                remaining_count = remaining.data.len(),
                "Payment method detached but customer still has others"
            );
        }

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentMethodRemoved,
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    async fn handle_subscription_deleted(&self, sub: Subscription) -> Result<(), Error> {
        let org_id = sub
            .metadata
            .get("organization_id")
            .ok_or_else(|| anyhow!("No organization_id in subscription metadata"))?;
        let org_id = Uuid::parse_str(org_id)?;

        // Guard 1: Skip auto-Free if this cancellation was triggered by an upgrade
        let is_upgrade = sub
            .metadata
            .get("cancel_reason")
            .is_some_and(|r| r == "upgrade");
        if is_upgrade {
            tracing::info!(
                organization_id = %org_id,
                subscription_id = %sub.id,
                "Subscription cancelled for upgrade — skipping auto-Free"
            );
            return Ok(());
        }

        // --- Synchronous phase: downgrade immediately, return 200 to Stripe ---

        let mut organization = self
            .organization_service
            .get_by_id(&org_id)
            .await?
            .ok_or_else(|| anyhow!("Could not find organization to update subscriptions status"))?;

        // Snapshot prior subscription state from the org row (before we
        // overwrite plan/status fields below).
        let cancelled_plan = organization
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        let was_trialing = organization.base.plan_status.as_deref() == Some("trialing");
        let customer_id = organization.base.stripe_customer_id.clone();
        let (stripe_feedback, cancel_comment) =
            extract_cancellation_details(sub.cancellation_details.as_ref());

        let free_plan = get_free_plan();
        organization.base.has_payment_method = false;
        self.organization_service
            .update(&mut organization, AuthenticatedEntity::System)
            .await?;

        tracing::info!(
            organization_id = %org_id,
            subscription_id = %sub.id,
            "Subscription canceled, downgraded to Free plan"
        );

        // --- Async phase: side effects that don't need to block the webhook response ---

        let sub_id = sub.id.to_string();
        let organization_service = Arc::clone(&self.organization_service);
        let user_service = Arc::clone(&self.user_service);
        let event_bus = Arc::clone(&self.event_bus);
        let stripe = self.stripe.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::process_subscription_deleted_side_effects(
                org_id,
                sub_id,
                customer_id,
                was_trialing,
                free_plan,
                Some(cancelled_plan),
                stripe_feedback,
                cancel_comment,
                sub.ended_at
                    .or(sub.canceled_at)
                    .or(sub.cancel_at)
                    .unwrap_or_else(|| Utc::now().timestamp()),
                organization_service,
                user_service,
                event_bus,
                stripe,
            )
            .await
            {
                tracing::error!(
                    organization_id = %org_id,
                    error = %e,
                    "Failed to process subscription deletion side effects"
                );
            }
        });

        Ok(())
    }

    /// Async side effects after subscription deletion: guard 2 (revert if needed),
    /// plan restriction enforcement, event publishing, and emails. Invite
    /// revocation runs separately via `InviteService::Subscriber<BillingOperation>`
    /// triggered by the `SubscriptionCancelled` event published below.
    #[allow(clippy::too_many_arguments)]
    async fn process_subscription_deleted_side_effects(
        org_id: Uuid,
        sub_id: String,
        customer_id: Option<String>,
        was_trialing: bool,
        free_plan: BillingPlan,
        cancelled_plan: Option<BillingPlan>,
        stripe_feedback: Option<CancellationDetailsFeedback>,
        cancel_comment: Option<String>,
        period_end_ts: i64,
        organization_service: Arc<OrganizationService>,
        user_service: Arc<UserService>,
        event_bus: Arc<EventBus>,
        stripe: stripe::Client,
    ) -> Result<(), Error> {
        // Guard 2: If org has another active subscription, revert the downgrade
        if let Some(customer_id) = &customer_id {
            let all_subs = ListSubscription::new()
                .customer(CustomerId::from(customer_id.clone()))
                .send(&stripe)
                .await?;
            if all_subs.data.iter().any(|s| {
                s.id.as_str() != sub_id
                    && matches!(
                        s.status,
                        SubscriptionStatus::Active | SubscriptionStatus::Trialing
                    )
            }) {
                // Revert: another active subscription exists, so the cancel
                // was an upgrade-side-effect. Restore has_payment_method;
                // the plan/status derivation already reflects the surviving
                // subscription via the ledger (no PlanChanged event is needed
                // because the prior CheckoutCompleted is still the latest).
                if let Some(mut organization) = organization_service.get_by_id(&org_id).await? {
                    organization.base.has_payment_method = true;
                    organization_service
                        .update(&mut organization, AuthenticatedEntity::System)
                        .await?;
                }
                tracing::info!(
                    organization_id = %org_id,
                    "Org has another active subscription — preserved previous plan derivation"
                );
                return Ok(());
            }
        }

        // Publish events and send emails. Invites get revoked downstream
        // by `InviteService::Subscriber<BillingOperation>` reacting to the
        // `SubscriptionCancelled` event we publish below.
        let owners = user_service.get_organization_owners(&org_id).await?;

        if let Some(owner) = owners.first() {
            let authentication: AuthenticatedEntity = owner.clone().into();

            let period_end =
                chrono::DateTime::<Utc>::from_timestamp(period_end_ts, 0).unwrap_or_else(Utc::now);
            event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::SubscriptionCancelled {
                        plan: cancelled_plan.unwrap_or(free_plan),
                        reason_code: None,
                        stripe_feedback,
                        comment: cancel_comment.clone(),
                        period_end,
                    },
                    authentication.clone(),
                ))
                .await?;

            if was_trialing {
                event_bus
                    .publish(Event::new(
                        OrgScope {
                            organization_id: org_id,
                        },
                        BillingOperation::TrialEnded {
                            plan: cancelled_plan.unwrap_or(free_plan),
                            converted: false,
                        },
                        authentication.clone(),
                    ))
                    .await?;
            }

            // Sync the Free plan to Brevo
            event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::PlanChanged {
                        from: cancelled_plan.unwrap_or(free_plan),
                        to: free_plan,
                        is_downgrade: true,
                    },
                    owner.clone().into(),
                ))
                .await?;
        }

        tracing::info!(
            organization_id = %org_id,
            "Subscription deletion side effects completed: invites revoked, events published"
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
            org.base.plan_status.as_deref(),
            Some("active") | Some("trialing") | Some("past_due")
        ))
    }

    /// Schedule a downgrade to Free at the end of the billing cycle.
    ///
    /// Sets `cancel_at_period_end: true` on the active subscription. Stripe keeps the
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
                .cancel_at_period_end(true)
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
                .cancel_at_period_end(false) // Clear any pending cancellation
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

    async fn get_org_from_invoice(
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

    async fn handle_invoice_payment_failed(
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
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    async fn handle_invoice_payment_action_required(
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
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    async fn handle_invoice_paid(&self, invoice: stripe_billing::Invoice) -> Result<(), Error> {
        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::debug!("No org found for invoice.paid — ignoring");
            return Ok(());
        };

        let was_past_due = organization.base.plan_status.as_deref() == Some("past_due");

        if was_past_due {
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: organization.id,
                    },
                    BillingOperation::PaymentRecovered {
                        amount_cents: invoice.amount_paid,
                    },
                    AuthenticatedEntity::System,
                ))
                .await?;
        }

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentSucceeded {
                    invoice: BillingInvoice::from(&invoice),
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }
}

fn extract_cancellation_details(
    details: Option<&stripe_billing::CancellationDetails>,
) -> (
    Option<stripe_billing::CancellationDetailsFeedback>,
    Option<String>,
) {
    let feedback = details.and_then(|d| d.feedback);
    let comment = details.and_then(|d| d.comment.clone());
    (feedback, comment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_cancellation_details_none_input() {
        assert_eq!(extract_cancellation_details(None), (None, None));
    }

    #[test]
    fn extract_cancellation_details_all_inner_none() {
        let details = stripe_billing::CancellationDetails {
            comment: None,
            feedback: None,
            reason: None,
        };
        assert_eq!(extract_cancellation_details(Some(&details)), (None, None));
    }

    #[test]
    fn extract_cancellation_details_fully_populated() {
        let details = stripe_billing::CancellationDetails {
            comment: Some("too pricey for our team".to_string()),
            feedback: Some(stripe_billing::CancellationDetailsFeedback::TooExpensive),
            reason: Some(stripe_billing::CancellationDetailsReason::CancellationRequested),
        };
        assert_eq!(
            extract_cancellation_details(Some(&details)),
            (
                Some(CancellationDetailsFeedback::TooExpensive),
                Some("too pricey for our team".to_string()),
            )
        );
    }
}
