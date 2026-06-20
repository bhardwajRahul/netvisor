#!/bin/sh
# ==============================================================================
# 889-evidence.sh - generate NDAA Section 889 review-evidence bundle
# ==============================================================================
#
# Generates CycloneDX SBOMs (source tree + published container images) with syft,
# assesses them with check-889.sh against the committed vendor list, and writes a
# self-contained, hash-anchored EVIDENCE bundle: the supporting data showing that
# an automated covered-entity review was performed.
#
# NOTE: the *attestation* is the signed letter sent to the customer. This bundle
# is the supporting evidence that letter points to - not the attestation itself.
#
# The bundle records: result, what was assessed (commit + image digests), the
# tool versions, the exact vendor list / allowlist used (with digests), the full
# SBOMs, and a SHA256SUMS manifest. It EXITS NON-ZERO if the check fails, so it
# can never emit a clean evidence bundle for a tree that contains a hit.
#
# Dependencies: syft, jq, git, sha256sum (or shasum), POSIX shell.
#
# USAGE
#   scripts/889-evidence.sh [--out DIR] [--tag TAG] [--no-images] [--image REF]...
#
#   --out DIR     Output directory (default: 889-evidence-<UTC-date>).
#   --tag TAG     Image tag to assess (default: latest). Used to build the
#                 default server / server-commercial / daemon image refs.
#   --image REF   Scan this exact image ref instead of the defaults. Repeatable.
#   --repo SLUG   owner/repo for the ghcr image namespace (default: from remote).
#   --no-images   Source tree only (e.g. offline, or images not published yet).
#   -h, --help    Show this help.
#
# EXAMPLES
#   scripts/889-evidence.sh                       # source + the three :latest images
#   scripts/889-evidence.sh --tag v1.4.2          # assess a specific released tag
#   scripts/889-evidence.sh --no-images           # source tree only
# ==============================================================================

set -eu

SCRIPT_DIR=$(unset CDPATH; cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(unset CDPATH; cd -- "$SCRIPT_DIR/.." && pwd)
CHECK="$SCRIPT_DIR/check-889.sh"
VENDORS="$SCRIPT_DIR/889-vendors.txt"
ALLOW="$SCRIPT_DIR/889-allow.txt"

TAG="latest"
OUT=""
WANT_IMAGES=1
EXPLICIT_IMAGES=""
REPO_SLUG=""

usage() { sed -n '/^# USAGE$/,/^# ===/p' "$0" | sed 's/^# \{0,1\}//; s/^#$//'; }

while [ $# -gt 0 ]; do
    case "$1" in
        --out) OUT="$2"; shift 2 ;;
        --tag) TAG="$2"; shift 2 ;;
        --image) EXPLICIT_IMAGES="$EXPLICIT_IMAGES $2"; shift 2 ;;
        --repo) REPO_SLUG="$2"; shift 2 ;;
        --no-images) WANT_IMAGES=0; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "889-evidence.sh: unknown option: $1" >&2; exit 2 ;;
    esac
done

for tool in syft jq git; do
    command -v "$tool" >/dev/null 2>&1 || { echo "889-evidence.sh: $tool is required" >&2; exit 2; }
done

# Portable sha256 over a file -> bare hex digest.
sha256() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
    else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

cd "$REPO_ROOT"

NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
DATE=$(date -u +%Y-%m-%d)
[ -n "$OUT" ] || OUT="$REPO_ROOT/889-evidence-$DATE"
mkdir -p "$OUT"

GIT_COMMIT=$(git rev-parse HEAD)
GIT_DESCRIBE=$(git describe --tags --always --dirty 2>/dev/null || echo "$GIT_COMMIT")
# owner/repo slug for the ghcr image namespace; derive from the remote unless
# given explicitly (--repo). Handles https and ssh remotes, with/without .git.
[ -n "$REPO_SLUG" ] || REPO_SLUG=$(git config --get remote.origin.url 2>/dev/null \
    | sed -E 's#\.git$##; s#^.*github\.com[/:]##' || echo unknown)
[ -n "$REPO_SLUG" ] || REPO_SLUG=unknown
SYFT_VER=$(syft version 2>/dev/null | awk '/^Version:/ {print $2}')
CHECK_COMMIT=$(git log -1 --format=%h -- "$CHECK" 2>/dev/null || echo "uncommitted")
VENDORS_COMMIT=$(git log -1 --format=%h -- "$VENDORS" 2>/dev/null || echo "uncommitted")
VENDORS_SHA=$(sha256 "$VENDORS")
ALLOW_SHA=$(sha256 "$ALLOW")

# Copy the exact policy files used, for reproducibility.
cp "$VENDORS" "$OUT/889-vendors.txt"
cp "$ALLOW" "$OUT/889-allow.txt"

