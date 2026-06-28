<script lang="ts">
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { TypedTypeMetadata, CredentialTypeMetadata } from '$lib/shared/stores/metadata';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import {
		daemons_integrationsSubtitle,
		credentials_lockedDaemonCapability
	} from '$lib/paraglide/messages';

	type CredType = TypedTypeMetadata<CredentialTypeMetadata>;

	interface Props {
		/** Selected integration cards. Configurable types prefill the wizard; auto-local
		 *  types (e.g. Docker socket) map to a daemon install flag. */
		selectedTypeIds: string[];
		/** Type ids rendered read-only (non-toggleable), reflecting a fixed daemon
		 *  capability — e.g. an already-installed daemon's local Docker socket. */
		lockedTypeIds?: string[];
		/** Type ids always rendered checked, independent of `selectedTypeIds` (used
		 *  for locked cards reflecting a fixed capability). */
		forceCheckedTypeIds?: string[];
	}

	let {
		selectedTypeIds = $bindable([]),
		lockedTypeIds = [],
		forceCheckedTypeIds = []
	}: Props = $props();

	// One flat list of cards: every user-selectable type plus the auto-local
	// capabilities (Docker socket), so all integration options look the same.
	let cards = $derived(
		credentialTypes
			.getItems()
			.filter(
				(t: CredType) => t.metadata?.is_user_selectable !== false || t.metadata?.is_local_auto
			)
	);

	// Group cards by their integration (the backend `associated_service`, e.g. SNMP / Docker /
	// Podman), preserving first-appearance order, so the grid breaks between integrations for
	// legibility. No section headers — just a clear gap between groups.
	let cardGroups = $derived.by(() => {
		const groups: { key: string; cards: CredType[] }[] = [];
		for (const card of cards) {
			const key = card.metadata?.associated_service ?? '';
			let group = groups.find((g) => g.key === key);
			if (!group) {
				group = { key, cards: [] };
				groups.push(group);
			}
			group.cards.push(card);
		}
		return groups;
	});

	function isLocked(id: string): boolean {
		return lockedTypeIds.includes(id);
	}

	function toggleType(id: string) {
		if (isLocked(id)) return;
		selectedTypeIds = selectedTypeIds.includes(id)
			? selectedTypeIds.filter((x) => x !== id)
			: [...selectedTypeIds, id];
	}
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-auto p-4 sm:p-6">
	<p class="text-secondary mb-4 text-sm">{daemons_integrationsSubtitle()}</p>

	<!-- One grid per integration (SNMP, Docker, Podman, …) so each starts on its own row,
	     with a wider gap between groups than within a group for a clear visual break. -->
	<div class="flex flex-col gap-6">
		{#each cardGroups as group (group.key)}
			<div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
				{#each group.cards as type (type.id)}
					{@const selected =
						selectedTypeIds.includes(type.id) || forceCheckedTypeIds.includes(type.id)}
					{@const locked = isLocked(type.id)}
					<!-- Wrapper is the grid item; it carries the tooltip so a disabled (locked)
					     card still shows the reason on hover. -->
					<span
						class="block"
						data-tooltip={locked
							? credentials_lockedDaemonCapability({
									integration: type.metadata?.associated_service ?? ''
								})
							: undefined}
						use:tooltip
					>
						<button
							type="button"
							onclick={() => toggleType(type.id)}
							aria-pressed={selected}
							disabled={locked}
							class="card w-full rounded-lg border p-3 text-left {locked
								? 'cursor-not-allowed opacity-60'
								: ''}"
							class:card-selected={selected}
						>
							<ListSelectItem
								item={type}
								displayComponent={CredentialTypeDisplay}
								context={{}}
								staticTags={true}
							/>
						</button>
					</span>
				{/each}
			</div>
		{/each}
	</div>
</div>
