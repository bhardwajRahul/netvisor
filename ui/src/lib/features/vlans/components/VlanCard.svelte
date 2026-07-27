<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import type { CardFieldItem } from '$lib/shared/components/data/types';
	import type { Vlan } from '../types/base';
	import { sourceLabel } from '../source';
	import {
		common_description,
		common_none,
		common_source,
		common_subnets,
		vlans_vlanNumber
	} from '$lib/paraglide/messages';

	let {
		vlan,
		subnetNames,
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		vlan: Vlan;
		subnetNames: (vlan: Vlan) => string[];
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	let subnetItems = $derived<CardFieldItem[]>(
		subnetNames(vlan).map((name, i) => ({ id: `${vlan.id}-subnet-${i}`, label: name }))
	);

	// No actions: VLANs are discovery-populated and this tab is view-only.
	let cardData = $derived({
		title: vlan.name,
		subtitle: `${vlans_vlanNumber()} ${vlan.vlan_number}`,
		iconColor: entities.getColorHelper('Vlan').icon,
		Icon: entities.getIconComponent('Vlan'),
		fields: [
			{
				label: common_description(),
				value: vlan.description
			},
			{
				label: common_source(),
				value: [
					{
						id: 'source',
						label: sourceLabel(vlan.source),
						color: entities.getColorString('Vlan')
					}
				]
			},
			{
				label: common_subnets(),
				value: subnetItems,
				color: entities.getColorString('Subnet'),
				emptyText: common_none()
			}
		]
	});
</script>

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
