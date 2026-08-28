/**
 * Rendering coded scan warnings into the sentences an operator reads.
 *
 * The daemon and the server both record one warning per occurrence — one device, one neighbour,
 * one credential attempt — because that is what makes them countable and what keeps each address's
 * own diagnostic attached to it. Grouping them back into readable prose is this module's job, and
 * it belongs here rather than on the backend for one reason: `Intl.ListFormat` joins a list in the
 * reader's own language, where the Rust helper it replaced only ever spoke English.
 *
 * Each code's sentence is a template with `{named}` slots (see `warning-codes.json`), and the
 * renderer below supplies exactly the slots that fixture declares. `WARNING_PARAMS` is typed
 * `satisfies Record<DiscoveryWarningCode, …>`, so TypeScript will not compile a new backend code
 * without a renderer for it.
 */

import { getLocale } from '$lib/paraglide/runtime';
import type { components } from '$lib/api/schema';
import claimSources from '$lib/data/claim-sources.json';
import discoveryIntegrations from '$lib/data/discovery-integrations.json';
import malformedNeighbourConsequences from '$lib/data/malformed-neighbour-consequences.json';
import snmpWalkGroups from '$lib/data/snmp-walk-groups.json';
import warningCodes from '$lib/data/warning-codes.json';
import warningRemedies from '$lib/data/warning-remedies.json';
import { metaDescription, metaDescriptionWith, metaName } from '$lib/i18n/metadata';
import { toColor, type Color } from '$lib/shared/utils/styling';
import {
	common_andNMore,
	common_moreItems,
	discovery_warningNoFurtherDetail,
	discovery_warningAtAddress,
	discovery_warningInferredFrom,
	discovery_warningSeenBy,
	discovery_warningNoPortId
} from '$lib/paraglide/messages';

export type DiscoveryWarning = components['schemas']['DiscoveryWarning'];
export type DiscoveryWarningCode = DiscoveryWarning['code'];

/**
 * What a row is about: a device, a range, or a bare address.
 *
 * `hostId` is present only where the warning carried one, which is the LLDP/CDP resolution family.
 * Those render as entity tags that navigate to the device; an address on its own has no entity to
 * point at and renders as plain text.
 */
export interface WarningSubject {
	label: string;
	hostId?: string;
}

/** One warning code's worth of a run, as a row in the report. */
export interface WarningEntry {
	code: DiscoveryWarningCode;
	/** The code's short name — "Credential incomplete" — which is what the row is headed by. */
	title: string;
	/** Severity, as the fixture already publishes it. */
	color: Color;
	icon: string;
	/** The devices, ranges or addresses this row concerns. */
	subjects: WarningSubject[];
	/** The full sentences, shown on disclosure: one per group, or one per occurrence. */
	details: string[];
	/** Every occurrence behind the row, so an action can read ids off the payload. */
	warnings: DiscoveryWarning[];
}

/** One rung of the report: what the reader has to do about everything under it. */
export interface WarningSection {
	/** The `WarningRemedy` id, which is what an action keys off. */
	remedy: string;
	title: string;
	description: string;
	icon: string;
	entries: WarningEntry[];
}

/** How many items a list names before eliding the rest. A line long enough to scroll is not read. */
const MAX_LISTED = 10;

type Params = Record<string, string | number>;
type WarningOf<C extends DiscoveryWarningCode> = Extract<DiscoveryWarning, { code: C }>;

/**
 * Resolve a host id to its name, or `undefined` when it cannot be resolved.
 *
 * Passed in rather than queried here: the warnings carry ids, and the component already holds the
 * host query that turns them into names. Returning `undefined` is a normal outcome — the query may
 * still be in flight, and a historical scan can name a host that has since been deleted.
 */
export type HostNameLookup = (hostId: string) => string | undefined;

/** No names available: every host segment is omitted, which is how this rendered before names. */
const NO_HOST_NAMES: HostNameLookup = () => undefined;

/** Metadata lookup for a slot value, falling back to the fixture's own English. */
function nameOf(fixture: { id: string; name: string | null }[], fixtureKey: string, id: string) {
	const entry = fixture.find((item) => item.id === id);
	return metaName(fixtureKey, id, entry?.name ?? id);
}

