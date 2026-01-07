<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { Edit, Trash2 } from 'lucide-svelte';
	import type { UserApiKey } from '../queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';

	// Queries

	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	let {
		apiKey,
		onDelete = () => {},
		onEdit = () => {},
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		apiKey: UserApiKey;
		onDelete?: (apiKey: UserApiKey) => void;
		onEdit?: (apiKey: UserApiKey) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Get network names
	let networkNames = $derived(
		(apiKey.network_ids ?? [])
			.map((id) => networksData.find((n) => n.id === id)?.name)
			.filter((name): name is string => !!name)
	);

	// Build card data
	let cardData = $derived({
		title: apiKey.name,
		iconColor: entities.getColorHelper('UserApiKey').icon,
		Icon: entities.getIconComponent('UserApiKey'),
		fields: [
			{
				label: 'Permissions',
				value: apiKey.permissions
			},
			{
				label: 'Networks',
				value: networkNames.length > 0 ? networkNames.join(', ') : 'All networks'
			},
			{
				label: 'Created',
				value: formatTimestamp(apiKey.created_at)
			},
			{
				label: 'Last Used',
				value: apiKey.last_used ? formatTimestamp(apiKey.last_used) : 'Never'
			},
			{
				label: 'Expires',
				value: apiKey.expires_at
					? new Date(apiKey.expires_at) < new Date()
						? 'Expired'
						: formatTimestamp(apiKey.expires_at)
					: 'Never'
			},
			{
				label: 'Enabled',
				value: apiKey.is_enabled ? 'Yes' : 'No'
			},
			{ label: 'Tags', snippet: tagsSnippet }
		],
		actions: [
			{
				label: 'Delete',
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => onDelete(apiKey)
			},
			{
				label: 'Edit',
				icon: Edit,
				class: 'btn-icon',
				onClick: () => onEdit(apiKey)
			}
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">Tags:</span>
		<TagPickerInline
			selectedTagIds={apiKey.tags ?? []}
			entityId={apiKey.id}
			entityType="UserApiKey"
		/>
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
