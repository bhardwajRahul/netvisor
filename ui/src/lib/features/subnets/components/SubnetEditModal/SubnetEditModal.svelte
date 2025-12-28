<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max, cidrNotation } from '$lib/shared/components/forms/validators';
	import { createEmptySubnetFormData, isContainerSubnet } from '../../store';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { entities, subnetTypes } from '$lib/shared/stores/metadata';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Subnet } from '../../types/base';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import { get } from 'svelte/store';
	import { useNetworksQuery } from '$lib/features/networks/queries';

	// TanStack Query hooks
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	interface Props {
		subnet?: Subnet | null;
		isOpen?: boolean;
		onCreate: (data: Subnet) => Promise<void> | void;
		onUpdate: (id: string, data: Subnet) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
	}

	let {
		subnet = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null
	}: Props = $props();

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(subnet !== null);
	let title = $derived(isEditing ? `Edit ${subnet?.name}` : 'Create Subnet');
	let saveLabel = $derived(isEditing ? 'Update Subnet' : 'Create Subnet');

	function getDefaultValues(): Subnet {
		return subnet ? { ...subnet } : createEmptySubnetFormData(defaultNetworkId);
	}

	// Create form with initial empty values - we'll reset it when the modal opens
	const form = createForm(() => ({
		defaultValues: createEmptySubnetFormData(''),
		onSubmit: async ({ value }) => {
			const subnetData: Subnet = {
				...value,
				name: value.name.trim(),
				description: value.description?.trim() || '',
				cidr: value.cidr.trim()
			};

			loading = true;
			try {
				if (isEditing && subnet) {
					await onUpdate(subnet.id, subnetData);
				} else {
					await onCreate(subnetData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
	}

	// CIDR disabled state - use a function to avoid reactive dependency on form.state
	function getIsCidrDisabled(): boolean {
		return !!get(isContainerSubnet(form.state.values.id)) || isEditing;
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && subnet) {
			deleting = true;
			try {
				await onDelete(subnet.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = entities.getColorHelper('Subnet');

	// Prepare subnet type options
	let subnetTypeOptions = $derived(
		subnetTypes.getItems().map((st) => ({
			value: st.id,
			label: st.name ?? st.id
		}))
	);
</script>

<GenericModal {isOpen} {title} size="xl" onClose={onClose} onOpen={handleOpen} showCloseButton={true}>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={entities.getIconComponent('Subnet')} color={colorHelper.color} />
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
			<div class="space-y-8">
				<!-- Subnet Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">Subnet Details</h3>

					<!-- Name Field -->
					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="Name"
								id="name"
								{field}
								placeholder="e.g., Home LAN, VPN Network"
								required
							/>
						{/snippet}
					</form.Field>

					<!-- CIDR Field -->
					<form.Field
						name="cidr"
						validators={{
							onBlur: ({ value }) => required(value) || cidrNotation(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="CIDR"
								id="cidr"
								{field}
								placeholder="192.168.1.0/24"
								disabled={getIsCidrDisabled()}
								helpText="Network address and prefix length (e.g., 192.168.1.0/24)"
								required
							/>
						{/snippet}
					</form.Field>

					<!-- Network Selection -->
					<form.Field name="network_id">
						{#snippet children(field)}
							<SelectNetwork
								selectedNetworkId={field.state.value}
								onNetworkChange={(id) => field.handleChange(id)}
							/>
						{/snippet}
					</form.Field>

					<!-- Subnet Type -->
					<form.Field name="subnet_type">
						{#snippet children(field)}
							<SelectInput
								label="Subnet Type"
								id="subnet_type"
								{field}
								options={subnetTypeOptions}
							/>
						{/snippet}
					</form.Field>

					<!-- Description Field -->
					<form.Field
						name="description"
						validators={{
							onBlur: ({ value }) => max(500)(value || '')
						}}
					>
						{#snippet children(field)}
							<TextArea
								label="Description"
								id="description"
								{field}
								placeholder="Describe the purpose of this subnet..."
								rows={3}
							/>
						{/snippet}
					</form.Field>

					<!-- Tags -->
					<form.Field name="tags">
						{#snippet children(field)}
							<TagPicker
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>
				</div>

				{#if isEditing && subnet}
					<EntityMetadataSection entities={[subnet]} />
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
							{deleting ? 'Deleting...' : 'Delete'}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<button
						type="button"
						disabled={loading || deleting}
						onclick={onClose}
						class="btn-secondary"
					>
						Cancel
					</button>
					<button type="submit" disabled={loading || deleting} class="btn-primary">
						{loading ? 'Saving...' : saveLabel}
					</button>
				</div>
			</div>
		</div>
	</form>
</GenericModal>
