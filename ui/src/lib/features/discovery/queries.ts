/**
 * TanStack Query hooks for Discovery
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Discovery } from './types/base';

/**
 * Query hook for fetching all discoveries
 */
export function useDiscoveriesQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.discovery.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/discovery');
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch discoveries');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook for creating a discovery
 */
export function useCreateDiscoveryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (discovery: Discovery) => {
			const { data } = await apiClient.POST('/api/discovery', { body: discovery });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create discovery');
			}
			return data.data;
		},
		onSuccess: (newDiscovery: Discovery) => {
			queryClient.setQueryData<Discovery[]>(queryKeys.discovery.all, (old) =>
				old ? [...old, newDiscovery] : [newDiscovery]
			);
		}
	}));
}

/**
 * Mutation hook for updating a discovery
 */
export function useUpdateDiscoveryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (discovery: Discovery) => {
			const { data } = await apiClient.PUT('/api/discovery/{id}', {
				params: { path: { id: discovery.id } },
				body: discovery
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update discovery');
			}
			return data.data;
		},
		onSuccess: (updatedDiscovery: Discovery) => {
			queryClient.setQueryData<Discovery[]>(
				queryKeys.discovery.all,
				(old) => old?.map((d) => (d.id === updatedDiscovery.id ? updatedDiscovery : d)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for deleting a discovery
 */
export function useDeleteDiscoveryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/discovery/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete discovery');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Discovery[]>(
				queryKeys.discovery.all,
				(old) => old?.filter((d) => d.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting discoveries
 */
export function useBulkDeleteDiscoveriesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/discovery/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete discoveries');
			}
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Discovery[]>(
				queryKeys.discovery.all,
				(old) => old?.filter((d) => !ids.includes(d.id)) ?? []
			);
		}
	}));
}

import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';
import type { Daemon } from '../daemons/types/base';
import type { FieldConfig } from '$lib/shared/components/data/types';

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create empty form data for a new discovery
 */
export function createEmptyDiscoveryFormData(daemon: Daemon | null): Discovery {
	return {
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		tags: [],
		discovery_type: {
			type: 'Network',
			subnet_ids: daemon ? daemon.capabilities.interfaced_subnet_ids : [],
			host_naming_fallback: 'Ip'
		},
		run_type: {
			type: 'Scheduled',
			last_run: null,
			cron_schedule: '0 0 * * * *',
			enabled: true
		},
		name: '',
		daemon_id: daemon ? daemon.id : uuidv4Sentinel,
		network_id: daemon ? daemon.network_id : uuidv4Sentinel
	};
}

/**
 * Parse a simple cron expression back to hours
 * Only handles the patterns we generate
 */
export function parseCronToHours(cron: string): number | null {
	const parts = cron.split(' ');
	if (parts.length !== 6) return null;

	const [, , hour, day, ,] = parts;

	// Daily pattern: "0 0 0 * * *"
	if (hour === '0' && day === '*') {
		return 24;
	}

	// Every N days: "0 0 0 */N * *"
	if (hour === '0' && day.startsWith('*/')) {
		const days = parseInt(day.slice(2));
		return days * 24;
	}

	// Every N hours: "0 0 */N * * *"
	if (hour.startsWith('*/')) {
		return parseInt(hour.slice(2));
	}

	// Every hour: "0 0 * * * *"
	if (hour === '*') {
		return 1;
	}

	return null;
}

/**
 * Generate a cron expression for "every N hours"
 * Format: "0 0 *\/N * * *" (second minute hour day month weekday)
 */
export function generateCronSchedule(hours: number): string {
	if (hours === 0) {
		return '0 0 * * * *'; // Every hour as fallback
	}
	if (hours === 1) {
		return '0 0 * * * *'; // Every hour
	}
	if (hours === 24) {
		return '0 0 0 * * *'; // Daily at midnight
	}
	if (hours % 24 === 0) {
		// Every N days at midnight
		const days = hours / 24;
		return `0 0 0 */${days} * *`;
	}
	// Every N hours
	return `0 0 */${hours} * * *`;
}

/**
 * Field configuration for the DataTableControls
 */
export const discoveryFields = (daemons: Daemon[]): FieldConfig<Discovery>[] => [
	{
		key: 'name',
		label: 'Name',
		type: 'string',
		searchable: true,
		filterable: false,
		sortable: true,
		getValue: (item: Discovery) => item.name
	},
	{
		key: 'daemon_id',
		label: 'Daemon',
		type: 'string',
		searchable: false,
		filterable: true,
		sortable: true,
		getValue: (item: Discovery) =>
			daemons.find((d) => d.id == item.daemon_id)?.name ?? 'Unknown Daemon'
	},
	{
		key: 'discovery_type',
		label: 'Type',
		type: 'string',
		searchable: false,
		filterable: true,
		sortable: true,
		getValue: (item: Discovery) => item.discovery_type.type
	}
];
