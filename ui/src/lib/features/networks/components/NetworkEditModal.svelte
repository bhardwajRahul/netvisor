<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Network } from '../types';
	import { createEmptyNetworkFormData } from '../store';
	import { pushError } from '$lib/shared/stores/feedback';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';

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

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(network !== null);
	let title = $derived(isEditing ? `Edit ${network?.name}` : 'Create Network');
	let saveLabel = $derived(isEditing ? 'Update Network' : 'Create Network');

	function getDefaultValues() {
		return network
			? { ...network, seedData: false }
			: { ...createEmptyNetworkFormData(), seedData: true };
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: { ...createEmptyNetworkFormData(), seedData: true },
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError('Could not load ID for current user');
				onClose();
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

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
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

	let colorHelper = entities.getColorHelper('Network');
</script>

<GenericModal {isOpen} {title} size="xl" onClose={onClose} onOpen={handleOpen} showCloseButton={true}>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={entities.getIconComponent('Network')} color={colorHelper.color} />
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
				<!-- Network Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">Network Details</h3>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="Network Name"
								id="name"
								{field}
								placeholder="e.g Home Network"
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
				</div>

				{#if !isEditing}
					<form.Field name="seedData">
						{#snippet children(field)}
							<Checkbox
								label="Track services Scanopy can't discover automatically?"
								helpText="Creates subnets for remote hosts and internet services that aren't on your local network (Cloud services, APIs, SaaS tools, etc). Includes example data to show how it works."
								id="seedData"
								{field}
							/>
						{/snippet}
					</form.Field>
				{/if}

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