const group = (id: string) => nameOf(snmpWalkGroups, 'snmp_walk_groups', id);
const claimSource = (id: string) => nameOf(claimSources, 'claim_sources', id);
const consequence = (id: string) =>
	nameOf(malformedNeighbourConsequences, 'malformed_neighbour_consequences', id);

/**
 * The integration's display name.
 *
 * Keyed by `CredentialQueryPayloadDiscriminants`, which is what the warning carries. Not
 * `credential-types.json`: that is keyed by `CredentialType` (SnmpV1/V2c/V3, UnifiApiKey, …), so
 * `DockerProxy` resolved there by coincidence while `Snmp`, `UnifiController` and `InstantOn` fell
 * through and rendered as their raw discriminants.
 */
const integration = (id: string) => nameOf(discoveryIntegrations, 'discovery_integrations', id);

/**
 * Join as a localized list, capped, saying how many were left out.
 *
 * `conjunction` for things that are all true at once ("A, B, and C stopped responding"),
 * `disjunction` for the alternatives a single failure applies to ("LLDP neighbours or the ARP
 * table"). That is the same and/or split the Rust joiners made, now made by `Intl`.
 */
function joinList(values: string[], type: 'conjunction' | 'disjunction'): string {
	const unique = [...new Set(values)];
	const listed = unique.slice(0, MAX_LISTED);
	const elided = unique.length - listed.length;
	if (elided > 0) {
		// A capped list that simply stops reads as though that was all of them.
		listed.push(common_andNMore({ count: elided }));
	}
	return new Intl.ListFormat(getLocale(), { style: 'long', type }).format(listed);
}

const addressesOf = (group_: { address: string }[]) =>
	joinList(
		group_.map((w) => w.address),
		'conjunction'
	);

const groupsOf = (group_: { group: string }[]) =>
	joinList(
		group_.map((w) => group(w.group)),
		'disjunction'
	);

const sum = (values: number[]) => values.reduce((total, n) => total + n, 0);

/**
 * Every code's slot values, built from the warnings sharing that code.
 *
 * `satisfies` rather than a plain annotation so the object keeps its precise type while still being
 * checked for completeness — a backend code with no entry here is a compile error, which is the
 * TypeScript half of the agreement the Rust `slots()` test enforces on the other side.
 */
