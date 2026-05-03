const STORAGE_KEY = 'plan_activated_at';
const WINDOW_MS = 24 * 60 * 60 * 1000;

export function markPlanActivated(): void {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_KEY, String(Date.now()));
}

export function isPlanActivationRecent(): boolean {
	if (typeof localStorage === 'undefined') return false;
	const raw = localStorage.getItem(STORAGE_KEY);
	if (!raw) return false;
	const ts = Number(raw);
	if (!Number.isFinite(ts)) return false;
	return Date.now() - ts < WINDOW_MS;
}

export function clearPlanActivationMarker(): void {
	if (typeof localStorage === 'undefined') return;
	localStorage.removeItem(STORAGE_KEY);
}
