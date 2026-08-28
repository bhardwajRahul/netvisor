<script lang="ts" context="module">
	import { entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { Binding, Service } from '$lib/features/services/types/base';
	import type { Host, IPAddress, Port } from '$lib/features/hosts/types/base';
	import { formatPort } from '$lib/shared/utils/formatting';
	import { ALL_IP_ADDRESSES } from '$lib/features/hosts/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';

	// Context for binding display - needs access to services, hosts, interfaces, ports
	export interface BindingWithServiceContext {
		services: Service[];
		hosts: Host[];
		ip_addresses: IPAddress[];
		ports: Port[];
		isContainerSubnet: (subnetId: string) => boolean;
		compact?: boolean;
	}

	// Helper to format IP address for display
	function formatInterfaceForBinding(
		iface: IPAddress | typeof ALL_IP_ADDRESSES,
		isContainerSubnet: (subnetId: string) => boolean
	): string {
		if (iface.id == null) return iface.name;
		return isContainerSubnet(iface.subnet_id)
			? (iface.name ?? iface.ip_address)
			: (iface.name ? iface.name + ': ' : '') + iface.ip_address;
	}

	// Helper to get binding display name
	function getBindingDisplayNameFromContext(
		binding: Binding,
		context: BindingWithServiceContext
	): string {
		if (binding.type === 'IPAddress') {
			const iface = context.ip_addresses.find((i) => i.id === binding.ip_address_id);
			return iface
				? formatInterfaceForBinding(iface, context.isContainerSubnet)
				: 'Unknown Interface';
		} else {
			const port = context.ports.find((p) => p.id === binding.port_id);
			const iface = binding.ip_address_id
				? context.ip_addresses.find((i) => i.id === binding.ip_address_id)
				: ALL_IP_ADDRESSES;
			const portFormatted = port ? formatPort(port) : 'Unknown Port';
			const interfaceFormatted = iface
				? formatInterfaceForBinding(iface, context.isContainerSubnet)
				: 'Unknown Interface';
			return interfaceFormatted + ' · ' + portFormatted;
		}
	}

	export const BindingWithServiceDisplay: EntityDisplayComponent<
		Binding,
		BindingWithServiceContext
	> = {
		getId: (binding: Binding) => binding.id,
		getLabel: (binding: Binding, context: BindingWithServiceContext) => {
			const servicesData = context?.services ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			return service?.name || 'Unknown Service';
		},
		getDescription: (binding: Binding, context: BindingWithServiceContext) =>
			getBindingDisplayNameFromContext(binding, context),
		getIcon: (binding: Binding, context: BindingWithServiceContext) => {
			const servicesData = context?.services ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			if (!service) return entities.getIconComponent('Service');

			return serviceDefinitions.getIconComponent(service.service_definition);
		},
		getIconColor: (binding: Binding, context: BindingWithServiceContext) => {
			const servicesData = context?.services ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			if (!service) return 'text-secondary';

			return serviceDefinitions.getColorHelper(service.service_definition).icon;
		},
		getTags: () => [],
		getCategory: (binding: Binding, context: BindingWithServiceContext) => {
			const servicesData = context?.services ?? [];
			const hostsData = context?.hosts ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			if (!service) return null;
			const host = hostsData.find((h) => h.id === service.host_id);
			if (!host) return null;

			return hostDisplayName(host);
		}
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: Binding;
	export let context: BindingWithServiceContext = {
		services: [],
		hosts: [],
		ip_addresses: [],
		ports: [],
		isContainerSubnet: () => false
	};
</script>

<ListSelectItem {context} {item} displayComponent={BindingWithServiceDisplay} />
