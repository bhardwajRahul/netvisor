//! The regression test for the report this whole change came from.
//!
//! A customer scanning remote VLANs through a FortiGate 400F got a phantom "SIP Server" host on 5060
//! on every VLAN. FortiOS ships `config system session-helper` with a `sip` entry enabled by
//! default, and the helper completes the TCP handshake for any destination routed through the
//! firewall. Their packet capture on the destination VLAN showed zero packets on the wire: nothing
//! was ever there.
//!
//! Privileged ports (22, 21, 23, 88, 445, 554) cannot be bound without root, so those probes are
//! skipped here and covered instead by their own parser tests, each of which asserts that empty
//! input is `NoAnswer`. The count actually exercised is printed, so a run that silently covers
//! almost nothing is visible rather than green.
//!
//! **A listener that accepts the connection and then says nothing is that firewall.** Every TCP
//! probe is pointed at one here and has to report `NoAnswer`. The test is written against the whole
//! registry rather than a list, so a probe added tomorrow is covered the day it lands — and a probe
//! that treats a completed handshake as evidence fails here rather than in a customer's inventory.

use std::net::SocketAddr;

use cidr::{IpCidr, Ipv4Cidr};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::daemon::utils::app_probe::{AppProbe, AppProbeOutcome, ProbeContext, all_app_probes};
use crate::daemon::utils::scanner::ScanConcurrencyController;

/// A context pointed at a listener on loopback.
fn context_for(addr: SocketAddr) -> ProbeContext {
    ProbeContext {
        ip: addr.ip(),
        subnet_cidr: IpCidr::V4(Ipv4Cidr::new(std::net::Ipv4Addr::new(127, 0, 0, 0), 8).unwrap()),
        is_gateway: false,
        cancel: CancellationToken::new(),
        scan_controller: ScanConcurrencyController::new(16),
    }
}

/// Every TCP probe, run against a socket that behaves exactly as an ALG-intercepted address does.
///
/// UDP probes are excluded: there is no connection to complete, so the failure mode this guards
/// against cannot arise for them.
async fn tcp_probes_against(listener_addr: SocketAddr) -> Vec<(&'static str, AppProbeOutcome)> {
    let ctx = context_for(listener_addr);
    let mut outcomes = Vec::new();

    for probe in all_app_probes().into_iter().filter(|p| p.port().is_tcp()) {
        // The probe addresses `ctx.ip` on its own port, so the listener has to be on that port for
        // the probe to reach it. Rebinding per probe is what lets one test cover them all.
        let Ok(listener) = TcpListener::bind((listener_addr.ip(), probe.port().number())).await
        else {
            // The port is in use on this machine, or is privileged and we are not root. Skipping is
            // correct rather than failing: the probe is still covered by its own parser tests.
            continue;
        };

        let accept = tokio::spawn(async move {
            // Accept, hold the connection open saying nothing, and let the probe time out. This is
            // the middlebox: a completed handshake and silence.
            if let Ok((stream, _)) = listener.accept().await {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                drop(stream);
            }
        });

        let outcome = probe.run(&ctx).await.unwrap_or(AppProbeOutcome::NoAnswer);
        accept.abort();
        outcomes.push((probe_name(probe.as_ref()), outcome));
    }
    outcomes
}

/// The probe's port, as a stable label for failure output.
fn probe_name(probe: &dyn AppProbe) -> &'static str {
    Box::leak(format!("{:?}", probe.port()).into_boxed_str())
}

#[tokio::test(flavor = "multi_thread")]
async fn no_tcp_probe_treats_a_silent_listener_as_its_service() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let outcomes = tcp_probes_against(addr).await;

    assert!(
        !outcomes.is_empty(),
        "no TCP probe could be exercised, so this proved nothing"
    );
    eprintln!(
        "exercised {} of {} TCP probes against a silent listener",
        outcomes.len(),
        all_app_probes()
            .iter()
            .filter(|p| p.port().is_tcp())
            .count()
    );

    let fooled: Vec<&str> = outcomes
        .iter()
        .filter(|(_, outcome)| !matches!(outcome, AppProbeOutcome::NoAnswer))
        .map(|(name, _)| *name)
        .collect();

    assert!(
        fooled.is_empty(),
        "these probes reported a service for a listener that accepted the connection and sent \
         nothing, which is exactly what a firewall session helper does on behalf of an address \
         with no device: {fooled:?}"
    );
}
