use super::{Email, EmailCategory};

/// Sent when a trial begins: welcomes the user and points them at adding a
/// payment method before the trial ends.
pub struct TrialStarted<'a> {
    pub plan_name: &'a str,
    pub trial_days: u32,
    pub billing_period: &'a str,
    pub base_price: &'a str,
}

impl Email for TrialStarted<'_> {
    fn subject(&self) -> String {
        "Welcome to Scanopy! Your Trial Has Started".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        "trial_started"
    }

    fn body_html(&self) -> String {
        BODY.replace("{plan_name}", self.plan_name)
            .replace("{trial_days}", &self.trial_days.to_string())
            .replace("{billing_period}", self.billing_period)
            .replace("{base_price}", self.base_price)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Welcome to Scanopy {plan_name} {billing_period}!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your trial of the {plan_name} {billing_period} plan has started. You have full access to all features for the next {trial_days} days.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">After your trial, you'll be billed {base_price}*. No credit card is required during the trial — add a payment method anytime from your Settings page to continue after the trial ends.</p>
                            <p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Add Payment Method</a>
                        </td>
                    </tr>
"#;
