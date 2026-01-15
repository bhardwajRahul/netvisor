/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_auth_youreallsetbodysingle4 = /** @type {(inputs: { networkNames: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`Register to finish setup. Your daemon will begin scanning "${i?.networkNames}" and populating your network map.`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ networkNames: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const auth_youreallsetbodysingle4 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.auth_youreallsetbodysingle4(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("auth_youreallsetbodysingle4", locale)
	return en_auth_youreallsetbodysingle4(inputs)
};
export { auth_youreallsetbodysingle4 as "auth_youreAllSetBodySingle" }