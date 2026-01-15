/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_hosts_interfaces_ipmustbewithin3 = /** @type {(inputs: { cidr: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`Must be within ${i?.cidr}`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ cidr: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const hosts_interfaces_ipmustbewithin3 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.hosts_interfaces_ipmustbewithin3(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("hosts_interfaces_ipmustbewithin3", locale)
	return en_hosts_interfaces_ipmustbewithin3(inputs)
};
export { hosts_interfaces_ipmustbewithin3 as "hosts_interfaces_ipMustBeWithin" }