<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { EnrichedTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import { common_hypervisor } from '$lib/paraglide/messages';

	/* eslint-disable @typescript-eslint/no-unused-vars -- component contract props */
	let {
		node,
		topology,
		editState,
		elementContext
	}: {
		node: Node;
		topology: EnrichedTopology;
		editState: TopologyEditState;
		elementContext?: ElementRenderContext;
	} = $props();
	/* eslint-enable @typescript-eslint/no-unused-vars */

	let isReadonly = $derived(editState.isReadonly);

	// Resolve the virtualizer host from the element's host virtualization data
	let virtualizerHost = $derived.by(() => {
		const virtualization = elementContext?.host?.virtualization;
		if (!virtualization) return null;

		// Look up the virtualizing service, then find its host
		const service = topology.services.find((s) => s.id === virtualization.details.service_id);
		if (!service?.host_id) return null;

		return topology.hosts.find((h) => h.id === service.host_id) ?? null;
	});

	let hostContext = $derived({
		services: virtualizerHost
			? topology.services.filter((s) => s.host_id === virtualizerHost.id)
			: [],
		showEntityTagPicker: !editState.isReadonly,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		compact: true
	});
</script>

{#if virtualizerHost}
	<div>
		<span class="text-secondary mb-2 block text-sm font-medium">
			{common_hypervisor()}
		</span>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={hostContext}
				item={virtualizerHost}
				displayComponent={HostDisplay}
			/>
		</div>
	</div>
{/if}
