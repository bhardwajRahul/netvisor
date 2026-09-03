import type { components } from '$lib/api/schema';

type PaginationMeta = components['schemas']['PaginationMeta'];

/**
 * What the pagination bar shows, and what the page buttons are allowed to do.
 *
 * Every value here has two derivations — one for a list the server paginated,
 * one for a list held whole in the browser — and getting a single pair out of
 * step is what produces a count that disagrees with the rows beneath it. They
 * are computed together, from one decision about which mode applies, so they
 * cannot disagree.
 */
export interface PaginationView {
	/** 1-based, resolved from the server's offset when the server paginates. */
	currentPage: number;
	totalPages: number;
	canGoPrev: boolean;
	canGoNext: boolean;
	/** 1-based index of the first row shown, clamped to the total. */
	showingStart: number;
	/** 1-based index of the last row shown, clamped to the total. */
	showingEnd: number;
	/** Rows matching the filters, across every page — not this page's length. */
	totalCount: number;
}

/**
 * Resolve the pagination state.
 *
 * `server` is null when the browser holds the whole list; pass the server's
 * metadata only when it is genuinely paginating, since every branch below keys
 * off that one distinction.
 *
 * `loadedCount` is how many rows the client currently holds: the whole filtered
 * list in client mode, one page of it in server mode.
 */
export function paginationView(
	server: PaginationMeta | null,
	clientPage: number,
	pageSize: number,
	loadedCount: number
): PaginationView {
	if (server) {
		const currentPage = Math.floor(server.offset / pageSize) + 1;

		return {
			currentPage,
			totalPages: Math.ceil(server.total_count / pageSize),
			canGoPrev: currentPage > 1,
			// The server says whether more rows exist; a page count derived from a
			// total it also supplied would only ever repeat that claim less exactly.
			canGoNext: server.has_more,
			showingStart: Math.min(server.offset + 1, server.total_count),
			showingEnd: Math.min(server.offset + loadedCount, server.total_count),
			totalCount: server.total_count
		};
	}

	const totalPages = Math.ceil(loadedCount / pageSize);

	return {
		currentPage: clientPage,
		totalPages,
		canGoPrev: clientPage > 1,
		canGoNext: clientPage < totalPages,
		showingStart: Math.min((clientPage - 1) * pageSize + 1, loadedCount),
		showingEnd: Math.min(clientPage * pageSize, loadedCount),
		totalCount: loadedCount
	};
}

/**
 * The rows to render for `page`.
 *
 * A server-paginated response is already the page, so it is returned untouched
 * — slicing it again would drop rows the server had chosen to send.
 */
export function pageSlice<T>(
	items: T[],
	page: number,
	pageSize: number,
	serverPaginated: boolean
): T[] {
	if (serverPaginated) return items;
	return items.slice((page - 1) * pageSize, page * pageSize);
}