const WARNING_PARAMS = {
	InterfaceSetCutShort: (w) => ({ addresses: addressesOf(w) }),
	InterfaceDetailsCutShort: (w) => ({ addresses: addressesOf(w) }),

	SnmpWalkEntryCap: (w) => ({
		addresses: addressesOf(w),
		groups: groupsOf(w),
		limit: w[0].limit
	}),
	SnmpWalkUnsupported: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkDesynchronised: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkPartialDiscarded: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkPartialRecorded: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkBridgeMibAbsent: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkNoAnswer: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),

	// The figures are the substance of a claim warning, so two switches disagreeing with themselves
	// by different amounts do not share a sentence — the bucketing below keeps them apart, and
	// merges the ones that do agree.
	ClaimedCountReadCutShort: (w) => ({
		addresses: addressesOf(w),
		source: claimSource(w[0].source),
		expected: w[0].expected,
		observed: w[0].observed
	}),
	ClaimedCountUnderRead: (w) => ({
		addresses: addressesOf(w),
		source: claimSource(w[0].source),
		expected: w[0].expected,
		observed: w[0].observed
	}),
	ClaimedCapabilityReadCutShort: (w) => ({
		addresses: addressesOf(w),
		source: claimSource(w[0].source),
		group: group(w[0].group)
	}),
	ClaimedCapabilityEmpty: (w) => ({
		addresses: addressesOf(w),
		source: claimSource(w[0].source),
		group: group(w[0].group)
	}),

	LldpLocalPortDropped: droppedPortParams,
	LldpLocalPortDroppedReadCutShort: droppedPortParams,
	LldpLocalPortMisplaced: (w) => ({
		addresses: addressesOf(w),
		misplaced: sum(w.map((x) => x.misplaced))
	}),

	MalformedNeighboursWalkCutShort: malformedParams,
	MalformedNeighboursGhostRows: malformedParams,
	MalformedNeighboursIncompleteRecords: malformedParams,
	MalformedNeighboursUnexpectedType: malformedParams,
	MalformedNeighboursUnreadableIndex: malformedParams,

	SnmpCollectedNothing: (w) => ({ addresses: addressesOf(w) }),
	VlanRecordingFailed: (w) => ({ addresses: addressesOf(w) }),

	CredentialTargetNotScanned: credentialParams,
	CredentialTargetNotResponding: credentialParams,
	CredentialGateClosed: (w) => ({
		credential: integration(w[0].integration),
		addresses: addressesOf(w),
		ports: joinList(w[0].ports.map(String), 'conjunction')
	}),
	CredentialRejected: attemptParams,
	CredentialMalformed: attemptParams,
	CredentialTlsFailed: attemptParams,
	CredentialNotThisService: attemptParams,
	CredentialCollectionFailed: attemptParams,
	CredentialCollectionTimedOut: attemptParams,
	CredentialUnreachable: attemptParams,
	CredentialTimedOut: attemptParams,

	ScanTimeLimitWithEstimate: (w) => ({
		hours: w[0].hours,
		hosts_not_scanned: w[0].hosts_not_scanned,
		minutes_remaining: w[0].minutes_remaining
	}),
	ScanTimeLimit: (w) => ({ hours: w[0].hours, hosts_not_scanned: w[0].hosts_not_scanned }),

	LldpNeighbourNotFound: neighbourParams,
	LldpNeighbourAmbiguous: neighbourParams,
	LldpPortNoStrategy: portParams,
	LldpPortNotFound: portParams,
	LldpPortAmbiguous: portParams,

	ProvisionalSubnetInferred: (w, hostName) => ({
		count: w.length,
		examples: joinList(
			w.map((x) => describeProvisionalSubnet(x, hostName)),
			'conjunction'
		)
	}),

	WarningsTruncated: (w) => ({ elided: sum(w.map((x) => x.elided)) }),
	// The whole sentence *is* the detail: a warning from another version, or one written before
	// warnings were coded at all. Rendered one per occurrence, so this only ever sees one.
	Unknown: (w) => ({ detail: w[0].detail })
} satisfies {
	[C in DiscoveryWarningCode]: (warnings: WarningOf<C>[], hostName: HostNameLookup) => Params;
};

/**
 * The two codes carry the same three slots and differ only in the sentence they fill.
 *
 * Which sentence is the whole point: one says the device numbers its LLDP ports separately from
 * its interfaces, the other says we did not finish reading that numbering. The backend decides
 * which, from whether the reads completed — offering both at once left the operator to guess.
 */
function droppedPortParams(
	w: WarningOf<'LldpLocalPortDropped' | 'LldpLocalPortDroppedReadCutShort'>[]
): Params {
	return {
		addresses: addressesOf(w),
		dropped: sum(w.map((x) => x.dropped)),
		total: sum(w.map((x) => x.total))
	};
}

function malformedParams(
	w: WarningOf<
		| 'MalformedNeighboursWalkCutShort'
		| 'MalformedNeighboursGhostRows'
		| 'MalformedNeighboursIncompleteRecords'
		| 'MalformedNeighboursUnexpectedType'
		| 'MalformedNeighboursUnreadableIndex'
	>[]
): Params {
	return {
		addresses: addressesOf(w),
		discarded: sum(w.map((x) => x.discarded)),
		// Worst case across the group: a device that lost every link must not be described as
		// having lost only some of them because another device in the same sentence kept its.
		consequence: consequence(
			w.some((x) => x.consequence === 'AllLinksLost') ? 'AllLinksLost' : 'SomeLinksLost'
		)
	};
}

function credentialParams(
	w: WarningOf<'CredentialTargetNotScanned' | 'CredentialTargetNotResponding'>[]
): Params {
	return { credential: integration(w[0].integration), addresses: addressesOf(w) };
}

function attemptParams(
	w: WarningOf<
		| 'CredentialRejected'
		| 'CredentialMalformed'
		| 'CredentialTlsFailed'
		| 'CredentialNotThisService'
		| 'CredentialCollectionFailed'
		| 'CredentialCollectionTimedOut'
		| 'CredentialUnreachable'
		| 'CredentialTimedOut'
	>[]
): Params {
	// The diagnostic the library returned, which is usually this address's alone — so the bucketing
	// below leaves these apart. Addresses that failed with the *same* message share a sentence,
	// which is the one case where merging loses nothing.
	return {
		credential: integration(w[0].integration),
		addresses: addressesOf(w),
		detail: w[0].detail || discovery_warningNoFurtherDetail()
	};
}

