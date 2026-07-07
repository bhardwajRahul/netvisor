//! Email-domain classification for Brevo contact attributes.
//!
//! `classify_email_domain` maps an email domain to `SCANOPY_DOMAIN_CLASS`
//! (`freemail` | `personal` | `company` | `institutional`) and, for
//! institutional domains, `SCANOPY_INSTITUTION_TYPE` (`government` |
//! `education` | `healthcare` | `utility`).
//!
//! The lookup data is vendored as *raw* source files under
//! `assets/domain-classification/` (refreshed by
//! `scripts/refresh-vendored-data.sh`, run from `make refresh-vendored-data`
//! and the release workflow); all parsing, host normalization, and merging
//! happens here at first use. The pure classifier performs no I/O; the only
//! networked path is the explicit website-liveness probe behind
//! [`classify_email_domain_probed`].
//!
//! Design bias is precision over recall: a missed institution classifies as
//! `company` (cheap, human review catches it), so every layer only fires on
//! high-confidence signals.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use std::time::Duration;

// Raw vendored sources — see scripts/refresh-vendored-data.sh for provenance.
const FREEMAIL_TXT: &str =
    include_str!("../../../assets/domain-classification/freemail-domains.txt");
const UNIVERSITIES_TXT: &str =
    include_str!("../../../assets/domain-classification/university-domains.txt");
const WIKIDATA_HEALTHCARE_CSV: &str =
    include_str!("../../../assets/domain-classification/wikidata-healthcare.csv");
const WIKIDATA_UTILITY_CSV: &str =
    include_str!("../../../assets/domain-classification/wikidata-utility.csv");
const WIKIDATA_GOVERNMENT_CSV: &str =
    include_str!("../../../assets/domain-classification/wikidata-government.csv");
const ANNUAIRE_CSV: &str = include_str!("../../../assets/domain-classification/annuaire-fr.csv");
const GSA_CSV: &str = include_str!("../../../assets/domain-classification/gsa-govt-urls.csv");
const OVERRIDES_JSON: &str =
    include_str!("../../../assets/domain-classification/institutional-overrides.json");

/// Mailbox/ISP providers seen in real signups that are missing from (or worth
/// pinning independently of) the vendored Kikobeats list.
const CURATED_FREEMAIL: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "yahoo.com",
    "yahoo.co.uk",
    "yahoo.co.jp",
    "yahoo.fr",
    "yahoo.de",
    "hotmail.com",
    "outlook.com",
    "live.com",
    "msn.com",
    "icloud.com",
    "me.com",
    "mac.com",
    "proton.me",
    "protonmail.com",
    "pm.me",
    "gmx.com",
    "gmx.de",
    "gmx.net",
    "web.de",
    "aol.com",
    "mail.com",
    "mail.ru",
    "yandex.ru",
    "yandex.com",
    "zoho.com",
    "fastmail.com",
    "hey.com",
    "t-online.de",
    "freenet.de",
    "arcor.de",
    "orange.fr",
    "wanadoo.fr",
    "free.fr",
    "sfr.fr",
    "laposte.net",
    "neuf.fr",
    "bbox.fr",
    "libero.it",
    "tiscali.it",
    "seznam.cz",
    "centrum.cz",
    "qq.com",
    "163.com",
    "126.com",
    "naver.com",
    "daum.net",
    "duck.com",
    "duckduckgo.com",
    "tutanota.com",
    "tuta.io",
    "tuta.com",
    "mailbox.org",
    "posteo.de",
    "posteo.net",
    "mailfence.com",
    "hushmail.com",
    "bluewin.ch",
    "sunrise.ch",
    "telenet.be",
    "skynet.be",
    "ziggo.nl",
    "xs4all.nl",
    "home.nl",
    "planet.nl",
    "comcast.net",
    "verizon.net",
    "att.net",
    "sbcglobal.net",
    "cox.net",
    "charter.net",
    "earthlink.net",
    "bell.net",
    "sympatico.ca",
    "rogers.com",
    "shaw.ca",
    "telus.net",
    "bigpond.com",
    "optusnet.com.au",
    "iinet.net.au",
    "tpg.com.au",
    "btinternet.com",
    "virginmedia.com",
    "talktalk.net",
    "ntlworld.com",
    "sky.com",
    "o2.co.uk",
    "bellsouth.net",
    "juno.com",
    "ocn.ne.jp",
    "biglobe.ne.jp",
    "nifty.com",
    "so-net.ne.jp",
    "mail.pf",
    "simplelogin.com",
    "simplelogin.io",
    "slmail.me",
    "anonaddy.me",
    "mozmail.com",
];

