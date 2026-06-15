var TEST_PLANS = [
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
    },
    {
      "id": "billing-trial-converted-email",
      "category": "Billing emails",
      "description": "When the trial converts to paid (Stripe `invoice.paid` for the first cycle), `TrialEnded { converted: true }` fires \u2192 trial-converted email.",
      "setup": "Continue from 'billing-trial-started-email'. With a payment method on file, advance the Stripe test clock past the trial end.",
      "steps": [
        "Advance the Stripe test clock past trial end",
        "Wait for `customer.subscription.updated` (status active) webhook",
        "Check inbox"
      ],
      "expected": "Trial-converted email arrives with plan name + base price.",
      "flow": "billing-trial-lifecycle",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "billing-cancellation-initiated-email",
      "category": "Billing emails",
      "description": "User clicks 'Cancel subscription' \u2192 `cancel_at_period_end: true` is set on Stripe \u2192 `CancellationInitiated` fires \u2192 email with period-end date.",
      "setup": "Test org on an active paid plan.",
      "steps": [
        "Settings \u2192 Billing \u2192 Cancel subscription (downgrade-to-Free flow)",
        "Confirm cancellation",
        "Check inbox"
      ],
      "expected": "Email subject contains the period-end date (e.g. 'Your Scanopy Subscription Will End on May 15, 2026'). Body confirms access continues until that date.",
      "flow": "billing-cancellation",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "billing-checkout-completed-email",
      "category": "Billing emails",
      "description": "First-time paid checkout (or upgrade from Free) emits `CheckoutCompleted` \u2192 welcome email with plan name.",
      "setup": "Free-plan test org. Pick a paid plan, complete Stripe checkout (test card 4242 4242 4242 4242).",
      "steps": [
        "Settings \u2192 Billing \u2192 Pick a paid plan \u2192 complete checkout",
        "Wait for webhook",
        "Check inbox"
      ],
      "expected": "Email subject 'Welcome to Scanopy <plan>' arrives with CTA to dashboard.",
      "status": null,
      "feedback": null
    },
    {
      "id": "billing-payment-failed-email",
      "category": "Billing emails",
      "description": "`invoice.payment_failed` webhook fires `PaymentFailed` event \u2192 email.",
      "setup": "Test org on a paid plan with payment method. Use Stripe test card `4000 0000 0000 0341` (always fails on subsequent charges) OR trigger via `stripe trigger invoice.payment_failed`.",
      "steps": [
        "Trigger payment failure",
        "Check inbox"
      ],
      "expected": "Payment-failed email arrives.",
      "flow": "billing-payment-recovery",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "billing-payment-action-required-email",
      "category": "Billing emails",
      "description": "`PaymentActionRequired` (3DS) \u2192 email asks user to authenticate.",
      "setup": "Use Stripe test card `4000 0027 6000 3184` (requires 3DS authentication) on a renewal.",
      "steps": [
        "Trigger renewal that requires 3DS",
        "Check inbox"
      ],
      "expected": "Payment-action-required email arrives with a link to complete auth.",
      "status": null,
      "feedback": null
    },
    {
      "id": "billing-payment-recovered-email",
      "category": "Billing emails",
      "description": "After a `PaymentFailed`, when the payment finally goes through, `PaymentRecovered` fires \u2192 email. Previously dropped silently \u2014 fix.",
      "setup": "Continue from 'billing-payment-failed-email'. Update the payment method to a working test card (`4242 4242 4242 4242`) and let Stripe retry, OR trigger `invoice.paid` for the past-due invoice.",
      "steps": [
        "Update payment method to a working card",
        "Wait for retry to succeed",
        "Check inbox"
      ],
      "expected": "Payment-recovered email arrives confirming the subscription is active again.",
      "flow": "billing-payment-recovery",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "billing-payment-method-removed-email",
      "category": "Billing emails",
      "description": "`PaymentMethodRemoved` \u2192 security-notice email. Previously dropped silently \u2014 fix.",
      "setup": "Continue from 'billing-payment-method-added-email'. Or any state where the org has a payment method.",
      "steps": [
        "Settings \u2192 Billing \u2192 Remove payment method",
        "Confirm",
        "Check inbox"
      ],
      "expected": "Email arrives noting the payment method was removed, with re-add CTA and 'if this wasn't you' messaging.",
      "flow": "billing-payment-method",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "billing-usage-summary-email-on-renewal",
      "category": "Billing emails",
      "description": "Recurring renewal (`invoice.paid` with `billing_reason = subscription_cycle`) fires `PaymentSucceeded { invoice: BillingInvoice }`; subscriber sends usage-summary email enumerating line items.",
      "setup": "Test org on a paid subscription. Advance Stripe test clock to the next billing period to trigger renewal.",
      "steps": [
        "Advance the test clock to the billing renewal date",
        "Wait for `invoice.paid` webhook",
        "Check inbox"
      ],
      "expected": "Usage summary email arrives. Body table shows line items (subscription base + any usage-based seats/networks/hosts), each with description and amount, plus total. The `billing_reason` filter must skip the *initial* invoice for the same subscription (only renewal cycles trigger this email).",
      "status": null,
      "feedback": null
    },
    {
      "id": "billing-no-usage-email-on-initial-invoice",
      "category": "Billing emails",
      "description": "The first invoice of a new subscription has `billing_reason = subscription_create`, NOT `subscription_cycle`. The subscriber's filter must skip \u2014 no usage-summary email.",
      "setup": "Fresh paid subscription without trial. Initial Stripe invoice fires.",
      "steps": [
        "Subscribe to a paid plan with payment method (no trial)",
        "Wait for `invoice.paid` webhook for the initial invoice",
        "Check inbox"
      ],
      "expected": "NO usage-summary email is sent (only the welcome / checkout-completed email).",
      "status": null,
      "feedback": null
    },
    {
      "id": "org-deleted-email-via-entity-event",
      "category": "Subscription side effects",
      "description": "Deleting an org now flows through `EntityOperation::Deleted { Entity::Organization }`; EmailService entity-event subscriber handles it and emails the initiating user. Previously a direct call from organizations/handlers.rs:348.",
      "setup": "Test org with you as owner. A second test org or fresh user account to log into after, since deletion logs you out.",
      "steps": [
        "Settings \u2192 Organization \u2192 Delete organization",
        "Confirm via the destructive-action modal",
        "Check the inbox of the user who initiated the delete"
      ],
      "expected": "Org-deleted confirmation email arrives at the initiator's address.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/phase2-topology-snapshots",
  "tests": [
    {
      "id": "snapshot-load-shows-captured-state",
      "category": "Snapshots",
      "description": "Loading a snapshot renders the captured topology graph (including entities deleted since)",
      "setup": "Take a snapshot, then delete one or more hosts so the live state diverges from the captured state.",
      "steps": [
        "Open the topology tab on the same network",
        "Pick the snapshot from the dropdown",
        "Confirm the topology canvas renders every captured node (including the deleted hosts) with their captured names/IPs",
        "Switch back to 'Live view'",
        "Confirm the canvas updates to show the new live state (deleted hosts gone)"
      ],
      "expected": "Snapshot view renders the captured entity set even after the live entities have been deleted from the network. Switching between Live view and a snapshot is instant and the inspector resolves entity names correctly for whichever view is active.",
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-disabled-on-free-plan",
      "category": "Snapshots",
      "description": "Free plan: Take snapshot button shows Upgrade badge and click fires paywall",
      "setup": "Use the API or admin tooling to set the org's plan to Free. Sign in as a Member of that org.",
      "steps": [
        "Open the topology tab",
        "Confirm the Take snapshot button is an icon-only Camera button with an 'Upgrade' badge",
        "Click the button"
      ],
      "expected": "Button is enabled and displays the 'Upgrade' badge (same style as gated formats in the export dropdown). Clicking triggers the upgrade modal/paywall (surface 'topology_tab', feature 'snapshots'). No POST request is fired.",
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-take-button-hidden-on-snapshot",
      "category": "Snapshots",
      "description": "Take snapshot button hidden when viewing a snapshot; visible (icon-only) on Live view",
      "setup": "Take at least one snapshot.",
      "steps": [
        "Open the topology tab on Live view; confirm the Camera button is visible (icon-only, with a tooltip on hover)",
        "Select a snapshot from the dropdown",
        "Confirm the Take snapshot button is no longer rendered"
      ],
      "expected": "The Camera button appears only on the live view. On a snapshot view, it's absent entirely.",
      "status": null,
      "feedback": null
    },
    {
      "id": "topology-export-still-works",
      "category": "Topology UI",
      "description": "Export to Mermaid/Confluence/CSV still works after the refactor",
      "setup": "Open the topology tab on a network with at least one host.",
      "steps": [
        "Click the Export menu",
        "Pick Mermaid; confirm download or copyable output",
        "Pick Confluence; confirm output",
        "Pick CSV; confirm download"
      ],
      "expected": "All three export paths work for both the live view and a selected snapshot.",
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-blocked-while-snapshot-running",
      "category": "Discovery / Snapshot coordination",
      "description": "Sessions started while a snapshot is running enter AwaitingSnapshot, then transition to Pending after release.",
      "setup": "Pick a network with a registered daemon. Trigger a long-ish discovery scan to confirm scans flow normally. Then take a snapshot from the topology tab; while the snapshot is running (close-and-clone takes some time on a busy network), trigger a discovery from the discovery tab.",
      "steps": [
        "Trigger a snapshot from the topology tab on a busy network",
        "Immediately switch to the discovery tab",
        "Trigger a manual discovery on the same network",
        "Watch the new discovery's status — it should show 'AwaitingSnapshot' (or the equivalent label)",
        "Wait for the snapshot to complete",
        "Confirm the discovery transitions to Pending and runs"
      ],
      "expected": "The discovery is blocked while the snapshot is in flight, then unblocks automatically when the snapshot finishes. No data loss; both operations complete.",
      "status": null,
      "feedback": null
    },
    {
      "id": "first-snapshot-onboarding-event",
      "category": "Onboarding",
      "description": "Taking the first snapshot on an org emits the FirstSnapshotCreated onboarding event",
      "setup": "Sign in as a Member of a Pro+ org that has never had a snapshot taken (verify via the organization's onboarding array — should not contain 'FirstSnapshotCreated' yet).",
      "steps": [
        "Open the topology tab",
        "Take a snapshot",
        "Refresh the page / inspect the organization onboarding array"
      ],
      "expected": "After the snapshot succeeds, the organization's onboarding array contains 'FirstSnapshotCreated'. Taking subsequent snapshots does not re-emit it.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/phase5-trial-ui",
  "tests": [
    {
      "id": "trial-pill-renders-at-t7",
      "category": "Trial urgency ramp",
      "description": "Sidebar shows the trial pill with clock icon and 'Trial: Nd left' copy when the org is trialing without payment and 7 or fewer days remain. Pill is visually aligned with sibling sidebar buttons (same height, padding, border, hover background).",
      "setup": "On the trialing org used for this run, set `organizations.trial_end_date = NOW() + INTERVAL '6 days'` and ensure `has_payment_method = false`. Confirm `plan_status = 'trialing'`.",
      "steps": [
        "Reload the app as the Owner.",
        "Look at the bottom of the left sidebar.",
        "Compare alignment with the sibling settings/support buttons (icon vertical centering, button height, padding, hover background)."
      ],
      "expected": "Button reads 'Trial: 6d left' with an amber clock icon. The button itself matches the standard sidebar nav style (transparent border, hover bg-gray-100); only the icon is amber. Indistinguishable in height and padding from the settings/support buttons next to it.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-pill-click-routes-to-add-payment",
      "category": "Trial urgency ramp",
      "description": "Clicking the sidebar trial pill drops the user into Stripe's add-payment-method flow, not the plan picker modal.",
      "setup": "Same as previous test (T-6d, no payment method).",
      "steps": [
        "Click the trial pill at the bottom of the sidebar.",
        "Observe where the browser navigates."
      ],
      "expected": "Browser redirects to a Stripe-hosted page titled 'Save your payment method' (or equivalent setup intent flow). NOT the BillingPlanModal in-app.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-card-not-dismissable",
      "category": "Trial card",
      "description": "The trial countdown InfoCard on BillingTab does NOT show a dismiss X — preventing the user from accidentally hiding their only path to add a payment method.",
      "setup": "Trialing org without payment method.",
      "steps": [
        "Open Settings → Billing.",
        "Locate the amber 'Trial ends in Nd' card at the top.",
        "Inspect the card for a close/X button."
      ],
      "expected": "No X icon, no dismiss button. The card cannot be hidden; the 'Add Payment Method' button is always present while trialing without payment.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "current-plan-card-has-change-plan-cta",
      "category": "Current plan",
      "description": "The Current Plan card has Change/Upgrade Plan as a primary CTA above Manage Subscription. The old standalone 'View Plans' InfoCard below it is gone.",
      "setup": "Org on any paid or trialing plan.",
      "steps": [
        "Open Settings → Billing.",
        "Scroll to the Current Plan card.",
        "Inspect the button stack at the bottom of the card.",
        "Scroll below the Current Plan card to confirm there is no longer a separate 'View Plans' InfoCard."
      ],
      "expected": "Inside the Current Plan card: a primary (blue) button reads 'Change Plan' (or 'Upgrade Plan' for free users) followed by a secondary 'Manage Subscription' button beneath it. No standalone 'View Plans' card exists below.",
      "flow": "setup",
      "sequence": 4,
      "status": null,
      "feedback": null
    },
    {
      "id": "welcome-banner-after-add-payment-during-trial",
      "category": "Post-Stripe welcome",
      "description": "After completing the Add Payment Method flow from any trial surface (card, banner, modal, sidebar pill), returning to the app shows a confirmation toast AND a 24h dismissible 'Payment method added' welcome banner.",
      "setup": "Trialing org without payment method. Have a working Stripe test card (4242 4242 4242 4242).",
      "steps": [
        "Click any 'Add Payment Method' CTA (trial card, T-3d banner, T-1d modal, or sidebar pill).",
        "Complete the Stripe-hosted payment-method form with a test card.",
        "Wait for the redirect back to the app."
      ],
      "expected": "On return: a green success toast reads 'Payment method added successfully.' AND a blue info banner appears at the top of every page reading 'Payment method added — your {plan} subscription will continue after the trial ends.' Banner is dismissible via X and persists across reloads for up to 24 hours.",
      "flow": "setup",
      "sequence": 5,
      "status": null,
      "feedback": null
    },
    {
      "id": "welcome-banner-after-subscription-activation",
      "category": "Post-Stripe welcome",
      "description": "Completing the Stripe Checkout (full subscription activation, not just payment-method setup) shows the 'Welcome to {plan}' banner with the original activation copy.",
      "setup": "Org on Free plan (or trialing). Initiate a full Stripe Checkout via the Change Plan flow with a test card.",
      "steps": [
        "From Settings → Billing, click 'Change Plan' / 'Upgrade Plan'.",
        "Pick a paid plan in the modal and proceed to Stripe Checkout.",
        "Complete checkout with a test card.",
        "Wait for the redirect back to the app."
      ],
      "expected": "Blue info banner reads 'Welcome to {plan name} — your subscription is now active.' Distinct copy from the trial-secured case. Dismissible via X.",
      "flow": "setup",
      "sequence": 6,
      "status": null,
      "feedback": null
    },
    {
      "id": "welcome-banner-dismiss",
      "category": "Post-Stripe welcome",
      "description": "Dismissing the welcome banner hides it permanently until localStorage is cleared.",
      "setup": "Continuing from either welcome-banner test — banner is visible.",
      "steps": [
        "Click the X on the banner.",
        "Reload the page.",
        "Open DevTools → Application → Local Storage and inspect `appbanner_dismissed:welcome_banner`."
      ],
      "expected": "Banner disappears immediately. Stays gone after reload. LocalStorage entry shows `appbanner_dismissed:welcome_banner = 'true'`.",
      "flow": "setup",
      "sequence": 7,
      "status": null,
      "feedback": null
    },
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
      "sequence": 8,
      "status": null,
      "feedback": null
    },
    {
      "id": "add-payment-cta-clicked-event-fires-from-all-surfaces",
      "category": "Telemetry",
      "description": "A single `add_payment_cta_clicked` PostHog event fires from every Add-Payment-Method surface (trial card, T-3d banner, T-1d modal, sidebar pill), with a `source` property identifying which surface.",
      "setup": "Trialing org without payment method. Open the browser DevTools network tab (filter for posthog) or check PostHog Live events.",
      "steps": [
        "Click 'Add Payment Method' on the trial countdown card in Settings → Billing. Note the event in PostHog.",
        "Cancel the Stripe redirect, come back to the app.",
        "Set trial_end_date to ~T-2d and reload. Click the CTA on the T-3d banner. Note the event.",
        "Set trial_end_date to ~T-12h, clear `dismissed_today:trial_expiry_modal`, reload. Click the CTA on the T-1d modal. Note the event.",
        "Set trial_end_date back to ~T-6d, reload. Click the sidebar trial pill. Note the event."
      ],
      "expected": "Four events fired, all named `add_payment_cta_clicked`, with `source` values `trial_card`, `trial_banner`, `trial_modal`, and `sidebar_trial_pill` respectively. No `trial_card_cta_clicked` / `trial_banner_cta_clicked` / `trial_modal_cta_clicked` events fire.",
      "flow": "setup",
      "sequence": 9,
      "status": null,
      "feedback": null
    },
    {
      "id": "no-trial-recap-card",
      "category": "Regression",
      "description": "The 'Your trial so far' recap card on BillingTab has been removed and does not render.",
      "setup": "Trialing org.",
      "steps": [
        "Open Settings → Billing.",
        "Look between the trial countdown card and the Current Plan card."
      ],
      "expected": "No 'Your trial so far' card. The trial countdown card sits directly above the Current Plan card.",
      "flow": "setup",
      "sequence": 10,
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
      "id": "checkout-completed-mrr-and-trialing-flag",
      "category": "Billing telemetry",
      "description": "Confirm CheckoutCompleted carries mrr_amount_cents and the new is_trialing flag, distinguishing trial starts from paid checkouts.",
      "setup": "Have an org on Free (or freshly created with no plan). Be ready to enter Stripe test card 4242 4242 4242 4242.",
      "steps": [
        "Sign in as the org owner.",
        "Open Settings -> Billing -> select a paid plan with trial enabled (e.g., Pro Monthly).",
        "Complete checkout WITHOUT adding a card (trial path).",
        "Wait ~10s and open PostHog, find the most recent checkout_completed.",
        "Then upgrade or have another org go through checkout WITH a card and verify the non-trialing case."
      ],
      "expected": "Trial start: checkout_completed has is_trialing=true and mrr_amount_cents = the plan's monthly base+addons (yearly /12). Paid checkout: is_trialing=false, same mrr math. Free direct-activation (no Stripe): is_trialing=false, mrr=0. PostHog analysts can now filter `where is_trialing = false` to isolate real paid conversions.",
      "status": null,
      "feedback": null
    },
    {
      "id": "payment-failed-recovered-enrichment",
      "category": "Billing telemetry",
      "description": "Confirm PaymentFailed and PaymentRecovered events carry plan + attempt_count + invoice_id.",
      "setup": "Have an org on a paid plan with an invoice in past_due state. Easiest setup: in Stripe test mode, switch the org's payment method to the failing card 4000 0000 0000 0341, wait for the next invoice, then switch back to a working card and let Stripe retry.",
      "steps": [
        "Wait for the failed payment webhook.",
        "Open PostHog and find the most recent payment_failed event for this org.",
        "After Stripe successfully retries (or you manually pay the invoice), wait for invoice.paid webhook.",
        "Find the most recent payment_recovered event for the same org."
      ],
      "expected": "payment_failed event has plan matching the org's current plan, attempt_count matching the Stripe invoice's attempt_count (>= 1), invoice_id matching the Stripe invoice id. payment_recovered event has all four fields populated (invoice_id, amount_cents, plan, attempt_count).",
      "status": null,
      "feedback": null
    },
    {
      "id": "stripe-portal-cancel-with-reason",
      "category": "Billing — Cancellation Telemetry",
      "description": "End-to-end: a Stripe Portal cancellation that selects a reason + writes a comment must surface in the cancellation_initiated PostHog event with the three new keys populated. (Portal cancels default to cancel_at_period_end=true and emit cancellation_initiated, not subscription_cancelled — the latter only fires when the period actually ends.)",
      "setup": "Pick (or create) a test organization on a paid plan with an active Stripe subscription. Open that org in the app, navigate to Settings → Billing → Manage Subscription to launch the Stripe Customer Portal. Confirm the Portal's cancel flow has 'Cancellation reason' and free-text 'Additional feedback' enabled in the Stripe dashboard's Billing → Customer Portal configuration (Scanopy already has this configured in production-like envs, but the test env may differ — verify before running).",
      "steps": [
        "From the app, click Manage Subscription to launch the Stripe Customer Portal.",
        "In the Portal, click Cancel subscription.",
        "Select a reason from the dropdown — pick 'Too expensive'.",
        "In the optional comment field, type 'Testing cancel telemetry — please ignore'.",
        "Confirm the cancellation in the Portal.",
        "Wait ~30 seconds for the customer.subscription.updated webhook to land and the async side-effects task to publish the event.",
        "Open PostHog → Activity → filter by event name 'cancellation_initiated' and the test org's distinct_id (or org_id).",
        "Open the most recent event and inspect the metadata properties."
      ],
      "expected": "The cancellation_initiated event's metadata object contains: stripe_reason = 'cancellation_requested', stripe_feedback = 'too_expensive', comment = 'Testing cancel telemetry — please ignore', reason_code = null. planned_period_end is set to the actual subscription period end.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "stripe-portal-cancel-no-reason",
      "category": "Billing — Cancellation Telemetry",
      "description": "End-to-end: a Stripe Portal cancellation with no reason selected (or the Portal config doesn't ask) emits cancellation_initiated with stripe_feedback/comment/reason_code as null and stripe_reason populated by Stripe (typically cancellation_requested).",
      "setup": "Pick (or create) a different test organization on a paid plan with an active Stripe subscription, distinct from the one used in the prior test. If the Portal cancel flow forces a reason selection, temporarily disable that requirement in the Stripe dashboard's Customer Portal configuration for the duration of this test, then restore it after.",
      "steps": [
        "From the app, click Manage Subscription to launch the Stripe Customer Portal.",
        "In the Portal, click Cancel subscription.",
        "Skip the reason dropdown if possible (leave it blank or do not select an option).",
        "Leave the comment field empty.",
        "Confirm the cancellation in the Portal.",
        "Wait ~30 seconds for the webhook + async task.",
        "Open PostHog → Activity → filter by event name 'cancellation_initiated' and the test org's distinct_id.",
        "Open the most recent event and inspect the metadata properties."
      ],
      "expected": "The cancellation_initiated event's metadata contains stripe_feedback = null, comment = null, reason_code = null. stripe_reason will typically be 'cancellation_requested' (Stripe sets this for any Portal cancel, even without an explicit user pick). Pre-existing keys unchanged.",
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
      "id": "digest-respects-per-user-toggle",
      "category": "Discovery digest — opt-out",
      "description": "Opening the Email tab no longer hangs the browser. A user who turns off the discovery digest stops receiving emails immediately; other users in the same org keep receiving them.",
      "setup": "Reuse any org/network. Make sure the user under test has email_settings.discovery_digest = true.",
      "steps": [
        "Sign in as the test user, open Settings → Email tab.",
        "Verify the tab renders without the browser freezing or any infinite-loop warning in the console.",
        "Verify the 'Discovery scan summary' checkbox is on by default.",
        "Uncheck the box and click Save.",
        "Trigger another Unified discovery on the network and wait for Complete.",
        "Check the test user's inbox — no digest should arrive.",
        "Check the Owner's inbox — should still receive the digest."
      ],
      "expected": "Email tab opens cleanly. Save succeeds with a success toast. No digest arrives at the opted-out user; other recipients still receive it.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-empty-suppressed",
      "category": "Discovery digest — empty session",
      "description": "When a discovery session produces zero changes, no digest is sent at all (regardless of recipient settings).",
      "setup": "Reuse the org and network. Run a discovery that re-scans the same hosts with no new entities and no removed children — i.e. all hosts and children are reported again unchanged.",
      "steps": [
        "Watch every recipient's inbox after the scan completes."
      ],
      "expected": "No digest email arrives at any recipient.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-no-email-on-failed-or-cancelled",
      "category": "Discovery digest — terminal phase gating",
      "description": "Failed and Cancelled discovery sessions do not produce a digest email (v1 only sends on Complete).",
      "setup": "Reuse the org. Force a discovery to fail (e.g. shut the daemon down mid-scan or supply bad credentials so the session transitions to Failed). Separately, start a discovery and cancel it from the UI.",
      "steps": [
        "Watch every recipient's inbox after the Failed and Cancelled sessions reach terminal."
      ],
      "expected": "No digest email arrives for either terminal state.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "email-tab-tenant-isolation",
      "category": "Email tab — tenant isolation",
      "description": "A user cannot use the self-update endpoint to write another user's email_settings (the existing PUT /users/{id} guard rejects cross-user writes).",
      "setup": "Two users in the same org. Note both user IDs.",
      "steps": [
        "Sign in as User A. Open the browser dev tools network panel.",
        "On the Email tab, click Save once with the box on.",
        "In the network panel, find the PUT /api/v1/users/{A.id} request.",
        "Replay the request with the body's id field changed to User B's UUID and the URL path changed to {B.id} (use 'Copy as fetch' or curl from the dev tools)."
      ],
      "expected": "The replayed request returns 401/403 'You can only update your own user record'. User B's email_settings remain unchanged.",
      "flow": "setup",
      "sequence": 4,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-host-only-changes-when-children-actually-change",
      "category": "Discovery digest — change-detection accuracy",
      "description": "Rescanning a host whose ports/services/IPs/interfaces are unchanged does NOT classify the host as Changed in the digest. Previously, every pre-existing child was wrongly flagged as Removed because the foundation-worker reconciliation path doesn't refresh `last_seen_at` on natural-key match — now fixed by using the daemon's ScannedEntityIds for the removed-child signal.",
      "setup": "Pick a host the daemon will rediscover with the same children it already has — same ports open, same services, same IPs, same interfaces.",
      "steps": [
        "Trigger a Unified discovery and wait for Complete.",
        "Open the digest email in the Owner's inbox."
      ],
      "expected": "If NO real children changed, the host should not appear in 'Hosts with changes'. The whole digest may be empty (no email at all) if no other hosts had real changes either. If a real delta exists on a different host, only that host shows in 'Hosts with changes' — not every scanned host.",
      "flow": "setup",
      "sequence": 5,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-summary-banner",
      "category": "Discovery digest — top summary",
      "description": "The digest opens with a stats banner showing per-bucket counts, including a non-zero subnets-scanned count when the daemon swept subnets.",
      "setup": "Run any Unified discovery that walks at least one subnet AND produces a mix of changes (≥1 new host, ≥1 vanished host, ≥1 changed host, ≥1 new VLAN).",
      "steps": [
        "Open the digest email in the Owner's inbox.",
        "Scroll to the top, just below the 'Network: X' header."
      ],
      "expected": "A single banner row with labelled count cells: new hosts, vanished hosts, changed hosts, VLANs detected, VLANs no longer detected, subnets scanned. The subnets-scanned count is greater than 0 and matches the subnets the daemon actually walked. Counts match the per-section counts further down.",
      "flow": "setup",
      "sequence": 6,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-host-card-shape",
      "category": "Discovery digest — host cards",
      "description": "Each affected host renders as a card mirroring HostCard.svelte from the UI, with child entities shown as inline colored tags rather than bulleted lists, and no UUIDs visible anywhere.",
      "setup": "Seed a host with multiple services, IPs, interfaces, and ports — at least 3 of each. Trigger a discovery that adds/removes children so the host is classified as Changed.",
      "steps": [
        "Open the digest email in the Owner's inbox.",
        "Find the Hosts-with-changes section and the card for the seeded host.",
        "Verify the card header shows the hostname plus a 'Changed' badge.",
        "Inspect the Services / IP Addresses / Interfaces / Ports rows.",
        "Inspect the 'What changed this scan' block."
      ],
      "expected": "Each child entity is a colored inline pill that wraps to the next line when the row fills. Colors match the in-app HostCard (Services=fuchsia, IPs=emerald, Interfaces=teal, Ports=sky, Subnets=indigo, VLANs=violet). No UUIDs visible anywhere — port labels say e.g. '22/tcp' or 'Ssh', not 'Ssh (ID: ...)'. Bindings are NOT shown.",
      "flow": "setup",
      "sequence": 7,
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
  "tests": [
    {
      "id": "snapshot-view-shows-as-of-entity-state",
      "category": "Snapshot As-Of Reads",
      "description": "On a snapshot view, inspector entity cards show entity state as of the snapshot's taken_at, not current live state.",
      "steps": [
        "Take a snapshot of the network (topology view → Take snapshot).",
        "On the live view, open a host in the inspector and add a new service to it (or add/rename via the UI), then save.",
        "In the topology view's snapshot dropdown, select the snapshot you just took.",
        "Open the same host's inspector card in the snapshot view.",
        "Confirm the newly-added service is NOT shown (snapshot state predates it).",
        "Switch the dropdown back to the live view and open the host again."
      ],
      "setup": "Network has at least one host with services. Snapshots must be enabled on the org's plan (snapshot_retention_days > 0).",
      "expected": "Snapshot view inspector shows the host's services/IPs as captured at snapshot time (without the post-snapshot addition). Live view shows current state including the new service.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "live-view-unchanged-after-fix",
      "category": "Snapshot As-Of Reads",
      "description": "Live view (no snapshot selected) shows current entity state exactly as before — regression check.",
      "steps": [
        "With no snapshot selected (live view), open the Hosts/Services/Subnets tabs.",
        "Confirm all current entities and their children render normally.",
        "Open several inspector cards and confirm services, IP addresses, ports, bindings, and tags all display correctly."
      ],
      "expected": "Live view behaves identically to before the change: current entities and children render with no missing or duplicated data.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/phase5-subscription-mechanics",
  "tests": [
    {
      "id": "cancel-modal-pause-redeem-flips-status",
      "category": "Cancel modal / Pause",
      "description": "Redeeming the pause save-offer pauses the subscription and surfaces the Resume button",
      "setup": "Pick an org with an active paid subscription (last_paused_at null so no cooldown applies).",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → click 'Continue cancelling' to reach the save-offer screen",
        "Click '60 days' in the Pause panel and verify the 'Pause until {date}' preview updates",
        "Click 'Pause subscription'",
        "Wait for the toast and modal close",
        "Look at the Billing tab status pill and the action button"
      ],
      "expected": "Toast 'Subscription paused until {date}'. Modal closes. Org status pill flips to 'Paused' (orange). The 'Resume now' button appears in place of the Manage / Cancel buttons. The blue inline alert reads 'Your subscription is paused. Resume any time...'",
      "flow": "cancel-too-expensive",
      "sequence": 1,
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
      "expected": "Toast 'Subscription resumed.' Status pill flips back to 'Active' (green). The 'Resume now' button disappears, replaced by the Manage Subscription + Cancel Subscription pair.",
      "flow": "cancel-too-expensive",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "cancel-modal-back-button",
      "category": "Cancel modal",
      "description": "Back button on step 2 returns to step 1 without losing form state",
      "setup": "Pick an org with an active paid subscription. Use a fresh modal session.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive', type 'too pricey for our team' in the comment, click 'Continue cancelling'",
        "On step 2 click 'Back'"
      ],
      "expected": "Step 1 is shown with 'Too expensive' still selected and the comment text intact. Continue cancelling again returns to the save-offer screen.",
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
      "id": "discount-unavailable-graceful-error",
      "category": "Discount save offer",
      "description": "When STRIPE_SAVE_OFFER_COUPON_ID is unset, applying the discount surfaces a clear error toast",
      "setup": "Ensure the deployment does NOT have STRIPE_SAVE_OFFER_COUPON_ID set in env. Pick an org with an active paid subscription.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Pick 'Too expensive' → continue to step 2",
        "Click 'Apply discount'"
      ],
      "expected": "Error toast 'Error applying discount: Discount save offer is not configured. Please try again.' Modal stays open on the save-offer screen; user can still pick Pause or click Confirm cancellation.",
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
    }
  ]
}
,
{
  "branch": "feat/phase5-quick-wins",
  "tests": [
    {
      "id": "payment-method-added-toast",
      "category": "Feature 1 — Payment method added",
      "description": "After adding a payment method via Stripe setup, returning to the app shows a success toast.",
      "setup": "Use a trialing org without a payment method. From Settings → Billing, click 'Add Payment Method' (or whichever surface initiates payment_setup flow) to be redirected to Stripe Checkout in setup mode. Complete Stripe with a test card (4242 4242 4242 4242).",
      "steps": [
        "After completing Stripe setup, observe the redirect back to the Scanopy app",
        "Look for a green success toast",
        "Verify the URL no longer contains the billing_flow query param"
      ],
      "expected": "A toast reading 'Payment method added.' appears briefly. The org's has_payment_method becomes true (visible in Settings → Billing — the trial card no longer prompts to add a payment method). No error toast.",
      "status": null,
      "feedback": null
    },
    {
      "id": "subscription-cancelled-email-period-end",
      "category": "Feature 2 — period_end in post-cancel email",
      "description": "The post-cancellation email body contains the formatted period end date.",
      "setup": "Have a paid org with an active subscription. Trigger a subscription cancellation via Stripe (either via the customer portal cancel-now flow, or by waiting for a cancel_at_period_end subscription to actually end). The Stripe webhook delivers customer.subscription.deleted to Scanopy.",
      "steps": [
        "Check the org owner's inbox for the 'Your Scanopy Subscription Has Been Cancelled' email",
        "Verify the body reads 'Your Scanopy subscription was cancelled and your access ended on {Month Day, Year}. Your account has been moved to the Free plan.'",
        "Verify the date in the body matches the subscription's actual period end / cancellation timestamp"
      ],
      "expected": "Email body contains a properly formatted human date (e.g., 'May 1, 2026'); no '{period_end_date}' literal placeholder visible.",
      "status": null,
      "feedback": null
    },
    {
      "id": "payment-recovered-email-amount",
      "category": "Feature 3 — payment_recovered amount in email",
      "description": "When a previously-failed payment recovers, the email body names the recovered amount.",
      "setup": "Have an org whose plan_status was past_due (typically because a previous invoice payment failed). Update the payment method in Stripe and trigger a retry, OR: in Stripe dashboard, manually mark the open past-due invoice as paid. The Stripe webhook delivers invoice.paid to Scanopy.",
      "steps": [
        "Check the org owner's inbox for the 'Your Scanopy Payment is Back On Track' email",
        "Verify the body reads 'Your previously failed payment of $XX.XX has gone through. Your subscription is active again — no action needed on your end.'",
        "Verify the dollar amount matches the recovered invoice's amount_paid (in dollars, formatted with cents)"
      ],
      "expected": "Email body contains the exact recovered amount as currency (e.g., '$29.99'); no '{amount}' literal placeholder visible.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "feat/license-grace-period",
  "tests": []
}
];