/**
 * Join the two ends of a neighbour relation.
 *
 * The arrow only carries meaning with something on both sides. A host deleted since the scan, or a
 * device that reported no interface description, can empty one end — and `-> TAMMIERENEW` or
 * `1/1 -> via InterfaceName("1/1")` reads as a rendering fault rather than as missing data. Drop
 * the arrow instead and show the end that survived.
 */
function arrow(near: string, far: string): string {
	if (!near) return far;
	if (!far) return near;
	return `${near} -> ${far}`;
}

/**
 * `switch7 Gi1/0/1 -> 00:ad:24:89:cc:f0 (core-sw) at 10.20.30.11`.
 *
 * Which of our devices saw the neighbour leads the line, because it is the first thing an operator
 * needs in order to act — the identifier alone says a link is missing without saying where to go
 * and look. Every segment is dropped when it is absent rather than filled with a placeholder: a
 * host whose name has not loaded yet, or one deleted since the scan, then reads exactly as it did
 * before names were resolved at all.
 *
 * The address is the segment that says which kind of gap this is. A far end that published one and
 * still matched nothing is a device on a range this network has not scanned, which an operator can
 * act on; one that published none cannot be placed however much gets scanned. Reading the two apart
 * used to cost a round trip to whoever reported the scan.
 */
function describeNeighbour(
	w: {
		host_id: string;
		if_descr: string;
		identifier: string;
		sys_name?: string | null;
		address?: string | null;
	},
	hostName: HostNameLookup
): string {
	const near = [hostName(w.host_id), w.if_descr].filter(Boolean).join(' ');
	const named = `${w.identifier}${w.sys_name ? ` (${w.sys_name})` : ''}`;
	const far = w.address ? `${named} ${discovery_warningAtAddress({ address: w.address })}` : named;
	return arrow(near, far);
}

function neighbourParams(
	w: WarningOf<'LldpNeighbourNotFound' | 'LldpNeighbourAmbiguous'>[],
	hostName: HostNameLookup
): Params {
	return {
		count: w.length,
		examples: joinList(
			w.map((x) => describeNeighbour(x, hostName)),
			'conjunction'
		)
	};
}

/**
 * `switch7 Gi0/1 -> core-sw via MacAddress("00:ad:…") (Port 9)`.
 *
 * Both ends are named here, unlike the unmatched case: the far end resolved to a device, and the
 * point of this warning is that the two are on the map and still not joined port-to-port. Both
 * halves of the advertised id are kept, because the subtype says which tier ran and the value says
 * what it looked for.
 */
function describePort(
	w: {
		host_id: string;
		remote_host_id: string;
		if_descr: string;
		port_id?: string | null;
		port_desc?: string | null;
	},
	hostName: HostNameLookup
): string {
	const near = [hostName(w.host_id), w.if_descr].filter(Boolean).join(' ');
	const desc = w.port_desc ? ` (${w.port_desc})` : '';
	// "via <id>" when the device advertised one, "with no port id" when it did not — the
	// distinction the tiers turn on, and the phrasing the prose these replaced used.
	const id = w.port_id ? `via ${w.port_id}${desc}` : `${discovery_warningNoPortId()}${desc}`;
	// The port only belongs to something when the far end resolved; without it the arrow points
	// straight at the identifier that was tried.
	const remote = hostName(w.remote_host_id);
	return arrow(near, remote ? `${remote} ${id}` : id);
}

/**
 * `10.20.30.0/24, from offsite-core-01 (10.20.30.11) and offsite-edge-01 (10.20.30.24) seen by
 * switch-offsite-01`.
 *
 * Names the far ends rather than only the range, because "we invented a subnet" is not something an
 * operator can act on: the labels are what let them recognise a segment as real, spot a device with
 * a factory-default address, or decide the range needs correcting.
 */