/// Hosting platforms, social networks, blog services, and municipal directory
/// sites that institutions list as their "official website" in the source
/// datasets. An email domain equal to one of these proves nothing about the
/// sender's organization — entries under them are dropped during the merge.
const DENY_SUFFIXES: &[&str] = &[
    "facebook.com",
    "wixsite.com",
    "wix.com",
    "wordpress.com",
    "blogspot.com",
    "blogspot.fr",
    "blogspot.de",
    "blogspot.co.uk",
    "google.com",
    "sites.google.com",
    "weebly.com",
    "jimdo.com",
    "jimdofree.com",
    "jimdosite.com",
    "business.site",
    "notion.site",
    "github.io",
    "gitlab.io",
    "over-blog.com",
    "over-blog.fr",
    "canalblog.com",
    "e-monsite.com",
    "pagesperso-orange.fr",
    "monsite-orange.fr",
    "wifeo.com",
    "instagram.com",
    "twitter.com",
    "x.com",
    "youtube.com",
    "linktr.ee",
    "archive.org",
    "tripod.com",
    "angelfire.com",
    "webs.com",
    "yolasite.com",
    "webnode.com",
    "webnode.fr",
    "webself.net",
    "site123.me",
    "godaddysites.com",
    "squarespace.com",
    "carrd.co",
    "netlify.app",
    "vercel.app",
    "herokuapp.com",
    "sharepoint.com",
    "wordpress.org",
    "tumblr.com",
    "medium.com",
    "ghidulprimariilor.ro",
    "inforpressca.com",
    "lapagelocale.fr",
    "e-primarii.ro",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainClass {
    Freemail,
    /// Custom domain with working email but no website — self-hoster vanity
    /// domains. Only assigned by [`classify_email_domain_probed`].
    Personal,
    Company,
    Institutional,
}

impl DomainClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainClass::Freemail => "freemail",
            DomainClass::Personal => "personal",
            DomainClass::Company => "company",
            DomainClass::Institutional => "institutional",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstitutionType {
    Government,
    Education,
    Healthcare,
    Utility,
}

impl InstitutionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InstitutionType::Government => "government",
            InstitutionType::Education => "education",
            InstitutionType::Healthcare => "healthcare",
            InstitutionType::Utility => "utility",
        }
    }
}

/// URL or bare host -> lowercase punycode host without `www.`, or None for
/// anything that can't be an organization's email domain (IPs, dotless names,
/// header/garbage rows from the raw datasets).
fn normalize_host(raw: &str) -> Option<String> {
    let raw = raw.trim().trim_matches('"').trim().to_lowercase();
    if raw.is_empty() || raw.contains(' ') {
        return None;
    }
    let host = if raw.contains("://") {
        url::Url::parse(&raw).ok()?.host_str()?.to_string()
    } else {
        raw.split(['/', '?', '#', ':']).next()?.to_string()
    };
    let mut host = host.trim_matches('.');
    if let Some(stripped) = host.strip_prefix("www.") {
        host = stripped;
    }
    if !host.contains('.') || host.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let ascii = if host.is_ascii() {
        host.to_string()
    } else {
        idna::domain_to_ascii(host).ok()?
    };
    if !ascii
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return None;
    }
    Some(ascii)
}

