/**
 * Collapse state management for C4 Context Zoom.
 *
 * Tracks which containers are collapsed, persists to localStorage,
 * and provides edge aggregation for collapsed containers.
 *
 * Supports leveled collapse/expand with 4 levels:
 *   1 = Fully collapsed
 *   2 = Containers expanded, subcontainers collapsed
 *   3 = Subcontainers expanded (except collapsed-by-default and infrastructure)
 *   4 = Fully expanded
 */

import { get, writable } from 'svelte/store';
import { browser } from '$app/environment';
import type { TopologyEdge, TopologyNode } from './types/base';
import type { ContainerTypeMetadata } from '$lib/shared/stores/metadata';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface AggregatedEdge {
	id: string;
	source: string;
	target: string;
	count: number;
	originalEdges: TopologyEdge[];
}

export type CollapseLevel = 1 | 2 | 3 | 4;

interface ContainerTypesAccessor {
	getMetadata: (id: string | null) => ContainerTypeMetadata;
}

// ---------------------------------------------------------------------------
// Store & persistence
// ---------------------------------------------------------------------------

const COLLAPSED_STORAGE_KEY = 'scanopy_topology_collapsed_containers';
const LEVEL_STORAGE_KEY = 'scanopy_topology_collapse_level';

function loadCollapsedFromStorage(): Set<string> {
	if (!browser) return new Set();
	try {
		const stored = localStorage.getItem(COLLAPSED_STORAGE_KEY);
		if (stored !== null) {
			const arr = JSON.parse(stored);
			if (Array.isArray(arr)) return new Set(arr);
		}
	} catch (error) {
		console.warn('Failed to load collapsed containers from localStorage:', error);
	}
	return new Set();
}

function saveCollapsedToStorage(collapsed: Set<string>): void {
	if (!browser) return;
	try {
		localStorage.setItem(COLLAPSED_STORAGE_KEY, JSON.stringify([...collapsed]));
	} catch (error) {
		console.error('Failed to save collapsed containers to localStorage:', error);
	}
}

function loadLevelFromStorage(): CollapseLevel | null {
	if (!browser) return null;
	try {
		const stored = localStorage.getItem(LEVEL_STORAGE_KEY);
		if (stored !== null) {
			const num = parseInt(stored, 10);
			if (num >= 1 && num <= 4) return num as CollapseLevel;
		}
	} catch {
		// ignore
	}
	return null;
}

function saveLevelToStorage(level: CollapseLevel): void {
	if (!browser) return;
	try {
		localStorage.setItem(LEVEL_STORAGE_KEY, String(level));
	} catch {
		// ignore
	}
}

export const collapsedContainers = writable<Set<string>>(loadCollapsedFromStorage());

// Persist on change (skip first subscription call)
let collapsedInitialized = false;
if (browser) {
	collapsedContainers.subscribe((value) => {
		if (collapsedInitialized) {
			saveCollapsedToStorage(value);
		}
		collapsedInitialized = true;
	});
}

const initialStoredLevel = loadLevelFromStorage();

// Module-load snapshot: was a level persisted in localStorage when the tab opened?
// The pipeline uses this to distinguish a truly fresh session from one where the
// user previously chose a level. Without this signal, an empty collapsed set on
// first render gets inferred as level 4 and overrides the default.
export const hadStoredLevelOnLoad = initialStoredLevel !== null;

export const collapseLevel = writable<CollapseLevel>(initialStoredLevel ?? 3);

if (browser) {
	let levelInitialized = false;
	collapseLevel.subscribe((value) => {
		if (levelInitialized) {
			saveLevelToStorage(value);
		}
		levelInitialized = true;
	});
}

// ---------------------------------------------------------------------------
// Level computation
// ---------------------------------------------------------------------------

/**
 * Check if a container node is an "auto-collapse" candidate:
 * either collapsed_by_default or matches the infrastructure rule.
 */
function isAutoCollapseContainer(
	node: TopologyNode,
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): boolean {
	if (node.node_type !== 'Container') return false;
	const data = node as Record<string, unknown>;
	const ct = data.container_type as string | undefined;
	if (ct && containerTypesStore.getMetadata(ct).collapsed_by_default === true) return true;
	if (infraRuleId && data.element_rule_id === infraRuleId) return true;
	return false;
}

/**
 * Compute the set of container IDs that should be collapsed at a given level.
 */
