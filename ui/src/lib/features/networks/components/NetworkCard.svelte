<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities, permissions } from '$lib/shared/stores/metadata';
	import type { Network } from '../types';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useGroupsQuery } from '$lib/features/groups/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import {
		common_daemons,
		common_delete,
		common_edit,
		common_groupsLabel,
		common_snmpCredential,
		common_subnets,
		common_tags
	} from '$lib/paraglide/messages';
	import { useSnmpCredentialsQuery } from '$lib/features/snmp/queries';
	import { uuidv4Sentinel } from '$lib/shared/utils/formatting';
	import { toColor } from '$lib/shared/utils/styling';

	interface Props {
		network: Network;
		onDelete?: (network: Network) => void;
		onEdit?: (network: Network) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	}

	let {
		network,
		onDelete = () => {},
		onEdit = () => {},
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: Props = $props();

	// TanStack Query hooks
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const daemonsQuery = useDaemonsQuery();
	const subnetsQuery = useSubnetsQuery();
	const groupsQuery = useGroupsQuery();

	// Derived data from queries
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let groupsData = $derived(groupsQuery.data ?? []);

	let networkDaemons = $derived(daemonsData.filter((d) => d.network_id == network.id));
	let networkSubnets = $derived(subnetsData.filter((s) => s.network_id == network.id));
	let networkGroups = $derived(groupsData.filter((g) => g.network_id == network.id));

	// Use the list query and find by ID (queries inside $derived don't work correctly)
	const snmpCredentialsQuery = useSnmpCredentialsQuery();
	let snmpCredentialsData = $derived(snmpCredentialsQuery.data ?? []);
	let snmpCredential = $derived(
		network.snmp_credential_id
			? (snmpCredentialsData.find((c) => c.id === network.snmp_credential_id) ?? null)
			: null
	);

	let canManageNetworks = $derived(
		(currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities) || false
	);

	// Build card data
	let cardData = $derived({
		title: network.name,
		iconColor: entities.getColorHelper('Network').icon,
		Icon: entities.getIconComponent('Network'),
		fields: [
			{
				label: common_daemons(),
				value: networkDaemons.map((d) => {
					return {
						id: d.id,
						label: d.name,
						color: entities.getColorHelper('Daemon').color
					};
				})
			},
			{
				label: common_snmpCredential(),
				value: snmpCredential
					? [
							{
								id: snmpCredential.id,
								label: snmpCredential.name,
								color: entities.getColorHelper('SnmpCredential').color
							}
						]
					: [
							{
								id: uuidv4Sentinel,
								label: 'None',
								color: toColor('Gray')
							}
						]
			},
			{
				label: common_subnets(),
				value: networkSubnets.map((s) => {
					return {
						id: s.id,
						label: s.name,
						color: entities.getColorHelper('Subnet').color
					};
				})
			},
			{
				label: common_groupsLabel(),
				value: networkGroups.map((g) => {
					return {
						id: g.id,
						label: g.name,
						color: entities.getColorHelper('Group').color
					};
				})
			},
			{ label: common_tags(), snippet: tagsSnippet }
		],

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

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline selectedTagIds={network.tags} entityId={network.id} entityType="Network" />
	</div>
{/snippet}

<GenericCard
	{...cardData}
	{viewMode}
	{selected}
	{onSelectionChange}
	selectable={canManageNetworks}
/>
