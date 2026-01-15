/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_errors_interface_ip_out_of_range = /** @type {(inputs: { ip: NonNullable<unknown>, subnet: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`IP address '${i?.ip}' is not within subnet '${i?.subnet}' range`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ ip: NonNullable<unknown>, subnet: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
export const errors_interface_ip_out_of_range = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.errors_interface_ip_out_of_range(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("errors_interface_ip_out_of_range", locale)
	return en_errors_interface_ip_out_of_range(inputs)
};