<script lang="ts">
	import {
		useTagsQuery,
		useCreateTagMutation,
		useUpdateTagMutation,
		useDeleteTagMutation,
		useBulkDeleteTagsMutation
	} from '../queries';
	import TagCard from './TagCard.svelte';
	import TagEditModal from './TagEditModal.svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Tag } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import {
		common_color,
		common_confirmDeleteName,
		common_create,
		common_created,
		common_description,
		common_name,
		common_tags,
		common_updated,
		tags_confirmBulkDelete,
		tags_noTagsHelp,
		tags_noTagsYet,
		tags_subtitle
	} from '$lib/paraglide/messages';

	type TagOrderField = components['schemas']['TagOrderField'];

	let { isReadOnly = false }: TabProps = $props();

	let showTagEditor = $state(false);
	let editingTag: Tag | null = $state(null);

	// Queries and mutations
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const tagsQuery = useTagsQuery();
	const createTagMutation = useCreateTagMutation();
	const updateTagMutation = useUpdateTagMutation();
	const deleteTagMutation = useDeleteTagMutation();
	const bulkDeleteTagsMutation = useBulkDeleteTagsMutation();

	// Derived state
	let tags = $derived(tagsQuery.data ?? []);
	let isLoading = $derived(tagsQuery.isLoading);

	let canManageNetworks = $derived(
		!isReadOnly &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	let allowBulkDelete = $derived(
		!isReadOnly && currentUser
			? permissions.getMetadata(currentUser.permissions).manage_org_entities
			: false
	);

	function handleCreateTag() {
		editingTag = null;
		showTagEditor = true;
	}

	function handleEditTag(tag: Tag) {
		editingTag = tag;
		showTagEditor = true;
	}

	async function handleDeleteTag(tag: Tag) {
		if (confirm(common_confirmDeleteName({ name: tag.name }))) {
			await deleteTagMutation.mutateAsync(tag.id);
		}
	}

	async function handleTagCreate(data: Tag) {
		await createTagMutation.mutateAsync(data);
		showTagEditor = false;
		editingTag = null;
	}

	async function handleTagUpdate(_id: string, data: Tag) {
		await updateTagMutation.mutateAsync(data);
		showTagEditor = false;
		editingTag = null;
	}

	function handleCloseTagEditor() {
		showTagEditor = false;
		editingTag = null;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(tags_confirmBulkDelete({ count: ids.length }))) {
			await bulkDeleteTagsMutation.mutateAsync(ids);
		}
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Tag', {});
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all TagOrderField values are covered
	const tagFields = defineFields<Tag, TagOrderField>(
		{
			name: { label: common_name(), type: 'string', searchable: true },
			color: { label: common_color(), type: 'string', filterable: true },
			created_at: { label: common_created(), type: 'date' },
			updated_at: { label: common_updated(), type: 'date' }
		},
		[{ key: 'description', label: common_description(), type: 'string', searchable: true }]
	);
</script>

<div class="space-y-6">
	<TabHeader title={common_tags()} subtitle={tags_subtitle()}>
		<svelte:fragment slot="actions">
			{#if canManageNetworks}
				<button class="btn-primary flex items-center" onclick={handleCreateTag}>
					<Plus class="h-5 w-5" />{common_create()}
				</button>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if isLoading}
		<Loading />
	{:else if tags.length === 0}
		<EmptyState
			title={tags_noTagsYet()}
			subtitle={tags_noTagsHelp()}
			onClick={handleCreateTag}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={tags}
			fields={tagFields}
			{allowBulkDelete}
			storageKey="scanopy-tags-table-state"
			onBulkDelete={handleBulkDelete}
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Tag,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<TagCard
					tag={item}
					selected={isSelected}
					{onSelectionChange}
					{viewMode}
					onEdit={handleEditTag}
					onDelete={handleDeleteTag}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<TagEditModal
	isOpen={showTagEditor}
	tag={editingTag}
	onCreate={handleTagCreate}
	onUpdate={handleTagUpdate}
	onClose={handleCloseTagEditor}
	onDelete={editingTag ? () => handleDeleteTag(editingTag!) : null}
/>
