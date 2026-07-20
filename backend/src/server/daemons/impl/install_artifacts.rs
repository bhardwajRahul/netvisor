//! Server-side assembly of daemon install artifacts.
//!
//! The server is the single source of truth for the install command (it knows its own
//! public URL and the canonical two-flag format), so the provision response hands back
//! ready-to-run commands instead of the UI/MCP/email each re-deriving them. Commands that
//! embed the api key are built here at provision time (the plaintext is only available
//! then, and never stored for DaemonPoll). The MSI itself is a static release asset the UI
//! links to directly; only its per-daemon pre-fill filename ([`encode_msi_filename`], no
//! secret) travels in the response.

use base64ct::{Base64UrlUnpadded, Encoding};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::base::{Daemon, DaemonMode};
use crate::daemon::shared::config::DaemonArgs;

/// The `install.sh` one-liner that fetches + runs the Unix installer bootstrap.
const UNIX_INSTALL_SCRIPT: &str = "bash -c \"$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)\"";
/// Windows daemon exe (matches the UI's hardcoded release URL).
const WINDOWS_EXE_URL: &str =
    "https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe";
/// The signed Windows MSI release asset. A static GitHub asset URL — the UI hardcodes it as a
/// const rather than the server sending it per-provision (it's the same for every tenant). Kept
/// here for reuse by any server-side MSI tooling; only the per-daemon [`encode_msi_filename`]
/// name is tenant-specific and travels in the provision response.
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
    /// Filename encoding this daemon's non-secret config. Save or rename the downloaded MSI
    /// to this name to pre-fill the installer — parse-filename.js decodes it. The api key is
    /// never encoded. Renaming a signed MSI doesn't affect its signature.
    pub msi_filename: String,
    /// Config keys that did not fit in `msi_filename` (a filename is capped at 255
    /// characters). Empty for any ordinary config. The MSI falls back to its built-in
    /// defaults for these, so the UI should tell the user to set them in the installer —
    /// the binary install commands are unaffected and carry the full config.
    pub msi_omitted_config_keys: Vec<String>,
}

/// Resolve the full daemon config to emit: the caller's advanced settings, with every
/// server-controlled field overwritten from the daemon record.
///
/// The advanced fields (`log_level`, `interfaces`, …) are the only ones a client can set —
/// the rest are `#[serde(skip)]` on [`DaemonArgs`] and so never survive deserialization — but
/// the overwrite is unconditional rather than trusting that, since this is what guarantees the
/// emitted artifacts describe the daemon that was actually provisioned.
fn install_args(
    public_url: &str,
    daemon: &Daemon,
    api_key: &str,
    install_config: Option<&DaemonArgs>,
) -> DaemonArgs {
    let mut args = install_config.cloned().unwrap_or_default();

    args.name = Some(daemon.base.name.clone());
    args.mode = Some(daemon.base.mode);
    args.daemon_api_key = Some(api_key.to_string());
    // Only DaemonPoll dials the server, so only it gets a server url. Its absence is also how
    // the daemon infers ServerPoll, which is why the command needs no `--mode`.
    args.server_url = match daemon.base.mode {
        DaemonMode::DaemonPoll if !public_url.is_empty() => Some(public_url.to_string()),
        _ => None,
    };
    // ServerPoll daemons must listen on the port the server dials, so take it from the record
    // rather than the caller. `url` is empty for DaemonPoll.
    if daemon.base.mode == DaemonMode::ServerPoll {
        args.daemon_port = url::Url::parse(&daemon.base.url)
            .ok()
            .and_then(|u| u.port_or_known_default());
    }

    args
}

/// Filesystem limit the encoded name has to live within.
const MAX_MSI_FILENAME_LEN: usize = 255;
const MSI_FILENAME_PREFIX: &str = "scanopy-daemon-";
const MSI_FILENAME_SUFFIX: &str = ".msi";

