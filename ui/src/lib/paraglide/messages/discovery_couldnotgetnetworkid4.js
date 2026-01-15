/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_discovery_couldnotgetnetworkid4 = /** @type {(inputs: {}) => LocalizedString} */ () => {
	return /** @type {LocalizedString} */ (`Could not get network ID from selected daemon. Please try a different daemon.`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{}} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const discovery_couldnotgetnetworkid4 = (inputs = {}, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.discovery_couldnotgetnetworkid4(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("discovery_couldnotgetnetworkid4", locale)
	return en_discovery_couldnotgetnetworkid4(inputs)
};
export { discovery_couldnotgetnetworkid4 as "discovery_couldNotGetNetworkId" }