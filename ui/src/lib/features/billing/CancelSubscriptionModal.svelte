<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal, { type ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import {
		usePauseSubscriptionMutation,
		useApplyDiscountSaveOfferMutation,
		useCancelSubscriptionMutation
	} from '$lib/features/billing/queries';
	import cancelReasons from '$lib/data/cancel-reasons.json';
	import saveOffers from '$lib/data/save-offers.json';
	import type { components } from '$lib/api/schema';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import {
		common_back,
		common_confirm,
		common_reason,
		settings_billing_cancelModal_title,
		settings_billing_cancelModal_step2Label,
		settings_billing_cancelModal_reasonHeading,
		settings_billing_cancelModal_reasonHelp,
		settings_billing_cancelModal_commentLabel,
		settings_billing_cancelModal_commentPlaceholder,
		settings_billing_cancelModal_continueCancel,
		settings_billing_cancelModal_keepSubscription,
		settings_billing_cancelModal_confirmHeading,
		settings_billing_cancelModal_confirmDisclosure,
		settings_billing_cancelModal_confirmCta,
		settings_billing_cancelModal_doneSummary,
		settings_billing_cancelModal_done,
		settings_billing_saveOffer_pauseTitle,
		settings_billing_saveOffer_pauseSubtitle,
		settings_billing_saveOffer_pauseDuration30,
		settings_billing_saveOffer_pauseDuration60,
		settings_billing_saveOffer_pauseDuration90,
		settings_billing_saveOffer_pausePreview,
		settings_billing_saveOffer_pauseCta,
		settings_billing_saveOffer_pauseCooldown,
		settings_billing_saveOffer_discountTitle,
		settings_billing_saveOffer_discountSubtitle,
		settings_billing_saveOffer_discountCta
	} from '$lib/paraglide/messages';

	type CancelReason = components['schemas']['CancelReason'];
	type PauseDuration = components['schemas']['PauseDuration'];

	let {
		isOpen = false,
		onClose,
		lastPausedAt = null,
		onSubscriptionChanged
	}: {
		isOpen?: boolean;
		onClose: () => void;
		/** Org's `last_paused_at` — used for 6-month rolling pause cooldown messaging. */
		lastPausedAt?: string | null;
		/** Called after pause/discount/cancel succeed so the caller can refresh the org payload. */
		onSubscriptionChanged?: () => void;
	} = $props();

	type Step = 1 | 2 | 3;
	let currentStep = $state<Step>(1);
	let confirmedPeriodEnd = $state<string | null>(null);

	// Mirror state for the reason value. TanStack's `form.state.values` is not
	// tracked by Svelte 5 `$derived`, so we mirror the form's reason_code into
	// plain `$state` via a store subscription and read from this for any
	// reactive UI (button enable/disable, save-offer lookup, etc).
	let selectedReason = $state<CancelReason | ''>('');
	let selectedPauseDuration = $state<PauseDuration>('days30');

	const cancelMutation = useCancelSubscriptionMutation();
	const pauseMutation = usePauseSubscriptionMutation();
	const discountMutation = useApplyDiscountSaveOfferMutation();

	const form = createForm(() => ({
		defaultValues: {
			reason_code: '' as CancelReason | '',
			comment: ''
		},
		onSubmit: () => {
			// Step transitions are imperative — submit handler is unused.
		}
	}));

	$effect(() => {
		// Mirror the form's reason_code into $state so $derived expressions
		// (offersForReason, button gating) react to changes.
		// `form.state.values` is NOT tracked by $derived — read it from inside
		// a `form.store.subscribe` callback (CLAUDE.md TanStack reactivity gap).
		return form.store.subscribe(() => {
			const v = form.state.values.reason_code as CancelReason | '';
			if (v !== selectedReason) {
				selectedReason = v;
			}
		});
	});

	const reasonOptions = $derived([
		{ value: '', label: '—', disabled: true },
		...cancelReasons.map((r) => ({
			value: r.id,
			label: r.name ?? r.id
		}))
	]);

	const offersForReason = $derived.by<string[]>(() => {
		if (!selectedReason) return [];
		const reason = cancelReasons.find((r) => r.id === selectedReason);
		const offers = (reason?.metadata as { save_offers?: string[] } | null | undefined)?.save_offers;
		return offers ?? [];
	});

	const offerMeta = (offerId: string) => saveOffers.find((o) => o.id === offerId);

	const pauseCooldownEnd = $derived.by<Date | null>(() => {
		if (!lastPausedAt) return null;
		const last = new Date(lastPausedAt);
		const eligible = new Date(
			last.getFullYear(),
			last.getMonth() + 6,
			last.getDate(),
			last.getHours(),
			last.getMinutes(),
			last.getSeconds()
		);
		return eligible.getTime() > Date.now() ? eligible : null;
	});

	const pauseResumesAt = $derived.by<Date>(() => {
		const days =
			selectedPauseDuration === 'days30' ? 30 : selectedPauseDuration === 'days60' ? 60 : 90;
		return new Date(Date.now() + days * 24 * 60 * 60 * 1000);
	});

	const pauseDurationOptions = $derived([
		{
			value: 'days30' as PauseDuration,
			label: settings_billing_saveOffer_pauseDuration30()
		},
		{
			value: 'days60' as PauseDuration,
			label: settings_billing_saveOffer_pauseDuration60()
		},
		{
			value: 'days90' as PauseDuration,
			label: settings_billing_saveOffer_pauseDuration90()
		}
	]);

	function fmtDate(d: Date | string): string {
		const dt = typeof d === 'string' ? new Date(d) : d;
		return dt.toLocaleDateString(undefined, {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		});
	}

	const tabs = $derived<ModalTab[]>([
		{ id: 'reason', label: common_reason() },
		{
			id: 'offer',
			label: settings_billing_cancelModal_step2Label(),
			disabled: offersForReason.length === 0
		},
		{ id: 'confirm', label: common_confirm() }
	]);

	const activeTab = $derived(
		currentStep === 1 ? 'reason' : currentStep === 2 ? 'offer' : 'confirm'
	);

	function reset() {
		currentStep = 1;
		confirmedPeriodEnd = null;
		selectedReason = '';
		selectedPauseDuration = 'days30';
		form.reset();
	}

	function handleClose() {
		onClose();
		// Defer reset until after close animation so step 1 doesn't flicker.
		setTimeout(reset, 200);
	}

	function goToStep2OrSkip() {
		currentStep = offersForReason.length === 0 ? 3 : 2;
	}

	function goBackFromStep3() {
		currentStep = offersForReason.length === 0 ? 1 : 2;
	}

	async function handlePauseRedeem() {
		try {
			await pauseMutation.mutateAsync(selectedPauseDuration);
			onSubscriptionChanged?.();
			handleClose();
		} catch {
			// Mutation onError surfaces toast.
		}
	}

	async function handleDiscountRedeem() {
		try {
			await discountMutation.mutateAsync();
			onSubscriptionChanged?.();
			handleClose();
		} catch {
			// Mutation onError surfaces toast.
		}
	}

	async function handleConfirmCancel() {
		if (!selectedReason) return;
		const shownOffers = offersForReason as Array<components['schemas']['SaveOffer']>;
		try {
			const response = await cancelMutation.mutateAsync({
				reason_code: selectedReason,
				comment: form.state.values.comment || null,
				save_offer_shown: shownOffers,
				save_offer_redeemed: null
			});
			confirmedPeriodEnd = response.period_end;
			onSubscriptionChanged?.();
		} catch {
			// Mutation onError surfaces toast; stay on step 3.
		}
	}
</script>

<GenericModal
	{isOpen}
	title={settings_billing_cancelModal_title()}
	size="md"
	onClose={handleClose}
	{tabs}
	{activeTab}
	tabStyle="stepper"
>
	<div class="flex flex-col gap-6 p-6">
		{#if currentStep === 1}
			<div class="space-y-4">
				<div>
					<h3 class="text-primary text-lg font-semibold">
						{settings_billing_cancelModal_reasonHeading()}
					</h3>
					<p class="text-secondary mt-1 text-sm">
						{settings_billing_cancelModal_reasonHelp()}
					</p>
				</div>
				<form.Field name="reason_code">
					{#snippet children(field: AnyFieldApi)}
						<SelectInput
							id="cancel-reason"
							label={settings_billing_cancelModal_reasonHeading()}
							{field}
							required={true}
							options={reasonOptions}
						/>
					{/snippet}
				</form.Field>
				<form.Field name="comment">
					{#snippet children(field: AnyFieldApi)}
						<TextArea
							id="cancel-comment"
							label={settings_billing_cancelModal_commentLabel()}
							placeholder={settings_billing_cancelModal_commentPlaceholder()}
							rows={3}
							{field}
						/>
					{/snippet}
				</form.Field>
			</div>
		{:else if currentStep === 2}
			<div class="space-y-4">
				{#each offersForReason as offerId (offerId)}
					{#if offerId === 'pause'}
						<div class="card card-static space-y-3 p-4">
							<div>
								<h4 class="text-primary text-base font-semibold">
									{offerMeta('pause')?.name ?? settings_billing_saveOffer_pauseTitle()}
								</h4>
								<p class="text-secondary mt-1 text-sm">
									{settings_billing_saveOffer_pauseSubtitle()}
								</p>
							</div>
							{#if pauseCooldownEnd}
								<p class="text-sm text-warning">
									{settings_billing_saveOffer_pauseCooldown({
										nextEligibleDate: fmtDate(pauseCooldownEnd)
									})}
								</p>
							{:else}
								<div class="grid grid-cols-3 gap-2">
									{#each pauseDurationOptions as d (d.value)}
										<button
											type="button"
											class="card-static rounded-md border p-2 text-sm {selectedPauseDuration ===
											d.value
												? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
												: ''}"
											onclick={() => (selectedPauseDuration = d.value)}
										>
											{d.label}
										</button>
									{/each}
								</div>
								<p class="text-tertiary text-sm">
									{settings_billing_saveOffer_pausePreview({
										resumesAt: fmtDate(pauseResumesAt)
									})}
								</p>
								<button
									type="button"
									class="btn-primary w-full"
									disabled={pauseMutation.isPending}
									onclick={handlePauseRedeem}
								>
									{settings_billing_saveOffer_pauseCta()}
								</button>
							{/if}
						</div>
					{:else if offerId === 'discount'}
						<div class="card card-static space-y-3 p-4">
							<div>
								<h4 class="text-primary text-base font-semibold">
									{offerMeta('discount')?.name ?? settings_billing_saveOffer_discountTitle()}
								</h4>
								<p class="text-secondary mt-1 text-sm">
									{settings_billing_saveOffer_discountSubtitle()}
								</p>
							</div>
							<button
								type="button"
								class="btn-primary w-full"
								disabled={discountMutation.isPending}
								onclick={handleDiscountRedeem}
							>
								{settings_billing_saveOffer_discountCta()}
							</button>
						</div>
					{/if}
				{/each}
			</div>
		{:else}
			<div class="space-y-4">
				<h3 class="text-primary text-lg font-semibold">
					{settings_billing_cancelModal_confirmHeading()}
				</h3>
				{#if confirmedPeriodEnd}
					<p class="text-secondary text-sm">
						{settings_billing_cancelModal_doneSummary({
							periodEnd: fmtDate(confirmedPeriodEnd)
						})}
					</p>
				{:else}
					<p class="text-secondary text-sm">
						{settings_billing_cancelModal_confirmDisclosure({
							periodEnd: 'the end of your current billing cycle'
						})}
					</p>
				{/if}
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<div class="modal-footer flex justify-between gap-2">
			{#if currentStep === 1}
				<button type="button" class="btn-secondary" onclick={handleClose}>
					{settings_billing_cancelModal_keepSubscription()}
				</button>
				<button
					type="button"
					class="btn-primary"
					disabled={!selectedReason}
					onclick={goToStep2OrSkip}
				>
					{settings_billing_cancelModal_continueCancel()}
				</button>
			{:else if currentStep === 2}
				<button type="button" class="btn-secondary" onclick={() => (currentStep = 1)}>
					{common_back()}
				</button>
				<button type="button" class="btn-tertiary" onclick={() => (currentStep = 3)}>
					{settings_billing_cancelModal_continueCancel()}
				</button>
			{:else if confirmedPeriodEnd}
				<button type="button" class="btn-primary ml-auto" onclick={handleClose}>
					{settings_billing_cancelModal_done()}
				</button>
			{:else}
				<button type="button" class="btn-secondary" onclick={goBackFromStep3}>
					{common_back()}
				</button>
				<button
					type="button"
					class="btn-danger"
					disabled={cancelMutation.isPending}
					onclick={handleConfirmCancel}
				>
					{settings_billing_cancelModal_confirmCta()}
				</button>
			{/if}
		</div>
	{/snippet}
</GenericModal>
