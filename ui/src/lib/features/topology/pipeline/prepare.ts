import { get } from 'svelte/store';
import type { RenderableTopology } from '../types/base';
import type { LayoutState, PrepareResult } from './types';
import { LayoutGraph } from '../layout/layout-graph';
import {
	collapsedContainers,
	collapseLevel,
	hadStoredLevelOnLoad,
	inferCurrentLevel,
	computeCollapsedForLevel,
	buildElementToContainer,
	computeCollapsedEdges
} from '../collapse';
import { elevateEdgesToContainers } from '../layout/edge-elevation';
import { containerTypes, views } from '$lib/shared/stores/metadata';
import { activeView, topologyOptions } from '../queries';
import { tagHiddenNodeIds } from '../interactions';
import { buildTopologyParentIndex } from '../topology-parent-index';

/**
 * Collections on Topology that can surface as inline entity content on element
 * cards. Keyed by EntityDiscriminants name — matches what views declare in
 * element_config.element_entities[].inline_entities.
 *
 * Entity-registry data, not view-specific: the only knowledge encoded here is
 * "Service entities live in topology.services". Adding a new inlinable entity
 * type is a one-line append.
 */
const INLINE_ENTITY_COLLECTIONS: Record<string, keyof RenderableTopology> = {
	Service: 'services',
	Port: 'ports',
	Interface: 'interfaces',
	IPAddress: 'ip_addresses',
	Host: 'hosts',
	Subnet: 'subnets',
	Binding: 'bindings',
	Dependency: 'dependencies',
	Vlan: 'vlans'
};

/**
 * Build a stable signature of everything the active view inlines on its
 * element cards. Returns '' when no view-declared inline entity types are in
 * play, so L2 / Workloads/Service / Application pay nothing here.
 *
 * Any field change on any inlined entity bumps the signature — that's the
 * design trade-off: view-agnostic, fully deterministic, bounded over-trigger
 * (a service name change that doesn't affect card height still re-layouts).
 */
function getInlineContentKey(topo: RenderableTopology, view: string): string {
	const meta = views.getMetadata(view) as {
		element_config?: {
			element_entities?: Array<{ entity_type: string; inline_entities: string[] }>;
		};
	} | null;
	const entries = meta?.element_config?.element_entities ?? [];
	const inlineTypes = new Set<string>();
	for (const ee of entries) {
		for (const t of ee.inline_entities) inlineTypes.add(t);
	}
	if (inlineTypes.size === 0) return '';

	const sigs: string[] = [];
	for (const type of inlineTypes) {
		const collectionKey = INLINE_ENTITY_COLLECTIONS[type];
		if (!collectionKey) continue;
		const collection = topo[collectionKey] as unknown;
		if (!Array.isArray(collection)) continue;
		for (const entity of collection) {
			sigs.push(JSON.stringify(entity));
		}
	}
	sigs.sort();
	return sigs.join(';');
}

// Tab-scoped guard: only apply the "fresh session" default-level seeding on the
// very first pipeline run of this tab. Subsequent runs (topology switches,
// re-renders) always use the inferrer so a user's explicit level (e.g. after
// stepExpand to 4) is respected on later navigations.
let defaultsAppliedThisSession = false;

/** Signature of the currently filtered-out set. Since the pipeline now
 *  removes these nodes structurally (not just fades), any change here must
 *  trigger a full re-run so ELK sees the new node/edge set. Hashes the
 *  resolved hidden-node set directly — which already reflects tag filters,
 *  category/metadata filters, and entity-hide via updateTagFilter. */
function getHideStateKey(): string {
	const hidden = get(tagHiddenNodeIds);
	if (hidden.size === 0) return '';
	return [...hidden].sort().join(',');
}

function getStructureKey(topo: RenderableTopology, view: string): string {
	const nodeKeys = topo.nodes
		.map((n) => {
			const parentId = n.node_type === 'Element' ? n.container_id : n.parent_container_id;
			return `${n.id}@${parentId ?? ''}`;
		})
		.sort()
		.join(',');
	const inlineKey = getInlineContentKey(topo, view);
	const hideKey = getHideStateKey();
	return `${topo.nodes.length}:${topo.edges.length}:${nodeKeys}|${inlineKey}|${hideKey}`;
}

