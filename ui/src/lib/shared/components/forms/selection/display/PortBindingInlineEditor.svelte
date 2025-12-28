<script lang="ts">
	import { formatInterface } from '$lib/features/hosts/queries';
	import { ALL_INTERFACES, type HostFormData } from '$lib/features/hosts/types/base';
	import { useServicesQuery } from '$lib/features/services/queries';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import type { PortBinding, Service } from '$lib/features/services/types/base';
	import { formatPort } from '$lib/shared/utils/formatting';

	// TanStack Query hooks
	const servicesQuery = useServicesQuery();
	const interfacesQuery = useInterfacesQuery();
	const portsQuery = usePortsQuery();
	const subnetsQuery = useSubnetsQuery();
	let servicesData = $derived(servicesQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	// Check if an interface is unsaved (not yet in the query cache)
	function isInterfaceUnsaved(id: string): boolean {
		return !interfacesData.some((i) => i.id === id);
	}

	// Check if a port is unsaved (not yet in the query cache)
	function isPortUnsaved(id: string): boolean {
		return !portsData.some((p) => p.id === id);
	}

	interface Props {
		binding: PortBinding;
		onUpdate?: (updates: Partial<PortBinding>) => void;
		service?: Service;
		host?: HostFormData;
	}

	let {
		binding,
		onUpdate = () => {},
		service = undefined,
		host = undefined
	}: Props = $props();

	// Type guard for services with Port bindings
	function isServiceWithPortBindings(svc: Service): svc is Service {
		return svc.bindings.length === 0 || svc.bindings.every((b) => b.type === 'Port');
	}

	// Check if this port+interface combination conflicts with existing bindings
	function getConflictingService(portId: string, interfaceId: string | null): Service | null {
		// Get services that have a binding on this port
		const servicesForPort = servicesData.filter((s) =>
			s.bindings.some((b) => b.type === 'Port' && b.port_id === portId)
		);

		// Check OTHER services
		const otherServices = servicesForPort
			.filter((s) => s.id !== service?.id)
			.filter(isServiceWithPortBindings);

		for (const svc of otherServices) {
			const hasConflict = svc.bindings.some((b) => {
				// If either binding is to ALL_INTERFACES (null), they conflict
				if (b.interface_id === null || interfaceId === null) {
					return true;
				}
				// Otherwise, they conflict only if they're the same specific interface
				return b.interface_id === interfaceId;
			});
			if (hasConflict) return svc;
		}

		// Check OTHER bindings in current service
		if (service) {
			const otherBindings = service.bindings.filter(
				(b) => b.type === 'Port' && b.id !== binding.id && b.port_id === portId
			);
			const hasConflict = otherBindings.some((b) => {
				// If either binding is to ALL_INTERFACES (null), they conflict
				if (b.interface_id === null || interfaceId === null) {
					return true;
				}
				// Otherwise, they conflict only if they're the same specific interface
				return b.interface_id === interfaceId;
			});
			if (hasConflict) return service;
		}

		return null;
	}

	// Create interface options with disabled state
	let interfaceOptions = $derived(
		host?.interfaces.map((iface) => {
			// Check if interface is unsaved (not in query cache) - can't bind until host is saved
			if (isInterfaceUnsaved(iface.id)) {
				return {
					iface,
					disabled: true,
					reason: 'Save host first',
					boundService: null
				};
			}

			// Check for Interface binding conflict - can't add Port binding if THIS service has Interface binding here
			const thisServiceHasInterfaceBinding = service?.bindings.some(
				(b) => b.type === 'Interface' && b.interface_id === iface.id && b.id !== binding.id
			);
			if (thisServiceHasInterfaceBinding) {
				return {
					iface,
					disabled: true,
					reason: 'This service has an Interface binding here',
					boundService: service
				};
			}

			// Check for Port binding conflict (port_id is required for Port bindings)
			const boundService = binding.port_id
				? getConflictingService(binding.port_id, iface.id)
				: null;
			return {
				iface,
				disabled: boundService !== null && iface.id !== binding.interface_id,
				reason: boundService ? `Port bound by ${boundService.name}` : null,
				boundService
			};
		}) || []
	);

	// Check ALL_INTERFACES option
	let allInterfacesOption = $derived(
		(() => {
			const boundService = binding.port_id ? getConflictingService(binding.port_id, null) : null;
			return {
				iface: ALL_INTERFACES,
				disabled: boundService !== null && binding.interface_id !== null,
				reason: boundService ? `Port bound by ${boundService.name}` : null,
				boundService
			};
		})()
	);

	// Create port options with disabled state
	let portOptions = $derived(
		host?.ports.map((p) => {
			// Check if port is unsaved (not in query cache) - can't bind until host is saved
			if (isPortUnsaved(p.id)) {
				return {
					port: p,
					disabled: true,
					reason: 'Save host first',
					boundService: null
				};
			}

			const boundService = getConflictingService(p.id, binding.interface_id);
			return {
				port: p,
				disabled: boundService !== null && p.id !== binding.port_id,
				reason: boundService ? `Bound by ${boundService.name}` : null,
				boundService
			};
		}) || []
	);

	// Local state for select values - use sentinel for ALL_INTERFACES
	const ALL_INTERFACES_SENTINEL = '__ALL_INTERFACES__';
	let selectedInterface = $state(binding.interface_id === null ? ALL_INTERFACES_SENTINEL : binding.interface_id);
	let selectedPort = $state(binding.port_id ?? '');

	// Sync local state when binding changes externally
	$effect(() => {
		selectedInterface = binding.interface_id === null ? ALL_INTERFACES_SENTINEL : binding.interface_id;
		selectedPort = binding.port_id ?? '';
	});

	// Handle interface selection change
	function handleInterfaceChange(event: Event) {
		const target = event.target as HTMLSelectElement;
		const newValue = target.value;
		selectedInterface = newValue;

		const interfaceId = newValue === ALL_INTERFACES_SENTINEL ? null : newValue;
		if (interfaceId !== binding.interface_id) {
			onUpdate({ interface_id: interfaceId });
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

<div class="flex-1">
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
			{#if host.interfaces && host.interfaces.length === 0}
				<div class="flex-1">
					<div
						class="rounded border border-yellow-600 bg-yellow-900/20 px-2 py-1 text-xs text-warning"
					>
						No interfaces configured on host
					</div>
				</div>
			{:else if host.interfaces.length > 0}
				<div class="flex-1">
					<label for="interface-select-{binding.id}" class="text-tertiary mb-1 block text-xs">Interface</label>
					<select
						id="interface-select-{binding.id}"
						class="input-field w-full"
						value={selectedInterface}
						onchange={handleInterfaceChange}
					>
						{#each interfaceOptions as { iface, disabled, reason }}
							<option value={iface.id} {disabled}>
								{formatInterface(iface, isContainerSubnetFn)}{disabled && reason
									? ` - ${reason}`
									: ''}
							</option>
						{/each}
						<option
							value={ALL_INTERFACES_SENTINEL}
							disabled={allInterfacesOption.disabled}
						>
							{formatInterface(ALL_INTERFACES, isContainerSubnetFn)}{allInterfacesOption.disabled &&
							allInterfacesOption.reason
								? ` - ${allInterfacesOption.reason}`
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
			{:else}
				<div class="flex-1">
					<label for="port-select-{binding.id}" class="text-tertiary mb-1 block text-xs">Port</label>
					<select
						id="port-select-{binding.id}"
						class="input-field w-full"
						value={selectedPort}
						onchange={handlePortChange}
					>
						{#each portOptions as { port, disabled, reason }}
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
