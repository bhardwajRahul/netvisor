import { writable } from 'svelte/store';

const STORAGE_KEY = 'plan_activated_at';
const WINDOW_MS = 24 * 60 * 60 * 1000;

function read(): number | null {
	if (typeof localStorage === 'undefined') return null;
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return null;
	const n = Number(raw);
	return Number.isFinite(n) ? n : null;
}

export const planActivatedAt = writable<number | null>(read());

export function markPlanActivated(): void {
	if (typeof localStorage === 'undefined') return;
	const now = Date.now();
	localStorage.setItem(STORAGE_KEY, String(now));
	planActivatedAt.set(now);
}

export function isPlanActivationRecent(ts: number | null): boolean {
	if (ts == null) return false;
	return Date.now() - ts < WINDOW_MS;
}

export function clearPlanActivationMarker(): void {
	if (typeof localStorage === 'undefined') return;
	localStorage.removeItem(STORAGE_KEY);
	planActivatedAt.set(null);
}
