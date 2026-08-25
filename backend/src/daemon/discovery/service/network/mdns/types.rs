use std::collections::{BTreeMap, BTreeSet};

/// What a device announced about itself over mDNS/DNS-SD.
///
/// Keyed by address at the point of use, because that is the only handle the rest of discovery
/// has. A device announcing several services collapses into one of these — a Chromecast answers
/// on `_googlecast._tcp` and `_googlezone._tcp` and is still one host.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DnsSdHost {
    /// The device's own `.local` name, from the SRV target — e.g. `chromecast-a1b2c3.local`.
    ///
    /// Machine-assigned and stable; distinct from [`Self::instance_name`], which a person chose.
    pub hostname: Option<String>,

    /// The DNS-SD instance name, or the friendlier value a TXT record carries for it — the
    /// Chromecast `fn=Living Room TV`, or the label a person typed into a device's setup app.
    pub instance_name: Option<String>,

    /// Service types this address answered for, as they appear on the wire
    /// (`_googlecast._tcp`, `_airplay._tcp`, …). Read by `Pattern::DnsSdService`.
    pub services: BTreeSet<String>,

    /// Merged TXT key/value pairs across every service. Carries model, vendor and version for
    /// most consumer devices — `md=Chromecast Ultra`, `ty=HP LaserJet`.
    ///
    /// Keys are lowercased on the way in; the specification says they are case-insensitive and
    /// vendors are inconsistent about it.
    pub txt: BTreeMap<String, String>,
}

impl DnsSdHost {
    /// Whether this carries anything worth recording. A bare address with no service, name or TXT
    /// data is an artefact of a partial response, not a discovery.
    pub fn is_empty(&self) -> bool {
        self.hostname.is_none()
            && self.instance_name.is_none()
            && self.services.is_empty()
            && self.txt.is_empty()
    }
}
