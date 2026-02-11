import { writable, get } from 'svelte/store';
import type { Edge } from '@xyflow/svelte';
import type { Node } from '@xyflow/svelte';
import type { QueryClient } from '@tanstack/svelte-query';
import { edgeTypes, subnetTypes } from '$lib/shared/stores/metadata';
import type { TopologyEdge, TopologyNode, Topology } from './types/base';
import { getHostFromInterfaceIdFromCache } from '../hosts/queries';
import {
	getInterfacesForHostFromCache,
	getInterfacesForSubnetFromCache
} from '../interfaces/queries';
import { getSubnetByIdFromCache } from '../subnets/queries';

// Shared stores for hover state across all component instances
export const groupHoverState = writable<Map<string, boolean>>(new Map());
export const edgeHoverState = writable<Map<string, boolean>>(new Map());
export const connectedNodeIds = writable<Set<string>>(new Set());
export const isExporting = writable(false);

// Tag filter stores - nodes/services hidden by tag filter
export const tagHiddenNodeIds = writable<Set<string>>(new Set());
export const tagHiddenServiceIds = writable<Set<string>>(new Set());

// Special sentinel value for "Untagged" pseudo-tag
export const UNTAGGED_SENTINEL = '__untagged__';

// Tag hover state for highlighting nodes with a specific tag
export interface HoveredTag {
	tagId: string;
	color: string;
	entityType: 'host' | 'service' | 'subnet';
}
export const hoveredTag = writable<HoveredTag | null>(null);

interface TagFilter {
	hidden_host_tag_ids?: string[];
	hidden_service_tag_ids?: string[];
	hidden_subnet_tag_ids?: string[];
}

/**
 * Update hidden nodes/services based on tag filter settings.
 * - Hosts with hidden tags -> their InterfaceNodes fade out
 * - Services with hidden tags -> hidden from node display (node does NOT fade)
 * - Subnets with hidden tags -> SubnetNodes fade out
 * - UNTAGGED_SENTINEL in hidden arrays -> hide entities with no tags
 */
export function updateTagFilter(topology: Topology | undefined, tagFilter: TagFilter | undefined) {
	if (!topology) {
		tagHiddenNodeIds.set(new Set());
		tagHiddenServiceIds.set(new Set());
		return;
	}

	if (!tagFilter || isTagFilterEmpty(tagFilter)) {
		tagHiddenNodeIds.set(new Set());
		tagHiddenServiceIds.set(new Set());
		return;
	}

	const hiddenHostTagIds = tagFilter.hidden_host_tag_ids ?? [];
	const hiddenServiceTagIds = tagFilter.hidden_service_tag_ids ?? [];
	const hiddenSubnetTagIds = tagFilter.hidden_subnet_tag_ids ?? [];

	const hideUntaggedHosts = hiddenHostTagIds.includes(UNTAGGED_SENTINEL);
	const hideUntaggedServices = hiddenServiceTagIds.includes(UNTAGGED_SENTINEL);
	const hideUntaggedSubnets = hiddenSubnetTagIds.includes(UNTAGGED_SENTINEL);

	const hiddenNodeIds = new Set<string>();
	const hiddenServiceIds = new Set<string>();

	// Host tags -> fade InterfaceNodes
	for (const host of topology.hosts) {
		const isUntagged = host.tags.length === 0;
		const hostHasHiddenTag = host.tags.some((t) => hiddenHostTagIds.includes(t));
		if (hostHasHiddenTag || (isUntagged && hideUntaggedHosts)) {
			// Add all InterfaceNodes for this host to hidden set
			const hostInterfaces = topology.interfaces.filter((i) => i.host_id === host.id);
			hostInterfaces.forEach((i) => hiddenNodeIds.add(i.id));
		}
	}

	// Service tags -> hide services from display (NOT fade the node)
	for (const service of topology.services) {
		const isUntagged = service.tags.length === 0;
		const serviceHasHiddenTag = service.tags.some((t) => hiddenServiceTagIds.includes(t));
		if (serviceHasHiddenTag || (isUntagged && hideUntaggedServices)) {
			hiddenServiceIds.add(service.id);
		}
	}

	// Subnet tags -> fade SubnetNodes
	for (const subnet of topology.subnets) {
		const isUntagged = subnet.tags.length === 0;
		const subnetHasHiddenTag = subnet.tags.some((t) => hiddenSubnetTagIds.includes(t));
		if (subnetHasHiddenTag || (isUntagged && hideUntaggedSubnets)) {
			hiddenNodeIds.add(subnet.id);
		}
	}

	tagHiddenNodeIds.set(hiddenNodeIds);
	tagHiddenServiceIds.set(hiddenServiceIds);
}