fn denied(host: &str) -> bool {
    DENY_SUFFIXES
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

/// Lines of a raw vendored file, trimmed of BOM/CR/whitespace; header and
/// garbage rows are rejected downstream by `normalize_host`.
fn raw_lines(raw: &'static str) -> impl Iterator<Item = &'static str> {
    raw.lines()
        .map(|l| l.trim_matches(['\u{feff}', '\r', ' ', '\t']))
        .filter(|l| !l.is_empty())
}

static FREEMAIL: LazyLock<HashSet<String>> = LazyLock::new(|| {
    raw_lines(FREEMAIL_TXT)
        .chain(CURATED_FREEMAIL.iter().copied())
        .filter_map(normalize_host)
        .collect()
});

static UNIVERSITIES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    raw_lines(UNIVERSITIES_TXT)
        .filter_map(normalize_host)
        .filter(|h| !FREEMAIL.contains(h))
        .collect()
});

/// GSA govt-urls is a real multi-column CSV; take the "Domain name" column.
fn gsa_domains() -> impl Iterator<Item = String> {
    let mut reader = csv::Reader::from_reader(GSA_CSV.trim_start_matches('\u{feff}').as_bytes());
    let idx = reader
        .headers()
        .ok()
        .and_then(|h| h.iter().position(|c| c.to_lowercase().contains("domain")))
        .unwrap_or(0);
    reader
        .into_records()
        .filter_map(|r| r.ok())
        .filter_map(move |r| r.get(idx).map(str::to_string))
}

/// Merged institutional map, built from the raw drops at first use. Sources
/// are processed in precedence order (healthcare > utility > government,
/// first insert wins) so e.g. a commune-run hospital reads healthcare. Hosts
/// on platform/directory suffixes or already known as freemail/university
/// are dropped.
static INSTITUTIONAL: LazyLock<HashMap<String, InstitutionType>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    let sources: [(InstitutionType, Box<dyn Iterator<Item = String>>); 3] = [
        (
            InstitutionType::Healthcare,
            Box::new(raw_lines(WIKIDATA_HEALTHCARE_CSV).filter_map(normalize_host)),
        ),
        (
            InstitutionType::Utility,
            Box::new(raw_lines(WIKIDATA_UTILITY_CSV).filter_map(normalize_host)),
        ),
        (
            InstitutionType::Government,
            Box::new(
                raw_lines(WIKIDATA_GOVERNMENT_CSV)
                    .chain(raw_lines(ANNUAIRE_CSV))
                    .filter_map(normalize_host)
                    .chain(gsa_domains().filter_map(|d| normalize_host(&d))),
            ),
        ),
    ];
    for (inst_type, hosts) in sources {
        for host in hosts {
            if denied(&host) || FREEMAIL.contains(&host) || UNIVERSITIES.contains(&host) {
                continue;
            }
            map.entry(host).or_insert(inst_type);
        }
    }
    map
});

#[derive(Debug, Deserialize)]
struct Overrides {
    domains: HashMap<String, InstitutionType>,
    prefixes: HashMap<String, InstitutionType>,
}

static OVERRIDES: LazyLock<Overrides> = LazyLock::new(|| {
    serde_json::from_str(OVERRIDES_JSON)
        .expect("institutional-overrides.json is vendored and must parse")
});

/// The registrable (eTLD+1) form of a domain, e.g. `mvb.biglobe.ne.jp` ->
/// `biglobe.ne.jp`. Falls back to the input when the PSL can't parse it.
fn registrable(domain: &str) -> &str {
    psl::domain_str(domain).unwrap_or(domain)
}

