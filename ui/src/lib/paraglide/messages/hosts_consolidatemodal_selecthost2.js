/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_hosts_consolidatemodal_selecthost2 = /** @type {(inputs: { hostName: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`Select host which ${i?.hostName} will be consolidated with:`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ hostName: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const hosts_consolidatemodal_selecthost2 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.hosts_consolidatemodal_selecthost2(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("hosts_consolidatemodal_selecthost2", locale)
	return en_hosts_consolidatemodal_selecthost2(inputs)
};
export { hosts_consolidatemodal_selecthost2 as "hosts_consolidateModal_selectHost" }