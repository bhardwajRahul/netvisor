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
import credentialTypes from '$lib/data/credential-types.json';
import malformedNeighbourConsequences from '$lib/data/malformed-neighbour-consequences.json';
import snmpWalkGroups from '$lib/data/snmp-walk-groups.json';
import warningCodes from '$lib/data/warning-codes.json';
import { metaDescriptionWith, metaName } from '$lib/i18n/metadata';
import {
	common_andNMore,
	discovery_warningNoFurtherDetail,
	discovery_warningNoPortId
} from '$lib/paraglide/messages';

export type DiscoveryWarning = components['schemas']['DiscoveryWarning'];
export type DiscoveryWarningCode = DiscoveryWarning['code'];

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
 * The credential type's display name, from the credential-types fixture rather than a hand-kept
 * map — the backend sends the integration discriminant, and its English lives with the rest of the
 * credential metadata.
 */
const integration = (id: string) => nameOf(credentialTypes, 'credential_types', id);

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

	// The claim warnings name one device each: the figures are the substance of the sentence, and
	// two switches disagreeing with themselves by different amounts share nothing but its shape.
	ClaimedCountReadCutShort: (w) => ({
		address: w[0].address,
		source: claimSource(w[0].source),
		expected: w[0].expected,
		observed: w[0].observed
	}),
	ClaimedCountUnderRead: (w) => ({
		address: w[0].address,
		source: claimSource(w[0].source),
		expected: w[0].expected,
		observed: w[0].observed
	}),
	ClaimedCapabilityReadCutShort: (w) => ({
		address: w[0].address,
		source: claimSource(w[0].source),
		group: group(w[0].group)
	}),
	ClaimedCapabilityEmpty: (w) => ({
		address: w[0].address,
		source: claimSource(w[0].source),
		group: group(w[0].group)
	}),

	LldpLocalPortDropped: (w) => ({
		addresses: addressesOf(w),
		dropped: sum(w.map((x) => x.dropped)),
		total: sum(w.map((x) => x.total))
	}),
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
		credentials: credentialsOf(w),
		addresses: addressesOf(w),
		ports: joinList(
			w.flatMap((x) => x.ports.map(String)),
			'conjunction'
		)
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

	WarningsTruncated: (w) => ({ elided: sum(w.map((x) => x.elided)) }),
	// The whole sentence *is* the detail: a warning from another version, or one written before
	// warnings were coded at all. Rendered one per occurrence, so this only ever sees one.
	Unknown: (w) => ({ detail: w[0].detail })
} satisfies {
	[C in DiscoveryWarningCode]: (warnings: WarningOf<C>[], hostName: HostNameLookup) => Params;
};

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

const credentialsOf = (w: { integration: string }[]) =>
	joinList(
		w.map((x) => integration(x.integration)),
		'conjunction'
	);

function credentialParams(
	w: WarningOf<'CredentialTargetNotScanned' | 'CredentialTargetNotResponding'>[]
): Params {
	return { credentials: credentialsOf(w), addresses: addressesOf(w) };
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
	// Every distinct diagnostic the group produced. The backend keeps one per address now, so a
	// host failing for its own reason is no longer shown a neighbour's.
	const details = [...new Set(w.map((x) => x.detail).filter((d): d is string => !!d))];
	return {
		credentials: credentialsOf(w),
		addresses: addressesOf(w),
		detail:
			details.length > 0 ? joinList(details, 'conjunction') : discovery_warningNoFurtherDetail()
	};
}

/**
 * `switch7 Gi1/0/1 -> 00:ad:24:89:cc:f0 (core-sw)`.
 *
 * Which of our devices saw the neighbour leads the line, because it is the first thing an operator
 * needs in order to act — the identifier alone says a link is missing without saying where to go
 * and look. Every segment is dropped when it is absent rather than filled with a placeholder: a
 * host whose name has not loaded yet, or one deleted since the scan, then reads exactly as it did
 * before names were resolved at all.
 */
function describeNeighbour(
	w: { host_id: string; if_descr: string; identifier: string; sys_name?: string | null },
	hostName: HostNameLookup
): string {
	const far = `${w.identifier}${w.sys_name ? ` (${w.sys_name})` : ''}`;
	return [hostName(w.host_id), w.if_descr, '->', far].filter(Boolean).join(' ');
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
	const id = `via ${w.port_id ?? discovery_warningNoPortId()}${w.port_desc ? ` (${w.port_desc})` : ''}`;
	return [hostName(w.host_id), w.if_descr, '->', hostName(w.remote_host_id), id]
		.filter(Boolean)
		.join(' ');
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
 * Codes that get one sentence per occurrence rather than one covering the group.
 *
 * Two reasons, both about the sentence being unshareable:
 *
 * - the claim warnings are built around two figures a device published about *itself*, and two
 *   switches disagreeing with themselves by different amounts have nothing to share but the shape
 *   of the complaint;
 * - `Unknown` carries a whole pre-coded sentence as its detail, and merging several into one
 *   bullet turns a historical scan's warning list into the wall of text the aggregation exists to
 *   prevent. Before warnings were coded each of those strings was its own bullet, and it stays
 *   that way.
 */
const PER_OCCURRENCE = new Set<DiscoveryWarningCode>([
	'ClaimedCountReadCutShort',
	'ClaimedCountUnderRead',
	'ClaimedCapabilityReadCutShort',
	'ClaimedCapabilityEmpty',
	'Unknown'
]);

/**
 * Group a run's warnings by code and render one sentence per group, in the order the codes first
 * appear — which is the order the producers emitted them, so the shortfall for a device still
 * precedes the contradiction that explains it.
 */
export function renderWarnings(
	warnings: DiscoveryWarning[],
	hostName: HostNameLookup = NO_HOST_NAMES
): string[] {
	const byCode = new Map<DiscoveryWarningCode, DiscoveryWarning[]>();
	for (const warning of warnings) {
		const existing = byCode.get(warning.code);
		if (existing) {
			existing.push(warning);
		} else {
			byCode.set(warning.code, [warning]);
		}
	}

	return [...byCode].flatMap(([code, group_]) => {
		const build = WARNING_PARAMS[code] as (w: DiscoveryWarning[], lookup: HostNameLookup) => Params;
		const fallback = warningCodes.find((c) => c.id === code)?.description;
		if (!fallback) return [];

		if (PER_OCCURRENCE.has(code)) {
			return group_.map((warning) =>
				metaDescriptionWith('warning_codes', code, build([warning], hostName), fallback)
			);
		}
		return [metaDescriptionWith('warning_codes', code, build(group_, hostName), fallback)];
	});
}
