<script lang="ts">
	import { loadStripe, type Stripe, type StripeElements } from '@stripe/stripe-js';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		common_cancel,
		common_continue,
		common_loading,
		common_processing,
		billing_cardError,
		billing_cardLoadError
	} from '$lib/paraglide/messages';

	let {
		clientSecret,
		description = undefined,
		submitLabel = common_continue(),
		onSuccess,
		onCancel = undefined
	}: {
		/** Client secret from a backend-created SetupIntent. */
		clientSecret: string;
		/** Optional lead-in text rendered above the Payment Element. */
		description?: string;
		submitLabel?: string;
		/**
		 * Called with the confirmed SetupIntent id after the card is collected.
		 * The caller finalizes the payment method (and proceeds, e.g. to
		 * checkout). May be async; the form stays disabled until it resolves.
		 */
		onSuccess: (setupIntentId: string) => void | Promise<void>;
		onCancel?: () => void;
	} = $props();

	const configQuery = useConfigQuery();
	let publishableKey = $derived(configQuery.data?.stripe_publishable_key ?? null);
	// Distinguish "config still loading" from "config loaded but key absent" so
	// we can surface a clear error instead of an indefinitely-blank element.
	let configLoaded = $derived(configQuery.data != null);

	let container = $state<HTMLDivElement | null>(null);
	let stripe: Stripe | null = null;
	let elements: StripeElements | null = null;
	let ready = $state(false);
	let busy = $state(false);
	let errorMessage = $state('');
	let loadFailed = $state(false);
	let initialized = false;

	// Mount the Payment Element once we have the publishable key, a client
	// secret, and the container node. loadStripe + element creation happen once.
	$effect(() => {
		if (initialized || !clientSecret || !container) return;

		// Billing is enabled but no publishable key is configured on this
		// deployment — Elements can't load. Fail loudly rather than rendering a
		// blank box. (Operator fix: set SCANOPY_STRIPE_KEY / --stripe-key.)
		if (configLoaded && !publishableKey) {
			initialized = true;
			loadFailed = true;
			errorMessage = billing_cardLoadError();
			console.error(
				'StripeCardForm: stripe_publishable_key missing from /api/config — set SCANOPY_STRIPE_KEY (or --stripe-key) on the server.'
			);
			return;
		}

		if (!publishableKey) return; // config still loading

		initialized = true;
		const node = container;
		void (async () => {
			stripe = await loadStripe(publishableKey);
			if (!stripe) {
				loadFailed = true;
				errorMessage = billing_cardLoadError();
				return;
			}
			// Match Elements to the app's active theme (toggled via a `dark` class
			// on <html>), so the card fields don't clash with a light/dark modal.
			const isDark = document.documentElement.classList.contains('dark');
			elements = stripe.elements({
				clientSecret,
				appearance: { theme: isDark ? 'night' : 'stripe' }
			});
			const paymentElement = elements.create('payment', { layout: 'accordion' });
			paymentElement.on('ready', () => (ready = true));
			paymentElement.mount(node);
		})();
	});

	async function handleSubmit() {
		if (!stripe || !elements || busy) return;
		busy = true;
		errorMessage = '';

		const { error, setupIntent } = await stripe.confirmSetup({
			elements,
			redirect: 'if_required'
		});

		if (error) {
			errorMessage = error.message ?? billing_cardError();
			busy = false;
			return;
		}

		if (setupIntent?.status === 'succeeded' && setupIntent.id) {
			try {
				await onSuccess(setupIntent.id);
			} catch {
				// onSuccess (finalize/checkout) surfaces its own toast; re-enable
				// so the user can retry. On success the caller unmounts this form.
				busy = false;
			}
			return;
		}

		errorMessage = billing_cardError();
		busy = false;
	}
</script>

<!-- Fills a modal-content flex column: the Payment Element scrolls, the action
     buttons stay pinned in the footer so Continue is always reachable. -->
<form
	onsubmit={(e) => {
		e.preventDefault();
		handleSubmit();
	}}
	class="flex min-h-0 flex-1 flex-col"
>
	<div class="min-h-0 flex-1 space-y-4 overflow-auto p-6">
		{#if description}
			<p class="text-secondary text-sm">{description}</p>
		{/if}

		<!-- Stripe mounts the Payment Element iframe here. Reserve height so the
		     layout doesn't jump while it loads. -->
		<div class="relative min-h-[3rem]">
			<div bind:this={container}></div>
			{#if !ready && !loadFailed}
				<p class="text-secondary absolute inset-0 flex items-center justify-center text-sm">
					{common_loading()}
				</p>
			{/if}
		</div>

		{#if errorMessage}
			<p class="text-sm text-red-400">{errorMessage}</p>
		{/if}
	</div>

	<div class="modal-footer flex items-center justify-end gap-3">
		{#if onCancel}
			<button type="button" class="btn-secondary" disabled={busy} onclick={onCancel}>
				{common_cancel()}
			</button>
		{/if}
		<button type="submit" class="btn-primary" disabled={busy || !ready}>
			{busy ? common_processing() : submitLabel}
		</button>
	</div>
</form>
