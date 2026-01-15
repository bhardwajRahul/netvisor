/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_hosts_consolidatemodal_warningbody2 = /** @type {(inputs: {}) => LocalizedString} */ () => {
	return /** @type {LocalizedString} */ (`The source host will be permanently deleted and converted to an interface. Make sure this is what you want before proceeding.`)
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
const hosts_consolidatemodal_warningbody2 = (inputs = {}, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.hosts_consolidatemodal_warningbody2(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("hosts_consolidatemodal_warningbody2", locale)
	return en_hosts_consolidatemodal_warningbody2(inputs)
};
export { hosts_consolidatemodal_warningbody2 as "hosts_consolidateModal_warningBody" }