function isTagFilterEmpty(filter: {
	hidden_host_tag_ids?: string[];
	hidden_service_tag_ids?: string[];
	hidden_subnet_tag_ids?: string[];
}): boolean {
	return (
		(filter.hidden_host_tag_ids?.length ?? 0) === 0 &&
		(filter.hidden_service_tag_ids?.length ?? 0) === 0 &&
		(filter.hidden_subnet_tag_ids?.length ?? 0) === 0
	);
}

/**
 * Helper function to get all virtualized container interface IDs for a ServiceVirtualization edge
 * Returns the set of interface IDs for all containers on Docker bridge subnets
 * Uses topology data directly if provided, otherwise falls back to query cache
 */
function getVirtualizedContainerNodes(
	dockerHostInterfaceId: string,
	queryClient: QueryClient,
	topology?: Topology
): Set<string> {
	const connected = new Set<string>();

	// Try to use topology data directly (for share views where cache is empty)
	if (topology) {
		const iface = topology.interfaces.find((i) => i.id === dockerHostInterfaceId);
		if (!iface) return connected;

		const dockerHost = topology.hosts.find((h) => h.id === iface.host_id);
		if (!dockerHost) return connected;

		// Get all interfaces for this host
		const hostInterfaces = topology.interfaces.filter((i) => i.host_id === dockerHost.id);
		const hostInterfaceSubnetIds = hostInterfaces.map((i) => i.subnet_id);

		// Find container subnets
		const dockerBridgeSubnets = hostInterfaceSubnetIds
			.map((subnetId) => topology.subnets.find((s) => s.id === subnetId))
			.filter((s) => s !== undefined)
			.filter((s) => subnetTypes.getMetadata(s.subnet_type).is_for_containers);

		// Get all interfaces on those container subnets
		const interfacesOnDockerSubnets = dockerBridgeSubnets.flatMap((s) =>
			topology.interfaces.filter((i) => i.subnet_id === s.id)
		);

		for (const iface of interfacesOnDockerSubnets) {
			connected.add(iface.id);
		}

		return connected;
	}

	// Fall back to query cache
	const dockerHost = getHostFromInterfaceIdFromCache(queryClient, dockerHostInterfaceId);
	if (dockerHost) {
		// Get all interfaces for this host from the cache
		const hostInterfaces = getInterfacesForHostFromCache(queryClient, dockerHost.id);
		const hostInterfaceSubnetIds = hostInterfaces.map((i) => i.subnet_id);

		const dockerBridgeSubnets = hostInterfaceSubnetIds
			.map((s) => getSubnetByIdFromCache(queryClient, s))
			.filter((s) => s !== null)
			.filter((s) => subnetTypes.getMetadata(s.subnet_type).is_for_containers);

		const interfacesOnDockerSubnets = dockerBridgeSubnets.flatMap((s) =>
			getInterfacesForSubnetFromCache(queryClient, s.id)
		);

		for (const iface of interfacesOnDockerSubnets) {
			connected.add(iface.id);
		}
	}

	return connected;
}

/**
 * Update connected nodes when a node or edge is selected
 * @param topology - Optional topology data for direct lookups (used in share views where cache is empty)
 */
