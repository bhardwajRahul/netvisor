<script lang="ts">
	import { formatInterface } from '$lib/features/hosts/queries';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import type { InterfaceBinding, Service } from '$lib/features/services/types/base';

	// TanStack Query hooks
	const interfacesQuery = useInterfacesQuery();
	const subnetsQuery = useSubnetsQuery();
	let interfacesData = $derived(interfacesQuery.data ?? []);
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

	interface Props {
		binding: InterfaceBinding;
		onUpdate?: (updates: Partial<InterfaceBinding>) => void;
		service?: Service;
		host?: HostFormData;
	}

	let {
		binding,
		onUpdate = () => {},
		service = undefined,
		host = undefined
	}: Props = $props();

	// Interface binding must have an interface_id - look up from query data
	let iface = $derived(
		binding.interface_id ? interfacesData.find((i) => i.id === binding.interface_id) : null
	);

	// Create interface options with disabled state
	let interfaceOptions = $derived(
		host?.interfaces.map((iface) => {
			// Check if interface is unsaved (not in query cache) - can't bind until host is saved
			if (isInterfaceUnsaved(iface.id)) {
				return {
					iface,
					disabled: true,
					reason: 'Save host first'
				};
			}

			// Can't select if THIS service has Port bindings on this interface
			const thisServiceHasPortBindings = service?.bindings.some(
				(b) => b.type === 'Port' && b.interface_id === iface.id
			);
			if (thisServiceHasPortBindings && iface.id !== binding.interface_id) {
				return {
					iface,
					disabled: true,
					reason: 'This service has Port bindings on this interface'
				};
			}

			return {
				iface,
				disabled: false,
				reason: null
			};
		}) || []
	);

	// Local state for the select value
	let selectedValue = $state(binding.interface_id ?? '');

	// Sync local state when binding changes externally
	$effect(() => {
		selectedValue = binding.interface_id ?? '';
	});

	// Handle selection change
	function handleChange(event: Event) {
		const target = event.target as HTMLSelectElement;
		const newValue = target.value;
		selectedValue = newValue;

		if (newValue !== binding.interface_id) {
			onUpdate({ interface_id: newValue });
		}
	}
</script>

<div class="flex-1">
	<div class="text-secondary mb-1 block text-xs font-medium">Interface Binding</div>

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
			<div class="flex-1">
				{#if host.interfaces && host.interfaces.length === 0}
					<div
						class="rounded border border-yellow-600 bg-yellow-900/20 px-2 py-1 text-xs text-warning"
					>
						No interfaces configured on host
					</div>
				{:else if host.interfaces && host.interfaces.length === 1}
					<!-- Single interface - show as read-only -->
					<div class="text-secondary rounded border border-gray-600 bg-gray-700 px-2 py-1 text-sm">
						{iface ? formatInterface(iface, isContainerSubnetFn) : 'Unknown Interface'}
					</div>
				{:else if host.interfaces.length > 0}
					<!-- Multiple interfaces - show as dropdown -->
					<select
						class="input-field w-full"
						value={selectedValue}
						onchange={handleChange}
					>
						{#each interfaceOptions as { iface, disabled, reason }}
							<option value={iface.id} {disabled}>
								{formatInterface(iface, isContainerSubnetFn)}{disabled && reason
									? ` - ${reason}`
									: ''}
							</option>
						{/each}
					</select>
				{/if}
			</div>
		</div>
	{/if}
</div>
