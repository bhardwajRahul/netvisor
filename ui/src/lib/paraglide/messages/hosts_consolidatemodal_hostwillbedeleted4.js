/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_hosts_consolidatemodal_hostwillbedeleted4 = /** @type {(inputs: { name: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`Host "${i?.name}" will be deleted`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ name: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const hosts_consolidatemodal_hostwillbedeleted4 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.hosts_consolidatemodal_hostwillbedeleted4(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("hosts_consolidatemodal_hostwillbedeleted4", locale)
	return en_hosts_consolidatemodal_hostwillbedeleted4(inputs)
};
export { hosts_consolidatemodal_hostwillbedeleted4 as "hosts_consolidateModal_hostWillBeDeleted" }