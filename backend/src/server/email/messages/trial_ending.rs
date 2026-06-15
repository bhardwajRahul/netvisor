use super::{Email, EmailCategory};

/// Sent 3 days before a trial ends: recaps trial value and prompts the user to
/// add a payment method (or confirms upcoming billing if one is on file).
pub struct TrialEnding<'a> {
    pub has_payment: bool,
    pub plan_name: &'a str,
    pub billing_period: &'a str,
    pub base_price: &'a str,
    pub hosts_count: u64,
    pub networks_count: u64,
    pub daemons_count: u64,
    pub services_count: u64,
    pub days_into_trial: i64,
}

impl Email for TrialEnding<'_> {
    fn subject(&self) -> String {
        "Your Scanopy Trial Ends in 3 Days".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        if self.has_payment {
            "trial_ending_has_payment"
        } else {
            "trial_ending_no_payment"
        }
    }

    fn body_html(&self) -> String {
        let body = if self.has_payment {
            BODY_HAS_PAYMENT
        } else {
            BODY_NO_PAYMENT
        };
        body.replace("{plan_name}", self.plan_name)
            .replace("{billing_period}", self.billing_period)
            .replace("{base_price}", self.base_price)
            .replace("{hosts_discovered}", &self.hosts_count.to_string())
            .replace("{networks_mapped}", &self.networks_count.to_string())
            .replace("{daemons_connected}", &self.daemons_count.to_string())
            .replace("{services_identified}", &self.services_count.to_string())
            .replace("{days_into_trial}", &self.days_into_trial.to_string())
    }
}

const BODY_NO_PAYMENT: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Ends Soon</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} {billing_period} trial ends in 3 days. To keep all your features and data, add a payment method ({base_price}*) before the trial expires.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If no payment method is added, your account will be downgraded to the Free plan, which includes up to 25 hosts with manual discovery only.</p>
                            <p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately.</p>
                        </td>
                    </tr>

                    <!-- Trial Recap -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h2 style="margin: 0 0 12px 0; font-size: 18px; font-weight: 600; color: #1a1a1a;">Here's what Scanopy found during your trial</h2>
                            <table cellpadding="0" cellspacing="0" border="0" width="100%" style="border-collapse: collapse;">
                                <tr>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{hosts_discovered}</strong> hosts discovered</td>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{networks_mapped}</strong> networks mapped</td>
                                </tr>
                                <tr>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{daemons_connected}</strong> daemons connected</td>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{services_identified}</strong> services identified</td>
                                </tr>
                                <tr>
                                    <td colspan="2" style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{days_into_trial}</strong> days into your trial</td>
                                </tr>
                            </table>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Add Payment Method</a>
                        </td>
                    </tr>
"#;

const BODY_HAS_PAYMENT: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Ends Soon</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} {billing_period} trial ends in 3 days. You'll be billed {base_price}* for your {plan_name} {billing_period} plan at the end of the trial period.</p>
                            <p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately.</p>
                        </td>
                    </tr>

                    <!-- Trial Recap -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h2 style="margin: 0 0 12px 0; font-size: 18px; font-weight: 600; color: #1a1a1a;">Here's what Scanopy found during your trial</h2>
                            <table cellpadding="0" cellspacing="0" border="0" width="100%" style="border-collapse: collapse;">
                                <tr>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{hosts_discovered}</strong> hosts discovered</td>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{networks_mapped}</strong> networks mapped</td>
                                </tr>
                                <tr>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{daemons_connected}</strong> daemons connected</td>
                                    <td style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{services_identified}</strong> services identified</td>
                                </tr>
                                <tr>
                                    <td colspan="2" style="padding: 8px 0; font-size: 14px; color: #4a4a4a;"><strong>{days_into_trial}</strong> days into your trial</td>
                                </tr>
                            </table>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">View Billing</a>
                        </td>
                    </tr>
"#;
