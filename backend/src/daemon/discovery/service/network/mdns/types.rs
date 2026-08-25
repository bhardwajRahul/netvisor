use std::collections::BTreeMap;

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

    /// What this address advertised: each DNS-SD service type as it appears on the wire
    /// (`_googlecast._tcp`, `_airplay._tcp`, …) against that service's own TXT key/value pairs.
    /// Read by `Pattern::DnsSd`.
    ///
    /// TXT is kept **per service** rather than merged into one bag, because that is what it is:
    /// a Sonos advertises `_airplay._tcp` with `model=Five` and `_spotify-connect._tcp` with an
    /// unrelated set, and a merged map would let one service's `model` satisfy a constraint
    /// written against another's.
    ///
    /// Keys are lowercased on the way in; the specification says they are case-insensitive and
    /// vendors are inconsistent about it. Values keep their case — `model=AppleTV6,2` is matched
    /// against as the device wrote it.
    pub services: BTreeMap<String, BTreeMap<String, String>>,
}

impl DnsSdHost {
    /// Whether this carries anything worth recording. A bare address with no service and no name
    /// is an artefact of a partial response, not a discovery.
    pub fn is_empty(&self) -> bool {
        self.hostname.is_none() && self.instance_name.is_none() && self.services.is_empty()
    }

    /// Whether the host advertised `service_type`, and if `txt` is given, whether that service's
    /// TXT carries the key with a value starting as specified.
    ///
    /// A value *prefix* rather than an exact match because the identifiers that matter are
    /// versioned: `AppleTV6,2` and `AppleTV11,1` are both Apple TVs, and `AudioAccessory5,1` is a
    /// HomePod whatever revision follows.
    pub fn advertises(&self, service_type: &str, txt: Option<(&str, &str)>) -> bool {
        let Some((_, entries)) = self
            .services
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(service_type))
        else {
            return false;
        };
        let Some((key, value_prefix)) = txt else {
            return true;
        };
        entries
            .get(&key.to_ascii_lowercase())
            .is_some_and(|value| value.starts_with(value_prefix))
    }

    /// The service types advertised, for callers that only care which are present.
    pub fn service_types(&self) -> impl Iterator<Item = &str> {
        self.services.keys().map(String::as_str)
    }
}
