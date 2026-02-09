<script lang="ts">
	import { createForm, type AnyFieldApi } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { type UseCase, type SetupRequest, USE_CASES } from '../../types/base';
	import { required, max, min } from '$lib/shared/components/forms/validators';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		auth_scanopyLogo,
		common_continue,
		common_settingUp,
		common_version,
		onboarding_mspNetworkHelp,
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

	// Track SNMP enabled state
	let snmpEnabled = $state(storeState.network.snmp_enabled ?? false);

	function getDefaultValues() {
		const storedNetwork = storeState.network;
		return {
			organizationName: storeState.organizationName || '',
			network: storedNetwork.name || '',
			snmp_enabled: storedNetwork.snmp_enabled ?? false,
			snmp_version: storedNetwork.snmp_version ?? 'V2c',
			snmp_community: storedNetwork.snmp_community ?? ''
		};
	}

	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			const formValues = value as Record<string, string | boolean>;
			const name = (formValues.network as string)?.trim();
			const network = {
				name,
				snmp_enabled: snmpEnabled,
				snmp_version: snmpEnabled ? (formValues.snmp_version as string) : undefined,
				snmp_community: snmpEnabled ? (formValues.snmp_community as string) : undefined
			};

			const formData: SetupRequest = {
				organization_name: (formValues.organizationName as string).trim(),
				network
			};

			trackEvent('onboarding_org_networks_selected', {
				networks_count: 1,
				snmp_enabled_count: snmpEnabled ? 1 : 0,
				use_case: useCase
			});

			// Update store with final values
			onboardingStore.setOrganizationName(formData.organization_name);
			onboardingStore.setNetwork(formData.network);

			onSubmit(formData);
		}
	}));

	async function handleSubmit() {
		await submitForm(form);
	}

	function handleOpen() {
		snmpEnabled = storeState.network.snmp_enabled ?? false;
		const defaults = getDefaultValues();
		form.reset(defaults);
	}

	function toggleSnmpEnabled(enabled: boolean) {
		snmpEnabled = enabled;
		form.setFieldValue('snmp_enabled' as never, enabled as never);
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
							required={true}
							{field}
						/>
					{/snippet}
				</form.Field>

				<div class="space-y-4">
					<div class="flex items-center gap-2">
						<div class="flex-1">
							<form.Field
								name="network"
								validators={{
									onBlur: ({ value }: { value: string }) => required(value) || min(1)(value)
								}}
							>
								{#snippet children(field: AnyFieldApi)}
									<TextInput
										label={useCaseConfig.networkLabel}
										id="network-0"
										{field}
										required={true}
										placeholder={useCaseConfig.networkPlaceholder}
										helpText={useCase === 'msp' ? onboarding_mspNetworkHelp() : ''}
									/>
								{/snippet}
							</form.Field>
						</div>
					</div>

					<!-- SNMP Configuration -->
					<div class="mt-4">
						<form.Field name="snmp_enabled">
							{#snippet children(field: AnyFieldApi)}
								<div class="flex items-center gap-2">
									<input
										type="checkbox"
										id="snmp-enabled"
										checked={snmpEnabled}
										onchange={(e) => {
											toggleSnmpEnabled(e.currentTarget.checked);
											field.handleChange(e.currentTarget.checked);
										}}
										class="h-4 w-4 rounded border-gray-600 bg-gray-700 text-blue-600 focus:ring-1 focus:ring-blue-500"
									/>
									<label for="snmp-enabled" class="text-secondary flex items-center gap-2 text-sm">
										{snmp_enableForNetwork()}
									</label>
								</div>
							{/snippet}
						</form.Field>

						{#if snmpEnabled}
							<div class="mt-3 space-y-3 pl-6">
								<div class="grid grid-cols-2 gap-3">
									<form.Field name="snmp_version">
										{#snippet children(field: AnyFieldApi)}
											<SelectInput
												label={common_version()}
												id="snmp-version"
												{field}
												options={[
													{ value: 'V2c', label: snmp_versionV2c() },
													{ value: 'V3', label: snmp_versionV3ComingSoon(), disabled: true }
												]}
											/>
										{/snippet}
									</form.Field>

									<form.Field
										name="snmp_community"
										validators={{
											onBlur: ({ value }: { value: string }) =>
												snmpEnabled ? required(value) || max(256)(value) : undefined
										}}
									>
										{#snippet children(field: AnyFieldApi)}
											<TextInput
												label={snmp_communityString()}
												id="snmp-community"
												type="password"
												{field}
												placeholder={snmp_communityStringPlaceholder()}
												required={snmpEnabled}
											/>
										{/snippet}
									</form.Field>
								</div>

								<InlineInfo title={snmp_hostOverrideTitle()} body={snmp_hostOverrideBody()} />
							</div>
						{/if}
					</div>
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
