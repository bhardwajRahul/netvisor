<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { subnetTypes } from '$lib/shared/stores/metadata';
	import { isContainerSubnet } from '../queries';
	import type { Subnet } from '../types/base';
	import type { EntityColumn } from '$lib/shared/components/data/table/columns';
	import { common_delete, common_edit } from '$lib/paraglide/messages';

	let {
		subnet,
		columns,
		onEdit,
		onDelete,
		selected,
		onSelectionChange = () => {}
	}: {
		subnet: Subnet;
		/** The shared field definition, from the tab — the same list the table renders. */
		columns: EntityColumn<Subnet>[];
		onEdit?: (subnet: Subnet) => void;
		onDelete?: (subnet: Subnet) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Build card data
	let cardData = $derived({
		title: subnet.name,
		subtitle: isContainerSubnet(subnet) ? '' : subnet.cidr,
		iconColor: subnetTypes.getColorHelper(subnet.subnet_type).icon,
		Icon: subnetTypes.getIconComponent(subnet.subnet_type),
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

<GenericCard {...cardData} {columns} item={subnet} {selected} {onSelectionChange} />
