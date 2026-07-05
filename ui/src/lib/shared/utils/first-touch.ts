import type { PostHog } from 'posthog-js';

/**
 * First-touch / session-entry attribution capture.
 *
 * PostHog initializes opted-out behind the GDPR banner, so the landing
 * pageview (the only event carrying UTM params) is normally dropped. This
 * module snapshots attribution data at app boot — before any auth redirect
 * can strip the query string — and feeds it to PostHog once capturing
 * becomes active. Nothing is sent anywhere until the user opts in.
 */

const FIRST_TOUCH_KEY = 'scanopy_first_touch';
const SESSION_ENTRY_KEY = 'scanopy_session_entry';
const LANDING_PAGEVIEW_DROPPED_KEY = 'scanopy_landing_pageview_dropped';

const UTM_PARAMS = ['utm_source', 'utm_medium', 'utm_campaign', 'utm_term', 'utm_content'] as const;
const CLICK_ID_PARAMS = ['gclid', 'fbclid', 'msclkid', 'ttclid', 'li_fat_id', 'twclid'] as const;

export interface AttributionSnapshot {
	landing_url: string;
	landing_path: string;
	referrer: string;
	ts: string;
	utm_source?: string;
	utm_medium?: string;
	utm_campaign?: string;
	utm_term?: string;
	utm_content?: string;
	gclid?: string;
	fbclid?: string;
	msclkid?: string;
	ttclid?: string;
	li_fat_id?: string;
	twclid?: string;
}

function buildSnapshot(): AttributionSnapshot {
	const snapshot: AttributionSnapshot = {
		landing_url: window.location.href,
		landing_path: window.location.pathname,
		referrer: document.referrer,
		ts: new Date().toISOString()
	};
	const params = new URLSearchParams(window.location.search);
	for (const key of [...UTM_PARAMS, ...CLICK_ID_PARAMS]) {
		const value = params.get(key);
		if (value) snapshot[key] = value;
	}
	return snapshot;
}

function readSnapshot(storage: Storage, key: string): AttributionSnapshot | null {
	const raw = storage.getItem(key);
	if (!raw) return null;
	try {
		return JSON.parse(raw) as AttributionSnapshot;
	} catch {
		return null;
	}
}

/**
 * Snapshot landing data at app boot. Set-once per browser (first touch) and
 * set-once per tab session (session entry). Call as early as possible —
 * before the auth-check redirects can rewrite the URL.
 */
export function captureFirstTouch() {
	if (typeof window === 'undefined') return;
	try {
		const snapshot = buildSnapshot();
		if (!localStorage.getItem(FIRST_TOUCH_KEY)) {
			localStorage.setItem(FIRST_TOUCH_KEY, JSON.stringify(snapshot));
		}
		if (!sessionStorage.getItem(SESSION_ENTRY_KEY)) {
			sessionStorage.setItem(SESSION_ENTRY_KEY, JSON.stringify(snapshot));
		}
	} catch {
		// Storage unavailable (private mode quota, disabled cookies) — attribution is best-effort
	}
}

export function getFirstTouch(): AttributionSnapshot | null {
	if (typeof localStorage === 'undefined') return null;
	return readSnapshot(localStorage, FIRST_TOUCH_KEY);
}

export function getSessionEntry(): AttributionSnapshot | null {
	if (typeof sessionStorage === 'undefined') return null;
	return readSnapshot(sessionStorage, SESSION_ENTRY_KEY);
}

/**
 * Record that this session's real landing $pageview was discarded because
 * PostHog initialized opted-out. Read back when the user opts in so the
 * pageview can be replayed with the true landing URL.
 */
export function markLandingPageviewDropped() {
	if (typeof sessionStorage === 'undefined') return;
	sessionStorage.setItem(LANDING_PAGEVIEW_DROPPED_KEY, '1');
}

/**
 * Extract campaign params (UTMs + click IDs) from a snapshot, without the
 * landing URL/referrer/timestamp fields.
 */
export function campaignParams(snapshot: AttributionSnapshot): Record<string, string> {
	const out: Record<string, string> = {};
	for (const key of [...UTM_PARAMS, ...CLICK_ID_PARAMS]) {
		const value = snapshot[key];
		if (value) out[key] = value;
	}
	return out;
}

/**
 * Feed stored attribution to PostHog. Call whenever capturing becomes active:
 * on opt-in from the consent banner, or at load for already-consented visitors.
 * - Registers session-entry campaign params as super properties.
 * - Replays the landing $pageview if it was dropped earlier this session.
 */
export function applyAttributionToPosthog(posthog: PostHog) {
	const entry = getSessionEntry();
	if (!entry) return;

	const params = campaignParams(entry);
	if (Object.keys(params).length > 0) {
		// Session-scoped: plain register() would persist stale campaign params
		// onto every future session's events.
		posthog.register_for_session(params);
	}

	if (
		typeof sessionStorage !== 'undefined' &&
		sessionStorage.getItem(LANDING_PAGEVIEW_DROPPED_KEY)
	) {
		sessionStorage.removeItem(LANDING_PAGEVIEW_DROPPED_KEY);
		posthog.capture('$pageview', {
			$current_url: entry.landing_url,
			$referrer: entry.referrer || '$direct'
		});
	}
}