function describeProvisionalSubnet(
	w: WarningOf<'ProvisionalSubnetInferred'>,
	hostName: HostNameLookup
): string {
	// A range is reported for as long as it is unconfirmed, so most of them arrive without the
	// far-end evidence that produced them — the pass that inferred it may have been scans ago, and
	// one inferred while placing a controller-reported address never had a neighbour at all. The
	// range alone is still the actionable part.
	if (!w.addresses.length) return w.cidr;

	// Paired positionally where both are present: `sys_names` omits far ends that sent none, so a
	// group where only some did would otherwise misalign names against addresses.
	const named =
		w.sys_names.length === w.addresses.length
			? w.addresses.map((address, i) => `${w.sys_names[i]} (${address})`)
			: w.addresses;
	const seenBy = [...new Set(w.seen_by_host_ids.map(hostName).filter(Boolean))] as string[];
	const from = discovery_warningInferredFrom({ far_ends: joinList(named, 'conjunction') });
	return seenBy.length
		? `${w.cidr}, ${from} ${discovery_warningSeenBy({ devices: joinList(seenBy, 'conjunction') })}`
		: `${w.cidr}, ${from}`;
}

function portParams(
	w: WarningOf<'LldpPortNoStrategy' | 'LldpPortNotFound' | 'LldpPortAmbiguous'>[],
	hostName: HostNameLookup
): Params {
	return {
		count: w.length,
		examples: joinList(
			w.map((x) => describePort(x, hostName)),
			'conjunction'
		)
	};
}

/**
 * The slots whose value is computed from a whole group rather than from one occurrence.
 *
 * This is what makes the grouping below general. Two warnings of the same code say the *same
 * thing* when every slot outside this set matches: four switches that each advertised the bridge
 * bit and served no bridge-port numbering differ only in which device it was, so they are one
 * sentence naming four devices. Two credential attempts that failed with different diagnostics
 * differ in `detail`, so they stay two sentences — which is what the per-occurrence records the
 * backend keeps exist for.
 *
 * A new aggregating slot missing from this set only costs grouping: those occurrences render as
 * separate sentences, each correct. Listing an *identifying* slot here would be the harmful
 * direction — it would merge two statements and let one set of figures speak for both — which is
 * why the set is written this way round.
 */
const AGGREGATING_SLOTS = new Set([
	'addresses',
	'groups',
	'count',
	'examples',
	'dropped',
	'total',
	'misplaced',
	'discarded',
	'consequence',
	'elided'
]);

/**
 * Split one code's occurrences into the distinct statements they make.
 *
 * Insertion-ordered, so the buckets come out in the order the producers emitted them.
 */
function bucketByStatement(
	code: DiscoveryWarningCode,
	group_: DiscoveryWarning[],
	hostName: HostNameLookup
): DiscoveryWarning[][] {
	const build = WARNING_PARAMS[code] as (w: DiscoveryWarning[], lookup: HostNameLookup) => Params;
	const buckets = new Map<string, DiscoveryWarning[]>();

	for (const warning of group_) {
		const params = build([warning], hostName);
		const identity = Object.entries(params)
			.filter(([slot]) => !AGGREGATING_SLOTS.has(slot))
			.sort(([a], [b]) => a.localeCompare(b));
		const key = JSON.stringify(identity);

		const existing = buckets.get(key);
		if (existing) {
			existing.push(warning);
		} else {
			buckets.set(key, [warning]);
		}
	}

	return [...buckets.values()];
}

/**
 * One sentence per distinct statement a code's occurrences make.
 *
 * Usually one. Several when the occurrences genuinely differ — different credential diagnostics,
 * different claimed figures — and the devices that made the *same* statement are named together
 * inside a single sentence rather than repeating it once each.
 */
function renderSentences(
	code: DiscoveryWarningCode,
	group_: DiscoveryWarning[],
	hostName: HostNameLookup
): string[] {
	const build = WARNING_PARAMS[code] as (w: DiscoveryWarning[], lookup: HostNameLookup) => Params;
	const fallback = warningCodes.find((c) => c.id === code)?.description;
	if (!fallback) return [];

	return bucketByStatement(code, group_, hostName).map((bucket) =>
		metaDescriptionWith('warning_codes', code, build(bucket, hostName), fallback)
	);
}

/**
 * What a warning is *about*, as the chips on its row.
 *
 * Read off the wire payload rather than out of the sentence: every variant carries a host id, a
 * range, or an address, and taking the subject out of the prose is what lets a row say who it
 * concerns before the reader has read anything. Scan-level codes carry none and get no chips.
 *
 * The host id is preferred where a warning has both. An unmatched neighbour carries the address the
 * *far* end published, and the device an operator would go and look at is the near one that
 * reported it. An id that resolves to nothing is dropped rather than shown raw, the same policy the
 * sentences use — a host deleted since the scan reads as no chip, not as a UUID.
 */
