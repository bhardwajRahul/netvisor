<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { subnetTypes } from '$lib/shared/stores/metadata';
	import { isContainerSubnet } from '../queries';
	import type { Subnet } from '../types/base';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { formatRelativeTime } from '$lib/shared/utils/formatting';
	import {
		common_delete,
		common_description,
		common_edit,
		common_lastSeen,
		common_never,
		common_noTypeSpecified,
		common_tags,
		subnets_subnetType
	} from '$lib/paraglide/messages';

	let {
		subnet,
		onEdit,
		onDelete,
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		subnet: Subnet;
		onEdit?: (subnet: Subnet) => void;
		onDelete?: (subnet: Subnet) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Build card data
	let cardData = $derived({
		title: subnet.name,
		subtitle: isContainerSubnet(subnet) ? '' : subnet.cidr,
		iconColor: subnetTypes.getColorHelper(subnet.subnet_type).icon,
		Icon: subnetTypes.getIconComponent(subnet.subnet_type),
		fields: [
			{
				label: common_description(),
				value: subnet.description
			},
			{
				label: subnets_subnetType(),
				value: [
					{
						id: 'type',
						label: subnetTypes.getName(subnet.subnet_type),
						color: subnetTypes.getColorString(subnet.subnet_type)
					}
				],
				emptyText: common_noTypeSpecified()
			},
			{
				label: common_lastSeen(),
				value: subnet.last_seen_at ? formatRelativeTime(subnet.last_seen_at) : common_never()
			},
			{ label: common_tags(), snippet: tagsSnippet }
		],

		actions: [
			...(onDelete
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(subnet)
						}
					]
				: []),
			...(onEdit
				? [
						{
							label: common_edit(),
							icon: Edit,
							onClick: () => onEdit(subnet)
						}
					]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline selectedTagIds={subnet.tags} entityId={subnet.id} entityType="Subnet" />
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
