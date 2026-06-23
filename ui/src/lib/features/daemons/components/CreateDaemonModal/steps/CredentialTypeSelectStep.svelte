<script lang="ts">
	import { Check } from 'lucide-svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { getScopeTagProps } from '$lib/features/credentials/types/base';
	import Tag from '$lib/shared/components/data/Tag.svelte';
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

	<div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
		{#each typeOptions as type (type.id)}
			{@const Icon = credentialTypes.getIconComponent(type.id)}
			{@const iconColor = credentialTypes.getColorHelper(type.id).icon}
			{@const selected = selectedTypeIds.includes(type.id)}
			{@const scopeTags = (type.metadata?.scope_models ?? []).map((s) => getScopeTagProps(s))}
			<button
				type="button"
				onclick={() => toggle(type.id)}
				aria-pressed={selected}
				class="card relative flex flex-col items-start gap-2 rounded-lg border p-4 text-left transition-all
					{selected ? 'ring-2 ring-blue-500' : 'hover:border-gray-400 dark:hover:border-gray-500'}"
			>
				{#if selected}
					<div
						class="absolute right-2 top-2 flex h-5 w-5 items-center justify-center rounded-full bg-blue-600 text-white"
					>
						<Check class="h-3.5 w-3.5" />
					</div>
				{/if}
				<Icon class="h-6 w-6 {iconColor}" />
				<span class="text-primary text-sm font-medium">{credentialTypes.getName(type.id)}</span>
				{#if scopeTags.length > 0}
					<div class="flex flex-wrap gap-1">
						{#each scopeTags as tag (tag.label)}
							<Tag label={tag.label} color={tag.color} title={tag.title} />
						{/each}
					</div>
				{/if}
			</button>
		{/each}
	</div>
</div>
