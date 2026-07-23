/**
 * TanStack Query hooks for Organizations and Invites
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys, queryClient } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { CreateInviteRequest, OrganizationInvite, Organization } from './types';
import type { UserOrgPermissions, User } from '../users/types';
import type { components } from '$lib/api/schema';

/**
 * Query hook for fetching current organization
 * Only fetches when user is authenticated
 */
export function useOrganizationQuery() {
	return createQuery(() => {
		// Check if user is authenticated before fetching
		const user = queryClient.getQueryData<User | null>(queryKeys.auth.currentUser());
		return {
			queryKey: queryKeys.organizations.current(),
			queryFn: async () => {
				const { data } = await apiClient.GET('/api/v1/organizations');
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch organization');
				}
				return data.data;
			},
			// Only fetch when user is authenticated
			enabled: !!user
		};
	});
}

/**
 * Query hook for fetching invites
 * @param options.enabled - Whether to enable the query (default: true). Can be a boolean or getter function for reactivity.
 */
export function useInvitesQuery(options?: { enabled?: boolean | (() => boolean) }) {
	return createQuery(() => {
		const enabled =
			typeof options?.enabled === 'function' ? options.enabled() : (options?.enabled ?? true);
		return {
			queryKey: queryKeys.invites.all,
			queryFn: async () => {
				const { data } = await apiClient.GET('/api/v1/invites');
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch invites');
				}
				return data.data;
			},
			enabled
		};
	});
}

/**
 * Mutation hook for updating organization name
 */
export function useUpdateOrganizationMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({ id, name }: { id: string; name: string }) => {
			const { data } = await apiClient.PUT('/api/v1/organizations/{id}', {
				params: { path: { id } },
				body: name
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to update organization');
			}
			return data.data;
		},
		onSuccess: (updatedOrg: Organization) => {
			queryClient.setQueryData(queryKeys.organizations.current(), updatedOrg);
		}
	}));
}

/**
 * Mutation hook for creating an invite
 */
export function useCreateInviteMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async ({
			permissions,
			network_ids,
			email
		}: {
			permissions: UserOrgPermissions;
			network_ids: string[];
			email: string;
		}) => {
			const request: CreateInviteRequest = {
				expiration_hours: null,
				permissions,
				network_ids,
				send_to: email?.length === 0 ? null : email
			};

			const { data } = await apiClient.POST('/api/v1/invites', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create invite');
			}
			return data.data;
		},
		onSuccess: (newInvite: OrganizationInvite) => {
			queryClient.setQueryData<OrganizationInvite[]>(queryKeys.invites.all, (old) =>
				old ? [...old, newInvite] : [newInvite]
			);
		}
	}));
}

/**
 * Mutation hook for revoking an invite
 */
export function useRevokeInviteMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/invites/{id}/revoke', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to revoke invite');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<OrganizationInvite[]>(
				queryKeys.invites.all,
				(old) => old?.filter((i) => i.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for resetting organization data
 */
export function useResetOrganizationDataMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (orgId: string) => {
			const { data } = await apiClient.POST('/api/v1/organizations/{id}/reset', {
				params: { path: { id: orgId } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to reset organization data');
			}
			return data;
		},
		onSuccess: () => {
			// Invalidate all data queries after reset
			queryClient.invalidateQueries();
		}
	}));
}

/**
 * Mutation hook for deleting an organization
 */
export function useDeleteOrganizationMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (orgId: string) => {
			const { data } = await apiClient.DELETE('/api/v1/organizations/{id}', {
				params: { path: { id: orgId } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete organization');
			}
			return data;
		},
		onSuccess: () => {
			queryClient.clear();
		}
	}));
}

/**
 * Mutation hook that kicks off demo-data population. The backend runs the work
 * in the background and returns `202` immediately with a `Running` status;
 * callers poll {@link fetchDemoPopulateStatus} for completion (and invalidate
 * queries then, not here).
 */
export function usePopulateDemoDataMutation() {
	return createMutation(() => ({
		mutationFn: async (orgId: string) => {
			const { data } = await apiClient.POST('/api/v1/organizations/{id}/populate-demo', {
				params: { path: { id: orgId } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to start demo data population');
			}
			return data.data;
		}
	}));
}

/**
 * Fetch the current status of an org's background demo-populate task.
 * Throws on transport/API error.
 */
export async function fetchDemoPopulateStatus(
	orgId: string
): Promise<components['schemas']['DemoPopulateStatus']> {
	const { data } = await apiClient.GET('/api/v1/organizations/{id}/populate-demo/status', {
		params: { path: { id: orgId } }
	});
	if (!data?.success || !data.data) {
		throw new Error(data?.error || 'Failed to fetch demo populate status');
	}
	return data.data;
}

/**
 * Mutation hook for recording the user's response to the daemon-install prompt.
 * Each CTA persists a distinct onboarding milestone so the prompt is not shown again
 * (and survives reload). Optimistically appends the milestone to the cached org so the
 * modal's open-gate flips immediately.
 */
export function useDaemonPromptResponseMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (action: components['schemas']['DaemonPromptAction']) => {
			const { data } = await apiClient.POST('/api/v1/organizations/daemon-prompt-response', {
				body: { action }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to record daemon prompt response');
			}
			return action;
		},
		onSuccess: (action) => {
			const milestone: components['schemas']['OnboardingOperationDiscriminants'] =
				action === 'dismissed' ? 'DaemonPromptDismissed' : 'DaemonPromptAccepted';
			queryClient.setQueryData<Organization>(queryKeys.organizations.current(), (old) =>
				old && !old.onboarding.includes(milestone)
					? { ...old, onboarding: [...old.onboarding, milestone] }
					: old
			);
		}
	}));
}

/**
 * Helper to format invite URL
 */
export function formatInviteUrl(invite: OrganizationInvite): string {
	return `${invite.url}/api/v1/invites/${invite.id}/accept`;
}

/**
 * Fetch organization directly (bypasses enabled check, useful after login/register)
 */
export async function fetchOrganization(): Promise<Organization> {
	return queryClient.fetchQuery({
		queryKey: queryKeys.organizations.current(),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/organizations');
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch organization');
			}
			return data.data;
		}
	});
}

/**
 * Mutation hook for submitting referral source
 */
export function useReferralSourceMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (request: { referral_source: string; referral_source_other?: string }) => {
			const { data } = await apiClient.POST('/api/v1/organizations/referral-source', {
				body: request
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to submit referral source');
			}
			return true;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.organizations.all });
		}
	}));
}
