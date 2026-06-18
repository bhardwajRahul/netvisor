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
  "tests": []
}
,
{
  "branch": "feat/phase2-session-digest",
  "tests": []
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
      "id": "topology-checklist-step-focused-tab-only",
      "category": "Onboarding / Getting Started checklist",
      "description": "The 'View your topology' step completes ONLY when the user is focused on the Topology tab — not from background activity on other tabs — and persists after reload.",
      "steps": [
        "As an org with discovery completed and at least one host, log in and stay on a NON-topology tab (e.g. Home). Open the Getting Started checklist and confirm 'View your topology' is still incomplete after a minute. (Optionally watch the network tab: no request to /api/v1/topology/data with mark_viewed=true should be sent while off the topology tab.)",
        "Now click into the Topology tab and let it load.",
        "Return to the checklist and confirm the 'View your topology' step is now complete.",
        "Reload the page and confirm it stays complete.",
        "Open the Topology tab again and confirm no duplicate mark-viewed request is sent (milestone already set)."
      ],
      "setup": "Seed an org that has FirstDaemonRegistered + FirstDiscoveryCompleted in onboarding and at least one host on its default network, but NOT FirstTopologyRebuild. Simplest: run a real discovery that finds a host, or create a host via the API on a network whose org already has FirstDiscoveryCompleted.",
      "expected": "While off the topology tab the step stays incomplete (no milestone fires). After focusing the Topology tab the step completes within a few seconds and remains complete after reload. No repeat trigger on subsequent visits.",
      "status": null,
      "feedback": null
    },
    {
      "id": "email-prefs-autosave-with-toast",
      "category": "Settings / Email",
      "description": "Email preferences auto-save on toggle (no Save button) and show a success toast confirming the save.",
      "steps": [
        "Open Settings → Email and confirm there is NO Save button.",
        "Toggle the 'discovery digest' checkbox off and wait ~1s.",
        "Confirm a success toast ('Email preferences updated') appears.",
        "Reload the page and confirm the value persisted; toggle back on and confirm the toast + persistence again."
      ],
      "setup": "Any logged-in user. No special setup.",
      "expected": "Each toggle persists immediately (after a brief debounce) without a Save button, a success toast confirms the save, and the value survives reload.",
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
      "id": "pause-cooldown-message",
      "category": "Pause cooldown",
      "description": "Pause panel renders cooldown message instead of duration buttons when org paused within last 6 months",
      "setup": "Pick an org with an active paid subscription. Set organizations.last_paused_at to NOW() - 30 days (still within the 6-month cooldown).",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → continue to step 2"
      ],
      "expected": "The Pause panel renders the cooldown copy inside a yellow InlineWarning banner — AlertTriangle icon on the left, yellow border, yellow background, with text 'You last paused on {last-paused-date}. You can pause again on {next-eligible-date}' (~5 months from now). NOT plain `text-warning` paragraph text. The last-paused date matches the org's `last_paused_at` you set in setup. The Discount panel renders normally below it. Footer still has Back / Confirm Cancellation.",
      "status": null,
      "feedback": null
    },
    {
      "id": "discount-panel-visible-when-coupon-set",
      "category": "Discount save offer",
      "description": "Cancel modal renders the Discount panel only on Stripe-managed plans when STRIPE_SAVE_OFFER_COUPON_ID is configured; applying it shows a chip on BillingTab and hides the panel on subsequent visits",
      "setup": "Set STRIPE_SAVE_OFFER_COUPON_ID to a coupon ID that exists IN THE SAME STRIPE MODE as the secret key — a test-mode key requires a test-mode coupon, a live-mode key requires a live-mode coupon. Mismatch produces a 400 from Stripe ('No such coupon … exists in <other> mode'). Restart the server after setting the env var. Pick an org with an active paid Stripe-managed subscription (Pro/Business) and `last_discount_at IS NULL`.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → click 'Continue cancelling'",
        "On the save-offer screen, click 'Apply discount'",
        "Watch for the success / warning toast and the BillingTab plan card",
        "Re-open Cancel Subscription and pick 'Too expensive' again to confirm the discount panel is gone",
        "Switch to a non-Stripe-managed plan org (Free / Community / Demo / CommercialSelfHosted) and repeat opening the cancel modal"
      ],
      "expected": "On a Stripe-managed plan: step 2 renders both Pause and Discount panels. Discount panel body reads 'Stay subscribed at {percent_off}% off for {duration_in_months} months.' — both numbers pulled live from the configured Stripe coupon. After 'Apply discount', within ~2-4 seconds the success toast fires: 'Discount applied to your subscription.' (or 'may take a moment' on timeout). BillingTab plan card shows the base price crossed out (e.g. ~~$49.99~~) with the post-discount price as the primary number, plus the green chip '{percent_off}% off your subscription until {date}' below the plan name. On a subsequent visit, picking 'Too expensive' shows ONLY the Pause panel (Discount panel hidden because `last_discount_at` is now non-null). On a non-Stripe-managed plan (Free / Community / Demo / CommercialSelfHosted): opening the cancel modal and picking any reason shows NO save-offer panels — neither Pause nor Discount — even with the coupon configured. The user goes straight from reason → confirm-cancellation footer. The strikethrough + chip on BillingTab are also hidden on these plans.",
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
