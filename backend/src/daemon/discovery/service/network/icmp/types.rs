use std::net::Ipv4Addr;

/// An address that answered an ICMP echo request.
///
/// Deliberately thinner than [`super::super::arp::ArpScanResult`]: an echo reply proves the
/// address is alive and nothing else. ARP is the only sweep that yields a MAC, which is why the
/// two signals are kept distinct all the way through to [`super::super::LivenessEvidence`] rather
/// than merged into one "responder" type here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IcmpScanResult {
    pub ip: Ipv4Addr,
}
