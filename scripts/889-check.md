# NDAA Section 889 supply-chain check

Automated evidence that Scanopy's dependency tree and built container images
contain no components originating with entities prohibited under **NDAA Section
889** (Huawei, ZTE, Hytera, Hikvision, Dahua) or their subsidiaries / affiliates.
The output is supporting evidence for a Section 889 compliance attestation.

## Pieces

| File | Role |
|------|------|
| `scripts/check-889.sh` | Matcher. POSIX shell + `jq`, no network. Scans CycloneDX SBOM(s), fails on a hit. |
| `scripts/889-vendors.txt` | Maintained prohibited-vendor pattern list (single source of truth). |
| `scripts/889-allow.txt` | Reviewed false-positive exceptions (only subtracts, never broadens). |
| `.github/workflows/889-check.yml` | PR gate — analyze-only source SBOM, blocks merge on a hit. |
| `.github/workflows/889-check-test.yml` | `workflow_dispatch` end-to-end exercise (source + image). |
| `.github/workflows/889-vendors-refresh.yml` | Quarterly cron — opens a vendor-list review issue. |
| `.github/workflows/release.yml` (`supply-chain-889` job) | Release gate — scans source + all images, uploads SBOMs as release assets. |

## Running locally

```sh
# Source tree. Pretty-print so the matcher reports meaningful file:line.
syft scan dir:. -o cyclonedx-json | jq . > sbom-source.cdx.json
./scripts/check-889.sh sbom-source.cdx.json

# A built image (syft pulls from the registry directly, no docker daemon needed).
syft scan registry:ghcr.io/scanopy/scanopy/server:latest -o cyclonedx-json | jq . > server.cdx.json
./scripts/check-889.sh --json server.cdx.json
```

`syft` emits minified single-line CycloneDX by default; pipe through `jq .` so
`file:line` in hit output points at the offending component. The matcher falls
back to line 1 on minified input. `./scripts/check-889.sh --help` documents all
options. Exit codes: `0` clean, `1` hit found, `2` usage/dependency error.

## Vendor list — seed methodology

`889-vendors.txt` entries are `pattern;source;date;note`. `pattern` is an ERE
matched case-insensitively against every component's identity fields (name,
group, publisher, author, supplier.name, purl, externalReference URLs). Seeded
from, in order of authority:

1. **Named covered entities** (`source=named-entity`) — the five entities named
   in NDAA Sec. 889(f)(3): Huawei, ZTE, Hytera, Hikvision, Dahua. Short/ambiguous
   tokens (ZTE, Nubia, Lorex) use portable word boundaries `(^|[^a-z0-9])…([^a-z0-9]|$)`
   to avoid matching substrings (e.g. "azteca" must not match "ZTE").
2. **Known subsidiaries / brand names** (`source=internal review`) — entities
   traceable to a covered entity: HiSilicon, HarmonyOS/OpenHarmony (Huawei);
   EZVIZ (Hikvision); Nubia (ZTE); Lorex (Dahua); plus legal-entity name variants.
3. **SAM.gov Exclusions, FAR 52.204-25** (`source=SAM.gov`) — federal excluded
   parties tagged to the 889 procurement clause. Seed pulled from the SAM.gov
   Exclusions extract: <https://sam.gov/data-services/Exclusions/Public>
   (Entity Management → Exclusions, filter cause/clause `FAR 52.204-25`).
4. **ASPI "Mapping China's Tech Giants"** (`source=ASPI`) — affiliate mapping used
   only where it adds patterns the lists above miss.
   <https://chinatechmap.aspi.org.au/>

Each row is date-stamped with its add/last-review date.

## Allowlist — false positives

`889-allow.txt` suppresses components a vendor pattern flags but that have been
reviewed and confirmed *not* to be covered-entity components. It only ever
subtracts documented exceptions; never add an entry to silence a real or
unexamined hit. Every entry cites why.

