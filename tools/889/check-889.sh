#!/bin/sh
# ==============================================================================
# check-889.sh — NDAA Section 889 supply-chain matcher
# ==============================================================================
#
# Scans one or more CycloneDX SBOM JSON files for components that originate with
# entities prohibited under NDAA Section 889 (Huawei, ZTE, Hytera, Hikvision,
# Dahua) or their subsidiaries / affiliates. Matches each component's identity
# fields (name, group, publisher, author, supplier.name, purl, externalReference
# URLs) against a maintained vendor pattern list. Exits non-zero on any hit and
# prints the offending component(s) with the SBOM file:line for traceability.
#
# The output is supporting evidence for a Section 889 compliance attestation.
#
# Dependencies: POSIX shell + `jq`. No network access. Runs anywhere.
#
# ------------------------------------------------------------------------------
# USAGE
# ------------------------------------------------------------------------------
#   check-889.sh [OPTIONS] SBOM.json [SBOM2.json ...]
#
# OPTIONS
#   --vendors FILE   Vendor pattern list (default: 889-vendors.txt next to this
#                    script). See that file for the format.
#   --allow FILE     Allowlist of reviewed false-positive exceptions (default:
#                    889-allow.txt next to this script, if present). Components
#                    whose identity matches an allow pattern are suppressed and
#                    reported as exceptions, not failures. Same format as the
#                    vendor list. Pass /dev/null to disable.
#   --json           Emit one JSON object per hit on stdout (machine-parseable).
#                    Without it, hits print as human-readable lines on stdout.
#                    A human summary always goes to stderr.
#   --quiet          Suppress the human summary on stderr (exit code still set;
#                    --json output still emitted).
#   -h, --help       Show this help and exit.
#
# EXIT STATUS
#   0   No prohibited components found in any SBOM.
#   1   At least one prohibited component found.
#   2   Usage error / missing dependency / malformed input.
#
# EXAMPLES
#   # Generate an SBOM with syft and check it:
#   syft scan dir:. -o cyclonedx-json=sbom.cdx.json
#   ./tools/889/check-889.sh sbom.cdx.json
#
#   # Check a built container image's SBOM, machine-readable output:
#   syft scan registry:ghcr.io/scanopy/scanopy/server:latest \
#     -o cyclonedx-json=server.cdx.json
#   ./tools/889/check-889.sh --json server.cdx.json | tee hits.jsonl
#
#   # Check several SBOMs at once with a custom vendor list:
#   ./tools/889/check-889.sh --vendors my-vendors.txt a.cdx.json b.cdx.json
# ==============================================================================

set -eu

PROG="check-889.sh"
SCRIPT_DIR=$(unset CDPATH; cd -- "$(dirname -- "$0")" && pwd)
VENDORS="$SCRIPT_DIR/889-vendors.txt"
ALLOW="$SCRIPT_DIR/889-allow.txt"
JSON=0
QUIET=0

usage() {
    # Print the header comment block (between the USAGE banner and the closing
    # banner) so --help and the source stay in sync.
    sed -n '/^# USAGE$/,/^# ===/p' "$0" | sed 's/^# \{0,1\}//; s/^#$//'
}

err() { printf '%s: %s\n' "$PROG" "$1" >&2; }

# ---- argument parsing --------------------------------------------------------
SBOMS=""
while [ $# -gt 0 ]; do
    case "$1" in
        --vendors) [ $# -ge 2 ] || { err "--vendors needs a value"; exit 2; }; VENDORS="$2"; shift 2 ;;
        --vendors=*) VENDORS="${1#*=}"; shift ;;
        --allow) [ $# -ge 2 ] || { err "--allow needs a value"; exit 2; }; ALLOW="$2"; shift 2 ;;
        --allow=*) ALLOW="${1#*=}"; shift ;;
        --json) JSON=1; shift ;;
        --quiet) QUIET=1; shift ;;
        -h|--help) usage; exit 0 ;;
        --) shift; while [ $# -gt 0 ]; do SBOMS="$SBOMS $1"; shift; done ;;
        -*) err "unknown option: $1"; exit 2 ;;
        *) SBOMS="$SBOMS $1"; shift ;;
    esac
done

# ---- preconditions -----------------------------------------------------------
command -v jq >/dev/null 2>&1 || { err "jq is required but not found on PATH"; exit 2; }
[ -n "$SBOMS" ] || { err "no SBOM file given"; usage >&2; exit 2; }
[ -f "$VENDORS" ] || { err "vendor list not found: $VENDORS"; exit 2; }

# ---- load vendor patterns ----------------------------------------------------
# Vendor file format (one entry per line): pattern;source;date;note
#   - pattern is an extended regular expression (ERE), matched case-insensitively
#   - lines beginning with '#' and blank lines are ignored
# Loaded into parallel temp files so we can attribute each hit to its source.
# Field separator for our intermediate TSV-like files: ASCII Unit Separator
# (0x1F). It is non-whitespace, so empty columns are preserved by `read` (a tab
# is IFS-whitespace and would collapse adjacent empty fields, misaligning rows).
US=$(printf '\037')
TMP=$(mktemp -d "${TMPDIR:-/tmp}/check889.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM
PAT_FILE="$TMP/patterns"
: >"$PAT_FILE"
npat=0
while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
        ''|'#'*) continue ;;
    esac
    pattern=${line%%;*}
    rest=${line#*;}
    source=${rest%%;*}
    [ -n "$pattern" ] || continue
    printf '%s%s%s\n' "$pattern" "$US" "$source" >>"$PAT_FILE"
    npat=$((npat + 1))
done <"$VENDORS"
[ "$npat" -gt 0 ] || { err "vendor list has no usable patterns: $VENDORS"; exit 2; }

# ---- load allowlist (reviewed false-positive exceptions) ---------------------
# Same format as the vendor list. A component flagged by a vendor pattern is
# suppressed (reported as an exception, not a failure) if its identity matches
# any allow pattern. Strict-by-default: the allowlist only ever subtracts known,
# documented false positives — it never broadens what is considered prohibited.
ALLOW_PAT="$TMP/allow"
: >"$ALLOW_PAT"
nallow=0
if [ -f "$ALLOW" ]; then
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            ''|'#'*) continue ;;
        esac
        ap=${line%%;*}
        [ -n "$ap" ] || continue
        printf '%s\n' "$ap" >>"$ALLOW_PAT"
        nallow=$((nallow + 1))
    done <"$ALLOW"
