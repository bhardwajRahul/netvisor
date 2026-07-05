# Daemon Modal — Analytics Event Reference

Handoff doc for building PostHog dashboards that visualize how users move through the **Create Daemon** modal and convert to a successfully running daemon.

Captured from the code on branch `feat/credentials-mgmt`. All events are sent via `trackEvent(name, properties)` → `posthog.capture(name, properties)` (`ui/src/lib/shared/utils/analytics.ts:57`).

---

## TL;DR — the conversion goal

A user "creates a daemon" successfully when the daemon they set up actually connects/registers. The success event is:

> **`daemon_connected`** — the daemon was detected as connected. This is the bottom-of-funnel conversion event.

A user who opens the modal but never reaches `daemon_connected` (and eventually fires `daemon_wizard_closed`) is an abandon.

The modal has three steps in order: **Configure → Credentials (optional) → Install**. The daemon connects during/after the Install step.

---

## The funnel (recommended ordered funnel)

1. `daemon_prompt_viewed` *(optional top-of-funnel — the nudge that precedes the modal)*
2. `daemon_wizard_opened`
3. `daemon_wizard_step_completed` where `step = configure`
4. `daemon_wizard_step_completed` where `step = credentials` *(optional step — see notes)*
5. `daemon_install_command_copied`
6. `daemon_install_confirmed`
7. `daemon_connected` ← **conversion**

Because the Credentials step is skippable, build the **core conversion funnel without step 4** (Configure → copy → confirm → connected) and analyze Credentials engagement separately.

---

## Event reference

### Lifecycle

| Event | Fires when | Properties | Source |
|---|---|---|---|
| `daemon_prompt_viewed` | The pre-modal prompt that nudges the user to install a daemon is shown | — | `DaemonPromptModal.svelte:26` |
| `daemon_wizard_opened` | The Create Daemon modal opens | — | `CreateDaemonModal.svelte:738` |
| `daemon_wizard_closed` | The modal closes (any reason: success, cancel, or abandon) | — | `CreateDaemonModal.svelte:702` |

### Step navigation (funnel backbone)

| Event | Fires when | Properties | Source |
|---|---|---|---|
| `daemon_wizard_step_viewed` | A step becomes the active step (on first entry **and** every re-entry) | `step`: `configure` \| `credentials` \| `install` | `CreateDaemonModal.svelte:321` |
| `daemon_wizard_step_completed` | The user advances out of a step via its primary action | `step`: `configure` \| `credentials` (see below) | `CreateDaemonModal.svelte:514` (configure), `:251` (credentials skip), `:936` (credentials continue) |

`daemon_wizard_step_completed` property detail:
- **Configure:** `{ step: "configure" }` — fired by the Configure step's "Next" button after validation + API-key generation.
- **Credentials (skipped):** `{ step: "credentials", skipped: true, types_selected: <int>, credentials_attached: 0 }` — fired by the "Skip" button.
- **Credentials (completed):** `{ step: "credentials", skipped: false, types_selected: <int>, credentials_attached: <int> }` — fired by the "Create … and continue to install" button.
  - `types_selected` = number of credential types picked in the selection grid.
  - `credentials_attached` = number of credentials attached to the daemon (new + existing).
- There is **no** `step_completed` for the Install step — install progress is tracked by the action/outcome events below.

### Install step actions

| Event | Fires when | Properties | Source |
|---|---|---|---|
| `daemon_install_os_selected` | User picks/changes the target OS | `os`: `linux` \| `macos` \| `windows` | `InstallStep.svelte:189` |
| `daemon_install_command_copied` | User copies an install command | `os`: `linux`\|`macos`\|`windows`; `context`: `footer-cta` \| `combined-install` \| `docker-compose` | `CreateDaemonModal.svelte:585` (`footer-cta`), `InstallStep.svelte:193` (`combined-install`, `docker-compose`) |
| `daemon_install_confirmed` | User clicks "I've run the command / I've started Docker" — begins waiting for the daemon to connect | — | `CreateDaemonModal.svelte:595` |
| `daemon_os_support_requested` | User requests support for an OS that isn't offered yet | `os`: requested OS string | `OsSelector.svelte:60` |

