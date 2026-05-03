import type { LayoutGraph } from '../layout/layout-graph';
import type { EnrichedTopology, TopologyNode, TopologyEdge } from '../types/base';
import type { SelectionStores } from '../selection';
import type { Node, Edge } from '@xyflow/svelte';
import type { Writable } from 'svelte/store';
import type { AggregatedEdge } from '../collapse';
import type { TopologyParentIndex } from '../topology-parent-index';

export type XY = { x: number; y: number };

/**
 * Mutable state shared across all pipeline phases.
 * Created once by the orchestrator, passed by reference to pipeline functions.
 */
export interface LayoutState {
	layoutGraph: LayoutGraph | null;
	containerSizeCache: Map<string, { collapsed?: XY; expanded?: XY }>;
	viewSizeCache: Map<string, Map<string, XY>>;
	sessionStructureKey: string;
	sessionBaseKey: string;
	seenAutoCollapseIds: Set<string>;
	collapseLevelInferred: boolean;
	lastSeenTopologyId: string;
	fitViewPending: boolean;
	prevExpandedPortIds: Set<string>;
	lastRenderedTopoKey: string;
	lastRenderedView: string;
	layoutGeneration: number;
}

/**
 * Immutable inputs for a single pipeline run.
 */
export interface PipelineContext {
	topology: EnrichedTopology;
	containerElement: HTMLDivElement;
	getNodes: () => Node[];
	selectionStores: SelectionStores;
	nodes: Writable<Node[]>;
	edges: Writable<Edge[]>;
	isStale: () => boolean;
	getInfrastructureRuleId: () => string | null;
}

/**
 * Output of the prepare phase, consumed by measure/execute/build phases.
 */
export interface PrepareResult {
	layoutNodes: TopologyNode[];
	collapsed: Set<string>;
	elevatedEdges: TopologyEdge[];
	elementToContainer: Map<string, string>;
	parentIndex: TopologyParentIndex;
	topoKey: string;
	structureKey: string;
	baseKey: string;
	isNewStructure: boolean;
	isNewBaseStructure: boolean;
	viewChanged: boolean;
	topologyChanged: boolean;
	deferCollapse: boolean;
	needsElkForExpand: boolean;
	collapseChanged: boolean;
	visibleNodes: TopologyNode[];
	aggregatedEdges: AggregatedEdge[];
	hiddenEdgeTypes: string[];
	prevExpandedSizes: Map<string, { width: number; height: number }> | undefined;
	prevChildPositions: Map<string, Map<string, { x: number; y: number }>> | undefined;
	currentView: string;
	topologyId: string;
	needsElk: boolean;
	isViewTransition: boolean;
}

export function createInitialState(): LayoutState {
	return {
		layoutGraph: null,
		containerSizeCache: new Map(),
		viewSizeCache: new Map(),
		sessionStructureKey: '',
		sessionBaseKey: '',
		seenAutoCollapseIds: new Set(),
		collapseLevelInferred: false,
		lastSeenTopologyId: '',
		fitViewPending: false,
		prevExpandedPortIds: new Set(),
		lastRenderedTopoKey: '',
		lastRenderedView: '',
		layoutGeneration: 0
	};
}
