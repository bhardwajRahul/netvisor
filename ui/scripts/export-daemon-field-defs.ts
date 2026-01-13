import { fieldDefs } from '../src/lib/features/daemons/config.ts';

function toSnakeCase(s: string): string {
	return s
		.replace(/([A-Z])/g, '_$1')
		.toLowerCase()
		.replace(/^_/, '');
}

// Determine output format based on command line argument
const format = process.argv[2] || 'test';

if (format === 'docs') {
	// Full export for documentation website
	const exported = fieldDefs.map((f) => {
		// Determine the default value to display
		// Only show actual defaults, not placeholders (which are just hints)
		let defaultDisplay: string | null = null;
		if (f.defaultValue !== undefined && f.defaultValue !== '') {
			defaultDisplay = String(f.defaultValue);
		} else if (f.required) {
			defaultDisplay = '_Required_';
		} else if (f.type === 'number' && f.placeholder !== undefined) {
			// For numbers, placeholder often represents the actual default
			defaultDisplay = String(f.placeholder);
		}

		return {
			id: toSnakeCase(f.id),
			label: f.label,
			cliFlag: f.cliFlag,
			envVar: f.envVar,
			configFileKey: toSnakeCase(f.id),
			default: defaultDisplay,
			description: f.helpText,
			docsOnly: f.docsOnly || false
		};
	});
	console.log(JSON.stringify(exported, null, 2));
} else {
	// Minimal export for Rust sync tests (original format)
	const exported = fieldDefs
		.filter((f) => !f.docsOnly) // Exclude docs-only fields from sync test
		.map((f) => ({
			id: toSnakeCase(f.id),
			cliFlag: f.cliFlag,
			envVar: f.envVar,
			helpText: f.helpText
		}));
	console.log(JSON.stringify(exported, null, 2));
}
