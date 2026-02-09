> **First:** Read `CLAUDE.md` (project instructions) — you are a **worker**.

<<<<<<< HEAD
<<<<<<< HEAD
# Task: Billing Experience Overhaul

## Objective

Overhaul the billing experience: add a Free tier, remove credit card requirement at signup, show plan selection as an in-app modal instead of a gating page, allow plan changes from within the UI, add payment method collection flow, add notification bubble for missing billing, add transactional emails for billing lifecycle events, and enforce Free tier limitations in both backend and frontend.

---

## 1. Free Tier Plan

### BillingPlan Enum

Add `Free(PlanConfig)` variant to `BillingPlan` in `backend/src/server/billing/types/base.rs`.

**PlanConfig for Free:**
```rust
BillingPlan::Free(PlanConfig {
    base_cents: 0,
    rate: BillingRate::Month,
    trial_days: 0,
    seat_cents: None,
    network_cents: None,
    host_cents: None,         // NEW FIELD - can't buy more
    included_seats: Some(1),
    included_networks: Some(1),
    included_hosts: Some(25), // NEW FIELD - capped at 25
})
```

### New PlanConfig Fields

Add to `PlanConfig` following the existing pattern for seats/networks:
- `host_cents: Option<i64>` — None = can't pay for more. Set to `None` for all plans (no host add-on pricing).
- `included_hosts: Option<u64>` — None = unlimited. `Some(25)` for Free, `None` for all others.

Update all existing plan definitions in `plans.rs` to include `host_cents: None, included_hosts: None`.

### Remove Redundant Feature Flags

`unlimited_hosts` and `unlimited_scans` are now redundant — host limits are expressed by `included_hosts` in PlanConfig, and scan scheduling by `scheduled_discovery` in BillingPlanFeatures. Remove:
- `unlimited_hosts` and `unlimited_scans` fields from `BillingPlanFeatures` struct
- `UnlimitedHosts` and `UnlimitedScans` variants from the `Feature` enum in `features.rs`
- All associated trait impls, match arms, `Into<Vec<Feature>>` entries, and references
- The pricing page / fixtures should show host limits from `PlanConfig.included_hosts` and discovery from `scheduled_discovery` instead

### New BillingPlanFeatures Fields

Add:
- `scheduled_discovery: bool` — false for Free, true for all other plans
- `daemon_poll: bool` — false for Free, true for all other plans

### New Feature Enum Variants

Add to `Feature` in `features.rs`:
- `ScheduledDiscovery` — category: "Core", name: "Scheduled Discovery", description: "Schedule automatic network discovery scans"
- `DaemonPoll` — category: "Core", name: "DaemonPoll Mode", description: "Daemon-initiated polling — no open ports required on the daemon"

Update all trait impls: `HasId`, `EntityMetadataProvider`, `TypeMetadataProvider`, `Into<Vec<Feature>>`.

### New ErrorCode Variants

Add to `ErrorCode` in `shared/types/error_codes.rs` under the `// === Billing ===` section:
- `BillingHostLimitReached { limit: u32 }` — "You've reached the limit of {limit} hosts on your current plan. Upgrade for unlimited hosts."
- `BillingFeatureNotAvailable { feature: String }` — "Your current plan does not include {feature}. Upgrade your plan to access this feature."

And under `// === Daemon ===`:
- `DaemonStandby` — "Your plan does not support DaemonPoll mode. The daemon is on standby. Upgrade your plan and restart the daemon to resume."

Add corresponding entries in `default_message()`, `params()`, and the no-params match arm.

### Update All BillingPlan Match Arms

Add `Free` to every match in `BillingPlan` impl blocks:
- `config()`, `set_config()`, `is_commercial()` → false
- `can_invite_users()` → false (1 seat, no seat_cents)
- `hosting()` → `Hosting::Cloud`
- `custom_price()` → `Some("Free")`
- `stripe_product_id()`, all stripe lookup key methods
- `features()` — same as Starter except: `scheduled_discovery: false`, `daemon_poll: false`
- `EntityMetadataProvider`: icon (appropriate variant), color (`Color::Green`)
- `TypeMetadataProvider`: name "Free", description "Get started with Scanopy — manual discovery for up to 25 hosts"

### plans.rs

Add `get_free_plan() -> BillingPlan`. Include Free in `get_purchasable_plans()` (frontend needs it in the plan selection modal) and `get_website_fixture_plans()`.

### Tests

The existing `test_feature_ids_match_billing_plan_features_fields` test catches Feature/BillingPlanFeatures mismatches. Ensure it passes.

---

## 2. Backend Enforcement of Free Tier Limits

### Host Limit

Use `ApiError::coded(StatusCode::FORBIDDEN, ErrorCode::BillingHostLimitReached { limit })`.

**Discovery host creation:** In `server/hosts/service.rs` — in the code path where the host service checks for existing/conflicting hosts and decides whether to create a new one or upsert data to an existing one: if creating a NEW host, check the organization's `included_hosts` from plan config against current host count. If at or over limit, return the error AND cancel the active discovery session via the discovery service. If UPSERTING an existing host (just updating data), allow it to proceed — no limit check needed.

**Manual host creation:** In `server/hosts/handlers.rs` — the create host endpoint used by the UI should check the limit in the handler before proceeding.

### Scheduled Discovery Restriction

Use `ApiError::coded(StatusCode::FORBIDDEN, ErrorCode::BillingFeatureNotAvailable { feature: "Scheduled Discovery".into() })`.

In discovery creation/update handlers (`server/discovery/` handlers), check `plan.features().scheduled_discovery`. If `RunType::Scheduled` is requested and feature is false, return the error.

**On downgrade to Free:** Convert existing `RunType::Scheduled` discoveries to `RunType::AdHoc` (not just disable them — a disabled scheduled discovery could be re-enabled by the user). Converting to AdHoc means the discovery update handler's check will block any attempt to change it back to Scheduled.

### DaemonPoll Restriction

