<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import UserCard from './UserCard.svelte';
	import InviteCard from './InviteCard.svelte';
	import { useInvitesQuery } from '$lib/features/organizations/queries';
	import { UserPlus } from 'lucide-svelte';
	import { isUser, type User, type UserOrInvite } from '../types';
	import InviteModal from './InviteModal.svelte';
	import { metadata, permissions } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import UserEditModal from './UserEditModal.svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useUsersQuery, useBulkDeleteUsersMutation } from '../queries';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import {
		common_confirmBulkDelete,
		common_created,
		common_email,
		common_emailAndPassword,
		common_role,
		common_users,
		users_authMethod,
		users_inviteUser,
		users_noUsersFound,
		users_noUsersSubtitle,
		users_subtitle,
		users_verifyEmailToInvite
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();

	// Query
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let seatLimit = $derived(org?.plan?.included_seats ?? null);
	let canBuyMoreSeats = $derived(
		org?.plan?.seat_cents !== undefined && org?.plan?.seat_cents !== null
	);

	const usersQuery = useUsersQuery();
	const bulkDeleteUsersMutation = useBulkDeleteUsersMutation();
	const invitesQuery = useInvitesQuery();

	// Derived data
	let usersData = $derived(usersQuery.data ?? []);
	let invitesData = $derived(invitesQuery.data ?? []);
	let isLoading = $derived(usersQuery.isPending);

	// Force Svelte to track metadata reactivity
	$effect(() => {
		void $metadata;
	});

	let showInviteModal = $state(false);
	let showEditModal = $state(false);
	let editingUser = $state<User | null>(null);

	// Deep-link: open invite modal from URL
	$effect(() => {
		if ($modalState.name === 'invite-user' && !showInviteModal) {
			showInviteModal = true;
		}
	});

	// Deep-link: open user editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'user-editor',
			usersData,
			showEditModal,
			editingUser?.id
		);
		if (result !== undefined) {
			editingUser = result;
			showEditModal = true;
		}
	});

	// Combine users and invites into single array
	let combinedItems = $derived([
		...usersData.map((user) => ({ type: 'user' as const, data: user, id: user.id })),
		...invitesData.map((invite) => ({ type: 'invite' as const, data: invite, id: invite.id }))
	] as UserOrInvite[]);

	let userCount = $derived(combinedItems.filter(isUser).length);
	let isAtSeatLimit = $derived(seatLimit !== null && userCount >= seatLimit && !canBuyMoreSeats);

	async function handleCreateInvite() {
		showInviteModal = true;
	}

	function handleCloseInviteModal() {
		showInviteModal = false;
	}

	// Check if user can invite
	let canInviteUsers = $derived(
		!isReadOnly && currentUser
			? (permissions.getMetadata(currentUser.permissions)?.grantable_user_permissions?.length ??
					0) > 0
			: false
	);

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_users() }))) {
			await bulkDeleteUsersMutation.mutateAsync(ids);
		}
	}

	function handleEditUser(user: User) {
		editingUser = user;
		showEditModal = true;
	}

	function handleCloseEditModal() {
		showEditModal = false;
		editingUser = null;
	}

	// CSV export handler (exports users only, not invites)
	async function handleCsvExport() {
		await downloadCsv('User', {});
	}

	// Only define fields for users (invites won't be filtered/sorted)
	const userFields: FieldConfig<UserOrInvite>[] = [
		{
			key: 'email',
			label: common_email(),
			type: 'string',
			searchable: true,
			sortable: true,
			getValue(item) {
				return isUser(item) ? item.data.email : '';
			}
		},
		{
			key: 'permissions',
			label: common_role(),
			type: 'string',
			searchable: true,
			filterable: true,
			groupable: true,
			getValue(item) {
				return isUser(item) ? item.data.permissions : '';
			}
		},
		{
			key: 'oidc_provider',
			label: users_authMethod(),
			type: 'string',
			searchable: true,
			filterable: true,
			groupable: true,
			getValue(item) {
				return isUser(item) ? item.data.oidc_provider || common_emailAndPassword() : '';
			}
		},
		{
			key: 'created_at',
			label: common_created(),
			type: 'date',
			sortable: true,
			getValue: (item) => item.data.created_at
		}
	];
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_users()} subtitle={users_subtitle()}>
		<svelte:fragment slot="actions">
			<div class="flex items-center gap-3">
				{#if seatLimit !== null && !canBuyMoreSeats}
					<span class="text-sm {isAtSeatLimit ? 'text-amber-400' : 'text-tertiary'}">
						{userCount} / {seatLimit}
					</span>
				{/if}
				{#if canInviteUsers}
					{#if isAtSeatLimit}
						<UpgradeButton feature="seats" surface="users_tab" gate_type="limit_hit" />
					{:else if currentUser && !currentUser.email_verified}
						<span data-tooltip={users_verifyEmailToInvite()} use:tooltip>
							<button class="btn-primary flex items-center opacity-50" disabled>
								<UserPlus class="mr-2 h-5 w-5" />
								{users_inviteUser()}
							</button>
						</span>
					{:else}
						<button class="btn-primary flex items-center" onclick={handleCreateInvite}>
							<UserPlus class="mr-2 h-5 w-5" />
							{users_inviteUser()}
						</button>
					{/if}
				{/if}
			</div>
		</svelte:fragment>
	</TabHeader>

	<!-- Loading state -->
	{#if isLoading}
		<Loading />
	{:else if combinedItems.length === 0}
		<!-- Empty state -->
		<EmptyState title={users_noUsersFound()} subtitle={users_noUsersSubtitle()} />
	{:else}
		<DataControls
			items={combinedItems}
			fields={userFields}
			storageKey="scanopy-users-table-state"
			onBulkDelete={handleBulkDelete}
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: UserOrInvite,
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				{#if isUser(item)}
					<UserCard
						user={item.data}
						selected={isSelected}
						{onSelectionChange}
						onEdit={handleEditUser}
					/>
				{:else}
					<InviteCard invite={item.data} />
				{/if}
			{/snippet}
		</DataControls>
	{/if}
</div>

<InviteModal name="invite-user" isOpen={showInviteModal} onClose={handleCloseInviteModal} />
<UserEditModal
	name="user-editor"
	isOpen={showEditModal}
	user={editingUser}
	onClose={handleCloseEditModal}
/>
