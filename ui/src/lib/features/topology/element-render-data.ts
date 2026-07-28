/**
 * Single owner of what an element card renders — and therefore of how tall it is.
 *
 * This computation used to live inside `ElementNode.svelte` as a `$derived.by`.
 * It never depended on component-local state: every input is either the node
 * itself or a module-level store, so it was always a pure function that merely
 * happened to live in a component.
 *
 * Moving it out matters because the render pipeline needs to reason about card
 * height *without* rendering the card. The measure pass currently mounts every
 * node just to read its height; to measure only one representative per distinct
 * card shape, something has to decide which cards share a shape. If that
 * decision were made by a second implementation, the two would drift and the
 * layout would silently corrupt. With one function, the shape key is derived
 * from the same result the component renders.
 *
 * Consequently: **anything that affects rendered height must be reflected in
 * `ElementRenderResult`.** Height-affecting markup driven by something outside
 * this function's inputs will not be visible to the shape key.
 */

import type {
	ElementRenderData,
	RenderableTopology,
	TopologyNode,
	TopologyOptions
} from './types/base';
import { resolveElementNode } from './resolvers';
import { getTopologyIndex } from './entity-index';
import { serviceDefinitions, views } from '$lib/shared/stores/metadata';

/**
 * Whether the active view inlines services / ports on this element, and whether
 * the user has hidden either. These gate whole blocks of card content, so they
 * belong with the render data rather than being recomputed by the template.
 */
export interface ElementInlineFlags {
	/** Entity types the active view inlines on this element (e.g. Service, Port). */
	inlineEntities: string[];
	inlinesService: boolean;
	inlinesPort: boolean;
	serviceInlineHidden: boolean;
	portInlineHidden: boolean;
}

export interface ElementRenderResult {
	data: ElementRenderData | null;
	flags: ElementInlineFlags;
}

export interface ElementRenderInputs {
	nodeId: string;
	node: TopologyNode;
	topology: RenderableTopology;
	activeView: string;
	options: TopologyOptions;
	/** Service ids hidden by tag filtering (`tagHiddenServiceIds`). */
	hiddenServiceIds: Set<string>;
}


type ViewElementConfig = {
	element_config?: {
		container_entity?: string;
		element_entities?: Array<{ entity_type: string; inline_entities: string[] }>;
	};
} | null;

function viewConfigFor(activeView: string): ViewElementConfig {
	return views.getMetadata(activeView) as ViewElementConfig;
}

/** Service categories the user hid via the metadata filter, for this view. */
function hiddenServiceCategories(options: TopologyOptions, activeView: string): string[] {
	const byView = (options.request.hide_metadata_values ?? {}) as Record<
		string,
		Record<string, Record<string, string[]>>
	>;
	return byView[activeView]?.['Service']?.['Category'] ?? [];
}

export function elementInlineFlags(
	inputs: Pick<ElementRenderInputs, 'activeView' | 'options'>,
	elementType: string | undefined
): ElementInlineFlags {
	const inlineEntities =
		viewConfigFor(inputs.activeView)?.element_config?.element_entities?.find(
			(e) => e.entity_type === elementType
		)?.inline_entities ?? [];

	// Entity types the user has hidden in this view via the filter panel's eye
	// toggle. (Element/container-level hiding is applied upstream via
	// tagHiddenNodeIds.)
	const hiddenEntities =
		((inputs.options.request.hide_entities ?? {}) as Record<string, string[]>)[inputs.activeView] ??
		[];

	return {
		inlineEntities,
		inlinesService: inlineEntities.includes('Service'),
		inlinesPort: inlineEntities.includes('Port'),
		serviceInlineHidden: hiddenEntities.includes('Service'),
		portInlineHidden: hiddenEntities.includes('Port')
	};
}