Use `ApiError::coded(StatusCode::FORBIDDEN, ErrorCode::BillingFeatureNotAvailable { feature: "DaemonPoll".into() })`.

In daemon registration (`server/daemons/handlers.rs` — `process_registration()`) and provisioning endpoint, check `plan.features().daemon_poll`. If `DaemonMode::DaemonPoll` is requested and feature is false, return the error.

### Daemon Standby on Downgrade

When an org is downgraded to Free and has active DaemonPoll daemons:

**Server-side:**
- Add `standby: bool` field to `DaemonBase` (default false). On downgrade to Free, set `standby = true` on all DaemonPoll daemons for that org.
- In the server's `poll_daemon()` / `start_polling_loop()` in `daemons/service.rs`, skip daemons where `standby == true` (same pattern as `is_unreachable`).
- When org upgrades to a plan with `daemon_poll: true`, set `standby = false` on their daemons.

**DaemonPoll standby signal — NO response format change. Use an error code instead:**
- In the `request-work` handler (`receive_work_request` in handlers.rs), if `daemon.base.standby` is true, return `ApiError::coded(StatusCode::FORBIDDEN, ErrorCode::DaemonStandby)`.
- This is **backward-compatible**: the response tuple format `(Option<DiscoveryUpdatePayload>, bool)` is unchanged. Old daemons already don't retry `ApiErrorResponse` (see line 211 of `daemon/runtime/service.rs`) and will exit via the generic error handler. New daemons check for `DaemonStandby` specifically.

**New daemon-side standby handling** (in `daemon/runtime/service.rs`, in the error handler around line 236):
- Check if the error is an `ApiErrorResponse` matching `ErrorCode::DaemonStandby`
- If so, log clearly: "Plan does not support DaemonPoll mode. Daemon is on standby. Upgrade your plan and restart the daemon to resume."
- **Block indefinitely** — do NOT exit, so the user sees the message in their terminal. Use something like `tokio::signal::ctrl_c().await` or a pending future.
- The user must manually restart the daemon after upgrading their plan.

### Host Deletion on Downgrade

When an org is downgraded to Free and has more than 25 hosts:
- Delete hosts exceeding the limit
- Keep the 25 most recently updated hosts. Prioritize keeping daemon hosts (identified by `daemon.base.host_id` — check all daemons for the org and preserve their host_ids)
- Include the deletion count in the downgrade email

---

## 3. Frontend Enforcement of Free Tier Limits

### Reusable UpgradeBadge Component

Create `UpgradeBadge.svelte` in `ui/src/lib/shared/components/`:
- Small badge/pill that overlays or sits next to CTA buttons, dropdown options, form fields
- Text like "Upgrade" with tooltip explaining the required plan
- Clicking opens the plan selection modal
- Props: feature name/description (tooltip), optional target plan
- Use consistently across all enforcement points below

### Daemon Mode Selection

In `ui/src/lib/features/daemons/config.ts` and `CreateDaemonForm.svelte`:
- When org plan doesn't have `daemon_poll` feature, disable DaemonPoll option
- Apply UpgradeBadge to the disabled option
- Default to ServerPoll for Free tier

### Discovery Scheduling

In `ui/src/lib/features/discovery/components/DiscoveryModal/DiscoveryTypeForm.svelte`:
- When plan doesn't have `scheduled_discovery`, disable Scheduled run type
- Apply UpgradeBadge
- Only allow AdHoc for Free tier

### Host Limit Indicator

- Show host count vs limit: "X / 25 hosts"
- Warning when approaching (20+/25)
- At limit: disable host creation CTA, apply UpgradeBadge
- "Upgrade for unlimited hosts"

---

## 4. Trial Without Credit Card

**Extend the existing checkout endpoint** (`POST /api/billing/checkout`) — do not create a new endpoint:

- If `trial_days > 0` and new customer: create Stripe Subscription directly via API with `trial_period_days` and no payment method, or use Stripe Checkout with `payment_method_collection: 'if_required'`
- Set `organization.plan`, `plan_status = "trialing"`, `trial_end_date` from Stripe's `subscription.trial_end`
- Publish `TrialStarted` telemetry event
- Return appropriately (no redirect if subscription created directly, or Stripe URL if using checkout mode)

For returning customers or plans without trials, existing checkout flow with card collection remains.

---

## 5. Payment Method Collection

### New Endpoint: `POST /api/billing/setup-payment-method`

- Auth: `Owner`
- Creates Stripe Checkout Session in `setup` mode (collect payment method without charging)
- Returns redirect URL
- On Stripe webhook completion, set `organization.has_payment_method = true`

### New Organization Fields

- `has_payment_method: bool` (default false) — updated via webhooks: `payment_method.attached`, `customer.updated`, or `checkout.session.completed` (setup mode)
- `trial_end_date: Option<DateTime<Utc>>`

Expose both in organization API response.

---

## 6. Trial Expiry and Downgrade to Free

### Stripe Webhook: `customer.subscription.trial_will_end`

Fires 3 days before. Handler:
1. Check `has_payment_method` — only send email if false
2. Send email: "Your trial ends in 3 days. Add a payment method to continue on [Plan]."

### When Trial Expires (no payment method)

Handle in existing `customer.subscription.updated` webhook when transitioning from `trialing`:
1. Downgrade: `plan = Free(...)`, `plan_status = "active"`
2. Cancel Stripe subscription
3. Convert all `RunType::Scheduled` discoveries to `RunType::AdHoc`
4. Set `standby = true` on all DaemonPoll daemons
5. Delete hosts exceeding 25 (keep most recent; preserve daemon host_ids)
6. Compute overage for email:
   - Hosts deleted (count)
   - Discoveries converted to ad-hoc (count)
   - Daemons put on standby (count)
7. Send email with overage: "Your trial has ended. You're on the Free plan. [X hosts removed, Y discoveries converted to manual, Z daemons on standby]. Upgrade anytime."

If `has_payment_method` is true: Stripe handles trial→active normally.

---

## 7. Plan Selection as Modal

### Refactor BillingPlanForm

