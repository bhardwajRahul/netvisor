use super::{Email, EmailCategory};

/// Notifies that a daemon was put on standby after 30 days without a discovery.
pub struct DaemonStandby<'a> {
    pub daemon_name: &'a str,
    pub network_name: &'a str,
}

impl Email for DaemonStandby<'_> {
    fn subject(&self) -> String {
        "Your Daemon Has Been Put on Standby".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Daemon
    }

    fn campaign(&self) -> &'static str {
        "daemon_standby"
    }

    fn body_html(&self) -> String {
        DAEMON_STANDBY_BODY
            .replace("{daemon_name}", self.daemon_name)
            .replace("{network_name}", self.network_name)
    }
}

const DAEMON_STANDBY_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Daemon on Standby</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your daemon <strong>{daemon_name}</strong> on <strong>{network_name}</strong> has been placed on <a href="https://scanopy.net/docs/reference/daemon-status/" style="color: #2563eb; text-decoration: none;">standby</a> because it hasn't completed a discovery session in over 30 days. While on standby, scheduled discoveries targeting this daemon will be skipped.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">To resume:</p>
                            <ol style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li>Ensure your daemon is running and connected (<a href="https://scanopy.net/docs/setting-up-daemons/troubleshooting/" style="color: #2563eb; text-decoration: none;">troubleshooting guide</a>).</li>
                                <li>Manually start a discovery by pressing the Play button on the Discoveries page.</li>
                            </ol>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your daemon will come off standby and scheduled discoveries will resume automatically.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?{utm}#discovery-scans" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Queue Discovery</a>
                        </td>
                    </tr>
"#;