echo "889-evidence: generating source SBOM..." >&2
syft scan dir:. -o cyclonedx-json | jq . > "$OUT/sbom-source.cdx.json"

# Build the list of (label, target, file) SBOMs assessed.
SBOM_FILES="$OUT/sbom-source.cdx.json"

scan_image() {
    ref="$1"; label="$2"
    out="$OUT/sbom-$label.cdx.json"
    echo "889-evidence: scanning image $ref..." >&2
    if syft scan "registry:$ref" -o cyclonedx-json 2>/dev/null | jq . > "$out" 2>/dev/null && [ -s "$out" ]; then
        SBOM_FILES="$SBOM_FILES $out"
        printf '%s\t%s\t%s\n' "$label" "$ref" "$out" >> "$OUT/.images"
    else
        rm -f "$out"
        echo "889-evidence: WARNING could not scan $ref - recording as not-assessed" >&2
        printf '%s\t%s\t%s\n' "$label" "$ref" "UNAVAILABLE" >> "$OUT/.images"
    fi
}

: > "$OUT/.images"
if [ "$WANT_IMAGES" -eq 1 ]; then
    if [ -n "$EXPLICIT_IMAGES" ]; then
        i=0
        for ref in $EXPLICIT_IMAGES; do i=$((i+1)); scan_image "$ref" "image$i"; done
    else
        scan_image "ghcr.io/$REPO_SLUG/server:$TAG" "server"
        scan_image "ghcr.io/$REPO_SLUG/server-commercial:$TAG" "server-commercial"
        scan_image "ghcr.io/$REPO_SLUG/daemon:$TAG" "daemon"
    fi
fi

# Assess. Capture machine hits and the human summary (includes exceptions).
echo "889-evidence: running check-889.sh..." >&2
set +e
# shellcheck disable=SC2086
"$CHECK" --json $SBOM_FILES > "$OUT/hits.jsonl" 2> "$OUT/summary.txt"
RESULT_RC=$?
set -e
if [ "$RESULT_RC" -eq 0 ]; then RESULT="PASS"; else RESULT="FAIL"; fi

N_COMPONENTS=$(awk '/scanned/ {for (i=1;i<=NF;i++) if ($i=="scanned") {print $(i+1)+0; exit}}' "$OUT/summary.txt")
[ -n "$N_COMPONENTS" ] || N_COMPONENTS=0
N_EXCEPTIONS=$(awk '/exception\(s\) suppressed/ {print $2+0; f=1} END {if (!f) print 0}' "$OUT/summary.txt")
N_HITS=$(awk 'END {print NR+0}' "$OUT/hits.jsonl")
# On a PASS hits.jsonl is empty; drop it so the bundle (and SHA256SUMS) only
# contains real files. GitHub's asset upload API also rejects zero-byte files.
[ -s "$OUT/hits.jsonl" ] || rm -f "$OUT/hits.jsonl"

# Per-SBOM digests + component counts (JSON array).
SBOM_JSON=$(
    for f in $SBOM_FILES; do
        base=$(basename "$f")
        jq -n --arg file "$base" --arg sha "$(sha256 "$f")" \
              --argjson n "$(jq '.components | length' "$f")" \
              '{file:$file, sha256:$sha, components:$n}'
    done | jq -s '.'
)

# Images table (assessed + unavailable).
IMAGES_JSON=$(
    if [ -s "$OUT/.images" ]; then
        while IFS="$(printf '\t')" read -r label ref file; do
            jq -n --arg label "$label" --arg ref "$ref" \
                  --arg status "$( [ "$file" = "UNAVAILABLE" ] && echo not-assessed || echo assessed )" \
                  '{label:$label, ref:$ref, status:$status}'
        done < "$OUT/.images" | jq -s '.'
    else echo '[]'; fi
)
rm -f "$OUT/.images"