function subjectsOf(warnings: DiscoveryWarning[], hostName: HostNameLookup): WarningSubject[] {
	const subjects = warnings.flatMap((w): WarningSubject[] => {
		if ('host_id' in w) {
			const label = hostName(w.host_id);
			return label ? [{ label, hostId: w.host_id }] : [];
		}
		if ('cidr' in w) return [{ label: w.cidr }];
		if ('address' in w) return [{ label: w.address }];
		return [];
	});

	const unique = [...new Map(subjects.map((s) => [s.label, s])).values()];
	const listed = unique.slice(0, MAX_LISTED);
	const elided = unique.length - listed.length;
	// Capped like the sentences are, and for the same reason: a row that wraps to five lines of
	// chips is a wall of text with rounded corners.
	if (elided > 0) listed.push({ label: common_moreItems({ count: elided }) });
	return listed;
}

/** The rung a code sits on, as the backend filed it. */
function remedyOf(code: DiscoveryWarningCode): string | null {
	return warningCodes.find((c) => c.id === code)?.category ?? null;
}

/**
 * Whether this run is asking the reader for something.
 *
 * The one bit the scan-history table and the modal's tab dot both need, and the reason they agree:
 * "needs you" means the same thing on every surface because it is the same question of the same
 * backend metadata.
 */
export function warningsNeedAttention(warnings: DiscoveryWarning[]): boolean {
	return warnings.some((w) => remedyOf(w.code) === NEEDS_ATTENTION_REMEDY);
}

/** The rung that means a person has to do something in Scanopy. */
const NEEDS_ATTENTION_REMEDY = 'FixInScanopy';

/**
 * A run's warnings, grouped into the sections an operator reads them in.
 *
 * The sections are the rungs of `warning-remedies.json` — what the reader has to do — in fixture
 * order, most demanding first. That, and not severity, is the top-level cut: severity measures what
 * the scan lost, and the two come apart badly enough that a thirty-second credential fix and a
 * permanently malformed LLDP table are both `Lost`/Red. Severity stays on the row, as its icon and
 * colour, which is the job it was written for.
 *
 * Within a section a row is one *code*, not one occurrence, and inside a row the occurrences that
 * say the same thing become one sentence naming every device — see {@link AGGREGATING_SLOTS}. The
 * wall of text this replaced came from every occurrence carrying its own full explanation: four
 * devices restricting the same SNMP view produced four near-identical paragraphs, and are now one
 * row and one sentence. Occurrences that genuinely differ — a credential diagnostic, a device's own
 * figures — still get a sentence each, so nothing the backend kept apart is merged.
 */
export function buildWarningReport(
	warnings: DiscoveryWarning[],
	hostName: HostNameLookup = NO_HOST_NAMES
): WarningSection[] {
	const byCode = new Map<DiscoveryWarningCode, DiscoveryWarning[]>();
	for (const warning of warnings) {
		const existing = byCode.get(warning.code);
		if (existing) {
			existing.push(warning);
		} else {
			byCode.set(warning.code, [warning]);
		}
	}

	const entries: WarningEntry[] = [];
	for (const [code, group_] of byCode) {
		const meta = warningCodes.find((c) => c.id === code);
		// A code this build has no fixture entry for renders nothing, as it did before: there is no
		// sentence to show and no rung to file it under.
		if (!meta) continue;

		entries.push({
			code,
			title: metaName('warning_codes', code, meta.name ?? code),
			color: toColor(meta.color),
			icon: meta.icon,
			subjects: subjectsOf(group_, hostName),
			details: renderSentences(code, group_, hostName),
			warnings: group_
		});
	}

	return warningRemedies
		.map((remedy) => ({
			remedy: remedy.id,
			title: metaName('warning_remedies', remedy.id, remedy.name ?? remedy.id),
			description: metaDescription('warning_remedies', remedy.id, remedy.description ?? ''),
			icon: remedy.icon,
			entries: entries.filter((entry) => remedyOf(entry.code) === remedy.id)
		}))
		.filter((section) => section.entries.length > 0);
}
