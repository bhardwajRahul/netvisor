<script lang="ts" module>
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { TypedTypeMetadata, CredentialTypeMetadata } from '$lib/shared/stores/metadata';
	import {
		getStabilityTagProps,
		getTargetTagProps,
		getUpstreamSupportTagProps
	} from '$lib/features/credentials/types/base';

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
		// Beta leads, then the unofficial-API marker, then the target tags. This is the one place
		// credential-type tags are built, so it covers the wizard card grid, the type dropdown,
		// and dropdown search (RichSelect folds tag labels into its filter text) in a single
		// definition. Beta and unofficial are independent, so a type may carry both.
		getTags: (item) => {
			const stability = getStabilityTagProps(item.metadata?.stability);
			const upstream = getUpstreamSupportTagProps(item.metadata?.upstream_support);
			const targets = (item.metadata?.targets ?? []).map((t: string) => getTargetTagProps(t));
			return [stability, upstream, ...targets].filter((tag) => tag !== null);
		},
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
