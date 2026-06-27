<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import StripeCardForm from '$lib/features/billing/StripeCardForm.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import {
		useCreateSetupIntentMutation,
		useFinalizePaymentMethodMutation
	} from '$lib/features/billing/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { modalState, closeModal } from '$lib/shared/stores/modal-registry';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import {
		common_save,
		billing_addPaymentMethod,
		billing_paymentMethodAdded
	} from '$lib/paraglide/messages';

	// Single global instance opened by any "Add/Update payment method" nudge via
	// openModal('payment-method').
	let isOpen = $derived($modalState.name === 'payment-method');

	const setupIntentMutation = useCreateSetupIntentMutation();
	const finalizeMutation = useFinalizePaymentMethodMutation();
	const currentUserQuery = useCurrentUserQuery();
	let userEmail = $derived(currentUserQuery.data?.email);

	let clientSecret = $state<string | null>(null);

	async function handleOpen() {
		clientSecret = null;
		try {
			clientSecret = await setupIntentMutation.mutateAsync();
		} catch {
			// setup-intent error is toasted by the mutation; close the empty dialog
			closeModal();
		}
	}

	async function handleSuccess(setupIntentId: string) {
		await finalizeMutation.mutateAsync(setupIntentId);
		closeModal();
		// Converge once the webhook/finalize records the new payment method, then
		// confirm to the user (mirrors the other billing flows' success cadence).
		await waitForOrgUpdate((o) => o.has_payment_method ?? false);
		pushSuccess(billing_paymentMethodAdded());
	}
</script>

<GenericModal
	{isOpen}
	name="payment-method"
	title={billing_addPaymentMethod()}
	size="md"
	compactPadding={true}
	showCloseButton={true}
	onClose={() => closeModal()}
	onOpen={handleOpen}
>
	{#if clientSecret}
		<StripeCardForm
			{clientSecret}
			email={userEmail}
			submitLabel={common_save()}
			onSuccess={handleSuccess}
			onCancel={() => closeModal()}
		/>
	{:else}
		<div class="flex min-h-[12rem] items-center justify-center p-6">
			<Loading />
		</div>
	{/if}
</GenericModal>
