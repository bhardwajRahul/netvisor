<script lang="ts">
	import { writable, derived, get, type Writable } from 'svelte/store';
	import {
		SvelteFlow,
		MiniMap,
		Background,
		BackgroundVariant,
		useNodesInitialized,
		type Connection,
		useSvelteFlow
	} from '@xyflow/svelte';
	import {
		common_collapse,
		common_expand,
		topology_levelFullyCollapsed,
		topology_levelContainersExpanded,
		topology_levelSubcontainersExpanded,
		topology_levelFullyExpanded,
		topology_parseFailed
	} from '$lib/paraglide/messages';
	import { type Node, type Edge } from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import './topology-viewer.css';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		previewEdges,
		baseFlowEdges,
		selectedNodes,
		selectedEdge as selectedEdgeStore,
		selectedNode as selectedNodeStore,
		topologyOptions,
		optionsPanelExpanded,
		editingDependencyId,
		OPTIONS_PANEL_FITVIEW_PADDING_PX,
		MINIMAP_WIDTH_PX,
		MINIMAP_HEIGHT_PX,
		MINIMAP_OFFSET_PX,
		aggregatedEdgeOriginals,
		getInfrastructureRuleIdForTopology,
		topologyReadOnly,
		topologyOptionsHydrated,
		activeView
	} from '../../queries';
	import { isExporting, expandedPortNodeIds } from '../../interactions';

	// Import custom node/edge components
	import ContainerNode from './ContainerNode.svelte';
	import ElementNode from './ElementNode.svelte';
	import CustomEdge from './CustomEdge.svelte';
	import TopologySidebarControls from './TopologySidebarControls.svelte';
	import type { RenderableTopology } from '../../types/base';
	import {
		collapsedContainers,
		collapseLevel,
		stepExpand,
		stepCollapse,
		nextEffectiveLevel
	} from '../../collapse';
	import type { CollapseLevel } from '../../collapse';
	import {
		updateConnectedNodes,
		setEdgeHover,
		clearEdgeHoverState,
		expandedBundles,
		collapseAllBundles,
		collectEdgeHandles,
		edgeHandlesByNode,
		searchHiddenNodeIds,
		tagHiddenNodeIds,
		hiddenEntityIds
	} from '../../interactions';
	import {
		selectNode,
		selectEdge,
		clearSelection,
		handleModifierNodeClick,
		handleBoxSelect,
		type SelectionStores
	} from '../../selection';
	import { onMount, tick, setContext, getContext } from 'svelte';
	import { writable as svelteWritable } from 'svelte/store';
	import { themeStore } from '$lib/shared/stores/theme.svelte';
	import { containerTypes } from '$lib/shared/stores/metadata';

	// Pipeline imports
	import { createInitialState, type XY } from '../../pipeline/types';
	import { prepareTopologyData, hiddenMetadataKey } from '../../pipeline/prepare';
	import { resolveNodeSizes } from '../../pipeline/measure';
	import { executeLayout, handlePortExpansion } from '../../pipeline/execute-layout';
	import { preloadElk } from '../../layout/elk-layout';
	import { buildFlowNodes, sortFlowNodes, stripSizeSeed } from '../../pipeline/build-flow-nodes';
	import { buildFlowEdges } from '../../pipeline/build-flow-edges';
	import { cacheCollapsedSizes, reconcileMeasuredSizes } from '../../pipeline/post-render';
	import throttle from 'just-throttle';
	import { computeEdgeDisplayUpdates } from '../../pipeline/sync-edge-display';
	import { shouldCull } from '../../pipeline/render-mode';
	import {
		installDiagnostics,
		noteNodeStoreWrite,
		noteRunDetail,
		noteRunStart,
		recordAfterRun,
		recordAfterViewportMove
	} from '../../diagnostics';
	import * as perf from '../../perf';
	import {
		reloadInputsDiff,
		snapshotReloadInputs,
		type ReloadInputs
	} from '../../pipeline/reload-guard';

	// Props
	let {
		topology,
		readonly = false,
		showControls = true,
		isEmbed = false,
		hideAttribution = false,
		showBranding = false,
		showMinimap = undefined,
		onNodeDragStop = null,
		onReconnect = null,
		onOpenShortcuts = null,
		onOpenSearch = null,
		editMode = false,
		onToggleEditMode = null,
		sidebarCollapsed = false
	}: {
		topology: RenderableTopology;
		readonly?: boolean;
		showControls?: boolean;
		isEmbed?: boolean;
		hideAttribution?: boolean;
		showBranding?: boolean;
		showMinimap?: boolean | undefined;
		onNodeDragStop?: ((node: Node) => void) | null;
		onReconnect?: ((edge: Edge, newConnection: Connection) => void) | null;
		onOpenShortcuts?: (() => void) | null;
		onOpenSearch?: (() => void) | null;
		editMode?: boolean;
		onToggleEditMode?: (() => void) | null;
		sidebarCollapsed?: boolean;
	} = $props();

	// Create a context store for the topology so child nodes can access it.
	// The effect below keeps the store in sync with the prop across updates;
	// the initial read of `topology` here is just seeding the store.
	// svelte-ignore state_referenced_locally
	const topologyContext = svelteWritable<RenderableTopology>(topology);
	setContext('topology', topologyContext);
	$effect(() => {
		topologyContext.set(topology);
	});

	// Resolve selection stores from context (share/embed) or fall back to global stores.
	// We pass the store *reference* through, not its value, so $/get() don't apply.
	/* eslint-disable svelte/require-store-reactive-access */
	const selNodeStore = getContext<Writable<Node | null>>('selectedNode') ?? selectedNodeStore;
	const selEdgeStore = getContext<Writable<Edge | null>>('selectedEdge') ?? selectedEdgeStore;
	const selNodesStore = getContext<Writable<Node[]>>('selectedNodes') ?? selectedNodes;
	/* eslint-enable svelte/require-store-reactive-access */
	const selectionStores: SelectionStores = {
		selectedNode: selNodeStore,
		selectedEdge: selEdgeStore,
		selectedNodes: selNodesStore
	};

	// Track viewport panning state
	let viewportMoved = false;
	let viewportMoveTimer: ReturnType<typeof setTimeout> | null = null;

	const { fitView, getNodes, getInternalNode } = useSvelteFlow();
	let containerElement: HTMLDivElement;

	/**
	 * Returns fitView padding that accounts for overlays (options panel, minimap).
	 *
	 * The minimap occupies a rectangle in the bottom-left corner. Rather than
	 * reserving an entire row or column, we simulate fitView with uniform padding,
	 * project each node into viewport coordinates, and check if any actually
	 * overlap the minimap region. Only adds padding if real overlap is detected,
	 * and picks the direction (left or bottom) that requires the smallest shift.
	 */
	function getFitViewPadding(): import('@xyflow/system').Padding {
		const minimapVisible =
			showMinimap !== undefined ? showMinimap : get(topologyOptions).local.show_minimap;
		const hasPanel = get(optionsPanelExpanded);

		if (!hasPanel && !minimapVisible) return 0.2;

		const BASE_PAD = 0.2;
		type Pad = number | `${number}px` | `${number}%`;
		let extraBottom: Pad = BASE_PAD;
		let extraLeft: Pad = BASE_PAD;

		if (minimapVisible && containerElement) {
			const cw = containerElement.clientWidth;
			const ch = containerElement.clientHeight;
			const allNodes = getNodes();

			if (allNodes.length > 0 && cw > 0 && ch > 0) {
				// 1. Compute topology bounding box
				let minX = Infinity,
					maxX = -Infinity,
					minY = Infinity,
					maxY = -Infinity;
				for (const n of allNodes) {
					const x = n.position.x;
					const y = n.position.y;
					const w = n.measured?.width ?? n.width ?? 0;
					const h = n.measured?.height ?? n.height ?? 0;
					if (x < minX) minX = x;
					if (x + w > maxX) maxX = x + w;
					if (y < minY) minY = y;
					if (y + h > maxY) maxY = y + h;
				}
				const topoW = maxX - minX || 1;
				const topoH = maxY - minY || 1;

				// 2. Simulate fitView with uniform base padding
				const availW = cw * (1 - 2 * BASE_PAD);
				const availH = ch * (1 - 2 * BASE_PAD);
				const zoom = Math.min(availW / topoW, availH / topoH);

				// Center offset: maps topology coords → viewport coords
				const cx = cw / 2 - (minX + topoW / 2) * zoom;
				const cy = ch / 2 - (minY + topoH / 2) * zoom;

				// 3. Minimap rectangle in viewport coords (with breathing room)
				const GAP = 8;
				const mmLeft = MINIMAP_OFFSET_PX - GAP;
				const mmTop = ch - MINIMAP_OFFSET_PX - MINIMAP_HEIGHT_PX - GAP;
				const mmRight = MINIMAP_OFFSET_PX + MINIMAP_WIDTH_PX + GAP;
				const mmBottom = ch - MINIMAP_OFFSET_PX + GAP;

				// 4. Check if any node overlaps the minimap region
				let hasOverlap = false;
				let maxNodeRight = 0; // rightmost edge of overlapping nodes (for left shift calc)
				let maxNodeBottom = 0; // bottommost edge of overlapping nodes (for bottom shift calc)

				for (const n of allNodes) {
					const nw = n.measured?.width ?? n.width ?? 0;
					const nh = n.measured?.height ?? n.height ?? 0;
					const vx = n.position.x * zoom + cx;
					const vy = n.position.y * zoom + cy;
					const vr = vx + nw * zoom;
					const vb = vy + nh * zoom;

					// Rectangle intersection test
					if (vx < mmRight && vr > mmLeft && vy < mmBottom && vb > mmTop) {
						hasOverlap = true;
						if (vr > maxNodeRight) maxNodeRight = vr;
						if (vb > maxNodeBottom) maxNodeBottom = vb;
					}
				}

				// 5. If overlap, compute minimum shift in each direction and pick the smaller
				if (hasOverlap) {
					const shiftRight = mmRight - mmLeft + GAP; // push content right past minimap
					const shiftUp = mmBottom - mmTop + GAP; // push content up past minimap

					if (shiftRight <= shiftUp) {
						extraLeft = `${MINIMAP_WIDTH_PX + MINIMAP_OFFSET_PX + GAP * 2}px`;
					} else {
						extraBottom = `${MINIMAP_HEIGHT_PX + MINIMAP_OFFSET_PX + GAP * 2}px`;
					}
				}
				// No overlap → extraLeft and extraBottom stay at BASE_PAD
			}
		}

		return {
			top: BASE_PAD,
			right: BASE_PAD,
			bottom: extraBottom,
			left: hasPanel ? `${OPTIONS_PANEL_FITVIEW_PADDING_PX}px` : extraLeft
		};
	}

	export function triggerFitView() {
		requestAnimationFrame(() => fitView({ padding: getFitViewPadding() }));
	}

	export function fitViewToNodes(nodeIds: string[]) {
		requestAnimationFrame(() =>
			fitView({ nodes: nodeIds.map((id) => ({ id })), padding: 0.5, duration: 300 })
		);
	}

	onMount(() => {
		const { fitView } = useSvelteFlow();
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting) {
					requestAnimationFrame(() => fitView());
					observer.disconnect();
				}
			},
			{ threshold: 0.1 }
		);
		if (containerElement) {
			observer.observe(containerElement);
		}
		return () => observer.disconnect();
	});

	// Define node types
	const nodeTypes = { Container: ContainerNode, Element: ElementNode };
	const customEdgeTypes = { custom: CustomEdge };

	// Refit viewport when panel expands/collapses (after 300ms CSS transition)
	let panelInitialized = false;
	$effect(() => {
		if ($optionsPanelExpanded !== undefined) {
			if (panelInitialized) {
				setTimeout(() => fitView({ padding: getFitViewPadding() }), 300);
			}
			panelInitialized = true;
		}
	});

	// Stores for SvelteFlow
	let nodes = writable<Node[]>([]);
	let edges = writable<Edge[]>([]);
	const nodesInitialized = useNodesInitialized();
	let pendingEdges: Edge[] = [];

	// Pipeline state
	const layoutState = createInitialState();

	/**
	 * A DOM measurement pass is running.
	 *
	 * Culling must stay off for its duration: the pass mounts every node and reads its height, and
	 * a culled node never mounts, so measurement would silently return sizes for the on-screen
	 * subset and hand ELK fallbacks for the rest. Set on *every* measure pass, not just the first.
	 */
	let measurePassActive = $state(false);

	/**
	 * The cold load is measuring, which additionally suppresses the expand animation.
	 *
	 * Split from `measurePassActive` because one flag used to do both jobs, and only ever on the
	 * first render (`lastRenderedTopoKey === ''`). Later measure passes still have to suspend
	 * culling, and with a single flag they did not — harmless only while culling was inert.
	 *
	 * Both flags now hide the pane. A measure pass mounts every node with no container sizes, so
	 * containers render as 2px slivers with their contents outside for as long as it runs; at a
	 * few thousand nodes that is a frame or more, and it was plainly visible because only the cold
	 * load hid anything. This flag still exists on its own because `shouldAnimate` means "not a
	 * cold load", which is a different question from "is something being measured".
	 */
	let coldLoadMeasure = $state(false);
	let animatingCollapse = $state(false);

	/** Minimum gap between collapsed-size refreshes while panning. */
	const COLLAPSED_SIZE_REFRESH_MS = 500;

	/** Both flags down: no pass is measuring and the pane may show. */
	function endMeasurePass(): void {
		measurePassActive = false;
		coldLoadMeasure = false;
	}

	// Cull off-screen nodes once the graph is big enough — see
	// `pipeline/render-mode.ts` for why measuring and exporting must suspend it.
	let cullOffscreen = $derived(
		shouldCull({
			renderedCount: $nodes.length,
			measuring: measurePassActive,
			exporting: $isExporting
		})
	);

	/// Everything the blank-canvas diagnostic cannot read from the DOM. Gathered here so the
	/// sample sees the same numbers the culling gate above was given, not a re-derived guess.
	const diagnosticInputs = () => ({
		storeNodes: $nodes.length,
		storeEdges: $edges.length,
		measuring: measurePassActive,
		coldLoadMeasure,
		exporting: $isExporting,
		culling: cullOffscreen,
		container: containerElement,
		// Read lazily: the diagnostic decides whether it wants all of them or a sample, and at
		// this customer's node count the difference matters on the throttled viewport path.
		internalNodes: () => $nodes.map((n) => getInternalNode(n.id))
	});
	installDiagnostics(diagnosticInputs);

	/**
	 * Single point of write for the node store.
	 *
	 * Every mount of the graph goes through here, so counting the writes distinguishes one large
	 * allocation from a remount loop — the question the customer's 247 out-of-memory throws left
	 * open, and one the per-sample ring buffer could not answer.
	 */
	function setStoreNodes(next: Node[]): void {
		noteNodeStoreWrite(next.length);
		nodes.set(next);
	}

	// --- Reactive triggers ---

	// Clear expanded bundles when bundling is toggled off
	$effect(() => {
		if (!$topologyOptions.local.bundle_edges) {
			collapseAllBundles();
		}
	});

	// Trigger loadTopologyData on topology or store changes
	const bundleEdgesStore = derived(topologyOptions, (o) => o.local.bundle_edges ?? false);
	const hideEdgeTypesStore = derived(topologyOptions, (o) =>
		(o.local.hide_edge_types ?? []).join(',')
	);
	// Metadata-value filters (e.g. hiding the OpenPorts service category) are read at render
	// time straight from the options, so no hidden-id store ever fires for them. Without this
	// the cards resize under a layout that never re-runs, and they overlap.
	const hiddenMetadataStore = derived([topologyOptions, activeView], ([, view]) =>
		hiddenMetadataKey(view)
	);

	// Infra rule id derived from the topology bundle being rendered (not the
	// global options store, which hydrates out-of-band and lags a network
	// switch). Keeps auto-collapse of the infra subcontainer correct on switch.
	const getInfrastructureRuleId = () => getInfrastructureRuleIdForTopology(topology);

	let loadInProgress = false;
	let pendingReload = false;
	/**
	 * Escape hatch for the hydration gate below.
	 *
	 * If `hydrateStoresFromTopology` never runs on some path, the view must still
	 * render — a blank topology is a far worse failure than one wasted layout. The
	 * timeout only ever costs something when hydration is genuinely absent.
	 */
	let hydrationWaivedAt = $state(false);
	/**
	 * The store values the in-flight run actually consumed, snapshotted once
	 * `prepare` has returned. A pending reload is honoured only if the current
	 * values differ from these — see `pipeline/reload-guard.ts`.
	 */
	let inFlightInputs: ReloadInputs | null = null;

	function currentReloadInputs(): ReloadInputs {
		return {
			collapsed: get(collapsedContainers),
			expandedBundles: get(expandedBundles),
			expandedPorts: get(expandedPortNodeIds),
			bundleEdges: get(topologyOptions).local.bundle_edges ?? false,
			hiddenEdgeTypes: (get(topologyOptions).local.hide_edge_types ?? []).join(','),
			tagHidden: get(tagHiddenNodeIds),
			hiddenEntities: get(hiddenEntityIds),
			hiddenMetadata: hiddenMetadataKey(get(activeView))
		};
	}
	function triggerLoad(source = 'unknown') {
		// Hold every entry point, not just the topology effect: the option stores
		// fire during hydration too, and any one of them starting the pipeline
		// early would defeat the gate. The effect below re-triggers once hydration
		// lands, so nothing is lost by returning here.
		if (topology && !loadInProgress && !$topologyOptionsHydrated && !hydrationWaivedAt) {
			perf.count(`deferred-until-hydration:${source}`);
			return;
		}
		if (!topology || loadInProgress) {
			if (topology && loadInProgress) {
				// A store wrote while the pipeline was mid-flight, so the whole run
				// will be repeated. Attributed by source because each one costs a
				// full re-layout (two elk.layout() calls).
				perf.count(`pending-reload:${source}`);
				pendingReload = true;
			}
			return;
		}
		// Counted at the point a run actually begins (as opposed to being queued),
		// so each full pipeline execution — two elk.layout() calls — is attributable
		// to what started it.
		perf.count(`run-start:${source}`);
		// Always-on twin of the counter above: `perf` records nothing in a customer's build, and
		// which trigger started a run is the missing half of the zero-sized-container reports.
		noteRunStart(source);
		loadInProgress = true;
		pendingReload = false;
		void loadTopologyData()
			.catch((err) => {
				endMeasurePass();
				pushError(topology_parseFailed({ error: String(err) }));
			})
			.finally(() => {
				loadInProgress = false;
				if (pendingReload) {
					pendingReload = false;
					// Only re-run if an input actually differs from what the run
					// consumed. Most mid-run writes are the pipeline's own (prepare
					// seeding collapse state) or derived stores re-emitting an
					// identical value during option hydration — re-running for those
					// costs two elk.layout() calls and changes nothing.
					const consumed = inFlightInputs;
					inFlightInputs = null;
					if (consumed) {
						const changed = reloadInputsDiff(consumed, currentReloadInputs());
						if (changed.length === 0) {
							perf.count('reload-suppressed');
							return;
						}
						for (const field of changed) perf.count(`reload-cause:${field}`);
					}
					triggerLoad('pending');
				}
				inFlightInputs = null;
			});
	}

	let storesInitialized = false;

	/**
	 * Start a run *after* the caller that wrote the store has finished.
	 *
	 * Svelte notifies subscribers synchronously from `set`, and `loadTopologyData` runs as far as
	 * `prepare` before its first `await` — so the whole first half of a pipeline run executes
	 * inside the `.set()` call, before the writing function's next statement. Anything a caller
	 * does after writing collapse state therefore lands too late to affect the run it just caused.
	 *
	 * `stepExpand` was one such caller: it marked the auto-collapse containers seen after writing,
	 * so `applyAutoCollapse` re-collapsed the very containers it was about to be told to leave
	 * alone. The level advanced, the collapsed set did not, `collapseChanged` came out false, no
	 * layout ran, and the diagram stayed on the previous level with anything new to it unsized.
	 * That call site is now ordered correctly, but the hazard belongs to every writer of these
	 * stores — a container chevron, the level buttons, a filter — and each would have to remember.
	 *
	 * Deferring by a microtask makes the ordering safe by construction: the caller always
	 * completes first, and the run still starts in the same frame.
	 */
	const deferTriggerLoad = (source: string) => {
		queueMicrotask(() => {
			if (storesInitialized) triggerLoad(source);
		});
	};

	collapsedContainers.subscribe(() => {
		if (storesInitialized) deferTriggerLoad('collapsed');
	});
	expandedBundles.subscribe(() => {
		if (storesInitialized) triggerLoad('bundles');
	});
	expandedPortNodeIds.subscribe(() => {
		if (storesInitialized) triggerLoad('ports');
	});
	bundleEdgesStore.subscribe(() => {
		if (storesInitialized) triggerLoad('bundle-option');
	});
	hideEdgeTypesStore.subscribe(() => {
		if (storesInitialized) triggerLoad('hidden-edge-types');
	});
	// Filter changes must re-run the pipeline so ELK sees the new node set and containers
	// reflow around the removed cards. Three stores, because a filter can change the layout
	// without removing a node: an entity shown inline on another node's card resizes that card,
	// and a metadata-value filter never reaches either hidden-id store at all.
	tagHiddenNodeIds.subscribe(() => {
		if (storesInitialized) triggerLoad('tag-filter');
	});
	hiddenEntityIds.subscribe(() => {
		if (storesInitialized) triggerLoad('entity-filter');
	});
	hiddenMetadataStore.subscribe(() => {
		if (storesInitialized) triggerLoad('metadata-filter');
	});
	storesInitialized = true;

	onMount(() => {
		if (get(topologyOptionsHydrated)) return;
		const timer = setTimeout(() => {
			// Re-check rather than waive blindly: on a slow first load hydration may
			// simply not have arrived yet when the timer was armed, and waiving then
			// would reintroduce the pre-hydration layout this gate exists to avoid.
			if (get(topologyOptionsHydrated)) return;
			perf.count('hydration-gate-waived');
			hydrationWaivedAt = true;
		}, 2000);
		return () => clearTimeout(timer);
	});

	// Wait for options to hydrate before the first layout. Without this the
	// pipeline races hydration and discards its first full run — see
	// `topologyOptionsHydrated`.
	$effect(() => {
		if (topology && ($topologyOptionsHydrated || hydrationWaivedAt)) triggerLoad('topology');
	});

	// Update edges when selection or search/tag filter changes
	$effect(() => {
		const curSelectedNode = $selNodeStore;
		const curSelectedEdge = $selEdgeStore;
		const multiSelected = $selNodesStore;
		const searchHidden = $searchHiddenNodeIds;
		const tagHidden = $tagHiddenNodeIds;

		if (topology && (topology.edges || topology.nodes)) {
			const currentBaseEdges = get(baseFlowEdges);
			const currentNodes = get(nodes);
			const opts = get(topologyOptions);

			updateConnectedNodes(
				curSelectedNode,
				curSelectedEdge,
				currentBaseEdges,
				currentNodes,
				topology,
				multiSelected,
				opts.local.hide_edge_types ?? []
			);
			const updatedEdges = computeEdgeDisplayUpdates(
				currentBaseEdges,
				curSelectedNode,
				curSelectedEdge,
				searchHidden,
				tagHidden
			);
			if (updatedEdges !== currentBaseEdges) baseFlowEdges.set(updatedEdges);
		}
	});

	// Add edges when nodes are ready
	$effect(() => {
		if (nodesInitialized.current && pendingEdges.length > 0) {
			baseFlowEdges.set(pendingEdges);
			pendingEdges = [];
		}
	});

	// --- Main layout pipeline ---

	async function loadTopologyData() {
		// Wait for containerElement to be available (bind:this fires after mount)
		if (!containerElement) {
			await tick();
			if (!containerElement) return;
		}
		const thisGeneration = ++layoutState.layoutGeneration;
		const isStale = (): boolean => thisGeneration !== layoutState.layoutGeneration;

		if (!topology || (!topology.edges && !topology.nodes)) return;
		perf.beginRun();
		// Fire-and-forget: elkjs is a large module and the measure pass below takes
		// far longer than loading it, so the two should overlap rather than run
		// back to back.
		preloadElk();

		const prepareDone = perf.stage('prepare');
		const prep = prepareTopologyData(topology, layoutState, getInfrastructureRuleId);
		prepareDone();
		// Inputs are fixed once prepare has run — it is the last stage that writes
		// to the watched stores as part of its own work. Snapshot here so those
		// self-writes don't read as external change at the end of the run.
		inFlightInputs = snapshotReloadInputs(currentReloadInputs());
		noteRunDetail({ isNewStructure: prep?.isNewStructure, needsElk: prep?.needsElk });
		if (!prep) return;
		const { needsElk, collapsed, visibleNodes: initialVisibleNodes } = prep;
		let visibleNodes = initialVisibleNodes;

		// Sizes measured by *this* run, once it has measured. Preferred over the view cache so the
		// same frame that lays a node out can also size it — which is what lets culling work on
		// the first render after an expand rather than only after everything has mounted once.
		const viewCacheKey = `${prep.currentView}:${prep.topologyId}`;
		let runSizes: Map<string, XY> | null = null;

		// Helper: build positioned flow nodes (called multiple times with different useGraph)
		const makeNodes = (useGraph: boolean) =>
			sortFlowNodes(
				buildFlowNodes({
					visibleNodes,
					collapsed,
					topology,
					useGraph,
					layoutGraph: layoutState.layoutGraph,
					isNewStructure: prep.isNewStructure,
					liveNodes: getNodes(),
					infraRuleId: getInfrastructureRuleId(),
					editMode: editMode ?? false,
					sizeHints: runSizes ?? layoutState.viewSizeCache.get(viewCacheKey) ?? null
				})
			);

		if (needsElk) {
			const measureDone = perf.stage('measure');
			const elementNodeSizes = await resolveNodeSizes(
				layoutState,
				prep,
				topology,
				containerElement,
				isStale,
				{
					setMeasuring: (v) => {
						// Culling is suspended for every measurement pass, unconditionally —
						// the pass reads heights out of the DOM, so a culled node measures as
						// absent and ELK gets a fallback size for it.
						measurePassActive = v;
						// Only hide viewport during measurement for initial load
						// (no nodes on screen). For subsequent measurements (e.g.
						// cacheMisses on collapse), nodes keep their current positions
						// so hiding is unnecessary — and skipping it lets shouldAnimate
						// fire normally.
						if (layoutState.lastRenderedTopoKey === '') {
							coldLoadMeasure = v;
						}
					},
					setNodes: (n) => setStoreNodes(n),
					setEdges: (e) => baseFlowEdges.set(e),
					buildMeasureNodes: () => {
						// Strip the seeded sizes. The pass exists to learn what a card actually
						// renders as, and a node carrying `measured` + `handles` reads as already
						// initialised — so it would be presented at the size we guessed and could
						// only ever confirm that guess. Dropping both also re-attaches
						// `NodeWrapper`'s ResizeObserver, which is otherwise never reattached.
						const measureNodes = stripSizeSeed(makeNodes(false));
						// Preserve current positions during measurement — DOM
						// measurement only needs element presence, not positions.
						// This prevents nodes from jumping to (0,0) while visible.
						const currentPositions = new Map(getNodes().map((n) => [n.id, n.position]));
						if (currentPositions.size === 0) return measureNodes;
						return measureNodes.map((n) => ({
							...n,
							position: currentPositions.get(n.id) ?? n.position
						}));
					},
					waitForNodesRendered: async (expectedIds?: Set<string>) => {
						// Wait for SvelteFlow to render node DOM elements.
						// We only need DOM presence for measurement, not full initialization.
						await tick();
						// Poll for DOM nodes with a short timeout — nodesInitialized
						// can hang indefinitely for large topologies.
						const start = performance.now();
						const expectedCount = expectedIds?.size ?? 0;
						while (performance.now() - start < 2000) {
							const nodeEls = containerElement?.querySelectorAll('.svelte-flow__node');
							if (nodeEls && nodeEls.length > 0) {
								if (expectedCount === 0) break;
								// Cheap gate first. Verifying every expected id means building
								// a Set of all rendered ids, which is O(nodes) — doing that on
								// every frame of a wait that can span a hundred frames is
								// itself a meaningful cost at a thousand-plus nodes. The DOM
								// can only hold every expected node once it holds at least
								// that many, so the count check rules out almost every frame
								// for the price of a property read.
								if (nodeEls.length >= expectedCount) {
									// Require every expected id to be present before breaking.
									// Breaking on the first node (old-render leftovers) lets a
									// newly-added SSE host miss measurement, so ELK falls back
									// to metadata defaults and positions siblings too close.
									const present = new Set(
										Array.from(nodeEls)
											.map((el) => (el as HTMLElement).dataset.id)
											.filter((id): id is string => !!id)
									);
									let allPresent = true;
									for (const id of expectedIds!) {
										if (!present.has(id)) {
											allPresent = false;
											break;
										}
									}
									if (allPresent) break;
								}
							}
							await new Promise((r) => requestAnimationFrame(r));
						}
					}
				}
			);
			measureDone();
			// The measurement pass is over the moment `resolveNodeSizes` returns — it has already
			// read every height it needed out of the DOM — so culling resumes here rather than at
			// the end of the render.
			//
			// It must be cleared on this path, not only in the branches below. `resolveNodeSizes`
			// calls `setMeasuring(false)` only when it goes stale, and of the render branches only
			// the cold-load one ends with `endMeasurePass()`. A measure pass on an already-rendered
			// graph — every expand at scale — therefore left the flag set for the rest of the
			// session, suspending culling permanently and mounting the whole graph on every run.
			measurePassActive = false;
			if (!elementNodeSizes) {
				endMeasurePass();
				return;
			}
			runSizes = elementNodeSizes;

			const layoutDone = perf.stage('layout');
			const layoutResult = await executeLayout(
				topology,
				layoutState,
				prep,
				elementNodeSizes,
				isStale
			);
			layoutDone();
			if (!layoutResult) {
				endMeasurePass();
				return;
			}
			visibleNodes = layoutResult.visibleNodes;
		}

		// Port expansion handling (no full ELK re-layout).
		//
		// This is the pipeline's second DOM measurement site: it mounts the rebuilt nodes and reads
		// `offsetWidth`/`offsetHeight` for the cards whose port list changed. Culling is
		// deliberately *not* suspended for it, unlike the full measurement pass. Suspending it
		// would mount every node in the graph to re-measure a handful of cards, which at this
		// customer's scale is the out-of-memory failure by another route. A card whose ports the
		// user just toggled is on screen by construction, so it is mounted and measures correctly;
		// `handlePortExpansion` drops the cached size of any id it could not find in the DOM, so a
		// card toggled and then scrolled away from re-measures for real the next time it mounts
		// rather than keeping a stale height.
		const currentExpandedPorts = get(expandedPortNodeIds);
		const portsChanged = await handlePortExpansion(
			layoutState,
			currentExpandedPorts,
			containerElement,
			() => makeNodes(false),
			(n) => setStoreNodes(n),
			isStale,
			needsElk,
			viewCacheKey
		);

		// Build final nodes and edges. Edge handles are computed inside
		// buildFlowEdges against final post-layout positions (from layoutGraph)
		// rather than being precomputed by the layout engines.
		const needsLayout = needsElk || portsChanged || prep.collapseChanged;
		const allNodes = makeNodes(needsLayout);

		const buildEdgesDone = perf.stage('build-edges');
		const { flowEdges, originalsMap } = buildFlowEdges({
			elevatedEdges: prep.elevatedEdges,
			collapsed,
			elementToContainer: prep.elementToContainer,
			aggregatedEdges: prep.aggregatedEdges,
			hiddenEdgeTypes: prep.hiddenEdgeTypes,
			layoutNodes: prep.layoutNodes,
			view: prep.currentView,
			layoutGraph: layoutState.layoutGraph,
			bundleEnabled: $topologyOptions.local.bundle_edges ?? false,
			currentExpandedBundles: get(expandedBundles),
			selectionStores
		});
		buildEdgesDone();
		aggregatedEdgeOriginals.set(originalsMap);

		// Publish the handles each node's edges name, before the nodes that reference them. Node
		// components render only these; SvelteFlow reads handle boxes out of the DOM, and only for
		// a handle an edge names, so a node whose handles arrive a frame after its edge has that
		// edge dropped.
		edgeHandlesByNode.set(collectEdgeHandles(flowEdges));

		// Render
		const shouldAnimate =
			needsElk && !coldLoadMeasure && layoutState.lastRenderedTopoKey !== '' && !prep.viewChanged;

		if (shouldAnimate) {
			animatingCollapse = true;
			const previousNodeIds = new Set(get(nodes).map((n) => n.id));
			const phase1Nodes = allNodes.filter((n) => previousNodeIds.has(n.id));
			setStoreNodes(phase1Nodes);
			baseFlowEdges.set(flowEdges);

			const fullNodes = [...allNodes];
			const fullEdges = [...flowEdges];
			// Await phase 2 + one rAF so new nodes are in the DOM with final
			// sizes before the post-render measurement below runs. Without
			// this, cacheCollapsedSizes would either measure pre-phase-2 state
			// (no new nodes) or be skipped entirely — letting ELK's fallback
			// sizes for fresh SSE hosts persist and produce the overlaps.
			await new Promise<void>((resolve) => {
				setTimeout(() => {
					animatingCollapse = false;
					const newNodeIds = new Set(
						fullNodes.filter((n) => !previousNodeIds.has(n.id)).map((n) => n.id)
					);
					if (newNodeIds.size > 0) {
						const fadingNodes = fullNodes.map((n) =>
							newNodeIds.has(n.id)
								? { ...n, style: 'opacity: 0; transition: opacity 0.3s ease-in-out;' }
								: n
						);
						setStoreNodes(fadingNodes);
						baseFlowEdges.set(fullEdges);
						requestAnimationFrame(() => {
							setStoreNodes(fullNodes);
							baseFlowEdges.set(fullEdges);
							requestAnimationFrame(() => resolve());
						});
					} else {
						setStoreNodes(fullNodes);
						baseFlowEdges.set(fullEdges);
						requestAnimationFrame(() => resolve());
					}
				}, 350);
			});
			if (isStale()) return;
		} else if (!coldLoadMeasure) {
			setStoreNodes(allNodes);
			baseFlowEdges.set(flowEdges);
		} else {
			baseFlowEdges.set([]);
			setStoreNodes(allNodes);
			pendingEdges = flowEdges;
			await tick();
			if (isStale()) {
				endMeasurePass();
				return;
			}
			if (pendingEdges.length > 0) {
				baseFlowEdges.set(pendingEdges);
				pendingEdges = [];
			}
			await tick();
			await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
			if (isStale()) {
				endMeasurePass();
				return;
			}
			endMeasurePass();
		}

		// Post-render: measure collapsed containers at their natural content
		// size (width:auto / height:auto) and trigger a corrective re-layout
		// when new entries are found. This self-heals any case where ELK's
		// first pass used stale/fallback sizes — e.g. a fresh SSE host whose
		// DOM wasn't reconciled in time for the measurement pass. Runs in
		// every branch; the animation branch above awaits phase 2 so new
		// nodes are in the DOM by the time we measure.
		if (containerElement && layoutState.layoutGraph) {
			await tick();
			const cacheSizesDone = perf.stage('post-render.cache-collapsed');
			const newEntries = cacheCollapsedSizes(
				containerElement,
				layoutState.layoutGraph,
				collapsed,
				layoutState.containerSizeCache
			);

			// Correct any cached card height the seeded sizes have drifted from. Nodes built with
			// `measured` never get a ResizeObserver, so this is the only thing that notices — see
			// `reconcileMeasuredSizes`. Bounded by the mounted set, and it converges, so a drift
			// costs one extra layout rather than repeating.
			const viewCache = layoutState.viewSizeCache.get(viewCacheKey);
			const drifted = viewCache
				? reconcileMeasuredSizes(
						containerElement,
						viewCache,
						layoutState.layoutGraph,
						layoutState.driftCorrectedIds
					)
				: 0;
			if (drifted > 0) perf.count('post-render.size-drift');

			cacheSizesDone();
			// Read the layout *model*, not the DOM: this separates "the graph lost its sizes" from
			// "the render has not caught up", which the per-sample degenerate count cannot.
			noteRunDetail({
				// Expanded containers only. A collapsed one has never been laid out expanded, so a
				// zero there is normal and would drown the signal — it is the *expanded* container
				// with no size that renders as its borders with its contents outside.
				containersZeroSizedAfter: [...layoutState.layoutGraph.containers.values()].filter(
					(c) => !c.collapsed && c.expandedSize.width === 0
				).length
			});
			if ((newEntries > 0 || drifted > 0) && !isStale()) {
				// Counted because on a cold load with many collapsed containers this
				// self-heal fires every time, and each recursion is a full pipeline
				// run including two more elk.layout() calls.
				perf.count('post-render-relayout');
				// Invalidate structureKey to force ELK re-run. Do NOT
				// invalidate baseKey — base structure hasn't changed, and
				// clearing it would delete viewSizeCache (element sizes).
				layoutState.sessionStructureKey = '';
				// Preserve fitView intent across the recursive call — the
				// re-run won't see viewChanged/topologyChanged since we
				// update the tracking keys here.
				if (prep.viewChanged || prep.topologyChanged) {
					layoutState.fitViewPending = true;
				}
				layoutState.lastRenderedTopoKey = prep.topoKey;
				layoutState.lastRenderedView = prep.currentView;
				await loadTopologyData();
				return;
			}
		}

		const isFirstRender = layoutState.lastRenderedTopoKey === '';
		layoutState.lastRenderedTopoKey = prep.topoKey;
		layoutState.lastRenderedView = prep.currentView;

		if (prep.viewChanged || prep.topologyChanged || isFirstRender || layoutState.fitViewPending) {
			layoutState.fitViewPending = false;
			// Double rAF: first lets SvelteFlow process node positions, second triggers fitView
			requestAnimationFrame(() =>
				requestAnimationFrame(() => {
					fitView({ padding: getFitViewPadding() });
					// fitView is the last thing a cold load does, so this is the
					// point the harness treats as "interactive".
					perf.count('fit-view');
					perf.endRun();
					// …and the first moment the canvas is final, so the honest place to ask
					// whether anything is actually on it.
					recordAfterRun(diagnosticInputs());
				})
			);
		} else {
			perf.endRun();
			recordAfterRun(diagnosticInputs());
		}
	}

	// --- Event handlers ---

	let ignoreNextSelectionChange = false;

	function handleNodeClick({ node, event }: { node: Node; event: MouseEvent | TouchEvent }) {
		if (viewportMoved) return;
		// Multi-select drives dependency creation, so it's an edit action: disable
		// it when read-only (snapshot / share). Single-select (read-only inspect)
		// still works.
		const isModifierClick =
			event instanceof MouseEvent && (event.ctrlKey || event.metaKey) && !$topologyReadOnly;
		if (isModifierClick) {
			handleModifierNodeClick(node, selectionStores);
			ignoreNextSelectionChange = true;
		} else {
			collapseAllBundles();
			selectNode(node, selectionStores);
			ignoreNextSelectionChange = true;
		}
	}

	function handleEdgeClick({ edge }: { edge: Edge; event: MouseEvent }) {
		if (viewportMoved) return;
		collapseAllBundles();
		selectEdge(edge, selectionStores);
		ignoreNextSelectionChange = true;
	}

	function handleMove() {
		viewportMoved = true;
		if (viewportMoveTimer) {
			clearTimeout(viewportMoveTimer);
			viewportMoveTimer = null;
		}
	}

	function handleMoveEnd() {
		viewportMoveTimer = setTimeout(() => {
			viewportMoved = false;
		}, 50);
		// Moving the viewport is what re-evaluates which nodes are inside it, so it is the other
		// moment a canvas can go blank — and the one a customer reported as "locking it in".
		recordAfterViewportMove(diagnosticInputs());
		refreshCollapsedSizesInView();
	}

	/**
	 * Record the real collapsed size of any container that has just come into view.
	 *
	 * `cacheCollapsedSizes` reads the DOM, so with culling on it only ever sees mounted containers
	 * — and at scale most collapsed containers are off screen, so they are laid out from their
	 * type's declared size instead (see `measure.ts`). Running it again as the user pans replaces
	 * those estimates with measurements without ever mounting the whole graph. Throttled because
	 * a pan fires move-end repeatedly, and deliberately silent: it fills the cache for the *next*
	 * layout rather than triggering one, since re-laying out mid-pan is exactly the jank the
	 * culling work exists to remove.
	 */
	const refreshCollapsedSizesInView = throttle(
		() => {
			if (!containerElement || !layoutState.layoutGraph) return;
			cacheCollapsedSizes(
				containerElement,
				layoutState.layoutGraph,
				get(collapsedContainers),
				layoutState.containerSizeCache
			);
		},
		COLLAPSED_SIZE_REFRESH_MS,
		{ leading: false, trailing: true }
	);

	function syncEdgeDisplayState() {
		const current = get(baseFlowEdges);
		const updated = computeEdgeDisplayUpdates(
			current,
			get(selectionStores.selectedNode),
			get(selectionStores.selectedEdge),
			get(searchHiddenNodeIds),
			get(tagHiddenNodeIds)
		);
		// Identity means nothing changed. Skipping the write matters on the hover
		// path, which calls this on every pointer enter/leave.
		if (updated !== current) baseFlowEdges.set(updated);
	}

	function handlePaneClick() {
		if (!viewportMoved) {
			clearSelection(selectionStores);
			clearEdgeHoverState();
			syncEdgeDisplayState();
		}
		viewportMoved = false;
		if (viewportMoveTimer) {
			clearTimeout(viewportMoveTimer);
			viewportMoveTimer = null;
		}
	}

	function handleEdgePointerEnter({ edge }: { edge: Edge }) {
		setEdgeHover(edge, true, get(edges));
		syncEdgeDisplayState();
	}

	function handleEdgePointerLeave({ edge }: { edge: Edge }) {
		setEdgeHover(edge, false, get(edges));
		syncEdgeDisplayState();
	}

	function handleSelectionChange({ nodes: selNodes }: { nodes: Node[]; edges: Edge[] }) {
		if (ignoreNextSelectionChange) {
			ignoreNextSelectionChange = false;
			return;
		}
		if (selNodes.length === 0 && !viewportMoved) {
			tick().then(() => {
				// Skip if a click handler has set an active selection, or a multi-selection is active
				if (
					get(selectionStores.selectedNode) ||
					get(selectionStores.selectedEdge) ||
					get(selectionStores.selectedNodes).length > 0
				)
					return;
				clearSelection(selectionStores);
				clearEdgeHoverState();
				syncEdgeDisplayState();
			});
			return;
		}
		handleBoxSelect(selNodes, selectionStores);
	}

	function handleNodeDragStop({
		targetNode
	}: {
		targetNode: Node | null;
		nodes: Node[];
		event: MouseEvent | TouchEvent;
	}) {
		if (onNodeDragStop && targetNode) onNodeDragStop(targetNode);
	}

	function handleReconnect(edge: Edge, newConnection: Connection) {
		if (onReconnect) onReconnect(edge, newConnection);
	}

	// --- Collapse controls ---

	function getCollapseLevelName(level: CollapseLevel): string {
		switch (level) {
			case 1:
				return topology_levelFullyCollapsed();
			case 2:
				return topology_levelContainersExpanded();
			case 3:
				return topology_levelSubcontainersExpanded();
			case 4:
				return topology_levelFullyExpanded();
		}
	}

	// Disabled when nothing further would change, not merely when the level is at its numeric
	// end. A view whose only root is collapsed_by_default has fewer distinct states than the
	// ladder has rungs, so the button can run out before the number does — and a button that
	// still looks live while doing nothing is worse than one that greys out.
	// `$collapsedContainers` is read so this re-evaluates when the rendered set changes.
	let expandDisabled = $derived(
		!!editMode ||
			($collapsedContainers &&
				nextEffectiveLevel('expand', topology.nodes, containerTypes, getInfrastructureRuleId()) ===
					null)
	);
	let collapseDisabled = $derived(
		!!editMode ||
			($collapsedContainers &&
				nextEffectiveLevel(
					'collapse',
					topology.nodes,
					containerTypes,
					getInfrastructureRuleId()
				) === null)
	);
	let collapseLevelTooltipCollapse = $derived(
		$collapseLevel > 1
			? `${common_collapse()}: ${getCollapseLevelName(($collapseLevel - 1) as CollapseLevel)}`
			: ''
	);
	let collapseLevelTooltipExpand = $derived(
		$collapseLevel < 4
			? `${common_expand()}: ${getCollapseLevelName(($collapseLevel + 1) as CollapseLevel)}`
			: ''
	);

	// Both step handlers only write the collapse stores; the relayout that
	// follows is a full asynchronous pipeline run (measure pass, ELK, and a
	// 350ms animation phase). Refitting on a fixed timer therefore fits the
	// *previous* graph — and nothing refits afterwards, because a collapse
	// change is neither `viewChanged` nor `topologyChanged` (`getStructureKey`
	// does not include collapse state). Hand the intent to the pipeline instead
	// and let the post-layout branch fit once the new layout exists.
	function handleStepCollapse() {
		if (editMode) return;
		clearSelection(selectionStores);
		stepCollapse(topology.nodes, containerTypes, getInfrastructureRuleId(), (idsLeftExpanded) => {
			for (const id of idsLeftExpanded) layoutState.seenAutoCollapseIds.add(id);
			layoutState.fitViewPending = true;
		});
	}

	function handleStepExpand() {
		if (editMode) return;
		clearSelection(selectionStores);
		// Marked seen before the collapse stores are written, not after: the write runs the
		// pipeline synchronously, so anything done afterwards is too late to affect the run it
		// caused. See `stepExpand`.
		stepExpand(topology.nodes, containerTypes, getInfrastructureRuleId(), (idsLeftExpanded) => {
			for (const id of idsLeftExpanded) layoutState.seenAutoCollapseIds.add(id);
			layoutState.fitViewPending = true;
		});
	}

	export function triggerStepExpand() {
		handleStepExpand();
	}
	export function triggerStepCollapse() {
		handleStepCollapse();
	}

	// Derive the xyflow `edges` store from three reactive sources:
	//   - baseFlowEdges: the real edges produced by the rebuild pipeline
	//   - previewEdges:  preview edges from the dependency editor
	//   - editingDependencyId: when set, hide the real edge for that dep
	// This is the ONLY writer of `edges`. Exits and rebuilds are symmetric:
	// clearing editingDependencyId naturally restores the filtered real edge.
	//
	// Svelte's `$baseFlowEdges` auto-subscribe hits a compiler bug here, so
	// bridge the store into a $state and depend on that in the merge effect.
	let currentBaseFlowEdges = $state<Edge[]>([]);
	$effect(() => {
		return baseFlowEdges.subscribe((v) => {
			currentBaseFlowEdges = v;
		});
	});
	$effect(() => {
		const base = currentBaseFlowEdges;
		const preview = $previewEdges;
		const editingId = $editingDependencyId;
		const aggregatedOriginals = $aggregatedEdgeOriginals;

		// Hide any edge whose underlying dependency is being edited. An edge may
		// directly carry `data.dependency_id` (plain dep edge), be a bundled
		// representative with `data.bundleEdges` (an array of originals), or be
		// an aggregated collapse edge whose originals live in the
		// `aggregatedEdgeOriginals` store (keyed by edge.id).
		const matchesEditingDep = (e: Edge): boolean => {
			const data = e.data as
				| {
						dependency_id?: string;
						bundleEdges?: Array<{ dependency_id?: string }>;
				  }
				| undefined;
			if (data?.dependency_id === editingId) return true;
			if (data?.bundleEdges?.some((o) => o.dependency_id === editingId)) return true;
			const originals = aggregatedOriginals.get(e.id);
			if (originals?.some((o) => (o as { dependency_id?: string }).dependency_id === editingId))
				return true;
			return false;
		};

		const visibleReal = editingId ? base.filter((e) => !matchesEditingDep(e)) : base;
		edges.set([...visibleReal, ...preview]);
	});