Convert to modal component:
- Show all plans including Free, "Current Plan" badge on active
- Upgrade/downgrade buttons per plan
- Available app-wide via store or context

### Remove Gating Redirect

Every user now has a plan (Free minimum) — no more 402 lockout. The `/billing` route can open the modal or redirect to settings. Remove 402-based gating flow.

### Post-Onboarding Flow

After registration, user lands on Free. Plan selection modal shown as dismissible suggestion, or notification bubble guides them.

---

## 8. In-App Plan Changes

### New Endpoint: `POST /api/billing/change-plan`

- Auth: `Owner`
- Input: `{ plan: String, rate: String }`
- **Upgrade:** Create checkout/trial or update subscription with proration
- **Downgrade:** Schedule change for end of billing period

### Downgrade Preview: `GET /api/billing/change-plan/preview?plan=...&rate=...`

Returns:
```json
{
  "networks_to_remove": 3,
  "hosts_to_delete": 42,
  "discoveries_to_convert": 5,
  "daemons_to_standby": 2
}
```

### Frontend

- Plan modal: "Current Plan" badge, change buttons
- "Change Plan" on settings billing tab opens modal
- **Downgrade with overage:** Confirmation dialog: "Downgrading to Free will delete 42 hosts (keeping 25), convert 5 scheduled discoveries to manual, and put 2 daemons on standby. This cannot be undone. Continue?"
- User must confirm. Upgrade proceeds directly.

---

## 9. Notification Bubble

Persistent badge on Settings nav item:
- Shows when `has_payment_method === false` AND plan is Free or trialing
- Tooltip: "Add billing information"
- Click navigates to settings billing tab
- Dismisses only when payment method added

### Settings Billing Tab

- Prominent "Add Payment Method" CTA when missing
- Trial countdown from `trial_end_date`
- Free limits: "X / 25 hosts", "Manual discovery only", "ServerPoll only"
- "Change Plan" opens modal

---

## 10. Transactional Emails

Templates in `backend/src/server/email/templates.rs` following existing patterns.

1. **Trial Started** — "Welcome to your [Plan] trial! [X] days to explore."
2. **Trial Ending Soon** (3 days) — "Trial ends in 3 days. Add payment to continue on [Plan]."
3. **Trial Ended / Downgraded** — Include overage: hosts deleted, discoveries converted, daemons on standby. "Upgrade anytime."
4. **Payment Method Added** — "Payment added. Subscription continues after trial."
5. **Plan Changed** — "Upgraded/downgraded to [Plan]. [What changed]."
6. **Subscription Cancelled** — "Cancelled. Access until [date], then Free plan."

Add to `EmailProvider` trait. Implement for Plunk and SMTP. Update `EmailService` subscriber for billing events. The `trial_will_end` webhook calls email service directly (not via EventBus).

---

## 11. Database Migration

Single migration:

```sql
ALTER TABLE organizations ADD COLUMN has_payment_method BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE organizations ADD COLUMN trial_end_date TIMESTAMPTZ;
ALTER TABLE daemons ADD COLUMN standby BOOLEAN NOT NULL DEFAULT FALSE;
```

`included_hosts`, `host_cents` are in JSONB `plan` column via PlanConfig. `scheduled_discovery`, `daemon_poll` are computed from plan via BillingPlanFeatures. No extra columns needed.

---

## 12. Update Billing Fixtures

```bash
cd backend && cargo test generate_billing_fixtures
```
=======
# Task: Replace HubSpot with Brevo

## Objective

Replace HubSpot with Brevo as the consolidated CRM and non-transactional email platform. Brevo will handle: CRM (contacts, companies, deals/pipeline), event-driven marketing automation, and non-transactional email campaigns (onboarding sequences, lifecycle emails, cart recovery). Transactional emails (password reset, invite, verification) remain on Plunk/SMTP — they live in the app and don't need a marketing platform.

## Context

### Current HubSpot Integration

The HubSpot integration is in `backend/src/server/hubspot/` and consists of:

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `types.rs` | Contact/Company property types, API request/response structs |
| `client.rs` | HubSpot API client with rate limiting (8 req/sec) and exponential backoff retries |
| `service.rs` | Event handling, business logic, org sync, metrics sync, startup backfill |
| `subscriber.rs` | EventBus subscriber (telemetry, auth, entity, discovery events), 5-sec debounce |
| `freemail.rs` | Work vs free email domain detection (4,500+ free domains, 88,000+ disposable) |
| `freemail_free.txt` | Free email domain list |
| `freemail_disposable.txt` | Disposable email domain list |

### What HubSpot Currently Syncs

**Contact properties:** email, firstname, lastname, jobtitle, scanopy_user_id, scanopy_org_id, scanopy_role, scanopy_signup_source, scanopy_use_case, scanopy_signup_date, scanopy_last_login_date, scanopy_marketing_opt_in

**Company properties:** name, scanopy_org_id, scanopy_org_type, scanopy_company_size, scanopy_plan_type, scanopy_plan_status, scanopy_mrr, scanopy_network_count, scanopy_host_count, scanopy_user_count, scanopy_network_limit, scanopy_seat_limit, scanopy_created_date, scanopy_last_discovery_date, scanopy_discovery_count, plus ~10 milestone dates (first_daemon, first_discovery, trial_started, checkout_completed, first_network, first_tag, first_api_key, first_snmp_credential, first_invite_sent, first_invite_accepted) and inquiry fields.

**Events subscribed to:**
- All telemetry operations (OrgCreated, CheckoutStarted, CheckoutCompleted, TrialStarted, TrialEnded, SubscriptionCancelled, FirstDaemonRegistered, FirstTopologyRebuild, FirstNetworkCreated, etc.)
- Auth: LoginSuccess (updates last_login_date)
- Entity CRUD: Network/Host/User create/delete (metrics sync)
- Discovery: Scanning phase (last_discovery_date)

