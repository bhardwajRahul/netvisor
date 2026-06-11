var TEST_PLANS = [
{
  "branch": "chore/license-key-make-target",
  "tests": []
}
,
{
  "branch": "feat/event-model-typed-payloads",
  "notes": "Covers all email-driven flows and event-bus side effects across the auth/billing/event-subscriber refactors. All emails now route through the event bus via `Subscriber<Op>` impls registered via `inventory::submit!`; cancellations cascade via `InviteService::Subscriber<BillingOperation>` instead of a direct call. Tests are grouped into flows where state can be reused; truly independent tests omit `flow`/`sequence`. Programmatic checks (DB row updates for Pattern B flag columns, subscriber-name uniqueness at startup) are verified via `cargo test --lib` (300/300 green) and not included here.",
  "tests": [
    {
      "id": "auth-resend-verification",
      "category": "Auth emails",
      "description": "The /resend-verification endpoint emits a fresh `EmailVerificationRequested` event; the EmailService subscriber sends a new verification email.",
      "setup": "Same flow as 'auth-register-new-org-verification-email' \u2014 register a new user but do NOT click the verification link yet.",
      "steps": [
        "While logged in but unverified, click 'Resend verification email' (in the unverified-banner UI)",
        "Wait the rate-limit cooldown (~60 seconds)",
        "Check the inbox"
      ],
      "expected": "A second verification email arrives with a different token. Both tokens work until the second is generated; the original token may be invalidated depending on storage logic.",
      "flow": "auth-password-register",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "auth-invited-user-no-verification-email",
      "category": "Auth emails",
      "description": "Registering via an invite link to an existing org auto-verifies the email \u2014 no verification email is sent (`Register { email_and_token: None }`).",
      "setup": "On an existing test org, send an invite to a fresh email. Open the invite link in incognito.",
      "steps": [
        "Click the invite link",
        "Complete the registration form (password)",
        "Check inbox for the invited email"
      ],
      "expected": "User lands logged in with `email_verified = true`. No verification email is in the inbox (only the original invite email).",
      "status": null,
      "feedback": null
    },
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
      "id": "billing-cancellation-revokes-org-invites",
      "category": "Subscription side effects",
      "description": "`SubscriptionCancelled` triggers `InviteService::Subscriber<BillingOperation>` which calls `revoke_org_invites`. Previously this was a direct method call from BillingService \u2014 now event-driven.",
      "setup": "Test org with an active subscription AND at least one outstanding invite (Settings \u2192 Members \u2192 Invite, send to a fresh email but don't accept). Then cancel the subscription and let the period end.",
      "steps": [
        "Cancel the subscription, advance clock past period_end",
        "Settings \u2192 Members \u2192 Pending invites"
      ],
      "expected": "All previously-sent invites are gone from the pending invites list.",
      "flow": "billing-cancellation",
      "sequence": 3,
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
      "id": "billing-payment-method-added-email",
      "category": "Billing emails",
      "description": "`PaymentMethodAdded` \u2192 confirmation email. Previously had no trait method on `EmailProvider` \u2014 fix.",
      "setup": "Test org without a payment method on file (or remove the existing one first).",
      "steps": [
        "Settings \u2192 Billing \u2192 Add payment method",
        "Enter test card 4242 4242 4242 4242",
        "Confirm",
        "Check inbox"
      ],
      "expected": "Payment-method-added email arrives.",
      "flow": "billing-payment-method",
      "sequence": 1,
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
    },
    {
      "id": "onboarding-milestones-persist-via-event",
      "category": "Subscription side effects",
      "description": "`OnboardingOperation::*` events flow through `OrganizationService::Subscriber<OnboardingOperation>` and append the discriminant to `organizations.onboarding`. The UI checklist reads from this column and shows ticks.",
      "setup": "Fresh test org with empty onboarding state.",
      "steps": [
        "Complete the onboarding wizard (sets `OnboardingModalCompleted`)",
        "Install a daemon (sets `FirstDaemonRegistered`)",
        "Run a discovery (sets `FirstDiscoveryCompleted`)",
        "Open the homepage / onboarding checklist UI"
      ],
      "expected": "Each completed milestone shows as ticked in the checklist UI. Reloading the page preserves the ticks (state is persisted via the subscriber).",
      "status": null,
      "feedback": null
    },
    {
      "id": "topology-staleness-on-entity-changes",
      "category": "Subscription side effects",
      "description": "Entity events for Host / Subnet / Service / etc. flow through `TopologyService::Subscriber<EntityOperation>`; the topology snapshot is marked stale and re-rendered.",
      "setup": "Test org with at least one network, one daemon, and one rendered topology view.",
      "steps": [
        "Open the topology page",
        "In another tab, create a host (e.g. via discovery or manual)",
        "Return to the topology tab"
      ],
      "expected": "The topology view picks up the new host without manual refresh, OR shows a 'stale' indicator that updates on the next render.",
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
      "id": "snapshot-take-from-live-view",
      "category": "Snapshots",
      "description": "Take a snapshot from the topology tab on the live view",
      "setup": "Sign in as a Member of an org on Pro or higher plan. Pick a network with at least one host, one service, and one subnet (run a discovery scan if the network is empty so there's something to capture).",
      "steps": [
        "Open the topology tab",
        "Confirm the snapshot dropdown shows 'Live view' selected by default",
        "Click 'Take snapshot'",
        "Wait for the toast"
      ],
      "expected": "Toast confirms the snapshot saved. The snapshot dropdown now lists the new snapshot at the top with a formatted timestamp; the snapshot is available to load.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-load-shows-captured-state",
      "category": "Snapshots",
      "description": "Loading a snapshot renders the captured topology graph",
      "setup": "After test snapshot-take-from-live-view completes, run a discovery scan or manually delete a host on the same network so the live state diverges from the captured state.",
      "steps": [
        "Open the topology tab on the same network",
        "Pick the snapshot from the dropdown",
        "Confirm the topology canvas renders the captured set of nodes/edges",
        "Switch back to 'Live view'",
        "Confirm the canvas updates to show the new live state"
      ],
      "expected": "The snapshot view shows the topology as it was when captured (matching the pre-divergence state). Live view shows the post-divergence state. Switching is instant.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-delete-removes-from-list",
      "category": "Snapshots",
      "description": "Deleting a snapshot removes it from the dropdown and reverts the view",
      "setup": "After test snapshot-take-from-live-view completes.",
      "steps": [
        "Open the topology tab on the same network",
        "Select the snapshot from the dropdown",
        "Click 'Delete snapshot'",
        "Confirm in the prompt"
      ],
      "expected": "The snapshot disappears from the dropdown. The selected view reverts to 'Live view'. No errors. Refreshing the page does not bring the snapshot back.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-disabled-on-free-plan",
      "category": "Snapshots",
      "description": "Free plan disables 'Take snapshot' with upgrade hook",
      "setup": "Use the API or admin tooling to set the org's plan to Free. Sign in as a Member of that org.",
      "steps": [
        "Open the topology tab",
        "Hover over 'Take snapshot' to see the disabled tooltip",
        "Click 'Take snapshot'"
      ],
      "expected": "Button is visibly disabled and shows the upgrade messaging. Click triggers the upgrade modal/paywall surface 'topology_tab' with feature 'snapshots'. No POST request is fired.",
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-creation-blocks-during-discovery",
      "category": "Snapshots",
      "description": "Take snapshot returns 409 if a discovery is in flight",
      "setup": "Pick a network with a daemon. Start a discovery scan (manually trigger it from the discovery tab or wait for a scheduled scan). While the scan is in flight, switch to the topology tab.",
      "steps": [
        "Click 'Take snapshot' on the topology tab",
        "Read the error toast"
      ],
      "expected": "The request fails with a 409 conflict and a toast saying the network is busy. Retrying after the discovery completes succeeds.",
      "status": null,
      "feedback": null
    },
    {
      "id": "topology-live-update-after-discovery",
      "category": "Topology UI",
      "description": "Live view auto-refreshes when discovery changes the network",
      "setup": "Open the topology tab on the live view of a network with a daemon. Trigger a discovery scan from another tab.",
      "steps": [
        "Watch the topology canvas while discovery runs",
        "Look for new hosts/services appearing without a manual reload"
      ],
      "expected": "When discovery commits new entities, the topology canvas updates within a few seconds (via the live_topology_updates_stream SSE). No banner or refresh prompt — it just reflects the new state.",
      "status": null,
      "feedback": null
    },
    {
      "id": "plan-usage-shows-retention-window",
      "category": "Billing UI",
      "description": "Dashboard PlanUsage panel shows the snapshot retention window",
      "setup": "Switch the org plan to Pro, Business, or higher.",
      "steps": [
        "Open the home/dashboard tab",
        "Find the PlanUsage section",
        "Confirm there is a row showing the retention window in days"
      ],
      "expected": "The PlanUsage card surfaces the snapshot_retention_days value (e.g. '30 days' for Pro, '90 days' for Business). On Free, it should either hide the row or show '0' / 'Not included'.",
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
      "id": "topology-share-still-works",
      "category": "Topology UI",
      "description": "Share view still renders correctly after the refactor",
      "setup": "Take a snapshot, then open the share modal from the topology tab.",
      "steps": [
        "Create or copy a share link from the topology tab",
        "Open the share link in an incognito window",
        "Confirm the topology graph renders"
      ],
      "expected": "The shared view shows the same nodes/edges as the originating topology view (live or snapshot). No errors related to missing entity-blob fields.",
      "status": null,
      "feedback": null
    },
    {
      "id": "snapshot-cascade-on-delete",
      "category": "Snapshots",
      "description": "Deleting a snapshot reaps closed entity rows + the snapshot's topology row",
      "setup": "Take a snapshot. Then via psql or admin tooling, count the rows in (a) snapshots WHERE id = '<snap_id>'; (b) topologies WHERE snapshot_id = '<snap_id>'; (c) hosts WHERE snapshot_id = '<snap_id>'. Delete the snapshot via the UI.",
      "steps": [
        "Re-run the same SQL counts",
        "Confirm all three counts are zero"
      ],
      "expected": "The snapshot row, its topology row, and every closed entity row that carried snapshot_id pointing to it are gone. Live rows (snapshot_id IS NULL) are untouched.",
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
      "id": "snapshot-disabled-button-tooltip",
      "category": "Snapshots",
      "description": "Take snapshot button shows a useful disabled tooltip on free plan",
      "setup": "Same as snapshot-disabled-on-free-plan.",
      "steps": [
        "Open the topology tab",
        "Hover over the Take snapshot button"
      ],
      "expected": "Tooltip explains snapshots aren't included on the current plan and points to upgrading.",
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
      "description": "Sidebar shows the amber trial pill with clock icon and 'Trial: Nd left' copy when the org is trialing without payment and 7 or fewer days remain.",
      "setup": "On the trialing org used for this run, set `organizations.trial_end_date = NOW() + INTERVAL '6 days'` and ensure `has_payment_method = false`. Confirm `plan_status = 'trialing'`.",
      "steps": [
        "Reload the app as the Owner.",
        "Look at the bottom of the left sidebar."
      ],
      "expected": "An amber pill with a clock icon and copy 'Trial: 6d left' renders. The standard 'Upgrade' button does NOT also show. Clicking the pill opens the BillingPlanModal.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-pill-not-shown-at-t8",
      "category": "Trial urgency ramp",
      "description": "Sidebar does NOT show the trial pill when more than 7 days remain.",
      "setup": "Set `organizations.trial_end_date = NOW() + INTERVAL '8 days'` for the same org.",
      "steps": [
        "Reload the app.",
        "Look at the bottom of the left sidebar."
      ],
      "expected": "No trial pill visible (free-plan upgrade button also absent — org is on a paid trial).",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-banner-renders-at-t3",
      "category": "Trial urgency ramp",
      "description": "Global TrialEndingBanner appears at the top of every page when 3 or fewer trial days remain and no payment method is set.",
      "setup": "Set `organizations.trial_end_date = NOW() + INTERVAL '2 days'`, `has_payment_method = false`.",
      "steps": [
        "Reload the app.",
        "Inspect the top of the page across at least two tabs (e.g. Topology and Settings)."
      ],
      "expected": "A yellow warning banner reads 'Your trial ends in 2 days. Add a payment method to keep your data and avoid interruption.' with an 'Add Payment Method' link/button. Banner persists across tabs. Clicking the CTA redirects to Stripe checkout.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-modal-renders-at-t1-and-stacks",
      "category": "Trial urgency ramp",
      "description": "TrialExpiryModal appears at T-1d. The T-3d banner stays visible behind it (stack semantics).",
      "setup": "Set `organizations.trial_end_date = NOW() + INTERVAL '12 hours'`. Clear `localStorage.dismissed_today:trial_expiry_modal` if present (DevTools → Application → Local Storage).",
      "steps": [
        "Reload the app.",
        "Observe the modal that appears.",
        "Look behind/under the modal at the top of the page."
      ],
      "expected": "Modal titled 'Your trial ends tomorrow' with body text, a 'Remind me later' button and an 'Add Payment Method' button. Behind the modal, the yellow T-3d banner is still rendered. Sidebar pill is also visible.",
      "flow": "setup",
      "sequence": 4,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-modal-dismiss-once-per-day",
      "category": "Trial urgency ramp",
      "description": "Dismissing the T-1d modal hides it for the rest of the day; reload should not bring it back until tomorrow.",
      "setup": "Continuing from the previous test (trial_end_date still ~T-12h). Make sure the modal is currently shown.",
      "steps": [
        "Click 'Remind me later' on the modal.",
        "Reload the page.",
        "Open DevTools → Application → Local Storage and confirm `dismissed_today:trial_expiry_modal` exists with today's date."
      ],
      "expected": "After clicking 'Remind me later', modal disappears immediately. After reload, modal does NOT reappear (banner and pill still do). LocalStorage entry visible with `YYYY-MM-DD` matching today.",
      "flow": "setup",
      "sequence": 5,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-recap-card-renders-with-counts",
      "category": "Trial value recap",
      "description": "BillingTab shows the new 'Your trial so far' recap card with five real metrics during the trial.",
      "setup": "Org with `plan_status = 'trialing'`, several discovered hosts/services and at least one daemon and one network. Created at least 2 days ago.",
      "steps": [
        "Open Settings → Billing.",
        "Scroll to find the 'Your trial so far' card (between the trial countdown and the Current Plan card)."
      ],
      "expected": "Card shows five tiles: hosts discovered, networks mapped, daemons connected, services identified, days into trial. Each shows a real, non-zero count where data exists. Days into trial roughly matches `now - org.created_at` in days.",
      "flow": "setup",
      "sequence": 6,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-recap-empty-state",
      "category": "Trial value recap",
      "description": "When all four entity counts are zero, the recap card shows the empty-state CTA pointing at the next incomplete onboarding step.",
      "setup": "Provision a brand-new trialing org (no daemons, no networks, no hosts, no services). The default network is fine but should have nothing in it.",
      "steps": [
        "Open Settings → Billing as the new org's Owner.",
        "Locate the 'Your trial so far' card."
      ],
      "expected": "Card title is 'Your trial so far'; body shows 'Get the most from your trial' with a 'Next: Install a daemon' line and a 'Get started' CTA. Clicking the CTA closes the Settings modal and lands on the Daemons tab with the create-daemon modal open.",
      "flow": "setup",
      "sequence": 7,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-ending-email-recap",
      "category": "Trial value recap",
      "description": "Trial-ending email (T-3d) renders the new five-metric recap block.",
      "setup": "Trigger a `BillingOperation::TrialWillEnd` event for a real trialing org with non-zero hosts/networks/daemons/services. Easiest: in dev, run a script that publishes the event directly via the event bus, or replay a Stripe `customer.subscription.trial_will_end` webhook for the org's subscription.",
      "steps": [
        "Wait for the email to arrive in the org owner's inbox (Brevo or SMTP).",
        "Open the email and view the rendered HTML."
      ],
      "expected": "Email body contains a 'Here's what Scanopy found during your trial' section with five rows: hosts discovered, networks mapped, daemons connected, services identified, and days into trial. Counts match the org's actual data.",
      "flow": "setup",
      "sequence": 8,
      "status": null,
      "feedback": null
    },
    {
      "id": "post-stripe-welcome-banner",
      "category": "Post-Stripe welcome",
      "description": "Welcome banner appears after a successful Stripe checkout completion and persists for 24 hours.",
      "setup": "Org currently `plan_status = 'trialing'` (or new). Have a working Stripe test card.",
      "steps": [
        "From Settings → Billing, click 'Manage Subscription' or otherwise initiate a Stripe Checkout flow.",
        "Complete checkout with a test card.",
        "Wait for the redirect back to the app."
      ],
      "expected": "After landing back in the app, a blue info banner appears at the top reading 'Welcome to {plan name} — your subscription is now active.' with an X close button. `localStorage.plan_activated_at` is set to a recent timestamp.",
      "flow": "setup",
      "sequence": 9,
      "status": null,
      "feedback": null
    },
    {
      "id": "post-stripe-welcome-banner-dismiss",
      "category": "Post-Stripe welcome",
      "description": "Dismissing the welcome banner hides it permanently (or until localStorage cleared).",
      "setup": "Continuing from previous test — banner is currently visible.",
      "steps": [
        "Click the X on the welcome banner.",
        "Reload the page.",
        "Open DevTools → Application → Local Storage and inspect `appbanner_dismissed:welcome_banner`."
      ],
      "expected": "Banner disappears immediately after click. After reload, banner stays gone. LocalStorage shows `appbanner_dismissed:welcome_banner = 'true'`.",
      "flow": "setup",
      "sequence": 10,
      "status": null,
      "feedback": null
    },
    {
      "id": "post-stripe-welcome-banner-24h-window",
      "category": "Post-Stripe welcome",
      "description": "Welcome banner stops rendering 24h after activation even without dismissal.",
      "setup": "Manually set `localStorage.plan_activated_at` to a timestamp 25 hours ago: `localStorage.setItem('plan_activated_at', String(Date.now() - 25*60*60*1000))`. Clear `appbanner_dismissed:welcome_banner` if present.",
      "steps": [
        "Reload the app while logged in as the org Owner with an active subscription.",
        "Look for the welcome banner."
      ],
      "expected": "Banner does NOT render despite `plan_status = 'active'` — the 24h window has elapsed.",
      "flow": "setup",
      "sequence": 11,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-pill-not-shown-with-payment",
      "category": "Trial urgency ramp",
      "description": "Pill / banner / modal all suppress when trialing user already has a payment method on file.",
      "setup": "Org with `plan_status = 'trialing'`, `trial_end_date = NOW() + INTERVAL '12 hours'`, AND `has_payment_method = true`.",
      "steps": [
        "Reload the app.",
        "Check sidebar pill, top-of-page banner, and any popup modal."
      ],
      "expected": "Sidebar trial pill: not visible. T-3d banner: not visible. T-1d modal: not visible. (Trial recap card on BillingTab still shows — it's informational.)",
      "flow": "setup",
      "sequence": 12,
      "status": null,
      "feedback": null
    },
    {
      "id": "paywall-gate-hit-export-modal",
      "category": "Paywall gate hit",
      "description": "paywall_gate_hit fires from ExportModal before upgrade modal opens",
      "setup": "Sign in as an Owner of an org on a plan that does NOT include PDF export (e.g. Free or a plan whose features.pdf_export is false). Open a topology view.",
      "steps": [
        "Open the export modal from a topology view",
        "Click a disabled paywalled export format (e.g. PDF)",
        "Watch PostHog activity feed for the events"
      ],
      "expected": "PostHog logs `paywall_gate_hit` with `{ feature: <format>, surface: 'export_modal', gate_type: 'plan_required' }` BEFORE `upgrade_button_clicked` and BEFORE the billing modal opens. The billing modal then opens.",
      "flow": "setup",
      "sequence": 13,
      "status": null,
      "feedback": null
    },
    {
      "id": "paywall-gate-hit-discovery-form",
      "category": "Paywall gate hit",
      "description": "paywall_gate_hit fires from disabled Scheduled discovery option",
      "setup": "Sign in as an Owner of an org on a plan WITHOUT scheduled_discovery (e.g. Free).",
      "steps": [
        "Open Discovery → New Scan → Details step",
        "Click the disabled 'Scheduled' run-type option",
        "Watch PostHog"
      ],
      "expected": "`paywall_gate_hit` with `{ feature: 'scheduled_discovery', surface: 'discovery_form', gate_type: 'plan_required' }` fires before the billing modal opens.",
      "flow": "setup",
      "sequence": 14,
      "status": null,
      "feedback": null
    },
    {
      "id": "paywall-gate-hit-share-panel-embeds",
      "category": "Paywall gate hit",
      "description": "paywall_gate_hit fires from ShareConfigPanel embeds upgrade button",
      "setup": "Sign in as an Owner of an org WITHOUT the `embeds` feature.",
      "steps": [
        "Open Shares → create or open a share view",
        "Open the share config panel and locate the embed code section",
        "Click the Upgrade button next to the locked embed code"
      ],
      "expected": "`paywall_gate_hit` with `{ feature: 'embeds', surface: 'share_panel', gate_type: 'plan_required' }` fires.",
      "flow": "setup",
      "sequence": 15,
      "status": null,
      "feedback": null
    },
    {
      "id": "paywall-gate-hit-sidebar",
      "category": "Paywall gate hit",
      "description": "paywall_gate_hit fires from sidebar Upgrade button",
      "setup": "Sign in as an Owner on a Free plan (so the sidebar Upgrade button is visible).",
      "steps": [
        "Click the Upgrade button at the bottom of the sidebar"
      ],
      "expected": "`paywall_gate_hit` with `{ feature: null, surface: 'sidebar', gate_type: 'plan_required' }` fires before the billing modal opens.",
      "flow": "setup",
      "sequence": 16,
      "status": null,
      "feedback": null
    },
    {
      "id": "paywall-gate-hit-billing-tab",
      "category": "Paywall gate hit",
      "description": "paywall_gate_hit fires from BillingTab View Plans button",
      "setup": "Sign in as an Owner on any non-Enterprise plan. Open Settings → Billing.",
      "steps": [
        "In the Billing tab, click the 'View Plans' / 'Upgrade Plan' / 'Change Plan' button"
      ],
      "expected": "`paywall_gate_hit` with `{ surface: 'billing_tab', gate_type: 'plan_required' }` fires before the billing modal opens.",
      "flow": "setup",
      "sequence": 17,
      "status": null,
      "feedback": null
    },
    {
      "id": "paywall-gate-hit-limit-hit-surfaces",
      "category": "Paywall gate hit",
      "description": "paywall_gate_hit fires with gate_type='limit_hit' for usage-limit gates",
      "setup": "Set the org to a plan with low limits and seed enough hosts/users/networks to be at the limit. Concretely: create a Free-tier org and add hosts/users/networks until each tab shows the at-limit Upgrade button. Verify visually that PlanUsage on the home dashboard also shows the Upgrade button.",
      "steps": [
        "On the home dashboard, click the Upgrade button in the Plan Usage card → expect surface='home_plan_usage', gate_type='limit_hit'",
        "In Settings → Networks, click the Upgrade button → expect surface='networks_tab', gate_type='limit_hit'",
        "In Settings → Users, click the Upgrade button → expect surface='users_tab', gate_type='limit_hit'",
        "In Hosts, click the Upgrade button → expect surface='hosts_tab', gate_type='limit_hit'"
      ],
      "expected": "Each click fires `paywall_gate_hit` with the matching surface and `gate_type: 'limit_hit'` before the billing modal opens.",
      "flow": "setup",
      "sequence": 18,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-card-impression-dedup",
      "category": "Trial card",
      "description": "trial_card_impression fires once per session and not again on reload",
      "setup": "Sign in as an Owner of an org with `plan_status='trialing'`, no payment method on file, and a non-null trial_end_date in the future. Clear sessionStorage in the browser DevTools first.",
      "steps": [
        "Open Settings → Billing — the amber trial card should be visible",
        "In the browser console, check sessionStorage for key `analytics_seen:trial_card_impression` (should be present)",
        "Watch PostHog — `trial_card_impression` should have fired with `{ trial_days_left, has_payment_method: false }`",
        "Refresh the page in the same tab",
        "Re-open Settings → Billing and watch PostHog — no new `trial_card_impression` should fire",
        "Open a new tab (different sessionStorage), sign in, navigate to Billing — `trial_card_impression` fires again"
      ],
      "expected": "Event fires exactly once per browser tab session even with reloads or remounts; new tab/session re-fires.",
      "flow": "setup",
      "sequence": 19,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-card-cta-clicked",
      "category": "Trial card",
      "description": "trial_card_cta_clicked fires on Add Payment Method click",
      "setup": "Same as trial-card-impression-dedup.",
      "steps": [
        "Open Settings → Billing",
        "Click 'Add Payment Method' on the trial card"
      ],
      "expected": "`trial_card_cta_clicked` fires with `{ trial_days_left, has_payment_method: false }` before the Stripe redirect.",
      "flow": "setup",
      "sequence": 20,
      "status": null,
      "feedback": null
    },
    {
      "id": "trial-card-dismissed",
      "category": "Trial card",
      "description": "Dismiss X fires trial_card_dismissed and persists across reloads",
      "setup": "Same as trial-card-impression-dedup. Clear localStorage entry `infocard_dismissed:trial_card_dismissed` first.",
      "steps": [
        "Open Settings → Billing — trial card visible with X in top-right",
        "Click the X dismiss button",
        "Confirm `trial_card_dismissed` fired in PostHog with `{ trial_days_left, has_payment_method: false }`",
        "Confirm the trial card is no longer rendered",
        "Refresh the page",
        "Re-open Settings → Billing"
      ],
      "expected": "Event fires exactly once. Card stays dismissed across page reloads (localStorage persistence). Other (non-trial) InfoCards on the page are unaffected.",
      "flow": "setup",
      "sequence": 21,
      "status": null,
      "feedback": null
    },
    {
      "id": "infocard-no-regression",
      "category": "Trial card",
      "description": "Other InfoCard usages still render normally without dismiss UI",
      "setup": "Sign in as a non-trial user (e.g. paid plan).",
      "steps": [
        "Visit Settings → Account, Settings → Organization, Settings → Billing (current plan card and need-help card)",
        "Visually inspect each InfoCard"
      ],
      "expected": "All InfoCards render normally without an X dismiss button. Layout unchanged from before.",
      "flow": "setup",
      "sequence": 22,
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
      "id": "cancellation-initiated-posthog-portal",
      "category": "Billing telemetry",
      "description": "Stripe Portal cancel (default 'cancel at period end') must surface in PostHog as a cancellation_initiated event. Replaces the earlier subscription-cancelled-end-to-end test, which expected the wrong event name for the Portal flow.",
      "setup": "Pick a test org on a paid subscription. Note the org's created_at and the subscription's monthly line totals for sanity-checking the enriched fields when they next ship.",
      "steps": [
        "Sign in as the org owner.",
        "Click Manage Subscription -> launches Stripe Customer Portal.",
        "In Portal, click Cancel subscription. Select reason 'Too expensive'. Add a comment.",
        "Confirm cancellation.",
        "Wait ~30s for the customer.subscription.updated webhook + async task.",
        "Open PostHog and filter for event name 'cancellation_initiated' on this org."
      ],
      "expected": "cancellation_initiated event present with metadata: stripe_reason = 'cancellation_requested', stripe_feedback = 'too_expensive', comment = the text you typed, reason_code = null (no app-side save-offer reason for a Portal cancel), planned_period_end is the actual subscription period end.",
      "status": null,
      "feedback": null
    },
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
  "branch": "ci/backward-compat-release-check",
  "tests": []
}
,
{
  "branch": "chore/compat-followups",
  "tests": [
    {
      "id": "coordinator-cp-command-fresh-dest",
      "category": "Coordinator setup docs",
      "description": "The updated coordinator cp command in /Users/maya/dev/scanopy/CLAUDE.md produces a flat ui/src/lib/data/*.json layout when the destination directory does not yet exist.",
      "steps": [
        "Open /Users/maya/dev/scanopy/CLAUDE.md and locate the 'Set up worktree dependencies' code block (around line 85).",
        "Confirm the command uses 'cp -r /Users/maya/dev/scanopy/ui/src/lib/data/. /Users/maya/dev/scanopy-<task-name>/ui/src/lib/data/' (note the trailing /. on the source).",
        "Confirm the command no longer contains 'rm -rf' for the data directory."
      ],
      "setup": "Pick any existing worktree under /Users/maya/dev/scanopy-* (or create a temporary one). Delete its ui/src/lib/data directory completely: rm -rf <worktree>/ui/src/lib/data. Then run the documented command verbatim with <task-name> substituted for that worktree's suffix.",
      "expected": "ls <worktree>/ui/src/lib/data shows flat *.json files (e.g. service-definitions.json, billing-plans.json) directly inside the data/ directory. There is no nested data/data/ subdirectory.",
      "expected_url": null,
      "expected_screenshot": null,
      "status": null,
      "feedback": null
    },
    {
      "id": "coordinator-cp-command-existing-dest",
      "category": "Coordinator setup docs",
      "description": "Re-running the updated cp command against an already-populated destination is idempotent — it does not create a nested ui/src/lib/data/data/ subdirectory.",
      "steps": [
        "Without deleting anything, run the documented cp command a second time against the same worktree used in coordinator-cp-command-fresh-dest."
      ],
      "setup": "Reuse the worktree from the previous test. Do NOT delete ui/src/lib/data first; the destination is intentionally pre-populated.",
      "expected": "ls <worktree>/ui/src/lib/data still shows flat *.json files. find <worktree>/ui/src/lib/data -type d returns only the data/ directory itself — no nested data/data/. service-definitions.json is still readable.",
      "expected_url": null,
      "expected_screenshot": null,
      "flow": "setup",
      "sequence": 2,
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
      "id": "stripe-customer-deleted-on-org-delete",
      "category": "Stripe teardown",
      "description": "When an org with a stripe_customer_id is deleted, the Stripe customer is deleted (and any active subscriptions auto-canceled).",
      "setup": "Create a fresh org and complete signup. Upgrade to a paid trial through the in-app billing flow so a Stripe customer + trialing subscription get created (this populates org.stripe_customer_id). Cancel the subscription via the in-app downgrade flow so the org returns to Free (deletion is gated on has_active_paid_subscription, which includes 'trialing'). Note the stripe_customer_id from the DB or Stripe dashboard before proceeding.",
      "steps": [
        "Sign in as the owner of the prepared org.",
        "Navigate to org settings and delete the organization.",
        "Confirm the deletion completes successfully (you are signed out / redirected).",
        "Open the Stripe dashboard (test mode), search for the customer ID noted in setup.",
        "Confirm the customer is shown as 'Deleted' (or `GET /v1/customers/{id}` returns `\"deleted\": true`).",
        "Confirm any subscription that existed on that customer is now canceled."
      ],
      "expected": "Stripe customer is marked deleted; any active subscription is canceled; backend logs show no errors from the deletion path.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
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
      "id": "digest-arrives-after-complete",
      "category": "Discovery digest — happy path",
      "description": "After a Discovery scan completes Successfully, every user with access to the scanned network (explicit + implicit Owner/Admin) receives the digest email, and the email reflects what the scan found.",
      "setup": "Create one paid org with one network. Add at least 3 users: one Owner, one Admin (no explicit network access), one Member with explicit user_network_access for the network. All three users should have email_settings.discovery_digest = true (the default). Seed the network with: ~5 hosts that existed before the scan window, including one that already has 1-2 ports/services. Run a real Unified discovery against the network that produces (a) ≥1 new host, (b) ≥1 host with a new port AND a stale port (port that existed before but isn't reported this scan), (c) ≥1 host that previously existed but isn't seen this session (pre-seed `last_seen_at` to a value before T_start).",
      "steps": [
        "Watch the Owner's, Admin's, and Member's inboxes after the scan reaches Complete.",
        "Open the digest email in each inbox.",
        "Verify the top stats banner shows non-zero counts for new / vanished / changed hosts and the subnets-scanned cell.",
        "Verify the Subnets-scanned section lists the subnets the daemon was configured for.",
        "Verify the New hosts section shows the newly-discovered host(s).",
        "Verify the Hosts-not-seen section shows the pre-seeded vanished host.",
        "Verify the Hosts-with-changes section shows the host whose ports changed, with a 'What changed this scan' subsection listing added vs removed ports."
      ],
      "expected": "All three users receive the same email. The stats banner counts match the section counts. Each affected host is rendered as a card with name + status badge + Services/IP Addresses/Interfaces/Ports rows mirroring the in-app HostCard. The 'Manage email preferences' link in the footer points to /settings?tab=email.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-respects-per-user-toggle",
      "category": "Discovery digest — opt-out",
      "description": "Opening the Email tab no longer hangs the browser. A user who turns off the discovery digest stops receiving emails immediately; other users in the same org keep receiving them.",
      "setup": "Reuse the org and network from the happy-path test.",
      "steps": [
        "Sign in as the Member user, open Settings → Email tab.",
        "Verify the tab renders without the browser freezing or any infinite-loop warning in the console.",
        "Verify the 'Discovery scan summary' checkbox is on by default.",
        "Uncheck the box and click Save.",
        "Trigger another Unified discovery on the network and wait for Complete.",
        "Check the Member's inbox — no digest should arrive.",
        "Check the Owner's and Admin's inboxes — both should still receive the digest."
      ],
      "expected": "Email tab opens cleanly. Save succeeds with a success toast. No digest arrives at the opted-out user; other recipients still receive it.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-empty-suppressed",
      "category": "Discovery digest — empty session",
      "description": "When a discovery session produces zero changes, no digest is sent at all (regardless of recipient settings).",
      "setup": "Reuse the org and network. Run a discovery that re-scans the same hosts with no new entities and no stale children — i.e. all hosts and children are simply refreshed (last_seen_at updated, no created_at in window, no last_seen_at < T_start on a scanned host).",
      "steps": [
        "Watch every recipient's inbox after the scan completes."
      ],
      "expected": "No digest email arrives at any recipient.",
      "flow": "setup",
      "sequence": 3,
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
      "sequence": 4,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-expandable-large-list",
      "category": "Discovery digest — rendering",
      "description": "Sections with more than 10 items show the first 10 inline and put the rest inside a collapsible disclosure that the recipient can expand. Replaces the previous '+N more' truncation.",
      "setup": "Run a discovery that adds at least 15 new hosts to the network. (A fresh subnet scan against a populated lab works.)",
      "steps": [
        "Open the digest email in the Owner's inbox.",
        "Find the New hosts section. Verify exactly 10 host cards are visible.",
        "Below the visible cards, click the 'Show all 15' disclosure (or whatever the true count is). The remaining cards should expand into view.",
        "Verify the section header still shows the true count (e.g. 'New hosts discovered (15)')."
      ],
      "expected": "10 cards inline + a 'Show all N' disclosure that expands to reveal the rest. No '+N more' text or truncation.",
      "flow": "setup",
      "sequence": 5,
      "status": null,
      "feedback": null
    },
    {
      "id": "email-tab-reflects-server-state",
      "category": "Email tab — load + save round-trip",
      "description": "The Email tab shows the current value of the user's preference and persists changes.",
      "setup": "Pick any user. Set their email_settings.discovery_digest = false directly (UPDATE users SET email_settings = '{\"discovery_digest\": false}'::jsonb WHERE id = '<user_id>').",
      "steps": [
        "Sign in as that user and open Settings → Email tab.",
        "Verify the 'Discovery scan summary' checkbox is unchecked.",
        "Check the box, click Save.",
        "Reload the page, reopen Settings → Email tab.",
        "Verify the box is checked."
      ],
      "expected": "The initial state matches the DB value. After save, the new value persists across reload.",
      "flow": "setup",
      "sequence": 6,
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
      "sequence": 7,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-no-topology-ready-email",
      "category": "Discovery digest — superseded emails removed",
      "description": "Completing the first Network/Unified discovery for an org no longer triggers the legacy 'Your Topology is Ready!' email. Only the per-session digest arrives.",
      "setup": "Create a brand new org (so FirstDiscoveryCompleted has not fired yet). Add one Owner, one daemon, one network with at least one subnet. Make sure the org has email_settings.discovery_digest = true on the Owner.",
      "steps": [
        "Trigger a Unified discovery against the network.",
        "Wait for it to reach Complete.",
        "Check the Owner's inbox."
      ],
      "expected": "Exactly one email arrives — the Discovery scan summary digest. No 'Your Topology is Ready!' email is delivered.",
      "flow": "setup",
      "sequence": 8,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-summary-banner",
      "category": "Discovery digest — top summary",
      "description": "The digest opens with a stats banner showing per-bucket counts.",
      "setup": "Run any discovery that produces a mix of changes (≥1 new host, ≥1 vanished host, ≥1 changed host, ≥1 new VLAN).",
      "steps": [
        "Open the digest email in the Owner's inbox.",
        "Scroll to the top, just below the 'Network: X' header."
      ],
      "expected": "A single banner row with labelled count cells: new hosts, vanished hosts, changed hosts, VLANs detected, VLANs no longer detected, subnets scanned. Counts match the per-section counts further down.",
      "flow": "setup",
      "sequence": 9,
      "status": null,
      "feedback": null
    },
    {
      "id": "digest-host-card-shape",
      "category": "Discovery digest — host cards",
      "description": "Each affected host renders as a card mirroring HostCard.svelte from the UI.",
      "setup": "Seed a host with multiple services, IPs, interfaces, and ports — at least 3 of each. Trigger a discovery that adds/removes children so the host is classified as Changed.",
      "steps": [
        "Open the digest email in the Owner's inbox.",
        "Find the Hosts-with-changes section and the card for the seeded host.",
        "Verify the card header shows the hostname plus a 'Changed' badge.",
        "Verify there are rows for Services, IP Addresses, Interfaces, and Ports, each enumerating the host's current live entities.",
        "Verify the card ends with a 'What changed this scan' block listing what was added vs removed."
      ],
      "expected": "Each affected host is one bordered card with name + colored status badge + the same field rows used in the UI's HostCard. Changed hosts also show the deltas block.",
      "flow": "setup",
      "sequence": 10,
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
  "branch": "feat/phase5-subscription-mechanics",
  "tests": [
    {
      "id": "billing-tab-shows-both-manage-and-cancel-buttons",
      "category": "Billing tab CTAs",
      "description": "Active paid org sees Manage Subscription (Stripe Portal) AND Cancel Subscription side by side",
      "setup": "Pick an org with an active paid subscription (plan_status = 'active', plan != Free).",
      "steps": ["Open Settings → Billing as the org Owner"],
      "expected": "The Current Plan card shows two stacked buttons: 'Manage Subscription' (opens Stripe Portal on click) and 'Cancel Subscription' below it (opens the in-app cancel modal on click). Paused orgs still show only 'Resume now'; past-due orgs still show only 'Manage Subscription'.",
      "status": null,
      "feedback": null
    },
    {
      "id": "cancel-modal-has-no-stepper",
      "category": "Cancel modal",
      "description": "The cancel modal does not render a breadcrumb / stepper at the top",
      "setup": "Pick an org with an active paid subscription.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "Inspect the top of the modal"
      ],
      "expected": "Modal header shows 'Cancel subscription' as the title and the close button. No numbered stepper, no 'Reason / Save Offer / Confirm' breadcrumb, no step labels of any kind are visible.",
      "status": null,
      "feedback": null
    },
    {
      "id": "cancel-modal-too-expensive-shows-pause-and-discount",
      "category": "Cancel modal",
      "description": "Selecting 'Too expensive' on step 1 advances to step 2 with both Pause and Discount panels and a Confirm Cancellation button",
      "setup": "Pick an org with an active paid subscription (last_paused_at is null so no pause cooldown applies).",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "On the reason screen, pick 'Too expensive'",
        "Click 'Continue cancelling'"
      ],
      "expected": "Modal advances to the save-offer screen. Two panels render: 'Pause subscription' (with 30/60/90 buttons + 'Pause until {date}' preview + 'Pause subscription' CTA) and 'Apply a discount' (with description + 'Apply discount' CTA). Below the panels: a confirmation disclosure starting 'If you confirm, you'll keep access until the end of your current billing cycle...'. Footer: 'Back' on the left, 'Confirm cancellation' (red, btn-danger) on the right.",
      "flow": "cancel-too-expensive",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "cancel-modal-pause-redeem-flips-status",
      "category": "Cancel modal / Pause",
      "description": "Redeeming the pause save-offer pauses the subscription and surfaces the Resume button",
      "steps": [
        "From the previous test (save-offer screen with Pause + Discount panels), click '60 days' in the Pause panel",
        "Verify the 'Pause until {date}' preview updates",
        "Click 'Pause subscription'",
        "Wait for the toast and modal close",
        "Look at the Billing tab status pill and the action button"
      ],
      "expected": "Toast 'Subscription paused until {date}'. Modal closes. Org status pill flips to 'Paused' (orange). The 'Resume now' button appears in place of the Manage / Cancel buttons. The blue inline alert reads 'Your subscription is paused. Resume any time...'",
      "flow": "cancel-too-expensive",
      "sequence": 2,
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
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "cancel-modal-other-reason-no-offers-still-confirms",
      "category": "Cancel modal",
      "description": "Reasons with no save offers go directly to the confirm screen — no offer panels, just the disclosure + Confirm Cancellation",
      "setup": "Pick an org with active paid subscription. Reset any prior modal state.",
      "steps": [
        "Open Settings → Billing → click 'Cancel Subscription'",
        "On the reason screen, pick 'Other'",
        "Optionally type a comment",
        "Click 'Continue cancelling'"
      ],
      "expected": "Modal advances to a step-2 view with NO save-offer panels (because Other has none). The confirmation disclosure renders, with Back / Confirm Cancellation footer.",
      "flow": "cancel-other",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "cancel-modal-confirm-shows-period-end-toast",
      "category": "Cancel modal",
      "description": "Confirming cancellation surfaces a success toast with the Stripe-derived period end and closes the modal",
      "steps": [
        "From the previous test (modal on confirm screen), click 'Confirm cancellation'",
        "Wait for the cancel mutation to complete",
        "Look at the toast area and the BillingTab beneath"
      ],
      "expected": "Toast: 'Your subscription has been cancelled. Access continues until {periodEnd}.' Modal closes. BillingTab status pill flips to 'Downgrading' (amber, mapped from pending_cancellation). The existing inline warning 'Your plan will change to Free at the end of your current billing cycle.' appears.",
      "flow": "cancel-other",
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
      "id": "manage-subscription-opens-stripe-portal",
      "category": "Billing tab CTAs",
      "description": "Manage Subscription on an active paid org redirects to the Stripe Customer Portal",
      "setup": "Pick an org with an active paid subscription.",
      "steps": ["Open Settings → Billing → click 'Manage Subscription'"],
      "expected": "Browser redirects to the Stripe Customer Portal (billing.stripe.com or the configured portal URL). Returning from the portal restores the Billing tab.",
      "status": null,
      "feedback": null
    },
    {
      "id": "stripe-metadata-stash",
      "category": "Cancel — Stripe-side verification",
      "description": "Confirmed cancellations write the canonical Scanopy reason to Stripe Subscription metadata",
      "setup": "After running cancel-modal-confirm-shows-period-end-toast, look up the affected subscription in the Stripe dashboard.",
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
      "id": "first-invoice-caption-inline",
      "category": "Feature 5 — First-invoice date caption",
      "description": "Each Cloud paid plan card that offers a free trial shows an inline 'First invoice on {date}' caption directly beneath its CTA. The CTA itself is one-click straight to Stripe — no intermediate confirmation step.",
      "setup": "Use an org on the Free plan that has never trialed (so trial offers are shown). Open the Billing modal (Settings → Billing → Upgrade, or via any UpgradeButton).",
      "steps": [
        "Visually scan the Cloud paid plan cards (e.g., Pro, Team, Business) — confirm each has a small caption directly below its primary CTA reading 'First invoice on {Month Day, Year}'",
        "Verify the date in each caption equals today + that plan's trial_days (cards for plans with different trial lengths should show different dates)",
        "Verify NO caption appears beneath the Free plan CTA or the Enterprise 'Request Information' CTA",
        "Click the primary CTA on a paid plan card (e.g., 'Start free trial — no card required')",
        "Verify the click goes DIRECTLY to Stripe Checkout — no intermediate confirmation step, no second click required"
      ],
      "expected": "Caption is visible on every trial-eligible Cloud paid plan card. Dates are correct per plan. CTA is one-tap to Stripe. No extra step.",
      "status": null,
      "feedback": null
    },
    {
      "id": "first-invoice-caption-hidden-for-trialing-switch",
      "category": "Feature 5 — First-invoice date caption",
      "description": "Users currently trialing who view the plan picker to switch plans should NOT see the caption (the caption applies to first-time trial entry, not plan switches).",
      "setup": "Use an org on a paid plan with plan_status='trialing'. Open the Billing modal.",
      "steps": [
        "Scan the Cloud plan cards — confirm NO 'First invoice on …' caption appears beneath any CTA",
        "Confirm the CTAs read 'Switch plan' (not 'Start free trial')"
      ],
      "expected": "No first-invoice caption rendered. Switch-plan CTA behaves as before.",
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