export function computeCollapsedForLevel(
	level: CollapseLevel,
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): Set<string> {
	const containers = allNodes.filter((n) => n.node_type === 'Container');

	switch (level) {
		case 1: {
			// All containers collapsed
			return new Set(containers.map((n) => n.id));
		}
		case 2: {
			// Root containers expanded except auto-collapse ones (collapsed_by_default
			// / infrastructure); all subcontainers collapsed. Auto-collapse containers
			// stay collapsed at every level except 4 (fully expanded).
			const collapsed = new Set<string>();
			for (const node of containers) {
				const data = node as Record<string, unknown>;
				const ct = data.container_type as string | undefined;
				const isSub = ct ? containerTypesStore.getMetadata(ct).is_subcontainer : false;
				if (isSub || isAutoCollapseContainer(node, containerTypesStore, infraRuleId)) {
					collapsed.add(node.id);
				}
			}
			return collapsed;
		}
		case 3: {
			// Subcontainers expanded except collapsed-by-default and infrastructure
			const collapsed = new Set<string>();
			for (const node of containers) {
				if (isAutoCollapseContainer(node, containerTypesStore, infraRuleId)) {
					collapsed.add(node.id);
				}
			}
			return collapsed;
		}
		case 4: {
			// Everything expanded
			return new Set();
		}
	}
}

/**
 * Infer the closest collapse level from the current collapsed set.
 * Returns exact match only; defaults to 1 if no match.
 */
export function inferCurrentLevel(
	collapsed: Set<string>,
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): CollapseLevel {
	// Check from most expanded to most collapsed
	for (const level of [4, 3, 2, 1] as CollapseLevel[]) {
		const expected = computeCollapsedForLevel(level, allNodes, containerTypesStore, infraRuleId);
		if (setsEqual(collapsed, expected)) return level;
	}
	// No exact match — determine closest by checking which level would expand more
	// If nothing is collapsed, that's level 4
	if (collapsed.size === 0) return 4;
	// If all containers are collapsed, that's level 1
	const allContainers = allNodes.filter((n) => n.node_type === 'Container');
	if (allContainers.every((n) => collapsed.has(n.id))) return 1;
	// Default: level 3 (collapsed-by-default subcontainers only).
	// This is the natural state — persisted set may not exactly match
	// any level due to stale IDs or manual user collapses.
	return 3;
}

function setsEqual(a: Set<string>, b: Set<string>): boolean {
	if (a.size !== b.size) return false;
	for (const item of a) {
		if (!b.has(item)) return false;
	}
	return true;
}

/**
 * Get the IDs of auto-collapse containers (for marking as seen when at level 4).
 */
