<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities, permissions } from '$lib/shared/stores/metadata';
	import type { Network } from '../types';
	import type { EntityColumn } from '$lib/shared/components/data/table/columns';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { common_delete, common_edit } from '$lib/paraglide/messages';

	interface Props {
		network: Network;
		/** The shared field definition, from the tab — the same list the table renders. */
		columns: EntityColumn<Network>[];
		onDelete?: (network: Network) => void;
		onEdit?: (network: Network) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	}

	let {
		network,
		columns,
		onDelete = () => {},
		onEdit = () => {},
		selected,
		onSelectionChange = () => {}
	}: Props = $props();

	// TanStack Query hooks
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Derived data from queries

	let canManageNetworks = $derived(
		(currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities) || false
	);

	// Build card data
	let cardData = $derived({
		title: network.name,
		iconColor: entities.getColorHelper('Network').icon,
		Icon: entities.getIconComponent('Network'),
		actions: [
			...(canManageNetworks
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(network)
						},
						{
							label: common_edit(),
							icon: Edit,
							onClick: () => onEdit(network)
						}
					]
				: [])
		]
	});
</script>

<GenericCard
	{...cardData}
	{columns}
	item={network}
	{selected}
	{onSelectionChange}
	selectable={canManageNetworks}
/>
