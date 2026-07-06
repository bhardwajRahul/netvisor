//! Email-domain classification for Brevo contact attributes.
//!
//! Pure, offline lookup: `classify_email_domain` maps an email domain to
//! `SCANOPY_DOMAIN_CLASS` (`freemail` | `company` | `institutional`) and, for
//! institutional domains, `SCANOPY_INSTITUTION_TYPE` (`government` |
//! `education` | `healthcare` | `utility`). Nothing here performs I/O; all
//! data is vendored under `assets/domain-classification/` and refreshed with
//! the `refresh-data.py` script in that directory.
//!
//! Design bias is precision over recall: a missed institution classifies as
//! `company` (cheap, human review catches it), so every layer only fires on
//! high-confidence signals.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

const FREEMAIL_TXT: &str =
    include_str!("../../../assets/domain-classification/freemail-domains.txt");
const UNIVERSITIES_TXT: &str =
    include_str!("../../../assets/domain-classification/university-domains.txt");
const INSTITUTIONAL_CSV: &str =
    include_str!("../../../assets/domain-classification/institutional-domains.csv");
const OVERRIDES_JSON: &str =
    include_str!("../../../assets/domain-classification/institutional-overrides.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainClass {
    Freemail,
    Company,
    Institutional,
}

impl DomainClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainClass::Freemail => "freemail",
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

    fn parse(s: &str) -> Option<Self> {
        match s {
            "government" => Some(InstitutionType::Government),
            "education" => Some(InstitutionType::Education),
            "healthcare" => Some(InstitutionType::Healthcare),
            "utility" => Some(InstitutionType::Utility),
            _ => None,
        }
    }
}

fn data_lines(raw: &'static str) -> impl Iterator<Item = &'static str> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

static FREEMAIL: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| data_lines(FREEMAIL_TXT).collect());

static UNIVERSITIES: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| data_lines(UNIVERSITIES_TXT).collect());

static INSTITUTIONAL: LazyLock<HashMap<&'static str, InstitutionType>> = LazyLock::new(|| {
    data_lines(INSTITUTIONAL_CSV)
        .filter_map(|line| {
            let (domain, type_str) = line.split_once(',')?;
            Some((domain, InstitutionType::parse(type_str)?))
        })
        .collect()
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

/// Classify an email domain. Returns the domain class and, when
/// institutional, the institution type.
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

    // Generated institutional dataset (Wikidata / Annuaire / GSA). Matched
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
    fn every_dataset_entry_has_a_valid_type() {
        // Invariant over the vendored CSV, not per-entry assertions: every
        // line parses and every type is one of the four known values.
        let parsed = INSTITUTIONAL.len();
        let lines = data_lines(INSTITUTIONAL_CSV).count();
        assert_eq!(
            parsed, lines,
            "unparseable lines in institutional-domains.csv"
        );
        assert!(parsed > 10_000, "institutional dataset suspiciously small");
    }

    #[test]
    fn freemail_and_institutional_disjoint() {
        // A domain in both sets would make classification order-dependent.
        let overlap: Vec<_> = INSTITUTIONAL
            .keys()
            .filter(|d| FREEMAIL.contains(**d))
            .collect();
        assert!(
            overlap.is_empty(),
            "freemail/institutional overlap: {overlap:?}"
        );
    }
}
