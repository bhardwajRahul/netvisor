<script lang="ts">
	import { Check } from 'lucide-svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { TypedTypeMetadata, CredentialTypeMetadata } from '$lib/shared/stores/metadata';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { common_integrations } from '$lib/paraglide/messages';

	type CredType = TypedTypeMetadata<CredentialTypeMetadata>;

	interface Props {
		/** Selected integration cards. Configurable types prefill the wizard; auto-local
		 *  types (e.g. Docker socket) map to a daemon install flag. */
		selectedTypeIds: string[];
	}

	let { selectedTypeIds = $bindable([]) }: Props = $props();

	// One flat list of cards: every user-selectable type plus the auto-local
	// capabilities (Docker socket), so all integration options look the same.
	let cards = $derived(
		credentialTypes
			.getItems()
			.filter(
				(t: CredType) => t.metadata?.is_user_selectable !== false || t.metadata?.is_local_auto
			)
	);

	function toggleType(id: string) {
		selectedTypeIds = selectedTypeIds.includes(id)
			? selectedTypeIds.filter((x) => x !== id)
			: [...selectedTypeIds, id];
	}
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-auto p-4 sm:p-6">
	<h3 class="text-primary mb-4 text-lg font-medium">{common_integrations()}</h3>

	<div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
		{#each cards as type (type.id)}
			{@const selected = selectedTypeIds.includes(type.id)}
			<button
				type="button"
				onclick={() => toggleType(type.id)}
				aria-pressed={selected}
				class="card relative rounded-lg border p-3 pr-9 text-left transition-all
					{selected ? 'ring-2 ring-blue-500' : 'hover:border-gray-400 dark:hover:border-gray-500'}"
			>
				{#if selected}
					<div
						class="absolute right-2 top-2 flex h-5 w-5 items-center justify-center rounded-full bg-blue-600 text-white"
					>
						<Check class="h-3.5 w-3.5" />
					</div>
				{/if}
				<ListSelectItem
					item={type}
					displayComponent={CredentialTypeDisplay}
					context={{}}
					staticTags={true}
				/>
			</button>
		{/each}
	</div>
</div>
