//! Signup-time DNS deliverability check for an email's domain.
//!
//! Confirms a domain can receive mail by resolving its MX records (falling back
//! to A/AAAA per RFC 5321 §5.1). This is a transient gate used at registration
//! to block confirmed-undeliverable domains — it persists nothing. Any timeout
//! or transient resolver failure yields [`DomainCheck::Inconclusive`] so a DNS
//! hiccup never blocks a legitimate signup (fail open).

use std::sync::LazyLock;
use std::time::Duration;

use hickory_resolver::TokioResolver;
use hickory_resolver::net::{DnsError, NetError};
use tokio::time::timeout;

/// Per-lookup timeout. Kept short so registration never stalls on slow DNS.
const DNS_TIMEOUT: Duration = Duration::from_secs(3);

/// Outcome of the DNS deliverability check for an email domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainCheck {
    /// Domain has MX or A/AAAA records — can receive mail.
    Deliverable,
    /// Domain definitively has no MX and no A/AAAA records — cannot receive mail.
    Undeliverable,
    /// DNS lookup failed transiently (timeout / resolver error) — fail open.
    Inconclusive,
}

/// Lazily-initialised system DNS resolver, reused across registration requests.
/// Reads `/etc/resolv.conf` on Unix. If it can't be constructed we treat every
/// check as inconclusive (fail open).
static RESOLVER: LazyLock<Option<TokioResolver>> = LazyLock::new(|| {
    TokioResolver::builder_tokio()
        .ok()
        .and_then(|b| b.build().ok())
});

/// Check whether an email domain can receive mail.
///
/// Resolves MX records first; falls back to A/AAAA per RFC 5321 §5.1 (a domain
/// with an address record but no MX is still a valid mail target). Only a
/// confirmed "no records at all" answer returns [`DomainCheck::Undeliverable`];
/// timeouts and transient resolver errors return [`DomainCheck::Inconclusive`].
pub async fn check_email_domain(domain: &str) -> DomainCheck {
    let Some(resolver) = RESOLVER.as_ref() else {
        return DomainCheck::Inconclusive; // resolver unavailable — fail open
    };

    // MX lookup: a non-empty answer means the domain can receive mail.
    match timeout(DNS_TIMEOUT, resolver.mx_lookup(domain)).await {
        Ok(Ok(lookup)) if !lookup.answers().is_empty() => return DomainCheck::Deliverable,
        Ok(Ok(_)) => {}                       // empty answer — try A/AAAA fallback
        Ok(Err(e)) if is_no_records(&e) => {} // no MX — try A/AAAA fallback
        Ok(Err(_)) => return DomainCheck::Inconclusive, // transient resolver error
        Err(_) => return DomainCheck::Inconclusive, // timed out
    }

    // A/AAAA fallback: implicit MX per RFC 5321 §5.1.
    match timeout(DNS_TIMEOUT, resolver.lookup_ip(domain)).await {
        Ok(Ok(lookup)) if lookup.iter().next().is_some() => DomainCheck::Deliverable,
        Ok(Ok(_)) => DomainCheck::Undeliverable, // resolved with no addresses
        Ok(Err(e)) if is_no_records(&e) => DomainCheck::Undeliverable, // no MX and no A/AAAA
        Ok(Err(_)) => DomainCheck::Inconclusive, // transient resolver error
        Err(_) => DomainCheck::Inconclusive,     // timed out
    }
}

/// Whether a resolver error means the name definitively has no such records
/// (NXDOMAIN / empty answer), as opposed to a transient failure.
fn is_no_records(err: &NetError) -> bool {
    matches!(err, NetError::Dns(DnsError::NoRecordsFound(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Network-dependent; run explicitly with `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn deliverable_domain_passes() {
        assert_eq!(
            check_email_domain("gmail.com").await,
            DomainCheck::Deliverable
        );
    }

    #[tokio::test]
    #[ignore]
    async fn nonexistent_domain_is_undeliverable() {
        assert_eq!(
            check_email_domain("asdkjfh-nope-zzz-does-not-exist.com").await,
            DomainCheck::Undeliverable
        );
    }
}
