<script lang="ts">
	import { SlidersHorizontal } from 'lucide-svelte';
	import type { components } from '$lib/api/schema';
	import type { Network } from '$lib/features/networks/types';
	import type { Host, IPAddress } from '$lib/features/hosts/types/base';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { NetworkDisplay } from '$lib/shared/components/forms/selection/display/NetworkDisplay.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import {
		IPAddressDisplay,
		type IPAddressDisplayContext
	} from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import {
		common_hosts,
		common_networks,
		credentials_assignNetworkEmpty,
		credentials_assignNetworkPlaceholder,
		credentials_assignHostEmpty,
		credentials_assignHostPlaceholder,
		credentials_ipScopeAllDefault,
		credentials_ipScopeLabel,
		credentials_ipScopePlaceholder
	} from '$lib/paraglide/messages';

	type CredentialHostAssignment = components['schemas']['CredentialHostAssignment'];

	interface Props {
		credentialTypeId: string;
		assignedNetworkIds: string[];
		hostAssignments: CredentialHostAssignment[];
	}

	let {
		credentialTypeId,
		assignedNetworkIds = $bindable([]),
		hostAssignments = $bindable([])
	}: Props = $props();

	let scopeModels = $derived(credentialTypes.getMetadata(credentialTypeId)?.scope_models ?? []);
	let supportsBroadcast = $derived(scopeModels.includes('Broadcast'));
	let supportsPerHost = $derived(scopeModels.includes('PerHost'));

	const networksQuery = useNetworksQuery();
	const hostsQuery = useHostsQuery({ limit: 0 });
	const ipAddressesQuery = useIPAddressesQuery();
	const subnetsQuery = useSubnetsQuery();

	let allNetworks = $derived(networksQuery.data ?? []);
	let allHosts = $derived(hostsQuery.data?.items ?? []);
	let allIpAddresses = $derived(ipAddressesQuery.data ?? []);
	let subnets = $derived(subnetsQuery.data ?? []);

	// --- Networks (Broadcast) ---
	let selectedNetworks = $derived(
		assignedNetworkIds
			.map((id) => allNetworks.find((n) => n.id === id))
			.filter((n): n is Network => n != null)
	);

	function addNetwork(id: string) {
		if (!assignedNetworkIds.includes(id)) {
			assignedNetworkIds = [...assignedNetworkIds, id];
		}
	}

	function removeNetwork(index: number) {
		const target = selectedNetworks[index];
		if (target) assignedNetworkIds = assignedNetworkIds.filter((id) => id !== target.id);
	}

	// --- Hosts (PerHost), with per-host IP scoping via row expansion ---
	let selectedHosts = $derived(
		hostAssignments
			.map((a) => allHosts.find((h) => h.id === a.host_id))
			.filter((h): h is Host => h != null)
	);

	let availableHosts = $derived(
		allHosts.filter((h) => !hostAssignments.some((a) => a.host_id === h.id))
	);

	// Which host row is expanded to show its IP-address scope (by host id)
	let expandedHostId = $state<string | null>(null);

	function toggleExpand(hostId: string) {
		expandedHostId = expandedHostId === hostId ? null : hostId;
	}

	function addHost(id: string) {
		if (!hostAssignments.some((a) => a.host_id === id)) {
			hostAssignments = [...hostAssignments, { host_id: id, ip_address_ids: null }];
		}
	}

	function removeHost(index: number) {
		const target = selectedHosts[index];
		if (target) hostAssignments = hostAssignments.filter((a) => a.host_id !== target.id);
	}

	function hostIpAddresses(hostId: string): IPAddress[] {
		return allIpAddresses.filter((ip) => ip.host_id === hostId);
	}

	function getInterfaceContext(): IPAddressDisplayContext {
		return { subnets };
	}

	// Scoped IP addresses for a host assignment (null = all)
	function getScopedInterfaces(hostId: string): IPAddress[] {
		const assignment = hostAssignments.find((a) => a.host_id === hostId);
		if (!assignment || assignment.ip_address_ids === null) return [];
		return assignment.ip_address_ids
			.map((id) => allIpAddresses.find((ip) => ip.id === id))
			.filter((ip): ip is IPAddress => ip != null);
	}

	function addInterfaceToScope(hostId: string, interfaceId: string) {
		hostAssignments = hostAssignments.map((a) => {
			if (a.host_id !== hostId) return a;
			const current = a.ip_address_ids;
			if (current === null) return { ...a, ip_address_ids: [interfaceId] };
			if (current.includes(interfaceId)) return a;
			return { ...a, ip_address_ids: [...current, interfaceId] };
		});
	}

	function removeInterfaceFromScope(hostId: string, interfaceIndex: number) {
		hostAssignments = hostAssignments.map((a) => {
			if (a.host_id !== hostId || a.ip_address_ids === null) return a;
			const next = a.ip_address_ids.filter((_, i) => i !== interfaceIndex);
			// Empty list reverts to "all interfaces" (null)
			return { ...a, ip_address_ids: next.length === 0 ? null : next };
		});
	}
