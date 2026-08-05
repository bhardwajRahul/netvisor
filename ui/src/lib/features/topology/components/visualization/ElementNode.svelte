<script lang="ts">
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { type NodeProps } from '@xyflow/svelte';
	import NodeHandles from './NodeHandles.svelte';
	import { concepts, entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import {
		selectedEdge as globalSelectedEdge,
		selectedNode as globalSelectedNode,
		topologyOptions,
		activeView
	} from '../../queries';
	import { useTopology, selectedTopologyId } from '../../context';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';

	const networksQuery = useNetworksQuery();
	import type { TopologyNode, ElementRenderData, RenderableTopology } from '../../types/base';
	import { resolveElementNode } from '../../resolvers';
	import { buildElementRender } from '../../element-render-data';
	import { getTopologyIndex } from '../../entity-index';
	import type { Writable } from 'svelte/store';
	import { formatPort } from '$lib/shared/utils/formatting';
	import {
		FILTER_VALUE_EXTRACTORS,
		expandedPortNodeIds,
		toggleExpandedPorts,
		UNTAGGED_SENTINEL
	} from '../../interactions';
	import * as sharedStores from '../../reactive-stores.svelte';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { getContext } from 'svelte';
	import type { Port } from '$lib/features/hosts/types/base';
	import type { Node, Edge } from '@xyflow/svelte';
	import { topology_hideOpenPorts, topology_openPortsSummary } from '$lib/paraglide/messages';
	import { ELEMENT_HANDLE_SIZE_PX } from '../../pipeline/build-flow-nodes';

	let { id, data, width }: NodeProps = $props();

	// Shared, refcounted views over the module-level stores — see
	// `reactive-stores.svelte.ts`. One subscription serves every node component,
	// and it tears down with the component instead of leaking on every unmount.
	// `.current` falls back to `get(store)` outside a tracking context, so the
	// first value can't be missed — which is what the previous hand-rolled
	// subscriptions existed to work around.
	let isExportingValue = $derived(sharedStores.exporting.current);
	let hiddenEntities = $derived(sharedStores.hiddenEntities.current);
	let searchHiddenNodes = $derived(sharedStores.searchHiddenNodes.current);
	let connectedNodes = $derived(sharedStores.connectedNodes.current);
	let highlightedNewNodes = $derived(sharedStores.highlightedNewNodes.current);
	let multiSelectedNodes = $derived(sharedStores.multiSelectedNodes.current);
	let currentHoveredTag = $derived(sharedStores.currentHoveredTag.current);
	let currentHoveredMetadata = $derived(sharedStores.currentHoveredMetadata.current);

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					| RenderableTopology
					| undefined)
	);

	// Try to get selection from context (for share/embed pages), fallback to global store
	const selectedNodeContext = getContext<Writable<Node | null> | undefined>('selectedNode');
	const selectedEdgeContext = getContext<Writable<Edge | null> | undefined>('selectedEdge');
	let selectedNode = $derived(
		selectedNodeContext ? $selectedNodeContext : $globalSelectedNode
	) as Node | null;
	let selectedEdge = $derived(
		selectedEdgeContext ? $selectedEdgeContext : $globalSelectedEdge
	) as Edge | null;

	let resolved = $derived(topology ? resolveElementNode(id, data as TopologyNode, topology) : null);

	// Networks are still needed locally for the metadata-filter extractors below.
	let networksData = $derived(networksQuery.data ?? []);
	const networkFor = (entity: { network_id?: string } | undefined | null) =>
		networksData.find((n) => n.id === entity?.network_id);

	let effectiveWidth = $derived(width ? width : 0);

	// Per-card toggle for expanding hidden open ports (lifted to topology-level store for re-layout)
	let expandedOpenPorts = $derived($expandedPortNodeIds.has(id));

	// All card content — and therefore card height — is computed by one function
	// in `element-render-data.ts`, shared with the render pipeline so it can reason
	// about card shape without mounting the card. Do not recompute any of this
	// locally; add to that function instead.
	let elementRender = $derived(
		topology
			? buildElementRender({
					nodeId: id,
					node: data as TopologyNode,
					topology,
					activeView: $activeView,
					options: $topologyOptions,
					hiddenEntityIds: hiddenEntities,
					networks: networksData
				})
			: null
	);
	let nodeRenderData: ElementRenderData | null = $derived(elementRender?.data ?? null);
	let inlinesService = $derived(elementRender?.flags.inlinesService ?? false);
	let inlinesPort = $derived(elementRender?.flags.inlinesPort ?? false);
	let serviceInlineHidden = $derived(elementRender?.flags.serviceInlineHidden ?? false);
	let portInlineHidden = $derived(elementRender?.flags.portInlineHidden ?? false);
	let inlineForThisElement = $derived(elementRender?.flags.inlineEntities ?? []);
	// Staleness pill — computed with the rest of the card content because it
	// renders in flow and therefore affects card height.
	let staleTag = $derived(elementRender?.staleTag ?? null);

	// Called once per service binding while rendering, so this must not scan
	// `topology.ports` — on a large graph that is nodes x bindings x ports.
	let portsById = $derived(topology ? getTopologyIndex(topology).portsById : null);
	function getPortById(portId: string): Port | null {
		return portsById?.get(portId) ?? null;
	}

	// Group services into bare vs containerized for dotted-border rendering.
	// Uses inline_groups from the topology node (populated by element rules)
	// instead of re-deriving from virtualization entity fields.
	type ServiceList = ElementRenderData['services'];
	type ServiceGroup = {
		runtimeService: ServiceList[number] | null;
		containers: ServiceList;
		runtimeId: string;
	};
	let serviceGroups = $derived.by(
		(): {
			bare: ServiceList;
			containerized: ServiceGroup[];
		} => {
			const services = nodeRenderData?.services ?? [];
			if (nodeRenderData?.elementType !== 'Host' || services.length === 0) {
				return { bare: services, containerized: [] };
			}

			// Read inline_groups from the topology node data.
			// Each entry has entity_id (the service), group_id (shared by group members), and role.
			const inlineGroups = ((data as Record<string, unknown>).inline_groups ?? []) as Array<{
				entity_id: string;
				group_id: string;
				role: string;
			}>;

			if (inlineGroups.length === 0) {
				return { bare: services, containerized: [] };
			}

			// Build groups from inline_groups — generic matching by entity_id, no domain logic
			const groupMembers = new SvelteMap<string, ServiceList>();
			const groupHeaders = new SvelteMap<string, ServiceList[number] | null>();
			const memberServiceIds = new SvelteSet<string>();

			for (const ig of inlineGroups) {
				if (!groupMembers.has(ig.group_id)) {
					groupMembers.set(ig.group_id, []);
					groupHeaders.set(ig.group_id, null);
				}
				const svc = services.find((s) => s.id === ig.entity_id);
				if (!svc) continue;
				memberServiceIds.add(svc.id);
				if (ig.role === 'Header') {
					groupHeaders.set(ig.group_id, svc);
				} else {
					groupMembers.get(ig.group_id)!.push(svc);
				}
			}

			const bareServices = services.filter((s) => !memberServiceIds.has(s.id));
			const groups: ServiceGroup[] = [];
			for (const [groupId, containers] of groupMembers) {
				if (containers.length > 0 || groupHeaders.get(groupId)) {
					groups.push({
						runtimeService: groupHeaders.get(groupId) ?? null,
						containers,
						runtimeId: groupId
					});
				}
			}

			return { bare: bareServices, containerized: groups };
		}
	);

	let isNewNode = $derived(nodeRenderData ? highlightedNewNodes.has(id) : false);

	let isNodeSelected = $derived(
		selectedNode?.id === nodeRenderData?.ip_address_id ||
			multiSelectedNodes.some((n) => n.id === nodeRenderData?.ip_address_id)
	);

	// Fade signals "focus elsewhere" (search, selection) — not filter state.
	// Filter hides (tag / metadata / entity-hide) remove nodes structurally
	// upstream of this component, so a rendered card is never filter-hidden.
	let shouldFadeOut = $derived.by(() => {
		if (isExportingValue) return false;

		// Search highlight: fade non-matching nodes.
		if (searchHiddenNodes.has(id)) {
			return true;
		}

		// Selection focus: fade unconnected nodes.
		if (!selectedNode && !selectedEdge && multiSelectedNodes.length < 2) return false;
		if (!nodeRenderData) return false;
		return !connectedNodes.has(id);
	});

	let nodeOpacity = $derived(shouldFadeOut ? 0.3 : 1);

	// Only the handle styling used this, and `NodeHandles` renders the geometry statically now.
	// const hostColorHelper = entities.getColorHelper('Host');
	const virtualizationColorHelper = concepts.getColorHelper('Virtualization');
	const containerizationColorHelper = concepts.getColorHelper('Containerization');
	const discoveryColorHelper = entities.getColorHelper('Discovery');

	// How does the hovered entity type relate to this card?
	//   'element' — the card IS the hovered entity type (Service element in
	//     Workloads/Application; IPAddress element in L3; Host VM in
	//     Workloads; Interface in L2).
	//   'inline'  — the hovered entity type is declared inline on this card
	//     (Service or Port on an L3 IPAddress; Service on a Workloads Host
	//     VM element).
	//   null      — no relationship; this card is unaffected by the hover.
	// Element cards get a card-border treatment; inline cards get a
	// card-glow / text-highlight treatment. This replaces the former
	// Host-specific branch and makes hover behaviour view-agnostic.
	let hoveredRelationship = $derived.by((): 'element' | 'inline' | null => {
		if (!currentHoveredTag) return null;
		const elType = nodeRenderData?.elementType;
		if (elType && currentHoveredTag.entityType === elType) return 'element';
		if (inlineForThisElement.includes(currentHoveredTag.entityType)) return 'inline';
		return null;
	});

	// Tags carried by this card's source entity (for tag-scoped ring).
	// IPAddress and Interface don't carry tags today — they fall back to
	// the host's tags so per-host tag hover still highlights IP / interface
	// element cards, matching the old Host-specific behaviour.
	function cardEntityTags(): string[] {
		if (!resolved) return [];
		switch (resolved.elementType) {
			case 'Host':
				return resolved.host?.tags ?? [];
			case 'Service':
				return resolved.services[0]?.tags ?? [];
			case 'IPAddress':
			case 'Interface':
				return resolved.host?.tags ?? [];
		}
		return [];
	}

	// Metadata hover context — mirrors `hoveredRelationship` + tag ring/pulse
	// but driven by `hoveredMetadata`. Element-mode when this card's own
	// entity matches the extractor, inline-mode when an inline row matches.
	// Host metadata also bubbles up to IPAddress/Interface cards (same
	// fallback as cardEntityTags).
	let metadataHoverContext = $derived.by(
		(): {
			mode: 'element' | 'inline';
			color: string;
		} | null => {
			if (!currentHoveredMetadata || !resolved) return null;
			const { entityType, filterType, valueId, color } = currentHoveredMetadata;
			const extractor = FILTER_VALUE_EXTRACTORS[entityType]?.[filterType];
			if (!extractor) return null;

			const elType = nodeRenderData?.elementType;
			let cardEntity: unknown | null = null;
			if (elType === entityType) {
				if (entityType === 'Host') cardEntity = resolved.host ?? null;
				else if (entityType === 'Service') cardEntity = resolved.services[0] ?? null;
			} else if (entityType === 'Host' && (elType === 'IPAddress' || elType === 'Interface')) {
				cardEntity = resolved.host ?? null;
			}
			if (
				cardEntity &&
				extractor(cardEntity, { network: networkFor(cardEntity as { network_id?: string }) }) ===
					valueId
			) {
				return { mode: 'element', color };
			}

			if (entityType === 'Service' && nodeRenderData?.services?.length) {
				for (const service of nodeRenderData.services) {
					if (extractor(service, { network: networkFor(service) }) === valueId)
						return { mode: 'inline', color };
				}
			}
			return null;
		}
	);

	// Card border for element-role hover. Subdued gray when entity-wide
	// (tagId null); tag-coloured when tag-scoped AND the card's entity
	// actually carries the hovered tag. Metadata-scoped hover uses the
	// same border treatment when this card's own entity matches.
	let tagHoverRingStyle = $derived.by(() => {
		if (metadataHoverContext?.mode === 'element') {
			const ch = createColorHelper(
				metadataHoverContext.color as Parameters<typeof createColorHelper>[0]
			);
			return `box-shadow: 0 0 0 3px ${ch.rgb};`;
		}
		if (hoveredRelationship !== 'element' || !currentHoveredTag) return '';
		const { tagId, color } = currentHoveredTag;
		if (tagId === null) {
			return 'box-shadow: 0 0 0 2px rgb(156, 163, 175);';
		}
		const tags = cardEntityTags();
		const hasTag = tagId === UNTAGGED_SENTINEL ? tags.length === 0 : tags.includes(tagId);
		if (!hasTag || !color) return '';
		const colorHelper = createColorHelper(color as Parameters<typeof createColorHelper>[0]);
		return `box-shadow: 0 0 0 3px ${colorHelper.rgb};`;
	});

	// Generic pulse style for an inline row whose entity-type matches the
	// active hover. Applied directly to the row's text span — works for any
	// inline entity (service name row, port line row, future ones) as long
	// as the caller passes the row's entity type and its tag list. Pass [] for
	// entities that don't carry tags (ports today).
	function inlineRowPulse(rowEntityType: string, rowTags: string[]): string {
		if (hoveredRelationship !== 'inline' || !currentHoveredTag) return '';
		if (currentHoveredTag.entityType !== rowEntityType) return '';
		const { tagId, color } = currentHoveredTag;
		// Entity-wide (tagId null): neutral gray pulse on every matching row.
		if (tagId === null) {
			return 'color: rgb(156, 163, 175); --text-pulse-color: rgb(156, 163, 175);';
		}
		// Tag-scoped: only rows whose entity carries the hovered tag.
		if (!color) return '';
		const hasTag = tagId === UNTAGGED_SENTINEL ? rowTags.length === 0 : rowTags.includes(tagId);
		if (!hasTag) return '';
		const ch = createColorHelper(color as Parameters<typeof createColorHelper>[0]);
		return `color: ${ch.rgb}; --text-pulse-color: ${ch.rgb};`;
	}

	// Card glow for tag-scoped inline hover (and legacy category hover).
	// Entity-wide inline hover uses per-row text pulses instead — every
	// card that inlines the hovered entity would glow otherwise, which
	// isn't useful discrimination.
	let serviceHoverShadowStyle = $derived.by(() => {
		if (!nodeRenderData?.showServices) return '';
		const services = nodeRenderData.services;
		if (
			hoveredRelationship === 'inline' &&
			currentHoveredTag &&
			currentHoveredTag.tagId !== null &&
			currentHoveredTag.color
		) {
			const { tagId, color, entityType } = currentHoveredTag;
			// Only fire for entity types that carry tags today (Service). New
			// taggable inline entities would extend via the same per-row tag
			// lookup. For now, no generic registry to resolve "tags of an
			// arbitrary inline entity instance on this card" exists.
			if (entityType === 'Service') {
				for (const service of services) {
					const isUntagged = service.tags.length === 0;
					const hasTag = tagId === UNTAGGED_SENTINEL ? isUntagged : service.tags.includes(tagId);
					if (hasTag) {
						const ch = createColorHelper(color as Parameters<typeof createColorHelper>[0]);
						return `--pulse-color: ${ch.rgb};`;
					}
				}
			}
		}
		if (metadataHoverContext?.mode === 'inline') {
			const ch = createColorHelper(
				metadataHoverContext.color as Parameters<typeof createColorHelper>[0]
			);
			return `--pulse-color: ${ch.rgb};`;
		}
		return '';
	});

	// Entity-wide hover matching THIS element's own type — drives the
	// dotted-underline on the header text. Inline-role hover has its own
	// visual treatment (card glow) and doesn't underline the header.
	let isEntityTypeHover = $derived(
		hoveredRelationship === 'element' && currentHoveredTag?.tagId === null
	);

	let cardClass = $derived(`card ${isNodeSelected ? 'card-selected' : ''}`);

	// Handle styling lived here; `NodeHandles` now renders the geometry statically.
