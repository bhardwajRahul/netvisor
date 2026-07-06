<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities, permissions } from '$lib/shared/stores/metadata';
	import { entityRef } from '$lib/shared/components/data/types';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { Edit, Trash2 } from 'lucide-svelte';
	import type { UserApiKey } from '../queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { common_tags } from '$lib/paraglide/messages';

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

	// Get network items for interactive tags
	let networkItems = $derived(
		(apiKey.network_ids ?? [])
			.map((id) => {
				const network = networksData.find((n) => n.id === id);
				if (!network) return null;
				return {
					id: network.id,
					label: network.name,
					color: entities.getColorHelper('Network').color,
					entityRef: entityRef('Network', network.id, network)
				};
			})
			.filter((item): item is NonNullable<typeof item> => item !== null)
	);

	// Build card data
	let cardData = $derived({
		title: apiKey.name,
		iconColor: entities.getColorHelper('UserApiKey').icon,
		Icon: entities.getIconComponent('UserApiKey'),
		fields: [
			{
				label: 'Permissions',
				value: [
					{
						id: apiKey.id,
						label: permissions.getName(apiKey.permissions ?? null),
						color: permissions.getColorHelper(apiKey.permissions ?? null).color
					}
				]
			},
			{
				label: 'Networks',
				value: networkItems.length > 0 ? networkItems : 'All networks'
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
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline
			selectedTagIds={apiKey.tags ?? []}
			entityId={apiKey.id}
			entityType="UserApiKey"
		/>
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
