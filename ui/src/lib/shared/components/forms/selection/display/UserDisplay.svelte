<script lang="ts" module>
	export const UserDisplay: EntityDisplayComponent<User, object> = {
		getId: (user) => user.id,
		// The User type carries no name or avatar — email is the identity everywhere in the UI.
		getLabel: (user) => user.email,
		getIcon: () => entities.getIconComponent('User'),
		getIconColor: () => entities.getColorHelper('User').icon,
		getTags: (user) => [
			{
				label: permissions.getName(user.permissions),
				color: permissions.getColorHelper(user.permissions).color
			}
		],
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { User } from '$lib/features/users/types';
	import { entities, permissions } from '$lib/shared/stores/metadata';

	interface Props {
		item: User;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={UserDisplay} />
