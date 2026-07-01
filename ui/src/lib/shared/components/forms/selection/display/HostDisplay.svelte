<script lang="ts" context="module">
	import type { Host, Interface, Port, Service } from '$lib/features/hosts/types/base';
	import { entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { entityRef } from '$lib/shared/components/data/types';

	// Context provides the host's children (interfaces, ports, services)
	export interface HostDisplayContext {
		interfaces?: Interface[];
		ports?: Port[];
		services?: Service[];
		showEntityTagPicker?: boolean;
		tagPickerDisabled?: boolean;
		entityTags?: import('$lib/features/tags/types/base').Tag[];
		allowTagCreate?: boolean;
		showEditableEntityDescription?: boolean;
		entityDescription?: string | null;
		entityDescriptionDisabled?: boolean;
		onEntityDescriptionSave?: (value: string | null) => void;
		compact?: boolean;
		/** A non-null `disabledReason` renders the option disabled with that tooltip. */
		disabledReason?: string | null;
	}

	export const HostDisplay: EntityDisplayComponent<Host, HostDisplayContext> = {
		getId: (host) => host.id,
		getDisabled: (_host, context) => !!context?.disabledReason,
		getDisabledReason: (_host, context) => context?.disabledReason ?? null,
		getLabel: (host) => host.name,
		getDescription: (host) => host.hostname || 'No Hostname',
		getIcon: (host, context) => {
			const services = context?.services?.filter((s) => s.host_id == host.id) ?? [];
			const firstService = services.length > 0 ? services[0] : null;
			if (firstService) {
				return serviceDefinitions.getIconComponent(firstService.service_definition);
			} else {
				return entities.getIconComponent('Host');
			}
		},
		getIconColor: () => entities.getColorHelper('Host').icon,
		getTags: (host, context) => {
			if (context?.compact) return [];
			const services = context?.services?.filter((s) => s.host_id == host.id) ?? [];
			return services.map((service) => ({
				label: serviceDefinitions.getName(service.service_definition),
				color: entities.getColorHelper('Service').color,
				entityRef: entityRef('Service', service.id, service)
			}));
		},
		getTagPickerProps: (host: Host, context: HostDisplayContext) => {
			if (!context.showEntityTagPicker) return null;
			return {
				selectedTagIds: host.tags,
				entityId: host.id,
				entityType: 'Host' as const,
				availableTags: context.entityTags,
				allowCreate: context.allowTagCreate
			};
		}
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: Host;
	export let context: HostDisplayContext = {};
</script>

<ListSelectItem {item} {context} displayComponent={HostDisplay} />
