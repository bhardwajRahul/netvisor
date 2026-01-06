/**
 * TanStack Query hooks for Groups
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Group } from './types/base';

/**
 * Query hook for fetching all groups
 */
export function useGroupsQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.groups.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/groups', {
				params: { query: { pagination: { limit: 0 } } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch groups');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook for creating a group
 */
export function useCreateGroupMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (group: Group) => {
			const { data } = await apiClient.POST('/api/v1/groups', { body: group });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create group');
			}
			return data.data;
		},
		onSuccess: (newGroup: Group) => {
			queryClient.setQueryData<Group[]>(queryKeys.groups.all, (old) =>
				old ? [...old, newGroup] : [newGroup]
			);
			// Invalidate services as group creation may affect service bindings
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for updating a group
 */
export function useUpdateGroupMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (group: Group) => {
			const { data } = await apiClient.PUT('/api/v1/groups/{id}', {
				params: { path: { id: group.id } },
				body: group
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update group');
			}
			return data.data;
		},
		onSuccess: (updatedGroup: Group) => {
			queryClient.setQueryData<Group[]>(
				queryKeys.groups.all,
				(old) => old?.map((g) => (g.id === updatedGroup.id ? updatedGroup : g)) ?? []
			);
			// Invalidate services as group update may affect service bindings
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for deleting a group
 */
export function useDeleteGroupMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/groups/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete group');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Group[]>(
				queryKeys.groups.all,
				(old) => old?.filter((g) => g.id !== id) ?? []
			);
			// Invalidate services as group deletion may affect service bindings
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

/**
 * Mutation hook for bulk deleting groups
 */
export function useBulkDeleteGroupsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/v1/groups/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete groups');
			}
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Group[]>(
				queryKeys.groups.all,
				(old) => old?.filter((g) => !ids.includes(g.id)) ?? []
			);
			// Invalidate services as group deletion may affect service bindings
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
		}
	}));
}

import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';
import { entities } from '$lib/shared/stores/metadata';
import type { Color } from '$lib/shared/utils/styling';

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create empty form data for creating a new group
 */
export function createEmptyGroupFormData(defaultNetworkId?: string): Group {
	return {
		id: uuidv4Sentinel,
		name: '',
		description: '',
		binding_ids: [],
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		group_type: 'RequestPath',
		source: {
			type: 'Manual'
		},
		network_id: defaultNetworkId ?? '',
		color: entities.getColorHelper('Group').color as Color,
		edge_style: 'Straight',
		tags: []
	};
}
