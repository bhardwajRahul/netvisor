<script lang="ts">
	import { Check } from 'lucide-svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { TypedTypeMetadata, CredentialTypeMetadata } from '$lib/shared/stores/metadata';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import {
		common_integrations,
		daemons_credentialTypeSelectSubtitle,
		daemons_integrationAutoLocalHelp,
		daemons_integrationAutoLocalLabel,
		daemons_integrationOtherTargets
	} from '$lib/paraglide/messages';

	type CredType = TypedTypeMetadata<CredentialTypeMetadata>;

	interface Props {
		/** Configurable credential types selected to prefill the wizard. */
		selectedTypeIds: string[];
		/** Auto-local capability type id → enabled (default on). Maps to a daemon flag. */
		localCapabilityEnabled: Record<string, boolean>;
	}

	let { selectedTypeIds = $bindable([]), localCapabilityEnabled = $bindable({}) }: Props = $props();

	// Group credential types by their integration (associated_service); within each,
	// split into auto-local capabilities (rendered as toggles) and configurable
	// credentials (rendered as selectable tiles).
	let integrations = $derived.by(() => {
		const order: string[] = [];
		const groups: Record<string, { name: string; auto: CredType[]; config: CredType[] }> = {};
		for (const t of credentialTypes.getItems()) {
			const name = t.metadata?.associated_service ?? t.name ?? t.id;
			if (!groups[name]) {
				groups[name] = { name, auto: [], config: [] };
				order.push(name);
			}
			const g = groups[name];
			if (t.metadata?.is_local_auto) g.auto.push(t);
			else if (t.metadata?.is_user_selectable !== false) g.config.push(t);
		}
		return order.map((n) => groups[n]).filter((g) => g.auto.length > 0 || g.config.length > 0);
	});

	function toggleType(id: string) {
		selectedTypeIds = selectedTypeIds.includes(id)
			? selectedTypeIds.filter((x) => x !== id)
			: [...selectedTypeIds, id];
	}

	function isLocalEnabled(id: string): boolean {
		return localCapabilityEnabled[id] !== false; // default on
	}

	function setLocalEnabled(id: string, enabled: boolean) {
		localCapabilityEnabled = { ...localCapabilityEnabled, [id]: enabled };
	}
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-auto p-4 sm:p-6">
	<div class="mb-4">
		<h3 class="text-primary text-lg font-medium">{common_integrations()}</h3>
		<p class="text-secondary mt-1 text-sm">{daemons_credentialTypeSelectSubtitle()}</p>
	</div>

	<div class="space-y-4">
		{#each integrations as integration (integration.name)}
			{@const Icon = credentialTypes.getIconComponent(
				(integration.config[0] ?? integration.auto[0])?.id ?? null
			)}
			<div class="card rounded-lg border p-4">
				<div class="text-primary mb-3 flex items-center gap-2 text-sm font-semibold">
					<Icon class="h-5 w-5" />
					{integration.name}
				</div>

				<!-- Auto-local capabilities → on/off toggle (daemon flag, not a credential) -->
				{#each integration.auto as autoType (autoType.id)}
					<label class="mb-3 flex items-start gap-2">
						<input
							type="checkbox"
							class="checkbox-card mt-0.5 h-4 w-4 focus:ring-1 focus:ring-blue-500"
							checked={isLocalEnabled(autoType.id)}
							onchange={(e) => setLocalEnabled(autoType.id, e.currentTarget.checked)}
						/>
						<span class="min-w-0">
							<span class="text-primary text-sm"
								>{daemons_integrationAutoLocalLabel({ integration: integration.name })}</span
							>
							<span class="text-tertiary block text-xs">{daemons_integrationAutoLocalHelp()}</span>
						</span>
					</label>
				{/each}

				<!-- Configurable credentials → selectable tiles (prefill the wizard) -->
				{#if integration.config.length > 0}
					{#if integration.auto.length > 0}
						<p class="text-tertiary mb-2 text-xs">
							{daemons_integrationOtherTargets({ integration: integration.name })}
						</p>
					{/if}
					<div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
						{#each integration.config as type (type.id)}
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
				{/if}
			</div>
		{/each}
	</div>
</div>
