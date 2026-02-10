<script lang="ts">
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { IfEntryDisplay } from '$lib/shared/components/forms/selection/display/IfEntryDisplay.svelte';
	import { useTopologiesQuery, selectedTopologyId } from '$lib/features/topology/queries';
	import type { Topology } from '$lib/features/topology/types/base';
	import { getContext } from 'svelte';
	import type { Writable } from 'svelte/store';
	import Tag from '$lib/shared/components/data/Tag.svelte';

	let {
		sourceIfEntryId,
		targetIfEntryId,
		protocol
	}: {
		sourceIfEntryId?: string;
		targetIfEntryId?: string;
		protocol?: 'LLDP' | 'CDP';
	} = $props();

	// Try to get topology from context (for share/embed pages), fallback to query + selected topology
	const topologyContext = getContext<Writable<Topology> | undefined>('topology');
	const topologiesQuery = useTopologiesQuery();
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let topology = $derived(
		topologyContext ? $topologyContext : topologiesData.find((t) => t.id === $selectedTopologyId)
	);

	// Derive IfEntry and Host data
	let sourceIfEntry = $derived(topology?.if_entries.find((e) => e.id === sourceIfEntryId));
	let targetIfEntry = $derived(topology?.if_entries.find((e) => e.id === targetIfEntryId));
	let sourceHost = $derived(
		sourceIfEntry ? topology?.hosts.find((h) => h.id === sourceIfEntry.host_id) : null
	);
	let targetHost = $derived(
		targetIfEntry ? topology?.hosts.find((h) => h.id === targetIfEntry.host_id) : null
	);
</script>

<div class="space-y-3">
	{#if protocol}
		<div class="flex items-center gap-2">
			<Tag label={protocol} color={protocol == 'CDP' ? 'Blue' : 'Green'} />
		</div>
	{/if}

	{#if sourceHost || sourceIfEntry}
		<span class="text-secondary mb-2 block text-sm font-medium">Source</span>
		{#if sourceHost}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id === sourceHost.id) ?? []
					}}
					item={sourceHost}
					displayComponent={HostDisplay}
				/>
			</div>
		{/if}
		{#if sourceIfEntry}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={undefined}
					item={sourceIfEntry}
					displayComponent={IfEntryDisplay}
				/>
			</div>
		{/if}
	{/if}

	{#if targetHost || targetIfEntry}
		<span class="text-secondary mb-2 block text-sm font-medium">Target</span>
		{#if targetHost}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id === targetHost.id) ?? []
					}}
					item={targetHost}
					displayComponent={HostDisplay}
				/>
			</div>
		{/if}
		{#if targetIfEntry}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={undefined}
					item={targetIfEntry}
					displayComponent={IfEntryDisplay}
				/>
			</div>
		{/if}
	{/if}
</div>
