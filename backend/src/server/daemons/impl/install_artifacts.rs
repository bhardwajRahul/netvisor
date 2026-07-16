//! Server-side assembly of daemon install artifacts.
//!
//! The server is the single source of truth for the install command (it knows its own
//! public URL, the release-asset locations, and the canonical two-flag format), so the
//! provision response hands back ready-to-run commands and a download link instead of
//! the UI/MCP/email each re-deriving them. Commands that embed the api key are built here
//! at provision time (the plaintext is only available then, and never stored for
//! DaemonPoll); the MSI link carries no secret and is re-derivable later.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::base::{Daemon, DaemonMode};

/// The `install.sh` one-liner that fetches + runs the Unix installer bootstrap.
const UNIX_INSTALL_SCRIPT: &str = "bash -c \"$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)\"";
/// Windows daemon exe (matches the UI's hardcoded release URL).
const WINDOWS_EXE_URL: &str =
    "https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe";
/// The signed Windows MSI release asset. The installer-download endpoint streams this,
/// renamed with the per-tenant [`encode_msi_filename`] name.
pub const WINDOWS_MSI_URL: &str =
    "https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.msi";

/// One ready-to-paste install command for a platform.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformInstallCommand {
    /// Platform key matching the UI's OS selector: `linux` | `macos` | `windows` | `freebsd`.
    pub platform: String,
    /// The full command, including fetching the binary.
    pub command: String,
}

/// Everything the UI needs to install a just-provisioned daemon.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstallArtifacts {
    /// Per-platform binary install commands (the api key is embedded — shown once).
    pub commands: Vec<PlatformInstallCommand>,
    /// Direct URL of the signed Windows MSI release artifact (a GitHub asset).
    pub msi_url: String,
    /// Filename encoding this daemon's non-secret values (mode/name/url). Save or rename
    /// the downloaded MSI to this name to pre-fill the installer — parse-filename.js decodes
    /// it. The api key is never encoded. Renaming a signed MSI doesn't affect its signature.
    pub msi_filename: String,
}

/// Hex-encode a value for a filename segment (see `backend/wix/parse-filename.js`).
fn hex(value: &str) -> String {
    hex::encode(value.as_bytes())
}

/// Build the MSI download filename with the daemon's non-secret values encoded. The api
/// key is deliberately absent — a live credential must never sit in a filename.
pub fn encode_msi_filename(daemon: &Daemon) -> String {
    let mode = match daemon.base.mode {
        DaemonMode::ServerPoll => "server_poll",
        DaemonMode::DaemonPoll => "daemon_poll",
    };
    let mut segments = vec![
        format!("mode={}", hex(mode)),
        format!("name={}", hex(&daemon.base.name)),
    ];
    // ServerPoll is dialed by the server at its reachable URL; DaemonPoll dials out, so
    // its url is unused and not encoded.
    if daemon.base.mode == DaemonMode::ServerPoll && !daemon.base.url.is_empty() {
        segments.push(format!("url={}", hex(&daemon.base.url)));
    }
    format!("scanopy-daemon~~{}.msi", segments.join("~~"))
}

/// The `install` flags for the canonical two-flag form: DaemonPoll dials the server (needs
/// `--server-url`), ServerPoll is dialed by the server (no `--server-url`). Name and network
/// both come from the provisioned record (the daemon learns its name via the handshake), so
/// the command carries neither — just the server url (DaemonPoll) and the key.
fn install_flags(public_url: &str, daemon: &Daemon, api_key: &str) -> String {
    match daemon.base.mode {
        DaemonMode::DaemonPoll => {
            format!("--server-url {public_url} --daemon-api-key {api_key}")
        }
        DaemonMode::ServerPoll => format!("--daemon-api-key {api_key}"),
    }
}

