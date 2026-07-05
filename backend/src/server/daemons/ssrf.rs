//! SSRF protection for server-originated requests to daemon URLs.
//!
//! In cloud deployments the server must never be tricked into issuing requests
//! to internal/metadata addresses via an attacker-influenced daemon URL. In
//! self-hosted deployments the server legitimately reaches LAN daemons, so the
//! guard is a no-op there.
use std::net::IpAddr;

use crate::server::config::DeploymentType;

/// Check if an address is private/loopback/internal (SSRF protection).
///
/// Canonicalizes IPv4-mapped IPv6 (`::ffff:a.b.c.d`) to V4 first so a mapped
/// literal can't slip past the V4 rules, and rejects the ranges an attacker
/// would use to reach internal services: RFC1918 private, loopback, link-local
/// (incl. the `169.254.169.254` cloud-metadata endpoint), unspecified,
/// broadcast, documentation, CGNAT/shared (`100.64.0.0/10`), plus IPv6 ULA
/// (`fc00::/7`) and link-local (`fe80::/10`).
pub(crate) fn is_private_ip(addr: &IpAddr) -> bool {
    match addr.to_canonical() {
        IpAddr::V4(ip) => {
            let o = ip.octets();
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_documentation()
                // CGNAT / shared address space: 100.64.0.0/10
                || (o[0] == 100 && (o[1] & 0xC0) == 0x40)
        }
        IpAddr::V6(ip) => {
            let first = ip.segments()[0];
            ip.is_loopback()
                || ip.is_unspecified()
                // Unique local addresses: fc00::/7
                || (first & 0xfe00) == 0xfc00
                // Link-local unicast: fe80::/10
                || (first & 0xffc0) == 0xfe80
        }
    }
}

/// In cloud deployments, reject a daemon URL whose host resolves to any internal
/// address. No-op for self-hosted deployments. Call before issuing a
/// server-originated request to a daemon-controlled URL.
pub(crate) async fn guard_daemon_url(
    url: &str,
    deployment_type: DeploymentType,
) -> anyhow::Result<()> {
    if deployment_type != DeploymentType::Cloud {
        return Ok(());
    }

    let parsed = url::Url::parse(url).map_err(|_| anyhow::anyhow!("Invalid daemon URL"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Daemon URL has no host"))?;
    let port = parsed.port_or_known_default().unwrap_or(80);

    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| anyhow::anyhow!("Could not resolve daemon host"))?;

    for addr in addrs {
        if is_private_ip(&addr.ip()) {
            anyhow::bail!(
                "Daemon URL resolves to a private/internal address (blocked in cloud mode)"
            );
        }
    }

    Ok(())
}
