<script lang="ts">
	import { type Node, type Edge, type Connection } from '@xyflow/svelte';
	import {
		selectedEdge,
		selectedNode,
		selectedNodes,
		useUpdateNodePositionMutation,
		useUpdateEdgeHandlesMutation
	} from '../../queries';
	import { type EdgeHandle, type TopologyEdge, type EnrichedTopology } from '../../types/base';
	import { searchOpen } from '../../interactions';
	import { editModeEnabled } from '../../state';
	import { createTopologyKeydownHandler } from '../../keyboard';
	import BaseTopologyViewer from './BaseTopologyViewer.svelte';
	import SearchOverlay from './SearchOverlay.svelte';
	import ShortcutsHelpOverlay from './ShortcutsHelpOverlay.svelte';
	import { onDestroy } from 'svelte';

	// Props for callbacks from parent
	let {
		topology,
		onToggleLock,
		onRebuild,
		isActive = false
	}: {
		topology: EnrichedTopology | null | undefined;
		onToggleLock?: () => void;
		onRebuild?: () => void;
		isActive?: boolean;
	} = $props();

	// TanStack Query hooks
	const updateNodePositionMutation = useUpdateNodePositionMutation();
	const updateEdgeHandlesMutation = useUpdateEdgeHandlesMutation();

	let baseViewer: BaseTopologyViewer | null = $state(null);

	// Overlay state
	let shortcutsHelpOpen = $state(false);

	// Edit mode state — defaults to view mode (locked), resets on page load
	let editMode = $state(false);

	function toggleEditMode() {
		editMode = !editMode;
		editModeEnabled.set(editMode);
	}

	// Sidebar buttons show labels briefly on first visit per session, then stay collapsed
	const SIDEBAR_SEEN_KEY = 'topology_sidebar_labels_shown';
	const alreadySeen =
		typeof sessionStorage !== 'undefined' && sessionStorage.getItem(SIDEBAR_SEEN_KEY) === '1';
	let sidebarCollapsed = $state(alreadySeen);

	$effect(() => {
		if (isActive && !alreadySeen && !sidebarCollapsed) {
			const timer = setTimeout(() => {
				sidebarCollapsed = true;
				sessionStorage.setItem(SIDEBAR_SEEN_KEY, '1');
			}, 2000);
			return () => clearTimeout(timer);
		}
	});

	// Reset edit mode when leaving this tab (tabs stay mounted, just hidden)
	$effect(() => {
		if (!isActive && editMode) {
			editMode = false;
			editModeEnabled.set(false);
		}
	});

	onDestroy(() => {
		editModeEnabled.set(false);
	});

	export function triggerFitView() {
		baseViewer?.triggerFitView();
	}

	async function handleNodeDragStop(targetNode: Node) {
		if (!topology) return;
		let movedNode = topology.nodes.find((node) => node.id == targetNode?.id);
		if (movedNode && targetNode && targetNode.position) {
			// Snap to 25px grid (matches SvelteFlow snapGrid and ELK post-layout snap)
			const SNAP = 25;
			const x = Math.round(targetNode.position.x / SNAP) * SNAP;
			const y = Math.round(targetNode.position.y / SNAP) * SNAP;
			// Update local state for immediate feedback
			movedNode.position.x = x;
			movedNode.position.y = y;
			// Send lightweight update to server (fixes HTTP 413 for large topologies)
			await updateNodePositionMutation.mutateAsync({
				topologyId: topology.id,
				networkId: topology.network_id,
				nodeId: movedNode.id,
				position: { x, y }
			});
		}
	}

	async function handleReconnect(edge: Edge, newConnection: Connection) {
		if (!topology) return;
		const edgeData = edge.data as TopologyEdge;

		if ($selectedEdge && edge.id === $selectedEdge.id) {
			let topologyEdge = topology.edges.find((e) => e.id == edgeData.id);
			if (
				topologyEdge &&
				newConnection.source == topologyEdge.source &&
				newConnection.target == topologyEdge.target &&
				newConnection.sourceHandle &&
				newConnection.targetHandle
			) {
				// Update local state for immediate feedback
				topologyEdge.source_handle = newConnection.sourceHandle as EdgeHandle;
				topologyEdge.target_handle = newConnection.targetHandle as EdgeHandle;
				// Send lightweight update to server (fixes HTTP 413 for large topologies)
				await updateEdgeHandlesMutation.mutateAsync({
					topologyId: topology.id,
					networkId: topology.network_id,
					edgeId: topologyEdge.id,
					sourceHandle: newConnection.sourceHandle as 'Top' | 'Bottom' | 'Left' | 'Right',
					targetHandle: newConnection.targetHandle as 'Top' | 'Bottom' | 'Left' | 'Right'
				});
			}
		}
	}

	const handleKeydown = createTopologyKeydownHandler({
		getBaseViewer: () => baseViewer,
		getShortcutsHelpOpen: () => shortcutsHelpOpen,
		setShortcutsHelpOpen: (open) => (shortcutsHelpOpen = open),
		selectionStores: { selectedNode, selectedEdge, selectedNodes },
		isEnabled: () => isActive,
		onToggleEditMode: toggleEditMode,
		onToggleLock: () => onToggleLock?.(),
		onRebuild: () => onRebuild?.()
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if topology}
	<div class="relative h-[calc(100vh-120px)] w-full">
		<BaseTopologyViewer
			bind:this={baseViewer}
			{topology}
			readonly={!editMode}
			showControls={true}
			{editMode}
			{sidebarCollapsed}
			onToggleEditMode={toggleEditMode}
			onNodeDragStop={handleNodeDragStop}
			onReconnect={handleReconnect}
			onOpenShortcuts={() => (shortcutsHelpOpen = true)}
			onOpenSearch={() => searchOpen.set(true)}
		/>
		<SearchOverlay />
		<ShortcutsHelpOverlay bind:isOpen={shortcutsHelpOpen} />
	</div>
{/if}
