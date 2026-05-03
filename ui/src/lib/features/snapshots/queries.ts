/**
 * TanStack Query hooks for Snapshots
 *
 * Snapshots capture point-in-time topology state for a network. Live view
 * is the topology row with `snapshot_id IS NULL`; each past snapshot has
 * its own topology row whose `snapshot_id` matches a row in this list.
 *
 * Endpoints (not yet in the generated OpenAPI schema; we call them via
 * fetch until `make generate-types` is run after the backend lands):
 * - GET    /api/v1/snapshots?network_id=...   → Snapshot[]
 * - POST   /api/v1/snapshots                  → Snapshot
 * - DELETE /api/v1/snapshots/{id}             → void
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { pushSuccess } from '$lib/shared/stores/feedback';
import { formatTimestamp } from '$lib/shared/utils/formatting';
import { topology_snapshotCreated } from '$lib/paraglide/messages';

/**
 * Snapshot type. Mirrors the backend `Snapshot` entity that the foundation
 * worker is adding. The shape comes from the project plan; replace this
 * declaration with `components['schemas']['Snapshot']` once backend types
 * are regenerated.
 */
export interface Snapshot {
	id: string;
	network_id: string;
	taken_at: string;
	created_by_user_id: string | null;
	created_at: string;
	updated_at: string;
}

interface ApiResponse<T> {
	success: boolean;
	data: T | null;
	error: string | null;
}

async function getSnapshotsForNetwork(networkId: string): Promise<Snapshot[]> {
	const params = new URLSearchParams({ network_id: networkId, limit: '0' });
	const response = await fetch(`/api/v1/snapshots?${params.toString()}`, {
		credentials: 'include'
	});
	const body = (await response.json()) as ApiResponse<Snapshot[]>;
	if (!body?.success || !body.data) {
		throw new Error(body?.error || 'Failed to fetch snapshots');
	}
	return body.data;
}

async function postSnapshot(networkId: string): Promise<Snapshot> {
	const response = await fetch('/api/v1/snapshots', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		credentials: 'include',
		body: JSON.stringify({ network_id: networkId })
	});
	const body = (await response.json()) as ApiResponse<Snapshot>;
	if (!body?.success || !body.data) {
		throw new Error(body?.error || 'Failed to take snapshot');
	}
	return body.data;
}

async function deleteSnapshotRequest(snapshotId: string): Promise<void> {
	const response = await fetch(`/api/v1/snapshots/${snapshotId}`, {
		method: 'DELETE',
		credentials: 'include'
	});
	const body = (await response.json()) as ApiResponse<unknown>;
	if (!body?.success) {
		throw new Error(body?.error || 'Failed to delete snapshot');
	}
}

/**
 * Query hook: list snapshots for a network, sorted by `taken_at DESC`.
 *
 * Pass a getter so the query refetches when the network selection changes.
 */
export function useSnapshotsQuery(networkId: () => string | undefined) {
	return createQuery(() => ({
		queryKey: queryKeys.snapshots.byNetwork(networkId() ?? ''),
		queryFn: async () => {
			const id = networkId();
			if (!id) return [] as Snapshot[];
			const snapshots = await getSnapshotsForNetwork(id);
			return [...snapshots].sort(
				(a, b) => new Date(b.taken_at).getTime() - new Date(a.taken_at).getTime()
			);
		},
		enabled: () => !!networkId()
	}));
}

/**
 * Mutation hook: capture a new snapshot for the given network.
 *
 * On success: invalidates the per-network snapshots list and the topology
 * list (the backend's snapshot subscriber inserts a topology row for the
 * new snapshot — clients refetch to pick it up).
 */
export function useTakeSnapshotMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: ({ network_id }: { network_id: string }) => postSnapshot(network_id),
		onSuccess: (snapshot: Snapshot) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.snapshots.byNetwork(snapshot.network_id)
			});
			queryClient.invalidateQueries({ queryKey: queryKeys.topology.all });
			pushSuccess(topology_snapshotCreated({ time: formatTimestamp(snapshot.taken_at) }));
		}
	}));
}

/**
 * Mutation hook: delete a snapshot. The backend's CASCADE FKs reap the
 * snapshot's topology row and all closed entity rows tied to it.
 */
export function useDeleteSnapshotMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: ({ snapshot_id }: { snapshot_id: string; network_id: string }) =>
			deleteSnapshotRequest(snapshot_id),
		onSuccess: (_void, variables) => {
			queryClient.invalidateQueries({
				queryKey: queryKeys.snapshots.byNetwork(variables.network_id)
			});
			queryClient.invalidateQueries({ queryKey: queryKeys.topology.all });
		}
	}));
}
