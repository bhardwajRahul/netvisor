<script lang="ts">
	import { Copy, Edit, ExternalLink, Trash2, Check, Link } from 'lucide-svelte';
	import type { Share } from '../types/base';
	import { generateShareUrl } from '../queries';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { useTopologiesQuery } from '$lib/features/topology/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { entities } from '$lib/shared/stores/metadata';
	import { entityRef } from '$lib/shared/components/data/types';
	import {
		common_copied,
		common_delete,
		common_disabled,
		common_edit,
		common_enabled,
		common_expires,
		common_network,
		common_never,
		common_open,
		common_status,
		common_topology,
		common_unknownNetwork,
		shares_allowedDomains,
		shares_copyUrl,
		shares_unknownTopology
	} from '$lib/paraglide/messages';

	let {
		share,
		onEdit,
		onDelete,
		viewMode = 'card',
		selected = false,
		onSelectionChange = () => {}
	}: {
		share: Share;
		onEdit?: (share: Share) => void;
		onDelete?: (share: Share) => void;
		viewMode?: 'card' | 'list';
		selected?: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	// Queries
	const topologiesQuery = useTopologiesQuery();
	const networksQuery = useNetworksQuery();
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);

	let copied = $state(false);

	function getUrl(): string {
		return generateShareUrl(share.id);
	}

	async function copyUrl() {
		await navigator.clipboard.writeText(getUrl());
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}

	function openUrl() {
		window.open(getUrl(), '_blank');
	}

	function formatExpiry(date: string | null): string {
		if (!date) return common_never();
		const d = new Date(date);
		return d.toLocaleDateString();
	}

	let cardData = $derived.by(() => {
		const topology = topologiesData.find((t) => t.id === share.topology_id);
		const network = networksData.find((n) => n.id === share.network_id);
		// Topology rows no longer carry a name (the field was dropped along with
		// the rest of the legacy entity-blob columns). Fall back to the network
		// name, which is what the live-view topology represents.
		const topologyLabel = topology
			? (network?.name ?? shares_unknownTopology())
			: shares_unknownTopology();

		return {
			title: share.name,
			iconColor: entities.getColorHelper('Share').icon,
			Icon: Link,
			fields: [
				{
					label: common_topology(),
					value: topology
						? [
								{
									id: topology.id,
									label: topologyLabel,
									color: entities.getColorHelper('Topology').color,
									entityRef: entityRef('Topology', topology.id, topology)
								}
							]
						: shares_unknownTopology()
				},
				{
					label: common_network(),
					value: network
						? [
								{
									id: network.id,
									label: network.name,
									color: entities.getColorHelper('Network').color,
									entityRef: entityRef('Network', network.id, network)
								}
							]
						: common_unknownNetwork()
				},
				{
					label: common_status(),
					value: share.is_enabled ? common_enabled() : common_disabled()
				},
				{
					label: common_expires(),
					value: formatExpiry(share.expires_at)
				},
				...(share.allowed_domains && share.allowed_domains.length > 0
					? [{ label: shares_allowedDomains(), value: share.allowed_domains.join(', ') }]
					: [])
			],
			actions: [
				...(onDelete
					? [
							{
								label: common_delete(),
								icon: Trash2,
								class: 'btn-icon-danger',
								onClick: () => onDelete(share)
							}
						]
					: []),
				{
					label: copied ? common_copied() : shares_copyUrl(),
					icon: copied ? Check : Copy,
					class: copied ? 'text-green-400' : '',
					onClick: copyUrl
				},
				{
					label: common_open(),
					icon: ExternalLink,
					onClick: openUrl
				},
				...(onEdit ? [{ label: common_edit(), icon: Edit, onClick: () => onEdit(share) }] : [])
			]
		};
	});
</script>

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
