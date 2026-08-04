<script lang="ts">
	import type { EntityColumn } from '$lib/shared/components/data/table/columns';
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { Tag } from '../types/base';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { TagIcon } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { permissions, concepts, billingPlans } from '$lib/shared/stores/metadata';
	import { common_delete, common_edit } from '$lib/paraglide/messages';

	let {
		tag,
		columns,
		onEdit = () => {},
		onDelete = () => {},
		selected,
		onSelectionChange = () => {}
	}: {
		tag: Tag;
		/** The shared field definition, from the tab — the same list the table renders. */
		columns: EntityColumn<Tag>[];
		onEdit?: (tag: Tag) => void;
		onDelete?: (tag: Tag) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let colorHelper = $derived(createColorHelper(tag.color));

	// Demo orgs are read-only for non-owners (mirrors the credentials tab)
	let isDemoOrg = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).is_demo === true
	);
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	let canManage = $derived(
		(!isNonOwnerInDemo &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities) ||
			false
	);

	let appIcon = $derived(tag.is_application ? concepts.getIconComponent('Application') : null);
	let appColor = $derived(tag.is_application ? concepts.getColorHelper('Application')?.icon : null);

	let cardData = $derived({
		title: tag.name,
		iconColor: appColor ?? colorHelper.icon,
		Icon: appIcon ?? TagIcon,
		actions: [
			...(canManage
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(tag)
						},
						{
							label: common_edit(),
							icon: Edit,
							onClick: () => onEdit(tag)
						}
					]
				: [])
		]
	});
</script>

<GenericCard
	{...cardData}
	{columns}
	item={tag}
	{selected}
	{onSelectionChange}
	selectable={canManage}
/>
