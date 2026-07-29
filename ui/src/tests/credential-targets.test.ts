import { describe, it, expect } from 'vitest';
import {
	integrationTargetFor,
	pruneAssignmentsForTargets,
	type CredentialTarget
} from '$lib/features/credentials/utils/credentialTargets';
import credentialTypes from '../lib/data/credential-types.json';

// Target sets standing in for the three shapes the backend declares, named by what they
// permit rather than by type so the cases stay readable if a type's targets change.
const BROADCAST_CAPABLE: CredentialTarget[] = ['DaemonHost', 'Hosts', 'Network']; // SNMP
const HOST_ONLY: CredentialTarget[] = ['DaemonHost', 'Hosts']; // UniFi, Docker proxy
const DAEMON_HOST_ONLY: CredentialTarget[] = ['DaemonHost']; // Docker/Podman socket

const CRED_ID = '11111111-1111-4111-8111-111111111111';

describe('pruneAssignmentsForTargets', () => {
	// The reporter's bug: the assignment surfaces are picked by `targets`, so switching the
	// credential type hides the Networks picker without clearing it — and the hidden value was
	// still submitted, reaching the daemon as a broadcast default for the whole subnet.
	it('surrenders network assignments when the new type cannot broadcast', () => {
		const result = pruneAssignmentsForTargets(HOST_ONLY, {
			assignedNetworkIds: ['net-a', 'net-b'],
			hostAssignments: [{ host_id: 'host-a', ip_address_ids: null }]
		});

		expect(result.assignedNetworkIds).toEqual([]);
		// A host assignment is still legitimate for this type and must survive the switch.
		expect(result.hostAssignments).toHaveLength(1);
		expect(result.changed).toBe(true);
	});

	// Drives whether the user is warned. Reporting `changed` on a no-op switch would nag on
	// every type change between two compatible types.
	it('reports no change when the new type permits everything already assigned', () => {
		const assignments = {
			assignedNetworkIds: ['net-a'],
			hostAssignments: [{ host_id: 'host-a', ip_address_ids: null }]
		};

		const result = pruneAssignmentsForTargets(BROADCAST_CAPABLE, assignments);

		expect(result.assignedNetworkIds).toEqual(['net-a']);
		expect(result.hostAssignments).toEqual(assignments.hostAssignments);
		expect(result.changed).toBe(false);
	});

	// A daemon host is a host: the local-socket type keeps its host assignment (that is how it
	// is assigned at all) while still losing any network.
	it('keeps host assignments for a daemon-host-only type', () => {
		const result = pruneAssignmentsForTargets(DAEMON_HOST_ONLY, {
			assignedNetworkIds: ['net-a'],
			hostAssignments: [{ host_id: 'daemon-host', ip_address_ids: null }]
		});

		expect(result.hostAssignments).toHaveLength(1);
		expect(result.assignedNetworkIds).toEqual([]);
	});

	// A local socket is reachable over its daemon's loopback and nowhere else, so an ordinary
	// host is not a valid target for it even though the type does target "a host".
	it('narrows a daemon-host-only type to hosts that actually run a daemon', () => {
		const result = pruneAssignmentsForTargets(
			DAEMON_HOST_ONLY,
			{
				assignedNetworkIds: [],
				hostAssignments: [
					{ host_id: 'daemon-host', ip_address_ids: null },
					{ host_id: 'ordinary-host', ip_address_ids: null }
				]
			},
			['daemon-host']
		);

		expect(result.hostAssignments).toEqual([{ host_id: 'daemon-host', ip_address_ids: null }]);
		expect(result.changed).toBe(true);
	});

	// The daemon list is loaded asynchronously. Treating "not loaded yet" as "no daemon hosts
	// exist" would strip every assignment the credential legitimately holds, on any save that
	// happened to race the query.
	it('does not narrow when the daemon host list is unavailable', () => {
		const hostAssignments = [{ host_id: 'daemon-host', ip_address_ids: null }];

		expect(
			pruneAssignmentsForTargets(DAEMON_HOST_ONLY, { assignedNetworkIds: [], hostAssignments })
				.hostAssignments
		).toHaveLength(1);
	});

	// A type that targets remote hosts is not restricted to daemon hosts — the narrowing must
	// not leak across to it.
	it('leaves a Hosts-capable type unnarrowed by the daemon host list', () => {
		const result = pruneAssignmentsForTargets(
			HOST_ONLY,
			{
				assignedNetworkIds: [],
				hostAssignments: [{ host_id: 'ordinary-host', ip_address_ids: null }]
			},
			['daemon-host']
		);

		expect(result.hostAssignments).toHaveLength(1);
		expect(result.changed).toBe(false);
	});

	// Absent metadata (an unknown type id yields `{}` from the store) must not be read as
	// "anything goes" — the credential would be broadcast on a guess.
	it('permits nothing when the type has no targets metadata', () => {
		const result = pruneAssignmentsForTargets(undefined, {
			assignedNetworkIds: ['net-a'],
			hostAssignments: [{ host_id: 'host-a', ip_address_ids: null }]
		});

		expect(result.assignedNetworkIds).toEqual([]);
		expect(result.hostAssignments).toEqual([]);
		expect(result.changed).toBe(true);
	});

	it('does not mutate the caller’s arrays', () => {
		const assignedNetworkIds = ['net-a'];
		const hostAssignments = [{ host_id: 'host-a', ip_address_ids: null }];

		pruneAssignmentsForTargets(HOST_ONLY, { assignedNetworkIds, hostAssignments });

		expect(assignedNetworkIds).toEqual(['net-a']);
		expect(hostAssignments).toHaveLength(1);
	});
});

