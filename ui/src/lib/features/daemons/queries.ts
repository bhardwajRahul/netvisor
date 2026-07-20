/**
 * TanStack Query hooks for Daemons
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Daemon } from './types/base';
import type { DiscoveryUpdatePayload } from '../discovery/types/api';
import type { ProvisionDaemonRequest, ProvisionDaemonResponse } from './types/base';
/**
 * Query hook for fetching all daemons
 * @param options.enabled - Optional getter function to control when query is enabled
 */
export function useDaemonsQuery(options?: { enabled?: () => boolean }) {
	return createQuery(() => ({
		queryKey: queryKeys.daemons.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/daemons', {
				params: { query: { limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch daemons');
			}
			return data.data;
		},
		enabled: options?.enabled?.() ?? true
	}));
}

/**
 * Query hook for fetching a single daemon by ID
 */
export function useDaemonQuery(id: () => string | null, options?: { enabled?: () => boolean }) {
	return createQuery(() => ({
		queryKey: queryKeys.daemons.detail(id() ?? ''),
		queryFn: async () => {
			const daemonId = id();
			if (!daemonId) throw new Error('No daemon ID');
			const { data } = await apiClient.GET('/api/v1/daemons/{id}', {
				params: { path: { id: daemonId } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch daemon');
			}
			return data.data;
		},
		enabled: (options?.enabled?.() ?? true) && !!id()
	}));
}

/**
 * Query hook for a daemon's reconfigure command — the idempotent, credential-free command that
 * re-asserts the server-held connectivity config on an already-installed daemon.
 */
export function useDaemonInstallCommandQuery(
	id: () => string | null,
	options?: { enabled?: () => boolean }
) {
	return createQuery(() => ({
		queryKey: [...queryKeys.daemons.detail(id() ?? ''), 'install-command', 'sync'],
		queryFn: async () => {
			const daemonId = id();
			if (!daemonId) throw new Error('No daemon ID');
			const { data } = await apiClient.GET('/api/v1/daemons/{id}/install-command', {
				params: { path: { id: daemonId }, query: { purpose: 'sync' } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch install command');
			}
			return data.data;
		},
		enabled: (options?.enabled?.() ?? true) && !!id()
	}));
}

/**
 * Mutation hook for updating a daemon's server-side record (name, maintainer, tags, and
 * the ServerPoll url). Identity and server-managed fields are restored server-side.
 */
export function useUpdateDaemonMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (daemon: Daemon) => {
			const { data } = await apiClient.PUT('/api/v1/daemons/{id}', {
				params: { path: { id: daemon.id } },
				body: daemon
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update daemon');
			}
			return data.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.daemons.all });
		}
	}));
}

/**
 * Mutation hook for deleting a daemon
 */
export function useDeleteDaemonMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/daemons/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete daemon');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Daemon[]>(
				queryKeys.daemons.all,
				(old) => old?.filter((d) => d.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for bulk deleting daemons
 */
export function useBulkDeleteDaemonsMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			const { data } = await apiClient.POST('/api/v1/daemons/bulk-delete', { body: ids });
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete daemons');
			}
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Daemon[]>(
				queryKeys.daemons.all,
				(old) => old?.filter((d) => !ids.includes(d.id)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for provisioning a daemon (either mode) before install.
 * Creates the daemon record + its 1:1 API key server-side.
 */
export function useProvisionDaemonMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: ProvisionDaemonRequest): Promise<ProvisionDaemonResponse> => {
			const { data } = await apiClient.POST('/api/v1/daemons/provision', {
				body: request
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to provision daemon');
			}
			return data.data;
		},
		onSuccess: (response: ProvisionDaemonResponse) => {
			// Re-provisioning returns an existing daemon, so replace it in place rather than
			// appending a duplicate; a fresh provision appends.
			queryClient.setQueryData<Daemon[]>(queryKeys.daemons.all, (old) => {
				const existing = old ?? [];
				return existing.some((d) => d.id === response.daemon.id)
					? existing.map((d) => (d.id === response.daemon.id ? response.daemon : d))
					: [...existing, response.daemon];
			});
		}
	}));
}

/**
 * Mutation hook for retrying connection to an unreachable daemon
 */
export function useRetryDaemonConnectionMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.POST('/api/v1/daemons/{id}/retry-connection', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to retry daemon connection');
			}
			return id;
		},
		onSuccess: (id: string) => {
			// Update the daemon in the cache to mark as reachable
			queryClient.setQueryData<Daemon[]>(
				queryKeys.daemons.all,
				(old) => old?.map((d) => (d.id === id ? { ...d, is_unreachable: false } : d)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for testing daemon URL reachability
 */
export function useTestReachabilityMutation() {
	return createMutation(() => ({
		mutationFn: async (request: { url: string; check_health: boolean }) => {
			const { data } = await apiClient.POST('/api/v1/daemons/test-reachability', {
				body: request
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to test reachability');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation to email the install command to the current user
 */
export function useEmailInstallCommandMutation() {
	return createMutation(() => ({
		mutationFn: async ({ installCommand, os }: { installCommand: string; os: string }) => {
			const { data } = await apiClient.POST('/api/v1/daemons/email-install-command', {
				body: { install_command: installCommand, os }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to send email');
			}
		}
	}));
}

/**
 * Helper to check if a daemon is currently running a discovery session
 */
export function getDaemonIsRunningDiscovery(
	daemon_id: string | null,
	sessions: DiscoveryUpdatePayload[]
): boolean {
	if (!daemon_id) return false;

	// Find any active session for this daemon
	for (const session of sessions) {
		if (
			session.daemon_id === daemon_id &&
			(session.phase === 'Pending' ||
				session.phase === 'Starting' ||
				session.phase === 'Started' ||
				session.phase === 'Scanning')
		) {
			return true;
		}
	}
	return false;
}
