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
use crate::server::credentials::r#impl::mapping::IntegrationTarget;

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
    /// A ready-to-run `docker-compose.yml`. Only present for a first install: re-keying or
    /// re-syncing a container means editing its existing compose file, not running a new one.
    pub docker_compose: Option<String>,
}

/// What the emitted artifacts are *for*. A command's correct contents depend on whether the
/// daemon is being stood up from nothing or adjusted in place — `scanopy-daemon install` layers
/// CLI flags over the existing `config.json`, so anything omitted keeps its current value, and
/// an installed daemon should only be told what is actually changing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactPurpose {
    /// First install: the command has to carry everything needed to bring the daemon up.
    Initial,
    /// Re-key an already-installed daemon. Only the credential — the canonical two-flag form.
    /// Carrying config here would silently re-apply server-side edits the operator never asked
    /// to push, which is what made a Details-tab port edit show up in a re-key command.
    Rekey,
    /// Re-assert the server-held connectivity config on an installed daemon, with no credential.
    /// Safe to display persistently and to run repeatedly.
    Sync,
}

impl ArtifactPurpose {
    /// Whether the emitted command embeds the daemon's api key.
    ///
    /// The plaintext key exists only at the moment it is minted (it is never stored for
    /// DaemonPoll), so a credential-embedding purpose can only be served from the provision
    /// endpoint that mints it — never from a plain read.
    pub fn embeds_credential(&self) -> bool {
        match self {
            Self::Initial | Self::Rekey => true,
            Self::Sync => false,
        }
    }
}

