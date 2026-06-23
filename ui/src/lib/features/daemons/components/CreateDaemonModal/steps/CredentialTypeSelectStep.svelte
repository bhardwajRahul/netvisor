<script lang="ts">
	import { Check } from 'lucide-svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import {
		daemons_credentialTypeSelectTitle,
		daemons_credentialTypeSelectSubtitle
	} from '$lib/paraglide/messages';

	interface Props {
		selectedTypeIds: string[];
	}

	let { selectedTypeIds = $bindable([]) }: Props = $props();

	let typeOptions = $derived(
		credentialTypes.getItems().filter((t) => t.metadata?.is_user_selectable !== false)
	);

	function toggle(id: string) {
		selectedTypeIds = selectedTypeIds.includes(id)
			? selectedTypeIds.filter((x) => x !== id)
			: [...selectedTypeIds, id];
	}
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-auto p-4 sm:p-6">
	<div class="mb-4">
		<h3 class="text-primary text-lg font-medium">{daemons_credentialTypeSelectTitle()}</h3>
		<p class="text-secondary mt-1 text-sm">{daemons_credentialTypeSelectSubtitle()}</p>
	</div>

	<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
		{#each typeOptions as type (type.id)}
			{@const selected = selectedTypeIds.includes(type.id)}
			<button
				type="button"
				onclick={() => toggle(type.id)}
				aria-pressed={selected}
				class="card relative rounded-lg border p-4 pr-9 text-left transition-all
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
