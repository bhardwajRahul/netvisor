//! A TLS handshake driven only as far as the server's certificate.
//!
//! Two ports here front services behind TLS the scanner can never complete a session with:
//! Docker Swarm's raft port (2377) demands a client certificate, and unbound's control channel
//! (8953) is self-signed and demands one too. Both were declared `NoDistinguishingHandshake` on the
//! grounds that an encrypted channel tells an unauthenticated peer nothing. That was wrong. **The
//! handshake that sets up the encryption is itself an exchange**, and a middlebox that merely
//! completes a TCP handshake cannot produce one.
//!
//! The trick is where to read it. `ClientConnection::peer_certificates` only populates once the
//! connection reaches `Connected`, which mutual TLS without a client certificate never does. But
//! the server's `Certificate` message precedes the client's in both TLS 1.2 and 1.3, so
//! [`rustls::client::danger::ServerCertVerifier::verify_server_cert`] is handed it *before* the
//! client-certificate requirement bites. [`CapturingVerifier`] keeps a copy and then refuses,
//! which makes rustls send an alert and tear the connection down immediately: one round trip, no
//! session, and the certificate in hand.
//!
//! Refusing rather than accepting is deliberate. Returning `Ok` would let rustls carry on to a
//! handshake that cannot succeed, spending another round trip to reach the same place.
//!
//! What the certificate is worth differs by port, and the two probes say so themselves: a swarm
//! manager's certificate names the swarm, while unbound's says nothing dependable and only its
//! existence counts.

use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as TlsError, SignatureScheme};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use x509_cert::Certificate;
use x509_cert::der::Decode;

use crate::daemon::utils::app_probe::ProbeContext;
use crate::daemon::utils::scanner::SCAN_TIMEOUT;
use crate::server::ports::r#impl::base::PortType;

/// A verifier whose only job is to keep the certificate it is shown.
///
/// It always refuses. Nothing here is validating a chain or trusting a peer — the certificate is
/// evidence about what is listening, not a credential, and no data is exchanged after it.
#[derive(Debug)]
struct CapturingVerifier {
    captured: Arc<Mutex<Option<Vec<u8>>>>,
}

impl ServerCertVerifier for CapturingVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        if let Ok(mut slot) = self.captured.lock() {
            *slot = Some(end_entity.to_vec());
        }
        // Refuse: we have what we came for, and the handshake cannot succeed anyway.
        Err(TlsError::General(
            "scanopy probe: certificate captured".into(),
        ))
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}

/// What a TLS handshake against a port established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsHandshake {
    /// The peer spoke TLS and presented this certificate, in DER.
    Certificate(Vec<u8>),
    /// Nothing answered, or what answered was not TLS.
    NotTls,
}

impl TlsHandshake {
    /// The certificate's subject as an RFC 4514 string, e.g.
    /// `CN=abc123,OU=swarm-manager,O=xyz789`.
    ///
    /// `None` when the peer did not speak TLS, or when the certificate cannot be parsed — a
    /// malformed certificate is not evidence of anything in particular, so it is treated the same
    /// as no certificate rather than as a weaker positive.
    pub fn subject(&self) -> Option<String> {
        match self {
            Self::Certificate(der) => Certificate::from_der(der)
                .ok()
                .map(|cert| cert.tbs_certificate().subject().to_string()),
            Self::NotTls => None,
        }
    }

    /// Whether the peer spoke TLS at all, whatever its certificate said.
    pub fn spoke_tls(&self) -> bool {
        matches!(self, Self::Certificate(_))
    }
}

