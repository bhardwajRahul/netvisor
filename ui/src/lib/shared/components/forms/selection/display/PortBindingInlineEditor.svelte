<script lang="ts">
	import { untrack } from 'svelte';
	import { formatIPAddress } from '$lib/features/hosts/queries';
	import { ALL_IP_ADDRESSES, type HostFormData } from '$lib/features/hosts/types/base';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import type { PortBinding, Service } from '$lib/features/services/types/base';
	import { formatPort } from '$lib/shared/utils/formatting';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { hosts_ports_noAvailableOnInterface } from '$lib/paraglide/messages';

	// TanStack Query hooks
	const servicesQuery = useServicesCacheQuery();
	const subnetsQuery = useSubnetsQuery();
	let servicesData = $derived(servicesQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	interface Props {
		binding: PortBinding;
		onUpdate?: (updates: Partial<PortBinding>) => void;
		service?: Service;
		host?: HostFormData;
		services?: Service[];
	}

	let {
		binding,
		onUpdate = () => {},
		service = undefined,
		host = undefined,
		services = undefined
	}: Props = $props();

	// Use services from props (current editing state) if provided, otherwise fall back to global cache
	let effectiveServicesData = $derived(services ?? servicesData);

	// Type guard for services with Port bindings
	function isServiceWithPortBindings(svc: Service): svc is Service {
		return svc.bindings.length === 0 || svc.bindings.every((b) => b.type === 'Port');
	}

	// Check if this port+interface combination conflicts with existing bindings
	function getConflictingService(portId: string, interfaceId: string | null): Service | null {
		// Get services that have a binding on this port
		const servicesForPort = effectiveServicesData.filter((s) =>
			s.bindings.some((b) => b.type === 'Port' && b.port_id === portId)
		);

		// Check OTHER services
		const otherServices = servicesForPort
			.filter((s) => s.id !== service?.id)
			.filter(isServiceWithPortBindings);

		for (const svc of otherServices) {
			const hasConflict = svc.bindings.some((b) => {
				// If either binding is to ALL_IP_ADDRESSES (null), they conflict
				if (b.ip_address_id === null || interfaceId === null) {
					return true;
				}
				// Otherwise, they conflict only if they're the same specific interface
				return b.ip_address_id === interfaceId;
			});
			if (hasConflict) return svc;
		}

		// Check OTHER bindings in current service
		if (service) {
			const otherBindings = service.bindings.filter(
				(b) => b.type === 'Port' && b.id !== binding.id && b.port_id === portId
			);
			const hasConflict = otherBindings.some((b) => {
				// If either binding is to ALL_IP_ADDRESSES (null), they conflict
				if (b.ip_address_id === null || interfaceId === null) {
					return true;
				}
				// Otherwise, they conflict only if they're the same specific interface
				return b.ip_address_id === interfaceId;
			});
			if (hasConflict) return service;
		}

		return null;
	}

	// Create interface options with disabled state
	let ipAddressOptions = $derived(
		host?.ip_addresses.map((ipAddr) => {
			// Check for IP Address binding conflict - can't add Port binding if THIS service has IP Address binding here
			const thisServiceHasIPAddressBinding = service?.bindings.some(
				(b) => b.type === 'IPAddress' && b.ip_address_id === ipAddr.id && b.id !== binding.id
			);
			if (thisServiceHasIPAddressBinding) {
				return {
					ipAddr,
					disabled: true,
					reason: 'This service has an IP Address binding here',
					boundService: service
				};
			}

			return {
				ipAddr,
				disabled: false,
				reason: null,
				boundService: null
			};
		}) || []
	);

	// Check ALL_IP_ADDRESSES option
	let allIPAddressesOption = $derived(
		(() => {
			// Can't select "All IP Addresses" if this service has ANY IP Address bindings
			// (since "All IP Addresses" would include those interfaces)
			const hasIPAddressBindings = service?.bindings.some((b) => b.type === 'IPAddress');
			if (hasIPAddressBindings) {
				return {
					ipAddr: ALL_IP_ADDRESSES,
					disabled: true,
					reason: 'Service has IP Address bindings',
					boundService: service
				};
			}

			return {
				ipAddr: ALL_IP_ADDRESSES,
				disabled: false,
				reason: null,
				boundService: null
			};
		})()
	);

	// Create port options with disabled state
	let portOptions = $derived(
		host?.ports.map((p) => {
			const boundService = getConflictingService(p.id, binding.ip_address_id);
			return {
				port: p,
				disabled: boundService !== null && p.id !== binding.port_id,
				reason: boundService ? `Bound by ${boundService.name}` : null,
				boundService
			};
		}) || []
	);

	// Local state for select values - use sentinel for ALL_IP_ADDRESSES
	const ALL_IP_ADDRESSES_SENTINEL = '__ALL_INTERFACES__';
	let selectedInterface = $state(
		untrack(() =>
			binding.ip_address_id === null ? ALL_IP_ADDRESSES_SENTINEL : binding.ip_address_id
		)
	);
	let selectedPort = $state(untrack(() => binding.port_id ?? ''));

	// Sync local state when binding changes externally
	$effect(() => {
		selectedInterface =
			binding.ip_address_id === null ? ALL_IP_ADDRESSES_SENTINEL : binding.ip_address_id;
		selectedPort = binding.port_id ?? '';
	});

	// Check if there are any valid (non-disabled) port options
	let hasValidPortOptions = $derived(portOptions.some((opt) => !opt.disabled));

	// Handle interface selection change
	function handleInterfaceChange(event: Event) {
		const target = event.target as HTMLSelectElement;
		const newValue = target.value;
		selectedInterface = newValue;

		const interfaceId = newValue === ALL_IP_ADDRESSES_SENTINEL ? null : newValue;
		if (interfaceId !== binding.ip_address_id) {
			// Check if current port is still valid on the new interface
			const currentPortConflict = binding.port_id
				? getConflictingService(binding.port_id, interfaceId)
				: null;

			if (currentPortConflict || !binding.port_id) {
				// Current port conflicts on new interface OR no port selected - find first valid port
				const firstValidPort = host?.ports.find((p) => !getConflictingService(p.id, interfaceId));
				// Reset to valid port, or empty if none available
				onUpdate({ ip_address_id: interfaceId, port_id: firstValidPort?.id ?? '' });
			} else {
				onUpdate({ ip_address_id: interfaceId });
			}
		}
	}

	// Handle port selection change
	function handlePortChange(event: Event) {
		const target = event.target as HTMLSelectElement;
		const newValue = target.value;
		selectedPort = newValue;

		if (newValue !== binding.port_id) {
			onUpdate({ port_id: newValue });
		}
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="flex-1" onclick={(e) => e.stopPropagation()}>
	<div class="text-secondary mb-1 block text-xs font-medium">Port Binding</div>

	{#if !service}
		<div class="text-danger rounded border border-red-600 bg-red-900/20 px-2 py-1 text-xs">
			Service not found
		</div>
	{:else if !host}
		<div class="text-danger rounded border border-red-600 bg-red-900/20 px-2 py-1 text-xs">
			Host not found
		</div>
	{:else}
		<div class="flex gap-3">
			{#if host.ip_addresses && host.ip_addresses.length === 0}
				<div class="flex-1">
					<InlineWarning title="" body="No IP addresses configured on host" />
				</div>
			{:else if host.ip_addresses.length > 0}
				<div class="flex-1">
					<label for="interface-select-{binding.id}" class="text-tertiary mb-1 block text-xs"
						>Interface</label
					>
					<select
						id="interface-select-{binding.id}"
						class="input-field w-full"
						value={selectedInterface}
						onchange={handleInterfaceChange}
					>
						{#each ipAddressOptions as { ipAddr, disabled, reason } (ipAddr.id)}
							<option value={ipAddr.id} {disabled}>
								{formatIPAddress(ipAddr, isContainerSubnetFn)}{disabled && reason
									? ` - ${reason}`
									: ''}
							</option>
						{/each}
						<option value={ALL_IP_ADDRESSES_SENTINEL} disabled={allIPAddressesOption.disabled}>
							{formatIPAddress(
								ALL_IP_ADDRESSES,
								isContainerSubnetFn
							)}{allIPAddressesOption.disabled && allIPAddressesOption.reason
								? ` - ${allIPAddressesOption.reason}`
								: ''}
						</option>
					</select>
				</div>
			{/if}

			{#if host.ports.length === 0}
				<div class="flex-1">
					<div
						class="rounded border border-yellow-600 bg-yellow-900/20 px-2 py-1 text-xs text-warning"
					>
						No ports configured on host
					</div>
				</div>
			{:else if !hasValidPortOptions}
				<div class="flex-1">
					<label for="port-select-{binding.id}" class="text-tertiary mb-1 block text-xs">Port</label
					>
					<InlineDanger title={hosts_ports_noAvailableOnInterface()} />
				</div>
			{:else}
				<div class="flex-1">
					<label for="port-select-{binding.id}" class="text-tertiary mb-1 block text-xs">Port</label
					>
					<select
						id="port-select-{binding.id}"
						class="input-field w-full"
						value={selectedPort}
						onchange={handlePortChange}
					>
						{#each portOptions as { port, disabled, reason } (port.id)}
							<option value={port.id} {disabled}>
								{formatPort(port)}{disabled && reason ? ` - ${reason}` : ''}
							</option>
						{/each}
					</select>
				</div>
			{/if}
		</div>
	{/if}
</div>