/// Resolve the daemon config to emit for a given purpose, with every server-controlled field
/// taken from the daemon record.
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
    purpose: ArtifactPurpose,
) -> DaemonArgs {
    // Only a first install seeds from the caller's advanced settings; an installed daemon keeps
    // whatever is already in its config.json.
    let mut args = match purpose {
        ArtifactPurpose::Initial => install_config.cloned().unwrap_or_default(),
        ArtifactPurpose::Rekey | ArtifactPurpose::Sync => DaemonArgs::default(),
    };

    // Only DaemonPoll dials the server, so only it gets a server url. Its absence is also how
    // the daemon infers ServerPoll, which is why no command needs `--mode`.
    let server_url = match daemon.base.mode {
        DaemonMode::DaemonPoll if !public_url.is_empty() => Some(public_url.to_string()),
        _ => None,
    };
    // ServerPoll daemons must listen on the port the server dials, so it comes from the record
    // rather than the caller. `url` is empty for DaemonPoll.
    let daemon_port = (daemon.base.mode == DaemonMode::ServerPoll)
        .then(|| {
            url::Url::parse(&daemon.base.url)
                .ok()
                .and_then(|u| u.port_or_known_default())
        })
        .flatten();

    match purpose {
        ArtifactPurpose::Initial => {
            args.name = Some(daemon.base.name.clone());
            args.mode = Some(daemon.base.mode);
            args.daemon_api_key = Some(api_key.to_string());
            args.server_url = server_url;
            args.daemon_port = daemon_port;
        }
        ArtifactPurpose::Rekey => {
            args.daemon_api_key = Some(api_key.to_string());
            args.server_url = server_url;
        }
        ArtifactPurpose::Sync => {
            // No credential: this command is shown persistently rather than once, and omitting
            // it leaves the daemon's existing key in place.
            //
            // No `--name` either: the daemon derives its service id from it
            // (`scanopy-daemon-{name}`), so a server-side rename would make this register a
            // *second* service instead of reconfiguring the existing one.
            args.server_url = server_url;
            args.daemon_port = daemon_port;
        }
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
    // The MSI only ever performs a first install, so it always pre-fills the full config.
    let args = install_args(
        public_url,
        daemon,
        "",
        install_config,
        ArtifactPurpose::Initial,
    );
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

/// Container image the compose file runs.
const DOCKER_IMAGE: &str = "ghcr.io/scanopy/scanopy/daemon:latest";

/// Quote a compose env value if YAML would otherwise mis-read it. Values sit in a
/// `- KEY=value` list item, where most characters are fine bare; leading/trailing whitespace
/// and a ` #` (which starts a comment) are the cases that need quoting.
fn quote_yaml(value: &str) -> String {
    let needs_quoting = value.trim() != value || value.contains(" #") || value.contains('"');
    if !needs_quoting {
        return value.to_string();
    }
    format!("\"{}\"", value.replace('\\', r"\\").replace('"', "\\\""))
}

/// Build the `docker-compose.yml` for a first install.
///
/// The env block comes from the same [`DaemonArgs::install_config_pairs`] table as the CLI and
/// MSI artifacts, so the three cannot drift. Notably it emits no network id, user id, name or
/// mode: those are `#[serde(skip)]` on [`DaemonArgs`] precisely because a client must not be
/// able to assert them, and the binary install command dropped them for the same reason —
/// identity comes from the 1:1 api-key binding and the handshake. A compose file that asserted
/// them could disagree with the record its key is bound to.
fn docker_compose(
    args: &DaemonArgs,
    daemon: &Daemon,
    seed_credential_refs: &[IntegrationTarget],
) -> String {
    let pairs = args.install_config_pairs();
    let mut env: Vec<String> = pairs
        .iter()
        .filter_map(|p| {
            p.env_var
                .map(|key| format!("{key}={}", quote_yaml(&p.value)))
        })
        .collect();

    if !seed_credential_refs.is_empty() {
        let tokens = seed_credential_refs
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(",");
        env.push(format!("SCANOPY_CREDENTIAL_IDS={}", quote_yaml(&tokens)));
    }

    // Docker-only: logs must land on the mounted volume to survive the container, so a log file
    // is always set even when the CLI install would leave the platform default in place.
    if !pairs.iter().any(|p| p.env_var == Some("SCANOPY_LOG_FILE")) {
        env.push(format!(
            "SCANOPY_LOG_FILE=/var/log/scanopy/{}.log",
            daemon.base.name
        ));
    }

    let volumes = [
        "daemon-config:/root/.config/scanopy/daemon",
        "/var/run/docker.sock:/var/run/docker.sock:ro",
        "/var/log/scanopy:/var/log/scanopy",
    ];

    let mut lines = vec![
        "services:".to_string(),
        "  daemon:".to_string(),
        format!("    image: {DOCKER_IMAGE}"),
        "    container_name: scanopy-daemon".to_string(),
        "    network_mode: host".to_string(),
        "    privileged: true".to_string(),
        "    restart: unless-stopped".to_string(),
        "    environment:".to_string(),
    ];
    lines.extend(env.iter().map(|e| format!("      - {e}")));
    lines.push("    volumes:".to_string());
    lines.extend(volumes.iter().map(|v| format!("      - {v}")));
    lines.push(String::new());
    lines.push("volumes:".to_string());
    lines.push("  daemon-config:".to_string());

    lines.join("\n")
}

/// Assemble the install artifacts for a daemon, shaped by what they are for.
pub fn build_install_artifacts(
    public_url: &str,
    daemon: &Daemon,
    api_key: &str,
    install_config: Option<&DaemonArgs>,
    seed_credential_refs: &[IntegrationTarget],
    purpose: ArtifactPurpose,
) -> InstallArtifacts {
    let public_url = public_url.trim_end_matches('/');
    let args = install_args(public_url, daemon, api_key, install_config, purpose);

    // A Sync command runs against an already-installed daemon, so it must not re-fetch the
    // binary — it only re-asserts config. Initial and Rekey both fetch: Rekey targets a legacy
    // daemon, where picking up the current binary alongside the new key is desirable.
    let (unix, windows) = match purpose {
        ArtifactPurpose::Sync => (
            format!(
                "sudo scanopy-daemon install {}",
                install_flags(&args, quote_posix)
            ),
            // On Windows the binary lives in Program Files rather than on PATH.
            format!(
                "& \"$env:ProgramFiles\\Scanopy\\scanopy-daemon.exe\" install {}",
                install_flags(&args, quote_powershell)
            ),
        ),
        ArtifactPurpose::Initial | ArtifactPurpose::Rekey => (
            // Unix binary platforms share the fetch-script + `install` shape.
            format!(
                "{UNIX_INSTALL_SCRIPT} && sudo scanopy-daemon install {}",
                install_flags(&args, quote_posix)
            ),
            format!(
                "Invoke-WebRequest -Uri \"{WINDOWS_EXE_URL}\" -OutFile \"scanopy-daemon-windows-amd64.exe\"; .\\scanopy-daemon-windows-amd64.exe install {}",
                install_flags(&args, quote_powershell)
            ),
        ),
    };

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
        docker_compose: (purpose == ArtifactPurpose::Initial)
            .then(|| docker_compose(&args, daemon, seed_credential_refs)),
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

    /// Parse the `install` half of an emitted unix command back through the daemon's own clap
    /// parser, so assertions are about what the daemon would actually receive.
    fn parse_unix_install(artifacts: &InstallArtifacts) -> DaemonArgs {
        use clap::Parser;

        let linux = artifacts
            .commands
            .iter()
            .find(|c| c.platform == "linux")
            .unwrap();
        // Initial/Rekey prefix the bootstrap with `&& sudo `; Sync has no bootstrap at all.
        let install = linux
            .command
            .split("&& sudo ")
            .nth(1)
            .unwrap_or(&linux.command)
            .trim_start_matches("sudo ");
        let parsed = DaemonCli::parse_from(shell_split(install));
        let Some(DaemonCommand::Install(args)) = parsed.command else {
            panic!("expected an install subcommand, got {install}");
        };
        args.args
    }

    /// Re-keying an installed daemon must change only its credential. Server-side record edits
    /// (an adjusted ServerPoll port, say) must not ride along and get silently re-applied —
    /// `install` layers CLI over the existing config.json, so anything omitted is preserved.
    #[test]
    fn rekey_command_carries_only_the_credential() {
        let config = DaemonArgs {
            log_level: Some("trace".to_string()),
            interfaces: Some(vec!["eth0".to_string()]),
            ..Default::default()
        };
        let args = parse_unix_install(&build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::ServerPoll, "https://edge.corp:60074"),
            "sk_rekey",
            // Even if advanced config is supplied, a re-key must not emit it.
            Some(&config),
            &[],
            ArtifactPurpose::Rekey,
        ));

        assert_eq!(args.daemon_api_key.as_deref(), Some("sk_rekey"));
        assert_eq!(args.daemon_port, None);
        assert_eq!(args.log_level, None);
        assert_eq!(args.interfaces, None);
        assert_eq!(args.name, None);
    }

    /// The sync command is displayed persistently rather than once, so it must never embed a
    /// credential — and it must not carry `--name`, which would re-target the service id and
    /// register a second service instead of reconfiguring the existing one.
    #[test]
    fn sync_command_reasserts_connectivity_without_a_credential() {
        let sp = parse_unix_install(&build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::ServerPoll, "https://edge.corp:60074"),
            "sk_should_not_appear",
            None,
            &[],
            ArtifactPurpose::Sync,
        ));
        assert_eq!(sp.daemon_api_key, None);
        assert_eq!(sp.name, None);
        assert_eq!(sp.daemon_port, Some(60074));

        let dp = parse_unix_install(&build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::DaemonPoll, ""),
            "sk_should_not_appear",
            None,
            &[],
            ArtifactPurpose::Sync,
        ));
        assert_eq!(dp.daemon_api_key, None);
        assert_eq!(dp.server_url.as_deref(), Some("https://app.scanopy.net"));
        assert_eq!(dp.daemon_port, None);
    }

    /// A sync command runs against an installed daemon, so it must not re-download the binary.
    #[test]
    fn sync_command_does_not_refetch_the_binary() {
        let artifacts = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::ServerPoll, "https://edge.corp:60074"),
            "",
            None,
            &[],
            ArtifactPurpose::Sync,
        );
        for command in &artifacts.commands {
            assert!(
                !command.command.contains("install.sh")
                    && !command.command.contains("Invoke-WebRequest"),
                "{} command re-fetches the binary: {}",
                command.platform,
                command.command
            );
        }
    }

    /// Compose must not assert identity. Those fields are `#[serde(skip)]` on DaemonArgs
    /// precisely so a client cannot set them, and the binary command dropped them for the same
    /// reason — identity comes from the 1:1 key binding. A compose file that carried them could
    /// disagree with the record its key is bound to.
    #[test]
    fn docker_compose_carries_config_but_never_identity() {
        let config = DaemonArgs {
            log_level: Some("debug".to_string()),
            heartbeat_interval: Some(45),
            ..Default::default()
        };
        let compose = build_install_artifacts(
            "https://app.scanopy.net",
            &daemon(DaemonMode::DaemonPoll, ""),
            "sk_test",
            Some(&config),
            &[],
            ArtifactPurpose::Initial,
        )
        .docker_compose
        .expect("a first install yields a compose file");

        assert!(compose.contains("SCANOPY_SERVER_URL=https://app.scanopy.net"));
        assert!(compose.contains("SCANOPY_DAEMON_API_KEY=sk_test"));
        assert!(compose.contains("SCANOPY_LOG_LEVEL=debug"));
        assert!(compose.contains("SCANOPY_HEARTBEAT_INTERVAL=45"));
        // Docker-only: logs must land on the mounted volume.
        assert!(compose.contains("SCANOPY_LOG_FILE=/var/log/scanopy/edge-01.log"));

        for identity in [
            "SCANOPY_NETWORK_ID",
            "SCANOPY_USER_ID",
            "SCANOPY_NAME",
            "SCANOPY_MODE",
        ] {
            assert!(
                !compose.contains(identity),
                "compose asserts {identity}, which the client must not be able to set"
            );
        }

        // Re-keying or syncing means editing an existing compose file, not running a new one.
        assert!(
            build_install_artifacts(
                "https://app.scanopy.net",
                &daemon(DaemonMode::DaemonPoll, ""),
                "sk",
                None,
                &[],
                ArtifactPurpose::Rekey,
            )
            .docker_compose
            .is_none()
        );
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
            &[],
            ArtifactPurpose::Initial,
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
            &[],
            ArtifactPurpose::Initial,
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
            &[],
            ArtifactPurpose::Initial,
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
            &[],
            ArtifactPurpose::Initial,
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
            &[],
            ArtifactPurpose::Initial,
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
            &[],
            ArtifactPurpose::Initial,
        );
        let linux = a.commands.iter().find(|c| c.platform == "linux").unwrap();
        assert!(!linux.command.contains("--name"));
    }
}
