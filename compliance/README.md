# Compliance artifacts

Supporting evidence and process docs for procurement compliance attestations.

- `ndaa-889/` — NDAA Section 889 supply-chain review evidence. Refreshed on
  every release by `.github/workflows/889-evidence.yml`. Methodology and
  source/refresh process: `tools/889/889-check.md`. On-demand bundling:
  `tools/889/889-evidence.sh`.

Files here are generated artifacts (SBOMs, hash manifests, evidence summaries),
not source. Treat them as a record, not as the spec.