export function updateConnectedNodes(
	selectedNode: Node | null,
	selectedEdge: Edge | null,
	allEdges: Edge[],
	allNodes: Node[],
	queryClient: QueryClient,
	topology?: Topology
) {
	const connected = new Set<string>();

	// If a node is selected
	if (selectedNode) {
		connected.add(selectedNode.id);
		const nodeData = selectedNode.data as TopologyNode;

		if (nodeData.node_type == 'SubnetNode') {
			allNodes.forEach((n) => {
				const nd = n.data as TopologyNode;
				if (nd.node_type == 'InterfaceNode' && nd.subnet_id == nodeData.id) {
					connected.add(nd.id);
				}
			});
		}

		for (const edge of allEdges) {
			const edgeData = edge.data as TopologyEdge | undefined;
			if (!edgeData) continue;

			// Add directly connected nodes (regular edges)
			if (edgeData.source === selectedNode.id) {
				connected.add(edgeData.target as string);
			}
			if (edgeData.target === selectedNode.id) {
				connected.add(edgeData.source as string);
			}

			// Include virtualized nodes
			if (edgeData.edge_type === 'ServiceVirtualization') {
				if (edgeData.source === selectedNode.id || edgeData.target === selectedNode.id) {
					connected.add(edgeData.source as string);

					// Add all virtualized container nodes
					const virtualizedNodes = getVirtualizedContainerNodes(
						edgeData.source as string,
						queryClient,
						topology
					);
					virtualizedNodes.forEach((nodeId) => connected.add(nodeId));
				}
			} else if (edgeData.edge_type === 'HostVirtualization') {
				if (edgeData.source === selectedNode.id || edgeData.target === selectedNode.id) {
					connected.add(edgeData.source as string);
					connected.add(edgeData.target as string);
				}
			}
		}

		connectedNodeIds.set(connected);
		return;
	}

	// If an edge is selected (group OR non-group)
	if (selectedEdge) {
		const edgeData = selectedEdge.data as TopologyEdge | undefined;
		if (!edgeData) {
			connectedNodeIds.set(new Set());
			return;
		}
		const edgeTypeMetadata = edgeTypes.getMetadata(edgeData.edge_type);

		// For group edges
		if (edgeTypeMetadata.is_group_edge && 'group_id' in edgeData) {
			const groupId = edgeData.group_id as string;

			// Find all edges in this group and add their connected nodes
			for (const edge of allEdges) {
				const eData = edge.data as TopologyEdge | undefined;
				if (!eData) continue;
				const eMetadata = edgeTypes.getMetadata(eData.edge_type);

				if (eMetadata.is_group_edge && 'group_id' in eData && eData.group_id === groupId) {
					connected.add(eData.source as string);
					connected.add(eData.target as string);
				}
			}
		} else if (edgeData.edge_type === 'ServiceVirtualization') {
			// For ServiceVirtualization edges, add source, target, and all virtualized containers
			connected.add(edgeData.source as string);
			connected.add(edgeData.target as string);

			// Add all virtualized container nodes
			const virtualizedNodes = getVirtualizedContainerNodes(
				edgeData.source as string,
				queryClient,
				topology
			);
			virtualizedNodes.forEach((nodeId) => connected.add(nodeId));
		} else if (edgeData.edge_type === 'HostVirtualization') {
			// For HostVirtualization edges, add source and target
			connected.add(edgeData.source as string);
			connected.add(edgeData.target as string);
		} else {
			// For other non-group edges, just add source and target
			connected.add(edgeData.source as string);
			connected.add(edgeData.target as string);
		}

		connectedNodeIds.set(connected);
		return;
	}

	// Nothing selected - clear
	connectedNodeIds.set(new Set());
}

/**
 * Toggle edge hover state - updates both individual edge and group hover states
 */
