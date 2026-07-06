<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { UserPlus, UserX } from 'lucide-svelte';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import type { Color } from '$lib/shared/utils/styling';
	import { formatInviteUrl, useRevokeInviteMutation } from '$lib/features/organizations/queries';
	import { entities, permissions } from '$lib/shared/stores/metadata';
	import type { OrganizationInvite } from '$lib/features/organizations/types';
	import { useUsersQuery } from '$lib/features/users/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import {
		common_expires,
		common_permissions,
		common_revoke,
		common_unknownEntity,
		common_url,
		common_user,
		invites_confirmRevoke,
		invites_createdBy,
		invites_pendingInvite
	} from '$lib/paraglide/messages';

	let { invite, viewMode }: { invite: OrganizationInvite; viewMode: 'card' | 'list' } = $props();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// TanStack Query for users
	const usersQuery = useUsersQuery();
	let usersData = $derived(usersQuery.data ?? []);

	// Mutation for revoking invite
	const revokeInviteMutation = useRevokeInviteMutation();

	function handleRevokeInvite() {
		if (confirm(invites_confirmRevoke())) {
			revokeInviteMutation.mutate(invite.id);
		}
	}

	let canManage = $derived(
		currentUser
			? (permissions
					.getMetadata(currentUser.permissions)
					?.grantable_user_permissions?.includes(invite.permissions) ?? false)
			: false
	);

	// Build card data
	let cardData = $derived({
		title: invite.send_to || invites_pendingInvite(),
		iconColor: entities.getColorHelper('User').icon,
		Icon: UserPlus,
		status: { label: invites_pendingInvite(), color: 'Yellow' as Color },
		fields: [
			{
				label: common_url(),
				value: formatInviteUrl(invite)
			},
			{
				label: common_permissions(),
				value: invite.permissions
			},
			{
				label: invites_createdBy(),
				value:
					usersData.find((u) => u.id == invite.created_by)?.email ||
					common_unknownEntity({ entity: common_user() })
			},
			{
				label: common_expires(),
				value: formatTimestamp(invite.expires_at)
			}
		],
		actions: [
			...(canManage
				? [
						{
							label: common_revoke(),
							icon: UserX,
							class: 'btn-icon-danger',
							onClick: () => handleRevokeInvite()
						}
					]
				: [])
		]
	});
</script>

<GenericCard {...cardData} {viewMode} selectable={false} />
