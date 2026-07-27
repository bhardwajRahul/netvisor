//! FreeBSD (and, via the shared cfg, a future OpenBSD) platform utilities.
//!
//! Templated on `macos.rs`: BSD and macOS both parse `arp` command output for MAC lookups, share
//! the portable `libc::getrlimit` FD-limit path, and both use BPF (`/dev/bpf*`) datalink channels
//! so the conservative ARP-concurrency cap applies verbatim. The Linux `procfs` / `/proc/net/arp`
//! path does not exist on BSD, so the `arp`-command approach is the right template.
//!
//! Gated `target_os = "freebsd"` for now; the code is structured so an OpenBSD arm can reuse it by
//! widening the cfg to `any(target_os = "freebsd", target_os = "openbsd")`.

#[cfg(target_os = "freebsd")]
use crate::daemon::utils::base::DaemonUtils;

#[cfg(target_os = "freebsd")]
#[derive(Clone)]
pub struct BsdDaemonUtils;

#[cfg(target_os = "freebsd")]
use anyhow::{Error, Result, anyhow};
#[cfg(target_os = "freebsd")]
use mac_address::MacAddress;
#[cfg(target_os = "freebsd")]
impl BsdDaemonUtils {
    /// Parse a MAC address from `arp` output, tolerating dropped leading zeros
    /// (e.g. `0:22:7:4a:21:d5`), as BSD `arp` prints them the same way macOS does.
    fn parse_bsd_mac_address(&self, mac_str: &str) -> Result<MacAddress, Error> {
        let parts: Vec<&str> = mac_str.split(':').collect();
        if parts.len() != 6 {
            return Err(anyhow!("Invalid MAC address format: {}", mac_str));
        }

        let mut mac_bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            mac_bytes[i] = u8::from_str_radix(part, 16)
                .map_err(|_| anyhow!("Invalid hex in MAC address: {}", part))?;
        }

        Ok(MacAddress::new(mac_bytes))
    }
}

#[cfg(target_os = "freebsd")]
use async_trait::async_trait;
#[cfg(target_os = "freebsd")]
use std::net::IpAddr;
#[cfg(target_os = "freebsd")]
#[async_trait]
impl DaemonUtils for BsdDaemonUtils {
    fn new() -> Self {
        Self {}
    }

    fn get_fd_limit() -> Result<usize, Error> {
        use libc::{RLIMIT_NOFILE, getrlimit, rlimit};

        let mut rlim = rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };

        let result = unsafe { getrlimit(RLIMIT_NOFILE, &mut rlim as *mut rlimit) };

        if result == 0 {
            Ok(rlim.rlim_cur as usize)
        } else {
            Err(anyhow!("Failed to get file descriptor limit"))
        }
    }

    fn get_optimal_arp_concurrency(&self) -> Result<usize, Error> {
        // Like macOS, BSD scanning goes through BPF (`/dev/bpf*`) datalink channels, and
        // pnet's `FD_SET` usage has a fixed-size slot array. Opening too many concurrent channels
        // can overrun it, so keep this conservative.
        Ok(10)
    }

    fn get_optimal_deep_scan_concurrency(
        &self,
        port_batch_size: usize,
        arp_subnet_count: usize,
    ) -> Result<usize, Error> {
        let fd_limit = Self::get_fd_limit()?;

        // Base reserved file descriptors:
        // - stdin, stdout, stderr (3)
        // - HTTP client connections for endpoints (50)
        // - Docker socket and other daemon operations (50)
        // - Async channels and miscellaneous (50)
        // - Safety buffer (50)
        let base_reserved = 203;

        // FDs consumed by ARP channels (2 FDs per subnet: tx + rx)
        let arp_fds = arp_subnet_count * 2;

        let total_reserved = base_reserved + arp_fds;
        let available = fd_limit.saturating_sub(total_reserved);

        // FDs consumed per deep-scanned host:
        // - TCP port scanning: port_batch_size concurrent connections
        // - Endpoint HTTP: min(port_batch_size/2, 50) concurrent requests
        // - UDP probes: ~10 concurrent (SNMP, DNS, NTP, DHCP, BACnet)
        let endpoint_batch = (port_batch_size / 2).min(50);
        let udp_probes = 10;
        let fds_per_deep_host = port_batch_size + endpoint_batch + udp_probes;

        // Be conservative when the FD limit is low.
        let concurrency = if fd_limit < 512 {
            std::cmp::max(1, available / fds_per_deep_host).min(2)
        } else {
            std::cmp::max(1, available / fds_per_deep_host)
        };

        tracing::debug!(
            fd_limit,
            base_reserved,
            arp_fds,
            total_reserved,
            available,
            port_batch_size,
            fds_per_deep_host,
            concurrency,
            arp_subnet_count,
            "Calculated deep scan concurrency"
        );

        Ok(concurrency)
    }

    async fn get_mac_address_for_ip(&self, ip: IpAddr) -> Result<Option<MacAddress>, Error> {
        use tokio::process::Command;

        tracing::debug!("Attempting to get MAC address for IP: {}", ip);

        let output = Command::new("arp")
            .args(["-n", &ip.to_string()])
            .output()
            .await?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // Parse BSD arp output: "? (192.168.1.1) at 0:22:7:4a:21:d5 on em0 [ethernet]"
            for line in output_str.lines() {
                tracing::debug!("Processing arp output line: {}", line);
                if line.contains(&ip.to_string()) {
                    if let Some(at_pos) = line.find(" at ") {
                        let after_at = &line[at_pos + 4..];
                        if let Some(space_pos) = after_at.find(' ') {
                            let mac_str = &after_at[..space_pos];
                            tracing::debug!("Found MAC string candidate: {}", mac_str);
                            if mac_str.contains(':') && mac_str.matches(':').count() == 5 {
                                match self.parse_bsd_mac_address(mac_str) {
                                    Ok(mac) => {
                                        tracing::debug!("Parsed MAC address: {}", mac);
                                        return Ok(Some(mac));
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to parse MAC address '{}': {:?}",
                                            mac_str,
                                            e
                                        );
                                        return Err(e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            tracing::debug!("No matching MAC address found for IP: {}", ip);
        } else {
            tracing::warn!("arp command failed with status: {}", output.status);
        }

        Ok(None)
    }
}
