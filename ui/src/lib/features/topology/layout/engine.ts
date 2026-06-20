import type { TopologyNode, TopologyEdge, RenderableTopology } from '../types/base';
import type { TopologyParentIndex } from '../topology-parent-index';
import { computeElkLayout } from './elk-layout';

export interface LayoutInput {
	nodes: TopologyNode[];
	edges: TopologyEdge[];
	topology: RenderableTopology;
	parentIndex?: TopologyParentIndex;
	collapsedContainers?: Set<string>;
	expandedContainerSizes?: Map<string, { width: number; height: number }>;
	elementNodeSizes?: Map<string, { x: number; y: number }>;
	hiddenEdgeTypes?: string[];
}

export interface LayoutResult {
	nodePositions: Map<string, { x: number; y: number }>;
	containerSizes: Map<string, { width: number; height: number }>;
	elementNodeSizes: Map<string, { x: number; y: number }>;
}

export interface LayoutEngine {
	compute(input: LayoutInput): Promise<LayoutResult>;
}

export class ElkLayoutEngine implements LayoutEngine {
	async compute(input: LayoutInput): Promise<LayoutResult> {
		return computeElkLayout(input);
	}
}
