# Accepted dependency risks

Known-vulnerable dependencies that ship in Scanopy on purpose, with the reasoning
and the conditions that would reverse the decision.

An entry earns a place here only if the advisory is real, the fix is genuinely
blocked, and the exposure has been reasoned about rather than waved away. Each
entry records what an auditor needs: where the dependency comes from, what an
attacker would actually get, why it is still here, and what has to change for the
answer to change. Entries are reviewed at each release and whenever a new
advisory lands on the same component.

Unlike the generated evidence under `ndaa-889/`, this file is written by hand.

---

## rustls-webpki 0.101.7 (3 advisories) — via `posthog-rs`

**Status:** Accepted · **Last reviewed:** 2026-08-24 · **Repo version at review:** 0.17.11

### Advisories

| Advisory | Severity | Vulnerable | Patched | Component |
|---|---|---|---|---|
| [GHSA-82j2-j2ch-gfr8](https://github.com/advisories/GHSA-82j2-j2ch-gfr8) | high | `< 0.103.13` | 0.103.13 | DoS via panic on malformed CRL BIT STRING |
| [GHSA-xgp8-3hg3-c2mh](https://github.com/advisories/GHSA-xgp8-3hg3-c2mh) | low | `>= 0.101.0, < 0.103.12` | 0.103.12 | Name constraints accepted for certificates asserting a wildcard name |
| [GHSA-965h-392x-2mh5](https://github.com/advisories/GHSA-965h-392x-2mh5) | low | `>= 0.101.0, < 0.103.12` | 0.103.12 | Name constraints for URI names incorrectly accepted |

All three are in certificate-path / CRL validation in the TLS client.

### Where it comes from

```
rustls-webpki 0.101.7
└── rustls 0.21.12
    ├── hyper-rustls 0.24.2 ─┐
    └── tokio-rustls 0.24.1 ─┴─> reqwest 0.11.27
                                 └── posthog-rs 0.4.7   (backend/Cargo.toml:219)
                                     └── scanopy-server
```

`posthog-rs` is the **only** crate in the tree on this line. Everything else —
Stripe (`async-stripe`), Docker (`bollard`), OIDC (`openidconnect`/`oauth2`) and
our own `reqwest 0.12.24` client — resolves to `rustls 0.23.35` /
**`rustls-webpki 0.103.13`**, which is the fixed line. Verify with:

```
cargo tree -i rustls-webpki@0.101.7 --target all   # only posthog-rs
cargo tree -i rustls-webpki@0.103.13 --target all  # everything else
```

### Blast radius

**Outbound telemetry TLS only.** The affected code validates the server
certificate on one outbound connection: product-analytics events POSTed to
`https://ph.scanopy.net` (`backend/src/server/shared/services/factory.rs:443`).
It is not on any path that serves a user request, terminates inbound TLS, reads a
customer network, or talks to the database, Stripe, or Docker. The whole surface
is `PosthogService::{capture, identify, group_identify}`
(`backend/src/server/posthog/service.rs`), each of which is fire-and-forget: a
failure is retried twice and then logged at `warn`, so a panic or a validation
error in this client degrades telemetry and nothing else. Telemetry is disabled
entirely when `posthog_key` is unset, which is the default for self-hosted
installs.

Reachability of each advisory in *this* configuration:

- **GHSA-82j2-j2ch-gfr8 (high)** requires rustls to parse a CRL. CRLs are only
  parsed when the client is built with `with_crls()`. `posthog-rs` builds a plain
  default client (`HttpClient::builder().timeout(..).build()`) and `reqwest 0.11`
  exposes no CRL configuration at all, so no CRL is ever handed to rustls. The
  panic path is not reachable as shipped.
- **The two low advisories** concern name constraints being under-enforced. To
  benefit, an attacker needs a name-constrained CA already in the host trust
  store to issue them a certificate for `ph.scanopy.net`, and a network position
  to intercept that connection. The payoff is the ability to receive, or drop,
  our own outbound analytics events.

### Why it is not fixed

The fixed `rustls-webpki` line requires `rustls 0.23`, which requires
`reqwest 0.12+`, which requires upgrading `posthog-rs` off `0.4`. That upgrade
is blocked, and the block was re-verified on 2026-08-24 rather than assumed:

`posthog-rs 0.25.1` (current latest) depends on
`reqwest ^0.13.2, default-features = false, features = ["rustls", ...]`. In
reqwest 0.13 that feature selects the **`aws-lc-rs`** rustls provider. Resolving
it locally pulls `aws-lc-rs v1.18.0` into the tree — and because Cargo unifies
features, it enables `aws-lc-rs` on the **shared** `rustls 0.23.35` that Stripe,
Docker and our own `reqwest 0.12` already use. Both providers then exist in one
binary and rustls cannot pick one, producing the runtime panic:

> Could not automatically determine the process-level CryptoProvider

So the damage is not scoped to telemetry: it lands on the TLS used by the paths
that matter. `aws-lc-rs` also breaks the daemon's musl cross-compilation, which
is why it is banned outright.

Two guard tests in `backend/src/tests/dependencies.rs` enforce this:
`test_no_aws_lc_rs_dependency` and `ensure_no_openssl_dependencies`. The former
fails on any upgrade that reintroduces `aws-lc-rs`.

Downgrading the risk by other means does not work either: there is no patched
`0.101.x` release, and `posthog-rs 0.4.x` pins `reqwest 0.11`, so no lockfile-only
bump can move `rustls-webpki` off `0.101.7`.

### What would change the answer

Any one of these should reopen the decision:

1. **`posthog-rs` ships a release that selects the `ring` provider** (or exposes a
   `rustls-ring`-style feature) — then it is an ordinary version bump. This is the
   cheapest exit and worth re-checking each release.
2. **A new `rustls-webpki 0.101.x` advisory lands that *is* reachable** from
   default server-certificate validation — i.e. not gated behind CRLs or name
   constraints. The reasoning above is what makes the current three tolerable; it
   does not generalise.
3. **The telemetry client stops being fire-and-forget** — if it ever carries
   customer data, blocks a user request, or its failures stop being swallowed.
4. **Procurement or a customer requires zero open alerts.** The acceptance is a
   risk judgement, not a claim the alerts are false positives; a contractual
   requirement outranks it.

### The sanctioned fix, if it is ever taken

Do **not** bump `posthog-rs`, and do not add `aws-lc-rs` by any route. The
supported path is to **remove `posthog-rs` entirely** and issue PostHog's capture
POST on the `reqwest 0.12` (ring) client already in the tree. This was scoped on
2026-08-24 and is small:

- We use six items from a 4,154-line crate: `ClientOptions::from`, `client()`,
  `Event::new`, `insert_prop`, `add_group`, `Client::capture`. Feature flags
  (2,825 lines) and local evaluation (619 lines) are never called.
- The wire format is one POST to `{host}/i/v0/e/`, `Content-Type: application/json`,
  body `{api_key, uuid (v7), event, "$distinct_id", properties, timestamp}`, with
  `$lib`, `$lib_version`, `$lib_version__{major,minor,patch}` and `$groups`
  injected into `properties`. Keep `$lib` as `posthog-rs` so existing PostHog
  insights that filter on it keep working.
- `PosthogService`'s three public methods keep their signatures, so
  `backend/src/server/posthog/subscriber.rs` is untouched.
- Removing the crate also deletes `reqwest 0.11.27`, `hyper 0.14.32`,
  `rustls 0.21.12`, `hyper-rustls 0.24.2` and `tokio-rustls 0.24.1` from the
  build — `posthog-rs` is their sole owner.

Estimated at roughly 130 lines replacing the dependency. It clears all three
advisories at the root rather than suppressing them.
