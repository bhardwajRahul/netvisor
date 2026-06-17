var TEST_PLANS = [
{
  "branch": "refactor/email-trait",
  "tests": []
}
,
{
  "branch": "feat/event-model-typed-payloads",
  "notes": "Covers all email-driven flows and event-bus side effects across the auth/billing/event-subscriber refactors. All emails now route through the event bus via `Subscriber<Op>` impls registered via `inventory::submit!`; cancellations cascade via `InviteService::Subscriber<BillingOperation>` instead of a direct call. Tests are grouped into flows where state can be reused; truly independent tests omit `flow`/`sequence`. Programmatic checks (DB row updates for Pattern B flag columns, subscriber-name uniqueness at startup) are verified via `cargo test --lib` (300/300 green) and not included here.",
  "tests": [
    {
      "id": "auth-self-hosted-no-email-service-auto-verify",
      "category": "Auth emails",
      "description": "Self-hosted deployments without SMTP/Brevo configured auto-verify on register \u2014 no email sent. Previously this path locked invited users out.",
      "setup": "Run a self-hosted instance (`stripe_secret` unset, no `brevo_api_key`, no `smtp_*`). Verify in config that `has_email_service = false`.",
      "steps": [
        "Navigate to /register",
        "Complete password registration with org-creation flow"
      ],
      "expected": "Account is created with `email_verified = true` immediately. No email is dispatched (none can be).",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/phase2-topology-snapshots",
  "tests": []
}
,
{
  "branch": "feat/phase5-trial-ui",
  "tests": [
    {
      "id": "add-payment-cta-clicked-event-fires-from-all-surfaces",
      "category": "Telemetry",
      "description": "A single `add_payment_cta_clicked` PostHog event fires from every Add-Payment-Method surface (trial card, T-3d banner, T-1d modal, sidebar pill), with a `source` property identifying which surface. Stripe opens in a new tab so the originating tab stays put and the event fires synchronously via the normal `trackEvent` path.",
      "setup": "Trialing org without payment method. Open PostHog Live events or filter the network tab for posthog.",
      "steps": [
        "Click 'Add Payment Method' on the trial countdown card in Settings → Billing. A Stripe tab opens; close it. Check PostHog Live events in the originating tab.",
        "Set trial_end_date to ~T-2d and reload. Click the CTA on the T-3d banner. Close the Stripe tab. Check PostHog.",
        "Set trial_end_date to ~T-12h, clear `dismissed_today:trial_expiry_modal`, reload. Click the CTA on the T-1d modal. Close the Stripe tab. Check PostHog.",
        "Set trial_end_date back to ~T-6d, reload. Click the sidebar trial pill. Close the Stripe tab. Check PostHog."
      ],
      "expected": "Four `add_payment_cta_clicked` events fired across the four trials, with `source` values `trial_card`, `trial_banner`, `trial_modal`, and `sidebar_trial_pill` respectively. No `trial_card_cta_clicked` / `trial_banner_cta_clicked` / `trial_modal_cta_clicked` / `payment_method_setup_initiated` events fire from these CTAs.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/billing-telemetry-enrichments",
  "tests": [
    {
      "id": "stripe-portal-cancel-with-reason",
      "category": "Billing — Cancellation Telemetry",
      "description": "End-to-end: a Stripe Portal cancellation that selects a reason + writes a comment must surface in the cancellation_initiated PostHog event with the three new keys populated AND flip the org's plan_status to pending_cancellation in the app. (Portal cancels default to cancel_at_period_end=true and emit cancellation_initiated, not subscription_cancelled — the latter only fires when the period actually ends.)",
      "setup": "Pick (or create) a test organization on a paid plan with an active Stripe subscription. Open that org in the app, navigate to Settings → Billing → Manage Subscription to launch the Stripe Customer Portal. Confirm the Portal's cancel flow has 'Cancellation reason' and free-text 'Additional feedback' enabled in the Stripe dashboard's Billing → Customer Portal configuration.",
      "steps": [
        "Start tailing server logs in a separate terminal (e.g., `kubectl logs -f` or `docker logs -f` for the API pod, filtered to `cancellation_initiated`, `OrganizationService`, and `org_subscriber` if possible).",
        "From the app, click Manage Subscription to launch the Stripe Customer Portal.",
        "In the Portal, click Cancel subscription.",
        "Select a reason from the dropdown — pick 'Too expensive'.",
        "In the optional comment field, type 'Testing cancel telemetry — please ignore'.",
        "Confirm the cancellation in the Portal.",
        "Wait ~30 seconds for the customer.subscription.updated webhook to land and the async side-effects task to publish the event.",
        "Reload the app's BillingTab / Settings → Billing page and verify plan_status display.",
        "Open PostHog → Activity → filter by event name 'cancellation_initiated' and the test org's distinct_id (or org_id).",
        "Open the most recent event and inspect the metadata properties.",
        "Copy the captured server log lines into the feedback field if this test stays partial or fails. Specifically include EVERY line that says `Subscription update webhook: scheduled cancel detected` (with the `cancellation_details=…` value) and any `Subscription already pending cancellation, skipping re-emit` lines — those resolve whether Stripe is putting the details on the update webhook payload at all, or whether two webhooks fire and the second (with details) is being idempotency-skipped."
      ],
      "expected": "The cancellation_initiated event's metadata object contains: stripe_reason = 'cancellation_requested', stripe_feedback = 'too_expensive', comment = 'Testing cancel telemetry — please ignore', reason_code = null. planned_period_end is set to the actual subscription period end. The app's plan_status reflects the pending cancellation (org row should show plan_status = 'pending_cancellation').",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "stripe-portal-cancel-no-reason",
      "category": "Billing — Cancellation Telemetry",
      "description": "End-to-end: a Stripe Portal cancellation with no reason selected (or the Portal config doesn't ask) emits cancellation_initiated with stripe_feedback/comment/reason_code as null and stripe_reason populated by Stripe (typically cancellation_requested), AND the org's plan_status flips to pending_cancellation in the app.",
      "setup": "Pick (or create) a different test organization on a paid plan with an active Stripe subscription, distinct from the one used in the prior test. If the Portal cancel flow forces a reason selection, temporarily disable that requirement in the Stripe dashboard's Customer Portal configuration for the duration of this test, then restore it after.",
      "steps": [
        "Start tailing server logs as in the prior test.",
        "From the app, click Manage Subscription to launch the Stripe Customer Portal.",
        "In the Portal, click Cancel subscription.",
        "Skip the reason dropdown if possible (leave it blank or do not select an option).",
        "Leave the comment field empty.",
        "Confirm the cancellation in the Portal.",
        "Wait ~30 seconds for the webhook + async task.",
        "Reload the app's BillingTab / Settings → Billing page and verify plan_status display.",
        "Open PostHog → Activity → filter by event name 'cancellation_initiated' and the test org's distinct_id.",
        "Open the most recent event and inspect the metadata properties.",
        "Copy the captured server log lines as in the prior test and paste them into feedback if this fails."
      ],
      "expected": "The cancellation_initiated event's metadata contains stripe_feedback = null, comment = null, reason_code = null. stripe_reason will typically be 'cancellation_requested' (Stripe sets this for any Portal cancel, even without an explicit user pick). The app's plan_status reflects pending cancellation.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "admin-initiated-cancel-via-stripe-dashboard",
      "category": "Billing — Cancellation Telemetry",
      "description": "End-to-end: an admin-initiated cancel from the Stripe dashboard using 'Cancel immediately' hits handle_subscription_deleted directly and emits subscription_cancelled (not cancellation_initiated).",
      "setup": "Pick (or create) a test organization on a paid plan with an active Stripe subscription, distinct from the prior tests. You will cancel this subscription from the Stripe dashboard, not from the Scanopy app.",
      "steps": [
        "Open the Stripe dashboard (test mode), navigate to Customers → find the test customer.",
        "Open the active subscription for that customer.",
        "Click the '…' menu → 'Cancel subscription' → 'Cancel immediately' (do not pick a reason — Stripe's dashboard cancel does not collect cancellation_details).",
        "Confirm the cancellation.",
        "Wait ~30 seconds for the webhook + async task.",
        "Open PostHog → Activity → filter by event name 'subscription_cancelled' and the test org's distinct_id.",
        "Open the most recent event and inspect the metadata properties."
      ],
      "expected": "The subscription_cancelled event fires with: stripe_reason = null (or cancellation_requested depending on Stripe), stripe_feedback = null, comment = null, was_trialing matches pre-cancel state, mrr_amount_cents matches the canceled subscription's monthly line total, tenure_days matches days since org.created_at. The org's plan_status in the app reflects the downgrade to Free.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "audit/billing-modal-ctas",
  "tests": []
}
,
{
  "branch": "audit/banner-conditions-and-payment-prompt",
  "tests": [
    {
      "id": "no-payment-banner-free-hidden",
      "category": "No-Payment Banner",
      "description": "Free / non-Stripe plans never show the banner",
      "setup": "Set the org to the Free plan (plan_status null). Repeat for Demo / Community / CommercialSelfHosted if those deployments are available.",
      "steps": [
        "Reload the app",
        "Observe the banner stack"
      ],
      "expected": "No no-payment-method banner is shown for any non-Stripe plan.",
      "flow": "setup",
      "sequence": 6,
      "status": null,
      "feedback": null
    },
    {
      "id": "demo-and-email-banner-copy",
      "category": "i18n Fix",
      "description": "Demo banner and email-verification banner render correct copy after i18n migration",
      "setup": "For the demo banner: log into a Demo-plan org. For the email banner: use an account whose email is not yet verified.",
      "steps": [
        "Load the app in each scenario",
        "Read the banner text and button labels"
      ],
      "expected": "Demo banner: \"You're exploring the demo. Ready to map your own network?\" with a 'Create Account' link. Email banner: 'Please verify your email...' with a 'Resend' button that shows 'Sending...' while in flight and a success toast 'Verification email sent. Check your inbox.'",
      "flow": "setup",
      "sequence": 10,
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/stripe-webhook-org-deleted",
  "tests": [
    {
      "id": "trial-will-end-webhook-after-org-deleted",
      "category": "Webhook safety net",
      "description": "After an org is deleted, a Stripe customer.subscription.trial_will_end webhook fired for the (now-orphaned) customer returns 200 instead of 500.",
      "setup": "Reuse the deleted org from test 1, OR if running standalone: create org, upgrade to paid trial, downgrade to Free, delete org. You need the stripe_customer_id and the subscription id of a (canceled) subscription that previously belonged to that customer. From the Stripe dashboard 'Events' or 'Webhooks' tab, identify a past `customer.subscription.trial_will_end` event for that subscription, OR use the Stripe CLI `stripe events resend <event_id>` to re-deliver one. If no real event exists, use Stripe CLI `stripe trigger customer.subscription.trial_will_end` against a fixture and manually edit the metadata.organization_id to match the deleted org (advanced — easier path is to simply replay an old event).",
      "steps": [
        "Tail the backend logs (`docker logs -f backend` or equivalent).",
        "Trigger / replay the `customer.subscription.trial_will_end` event from the Stripe dashboard or CLI so it hits the local webhook endpoint.",
        "Confirm the webhook response is HTTP 200 (visible in Stripe dashboard 'Events' tab and/or backend access logs).",
        "Confirm backend logs show a WARN with `organization_id=<id>` and `event=\"trial_will_end\"` and the message `Stripe webhook for deleted organization — skipping`.",
        "Confirm Stripe does NOT mark the event for retry (no '500' status, no exponential-backoff retry attempts in the dashboard)."
      ],
      "expected": "Webhook returns 200, single WARN log line, no retries, no error logs.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "subscription-deleted-webhook-after-org-deleted",
      "category": "Webhook safety net",
      "description": "After an org is deleted, a Stripe customer.subscription.deleted webhook returns 200 and does NOT spawn the async side-effect task (which would try to look up the deleted org).",
      "setup": "Same as test 2 but with a `customer.subscription.deleted` event instead. Note: the customer-delete in test 1 will itself trigger a real `subscription.deleted` event for any active subs at the time — that event arrives *after* the org row has been deleted, which is exactly the scenario this guards against. So this test naturally falls out of test 1: just watch the logs after running test 1.",
      "steps": [
        "After completing test 1, check the Stripe dashboard 'Events' tab for the `customer.subscription.deleted` event(s) generated when DeleteCustomer auto-canceled the subscription.",
        "Confirm each such event was delivered with HTTP 200.",
        "In backend logs, confirm a WARN line with `organization_id=<id>`, `subscription_id=<sub_id>`, and `event=\"subscription_deleted\"`.",
        "Confirm there are NO 'Failed to process subscription deletion side effects' ERROR logs — the early-return prevents the spawned side-effect task from running."
      ],
      "expected": "Webhook returns 200; WARN log emitted; no async side-effect ERROR logs; no spawned tokio task ran.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "checkout-session-completed-webhook-after-org-deleted",
      "category": "Webhook safety net",
      "description": "After an org is deleted, a Stripe checkout.session.completed webhook returns 200 instead of 500.",
      "setup": "Reuse a deleted org with stripe_customer_id, OR replay an old `checkout.session.completed` event from the Stripe dashboard for that customer's prior checkout session.",
      "steps": [
        "From the Stripe dashboard, find a past `checkout.session.completed` event tied to the deleted org's customer (the original signup checkout).",
        "Use 'Resend webhook' on that event so it hits the local endpoint.",
        "Confirm HTTP 200 response.",
        "Confirm backend logs show a WARN with `organization_id=<id>` and `event=\"checkout_session_completed\"`."
      ],
      "expected": "Webhook returns 200; WARN log emitted; no error logs.",
      "flow": "setup",
      "sequence": 4,
      "status": null,
      "feedback": null
    },
    {
      "id": "org-delete-survives-stripe-failure",
      "category": "Stripe teardown",
      "description": "If the Stripe API call fails during org deletion (network error, invalid customer, etc.), the org is still deleted and the failure is logged.",
      "setup": "Create an org, upgrade to a paid trial, downgrade to Free. Then directly edit the org's stripe_customer_id in the DB to a syntactically valid but non-existent customer ID (e.g., `cus_NEVER_EXISTED_12345`). This forces Stripe to return an error on DeleteCustomer.",
      "steps": [
        "Sign in as the owner.",
        "Tail backend logs.",
        "Delete the organization via the UI.",
        "Confirm the deletion succeeds (you are signed out, the org is gone from the DB).",
        "Confirm backend logs contain an ERROR line: `Failed to delete Stripe customer during org deletion — proceeding` with the bogus customer_id and the Stripe error string."
      ],
      "expected": "Org deletion completes (200); ERROR log captured with the failure detail; no 500 returned to the user.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/phase2-session-digest",
  "tests": [
    {
      "id": "digest-utm-and-glyphs",
      "category": "Discovery digest — glyphs + tracking + settings link",
      "description": "Host cards use the same glyphs as tag chips (+, ?, −) instead of textual badges; every clickable link in the digest carries UTM tracking; and the 'Manage email preferences' footer link opens the in-app Settings modal on the Email tab.",
      "setup": "Run two Unified discoveries on a multi-subnet network. The first scan covers all subnets (seed). The second scan is restricted to one subnet so other subnets' hosts go missing — producing a digest with hosts_added, hosts_vanished, and tag deltas to inspect.",
      "steps": [
        "Open the second digest email in the Owner's inbox.",
        "In the 'New hosts discovered' section, confirm each card's badge is a small green pill containing only the '+' glyph (hover shows 'New').",
        "In the 'Missing hosts' section, confirm each card's badge is the '?' or '−' glyph (italic amber or strikethrough red), not the words 'Possibly missing' / 'Missing'.",
        "Hover (or long-press) any host card title link, any tag chip, and the 'Manage email preferences' link in the footer.",
        "For each, verify the URL contains 'utm_source=email', 'utm_campaign=discovery_digest', and 'utm_medium=digest'.",
        "Click the 'Manage email preferences' link in the footer.",
        "Verify the app opens with the Settings modal on the Email tab (URL is /?modal=settings&tab=email, not /settings)."
      ],
      "expected": "Host badges are glyph-only with hover titles; every link in the digest carries the three UTM params; the settings link opens the in-app modal on the Email tab.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/cached-plan-deploy-window",
  "tests": [
    {
      "id": "app-loads-with-new-pool-config",
      "category": "Smoke",
      "description": "Server starts and the app loads in the browser. Confirms PgConnectOptions/PgPoolOptions pool construction works end-to-end (the change is to DB pool init — if this works, the pool is functional).",
      "steps": [
        "Start the backend with the new build",
        "Open the UI in a browser and sign in",
        "Navigate to the Hosts page (or any page that lists entities) — list loads with no error toast"
      ],
      "expected": "App loads, login succeeds, hosts list renders (or empty-state if no data). No DB-related error toast."
    },
    {
      "id": "daemon-poll-hot-path-still-works",
      "category": "Smoke",
      "description": "Hot-path daemon endpoints that query the `daemons` table (the table altered in the v0.16.0..v0.16.1 window) still function with the new pool config.",
      "setup": "Generate a fresh daemon enrollment token via the UI (or API) so a daemon can register against this server.",
      "steps": [
        "Install/start a daemon against this server using the enrollment token",
        "Wait ~30s for the daemon to register and start polling",
        "Open the Daemons page in the UI and confirm the new daemon appears as 'online' / 'active'"
      ],
      "expected": "Daemon registers, polls succeed, UI shows it as healthy. Backend logs show no SQLSTATE 0A000 / cached_plan_invalidated entries."
    }
  ]
}
,
{
  "branch": "fix/onboarding-and-ux-polish",
  "tests": [
    {
      "id": "no-payment-banner-no-reload",
      "category": "Billing / Banners",
      "description": "The 'no payment method on file' banner appears immediately after selecting a paid/trial plan, without a page reload.",
      "steps": [
        "Sign up as a brand-new user (fresh org) so the plan-selection modal appears.",
        "Select a paid plan that includes a free trial (so it activates in-app rather than redirecting to Stripe).",
        "Without reloading the page, watch the top of the app after the modal closes."
      ],
      "setup": "Use a cloud-mode environment with billing enabled. Register a fresh account with no payment method on file so the plan-selection modal is shown on first login. If needed, ensure the selected plan has trial_days > 0 and base_cents > 0.",
      "expected": "Within a few seconds (no manual reload) the persistent warning banner 'Your subscription has no payment method on file. Add one to avoid losing access.' appears.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "daemon-prompt-skip-first-click",
      "category": "Onboarding / Daemon prompt",
      "description": "'Skip for now' on the 'Start Discovering Your Network' modal closes it on the first click and it does not return after reload.",
      "steps": [
        "Continuing as the new org with no daemons installed, wait for the 'Start Discovering Your Network' modal to appear (it auto-opens, including right after the plan modal closes).",
        "Click 'Skip for now' exactly once and observe whether the modal closes.",
        "Reload the page and confirm the modal does not reappear."
      ],
      "setup": "Member+ user in a fresh org that has completed OrgCreated, has no daemons, and has not yet responded to the daemon prompt. The plan-selection step (test 1) is one reliable way to reach this state, since the prompt opens when the billing modal closes.",
      "expected": "The modal closes on the FIRST click of 'Skip for now' (no second click needed). After reloading, the modal stays gone.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "daemon-prompt-get-started-persists",
      "category": "Onboarding / Daemon prompt",
      "description": "Choosing 'Get Started' and then backing out of the daemon-create modal does not re-show the prompt (now or after reload).",
      "steps": [
        "Trigger the 'Start Discovering Your Network' modal for a fresh org (no daemons, no prior response).",
        "Click 'Get Started' — the daemon-create modal opens.",
        "Close the daemon-create modal without finishing, then reload the page."
      ],
      "setup": "Member+ user in a fresh org with no daemons and no prior daemon-prompt response. Reset the org's onboarding (remove DaemonPromptDismissed/DaemonPromptAccepted) if it was already dismissed in a prior test.",
      "expected": "The 'Start Discovering Your Network' prompt does not reappear after closing the create-daemon modal, and stays gone after reload.",
      "status": null,
      "feedback": null
    },
    {
      "id": "daemon-prompt-hidden-for-viewers",
      "category": "Onboarding / Daemon prompt",
      "description": "The daemon prompt is not auto-shown to Viewer-permission users.",
      "steps": [
        "Log in as a user whose org permission is Viewer, into an org that has no daemons and would otherwise show the prompt.",
        "Land on the home/app view and observe."
      ],
      "setup": "Create/seed an org with no daemons and OrgCreated set, then add a user with Viewer permissions (invite + accept, or set permissions directly). Log in as that Viewer.",
      "expected": "The 'Start Discovering Your Network' modal never auto-opens for the Viewer.",
      "status": null,
      "feedback": null
    },
    {
      "id": "topology-checklist-step-completes",
      "category": "Onboarding / Getting Started checklist",
      "description": "The 'View your topology' Getting Started step completes after the user opens the topology tab with at least one host, and stays complete after reload.",
      "steps": [
        "As an org that has completed discovery and has at least one host, open the Getting Started checklist and note the 'View your topology' step is incomplete.",
        "Navigate to the Topology tab and let it load.",
        "Return to the checklist (home or sidebar) and observe the step.",
        "Reload the page and confirm the step is still complete."
      ],
      "setup": "Seed an org that has FirstDaemonRegistered + FirstDiscoveryCompleted in onboarding and at least one host on its default network, but does NOT yet have FirstTopologyRebuild. The simplest path: run a real discovery that finds a host; alternatively create a host via the API on a network whose org already has FirstDiscoveryCompleted.",
      "expected": "After opening the Topology tab, the 'View your topology' checklist step transitions to completed within a few seconds, and remains completed after reload.",
      "status": null,
      "feedback": null
    },
    {
      "id": "email-prefs-autosave",
      "category": "Settings / Email",
      "description": "Email preferences auto-save on toggle with no Save button.",
      "steps": [
        "Open Settings → Email.",
        "Confirm there is NO Save button next to the preference.",
        "Toggle the 'discovery digest' checkbox off, then reload the page and confirm it is still off.",
        "Toggle it back on, reload, and confirm it persisted."
      ],
      "setup": "Any logged-in user. No special setup.",
      "expected": "Each toggle persists immediately (after a brief debounce) without a Save button; the value survives reload.",
      "status": null,
      "feedback": null
    },
    {
      "id": "email-prefs-revert-on-error",
      "category": "Settings / Email",
      "description": "A failed save reverts the toggle and shows an error toast.",
      "steps": [
        "Open Settings → Email.",
        "Using browser devtools, block/offline the network (or block PUT /api/v1/users/*).",
        "Toggle the 'discovery digest' checkbox and wait ~1s."
      ],
      "setup": "Any logged-in user. Prepare to simulate a failed request to PUT /api/v1/users/{id} (devtools offline mode or request blocking).",
      "expected": "The checkbox snaps back to its previous state and an error toast ('Failed to update email preferences') appears — the UI never shows a state the server rejected.",
      "status": null,
      "feedback": null
    },
    {
      "id": "sidebar-upgrade-cta-alignment",
      "category": "Layout / Sidebar",
      "description": "The sidebar 'Upgrade' CTA and trial-ending badge align with Settings and Support.",
      "steps": [
        "With the sidebar expanded, view the bottom navigation where 'Upgrade' (or a trial-ending pill) sits above Settings and Support.",
        "Compare the icon column and the text column across the Upgrade/trial row and the Settings/Support rows."
      ],
      "setup": "Log in as an org on a Free plan (shows the 'Upgrade' CTA) or a trialing org (shows the trial-ending pill) so the amber CTA row renders in the sidebar.",
      "expected": "The amber CTA's icon and text line up with the Settings/Support rows (same icon size and same text start position) — no slight left/up shift.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/scd2-data-integrity-and-snapshot-views",
  "tests": []
}
,
{
  "branch": "feat/phase5-subscription-mechanics",
  "tests": [
    {
      "id": "cancel-modal-pause-redeem-flips-status",
      "category": "Cancel modal / Pause",
      "description": "Redeeming the pause save-offer pauses the subscription and surfaces the Resume button",
      "setup": "Pick an org with an active paid subscription (last_paused_at null so no cooldown applies). Confirm the subscription is in Active state on Stripe (not trialing, not past_due).",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → click 'Continue cancelling' to reach the save-offer screen",
        "Click '60 days' in the Pause panel and verify the 'Pause until {date}' preview updates",
        "Click 'Pause subscription'",
        "Watch for either the success toast OR the 'may take a moment' warning toast, then look at the Billing tab status pill and the action button"
      ],
      "expected": "Within ~2-4 seconds the success toast fires: 'Subscription paused until {date}' AFTER the org payload has flipped to paused (not just on the API 200). Status pill shows 'Paused' (orange). The hard-gate Settings modal auto-opens to Billing tab. The Billing tab shows ONLY a 'Resume now' button — no 'Change Your Plan' / 'View Plans' button alongside it, and no BillingPlanModal opens on top of the hard gate. The owner inbox receives an email with subject 'Your Scanopy Subscription is Paused'. If the auto-poll reaches its 20s window without seeing the flip, the toast is a warning instead: 'Pause request accepted. It may take a moment to reflect across your account.' Server logs (INFO) show 'Stripe accepted pause_collection' with pause_collection_set=true.",
      "status": null,
      "feedback": null
    },
    {
      "id": "pause-rejected-when-not-active",
      "category": "Cancel modal / Pause eligibility",
      "description": "Backend refuses pause when the Stripe subscription is not in Active status",
      "setup": "Pick an org whose Stripe subscription is in PastDue or already Paused state. Trialing is intentionally excluded: while trialing, the cancel modal hides the save-offer screen entirely (by design — pause/discount only make sense for billing subscribers), so the backend eligibility check is unreachable from the modal in that state.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → continue to the save-offer screen",
        "Click 'Pause subscription'"
      ],
      "expected": "Error toast: 'Error pausing subscription: Subscription must be active to pause; current status: <past_due | paused>'. Modal stays open. Subscription on Stripe is unchanged (pause_collection not set).",
      "status": null,
      "feedback": null
    },
    {
      "id": "resume-restores-active",
      "category": "Pause/resume",
      "description": "Clicking Resume now flips the subscription back to active",
      "steps": [
        "From the previous test (org now paused), click 'Resume now'",
        "Confirm the browser confirm() prompt",
        "Wait for the toast"
      ],
      "expected": "Success toast 'Subscription resumed.' fires AFTER the org actually flips to active (auto-poll up to 20s). On timeout, warning toast: 'Resume request accepted. It may take a moment to reflect across your account.' Status pill flips back to 'Active' (green) without a manual page reload. Hard-gate Settings modal becomes dismissible again; 'Resume now' is replaced by Manage Subscription + Cancel Subscription. The owner inbox receives an email with subject 'Your Scanopy Subscription is Active Again'. Server logs do NOT show the prior 'You can only resume a subscription if it is paused' Stripe error — the resume path now sends `pause_collection=` via a custom StripeRequest impl rather than the SDK's ResumeSubscription endpoint.",
      "flow": "cancel-too-expensive",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "pause-cooldown-message",
      "category": "Pause cooldown",
      "description": "Pause panel renders cooldown message instead of duration buttons when org paused within last 6 months",
      "setup": "Pick an org with an active paid subscription. Set organizations.last_paused_at to NOW() - 30 days (still within the 6-month cooldown).",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → continue to step 2"
      ],
      "expected": "The Pause panel renders 'You last paused on {last-paused-date}. You can pause again on {next-eligible-date}' (~5 months from now) instead of the 30/60/90 buttons. The last-paused date matches the org's `last_paused_at` you set in setup. The Discount panel renders normally. Footer still has Back / Confirm Cancellation.",
      "status": null,
      "feedback": null
    },
    {
      "id": "discount-panel-visible-when-coupon-set",
      "category": "Discount save offer",
      "description": "Cancel modal renders the Discount panel when STRIPE_SAVE_OFFER_COUPON_ID is configured, applying it shows a chip on BillingTab and hides the panel on subsequent visits",
      "setup": "Set STRIPE_SAVE_OFFER_COUPON_ID to a coupon ID that exists IN THE SAME STRIPE MODE as the secret key — a test-mode key requires a test-mode coupon, a live-mode key requires a live-mode coupon. Mismatch produces a 400 from Stripe ('No such coupon … exists in <other> mode'). Restart the server after setting the env var. Pick an org with an active paid subscription and `last_discount_at IS NULL` (one-time-use is fresh).",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → click 'Continue cancelling'",
        "On the save-offer screen, click 'Apply discount'",
        "Watch for the success / warning toast and the BillingTab plan card",
        "Re-open Cancel Subscription and pick 'Too expensive' again to confirm the discount panel is gone"
      ],
      "expected": "Initial visit: Step 2 renders both Pause and Discount panels. The Discount panel body reads 'Stay subscribed at {percent_off}% off for {duration_in_months} months.' — both numbers pulled live from the configured Stripe coupon. After 'Apply discount', within ~2-4 seconds the success toast fires: 'Discount applied to your subscription.' (or the 'may take a moment' warning on timeout). BillingTab plan card shows the base price crossed out (e.g. ~~$49.99~~) with the post-discount price as the primary number, plus the green chip '{percent_off}% off your subscription until {date}' below the plan name. The strikethrough + chip render ONLY for Stripe-managed plans — on a Demo / Community / CommercialSelfHosted plan, even if the discount columns are populated, neither shows. On a subsequent Cancel Subscription visit, picking 'Too expensive' shows ONLY the Pause panel (Discount panel hidden because `last_discount_at` is now non-null).",
      "status": null,
      "feedback": null
    },
    {
      "id": "discount-second-attempt-rejected-server-side",
      "category": "Discount save offer / abuse prevention",
      "description": "Backend refuses a second discount apply even if the frontend gate is bypassed",
      "setup": "Pick an org with `organizations.last_discount_at IS NOT NULL` (any past timestamp). STRIPE_SAVE_OFFER_COUPON_ID configured.",
      "steps": [
        "POST /api/billing/cancel/apply-discount directly (e.g., from devtools console with the session cookie) — bypassing the modal-side filter"
      ],
      "expected": "Server returns 400 with body containing 'You've already used your one-time discount.' Stripe subscription is unchanged (no second discount applied).",
      "status": null,
      "feedback": null
    },
    {
      "id": "stripe-metadata-stash",
      "category": "Cancel — Stripe-side verification",
      "description": "Confirmed cancellations write the canonical Scanopy reason to Stripe Subscription metadata",
      "setup": "Run the confirm-cancel flow once: pick an org with active paid subscription, open Cancel Subscription, pick 'Other', Continue, Confirm cancellation. Then look up the subscription in the Stripe dashboard.",
      "steps": [
        "Open Stripe Dashboard → Customers → {test customer}",
        "Click the subscription",
        "Scroll to Metadata"
      ],
      "expected": "Subscription metadata contains 'scanopy_cancel_reason: other' (or whatever reason was picked). Cancellation Details shows the comment if one was entered.",
      "status": null,
      "feedback": null
    },
    {
      "id": "reactivate-clears-pending-cancellation-and-emails",
      "category": "Reactivate flow",
      "description": "Clicking Reactivate Subscription clears the pending cancellation on Stripe, the BillingTab returns to its active state without a manual reload, and the owner receives an email",
      "setup": "Pick an org with an active paid subscription. Open Cancel Subscription, pick 'Other', click Continue cancelling, then Confirm cancellation. Wait for status to read 'Downgrading'.",
      "steps": [
        "Click 'Reactivate Subscription'",
        "Watch for the success / warning toast",
        "Watch the BillingTab — do NOT manually reload the page",
        "Check the org owner's inbox after ~1 minute"
      ],
      "expected": "Success toast 'Subscription reactivated.' fires AFTER the org payload actually flips to active (auto-poll up to 20s). On timeout, warning toast: 'Reactivate request accepted. It may take a moment to reflect across your account.' Status pill flips from 'Downgrading' back to 'Active' (green) on its own. The Reactivate Subscription button disappears, replaced by Manage Subscription + Cancel Subscription. The 'plan will switch to Free' inline warning is gone. Stripe dashboard shows `cancel_at_period_end: false`. Owner's inbox contains an email subject 'Your Scanopy subscription is active again'.",
      "flow": "reactivate-flow",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "pause-status-triggers-billing-modal-gate",
      "category": "Pause UI gate (past_due parity)",
      "description": "Paused subscriptions trigger the same UI hard-gate as past_due: Settings modal auto-opens to Billing and is non-dismissible",
      "setup": "Pick an org currently in 'paused' state (run cancel-modal-pause-redeem-flips-status first, or set organizations.plan_status = 'paused' directly).",
      "steps": [
        "Reload the app / log in fresh",
        "Try to close the auto-opened Settings modal: click the X, click outside, press Escape",
        "Click Resume now"
      ],
      "expected": "On load, the Settings modal opens automatically on the Billing tab. None of X, click-outside, or Escape close it. After clicking Resume now and the status flips to Active, the modal becomes dismissible again and the user can navigate freely.",
      "status": null,
      "feedback": null
    },
    {
      "id": "paused-state-has-no-sidebar-dot-or-inline-alert",
      "category": "Pause UI gate (past_due parity)",
      "description": "Paused state does NOT add a sidebar notification dot or an inline alert — the hard gate is the only attention surface",
      "setup": "Same setup as pause-status-triggers-billing-modal-gate — org in 'paused' state.",
      "steps": [
        "Look at the sidebar gear / settings icon (note: the hard gate covers most of the app; the sidebar should still be visible behind it)",
        "Once the gate is dismissible again (after Resume), or by temporarily setting plan_status='active' for inspection, look at the BillingTab content above the action button"
      ],
      "expected": "No red dot on the sidebar gear icon while paused (past_due still shows one; paused does not). The BillingTab has no inline alert above the action button while paused — the hard gate carries the message. The status pill in the BillingTab still reads 'Paused' (orange) so the state is visible once the gate is opened.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/phase5-quick-wins",
  "tests": []
}
,
{
  "branch": "feat/license-grace-period",
  "tests": []
}
];
