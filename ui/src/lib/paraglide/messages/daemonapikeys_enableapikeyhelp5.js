/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_daemonapikeys_enableapikeyhelp5 = /** @type {(inputs: {}) => LocalizedString} */ () => {
	return /** @type {LocalizedString} */ (`Manually enable or disable API Key. Will be auto-disabled if used in a request after expiry date passes.`)
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
const daemonapikeys_enableapikeyhelp5 = (inputs = {}, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.daemonapikeys_enableapikeyhelp5(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("daemonapikeys_enableapikeyhelp5", locale)
	return en_daemonapikeys_enableapikeyhelp5(inputs)
};
export { daemonapikeys_enableapikeyhelp5 as "daemonApiKeys_enableApiKeyHelp" }