</script>

<div
	class="h-full w-full overflow-hidden !p-0"
	class:card={!isEmbed}
	class:card-static={!isEmbed}
	class:collapse-transition={animatingCollapse}
	style:visibility={coldLoadMeasure ? 'hidden' : 'visible'}
	bind:this={containerElement}
>
	<SvelteFlow
		nodes={$nodes}
		edges={$edges}
		{nodeTypes}
		edgeTypes={customEdgeTypes}
		onpaneclick={handlePaneClick}
		onedgeclick={handleEdgeClick}
		onnodeclick={handleNodeClick}
		onedgepointerenter={handleEdgePointerEnter}
		onedgepointerleave={handleEdgePointerLeave}
		onnodedragstop={readonly ? undefined : handleNodeDragStop}
		onreconnect={readonly ? undefined : handleReconnect}
		onselectionchange={handleSelectionChange}
		onmove={handleMove}
		onmoveend={handleMoveEnd}
		fitView={true}
		onlyRenderVisibleElements={cullOffscreen}
		minZoom={0.1}
		noPanClass="nopan"
		snapGrid={[25, 25]}
		nodesDraggable={!readonly}
		nodesConnectable={!readonly}
		elementsSelectable={true}
		selectionOnDrag={true}
		selectionKey="Shift"
		panOnDrag={true}
		zoomOnScroll={true}
		proOptions={{ hideAttribution }}
	>
		<Background
			variant={BackgroundVariant.Dots}
			bgColor="var(--color-topology-bg)"
			gap={50}
			size={1}
		/>

		{#if showControls}
			<TopologySidebarControls
				{editMode}
				{onToggleEditMode}
				{onOpenShortcuts}
				{onOpenSearch}
				{sidebarCollapsed}
				onStepExpand={handleStepExpand}
				onStepCollapse={handleStepCollapse}
				onFitView={() => triggerFitView()}
				{expandDisabled}
				{collapseDisabled}
				collapseLevel={$collapseLevel}
				{collapseLevelTooltipExpand}
				{collapseLevelTooltipCollapse}
			/>
		{/if}

		{#if (showMinimap !== undefined ? showMinimap : $topologyOptions.local.show_minimap) && !$isExporting}
			<MiniMap
				position="bottom-left"
				width={MINIMAP_WIDTH_PX}
				height={MINIMAP_HEIGHT_PX}
				bgColor={themeStore.resolvedTheme === 'dark' ? '#1f2937' : '#ffffff'}
				nodeColor={themeStore.resolvedTheme === 'dark' ? '#6b7280' : '#9ca3af'}
				maskColor={themeStore.resolvedTheme === 'dark'
					? 'rgba(17, 24, 39, 0.7)'
					: 'rgba(243, 244, 246, 0.7)'}
				maskStrokeColor={themeStore.resolvedTheme === 'dark' ? '#374151' : '#d1d5db'}
			/>
		{/if}

		{#if showBranding}
			<a
				href="https://scanopy.net?utm_source={isEmbed
					? 'embed'
					: 'share'}&utm_medium=referral&utm_campaign=created_with"
				target="_blank"
				rel="noopener noreferrer"
				class="branding-badge"
			>
				<img src="/logos/scanopy-logo.png" alt="Scanopy" class="h-4 w-4" />
				<span>Powered by Scanopy</span>
			</a>
		{/if}
	</SvelteFlow>
</div>

<style>
	.branding-badge {
		position: absolute;
		bottom: 10px;
		right: 10px;
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--color-text-muted);
		font-size: 12px;
		text-decoration: none;
		z-index: 5;
		transition: color 0.2s;
	}

	.branding-badge:hover {
		color: var(--color-text-secondary);
	}
</style>