describe('integrationTargetFor', () => {
	// The Gap-2 substitution: scope used to come from the IP count alone, so "no target chosen"
	// on a controller credential serialized as a network-wide broadcast the server then dropped
	// — the credential silently never ran.
	it('yields no target when a type that cannot broadcast has no IPs', () => {
		expect(integrationTargetFor(CRED_ID, HOST_ONLY, [])).toBeNull();
	});

	it('yields a network target when the type can broadcast and no IPs are given', () => {
		expect(integrationTargetFor(CRED_ID, BROADCAST_CAPABLE, [])).toEqual({
			credential_id: CRED_ID,
			scope: 'Network'
		});
	});

	// `targetIps: ['']` is the literal seed the wizard and the discovery modal use for a row
	// with nothing entered, so blank rows must read as "no selection", not as a host list.
	it('treats blank IP rows as no selection at all', () => {
		expect(integrationTargetFor(CRED_ID, HOST_ONLY, ['', '  '])).toBeNull();
		expect(integrationTargetFor(CRED_ID, BROADCAST_CAPABLE, ['', '  '])).toEqual({
			credential_id: CRED_ID,
			scope: 'Network'
		});
	});

	it('routes loopback-only selections to the daemon host', () => {
		expect(integrationTargetFor(CRED_ID, HOST_ONLY, ['127.0.0.1', '::1'])).toEqual({
			credential_id: CRED_ID,
			scope: 'DaemonHost'
		});
	});

	// "every" vs "some": one routable address means the user is targeting real hosts, and the
	// loopback entry has to travel with them rather than collapsing the whole set.
	it('keeps a mixed loopback/routable selection as a host list', () => {
		expect(integrationTargetFor(CRED_ID, HOST_ONLY, ['127.0.0.1', '10.0.0.5'])).toEqual({
			credential_id: CRED_ID,
			ips: ['127.0.0.1', '10.0.0.5'],
			scope: 'Hosts'
		});
	});

	it('pins a daemon-host-only type to the daemon host even when IPs are entered', () => {
		expect(integrationTargetFor(CRED_ID, DAEMON_HOST_ONLY, ['10.0.0.5'])).toEqual({
			credential_id: CRED_ID,
			scope: 'DaemonHost'
		});
	});
});

describe('integrationTargetFor against the real credential-type fixture', () => {
	// The server drops any target whose scope its credential type does not permit
	// (apply_integration_target). Restating that rule here means a newly added credential type
	// whose target set this derivation mishandles fails the build rather than shipping a
	// credential that silently never runs.
	it('never emits a scope the type does not declare', () => {
		const ipCases = [[], [''], ['127.0.0.1'], ['10.0.0.5'], ['127.0.0.1', '10.0.0.5']];

		for (const type of credentialTypes) {
			const targets = (type.metadata?.targets ?? []) as CredentialTarget[];
			expect(targets.length, `${type.id} declares no targets`).toBeGreaterThan(0);

			for (const ips of ipCases) {
				const target = integrationTargetFor(CRED_ID, targets, ips);
				if (target === null) continue;
				expect(targets, `${type.id} with ips=${JSON.stringify(ips)}`).toContain(target.scope);
			}
		}
	});
});
