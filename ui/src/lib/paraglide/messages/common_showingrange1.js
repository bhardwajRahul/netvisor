/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_common_showingrange1 = /** @type {(inputs: { start: NonNullable<unknown>, end: NonNullable<unknown>, total: NonNullable<unknown>, itemLabel: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`Showing ${i?.start}-${i?.end} of ${i?.total} ${i?.itemLabel}`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ start: NonNullable<unknown>, end: NonNullable<unknown>, total: NonNullable<unknown>, itemLabel: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const common_showingrange1 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.common_showingrange1(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("common_showingrange1", locale)
	return en_common_showingrange1(inputs)
};
export { common_showingrange1 as "common_showingRange" }