/// Registry-restricted public suffixes whose registration is eligibility-gated
/// (e.g. `.gov`, `.edu`, `gob.mx`, `ac.jp`), keyed by the suffix's first
/// label. The PSL carries no semantic tags, so this mapping is ours; single
/// labels like `ac` (Ascension Island TLD) or `go` only count inside a
/// multi-label suffix.
fn institution_from_suffix(suffix: &str) -> Option<InstitutionType> {
    let first = suffix.split('.').next()?;
    let multi_label = suffix.contains('.');
    match first {
        "gov" | "mil" | "gob" | "gouv" | "govt" | "gub" | "int" => {
            Some(InstitutionType::Government)
        }
        "go" if multi_label => Some(InstitutionType::Government),
        "edu" => Some(InstitutionType::Education),
        "ac" | "k12" | "sch" if multi_label => Some(InstitutionType::Education),
        _ => None,
    }
}

fn institution_from_overrides(domain: &str, reg: &str) -> Option<InstitutionType> {
    for (entry, inst_type) in &OVERRIDES.domains {
        if domain == entry || domain.ends_with(&format!(".{entry}")) {
            return Some(*inst_type);
        }
    }
    let first_label = reg.split('.').next()?;
    for (prefix, inst_type) in &OVERRIDES.prefixes {
        if first_label.starts_with(prefix.as_str()) {
            return Some(*inst_type);
        }
    }
    None
}

/// Classify an email domain offline. Returns the domain class and, when
/// institutional, the institution type. Never returns [`DomainClass::Personal`]
/// — distinguishing a company from a vanity mail-only domain requires the
/// website probe in [`classify_email_domain_probed`].
pub fn classify_email_domain(domain: &str) -> (DomainClass, Option<InstitutionType>) {
    let domain = domain.trim().trim_end_matches('.').to_lowercase();
    if domain.is_empty() || !domain.contains('.') {
        return (DomainClass::Company, None);
    }
    let reg = registrable(&domain);

    // Freemail: consumer/ISP mailbox providers (subdomains included via the
    // registrable form, e.g. mvb.biglobe.ne.jp) and disposable providers.
    if FREEMAIL.contains(domain.as_str()) || FREEMAIL.contains(reg) {
        return (DomainClass::Freemail, None);
    }
    if !mailchecker::is_valid(&format!("probe@{domain}")) {
        return (DomainClass::Freemail, None);
    }

    // Registry-restricted suffixes: the registry itself guarantees the
    // category (.gov, .edu, gob.mx, ac.jp, ...).
    if let Some(suffix) = psl::suffix_str(&domain)
        && let Some(inst_type) = institution_from_suffix(suffix)
    {
        return (DomainClass::Institutional, Some(inst_type));
    }

    // Hand-maintained overrides win over the generated dataset.
    if let Some(inst_type) = institution_from_overrides(&domain, reg) {
        return (DomainClass::Institutional, Some(inst_type));
    }

    // World-university dataset (universities on unrestricted TLDs).
    if UNIVERSITIES.contains(domain.as_str()) || UNIVERSITIES.contains(reg) {
        return (DomainClass::Institutional, Some(InstitutionType::Education));
    }

    // Merged institutional dataset (Wikidata / Annuaire / GSA). Matched
    // exactly or via the registrable form (mail.aphp.fr -> aphp.fr). Dataset
    // entries that are themselves subdomains (hospital.region.example) only
    // match exactly, so they never claim their whole parent domain.
    if let Some(inst_type) = INSTITUTIONAL
        .get(domain.as_str())
        .or_else(|| INSTITUTIONAL.get(reg))
    {
        return (DomainClass::Institutional, Some(*inst_type));
    }

    (DomainClass::Company, None)
}

/// Per-URL outcome of the website-liveness probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeOutcome {
    /// The host answered HTTP — any status counts (WAF 403s, parked 200s,
    /// broken 5xxs are all evidence something is hosted there).
    Responded,
    /// DNS resolution or TCP connection failed — nothing is hosted there.
    ConnectFailure,
    /// Timeout or other transport error — proves nothing either way.
    Inconclusive,
}

/// Precision bias: only a connect failure on EVERY probed URL is evidence of
/// "no website"; any response or ambiguity keeps the domain a company.
fn has_website_from_outcomes(outcomes: impl IntoIterator<Item = ProbeOutcome>) -> bool {
    !outcomes
        .into_iter()
        .all(|o| o == ProbeOutcome::ConnectFailure)
}

