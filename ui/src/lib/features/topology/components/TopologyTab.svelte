<script lang="ts">
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import TopologyViewer from './visualization/TopologyViewer.svelte';
	import TopologyOptionsPanel from './panel/TopologyOptionsPanel.svelte';
	import { Edit, Globe, Lock, Plus, Radio, RefreshCcw, Share2, Trash2 } from 'lucide-svelte';
	import ExportButton from './ExportButton.svelte';
	import ShareModal from '$lib/features/shares/components/ShareModal.svelte';
	import { SvelteFlowProvider } from '@xyflow/svelte';
	import {
		useTopologiesQuery,
		useDeleteTopologyMutation,
		useRebuildTopologyMutation,
		useLockTopologyMutation,
		useUnlockTopologyMutation,
		autoRebuild,
		hasConflicts,
		selectedTopologyId,
		consumePreferredNetwork
	} from '../queries';
	import type { Topology } from '../types/base';
	import TopologyModal from './TopologyModal.svelte';
	import { getTopologyState } from '../state';
	import StateBadge from './StateBadge.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import RefreshConflictsModal from './RefreshConflictsModal.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { TopologyDisplay } from '$lib/shared/components/forms/selection/display/TopologyDisplay.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useGroupsQuery } from '$lib/features/groups/queries';
	import { useUsersQuery } from '$lib/features/users/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import type { TabProps } from '$lib/shared/types';

	let { isReadOnly = false }: TabProps = $props();

	// Get current user to check permissions
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);
	let canViewUsers = $derived(
		currentUser
			? (permissions.getMetadata(currentUser.permissions)?.grantable_user_permissions?.length ??
					0) > 0
			: false
	);

	// Queries - TanStack Query handles deduplication
	// Use limit: 0 to get all hosts for topology visualization
	const hostsQuery = useHostsQuery({ limit: 0 });
	const subnetsQuery = useSubnetsQuery();
	const groupsQuery = useGroupsQuery();
	const usersQuery = useUsersQuery({ enabled: () => canViewUsers });
	const topologiesQuery = useTopologiesQuery();

	// Mutations
	const deleteTopologyMutation = useDeleteTopologyMutation();
	const rebuildTopologyMutation = useRebuildTopologyMutation();
	const lockTopologyMutation = useLockTopologyMutation();
	const unlockTopologyMutation = useUnlockTopologyMutation();

	// Derived data
	let usersData = $derived(usersQuery.data ?? []);
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let isLoading = $derived(
		hostsQuery.isPending ||
			subnetsQuery.isPending ||
			groupsQuery.isPending ||
			topologiesQuery.isPending
	);

	// Selected topology (derived from ID + query data)
	let currentTopology = $derived(
		$selectedTopologyId ? topologiesData.find((t) => t.id === $selectedTopologyId) : null
	);

	// Initialize selected topology when data loads
	$effect(() => {
		if (topologiesData.length > 0 && !$selectedTopologyId) {
			// Check for preferred network from onboarding
			const preferredNetworkId = consumePreferredNetwork();
			if (preferredNetworkId) {
				const preferred = topologiesData.find((t) => t.network_id === preferredNetworkId);
				if (preferred) {
					selectedTopologyId.set(preferred.id);
					return;
				}
			}
			// Default to first topology
			selectedTopologyId.set(topologiesData[0].id);
		}
	});

	let isCreateEditOpen = $state(false);
	let editingTopology: Topology | null = $state(null);

	let isRefreshConflictsOpen = $state(false);
	let isShareModalOpen = $state(false);

	let topologyViewer: TopologyViewer | null = $state(null);

	function handleCreateTopology() {
		isCreateEditOpen = true;
		editingTopology = null;
	}

	function handleEditTopology() {
		isCreateEditOpen = true;
		editingTopology = currentTopology ?? null;
	}

	function onSubmit() {
		isCreateEditOpen = false;
		editingTopology = null;
	}

	function onClose() {
		isCreateEditOpen = false;
		editingTopology = null;
	}

	// Handle topology selection
	function handleTopologyChange(value: string) {
		selectedTopologyId.set(value);
	}

	async function handleDelete() {
		if (!currentTopology) return;
		// Capture values before async operation (currentTopology becomes null after query refetch)
		const toDeleteId = currentTopology.id;
		const toDeleteName = currentTopology.name;
		if (confirm(`Are you sure you want to delete topology ${toDeleteName}?`)) {
			await deleteTopologyMutation.mutateAsync(toDeleteId);
			// After mutation, topologiesData is already updated without the deleted topology
			if (topologiesData.length > 0) {
				selectedTopologyId.set(topologiesData[0].id);
			} else {
				selectedTopologyId.set(null);
			}
		}
	}

	async function handleAutoRebuildToggle() {
		autoRebuild.set(!$autoRebuild);
		if ($autoRebuild) {
			await handleRefresh();
		}
	}

	async function handleRefresh() {
		if (!currentTopology) return;

		if (hasConflicts(currentTopology)) {
			// Open modal to review conflicts
			isRefreshConflictsOpen = true;
		} else {
			// Safe to refresh directly
			await rebuildTopologyMutation.mutateAsync(currentTopology);
			topologyViewer?.triggerFitView();
		}
	}

	async function handleReset() {
		if (!currentTopology) return;
		let resetTopology = { ...currentTopology };
		resetTopology.nodes = [];
		resetTopology.edges = [];
		await rebuildTopologyMutation.mutateAsync(resetTopology);
		topologyViewer?.triggerFitView();
	}

	async function handleConfirmRefresh() {
		if (!currentTopology) return;
		await rebuildTopologyMutation.mutateAsync(currentTopology);
		topologyViewer?.triggerFitView();
		isRefreshConflictsOpen = false;
	}

	async function handleLockFromConflicts() {
		if (!currentTopology) return;
		await lockTopologyMutation.mutateAsync(currentTopology);
		isRefreshConflictsOpen = false;
	}

	async function handleToggleLock() {
		if (!currentTopology) return;
		if (currentTopology.is_locked) {
			await unlockTopologyMutation.mutateAsync(currentTopology);
		} else {
			await lockTopologyMutation.mutateAsync(currentTopology);
		}
	}

	let stateConfig = $derived(
		currentTopology
			? getTopologyState(currentTopology, $autoRebuild, {
					onRefresh: handleRefresh,
					onReset: handleReset
				})
			: null
	);

	let lockedByUser = $derived(
		currentTopology?.locked_by ? usersData.find((u) => u.id === currentTopology.locked_by) : null
	);
	let lockedByDisplay = $derived(
		lockedByUser?.email ?? (currentTopology?.locked_by ? 'another user' : null)
	);
