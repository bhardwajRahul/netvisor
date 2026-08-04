<script lang="ts">
	import type { EntityColumn } from '$lib/shared/components/data/table/columns';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import type { Vlan } from '../types/base';
	import { vlans_vlanNumber } from '$lib/paraglide/messages';

	let {
		vlan,
		columns,
		selected,
		onSelectionChange = () => {}
	}: {
		vlan: Vlan;
		/** The shared field definition, from the tab — the same list the table renders. */
		columns: EntityColumn<Vlan>[];
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// No actions: VLANs are discovery-populated and this tab is view-only.
	let cardData = $derived({
		title: vlan.name,
		subtitle: `${vlans_vlanNumber()} ${vlan.vlan_number}`,
		iconColor: entities.getColorHelper('Vlan').icon,
		Icon: entities.getIconComponent('Vlan')
	});
</script>

<GenericCard {...cardData} {columns} item={vlan} {selected} {onSelectionChange} />
