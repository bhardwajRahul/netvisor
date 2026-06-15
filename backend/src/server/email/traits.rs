use std::sync::Arc;

use anyhow::{Error, Result};
use async_trait::async_trait;
use email_address::EmailAddress;
use uuid::Uuid;

use crate::server::{
    daemons::{r#impl::base::Daemon, service::DaemonService},
    email::templates::{
        CANCELLATION_INITIATED_BODY, CANCELLATION_INITIATED_TITLE, CHECKOUT_COMPLETED_BODY,
        CHECKOUT_COMPLETED_TITLE, DAEMON_STANDBY_BODY, DAEMON_STANDBY_TITLE,
        DAEMON_UNREACHABLE_BODY, DAEMON_UNREACHABLE_TITLE, DISCOVERY_GUIDE_FREE_BODY,
        DISCOVERY_GUIDE_FREE_TITLE, DISCOVERY_GUIDE_PAID_BODY, DISCOVERY_GUIDE_PAID_TITLE,
        EMAIL_CHANGED_OLD_BODY, EMAIL_CHANGED_OLD_TITLE, EMAIL_FOOTER, EMAIL_HEADER,
        EMAIL_VERIFICATION_BODY, INSTALL_COMMAND_BODY, INSTALL_COMMAND_TITLE, INVITE_LINK_BODY,
        OIDC_LINKED_BODY, OIDC_LINKED_TITLE, OIDC_UNLINKED_BODY, OIDC_UNLINKED_TITLE,
        ORGANIZATION_DELETED_BODY, ORGANIZATION_DELETED_TITLE, PASSWORD_CHANGED_BODY,
        PASSWORD_CHANGED_TITLE, PASSWORD_RESET_BODY, PAYMENT_ACTION_REQUIRED_BODY,
        PAYMENT_ACTION_REQUIRED_TITLE, PAYMENT_FAILED_BODY, PAYMENT_FAILED_TITLE,
        PAYMENT_METHOD_ADDED_BODY, PAYMENT_METHOD_ADDED_TITLE, PAYMENT_METHOD_REMOVED_BODY,
        PAYMENT_METHOD_REMOVED_TITLE, PAYMENT_RECOVERED_BODY, PAYMENT_RECOVERED_TITLE,
        PLAN_CHANGED_BODY, PLAN_CHANGED_TITLE, PLAN_LIMIT_APPROACHING_BODY,
        PLAN_LIMIT_APPROACHING_TITLE, PLAN_LIMIT_REACHED_BODY, PLAN_LIMIT_REACHED_TITLE,
        SUBSCRIPTION_CANCELLED_BODY, SUBSCRIPTION_CANCELLED_TITLE, TRIAL_CONVERTED_BODY,
        TRIAL_CONVERTED_TITLE, TRIAL_ENDING_BODY_HAS_PAYMENT, TRIAL_ENDING_BODY_NO_PAYMENT,
        TRIAL_ENDING_TITLE, TRIAL_EXPIRED_BODY, TRIAL_EXPIRED_TITLE, TRIAL_STARTED_BODY,
        TRIAL_STARTED_TITLE, USAGE_SUMMARY_BODY, USAGE_SUMMARY_TITLE,
    },
    hosts::{r#impl::base::Host, service::HostService},
    networks::{r#impl::Network, service::NetworkService},
    organizations::{r#impl::base::LimitNotificationLevel, service::OrganizationService},
    services::service::ServiceService,
    shared::{
        entities::EntityDiscriminants,
        services::traits::CrudService,
        storage::filter::StorableFilter,
        types::{Color, metadata::EntityMetadataProvider},
    },
    users::{r#impl::base::User, service::UserService},
};

/// Trait for email provider implementations
#[async_trait]
pub trait EmailProvider: Send + Sync {
    fn build_email(&self, body: String) -> String {
        let year = chrono::Utc::now().format("%Y").to_string();
        format!("{}{}{}", EMAIL_HEADER, body, EMAIL_FOOTER).replace("{current_year}", &year)
    }

    fn build_invite_title(&self, from_user: EmailAddress) -> String {
        format!("You've been invited to join {} on Scanopy", from_user)
    }

    fn build_password_reset_email(&self, url: String, token: String) -> String {
        self.build_email(PASSWORD_RESET_BODY.replace(
            "{reset_url}",
            &format!(
                "{}/reset-password?token={}",
                url.trim_end_matches('/'),
                token
            ),
        ))
    }

    fn build_invite_email(&self, url: String, from: EmailAddress) -> String {
        self.build_email(
            INVITE_LINK_BODY
                .replace("{invite_url}", &url)
                .replace("{inviter_name}", from.as_str()),
        )
    }

    fn build_verification_email(&self, url: String, token: String) -> String {
        self.build_email(EMAIL_VERIFICATION_BODY.replace(
            "{verify_url}",
            &format!("{}/verify-email?token={}", url.trim_end_matches('/'), token),
        ))
    }

    /// Send an HTML email
    async fn send_password_reset(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<(), Error>;

    /// Send an invite via email
    async fn send_invite(
        &self,
        to: EmailAddress,
        from: EmailAddress,
        url: String,
    ) -> Result<(), Error>;

    /// Send email verification link
    async fn send_verification_email(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<(), Error>;

    /// Send a billing lifecycle email
    async fn send_billing_email(
        &self,
        to: EmailAddress,
        subject: String,
        body: String,
    ) -> Result<(), Error>;

    async fn send_trial_started_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        trial_days: u32,
        billing_period: &str,
        base_price: &str,
    ) -> Result<(), Error> {
        let (subject, body) =
            self.build_trial_started_email(plan_name, trial_days, billing_period, base_price);
        self.send_billing_email(to, subject, body).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_trial_ending_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        has_payment: bool,
        billing_period: &str,
        base_price: &str,
        hosts_count: u64,
        networks_count: u64,
        daemons_count: u64,
        services_count: u64,
        days_into_trial: i64,
    ) -> Result<(), Error> {
        let (subject, body) = if has_payment {
            self.build_trial_ending_email_has_payment(
                plan_name,
                billing_period,
                base_price,
                hosts_count,
                networks_count,
                daemons_count,
                services_count,
                days_into_trial,
            )
        } else {
            self.build_trial_ending_email_no_payment(
                plan_name,
                billing_period,
                base_price,
                hosts_count,
                networks_count,
                daemons_count,
                services_count,
                days_into_trial,
            )
        };
        self.send_billing_email(to, subject, body).await
    }

    async fn send_trial_expired_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        billing_period: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_trial_expired_email(plan_name, billing_period);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_plan_changed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_plan_changed_email(plan_name);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_subscription_cancelled_email(
        &self,
        to: EmailAddress,
        period_end_date: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_subscription_cancelled_email(period_end_date);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_organization_deleted_email(&self, to: EmailAddress) -> Result<(), Error> {
        let (subject, body) = self.build_organization_deleted_email();
        self.send_billing_email(to, subject, body).await
    }

    async fn send_payment_method_added_email(&self, to: EmailAddress) -> Result<(), Error> {
        let (subject, body) = self.build_payment_method_added_email();
        self.send_billing_email(to, subject, body).await
    }

    async fn send_payment_method_removed_email(&self, to: EmailAddress) -> Result<(), Error> {
        let (subject, body) = self.build_payment_method_removed_email();
        self.send_billing_email(to, subject, body).await
    }

    async fn send_payment_recovered_email(
        &self,
        to: EmailAddress,
        amount: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_payment_recovered_email(amount);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_cancellation_initiated_email(
        &self,
        to: EmailAddress,
        period_end: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_cancellation_initiated_email(period_end);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_checkout_completed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_checkout_completed_email(plan_name);
        self.send_billing_email(to, subject, body).await
    }

    fn build_trial_started_email(
        &self,
        plan_name: &str,
        trial_days: u32,
        billing_period: &str,
        base_price: &str,
    ) -> (String, String) {
        let body = self.build_email(
            TRIAL_STARTED_BODY
                .replace("{plan_name}", plan_name)
                .replace("{trial_days}", &trial_days.to_string())
                .replace("{billing_period}", billing_period)
                .replace("{base_price}", base_price),
        );
        (TRIAL_STARTED_TITLE.to_string(), body)
    }

    #[allow(clippy::too_many_arguments)]
    fn build_trial_ending_email_no_payment(
        &self,
        plan_name: &str,
        billing_period: &str,
        base_price: &str,
        hosts_count: u64,
        networks_count: u64,
        daemons_count: u64,
        services_count: u64,
        days_into_trial: i64,
    ) -> (String, String) {
        let body = self.build_email(
            TRIAL_ENDING_BODY_NO_PAYMENT
                .replace("{plan_name}", plan_name)
                .replace("{billing_period}", billing_period)
                .replace("{base_price}", base_price)
                .replace("{hosts_discovered}", &hosts_count.to_string())
                .replace("{networks_mapped}", &networks_count.to_string())
                .replace("{daemons_connected}", &daemons_count.to_string())
                .replace("{services_identified}", &services_count.to_string())
                .replace("{days_into_trial}", &days_into_trial.to_string()),
        );
        (TRIAL_ENDING_TITLE.to_string(), body)
    }

    #[allow(clippy::too_many_arguments)]
    fn build_trial_ending_email_has_payment(
        &self,
        plan_name: &str,
        billing_period: &str,
        base_price: &str,
        hosts_count: u64,
        networks_count: u64,
        daemons_count: u64,
        services_count: u64,
        days_into_trial: i64,
    ) -> (String, String) {
        let body = self.build_email(
            TRIAL_ENDING_BODY_HAS_PAYMENT
                .replace("{plan_name}", plan_name)
                .replace("{billing_period}", billing_period)
                .replace("{base_price}", base_price)
                .replace("{hosts_discovered}", &hosts_count.to_string())
                .replace("{networks_mapped}", &networks_count.to_string())
                .replace("{daemons_connected}", &daemons_count.to_string())
                .replace("{services_identified}", &services_count.to_string())
                .replace("{days_into_trial}", &days_into_trial.to_string()),
        );
        (TRIAL_ENDING_TITLE.to_string(), body)
    }

    fn build_trial_expired_email(&self, plan_name: &str, billing_period: &str) -> (String, String) {
        let body = self.build_email(
            TRIAL_EXPIRED_BODY
                .replace("{plan_name}", plan_name)
                .replace("{billing_period}", billing_period),
        );
        (TRIAL_EXPIRED_TITLE.to_string(), body)
    }

    fn build_plan_changed_email(&self, plan_name: &str) -> (String, String) {
        let body = self.build_email(PLAN_CHANGED_BODY.replace("{plan_name}", plan_name));
        (PLAN_CHANGED_TITLE.to_string(), body)
    }

    fn build_subscription_cancelled_email(&self, period_end_date: &str) -> (String, String) {
        let body = self
            .build_email(SUBSCRIPTION_CANCELLED_BODY.replace("{period_end_date}", period_end_date));
        (SUBSCRIPTION_CANCELLED_TITLE.to_string(), body)
    }

    fn build_organization_deleted_email(&self) -> (String, String) {
        let body = self.build_email(ORGANIZATION_DELETED_BODY.to_string());
        (ORGANIZATION_DELETED_TITLE.to_string(), body)
    }

    fn build_payment_method_added_email(&self) -> (String, String) {
        let body = self.build_email(PAYMENT_METHOD_ADDED_BODY.to_string());
        (PAYMENT_METHOD_ADDED_TITLE.to_string(), body)
    }

    fn build_payment_method_removed_email(&self) -> (String, String) {
        let body = self.build_email(PAYMENT_METHOD_REMOVED_BODY.to_string());
        (PAYMENT_METHOD_REMOVED_TITLE.to_string(), body)
    }

    fn build_payment_recovered_email(&self, amount: &str) -> (String, String) {
        let body = self.build_email(PAYMENT_RECOVERED_BODY.replace("{amount}", amount));
        (PAYMENT_RECOVERED_TITLE.to_string(), body)
    }

    fn build_cancellation_initiated_email(&self, period_end: &str) -> (String, String) {
        let body =
            self.build_email(CANCELLATION_INITIATED_BODY.replace("{period_end}", period_end));
        let subject = CANCELLATION_INITIATED_TITLE.replace("{period_end}", period_end);
        (subject, body)
    }

    fn build_checkout_completed_email(&self, plan_name: &str) -> (String, String) {
        let body = self.build_email(CHECKOUT_COMPLETED_BODY.replace("{plan_name}", plan_name));
        let subject = CHECKOUT_COMPLETED_TITLE.replace("{plan_name}", plan_name);
        (subject, body)
    }

    fn build_payment_failed_email(&self) -> (String, String) {
        let body = self.build_email(PAYMENT_FAILED_BODY.to_string());
        (PAYMENT_FAILED_TITLE.to_string(), body)
    }

    fn build_payment_action_required_email(&self, cta_href: &str) -> (String, String) {
        let body = self.build_email(PAYMENT_ACTION_REQUIRED_BODY.replace("{cta_href}", cta_href));
        (PAYMENT_ACTION_REQUIRED_TITLE.to_string(), body)
    }

    fn build_daemon_standby_email(
        &self,
        daemon_name: &str,
        network_name: &str,
    ) -> (String, String) {
        let body = self.build_email(
            DAEMON_STANDBY_BODY
                .replace("{daemon_name}", daemon_name)
                .replace("{network_name}", network_name),
        );
        (DAEMON_STANDBY_TITLE.to_string(), body)
    }

    fn build_daemon_unreachable_email(
        &self,
        daemon_name: &str,
        network_name: &str,
    ) -> (String, String) {
        let body = self.build_email(
            DAEMON_UNREACHABLE_BODY
                .replace("{daemon_name}", daemon_name)
                .replace("{network_name}", network_name),
        );
        (DAEMON_UNREACHABLE_TITLE.to_string(), body)
    }

    fn build_install_command_email(&self, install_command: &str, os: &str) -> (String, String) {
        let body = self.build_email(
            INSTALL_COMMAND_BODY
                .replace("{install_command}", install_command)
                .replace("{os}", os),
        );
        (INSTALL_COMMAND_TITLE.to_string(), body)
    }

    fn build_trial_converted_email(
        &self,
        plan_name: &str,
        billing_period: &str,
        base_price: &str,
    ) -> (String, String) {
        let body = self.build_email(
            TRIAL_CONVERTED_BODY
                .replace("{plan_name}", plan_name)
                .replace("{billing_period}", billing_period)
                .replace("{base_price}", base_price),
        );
        (TRIAL_CONVERTED_TITLE.to_string(), body)
    }

    fn build_usage_summary_email(
        &self,
        period: &str,
        invoice_date: &str,
        line_items_html: &str,
        total: &str,
    ) -> (String, String) {
        let body = self.build_email(
            USAGE_SUMMARY_BODY
                .replace("{period}", period)
                .replace("{invoice_date}", invoice_date)
                .replace("{line_items_html}", line_items_html)
                .replace("{total}", total),
        );
        let subject = USAGE_SUMMARY_TITLE.replace("{period}", period);
        (subject, body)
    }

    // ========================================================================
    // Account change notification builders
    // ========================================================================

    fn build_password_changed_email(&self, timestamp: &str) -> (String, String) {
        let body = self.build_email(PASSWORD_CHANGED_BODY.replace("{timestamp}", timestamp));
        (PASSWORD_CHANGED_TITLE.to_string(), body)
    }

    fn build_oidc_linked_email(&self, provider_name: &str) -> (String, String) {
        let body = self.build_email(OIDC_LINKED_BODY.replace("{provider_name}", provider_name));
        let subject = OIDC_LINKED_TITLE.replace("{provider_name}", provider_name);
        (subject, body)
    }

    fn build_oidc_unlinked_email(&self, provider_name: &str) -> (String, String) {
        let body = self.build_email(OIDC_UNLINKED_BODY.replace("{provider_name}", provider_name));
        let subject = OIDC_UNLINKED_TITLE.replace("{provider_name}", provider_name);
        (subject, body)
    }

    fn build_email_changed_old_email(&self, new_email: &str) -> (String, String) {
        let body = self.build_email(EMAIL_CHANGED_OLD_BODY.replace("{new_email}", new_email));
        (EMAIL_CHANGED_OLD_TITLE.to_string(), body)
    }

    async fn send_password_changed_email(
        &self,
        to: EmailAddress,
        timestamp: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_password_changed_email(timestamp);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_oidc_linked_email(
        &self,
        to: EmailAddress,
        provider_name: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_oidc_linked_email(provider_name);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_oidc_unlinked_email(
        &self,
        to: EmailAddress,
        provider_name: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_oidc_unlinked_email(provider_name);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_email_changed_old_email(
        &self,
        to: EmailAddress,
        new_email: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_email_changed_old_email(new_email);
        self.send_billing_email(to, subject, body).await
    }

    // ========================================================================
    // Onboarding email builders
    // ========================================================================

    fn build_discovery_guide_free_email(
        &self,
        first_name: Option<&str>,
        daemon_name: &str,
        network_name: &str,
    ) -> (String, String) {
        let body = self.build_email(
            DISCOVERY_GUIDE_FREE_BODY
                .replace("{first_name}", first_name.unwrap_or("there"))
                .replace("{daemon_name}", daemon_name)
                .replace("{network_name}", network_name),
        );
        (DISCOVERY_GUIDE_FREE_TITLE.to_string(), body)
    }

    fn build_discovery_guide_paid_email(
        &self,
        first_name: Option<&str>,
        daemon_name: &str,
        network_name: &str,
    ) -> (String, String) {
        let body = self.build_email(
            DISCOVERY_GUIDE_PAID_BODY
                .replace("{first_name}", first_name.unwrap_or("there"))
                .replace("{daemon_name}", daemon_name)
                .replace("{network_name}", network_name),
        );
        (DISCOVERY_GUIDE_PAID_TITLE.to_string(), body)
    }

    fn build_plan_limit_approaching_email(
        &self,
        first_name: Option<&str>,
        limit_type: &str,
        current_count: u64,
        limit: u64,
        plan_name: &str,
        has_overage: bool,
    ) -> (String, String) {
        let (limit_message, cta_modal, cta_label) = if has_overage {
            (
                format!(
                    "Additional {} beyond your included amount will be billed automatically.",
                    limit_type
                ),
                "settings&tab=billing",
                "View Billing",
            )
        } else {
            (
                "Upgrade your plan to increase your limits and keep growing.".to_string(),
                "billing-plan",
                "Upgrade Plan",
            )
        };
        let body = self.build_email(
            PLAN_LIMIT_APPROACHING_BODY
                .replace("{first_name}", first_name.unwrap_or("there"))
                .replace("{limit_type}", limit_type)
                .replace("{current_count}", &current_count.to_string())
                .replace("{limit}", &limit.to_string())
                .replace("{plan_name}", plan_name)
                .replace("{limit_message}", &limit_message)
                .replace("{cta_modal}", cta_modal)
                .replace("{cta_label}", cta_label),
        );
        let subject = PLAN_LIMIT_APPROACHING_TITLE.replace("{limit_type}", limit_type);
        (subject, body)
    }

    fn build_plan_limit_reached_email(
        &self,
        first_name: Option<&str>,
        limit_type: &str,
        current_count: u64,
        limit: u64,
        plan_name: &str,
        has_overage: bool,
    ) -> (String, String) {
        let (limit_message, cta_modal, cta_label) = if has_overage {
            (
                format!(
                    "Additional {} beyond your included amount are being billed automatically.",
                    limit_type
                ),
                "settings&tab=billing",
                "View Billing",
            )
        } else {
            (
                format!(
                    "You won't be able to add new {} until you upgrade.",
                    limit_type
                ),
                "billing-plan",
                "Upgrade Plan",
            )
        };
        let body = self.build_email(
            PLAN_LIMIT_REACHED_BODY
                .replace("{first_name}", first_name.unwrap_or("there"))
                .replace("{limit_type}", limit_type)
                .replace("{current_count}", &current_count.to_string())
                .replace("{limit}", &limit.to_string())
                .replace("{plan_name}", plan_name)
                .replace("{limit_message}", &limit_message)
                .replace("{cta_modal}", cta_modal)
                .replace("{cta_label}", cta_label),
        );
        let subject = PLAN_LIMIT_REACHED_TITLE.replace("{limit_type}", limit_type);
        (subject, body)
    }

    fn build_discovery_digest_email(
        &self,
        payload: &crate::server::digest::payload::DiscoveryDigestPayload,
        public_url: &str,
    ) -> (String, String) {
        use crate::server::email::templates::{DISCOVERY_DIGEST_BODY, DISCOVERY_DIGEST_TITLE};
        let started = payload
            .started_at
            .format("%b %-d, %Y %H:%M UTC")
            .to_string();
        let finished = payload
            .finished_at
            .format("%b %-d, %Y %H:%M UTC")
            .to_string();
        let base = public_url.trim_end_matches('/');
        let settings_url = format!("{base}/settings?tab=email");

        let summary_section = render_summary_banner(payload);
        let legend_section = render_legend();
        let subnets_section = render_subnets_section(&payload.subnets_scanned, base);
        let hosts_added_section =
            render_host_cards_section("New hosts discovered", &payload.hosts_added, base);
        let hosts_vanished_section =
            render_host_cards_section("Hosts not seen this scan", &payload.hosts_vanished, base);
        let hosts_changed_section =
            render_host_cards_section("Hosts with changes", &payload.hosts_changed, base);
        let vlans_added_section = render_vlan_list_section("VLANs detected", &payload.vlans_added);
        let vlans_removed_section =
            render_vlan_list_section("VLANs no longer detected", &payload.vlans_removed);

        let body = self.build_email(
            DISCOVERY_DIGEST_BODY
                .replace("{network_name}", &html_escape(&payload.network_name))
                .replace("{started_at}", &started)
                .replace("{finished_at}", &finished)
                .replace("{settings_url}", &settings_url)
                .replace("{summary_section}", &summary_section)
                .replace("{legend_section}", legend_section)
                .replace("{subnets_section}", &subnets_section)
                .replace("{hosts_added_section}", &hosts_added_section)
                .replace("{hosts_vanished_section}", &hosts_vanished_section)
                .replace("{hosts_changed_section}", &hosts_changed_section)
                .replace("{vlans_added_section}", &vlans_added_section)
                .replace("{vlans_removed_section}", &vlans_removed_section),
        );
        let subject = format!("{}: {}", DISCOVERY_DIGEST_TITLE, payload.network_name);
        (subject, body)
    }
}

/// Inline tag bags expand at 10 items. Host-card sections expand at 5 —
/// recipients only want a peek at each host's footprint by default.
const MAX_TAG_ITEMS_INLINE: usize = 10;
const MAX_HOST_CARDS_INLINE: usize = 5;

/// Carrier for a single tag chip — entity colour stays bound to the
/// discriminant; status is encoded with glyph + strikethrough, never with
/// colour.
struct TagItem {
    label: String,
    color: Color,
    status: crate::server::digest::payload::TagStatus,
    href: Option<String>,
    /// Already-absolute URL (relative `/logos/...` paths rewritten by the
    /// caller against `public_url`).
    logo_url: Option<String>,
}

/// Render a single tag chip. Background/text colour come from the entity
/// type. Status is conveyed by the prefix glyph + (for Removed) strike-
/// through on the label. When `href` is `Some` the entire chip becomes a
/// clickable anchor that opens the corresponding modal in the app.
fn render_tag(tag: &TagItem) -> String {
    use crate::server::digest::payload::TagStatus;
    let (bg, fg) = tag.color.email_tag_hex();
    let (prefix, label_style) = match tag.status {
        TagStatus::New => ("+ ", ""),
        TagStatus::Removed => ("− ", "text-decoration: line-through;"),
        TagStatus::Unchanged => ("", ""),
    };
    let logo = tag.logo_url.as_deref().filter(|u| !u.is_empty()).map_or_else(
        String::new,
        |u| {
            format!(
                r#"<img src="{src}" alt="" width="14" height="14" style="vertical-align: middle; margin-right: 4px; border: 0;">"#,
                src = html_escape(u),
            )
        },
    );
    let chip = format!(
        r#"<span style="display: inline-block; padding: 3px 10px; margin: 2px 4px 2px 0; border-radius: 12px; background-color: {bg}; color: {fg}; font-size: 13px; line-height: 1.4; white-space: nowrap;">{prefix}{logo}<span style="{label_style}">{label}</span></span>"#,
        bg = bg,
        fg = fg,
        prefix = prefix,
        logo = logo,
        label_style = label_style,
        label = html_escape(&tag.label),
    );
    match &tag.href {
        Some(href) if !href.is_empty() => format!(
            r#"<a href="{href}" style="text-decoration: none; color: inherit;">{chip}</a>"#,
            href = html_escape(href),
            chip = chip,
        ),
        _ => chip,
    }
}

/// Wrap-inline tag bag. Up to `MAX_TAG_ITEMS_INLINE` tags render visible;
/// remaining items live inside a `<details>` whose `<summary>` reads
/// "Show N more".
fn render_tag_bag(tags: &[TagItem]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    let visible: String = tags
        .iter()
        .take(MAX_TAG_ITEMS_INLINE)
        .map(render_tag)
        .collect();
    if tags.len() <= MAX_TAG_ITEMS_INLINE {
        return visible;
    }
    let hidden: String = tags
        .iter()
        .skip(MAX_TAG_ITEMS_INLINE)
        .map(render_tag)
        .collect();
    let more = tags.len() - MAX_TAG_ITEMS_INLINE;
    format!(
        r#"{visible}<details style="margin: 4px 0 0 0;"><summary style="cursor: pointer; font-size: 12px; color: #2563eb;">Show {more} more</summary><div style="margin-top: 6px;">{hidden}</div></details>"#,
        visible = visible,
        more = more,
        hidden = hidden,
    )
}

/// Field row inside a host card: bold label + inline tag bag below it.
/// Hidden entirely when the tag bag is empty.
fn render_tag_row(label: &str, tags: &[TagItem]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    format!(
        r#"<div style="margin: 0 0 10px 0; font-size: 13px;"><div style="font-weight: 600; color: #4b5563; margin: 0 0 4px 0;">{label}</div><div>{tags}</div></div>"#,
        label = html_escape(label),
        tags = render_tag_bag(tags),
    )
}

fn render_section(heading: &str, body_html: &str) -> String {
    format!(
        r#"<h2 style="margin: 24px 0 8px 0; font-size: 16px; font-weight: 600; color: #1a1a1a;">{}</h2>{}"#,
        html_escape(heading),
        body_html,
    )
}

/// Glyph legend explaining the per-tag status encoding. No colour — colour
/// stays bound to the entity type. Placed at the top of the body just
/// below the summary banner.
fn render_legend() -> &'static str {
    r#"<div style="margin: 0 0 16px 0; padding: 8px 12px; background-color: #f9fafb; border-radius: 6px; font-size: 12px; color: #4b5563;"><span style="margin-right: 14px;"><strong>+</strong> new</span><span style="margin-right: 14px;">unchanged</span><span><strong>−</strong> <span style="text-decoration: line-through;">removed</span></span></div>"#
}

fn render_subnets_section(
    subnets: &[crate::server::digest::payload::SubnetSummary],
    base: &str,
) -> String {
    if subnets.is_empty() {
        return String::new();
    }
    let color = EntityDiscriminants::Subnet.color();
    let tags: Vec<TagItem> = subnets
        .iter()
        .map(|s| TagItem {
            label: s.label.clone(),
            color,
            status: Default::default(),
            href: Some(format!("{base}/?modal=subnet-editor&id={}", s.id)),
            logo_url: None,
        })
        .collect();
    let header = format!("Subnets scanned ({})", subnets.len());
    render_section(&header, &render_tag_bag(&tags))
}

fn render_vlan_list_section(
    heading: &str,
    vlans: &[crate::server::digest::payload::VlanSummary],
) -> String {
    if vlans.is_empty() {
        return String::new();
    }
    let color = EntityDiscriminants::Vlan.color();
    let tags: Vec<TagItem> = vlans
        .iter()
        .map(|v| {
            let label = if v.name.is_empty() {
                format!("VLAN {}", v.vlan_number)
            } else {
                format!("VLAN {} — {}", v.vlan_number, v.name)
            };
            TagItem {
                label,
                color,
                status: Default::default(),
                href: None, // VLANs aren't deep-linkable in the UI
                logo_url: None,
            }
        })
        .collect();
    let header = format!("{} ({})", heading, vlans.len());
    render_section(&header, &render_tag_bag(&tags))
}

/// Stats banner at the top of the digest body.
fn render_summary_banner(
    payload: &crate::server::digest::payload::DiscoveryDigestPayload,
) -> String {
    let cells: Vec<(usize, &str)> = vec![
        (payload.hosts_added.len(), "new hosts"),
        (payload.hosts_vanished.len(), "vanished hosts"),
        (payload.hosts_changed.len(), "changed hosts"),
        (payload.vlans_added.len(), "VLANs detected"),
        (payload.vlans_removed.len(), "VLANs no longer detected"),
        (payload.subnets_scanned.len(), "subnets scanned"),
    ];
    let inner: String = cells
        .iter()
        .map(|(count, label)| {
            format!(
                r#"<td style="padding: 8px 12px; vertical-align: top;"><div style="font-size: 22px; font-weight: 700; color: #1a1a1a; line-height: 1.2;">{}</div><div style="font-size: 12px; color: #6b7280;">{}</div></td>"#,
                count,
                html_escape(label),
            )
        })
        .collect();
    format!(
        r#"<table role="presentation" style="width: 100%; border-collapse: collapse; margin: 16px 0; background-color: #f9fafb; border-radius: 6px;"><tr>{}</tr></table>"#,
        inner,
    )
}

/// Render one section of host cards. Sections with more than 5 cards put
/// the overflow inside `<details>` so recipients can opt-in to the full
/// list. Summary reads "Show {N} more hosts".
fn render_host_cards_section(
    heading: &str,
    cards: &[crate::server::digest::payload::AffectedHostCard],
    base: &str,
) -> String {
    if cards.is_empty() {
        return String::new();
    }
    let header = format!("{} ({})", heading, cards.len());
    let visible: String = cards
        .iter()
        .take(MAX_HOST_CARDS_INLINE)
        .map(|c| render_host_card(c, base))
        .collect();
    if cards.len() <= MAX_HOST_CARDS_INLINE {
        return render_section(&header, &visible);
    }
    let hidden: String = cards
        .iter()
        .skip(MAX_HOST_CARDS_INLINE)
        .map(|c| render_host_card(c, base))
        .collect();
    let more = cards.len() - MAX_HOST_CARDS_INLINE;
    let inner = format!(
        r#"{visible}<details style="margin: 0 0 16px 0;"><summary style="cursor: pointer; font-size: 13px; color: #2563eb;">Show {more} more hosts</summary>{hidden}</details>"#,
        visible = visible,
        more = more,
        hidden = hidden,
    );
    render_section(&header, &inner)
}

fn render_host_card(card: &crate::server::digest::payload::AffectedHostCard, base: &str) -> String {
    use crate::server::digest::payload::HostCardStatus;
    let (badge_label, badge_bg, badge_fg) = match card.status {
        HostCardStatus::New => ("New", "#dcfce7", "#166534"),
        HostCardStatus::Vanished => ("Vanished", "#fee2e2", "#991b1b"),
        HostCardStatus::Changed => ("Changed", "#fef3c7", "#92400e"),
    };
    let badge = format!(
        r#"<span style="display: inline-block; padding: 2px 8px; font-size: 12px; font-weight: 600; border-radius: 999px; background-color: {bg}; color: {fg};">{label}</span>"#,
        bg = badge_bg,
        fg = badge_fg,
        label = badge_label,
    );
    let host_href = format!("{base}/?modal=host-editor&id={}", card.host.id);
    let host_link = format!(
        r#"<a href="{href}" style="text-decoration: none; color: #1a1a1a;">{label}</a>"#,
        href = html_escape(&host_href),
        label = html_escape(&card.host.label),
    );

    // Split services into the non-container set + the Containers set,
    // mirroring HostCard.svelte. The Containers row only renders when the
    // host has at least one container service.
    let (containers, services): (Vec<_>, Vec<_>) =
        card.services.iter().partition(|s| s.is_container);

    let services_tags = tags_services(&services, card.host.id, base);
    let containers_tags = tags_services(&containers, card.host.id, base);
    let ip_tags = tags_ips(&card.ip_addresses, card.host.id, base);
    let interface_tags = tags_interfaces(&card.interfaces, card.host.id, base);
    let port_tags = tags_ports(&card.ports, card.host.id, base);

    let mut rows = String::new();
    rows.push_str(&render_tag_row("Services", &services_tags));
    rows.push_str(&render_tag_row("Containers", &containers_tags));
    rows.push_str(&render_tag_row("IP Addresses", &ip_tags));
    rows.push_str(&render_tag_row("Interfaces", &interface_tags));
    rows.push_str(&render_tag_row("Ports", &port_tags));

    format!(
        r#"<div style="margin: 0 0 16px 0; padding: 14px; background-color: #ffffff; border: 1px solid #e5e7eb; border-radius: 8px;"><div style="display: flex; align-items: center; justify-content: space-between; margin: 0 0 10px 0;"><div style="font-size: 16px; font-weight: 600;">{host_link}</div>{badge}</div>{rows}</div>"#,
        host_link = host_link,
        badge = badge,
        rows = rows,
    )
}

/// Rewrite a relative `/logos/...` path to an absolute URL using the
/// installed app's `public_url`. Absolute URLs pass through.
fn absolute_logo(raw: &Option<String>, base: &str) -> Option<String> {
    let raw = raw.as_deref()?;
    if raw.is_empty() {
        return None;
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        Some(raw.to_string())
    } else if let Some(path) = raw.strip_prefix('/') {
        Some(format!("{base}/{path}"))
    } else {
        Some(raw.to_string())
    }
}

fn tags_ports(
    items: &[crate::server::digest::payload::PortSummary],
    host_id: Uuid,
    base: &str,
) -> Vec<TagItem> {
    let color = EntityDiscriminants::Port.color();
    items
        .iter()
        .map(|p| TagItem {
            label: p.label.clone(),
            color,
            status: p.status,
            href: Some(format!(
                "{base}/?modal=host-editor&id={host_id}&tab=ports&subEntityId={}",
                p.id
            )),
            logo_url: None,
        })
        .collect()
}

fn tags_services(
    items: &[&crate::server::digest::payload::ServiceSummary],
    _host_id: Uuid,
    base: &str,
) -> Vec<TagItem> {
    let color = EntityDiscriminants::Service.color();
    items
        .iter()
        .map(|s| TagItem {
            label: s.name.clone(),
            color,
            status: s.status,
            href: Some(format!("{base}/?modal=service-editor&id={}", s.id)),
            logo_url: absolute_logo(&s.logo_url, base),
        })
        .collect()
}

fn tags_ips(
    items: &[crate::server::digest::payload::IpAddressSummary],
    host_id: Uuid,
    base: &str,
) -> Vec<TagItem> {
    let color = EntityDiscriminants::IPAddress.color();
    items
        .iter()
        .map(|ip| TagItem {
            label: ip.address.clone(),
            color,
            status: ip.status,
            href: Some(format!(
                "{base}/?modal=host-editor&id={host_id}&tab=ip-addresses&subEntityId={}",
                ip.id
            )),
            logo_url: None,
        })
        .collect()
}

fn tags_interfaces(
    items: &[crate::server::digest::payload::InterfaceSummary],
    host_id: Uuid,
    base: &str,
) -> Vec<TagItem> {
    let color = EntityDiscriminants::Interface.color();
    items
        .iter()
        .map(|i| TagItem {
            label: i.label.clone(),
            color,
            status: i.status,
            href: Some(format!(
                "{base}/?modal=host-editor&id={host_id}&tab=interfaces&subEntityId={}",
                i.id
            )),
            logo_url: None,
        })
        .collect()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

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

/// Email service that wraps the provider
pub struct EmailService {
    provider: Box<dyn EmailProvider>,
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
        provider: Box<dyn EmailProvider>,
        user_service: Arc<UserService>,
        organization_service: Arc<OrganizationService>,
        host_service: Arc<HostService>,
        network_service: Arc<NetworkService>,
        service_service: Arc<ServiceService>,
        daemon_service: Arc<DaemonService>,
        public_url: String,
    ) -> Self {
        Self {
            provider,
            user_service,
            organization_service,
            host_service,
            network_service,
            service_service,
            daemon_service,
            public_url,
        }
    }

    // ========================================================================
    // Existing email methods
    // ========================================================================

    /// Send an HTML email
    pub async fn send_password_reset(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<()> {
        self.provider.send_password_reset(to, url, token).await
    }

    pub async fn send_invite(
        &self,
        to: EmailAddress,
        from: EmailAddress,
        url: String,
    ) -> Result<()> {
        self.provider.send_invite(to, from, url).await
    }

    /// Send email verification link
    pub async fn send_verification_email(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<()> {
        self.provider.send_verification_email(to, url, token).await
    }

    /// Send billing lifecycle email
    pub async fn send_billing_email(
        &self,
        to: EmailAddress,
        subject: String,
        body: String,
    ) -> Result<()> {
        self.provider.send_billing_email(to, subject, body).await
    }

    /// Send a per-discovery-session digest. Routed through the existing
    /// billing-email provider channel — same Brevo/SMTP transport with no
    /// extra wiring needed.
    pub async fn send_discovery_digest_email(
        &self,
        to: EmailAddress,
        payload: &crate::server::digest::payload::DiscoveryDigestPayload,
    ) -> Result<()> {
        let (subject, body) = self
            .provider
            .build_discovery_digest_email(payload, &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_trial_started_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        trial_days: u32,
        billing_period: &str,
        base_price: &str,
    ) -> Result<()> {
        let (subject, body) = self.provider.build_trial_started_email(
            plan_name,
            trial_days,
            billing_period,
            base_price,
        );
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    #[allow(clippy::too_many_arguments)]
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

        let (subject, body) = if has_payment {
            self.provider.build_trial_ending_email_has_payment(
                plan_name,
                billing_period,
                base_price,
                metrics.hosts_count,
                metrics.networks_count,
                metrics.daemons_count,
                metrics.services_count,
                metrics.days_into_trial,
            )
        } else {
            self.provider.build_trial_ending_email_no_payment(
                plan_name,
                billing_period,
                base_price,
                metrics.hosts_count,
                metrics.networks_count,
                metrics.daemons_count,
                metrics.services_count,
                metrics.days_into_trial,
            )
        };
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
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
            .get_all(StorableFilter::<
                crate::server::services::r#impl::base::Service,
            >::new_from_network_ids(&network_ids))
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

    pub async fn send_trial_expired_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        billing_period: &str,
    ) -> Result<()> {
        let (subject, body) = self
            .provider
            .build_trial_expired_email(plan_name, billing_period);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_plan_changed_email(&self, to: EmailAddress, plan_name: &str) -> Result<()> {
        let (subject, body) = self.provider.build_plan_changed_email(plan_name);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_subscription_cancelled_email(
        &self,
        to: EmailAddress,
        period_end_date: &str,
    ) -> Result<()> {
        let (subject, body) = self
            .provider
            .build_subscription_cancelled_email(period_end_date);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_organization_deleted_email(&self, to: EmailAddress) -> Result<()> {
        let (subject, body) = self.provider.build_organization_deleted_email();
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_payment_method_added_email(&self, to: EmailAddress) -> Result<()> {
        let (subject, body) = self.provider.build_payment_method_added_email();
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_payment_method_removed_email(&self, to: EmailAddress) -> Result<()> {
        let (subject, body) = self.provider.build_payment_method_removed_email();
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_payment_recovered_email(&self, to: EmailAddress, amount: &str) -> Result<()> {
        let (subject, body) = self.provider.build_payment_recovered_email(amount);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_cancellation_initiated_email(
        &self,
        to: EmailAddress,
        period_end: &str,
    ) -> Result<()> {
        let (subject, body) = self.provider.build_cancellation_initiated_email(period_end);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_checkout_completed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
    ) -> Result<()> {
        let (subject, body) = self.provider.build_checkout_completed_email(plan_name);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_payment_failed_email(&self, to: EmailAddress) -> Result<()> {
        let (subject, body) = self.provider.build_payment_failed_email();
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_payment_action_required_email(
        &self,
        to: EmailAddress,
        hosted_invoice_url: Option<String>,
    ) -> Result<()> {
        let cta_href = hosted_invoice_url
            .unwrap_or_else(|| format!("{}/?modal=settings&tab=billing", self.public_url));
        let (subject, body) = self.provider.build_payment_action_required_email(&cta_href);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_trial_converted_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        billing_period: &str,
        base_price: &str,
    ) -> Result<()> {
        let (subject, body) =
            self.provider
                .build_trial_converted_email(plan_name, billing_period, base_price);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_usage_summary_email(
        &self,
        to: EmailAddress,
        invoice: &crate::server::billing::types::base::BillingInvoice,
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

        let (subject, body) = self.provider.build_usage_summary_email(
            &period,
            &invoice_date,
            &line_items_html,
            &total,
        );
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_daemon_standby_email(
        &self,
        to: EmailAddress,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        let (subject, body) = self
            .provider
            .build_daemon_standby_email(daemon_name, network_name);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_daemon_unreachable_email(
        &self,
        to: EmailAddress,
        daemon_name: &str,
        network_name: &str,
    ) -> Result<()> {
        let (subject, body) = self
            .provider
            .build_daemon_unreachable_email(daemon_name, network_name);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_install_command_email(
        &self,
        to: EmailAddress,
        install_command: &str,
        os: &str,
    ) -> Result<()> {
        let (subject, body) = self
            .provider
            .build_install_command_email(install_command, os);
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
    }

    // ========================================================================
    // Account change notification methods
    // ========================================================================

    pub async fn send_password_changed_email(
        &self,
        to: EmailAddress,
        timestamp: &str,
    ) -> Result<()> {
        self.provider
            .send_password_changed_email(to, timestamp)
            .await
    }

    pub async fn send_oidc_linked_email(
        &self,
        to: EmailAddress,
        provider_name: &str,
    ) -> Result<()> {
        self.provider
            .send_oidc_linked_email(to, provider_name)
            .await
    }

    pub async fn send_oidc_unlinked_email(
        &self,
        to: EmailAddress,
        provider_name: &str,
    ) -> Result<()> {
        self.provider
            .send_oidc_unlinked_email(to, provider_name)
            .await
    }

    pub async fn send_email_changed_old_email(
        &self,
        to: EmailAddress,
        new_email: EmailAddress,
    ) -> Result<()> {
        self.provider
            .send_email_changed_old_email(to, new_email.as_str())
            .await
    }

    // ========================================================================
    // Onboarding email methods
    // ========================================================================

    /// Send discovery guide email (Free or Paid variant based on `is_free`)
    pub async fn send_discovery_guide_email(
        &self,
        to: EmailAddress,
        first_name: Option<String>,
        daemon_name: &str,
        network_name: &str,
        is_free: bool,
    ) -> Result<()> {
        let first_name_ref = first_name.as_deref();
        let (subject, body) = if is_free {
            self.provider.build_discovery_guide_free_email(
                first_name_ref,
                daemon_name,
                network_name,
            )
        } else {
            self.provider.build_discovery_guide_paid_email(
                first_name_ref,
                daemon_name,
                network_name,
            )
        };
        let body = body.replace("{base_url}", &self.public_url);
        self.provider.send_billing_email(to, subject, body).await
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
        let is_free = self
            .organization_service
            .get_by_id(&org_id)
            .await?
            .and_then(|o| o.base.plan)
            .map(|p| p.is_free())
            .unwrap_or(true);

        self.send_discovery_guide_email(owner_email, None, daemon_name, network_name, is_free)
            .await
    }

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
        let mut emails_to_send: Vec<(String, String)> = Vec::new();

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
                    let (subject, body) = self.provider.build_plan_limit_reached_email(
                        None,
                        check.limit_type,
                        check.count,
                        limit,
                        &plan_name,
                        check.has_overage,
                    );
                    emails_to_send.push((subject, body));
                }
                LimitNotificationLevel::Reached
            } else if check.count >= threshold_80 {
                if check.level != LimitNotificationLevel::Approaching {
                    let (subject, body) = self.provider.build_plan_limit_approaching_email(
                        None,
                        check.limit_type,
                        check.count,
                        limit,
                        &plan_name,
                        check.has_overage,
                    );
                    emails_to_send.push((subject, body));
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
            for (subject, body) in emails_to_send {
                let body = body.replace("{base_url}", &self.public_url);
                if let Err(e) = self
                    .provider
                    .send_billing_email(owner_email.clone(), subject, body)
                    .await
                {
                    tracing::warn!(error = %e, "Failed to send plan limit email");
                }
            }
        }

        if changed {
            org.base.plan_limit_notifications = notifications;
            self.organization_service
                .update(
                    &mut org,
                    crate::server::auth::middleware::auth::AuthenticatedEntity::System,
                )
                .await?;
        }

        Ok(())
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

use crate::server::billing::types::base::{BillingPlan, BillingRate};

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

/// Strip HTML tags for plain text fallback
pub fn strip_html_tags(html: String) -> String {
    html2text::from_read(html.as_bytes(), 80).unwrap_or_else(|_| html.to_string())
}
