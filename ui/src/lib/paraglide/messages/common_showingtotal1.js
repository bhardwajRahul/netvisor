/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_common_showingtotal1 = /** @type {(inputs: { count: NonNullable<unknown>, total: NonNullable<unknown>, itemLabel: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`Showing ${i?.count} of ${i?.total} ${i?.itemLabel}`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ count: NonNullable<unknown>, total: NonNullable<unknown>, itemLabel: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const common_showingtotal1 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.common_showingtotal1(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("common_showingtotal1", locale)
	return en_common_showingtotal1(inputs)
};
export { common_showingtotal1 as "common_showingTotal" }