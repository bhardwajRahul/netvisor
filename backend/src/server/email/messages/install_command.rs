use super::{Email, EmailCategory};

/// Delivers a ready-to-run daemon install command for the recipient's OS.
pub struct InstallCommand<'a> {
    pub install_command: &'a str,
    pub os: &'a str,
}

impl Email for InstallCommand<'_> {
    fn subject(&self) -> String {
        "Your Scanopy Daemon Install Command".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Daemon
    }

    fn campaign(&self) -> &'static str {
        "install_command"
    }

    fn body_html(&self) -> String {
        INSTALL_COMMAND_BODY
            .replace("{install_command}", self.install_command)
            .replace("{os}", self.os)
    }
}

const INSTALL_COMMAND_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Install Command</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Here's the daemon install command you requested. Run it on your {os} machine to set up your Scanopy daemon.</p>
                            <div style="margin: 0 0 20px 0; padding: 16px; background-color: #1e293b; border-radius: 6px; overflow-x: auto;">
                                <pre style="margin: 0; font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; font-size: 13px; line-height: 20px; color: #e2e8f0; white-space: pre-wrap; word-break: break-all;">{install_command}</pre>
                            </div>
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">Copy and paste this command into your terminal. The daemon will download, install, and start automatically.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Open Scanopy</a>
                        </td>
                    </tr>
"#;
