use super::{Email, EmailCategory};

/// Alerts that a daemon is unreachable and scheduled discoveries are skipped.
pub struct DaemonUnreachable<'a> {
    pub daemon_name: &'a str,
    pub network_name: &'a str,
}

impl Email for DaemonUnreachable<'_> {
    fn subject(&self) -> String {
        "Your Daemon Is Unreachable".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Daemon
    }

    fn campaign(&self) -> &'static str {
        "daemon_unreachable"
    }

    fn body_html(&self) -> String {
        DAEMON_UNREACHABLE_BODY
            .replace("{daemon_name}", self.daemon_name)
            .replace("{network_name}", self.network_name)
    }
}

const DAEMON_UNREACHABLE_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Daemon Unreachable</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your daemon <strong>{daemon_name}</strong> on <strong>{network_name}</strong> is unreachable. Scheduled discoveries targeting this daemon will be skipped until connectivity is restored.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">To resolve:</p>
                            <ol style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li>Check that the daemon host is online and the daemon process is running.</li>
                                <li>Verify network connectivity between the server and daemon (<a href="https://scanopy.net/docs/setting-up-daemons/troubleshooting/" style="color: #2563eb; text-decoration: none;">troubleshooting guide</a>).</li>
                            </ol>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Once connectivity is restored, your daemon will automatically resume and scheduled discoveries will run again.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?{utm}#daemons" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">View Daemons</a>
                        </td>
                    </tr>
"#;
