/**
 * Watch a zero-height sentinel to tell whether the controls bar has stuck to
 * the top of its scroll container.
 *
 * The sentinel leaving the viewport is not sufficient on its own: switching to
 * a hidden tab also un-intersects it, which flashed the stuck border on every
 * tab change. Requiring a non-zero scroll position distinguishes "scrolled past
 * it" from "not on screen at all".
 *
 * Returns the observer's disconnect, for an effect to use as its cleanup.
 */
export function observeStuck(
	sentinel: HTMLElement,
	setStuck: (stuck: boolean) => void
): () => void {
	// The app scrolls inside <main>, not the window, so that is the root the
	// intersection has to be measured against.
	const scrollContainer = sentinel.closest('main');

	const observer = new IntersectionObserver(
		([entry]) => {
			const scrollTop = scrollContainer?.scrollTop ?? 0;
			setStuck(!entry.isIntersecting && scrollTop > 0);
		},
		{ threshold: 0, root: scrollContainer }
	);
	observer.observe(sentinel);

	return () => observer.disconnect();
}
