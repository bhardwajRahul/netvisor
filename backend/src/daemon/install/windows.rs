//! Windows service registration via the Service Control Manager (`sc.exe`). **New capability** —
//! Windows previously had no background-service story.

use anyhow::Result;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::{ServiceSpec, run};

/// `net session` succeeds only for administrators — a dependency-free elevation probe.
pub fn is_elevated() -> bool {
    Command::new("net")
        .args(["session"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn elevation_hint() -> String {
    format!(
        "This command must be run as Administrator. Re-open your terminal with \
         \"Run as administrator\" and re-run:\n  {}",
        super::current_invocation()
    )
}

pub fn default_bin_dir() -> PathBuf {
    let program_files =
        std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    PathBuf::from(program_files).join("Scanopy")
}

pub fn register_service(spec: &ServiceSpec) -> Result<()> {
    // The service reads secrets from config.json; only the binary path + --name go to the SCM.
    let bin_path_value = format!(
        "\"{}\" --name {}",
        spec.bin_path.display(),
        spec.daemon_name
    );

    // Idempotent: remove any prior registration first.
    let _ = Command::new("sc").args(["stop", &spec.service_id]).status();
    let _ = Command::new("sc")
        .args(["delete", &spec.service_id])
        .status();

    // `sc create` requires a space after each `key=`, so the value is a separate argument.
    run(
        "sc",
        &[
            "create",
            &spec.service_id,
            "binPath=",
            &bin_path_value,
            "start=",
            "auto",
            "DisplayName=",
            &spec.display_name,
        ],
    )?;
    run("sc", &["start", &spec.service_id])?;
    Ok(())
}

/// Returns whether the service existed before removal. Stop/delete are best-effort (idempotent).
pub fn deregister_service(spec: &ServiceSpec) -> Result<bool> {
    let existed = Command::new("sc")
        .args(["query", &spec.service_id])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let _ = Command::new("sc").args(["stop", &spec.service_id]).status();
    let _ = Command::new("sc")
        .args(["delete", &spec.service_id])
        .status();

    Ok(existed)
}
