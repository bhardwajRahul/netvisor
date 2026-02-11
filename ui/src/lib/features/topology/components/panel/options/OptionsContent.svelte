<script lang="ts">
	import { topologyOptions, selectedTopologyId, useTopologiesQuery } from '../../../queries';
	import { updateTagFilter } from '../../../interactions';
	import { edgeTypes, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import TagFilterGroup from './TagFilterGroup.svelte';
	import {
		common_categories,
		common_docker,
		common_hosts,
		common_infrastructure,
		common_services,
		common_subnets,
		common_title,
		common_visual,
		topology_dontFadeEdges,
		topology_dontFadeEdgesHelp,
		topology_groupDockerBridges,
		topology_groupDockerBridgesHelp,
		topology_hideEdgeTypes,
		topology_hideEdgeTypesHelp,
		topology_hidePorts,
		topology_hidePortsHelp,
		topology_hideResizeHandles,
		topology_hideResizeHandlesHelp,
		topology_hideServiceCategories,
		topology_hideServiceCategoriesHelp,
		topology_hideStuff,
		topology_hideVmOnContainer,
		topology_hideVmOnContainerHelp,
		topology_leftZone,
		topology_leftZoneCategoriesHelp,
		topology_leftZoneTitleHelp,
		topology_multiselectHelp,
		topology_showGatewayInLeftZone,
		topology_showGatewayInLeftZoneHelp,
		topology_tagFilter,
		topology_tagFilterHelp
	} from '$lib/paraglide/messages';

	// Get topology for entity_tags
	const topologiesQuery = useTopologiesQuery();
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let topology = $derived(topologiesData.find((t) => t.id === $selectedTopologyId));

	// Derive tags that are actually used per entity type
	let hostTagIds = $derived(new Set(topology?.hosts.flatMap((h) => h.tags) ?? []));
	let serviceTagIds = $derived(new Set(topology?.services.flatMap((s) => s.tags) ?? []));
	let subnetTagIds = $derived(new Set(topology?.subnets.flatMap((s) => s.tags) ?? []));

	// Filter entity_tags to only those used by each entity type
	let hostTags = $derived(topology?.entity_tags?.filter((t) => hostTagIds.has(t.id)) ?? []);
	let serviceTags = $derived(topology?.entity_tags?.filter((t) => serviceTagIds.has(t.id)) ?? []);
	let subnetTags = $derived(topology?.entity_tags?.filter((t) => subnetTagIds.has(t.id)) ?? []);

	// Check if there are any untagged entities
	let hasUntaggedHosts = $derived(topology?.hosts.some((h) => h.tags.length === 0) ?? false);
	let hasUntaggedServices = $derived(topology?.services.some((s) => s.tags.length === 0) ?? false);
	let hasUntaggedSubnets = $derived(topology?.subnets.some((s) => s.tags.length === 0) ?? false);

	// Toggle functions for tag filter
	function toggleHostTag(tagId: string) {
		topologyOptions.update((opts) => {
			const currentFilter = opts.local.tag_filter;
			const hiddenIds = currentFilter?.hidden_host_tag_ids ?? [];
			const idx = hiddenIds.indexOf(tagId);
			const newHiddenIds =
				idx === -1 ? [...hiddenIds, tagId] : hiddenIds.filter((id) => id !== tagId);
			opts.local.tag_filter = {
				hidden_host_tag_ids: newHiddenIds,
				hidden_service_tag_ids: currentFilter?.hidden_service_tag_ids ?? [],
				hidden_subnet_tag_ids: currentFilter?.hidden_subnet_tag_ids ?? []
			};
			return opts;
		});
	}

	function toggleServiceTag(tagId: string) {
		topologyOptions.update((opts) => {
			const currentFilter = opts.local.tag_filter;
			const hiddenIds = currentFilter?.hidden_service_tag_ids ?? [];
			const idx = hiddenIds.indexOf(tagId);
			const newHiddenIds =
				idx === -1 ? [...hiddenIds, tagId] : hiddenIds.filter((id) => id !== tagId);
			opts.local.tag_filter = {
				hidden_host_tag_ids: currentFilter?.hidden_host_tag_ids ?? [],
				hidden_service_tag_ids: newHiddenIds,
				hidden_subnet_tag_ids: currentFilter?.hidden_subnet_tag_ids ?? []
			};
			return opts;
		});
	}

	function toggleSubnetTag(tagId: string) {
		topologyOptions.update((opts) => {
			const currentFilter = opts.local.tag_filter;
			const hiddenIds = currentFilter?.hidden_subnet_tag_ids ?? [];
			const idx = hiddenIds.indexOf(tagId);
			const newHiddenIds =
				idx === -1 ? [...hiddenIds, tagId] : hiddenIds.filter((id) => id !== tagId);
			opts.local.tag_filter = {
				hidden_host_tag_ids: currentFilter?.hidden_host_tag_ids ?? [],
				hidden_service_tag_ids: currentFilter?.hidden_service_tag_ids ?? [],
				hidden_subnet_tag_ids: newHiddenIds
			};
			return opts;
		});
	}

	// Update tag filter stores when topology or options change
	$effect(() => {
		updateTagFilter(topology, $topologyOptions.local.tag_filter);
	});

	// Dynamic options loaded on mount
	let serviceCategories: { value: string; label: string }[] = $derived.by(() => {
		const serviceDefinitionItems = serviceDefinitions.getItems() || [];
		const categoriesSet = new Set(
			serviceDefinitionItems.map((i) => serviceDefinitions.getCategory(i.id))
		);
		return Array.from(categoriesSet)
			.filter((c) => c)
			.sort()
			.map((c) => ({ value: c, label: c }));
	});
	let eTypes: { value: string; label: string }[] = $derived.by(() => {
		return (edgeTypes.getItems() || []).map((e) => ({ value: e.id, label: e.id }));
	});

	interface TopologyFieldDef {
		id: string;
		label: () => string;
		type: 'boolean' | 'string' | 'multiselect';
		path: 'local' | 'request';
		key: string;
		helpText: () => string;
		section: () => string;
		getOptions?: () => { value: string; label: string }[];
		placeholder?: () => string;
	}

	const fieldDefs: TopologyFieldDef[] = [
		// Visual section
		{
			id: 'no_fade_edges',
			label: () => topology_dontFadeEdges(),
			type: 'boolean',
			path: 'local',
			key: 'no_fade_edges',
			helpText: () => topology_dontFadeEdgesHelp(),
			section: () => common_visual()
		},
		{
			id: 'hide_resize_handles',
			label: () => topology_hideResizeHandles(),
			type: 'boolean',
			path: 'local',
			key: 'hide_resize_handles',
			helpText: () => topology_hideResizeHandlesHelp(),
			section: () => common_visual()
		},
		// Docker section
		{
			id: 'group_docker_bridges_by_host',
			label: () => topology_groupDockerBridges(),
			type: 'boolean',
			path: 'request',
			key: 'group_docker_bridges_by_host',
			helpText: () => topology_groupDockerBridgesHelp(),
			section: () => common_docker()
		},
		{
			id: 'hide_vm_title_on_docker_container',
			label: () => topology_hideVmOnContainer(),
			type: 'boolean',
			path: 'request',
			key: 'hide_vm_title_on_docker_container',
			helpText: () => topology_hideVmOnContainerHelp(),
			section: () => common_docker()
		},
		// Left Zone section
		{
			id: 'left_zone_title',
			label: () => common_title(),
			type: 'string',
			path: 'local',
			key: 'left_zone_title',
			helpText: () => topology_leftZoneTitleHelp(),
			section: () => topology_leftZone(),
			placeholder: () => common_infrastructure()
		},
		{
			id: 'left_zone_service_categories',
			label: () => common_categories(),
			type: 'multiselect',
			path: 'request',
			key: 'left_zone_service_categories',
			helpText: () => topology_leftZoneCategoriesHelp(),
			section: () => topology_leftZone(),
			getOptions: () => serviceCategories
		},
		{
			id: 'show_gateway_in_left_zone',
			label: () => topology_showGatewayInLeftZone(),
			type: 'boolean',
			path: 'request',
			key: 'show_gateway_in_left_zone',
			helpText: () => topology_showGatewayInLeftZoneHelp(),
			section: () => topology_leftZone()
		},
		// Hide Stuff section
		{
			id: 'hide_ports',
			label: () => topology_hidePorts(),
			type: 'boolean',
			path: 'request',
			key: 'hide_ports',
			helpText: () => topology_hidePortsHelp(),
			section: () => topology_hideStuff()
		},
		{
			id: 'hide_service_categories',
			label: () => topology_hideServiceCategories(),
			type: 'multiselect',
			path: 'request',
			key: 'hide_service_categories',
			helpText: () => topology_hideServiceCategoriesHelp(),
			section: () => topology_hideStuff(),
			getOptions: () => serviceCategories
		},
		{
			id: 'hide_edge_types',
			label: () => topology_hideEdgeTypes(),
			type: 'multiselect',
			path: 'local',
			key: 'hide_edge_types',
			helpText: () => topology_hideEdgeTypesHelp(),
			section: () => topology_hideStuff(),
			getOptions: () => eTypes
		}
	];

	// Get unique section names in order
	let sectionNames = $derived([...new Set(fieldDefs.map((d) => d.section()))]);

	// Group fields by section
	let sections = $derived(
		sectionNames.map((name) => ({
			name,
			fields: fieldDefs.filter((d) => d.section() === name)
		}))
	);

	// Track expanded sections
	let expandedSections = $state<Record<string, boolean>>(
		Object.fromEntries(
			[
				common_visual(),
				common_docker(),
				topology_leftZone(),
				topology_hideStuff(),
				topology_tagFilter()
			].map((name) => [name, true])
		)
	);

	// Create form values initialized from topologyOptions
	let values = $state<Record<string, boolean | string | string[]>>({});

	// Initialize values from topologyOptions
	$effect(() => {
		const opts = $topologyOptions;
		const newValues: Record<string, boolean | string | string[]> = {};
		for (const def of fieldDefs) {
			const value =
				def.path === 'local'
					? opts.local[def.key as keyof typeof opts.local]
					: opts.request[def.key as keyof typeof opts.request];
			newValues[def.id] = value as boolean | string | string[];
		}
		values = newValues;
	});

	// Update a field value and sync to topologyOptions
	function updateValue(def: TopologyFieldDef, newValue: boolean | string | string[]) {
		values = { ...values, [def.id]: newValue };

		topologyOptions.update((opts) => {
			if (def.path === 'local') {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(opts.local as any)[def.key] = newValue;
			} else {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(opts.request as any)[def.key] = newValue;
			}
			return opts;
		});
	}

	function toggleSection(sectionName: string) {
		expandedSections[sectionName] = !expandedSections[sectionName];
	}

	function handleMultiSelectChange(def: TopologyFieldDef, event: Event) {
		const select = event.target as HTMLSelectElement;
		const selectedOptions = Array.from(select.selectedOptions).map((o) => o.value);
		updateValue(def, selectedOptions);
	}
</script>

<div class="space-y-4">
	<!-- Helper text -->
	<div class="rounded bg-gray-800/50 pt-2">
		<p class="text-tertiary text-[10px] leading-tight">
			{topology_multiselectHelp()}
		</p>
	</div>

	<!-- Tag Filter Section -->
	<div class="card card-static px-0 py-2">
		<button
			type="button"
			class="text-secondary hover:text-primary flex w-full items-center gap-2 px-3 py-2 text-sm font-medium"
			onclick={() => toggleSection(topology_tagFilter())}
		>
			{#if expandedSections[topology_tagFilter()]}
				<ChevronDown class="h-4 w-4" />
			{:else}
				<ChevronRight class="h-4 w-4" />
			{/if}
			{topology_tagFilter()}
		</button>

		{#if expandedSections[topology_tagFilter()]}
			<div class="space-y-4 px-3 pb-3">
				<p class="text-tertiary text-xs">{topology_tagFilterHelp()}</p>

				<TagFilterGroup
					label={common_hosts()}
					tags={hostTags}
					hiddenTagIds={$topologyOptions.local.tag_filter?.hidden_host_tag_ids ?? []}
					onToggle={toggleHostTag}
					entityType="host"
					hasUntagged={hasUntaggedHosts}
				/>

				<TagFilterGroup
					label={common_services()}
					tags={serviceTags}
					hiddenTagIds={$topologyOptions.local.tag_filter?.hidden_service_tag_ids ?? []}
					onToggle={toggleServiceTag}
					entityType="service"
					hasUntagged={hasUntaggedServices}
				/>

				<TagFilterGroup
					label={common_subnets()}
					tags={subnetTags}
					hiddenTagIds={$topologyOptions.local.tag_filter?.hidden_subnet_tag_ids ?? []}
					onToggle={toggleSubnetTag}
					entityType="subnet"
					hasUntagged={hasUntaggedSubnets}
				/>
			</div>
		{/if}
	</div>

	{#each sections as section (section.name)}
		<div class="card card-static px-0 py-2">
			<button
				type="button"
				class="text-secondary hover:text-primary flex w-full items-center gap-2 px-3 py-2 text-sm font-medium"
				onclick={() => toggleSection(section.name)}
			>
				{#if expandedSections[section.name]}
					<ChevronDown class="h-4 w-4" />
				{:else}
					<ChevronRight class="h-4 w-4" />
				{/if}
				{section.name}
			</button>

			{#if expandedSections[section.name]}
				<div class="space-y-3 px-3 pb-3">
					{#each section.fields as def (def.id)}
						{#if def.type === 'boolean'}
							<div>
								<label class="flex cursor-pointer items-center gap-2">
									<input
										type="checkbox"
										id={def.id}
										class="checkbox-card h-4 w-4"
										checked={!!values[def.id]}
										onchange={(e) => updateValue(def, e.currentTarget.checked)}
									/>
									<span class="text-secondary text-sm">{def.label()}</span>
								</label>
								{#if def.helpText}
									<p class="text-tertiary ml-6 mt-1 text-xs">{def.helpText()}</p>
								{/if}
							</div>
						{:else if def.type === 'string'}
							<div>
								<label for={def.id} class="text-secondary mb-1 block text-sm font-medium">
									{def.label()}
								</label>
								<input
									type="text"
									id={def.id}
									class="input-field w-full"
									placeholder={def.placeholder?.() ?? ''}
									value={values[def.id] ?? ''}
									oninput={(e) => updateValue(def, e.currentTarget.value)}
								/>
								{#if def.helpText}
									<p class="text-tertiary mt-1 text-xs">{def.helpText()}</p>
								{/if}
							</div>
						{:else if def.type === 'multiselect'}
							<div>
								<label for={def.id} class="text-secondary mb-1 block text-sm font-medium">
									{def.label()}
								</label>
								<select
									id={def.id}
									class="input-field w-full"
									multiple
									size={4}
									onchange={(e) => handleMultiSelectChange(def, e)}
								>
									{#each def.getOptions?.() ?? [] as option (option.value)}
										<option
											value={option.value}
											selected={(values[def.id] as string[])?.includes(option.value)}
										>
											{option.label}
										</option>
									{/each}
								</select>
								{#if def.helpText}
									<p class="text-tertiary mt-1 text-xs">{def.helpText()}</p>
								{/if}
							</div>
						{/if}
					{/each}
				</div>
			{/if}
		</div>
	{/each}
</div>
