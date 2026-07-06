<script lang="ts" module>
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { TypedTypeMetadata, CredentialTypeMetadata } from '$lib/shared/stores/metadata';
	import { getTargetTagProps } from '$lib/features/credentials/types/base';

	export type CredentialTypeOption = TypedTypeMetadata<CredentialTypeMetadata>;

	/** Optional context for the dropdown: a non-null `disabledReason` renders the
	 *  option as disabled (unselectable) with the reason as a hover tooltip. */
	export type CredentialTypeDisplayContext = { disabledReason?: string | null };

	export const CredentialTypeDisplay: EntityDisplayComponent<
		CredentialTypeOption,
		CredentialTypeDisplayContext
	> = {
		getId: (item) => item.id,
		getLabel: (item) => credentialTypes.getName(item.id),
		getDescription: (item) => credentialTypes.getDescription(item.id),
		getIcon: (item) => credentialTypes.getIconComponent(item.id),
		getIconColor: (item) => credentialTypes.getColorHelper(item.id).icon,
		getCategory: (item) => item.category ?? null,
		getTags: (item) => (item.metadata?.targets ?? []).map((t: string) => getTargetTagProps(t)),
		getDisabled: (_item, context) => !!context?.disabledReason,
		getDisabledReason: (_item, context) => context?.disabledReason ?? null
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	interface Props {
		item: CredentialTypeOption;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={CredentialTypeDisplay} />
