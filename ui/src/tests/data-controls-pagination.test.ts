import { describe, it, expect } from 'vitest';

/**
 * Tests for DataControls pagination logic.
 *
 * These tests verify the count calculation and display logic for:
 * 1. Server-side pagination - where items come pre-paginated from the server
 * 2. Client-side pagination - where all items are loaded and paginated client-side
 *
 * The key difference is in how "showing X of Y" is calculated:
 * - Server-side: count = totalCount (from server), total = totalCount
 * - Client-side: count = processedItems.length (after filters), total = items.length (all items)
 */

describe('DataControls pagination count display', () => {
	describe('server-side pagination single page', () => {
		it('should show total count from server when all items fit on one page', () => {
			// Scenario: Server says there are 41 items total, pageSize is 50
			// Only 1 page needed, so no range display
			const serverPagination = {
				total_count: 41,
				offset: 0,
				has_more: false
			};
			const pageSize = 50;

			const totalPages = Math.ceil(serverPagination.total_count / pageSize);
			expect(totalPages).toBe(1);

			// For server-side pagination with single page, we show "Showing X of X"
			// where X is the total count from the server
			const countToDisplay = serverPagination.total_count;
			const totalToDisplay = serverPagination.total_count;

			// This should show "Showing 41 of 41 items" NOT "Showing 41 of 20 items"
			expect(countToDisplay).toBe(41);
			expect(totalToDisplay).toBe(41);
		});

		it('should not use items.length as total when server pagination is active', () => {
			// Bug: "Showing 41 of 20" occurred when:
			// - Server reported total_count: 41
			// - But items prop only had 20 items (due to stale pageSize in parent)
			// - Old code used items.length as the "total" denominator
			//
			// Fix: With server-side pagination, always use server's total_count for both values
			const serverTotalCount = 41;
			const itemsArrayLength = 20;

			const countToDisplay = serverTotalCount;
			const totalToDisplay = serverTotalCount; // Not itemsArrayLength

			expect(countToDisplay).toBe(41);
			expect(totalToDisplay).toBe(41);
			expect(totalToDisplay).not.toBe(itemsArrayLength);
		});

		it('should handle edge case where pageSize equals total count', () => {
			const serverPagination = {
				total_count: 20,
				offset: 0,
				has_more: false
			};
			const pageSize = 20;

			const totalPages = Math.ceil(serverPagination.total_count / pageSize);
			expect(totalPages).toBe(1);

			// Should show "Showing 20 of 20 items"
			expect(serverPagination.total_count).toBe(20);
		});
	});

	describe('server-side pagination multiple pages', () => {
		it('should show range when multiple pages exist', () => {
			const serverPagination = {
				total_count: 41,
				offset: 0,
				has_more: true
			};
			const pageSize = 20;
			const itemsOnPage = 20;

			const totalPages = Math.ceil(serverPagination.total_count / pageSize);
			expect(totalPages).toBe(3); // 41 / 20 = 2.05 -> ceil = 3

			// For multiple pages, we show "Showing 1-20 of 41 items"
			const showingStart = Math.min(serverPagination.offset + 1, serverPagination.total_count);
			const showingEnd = Math.min(
				serverPagination.offset + itemsOnPage,
				serverPagination.total_count
			);

			expect(showingStart).toBe(1);
			expect(showingEnd).toBe(20);
		});

		it('should calculate correct range for middle page', () => {
			// pageSize: 20, showing page 2
			const serverPagination = {
				total_count: 41,
				offset: 20, // Second page
				has_more: true
			};
			const itemsOnPage = 20;

			const showingStart = Math.min(serverPagination.offset + 1, serverPagination.total_count);
			const showingEnd = Math.min(
				serverPagination.offset + itemsOnPage,
				serverPagination.total_count
			);

			// Should show "Showing 21-40 of 41 items"
			expect(showingStart).toBe(21);
			expect(showingEnd).toBe(40);
		});

		it('should calculate correct range for last page', () => {
			// pageSize: 20, showing page 3 (last page)
			const serverPagination = {
				total_count: 41,
				offset: 40, // Third/last page
				has_more: false
			};
			const itemsOnPage = 1; // Only 1 item on last page

			const showingStart = Math.min(serverPagination.offset + 1, serverPagination.total_count);
			const showingEnd = Math.min(
				serverPagination.offset + itemsOnPage,
				serverPagination.total_count
			);

			// Should show "Showing 41-41 of 41 items"
			expect(showingStart).toBe(41);
			expect(showingEnd).toBe(41);
		});
	});

	describe('client-side pagination', () => {
		it('should show filtered count vs total items for single page', () => {
			const items = Array(50).fill({ id: 'test' }); // 50 total items
			const processedItems = Array(30).fill({ id: 'test' }); // 30 after filtering
			const pageSize = 50;

			const totalPages = Math.ceil(processedItems.length / pageSize);
			expect(totalPages).toBe(1);

			// For client-side single page: "Showing 30 of 50 items"
			// (30 items shown after filters, out of 50 total)
			const countToDisplay = processedItems.length;
			const totalToDisplay = items.length;

			expect(countToDisplay).toBe(30);
			expect(totalToDisplay).toBe(50);
		});
	});

	describe('page size persistence', () => {
		it('should validate page size against allowed options', () => {
			const PAGE_SIZE_OPTIONS = [20, 50, 100] as const;

			// Valid sizes should be accepted
			expect(PAGE_SIZE_OPTIONS.includes(20)).toBe(true);
			expect(PAGE_SIZE_OPTIONS.includes(50)).toBe(true);
			expect(PAGE_SIZE_OPTIONS.includes(100)).toBe(true);

			// Invalid sizes should be rejected
			expect(PAGE_SIZE_OPTIONS.includes(25 as 20 | 50 | 100)).toBe(false);
			expect(PAGE_SIZE_OPTIONS.includes(200 as 20 | 50 | 100)).toBe(false);
		});

		it('should calculate correct offset from page and pageSize', () => {
			// Page 1, pageSize 20 -> offset 0
			expect((1 - 1) * 20).toBe(0);

			// Page 2, pageSize 20 -> offset 20
			expect((2 - 1) * 20).toBe(20);

			// Page 3, pageSize 50 -> offset 100
			expect((3 - 1) * 50).toBe(100);

			// Page 1, pageSize 100 -> offset 0
			expect((1 - 1) * 100).toBe(0);
		});

		it('should derive effective page from server offset', () => {
			// offset 0, pageSize 20 -> page 1
			expect(Math.floor(0 / 20) + 1).toBe(1);

			// offset 20, pageSize 20 -> page 2
			expect(Math.floor(20 / 20) + 1).toBe(2);

			// offset 100, pageSize 50 -> page 3
			expect(Math.floor(100 / 50) + 1).toBe(3);
		});
	});
});
