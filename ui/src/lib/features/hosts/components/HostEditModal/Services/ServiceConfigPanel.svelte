<script lang="ts">
	import type { InterfaceBinding, PortBinding, Service } from '$lib/features/services/types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { pushWarning } from '$lib/shared/stores/feedback';
	import { required, max } from '$lib/shared/components/forms/validators';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import { v4 as uuidv4 } from 'uuid';
	import { useServicesQuery } from '$lib/features/services/queries';
	import { PortBindingDisplay } from '$lib/shared/components/forms/selection/display/PortBindingDisplay.svelte';
	import { InterfaceBindingDisplay } from '$lib/shared/components/forms/selection/display/InterfaceBindingDisplay.svelte';
	import MatchDetails from './MatchDetails.svelte';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';

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

	// Check if an interface is unsaved (not yet in the global cache)
	function isInterfaceUnsaved(id: string): boolean {
		return !interfacesData.some((i) => i.id === id);
	}

	// Check if a port is unsaved (not yet in the global cache)
	function isPortUnsaved(id: string): boolean {
		return !portsData.some((p) => p.id === id);
	}

	// Get services for a specific port
	function getServicesForPort(portId: string): Service[] {
		const port = portsData.find((p) => p.id === portId);
		if (!port) return [];

		return servicesData.filter(
			(s) =>
				s.host_id === port.host_id &&
				s.bindings.some((b) => b.type === 'Port' && (b as PortBinding).port_id === portId)
		);
	}

	interface Props {
		host: HostFormData;
		service: Service;
		onChange?: (updatedService: Service) => void;
		selectedPortBindings?: PortBinding[];
		index?: number;
	}

	let {
		host,
		service,
		onChange = () => {},
		selectedPortBindings = $bindable([]),
		index = -1
	}: Props = $props();

	// Local state for form fields
	let name = $state(service.name);
	let nameError = $state<string | undefined>(undefined);

	let currentServiceId = $state(service.id);
	let currentServiceIndex = $state(index);

	let serviceMetadata = $derived(
		service ? serviceDefinitions.getItem(service.service_definition) : null
	);

	// Reset fields when service changes
	$effect(() => {
		if (currentServiceIndex !== index || currentServiceId !== service.id) {
			currentServiceIndex = index;
			currentServiceId = service.id;
			name = service.name;
			nameError = undefined;
		}
	});

	// Handle name change with validation
	function handleNameChange(e: Event) {
		const value = (e.target as HTMLInputElement).value;
		name = value;
		nameError = required(value) || max(100)(value);
		triggerNameChange();
	}

	function triggerNameChange() {
		if (name !== service.name) {
			onChange({
				...service,
				name: name
			});
		}
	}

	// Port Bindings Logic
	let portBindings = $derived(service.bindings.filter((b) => b.type === 'Port') as PortBinding[]);

	// Interface Bindings Logic
	let interfaceBindings = $derived(
		service.bindings.filter((b) => b.type === 'Interface') as InterfaceBinding[]
	);

	// Get interfaces that this service has Port bindings on
	let interfacesWithPortBindingsThisService = $derived(
		new Set(portBindings.map((b) => b.interface_id).filter((id): id is string => id !== null))
	);

	// Check if this service has a Port binding on "All Interfaces"
	let hasPortBindingOnAllInterfaces = $derived(portBindings.some((b) => b.interface_id === null));

	// Get interfaces that this service has Interface bindings on
	let interfacesWithInterfaceBindingsThisService = $derived(
		new Set(interfaceBindings.map((b) => b.interface_id))
	);

	// Available port+interface combinations for new Port bindings
	// Only includes saved interfaces and ports (those not yet in global stores)
	let availablePortCombinations = $derived(
		host.interfaces
			.filter((iface) => !isInterfaceUnsaved(iface.id)) // Skip unsaved interfaces
			.flatMap((iface) => {
				// Can't add Port binding if THIS service has an Interface binding on this interface
				if (interfacesWithInterfaceBindingsThisService.has(iface.id)) {
					return [];
				}

				return host.ports
					.filter((port) => {
						// Skip unsaved ports
						if (isPortUnsaved(port.id)) return false;

						// Check if this specific port+interface combo is already bound by this service
						const alreadyBoundByThisService = portBindings.some(
							(b) => b.port_id === port.id && b.interface_id === iface.id
						);
						if (alreadyBoundByThisService) return false;

						// Get services for this port
						const servicesForPort = getServicesForPort(port.id);
						const otherServices = servicesForPort.filter((s) => s.id !== service.id);

						const boundByOtherService = otherServices.some((s) =>
							s.bindings.some(
								(b) =>
									b.type === 'Port' &&
									(b as PortBinding).port_id === port.id &&
									(b.interface_id === iface.id || b.interface_id === null)
							)
						);
						if (boundByOtherService) return false;

						// Check if this service has bound this port to ALL interfaces (null)
						const boundToAllInterfaces = portBindings.some(
							(b) => b.port_id === port.id && b.interface_id === null
						);
						if (boundToAllInterfaces) return false;

						return true;
					})
					.map((port) => ({ port, iface }));
			})
	);

	// Count of unsaved items for messaging
	let unsavedInterfaceCount = $derived(
		host.interfaces.filter((i) => isInterfaceUnsaved(i.id)).length
	);
	let unsavedPortCount = $derived(host.ports.filter((p) => isPortUnsaved(p.id)).length);
	let hasUnsavedItems = $derived(unsavedInterfaceCount > 0 || unsavedPortCount > 0);

	let canCreatePortBinding = $derived(availablePortCombinations.length > 0);

	// Available interfaces for new Interface bindings
	// Only includes saved interfaces (those already in global store)
	let availableInterfacesForInterfaceBinding = $derived(
		host.interfaces.filter((iface) => {
			// Skip unsaved interfaces
			if (isInterfaceUnsaved(iface.id)) return false;

			// Can't add Interface binding if service has Port binding on "All Interfaces"
			if (hasPortBindingOnAllInterfaces) return false;

			// Can't add Interface binding if this service already has one on this interface
			if (interfaceBindings.some((b) => b.interface_id === iface.id)) {
				return false;
			}

			// Can't add Interface binding if THIS service has Port bindings on this interface
			if (interfacesWithPortBindingsThisService.has(iface.id)) {
				return false;
			}

			return true;
		})
	);

	let canCreateInterfaceBinding = $derived(availableInterfacesForInterfaceBinding.length > 0);

	// Port Binding Handlers
	function handleCreatePortBinding() {
		if (!service) {
			pushWarning('Could not find service to create binding for');
			return;
		}

		if (host.interfaces.length === 0) {
			pushWarning("Host does not have any interfaces, can't create binding");
			return;
		}

		if (host.ports.length === 0) {
			pushWarning("Host does not have any ports, can't create binding");
			return;
		}

		if (!canCreatePortBinding) {
			pushWarning('No available port+interface combinations to bind');
			return;
		}

		const firstAvailable = availablePortCombinations[0];

		const binding: PortBinding = {
			type: 'Port',
			id: uuidv4(),
			service_id: service.id,
			network_id: service.network_id,
			port_id: firstAvailable.port.id,
			interface_id: firstAvailable.iface.id,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		};

		onChange({
			...service,
			bindings: [...service.bindings, binding]
		});
	}

	function handleRemovePortBinding(index: number) {
		if (!service) {
			pushWarning('Could not find service to remove binding for');
			return;
		}

		const portBindingToRemove = portBindings[index];
		const fullIndex = service.bindings.findIndex((b) => b.id === portBindingToRemove.id);
		onChange({
			...service,
			bindings: service.bindings.filter((_, i) => i !== fullIndex)
		});
	}

	function handleUpdatePortBinding(binding: PortBinding, index: number) {
		if (!service) return;

		const portBindingToUpdate = portBindings[index];
		const fullIndex = service.bindings.findIndex((b) => b.id === portBindingToUpdate.id);

		const updatedBindings = [...service.bindings];
		updatedBindings[fullIndex] = {
			...updatedBindings[fullIndex],
			interface_id: binding.interface_id,
			port_id: binding.port_id
		} as PortBinding;

		onChange({
			...service,
			bindings: updatedBindings
		});
	}

	// Interface Binding Handlers
	function handleCreateInterfaceBinding() {
		if (!service) {
			pushWarning('Could not find service to create binding for');
			return;
		}

		if (host.interfaces.length === 0) {
			pushWarning("Host does not have any interfaces, can't create binding");
			return;
		}

		if (!canCreateInterfaceBinding) {
			pushWarning('No available interfaces to bind');
			return;
		}

		const firstAvailable = availableInterfacesForInterfaceBinding[0];

		const binding: InterfaceBinding = {
			type: 'Interface',
			id: uuidv4(),
			service_id: service.id,
			network_id: service.network_id,
			interface_id: firstAvailable.id,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		};

		onChange({
			...service,
			bindings: [...service.bindings, binding]
		});
	}

	function handleRemoveInterfaceBinding(index: number) {
		if (!service) {
			pushWarning('Could not find service to remove binding for');
			return;
		}

		const interfaceBindingToRemove = interfaceBindings[index];
		const fullIndex = service.bindings.findIndex((b) => b.id === interfaceBindingToRemove.id);

		onChange({
			...service,
			bindings: service.bindings.filter((_, i) => i !== fullIndex)
		});
	}

	function handleUpdateInterfaceBinding(binding: InterfaceBinding, index: number) {
		if (!service) return;

		const interfaceBindingToUpdate = interfaceBindings[index];
		const fullIndex = service.bindings.findIndex((b) => b.id === interfaceBindingToUpdate.id);

		const updatedBindings = [...service.bindings];
		updatedBindings[fullIndex] = {
			...updatedBindings[fullIndex],
			interface_id: binding.interface_id
		} as InterfaceBinding;

		onChange({
			...service,
			bindings: updatedBindings
		});
	}
