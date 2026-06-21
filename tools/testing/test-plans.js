var TEST_PLANS = [
{
  "branch": "fix/topology-view-switching",
  "tests": [
    {
      "id": "tags-survive-snapshot",
      "category": "Snapshots",
      "description": "Entity tags appear when viewing a snapshot, just as on live.",
      "steps": [
        "On a network with hosts, tag one or more hosts (and/or services/subnets).",
        "Confirm the tags show on those entities in the live view (and that grouping by tag works).",
        "Take a snapshot, then select it from the snapshot dropdown.",
        "Confirm the same tags appear on the same entities in the snapshot, and tag-based grouping still works.",
        "Optionally: remove a tag from a host on Live, then re-select the snapshot — the snapshot still shows the tag as it was captured (the association is per-snapshot; the tag's name/color is shown as it is now)."
      ],
      "setup": "A network with at least one host and at least one tag applied to a host/service/subnet. Create via the UI or API before snapshotting.",
      "expected": "Tags captured at snapshot time render on the snapshot's entities. (Previously snapshots showed no tags because hydration read live entity_tags keyed on live ids.)",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-read-only",
      "category": "Snapshots",
      "description": "Viewing a snapshot is view-only: entity-data, grouping, and canvas edits are disabled; live view stays editable.",
      "steps": [
        "On Live, confirm you can: add/remove a tag on a node, create a dependency (select two nodes), edit a host/subnet description, edit a grouping rule in the options panel, and toggle canvas edit mode (node dragging).",
        "Select a snapshot from the dropdown.",
        "Confirm all of the above are now disabled/unavailable: tag add/remove disabled, dependency creation disabled, description fields read-only, grouping-rule editing disabled, and the canvas edit-mode toggle is hidden (no node dragging/resizing).",
        "Confirm navigation still works on the snapshot: switching perspective (L3/Workloads/Application/L2) and switching network/snapshot.",
        "Switch back to Live and confirm all editing is enabled again."
      ],
      "setup": "A network with hosts/services and snapshots enabled on the plan (or snapshot_retention_days_override > 0).",
      "expected": "Snapshot view behaves like a read-only embed for all backend-mutating actions; navigation/perspective switching remains.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "shares-live-only",
      "category": "Shares",
      "description": "The Share button is only available on the live view.",
      "steps": [
        "On the live view, confirm the Share button is present and a created share renders correctly.",
        "Select a snapshot from the dropdown.",
        "Confirm the Share button is hidden while a snapshot is selected.",
        "Switch back to Live and confirm the Share button returns."
      ],
      "setup": "A network with a topology and at least one snapshot.",
      "expected": "Sharing is reachable only from live, so shares never mix snapshot layout with live entity data.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    }
  ]
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
];
