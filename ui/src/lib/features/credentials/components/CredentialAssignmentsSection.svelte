<script lang="ts">
	import type { components } from '$lib/api/schema';
	import type { Network } from '$lib/features/networks/types';
	import type { Host } from '$lib/features/hosts/types/base';
	import type { IPAddress } from '$lib/features/hosts/types/base';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
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
		credentials_ipScopePlaceholder,
		credentials_noHostSelected,
		credentials_selectHostSubtitle,
		credentials_addInterfaces,
		hosts_credentialScopeSubtitle
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
	let dualScope = $derived(supportsBroadcast && supportsPerHost);

	// Which surface is shown for dual-scope credentials
	let activeSurface = $state<'networks' | 'hosts'>('networks');
	$effect(() => {
		// Default single-scope credentials to their one surface
		if (!supportsBroadcast) activeSurface = 'hosts';
		else if (!supportsPerHost) activeSurface = 'networks';
	});

	const networksQuery = useNetworksQuery();
	const hostsQuery = useHostsQuery({ limit: 0 });
	const ipAddressesQuery = useIPAddressesQuery();
	const subnetsQuery = useSubnetsQuery();

	let allNetworks = $derived(networksQuery.data ?? []);
	let allHosts = $derived(hostsQuery.data?.items ?? []);
	let allIpAddresses = $derived(ipAddressesQuery.data ?? []);
	let subnets = $derived(subnetsQuery.data ?? []);

	let segmentOptions = $derived([
		{ value: 'networks', label: `${common_networks()} (${assignedNetworkIds.length})` },
		{ value: 'hosts', label: `${common_hosts()} (${hostAssignments.length})` }
	]);

	// --- Broadcast (network) surface ---
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
		assignedNetworkIds = assignedNetworkIds.filter((_, i) => i !== index);
	}

	// --- PerHost surface ---
	let selectedHosts = $derived(
		hostAssignments
			.map((a) => allHosts.find((h) => h.id === a.host_id))
			.filter((h): h is Host => h != null)
	);

	let availableHosts = $derived(
		allHosts.filter((h) => !hostAssignments.some((a) => a.host_id === h.id))
	);

	function addHost(id: string) {
		if (!hostAssignments.some((a) => a.host_id === id)) {
			hostAssignments = [...hostAssignments, { host_id: id, ip_address_ids: null }];
		}
	}

	function removeHost(index: number) {
		hostAssignments = hostAssignments.filter((_, i) => i !== index);
	}

	function hostIpAddresses(hostId: string): IPAddress[] {
		return allIpAddresses.filter((ip) => ip.host_id === hostId);
	}

	function getInterfaceContext(): IPAddressDisplayContext {
		return { subnets };
	}

	// Resolve the scoped IP addresses for a host assignment (null = all)
	function getScopedInterfaces(index: number): IPAddress[] {
		const assignment = hostAssignments[index];
		if (!assignment || assignment.ip_address_ids === null) return [];
		return assignment.ip_address_ids
			.map((id) => allIpAddresses.find((ip) => ip.id === id))
			.filter((ip): ip is IPAddress => ip != null);
	}

	function addInterfaceToScope(index: number, interfaceId: string) {
		const updated = [...hostAssignments];
		const assignment = updated[index];
		if (!assignment) return;
		const current = assignment.ip_address_ids;
		if (current === null) {
			updated[index] = { ...assignment, ip_address_ids: [interfaceId] };
		} else if (!current.includes(interfaceId)) {
			updated[index] = { ...assignment, ip_address_ids: [...current, interfaceId] };
		}
		hostAssignments = updated;
	}

	function removeInterfaceFromScope(index: number, interfaceIndex: number) {
		const updated = [...hostAssignments];
		const assignment = updated[index];
		if (!assignment || assignment.ip_address_ids === null) return;
		const next = assignment.ip_address_ids.filter((_, i) => i !== interfaceIndex);
		// Empty list reverts to "all interfaces" (null)
		updated[index] = { ...assignment, ip_address_ids: next.length === 0 ? null : next };
		hostAssignments = updated;
	}
</script>

{#snippet networksSurface()}
	<ListManager
		label={common_networks()}
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
{/snippet}

{#snippet hostsSurface()}
	<ListConfigEditor items={selectedHosts}>
		<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex>
			<ListManager
				label={common_hosts()}
				placeholder={credentials_assignHostPlaceholder()}
				emptyMessage={credentials_assignHostEmpty()}
				allowReorder={false}
				options={availableHosts}
				{items}
				itemClickAction="edit"
				optionDisplayComponent={HostDisplay}
				itemDisplayComponent={HostDisplay}
				{onEdit}
				{highlightedIndex}
				onAdd={addHost}
				onRemove={removeHost}
			/>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem let:selectedIndex>
			{#if selectedItem}
				{@const hostIps = hostIpAddresses(selectedItem.id)}
				{#if hostIps.length > 0}
					<div class="space-y-4">
						<ConfigHeader title={selectedItem.name} subtitle={hosts_credentialScopeSubtitle()} />
						<ListManager
							label={credentials_ipScopeLabel()}
							emptyMessage={credentials_ipScopeAllDefault()}
							placeholder={credentials_ipScopePlaceholder()}
							allowReorder={false}
							options={hostIps}
							items={getScopedInterfaces(selectedIndex)}
							optionDisplayComponent={IPAddressDisplay}
							itemDisplayComponent={IPAddressDisplay}
							getOptionContext={() => getInterfaceContext()}
							getItemContext={() => getInterfaceContext()}
							onAdd={(id) => addInterfaceToScope(selectedIndex, id)}
							onRemove={(i) => removeInterfaceFromScope(selectedIndex, i)}
						/>
					</div>
				{:else}
					<EntityConfigEmpty title={selectedItem.name} subtitle={credentials_addInterfaces()} />
				{/if}
			{:else}
				<EntityConfigEmpty
					title={credentials_noHostSelected()}
					subtitle={credentials_selectHostSubtitle()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>
{/snippet}

<div class="flex min-h-0 flex-1 flex-col gap-4">
	{#if dualScope}
		<SegmentedControl
			options={segmentOptions}
			selected={activeSurface}
			onchange={(v) => (activeSurface = v as 'networks' | 'hosts')}
			size="md"
		/>
		{#if activeSurface === 'networks'}
			{@render networksSurface()}
		{:else}
			{@render hostsSurface()}
		{/if}
	{:else if supportsBroadcast}
		{@render networksSurface()}
	{:else if supportsPerHost}
		{@render hostsSurface()}
	{/if}
</div>
