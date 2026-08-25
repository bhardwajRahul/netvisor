import { describe, it, expect } from 'vitest';
import warningCodes from '$lib/data/warning-codes.json';
import { renderWarnings, type DiscoveryWarning } from '$lib/features/discovery/utils/warnings';

/**
 * The frontend half of the slot contract.
 *
 * A code's sentence is a template with `{named}` slots, and three things have to agree on those
 * names: the Rust `description()`, the `slots()` the fixture publishes, and the parameters this
 * renderer passes to the paraglide message. The backend test
 * (`every_description_interpolates_exactly_the_slots_it_declares`) pins the first two together;
 * this pins the third to them. Without it, a renderer that forgets a slot compiles fine and shows
 * the operator a literal `{addresses}`.
 */
describe('discovery warning rendering', () => {
	/** One warning per code, with every field its variant carries. */
	const sample = (code: string): DiscoveryWarning =>
		({
			code,
			address: '10.0.0.1',
			collected: 3,
			group: 'Lldp',
			limit: 10000,
			source: 'IfNumber',
			expected: 23,
			observed: 1,
			dropped: 1,
			total: 4,
			misplaced: 2,
			discarded: 14,
			kept: 0,
			consequence: 'AllLinksLost',
			integration: 'Snmp',
			ports: [443],
			detail: 'diagnostic',
			hours: 4,
			hosts_not_scanned: 12,
			minutes_remaining: 40,
			host_id: '00000000-0000-0000-0000-000000000001',
			remote_host_id: '00000000-0000-0000-0000-000000000002',
			if_descr: 'Gi1/0/1',
			identifier: '00:ad:24:89:cc:f0',
			sys_name: 'core-sw',
			port_id: 'MacAddress("00:ad:24:af:4e:00")',
			port_desc: 'Port 9',
			elided: 7
		}) as unknown as DiscoveryWarning;

	it('renders every code the backend can send, with no slot left unfilled', () => {
		const unfilled: string[] = [];

		for (const entry of warningCodes) {
			const rendered = renderWarnings([sample(entry.id)]);

			expect(rendered, `${entry.id} rendered nothing`).toHaveLength(1);
			// A `{slot}` surviving into the output is a parameter the renderer did not supply.
			const holes = rendered[0].match(/\{\w+\}/g);
			if (holes) {
				unfilled.push(`  ${entry.id}: ${holes.join(', ')}`);
			}
		}

		if (unfilled.length > 0) {
			expect.fail(
				`Warning templates rendered with unfilled slots:\n\n${unfilled.join('\n')}\n\n` +
					'Add the missing parameters to WARNING_PARAMS in ' +
					'src/lib/features/discovery/utils/warnings.ts.'
			);
		}
	});

	it('groups warnings sharing a code into one sentence, naming every address', () => {
		const warnings = ['192.168.7.235', '192.168.7.242'].map(
			(address) => sample('InterfaceSetCutShort') && { ...sample('InterfaceSetCutShort'), address }
		);

		const rendered = renderWarnings(warnings as DiscoveryWarning[]);

		// The reported problem this aggregation exists for: fifteen switches produced fifteen
		// paragraphs. One sentence per code, always — and no device silently dropped from it.
		expect(rendered).toHaveLength(1);
		expect(rendered[0]).toContain('192.168.7.235');
		expect(rendered[0]).toContain('192.168.7.242');
	});

	it('keeps devices that failed differently in separate sentences', () => {
		const rendered = renderWarnings([
			sample('SnmpWalkNoAnswer'),
			{ ...sample('SnmpWalkUnsupported'), address: '10.0.0.2' } as DiscoveryWarning
		]);

		// One says a rescan may help and the other says it never will; merging them would have to
		// pick one answer and be wrong for the other device.
		expect(rendered).toHaveLength(2);
	});

	it('renders a legacy string warning as its own text', () => {
		const legacy = {
			code: 'Unknown',
			detail: 'Scan hit its time limit (4h) — 12 host(s) not scanned.'
		} as unknown as DiscoveryWarning;

		// Historical sessions hold bare sentences, and they have to keep reading exactly as they
		// did before warnings were coded.
		expect(renderWarnings([legacy])).toEqual([
			'Scan hit its time limit (4h) — 12 host(s) not scanned.'
		]);
	});
});
