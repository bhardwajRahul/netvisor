/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_users_sensitivelinkwarning2 = /** @type {(inputs: { permissions: NonNullable<unknown> }) => LocalizedString} */ (i) => {
	return /** @type {LocalizedString} */ (`Anyone with this link can join your organization with ${i?.permissions} permissions. Keep it secure and only share it with people you trust.`)
};

/**
* This function has been compiled by [Paraglide JS](https://inlang.com/m/gerre34r).
*
* - Changing this function will be over-written by the next build.
*
* - If you want to change the translations, you can either edit the source files e.g. `en.json`, or
* use another inlang app like [Fink](https://inlang.com/m/tdozzpar) or the [VSCode extension Sherlock](https://inlang.com/m/r7kp499g).
* 
* @param {{ permissions: NonNullable<unknown> }} inputs
* @param {{ locale?: "en" }} options
* @returns {LocalizedString}
*/
/* @__NO_SIDE_EFFECTS__ */
const users_sensitivelinkwarning2 = (inputs, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.users_sensitivelinkwarning2(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("users_sensitivelinkwarning2", locale)
	return en_users_sensitivelinkwarning2(inputs)
};
export { users_sensitivelinkwarning2 as "users_sensitiveLinkWarning" }