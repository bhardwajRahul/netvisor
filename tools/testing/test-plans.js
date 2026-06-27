var TEST_PLANS = [
{
  "branch": "fix/snmp-interfaces-dropped-614",
  "tests": [
    {
      "id": "snmp-switch-all-ports-persist",
      "category": "SNMP Discovery",
      "description": "An SNMP switch whose physical ports are IP-less, share the chassis MAC, and have no ifName must show every ifTable interface, not just the management interface.",
      "steps": [
        "Run an SNMP (v2c) discovery scan against a multi-port L2 switch (e.g. TP-Link Omada TL-SG3216) whose access ports have no IP, share the chassis MAC, and report no ifName.",
        "Open the discovered host's detail page and view its Interfaces list."
      ],
      "setup": "Requires a real (or simulated) SNMP agent presenting an ifTable with one IP-bearing management interface (low ifIndex, has ifName + MAC) plus N physical ports at high ifIndex that all share the management interface's MAC, have no IP, and no ifName. If hardware is unavailable, point the SNMP credential at an snmpsim instance loaded with the reporter's walk from issue #614.",
      "expected": "All interfaces appear (1 management/Vlan interface + every physical port). Previously only the single 'Vlan-interface1' row appeared.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "snmp-switch-rescan-no-duplicates",
      "category": "SNMP Discovery",
      "description": "Re-scanning the same switch updates the existing interface rows in place rather than duplicating or re-collapsing them.",
      "steps": [
        "After the first scan from the previous test, trigger a second discovery scan of the same switch.",
        "Re-open the host's Interfaces list and compare counts/identities to the first scan."
      ],
      "setup": "Same SNMP target as snmp-switch-all-ports-persist; just run discovery a second time.",
      "expected": "Interface count is unchanged (no duplicates), each port retains its identity, and no interface is collapsed onto another.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "snmp-host-with-ip-interfaces-unaffected",
      "category": "SNMP Discovery",
      "description": "Hosts whose interfaces DO have IPs / distinct MACs / ifNames still dedup and display correctly (no regression from the fix).",
      "steps": [
        "Run an SNMP discovery scan against a router or server with per-interface IPs and distinct MACs (and ifNames if available).",
        "View the host's Interfaces list and confirm each interface is present once with correct IP/MAC linkage."
      ],
      "setup": "Any SNMP-capable host with multiple IP-bearing interfaces having distinct MAC addresses (e.g. a Linux server with several NICs, or a layer-3 router).",
      "expected": "Every interface is present exactly once; IP↔interface MAC links are intact. No duplication and no collapse.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/email-dns-verification",
  "tests": []
}
,
{
  "branch": "feat/billing-events-audit",
  "tests": []
}
,
{
  "branch": "feat/credentials-mgmt",
  "tests": [
    {
      "id": "daemon-flow-regression",
      "category": "Daemon setup — shared CredentialsStep regression",
      "description": "The daemon modal's credentials flow is unchanged after extracting the shared component",
      "setup": "Fresh org with no credentials.",
      "steps": [
        "Open Add Daemon, complete Configure, advance to the Integrations step",
        "Confirm the flat Integrations grid shows with the Docker Socket card selected by default",
        "Deselect the Docker Socket, continue to Install, and inspect the run command",
        "Re-open, this time keep the socket + add a Docker Proxy, continue to the wizard, configure it, and submit ('Create N credentials and continue to install')",
        "With the socket present, on the Docker Proxy confirm 'Add daemon host' is disabled with the integration-named tooltip"
      ],
      "expected": "Identical to before the refactor: socket default-selected; deselecting adds --enable-local-docker-socket false; wizard creates credentials and advances to Install; the 'Add daemon host' conflict prevention still works.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-integrations-flow",
      "category": "Discovery modal — unified credentials flow",
      "description": "A new Unified discovery shows the Integrations grid → wizard like the daemon modal",
      "setup": "Create a new Unified discovery against a daemon. Use a daemon WITHOUT local Docker socket for this run.",
      "steps": [
        "Open the Create Discovery modal, set type to Unified, pick the daemon, and advance to the Credentials step",
        "Confirm the Integrations grid appears (subtitle + cards), not the bare wizard",
        "Select an SNMP type and an integration, click Next",
        "Confirm it advances to the wizard (still on the Credentials tab) seeded with the chosen types; configure them",
        "Click Back and confirm it returns to the Integrations grid (not the previous tab); Next again returns to the wizard",
        "Finish the remaining tabs and Save; confirm the credentials are created and attached"
      ],
      "expected": "Discovery's Credentials step mirrors the daemon flow: Integrations grid → wizard, with Next/Back stepping through the sub-flow. Saving creates/updates credentials and sets pending_credential_ids.",
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-socket-readonly",
      "category": "Discovery modal — read-only socket",
      "description": "The Docker socket card is read-only in discovery, reflecting the daemon's capability, and prevents a daemon-host proxy",
      "setup": "Two daemons: one WITH local Docker socket (has_docker_socket true) and one WITHOUT.",
      "steps": [
        "New Unified discovery against the daemon WITH the socket → Credentials → Integrations grid",
        "Confirm the Docker Socket card is shown checked but disabled (read-only); hover it and read the tooltip",
        "Continue to the wizard, add a Docker Proxy, and check its 'Add daemon host' button",
        "Repeat against the daemon WITHOUT the socket: the Docker Socket card is shown unchecked + disabled"
      ],
      "expected": "Socket card is non-toggleable, checked iff the daemon has the socket. Tooltip reads 'Local Docker access is set when the daemon is installed. Reinstall the daemon to enable or disable it.' On the socket-enabled daemon, a Docker Proxy's 'Add daemon host' is disabled with the integration-named tooltip (no 'proxy will take priority' note anywhere).",
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-edit-existing-creds",
      "category": "Discovery modal — edit existing",
      "description": "Editing a discovery with existing credentials lands on the wizard",
      "setup": "An existing Unified discovery that already has pending_credential_ids attached.",
      "steps": [
        "Open it for editing and go to the Credentials step",
        "Observe whether it opens on the Integrations grid or the wizard",
        "Adjust a credential's target and save"
      ],
      "expected": "The Credentials step opens directly on the wizard showing the existing credentials (not the empty grid). Saving updates their target_ips and preserves the attachment.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/subnet-metadata-deser",
  "tests": [],
  "notes": "No human/UI tests. This is a backend serde forward-compatibility change with no UI surface. Everything is verified programmatically and runs in `cd backend && cargo test --lib`: (1) `daemon::shared::forward_compat::tests::registered_daemon_responses_are_forward_compatible` deserializes a simulated newer-server payload for every daemon-consumed response type; (2) SubnetType/EntitySource characterization tests cover both production errors plus a reproduction of the original `missing field 'metadata'` failure. A manual test is not feasible: reproducing the failure needs an OLD daemon binary, and the fix by design cannot help an already-deployed old daemon (it only takes effect once the daemon runs this build), so there is no by-hand action that demonstrates the fix."
}
,
{
  "branch": "feat/card-required-signup",
  "tests": []
}
];
