<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import DaemonCard from './DaemonCard.svelte';
	import { hasSunsetWarning } from '$lib/features/daemons/utils';
	import CreateDaemonModal from './CreateDaemonModal/CreateDaemonModal.svelte';
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
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import DaemonEditModal from './DaemonEditModal.svelte';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import {
		common_active,
		common_create,
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_created,
		common_daemons,
		common_name,
		common_network,
		common_standby,
		common_status,
		common_tags,
		common_noEntityYet,
		common_unknownNetwork,
		common_unreachable,
		common_updated,
		daemons_config_mode,
		daemons_lastSeen,
		daemons_mode_daemonPoll,
		daemons_mode_serverPoll,
		daemons_sunsetBannerTitle,
		daemons_sunsetBannerBody
	} from '$lib/paraglide/messages';

	type DaemonOrderField = components['schemas']['DaemonOrderField'];

	let { isReadOnly = false }: TabProps = $props();

	// Queries
	const tagsQuery = useTagsQuery();
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();

	// Mutations
	const deleteDaemonMutation = useDeleteDaemonMutation();
	const bulkDeleteDaemonsMutation = useBulkDeleteDaemonsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);

	// Only the hosts the daemons actually run on. This was an unpaginated
	// org-wide hosts query (~1.9MB on a few hundred hosts) issued to resolve one
	// name per daemon card — and because TanStack dedupes by key, it was shared
	// with every other consumer, so it loaded on pages that never showed a
	// daemon. Scoped to the ids in hand and passed down to the cards.
	let daemonHostIds = $derived([
		...new Set(daemonsData.map((d) => d.host_id).filter((id): id is string => !!id))
	]);
	const daemonHostsQuery = useHostsByIds(() => daemonHostIds);
	let daemonHosts = $derived(daemonHostsQuery.data ?? []);

	// Any daemon with a scheduled/active sunset. Drives a non-dismissable banner
	// so the warning re-arms as long as an affected daemon exists.
	let hasSunsetDaemons = $derived(daemonsData.some(hasSunsetWarning));
	// The shared sunset date across affected daemons (server-published, same value
	// the email uses), formatted for display. All below-floor daemons share the
	// one announced date.
	let sunsetDateDisplay = $derived.by(() => {
		const iso = daemonsData.find(hasSunsetWarning)?.version_status.sunset_date;
		if (!iso) return null;
		return new Date(`${iso}T00:00:00Z`).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'long',
			day: 'numeric',
			timeZone: 'UTC'
		});
	});
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(daemonsQuery.isPending || networksQuery.isPending);

	let showCreateDaemonModal = $state(false);
	let showDaemonEditor = $state(false);
	let editingDaemon = $state<Daemon | null>(null);

	// Auto-open modal when deep-linked via ?modal=create-daemon
	$effect(() => {
		if ($modalState.name === 'create-daemon' && !showCreateDaemonModal) {
			showCreateDaemonModal = true;
		}
	});

	// Deep-link the daemon editor via ?modal=daemon-editor&id=<daemon-id>
	$effect(() => {
		const resolved = resolveModalDeepLink(
			$modalState,
			'daemon-editor',
			daemonsData,
			showDaemonEditor,
			editingDaemon?.id
		);
		if (resolved !== undefined) {
			editingDaemon = resolved;
			showDaemonEditor = true;
		}
	});

	function handleEditDaemon(daemon: Daemon) {
		editingDaemon = daemon;
		showDaemonEditor = true;
	}

	function handleCloseDaemonEditor() {
		showDaemonEditor = false;
		editingDaemon = null;
	}

	function handleDeleteDaemon(daemon: Daemon) {
		if (confirm(common_confirmDeleteName({ name: daemon.name }))) {
			deleteDaemonMutation.mutate(daemon.id);
		}
	}

	function handleCreateDaemon() {
		showCreateDaemonModal = true;
	}

	function handleCloseCreateDaemon() {
		showCreateDaemonModal = false;
		// Only redirect to home if no daemons AND we're still on the daemons tab
		// (user may have navigated away via "View Topology" button)
		if (daemonsData.length === 0 && window.location.hash === '#daemons') {
			window.location.hash = 'home';
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_daemons() }))) {
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
				// Identity field: grouping by it would render a header per daemon.
				name: { label: common_name(), type: 'string', searchable: true, groupable: false },
				network_id: {
					label: common_network(),
					type: 'string',
					searchable: true,
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
					// Reachability as one value rather than a raw boolean: "which
					// daemons are down" is the question during an incident, and a
					// standby daemon is a third answer, not a shade of unreachable.
					key: 'status',
					label: common_status(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (daemon) =>
						daemon.is_unreachable
							? common_unreachable()
							: daemon.standby
								? common_standby()
								: common_active()
				},
				{
					key: 'mode',
					label: daemons_config_mode(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (daemon) =>
						daemon.mode === 'server_poll' ? daemons_mode_serverPoll() : daemons_mode_daemonPoll()
				},
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
			title={common_noEntityYet({ entity: common_daemons() })}
			subtitle=""
			onClick={handleCreateDaemon}
			cta={common_create()}
		/>
	{:else}
		{#if hasSunsetDaemons && sunsetDateDisplay}
			<div class="mb-4">
				<InlineWarning
					title={daemons_sunsetBannerTitle()}
					body={daemons_sunsetBannerBody({ date: sunsetDateDisplay })}
				/>
			</div>
		{/if}
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
					hosts={daemonHosts}
					{viewMode}
					onDelete={isReadOnly ? undefined : handleDeleteDaemon}
					onEdit={isReadOnly ? undefined : handleEditDaemon}
					selected={isSelected}
					{onSelectionChange}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<CreateDaemonModal
	isOpen={showCreateDaemonModal}
	name="create-daemon"
	onClose={handleCloseCreateDaemon}
	onNavigate={(tab) => {
		window.location.hash = tab;
	}}
/>

<DaemonEditModal
	isOpen={showDaemonEditor}
	name="daemon-editor"
	daemon={editingDaemon}
	onClose={handleCloseDaemonEditor}
/>
