<script lang="ts">
	/**
	 * The handle geometry SvelteFlow needs, with none of the connection machinery.
	 *
	 * `<Handle>` renders a div plus the whole editing affordance: mousedown/touchstart/click/keypress
	 * listeners, `role="button"`, and a set of reactive classes driven by live connection state —
	 * eight of those per node, on a view where nothing can be connected. Topology editing is
	 * disabled (`editModeEnabled` is only ever set to false) and edge anchors are not editable, so
	 * all of it was cost with no behaviour. Measured at this customer's scale it was 12,544 elements
	 * and ~13,400 listeners.
	 *
	 * Deleting the handles outright does not work: `updateNodeInternals` re-derives `handleBounds`
	 * from the DOM whenever a node's rendered size differs from its `measured`, and with no handle
	 * elements `getHandleBounds` returns null. It then stores `{ source: null, target: null }`,
	 * which is truthy, so SvelteFlow's own `toHandleBounds(node.handles)` fallback never fires and
	 * edges referencing a named handle are dropped with "Couldn't create edge for source handle id".
	 *
	 * So the DOM keeps exactly what `getHandleBounds` reads — a `source`/`target` class,
	 * `data-handleid`, `data-handlepos`, and a measurable box — and nothing else. Geometry stays in
	 * agreement with `synthesizeHandles`, which reproduces these same boxes for nodes that have
	 * never mounted.
	 *
	 * Restore `<Handle>` in both node components together if edge reconnection is ever revived.
	 *
	 * `used` narrows it further, to the handles an edge on this node actually names — see
	 * `edgeHandlesByNode`. A node's `handles` *data* still declares all eight, so bounds can be
	 * synthesized for a node that has never mounted; only the DOM is trimmed.
	 */
	let { size, used }: { size: number; used: Set<string> } = $props();

	const POSITIONS = [
		{ position: 'top', id: 'Top' },
		{ position: 'right', id: 'Right' },
		{ position: 'bottom', id: 'Bottom' },
		{ position: 'left', id: 'Left' }
	] as const;

	const TYPES = ['source', 'target'] as const;
</script>

{#each POSITIONS as { position, id } (id)}
	{#each TYPES as type (type)}
		{#if used.has(`${type}:${id}`)}
			<div
				class="svelte-flow__handle svelte-flow__handle-{position} {position} {type}"
				data-handleid={id}
				data-handlepos={position}
				style="opacity: 0; width: {size}px; height: {size}px;"
			></div>
		{/if}
	{/each}
{/each}
