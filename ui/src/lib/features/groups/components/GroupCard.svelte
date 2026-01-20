<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { Group } from '../types/base';
	import { entities, groupTypes } from '$lib/shared/stores/metadata';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { toColor } from '$lib/shared/utils/styling';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import {
		common_color,
		common_delete,
		common_description,
		common_edit,
		common_noTypeSpecified,
		common_services,
		common_tags,
		groups_edgeStyleLabel,
		groups_groupType,
		groups_loadingServices,
		groups_noServicesInGroup
	} from '$lib/paraglide/messages';

	// Queries
	const servicesQuery = useServicesCacheQuery();

	// Derived data
	let servicesData = $derived(servicesQuery.data ?? []);
	let isServicesLoading = $derived(servicesQuery.isLoading);

	let {
		group,
		onEdit,
		onDelete,
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		group: Group;
		onEdit?: (group: Group) => void;
		onDelete?: (group: Group) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Get services for this group via binding_ids
	// Using $derived.by() for proper reactivity with complex computation
	let groupServices = $derived.by(() => {
		if (group.group_type !== 'RequestPath' && group.group_type !== 'HubAndSpoke') {
			return [];
		}
		if (servicesData.length === 0 || group.binding_ids.length === 0) {
			return [];
		}
		// Build a map of binding.id -> service for lookup
		const serviceMap = new Map(servicesData.flatMap((s) => s.bindings.map((b) => [b.id, s])));

		return group.binding_ids
			.map((bindingId) => serviceMap.get(bindingId))
			.filter((s): s is NonNullable<typeof s> => s !== null && s !== undefined);
	});

	let groupServiceLabels = $derived(
		groupServices.map((s) => {
			const def = serviceDefinitions.getItem(s.service_definition);
			return {
				id: s.id,
				label: def ? `${s.name} (${def.name})` : s.name
			};
		})
	);

	// Build card data
	let cardData = $derived({
		title: group.name,
		iconColor: groupTypes.getColorHelper(group.group_type).icon,
		Icon: groupTypes.getIconComponent(group.group_type),
		fields: [
			{
				label: common_description(),
				value: group.description
			},
			{
				label: groups_groupType(),
				value: [
					{
						id: 'type',
						label: groupTypes.getName(group.group_type),
						color: groupTypes.getColorString(group.group_type)
					}
				],
				emptyText: common_noTypeSpecified()
			},
			{
				label: common_color(),
				value: [
					{
						id: 'color',
						label: group.color.charAt(0).toUpperCase() + group.color.slice(1),
						color: group.color
					}
				],
				emptyText: common_noTypeSpecified()
			},
			{
				label: groups_edgeStyleLabel(),
				value: [
					{
						id: 'type',
						label: group.edge_style,
						color: toColor('gray')
					}
				],
				emptyText: common_noTypeSpecified()
			},
			{
				label: common_services(),
				value: groupServiceLabels.map(({ id, label }, i) => {
					return {
						id: id + i,
						label,
						color: entities.getColorString('Service')
					};
				}),
				emptyText: isServicesLoading ? groups_loadingServices() : groups_noServicesInGroup()
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
							onClick: () => onDelete(group)
						}
					]
				: []),
			...(onEdit ? [{ label: common_edit(), icon: Edit, onClick: () => onEdit(group) }] : [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline selectedTagIds={group.tags} entityId={group.id} entityType="Group" />
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
