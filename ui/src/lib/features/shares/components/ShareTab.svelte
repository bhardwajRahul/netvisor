<script lang="ts">
	import ShareCard from './ShareCard.svelte';
	import type { Share } from '../types/base';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import ShareModal from './ShareModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { useTopologiesQuery } from '$lib/features/topology/queries';
	import { useSharesQuery, useDeleteShareMutation, useBulkDeleteSharesMutation } from '../queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import {
		common_confirmDeleteName,
		common_created,
		common_enabled,
		common_expires,
		common_name,
		common_network,
		common_sharing,
		common_topology,
		common_unknownNetwork,
		shares_confirmBulkDelete,
		shares_noSharesSubtitle,
		shares_noSharesYet,
		shares_unknownTopology
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();

	// Queries
	const organizationQuery = useOrganizationQuery();
	let hasShareViews = $derived.by(() => {
		const org = organizationQuery.data;
		if (!org?.plan) return true;
		return billingPlans.getMetadata(org.plan.type).features.share_views;
	});

	const sharesQuery = useSharesQuery();
	const networksQuery = useNetworksQuery();
	const topologiesQuery = useTopologiesQuery();

	// Mutations
	const deleteShareMutation = useDeleteShareMutation();
	const bulkDeleteSharesMutation = useBulkDeleteSharesMutation();

	// Derived data
	let sharesData = $derived(sharesQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let isLoading = $derived(sharesQuery.isPending);

	let showEditor = $state(false);
	let editingShare = $state<Share | null>(null);

	// Define field configuration for DataControls
	const shareFields: FieldConfig<Share>[] = [
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true
		},
		{
			key: 'topology_id',
			label: common_topology(),
			type: 'string',
			searchable: true,
			filterable: true,
			getValue: (share) => {
				return (
					topologiesData.find((t) => t.id === share.topology_id)?.name || shares_unknownTopology()
				);
			}
		},
		{
			key: 'network_id',
			label: common_network(),
			type: 'string',
			filterable: true,
			getValue: (share) => {
				return networksData.find((n) => n.id === share.network_id)?.name || common_unknownNetwork();
			}
		},
		{
			key: 'is_enabled',
			label: common_enabled(),
			type: 'boolean',
			filterable: true
		},
		{
			key: 'expires_at',
			label: common_expires(),
			type: 'date'
		},
		{
			key: 'created_at',
			label: common_created(),
			type: 'date'
		}
	];

	function handleEdit(share: Share) {
		editingShare = share;
		showEditor = true;
	}

	function handleDelete(share: Share) {
		if (confirm(common_confirmDeleteName({ name: share.name }))) {
			deleteShareMutation.mutate(share.id);
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(shares_confirmBulkDelete({ count: ids.length }))) {
			await bulkDeleteSharesMutation.mutateAsync(ids);
		}
	}

	function handleCloseEditor() {
		showEditor = false;
		editingShare = null;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Share', {});
	}
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_sharing()} />

	{#if !hasShareViews}
		<EmptyState
			title="Sharing Not Available"
			subtitle="Upgrade to share live network diagrams with others."
		>
			<UpgradeButton feature="sharing" />
		</EmptyState>
	{:else if isLoading}
		<Loading />
	{:else if sharesData.length === 0}
		<!-- Empty state -->
		<EmptyState title={shares_noSharesYet()} subtitle={shares_noSharesSubtitle()} />
	{:else}
		<DataControls
			items={sharesData}
			fields={shareFields}
			storageKey="scanopy-shares-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Share,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<ShareCard
					share={item}
					{viewMode}
					selected={isSelected}
					{onSelectionChange}
					onEdit={isReadOnly ? undefined : handleEdit}
					onDelete={isReadOnly ? undefined : handleDelete}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<ShareModal isOpen={showEditor} share={editingShare} onClose={handleCloseEditor} />
