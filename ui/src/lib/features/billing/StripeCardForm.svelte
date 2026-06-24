<script lang="ts">
	import { loadStripe, type Stripe, type StripeElements } from '@stripe/stripe-js';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		common_cancel,
		common_continue,
		common_processing,
		billing_cardError
	} from '$lib/paraglide/messages';

	let {
		clientSecret,
		submitLabel = common_continue(),
		onSuccess,
		onCancel = undefined
	}: {
		/** Client secret from a backend-created SetupIntent. */
		clientSecret: string;
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

	let container = $state<HTMLDivElement | null>(null);
	let stripe: Stripe | null = null;
	let elements: StripeElements | null = null;
	let ready = $state(false);
	let busy = $state(false);
	let errorMessage = $state('');
	let initialized = false;

	// Mount the Payment Element once we have the publishable key, a client
	// secret, and the container node. loadStripe + element creation happen once.
	$effect(() => {
		if (initialized || !publishableKey || !clientSecret || !container) return;
		initialized = true;
		const node = container;
		void (async () => {
			stripe = await loadStripe(publishableKey);
			if (!stripe) {
				errorMessage = billing_cardError();
				return;
			}
			elements = stripe.elements({ clientSecret, appearance: { theme: 'night' } });
			const paymentElement = elements.create('payment');
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

<form
	onsubmit={(e) => {
		e.preventDefault();
		handleSubmit();
	}}
	class="flex flex-col gap-4"
>
	<div bind:this={container}></div>

	{#if errorMessage}
		<p class="text-sm text-red-400">{errorMessage}</p>
	{/if}

	<div class="flex items-center justify-end gap-3">
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
