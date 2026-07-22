import type { TopologyEdge } from '../types/base';
import type { components } from '$lib/api/schema';
import { views } from '$lib/shared/stores/metadata';

type EdgeTypeDiscriminants = components['schemas']['EdgeTypeDiscriminants'];
type EdgeViewConfig = components['schemas']['EdgeViewConfig'];
type TopologyView = components['schemas']['TopologyView'];

/** Get the view config from an edge, defaulting to Disabled if absent. */
export function getViewConfig(edge: TopologyEdge): EdgeViewConfig {
	const vc = (edge as Record<string, unknown>).view_config as EdgeViewConfig | undefined;
	return vc ?? { type: 'disabled' };
}

/** Whether this edge is disabled (not available) in the current view. */
export function isDisabledEdge(edge: TopologyEdge): boolean {
	return getViewConfig(edge).type === 'disabled';
}

/** Whether this edge should affect ELK layout positioning. */
export function affectsLayout(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.affects_layout;
}

/** Whether this edge is hidden by default (togglable). */
export function isHiddenByDefault(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.default_visibility === 'hidden';
}

/** Whether this edge uses dotted stroke in the current view. */
export function isDottedEdge(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.stroke === 'dotted';
}

/**
 * Whether this edge annotates the graph rather than structuring it — any non-solid stroke.
 * Drives the shared overlay treatment (thinner, dimmed until highlighted); the specific stroke
 * only decides the dash pattern.
 */
export function isOverlayEdge(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.stroke !== 'solid';
}

/** Get the highlight behavior for an edge. Defaults to 'when_visible'. */
export function getHighlightBehavior(edge: TopologyEdge): 'when_visible' | 'always' | 'never' {
	const vc = getViewConfig(edge);
	if (vc.type !== 'active') return 'when_visible';
	return (
		((vc as Record<string, unknown>).highlight_behavior as
			| 'when_visible'
			| 'always'
			| 'never'
			| undefined) ?? 'when_visible'
	);
}

/** Whether this edge should show directional animation when highlighted. */
export function showDirectionality(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.show_directionality;
}

/** Whether this edge should be elevated to target an accepting container. */
export function willTargetContainer(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.will_target_container;
}

/** Returns the edge types that should be hidden by default for a given view.
 * Reads from view metadata — edge types with default_visibility = 'hidden'. */
export function getDefaultHiddenEdgeTypes(view: TopologyView): EdgeTypeDiscriminants[] {
	const meta = views.getMetadata(view) as {
		edge_view_configs?: Record<string, EdgeViewConfig>;
	} | null;
	if (!meta?.edge_view_configs) return [];
	return Object.entries(meta.edge_view_configs)
		.filter(([, c]) => c.type === 'active' && c.default_visibility === 'hidden')
		.map(([id]) => id as EdgeTypeDiscriminants);
}