/// Does anything answer HTTP on this domain? Tries `https://domain`,
/// `https://www.domain`, and `http://domain`. `pub(super)` so the backfill
/// can memoize probe results across users sharing a domain.
pub(super) async fn domain_has_website(domain: &str) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    else {
        return true; // can't probe -> inconclusive -> keep company
    };
    let mut outcomes = Vec::with_capacity(3);
    for url in [
        format!("https://{domain}"),
        format!("https://www.{domain}"),
        format!("http://{domain}"),
    ] {
        let outcome = match client.get(&url).send().await {
            Ok(_) => ProbeOutcome::Responded,
            Err(e) if e.is_connect() => ProbeOutcome::ConnectFailure,
            Err(_) => ProbeOutcome::Inconclusive,
        };
        if outcome == ProbeOutcome::Responded {
            return true;
        }
        outcomes.push(outcome);
    }
    has_website_from_outcomes(outcomes)
}

/// [`classify_email_domain`] plus the website-liveness probe: a would-be
/// `company` domain that serves no website at all (vanity mail-only domains
/// like a self-hoster's `jendoubi.fr`) is downgraded to `personal`. Only
/// `company` results are probed, so freemail/institutional classification
/// stays fully offline.
pub async fn classify_email_domain_probed(domain: &str) -> (DomainClass, Option<InstitutionType>) {
    let (class, inst_type) = classify_email_domain(domain);
    if class == DomainClass::Company && !domain_has_website(domain).await {
        return (DomainClass::Personal, None);
    }
    (class, inst_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(domain: &str) -> (DomainClass, Option<InstitutionType>) {
        classify_email_domain(domain)
    }

    #[test]
    fn freemail_provider() {
        assert_eq!(classify("gmail.com"), (DomainClass::Freemail, None));
    }

    #[test]
    fn freemail_isp_subdomain() {
        // ISP mailbox subdomain: matches via the registrable form biglobe.ne.jp.
        assert_eq!(classify("mvb.biglobe.ne.jp"), (DomainClass::Freemail, None));
    }

    #[test]
    fn disposable_counts_as_freemail() {
        assert_eq!(classify("mailinator.com"), (DomainClass::Freemail, None));
    }

    #[test]
    fn healthcare_from_dataset() {
        // No keyword or suffix signal — must come from the vendored dataset.
        assert_eq!(
            classify("aphp.fr"),
            (
                DomainClass::Institutional,
                Some(InstitutionType::Healthcare)
            )
        );
    }

    #[test]
    fn government_from_restricted_suffix() {
        assert_eq!(
            classify("sanpedro.gob.mx"),
            (
                DomainClass::Institutional,
                Some(InstitutionType::Government)
            )
        );
    }

    #[test]
    fn education_from_edu_suffix() {
        assert_eq!(
            classify("millsaps.edu"),
            (DomainClass::Institutional, Some(InstitutionType::Education))
        );
    }

    #[test]
    fn education_from_ac_jp_suffix() {
        assert_eq!(
            classify("saitama-med.ac.jp"),
            (DomainClass::Institutional, Some(InstitutionType::Education))
        );
    }

    #[test]
    fn government_municipality_unrestricted_tld() {
        assert_eq!(
            classify("ville-huahine.pf"),
            (
                DomainClass::Institutional,
                Some(InstitutionType::Government)
            )
        );
    }

    #[test]
    fn utility_from_overrides() {
        assert_eq!(
            classify("naruwasco.co.ke"),
            (DomainClass::Institutional, Some(InstitutionType::Utility))
        );
    }

    #[test]
    fn company_default() {
        assert_eq!(classify("acme-corp.com"), (DomainClass::Company, None));
    }

    #[test]
    fn healthcare_nhs_subdomain() {
        // Parent-domain override semantics: trust.nhs.uk matches the nhs.uk entry.
        assert_eq!(
            classify("somehospital.nhs.uk"),
            (
                DomainClass::Institutional,
                Some(InstitutionType::Healthcare)
            )
        );
    }

    #[test]
    fn case_and_dot_insensitive() {
        assert_eq!(classify("GMAIL.COM."), (DomainClass::Freemail, None));
        assert_eq!(
            classify("Millsaps.EDU"),
            (DomainClass::Institutional, Some(InstitutionType::Education))
        );
    }

    #[test]
    fn garbage_defaults_to_company() {
        assert_eq!(classify(""), (DomainClass::Company, None));
        assert_eq!(classify("localhost"), (DomainClass::Company, None));
    }

    #[test]
    fn ac_tld_alone_is_not_education() {
        // .ac is Ascension Island's TLD; only multi-label ac.* suffixes are academic.
        assert_eq!(classify("example.ac"), (DomainClass::Company, None));
    }

    #[test]
    fn dataset_floors() {
        // Truncation guard over the raw vendored drops: a corrupt/partial
        // refresh must fail the release job's test gate before being
        // committed or baked into images.
        assert!(FREEMAIL.len() >= 10_000, "freemail set: {}", FREEMAIL.len());
        assert!(
            UNIVERSITIES.len() >= 8_000,
            "university set: {}",
            UNIVERSITIES.len()
        );
        assert!(
            INSTITUTIONAL.len() >= 100_000,
            "institutional map: {}",
            INSTITUTIONAL.len()
        );
    }

    #[test]
    fn platform_hosts_never_institutional() {
        // Institutions list Facebook pages etc. as official websites in the
        // raw data; the deny list must keep those hosts out of the map.
        for host in ["facebook.com", "sites.google.com", "wixsite.com"] {
            assert!(!INSTITUTIONAL.contains_key(host), "{host} leaked into map");
        }
    }

    #[test]
    fn freemail_and_institutional_disjoint() {
        // A domain in both sets would make classification order-dependent.
        let overlap: Vec<_> = INSTITUTIONAL
            .keys()
            .filter(|d| FREEMAIL.contains(*d))
            .collect();
        assert!(
            overlap.is_empty(),
            "freemail/institutional overlap: {overlap:?}"
        );
    }

    #[test]
    fn normalize_host_handles_raw_rows() {
        assert_eq!(
            normalize_host("https://www.Ville-Huahine.pf/accueil"),
            Some("ville-huahine.pf".to_string())
        );
        assert_eq!(normalize_host("\"\""), None);
        assert_eq!(normalize_host("site"), None); // wikidata CSV header
        assert_eq!(normalize_host("coordonneesnum_url"), None); // annuaire header
        assert_eq!(normalize_host("174.132.145.94/~hope"), None); // bare IP
        assert_eq!(
            normalize_host("münchen.de"),
            Some("xn--mnchen-3ya.de".to_string())
        );
    }

    #[test]
    fn probe_outcome_fold_is_precision_biased() {
        use ProbeOutcome::*;
        // Only all-connect-failures means "no website".
        assert!(!has_website_from_outcomes([
            ConnectFailure,
            ConnectFailure,
            ConnectFailure
        ]));
        // Any response or ambiguity keeps the domain a company.
        assert!(has_website_from_outcomes([ConnectFailure, Responded]));
        assert!(has_website_from_outcomes([
            ConnectFailure,
            Inconclusive,
            ConnectFailure
        ]));
    }

    #[tokio::test]
    #[ignore] // live network; run with: cargo test --lib probed -- --ignored
    async fn probed_mail_only_domain_is_personal() {
        assert_eq!(
            classify_email_domain_probed("jendoubi.fr").await,
            (DomainClass::Personal, None)
        );
    }

    #[tokio::test]
    #[ignore] // live network
    async fn probed_live_company_stays_company() {
        assert_eq!(
            classify_email_domain_probed("shopify.com").await,
            (DomainClass::Company, None)
        );
    }
}
