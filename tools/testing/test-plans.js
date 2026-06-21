var TEST_PLANS = [
{
  "branch": "fix/topology-view-switching",
  "tests": []
}
,
{
  "branch": "fix/misc-bugs-2026-06-21",
  "tests": []
}
,
{
  "branch": "feat/service-definitions-batch-2026-06",
  "tests": []
}
,
{
  "branch": "chore/docs-audit-since-v0.16.2",
  "tests": []
}
,
{
  "branch": "feat/snmpv3-support",
  "tests": []
}
,
{
  "branch": "fix/png-export-blur-artifacts",
  "tests": []
}
,
{
  "branch": "fix/topology-per-view-layouts",
  "tests": []
}
,
{
  "branch": "fix/snmp-lldp-discovery-bugs",
  "tests": []
}
,
{
  "branch": "fix/daemon-wizard-credential-ids",
  "tests": []
}
,
{
  "branch": "fix/multi-ip-host-on-single-mac",
  "tests": []
}
,
{
  "branch": "refactor/large-file-modularization",
  "tests": [
    {
      "id": "no-human-tests-required",
      "category": "Refactor — no behavior change",
      "description": "This branch is pure code-organization: 8 large backend files were split into per-responsibility submodules with no logic changes. Byte-for-byte preservation was verified by reassembling each split and diffing against the pre-split file (clean modulo documented `pub(crate)` visibility widening). Correctness is fully covered programmatically — `cargo test --lib` passes, `cargo check` compiles, `make format`/`make lint` (backend) clean. There is nothing for a human to click through: no UI, API, or runtime behavior changed. No human test steps are warranted.",
      "expected": "N/A — verified programmatically; no user-facing surface changed.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/snapshot-and-live-topology-bootstrap",
  "tests": [
    {
      "id": "snapshot-renders-element-nodes",
      "category": "Topology Snapshots",
      "description": "A snapshot converted from a v0.16.2 lock renders with element nodes (hosts/services/IPs), not just empty containers, across all four views.",
      "steps": [
        "Open the app and select the network whose v0.16.2 topology was locked.",
        "Open the Topology tab and switch to the snapshot (snapshot picker / converted lock).",
        "Cycle through all four perspectives: L3 Logical, L2 Physical, Workloads, Application.",
        "Confirm each view shows element nodes inside containers (not empty container boxes)."
      ],
      "setup": "Load /tmp/scanopy-v0.16.2-populated.sql into a fresh DB, lock two topologies on different networks (existing fixture-edit pattern), run `make migrate-db`, then boot the server once (this triggers the one-shot rebuild).",
      "expected": "Every view renders populated graphs: containers contain element nodes; dependency edges appear in the Application view; physical-link/neighbor edges appear in L2 Physical.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "empty-live-row-populates",
      "category": "Topology Live",
      "description": "A network whose legacy live topology row was empty/locked shows a populated live topology after the upgrade boot, without needing a new discovery.",
      "steps": [
        "Select a network that had no usable live topology row pre-upgrade (or whose live row was the locked one).",
        "Open the Topology tab on the live (non-snapshot) view.",
        "Confirm the graph is populated for all four perspectives immediately (no discovery run triggered)."
      ],
      "setup": "Same upgraded DB as the previous test, booted once.",
      "expected": "The live topology renders current entity data across all four views right after boot.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-grouping-and-layout-usable",
      "category": "Topology Snapshots",
      "description": "Converted-snapshot graphs are visually coherent: grouping (subnets, hosts, application tags) and edges look sensible even though saved layout/options were reset to defaults during the upgrade.",
      "steps": [
        "Open a converted snapshot's Application view.",
        "Confirm application-tag groups contain the expected services.",
        "Switch to L3 Logical and confirm hosts group under their subnets.",
        "Confirm there are no overlapping/garbled nodes that make the graph unreadable."
      ],
      "setup": "Same upgraded DB as the previous tests.",
      "expected": "Default grouping rules apply and the graph is legible; no crashes or blank panels.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    }
  ]
}
];
