use super::{Email, EmailCategory};

/// Post-daemon-registration walkthrough; the free/paid variant differs in
/// copy and CTA based on whether the org is on the Free plan.
pub struct DiscoveryGuide<'a> {
    pub daemon_name: &'a str,
    pub network_name: &'a str,
}

impl Email for DiscoveryGuide<'_> {
    fn subject(&self) -> String {
        "Your Daemon is Connected - Discovery is Running".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Onboarding
    }

    fn campaign(&self) -> &'static str {
        "discovery_guide"
    }

    fn body_html(&self) -> String {
        DISCOVERY_GUIDE_BODY
            .replace("{daemon_name}", self.daemon_name)
            .replace("{network_name}", self.network_name)
            .to_string()
    }
}

const DISCOVERY_GUIDE_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Daemon is Connected!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Great news — your daemon <strong>{daemon_name}</strong> just registered on <strong>{network_name}</strong>. Scanopy is now running an initial discovery to map out your network.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Here's what happens next:</p>
                            <ul style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li><strong>Self-report:</strong> The daemon host's own services and IP addresses are mapped automatically.</li>
                                <li><strong>Network scan:</strong> Scanopy scans your local subnets for other hosts, ports, and services.</li>
                                <li><strong>Topology:</strong> Once discovery finishes, your interactive topology map will be ready.</li>
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
