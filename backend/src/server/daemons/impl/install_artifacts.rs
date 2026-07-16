//! Server-side assembly of daemon install artifacts.
//!
//! The server is the single source of truth for the install command (it knows its own
//! public URL, the release-asset locations, and the canonical two-flag format), so the
//! provision response hands back ready-to-run commands and a download link instead of
//! the UI/MCP/email each re-deriving them. Commands that embed the api key are built here
//! at provision time (the plaintext is only available then, and never stored for
//! DaemonPoll); the MSI link carries no secret and is re-derivable later.

use base64ct::{Base64UrlUnpadded, Encoding};
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

/// Build the config query string the MSI filename encodes. Keys match the property map in
/// `backend/wix/parse-filename.js`; values are percent-encoded so `&`/`=`/specials are safe.
/// The api key is deliberately absent — a live credential must never sit in a filename.
fn msi_config_query(public_url: &str, daemon: &Daemon) -> String {
    let mode = match daemon.base.mode {
        DaemonMode::ServerPoll => "server_poll",
        DaemonMode::DaemonPoll => "daemon_poll",
    };
    let mut pairs = vec![
        format!("mode={}", urlencoding::encode(mode)),
        format!("name={}", urlencoding::encode(&daemon.base.name)),
    ];
    // Only DaemonPoll needs a server url to pre-fill (it dials the server → SERVERURL →
    // --server-url). ServerPoll is dialed by the server, so SERVERURL is unused and its
    // reachable url stays server-side; nothing url-related is encoded for it.
    if daemon.base.mode == DaemonMode::DaemonPoll && !public_url.is_empty() {
        pairs.push(format!("url={}", urlencoding::encode(public_url)));
    }
    pairs.join("&")
}

/// Build the MSI download filename: the whole config query string as ONE base64url segment,
/// so the name stays short even as more config fields are added (vs one `~~field=hex~~` per
/// field, which would blow past the ~255-char filename limit). Decoded by parse-filename.js.
pub fn encode_msi_filename(public_url: &str, daemon: &Daemon) -> String {
    let blob = Base64UrlUnpadded::encode_string(msi_config_query(public_url, daemon).as_bytes());
    format!("scanopy-daemon-{blob}.msi")
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
        msi_filename: encode_msi_filename(public_url, daemon),
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

    // Decode the filename the way parse-filename.js does (strip prefix, base64url-decode,
    // parse the query string) so the encode<->decode scheme is validated in Rust; the
    // JScript CA is a faithful port of this. Percent-decoding here is ASCII-only (matching
    // the JScript's manual decoder), sufficient for mode/name/url values.
    fn decode_msi_filename(filename: &str) -> std::collections::HashMap<String, String> {
        let blob = filename
            .strip_prefix("scanopy-daemon-")
            .and_then(|s| s.strip_suffix(".msi"))
            .expect("scanopy-daemon-<blob>.msi");
        let query = String::from_utf8(Base64UrlUnpadded::decode_vec(blob).unwrap()).unwrap();
        query
            .split('&')
            .filter_map(|p| p.split_once('='))
            .map(|(k, v)| (k.to_string(), urlencoding::decode(v).unwrap().into_owned()))
            .collect()
    }

    #[test]
    fn msi_filename_is_one_base64_segment_that_round_trips() {
        // DaemonPoll pre-fills the server url it dials.
        let name = encode_msi_filename(
            "https://app.scanopy.net:60072",
            &daemon(DaemonMode::DaemonPoll, ""),
        );
        // One compact segment, no per-field `~~` markers.
        assert!(name.starts_with("scanopy-daemon-"));
        assert!(name.ends_with(".msi"));
        assert!(!name.contains("~~"));

        let fields = decode_msi_filename(&name);
        assert_eq!(fields.get("mode").map(String::as_str), Some("daemon_poll"));
        assert_eq!(fields.get("name").map(String::as_str), Some("edge-01"));
        // The url survives its :// and : intact through percent-encode + base64.
        assert_eq!(
            fields.get("url").map(String::as_str),
            Some("https://app.scanopy.net:60072")
        );

        // ServerPoll is dialed by the server → no server url encoded.
        let sp = decode_msi_filename(&encode_msi_filename(
            "https://app.scanopy.net",
            &daemon(DaemonMode::ServerPoll, "https://edge.corp"),
        ));
        assert_eq!(sp.get("mode").map(String::as_str), Some("server_poll"));
        assert!(!sp.contains_key("url"));
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
        assert!(a.msi_filename.starts_with("scanopy-daemon-"));
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
