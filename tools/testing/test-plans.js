var TEST_PLANS = [
{
  "branch": "fix/637-init-targeting",
  "tests": []
}
,
{
  "branch": "fix/topology-layout-export",
  "tests": []
}
,
{
  "branch": "feat/podman-integration",
  "tests": [
    {
      "id": "podman-proxy-http-discovery",
      "category": "Podman Discovery — Proxy (HTTP)",
      "description": "Discover Podman containers over the HTTP proxy transport and confirm they appear as Podman-virtualized services with their published ports.",
      "setup": "On a host with Podman: `podman machine init && podman machine start` (macOS) or ensure the Podman socket is running (Linux). Seed a workload: `make podman-workload-up` (creates pod 'scanopy-test-pod' with an nginx on host port 8088 and a standalone nginx on host port 8089). Start the proxy: `make podman-proxy-up` (nginx fronts the Podman socket on http://127.0.0.1:2378). Verify with `curl http://127.0.0.1:2378/version`. Ensure a Scanopy daemon is registered and reachable.",
      "steps": [
        "Open Credentials and create a new credential of type 'Podman Proxy'.",
        "Set the port to 2378, leave TLS fields blank, target the daemon host (127.0.0.1), and save.",
        "Run discovery on the daemon's network.",
        "Open the discovered daemon host and inspect its services."
      ],
      "expected": "Discovery completes successfully. The nginx containers appear as services on the host, tagged with Podman virtualization (not Docker). Published ports 8088 and 8089 are present. No errors are shown.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "podman-proxy-tls-discovery",
      "category": "Podman Discovery — Proxy (mTLS)",
      "description": "Discover Podman containers over the TLS proxy transport with mutual TLS, confirming the SSL cert/key/chain fields work.",
      "setup": "Tear down the HTTP proxy if running: `make podman-proxy-down`. Start the TLS proxy: `make podman-proxy-up-tls` (generates a CA + server + client certs and prints the PEM contents). Keep the workload from the previous test (or re-run `make podman-workload-up`). Copy the printed CA cert, client cert, and client key PEM blocks.",
      "steps": [
        "Create a new 'Podman Proxy' credential.",
        "Set the port to 2378.",
        "Paste the client certificate into SSL Certificate, the client key into SSL Private Key, and the CA chain into SSL CA Chain (all three inline).",
        "Target the daemon host and save.",
        "Run discovery and inspect the daemon host's services."
      ],
      "expected": "Discovery completes over HTTPS with mutual TLS. The same Podman containers/ports are discovered as in the HTTP test. A partial-TLS config (only some of the three fields) should fall back to HTTP with a warning rather than crash.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "podman-rootless-published-ports",
      "category": "Podman Discovery — Rootless",
      "description": "Confirm rootless Podman still surfaces containers via their host-published ports even when no routable bridge subnet/container IP is available.",
      "setup": "Use ROOTLESS Podman (pasta/slirp4netns). Seed the workload with `make podman-workload-up`. Run discovery via the socket or proxy transport.",
      "steps": [
        "After discovery, inspect the daemon host's services and ports."
      ],
      "expected": "Containers are still discovered as Podman services with their published host ports (8088, 8089). It is acceptable that no Podman bridge subnet and no per-container routable IP appear under rootless networking — discovery must not error.",
      "status": null,
      "feedback": null
    },
    {
      "id": "api-daemon-host-single-endpoint-rejected",
      "category": "API — Single-endpoint enforcement",
      "description": "Server-side backstop: persisting a discovery whose integration_targets put two same-integration single-endpoint credentials on the daemon host is rejected.",
      "setup": "Have a daemon + its primary discovery, plus a Docker socket and a Docker proxy credential (or the Podman pair).",
      "steps": [
        "Via the API, PUT the daemon's discovery with integration_targets containing BOTH the socket and the proxy of the same integration scoped to the daemon host (DaemonHost)."
      ],
      "expected": "The update is rejected with a 400 naming both credentials (same integration allows only one per host). Submitting just one, or one of each of two DIFFERENT integrations (Docker + Podman), succeeds.",
      "status": null,
      "feedback": null
    },
    {
      "id": "daemon-host-cred-merge-durability",
      "category": "Discovery modal credential parity",
      "description": "A daemon-host socket credential assigned via the DISCOVERY modal reaches the host_credentials junction (DaemonHost scope), appears in the banner, and survives a no-op Update.",
      "setup": "A daemon host running Podman. Have a Docker socket credential and a Podman socket credential.",
      "steps": [
        "Open the discovery (edit) modal and add the Podman socket credential (add-existing).",
        "Save the discovery; run discovery and check the banner.",
        "Open the discovery modal again and click Update Discovery WITHOUT changes.",
        "Inspect the daemon host credential assignments / banner."
      ],
      "expected": "The socket added via the discovery modal is stored in host_credentials (NOT as a Network integration_target), appears in the discovery banner, and a no-op Update does not wipe it. Parity with the daemon-create modal and the host/credential modal.",
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-modal-targeting-scopes",
      "category": "Discovery modal credential parity",
      "description": "The discovery modal assigns credentials by all three scopes like the daemon-create modal: Network (no IPs), Hosts (explicit IPs, even undiscovered), DaemonHost (socket / loopback).",
      "setup": "Have a Network-capable credential (e.g. SNMP), a Hosts-capable credential, and a daemon-host socket credential.",
      "steps": [
        "In the discovery modal, add a credential with no target IPs (Network), one with explicit non-loopback IPs (Hosts), and a socket (DaemonHost).",
        "Save and inspect where each lands."
      ],
      "expected": "Network/Hosts credentials persist as integration_targets on the discovery; the socket is merged into the daemon host junction. DB: discovery.integration_targets holds only Network/Hosts; host_credentials holds the socket.",
      "status": null,
      "feedback": null
    },
    {
      "id": "podman-pod-four-distinct-services",
      "category": "Podman Discovery - per-container attribution",
      "description": "A pod with infra + nginx + grafana members plus a standalone nginx yields FOUR distinct container services, each attributed by its own image-exposed ports.",
      "setup": "Run make podman-workload-up (pod publishes nginx:80 and grafana:3000 as members; plus standalone nginx). Assign a Podman socket credential to the daemon host.",
      "steps": [
        "Run discovery against the daemon host.",
        "Inspect the discovered services."
      ],
      "expected": "Four distinct container services: scanopy-test-grafana matched to Grafana (its exposed 3000), scanopy-test-web and the pod infra and scanopy-test-standalone as generic Podman Container (distinct by container_id). The grafana member is NOT duplicated and the nginx members are NOT mislabeled as Grafana.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/credentials-mgmt",
  "tests": []
}
,
{
  "branch": "feat/radar-loading-spinner",
  "tests": []
}
,
{
  "branch": "feat/integrations-fixture",
  "tests": []
}
,
{
  "branch": "fix/csp-stripe-elements",
  "tests": [
    {
      "id": "stripe-elements-renders",
      "category": "CSP / Stripe Elements",
      "description": "Stripe Payment Element loads and renders inside the in-app card modal (verifies script-src + frame-src).",
      "steps": [
        "Open the app in a browser with the dev console open (Console tab).",
        "Trigger the in-app card modal (PaymentMethodModal) via a billing nudge / 'Add payment method'.",
        "Wait for the Payment Element to render."
      ],
      "setup": "Sign in as a user/org whose billing state surfaces an 'Add payment method' entry point (e.g. an org without a saved card). If needed, create such an org via the API and log in as its owner.",
      "expected": "The Stripe card-input iframe appears and is interactive. No CSP violation errors in the console mentioning js.stripe.com or hooks.stripe.com.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "stripe-setupintent-confirm",
      "category": "CSP / Stripe Elements",
      "description": "Submitting a test card succeeds via SetupIntent confirm and the card is saved (verifies connect-src to api.stripe.com).",
      "steps": [
        "In the open card modal, enter Stripe test card 4242 4242 4242 4242, a future expiry, any CVC and ZIP.",
        "Submit the form.",
        "Wait for the success state / modal close, then re-open billing to confirm the saved card is listed."
      ],
      "expected": "SetupIntent confirm succeeds (network call to api.stripe.com is not blocked), the card is saved and shown in billing. No CSP violations in the console.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "stripe-no-residual-csp-violations",
      "category": "CSP / Stripe Elements",
      "description": "No further Stripe domains are blocked by the CSP during the full card flow.",
      "steps": [
        "With the console open, repeat the render + submit flow above.",
        "Scan the console for any 'violates the following Content Security Policy directive' messages referencing a stripe domain (e.g. m.stripe.network)."
      ],
      "expected": "No residual CSP violations. If m.stripe.network (Stripe fraud signals, frame-src) is reported blocked, note it — it would need to be added to frame-src as a follow-up.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    }
  ]
}
];