### Connection outcome

| Event | Fires when | Properties | Source |
|---|---|---|---|
| `daemon_connected` | The daemon is detected as connected/registered — **conversion success** | — | `CreateDaemonModal.svelte:640` |
| `daemon_connection_timeout` | 45s elapsed after "confirmed" with no connection detected → modal shows the "trouble" state | — | `CreateDaemonModal.svelte:567`, `:617` |
| `daemon_trouble_review_commands` | From the trouble state, user goes back to review the install commands | — | `CreateDaemonModal.svelte:623` |
| `daemon_trouble_enable_self_signed` | From the trouble state, user enables self-signed certificate acceptance | — | `CreateDaemonModal.svelte:629` |

> Not part of this funnel: `daemon_upgrade_os_selected` (`DaemonUpgradeModal.svelte:79`) belongs to the separate daemon **upgrade** flow, not creation — exclude it.

---

## Property value enumerations

- `step`: `configure`, `credentials`, `install`
- `os`: `linux`, `macos`, `windows` (OS-support-request can carry other strings)
- `context` (on `daemon_install_command_copied`): `footer-cta`, `combined-install`, `docker-compose`
- `skipped`: `true` / `false`
- `types_selected`, `credentials_attached`: non-negative integers

---

## Suggested dashboards / insights

**1. Core conversion funnel** (ordered, per unique user/session)
`daemon_wizard_opened` → `daemon_wizard_step_completed [step=configure]` → `daemon_install_command_copied` → `daemon_install_confirmed` → `daemon_connected`.
Headline metric: **open → connected conversion rate**. Watch the biggest drop-off step.

**2. Step drop-off / abandonment**
Trend of `daemon_wizard_step_viewed` broken down by `step`, vs `daemon_wizard_closed` without a subsequent `daemon_connected`. Shows where users abandon.

**3. Credentials step engagement**
On `daemon_wizard_step_completed [step=credentials]`: rate of `skipped=true` vs `false`, and distributions of `types_selected` / `credentials_attached`. Answers "do users set up credentials during daemon creation, or skip?"

**4. Install friction & recovery**
- `daemon_install_confirmed` → `daemon_connected` (success) vs `daemon_install_confirmed` → `daemon_connection_timeout` (stuck).
- Among timeouts, how many recover via `daemon_trouble_review_commands` / `daemon_trouble_enable_self_signed` and then reach `daemon_connected`.

**5. OS breakdown**
Break the funnel and `daemon_install_command_copied` down by `os` and `context` (binary vs docker-compose). Surface `daemon_os_support_requested` as unmet demand.

**6. Time-to-connect**
Median/percentiles of the duration between `daemon_install_confirmed` and `daemon_connected` (and how often connection happens after the 45s `daemon_connection_timeout`).

---

## Caveats for whoever builds the dashboards

- **No modal-session id on events.** Events don't carry a shared session/correlation id, so within-session funnels must rely on PostHog person/session + event time ordering. Treat each `daemon_wizard_opened` … `daemon_wizard_closed` as one attempt by time.
- **`daemon_wizard_step_viewed` fires on re-entry**, not just first view (e.g., the Install step's "Scan Credentials" CTA returns the user to the Credentials step, re-firing `step_viewed [step=credentials]`, then `step_viewed [step=install]` again). For funnels, use "first occurrence" / unique counts, not raw totals.
- **Timeout ≠ failure.** `daemon_connection_timeout` only means 45s passed without detection; `daemon_connected` can still fire afterward. Don't treat a timeout as a terminal failure.
- **Two daemon modes converge on `daemon_connected`.** Server-poll daemons are provisioned earlier in Configure; client-poll daemons register themselves when the user runs the install command. Either way, `daemon_connected` is the universal success signal — use it as the conversion event regardless of mode.
- **Known property gaps (optional enrichment).** Events do not currently carry `daemon_mode` (server vs client poll) or `first_daemon` (the user's first daemon vs an additional one). If you want to segment conversion by those, ask eng to add them to `daemon_wizard_opened` / `daemon_connected`.
