<script lang="ts">
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import type { Network } from '$lib/features/networks/types';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import RadioGroup from '$lib/shared/components/forms/input/RadioGroup.svelte';
	import { useSnmpCredentialsQuery } from '$lib/features/snmp/queries';
	import { SnmpCredentialDisplay } from '$lib/shared/components/forms/selection/display/SnmpCredentialDisplay.svelte';
	import BetaTag from '$lib/shared/components/data/BetaTag.svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import {
		common_betaSnmpExplainer,
		common_contact,
		common_location,
		common_none,
		common_snmpCredential,
		common_unknown,
		hosts_snmp_chassisId,
		hosts_snmp_credentialOverride,
		hosts_snmp_demoModeReadOnly,
		hosts_snmp_managementUrl,
		hosts_snmp_sysDescr,
		hosts_snmp_sysObjectId,
		hosts_snmp_systemInfo,
		hosts_snmp_systemInfoHelp,
		hosts_snmp_systemInfoPending,
		hosts_snmp_useNetworkDefault
	} from '$lib/paraglide/messages';

	interface Props {
		formData: HostFormData;
		form: {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			Field: any;
			store: { subscribe: (fn: () => void) => () => void };
			state: { values: Record<string, unknown> };
		};
		isEditing: boolean;
		network?: Network | null;
	}

	let { formData = $bindable(), form, isEditing, network = null }: Props = $props();

	// Local state for credential mode to enable Svelte 5 reactivity
	let credentialMode = $state<'default' | 'override'>('default');
	let previousCredentialMode = $state<'default' | 'override'>('default');

	// Sync credential mode from form store and handle changes
	$effect(() => {
		return form.store.subscribe(() => {
			const newMode = (form.state.values as { credential_mode?: string }).credential_mode as
				| 'default'
				| 'override';
			if (newMode !== previousCredentialMode) {
				previousCredentialMode = newMode;
				credentialMode = newMode;
				// Update snmp_credential_id based on mode change
				if (newMode === 'default') {
					formData.snmp_credential_id = null;
				} else if (snmpCredentials.length > 0 && !formData.snmp_credential_id) {
					formData.snmp_credential_id = snmpCredentials[0].id;
				}
			}
		});
	});

	// TanStack Query for organization and current user (for demo mode check)
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Demo mode check: only Owner can modify SNMP settings in demo orgs
	let isDemoOrg = $derived(organization?.plan?.type === 'Demo');
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	// TanStack Query for SNMP credentials
	const snmpCredentialsQuery = useSnmpCredentialsQuery();
	let snmpCredentials = $derived(snmpCredentialsQuery.data ?? []);

	// Get the network's default credential name for display
	let networkCredentialName = $derived(() => {
		if (!network?.snmp_credential_id) return common_none();
		const cred = snmpCredentials.find((c) => c.id === network.snmp_credential_id);
		return cred?.name ?? common_unknown();
	});

	// Credential mode options
	let credentialModeOptions = $derived([
		{ value: 'default', label: hosts_snmp_useNetworkDefault() + ` (${networkCredentialName()})` },
		{ value: 'override', label: 'Override with specific credential' }
	]);

	// Check if we have SNMP data to display
	let hasSnmpData = $derived(
		formData.sys_descr ||
			formData.sys_object_id ||
			formData.sys_location ||
			formData.sys_contact ||
			formData.chassis_id ||
			formData.management_url
	);
</script>

<div class="space-y-6 p-6">
	<!-- Credential Override Section -->
	<div class="space-y-4">
		<h3 class="text-primary flex items-center gap-2 text-lg font-medium">
			{common_snmpCredential()}
			<BetaTag tooltip={common_betaSnmpExplainer()} />
		</h3>
		<!-- Credential Mode Radio Buttons -->
		<form.Field name="credential_mode">
			{#snippet children(field: AnyFieldApi)}
				<RadioGroup
					label={hosts_snmp_credentialOverride()}
					id="credential_mode"
					{field}
					options={credentialModeOptions}
					disabled={isNonOwnerInDemo}
				/>
			{/snippet}
		</form.Field>

		{#if credentialMode === 'override'}
			<RichSelect
				label="Select Credential"
				required={false}
				selectedValue={formData.snmp_credential_id}
				options={snmpCredentials}
				displayComponent={SnmpCredentialDisplay}
				onSelect={(id) => (formData.snmp_credential_id = id)}
				disabled={isNonOwnerInDemo}
			/>
		{/if}
		{#if isNonOwnerInDemo}
			<p class="text-muted mt-1 text-xs">{hosts_snmp_demoModeReadOnly()}</p>
		{/if}
	</div>

	<!-- SNMP System Information (read-only, only shown when editing with data) -->
	{#if isEditing && hasSnmpData}
		<div class="space-y-4">
			<h3 class="text-primary text-lg font-medium">{hosts_snmp_systemInfo()}</h3>
			<p class="text-muted text-sm">
				{hosts_snmp_systemInfoHelp()}
			</p>

			<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
				{#if formData.sys_descr}
					<div class="bg-tertiary/30 rounded-lg p-4">
						<span class="text-secondary block text-xs font-medium uppercase tracking-wide"
							>{hosts_snmp_sysDescr()}</span
						>
						<p class="text-primary mt-1 break-words text-sm">{formData.sys_descr}</p>
					</div>
				{/if}

				{#if formData.sys_object_id}
					<div class="bg-tertiary/30 rounded-lg p-4">
						<span class="text-secondary block text-xs font-medium uppercase tracking-wide"
							>{hosts_snmp_sysObjectId()}</span
						>
						<p class="text-primary mt-1 font-mono text-sm">{formData.sys_object_id}</p>
					</div>
				{/if}

				{#if formData.sys_location}
					<div class="bg-tertiary/30 rounded-lg p-4">
						<span class="text-secondary block text-xs font-medium uppercase tracking-wide"
							>{common_location()}</span
						>
						<p class="text-primary mt-1 text-sm">{formData.sys_location}</p>
					</div>
				{/if}

				{#if formData.sys_contact}
					<div class="bg-tertiary/30 rounded-lg p-4">
						<span class="text-secondary block text-xs font-medium uppercase tracking-wide"
							>{common_contact()}</span
						>
						<p class="text-primary mt-1 text-sm">{formData.sys_contact}</p>
					</div>
				{/if}

				{#if formData.chassis_id}
					<div class="bg-tertiary/30 rounded-lg p-4">
						<span class="text-secondary block text-xs font-medium uppercase tracking-wide"
							>{hosts_snmp_chassisId()}</span
						>
						<p class="text-primary mt-1 font-mono text-sm">{formData.chassis_id}</p>
					</div>
				{/if}

				{#if formData.management_url}
					<div class="bg-tertiary/30 rounded-lg p-4">
						<span class="text-secondary block text-xs font-medium uppercase tracking-wide"
							>{hosts_snmp_managementUrl()}</span
						>
						<!-- eslint-disable svelte/no-navigation-without-resolve -->
						<a
							href={formData.management_url}
							target="_blank"
							rel="external noopener noreferrer"
							class="mt-1 break-all text-sm text-blue-400 hover:text-blue-300"
						>
							{formData.management_url}
						</a>
						<!-- eslint-enable svelte/no-navigation-without-resolve -->
					</div>
				{/if}
			</div>
		</div>
	{/if}

	{#if !isEditing}
		<div class="bg-tertiary/30 rounded-lg p-4">
			<p class="text-muted text-sm">
				{hosts_snmp_systemInfoPending()}
			</p>
		</div>
	{/if}
</div>
