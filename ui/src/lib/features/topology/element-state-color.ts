/**
 * The one thing an element card can still say when it is three pixels wide.
 *
 * Below the detail threshold a card draws as its box, and a white box is noise — 2,960 of them
 * tell an operator nothing they could not get from the container outlines. A *coloured* box is a
 * map: which links are down, which parts of the estate have gone quiet.
 *
 * # Why state and not identity
 *
 * The tempting sources are identity palettes, and they all fail the same way. Entity type
 * (`entities.getColorHelper`) is constant within a view — every element in L3 Logical is an
 * `IPAddress`, so the whole graph comes out one shade of emerald. A service definition's colour
 * does vary per instance, but nginx-blue beside postgres-green is a rainbow, not information: at
 * this size the eye can pick out an ordered vocabulary it is scanning for, and nothing else.
 *
 * So: a small ordered vocabulary, or nothing. Down beats stale beats fine, because that is the
 * order someone scanning a wall of these cares about.
 */

/** Deliberately few, and ordered by how much they want attention. */
export type ElementState = 'down' | 'stale' | 'nominal' | 'unknown';

/**
 * Fills for each state.
 *
 * Muted rather than saturated: at a few pixels these tile into large fields of flat colour, and a
 * full-strength red field reads as an emergency covering half the estate. The dot on a full-size
 * card can afford `#ef4444` because there is one of it surrounded by white.
 */
export const ELEMENT_STATE_FILL: Record<ElementState, string> = {
	down: '#fca5a5',
	stale: '#fcd34d',
	nominal: '#86efac',
	unknown: '#e5e7eb'
};

export interface ElementStateInputs {
	/** `operStatus` from the element's port status, where it has one. */
	operStatus?: string | null;
	/** Whether the freshness check produced a staleness badge for this element. */
	isStale?: boolean;
}

/**
 * Reduce an element to the state its shell should paint.
 *
 * `unknown` is a real answer, not a fallback for laziness: an L3 address or an application service
 * has no link state and no staleness worth colouring, and the honest rendering there is neutral
 * texture with the labelled containers carrying the meaning.
 */
export function elementState({ operStatus, isStale }: ElementStateInputs): ElementState {
	if (operStatus === 'Down') return 'down';
	if (isStale) return 'stale';
	if (operStatus === 'Up') return 'nominal';
	return 'unknown';
}

/** The full-size card's status dot keeps its own saturated palette — one dot, not a field. */
export const PORT_STATUS_DOT: Record<string, string> = {
	Up: '#22c55e',
	Down: '#ef4444'
};

export function portStatusDotColor(operStatus: string | null | undefined): string {
	return PORT_STATUS_DOT[operStatus ?? ''] ?? '#9ca3af';
}
