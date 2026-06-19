# NDAA Section 889 Supply-Chain Review - Evidence

> Supporting evidence for a Section 889 compliance attestation. This records that
> an automated covered-entity review was performed over the SBOMs below.

**Result: PASS**

| Field | Value |
|-------|-------|
| Standard | NDAA FY2019 Section 889 (covered-entity components) |
| Generated (UTC) | 2026-06-19T02:17:29Z |
| Repository | `scanopy/scanopy` |
| Assessed commit | `5d9acd804e323f15cdcd6e9f92d00f6d21838205` (5d9acd8) |
| Components assessed | 8032 |
| Prohibited-entity hits | 0 |
| Reviewed exceptions | 2 |
| SBOM generator | syft 1.45.1 |
| Matcher | `scripts/check-889.sh` @ 5d9acd8 |
| Vendor list | `scripts/889-vendors.txt` @ 5d9acd8 (sha256 `90100a2e31daea18d39e689d7edae1ae68b812f5ca9fcecfd746a70d1eb1deab`) |

## Scope assessed

- Source tree at commit `5d9acd804e323f15cdcd6e9f92d00f6d21838205`
- Image: `server` (`latest`)
- Image: `daemon` (`latest`)

## Statement

The CycloneDX SBOM(s) listed below were generated with syft and assessed
with `scripts/check-889.sh` against the committed Section 889 covered-entity
vendor list. **No component originating with a covered entity (Huawei, ZTE,
Hytera, Hikvision, Dahua) or a known subsidiary/affiliate was found.**

2 reviewed exception(s) were suppressed as documented
false positives (see [889-allow.txt](889-allow.txt) and [summary.txt](summary.txt)).

## Files in this bundle

- [evidence.json](evidence.json) - machine-readable evidence record
- [summary.txt](summary.txt) - matcher human summary (counts + exceptions)
- [sbom-source.cdx.json](sbom-source.cdx.json) - CycloneDX SBOM
- [sbom-server.cdx.json](sbom-server.cdx.json) - CycloneDX SBOM
- [sbom-daemon.cdx.json](sbom-daemon.cdx.json) - CycloneDX SBOM
- [889-vendors.txt](889-vendors.txt), [889-allow.txt](889-allow.txt) - the exact policy used
- [SHA256SUMS](SHA256SUMS) - digests of every file above
