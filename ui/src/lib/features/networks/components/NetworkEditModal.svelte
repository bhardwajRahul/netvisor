<script lang="ts">
	import { tick } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Network } from '../types';
	import { createEmptyNetworkFormData } from '../queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import RadioGroup from '$lib/shared/components/forms/input/RadioGroup.svelte';
	import { useSnmpCredentialsQuery } from '$lib/features/snmp/queries';
	import BetaTag from '$lib/shared/components/data/BetaTag.svelte';
	import { SnmpCredentialDisplay } from '$lib/shared/components/forms/selection/display/SnmpCredentialDisplay.svelte';
	import {
		common_betaSnmpExplainer,
		common_cancel,
		common_couldNotLoadUser,
		common_create,
		common_delete,
		common_deleting,
		common_details,
		common_editName,
		common_name,
		common_saving,
		common_update,
		networks_createNetwork,
		networks_networkNamePlaceholder
	} from '$lib/paraglide/messages';

	let {
		network = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null
	}: {
		network?: Network | null;
		isOpen?: boolean;
		onCreate: (data: Network) => Promise<void> | void;
		onUpdate: (id: string, data: Network) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
	} = $props();

	// TanStack Query for organization and current user
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

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(network !== null);
	let title = $derived(
		isEditing ? common_editName({ name: network?.name ?? '' }) : networks_createNetwork()
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	function getDefaultValues() {
		return network
			? { ...network, seedData: false }
			: { ...createEmptyNetworkFormData(), seedData: true };
	}

	// Create form with additional snmp_mode field for UI
	const form = createForm(() => ({
		defaultValues: {
			...createEmptyNetworkFormData(),
			seedData: true,
			snmp_mode: 'none' as 'none' | 'custom'
		},
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadUser());
				handleClose();
				return;
			}

			const networkData: Network = {
				...(value as Network),
				name: value.name.trim(),
				organization_id: organization.id
			};

			loading = true;
			try {
				if (isEditing && network) {
					await onUpdate(network.id, networkData);
				} else {
					await onCreate(networkData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Local state for snmp_mode to enable Svelte 5 reactivity
	let snmpMode = $state<'none' | 'custom'>('none');
	let previousSnmpMode = $state<'none' | 'custom'>('none');
	let isInitialized = $state(false);
	// Key to force form.Field components to re-mount on modal open
	let formKey = $state(0);

	// Sync snmp mode from form store and handle mode changes (after initialization)
	$effect(() => {
		return form.store.subscribe(() => {
			// Skip until modal has been opened and initialized
			if (!isInitialized) return;

			const newMode = (form.state.values as { snmp_mode?: string }).snmp_mode as 'none' | 'custom';
			if (newMode !== previousSnmpMode) {
				previousSnmpMode = newMode;
				snmpMode = newMode;
				// Update snmp_credential_id based on mode change
				if (newMode === 'none') {
					form.setFieldValue('snmp_credential_id', null);
				} else if (snmpCredentials.length > 0 && !form.state.values.snmp_credential_id) {
					form.setFieldValue('snmp_credential_id', snmpCredentials[0].id);
				}
			}
		});
	});

	// Reset form when modal opens
	async function handleOpen() {
		const defaults = getDefaultValues();
		// Check for both null and undefined (API might return either)
		const hasCredential = defaults.snmp_credential_id != null;
		const mode = hasCredential ? 'custom' : 'none';

		// Set local state first
		snmpMode = mode;
		previousSnmpMode = mode;

		// Reset form with all values including snmp_mode
		form.reset({
			...defaults,
			snmp_mode: mode
		});

		// Explicitly set the field value after reset to ensure it takes effect
		form.setFieldValue('snmp_mode', mode);

		// Wait for Svelte to process state updates
		await tick();

		// Increment formKey to force form.Field components to re-mount with fresh state
		formKey++;

		// Mark as initialized so the effect starts handling subsequent changes
		isInitialized = true;
	}

	function handleClose() {
		isInitialized = false;
		onClose();
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && network) {
			deleting = true;
			try {
				await onDelete(network.id);
			} finally {
				deleting = false;
			}
		}
	}

	// SNMP mode options
	const snmpModeOptions = [
		{ value: 'none', label: 'No SNMP (disabled)' },
		{ value: 'custom', label: 'Select credential' }
	];

	let colorHelper = entities.getColorHelper('Network');
</script>

<GenericModal
	{isOpen}
	{title}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Network')} color={colorHelper.color} />
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
			<div class="space-y-8">
				<!-- Network Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">{common_details()}</h3>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={common_name()}
								id="name"
								{field}
								placeholder={networks_networkNamePlaceholder()}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field name="tags">
						{#snippet children(field)}
							<TagPicker
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>

					{#key formKey}
						<div class="space-y-2">
							<label class="text-primary flex items-center gap-2 text-sm font-medium">
								Default SNMP Credential
								<BetaTag tooltip={common_betaSnmpExplainer()} />
							</label>
							<form.Field name="snmp_mode">
								{#snippet children(field)}
									<RadioGroup
										label=""
										id="snmp_mode"
										{field}
										options={snmpModeOptions}
										disabled={isNonOwnerInDemo}
									/>
								{/snippet}
							</form.Field>
						</div>
					{/key}

					{#if snmpMode === 'custom'}
						<form.Field name="snmp_credential_id">
							{#snippet children(field)}
								<RichSelect
									label="Select Credential"
									required={false}
									selectedValue={field.state.value}
									options={snmpCredentials}
									displayComponent={SnmpCredentialDisplay}
									onSelect={(id) => field.handleChange(id)}
									disabled={isNonOwnerInDemo}
								/>
							{/snippet}
						</form.Field>
					{/if}

					<p class="text-muted mt-1 text-xs">
						{#if isNonOwnerInDemo}
							SNMP settings are read-only in demo mode.
						{:else}
							Setting a credential enables SNMP discovery for this network. Hosts can override.
						{/if}
					</p>
				</div>

				{#if isEditing && network}
					<EntityMetadataSection entities={[network]} />
				{/if}
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? common_deleting() : common_delete()}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<button
						type="button"
						disabled={loading || deleting}
						onclick={handleClose}
						class="btn-secondary"
					>
						{common_cancel()}
					</button>
					<button type="submit" disabled={loading || deleting} class="btn-primary">
						{loading ? common_saving() : saveLabel}
					</button>
				</div>
			</div>
		</div>
	</form>
</GenericModal>
