<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { Tag } from '../types/base';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { TagIcon } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { permissions, concepts, billingPlans } from '$lib/shared/stores/metadata';
	import {
		common_color,
		common_delete,
		common_description,
		common_edit,
		common_no,
		common_yes,
		common_application
	} from '$lib/paraglide/messages';

	let {
		tag,
		onEdit = () => {},
		onDelete = () => {},
		selected,
		onSelectionChange = () => {}
	}: {
		tag: Tag;
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
		fields: [
			{
				label: common_description(),
				value: tag.description
			},
			{
				label: common_color(),
				value: [
					{
						id: 'color',
						label: tag.color.charAt(0).toUpperCase() + tag.color.slice(1),
						color: tag.color
					}
				]
			},
			{
				label: common_application(),
				value: tag.is_application ? common_yes() : common_no()
			}
		],
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

<GenericCard {...cardData} {selected} {onSelectionChange} selectable={canManage} />
