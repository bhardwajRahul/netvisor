var TEST_PLANS = [
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
,
{
  "branch": "refactor/topology-build-on-request",
  "tests": [
    {
      "id": "live-topology-renders",
      "category": "Rendering",
      "description": "Live topology renders with correct structure + grouping after the build-on-request change",
      "steps": [
        "Open the Topology tab and select a network with discovered hosts/services",
        "Confirm nodes, containers, grouping (subnets/apps/etc.) and edges render as before",
        "Switch between L3 Logical / Workloads / L2 Physical / Application views"
      ],
      "setup": "Ensure the selected network has hosts, services, subnets and at least one dependency. If empty, run a discovery (or create entities via the API) so the graph has content.",
      "expected": "Graph renders identically to prior behavior; view switching is instant (client-side slice) with no flicker or refetch spinner.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "entity-change-reflects",
      "category": "Live updates",
      "description": "Entity changes reflect on next request via SSE ping (no subscriber rebuild)",
      "steps": [
        "With the Topology tab open on a network, add or remove a host/service (or run a discovery)",
        "Watch the canvas without manually reloading the page"
      ],
      "setup": "Trigger an entity change on the open network: run a discovery, or create/delete a host or service via the API for that network.",
      "expected": "Within a couple seconds the topology refetches and re-renders to include/exclude the changed entity (driven by the SSE live-update ping).",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "options-persist-reload",
      "category": "Options",
      "description": "Grouping/options edits persist across reload (built graph reflects them)",
      "steps": [
        "Open the options panel and change a grouping rule, a hide filter, and a visual toggle",
        "Confirm the graph updates to reflect the new grouping",
        "Reload the page and reopen the topology"
      ],
      "expected": "After reload the same grouping/options are applied and the graph rebuilds to match (options persisted on the topology row).",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-renders",
      "category": "Snapshots",
      "description": "Snapshot view builds on request from closed copies (no stored snapshot graph)",
      "steps": [
        "Take a snapshot (Camera button) on a populated network",
        "Select the snapshot from the snapshot dropdown",
        "Confirm the graph renders, then switch views within the snapshot",
        "Change live entities (discovery/API), then re-select the snapshot"
      ],
      "setup": "Network must have hosts/services and snapshots enabled on the plan. Take the snapshot via the UI Camera button (or POST /api/v1/snapshots).",
      "expected": "Snapshot renders the as-of-capture graph; available views are restricted to what the snapshot captured; the snapshot is unaffected by later live entity changes.",
      "flow": "setup",
      "sequence": 4,
      "status": null,
      "feedback": null
    },
    {
      "id": "share-embed-renders",
      "category": "Shares",
      "description": "Public share / embed renders via the unified toRenderableTopology path",
      "steps": [
        "Create a share for a populated live topology (Share button)",
        "Open the public share URL in a logged-out browser/incognito window",
        "Switch views in the share viewer",
        "Repeat with embed mode (and with a password-protected share)"
      ],
      "setup": "Create a share via the UI for a network with rendered topology. For the password case, set a password on the share.",
      "expected": "Share/embed renders the same graph + entities as the app; view switching works; password gate works. (Backend now returns the slim row + bundle and the viewer composes them client-side.)",
      "flow": "setup",
      "sequence": 5,
      "status": null,
      "feedback": null
    },
    {
      "id": "exports-match",
      "category": "Export",
      "description": "Mermaid/Confluence exports match the rendered graph (built on request)",
      "steps": [
        "Open the Export modal on a live topology",
        "Export Mermaid and Confluence for the current view",
        "Compare node/edge content against the on-screen graph"
      ],
      "expected": "Exported Mermaid/Confluence content matches the rendered nodes/edges for the selected view.",
      "flow": "setup",
      "sequence": 6,
      "status": null,
      "feedback": null
    },
    {
      "id": "overrides-disabled",
      "category": "Disabled overrides",
      "description": "Node drag / container resize / edge reconnect do not persist (feature disabled)",
      "steps": [
        "On a live topology, confirm there is no enabled edit-mode affordance to move/resize nodes",
        "If any drag/resize is possible, perform it, then reload the page"
      ],
      "expected": "No way to persist layout changes; after reload the layout is the freshly-computed ELK layout (no saved positions/sizes/handles).",
      "flow": "setup",
      "sequence": 7,
      "status": null,
      "feedback": null
    },
    {
      "id": "bytag-pill-name",
      "category": "Grouping labels",
      "description": "ByTag grouping pill shows the tag name even for a tag applied to no entity",
      "steps": [
        "Open the topology options → Group tab on a populated network",
        "Edit the ByTag element rule and add a tag that is NOT applied to any host/service/subnet (in demo data, add 'Development' to the 'Critical' rule)",
        "Inspect the resulting grouping subgroup's tag pills"
      ],
      "setup": "Use a network with the demo grouping data (a ByTag rule with the 'Critical' tag). Ensure a second tag (e.g. 'Development') exists but is applied to no entity.",
      "expected": "The added tag's pill renders its name + color (e.g. 'Development'), not the raw UUID. Selecting a snapshot also shows resolved names.",
      "flow": "setup",
      "sequence": 8,
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-grouping-readonly",
      "category": "Snapshots",
      "description": "Grouping-rule editor is fully read-only while viewing a snapshot",
      "steps": [
        "On a populated network, take a snapshot and select it",
        "Open the topology options → Group tab",
        "Inspect the container + element grouping rules"
      ],
      "setup": "Network with hosts/services and snapshots enabled; take the snapshot via the Camera button.",
      "expected": "An info banner says grouping rules are read-only while viewing a snapshot; no edit (pencil), add, remove, or reorder controls are available. Returning to the live view restores full editing.",
      "flow": "setup",
      "sequence": 9,
      "status": null,
      "feedback": null
    },
    {
      "id": "share-on-snapshot-notice",
      "category": "Shares",
      "description": "Share modal is available on a snapshot with a live-view notice",
      "steps": [
        "Select a snapshot on a populated network",
        "Click the Share button and open the share modal",
        "Read the inline notice; create a share and open its public URL"
      ],
      "setup": "Network with topology + snapshots; email-verified user on a plan with share_views.",
      "expected": "The Share button is visible in snapshot view; the modal shows an inline info that the share reflects the live view (not the snapshot); the created share renders the live topology.",
      "flow": "setup",
      "sequence": 10,
      "status": null,
      "feedback": null
    }
  ]
}
];
