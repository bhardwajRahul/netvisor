<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { Edit, Play, Trash2 } from 'lucide-svelte';
	import type { Discovery } from '../../types/base';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { parseCronToHours } from '../../queries';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';

	// Queries
	const daemonsQuery = useDaemonsQuery();

	// Derived data
	let daemonsData = $derived(daemonsQuery.data ?? []);

	let {
		viewMode,
		discovery,
		onEdit,
		onDelete,
		onRun,
		selected,
		onSelectionChange = () => {}
	}: {
		viewMode: 'card' | 'list';
		discovery: Discovery;
		onEdit?: (discovery: Discovery) => void;
		onDelete?: (discovery: Discovery) => void;
		onRun?: (discovery: Discovery) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	let cardData = $derived({
		title: discovery.name,
		iconColor: entities.getColorHelper('Discovery').icon,
		Icon: entities.getIconComponent('Discovery'),
		fields: [
			{
				label: 'Daemon',
				value: daemonsData.find((d) => d.id == discovery.daemon_id)?.name || 'Unknown Daemon'
			},
			{
				label: 'Type',
				value: discovery.discovery_type.type
			},
			{
				label: 'Schedule',
				value:
					discovery.run_type.type == 'Scheduled'
						? `Every ${parseCronToHours(discovery.run_type.cron_schedule) || 'Unknown'} Hours`
						: 'Manual'
			},
			{
				label: 'Last Run',
				value:
					discovery.run_type.type != 'Historical' && discovery.run_type.last_run
						? formatTimestamp(discovery.run_type.last_run)
						: 'Never'
			},
			{ label: 'Tags', snippet: tagsSnippet }
		],
		actions: [
			...(onDelete
				? [{ label: 'Delete', icon: Trash2, class: `btn-icon`, onClick: () => onDelete(discovery) }]
				: []),
			...(onRun
				? [{ label: 'Run', icon: Play, class: `btn-icon`, onClick: () => onRun(discovery) }]
				: []),
			...(onEdit
				? [{ label: 'Edit', icon: Edit, class: `btn-icon`, onClick: () => onEdit(discovery) }]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">Tags:</span>
		<TagPickerInline
			selectedTagIds={discovery.tags}
			entityId={discovery.id}
			entityType="Discovery"
		/>
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
