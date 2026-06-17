<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import type { EnrichedTopology, TopologyNode } from '$lib/features/topology/types/base';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import InterfaceDetailsCard from '$lib/features/hosts/components/InterfaceDetailsCard.svelte';

	let {
		node,
		topology,
		elementContext
	}: {
		node: Node;
		topology: EnrichedTopology;
		elementContext?: ElementRenderContext;
	} = $props();

	// Find the SNMP Interface: from node data or element context
	let iface = $derived.by(() => {
		const nodeData = node.data as TopologyNode;
		const interfaceId = 'interface_id' in nodeData ? (nodeData.interface_id as string) : undefined;
		if (interfaceId) {
			return topology.interfaces.find((e) => e.id === interfaceId) ?? null;
		}
		if (!elementContext?.interfaceId) return null;
		return topology.interfaces.find((e) => e.id === elementContext.interfaceId) ?? null;
	});

	// Resolve linked IPAddress from the SNMP interface's ip_address_id FK
	let linkedIpAddress = $derived.by(() => {
		if (!iface?.ip_address_id) return null;
		return topology.ip_addresses.find((i) => i.id === iface!.ip_address_id) ?? null;
	});

	let linkedSubnet = $derived.by(() => {
		if (!linkedIpAddress) return null;
		return topology.subnets.find((s) => s.id === linkedIpAddress!.subnet_id) ?? null;
	});

	let neighborHost = $derived.by(() => {
		if (!iface?.neighbor) return null;
		if (iface.neighbor.type === 'Host') {
			return topology.hosts.find((h) => h.id === iface!.neighbor!.id) ?? null;
		}
		const remoteEntry = topology.interfaces.find((e) => e.id === iface!.neighbor!.id);
		if (remoteEntry) {
			return topology.hosts.find((h) => h.id === remoteEntry.host_id) ?? null;
		}
		return null;
	});

	let neighborInterface = $derived.by(() => {
		if (!iface?.neighbor || iface.neighbor.type !== 'Interface') return null;
		return topology.interfaces.find((e) => e.id === iface!.neighbor!.id) ?? null;
	});

	type VlanShape = { id: string; vlan_number: number; name: string };

	let nativeVlan = $derived.by(() => {
		if (!iface?.native_vlan_id) return null;
		return (topology.vlans as VlanShape[]).find((v) => v.id === iface!.native_vlan_id) ?? null;
	});

	let taggedVlans = $derived.by(() => {
		if (!iface?.vlan_ids?.length) return [];
		const vlans = topology.vlans as VlanShape[];
		return iface!
			.vlan_ids!.map((id) => vlans.find((v) => v.id === id))
			.filter(Boolean) as VlanShape[];
	});
</script>

{#if iface}
	<InterfaceDetailsCard
		{iface}
		{linkedIpAddress}
		{linkedSubnet}
		{neighborHost}
		{neighborInterface}
		{nativeVlan}
		{taggedVlans}
	/>
{/if}
