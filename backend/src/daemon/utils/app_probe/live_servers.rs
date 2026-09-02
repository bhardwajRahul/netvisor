//! Every probe against a real server, rather than against bytes we wrote ourselves.
//!
//! The parser tests in each probe module assert what a response *should* mean. They cannot catch a
//! request we constructed wrongly: a malformed packet gets no reply, the parser is handed nothing,
//! and `NoAnswer` looks exactly like "there is no server there". Only a real implementation on the
//! other end closes that gap.
//!
//! `#[ignore]` because it needs servers running, which `cargo test --lib` has no way to arrange.
//! Bring them up with `scratchpad/probe_live.sh` (or by hand) and run:
//!
//! ```text
//! cargo test --lib -- --ignored live_servers
//! ```
//!
//! Ports with nothing listening are reported rather than skipped in silence, so a run that
//! exercised three probes cannot be mistaken for one that exercised all of them.

use std::net::SocketAddr;

use cidr::{IpCidr, Ipv4Cidr};
use tokio_util::sync::CancellationToken;

use crate::daemon::utils::app_probe::{AppProbeOutcome, ProbeContext, all_app_probes};
use crate::daemon::utils::scanner::ScanConcurrencyController;

const LOOPBACK: std::net::Ipv4Addr = std::net::Ipv4Addr::new(127, 0, 0, 1);

fn context() -> ProbeContext {
    ProbeContext {
        ip: std::net::IpAddr::V4(LOOPBACK),
        subnet_cidr: IpCidr::V4(Ipv4Cidr::new(std::net::Ipv4Addr::new(127, 0, 0, 0), 8).unwrap()),
        is_gateway: false,
        cancel: CancellationToken::new(),
        scan_controller: ScanConcurrencyController::new(16),
    }
}

/// Whether anything is accepting connections on this port.
async fn is_listening(port: u16) -> bool {
    let addr = SocketAddr::from((LOOPBACK, port));
    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        tokio::net::TcpStream::connect(addr),
    )
    .await
    .is_ok_and(|r| r.is_ok())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs reference servers running on loopback; see the module docs"]
async fn every_tcp_probe_recognises_its_own_reference_server() {
    let ctx = context();
    let mut recognised = Vec::new();
    let mut missed = Vec::new();
    let mut absent = Vec::new();

    for probe in all_app_probes().into_iter().filter(|p| p.port().is_tcp()) {
        let port = probe.port().number();
        if !is_listening(port).await {
            absent.push(port);
            continue;
        }
        match probe.run(&ctx).await {
            Ok(AppProbeOutcome::Answered { .. }) => recognised.push(port),
            Ok(AppProbeOutcome::NoAnswer) => missed.push(port),
            Err(e) => missed.push_and_note(port, e),
        }
    }

    eprintln!("recognised: {recognised:?}");
    eprintln!("no server listening, so unexercised: {absent:?}");

    assert!(
        missed.is_empty(),
        "a real server was listening on these ports and the probe did not recognise it, which \
         means the request we send is wrong rather than the parser: {missed:?}"
    );
    assert!(
        !recognised.is_empty(),
        "no reference server was reachable, so this run proved nothing"
    );
}

/// Small helper so an `Err` from a probe reads the same as a miss in the failure output.
trait PushAndNote {
    fn push_and_note(&mut self, port: u16, error: anyhow::Error);
}

impl PushAndNote for Vec<u16> {
    fn push_and_note(&mut self, port: u16, error: anyhow::Error) {
        eprintln!("probe on {port} errored: {error}");
        self.push(port);
    }
}