export function toggleEdgeHover(edge: Edge, allEdges: Edge[]) {
	const edgeData = edge.data as TopologyEdge | undefined;
	if (!edgeData) return;
	const edgeTypeMetadata = edgeTypes.getMetadata(edgeData.edge_type);

	// Toggle individual edge hover state
	edgeHoverState.update((state) => {
		const currentHoverState = state.get(edge.id) || false;
		const newState = new Map(state);
		newState.set(edge.id, !currentHoverState);
		return newState;
	});

	// For group edges, update group hover state
	if (edgeTypeMetadata.is_group_edge && 'group_id' in edgeData) {
		const groupId = edgeData.group_id as string;

		groupHoverState.update((state) => {
			const newState = new Map(state);

			// Get the UPDATED edge hover states (after we just toggled this edge)
			const updatedEdgeStates = get(edgeHoverState);
			let anyEdgeInGroupHovered = false;

			// Check if ANY edge in this group is hovered
			for (const e of allEdges) {
				const eData = e.data as TopologyEdge | undefined;
				if (!eData) continue;
				const eMetadata = edgeTypes.getMetadata(eData.edge_type);
				if (eMetadata.is_group_edge && 'group_id' in eData && eData.group_id === groupId) {
					const eIsHovered = updatedEdgeStates.get(e.id) || false;
					if (eIsHovered) {
						anyEdgeInGroupHovered = true;
						break;
					}
				}
			}

			newState.set(groupId, anyEdgeInGroupHovered);
			return newState;
		});
	}
}

/**
 * Get display state for an edge based on hover and selection
 * Returns: { shouldShowFull, shouldAnimate }
 */
export function getEdgeDisplayState(
	edge: Edge,
	selectedNode: Node | null,
	selectedEdge: Edge | null
): { shouldShowFull: boolean; shouldAnimate: boolean } {
	const edgeData = edge.data as TopologyEdge | undefined;
	if (!edgeData) {
		return { shouldShowFull: false, shouldAnimate: false };
	}
	const edgeTypeMetadata = edgeTypes.getMetadata(edgeData.edge_type);
	const isGroupEdge = edgeTypeMetadata.is_group_edge;

	let shouldShowFull = false;
	let shouldAnimate = false;

	// Check if this edge is hovered
	const isThisEdgeHovered = get(edgeHoverState).get(edge.id) || false;

	// Check if this edge is selected
	const isThisEdgeSelected = selectedEdge?.id === edge.id;

	// For group edges, check group hover/selection state
	if (isGroupEdge && 'group_id' in edgeData) {
		const groupId = edgeData.group_id as string;
		const isGroupHovered = get(groupHoverState).get(groupId) || false;

		// Check if any edge in this group is selected
		let isGroupSelected = false;
		if (selectedEdge) {
			const selectedEdgeData = selectedEdge.data as TopologyEdge | undefined;
			if (selectedEdgeData) {
				const selectedMetadata = edgeTypes.getMetadata(selectedEdgeData.edge_type);
				if (selectedMetadata.is_group_edge && 'group_id' in selectedEdgeData) {
					isGroupSelected = selectedEdgeData.group_id === groupId;
				}
			}
		}

		// Check if connected node is selected
		const isConnectedNodeSelected =
			selectedNode && (edgeData.source === selectedNode.id || edgeData.target === selectedNode.id);

		// Should show full if: group hovered, group selected, or connected node selected
		shouldShowFull = isGroupHovered || isGroupSelected || !!isConnectedNodeSelected;

		// Should animate if: group hovered, group selected, or connected node selected
		shouldAnimate = isGroupHovered || isGroupSelected || !!isConnectedNodeSelected;
	} else {
		// Non-group edges: show full if hovered, selected, or connected node selected
		const isConnectedNodeSelected =
			selectedNode && (edgeData.source === selectedNode.id || edgeData.target === selectedNode.id);

		shouldShowFull = isThisEdgeHovered || isThisEdgeSelected || !!isConnectedNodeSelected;
		shouldAnimate = false; // Non-group edges don't animate
	}

	return { shouldShowFull, shouldAnimate };
}
