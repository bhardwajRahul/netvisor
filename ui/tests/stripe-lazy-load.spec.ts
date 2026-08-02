import { test, expect } from '@playwright/test';

/**
 * Stripe.js must not load until billing actually needs it.
 *
 * `@stripe/stripe-js`'s default entry injects the Stripe.js <script> as a top-level side effect
 * of being imported, and `StripeCardForm` is reachable from the root layout (AppShell ->
 * PaymentMethodModal), so every page load fetched it. That went unnoticed on the main app, whose
 * CSP allows js.stripe.com, and surfaced in production on share/embed routes, whose CSP
 * deliberately allows no third-party scripts:
 *
 *   Loading the script 'https://js.stripe.com/dahlia/stripe.js' violates the following
 *   Content Security Policy directive: "script-src 'self' 'unsafe-inline'"
 *
 * The fix is importing from `@stripe/stripe-js/pure`, which defers injection to the first
 * `loadStripe()` call. Reverting that import makes this test fail with exactly the URL above,
 * which is what keeps it honest.
 *
 * Requires a live dev stack and a SESSION_ID from a logged-in browser.
 */
test('no stripe script on a page that has no billing UI', async ({ page, context }) => {
	test.setTimeout(120_000);
	await context.addCookies([
		{ name: 'session_id', value: process.env.SESSION_ID ?? '', domain: 'localhost', path: '/' }
	]);

	// Matched on the parsed hostname rather than as a substring: `https://elsewhere.test/?
	// ref=js.stripe.com` is not Stripe.js, and a substring test would fail this test on it.
	const isStripeHost = (url: string) => {
		try {
			return new URL(url).hostname === 'js.stripe.com';
		} catch {
			return false;
		}
	};

	const stripeRequests: string[] = [];
	page.on('request', (r) => {
		if (isStripeHost(r.url())) stripeRequests.push(r.url());
	});

	await page.goto('/');
	// Not `networkidle` — the app holds an SSE stream open, so it never settles.
	await page.waitForSelector('nav', { timeout: 60_000 });
	// Generous settle: the default entry injects on the microtask after import, so anything
	// eager would have fired long before this.
	await page.waitForTimeout(3000);

	const tags = await page.evaluate(
		() =>
			Array.from(document.querySelectorAll<HTMLScriptElement>('script[src]')).filter((s) => {
				try {
					return new URL(s.src, document.baseURI).hostname === 'js.stripe.com';
				} catch {
					return false;
				}
			}).length
	);
	console.log('stripe requests:', JSON.stringify(stripeRequests), 'script tags:', tags);
	expect(stripeRequests, `Stripe.js was fetched: ${stripeRequests.join(', ')}`).toEqual([]);
	expect(tags).toBe(0);
});
