use std::sync::Arc;

use anyhow::Result;
use email_address::EmailAddress;
use uuid::Uuid;

use super::messages::{
    CancellationInitiated, CheckoutCompleted, DaemonStandby, DaemonUnreachable, DiscoveryDigest,
    DiscoveryGuide, Email, EmailChangedOld, InstallCommand, Invite, OidcLinked, OidcUnlinked,
    OrganizationDeleted, PasswordChanged, PasswordReset, PaymentActionRequired, PaymentFailed,
    PaymentMethodAdded, PaymentMethodRemoved, PaymentRecovered, PlanChanged, PlanLimitApproaching,
    PlanLimitReached, SubscriptionCancelled, SubscriptionReactivated, TrialConverted, TrialEnding,
    TrialExpired, TrialStarted, UsageSummary, Verification,
};
use super::transport::EmailTransport;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    billing::types::base::{BillingInvoice, BillingPlan, BillingRate},
    daemons::{r#impl::base::Daemon, service::DaemonService},
    digest::payload::DiscoveryDigestPayload,
    hosts::{r#impl::base::Host, service::HostService},
    networks::{r#impl::Network, service::NetworkService},
    organizations::{r#impl::base::LimitNotificationLevel, service::OrganizationService},
    services::{r#impl::base::Service, service::ServiceService},
    shared::{services::traits::CrudService, storage::filter::StorableFilter},
    users::{r#impl::base::User, service::UserService},
};

/// Counts of entities discovered/created during the trial, plus elapsed days
/// since org creation. Populates the trial-ending email and the in-app trial
/// value recap card.
pub struct TrialRecapMetrics {
    pub hosts_count: u64,
    pub networks_count: u64,
    pub daemons_count: u64,
    pub services_count: u64,
    pub days_into_trial: i64,
}

/// Per-limit pending notification, collected during [`EmailService::check_plan_limits`]
/// and dispatched once the org owner has been resolved.
struct PendingLimitEmail {
    reached: bool,
    limit_type: &'static str,
    count: u64,
    limit: u64,
    has_overage: bool,
}

/// Email service: builds the right [`Email`] for each event and hands it to
/// the configured transport (Brevo or SMTP).
pub struct EmailService {
    transport: Box<dyn EmailTransport>,
    pub user_service: Arc<UserService>,
    pub organization_service: Arc<OrganizationService>,
    pub host_service: Arc<HostService>,
    pub network_service: Arc<NetworkService>,
    pub service_service: Arc<ServiceService>,
    pub daemon_service: Arc<DaemonService>,
    pub public_url: String,
}

impl EmailService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transport: Box<dyn EmailTransport>,
        user_service: Arc<UserService>,
        organization_service: Arc<OrganizationService>,
        host_service: Arc<HostService>,
        network_service: Arc<NetworkService>,
        service_service: Arc<ServiceService>,
        daemon_service: Arc<DaemonService>,
        public_url: String,
    ) -> Self {
        Self {
            transport,
            user_service,
            organization_service,
            host_service,
            network_service,
            service_service,
            daemon_service,
            public_url,
        }
    }

    /// Render `email` against the installed `public_url` and send it to `to`.
    async fn dispatch(&self, to: EmailAddress, email: &dyn Email) -> Result<()> {
        self.transport.send(to, email, &self.public_url).await
    }

    // ========================================================================
    // Auth emails
    // ========================================================================

    pub async fn send_password_reset(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<()> {
        self.dispatch(
            to,
            &PasswordReset {
                url: &url,
                token: &token,
            },
        )
        .await
    }

    pub async fn send_invite(
        &self,
        to: EmailAddress,
        from: EmailAddress,
        url: String,
    ) -> Result<()> {
        self.dispatch(
            to,
            &Invite {
                url: &url,
                inviter: from.as_str(),
            },
        )
        .await
    }

    pub async fn send_verification_email(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<()> {
        self.dispatch(
            to,
            &Verification {
                url: &url,
                token: &token,
            },
        )
        .await
    }

    pub async fn send_password_changed_email(
        &self,
        to: EmailAddress,
        timestamp: &str,
    ) -> Result<()> {
        self.dispatch(to, &PasswordChanged { timestamp }).await
    }

    pub async fn send_oidc_linked_email(
        &self,
        to: EmailAddress,
        provider_name: &str,
    ) -> Result<()> {
        self.dispatch(to, &OidcLinked { provider_name }).await
    }

    pub async fn send_oidc_unlinked_email(
        &self,
        to: EmailAddress,
        provider_name: &str,
    ) -> Result<()> {
        self.dispatch(to, &OidcUnlinked { provider_name }).await
    }

    pub async fn send_email_changed_old_email(
        &self,
        to: EmailAddress,
        new_email: EmailAddress,
    ) -> Result<()> {
        self.dispatch(
            to,
            &EmailChangedOld {
                new_email: new_email.as_str(),
            },
        )
        .await
    }

    // ========================================================================
    // Billing emails
    // ========================================================================

    pub async fn send_trial_started_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        trial_days: u32,
        billing_period: &str,
        base_price: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &TrialStarted {
                plan_name,
                trial_days,
                billing_period,
                base_price,
            },
        )
        .await
    }

    pub async fn send_trial_ending_email(
        &self,
        to: EmailAddress,
        org_id: Uuid,
        plan_name: &str,
        has_payment: bool,
        billing_period: &str,
        base_price: &str,
    ) -> Result<()> {
        let metrics = self.compute_trial_recap_metrics(org_id).await?;
        self.dispatch(
            to,
            &TrialEnding {
                has_payment,
                plan_name,
                billing_period,
                base_price,
                hosts_count: metrics.hosts_count,
                networks_count: metrics.networks_count,
                daemons_count: metrics.daemons_count,
                services_count: metrics.services_count,
                days_into_trial: metrics.days_into_trial,
            },
        )
        .await
    }

    pub async fn send_trial_expired_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        billing_period: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &TrialExpired {
                plan_name,
                billing_period,
            },
        )
        .await
    }

    pub async fn send_trial_converted_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        billing_period: &str,
        base_price: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &TrialConverted {
                plan_name,
                billing_period,
                base_price,
            },
        )
        .await
    }

    pub async fn send_plan_changed_email(&self, to: EmailAddress, plan_name: &str) -> Result<()> {
        self.dispatch(to, &PlanChanged { plan_name }).await
    }

    pub async fn send_subscription_cancelled_email(
        &self,
        to: EmailAddress,
        period_end_date: &str,
    ) -> Result<()> {
        self.dispatch(to, &SubscriptionCancelled { period_end_date })
            .await
    }

    pub async fn send_organization_deleted_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &OrganizationDeleted).await
    }

    pub async fn send_payment_method_added_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &PaymentMethodAdded).await
    }

    pub async fn send_payment_method_removed_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &PaymentMethodRemoved).await
    }

    pub async fn send_payment_recovered_email(&self, to: EmailAddress, amount: &str) -> Result<()> {
        self.dispatch(to, &PaymentRecovered { amount }).await
    }

    pub async fn send_cancellation_initiated_email(
        &self,
        to: EmailAddress,
        period_end: &str,
    ) -> Result<()> {
        self.dispatch(to, &CancellationInitiated { period_end })
            .await
    }

    pub async fn send_subscription_reactivated_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &SubscriptionReactivated).await
    }

    pub async fn send_checkout_completed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
    ) -> Result<()> {
        self.dispatch(to, &CheckoutCompleted { plan_name }).await
    }

    pub async fn send_payment_failed_email(&self, to: EmailAddress) -> Result<()> {
        self.dispatch(to, &PaymentFailed).await
    }

    pub async fn send_payment_action_required_email(
        &self,
        to: EmailAddress,
        hosted_invoice_url: Option<String>,
    ) -> Result<()> {
        let cta_href = hosted_invoice_url
            .unwrap_or_else(|| format!("{}/?modal=settings&tab=billing", self.public_url));
        self.dispatch(
            to,
            &PaymentActionRequired {
                cta_href: &cta_href,
            },
        )
        .await
    }

    pub async fn send_usage_summary_email(
        &self,
        to: EmailAddress,
        invoice: &BillingInvoice,
    ) -> Result<()> {
        // Use line item period (actual service dates), not invoice-level period
        // (which is when items were added to the invoice)
        let period = invoice
            .line_items
            .first()
            .map(|item| format_invoice_period(item.period_start, item.period_end))
            .unwrap_or_else(|| format_invoice_period(invoice.period_start, invoice.period_end));
        let invoice_date = format_timestamp(invoice.created_at);
        let currency_str = invoice.currency.clone();

        let mut line_items_html = String::new();
        for item in &invoice.line_items {
            let description = item.description.as_deref().unwrap_or("Subscription");
            let amount = format_cents(item.amount_cents, &currency_str);
            line_items_html.push_str(&format!(
                r#"<tr><td style="padding: 8px 0; border-bottom: 1px solid #e5e7eb; font-size: 14px; color: #4a4a4a;">{}</td><td style="padding: 8px 0; border-bottom: 1px solid #e5e7eb; font-size: 14px; color: #4a4a4a; text-align: right;">{}</td></tr>"#,
                description, amount
            ));
        }

        let total = format_cents(invoice.amount_paid_cents, &currency_str);

        self.dispatch(
            to,
            &UsageSummary {
                period: &period,
                invoice_date: &invoice_date,
                line_items_html: &line_items_html,
                total: &total,
            },
        )
        .await
    }

    // ========================================================================
    // Daemon emails
    // ========================================================================

    pub async fn send_daemon_standby_email(
        &self,
        to: EmailAddress,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DaemonStandby {
                daemon_name,
                network_name,
            },
        )
        .await
    }

    pub async fn send_daemon_unreachable_email(
        &self,
        to: EmailAddress,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DaemonUnreachable {
                daemon_name,
                network_name,
            },
        )
        .await
    }

    pub async fn send_install_command_email(
        &self,
        to: EmailAddress,
        install_command: &str,
        os: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &InstallCommand {
                install_command,
                os,
            },
        )
        .await
    }

    // ========================================================================
    // Digest email
    // ========================================================================

    /// Send a per-discovery-session digest. Routed through the same transport
    /// as every other email.
    pub async fn send_discovery_digest_email(
        &self,
        to: EmailAddress,
        payload: &DiscoveryDigestPayload,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DiscoveryDigest {
                payload,
                base_url: &self.public_url,
            },
        )
        .await
    }

    // ========================================================================
    // Onboarding emails
    // ========================================================================

    /// Send discovery guide email (Free or Paid variant based on `is_free`)
    pub async fn send_discovery_guide_email(
        &self,
        to: EmailAddress,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        self.dispatch(
            to,
            &DiscoveryGuide {
                daemon_name,
                network_name,
            },
        )
        .await
    }

    /// Send discovery guide email for an organization after first daemon registration.
    /// Determines free/paid variant from org plan and looks up owner email.
    pub async fn send_discovery_guide_for_org(
        &self,
        org_id: Uuid,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        // Verify org exists
        self.organization_service
            .get_by_id(&org_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization not found"))?;

        let owner_email = self.get_owner_email(&org_id).await?;

        self.send_discovery_guide_email(owner_email, daemon_name, network_name)
            .await
    }

    // ========================================================================
    // Plan-limit checks
    // ========================================================================

    /// Check plan limits for an organization and send notification emails on threshold crossings
    pub async fn check_plan_limits(&self, org_id: Uuid, suppress_emails: bool) -> Result<()> {
        let mut org = match self.organization_service.get_by_id(&org_id).await? {
            Some(org) => org,
            None => return Ok(()),
        };

        let plan = org
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        let plan_name = plan.to_string();
        let mut notifications = org.base.plan_limit_notifications.clone();
        let mut changed = false;
        let mut emails_to_send: Vec<PendingLimitEmail> = Vec::new();

        // Check each limit type
        struct LimitCheck {
            limit: Option<u64>,
            count: u64,
            limit_type: &'static str,
            level: LimitNotificationLevel,
            has_overage: bool,
        }

        let network_filter = StorableFilter::<Network>::new_from_org_id(&org_id);
        let network_count = self.network_service.get_all(network_filter).await?.len() as u64;

        let networks = self
            .network_service
            .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
            .await?;
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();

        let host_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids);
        let host_count = self.host_service.get_all(host_filter).await?.len() as u64;

        let user_filter = StorableFilter::<User>::new_from_org_id(&org_id);
        let seat_count = self.user_service.get_all(user_filter).await?.len() as u64;

        let config = plan.config();
        let checks = vec![
            LimitCheck {
                limit: plan.host_limit(),
                count: host_count,
                limit_type: "hosts",
                level: notifications.hosts.clone(),
                has_overage: config.host_cents.is_some(),
            },
            LimitCheck {
                limit: plan.network_limit(),
                count: network_count,
                limit_type: "networks",
                level: notifications.networks.clone(),
                has_overage: config.network_cents.is_some(),
            },
            LimitCheck {
                limit: plan.seat_limit(),
                count: seat_count,
                limit_type: "seats",
                level: notifications.seats.clone(),
                has_overage: config.seat_cents.is_some(),
            },
        ];

        for check in checks {
            let limit = match check.limit {
                Some(l) if l > 1 => l,
                _ => continue, // Skip unlimited or limits <= 1 (always at capacity)
            };

            let threshold_80 = (limit as f64 * 0.8) as u64;
            let new_level = if check.count >= limit {
                if check.level != LimitNotificationLevel::Reached {
                    emails_to_send.push(PendingLimitEmail {
                        reached: true,
                        limit_type: check.limit_type,
                        count: check.count,
                        limit,
                        has_overage: check.has_overage,
                    });
                }
                LimitNotificationLevel::Reached
            } else if check.count >= threshold_80 {
                if check.level != LimitNotificationLevel::Approaching {
                    emails_to_send.push(PendingLimitEmail {
                        reached: false,
                        limit_type: check.limit_type,
                        count: check.count,
                        limit,
                        has_overage: check.has_overage,
                    });
                }
                LimitNotificationLevel::Approaching
            } else {
                LimitNotificationLevel::None
            };

            if new_level != check.level {
                changed = true;
                match check.limit_type {
                    "hosts" => notifications.hosts = new_level,
                    "networks" => notifications.networks = new_level,
                    "seats" => notifications.seats = new_level,
                    _ => {}
                }
            }
        }

        if !suppress_emails
            && !emails_to_send.is_empty()
            && let Ok(owner_email) = self.get_owner_email(&org_id).await
        {
            for pending in emails_to_send {
                let result = if pending.reached {
                    self.dispatch(
                        owner_email.clone(),
                        &PlanLimitReached {
                            first_name: None,
                            limit_type: pending.limit_type,
                            current_count: pending.count,
                            limit: pending.limit,
                            plan_name: &plan_name,
                            has_overage: pending.has_overage,
                        },
                    )
                    .await
                } else {
                    self.dispatch(
                        owner_email.clone(),
                        &PlanLimitApproaching {
                            first_name: None,
                            limit_type: pending.limit_type,
                            current_count: pending.count,
                            limit: pending.limit,
                            plan_name: &plan_name,
                            has_overage: pending.has_overage,
                        },
                    )
                    .await
                };
                if let Err(e) = result {
                    tracing::warn!(error = %e, "Failed to send plan limit email");
                }
            }
        }

        if changed {
            org.base.plan_limit_notifications = notifications;
            self.organization_service
                .update(&mut org, AuthenticatedEntity::System)
                .await?;
        }

        Ok(())
    }

    /// Counts of entities discovered/created during the trial, plus how many
    /// days have elapsed since org creation. Used to populate the trial-ending
    /// email and the in-app trial value recap card.
    async fn compute_trial_recap_metrics(&self, org_id: Uuid) -> Result<TrialRecapMetrics> {
        let org = self
            .organization_service
            .get_by_id(&org_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization not found"))?;

        let networks = self
            .network_service
            .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
            .await?;
        let networks_count = networks.len() as u64;
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();

        let hosts_count = self
            .host_service
            .get_all(StorableFilter::<Host>::new_from_network_ids(&network_ids))
            .await?
            .len() as u64;

        let services_count = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_network_ids(
                &network_ids,
            ))
            .await?
            .len() as u64;

        let daemons_count = self
            .daemon_service
            .get_all(StorableFilter::<Daemon>::new_from_network_ids(&network_ids))
            .await?
            .len() as u64;

        let days_into_trial = (chrono::Utc::now() - org.created_at).num_days();

        Ok(TrialRecapMetrics {
            hosts_count,
            networks_count,
            daemons_count,
            services_count,
            days_into_trial,
        })
    }

    /// Get the owner email for an organization
    pub async fn get_owner_email(&self, org_id: &Uuid) -> Result<EmailAddress> {
        let owners = self.user_service.get_organization_owners(org_id).await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        Ok(owner.base.email.clone())
    }
}

/// Format a plan's base price for display in emails (e.g. "$14.99/mo")
pub fn format_plan_price(plan: &BillingPlan) -> String {
    let config = plan.config();
    let amount = format_cents(config.base_cents, "usd");
    match config.rate {
        BillingRate::Month => format!("{}/mo", amount),
        BillingRate::Year => format!("{}/yr", amount),
    }
}

/// Format an amount in cents to a display string (e.g. 2999 → "$29.99")
pub fn format_cents(amount: i64, currency: &str) -> String {
    let dollars = amount as f64 / 100.0;
    match currency {
        "usd" => format!("${:.2}", dollars),
        _ => format!("{:.2} {}", dollars, currency.to_uppercase()),
    }
}

/// Format a chrono timestamp into "February 22, 2026"
fn format_timestamp(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%B %-d, %Y").to_string()
}

/// Format invoice period timestamps into a human-readable range
fn format_invoice_period(
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> String {
    format!(
        "{} – {}",
        start.format("%b %-d, %Y"),
        end.format("%b %-d, %Y")
    )
}
