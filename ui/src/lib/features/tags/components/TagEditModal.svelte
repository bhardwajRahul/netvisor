<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Tag } from '../types/base';
	import { createDefaultTag } from '../types/base';
	import { createColorHelper, AVAILABLE_COLORS } from '$lib/shared/utils/styling';
	import { TagIcon } from 'lucide-svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';

	let {
		tag = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null
	}: {
		tag?: Tag | null;
		isOpen?: boolean;
		onCreate: (data: Tag) => Promise<void> | void;
		onUpdate: (id: string, data: Tag) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
	} = $props();

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(tag !== null);
	let title = $derived(isEditing ? `Edit ${tag?.name}` : 'Create Tag');
	let saveLabel = $derived(isEditing ? 'Update Tag' : 'Create Tag');

	function getDefaultValues(): Tag {
		if (tag) return { ...tag };
		if (organization) return createDefaultTag(organization.id);
		return createDefaultTag('');
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: createDefaultTag(''),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError('Could not load organization');
				onClose();
				return;
			}

			const tagData: Tag = {
				...(value as Tag),
				name: value.name.trim(),
				description: value.description?.trim() || null,
				organization_id: organization.id
			};

			loading = true;
			try {
				if (isEditing && tag) {
					await onUpdate(tag.id, tagData);
				} else {
					await onCreate(tagData);
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
		if (onDelete && tag) {
			deleting = true;
			try {
				await onDelete(tag.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = $derived(createColorHelper(form.state.values.color));
</script>

<GenericModal {isOpen} {title} size="xl" onClose={onClose} onOpen={handleOpen} showCloseButton={true}>
	<svelte:fragment slot="header-icon">
		<ModalHeaderIcon Icon={TagIcon} color={colorHelper.color} />
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
				<!-- Tag Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">Tag Details</h3>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label="Tag Name"
								id="name"
								{field}
								placeholder="e.g., Production, Critical, Staging"
								required
							/>
						{/snippet}
					</form.Field>

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
								placeholder="Describe what this tag represents..."
							/>
						{/snippet}
					</form.Field>

					<!-- Color Selector -->
					<form.Field name="color">
						{#snippet children(field)}
							<div class="space-y-3">
								<div class="text-secondary block text-sm font-medium">Color</div>
								<div class="grid grid-cols-7 gap-2">
									{#each AVAILABLE_COLORS as color (color)}
										{@const ch = createColorHelper(color)}
										<button
											type="button"
											onclick={() => field.handleChange(color)}
											class="group relative aspect-square w-full rounded-lg border-2 transition-all hover:scale-110"
											class:border-gray-500={field.state.value !== color}
											class:border-white={field.state.value === color}
											class:ring-2={field.state.value === color}
											class:ring-white={field.state.value === color}
											style="background-color: {ch.rgb};"
											title={color}
										></button>
									{/each}
								</div>
							</div>
						{/snippet}
					</form.Field>
				</div>

				{#if isEditing && tag}
					<EntityMetadataSection entities={[tag]} />
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