/// Open a TLS handshake against `port` and return whatever certificate the server sent.
pub(crate) async fn capture_certificate(ctx: &ProbeContext, port: PortType) -> TlsHandshake {
    let captured: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));

    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(CapturingVerifier {
            captured: Arc::clone(&captured),
        }))
        .with_no_client_auth();
    // The peer is addressed by IP and its certificate is evidence rather than a credential, so
    // there is nothing for SNI or ALPN to negotiate.
    config.enable_sni = false;

    let addr = std::net::SocketAddr::new(ctx.ip, port.number());
    let stream = match tokio::time::timeout(SCAN_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            ctx.note_connect_error(&e);
            return TlsHandshake::NotTls;
        }
        Err(_) => return TlsHandshake::NotTls,
    };

    // A name is required by the API and is never sent, because SNI is off above.
    let server_name = ServerName::try_from("scanopy.invalid").expect("a literal, valid DNS name");
    let connector = TlsConnector::from(Arc::new(config));

    // This is expected to fail: the verifier refuses by design, and a mutual-TLS server would
    // refuse us in turn. The certificate, if there was one, is already captured.
    let _ = tokio::time::timeout(SCAN_TIMEOUT, connector.connect(server_name, stream)).await;

    match captured.lock() {
        Ok(slot) => match slot.clone() {
            Some(der) => TlsHandshake::Certificate(der),
            None => TlsHandshake::NotTls,
        },
        Err(_) => TlsHandshake::NotTls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_peer_that_did_not_speak_tls_has_no_subject_and_no_handshake() {
        let outcome = TlsHandshake::NotTls;
        assert_eq!(outcome.subject(), None);
        assert!(!outcome.spoke_tls());
    }

    /// A middlebox that completes a TCP handshake and sends nothing gets this far and no further:
    /// no certificate arrives, so the verifier never runs.
    #[test]
    fn bytes_that_are_not_a_certificate_yield_no_subject() {
        let outcome = TlsHandshake::Certificate(b"not a certificate".to_vec());
        assert!(
            outcome.spoke_tls(),
            "something did speak TLS to produce this"
        );
        assert_eq!(
            outcome.subject(),
            None,
            "an unparseable certificate identifies nothing"
        );
    }

    /// The real thing: a live swarm manager on the port named by `SCANOPY_SWARM_PORT`. Ignored by
    /// default because it needs `docker swarm init`; see `live_servers.rs`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs a live swarm; set SCANOPY_SWARM_PORT"]
    async fn a_live_swarm_manager_presents_a_certificate_naming_the_swarm() {
        use crate::daemon::utils::scanner::ScanConcurrencyController;
        use cidr::{IpCidr, Ipv4Cidr};
        let port: u16 = std::env::var("SCANOPY_SWARM_PORT")
            .expect("set SCANOPY_SWARM_PORT to a reachable swarm raft port")
            .parse()
            .expect("a port number");
        let ctx = ProbeContext {
            ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            subnet_cidr: IpCidr::V4(
                Ipv4Cidr::new(std::net::Ipv4Addr::new(127, 0, 0, 0), 8).unwrap(),
            ),
            is_gateway: false,
            cancel: tokio_util::sync::CancellationToken::new(),
            scan_controller: ScanConcurrencyController::new(4),
        };
        let handshake = capture_certificate(&ctx, PortType::new_tcp(port)).await;
        let subject = handshake
            .subject()
            .expect("a swarm manager presents a certificate we can parse");
        eprintln!("swarm certificate subject: {subject}");
        assert!(
            subject.contains("swarm-manager") || subject.contains("swarm-worker"),
            "expected the subject to name a swarm role, got {subject}"
        );
    }

    /// The shape a Docker Swarm manager presents. Built here rather than captured so the test
    /// states what the probe depends on; `live_servers.rs` proves a real swarm matches it.
    #[test]
    fn a_certificate_subject_reads_back_as_rfc4514() {
        // A minimal self-signed certificate with a swarm-shaped subject, generated once and
        // inlined so this test needs no key generation at runtime.
        let der = include_bytes!("testdata/swarm_like_cert.der");
        let outcome = TlsHandshake::Certificate(der.to_vec());
        let subject = outcome.subject().expect("parses");
        assert!(
            subject.contains("swarm-manager"),
            "expected an OU naming the swarm role, got {subject}"
        );
    }
}
