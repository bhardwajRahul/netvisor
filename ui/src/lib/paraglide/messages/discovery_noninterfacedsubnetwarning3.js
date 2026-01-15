/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_discovery_noninterfacedsubnetwarning3 = /** @type {(inputs: { subnets: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`The selected daemon does not have a direct network interface with the following subnets: 
${i?.subnets}. 
You can still include them, but hostnames and MAC addresses will not be available for any discovered hosts.`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ subnets: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const discovery_noninterfacedsubnetwarning3 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.discovery_noninterfacedsubnetwarning3(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("discovery_noninterfacedsubnetwarning3", locale)
	return en_discovery_noninterfacedsubnetwarning3(inputs)
};
export { discovery_noninterfacedsubnetwarning3 as "discovery_nonInterfacedSubnetWarning" }