**Known false positive (seeded):** `@esbuild/openharmony-arm64` and
`@rollup/rollup-openharmony-arm64`. esbuild and rollup publish one optional
package per cross-compilation target; one target is Huawei's OpenHarmony OS.
These packages are published by the esbuild / rollup OSS projects (not Huawei)
and carry only that bundler's prebuilt binary. They match the `openharmony`
vendor pattern but are not covered-entity components, so they are allowlisted by
version-independent name. Disabling the allowlist (`--allow /dev/null`) makes the
current source SBOM fail on exactly these two — a convenient live demonstration
that the matcher catches pattern hits.

## Refresh mechanism

The covered-entity list changes over time (SAM.gov publishes updates; affiliates
are added). `.github/workflows/889-vendors-refresh.yml` runs **quarterly** and
opens (or updates) a tracking issue prompting a maintainer to re-pull SAM.gov
(FAR 52.204-25) and review ASPI, then update `889-vendors.txt` with new
patterns + date stamps.

**Why a review issue, not an automated fetch-and-PR (deferred):** a fully
automated job that fetches the SAM.gov dataset and opens a diff PR was
considered and deferred. The SAM.gov Entity Management API requires a registered
API key, and the Exclusions extract is a large structured download that needs
FAR-cause filtering and name-to-pattern translation — fragile to run unattended
on a quarterly cadence, and the matcher must stay offline (no run-time fetch).
A quarterly human review is the right cost/benefit; automating the fetch is a
separate future task if the manual cadence proves too slow.

## SBOM storage decision

**Released SBOMs are uploaded as GitHub Release assets, not committed to git.**

TASK.md assumed committing the SBOM to the public repo is fine because it only
contains public OSS dependency metadata. That privacy assessment holds — a
CycloneDX SBOM from our source + images contains package names, versions, purls,
and licenses for public OSS crates / npm packages / Debian base packages, plus
our own workspace crate names (already public in this repo) and syft-derived
file paths. Nothing secret: no credentials, no private registry tokens, no
customer data.

Privacy is therefore *not* the deciding factor. The decision is driven by:

- **History churn** — SBOMs regenerate every release; committing ~1k-component
  JSON blobs each tag bloats git history for no diff value.
- **Existing mechanism** — the release workflow already attaches assets via the
  `upload-assets` job (`softprops/action-gh-release@v1`). SBOMs ride that path.
- **Discoverability** — assets are downloadable from the Releases tab, the
  natural place an auditor looks for per-release compliance evidence.

If a future requirement needs versioned-in-history SBOMs (e.g. an internal audit
trail diffable in git), revisit by committing to a separate private compliance
repo rather than the public source repo.

## Images scanned at release

The `supply-chain-889` job in `release.yml` runs after `create-manifests` and
scans the released source tree plus **all three** published images:

- `ghcr.io/<repo>/server` (community)
- `ghcr.io/<repo>/server-commercial`
- `ghcr.io/<repo>/daemon`

All three are first-party artifacts shipped to users, so all three are in scope.
The external `postgres` base image is pulled, not built by the release, and is
out of scope here; the Debian base packages inside our images *are* captured
when those images are scanned.

**Known tradeoff:** buildx pushes each image during its build job, so a scan
that runs after `create-manifests` cannot un-push a bad image. On a hit it fails
the release status and skips the SBOM upload, and gates the separate, manual
`promote_to_latest` workflow — an effective gate even though the per-arch tags
are already in GHCR. Moving the scan strictly before push would require scanning
each per-arch image inside its matrix build job before `push: true`; deferred as
a refinement (see Follow-ups).

## Follow-ups (separate tasks)

- **Pre-push image scan** — scan each image in its matrix build job before
  `push: true` so a hit prevents the image reaching GHCR at all.
- **Automated SAM.gov fetch-and-PR** — replace the quarterly review issue with a
  job that pulls the FAR 52.204-25 extract and opens a diff PR (needs a SAM.gov
  API key + name-to-pattern translation).
- **Optional FOSSA license/CVE scanning** — orthogonal to 889 (license policy
  and vulnerabilities, not covered-entity identity); a separate effort if wanted.