/// Percent-escape only what the query grammar actually needs — the `%` escape marker and the
/// `&`/`=` delimiters — plus non-ASCII, which the JScript decoder handles byte-wise. Everything
/// else survives verbatim: the whole query is base64url'd into the filename anyway, so escaping
/// spaces and backslashes bought nothing and cost 3 characters each against a tight budget.
fn escape_msi_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '%' => out.push_str("%25"),
            '&' => out.push_str("%26"),
            '=' => out.push_str("%3D"),
            c if c.is_ascii() => out.push(c),
            c => {
                let mut buf = [0u8; 4];
                for byte in c.encode_utf8(&mut buf).as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

/// Longest query that still fits, given base64 expands 3 bytes to 4 characters.
fn max_msi_query_len() -> usize {
    let budget = MAX_MSI_FILENAME_LEN - MSI_FILENAME_PREFIX.len() - MSI_FILENAME_SUFFIX.len();
    budget / 4 * 3
}

/// Build the config query string the MSI filename encodes. Keys match the property map in
/// `backend/wix/parse-filename.js`. The api key is deliberately absent — a live credential must
/// never sit in a filename.
///
/// The whole config has to fit in a filename, which a pathological config (long url + long log
/// path + several long Windows interface names) can exceed. Pairs are taken in order until the
/// budget runs out, and [`DaemonArgs::install_config_pairs`] yields identity first, so what
/// survives is always the part that makes the installer usable. Anything dropped is returned so
/// the caller can tell the user, rather than the MSI quietly installing a differently-configured
/// daemon than the command on screen.
fn msi_config_query(args: &DaemonArgs) -> (String, Vec<String>) {
    let budget = max_msi_query_len();
    let mut query = String::new();
    let mut omitted = Vec::new();

    for pair in args.install_config_pairs() {
        let Some(key) = pair.msi_key else { continue };
        let encoded = format!("{key}={}", escape_msi_value(&pair.value));
        let separator = usize::from(!query.is_empty());
        if query.len() + separator + encoded.len() > budget {
            omitted.push(key.to_string());
            continue;
        }
        if separator == 1 {
            query.push('&');
        }
        query.push_str(&encoded);
    }

    (query, omitted)
}

/// Build the MSI download filename: the whole config query string as ONE base64url segment,
/// so the name stays short even as more config fields are added (vs one `~~field=hex~~` per
/// field, which would blow past the ~255-char filename limit). Decoded by parse-filename.js.
/// Also returns the config keys that did not fit (see [`msi_config_query`]).
pub fn encode_msi_filename(
    public_url: &str,
    daemon: &Daemon,
    install_config: Option<&DaemonArgs>,
) -> (String, Vec<String>) {
    let args = install_args(public_url, daemon, "", install_config);
    let (query, omitted) = msi_config_query(&args);
    let blob = Base64UrlUnpadded::encode_string(query.as_bytes());
    (
        format!("{MSI_FILENAME_PREFIX}{blob}{MSI_FILENAME_SUFFIX}"),
        omitted,
    )
}

/// Quote a value for a POSIX shell, leaving already-safe values bare so the common command
/// stays readable. Values like a Windows log path or an interface name can contain spaces.
fn quote_posix(value: &str) -> String {
    if is_shell_safe(value) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Quote a value for PowerShell. Same intent as [`quote_posix`], but a literal single quote is
/// escaped by doubling rather than by the POSIX close-escape-reopen dance.
fn quote_powershell(value: &str) -> String {
    if is_shell_safe(value) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "''"))
}

fn is_shell_safe(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:,=@+".contains(c))
}

