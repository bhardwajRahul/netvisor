/**
 * CSV Export utility for downloading filtered entity data
 */

import { getServerUrl } from '$lib/api/client';
import { pushError } from '$lib/shared/stores/feedback';
import { entityToExportPath, type EntityDiscriminants } from '$lib/api/entities';

/**
 * Parameters for CSV export - matches backend query parameters
 */
export interface CsvExportParams {
	tag_ids?: string[];
	order_by?: string;
	order_direction?: 'asc' | 'desc';
	// Entity-specific filters can be added via index signature
	[key: string]: string | string[] | undefined;
}

/**
 * Download CSV export for an entity with the given filter parameters.
 * Uses fetch with credentials to handle authentication.
 */
export async function downloadCsv(
	entityType: EntityDiscriminants,
	params: CsvExportParams
): Promise<void> {
	const apiPath = entityToExportPath[entityType];
	if (!apiPath) {
		pushError(`CSV export not supported for ${entityType}`);
		return;
	}

	const baseUrl = getServerUrl();
	const url = new URL(`/api/v1/${apiPath}/export/csv`, baseUrl);

	// Add all params to URL
	for (const [key, value] of Object.entries(params)) {
		if (value === undefined) continue;
		if (Array.isArray(value)) {
			value.forEach((v) => url.searchParams.append(key, v));
		} else {
			url.searchParams.set(key, value);
		}
	}

	const response = await fetch(url.toString(), {
		method: 'GET',
		credentials: 'include'
	});

	if (!response.ok) {
		pushError(`Export failed: ${response.statusText}`);
		throw new Error(`Export failed: ${response.statusText}`);
	}

	// Trigger download
	const blob = await response.blob();
	const blobUrl = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.href = blobUrl;
	link.download = `${apiPath.replace('/', '-')}.csv`;
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
	URL.revokeObjectURL(blobUrl);
}
