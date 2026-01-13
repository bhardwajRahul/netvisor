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
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import * as m from '$lib/paraglide/messages';

	// TanStack Query hooks
	const servicesQuery = useServicesQuery();
	const portsQuery = usePortsQuery();
	const subnetsQuery = useSubnetsQuery();

	let servicesData = $derived(servicesQuery.data?.items ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

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
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		onChange?: (updatedService: Service) => void;
		selectedPortBindings?: PortBinding[];
		index?: number;
		currentServices?: Service[];
	}

	let {
		host,
		service,
		form,
		onChange = () => {},
		selectedPortBindings = $bindable([]),
		index = -1,
		currentServices = []
	}: Props = $props();

	let serviceMetadata = $derived(
		service ? serviceDefinitions.getItem(service.service_definition) : null
	);

	// Field name for this service's name in the form array
	let nameFieldName = $derived(`services[${index}].name`);

	// Notify parent of name changes for real-time sync
	function handleNameChange(value: string) {
		onChange({ ...service, name: value });
	}

	// Port Bindings Logic
	let portBindings = $derived(service.bindings.filter((b) => b.type === 'Port') as PortBinding[]);

	// Get the actual index of a binding in service.bindings array (for form field naming)
	function getBindingIndex(bindingId: string): number {
		return service.bindings.findIndex((b) => b.id === bindingId);
	}

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
		host.interfaces.flatMap((iface) => {
			// Can't add Port binding if THIS service has an Interface binding on this interface
			if (interfacesWithInterfaceBindingsThisService.has(iface.id)) {
				return [];
			}

			return host.ports
				.filter((port) => {
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

	let canCreatePortBinding = $derived(availablePortCombinations.length > 0);

	// Available interfaces for new Interface bindings
	// Only includes saved interfaces (those already in global store)
	let availableInterfacesForInterfaceBinding = $derived(
		host.interfaces.filter((iface) => {
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
			<div class="text-primary font-medium">{m.hosts_services_detailsSection()}</div>
			<!-- Service Name Field -->
			<form.Field
				name={nameFieldName}
				validators={{
					onBlur: ({ value }: { value: string }) => required(value) || max(100)(value),
					onChange: ({ value }: { value: string }) => required(value) || max(100)(value)
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleNameChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={m.common_name()}
						id="service_name_{service.id}"
						placeholder={m.hosts_services_namePlaceholder()}
						required={true}
						{field}
					/>
				{/snippet}
			</form.Field>

			<!-- service prop comes via slot, so use callback pattern instead of bind: -->
			<TagPicker
				selectedTagIds={service.tags}
				onChange={(tags) => onChange({ ...service, tags })}
			/>
		</div>

		<div>
			<div class="text-primary font-medium">{m.hosts_services_bindingsSection()}</div>
			<span class="text-muted text-xs">
				{m.hosts_services_bindingsHelp()}
			</span>
		</div>
		<!-- Port Bindings -->
		<div class="space-y-4">
			{#key `${service.id}`}
				<ListManager
					label={m.hosts_services_portBindings()}
					helpText={m.hosts_services_portBindingsHelp()}
					placeholder={m.hosts_services_selectBinding()}
					createNewLabel={m.hosts_services_newBinding()}
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
						services: currentServices.length > 0 ? currentServices : servicesData,
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

			<!-- Hidden form fields for port binding validation -->
			{#each portBindings as binding (binding.id)}
				{@const bindingIndex = getBindingIndex(binding.id)}
				<form.Field
					name={`services[${index}].bindings[${bindingIndex}].port_id`}
					validators={{
						onChange: () => (!binding.port_id ? m.hosts_services_portRequired() : undefined),
						onBlur: () => (!binding.port_id ? m.hosts_services_portRequired() : undefined)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<input type="hidden" value={field.state.value} />
					{/snippet}
				</form.Field>
			{/each}
		</div>

		<!-- Interface Bindings -->
		<div class="space-y-4">
			{#key service.id}
				<ListManager
					label={m.hosts_services_interfaceBindings()}
					helpText={m.hosts_services_interfaceBindingsHelp()}
					placeholder={m.hosts_services_selectBinding()}
					createNewLabel={m.hosts_services_newBinding()}
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
						services: currentServices.length > 0 ? currentServices : servicesData,
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