/// The `install` flags for a resolved config. DaemonPoll dials the server (so it carries
/// `--server-url`), ServerPoll is dialed by the server (so it does not). Name and network both
/// come from the provisioned record — the daemon learns its name via the handshake — so the
/// command carries neither; everything else set on `args` is emitted.
fn install_flags(args: &DaemonArgs, quote: fn(&str) -> String) -> String {
    args.install_config_pairs()
        .iter()
        .filter_map(|p| p.cli_flag.map(|flag| format!("{flag} {}", quote(&p.value))))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Assemble the install artifacts for a freshly provisioned daemon.
pub fn build_install_artifacts(
    public_url: &str,
    daemon: &Daemon,
    api_key: &str,
    install_config: Option<&DaemonArgs>,
) -> InstallArtifacts {
    let public_url = public_url.trim_end_matches('/');
    let args = install_args(public_url, daemon, api_key, install_config);

    // Unix binary platforms share the fetch-script + `install` shape.
    let unix = format!(
        "{UNIX_INSTALL_SCRIPT} && sudo scanopy-daemon install {}",
        install_flags(&args, quote_posix)
    );
    let windows = format!(
        "Invoke-WebRequest -Uri \"{WINDOWS_EXE_URL}\" -OutFile \"scanopy-daemon-windows-amd64.exe\"; .\\scanopy-daemon-windows-amd64.exe install {}",
        install_flags(&args, quote_powershell)
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

    let (msi_filename, msi_omitted_config_keys) =
        encode_msi_filename(public_url, daemon, install_config);

    InstallArtifacts {
        commands,
        msi_filename,
        msi_omitted_config_keys,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::shared::config::{DaemonCli, DaemonCommand};
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

    /// Split a POSIX command the way a shell would, honouring the single-quoting
    /// [`quote_posix`] applies, so a round-trip test exercises the real emitted string.
    fn shell_split(command: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut has_token = false;
        let mut chars = command.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\'' => {
                    in_quotes = !in_quotes;
                    has_token = true;
                }
                '\\' if in_quotes => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                c if c.is_whitespace() && !in_quotes => {
                    if has_token {
                        tokens.push(std::mem::take(&mut current));
                        has_token = false;
                    }
                }
                c => {
                    current.push(c);
                    has_token = true;
                }
            }
        }
        if has_token {
            tokens.push(current);
        }
        tokens
    }

    /// The command is only useful if the daemon can actually parse it. Emit a fully populated
    /// config, then feed the emitted string back through the daemon's own clap parser and
    /// compare values — this covers the flag names, the value rendering, and the shell quoting
    /// in one go, and fails only on a real regression rather than a reworded command.
    #[test]
    fn emitted_command_parses_back_into_the_same_config() {
        use clap::Parser;

        let config = DaemonArgs {
            log_level: Some("debug".to_string()),
            // A path with a space is the case bare interpolation would break.
            log_file: Some("/var/log/my daemon/d.log".to_string()),
            heartbeat_interval: Some(45),
            bind_address: Some("10.0.0.5".to_string()),
            interfaces: Some(vec!["eth0".to_string(), "Ethernet 2".to_string()]),
            allow_self_signed_certs: Some(true),
            accept_invalid_scan_certs: Some(false),
            ..Default::default()
        };
        let artifacts = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::DaemonPoll, ""),
            "sk_test",
            Some(&config),
        );
        let linux = artifacts
            .commands
            .iter()
            .find(|c| c.platform == "linux")
            .unwrap();

        // Take the `scanopy-daemon install ...` half of the `bootstrap && install` one-liner.
        let install = linux.command.split("&& sudo ").nth(1).unwrap();
        let parsed = DaemonCli::parse_from(shell_split(install));
        let Some(DaemonCommand::Install(install_args)) = parsed.command else {
            panic!("expected an install subcommand, got {install}");
        };
        let args = install_args.args;

        assert_eq!(args.log_level.as_deref(), Some("debug"));
        assert_eq!(args.log_file.as_deref(), Some("/var/log/my daemon/d.log"));
        assert_eq!(args.heartbeat_interval, Some(45));
        assert_eq!(args.bind_address.as_deref(), Some("10.0.0.5"));
        assert_eq!(
            args.interfaces,
            Some(vec!["eth0".to_string(), "Ethernet 2".to_string()])
        );
        assert_eq!(args.allow_self_signed_certs, Some(true));
        assert_eq!(args.accept_invalid_scan_certs, Some(false));
        assert_eq!(args.daemon_api_key.as_deref(), Some("sk_test"));
        assert_eq!(args.server_url.as_deref(), Some("https://app.scanopy.net"));
    }

    /// Server-controlled and secret fields are `#[serde(skip)]`, so a client cannot smuggle
    /// them in through `install_config` — the emitted command uses the provisioned record's
    /// values regardless of what the request body claimed.
    #[test]
    fn client_supplied_config_cannot_override_server_controlled_fields() {
        let body = r#"{
            "log_level": "trace",
            "daemon_api_key": "attacker-key",
            "server_url": "https://evil.example",
            "network_id": "00000000-0000-0000-0000-000000000001",
            "name": "impostor"
        }"#;
        let config: DaemonArgs = serde_json::from_str(body).unwrap();

        assert_eq!(config.log_level.as_deref(), Some("trace"));
        assert_eq!(config.daemon_api_key, None);
        assert_eq!(config.server_url, None);
        assert_eq!(config.network_id, None);
        assert_eq!(config.name, None);

        let artifacts = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::DaemonPoll, ""),
            "real-key",
            Some(&config),
        );
        let linux = artifacts
            .commands
            .iter()
            .find(|c| c.platform == "linux")
            .unwrap();
        assert!(linux.command.contains("--daemon-api-key real-key"));
        assert!(linux.command.contains("--log-level trace"));
        assert!(!linux.command.contains("evil.example"));
        assert!(!linux.command.contains("attacker-key"));
    }

    /// An ordinary advanced config rides in the MSI filename intact — nothing is dropped, and
    /// the values survive the escape + base64 round trip.
    #[test]
    fn msi_filename_carries_advanced_config() {
        let config = DaemonArgs {
            log_level: Some("debug".to_string()),
            heartbeat_interval: Some(45),
            interfaces: Some(vec!["eth0".to_string(), "Ethernet 2".to_string()]),
            allow_self_signed_certs: Some(true),
            ..Default::default()
        };
        let (filename, omitted) = encode_msi_filename(
            "https://app.scanopy.net",
            &daemon(DaemonMode::DaemonPoll, ""),
            Some(&config),
        );

        assert!(omitted.is_empty(), "unexpectedly dropped {omitted:?}");
        assert!(filename.len() <= 255);

        let fields = decode_msi_filename(&filename);
        assert_eq!(fields.get("loglevel").map(String::as_str), Some("debug"));
        assert_eq!(fields.get("heartbeat").map(String::as_str), Some("45"));
        assert_eq!(
            fields.get("interfaces").map(String::as_str),
            Some("eth0,Ethernet 2")
        );
        assert_eq!(
            fields.get("allowselfsigned").map(String::as_str),
            Some("true")
        );
    }

    /// The whole config rides in a filename, so a pathological one cannot fit. It must stay
    /// within the limit by dropping trailing fields — never by emitting an oversized name —
    /// and identity (which is what makes the installer usable at all) must always survive.
    /// Whatever is dropped is reported so the user can be told.
    #[test]
    fn oversized_msi_config_is_truncated_and_reported() {
        let config = DaemonArgs {
            log_level: Some("trace".to_string()),
            log_file: Some(
                r"C:\ProgramData\Scanopy\daemon\logs\scanopy-daemon-verbose.log".to_string(),
            ),
            heartbeat_interval: Some(300),
            bind_address: Some("255.255.255.255".to_string()),
            interfaces: Some(vec![
                "Ethernet Adapter Multiplexor Driver".to_string(),
                "Wi-Fi 6 AX201 160MHz".to_string(),
                "vEthernet (Default Switch)".to_string(),
            ]),
            allow_self_signed_certs: Some(true),
            accept_invalid_scan_certs: Some(true),
            ..Default::default()
        };
        let (filename, omitted) = encode_msi_filename(
            "https://scanopy.some-quite-long-customer-subdomain.example.com:60072",
            &daemon(DaemonMode::DaemonPoll, ""),
            Some(&config),
        );

        assert!(
            filename.len() <= 255,
            "MSI filename is {} chars, over the 255 limit",
            filename.len()
        );
        assert!(
            !omitted.is_empty(),
            "a config this large cannot fit, so something must be reported as dropped"
        );

        // Identity survives; the dropped keys are absent from the filename and named in the
        // report, so the two always agree.
        let fields = decode_msi_filename(&filename);
        assert_eq!(fields.get("mode").map(String::as_str), Some("daemon_poll"));
        assert_eq!(fields.get("name").map(String::as_str), Some("edge-01"));
        for key in &omitted {
            assert!(
                !fields.contains_key(key),
                "{key} was reported dropped but is present in the filename"
            );
        }
    }

    #[test]
    fn daemon_poll_command_dials_the_server_serverpoll_does_not() {
        let dp = build_install_artifacts(
            "https://app.scanopy.net/",
            &daemon(DaemonMode::DaemonPoll, ""),
            "sk_test",
            None,
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
            None,
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
        let (name, _) = encode_msi_filename(
            "https://app.scanopy.net:60072",
            &daemon(DaemonMode::DaemonPoll, ""),
            None,
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
        let sp = decode_msi_filename(
            &encode_msi_filename(
                "https://app.scanopy.net",
                &daemon(DaemonMode::ServerPoll, "https://edge.corp"),
                None,
            )
            .0,
        );
        assert_eq!(sp.get("mode").map(String::as_str), Some("server_poll"));
        assert!(!sp.contains_key("url"));
    }

    #[test]
    fn msi_filename_is_encoded_for_rename_prefill() {
        let a = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::ServerPoll, "https://edge.corp"),
            "sk",
            None,
        );
        // Filename carries the encoded values for a rename-to-prefill; the static MSI URL
        // is a UI-side const, not part of the per-tenant provision response.
        assert!(a.msi_filename.starts_with("scanopy-daemon-"));
        assert!(a.msi_filename.ends_with(".msi"));
    }

    #[test]
    fn install_command_omits_name() {
        let a = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::DaemonPoll, ""),
            "sk",
            None,
        );
        let linux = a.commands.iter().find(|c| c.platform == "linux").unwrap();
        assert!(!linux.command.contains("--name"));
    }
}
