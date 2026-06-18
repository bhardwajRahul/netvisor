import * as m from '$lib/paraglide/messages';
import type { components } from '$lib/api/schema';

type OrgBase = components['schemas']['OrganizationBase'];

/**
 * Human-facing label for the next renewal / period-end date on the org's
 * current plan. The underlying field (`next_renewal_at`) mirrors Stripe's
 * `subscription.items.data[0].current_period_end`; how that date should be
 * described depends on `plan_status`:
 *   - active → "Next renewal on ..."
 *   - trialing → "First invoice on ..." (Stripe sets period_end = trial_end
 *     for trialing subs; the user is billed at that point if they don't cancel)
 *   - pending_cancellation → "Subscription ends on ..."
 *   - past_due / paused / cancelled / unknown → null (hide; the stored value
 *     can be stale, meaningless, or the wrong concept for these states)
 *
 * Returns null when no label should render so callers can write
 * `{#if renewalLabel(org)} <p>{...}</p> {/if}` without extra guards.
 */
export function renewalLabel(org: OrgBase | undefined | null): string | null {
	if (!org?.next_renewal_at) return null;
	const formatted = new Date(org.next_renewal_at).toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric'
	});
	switch (org.plan_status) {
		case 'active':
			return m.billing_nextRenewalOn({ date: formatted });
		case 'trialing':
			return m.billing_firstInvoiceOn({ date: formatted });
		case 'pending_cancellation':
			return m.billing_subscriptionEndsOn({ date: formatted });
		default:
			return null;
	}
}
