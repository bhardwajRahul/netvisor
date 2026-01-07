<script lang="ts">
	import { X, Plus } from 'lucide-svelte';
	import {
		useTagsQuery,
		useCreateTagMutation,
		useBulkAddTagMutation,
		useBulkRemoveTagMutation,
		type EntityDiscriminants
	} from '$lib/features/tags/queries';
	import { createDefaultTag } from '$lib/features/tags/types/base';
	import { createColorHelper, AVAILABLE_COLORS, type Color } from '$lib/shared/utils/styling';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';

	/**
	 * Compact inline tag picker for use in cards and bulk actions.
	 * Shows tags as pills with remove buttons, and a + button to add tags.
	 *
	 * Two modes of operation:
	 * 1. **Entity mode**: Provide `entityId` and `entityType` - mutations handled internally
	 * 2. **Callback mode**: Provide `onAdd`/`onRemove` callbacks - for bulk operations or custom handling
	 */
	let {
		selectedTagIds = [],
		disabled = false,
		// Entity mode props - when provided, uses generic tag assignment API
		entityId,
		entityType,
		// Callback mode props - for bulk operations or custom handling
		onAdd,
		onRemove
	}: {
		selectedTagIds?: string[];
		disabled?: boolean;
		entityId?: string;
		entityType?: EntityDiscriminants;
		onAdd?: (tagId: string) => void;
		onRemove?: (tagId: string) => void;
	} = $props();

	// Entity mode: use generic mutations
	const bulkAddTagMutation = useBulkAddTagMutation();
	const bulkRemoveTagMutation = useBulkRemoveTagMutation();

	// Determine if entity mode is enabled
	let isEntityMode = $derived(entityId !== undefined && entityType !== undefined);

	let inputValue = $state('');
	let isDropdownOpen = $state(false);
	let inputElement: HTMLInputElement | undefined = $state();

	// Query and mutation
	const tagsQuery = useTagsQuery();
	const createTagMutation = useCreateTagMutation();
	const organizationQuery = useOrganizationQuery();
	const currentUserQuery = useCurrentUserQuery();

	// Derived state
	let tags = $derived(tagsQuery.data ?? []);
	let isCreating = $derived(createTagMutation.isPending);
	let organization = $derived(organizationQuery.data);
	let currentUser = $derived(currentUserQuery.data);

	// Check if user can create tags
	let canCreateTags = $derived(
		currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	// Check if typed value matches an existing tag name exactly
	let exactMatch = $derived(
		tags.some((t) => t.name.toLowerCase() === inputValue.trim().toLowerCase())
	);

	// Show create option if user typed something, can create, and no exact match exists
	let showCreateOption = $derived(inputValue.trim().length > 0 && canCreateTags && !exactMatch);

	// Get tag by ID
	function getTag(id: string) {
		return tags.find((t) => t.id === id) ?? null;
	}

	// Filter available tags based on input and exclude already selected
	let availableTags = $derived(
		tags.filter(
			(tag) =>
				!selectedTagIds.includes(tag.id) &&
				tag.name.toLowerCase().includes(inputValue.toLowerCase())
		)
	);

	let showDropdown = $derived(isDropdownOpen && (availableTags.length > 0 || showCreateOption));

	function getRandomColor(): Color {
		return AVAILABLE_COLORS[Math.floor(Math.random() * AVAILABLE_COLORS.length)];
	}

	async function handleCreateTag() {
		if (!organization || isCreating) return;
		if (!isEntityMode && !onAdd) return;

		const name = inputValue.trim();
		if (!name) return;

		try {
			const newTag = createDefaultTag(organization.id);
			newTag.name = name;
			newTag.color = getRandomColor();

			const result = await createTagMutation.mutateAsync(newTag);
			await handleAddTag(result.id);
			inputValue = '';
			isDropdownOpen = false;
		} finally {
			inputElement?.focus();
		}
	}

	async function handleAddTag(tagId: string) {
		if (isEntityMode && entityId && entityType) {
			await bulkAddTagMutation.mutateAsync({
				entity_ids: [entityId],
				entity_type: entityType,
				tag_id: tagId
			});
		} else {
			onAdd?.(tagId);
		}
		inputValue = '';
		isDropdownOpen = false;
	}

	async function handleRemoveTag(tagId: string) {
		if (isEntityMode && entityId && entityType) {
			await bulkRemoveTagMutation.mutateAsync({
				entity_ids: [entityId],
				entity_type: entityType,
				tag_id: tagId
			});
		} else {
			onRemove?.(tagId);
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			if (showCreateOption && availableTags.length === 0) {
				handleCreateTag();
			} else if (availableTags.length > 0) {
				handleAddTag(availableTags[0].id);
			} else if (showCreateOption) {
				handleCreateTag();
			}
		} else if (e.key === 'Escape') {
			inputValue = '';
			isDropdownOpen = false;
			inputElement?.blur();
		}
	}

	function handleBlur() {
		// Delay to allow click on dropdown item
		setTimeout(() => {
			isDropdownOpen = false;
		}, 150);
	}

	function handleAddClick() {
		if (disabled) return;
		isDropdownOpen = true;
		// Focus input after dropdown opens
		setTimeout(() => inputElement?.focus(), 0);
	}
</script>

<div class="flex flex-wrap items-center gap-1">
	<!-- Selected tags -->
	{#each selectedTagIds as tagId (tagId)}
		{@const tag = getTag(tagId)}
		{#if tag}
			{@const colorHelper = createColorHelper(tag.color)}
			<span
				class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium {colorHelper.bg} {colorHelper.text}"
			>
				{tag.name}
				{#if !disabled && (onRemove || isEntityMode)}
					<button
						type="button"
						onclick={() => handleRemoveTag(tagId)}
						class="rounded-full p-0.5 transition-colors hover:bg-white/20"
					>
						<X class="h-3 w-3" />
					</button>
				{/if}
			</span>
		{:else}
			<span
				class="inline-flex items-center gap-1 rounded-full bg-gray-600 px-2 py-0.5 text-xs font-medium text-gray-300"
			>
				Unknown
				{#if !disabled && (onRemove || isEntityMode)}
					<button
						type="button"
						onclick={() => handleRemoveTag(tagId)}
						class="rounded-full p-0.5 transition-colors hover:bg-white/20"
					>
						<X class="h-3 w-3" />
					</button>
				{/if}
			</span>
		{/if}
	{/each}

	<!-- Add button / dropdown -->
	{#if (onAdd || isEntityMode) && !disabled}
		<div class="relative flex h-5 items-center">
			{#if isDropdownOpen}
				<!-- Input for searching/creating tags -->
				<input
					bind:this={inputElement}
					bind:value={inputValue}
					type="text"
					placeholder="Add tag..."
					class="h-5 w-24 rounded-full border border-gray-600 bg-gray-700 px-2 text-xs text-gray-200 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
					onfocus={() => (isDropdownOpen = true)}
					onblur={handleBlur}
					onkeydown={handleKeydown}
				/>
			{:else}
				<!-- Add button -->
				<button
					type="button"
					onclick={handleAddClick}
					class="inline-flex h-5 w-5 items-center justify-center rounded-full border border-dashed border-gray-500 text-gray-400 transition-colors hover:border-gray-400 hover:text-gray-300"
				>
					<Plus class="h-3 w-3" />
				</button>
			{/if}

			<!-- Dropdown -->
			{#if showDropdown}
				<div
					class="absolute left-0 top-full z-50 mt-1 max-h-48 min-w-40 overflow-y-auto rounded-md border border-gray-600 bg-gray-700 shadow-lg"
				>
					<!-- Create new tag option -->
					{#if showCreateOption}
						<button
							type="button"
							class="flex w-full items-center gap-2 border-b border-gray-600 px-3 py-2 text-left text-xs transition-colors hover:bg-gray-600"
							onmousedown={handleCreateTag}
							disabled={isCreating}
						>
							<Plus class="h-3 w-3 shrink-0 text-green-400" />
							<span class="text-primary">
								{isCreating ? 'Creating...' : `Create "${inputValue.trim()}"`}
							</span>
						</button>
					{/if}

					<!-- Existing tags -->
					{#each availableTags as tag (tag.id)}
						{@const colorHelper = createColorHelper(tag.color)}
						<button
							type="button"
							class="flex w-full items-center gap-2 px-3 py-2 text-left text-xs transition-colors hover:bg-gray-600"
							onmousedown={() => handleAddTag(tag.id)}
						>
							<span
								class="h-2.5 w-2.5 shrink-0 rounded-full"
								style="background-color: {colorHelper.rgb};"
							></span>
							<span class="text-primary">{tag.name}</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>
