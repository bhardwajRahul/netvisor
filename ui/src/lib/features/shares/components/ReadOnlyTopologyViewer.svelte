<script lang="ts">
	import { SvelteFlowProvider, type Node, type Edge } from '@xyflow/svelte';
	import BaseTopologyViewer from '$lib/features/topology/components/visualization/BaseTopologyViewer.svelte';
	import SearchOverlay from '$lib/features/topology/components/visualization/SearchOverlay.svelte';
	import ShortcutsHelpOverlay from '$lib/features/topology/components/visualization/ShortcutsHelpOverlay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import { setContext } from 'svelte';
	import { writable } from 'svelte/store';
	import ReadOnlyInspectorPanel from './ReadOnlyInspectorPanel.svelte';
	import ExportButton from '$lib/features/topology/components/ExportButton.svelte';
	import ExportModal from '$lib/features/topology/components/ExportModal.svelte';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import { Share2 } from 'lucide-svelte';
	import type { ExportFeatures } from '../types/base';
	import {
		hydrateStoresFromTopology,
		optionsPanelExpanded,
		MINIMAP_WIDTH_PX,
		MINIMAP_OFFSET_PX
	} from '$lib/features/topology/queries';
	import { searchOpen } from '$lib/features/topology/interactions';
	import { createTopologyKeydownHandler } from '$lib/features/topology/keyboard';
	import { views } from '$lib/shared/stores/metadata';

	export let topology: RenderableTopology;
	export let showControls: boolean = true;
	export let showInspectPanel: boolean = true;
	export let showExport: boolean = false;
	export let isEmbed: boolean = false;
	export let shareName: string = '';
	export let showMinimap: boolean = false;
	export let exportFeatures: ExportFeatures | undefined = undefined;
	export let enabledViews: string[] = [];
	export let currentView: string = 'L3Logical';
	export let onViewChange: (view: string) => void = () => {};
	export let viewLoading: boolean = false;

	let isExportModalOpen = false;
	let shortcutsHelpOpen = false;
	let baseViewer: BaseTopologyViewer | null = null;

	$: showViewSwitcher = enabledViews.length > 1;
	const minimapClearance = MINIMAP_WIDTH_PX + MINIMAP_OFFSET_PX;

	// Build SegmentedControl options from enabled views
	$: viewOptions = enabledViews.map((viewId) => ({
		value: viewId,
		label: views.getName(viewId),
		icon: views.getIconComponent(viewId),
		tooltip: views.getDescription(viewId)
	}));

	// Collapse the inspector panel by default in share/embed context —
	// prevents localStorage state from the app leaking into embeds.
	optionsPanelExpanded.set(false);

	// Hydrate the global activeView + topologyOptions stores so the rendering
	// pipeline sees the correct view for this shared topology.
	hydrateStoresFromTopology(topology, true, true);

	// Create a context store for the topology so child components (inspectors) can access it
	const topologyContext = writable<RenderableTopology>(topology);
	setContext('topology', topologyContext);

	// Create local stores for selected node/edge (instead of using global store).
	// BaseTopologyViewer resolves these via getContext and uses them as its selection source.
	const selectedNodeStore = writable<Node | null>(null);
	const selectedEdgeStore = writable<Edge | null>(null);
	const selectedNodesStore = writable<Node[]>([]);
	setContext('selectedNode', selectedNodeStore);
	setContext('selectedEdge', selectedEdgeStore);
	setContext('selectedNodes', selectedNodesStore);

	// Keep context in sync with prop and re-hydrate on topology change (view switch)
	$: {
		topologyContext.set(topology);
		hydrateStoresFromTopology(topology, true, true);
	}

	// Keyboard shortcuts — same shared handler, no edit-only callbacks
	const handleKeydown = createTopologyKeydownHandler({
		getBaseViewer: () => baseViewer,
		getShortcutsHelpOpen: () => shortcutsHelpOpen,
		setShortcutsHelpOpen: (open) => (shortcutsHelpOpen = open),
		selectionStores: {
			selectedNode: selectedNodeStore,
			selectedEdge: selectedEdgeStore,
			selectedNodes: selectedNodesStore
		}
	});
</script>

<svelte:window on:keydown={handleKeydown} />

<SvelteFlowProvider>
	<div class="flex h-full w-full flex-col">
		{#if shareName}
			<header
				class="flex flex-shrink-0 items-center justify-between border-b px-4 py-3"
				style="border-color: var(--color-border); background: var(--color-bg-elevated)"
			>
				<div class="flex items-center gap-3">
					<Share2 class="text-info h-8 w-8" />
					<h1 class="text-primary font-semibold">{shareName}</h1>
				</div>
				<div class="flex items-center gap-4">
					{#if showViewSwitcher}
						<div class="flex items-center gap-2">
							<SegmentedControl
								options={viewOptions}
								selected={currentView}
								onchange={onViewChange}
								size="sm"
								disabled={viewLoading}
							/>
						</div>
					{/if}
					{#if showExport}
						<ExportButton onclick={() => (isExportModalOpen = true)} />
					{/if}
				</div>
			</header>
		{/if}
		<div class="relative min-h-0 flex-1">
			{#if showInspectPanel}
				<ReadOnlyInspectorPanel {showMinimap} />
			{/if}

			<div class="bottom-bar">
				{#if showMinimap}
					<div style="width: {minimapClearance}px; flex-shrink: 0;"></div>
				{/if}
				<div class="bottom-bar-center">
					{#if showViewSwitcher && !shareName}
						<SegmentedControl
							options={viewOptions}
							selected={currentView}
							onchange={onViewChange}
							size="sm"
							disabled={viewLoading}
						/>
					{/if}
				</div>
				<a
					href="https://scanopy.net?utm_source={isEmbed
						? 'embed'
						: 'share'}&utm_medium=referral&utm_campaign=created_with"
					target="_blank"
					rel="noopener noreferrer"
					class="branding-badge"
				>
					<img src="/logos/scanopy-logo.png" alt="Scanopy" class="h-4 w-4" />
					<span>Created with Scanopy</span>
				</a>
			</div>

			<BaseTopologyViewer
				bind:this={baseViewer}
				{topology}
				readonly={true}
				{showControls}
				{isEmbed}
				showBranding={false}
				{showMinimap}
				sidebarCollapsed={true}
				onOpenShortcuts={() => (shortcutsHelpOpen = true)}
				onOpenSearch={() => searchOpen.set(true)}
			/>
			<SearchOverlay />
			<ShortcutsHelpOverlay bind:isOpen={shortcutsHelpOpen} readonly={true} />
		</div>
	</div>

	{#if showExport}
		<ExportModal
			topologyId={topology.id}
			topologyName={topology.name}
			bind:isOpen={isExportModalOpen}
			isShareView={true}
			{exportFeatures}
		/>
	{/if}
</SvelteFlowProvider>

<style>
	.bottom-bar {
		position: absolute;
		bottom: 10px;
		left: 10px;
		right: 10px;
		z-index: 5;
		display: flex;
		justify-content: space-between;
		align-items: center;
		pointer-events: none;
	}

	.bottom-bar > :global(*) {
		pointer-events: auto;
	}

	.bottom-bar-center {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.branding-badge {
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--color-text-muted);
		font-size: 12px;
		text-decoration: none;
		transition: color 0.2s;
		flex-shrink: 0;
	}

	.branding-badge:hover {
		color: var(--color-text-secondary);
	}
</style>
