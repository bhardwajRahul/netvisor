/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_topology_lockedtimestamp1 = /** @type {(inputs: { timestamp: NonNullable<unknown>, user: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`Locked: ${i?.timestamp} by ${i?.user}`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ timestamp: NonNullable<unknown>, user: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const topology_lockedtimestamp1 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.topology_lockedtimestamp1(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("topology_lockedtimestamp1", locale)
	return en_topology_lockedtimestamp1(inputs)
};
export { topology_lockedtimestamp1 as "topology_lockedTimestamp" }