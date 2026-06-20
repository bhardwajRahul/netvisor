// Re-export generated types from OpenAPI schema
import type { components } from '$lib/api/schema';
import type { RenderableTopology } from '$lib/features/topology/types/base';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

export type Share = components['schemas']['Share'];
export type ShareOptions = components['schemas']['ShareOptions'];
export type CreateUpdateShareRequest = components['schemas']['CreateUpdateShareRequest'];
export type PublicShareMetadata = components['schemas']['PublicShareMetadata'];

export interface ExportFeatures {
	png_export: boolean;
	svg_export: boolean;
	mermaid_export: boolean;
	confluence_export: boolean;
	pdf_export: boolean;
	html_export: boolean;
	remove_created_with: boolean;
}

// Frontend-specific type: combines share metadata with topology data.
// The backend's shared-topology endpoint returns the topology row plus
// the entity arrays the read-only viewer needs (shared users can't load
// them via the per-entity endpoints with their own credentials), so the
// frontend treats the payload as the enriched shape.
export interface ShareWithTopology {
	share: PublicShareMetadata;
	topology: RenderableTopology;
	export_features: ExportFeatures;
}

export const defaultShareOptions: ShareOptions = {
	show_inspect_panel: true,
	show_zoom_controls: true,
	show_export_button: true,
	show_minimap: true
};

export function createEmptyShare(topology_id: string, network_id: string): Share {
	return {
		topology_id,
		network_id,
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		created_by: uuidv4Sentinel,
		expires_at: null,
		allowed_domains: null,
		name: '',
		is_enabled: true,
		options: { ...defaultShareOptions },
		enabled_views: null
	};
}
