<script lang="ts">
	import { createForm, type AnyFieldApi } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { type UseCase, type SetupRequest, USE_CASES } from '../../types/base';
	import { required, max, min } from '$lib/shared/components/forms/validators';
	import { onboardingStore } from '../../stores/onboarding';
	import { Plus, Trash2 } from 'lucide-svelte';
	import BetaTag from '$lib/shared/components/data/BetaTag.svelte';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		auth_scanopyLogo,
		common_betaSnmpExplainer,
		common_continue,
		common_settingUp,
		common_version,
		onboarding_addAnotherNetwork,
		onboarding_mspNetworkHelp,
		onboarding_orgHelpText,
		onboarding_removeNetwork,
		onboarding_visualizeCompany,
		onboarding_visualizeHomelab,
		onboarding_visualizeMsp,
		snmp_communityString,
		snmp_communityStringPlaceholder,
		snmp_enableForNetwork,
		snmp_hostOverrideBody,
		snmp_hostOverrideTitle,
		snmp_versionV2c,
		snmp_versionV3ComingSoon
	} from '$lib/paraglide/messages';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';

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

	// Track SNMP enabled state per network (using reactive map)
	let snmpEnabledMap = $state<Record<number, boolean>>({});

	function getDefaultValues() {
		const storedNetworks = storeState.networks;
		const values: Record<string, string | boolean> = {};

		if (storedNetworks.length > 0 && storedNetworks.some((n) => n.name)) {
			storedNetworks.forEach((n, i) => {
				values[`network_${i}`] = n.name;
				values[`snmp_${i}_enabled`] = n.snmp_enabled ?? false;
				values[`snmp_${i}_version`] = n.snmp_version ?? 'V2c';
				values[`snmp_${i}_community`] = n.snmp_community ?? '';
				// Initialize snmpEnabledMap
				snmpEnabledMap[i] = n.snmp_enabled ?? false;
			});
		} else {
			values['network_0'] = '';
			values['snmp_0_enabled'] = false;
			values['snmp_0_version'] = 'V2c';
			values['snmp_0_community'] = '';
			snmpEnabledMap[0] = false;
		}

		return {
			organizationName: storeState.organizationName || '',
			...values
		};
	}

	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			const formValues = value as Record<string, string | boolean>;
			const networks: SetupRequest['networks'] = [];
			for (let i = 0; i < networkCount; i++) {
				const name = (formValues[`network_${i}`] as string)?.trim();
				if (name) {
					const snmpEnabled = snmpEnabledMap[i] ?? false;
					networks.push({
						name,
						snmp_enabled: snmpEnabled,
						snmp_version: snmpEnabled ? (formValues[`snmp_${i}_version`] as string) : undefined,
						snmp_community: snmpEnabled ? (formValues[`snmp_${i}_community`] as string) : undefined
					});
				}
			}

			const formData: SetupRequest = {
				organization_name: (formValues.organizationName as string).trim(),
				networks
			};

			trackEvent('onboarding_org_networks_selected', {
				networks_count: networks.length,
				snmp_enabled_count: networks.filter((n) => n.snmp_enabled).length,
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
		form.setFieldValue(`snmp_${newIndex}_enabled` as never, false as never);
		form.setFieldValue(`snmp_${newIndex}_version` as never, 'V2c' as never);
		form.setFieldValue(`snmp_${newIndex}_community` as never, '' as never);
		snmpEnabledMap[newIndex] = false;
		networkCount++;
	}

	function removeNetwork(index: number) {
		// Shift all networks and their SNMP config after the removed one
		for (let i = index; i < networkCount - 1; i++) {
			const nextValue = form.state.values[`network_${i + 1}` as keyof typeof form.state.values];
			const nextSnmpEnabled =
				form.state.values[`snmp_${i + 1}_enabled` as keyof typeof form.state.values];
			const nextSnmpVersion =
				form.state.values[`snmp_${i + 1}_version` as keyof typeof form.state.values];
			const nextSnmpCommunity =
				form.state.values[`snmp_${i + 1}_community` as keyof typeof form.state.values];

			form.setFieldValue(`network_${i}` as never, nextValue as never);
			form.setFieldValue(`snmp_${i}_enabled` as never, nextSnmpEnabled as never);
			form.setFieldValue(`snmp_${i}_version` as never, nextSnmpVersion as never);
			form.setFieldValue(`snmp_${i}_community` as never, nextSnmpCommunity as never);
			snmpEnabledMap[i] = snmpEnabledMap[i + 1] ?? false;
		}
		delete snmpEnabledMap[networkCount - 1];
		networkCount--;
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	function handleOpen() {
		// Reset SNMP enabled map
		snmpEnabledMap = {};
		const defaults = getDefaultValues();
		form.reset(defaults);
		networkCount =
			storeState.networks.length > 0 && storeState.networks.some((n) => n.name)
				? storeState.networks.length
				: 1;
	}

	function toggleSnmpEnabled(index: number, enabled: boolean) {
		snmpEnabledMap[index] = enabled;
		form.setFieldValue(`snmp_${index}_enabled` as never, enabled as never);
	}

	let title = $derived(
		useCase === 'msp'
			? onboarding_visualizeMsp()
			: useCase === 'company'
				? onboarding_visualizeCompany()
				: onboarding_visualizeHomelab()
	);
</script>

<GenericModal
	{isOpen}
	{title}
	size="lg"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
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
							helpText={useCase === 'homelab' ? '' : onboarding_orgHelpText()}
							required={true}
							{field}
						/>
					{/snippet}
				</form.Field>

				<div class="space-y-4">
					{#each Array.from({ length: networkCount }, (_, i) => i) as index (index)}
						<div class="border-secondary/20 rounded-lg border p-4">
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
													? onboarding_mspNetworkHelp()
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
										aria-label={onboarding_removeNetwork()}
									>
										<Trash2 class="h-4 w-4" />
									</button>
								{/if}
							</div>

							<!-- SNMP Configuration -->
							<div class="mt-4">
								<form.Field name={`snmp_${index}_enabled` as never}>
									{#snippet children(field: AnyFieldApi)}
										<div class="flex items-center gap-2">
											<input
												type="checkbox"
												id="snmp-{index}-enabled"
												checked={snmpEnabledMap[index] ?? false}
												onchange={(e) => {
													toggleSnmpEnabled(index, e.currentTarget.checked);
													field.handleChange(e.currentTarget.checked);
												}}
												class="h-4 w-4 rounded border-gray-600 bg-gray-700 text-blue-600 focus:ring-1 focus:ring-blue-500"
											/>
											<label
												for="snmp-{index}-enabled"
												class="text-secondary flex items-center gap-2 text-sm"
											>
												{snmp_enableForNetwork()}
												<BetaTag tooltip={common_betaSnmpExplainer()} />
											</label>
										</div>
									{/snippet}
								</form.Field>

								{#if snmpEnabledMap[index]}
									<div class="mt-3 space-y-3 pl-6">
										<div class="grid grid-cols-2 gap-3">
											<form.Field name={`snmp_${index}_version` as never}>
												{#snippet children(field: AnyFieldApi)}
													<SelectInput
														label={common_version()}
														id="snmp-{index}-version"
														{field}
														options={[
															{ value: 'V2c', label: snmp_versionV2c() },
															{ value: 'V3', label: snmp_versionV3ComingSoon(), disabled: true }
														]}
													/>
												{/snippet}
											</form.Field>

											<form.Field
												name={`snmp_${index}_community` as never}
												validators={{
													onBlur: ({ value }: { value: string }) =>
														snmpEnabledMap[index] ? required(value) || max(256)(value) : undefined
												}}
											>
												{#snippet children(field: AnyFieldApi)}
													<TextInput
														label={snmp_communityString()}
														id="snmp-{index}-community"
														type="password"
														{field}
														placeholder={snmp_communityStringPlaceholder()}
														required={snmpEnabledMap[index]}
													/>
												{/snippet}
											</form.Field>
										</div>

										<InlineInfo title={snmp_hostOverrideTitle()} body={snmp_hostOverrideBody()} />
									</div>
								{/if}
							</div>
						</div>
					{/each}

					{#if useCase && useCase != 'homelab'}
						<button
							type="button"
							class="text-secondary hover:text-primary flex items-center gap-1 text-sm transition-colors"
							onclick={addNetwork}
						>
							<Plus class="h-4 w-4" />
							{onboarding_addAnotherNetwork()}
						</button>
					{/if}
				</div>
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				<button type="submit" disabled={loading} class="btn-primary w-full">
					{loading ? common_settingUp() : common_continue()}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