export function getAutoCollapseIds(
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): string[] {
	return allNodes
		.filter((n) => isAutoCollapseContainer(n, containerTypesStore, infraRuleId))
		.map((n) => n.id);
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export function toggleCollapse(containerId: string, allNodes?: TopologyNode[]): void {
	collapsedContainers.update((set) => {
		const next = new Set(set);
		if (next.has(containerId)) {
			// Expanding: also expand child containers (subgroups)
			next.delete(containerId);
			if (allNodes) {
				for (const node of allNodes) {
					if (node.node_type === 'Container') {
						const parentId = (node as Record<string, unknown>).parent_container_id as
							| string
							| undefined;
						if (parentId === containerId) next.delete(node.id);
					}
				}
			}
		} else {
			// Collapsing: also collapse child containers (subgroups)
			next.add(containerId);
			if (allNodes) {
				for (const node of allNodes) {
					if (node.node_type === 'Container') {
						const parentId = (node as Record<string, unknown>).parent_container_id as
							| string
							| undefined;
						if (parentId === containerId) next.add(node.id);
					}
				}
			}
		}
		return next;
	});
}

/**
 * Step expand level up by one. Returns the new level and the set of
 * auto-collapse IDs that should be marked as "seen" (relevant at level 4).
 */
export function stepExpand(
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): { newLevel: CollapseLevel; autoCollapseIds: string[] } {
	const current = get(collapseLevel);
	const newLevel = Math.min(current + 1, 4) as CollapseLevel;
	const collapsed = computeCollapsedForLevel(newLevel, allNodes, containerTypesStore, infraRuleId);
	collapsedContainers.set(collapsed);
	collapseLevel.set(newLevel);

	// At level 4, return auto-collapse IDs so caller can mark them as seen
	const autoCollapseIds =
		newLevel === 4 ? getAutoCollapseIds(allNodes, containerTypesStore, infraRuleId) : [];
	return { newLevel, autoCollapseIds };
}

/**
 * Step collapse level down by one.
 */
export function stepCollapse(
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): { newLevel: CollapseLevel } {
	const current = get(collapseLevel);
	const newLevel = Math.max(current - 1, 1) as CollapseLevel;
	const collapsed = computeCollapsedForLevel(newLevel, allNodes, containerTypesStore, infraRuleId);
	collapsedContainers.set(collapsed);
	collapseLevel.set(newLevel);
	return { newLevel };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Build a mapping from element node ID to its parent container ID.
 */
export function buildElementToContainer(nodes: TopologyNode[]): Map<string, string> {
	const map = new Map<string, string>();
	for (const node of nodes) {
		if (node.node_type === 'Element') {
			const parentId =
				(node as Record<string, unknown>).container_id ??
				(node as Record<string, unknown>).subnet_id;
			if (typeof parentId === 'string') {
				map.set(node.id, parentId);
			}
		}
	}
	return map;
}

/**
 * Resolve a node ID to its nearest collapsed ancestor.
 * Walks up the parent chain (element → container → parent_container → …)
 * and returns the outermost collapsed container, or null if none.
 */
export function resolveCollapsedAncestor(
	nodeId: string,
	collapsed: Set<string>,
	parentMap: Map<string, string>
): string | null {
	let current = nodeId;
	let result: string | null = null;
	const visited = new Set<string>();

	while (current && !visited.has(current)) {
		visited.add(current);
		if (collapsed.has(current)) {
			result = current;
		}
		const parent = parentMap.get(current);
		if (!parent || parent === current) break;
		current = parent;
	}

	return result;
}

/**
 * Build a parent map covering both Element and Container nodes.
 * Elements map to their container_id/subnet_id, containers map to parent_container_id.
 */
export function buildFullParentMap(nodes: TopologyNode[]): Map<string, string> {
	const parentMap = new Map<string, string>();
	for (const node of nodes) {
		if (node.node_type === 'Element') {
			const parentId =
				(node as Record<string, unknown>).container_id ??
				(node as Record<string, unknown>).subnet_id;
			if (typeof parentId === 'string') {
				parentMap.set(node.id, parentId);
			}
		} else if (node.node_type === 'Container') {
			const parentId = (node as Record<string, unknown>).parent_container_id as string | undefined;
			if (parentId) {
				parentMap.set(node.id, parentId);
			}
		}
	}
	return parentMap;
}

/**
 * Compute aggregated edges for collapsed containers.
 *
 * - Resolves each edge endpoint to its nearest collapsed ancestor
 *   (works for elements, subcontainers, and containers at any nesting depth)
 * - Groups edges between the same pair of (resolved) nodes
 * - Returns aggregated edges with count
 */
export function computeCollapsedEdges(
	edges: TopologyEdge[],
	collapsed: Set<string>,
	nodes: TopologyNode[],
	hiddenEdgeTypes: string[],
	prebuiltParentMap?: Map<string, string>
): AggregatedEdge[] {
	if (collapsed.size === 0) return [];

	const parentMap = prebuiltParentMap ?? buildFullParentMap(nodes);

	const hiddenSet = new Set(hiddenEdgeTypes);

	// Cache resolved ancestors
	const ancestorCache = new Map<string, string | null>();
	function getCollapsedAncestor(nodeId: string): string | null {
		if (ancestorCache.has(nodeId)) return ancestorCache.get(nodeId)!;
		const result = resolveCollapsedAncestor(nodeId, collapsed, parentMap);
		ancestorCache.set(nodeId, result);
		return result;
	}

	// Group by resolved (source, target) pair
	const groups = new Map<string, TopologyEdge[]>();

	for (const edge of edges) {
		let src = edge.source as string;
		let tgt = edge.target as string;

		// Remap to nearest collapsed ancestor
		const srcAncestor = getCollapsedAncestor(src);
		if (srcAncestor) src = srcAncestor;
		const tgtAncestor = getCollapsedAncestor(tgt);
		if (tgtAncestor) tgt = tgtAncestor;

		// Skip if neither endpoint was remapped (edge is fully outside collapsed containers)
		if (!srcAncestor && !tgtAncestor) continue;

		// Skip self-loops (both endpoints inside same collapsed container)
		if (src === tgt) continue;

		// Normalize key so (A,B) and (B,A) are the same group
		const key = src < tgt ? `${src}->${tgt}` : `${tgt}->${src}`;

		let group = groups.get(key);
		if (!group) {
			group = [];
			groups.set(key, group);
		}
		group.push(edge);
	}

	const result: AggregatedEdge[] = [];
	let idx = 0;

	for (const [key, groupEdges] of groups) {
		// If all edges in this group are hidden types, skip
		const visibleEdges = groupEdges.filter((e) => !hiddenSet.has(e.edge_type));
		if (visibleEdges.length === 0) continue;

		const [src, tgt] = key.split('->');
		result.push({
			id: `collapsed-edge-${idx++}`,
			source: src,
			target: tgt,
			count: visibleEdges.length,
			originalEdges: visibleEdges
		});
	}

	return result;
}
