use super::{Body, Content, Email, EmailCategory, EmailPreference, PausableCategory};

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

    fn preference(&self) -> EmailPreference {
        EmailPreference::Pausable(PausableCategory::DaemonAlerts)
    }

    fn campaign(&self) -> &'static str {
        "daemon_unreachable"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Daemon Unreachable")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your daemon <strong>{}</strong> on <strong>{}</strong> is unreachable. Scheduled discoveries targeting this daemon will be skipped until connectivity is restored.",
                        self.daemon_name, self.network_name
                    ))
                    .paragraph("To resolve:")
                    .raw(
r#"                            <ol style="margin: 0 0 20px 0; padding-left: 20px; font-size: 16px; line-height: 28px; color: #4a4a4a;">
                                <li>Check that the daemon host is online and the daemon process is running.</li>
                                <li>Verify network connectivity between the server and daemon (<a href="https://scanopy.net/docs/setting-up-daemons/troubleshooting/" style="color: #2563eb; text-decoration: none;">troubleshooting guide</a>).</li>
                            </ol>
"#,
                    )
                    .paragraph("Once connectivity is restored, your daemon will automatically resume and scheduled discoveries will run again."),
            )
            .cta("{base_url}/?{utm}#daemons", "View Daemons")
            .render()
    }
}