# Machine-readable evidence record.
jq -n \
  --arg document "supporting evidence (the attestation is a separate signed letter)" \
  --arg standard "NDAA FY2019 Section 889" \
  --arg generated_at "$NOW" \
  --arg repo "$REPO_SLUG" \
  --arg git_commit "$GIT_COMMIT" \
  --arg git_describe "$GIT_DESCRIBE" \
  --arg image_tag "$TAG" \
  --arg syft_version "$SYFT_VER" \
  --arg checker_commit "$CHECK_COMMIT" \
  --arg vendors_commit "$VENDORS_COMMIT" \
  --arg vendors_sha256 "$VENDORS_SHA" \
  --arg allow_sha256 "$ALLOW_SHA" \
  --arg result "$RESULT" \
  --argjson components "$N_COMPONENTS" \
  --argjson exceptions "$N_EXCEPTIONS" \
  --argjson hits "$N_HITS" \
  --argjson sboms "$SBOM_JSON" \
  --argjson images "$IMAGES_JSON" \
  '{
     document_type: $document,
     standard: $standard,
     result: $result,
     generated_at: $generated_at,
     repository: $repo,
     assessed_commit: $git_commit,
     assessed_describe: $git_describe,
     image_tag: $image_tag,
     components_assessed: $components,
     allowed_exceptions: $exceptions,
     prohibited_hits: $hits,
     tooling: { sbom_generator: ("syft " + $syft_version), matcher_commit: $checker_commit },
     policy: { vendor_list_commit: $vendors_commit, vendor_list_sha256: $vendors_sha256, allowlist_sha256: $allow_sha256 },
     images: $images,
     sboms: $sboms
   }' > "$OUT/evidence.json"

# Human-readable evidence record.
{
    echo "# NDAA Section 889 Supply-Chain Review - Evidence"
    echo
    echo "> Supporting evidence for a Section 889 compliance attestation. This records that"
    echo "> an automated covered-entity review was performed over the SBOMs below."
    echo
    echo "**Result: $RESULT**"
    echo
    echo "| Field | Value |"
    echo "|-------|-------|"
    echo "| Standard | NDAA FY2019 Section 889 (covered-entity components) |"
    echo "| Generated (UTC) | $NOW |"
    echo "| Repository | \`$REPO_SLUG\` |"
    echo "| Assessed commit | \`$GIT_COMMIT\` ($GIT_DESCRIBE) |"
    echo "| Components assessed | $N_COMPONENTS |"
    echo "| Prohibited-entity hits | $N_HITS |"
    echo "| Reviewed exceptions | $N_EXCEPTIONS |"
    echo "| SBOM generator | syft $SYFT_VER |"
    echo "| Matcher | \`scripts/check-889.sh\` @ $CHECK_COMMIT |"
    echo "| Vendor list | \`scripts/889-vendors.txt\` @ $VENDORS_COMMIT (sha256 \`$VENDORS_SHA\`) |"
    echo
    echo "## Scope assessed"
    echo
    echo "- Source tree at commit \`$GIT_COMMIT\`"
    if [ "$WANT_IMAGES" -eq 1 ]; then
        for f in $SBOM_FILES; do
            case "$(basename "$f")" in
                sbom-source.cdx.json) ;;
                sbom-*.cdx.json) echo "- Image: \`$(basename "$f" | sed 's/^sbom-//; s/\.cdx\.json$//')\` (\`$TAG\`)" ;;
            esac
        done
    else
        echo "- (images not assessed: --no-images)"
    fi
    echo
    echo "## Statement"
    echo
    if [ "$RESULT" = "PASS" ]; then
        echo "The CycloneDX SBOM(s) listed below were generated with syft and assessed"
        echo "with \`scripts/check-889.sh\` against the committed Section 889 covered-entity"
        echo "vendor list. **No component originating with a covered entity (Huawei, ZTE,"
        echo "Hytera, Hikvision, Dahua) or a known subsidiary/affiliate was found.**"
        if [ "$N_EXCEPTIONS" -gt 0 ]; then
            echo
            echo "$N_EXCEPTIONS reviewed exception(s) were suppressed as documented"
            echo "false positives (see [889-allow.txt](889-allow.txt) and [summary.txt](summary.txt))."
        fi
    else
        echo "**One or more prohibited-entity components were found.** See"
        echo "[hits.jsonl](hits.jsonl). This evidence bundle is NOT clean."
    fi
    echo
    echo "## Files in this bundle"
    echo
    echo "- [evidence.json](evidence.json) - machine-readable evidence record"
    echo "- [summary.txt](summary.txt) - matcher human summary (counts + exceptions)"
    [ -f "$OUT/hits.jsonl" ] && echo "- [hits.jsonl](hits.jsonl) - machine-readable hits (present only when a hit is found)"
    for f in $SBOM_FILES; do
        base=$(basename "$f")
        echo "- [$base]($base) - CycloneDX SBOM"
    done
    echo "- [889-vendors.txt](889-vendors.txt), [889-allow.txt](889-allow.txt) - the exact policy used"
    echo "- [SHA256SUMS](SHA256SUMS) - digests of every file above"
} > "$OUT/EVIDENCE.md"

# Digest manifest over everything in the bundle.
( cd "$OUT" && for f in *; do [ "$f" = "SHA256SUMS" ] || sha256 "$f" | sed "s|\$| $f|"; done > SHA256SUMS )

echo "889-evidence: $RESULT - bundle written to $OUT" >&2
exit "$RESULT_RC"