fi

# ---- scan each SBOM ----------------------------------------------------------
total_components=0
total_hits=0
total_allowed=0
nsbom=0

emit_hit() {
    # $1 sbom  $2 line  $3 component  $4 version  $5 purl  $6 field  $7 pattern  $8 source
    if [ "$JSON" -eq 1 ]; then
        jq -cn \
            --arg sbom "$1" --argjson line "$2" --arg component "$3" \
            --arg version "$4" --arg purl "$5" --arg matched_field "$6" \
            --arg pattern "$7" --arg source "$8" \
            '{sbom:$sbom,line:$line,component:$component,version:$version,purl:$purl,matched_field:$matched_field,pattern:$pattern,source:$source}'
    else
        printf '%s:%s  %s@%s  [%s matches /%s/ (%s)]\n' \
            "$1" "$2" "$3" "${4:-?}" "$6" "$7" "$8"
    fi
}

for sbom in $SBOMS; do
    if [ ! -f "$sbom" ]; then
        err "SBOM not found: $sbom"; exit 2
    fi
    if ! jq -e 'has("components")' "$sbom" >/dev/null 2>&1; then
        err "not a CycloneDX SBOM (no .components): $sbom"; exit 2
    fi
    nsbom=$((nsbom + 1))

    # Flatten components to TSV: bom-ref, name, version, group, publisher,
    # author, supplier.name, purl, joined externalReference URLs.
    COMP_TSV="$TMP/components.tsv"
    jq -r '
        .components[]? | [
            (."bom-ref" // ""),
            (.name // ""),
            (.version // ""),
            (.group // ""),
            (.publisher // ""),
            (.author // ""),
            (.supplier.name // ""),
            (.purl // ""),
            ([.externalReferences[]?.url] | join(" "))
        ] | map(tostring) | join("\u001f")
    ' "$sbom" >"$COMP_TSV"
    ncomp=$(wc -l <"$COMP_TSV" | tr -d ' ')
    total_components=$((total_components + ncomp))

    # For each vendor pattern, grep the flattened component rows once. Hits are
    # rare (usually none), so the per-hit field/line resolution below is cheap.
    while IFS="$US" read -r pattern source; do
        grep -i -E -- "$pattern" "$COMP_TSV" 2>/dev/null >"$TMP/match" || true
        [ -s "$TMP/match" ] || continue
        while IFS="$US" read -r bomref name version group publisher author supplier purl urls; do
            # Identify which field actually matched, for the report.
            field="identity"
            for fv in "name=$name" "group=$group" "publisher=$publisher" \
                      "author=$author" "supplier=$supplier" "purl=$purl" "url=$urls" "bom-ref=$bomref"; do
                fname=${fv%%=*}; fval=${fv#*=}
                if [ -n "$fval" ] && printf '%s' "$fval" | grep -i -E -q -- "$pattern" 2>/dev/null; then
                    field="$fname"; break
                fi
            done
            # Suppress reviewed false positives (allowlist).
            if [ "$nallow" -gt 0 ]; then
                allowhay="$name $group $publisher $author $supplier $purl $urls $bomref"
                if printf '%s' "$allowhay" | grep -i -E -q -f "$ALLOW_PAT" 2>/dev/null; then
                    total_allowed=$((total_allowed + 1))
                    [ "$QUIET" -eq 1 ] || printf '%s: allowed exception — %s@%s (%s)\n' \
                        "$PROG" "$name" "${version:-?}" "$purl" >&2
                    continue
                fi
            fi
            # Resolve SBOM file:line via the component's unique bom-ref (falls
            # back to line 1 for minified single-line SBOMs).
            lineno=1
            if [ -n "$bomref" ]; then
                ln=$(grep -n -F -- "$bomref" "$sbom" 2>/dev/null | head -n1 | cut -d: -f1 || true)
                [ -n "$ln" ] && lineno="$ln"
            fi
            emit_hit "$sbom" "$lineno" "$name" "$version" "$purl" "$field" "$pattern" "$source"
            total_hits=$((total_hits + 1))
        done <"$TMP/match"
    done <"$PAT_FILE"
done

# ---- summary -----------------------------------------------------------------
if [ "$QUIET" -ne 1 ]; then
    {
        printf '%s: scanned %d component(s) across %d SBOM(s) against %d pattern(s)\n' \
            "$PROG" "$total_components" "$nsbom" "$npat"
        [ "$total_allowed" -eq 0 ] || printf '%s: %d allowed exception(s) suppressed\n' \
            "$PROG" "$total_allowed"
        if [ "$total_hits" -eq 0 ]; then
            printf '%s: PASS — no NDAA Section 889 prohibited components found\n' "$PROG"
        else
            printf '%s: FAIL — %d prohibited component hit(s) found (see above)\n' "$PROG" "$total_hits"
        fi
    } >&2
fi

[ "$total_hits" -eq 0 ]
