#!/usr/bin/env bash
# Refresh the vendored data files embedded into the server binary:
#   backend/assets/oui.csv                       IEEE MAC-address registry
#   backend/assets/domain-classification/*       email-domain classification sources
#
# Curl-only file drops — all parsing/normalization/merging happens at load
# time in backend/src/server/brevo/domain_classification.rs (and oui.rs), so
# this script never transforms data beyond flattening two JSON files to
# domain-per-line with jq.
#
# Invoked by `make refresh-vendored-data` and by the release workflow
# (.github/workflows/release.yml, test-and-update-fixtures job), which
# commits the refreshed files back to the release branch after `cargo test`
# validates them (the classifier has floor + ground-truth invariant tests).
#
# Failure semantics: each source is independent and best-effort. A failed
# download or a result below its sanity floor keeps the committed copy and
# warns; the script only exits non-zero on environment errors (missing curl/
# jq). institutional-overrides.json is hand-maintained and never touched.
#
# Requires: curl, jq.

set -u
cd "$(dirname "$0")/.." || exit 1

DC_DIR=assets/domain-classification
CURL=(curl -sfL --max-time 300 --retry 5 --retry-delay 70)
WIKIDATA=https://query.wikidata.org/sparql
UA="ScanopyVendoredDataRefresh/1.0 (release workflow)"

command -v jq >/dev/null || { echo "ERROR: jq is required" >&2; exit 1; }
command -v curl >/dev/null || { echo "ERROR: curl is required" >&2; exit 1; }

# install <tmp-file> <dest> <line-floor> — move tmp into place if it clears
# the sanity floor, else keep the committed copy.
install_if_sane() {
    local tmp="$1" dest="$2" floor="$3" lines
    lines=$(wc -l < "$tmp" | tr -d ' ')
    if [ "$lines" -ge "$floor" ]; then
        mv "$tmp" "$dest"
        echo "refreshed $dest ($lines lines)"
    else
        echo "WARN: $dest refresh has $lines lines (floor $floor) — keeping committed copy" >&2
        rm -f "$tmp"
    fi
}

# fetch <dest> <floor> <url> [jq-filter] — plain download, optional jq flatten.
fetch() {
    local dest="$1" floor="$2" url="$3" filter="${4:-}" tmp
    tmp=$(mktemp)
    if "${CURL[@]}" -A "$UA" -o "$tmp" "$url"; then
        if [ -n "$filter" ]; then
            if ! jq -r "$filter" "$tmp" > "$tmp.flat"; then
                echo "WARN: $dest jq flatten failed — keeping committed copy" >&2
                rm -f "$tmp" "$tmp.flat"
                return
            fi
            mv "$tmp.flat" "$tmp"
        fi
        install_if_sane "$tmp" "$dest" "$floor"
    else
        echo "WARN: $dest download failed — keeping committed copy" >&2
        rm -f "$tmp"
    fi
}

# wikidata <dest> <floor> <sparql-query...> — one CSV per query, concatenated
# (each keeps its own "site" header line; the loader skips non-URL lines).
wikidata() {
    local dest="$1" floor="$2" tmp ok=1
    shift 2
    tmp=$(mktemp)
    for query in "$@"; do
        if ! "${CURL[@]}" -A "$UA" -H "Accept: text/csv" \
            --data-urlencode "query=$query" -G "$WIKIDATA" >> "$tmp"; then
            ok=0
            break
        fi
        sleep 2 # politeness between queries
    done
    if [ "$ok" = 1 ]; then
        install_if_sane "$tmp" "$dest" "$floor"
    else
        echo "WARN: $dest download failed — keeping committed copy" >&2
        rm -f "$tmp"
    fi
}

sites_query() { # instances (incl. subclasses) of $1 with an official website
    echo "SELECT DISTINCT ?site WHERE { ?item wdt:P31/wdt:P279* wd:$1 . ?item wdt:P856 ?site }"
}

# --- IEEE OUI registry (MAC-address vendor lookup) ---
tmp=$(mktemp)
if "${CURL[@]}" -o "$tmp" "https://standards-oui.ieee.org/oui/oui.csv" \
    && head -1 "$tmp" | grep -q "Registry"; then
    install_if_sane "$tmp" assets/oui.csv 20000
else
    echo "WARN: assets/oui.csv download failed or malformed — keeping committed copy" >&2
    rm -f "$tmp"
fi

# --- Freemail providers (Kikobeats/free-email-domains, MIT) ---
fetch "$DC_DIR/freemail-domains.txt" 10000 \
    "https://raw.githubusercontent.com/Kikobeats/free-email-domains/master/domains.json" \
    '.[]'

# --- World universities (Hipo/university-domains-list, MIT; names dropped) ---
fetch "$DC_DIR/university-domains.txt" 8000 \
    "https://raw.githubusercontent.com/Hipo/university-domains-list/master/world_universities_and_domains.json" \
    '.[].domains[]'

# --- Wikidata official websites (CC0) ---
wikidata "$DC_DIR/wikidata-healthcare.csv" 8000 \
    "$(sites_query Q16917)" # hospitals
wikidata "$DC_DIR/wikidata-utility.csv" 400 \
    "$(sites_query Q1951366)" \
    "$(sites_query Q2127330)" # public utilities + public transport companies
wikidata "$DC_DIR/wikidata-government.csv" 50000 \
    "$(sites_query Q15284)" # municipalities

# --- Annuaire de l'administration (FR local administrations, Licence Ouverte v2).
#     URL column only: the email column carries ISP mailbox domains. ---
fetch "$DC_DIR/annuaire-fr.csv" 15000 \
    "https://public.opendatasoft.com/api/explore/v2.1/catalog/datasets/annuaire-de-ladministration-base-de-donnees-locales/exports/csv?select=coordonneesnum_url&delimiter=%3B"

# --- GSA govt-urls (US government sites incl. non-.gov, US public domain) ---
fetch "$DC_DIR/gsa-govt-urls.csv" 5000 \
    "https://raw.githubusercontent.com/GSA/govt-urls/master/1_govt_urls_full.csv"

echo "done"
