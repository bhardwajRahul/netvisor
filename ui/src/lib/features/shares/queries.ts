/**
 * TanStack Query hooks for Shares
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Share, CreateUpdateShareRequest } from './types/base';

/**
 * Query hook for fetching all shares
 */
export function useSharesQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.shares.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/shares', {
				params: { query: { pagination: { limit: 0 } } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch shares');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook for creating a share
 */
export function useCreateShareMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: CreateUpdateShareRequest) => {
			const { data } = await apiClient.POST('/api/v1/shares', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create share');
			}
			return data.data;
		},
		onSuccess: (newShare: Share) => {
			queryClient.setQueryData<Share[]>(queryKeys.shares.all, (old) =>
				old ? [...old, newShare] : [newShare]
			);
		}
	}));
}

/**
 * Mutation hook for updating a share
 */
export function useUpdateShareMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ id, request }: { id: string; request: CreateUpdateShareRequest }) => {
			const { data } = await apiClient.PUT('/api/v1/shares/{id}', {
				params: { path: { id } },
				body: request
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update share');
			}
			return data.data;
		},
		onSuccess: (updatedShare: Share) => {
			queryClient.setQueryData<Share[]>(
				queryKeys.shares.all,
				(old) => old?.map((s) => (s.id === updatedShare.id ? updatedShare : s)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a share
 */
export function useDeleteShareMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/shares/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete share');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Share[]>(
				queryKeys.shares.all,
				(old) => old?.filter((s) => s.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting shares
 */
export function useBulkDeleteSharesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/v1/shares/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete shares');
			}
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Share[]>(
				queryKeys.shares.all,
				(old) => old?.filter((s) => !ids.includes(s.id)) ?? []
			);
		}
	}));
}

import type { PublicShareMetadata, ShareWithTopology } from './types/base';

// ============================================================================
// Public API Functions (no auth required)
// ============================================================================

/**
 * Fetch public share metadata
 */
export async function getPublicShareMetadata(
	shareId: string
): Promise<{ success: boolean; data?: PublicShareMetadata; error?: string }> {
	try {
		const response = await fetch(`/api/v1/shares/public/${shareId}`, {
			method: 'GET',
			headers: {
				'Content-Type': 'application/json'
			}
		});

		const result = await response.json();

		if (!response.ok || result.error) {
			return { success: false, error: result.error || 'Failed to fetch share' };
		}

		return { success: true, data: result.data };
	} catch {
		return { success: false, error: 'Failed to fetch share' };
	}
}

/**
 * Verify share password
 */
export async function verifySharePassword(
	shareId: string,
	password: string
): Promise<{ success: boolean; error?: string }> {
	try {
		const response = await fetch(`/api/v1/shares/public/${shareId}/verify`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(password)
		});

		const result = await response.json();

		if (!response.ok || result.error) {
			return { success: false, error: result.error || 'Invalid password' };
		}

		return { success: true };
	} catch {
		return { success: false, error: 'Failed to verify password' };
	}
}

/**
 * Fetch public share topology
 */
export async function getPublicShareTopology(
	shareId: string,
	options: { embed?: boolean; password?: string } = {}
): Promise<{ success: boolean; data?: ShareWithTopology; error?: string }> {
	try {
		const url = options.embed
			? `/api/v1/shares/public/${shareId}/topology?embed=true`
			: `/api/v1/shares/public/${shareId}/topology`;
		const response = await fetch(url, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ password: options.password })
		});

		const result = await response.json();

		if (!response.ok || result.error) {
			return { success: false, error: result.error || 'Failed to fetch topology' };
		}

		return { success: true, data: result.data };
	} catch {
		return { success: false, error: 'Failed to fetch topology' };
	}
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Generate share URL
 */
export function generateShareUrl(shareId: string): string {
	if (typeof window !== 'undefined') {
		return `${window.location.origin}/share/${shareId}`;
	}
	return `/share/${shareId}`;
}

/**
 * Generate embed URL for a share
 */
export function generateEmbedUrl(shareId: string): string {
	if (typeof window !== 'undefined') {
		return `${window.location.origin}/share/${shareId}?embed=true`;
	}
	return `/share/${shareId}?embed=true`;
}

/**
 * Generate embed code for a share
 */
export function generateEmbedCode(
	shareId: string,
	width: string | number = '100%',
	height: string | number = '600px'
): string {
	const embedUrl = generateEmbedUrl(shareId);
	const widthStr = typeof width === 'number' ? `${width}px` : width;
	const heightStr = typeof height === 'number' ? `${height}px` : height;
	return `<iframe src="${embedUrl}" width="${widthStr}" height="${heightStr}" frameborder="0" style="border: 1px solid #374151; border-radius: 8px;"></iframe>`;
}

/**
 * Store share password in session storage
 */
export function storeSharePassword(shareId: string, password: string): void {
	if (typeof window !== 'undefined') {
		sessionStorage.setItem(`share_password_${shareId}`, password);
	}
}

/**
 * Get stored share password from session storage
 */
export function getStoredSharePassword(shareId: string): string | null {
	if (typeof window !== 'undefined') {
		return sessionStorage.getItem(`share_password_${shareId}`);
	}
	return null;
}