</script>

{#snippet networksSurface()}
	<div class="min-w-0 flex-1">
		<ListManager
			label={`${common_networks()} (${assignedNetworkIds.length})`}
			placeholder={credentials_assignNetworkPlaceholder()}
			emptyMessage={credentials_assignNetworkEmpty()}
			allowReorder={false}
			options={allNetworks}
			items={selectedNetworks}
			optionDisplayComponent={NetworkDisplay}
			itemDisplayComponent={NetworkDisplay}
			onAdd={addNetwork}
			onRemove={removeNetwork}
		/>
	</div>
{/snippet}

{#snippet hostsSurface()}
	<div class="min-w-0 flex-1">
		<ListManager
			label={`${common_hosts()} (${hostAssignments.length})`}
			placeholder={credentials_assignHostPlaceholder()}
			emptyMessage={credentials_assignHostEmpty()}
			allowReorder={false}
			options={availableHosts}
			items={selectedHosts}
			optionDisplayComponent={HostDisplay}
			itemDisplayComponent={HostDisplay}
			itemClickAction="edit"
			editIcon={() => SlidersHorizontal}
			isItemEditing={(host) => host.id === expandedHostId}
			onEdit={(host) => toggleExpand(host.id)}
			onAdd={addHost}
			onRemove={removeHost}
		>
			{#snippet itemExpandedSnippet({ item })}
				{#if item.id === expandedHostId}
					{@const hostIps = hostIpAddresses(item.id)}
					<div
						role="presentation"
						onclick={(e) => e.stopPropagation()}
						onkeydown={(e) => e.stopPropagation()}
						class="mt-2 w-full border-t border-gray-200 pt-3 dark:border-gray-700"
					>
						<ListManager
							label={credentials_ipScopeLabel()}
							emptyMessage={credentials_ipScopeAllDefault()}
							placeholder={credentials_ipScopePlaceholder()}
							allowReorder={false}
							options={hostIps}
							items={getScopedInterfaces(item.id)}
							optionDisplayComponent={IPAddressDisplay}
							itemDisplayComponent={IPAddressDisplay}
							getOptionContext={() => getInterfaceContext()}
							getItemContext={() => getInterfaceContext()}
							onAdd={(id) => addInterfaceToScope(item.id, id)}
							onRemove={(i) => removeInterfaceFromScope(item.id, i)}
						/>
					</div>
				{/if}
			{/snippet}
		</ListManager>
	</div>
{/snippet}

<div class="flex min-h-[18rem] flex-1 gap-6">
	{#if supportsBroadcast}
		{@render networksSurface()}
	{/if}
	{#if supportsPerHost}
		{@render hostsSurface()}
	{/if}
</div>
