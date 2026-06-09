<script lang="ts">
	import type { Edge } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { IPAddressDisplay } from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import type { EnrichedTopology } from '$lib/features/topology/types/base';

	import type { components } from '$lib/api/schema';
	type TopologyView = components['schemas']['TopologyView'];

	/* eslint-disable @typescript-eslint/no-unused-vars -- component contract props */
	let {
		edge,
		hostId,
		view = 'L3Logical'
	}: {
		edge: Edge;
		hostId: string;
		view?: TopologyView;
	} = $props();
	/* eslint-enable @typescript-eslint/no-unused-vars */

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let isReadonly = topo.isReadonly;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					| EnrichedTopology
					| undefined)
	);

	let editState = $derived(getTopologyEditState(topology, false, isReadonly));

	let host = $derived(topology ? topology.hosts.find((h) => h.id == hostId) : null);

	let sourceInterface = $derived(topology?.ip_addresses.find((i) => i.id == edge.source));
	let targetInterface = $derived(topology?.ip_addresses.find((i) => i.id == edge.target));

	// Context for interface displays
	let interfaceContext = $derived({ subnets: topology?.subnets ?? [], compact: true });
</script>

<div class="space-y-3">
	{#if host}
		<span class="text-secondary mb-2 block text-sm font-medium">Host</span>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={{
					services: topology?.services.filter((s) => host && s.host_id == host.id) ?? [],
					showEntityTagPicker: true,
					tagPickerDisabled: !editState.isEditable,
					entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
					compact: true
				}}
				item={host}
				displayComponent={HostDisplay}
			/>
		</div>
	{/if}
	<span class="text-secondary mb-2 block text-sm font-medium">IP Addresses</span>
	{#if sourceInterface}
		<div class="card card-static">
			<EntityDisplayWrapper
				context={interfaceContext}
				item={sourceInterface}
				displayComponent={IPAddressDisplay}
			/>
		</div>
	{/if}

	{#if targetInterface}
		<div class="card card-static">
			<EntityDisplayWrapper
				context={interfaceContext}
				item={targetInterface}
				displayComponent={IPAddressDisplay}
			/>
		</div>
	{/if}
</div>
