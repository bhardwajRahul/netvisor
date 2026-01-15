/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_auth_termsandprivacy2 = /** @type {(inputs: {}) => LocalizedString} */ () => {
	return /** @type {LocalizedString} */ (`I agree to the <a class='text-link' target='_blank' href='https://scanopy.net/terms'>terms</a> and <a target='_blank' class='text-link' href='https://scanopy.net/privacy'>privacy policy</a>`)
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
const auth_termsandprivacy2 = (inputs = {}, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.auth_termsandprivacy2(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("auth_termsandprivacy2", locale)
	return en_auth_termsandprivacy2(inputs)
};
export { auth_termsandprivacy2 as "auth_termsAndPrivacy" }