</script>

{#if nodeRenderData}
	<div
		class={`${cardClass} ${isNewNode ? 'animate-pulse-highlight' : ''} ${serviceHoverShadowStyle ? 'animate-pulse-highlight-once' : ''} ${isEntityTypeHover ? 'entity-type-hover-active' : ''}`}
		style={`width: ${effectiveWidth}px; height: 100%; display: flex; flex-direction: column; padding: 0; opacity: ${nodeOpacity}; transition: opacity 0.2s ease-in-out, box-shadow 0.15s ease-in-out; ${isNewNode ? `--pulse-color: ${discoveryColorHelper.rgb};` : ''} ${serviceHoverShadowStyle} ${tagHoverRingStyle}`}
	>
		<!-- Staleness tag: the same Tag component, label, colour and icon the
		     inventory card badge uses (both come from `getFreshnessTag`), so one
		     entity reads identically in the list, the map and the digest.
		     Additive rather than an opacity change — opacity is already the
		     filter/search dimming channel. -->

		<!-- Topmost element of the card, centred: one placement that reads the
		     same on every element type, rather than depending on which title row
		     a given card happens to have. In flow so it stays inside the card
		     bounds and never overlaps the title; node heights are DOM-measured
		     (pipeline/measure.ts), so layout absorbs the row. -->
		{#if staleTag}
			<div class="flex flex-shrink-0 justify-center px-2 pt-2">
				<div class="scale-90">
					<Tag {...staleTag} pill />
				</div>
			</div>
		{/if}

		<!-- Rest of component stays the same -->
		<!-- Header section with gradient transition to body -->
		{#if nodeRenderData.headerText}
			<div class="relative flex-shrink-0 px-2 pt-2 text-center">
				<div
					data-entity-header
					class={`truncate text-xs font-medium leading-none ${nodeRenderData.isVirtualized ? virtualizationColorHelper.text : nodeRenderData.isContainerized ? containerizationColorHelper.text : 'text-tertiary'}`}
				>
					{nodeRenderData.headerText}
				</div>
			</div>
		{/if}

		{#if nodeRenderData.subtitleText}
			<div
				data-entity-header
				class="text-primary truncate px-2 pt-2 text-center text-sm font-medium {!nodeRenderData.headerText &&
				!nodeRenderData.showServices
					? 'pb-2'
					: ''}"
			>
				{nodeRenderData.subtitleText}
			</div>
		{/if}

		<!-- Body section -->
		<div class="flex flex-1 flex-col items-center justify-center px-3 py-2">
			{#if nodeRenderData.showServices}
				{#snippet serviceCard(service: (typeof nodeRenderData.services)[number])}
					{@const ServiceIcon = serviceDefinitions.getIconComponent(service.service_definition)}
					{@const serviceColorHelper = serviceDefinitions.getColorHelper(
						service.service_definition
					)}
					{@const serviceTagHighlight = inlineRowPulse('Service', service.tags)}
					{@const serviceMetadataHighlight = (() => {
						if (metadataHoverContext?.mode !== 'inline') return '';
						if (!currentHoveredMetadata || currentHoveredMetadata.entityType !== 'Service')
							return '';
						const extractor =
							FILTER_VALUE_EXTRACTORS['Service']?.[currentHoveredMetadata.filterType];
						if (!extractor) return '';
						if (
							extractor(service, { network: networkFor(service) }) !==
							currentHoveredMetadata.valueId
						)
							return '';
						const ch = createColorHelper(
							currentHoveredMetadata.color as Parameters<typeof createColorHelper>[0]
						);
						return `color: ${ch.rgb}; --text-pulse-color: ${ch.rgb};`;
					})()}
					<div
						class="flex flex-col items-center justify-center py-2"
						style="min-width: 0; max-width: 100%; width: 100%;"
					>
						<!-- Render the service name when either: (a) this card IS
						  a Service element (the row is the card's own identity,
						  not inlined content — always show), or (b) the card
						  inlines services and the user hasn't toggled them off. -->
						{#if nodeRenderData.elementType === 'Service' || (inlinesService && !serviceInlineHidden)}
							<div
								class="flex items-center justify-center gap-1"
								style="line-height: 1.3; width: 100%; min-width: 0; max-width: 100%;"
								title={service.name}
							>
								<ServiceIcon class="h-5 w-5 flex-shrink-0 {serviceColorHelper.icon}" />
								<span
									class="text-m text-secondary truncate {serviceTagHighlight ||
									serviceMetadataHighlight
										? 'animate-text-pulse-highlight'
										: ''}"
									style="transition: color 0.15s; {serviceTagHighlight || serviceMetadataHighlight}"
								>
									{service.name}
								</span>
							</div>
						{/if}
						{#if inlinesPort && !portInlineHidden && service.bindings.filter((b) => b.type == 'Port').length > 0}
							{@const portPulse = inlineRowPulse('Port', [])}
							<span
								class="text-tertiary mt-1 text-center text-xs {portPulse
									? 'animate-text-pulse-highlight'
									: ''}"
								style="transition: color 0.15s; {portPulse}"
								>{service.bindings
									.map((b) => {
										if (
											(b.ip_address_id == nodeRenderData.ip_address_id ||
												b.ip_address_id == null) &&
											b.type == 'Port' &&
											b.port_id
										) {
											const port = getPortById(b.port_id);
											if (port) {
												return formatPort(port);
											}
										}
									})
									.filter((p) => {
										return p !== undefined;
									})
									.join(', ')}</span
							>
						{/if}
					</div>
				{/snippet}
				<!-- Show services list -->
				<div class="flex w-full flex-col items-center" style="min-width: 0; max-width: 100%;">
					{#if serviceGroups.containerized.length > 0}
						<!-- Grouped rendering: bare services + containerized groups with dotted border -->
						{#each serviceGroups.bare as service (service.id)}
							{@render serviceCard(service)}
						{/each}
						{#each serviceGroups.containerized as group (group.runtimeId)}
							{@const RuntimeIcon = group.runtimeService
								? serviceDefinitions.getIconComponent(group.runtimeService.service_definition)
								: null}
							<div
								class="mb-1 mt-1 w-full rounded-md border border-dashed border-gray-300 px-1 py-0.5 dark:border-gray-600"
							>
								<div class="flex items-center gap-1 px-1 pb-2 pt-1">
									{#if RuntimeIcon}
										<RuntimeIcon class="h-5 w-5 flex-shrink-0" />
									{/if}
									<span class="text-secondary truncate text-xs font-medium">
										{group.runtimeService?.name ?? 'Containers'}
									</span>
								</div>
								{#each group.containers as service (service.id)}
									{@render serviceCard(service)}
								{/each}
							</div>
						{/each}
					{:else}
						{#each nodeRenderData.services as service (service.id)}
							{@render serviceCard(service)}
						{/each}
					{/if}
					{#if nodeRenderData.hiddenOpenPorts.length > 0 && nodeRenderData.elementType !== 'Host'}
						{#if expandedOpenPorts}
							{#each nodeRenderData.hiddenOpenPorts as service (service.id)}
								{@const ServiceIcon = serviceDefinitions.getIconComponent(
									service.service_definition
								)}
								{@const svcColor = serviceDefinitions.getColorHelper(service.service_definition)}
								<div
									class="flex flex-col items-center justify-center"
									style="min-width: 0; max-width: 100%; width: 100%;"
								>
									{#if inlinesService && !serviceInlineHidden}
										<div
											class="flex items-center justify-center gap-1"
											style="line-height: 1.3; width: 100%; min-width: 0; max-width: 100%;"
											title={service.name}
										>
											<ServiceIcon class="h-5 w-5 flex-shrink-0 {svcColor.icon}" />
											<span class="text-m text-secondary truncate" style="transition: color 0.15s;">
												{service.name}
											</span>
										</div>
									{/if}
									{#if inlinesPort && !portInlineHidden && service.bindings.filter((b) => b.type == 'Port').length > 0}
										{@const portPulseExp = inlineRowPulse('Port', [])}
										<span
											class="text-tertiary mt-1 text-center text-xs {portPulseExp
												? 'animate-text-pulse-highlight'
												: ''}"
											style="transition: color 0.15s; {portPulseExp}"
											>{service.bindings
												.map((b) => {
													if (
														(b.ip_address_id == nodeRenderData.ip_address_id ||
															b.ip_address_id == null) &&
														b.type == 'Port' &&
														b.port_id
													) {
														const port = getPortById(b.port_id);
														if (port) {
															return formatPort(port);
														}
													}
												})
												.filter((p) => p !== undefined)
												.join(', ')}</span
										>
									{/if}
								</div>
							{/each}
							<button
								class="nopan text-tertiary hover:text-secondary mb-2 mt-1 cursor-pointer text-xs underline"
								onclick={(e) => {
									e.stopPropagation();
									toggleExpandedPorts(id);
								}}
							>
								{topology_hideOpenPorts()}
							</button>
						{:else}
							<button
								class="nopan bg-surface-secondary text-tertiary hover:text-secondary mb-2 mt-1 cursor-pointer rounded-full px-2 py-0.5 text-xs underline"
								onclick={(e) => {
									e.stopPropagation();
									toggleExpandedPorts(id);
								}}
							>
								{topology_openPortsSummary({
									count:
										nodeRenderData.hiddenOpenPorts.reduce(
											(sum, s) =>
												sum +
												s.bindings.filter(
													(b) =>
														(b.ip_address_id == nodeRenderData.ip_address_id ||
															b.ip_address_id == null) &&
														b.type == 'Port'
												).length,
											0
										) || nodeRenderData.hiddenOpenPorts.length
								})}
							</button>
						{/if}
					{/if}
				</div>
			{:else}
				<!-- Show host name as body text -->
				<div
					class="text-secondary truncate text-center text-xs leading-none"
					title={nodeRenderData.bodyText}
				>
					{nodeRenderData.bodyText}
				</div>
			{/if}
			{#if nodeRenderData.portStatus}
				<div class="flex flex-col items-center gap-0.5">
					<div class="flex items-center gap-1.5">
						<span
							class="text-xs font-medium"
							style="color: {nodeRenderData.portStatus.operStatus === 'Up'
								? '#22c55e'
								: nodeRenderData.portStatus.operStatus === 'Down'
									? '#ef4444'
									: '#9ca3af'}">●</span
						>
						{#if nodeRenderData.portStatus.speed}
							<span class="text-tertiary text-xs">{nodeRenderData.portStatus.speed}</span>
						{/if}
					</div>
					{#if nodeRenderData.portStatus.macAddress}
						<span class="text-tertiary truncate font-mono" style="font-size: 0.55rem; opacity: 0.7"
							>{nodeRenderData.portStatus.macAddress}</span
						>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Footer section -->
		{#if nodeRenderData.footerText}
			<div class="relative flex flex-shrink-0 items-center justify-center px-2 pb-2">
				<div class="text-tertiary truncate text-xs font-medium leading-none">
					{nodeRenderData.footerText}
				</div>
			</div>
		{/if}
	</div>
{/if}

<NodeHandles size={ELEMENT_HANDLE_SIZE_PX} />
