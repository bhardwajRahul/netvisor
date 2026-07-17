//! `scanopy-daemon install` / `uninstall` — a single install engine shared across all desktop
//! operating systems.
//!
//! One config model, one engine. The **only** configuration input is the same connection/identity
//! flags the daemon already parses ([`DaemonArgs`](crate::daemon::shared::config::DaemonArgs)); the
//! installer feeds them straight to [`AppConfig::load`] and persists the result with
//! [`ConfigStore::persist`]. There is no blob, no prompt, and no second config store.
//!
//! Install performs three steps, in order:
//! 1. **Place the binary** — copy the running executable to the platform location.
//! 2. **Write `config.json`** — the full merged config, including secrets.
//! 3. **Register a system service** — systemd / launchd / Windows SCM.
//!
//! The registered service runs `<binary> --name <name>` and reads everything else from
//! `config.json`, so credentials never land in a unit file, plist, or process arguments.
//!
//! Uninstall reverses this (service → config → binary) and is idempotent: removing a daemon that
//! was never installed succeeds with a "nothing to remove" message.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::daemon::shared::config::{
    AppConfig, ConfigStore, DaemonCommand, InstallArgs, UninstallArgs,
};
use crate::server::daemons::r#impl::base::DaemonMode;

#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
mod bsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_family = "windows")]
mod windows;

#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
use bsd as platform;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_family = "windows")]
use windows as platform;

/// The default daemon name; kept unnamespaced for backward compatibility (see
/// [`AppConfig::get_config_path_for_name`]).
const DEFAULT_NAME: &str = "scanopy-daemon";

#[cfg(target_family = "windows")]
const BINARY_FILE_NAME: &str = "scanopy-daemon.exe";
#[cfg(not(target_family = "windows"))]
const BINARY_FILE_NAME: &str = "scanopy-daemon";

/// Identity and paths for the system service. The service runs
/// `{bin_path} --name {daemon_name} --config-dir {config_dir} --log-file {log_file}` and reads all
/// other settings (including secrets) from `config.json`. The explicit `--config-dir`/`--log-file`
/// are what let the service (running under a different profile than the installer) find the config
/// and logs the installer wrote — without depending on the runtime `$HOME`/`%APPDATA%`.
pub struct ServiceSpec {
    /// Service/unit identifier, e.g. `scanopy-daemon` or `scanopy-daemon-edge1`.
    pub service_id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Installed binary path the service should execute.
    pub bin_path: std::path::PathBuf,
    /// Value passed to `--name` so the service loads the right (possibly namespaced) config.
    pub daemon_name: String,
    /// System config directory to bake into the launch command (`--config-dir`).
    pub config_dir: std::path::PathBuf,
    /// System log file to bake into the launch command (`--log-file`).
    pub log_file: std::path::PathBuf,
}

/// Dispatch an `install`/`uninstall` subcommand.
pub async fn run_command(command: DaemonCommand) -> Result<()> {
    match command {
        DaemonCommand::Install(args) => run_install(args).await,
        DaemonCommand::Uninstall(args) => run_uninstall(args).await,
    }
}

async fn run_install(args: InstallArgs) -> Result<()> {
    require_elevation("install")?;

    let InstallArgs {
        args: daemon_args,
        no_service,
        bin_dir,
    } = args;

    // Same config layering the daemon itself uses — single source of truth.
    let config = AppConfig::load(daemon_args)?;

    // Only DaemonPoll dials out and needs a server URL. ServerPoll is dialed by the
    // server (installed with just --daemon-api-key), so it must not require one. Mode is
    // already resolved (inferred from server_url presence) by AppConfig::load above.
    if config.mode == DaemonMode::DaemonPoll
        && config.server_url.is_none()
        && config.server_target.is_none()
    {
        anyhow::bail!(
            "Missing --server-url: a DaemonPoll daemon needs a server URL to connect to. \
             Re-run: scanopy-daemon install --server-url <url> --daemon-api-key <key>"
        );
    }

    // 1. Place the binary.
    let bin_dir = bin_dir.unwrap_or_else(platform::default_bin_dir);
    let bin_path = bin_dir.join(BINARY_FILE_NAME);
    place_binary(&bin_path)?;

    // 2. Write config.json. A registered service runs under a different profile than this
    //    installer, so its config must live at a fixed *system* path it can always resolve — not
    //    the per-user $HOME/%APPDATA% path. `--no-service` installs keep the per-user path since
    //    the user runs the daemon themselves.
    let config_dir = system_config_dir(&config.name);
    let config_override = if no_service {
        None
    } else {
        Some(config_dir.as_path())
    };
    let (_, config_path) =
        AppConfig::get_config_path_for_name(Some(&config.name), config_override)?;
    let store = ConfigStore::new(config_path.clone(), config.clone());
    store
        .persist()
        .await
        .context("Failed to write daemon config")?;
    println!("Wrote config to {}", config_path.display());

    // 3. Register the service. The launch command carries --config-dir/--log-file so the service
    //    reads exactly what we just wrote, independent of its runtime profile.
    let spec = ServiceSpec {
        service_id: service_id(&config.name),
        display_name: display_name(&config.name),
        bin_path: bin_path.clone(),
        daemon_name: config.name.clone(),
        config_dir,
        log_file: AppConfig::default_system_log_path(&config.name),
    };

    if no_service {
        println!(
            "Skipping service registration (--no-service). Start the daemon manually with:\n  {} --name {}",
            bin_path.display(),
            config.name
        );
    } else {
        platform::register_service(&spec).context("Failed to register the system service")?;
        println!("Registered and started service '{}'.", spec.service_id);
        // Tell the operator where to find the config and logs — the first thing they need
        // when a freshly installed service isn't behaving.
        println!("  Config: {}", config_path.display());
        println!("  Logs:   {}", spec.log_file.display());
    }

    println!("Scanopy daemon installed successfully.");
    Ok(())
}

