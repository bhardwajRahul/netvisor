<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { entityRef, type CardFieldItem } from '$lib/shared/components/data/types';
	import type { Vlan } from '../types/base';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { formatRelativeTime } from '$lib/shared/utils/formatting';
	import {
		common_description,
		common_lastSeen,
		common_never,
		common_none,
		common_subnets,
		vlans_vlanNumber
	} from '$lib/paraglide/messages';

	let {
		vlan,
		subnets,
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		vlan: Vlan;
		subnets: (vlan: Vlan) => Subnet[];
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Rendered as EntityTags so each subnet links through to the subnet itself,
	// the same way DaemonCard lists a daemon's interfaced subnets.
	let subnetItems = $derived<CardFieldItem[]>(
		subnets(vlan).map((s) => ({
			id: s.id,
			label: s.name,
			color: entities.getColorHelper('Subnet').color,
			entityRef: entityRef('Subnet', s.id, s)
		}))
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
				label: common_subnets(),
				value: subnetItems,
				emptyText: common_none()
			},
			{
				label: common_lastSeen(),
				value: vlan.last_seen_at ? formatRelativeTime(vlan.last_seen_at) : common_never()
			}
		]
	});
</script>

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
