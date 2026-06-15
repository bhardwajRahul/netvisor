use super::{Email, EmailCategory};

/// Post-daemon-registration walkthrough; the free/paid variant differs in
/// copy and CTA based on whether the org is on the Free plan.
pub struct DiscoveryGuide<'a> {
    pub is_free: bool,
    pub first_name: Option<&'a str>,
    pub daemon_name: &'a str,
    pub network_name: &'a str,
}

impl Email for DiscoveryGuide<'_> {
    fn subject(&self) -> String {
        if self.is_free {
            "Your Daemon is Connected - Start Your First Discovery".to_string()
        } else {
            "Your Daemon is Connected - Discovery is Running".to_string()
        }
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Onboarding
    }

    fn campaign(&self) -> &'static str {
        if self.is_free {
            "discovery_guide_free"
        } else {
            "discovery_guide_paid"
        }
    }

    fn body_html(&self) -> String {
        let body = if self.is_free {
            DISCOVERY_GUIDE_FREE_BODY
        } else {
            DISCOVERY_GUIDE_PAID_BODY
        };
        body.replace("{first_name}", self.first_name.unwrap_or("there"))
            .replace("{daemon_name}", self.daemon_name)
            .replace("{network_name}", self.network_name)
    }
}

const DISCOVERY_GUIDE_FREE_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Daemon is Connected!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi {first_name},</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Great news — your daemon <strong>{daemon_name}</strong> just registered on <strong>{network_name}</strong>. Scanopy is now running an initial discovery to map out your network.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Here's what happens next:</p>
                            <ul style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li><strong>Self-report:</strong> The daemon host's own services and IP addresses are mapped automatically.</li>
                                <li><strong>Network scan:</strong> Scanopy scans your local subnets for other hosts, ports, and services.</li>
                                <li><strong>Topology:</strong> Once discovery finishes, your interactive topology map will be ready.</li>
                                <li><strong>Docker discovery:</strong> If your daemon has access to the Docker socket, it'll also discover all your containers — images, ports, networks, and labels — automatically.</li>
                            </ul>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">The first discovery runs automatically, but you'll need to trigger subsequent sessions manually. To keep your network map up to date, consider upgrading to a plan with scheduled discovery.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=billing-plan&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Explore Plans</a>
                        </td>
                    </tr>
"#;

const DISCOVERY_GUIDE_PAID_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Daemon is Connected!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi {first_name},</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Great news — your daemon <strong>{daemon_name}</strong> just registered on <strong>{network_name}</strong>. Scanopy is now running an initial discovery to map out your network.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Here's what happens next:</p>
                            <ul style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li><strong>Self-report:</strong> The daemon host's own services and IP addresses are mapped automatically.</li>
                                <li><strong>Network scan:</strong> Scanopy scans your local subnets for other hosts, ports, and services.</li>
                                <li><strong>Topology:</strong> Once discovery finishes, your interactive topology map will be ready.</li>
                                <li><strong>Scheduled discovery:</strong> Your plan includes daily scheduled discovery — your network documentation stays up to date automatically.</li>
                                <li><strong>Docker discovery:</strong> If your daemon has access to the Docker socket, it'll also discover all your containers — images, ports, networks, and labels — automatically.</li>
                            </ul>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Open Scanopy</a>
                        </td>
                    </tr>
"#;