/**
 * Prepare topology data for layout: validate inputs, manage collapse state,
 * filter nodes, elevate edges, compute structure keys.
 *
 * @returns null to signal "skip this run" (view mismatch, stale data)
 */
export function prepareTopologyData(
	topology: RenderableTopology,
	state: LayoutState,
	getInfrastructureRuleId: () => string | null
): PrepareResult | null {
	const currentView = get(activeView);
	const topoKey = getStructureKey(topology, currentView);
	const viewChanged = state.lastRenderedView !== '' && currentView !== state.lastRenderedView;
	const topologyChanged = topoKey !== state.lastRenderedTopoKey;

	if (topologyChanged) {
		state.viewSizeCache.clear();
		state.containerSizeCache.clear();
		// Remove seenAutoCollapseIds entries that don't exist in the new topology
		const newContainerIds = new Set(
			topology.nodes.filter((n) => n.node_type === 'Container').map((n) => n.id)
		);
		for (const id of state.seenAutoCollapseIds) {
			if (!newContainerIds.has(id)) state.seenAutoCollapseIds.delete(id);
		}
	}

	// Skip if view changed but the enriched topology hasn't re-sliced yet.
	// The topology now carries one node/edge set per view and the active
	// view's slice is selected upstream (toRenderableTopology), so a view switch
	// always changes the structure key — no per-view data-readiness guard
	// is needed here.
	if (viewChanged && !topologyChanged) {
		return null;
	}

	let collapsed = get(collapsedContainers);

	// Drop stale IDs from the persisted collapsed set before any level logic.
	// A set carried over from a different topology (e.g. auth app → share) can
	// contain IDs not present here; the inferrer's "all current containers are
	// collapsed → level 1" fallback then triggers spuriously whenever the stale
	// superset happens to cover every current container.
	if (collapsed.size > 0) {
		const currentContainerIds = new Set(
			topology.nodes.filter((n) => n.node_type === 'Container').map((n) => n.id)
		);
		const stripped = new Set([...collapsed].filter((id) => currentContainerIds.has(id)));
		if (stripped.size !== collapsed.size) {
			collapsedContainers.set(stripped);
			collapsed = stripped;
		}
	}

	// Seed collapse state on first topology render.
	//
	// Truly fresh session (no stored level, first pipeline run this tab):
	// apply the default level's collapsed set so the initial view matches
	// the intended default instead of being inferred as level 4 from an
	// empty collapsed set.
	//
	// Otherwise: infer the level from the persisted collapsed set so any
	// prior user choice (including an explicit stepExpand to 4) is respected.
	if (!state.collapseLevelInferred) {
		state.collapseLevelInferred = true;
		if (!defaultsAppliedThisSession && !hadStoredLevelOnLoad) {
			const defaultLevel = get(collapseLevel);
			const defaultCollapsed = computeCollapsedForLevel(
				defaultLevel,
				topology.nodes,
				containerTypes,
				getInfrastructureRuleId()
			);
			collapsedContainers.set(defaultCollapsed);
			collapsed = defaultCollapsed;
			collapseLevel.set(defaultLevel);
		} else {
			const inferred = inferCurrentLevel(
				collapsed,
				topology.nodes,
				containerTypes,
				getInfrastructureRuleId()
			);
			collapseLevel.set(inferred);
		}
		defaultsAppliedThisSession = true;
	}

	// On view switch, apply the current collapse level to the new view's containers
	if (viewChanged && topologyChanged && state.collapseLevelInferred) {
		const currentLevel = get(collapseLevel);
		const levelCollapsed = computeCollapsedForLevel(
			currentLevel,
			topology.nodes,
			containerTypes,
			getInfrastructureRuleId()
		);
		collapsedContainers.set(levelCollapsed);
		collapsed = levelCollapsed;
	}

	// When topology identity changes, reset tracking and strip stale collapsed IDs
	const topologyId = topology.id ?? '';
	if (topologyId !== state.lastSeenTopologyId && state.lastSeenTopologyId !== '') {
		state.seenAutoCollapseIds = new Set<string>();
		state.containerSizeCache.clear();
		state.collapseLevelInferred = false;

		if (collapsed.size > 0) {
			const newContainerIds = new Set(
				topology.nodes.filter((n) => n.node_type === 'Container').map((n) => n.id)
			);
			const validCollapsed = new Set([...collapsed].filter((id) => newContainerIds.has(id)));
			const staleCount = collapsed.size - validCollapsed.size;

			// If ALL old root containers were collapsed, preserve "overview mode"
			if (state.layoutGraph) {
				const oldRootIds = [...state.layoutGraph.containers.values()]
					.filter((c) => !c.parent)
					.map((c) => c.id);
				const wasFullyCollapsed =
					oldRootIds.length > 0 && oldRootIds.every((id) => collapsed.has(id));
				if (wasFullyCollapsed) {
					const allContainerIds = topology.nodes
						.filter((n) => n.node_type === 'Container')
						.map((n) => n.id);
					const allCollapsed = new Set(allContainerIds);
					collapsedContainers.set(allCollapsed);
					collapseLevel.set(1);
					collapsed = allCollapsed;
					state.fitViewPending = true;
				} else if (staleCount > 0) {
					collapsedContainers.set(validCollapsed);
					collapsed = validCollapsed;
				}
			} else if (staleCount > 0) {
				collapsedContainers.set(validCollapsed);
				collapsed = validCollapsed;
			}
		}
	}
	state.lastSeenTopologyId = topologyId;

	// Filter out nodes hidden by any filter source (tag, category/metadata,
	// entity-hide). Filter = structural remove, uniformly across sources —
	// the node is absent from ELK input, DOM, and edge graph. Fade is now
	// reserved for focus operations (search, selection).
	const hiddenByFilter = get(tagHiddenNodeIds);
	let layoutNodes =
		hiddenByFilter.size > 0
			? topology.nodes.filter((n) => !hiddenByFilter.has(n.id))
			: topology.nodes;

	// Remove subcontainers with no remaining element children
	const subcontainerIds = new Set(
		layoutNodes
			.filter(
				(n) =>
					n.node_type === 'Container' &&
					containerTypes.getMetadata(
						((n as Record<string, unknown>).container_type as string) ?? 'Subnet'
					).is_subcontainer
			)
			.map((n) => n.id)
	);
	if (subcontainerIds.size > 0) {
		const childCounts = new Map<string, number>();
		for (const n of layoutNodes) {
			if (n.node_type === 'Element') {
				const cid = (n as Record<string, unknown>).container_id as string;
				if (subcontainerIds.has(cid)) {
					childCounts.set(cid, (childCounts.get(cid) ?? 0) + 1);
				}
			}
		}
		layoutNodes = layoutNodes.filter(
			(n) =>
				!(
					n.node_type === 'Container' &&
					subcontainerIds.has(n.id) &&
					!childCounts.has(n.id) &&
					!collapsed.has(n.id)
				)
		);
	}

	const elementToContainer = buildElementToContainer(layoutNodes);
	const parentIndex = buildTopologyParentIndex(topology.nodes);
	const hiddenEdgeTypes = get(topologyOptions).local.hide_edge_types ?? [];

	// Elevate edges targeting elements inside absorbing containers.
	// Then drop edges whose endpoints were filtered out so ELK doesn't
	// see orphaned references and the renderer doesn't draw ghost lines.
	const elevatedEdgesRaw = elevateEdgesToContainers(topology.edges, layoutNodes);
	const elevatedEdges =
		hiddenByFilter.size > 0
			? elevatedEdgesRaw.filter(
					(e) => !hiddenByFilter.has(e.source) && !hiddenByFilter.has(e.target)
				)
			: elevatedEdgesRaw;

	// Map containers to themselves for bundling
	for (const node of layoutNodes) {
		if (node.node_type === 'Container' && !elementToContainer.has(node.id)) {
			elementToContainer.set(node.id, node.id);
		}
	}

	// Compute structure and base keys
	// Edge visibility intentionally excluded — layout-affecting edges are always
	// fed to ELK regardless of visibility, so toggling shouldn't trigger rebuild.
	const baseKey = currentView + ':' + topoKey;
	const structureKey = baseKey + ':' + Array.from(collapsed).sort().join(',');
	const isNewStructure = state.sessionStructureKey !== structureKey;
	const isNewBaseStructure = state.sessionBaseKey !== baseKey;

	// Capture expanded sizes/positions before rebuilding the graph — but NOT
	// across a view switch. The existing layoutGraph belongs to the previous
	// view, whose nodes/containers differ from this view's slice; restoring its
	// sizes/positions onto the new view's graph piles children at the origin on
	// the first expand. On a view switch we start fresh (like a reload) and let
	// ELK lay out; each view's persisted positions come from its own backend
	// slice. Same-view re-renders (e.g. expanding a container) still reuse them.
	const prevExpandedSizes = viewChanged
		? undefined
		: state.layoutGraph?.getExpandedContainerSizes();
	const prevChildPositions = viewChanged
		? undefined
		: state.layoutGraph?.getContainerChildPositions();

	// Build/rebuild layout graph when structure changes
	if (!state.layoutGraph || isNewStructure) {
		state.layoutGraph = LayoutGraph.fromTopology(layoutNodes);
	}

	// Defer collapse so ELK runs with everything expanded — only if
	// no expanded size is available from either the graph or the cache.
	let deferCollapse = false;
	if (isNewStructure && collapsed.size > 0) {
		for (const id of collapsed) {
			const hasChildren = layoutNodes.some(
				(n) =>
					(n.node_type === 'Element' && (n as Record<string, unknown>).container_id === id) ||
					(n.node_type === 'Container' && (n as Record<string, unknown>).parent_container_id === id)
			);
			const hasExpandedSize =
				prevExpandedSizes?.has(id) || !!state.containerSizeCache.get(id)?.expanded;
			if (hasChildren && !hasExpandedSize) {
				deferCollapse = true;
				break;
			}
		}
	}

	// Sync collapse state from store -> graph
	let collapseChanged = false;
	if (!deferCollapse) {
		collapseChanged = state.layoutGraph.syncCollapseState(collapsed);
	}

	// Force ELK re-layout when a container was expanded but has no cached layout
	let needsElkForExpand = false;
	if (collapseChanged) {
		for (const c of state.layoutGraph.containers.values()) {
			if (!c.collapsed && c.allChildren.length > 0) {
				const hasZeroExpandedSize = c.expandedSize.width === 0;
				const hasUninitializedChildren = c.childElements.some((el) => el.size.y === 0);
				if (hasZeroExpandedSize || hasUninitializedChildren) {
					needsElkForExpand = true;
					state.seenAutoCollapseIds.add(c.id);
				}
			}
		}
	}

	// Compute aggregated edges for collapsed containers
	const aggregatedEdges = computeCollapsedEdges(
		elevatedEdges,
		collapsed,
		layoutNodes,
		hiddenEdgeTypes,
		parentIndex.parentMap
	);

	const visibleNodes = state.layoutGraph.getVisibleNodes(layoutNodes);

	const isViewTransition = isNewStructure && viewChanged && topologyChanged;
	const needsElk = isNewStructure || needsElkForExpand;

	// Clear view size cache on base structure change
	if (isNewBaseStructure) {
		state.viewSizeCache.delete(`${currentView}:${topology.id}`);
	}

	return {
		layoutNodes,
		collapsed,
		elevatedEdges,
		elementToContainer,
		parentIndex,
		topoKey,
		structureKey,
		baseKey,
		isNewStructure,
		isNewBaseStructure,
		viewChanged,
		topologyChanged,
		deferCollapse,
		needsElkForExpand,
		collapseChanged,
		visibleNodes,
		aggregatedEdges,
		hiddenEdgeTypes,
		prevExpandedSizes,
		prevChildPositions,
		currentView,
		topologyId: topology.id ?? '',
		needsElk,
		isViewTransition
	};
}
