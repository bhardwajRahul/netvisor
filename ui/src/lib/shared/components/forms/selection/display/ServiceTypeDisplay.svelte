<script lang="ts" context="module">
	import type { ServicedDefinitionMetadata, TypedTypeMetadata } from '$lib/shared/stores/metadata';

	type ServiceType = TypedTypeMetadata<ServicedDefinitionMetadata>;

	export const ServiceTypeDisplay: EntityDisplayComponent<ServiceType, object> = {
		getId: (serviceType: ServiceType) => serviceType.id,
		getLabel: (serviceType: ServiceType) => serviceType.name ?? '',
		getDescription: (serviceType: ServiceType) => serviceType.description ?? '',
		getIcon: (serviceType: ServiceType) => serviceDefinitions.getIconComponent(serviceType.id),
		getIconColor: (serviceType: ServiceType) =>
			serviceDefinitions.getColorHelper(serviceType.id).icon,
		getTags: (serviceType: ServiceType) => [
			{
				label: serviceType.category ?? '',
				color: serviceType.color
			}
		],
		getCategory: (serviceType: ServiceType) => serviceType.category ?? ''
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { EntityDisplayComponent } from '../types';

	export let item: ServiceType;
	export let context = {};
</script>

<ListSelectItem {item} {context} displayComponent={ServiceTypeDisplay} />
