/**
 * Rune-friendly views over the module-level topology stores.
 *
 * Node components (`ElementNode`, `ContainerNode`) are instantiated once per
 * graph node — hundreds of times on a large topology, and repeatedly, because
 * the measure pass mounts and unmounts the whole graph. They used to read these
 * stores by calling `store.subscribe(...)` at component init and discarding the
 * unsubscriber, which leaked: the stores outlive every node, so each mount grew
 * the subscriber list permanently and every later `.set()` walked a list of dead
 * closures writing `$state` into destroyed components.
 *
 * `fromStore` fixes both halves of that:
 *
 * - It subscribes through `createSubscriber`, so the subscription belongs to the
 *   reading effect and is torn down with the component.
 * - `createSubscriber` refcounts, so N node components share **one** underlying
 *   `.subscribe()`. That is strictly cheaper than per-component `$store` auto-
 *   subscription, which unsubscribes correctly but still invokes N callbacks on
 *   every `.set()`.
 *
 * Reading `.current` outside a tracking context falls back to `get(store)`, so
 * the value is never missed on first read — which is what the hand-rolled
 * subscriptions were working around.
 *
 * These are deliberately module-scope: one wrapper per store for the whole app,
 * not one per component.
 */

import { fromStore } from 'svelte/store';
import {
	connectedNodeIds,
	edgeHandlesByNode,
	hoveredMetadata,
	hoveredTag,
	isExporting,
	newNodeIds,
	searchHiddenNodeIds,
	searchMatchContainerMap,
	hiddenEntityIds
} from './interactions';
import { selectedNodes } from './queries';
import { collapsedContainers } from './collapse';

export const connectedNodes = fromStore(connectedNodeIds);
export const edgeHandles = fromStore(edgeHandlesByNode);
export const exporting = fromStore(isExporting);
export const searchHiddenNodes = fromStore(searchHiddenNodeIds);
export const searchContainerMatches = fromStore(searchMatchContainerMap);
export const hiddenEntities = fromStore(hiddenEntityIds);
export const highlightedNewNodes = fromStore(newNodeIds);
export const multiSelectedNodes = fromStore(selectedNodes);
export const currentHoveredTag = fromStore(hoveredTag);
export const currentHoveredMetadata = fromStore(hoveredMetadata);
export const collapsedNodes = fromStore(collapsedContainers);
