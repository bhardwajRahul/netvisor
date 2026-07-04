<script lang="ts">
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { TypedTypeMetadata, CredentialTypeMetadata } from '$lib/shared/stores/metadata';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { daemonTooOldForCredential } from '$lib/features/credentials/utils/versionGate';
	import {
		daemons_integrationsSubtitle,
		credentials_lockedDaemonCapability,
		credentials_requiresDaemonVersion
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
		/** Version of the single daemon this picker targets (the discovery modal's bound
		 *  daemon). A card is disabled when this version is older than the credential
		 *  type's `minimum_daemon_version`. `null`/absent (e.g. create-daemon flow, where
		 *  no daemon is connected yet) ⇒ no version gate. Assignment surfaces that span
		 *  many daemons don't pass this — those are handled by the backend dispatch filter. */
		daemonVersion?: string | null;
		/** Name of that daemon, used in the version-requirement tooltip. */
		daemonName?: string | null;
	}

	let {
		selectedTypeIds = $bindable([]),
		lockedTypeIds = [],
		forceCheckedTypeIds = [],
		daemonVersion = null,
		daemonName = null
	}: Props = $props();

	// One flat list of cards: every user-selectable type plus the auto-local
	// capabilities (Docker socket), so all integration options look the same.
	// Every credential type is user-selectable now (sockets included), so no filtering.
	let cards = $derived(credentialTypes.getItems());

	// Rank a type by how far its applicable targets reach: daemon-only first (0), host (1),
	// network-applicable last (2). Drives the daemon→network ordering below.
	function targetRank(card: CredType): number {
		const targets = card.metadata?.targets ?? [];
		if (targets.includes('Network')) return 2;
		if (targets.includes('Hosts')) return 1;
		return 0;
	}

	// Group cards by their integration (the backend `associated_service`, e.g. SNMP / Docker /
	// Podman) so the grid breaks between integrations for legibility — no section headers, just a
	// clear gap. Then order by applicable targets: daemon-only integrations first, any that apply
	// to the network last. Within a group, daemon-only types precede network-applicable ones.
	// Sorts are stable, so original order is preserved on ties.
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
		for (const group of groups) {
			group.cards.sort((a, b) => targetRank(a) - targetRank(b));
		}
		const groupRank = (g: { cards: CredType[] }) => Math.min(...g.cards.map(targetRank));
		groups.sort((a, b) => groupRank(a) - groupRank(b));
		return groups;
	});

	function isLocked(id: string): boolean {
		return lockedTypeIds.includes(id);
	}

	// The target daemon is too old for this credential type when its version is below
	// the type's `minimum_daemon_version` floor.
	function isIncompatible(type: CredType): boolean {
		return daemonTooOldForCredential(type.metadata?.minimum_daemon_version, daemonVersion);
	}

	function isDisabled(type: CredType): boolean {
		return isLocked(type.id) || isIncompatible(type);
	}

	// Disabled reason for the hover tooltip: version-incompatibility takes
	// precedence over a fixed-capability lock (a too-old daemon can't run it at all).
	function disabledReason(type: CredType): string | undefined {
		if (isIncompatible(type)) {
			return credentials_requiresDaemonVersion({
				version: type.metadata?.minimum_daemon_version ?? '',
				name: daemonName ?? ''
			});
		}
		if (isLocked(type.id)) {
			return credentials_lockedDaemonCapability({
				integration: type.metadata?.associated_service ?? ''
			});
		}
		return undefined;
	}

	function toggleType(type: CredType) {
		if (isDisabled(type)) return;
		const id = type.id;
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
					{@const locked = isDisabled(type)}
					<!-- Wrapper is the grid item; it carries the tooltip so a disabled (locked
					     or version-incompatible) card still shows the reason on hover. -->
					<span class="block" data-tooltip={disabledReason(type)} use:tooltip>
						<button
							type="button"
							onclick={() => toggleType(type)}
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
