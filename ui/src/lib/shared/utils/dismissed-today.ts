function todayKey(): string {
	const d = new Date();
	const yyyy = d.getFullYear();
	const mm = String(d.getMonth() + 1).padStart(2, '0');
	const dd = String(d.getDate()).padStart(2, '0');
	return `${yyyy}-${mm}-${dd}`;
}

export function wasDismissedToday(key: string): boolean {
	if (typeof localStorage === 'undefined') return false;
	return localStorage.getItem(`dismissed_today:${key}`) === todayKey();
}

export function markDismissedToday(key: string): void {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(`dismissed_today:${key}`, todayKey());
}
