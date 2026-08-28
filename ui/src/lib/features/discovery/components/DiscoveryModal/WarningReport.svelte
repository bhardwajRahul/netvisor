<!--
	A completed run's warnings, grouped by what they ask of the reader.

	This replaced a single amber callout holding one bulleted paragraph per warning. Fourteen of
	them, in emission order, with a broken credential that needs re-entering sitting at the same
	weight as a device quirk nobody can act on — and four near-identical paragraphs about the same
	SNMP view on four different switches. Nothing was wrong with the sentences; they had nowhere to
	sit.

	Two moves fix it, and they are separate.

	**The cards are the instruction.** `WarningRemedy` on the backend files every code under one of
	four rungs, and the card a row appears in is what the reader is being asked to do about it.
	Severity is deliberately not the cut: it measures what the scan lost, which is a different
	question, and it puts a thirty-second credential fix at the same weight as an LLDP table that
	will never parse. It stays on the row as the icon and colour, which is what it was written for.

	**A row is one code, and inside it one sentence per distinct statement.** The wall of text came
	from every occurrence carrying its own copy of the explanation. Four switches restricting the
	same view are now one row and one sentence naming four devices; a fifth that restricted a
	different capability keeps a sentence of its own. `warnings.ts` decides that generically, by
	comparing the slots that are not aggregates.

	The nesting is drawn, not implied. Two levels of `└─` say which row belongs to which card and
	which explanation belongs to which row, because a bare left margin stops reading as hierarchy
	the moment a row's chips wrap onto a second line.
-->
<script lang="ts">
	import { ChevronRight } from 'lucide-svelte';
	import { SvelteSet } from 'svelte/reactivity';

	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { entities } from '$lib/shared/stores/metadata';
	import { createColorHelper, createIconComponent } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import {
		common_credentials,
		common_warnings,
		discovery_noWarnings,
		discovery_noWarningsSubtitle,
		discovery_scanSettings,
		subnets_resolveRange
	} from '$lib/paraglide/messages';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { buildWarningReport, type WarningEntry } from '../../utils/warnings';

	interface Props {
		payload: DiscoveryUpdatePayload;
	}

	let { payload }: Props = $props();

	let warnings = $derived(payload.warnings ?? []);

	/**
	 * The devices a warning names, fetched by id.
	 *
	 * Only the LLDP/CDP resolution warnings carry one — everything else carries an address, which
	 * this deliberately does not try to resolve back to a host: there is no address index on the
	 * hosts API, and buying names for those rows means downloading every host on the network with
	 * its nested children to serve one modal.
	 */
	let neededHostIds = $derived([
		...new Set(
			warnings.flatMap((w) => [
				...('host_id' in w ? [w.host_id] : []),
				...('remote_host_id' in w ? [w.remote_host_id] : [])
			])
		)
	]);
	const hostsQuery = useHostsByIds(() => neededHostIds);
	let hostsData = $derived(hostsQuery.data ?? []);
	let hostNameById = $derived((id: string) => hostsData.find((h) => h.id === id)?.name);

	let sections = $derived(buildWarningReport(warnings, hostNameById));

	/** Which rows are showing their explanation. Keyed by code, which is unique within a run. */
	const openRows = new SvelteSet<string>();

	function toggleRow(code: string) {
		if (!openRows.delete(code)) openRows.add(code);
	}

	/** The codes whose answer is a scan setting rather than a credential. */
	const SCAN_SETTING_CODES = new Set<WarningEntry['code']>([
		'ScanTimeLimit',
		'ScanTimeLimitWithEstimate',
		'WarningsTruncated'
	]);

	/**
	 * Whether the run still has a scan to send the reader to.
	 *
	 * The historical row carries no parent, but the payload does. It is absent on records written
	 * before the field existed, and it dangles for a rescan — that discovery is deleted the moment
	 * the run finishes.
	 */
	let scanSettingsTarget = $derived(
		payload.discovery_type.type === 'Rescan' ? null : (payload.discovery_id ?? null)
	);

	/**
	 * Where a row hands off to, when Scanopy owns the fix.
	 *
	 * Only three destinations are real, and a row without one renders no button rather than a dead
	 * one. The credential family is recognised by the `integration` field on the payload rather
	 * than by a list of code names here — those eleven variants are exactly the ones that carry it,
	 * so the set cannot drift out of step with the backend.
	 */
	function actionFor(
		entry: WarningEntry
	): { label: string; icon: IconComponent; run: () => void } | null {
		if (entry.code === 'ProvisionalSubnetInferred') {
			const subnetIds = entry.warnings.flatMap((w) => ('subnet_id' in w ? [w.subnet_id] : []));
			return {
				label: subnets_resolveRange(),
				icon: createIconComponent('cloud-alert'),
				run: () => {
					window.location.hash = 'subnets';
					// One range resolves in place; several is a trip to the list, because the
					// four-way choice is made per subnet and there is no "all of them" answer.
					if (subnetIds.length === 1) openModal('provisional-range', { id: subnetIds[0] });
				}
			};
		}

		const scanId = scanSettingsTarget;
		if (scanId && SCAN_SETTING_CODES.has(entry.code)) {
			return {
				label: discovery_scanSettings(),
				icon: createIconComponent('sliders-horizontal'),
				run: () => {
					window.location.hash = 'discovery-scans';
					// Max Discovery Duration lives on Detection, not Performance.
					openModal('discovery-editor', { id: scanId, tab: 'detection' });
				}
			};
		}

		if (entry.warnings.some((w) => 'integration' in w)) {
			return {
				label: common_credentials(),
				icon: createIconComponent('key-round'),
				run: () => {
					// The list, not a create-mode editor: the credential exists and is wrong, and
					// the warning carries no id to open it by.
					window.location.hash = 'credentials';
				}
			};
		}

		return null;
	}

	/** A named device is a real entity, so its chip is drawn the way one is drawn everywhere. */
	const HostIcon = entities.getIconComponent('Host');
	const hostColor = entities.getColorHelper('Host').color;