</script>

{#if service && serviceMetadata}
	<div class="space-y-6">
		<ConfigHeader title={serviceMetadata.name ?? ''} subtitle={serviceMetadata.description} />

		<!-- Basic Configuration -->
		<div class="space-y-4">
			<div class="text-primary font-medium">Details</div>
			<!-- Service Name Field -->
			<div>
				<label
					for="service_name_{service.id}"
					class="text-secondary mb-1 block text-sm font-medium"
				>
					Name <span class="text-red-400">*</span>
				</label>
				<input
					type="text"
					id="service_name_{service.id}"
					class="input-field w-full"
					placeholder="Enter a descriptive name..."
					value={name}
					oninput={handleNameChange}
				/>
				{#if nameError}
					<p class="mt-1 text-xs text-red-400">{nameError}</p>
				{/if}
			</div>

			<!-- service prop comes via slot, so use callback pattern instead of bind: -->
			<TagPicker
				selectedTagIds={service.tags}
				onChange={(tags) => onChange({ ...service, tags })}
			/>
		</div>

		<div>
			<div class="text-primary font-medium">Bindings</div>
			<span class="text-muted text-xs">
				For a given interface, a service can have either port bindings OR an interface binding, not
				both.
			</span>
			{#if hasUnsavedItems}
				<InlineWarning
					title={(() => {
						if (unsavedInterfaceCount > 0 && unsavedPortCount > 0) {
							return `${unsavedInterfaceCount} unsaved interface${unsavedInterfaceCount > 1 ? 's' : ''} and ${unsavedPortCount}
							unsaved port${unsavedPortCount > 1 ? 's' : ''} — save host to bind to them.`;
						} else if (unsavedInterfaceCount > 0) {
							return `${unsavedInterfaceCount} unsaved interface${unsavedInterfaceCount > 1 ? 's' : ''} — save
							host to bind to ${unsavedInterfaceCount > 1 ? 'them' : 'it'}.`;
						} else {
							return `${unsavedPortCount} unsaved port${unsavedPortCount > 1 ? 's' : ''} — save host to bind to
							${unsavedPortCount > 1 ? 'them' : 'it'}.`;
						}
					})()}
				/>
			{/if}
		</div>
		<!-- Port Bindings -->
		<div class="space-y-4">
			{#key `${service.id}`}
				<ListManager
					label="Port Bindings"
					helpText="Configure which ports this service listens on for a given interface"
					placeholder="Select a binding to add"
					createNewLabel="New Binding"
					allowDuplicates={false}
					allowItemEdit={() => true}
					allowItemRemove={() => true}
					allowSelection={true}
					allowReorder={false}
					allowCreateNew={true}
					itemClickAction="select"
					allowAddFromOptions={false}
					disableCreateNewButton={!canCreatePortBinding}
					options={[] as PortBinding[]}
					optionDisplayComponent={PortBindingDisplay}
					itemDisplayComponent={PortBindingDisplay}
					items={portBindings}
					getItemContext={() => ({
						service,
						host,
						services: servicesData,
						interfaces: host.interfaces,
						ports: host.ports,
						isContainerSubnet: isContainerSubnetFn
					})}
					onCreateNew={handleCreatePortBinding}
					onRemove={handleRemovePortBinding}
					onEdit={handleUpdatePortBinding}
					onItemUpdate={(binding, index, updates) =>
						handleUpdatePortBinding({ ...binding, ...updates }, index)}
					bind:selectedItems={selectedPortBindings}
				/>
			{/key}
		</div>

		<!-- Interface Bindings -->
		<div class="space-y-4">
			{#key service.id}
				<ListManager
					label="Interface Bindings"
					helpText="Configure which interfaces this service is present on (without listening on ports)."
					placeholder="Select a binding to add"
					createNewLabel="New Binding"
					allowDuplicates={false}
					allowItemEdit={() => true}
					allowItemRemove={() => true}
					allowReorder={false}
					allowCreateNew={true}
					allowAddFromOptions={false}
					disableCreateNewButton={!canCreateInterfaceBinding}
					options={[] as InterfaceBinding[]}
					optionDisplayComponent={InterfaceBindingDisplay}
					itemDisplayComponent={InterfaceBindingDisplay}
					items={interfaceBindings}
					getItemContext={() => ({
						service,
						host,
						services: servicesData,
						interfaces: host.interfaces,
						isContainerSubnet: isContainerSubnetFn
					})}
					onCreateNew={handleCreateInterfaceBinding}
					onRemove={handleRemoveInterfaceBinding}
					onEdit={handleUpdateInterfaceBinding}
					onItemUpdate={(binding, index, updates) =>
						handleUpdateInterfaceBinding({ ...binding, ...updates }, index)}
				/>
			{/key}
		</div>

		{#if service.source.type === 'DiscoveryWithMatch' && service.source.details}
			<MatchDetails details={service.source.details} />
		{/if}
	</div>
{/if}
