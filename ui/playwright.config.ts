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
		}
	]
});