</script>

{#if sections.length === 0}
	<EmptyState title={discovery_noWarnings()} subtitle={discovery_noWarningsSubtitle()} />
{:else}
	<div class="space-y-4" aria-label={common_warnings()}>
		{#each sections as section (section.remedy)}
			<CollapsibleCard
				title={section.title}
				description={section.description}
				icon={createIconComponent(section.icon)}
			>
				{#snippet badge()}
					<Tag label={String(section.entries.length)} color="Amber" pill />
				{/snippet}

				<ul>
					{#each section.entries as entry (entry.code)}
						{@const EntryIcon = createIconComponent(entry.icon)}
						{@const colors = createColorHelper(entry.color)}
						{@const action = actionFor(entry)}
						{@const isOpen = openRows.has(entry.code)}
						<li class="flex items-start gap-2">
							<!-- The branch into this row, from the card that heads it. -->
							<span
								class="text-tertiary/60 mt-1.5 shrink-0 select-none font-mono text-xs leading-5"
								aria-hidden="true">└─</span
							>
							<div class="min-w-0 flex-1">
								<div class="flex items-start gap-2">
									<button
										type="button"
										class="hover:bg-tertiary/40 -mx-1 flex min-w-0 flex-1 cursor-pointer items-start gap-2 rounded px-1 py-1 text-left"
										aria-expanded={isOpen}
										onclick={() => toggleRow(entry.code)}
									>
										<ChevronRight
											class="text-secondary mt-0.5 h-4 w-4 shrink-0 transition-transform {isOpen
												? 'rotate-90'
												: ''}"
										/>
										<EntryIcon class="mt-0.5 h-4 w-4 shrink-0 {colors.icon}" />
										<span class="text-primary shrink-0 text-sm">{entry.title}</span>
										{#if entry.subjects.length > 0}
											<span class="flex flex-wrap items-center gap-1">
												{#each entry.subjects as subject (subject.label)}
													{#if subject.hostId}
														<EntityTag
															entityRef={entityRef('Host', subject.hostId, {
																id: subject.hostId,
																name: subject.label
															})}
															label={subject.label}
															icon={HostIcon}
															color={hostColor}
														/>
													{:else}
														<!-- No entity to point at, so no colour and no hover state:
														     an address here is a label, not a link. -->
														<Tag label={subject.label} color={null} />
													{/if}
												{/each}
											</span>
										{/if}
									</button>
									{#if action}
										{@const ActionIcon = action.icon}
										<button
											type="button"
											class="btn-secondary flex shrink-0 items-center gap-1 py-1 text-xs"
											onclick={action.run}
										>
											<ActionIcon class="h-3.5 w-3.5" />
											<span>{action.label}</span>
										</button>
									{/if}
								</div>

								{#if isOpen}
									<div class="space-y-1 pb-2">
										{#each entry.details as detail, i (i)}
											<div class="flex items-start gap-2">
												<!-- The branch into an explanation, from the row it explains. -->
												<span
													class="text-tertiary/60 shrink-0 select-none pl-5 font-mono text-xs leading-5"
													aria-hidden="true">└─</span
												>
												<p class="text-tertiary min-w-0 text-sm">{detail}</p>
											</div>
										{/each}
									</div>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			</CollapsibleCard>
		{/each}
	</div>
{/if}