</script>

<SvelteFlowProvider>
	<div class="space-y-6">
		<!-- Header -->
		<div class="card card-static flex items-center justify-evenly gap-4 px-4 py-2">
			{#if currentTopology}
				<div class="flex items-center gap-4 py-2">
					<ExportButton />
					{#if !isReadOnly}
						<button class="btn-secondary" onclick={() => (isShareModalOpen = true)}>
							<Share2 class="my-1 h-5 w-5" />
						</button>
					{/if}
					<a
						href="https://tally.so/r/lbqLAv"
						target="_blank"
						rel="noopener noreferrer"
						class="btn-secondary"
						title="Submit to Community Showcase"
					>
						<Globe class="my-1 h-5 w-5" />
					</a>
				</div>

				{#if !isReadOnly}
					<div class="card-divider-v self-stretch"></div>

					<div class="flex items-center py-2">
						<div class="mr-2 flex flex-col text-center">
							<div class="flex justify-around gap-6">
								<button
									onclick={handleToggleLock}
									class={`text-xs ${currentTopology.is_locked ? 'btn-icon-info' : 'btn-icon'}`}
								>
									<Lock class="mr-2 h-4 w-4" />
									{currentTopology.is_locked ? 'Unlock' : 'Lock'}
								</button>

								{#if !currentTopology.is_locked}
									<button
										onclick={handleAutoRebuildToggle}
										type="button"
										class={`text-xs ${$autoRebuild && !currentTopology.is_locked ? 'btn-icon-success' : 'btn-icon'}`}
										disabled={currentTopology.is_locked}
									>
										{#if $autoRebuild}
											<Radio class="mr-2 h-4 w-4" /> Auto
										{:else}
											<RefreshCcw class="mr-2 h-4 w-4" /> Manual
										{/if}
									</button>
								{/if}
							</div>
							{#if currentTopology.is_locked && currentTopology.locked_at}
								<span class="text-tertiary whitespace-nowrap text-[10px]"
									>Locked: {formatTimestamp(currentTopology.locked_at)} by {lockedByDisplay}</span
								>
							{:else}
								<span class="text-tertiary whitespace-nowrap text-[10px]"
									>Last Rebuild: {formatTimestamp(currentTopology.last_refreshed)}</span
								>
							{/if}
						</div>
						<!-- State Badge / Action Button -->
						{#if stateConfig && !currentTopology.is_locked && !$autoRebuild}
							<div class="flex flex-col items-center gap-2">
								<div class="flex items-center">
									<StateBadge
										disabled={stateConfig?.disabled || false}
										Icon={stateConfig.icon}
										label={stateConfig.buttonText}
										cls={stateConfig.class}
										onClick={stateConfig.action}
									/>
								</div>
							</div>
						{/if}
					</div>

					<div class="card-divider-v self-stretch"></div>
				{/if}

				{#if topologiesData.length > 0}
					<RichSelect
						label=""
						selectedValue={currentTopology.id}
						displayComponent={TopologyDisplay}
						onSelect={handleTopologyChange}
						options={topologiesData}
					/>
				{/if}
			{/if}

			{#if !isReadOnly}
				{#if currentTopology}
					<div class="card-divider-v self-stretch"></div>
				{/if}

				<div class="flex items-center gap-4 py-2">
					{#if currentTopology}
						<button class="btn-primary" onclick={handleEditTopology}>
							<Edit class="my-1 h-4 w-4" />
						</button>
					{/if}

					<button class="btn-primary" onclick={handleCreateTopology}>
						<Plus class="my-1 h-4 w-4" />
					</button>

					{#if currentTopology}
						<button class="btn-danger" onclick={handleDelete}>
							<Trash2 class="my-1 h-5 w-5" />
						</button>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Contextual Info Banner -->
		{#if currentTopology && stateConfig}
			{#if stateConfig.type === 'locked'}
				<InlineInfo
					dismissableKey="topology-locked-info"
					title={`Topology Locked${lockedByDisplay ? ` by ${lockedByDisplay}` : ''}`}
					body="Data can't be refreshed while this topology is locked. You can still move and resize nodes and edges, but you won't be able to make any other changes. Click the badge above to unlock and enable data refresh."
				/>
			{:else if stateConfig.type === 'stale_conflicts'}
				<InlineDanger
					dismissableKey="topology-conflict-info"
					title="Conflicts Detected"
					body="Some entities in this diagram no longer exist. Click the badge above to review
								changes before updating."
				/>
			{:else if stateConfig.type === 'stale_safe'}
				<InlineWarning
					dismissableKey="topology-refresh-info"
					title="Stale Data"
					body="Entities have been updated, and the diagram layout may need to change to fit them."
				/>
			{/if}
		{/if}

		{#if isLoading}
			<Loading />
		{:else if currentTopology}
			<div class="relative">
				<TopologyOptionsPanel />
				<TopologyViewer bind:this={topologyViewer} />
			</div>
		{:else}
			<div class="card card-static text-secondary">
				No topology selected. Create one to get started.
			</div>
		{/if}
	</div>
</SvelteFlowProvider>

<TopologyModal bind:isOpen={isCreateEditOpen} {onSubmit} {onClose} topo={editingTopology} />

{#if currentTopology}
	<RefreshConflictsModal
		bind:isOpen={isRefreshConflictsOpen}
		topology={currentTopology}
		onConfirm={handleConfirmRefresh}
		onLock={handleLockFromConflicts}
		onCancel={() => (isRefreshConflictsOpen = false)}
	/>
	<ShareModal
		isOpen={isShareModalOpen}
		topologyId={currentTopology.id}
		networkId={currentTopology.network_id}
		onClose={() => (isShareModalOpen = false)}
	/>
{/if}
