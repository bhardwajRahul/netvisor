<script lang="ts">
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { Handle, Position, type NodeProps } from '@xyflow/svelte';
	import { concepts, entities, serviceDefinitions, views } from '$lib/shared/stores/metadata';
	import {
		selectedEdge as globalSelectedEdge,
		selectedNode as globalSelectedNode,
		selectedNodes as globalSelectedNodes,
		topologyOptions,
		activeView
	} from '../../queries';
	import { useTopology, selectedTopologyId } from '../../context';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { getFreshnessTag } from '$lib/shared/utils/freshness';

	const networksQuery = useNetworksQuery();
	import type { TopologyNode, ElementRenderData, RenderableTopology } from '../../types/base';
	import { resolveElementNode } from '../../resolvers';
	import { type Writable, get } from 'svelte/store';
	import { formatPort } from '$lib/shared/utils/formatting';
	import {
		connectedNodeIds,
		isExporting,
		newNodeIds,
		tagHiddenServiceIds,
		searchHiddenNodeIds,
		hoveredTag,
		hoveredMetadata,
		FILTER_VALUE_EXTRACTORS,
		expandedPortNodeIds,
		toggleExpandedPorts,
		UNTAGGED_SENTINEL
	} from '../../interactions';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { getContext } from 'svelte';
	import type { Port } from '$lib/features/hosts/types/base';
	import type { Node, Edge } from '@xyflow/svelte';
	import { topology_hideOpenPorts, topology_openPortsSummary } from '$lib/paraglide/messages';

	let { id, data, width }: NodeProps = $props();

	// Subscribe to isExporting for reactivity
	let isExportingValue = $state(get(isExporting));
	isExporting.subscribe((value) => {
		isExportingValue = value;
	});

	// Subscribe to tag filter stores for reactivity
	let hiddenServices = $state(get(tagHiddenServiceIds));
	tagHiddenServiceIds.subscribe((value) => {
		hiddenServices = value;
	});

	// Subscribe to search filter store for reactivity
	let searchHiddenNodes = $state(get(searchHiddenNodeIds));
	searchHiddenNodeIds.subscribe((value) => {
		searchHiddenNodes = value;
	});

	// Subscribe to connected node IDs for reactivity (manual subscription needed
	// because $derived.by may not read this store on initial evaluation, so auto-
	// subscription via $connectedNodeIds would miss the first update)
	let connectedNodes = $state(get(connectedNodeIds));
	connectedNodeIds.subscribe((value) => {
		connectedNodes = value;
	});

	// Subscribe to new node highlight store
	let highlightedNewNodes = $state(get(newNodeIds));
	newNodeIds.subscribe((value) => {
		highlightedNewNodes = value;
	});

	// Subscribe to multi-select store
	let multiSelectedNodes = $state(get(globalSelectedNodes));
	globalSelectedNodes.subscribe((value) => {
		multiSelectedNodes = value;
	});

	// Subscribe to tag hover state
	let currentHoveredTag = $state(get(hoveredTag));
	hoveredTag.subscribe((value) => {
		currentHoveredTag = value;
	});

	// Subscribe to service category hover state
	let currentHoveredMetadata = $state(get(hoveredMetadata));
	hoveredMetadata.subscribe((value) => {
		currentHoveredMetadata = value;
	});

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
	let host = $derived(resolved?.host);

	// Staleness marker. Deliberately additive rather than an opacity change:
	// node opacity is already the filter/search dimming channel (see
	// `nodeOpacity`), so fading stale nodes would make "stale" and "filtered
	// out" indistinguishable — and a node that is both, invisible as either.
	let networksData = $derived(networksQuery.data ?? []);
	const networkFor = (entity: { network_id?: string } | undefined | null) =>
		networksData.find((n) => n.id === entity?.network_id);

	// The entity this node actually depicts.
	let elementEntity = $derived.by(() => {
		switch (resolved?.elementType) {
			case 'Service':
				return resolved.services[0];
			case 'IPAddress':
				return resolved.ipAddress;
			case 'Interface':
				return resolved.snmpInterface;
			default:
				return host;
		}
	});

	// The verdict applies the digest's parent/child rule via `resolvedFreshness`,
	// so a child of a stale host inherits rather than claiming its own decay.
	// Judged on the node's own entity, with no inheritance from its host — the
	// same rule the inventory cards apply, so the two surfaces agree and the
	// tooltip's timestamp can never contradict the tag. Type names come from
	// the entity metadata fixture, so they stay localized and in step with the
	// backend.
	let subject = $derived(elementEntity ?? host);
	let staleTag = $derived(
		subject
			? getFreshnessTag(subject, networkFor(subject), {
					entityTypeLabel: entities.getName(resolved?.elementType ?? 'Host') || undefined
				})
			: null
	);
	let servicesForHost = $derived(resolved?.services ?? []);
	let ipAddress = $derived(resolved?.ipAddress ?? null);

	let effectiveWidth = $derived(width ? width : 0);

	// Per-card toggle for expanding hidden open ports (lifted to topology-level store for re-layout)
	let expandedOpenPorts = $derived($expandedPortNodeIds.has(id));

	// All services for this host (tag-hidden services stay in list to preserve card height)
	let visibleServicesForHost = $derived(servicesForHost);

	// Inline entity types declared for THIS element entity in the active view.
	// Drives what renders inside the card (services list, port rows) — replaces
	// the old per-element hardcoded gates.
	let inlineForThisElement = $derived(
		(
			views.getMetadata($activeView) as {
				element_config?: {
					element_entities?: Array<{ entity_type: string; inline_entities: string[] }>;
				};
			} | null
		)?.element_config?.element_entities?.find((e) => e.entity_type === resolved?.elementType)
			?.inline_entities ?? []
	);
	let inlinesService = $derived(inlineForThisElement.includes('Service'));
	let inlinesPort = $derived(inlineForThisElement.includes('Port'));

	// Entity types the user has hidden in this view via the filter panel's
	// eye toggle. Gates inline rendering of Service rows and Port lines.
	// (Element/container-level hiding is applied upstream via tagHiddenNodeIds.)
	let hiddenEntitiesThisView = $derived(
		(($topologyOptions.request.hide_entities ?? {}) as Record<string, string[]>)[$activeView] ?? []
	);
	let portInlineHidden = $derived(hiddenEntitiesThisView.includes('Port'));
	let serviceInlineHidden = $derived(hiddenEntitiesThisView.includes('Service'));

	// Reactively subscribe to the container subnet store
	let isContainerSubnetValue = $derived(
		ipAddress
			? topology?.subnets.find((s) => s.id == ipAddress.subnet_id)?.cidr == '0.0.0.0/0'
			: false
	);

	function getPortById(portId: string): Port | null {
		return topology?.ports.find((p) => p.id == portId) ?? null;
	}

	// Compute nodeRenderData reactively
	let nodeRenderData: ElementRenderData | null = $derived.by(() => {
		if (!resolved) return null;
		return (() => {
			const elementType = resolved.elementType ?? 'Interface';

			// Service elements: simpler rendering — single service with host name.
			// Intentionally does NOT read $topologyOptions here — category/tag
			// fading is handled by shouldFadeOut via hiddenServices store, so
			// category toggles don't trigger nodeRenderData recomputation.
			if (elementType === 'Service') {
				const service = resolved.services[0];
				// Hide hostname in views where Host is the container — it's redundant
				const viewConfig = (
					views.getMetadata($activeView) as {
						element_config?: { container_entity?: string };
					} | null
				)?.element_config;
				const showHostname = viewConfig?.container_entity !== 'Host';
				return {
					elementType,
					footerText: null,
					services: service ? [service] : [],
					hiddenOpenPorts: [],
					headerText: showHostname ? (host?.name ?? null) : null,
					bodyText: service ? null : 'Unknown Service',
					showServices: !!service,
					isVirtualized: false,
					isContainerized: service?.virtualization != null,
					isCategoryHidden: false,
					ip_address_id: id
				} as ElementRenderData;
			}

			// Host elements: show host name with services
			if (elementType === 'Host') {
				if (!host || !resolved.hostId) return null;

				// Hidden Service categories come from the generic metadata-filter
				// hide-set: request.hide_metadata_values[view].Service.Category.
				const hiddenCategories =
					(
						($topologyOptions.request.hide_metadata_values ?? {}) as Record<
							string,
							Record<string, Record<string, string[]>>
						>
					)[$activeView]?.['Service']?.['Category'] ?? [];

				type CategoryType = (typeof hiddenCategories)[number];
				// Services visible in card. Filter = structural remove: hidden
				// services are dropped from the list entirely, not faded. The
				// OpenPorts-category subset is routed to the collapsed
				// "+N open ports" indicator below (expand-to-see UX).
				const servicesOnHost = visibleServicesForHost.filter((s) => {
					if (hiddenServices.has(s.id)) return false;
					const category = serviceDefinitions.getCategory(s.service_definition);
					if (category === 'OpenPorts' && hiddenCategories.includes(category as CategoryType))
						return false;
					return true;
				});

				// OpenPorts hidden by category → collapsed indicator.
				// (Tag-hidden services of any category are already removed above.)
				const hiddenOpenPorts = visibleServicesForHost.filter((s) => {
					if (hiddenServices.has(s.id)) return false;
					const category = serviceDefinitions.getCategory(s.service_definition);
					return category === 'OpenPorts' && hiddenCategories.includes(category as CategoryType);
				});

				// Service names and port lines hide independently. Render the
				// services block if the view declares EITHER inlined and the
				// user hasn't hidden it — so toggling Services off still
				// leaves port lines visible (and vice versa).
				const showServiceNames = inlinesService && !serviceInlineHidden;
				const showPortLines = inlinesPort && !portInlineHidden;
				const showServices =
					(showServiceNames || showPortLines) &&
					(servicesOnHost.length !== 0 || hiddenOpenPorts.length !== 0);

				const hostLabel = (data as TopologyNode).header ?? (host.name || host.hostname || null);

				return {
					elementType,
					footerText: null,
					services: servicesOnHost,
					hiddenOpenPorts,
					headerText: hostLabel,
					bodyText: showServices ? null : hostLabel,
					showServices,
					isVirtualized: host.virtualization !== null,
					isContainerized: false,
					ip_address_id: id
				} as ElementRenderData;
			}

			// Port elements: show port name + status/MAC info
			if (elementType === 'Interface') {
				const ifEntryId =
					'interface_id' in (data as Record<string, unknown>)
						? ((data as Record<string, unknown>).interface_id as string)
						: undefined;
				const iface = ifEntryId ? topology?.interfaces.find((e) => e.id === ifEntryId) : undefined;

				let speed: string | null = null;
				if (iface?.speed_bps) {
					const bps = iface?.speed_bps;
					if (bps >= 1_000_000_000) speed = `${(bps / 1_000_000_000).toFixed(0)}G`;
					else if (bps >= 1_000_000) speed = `${(bps / 1_000_000).toFixed(0)}M`;
					else speed = `${bps} bps`;
				}

				return {
					elementType,
					headerText: (data as TopologyNode).header ?? null,
					footerText: null,
					bodyText: null,
					showServices: false,
					isVirtualized: false,
					isContainerized: false,
					services: [],
					hiddenOpenPorts: [],
					ip_address_id: '',
					portStatus: iface
						? {
								operStatus: iface.oper_status,
								speed,
								macAddress: iface.mac_address ?? null
							}
						: undefined
				} as ElementRenderData;
			}

			// Interface elements: existing behavior
			if (!host || !resolved.hostId) return null;

			const hiddenCategories =
				(
					($topologyOptions.request.hide_metadata_values ?? {}) as Record<
						string,
						Record<string, Record<string, string[]>>
					>
				)[$activeView]?.['Service']?.['Category'] ?? [];

			// All services bound to this interface (after tag filtering)
			const allServicesOnIPAddress = visibleServicesForHost
				? visibleServicesForHost.filter((s) =>
						s.bindings.some(
							(b) => b.ip_address_id == null || (ipAddress && b.ip_address_id == ipAddress.id)
						)
					)
				: [];

			// Filter = structural remove (see Host branch for context).
			type CategoryType = (typeof hiddenCategories)[number];
			const servicesOnIPAddress = allServicesOnIPAddress.filter((s) => {
				if (hiddenServices.has(s.id)) return false;
				const category = serviceDefinitions.getCategory(s.service_definition);
				if (category === 'OpenPorts' && hiddenCategories.includes(category as CategoryType))
					return false;
				return true;
			});

			const hiddenOpenPorts = allServicesOnIPAddress.filter((s) => {
				if (hiddenServices.has(s.id)) return false;
				const category = serviceDefinitions.getCategory(s.service_definition);
				return category === 'OpenPorts' && hiddenCategories.includes(category as CategoryType);
			});

			let bodyText: string | null = null;
			let footerText: string | null = null;
			let subtitleText: string | null = null;
			let headerText: string | null = (data as TopologyNode).header ?? null;
			// Service names and port lines hide independently — see the Host
			// branch above for the same pattern.
			const showServiceNames = inlinesService && !serviceInlineHidden;
			const showPortLines = inlinesPort && !portInlineHidden;
			let showServices =
				(showServiceNames || showPortLines) &&
				(servicesOnIPAddress.length != 0 || hiddenOpenPorts.length != 0);

			if (ipAddress && !isContainerSubnetValue) {
				subtitleText = (ipAddress.name ? ipAddress.name + ': ' : '') + ipAddress.ip_address;
			}

			if (!showServices) {
				bodyText = host.name;
			}

			return {
				elementType,
				footerText,
				subtitleText,
				services: servicesOnIPAddress,
				hiddenOpenPorts,
				headerText,
				bodyText,
				showServices,
				isVirtualized:
					headerText?.startsWith('Docker @') || isContainerSubnetValue
						? false
						: host.virtualization !== null,
				isContainerized: false,
				ip_address_id: resolved?.ipAddressId ?? ''
			} as ElementRenderData;
		})();
	});

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

	const hostColorHelper = entities.getColorHelper('Host');
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

	let handleStyle = $derived.by(() => {
		const baseSize = 8;
		const baseOpacity = selectedEdge?.source == id || selectedEdge?.target == id ? 1 : 0;

		const fillColor = hostColorHelper.rgb;

		return `
			width: ${baseSize}px;
			height: ${baseSize}px;
			border: 2px solid #374151;
			background-color: ${fillColor};
			opacity: ${baseOpacity};
			transition: opacity 0.2s ease-in-out;
		`;
	});
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

<Handle type="target" id="Top" position={Position.Top} style={handleStyle} />
<Handle type="target" id="Right" position={Position.Right} style={handleStyle} />
<Handle type="target" id="Bottom" position={Position.Bottom} style={handleStyle} />
<Handle type="target" id="Left" position={Position.Left} style={handleStyle} />

<Handle type="source" id="Top" position={Position.Top} style={handleStyle} />
<Handle type="source" id="Right" position={Position.Right} style={handleStyle} />
<Handle type="source" id="Bottom" position={Position.Bottom} style={handleStyle} />
<Handle type="source" id="Left" position={Position.Left} style={handleStyle} />
