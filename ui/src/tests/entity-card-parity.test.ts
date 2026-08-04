import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const SRC = path.resolve(__dirname, '..');
const LIB = path.join(SRC, 'lib');

/** The one component allowed to render a card, and the module that defines it. */
const CARD_RENDERER = path.join(LIB, 'shared/components/data/DataControls.svelte');
const CARD_COMPONENT = path.join(LIB, 'shared/components/data/EntityCard.svelte');

function findFilesRecursively(dir: string, extensions: string[]): string[] {
	const files: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory() && entry.name !== 'node_modules') {
			files.push(...findFilesRecursively(fullPath, extensions));
		} else if (entry.isFile() && extensions.some((ext) => entry.name.endsWith(ext))) {
			files.push(fullPath);
		}
	}
	return files;
}

/**
 * A card and a table are two presentations of one field definition.
 *
 * Both views render `renderedColumns`, which a tab declares once. That is the
 * only reason a field cannot exist in one view and be missing from the other —
 * and it holds only while `EntityCard` has no way to be handed content the
 * definition never saw. Every gap this refactor closed (schedule, progress,
 * status, credentials, host) reached production through exactly one of the
 * escape hatches below.
 *
 * So: one component renders cards, one component renders it, and it takes no
 * content props. Per-entity difference lives in the field definition.
 */
describe('entity card parity', () => {
	it('is rendered only by DataControls', () => {
		const offenders = findFilesRecursively(SRC, ['.svelte', '.ts'])
			.filter((file) => file !== CARD_RENDERER && file !== CARD_COMPONENT)
			.filter((file) =>
				/import\s+\w+\s+from\s+['"][^'"]*EntityCard\.svelte['"]/.test(
					fs.readFileSync(file, 'utf-8')
				)
			)
			.map((file) => path.relative(SRC, file));

		expect(
			offenders,
			`EntityCard is imported outside DataControls:\n\n  ${offenders.join('\n  ')}\n\n` +
				`A per-entity card component is how a field ends up on the card and ` +
				`missing from the table. Declare the field in the tab's field ` +
				`definition instead — both views render it from there.`
		).toEqual([]);
	});

	it('takes no per-entity content props', () => {
		// `columns` is the shared definition, so it is the one content prop the
		// card accepts. A prop that lets a caller supply its own field list, its
		// own header tag, or arbitrary markup reopens the gap.
		const source = fs.readFileSync(CARD_COMPONENT, 'utf-8');
		const propsBlock = source.slice(source.indexOf('let {'), source.indexOf('} = $props()'));

		const banned = ['fields', 'status', 'children', 'snippet'].filter((prop) =>
			new RegExp(`(^|[\\s,{])${prop}\\s*[,=:}]`, 'm').test(propsBlock)
		);

		expect(
			banned,
			`EntityCard declares content props: ${banned.join(', ')}. ` +
				`Card chrome comes from field metadata (display.primary / statusTag / ` +
				`subtitle), not from props a single tab can pass.`
		).toEqual([]);
	});

	it('is not bypassed by a tab passing its own content to DataControls', () => {
		const tabs = findFilesRecursively(path.join(LIB, 'features'), ['.svelte']).filter((file) =>
			/from\s+['"][^'"]*DataControls\.svelte['"]/.test(fs.readFileSync(file, 'utf-8'))
		);

		// A `children` snippet on DataControls was the original smuggling route:
		// markup that rendered on the card and nowhere else.
		const offenders = tabs
			.filter((file) => /\{#snippet\s+children\s*\(/.test(fs.readFileSync(file, 'utf-8')))
			.map((file) => path.relative(SRC, file));

		expect(
			offenders,
			`Tabs pass a children snippet to DataControls:\n\n  ${offenders.join('\n  ')}\n\n` +
				`Markup that only one view renders is the gap this test exists to ` +
				`prevent. Give the field a display.cell snippet — the table renders ` +
				`that too.`
		).toEqual([]);

		// Guard against the test silently passing because nothing matched.
		expect(tabs.length).toBeGreaterThan(10);
	});
});
