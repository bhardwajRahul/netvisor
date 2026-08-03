import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: './tests',
	testMatch: '**/*.ts',
	timeout: 60000,
	use: {
		// Overridable so a second dev server (e.g. an older build serving on
		// another port) can be profiled for a before/after comparison without
		// disturbing the primary one on 5173.
		baseURL: process.env.PW_BASE_URL ?? 'http://localhost:5173',
		headless: true,
		screenshot: 'only-on-failure'
	},
	projects: [
		{
			name: 'chromium',
			use: { browserName: 'chromium' }
		},
		// Firefox manages memory differently enough that a graph which merely janks in Chromium
		// can exhaust it outright — the out-of-memory report that prompted the culling work came
		// from Firefox 142 and did not reproduce in Chromium at the same node count. Listed
		// second so `npx playwright test` keeps its existing behaviour, and run explicitly with
		// `--project=firefox` (needs `npx playwright install firefox`).
		{
			name: 'firefox',
			use: { browserName: 'firefox' }
		}
	]
});
