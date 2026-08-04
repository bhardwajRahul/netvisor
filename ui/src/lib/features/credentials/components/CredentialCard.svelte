<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { Credential } from '../types/base';
	import type { EntityColumn } from '$lib/shared/components/data/table/columns';
	import { getCredentialTypeId } from '../types/base';
	import { credentialTypes, permissions } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { common_delete, common_edit } from '$lib/paraglide/messages';

	let {
		credential,
		columns,
		onEdit = () => {},
		onDelete = () => {},
		selected,
		onSelectionChange = () => {}
	}: {
		credential: Credential;
		/** The shared field definition, from the tab — the same list the table renders. */
		columns: EntityColumn<Credential>[];
		onEdit?: (credential: Credential) => void;
		onDelete?: (credential: Credential) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	let typeId = $derived(getCredentialTypeId(credential));

	let canManage = $derived(
		(currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities) || false
	);

	let cardData = $derived({
		title: credential.name,
		iconColor: credentialTypes.getColorHelper(typeId).icon,
		Icon: credentialTypes.getIconComponent(typeId),
		actions: [
			...(canManage
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(credential)
						},
						{
							label: common_edit(),
							icon: Edit,
							onClick: () => onEdit(credential)
						}
					]
				: [])
		]
	});
</script>

<GenericCard
	{...cardData}
	{columns}
	item={credential}
	{selected}
	{onSelectionChange}
	selectable={canManage}
/>