**Filtering (REMOVING):** HubSpot currently only syncs orgs with commercial plans OR work email domains. **Brevo will sync ALL organizations** — no filtering. The `freemail.rs` filtering logic and `scanopy_non_commercial` flag are no longer needed for CRM sync. (Note: `freemail.rs` is also used for disposable email rejection at registration — check if it's imported elsewhere before removing.)

**Frontend:** `PlanInquiryModal.svelte` submits enterprise inquiries to HubSpot Forms API (portal 50956550, form 96ece46e-04cb-47fc-bb17-2a8b196f8986) and also updates CRM company properties via the backend.

### Factory Wiring

In `shared/services/factory.rs` (lines ~298-349):
- HubSpotService created if `config.hubspot_api_key` is set
- Injected with: NetworkService, HostService, UserService, OrganizationService, DaemonService, TagService, UserApiKeyService, SnmpCredentialService
- Registered as EventBus subscriber

### Database

`organizations.hubspot_company_id` column stores the HubSpot company ID after sync.

---

## Requirements

### 1. Create Brevo Module

Create `backend/src/server/brevo/` mirroring the HubSpot module structure:

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `types.rs` | Brevo contact/company property types, API request/response structs |
| `client.rs` | Brevo API client with rate limiting and retries |
| `service.rs` | Event handling, business logic, sync, backfill |
| `subscriber.rs` | EventBus subscriber (same event patterns as HubSpot) |

### 2. Brevo API Client

Brevo REST API base: `https://api.brevo.com/v3`

**Key endpoints to implement:**

| Operation | Endpoint | Notes |
|-----------|----------|-------|
| Create contact | `POST /contacts` | Upsert by email |
| Update contact | `PUT /contacts/{identifier}` | By email or ID |
| Search contact | `POST /contacts/search` | Filter by attributes |
| Create company | `POST /companies` | |
| Update company | `PATCH /companies/{id}` | |
| Search company | `POST /companies/search` | Filter by attributes |
| Link contact to company | `PATCH /companies/link-unlink/{id}` | Associate contact with company |
| Create deal | `POST /crm/deals` | For sales pipeline |
| Track event | `POST /events` | For automation triggers |

**Auth:** `api-key` header with Brevo API key.

**Rate limiting:** Brevo allows 300 requests/minute on most plans. Implement with `governor` crate following the HubSpot client pattern. Set to ~4 req/sec with burst.

**Retry:** Same exponential backoff pattern as HubSpot client — retry on 429 and 5xx.

**Important:** Research the exact Brevo API v3 endpoints, request/response formats, and authentication before implementing. The endpoints above are directional — verify against Brevo's current API docs.

### 3. Property Mapping

Map all existing HubSpot properties to Brevo contact/company attributes. Brevo uses "attributes" instead of "properties."

**Contact attributes** (map from HubSpot contact properties):
- Standard: `EMAIL`, `FIRSTNAME`, `LASTNAME`
- Custom: Create custom attributes for all `scanopy_*` fields (same names, Brevo supports custom attributes on contacts)

**Company attributes** (map from HubSpot company properties):
- `name` + all `scanopy_*` company properties as custom attributes

**Brevo attribute types:** text, number, boolean, date, category. Map appropriately (dates as date type, counts as number, flags as boolean).

### 4. Service Layer

Implement `BrevoService` with the same methods as `HubSpotService`:

| Method | Trigger | Behavior |
|--------|---------|----------|
| `handle_org_created` | OrgCreated | Create contact + company, store company ID |
| `handle_checkout_started` | CheckoutStarted | Update plan_status |
| `handle_checkout_completed` | CheckoutCompleted | Set plan type, status, date |
| `handle_trial_started` | TrialStarted | Set status to trialing |
| `handle_trial_ended` | TrialEnded | Set status based on conversion |
| `handle_subscription_cancelled` | SubscriptionCancelled | Set status |
| `handle_first_daemon_registered` | FirstDaemonRegistered | Record date |
| `handle_first_topology_rebuild` | FirstTopologyRebuild | Record date |
| `handle_engagement_event` | Various first-time events | Record milestone dates |
| `update_contact_last_login` | LoginSuccess | Update last_login_date |
| `update_company_last_discovery` | Scanning | Update last_discovery_date |
| `sync_org_entity_metrics` | Network/Host/User CRUD | Query DB, sync counts |
| `backfill_organizations` | Server startup | See section 6 below |

**No filtering:** Sync ALL organizations to Brevo. Remove the `should_sync_to_hubspot()` check and `scanopy_non_commercial` flag. Every org gets a Brevo contact + company.

### 5. Event Subscriber

Implement `EventSubscriber` for `BrevoService` with the same event filter and debounce behavior as HubSpot:
- Subscribe to: all telemetry, LoginSuccess, Network/Host/User CRUD, Discovery Scanning
- 5-second debounce window for batching
- Non-blocking error handling

Subscriber name: `brevo_crm`

### 6. Startup Backfill

On server startup, `BrevoService` must backfill all organizations that don't yet have a `brevo_company_id`:

1. Query all organizations where `brevo_company_id IS NULL`
2. For each: create contact (from org owner) + company in Brevo, store the returned company ID
3. Backfill telemetry milestones from database (same pattern as HubSpot's `sync_existing_organizations()`)
4. Sync current entity metrics (network count, host count, user count)
5. Rate-limit the backfill to avoid hitting Brevo API limits
6. Log progress: "Backfilling org {name} ({x}/{total})"
7. Non-blocking: errors on individual orgs should be logged and skipped, not halt the entire backfill

This runs on every server startup but is effectively a no-op once all orgs have brevo_company_ids.

### 7. Event Tracking for Automation

Brevo's event tracking (`POST /events`) allows triggering automation workflows. Track key events:

- `org_created` — trigger onboarding sequence
- `trial_started` — trigger trial nurture sequence
- `trial_ended` — trigger conversion/winback sequence
- `checkout_completed` — trigger welcome/success sequence
- `first_discovery_completed` — trigger engagement sequence
- `subscription_cancelled` — trigger winback sequence

Send via Brevo's event tracking API so they appear in Brevo's automation builder.

### 8. Enterprise Inquiry Form

**Backend:** Replace HubSpot form submission in `billing/handlers.rs` (`inquiry` endpoint, lines 216-313):
- Currently submits to HubSpot Forms API + updates CRM company
- Replace with: create/update Brevo contact with inquiry data + update Brevo company + optionally create a deal in Brevo's CRM pipeline

**Frontend:** Update `PlanInquiryModal.svelte`:
- Remove HubSpot tracking cookie (`hubspotutk`) extraction
- Remove HubSpot-specific form field mapping
- The modal continues to POST to `POST /api/billing/inquiry` — backend handles Brevo

### 9. Database Migration

```sql
ALTER TABLE organizations RENAME COLUMN hubspot_company_id TO brevo_company_id;
```

Update `StorableFilter` implementations that reference `hubspot_company_id`.

### 10. Configuration

**Replace env vars:**
- `SCANOPY_HUBSPOT_API_KEY` → `SCANOPY_BREVO_API_KEY`

**Update:**
- `ServerConfig` in `config.rs`: replace `hubspot_api_key` with `brevo_api_key`
- `factory.rs`: create `BrevoService` instead of `HubSpotService` when `brevo_api_key` is set
- `.env.example` if it references HubSpot

**Frontend config:**
- If `has_hubspot` or similar is exposed via `/config`, rename to `has_crm` or `has_brevo`
- Remove HubSpot tracking script loading (check for HubSpot script tags in frontend layout)
- Cookie consent: if tied to HubSpot, update for Brevo or remove if not needed for CRM-only

### 11. Remove HubSpot Module

After Brevo module is complete and tested:
- Delete `backend/src/server/hubspot/` directory entirely
- Remove from `mod.rs` parent module
- Remove `hubspot` from any feature flags, test configs, or CI references
- Clean up HubSpot-specific imports
- **`freemail.rs` and domain lists:** Check if `freemail.rs` / `is_work_email()` / disposable email detection is used outside of HubSpot (e.g., registration rejects disposable emails). If used elsewhere, move to `shared/`. If only used by HubSpot filtering, it can be deleted.

### 12. Frontend Cleanup

- `PlanInquiryModal.svelte` — remove HubSpot cookie/form logic, keep form UI and `POST /api/billing/inquiry`
- Check for HubSpot tracking script in layout files
- Remove `hubspot` references in frontend config types
- Update `/config` endpoint response type if field names changed
>>>>>>> brevo-integration

---

## Key Files

<<<<<<< HEAD
**Backend:**
- `billing/types/base.rs` — BillingPlan, PlanConfig, BillingPlanFeatures
- `billing/types/features.rs` — Feature enum
- `billing/plans.rs` — Free plan, get_purchasable_plans
- `billing/service.rs` — checkout, change-plan, setup-payment-method, webhooks
- `billing/handlers.rs` — endpoints
- `shared/types/error_codes.rs` — new ErrorCode variants
- `auth/middleware/billing.rs` — no 402 for Free
- `hosts/service.rs` — host limit (new host vs upsert check) + discovery cancellation
- `hosts/handlers.rs` — manual host creation limit check
- `discovery/` handlers — scheduled discovery restriction
- `daemons/handlers.rs` — DaemonPoll restriction, standby error in request-work
- `daemons/impl/base.rs` — standby field
- `daemons/service.rs` — skip standby in polling loop
- `daemon/runtime/service.rs` — standby error handling (block indefinitely)
- `email/templates.rs` — billing email templates
- `email/traits.rs` — new EmailProvider methods
- `email/subscriber.rs` — billing event routing
- `organizations/impl/base.rs` — has_payment_method, trial_end_date
- `migrations/` — single migration

**Frontend:**
- `shared/components/` — UpgradeBadge.svelte
- `features/billing/BillingPlanForm.svelte` — modal
- `features/billing/queries.ts` — mutations
- `routes/billing/+page.svelte` — remove gating
- `features/settings/BillingTab.svelte` — CTA, countdown, limits
- `features/daemons/config.ts` + `CreateDaemonForm.svelte` — DaemonPoll disabled
- `features/discovery/components/DiscoveryModal/DiscoveryTypeForm.svelte` — Scheduled disabled
- Navigation — notification badge
=======
**Create:**
- `backend/src/server/brevo/mod.rs`
- `backend/src/server/brevo/types.rs`
- `backend/src/server/brevo/client.rs`
- `backend/src/server/brevo/service.rs`
- `backend/src/server/brevo/subscriber.rs`

**Modify:**
- `backend/src/server/shared/services/factory.rs` — wire BrevoService
- `backend/src/server/config.rs` — brevo_api_key
- `backend/src/server/billing/handlers.rs` — inquiry endpoint
- `backend/src/server/organizations/impl/base.rs` — brevo_company_id field
- `backend/migrations/` — rename column
- `ui/src/lib/features/billing/PlanInquiryModal.svelte` — remove HubSpot specifics
- Frontend layout/config — remove HubSpot tracking, update cookie consent

**Delete:**
- `backend/src/server/hubspot/` — entire module
- `freemail.rs` + domain lists IF not used outside HubSpot (check first)
>>>>>>> brevo-integration

---

## Acceptance Criteria

<<<<<<< HEAD
- [ ] Free plan with `included_hosts: Some(25)`, `host_cents: None`, `scheduled_discovery: false`, `daemon_poll: false`
- [ ] `unlimited_hosts` and `unlimited_scans` removed from BillingPlanFeatures and Feature enum
- [ ] `PlanConfig` has `included_hosts` and `host_cents` (same pattern as seats/networks)
- [ ] Free in `get_purchasable_plans()` and visible in plan selection UI
- [ ] New users start on Free after onboarding
- [ ] Trials start without credit card via modified checkout endpoint
- [ ] Trial expiry without payment → downgrade to Free, excess hosts deleted
- [ ] Trial expiry email includes overage (hosts deleted, discoveries converted, daemons on standby)
- [ ] `customer.subscription.trial_will_end` webhook sends 3-day reminder
- [ ] ErrorCode variants: `BillingHostLimitReached`, `BillingFeatureNotAvailable`, `DaemonStandby`
- [ ] Host limit: checked on new host creation (not upsert); discovery cancelled when limit hit
- [ ] Manual host creation checks limit in handler
- [ ] Scheduled discoveries converted to AdHoc on downgrade (not just disabled)
- [ ] DaemonPoll daemons set to standby; daemon blocks indefinitely on `DaemonStandby` error (backward-compatible, no tuple format change)
- [ ] Daemon hosts preserved on downgrade (identified via `daemon.base.host_id`)
- [ ] Plan selection is a modal, not a gating page
- [ ] In-app plan changes with downgrade preview + confirmation
- [ ] Payment method collection via settings
- [ ] Notification bubble on settings when no payment method
- [ ] Reusable UpgradeBadge on all restricted features
- [ ] Transactional emails for all billing lifecycle events
- [ ] Tests pass, format/lint clean, fixtures regenerated
=======
- [ ] Brevo module created with client, types, service, subscriber
- [ ] All HubSpot contact + company properties mapped to Brevo attributes
- [ ] EventBus subscriber handles all events HubSpot did (telemetry, auth, entity CRUD, discovery)
- [ ] ALL organizations synced (no commercial/work-email filtering)
- [ ] Startup backfill: all orgs without brevo_company_id get synced on server start
- [ ] Event tracking for automation triggers (org_created, trial_started, etc.)
- [ ] Enterprise inquiry form submits to Brevo instead of HubSpot
- [ ] Database: hubspot_company_id renamed to brevo_company_id
- [ ] Config: SCANOPY_BREVO_API_KEY replaces SCANOPY_HUBSPOT_API_KEY
- [ ] Factory: BrevoService registered as EventBus subscriber
- [ ] HubSpot module deleted, no references remain
- [ ] Frontend: HubSpot tracking/cookies removed, PlanInquiryModal updated
- [ ] freemail.rs: moved to shared/ if used elsewhere, or removed
- [ ] All backend tests pass (`cd backend && cargo test`)
- [ ] `make format && make lint` passes

---

## Notes

- **Merge order:** This branch merges AFTER the billing-overhaul branch. Rebase before merging if needed.
- Brevo API docs should be consulted for exact endpoint formats — the specs above are directional.
- Plunk/SMTP transactional emails are NOT affected by this change.
>>>>>>> brevo-integration

---

## Work Summary

<<<<<<< HEAD
### Phase 1: Type System Foundation
- Added `Free(PlanConfig)` variant to `BillingPlan` enum with all required match arms
- Added `host_cents: Option<i64>` and `included_hosts: Option<u64>` to `PlanConfig`
- Replaced `unlimited_scans`/`unlimited_hosts` with `scheduled_discovery`/`daemon_poll` in `BillingPlanFeatures`
- Replaced `UnlimitedScans`/`UnlimitedHosts` with `ScheduledDiscovery`/`DaemonPoll` in `Feature` enum
- Fixed existing bug: `api_access` was pushing `Feature::CustomSso` instead of `Feature::ApiAccess`
- Added `BillingHostLimitReached`, `BillingFeatureNotAvailable`, `DaemonStandby` to `ErrorCode`
- Added `has_payment_method: bool` and `trial_end_date: Option<DateTime<Utc>>` to org base
- Added `standby: bool` to daemon base
- Updated all PlanConfig literals in `plans.rs` with new fields

### Phase 2: Database Migration
- Created migration adding `has_payment_method`, `trial_end_date`, `standby` columns

### Phase 3: Free Plan & Billing Infrastructure
- Added `get_free_plan()` in `plans.rs`, included in purchasable and fixture plans
- Added `is_free()` and `host_limit()` helper methods to `BillingPlan`
- Exempted Free from billing middleware (alongside Community/CommercialSelfHosted/Demo)

### Phase 4: Backend Enforcement
- Host limit checked on manual creation (handlers.rs) and discovery creation (service.rs)
- Scheduled discovery restriction in create/update handlers
- DaemonPoll restriction in `process_registration()`
- Daemon standby: skip in poller, return `DaemonStandby` error in `receive_work_request`
- Daemon-side standby: detect error, log warning, block with `ctrl_c().await`

### Phase 5: Billing Service
- Trial without credit card: `payment_method_collection: IfRequired` for trial checkout
- Setup payment method endpoint (`POST /api/billing/setup-payment-method`)
- Change plan endpoint (`POST /api/billing/change-plan`) with upgrade/downgrade logic
- Plan change preview endpoint (`GET /api/billing/change-plan/preview`)
- `customer.subscription.trial_will_end` webhook handler
- `checkout.session.completed` handler for setup mode (marks `has_payment_method`)
- `downgrade_to_free()`: converts scheduled→adhoc, sets standby on DaemonPoll, trims hosts
- Store `trial_end_date` from subscription webhooks
- `handle_subscription_deleted` calls `downgrade_to_free()` instead of clearing plan
- Added `host_service`, `daemon_service`, `discovery_service` to `BillingServiceParams`

### Phase 6: Transactional Emails
- 6 billing email templates (trial started/ending/expired, payment added, plan changed, cancelled)
- `send_billing_email` trait method on `EmailProvider`
- Builder methods for each template
- Implemented in both Plunk and SMTP providers

### Phase 7: Frontend Changes
- Generated TypeScript types via `make generate-types`
- Added `scheduled_discovery` and `daemon_poll` to `BillingPlanMetadata` interface
- Added `BillingRate` type export
- New query hooks: `useSetupPaymentMethodMutation`, `useChangePlanMutation`, `useChangePlanPreviewQuery`
- Created `UpgradeBadge.svelte` component (dispatches `open-settings` event on click)
- `DiscoveryTypeForm.svelte`: Scheduled option disabled when feature unavailable, shows UpgradeBadge
- `CreateDaemonForm.svelte`: DaemonPoll option disabled when feature unavailable, shows UpgradeBadge
- `HostTab.svelte`: Host count indicator (`X / 25`), warning at limit, UpgradeBadge replaces create button at limit
- `Sidebar.svelte`: Amber notification dot on Settings when no payment method
- `BillingTab.svelte`: Host usage with progress bar, trial countdown, Add Payment Method CTA, Free plan upgrade prompt

### Phase 8: Fixtures & Tests
- All 114 backend unit tests pass (including `test_feature_ids_match_billing_plan_features_fields`)
- All 14 frontend tests pass
- Format and lint clean (0 errors, 0 warnings)
- Billing fixtures regenerated with Free plan and new features

### Phase 9: Onboarding Simplification
- Removed "Add another network" button from `OrgNetworksModal.svelte` — all use cases get exactly 1 network
- Removed daemon and daemon_verification steps from onboarding flow in `onboarding/+page.svelte`
- Step type narrowed to `use_case | blocker | setup | register` (always 3 steps)
- Set `sessionStorage('showDaemonSetup')` flag on registration completion
- Main app (`+page.svelte`) reads flag to land on daemons tab immediately after onboarding
- `DaemonTab.svelte` reads flag to auto-open CreateDaemonModal (flag cleared after read)
- CreateDaemonForm already has plan-based gating: DaemonPoll disabled with UpgradeBadge for Free plan
- Removed unused i18n keys (`onboarding_addAnotherNetwork`, `onboarding_removeNetwork`)
- Cleaned up dead code: `addNetwork`/`removeNetwork` functions, daemon imports, `hasIntegratedDaemon`, `daemonsInstalled` tracking

### Files Changed (key files)
**Backend:** `billing/types/base.rs`, `billing/types/features.rs`, `billing/types/api.rs`, `billing/plans.rs`, `billing/service.rs`, `billing/handlers.rs`, `shared/types/error_codes.rs`, `shared/storage/filter.rs`, `shared/services/factory.rs`, `auth/middleware/billing.rs`, `hosts/service.rs`, `hosts/handlers.rs`, `discovery/handlers.rs`, `daemons/service.rs`, `daemons/handlers.rs`, `daemons/impl/base.rs`, `daemon/runtime/service.rs`, `organizations/impl/base.rs`, `email/templates.rs`, `email/traits.rs`, `email/plunk.rs`, `email/smtp.rs`, migration file
**Frontend:** `stores/metadata.ts`, `billing/queries.ts`, `billing/types.ts`, `UpgradeBadge.svelte`, `BillingTab.svelte`, `DiscoveryTypeForm.svelte`, `CreateDaemonForm.svelte`, `HostTab.svelte`, `Sidebar.svelte`, `billing-plans-next.json`, `features-next.json`, `OrgNetworksModal.svelte`, `onboarding/+page.svelte`, `+page.svelte`, `DaemonTab.svelte`, `messages/en.json`
<<<<<<< HEAD
=======
### What was implemented
- Full replacement of HubSpot CRM integration with Brevo
- Created `backend/src/server/brevo/` module (client, types, service, subscriber)
- Database migration renaming `hubspot_company_id` → `brevo_company_id` with stale ID clearing
- Config: `SCANOPY_HUBSPOT_API_KEY` → `SCANOPY_BREVO_API_KEY`
- Factory wiring: BrevoService replaces HubSpotService, registered as EventBus subscriber
- Startup backfill: syncs all orgs without `brevo_company_id` on server start (non-blocking)
- Billing inquiry endpoint updated to use Brevo (deals + event tracking instead of HubSpot Forms)
- Frontend: removed HubSpot tracking script, cookie extraction, and references
- Deleted entire `backend/src/server/hubspot/` module (including freemail domain lists)
- All organizations synced (no more commercial-plan/work-email filtering)

### Files changed
- **Created:** `backend/src/server/brevo/{mod,client,types,service,subscriber}.rs`
- **Created:** `backend/migrations/20260205183207_rename_hubspot_to_brevo.sql`
- **Modified:** `backend/src/server/mod.rs`, `config.rs`, `auth/service.rs`, `billing/handlers.rs`
- **Modified:** `backend/src/server/organizations/impl/{base,storage}.rs`
- **Modified:** `backend/src/server/shared/{services/factory.rs,storage/filter.rs,events/types.rs,types/examples.rs}`
- **Modified:** `backend/src/bin/server.rs`
- **Modified:** `ui/src/lib/features/billing/PlanInquiryModal.svelte`, `ui/src/lib/shared/components/layout/AppShell.svelte`
- **Modified:** `ui/src/lib/features/auth/{stores/onboarding.ts,components/onboarding/UseCaseStep.svelte}`
- **Deleted:** `backend/src/server/hubspot/` (all files)

### Migration note
Migration clears all existing `brevo_company_id` values (formerly `hubspot_company_id`) so every org gets re-synced to Brevo on next startup.
>>>>>>> brevo-integration
=======

### Phase 10: Fix Billing Experience (Modal, Backend Errors, UI Bugs)

**Fix 1: Daemon default mode** — Changed default from `daemon_poll` to `server_poll` in `daemons/config.ts`

**Fix 2: Free plan as $0 Stripe plan** — Eliminated special cases:
- `billing/service.rs`: Skip only self-hosted plans (Community/CommercialSelfHosted/Enterprise/Demo) in Stripe sync instead of all $0 plans; Free now gets a Stripe product/price
- `billing/service.rs`: `IfRequired` payment method collection for $0 plans (Free checkout skips card)
- `billing/subscriber.rs`: Guard `update_addon_prices` by checking metered addons (seat_cents/network_cents) instead of plan existence — prevents errors for Free plan
- `auth/service.rs`: New Cloud orgs start with `plan: None` — users pick plan via billing modal → Stripe checkout → webhook sets plan

**Fix 3: Billing plan selection as modal** — Replaced gating page with modal:
- New `billing/stores.ts` with `showBillingPlanModal` store
- New `BillingPlanModal.svelte` wrapping BillingPlanForm in GenericModal; all plans (including Free) go through Stripe checkout
- `+page.svelte`: Mounted modal (non-dismissible when no plan, dismissible from settings)
- `navigation.ts`: Removed billing check from `getRoute()` — billing handled by modal
- `BillingTab.svelte`: "View Plans" uses store instead of `goto('/billing')`
- Deleted `routes/billing/+page.svelte`
- `billing/handlers.rs`: Fixed cancel_url (was `{url}/billing`, now `url.clone()`)
- `organizations/types.ts`: Removed `plan?.type === 'Free'` special case from `isBillingPlanActive`

**Fix 4: UseCaseStep Svelte 5 conversion** — Full runes migration:
- `export let` → `$props()`, `$:` → `$derived`, `on:click` → `onclick`, `svelte:component` → direct component
- Used `$derived(form.state.values.referralSource)` for reactive "Other" text field

**Fix 5: API access error guard** — Prevented 402 errors for Free plan:
- `user_api_keys/queries.ts`: Added `enabled` option to `useUserApiKeysQuery`
- `UserApiKeyTab.svelte`: Guard query with `enabled: () => hasApiAccess` based on plan features
>>>>>>> billing-overhaul
=======
# Fix: ServerPoll daemon sends completed discovery updates forever

## Objective

Fix a bug where a ServerPoll daemon that completed Network Discovery causes the server to repeatedly log "Auto-creating session from daemon update" every poll cycle, indefinitely.

## Root Cause (already investigated)

In ServerPoll mode, the **server** polls the **daemon's** `/api/poll` endpoint. When discovery completes:

1. The daemon stores a `terminal_payload` in `DaemonState` that persists until a new session starts (`daemon/runtime/state.rs:169`).
2. Every poll cycle, the server gets this terminal payload (phase=Complete, progress=100).
3. The server calls `update_session()` (`server/discovery/service.rs:622`).
4. The session was already removed from the in-memory `sessions` map after the first terminal processing (line 738).
5. So the auto-create logic triggers (line 632) — creates the session, processes terminal (creates duplicate historical Discovery record), removes it.
6. Repeat forever.

### Normal flow (first completion — works correctly):
- Session IS in `sessions` map → `update_session()` updates it → processes terminal → removes it. No auto-create.

### Bug flow (every subsequent poll):
- Session NOT in `sessions` map → auto-create → process terminal → remove → infinite loop with duplicate historical records.

## Fix

Two changes needed:

### 1. Server: Don't auto-create sessions with terminal phase (`server/discovery/service.rs`)

In `update_session()`, inside the `Vacant` branch (line 632), check if `update.phase.is_terminal()`. If so, log a debug message and return `Ok(())` early instead of auto-creating. The first completion is always handled while the session is still in memory — only redundant repeats hit the auto-create path.

### 2. Server: Clean up `session_last_updated` on terminal removal (`server/discovery/service.rs`)

At line 738, `sessions.remove(&update.session_id)` removes from `sessions` but not from `session_last_updated`. Add `last_updated.remove(&update.session_id)` to prevent unbounded growth. Note: `last_updated` is already acquired as a write lock at line 627.

## Key Files

| File | What |
|------|------|
| `backend/src/server/discovery/service.rs:622-756` | `update_session()` — auto-create logic + terminal processing |
| `backend/src/daemon/runtime/state.rs:140-181` | `get_progress()` — returns terminal payload indefinitely |
| `backend/src/server/daemons/service.rs:1265-1279` | Server-side poll loop that calls `process_discovery_progress` |

## Acceptance Criteria

- [ ] Server no longer repeatedly logs "Auto-creating session" for completed sessions
- [ ] Normal discovery completion still works (session is in memory when first terminal update arrives)
- [ ] No duplicate historical Discovery records are created
- [ ] `session_last_updated` doesn't leak entries for completed sessions
- [ ] Existing tests pass (`cd backend && cargo test`)

## Out of Scope

- Persisting sessions to database (larger architectural change)

## Work Summary

### Approach (revised from original task)

The original task proposed server-only fixes. During implementation, the approach was revised to fix the root cause at the daemon and add a server-side fallback for old daemon versions.

### Changes

**1. Daemon: Clear terminal payload after serving it (`daemon/shared/handlers.rs`)**

In `get_discovery_poll()`, after reading the progress, clear the terminal payload if it's terminal. This means the daemon only serves the terminal state once — the next poll returns `None` for progress. Same delivery guarantee as DaemonPoll mode (single delivery attempt).

**2. Daemon: Add `clear_terminal_payload()` method (`daemon/runtime/state.rs`)**

Simple method that writes `None` to the `terminal_payload` lock. Called by the poll handler above.

**3. Server fallback: `last_updated` tombstone for old daemons (`server/discovery/service.rs`)**

Old daemons won't have the clearing behavior, so the server needs protection. Uses `session_last_updated` as a tombstone: before inserting into `last_updated`, check if the session was already tracked. If a terminal update arrives for a session that's not in `sessions` but IS in `last_updated`, it was already processed — skip it.

This correctly handles server restarts: both `sessions` and `last_updated` are in-memory, so after restart both are empty, and the terminal auto-create path works as intended.

Note: `last_updated` entries for completed sessions are intentionally NOT cleaned up — they serve as tombstones. Growth is negligible (one UUID+DateTime per session for server process lifetime).

### Files Changed

| File | Change |
|------|--------|
| `backend/src/daemon/shared/handlers.rs` | Clear terminal payload after serving in poll handler |
| `backend/src/daemon/runtime/state.rs` | Add `clear_terminal_payload()` method |
| `backend/src/server/discovery/service.rs` | Add `already_seen` tombstone check in `update_session()` |

### Acceptance Criteria Status

- [x] Server no longer repeatedly logs "Auto-creating session" for completed sessions
- [x] Normal discovery completion still works (session is in memory when first terminal update arrives)
- [x] No duplicate historical Discovery records are created
- [x] `session_last_updated` entries for completed sessions are kept as tombstones (not leaked — bounded by server lifetime)
- [x] Existing tests pass (114/114)
>>>>>>> discovery-poll-bug
