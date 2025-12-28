<script lang="ts">
	import { createForm, type AnyFieldApi } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { type UseCase, type SetupRequest, USE_CASES } from '../../types/base';
	import { required, max, min } from '$lib/shared/components/forms/validators';
	import { onboardingStore } from '../../stores/onboarding';
	import { Plus, Trash2 } from 'lucide-svelte';
	import { trackEvent } from '$lib/shared/utils/analytics';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onSubmit: (formData: SetupRequest) => void;
		useCase?: UseCase | null;
	}

	let { isOpen = false, onClose, onSubmit, useCase = null }: Props = $props();

	let loading = $state(false);

	// Get use case config (default to company)
	let useCaseConfig = $derived(useCase ? USE_CASES[useCase] : USE_CASES.company);

	// Initialize from store (for back navigation persistence)
	const storeState = onboardingStore.getState();

	// Track network fields dynamically
	let networkCount = $state(
		storeState.networks.length > 0 && storeState.networks.some((n) => n.name)
			? storeState.networks.length
			: 1
	);

	function getDefaultValues() {
		const storedNetworks = storeState.networks;
		const networks: Record<string, string> = {};

		if (storedNetworks.length > 0 && storedNetworks.some((n) => n.name)) {
			storedNetworks.forEach((n, i) => {
				networks[`network_${i}`] = n.name;
			});
		} else {
			networks['network_0'] = '';
		}

		return {
			organizationName: storeState.organizationName || '',
			...networks
		};
	}

	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			const networks: { name: string }[] = [];
			for (let i = 0; i < networkCount; i++) {
				const name = (value as Record<string, string>)[`network_${i}`]?.trim();
				if (name) {
					networks.push({ name });
				}
			}

			const formData: SetupRequest = {
				organization_name: value.organizationName.trim(),
				networks
			};

			trackEvent('onboarding_org_networks_selected', {
				networks_count: networks.length,
				use_case: useCase
			});

			// Update store with final values
			onboardingStore.setOrganizationName(formData.organization_name);
			onboardingStore.setNetworks(formData.networks);

			onSubmit(formData);
		}
	}));

	function addNetwork() {
		const newIndex = networkCount;
		form.setFieldValue(`network_${newIndex}` as never, '' as never);
		networkCount++;
	}

	function removeNetwork(index: number) {
		// Shift all networks after the removed one
		for (let i = index; i < networkCount - 1; i++) {
			const nextValue = form.state.values[`network_${i + 1}` as keyof typeof form.state.values];
			form.setFieldValue(`network_${i}` as never, nextValue as never);
		}
		networkCount--;
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	function handleOpen() {
		form.reset(getDefaultValues());
		networkCount =
			storeState.networks.length > 0 && storeState.networks.some((n) => n.name)
				? storeState.networks.length
				: 1;
	}

	let title = $derived(
		useCase === 'msp'
			? "Let's visualize your customers' networks!"
			: useCase === 'company'
				? "Let's visualize your networks!"
				: "Let's visualize your network!"
	);
</script>

<GenericModal
	{isOpen}
	{title}
	size="lg"
	onClose={onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	<!-- Header icon -->
	<svelte:fragment slot="header-icon">
		<img
			src="https://cdn.jsdelivr.net/gh/scanopy/website@main/static/scanopy-logo.png"
			alt="Scanopy Logo"
			class="h-8 w-8"
		/>
	</svelte:fragment>

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex h-full flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				<form.Field
					name="organizationName"
					validators={{
						onBlur: ({ value }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label={useCaseConfig.orgLabel}
							id="organizationName"
							placeholder={useCaseConfig.orgPlaceholder}
							helpText={useCase === 'homelab' ? '' : 'Your company, team, or project name'}
							required={true}
							{field}
						/>
					{/snippet}
				</form.Field>

				<div class="space-y-3">
					{#each Array(networkCount) as _, index (index)}
						<div class="flex items-center gap-2">
							<div class="flex-1">
								<form.Field
									name={`network_${index}` as never}
									validators={{
										onBlur: ({ value }: { value: string }) =>
											index === 0 ? required(value) || min(1)(value) : min(1)(value)
									}}
								>
									{#snippet children(field: AnyFieldApi)}
										<TextInput
											label={index === 0 ? useCaseConfig.networkLabel : ''}
											id="network-{index}"
											{field}
											required={index == 0}
											placeholder={useCaseConfig.networkPlaceholder}
											helpText={index === 0 && useCase === 'msp'
												? 'Each network represents a customer environment. One customer can have multiple networks.'
												: ''}
										/>
									{/snippet}
								</form.Field>
							</div>
							{#if index > 0}
								<button
									type="button"
									class="btn-icon-danger"
									onclick={() => removeNetwork(index)}
									aria-label="Remove network"
								>
									<Trash2 class="h-4 w-4" />
								</button>
							{/if}
						</div>
					{/each}

					{#if useCase && useCase != 'homelab'}
						<button
							type="button"
							class="text-secondary hover:text-primary flex items-center gap-1 text-sm transition-colors"
							onclick={addNetwork}
						>
							<Plus class="h-4 w-4" />
							Add another network
						</button>
					{/if}
				</div>
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				<button type="submit" disabled={loading} class="btn-primary w-full">
					{loading ? 'Setting up...' : 'Continue'}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
