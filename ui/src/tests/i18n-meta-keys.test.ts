import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildMetaMessages } from '../../scripts/generate-meta-messages.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Drift guard for backend-metadata i18n keys.
 *
 * The fixtures in src/lib/data/*.json are generated from backend metadata
 * providers; scripts/generate-meta-messages.js syncs their prose into
 * messages/en.json as meta_* keys (resolved at runtime by
 * src/lib/i18n/metadata.ts with fixture-string fallback). If a backend
 * metadata change regenerates fixtures without re-running the key sync,
 * the translated path silently goes stale — this test catches that.
 */
describe('i18n meta keys', () => {
	const messagesPath = path.resolve(__dirname, '../../../messages/en.json');
	const messages = JSON.parse(fs.readFileSync(messagesPath, 'utf8'));
	const expected = buildMetaMessages() as Record<string, string>;

	it('en.json contains a meta_* key for every covered fixture string', () => {
		const missing = Object.keys(expected).filter((key) => !(key in messages));

		if (missing.length > 0) {
			expect.fail(
				`Found ${missing.length} fixture strings without meta_* keys in en.json:\n\n${missing.map((k) => `  - ${k}`).join('\n')}\n\nRun \`node scripts/generate-meta-messages.js\` (or \`make generate-fixtures\`) to sync.`
			);
		}
	});

	it('en.json meta_* keys match the covered fixtures (no stale or drifted keys)', () => {
		const stale = Object.keys(messages).filter(
			(key) => key.startsWith('meta_') && !(key in expected)
		);
		const drifted = Object.entries(expected).filter(
			([key, value]) => key in messages && messages[key] !== value
		);

		if (stale.length > 0 || drifted.length > 0) {
			const staleLines = stale.map((k) => `  - stale: ${k}`);
			const driftedLines = drifted.map(
				([k, v]) => `  - drifted: ${k}\n      fixture: "${v}"\n      en.json: "${messages[k]}"`
			);
			expect.fail(
				`meta_* keys in en.json are out of sync with fixtures:\n\n${[...staleLines, ...driftedLines].join('\n')}\n\nRun \`node scripts/generate-meta-messages.js\` (or \`make generate-fixtures\`) to sync.`
			);
		}
	});
});
