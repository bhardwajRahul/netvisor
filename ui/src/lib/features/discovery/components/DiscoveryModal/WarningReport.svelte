<!--
	A completed run's warnings, grouped by what they ask of the reader.

	This replaced a single amber callout holding one bulleted paragraph per warning. Fourteen of
	them, in emission order, with a broken credential that needs re-entering sitting at the same
	weight as a device quirk nobody can act on — and four near-identical paragraphs about the same
	SNMP view on four different switches. Nothing was wrong with the sentences; they had nowhere to
	sit.

	Two moves fix it, and they are separate.

	**The sections are the instruction.** `WarningRemedy` on the backend files every code under one
	of four rungs, and the heading a row appears under is what the reader is being asked to do about
	it. Severity is deliberately not the cut: it measures what the scan lost, which is a different
	question, and it puts a thirty-second credential fix at the same weight as an LLDP table that
	will never parse. It stays on the row as the icon and colour, which is what it was written for.

	**A row is one code, not one occurrence.** The wall of text came from every occurrence carrying
	its own copy of the explanation. Here the four switches restricting the same view are one row
	naming four devices, with all four sentences intact behind its disclosure. Nothing the backend
	kept apart is merged — `PER_OCCURRENCE` still decides how many sentences a row holds — and
	nothing is thrown away.

	The last two sections start closed. They are the ones whose answer is "nothing", and a reader
	who wants them can say so.
-->
<script lang="ts">
	import { ChevronRight } from 'lucide-svelte';

	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { openModal } from '$lib/shared/stores/modal-registry';
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

	/** The rungs that open on arrival: the ones with something for the reader to do. */
	const OPEN_BY_DEFAULT = new Set(['FixInScanopy', 'CheckTheDevice']);

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
</script>

{#if sections.length === 0}
	<EmptyState title={discovery_noWarnings()} subtitle={discovery_noWarningsSubtitle()} />
{:else}
	<div class="space-y-6" aria-label={common_warnings()}>
		{#each sections as section (section.remedy)}
			{@const SectionIcon = createIconComponent(section.icon)}
			<details class="group" open={OPEN_BY_DEFAULT.has(section.remedy)}>
				<summary class="flex cursor-pointer list-none items-center gap-2">
					<ChevronRight
						class="text-tertiary h-4 w-4 shrink-0 transition-transform group-open:rotate-90"
					/>
					<SectionIcon class="text-secondary h-4 w-4 shrink-0" />
					<span class="text-primary font-medium">{section.title}</span>
					<span class="text-tertiary text-sm">{section.entries.length}</span>
				</summary>

				<p class="text-tertiary ml-10 mt-1 text-sm">{section.description}</p>

				<div class="ml-10 mt-2 space-y-1">
					{#each section.entries as entry (entry.code)}
						{@const EntryIcon = createIconComponent(entry.icon)}
						{@const colors = createColorHelper(entry.color)}
						{@const action = actionFor(entry)}
						<div class="flex items-start gap-2">
							<details class="group/entry min-w-0 flex-1">
								<summary
									class="flex cursor-pointer list-none items-start gap-2 rounded px-1 py-1 hover:bg-gray-100 dark:hover:bg-gray-800"
								>
									<EntryIcon class="mt-0.5 h-4 w-4 shrink-0 {colors.icon}" />
									<span class="text-primary shrink-0 text-sm">{entry.title}</span>
									{#if entry.subjects.length > 0}
										<span class="flex flex-wrap gap-1">
											{#each entry.subjects as subject (subject)}
												<Tag label={subject} color={entry.color} faded pill />
											{/each}
										</span>
									{/if}
								</summary>
								<div class="text-tertiary ml-6 mt-1 space-y-1 text-sm">
									{#each entry.details as detail, i (i)}
										<p>{detail}</p>
									{/each}
								</div>
							</details>
							{#if action}
								{@const ActionIcon = action.icon}
								<button
									type="button"
									class="btn-icon flex shrink-0 items-center gap-1 text-sm"
									onclick={action.run}
								>
									<ActionIcon class="h-4 w-4" />
									<span>{action.label}</span>
								</button>
							{/if}
						</div>
					{/each}
				</div>
			</details>
		{/each}
	</div>
{/if}
