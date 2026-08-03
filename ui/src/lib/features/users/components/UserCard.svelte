<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { User } from '../types';
	import { Edit, Trash2 } from 'lucide-svelte';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import type { Color } from '$lib/shared/utils/styling';
	import { entities, permissions, metadata } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useDeleteUserMutation } from '$lib/features/users/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { entityRef } from '$lib/shared/components/data/types';
	import {
		common_all,
		common_authentication,
		common_delete,
		common_edit,
		common_emailAndPassword,
		common_joined,
		common_networks,
		common_role,
		common_unknownNetwork,
		common_you,
		users_confirmDeleteUser
	} from '$lib/paraglide/messages';

	let {
		user,
		selected,
		onSelectionChange,
		onEdit
	}: {
		user: User;
		selected: boolean;
		onSelectionChange: (selected: boolean) => void;
		onEdit?: (user: User) => void;
	} = $props();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// TanStack Query mutation for deleting user
	const deleteUserMutation = useDeleteUserMutation();

	// Force Svelte to track metadata reactivity
	$effect(() => {
		void $metadata;
		void currentUser;
	});

	function handleDeleteUser() {
		if (confirm(users_confirmDeleteUser())) {
			deleteUserMutation.mutate(user.id);
		}
	}
	function handleEditUser() {
		onEdit?.(user);
	}

	let canManage = $derived(
		currentUser
			? (permissions
					.getMetadata(currentUser.permissions)
					?.grantable_user_permissions?.includes(user.permissions) ?? false)
			: false
	);

	// Build card data
	let cardData = $derived({
		title: user.email,
		iconColor: entities.getColorHelper('User').icon,
		Icon: entities.getIconComponent('User'),
		status:
			user.id == currentUser?.id
				? {
						label: common_you(),
						color: 'Yellow' as Color
					}
				: null,
		fields: [
			{
				label: common_role(),
				value: [
					{
						id: user.id,
						label: permissions.getName(user.permissions),
						color: permissions.getColorHelper(user.permissions).color
					}
				]
			},
			{
				label: common_authentication(),
				value: user.oidc_provider || common_emailAndPassword()
			},
			{
				label: common_joined(),
				value: formatTimestamp(user.created_at)
			},
			{
				label: common_networks(),
				value:
					user.permissions == 'Admin' || user.permissions == 'Owner'
						? [
								{
									id: user.id,
									label: common_all(),
									color: entities.getColorHelper('Network').color
								}
							]
						: user.network_ids.map((n) => {
								const network = networksData.find((net) => net.id == n);
								return {
									id: n,
									label: network?.name ?? common_unknownNetwork(),
									color: entities.getColorHelper('Network').color,
									...(network ? { entityRef: entityRef('Network', n, network) } : {})
								};
							})
			}
		],
		actions:
			canManage && user.id != currentUser?.id
				? [
						{
							label: common_delete(),
							icon: Trash2,
							onClick: () => handleDeleteUser(),
							class: 'btn-icon-danger'
						},
						{
							label: common_edit(),
							icon: Edit,
							onClick: () => handleEditUser(),
							class: 'btn-icon'
						}
					]
				: []
	});
</script>

<GenericCard {...cardData} {selected} {onSelectionChange} selectable={user.id != currentUser?.id} />
