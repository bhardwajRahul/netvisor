# Compliance artifacts

Supporting evidence and process docs for procurement compliance attestations.

- `ndaa-889/` — NDAA Section 889 supply-chain review evidence. Refreshed on
  every release by `.github/workflows/889-evidence.yml`. Methodology and
  source/refresh process: `tools/889/889-check.md`. On-demand bundling:
  `tools/889/889-evidence.sh`.
- `accepted-risks.md` — known-vulnerable dependencies that ship on purpose, with
  the blast radius, what blocks the fix, and what would reverse the decision.
  Written by hand; reviewed each release and whenever a new advisory lands on a
  component already listed.

Generated artifacts here (SBOMs, hash manifests, evidence summaries) are a
record, not a spec. `accepted-risks.md` is the exception: it is authored, and it
is the decision of record for the advisories it covers.
