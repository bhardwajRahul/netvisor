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
      "id": "welcome-banner-24h-window",
      "category": "Post-Stripe welcome",
      "description": "Welcome banner stops rendering 24h after the activation marker even without dismissal.",
      "setup": "Manually set `localStorage.plan_activated_at` to 25 hours ago: `localStorage.setItem('plan_activated_at', String(Date.now() - 25*60*60*1000))`. Clear `appbanner_dismissed:welcome_banner` if present.",
      "steps": [
        "Reload the app while logged in as the org Owner with an active or trialing-with-payment subscription.",
        "Look for the welcome banner."
      ],
      "expected": "No banner renders — the 24h window has elapsed.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "add-payment-cta-clicked-event-fires-from-all-surfaces",
      "category": "Telemetry",
      "description": "A single `add_payment_cta_clicked` PostHog event fires from every Add-Payment-Method surface (trial card, T-3d banner, T-1d modal, sidebar pill), with a `source` property identifying which surface. The event is deferred until after the Stripe redirect so it survives the hard navigation. The pre-existing `payment_method_setup_initiated` event no longer fires from these CTAs.",
      "setup": "Trialing org without payment method. Open PostHog Live events or filter the network tab for posthog.",
      "steps": [
        "Click 'Add Payment Method' on the trial countdown card in Settings → Billing. Cancel the Stripe redirect, come back to the app. Check PostHog Live events.",
        "Set trial_end_date to ~T-2d and reload. Click the CTA on the T-3d banner. Cancel and return. Check PostHog.",
        "Set trial_end_date to ~T-12h, clear `dismissed_today:trial_expiry_modal`, reload. Click the CTA on the T-1d modal. Cancel and return. Check PostHog.",
        "Set trial_end_date back to ~T-6d, reload. Click the sidebar trial pill. Cancel and return. Check PostHog."
      ],
      "expected": "Four events fired across the four trials, all named `add_payment_cta_clicked`, with `source` values `trial_card`, `trial_banner`, `trial_modal`, and `sidebar_trial_pill` respectively. No `trial_card_cta_clicked` / `trial_banner_cta_clicked` / `trial_modal_cta_clicked` / `payment_method_setup_initiated` events fire from these CTAs.",
      "flow": "setup",
      "sequence": 2,
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
        "Copy the captured server log lines (especially any lines mentioning `Publishing CancellationInitiated`, `Published CancellationInitiated`, `OrganizationService` subscriber `handle()`, `current_plan_status`, `implied_status`, `changed=true/false`, `noop=true`, or update errors) and paste them into the feedback field if this test fails."
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
  "tests": [
    {
      "id": "cancelled-state-hides-manage-cancel",
      "category": "Billing CTA visibility",
      "description": "A fully-cancelled paid org no longer shows Manage Subscription or Cancel Subscription; only the resubscribe button remains.",
      "steps": [
        "Open Settings → Billing tab as an Owner of the cancelled org.",
        "Inspect the Current Plan card buttons."
      ],
      "setup": "Put a test org into the cancelled state with the paid plan retained: set organizations.plan to a paid plan (e.g. Pro) and organizations.plan_status = 'cancelled' for the org. (Mirrors a real cancellation, where plan_status flips to 'cancelled' but org.plan keeps the old paid plan.)",
      "expected": "Only one primary button labeled 'Upgrade plan' is shown (opens the plan-selection modal). No 'Manage Subscription' and no 'Cancel Subscription' buttons appear.",
      "status": null,
      "feedback": null
    },
    {
      "id": "non-stripe-plan-hides-manage-cancel",
      "category": "Billing CTA visibility",
      "description": "Non-Stripe non-Free plans (Community/Demo/CommercialSelfHosted/Enterprise) show only the upgrade button, no Stripe CTAs.",
      "steps": [
        "Open Settings → Billing tab as an Owner of a Community-plan org.",
        "Inspect the Current Plan card buttons."
      ],
      "setup": "Use (or create) an org on the Community plan: organizations.plan = Community, organizations.plan_status = NULL, stripe_customer_id = NULL (the default self-hosted state).",
      "expected": "Only an 'Upgrade plan' button is shown. No 'Manage Subscription' and no 'Cancel Subscription' buttons (these would hit Stripe endpoints with no customer/subscription).",
      "status": null,
      "feedback": null
    },
    {
      "id": "active-paid-ctas-regression",
      "category": "Billing CTA visibility (regression)",
      "description": "Active paid orgs still show Manage Subscription + Cancel Subscription.",
      "steps": [
        "Open Settings → Billing tab as Owner of an active paid org.",
        "Inspect the Current Plan card buttons."
      ],
      "setup": "Org with organizations.plan = paid plan, plan_status = 'active', has_payment_method = true.",
      "expected": "'Change plan' (primary) plus 'Manage Subscription' and 'Cancel Subscription' (secondary) are all shown. Cancel opens the cancel modal which DOES show configured save offers for the chosen reason.",
      "status": null,
      "feedback": null
    },
    {
      "id": "pending-cancellation-hides-change-plan",
      "category": "Billing CTA visibility (regression)",
      "description": "Pending-cancellation (downgrading) orgs show Reactivate + Manage Subscription, and NO 'Change plan' button.",
      "steps": [
        "Open Settings → Billing tab as Owner of a pending-cancellation org.",
        "Inspect the Current Plan card buttons."
      ],
      "setup": "Org with organizations.plan = paid plan and plan_status = 'pending_cancellation' (a subscription with cancel_at_period_end set).",
      "expected": "'Reactivate Subscription' (primary) and 'Manage Subscription' (secondary) are shown. The 'Change plan' button is NOT shown (users are pushed to Reactivate). Reactivate clears the pending cancellation.",
      "status": null,
      "feedback": null
    },
    {
      "id": "stripe-ctas-open-new-tab-and-poller-converges",
      "category": "Stripe redirects",
      "description": "Manage Subscription, Add Payment Method, and plan checkout each open Stripe in a NEW tab; the original tab updates on its own (poller) once the change is processed, without a manual refresh.",
      "steps": [
        "As Owner of an active paid org, open Settings → Billing and click 'Manage Subscription' — confirm Stripe portal opens in a new browser tab (original tab stays on the app).",
        "In the Stripe portal, make a change (e.g. update the payment method or cancel), finish, and switch back to the original app tab WITHOUT manually refreshing.",
        "Wait a few seconds and confirm the Billing tab reflects the change (status/payment method) on its own.",
        "Repeat the new-tab check for 'Add Payment Method' (on a trialing org without a card) and for selecting a plan in the plan-selection modal (checkout)."
      ],
      "setup": "Have a Stripe-backed org (test mode). For Add Payment Method: a trialing org with has_payment_method = false. For checkout: an org eligible to start/checkout a paid plan.",
      "expected": "Every Stripe CTA opens in a new tab (popup not blocked under normal click). After completing the action in the new tab, the original tab converges to the new billing state within ~seconds via the poller. PostHog events (billing_portal_opened / add_payment_cta_clicked / plan_selected) are still recorded.",
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-cancel-reactivate-returns-to-trialing",
      "category": "Subscription state",
      "description": "A trialing user who cancels then reactivates returns to 'trialing', not 'active'.",
      "steps": [
        "As Owner of a trialing org (trial not yet ended), open Settings → Billing and click 'Cancel Subscription'; complete the cancel flow.",
        "Confirm the status shows pending cancellation / 'Downgrading' and a 'Reactivate Subscription' button appears.",
        "Click 'Reactivate Subscription'.",
        "Re-open the Billing tab and check the plan status."
      ],
      "setup": "Trialing Stripe org: organizations.plan = paid plan, plan_status = 'trialing', trial_end_date in the future, with a live Stripe subscription in trial. (Test-mode Stripe so the cancel/reactivate webhooks fire.)",
      "expected": "After reactivating, plan_status returns to 'trialing' (the trial resumes) — NOT 'active'. The user is not shown as converted/charged.",
      "status": null,
      "feedback": null
    },
    {
      "id": "generic-add-payment-copy",
      "category": "Billing CTA copy",
      "description": "Add-payment messaging no longer implies trial continuation.",
      "steps": [
        "As Owner of a trialing org without a payment method, open Settings → Billing.",
        "Read the trial card's add-payment subtitle and button label."
      ],
      "setup": "Trialing org, has_payment_method = false, trial_end_date in the future.",
      "expected": "The add-payment subtitle is generic (e.g. 'Add a payment method to keep your account active') and does not say 'to continue after the trial'. The button reads 'Add Payment Method'. After adding a card, only ONE 'Payment method added' toast appears.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "audit/banner-conditions-and-payment-prompt",
  "tests": [
    {
      "id": "no-payment-banner-has-card-hidden",
      "category": "No-Payment Banner",
      "description": "Banner is hidden once a payment method exists",
      "setup": "Set the org to a paid plan with plan_status='active' and has_payment_method=true.",
      "steps": [
        "Reload the app",
        "Observe the banner stack"
      ],
      "expected": "No no-payment-method banner is shown.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
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
      "id": "billingtab-cta-hidden-for-non-stripe",
      "category": "BillingTab CTA Fix",
      "description": "Non-Stripe plans no longer see Manage/Cancel Subscription CTAs in the Billing settings tab",
      "setup": "Set the org to a non-Stripe plan (Demo, Community, CommercialSelfHosted, or Enterprise) so plan_status is null.",
      "steps": [
        "Open Settings -> Billing",
        "Inspect the action buttons under the plan card"
      ],
      "expected": "Only the plan upgrade/change button is shown. No 'Manage Subscription' or 'Cancel Subscription' buttons appear.",
      "flow": "setup",
      "sequence": 8,
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
      "id": "digest-empty-suppressed",
      "category": "Discovery digest — empty session",
      "description": "When a discovery session produces zero deltas, no digest email is sent — regardless of recipient settings. Run via Docker discovery on the daemon's own host to keep the scan deterministic; raw port-scan results vary in ways unit tests don't.",
      "setup": "Pick the host the daemon is running on. Make sure it has a stable set of Docker containers (no containers starting/stopping in the background — `docker ps` should show the same set throughout the test). Trigger a Docker discovery against that host; wait for it to reach Complete and confirm the first digest arrives (this seeds the network with the current container set). Without touching any containers, immediately trigger a second Docker discovery on the same host.",
      "steps": [
        "Watch the Owner's inbox after the second Docker discovery completes."
      ],
      "expected": "Second discovery produces no digest email. The first one (seed) is expected to arrive.",
      "flow": "setup",
      "sequence": 1,
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
        "Wait for the toast and modal close",
        "Look at the Billing tab status pill and the action button"
      ],
      "expected": "Toast 'Subscription paused until {date}'. Modal closes. Org status pill flips to 'Paused' (orange) within ~2-4 seconds (auto-poll up to 20s). The hard-gate Settings modal auto-opens to Billing tab with the 'Resume now' button visible. If the Stripe API rejects the pause (e.g., subscription is not Active), the toast is now specific: 'Error pausing subscription: Stripe rejected the pause request: <stripe error>' — modal stays open so the user can retry or back out.",
      "status": null,
      "feedback": null
    },
    {
      "id": "pause-rejected-when-not-active",
      "category": "Cancel modal / Pause eligibility",
      "description": "Backend refuses pause when the Stripe subscription is not in Active status",
      "setup": "Pick an org whose Stripe subscription is in Trialing, PastDue, or already Paused state.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → continue to the save-offer screen",
        "Click 'Pause subscription'"
      ],
      "expected": "Error toast: 'Error pausing subscription: Subscription must be active to pause; current status: <trialing | past_due | paused>'. Modal stays open. Subscription on Stripe is unchanged (pause_collection not set).",
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
      "expected": "Toast 'Subscription resumed.' Status pill flips back to 'Active' (green) within a few seconds without a manual page reload. The hard-gate Settings modal becomes dismissible again; the 'Resume now' button disappears, replaced by the Manage Subscription + Cancel Subscription pair.",
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
      "expected": "The Pause panel renders 'You paused recently. You can pause again on {next-eligible-date}' (~5 months from now) instead of the 30/60/90 buttons. The Discount panel renders normally. Footer still has Back / Confirm Cancellation.",
      "status": null,
      "feedback": null
    },
    {
      "id": "discount-panel-visible-when-coupon-set",
      "category": "Discount save offer",
      "description": "Cancel modal renders the Discount panel when STRIPE_SAVE_OFFER_COUPON_ID is configured",
      "setup": "Set STRIPE_SAVE_OFFER_COUPON_ID to a coupon ID that exists IN THE SAME STRIPE MODE as the secret key — a test-mode key requires a test-mode coupon, a live-mode key requires a live-mode coupon. Mismatch produces a 400 from Stripe ('No such coupon … exists in <other> mode'). Restart the server after setting the env var. Pick an org with an active paid subscription.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → click 'Continue cancelling'"
      ],
      "expected": "Step 2 renders both the Pause and Discount panels. Clicking 'Apply discount' succeeds with a 'Discount applied to your subscription.' toast; modal closes.",
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
      "id": "stripe-pause-collection-set",
      "category": "Pause — Stripe-side verification",
      "description": "Pausing sets pause_collection on Stripe with the correct resumes_at",
      "setup": "After running cancel-modal-pause-redeem-flips-status, look up the subscription in the Stripe dashboard.",
      "steps": [
        "Open Stripe Dashboard → Customers → {test customer}",
        "Click the paused subscription"
      ],
      "expected": "Subscription has 'Pause Collection' set with behavior 'keep_as_draft' and resumes_at matching the duration the user picked (e.g., 60 days from when pause was clicked).",
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
        "Wait for the success toast",
        "Watch the BillingTab — do NOT manually reload the page",
        "Check the org owner's inbox after ~1 minute"
      ],
      "expected": "Success toast 'Subscription reactivated.'. Within ~2-4 seconds (auto-poll up to 20s), the status pill flips from 'Downgrading' back to 'Active' (green) on its own. The Reactivate Subscription button disappears, replaced by the Manage Subscription + Cancel Subscription pair. The 'plan will switch to Free' inline warning is gone. Stripe dashboard shows `cancel_at_period_end: false` on the subscription. Owner's inbox contains an email subject 'Your Scanopy subscription is active again' from the configured email provider (Brevo or SMTP).",
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
      "id": "pause-status-sidebar-and-alert-styling",
      "category": "Pause UI gate (past_due parity)",
      "description": "Paused state surfaces a sidebar notification dot and a red (Danger) inline alert on the BillingTab",
      "setup": "Same setup as the previous test — org in 'paused' state.",
      "steps": [
        "Look at the sidebar gear / settings icon",
        "Open the Billing tab and look at the inline alert above the action button"
      ],
      "expected": "Sidebar gear icon shows the billing-attention red dot (same indicator as past_due). The BillingTab inline alert is red InlineDanger (NOT blue InlineInfo) and reads: 'Your subscription is paused. Click Resume now to restart billing and unlock changes.'",
      "status": null,
      "feedback": null
    },
    {
      "id": "cancel-modal-confirm-card-layout",
      "category": "Cancel modal",
      "description": "On step 2 of the cancel modal, the confirm disclosure and Confirm Cancellation button live inside a card, matching the pause / discount card pattern",
      "setup": "Pick an org with active paid subscription. Use a fresh modal session.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick any reason → click 'Continue cancelling'",
        "Look at step 2"
      ],
      "expected": "Step 2 renders a 'Cancel my subscription' card (same card-static styling as the pause card) containing the disclosure ('If you confirm, you'll keep access until …') and the red 'Confirm cancellation' button (btn-danger w-full). The modal footer below the cards contains only the 'Back' button — no Confirm CTA in the footer.",
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