/// Assemble the install artifacts for a freshly provisioned daemon.
pub fn build_install_artifacts(
    public_url: &str,
    daemon: &Daemon,
    api_key: &str,
) -> InstallArtifacts {
    let public_url = public_url.trim_end_matches('/');
    let flags = install_flags(public_url, daemon, api_key);

    // Unix binary platforms share the fetch-script + `install` shape.
    let unix = format!("{UNIX_INSTALL_SCRIPT} && sudo scanopy-daemon install {flags}");
    let windows = format!(
        "Invoke-WebRequest -Uri \"{WINDOWS_EXE_URL}\" -OutFile \"scanopy-daemon-windows-amd64.exe\"; .\\scanopy-daemon-windows-amd64.exe install {flags}"
    );

    let commands = vec![
        PlatformInstallCommand {
            platform: "linux".to_string(),
            command: unix.clone(),
        },
        PlatformInstallCommand {
            platform: "macos".to_string(),
            command: unix.clone(),
        },
        PlatformInstallCommand {
            platform: "freebsd".to_string(),
            command: unix,
        },
        PlatformInstallCommand {
            platform: "windows".to_string(),
            command: windows,
        },
    ];

    InstallArtifacts {
        commands,
        msi_url: WINDOWS_MSI_URL.to_string(),
        msi_filename: encode_msi_filename(daemon),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::daemons::r#impl::base::DaemonBase;
    use crate::server::shared::storage::traits::Storable;

    fn daemon(mode: DaemonMode, url: &str) -> Daemon {
        Daemon::new(DaemonBase {
            host_id: uuid::Uuid::new_v4(),
            network_id: uuid::Uuid::new_v4(),
            url: url.to_string(),
            last_seen: None,
            mode,
            name: "edge-01".to_string(),
            tags: Vec::new(),
            version: None,
            user_id: uuid::Uuid::new_v4(),
            api_key_id: None,
            is_unreachable: false,
            standby: false,
            standby_cleared_at: None,
        })
    }

    #[test]
    fn daemon_poll_command_dials_the_server_serverpoll_does_not() {
        let dp = build_install_artifacts(
            "https://app.scanopy.net/",
            &daemon(DaemonMode::DaemonPoll, ""),
            "sk_test",
        );
        let linux = dp.commands.iter().find(|c| c.platform == "linux").unwrap();
        assert!(
            linux
                .command
                .contains("--server-url https://app.scanopy.net")
        );
        assert!(linux.command.contains("--daemon-api-key sk_test"));

        let sp = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::ServerPoll, "https://edge.corp:60073"),
            "sk_test",
        );
        let sp_linux = sp.commands.iter().find(|c| c.platform == "linux").unwrap();
        assert!(!sp_linux.command.contains("--server-url"));
        assert!(sp_linux.command.contains("--daemon-api-key sk_test"));
    }

    #[test]
    fn msi_filename_hex_encodes_mode_and_omits_url_for_daemonpoll() {
        let sp_name = encode_msi_filename(&daemon(DaemonMode::ServerPoll, "https://edge.corp"));
        // mode segment decodes back to server_poll
        assert!(sp_name.contains(&format!("mode={}", hex::encode("server_poll"))));
        assert!(sp_name.contains("url="));
        assert!(sp_name.ends_with(".msi"));

        let dp_name = encode_msi_filename(&daemon(DaemonMode::DaemonPoll, ""));
        assert!(dp_name.contains(&format!("mode={}", hex::encode("daemon_poll"))));
        // DaemonPoll dials out; no reachable url encoded.
        assert!(!dp_name.contains("url="));
    }

    #[test]
    fn msi_url_is_the_github_artifact_and_filename_is_encoded() {
        let a = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::ServerPoll, "https://edge.corp"),
            "sk",
        );
        // Direct release asset, not a server path.
        assert_eq!(a.msi_url, WINDOWS_MSI_URL);
        assert!(!a.msi_url.contains("app.scanopy.net"));
        // Filename carries the encoded values for a rename-to-prefill.
        assert!(a.msi_filename.starts_with("scanopy-daemon~~"));
        assert!(a.msi_filename.ends_with(".msi"));
    }

    #[test]
    fn install_command_omits_name() {
        let a = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::DaemonPoll, ""),
            "sk",
        );
        let linux = a.commands.iter().find(|c| c.platform == "linux").unwrap();
        assert!(!linux.command.contains("--name"));
    }
}