export function buildElementRender(inputs: ElementRenderInputs): ElementRenderResult {
	const { nodeId, node, topology, activeView, options, hiddenServiceIds } = inputs;

	const resolved = resolveElementNode(nodeId, node, topology);
	const flags = elementInlineFlags(inputs, resolved.elementType);

	const elementType = resolved.elementType ?? 'Interface';
	const host = resolved.host;
	const ipAddress = resolved.ipAddress ?? null;
	const servicesForHost = resolved.services ?? [];

	// Service elements: simpler rendering — single service with host name.
	// Intentionally does NOT read the hidden-category set here — category/tag
	// fading is handled by shouldFadeOut via the hidden-services store, so
	// category toggles don't trigger a recomputation.
	if (elementType === 'Service') {
		const service = resolved.services[0];
		// Hide hostname in views where Host is the container — it's redundant
		const showHostname = viewConfigFor(activeView)?.element_config?.container_entity !== 'Host';
		return {
			flags,
			data: {
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
				ip_address_id: nodeId
			} as ElementRenderData
		};
	}

	// Host elements: show host name with services
	if (elementType === 'Host') {
		if (!host || !resolved.hostId) return { data: null, flags };

		const hiddenCategories = hiddenServiceCategories(options, activeView);

		// Services visible in card. Filter = structural remove: hidden services
		// are dropped from the list entirely, not faded. The OpenPorts-category
		// subset is routed to the collapsed "+N open ports" indicator below.
		const servicesOnHost = servicesForHost.filter((s) => {
			if (hiddenServiceIds.has(s.id)) return false;
			const category = serviceDefinitions.getCategory(s.service_definition);
			if (category === 'OpenPorts' && hiddenCategories.includes(category)) return false;
			return true;
		});

		// OpenPorts hidden by category → collapsed indicator.
		// (Tag-hidden services of any category are already removed above.)
		const hiddenOpenPorts = servicesForHost.filter((s) => {
			if (hiddenServiceIds.has(s.id)) return false;
			const category = serviceDefinitions.getCategory(s.service_definition);
			return category === 'OpenPorts' && hiddenCategories.includes(category);
		});

		// Service names and port lines hide independently. Render the services
		// block if the view declares EITHER inlined and the user hasn't hidden
		// it — so toggling Services off still leaves port lines visible.
		const showServices =
			((flags.inlinesService && !flags.serviceInlineHidden) ||
				(flags.inlinesPort && !flags.portInlineHidden)) &&
			(servicesOnHost.length !== 0 || hiddenOpenPorts.length !== 0);

		const hostLabel = node.header ?? (host.name || host.hostname || null);

		return {
			flags,
			data: {
				elementType,
				footerText: null,
				services: servicesOnHost,
				hiddenOpenPorts,
				headerText: hostLabel,
				bodyText: showServices ? null : hostLabel,
				showServices,
				isVirtualized: host.virtualization !== null,
				isContainerized: false,
				ip_address_id: nodeId
			} as ElementRenderData
		};
	}

	// Port elements: show port name + status/MAC info
	if (elementType === 'Interface') {
		const ifEntryId =
			'interface_id' in (node as Record<string, unknown>)
				? ((node as Record<string, unknown>).interface_id as string)
				: undefined;
		const iface = ifEntryId ? getTopologyIndex(topology).interfacesById.get(ifEntryId) : undefined;

		let speed: string | null = null;
		if (iface?.speed_bps) {
			const bps = iface.speed_bps;
			if (bps >= 1_000_000_000) speed = `${(bps / 1_000_000_000).toFixed(0)}G`;
			else if (bps >= 1_000_000) speed = `${(bps / 1_000_000).toFixed(0)}M`;
			else speed = `${bps} bps`;
		}

		return {
			flags,
			data: {
				elementType,
				headerText: node.header ?? null,
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
			} as ElementRenderData
		};
	}

	// IPAddress elements
	if (!host || !resolved.hostId) return { data: null, flags };

	const hiddenCategories = hiddenServiceCategories(options, activeView);

	const isContainerSubnet = ipAddress
		? getTopologyIndex(topology).subnetsById.get(ipAddress.subnet_id)?.cidr === '0.0.0.0/0'
		: false;

	// All services bound to this interface (after tag filtering)
	const allServicesOnIPAddress = servicesForHost.filter((s) =>
		s.bindings.some((b) => b.ip_address_id == null || (ipAddress && b.ip_address_id == ipAddress.id))
	);

	// Filter = structural remove (see Host branch for context).
	const servicesOnIPAddress = allServicesOnIPAddress.filter((s) => {
		if (hiddenServiceIds.has(s.id)) return false;
		const category = serviceDefinitions.getCategory(s.service_definition);
		if (category === 'OpenPorts' && hiddenCategories.includes(category)) return false;
		return true;
	});

	const hiddenOpenPorts = allServicesOnIPAddress.filter((s) => {
		if (hiddenServiceIds.has(s.id)) return false;
		const category = serviceDefinitions.getCategory(s.service_definition);
		return category === 'OpenPorts' && hiddenCategories.includes(category);
	});

	const headerText: string | null = node.header ?? null;
	// Service names and port lines hide independently — see the Host branch.
	const showServices =
		((flags.inlinesService && !flags.serviceInlineHidden) ||
			(flags.inlinesPort && !flags.portInlineHidden)) &&
		(servicesOnIPAddress.length != 0 || hiddenOpenPorts.length != 0);

	const subtitleText =
		ipAddress && !isContainerSubnet
			? (ipAddress.name ? ipAddress.name + ': ' : '') + ipAddress.ip_address
			: null;

	return {
		flags,
		data: {
			elementType,
			footerText: null,
			subtitleText,
			services: servicesOnIPAddress,
			hiddenOpenPorts,
			headerText,
			bodyText: showServices ? null : host.name,
			showServices,
			isVirtualized:
				headerText?.startsWith('Docker @') || isContainerSubnet
					? false
					: host.virtualization !== null,
			isContainerized: false,
			ip_address_id: resolved.ipAddressId ?? ''
		} as ElementRenderData
	};
}
