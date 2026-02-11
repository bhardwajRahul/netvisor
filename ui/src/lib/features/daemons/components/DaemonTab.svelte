<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import DaemonCard from './DaemonCard.svelte';
	import CreateDaemonModal from './CreateDaemonModal.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useDaemonsQuery,
		useDeleteDaemonMutation,
		useBulkDeleteDaemonsMutation
	} from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import {
		common_create,
		common_created,
		common_daemons,
		common_name,
		common_network,
		common_tags,
		common_unknownNetwork,
		common_updated,
		daemons_confirmBulkDelete,
		daemons_confirmDelete,
		daemons_lastSeen,
		daemons_noDaemonsYet
	} from '$lib/paraglide/messages';

	type DaemonOrderField = components['schemas']['DaemonOrderField'];

	let { isReadOnly = false }: TabProps = $props();

	// Queries
	const tagsQuery = useTagsQuery();
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();
	// Hosts query to ensure data is loaded (needed for daemon display)
	useHostsQuery({ limit: 0 });

	// Mutations
	const deleteDaemonMutation = useDeleteDaemonMutation();
	const bulkDeleteDaemonsMutation = useBulkDeleteDaemonsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(daemonsQuery.isPending || networksQuery.isPending);

	let showCreateDaemonModal = $state(false);
	let daemon = $state<Daemon | null>(null);

	// Auto-open modal after onboarding (only if no daemons exist yet)
	$effect(() => {
		if (typeof sessionStorage !== 'undefined' && daemonsQuery.isSuccess) {
			const shouldShow = sessionStorage.getItem('showDaemonSetup');
			if (shouldShow === 'true') {
				sessionStorage.removeItem('showDaemonSetup');
				if (daemonsData.length === 0) {
					showCreateDaemonModal = true;
				}
			}
		}
	});

	function handleDeleteDaemon(daemon: Daemon) {
		if (confirm(daemons_confirmDelete({ name: daemon.name }))) {
			deleteDaemonMutation.mutate(daemon.id);
		}
	}

	function handleCreateDaemon() {
		showCreateDaemonModal = true;
		daemon = null;
	}

	function handleCloseCreateDaemon() {
		showCreateDaemonModal = false;
		daemon = null;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(daemons_confirmBulkDelete({ count: ids.length }))) {
			await bulkDeleteDaemonsMutation.mutateAsync(ids);
		}
	}

	function getDaemonTags(daemon: Daemon): string[] {
		return daemon.tags;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Daemon', {});
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all DaemonOrderField values are covered
	let daemonFields = $derived(
		defineFields<Daemon, DaemonOrderField>(
			{
				name: { label: common_name(), type: 'string', searchable: true },
				network_id: {
					label: common_network(),
					type: 'string',
					filterable: true,
					groupable: true,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork()
				},
				last_seen: { label: daemons_lastSeen(), type: 'date' },
				created_at: { label: common_created(), type: 'date' },
				updated_at: { label: common_updated(), type: 'date' }
			},
			[
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) =>
						entity.tags
							.map((id) => tagsData.find((t) => t.id === id)?.name)
							.filter((name): name is string => !!name)
				}
			]
		)
	);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_daemons()}>
		<svelte:fragment slot="actions">
			{#if !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateDaemon}
					><Plus class="h-5 w-5" />{common_create()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	<!-- Loading state -->
	{#if isLoading}
		<Loading />
	{:else if daemonsData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={daemons_noDaemonsYet()}
			subtitle=""
			onClick={handleCreateDaemon}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={daemonsData}
			fields={daemonFields}
			storageKey="scanopy-daemons-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Daemon'}
			getItemTags={getDaemonTags}
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Daemon,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<DaemonCard
					daemon={item}
					{viewMode}
					onDelete={isReadOnly ? undefined : handleDeleteDaemon}
					selected={isSelected}
					{onSelectionChange}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<CreateDaemonModal isOpen={showCreateDaemonModal} onClose={handleCloseCreateDaemon} {daemon} />