async fn run_uninstall(args: UninstallArgs) -> Result<()> {
    require_elevation("uninstall")?;

    let name = args.name.unwrap_or_else(|| DEFAULT_NAME.to_string());
    let mut removed_anything = false;

    // 1. Stop + deregister the service (tolerant of an already-absent service).
    let config_dir = system_config_dir(&name);
    let spec = ServiceSpec {
        service_id: service_id(&name),
        display_name: display_name(&name),
        bin_path: platform::default_bin_dir().join(BINARY_FILE_NAME),
        daemon_name: name.clone(),
        config_dir: config_dir.clone(),
        log_file: AppConfig::default_system_log_path(&name),
    };
    if platform::deregister_service(&spec).context("Failed to remove the system service")? {
        removed_anything = true;
        println!("Removed service '{}'.", spec.service_id);
    } else {
        println!("No service '{}' found.", spec.service_id);
    }

    // 2. Remove config.json from both the system location (service installs) and the per-user
    //    profile location (--no-service / manual installs).
    let system_cfg =
        AppConfig::get_config_path_for_name(Some(&name), Some(config_dir.as_path()))?.1;
    let profile_cfg = AppConfig::get_config_path_for_name(Some(&name), None)?.1;
    for config_path in [system_cfg, profile_cfg] {
        if config_path.exists() {
            std::fs::remove_file(&config_path)
                .with_context(|| format!("Failed to delete config {}", config_path.display()))?;
            removed_anything = true;
            println!("Deleted config {}.", config_path.display());
        }
    }

    // 3. Remove the binary only when explicitly requested.
    if args.purge {
        let bin_path = platform::default_bin_dir().join(BINARY_FILE_NAME);
        if bin_path.exists() {
            std::fs::remove_file(&bin_path)
                .with_context(|| format!("Failed to delete binary {}", bin_path.display()))?;
            removed_anything = true;
            println!("Deleted binary {}.", bin_path.display());
        }
    }

    if removed_anything {
        println!("Scanopy daemon uninstalled.");
    } else {
        println!("Nothing to remove — no Scanopy daemon install found for '{name}'.");
    }
    Ok(())
}

/// Copy the running executable to `dest`, with an already-in-place fast path. Idempotent.
fn place_binary(dest: &Path) -> Result<()> {
    let src = std::env::current_exe().context("Failed to locate the running executable")?;

    if let (Ok(s), Ok(d)) = (src.canonicalize(), dest.canonicalize())
        && s == d
    {
        println!("Binary already in place at {}.", dest.display());
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    std::fs::copy(&src, dest)
        .with_context(|| format!("Failed to copy binary to {}", dest.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("Failed to set permissions on {}", dest.display()))?;
    }

    println!("Installed binary to {}.", dest.display());
    Ok(())
}

/// Bail with the platform-specific elevated re-invocation hint when not running as root/admin.
fn require_elevation(action: &str) -> Result<()> {
    if platform::is_elevated() {
        return Ok(());
    }
    eprintln!("{}", platform::elevation_hint());
    anyhow::bail!("Insufficient privileges: re-run the {action} command with elevated permissions")
}

/// Service/unit identifier for a daemon name. The default name keeps its bare id for backward
/// compatibility; custom names are namespaced under the `scanopy-daemon-` prefix.
fn service_id(name: &str) -> String {
    if name == DEFAULT_NAME {
        DEFAULT_NAME.to_string()
    } else {
        format!("scanopy-daemon-{name}")
    }
}

fn display_name(name: &str) -> String {
    if name == DEFAULT_NAME {
        "Scanopy Daemon".to_string()
    } else {
        format!("Scanopy Daemon ({name})")
    }
}

/// The system config directory for a daemon instance — a fixed, profile-independent location
/// (`{default_system_config_dir}/{name}`) that a system service can always resolve. Baked into the
/// service via `--config-dir`, this replaces the old `$HOME`-pinning approach and works uniformly
/// across systemd/launchd/rc.d and the Windows LocalSystem service (which has no `$HOME` lever).
fn system_config_dir(name: &str) -> std::path::PathBuf {
    AppConfig::default_system_config_dir().join(name)
}

/// The current command line, re-quoted well enough to paste after `sudo`.
fn current_invocation() -> String {
    std::env::args()
        .map(|a| {
            if a.is_empty() || a.contains(char::is_whitespace) {
                format!("\"{a}\"")
            } else {
                a
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run a command and fail if it exits non-zero. Shared by the platform service modules.
fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute `{cmd}`"))?;
    if !status.success() {
        anyhow::bail!("`{cmd} {}` failed ({status})", args.join(" "));
    }
    Ok(())
}
