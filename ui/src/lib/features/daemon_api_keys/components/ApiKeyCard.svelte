<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { Edit, Trash2 } from 'lucide-svelte';
	import type { ApiKey } from '../types/base';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';

	let {
		apiKey,
		onDelete = () => {},
		onEdit = () => {},
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		apiKey: ApiKey;
		onDelete?: (apiKey: ApiKey) => void;
		onEdit?: (apiKey: ApiKey) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Build card data
	let cardData = $derived({
		title: apiKey.name,
		iconColor: entities.getColorHelper('DaemonApiKey').icon,
		Icon: entities.getIconComponent('DaemonApiKey'),
		fields: [
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
		<TagPickerInline selectedTagIds={apiKey.tags} entityId={apiKey.id} entityType="DaemonApiKey" />
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
