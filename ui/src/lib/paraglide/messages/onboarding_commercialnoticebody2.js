/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_onboarding_commercialnoticebody2 = /** @type {(inputs: {}) => LocalizedString} */ () => {
	return /** @type {LocalizedString} */ (`Scanopy Community is licensed under AGPL-3.0. For commercial use, please ensure your use complies with the license terms, or consider our <a href="https://scanopy.net/pricing" class="underline hover:no-underline">commercial self-hosted offering</a>.`)
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
const onboarding_commercialnoticebody2 = (inputs = {}, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.onboarding_commercialnoticebody2(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("onboarding_commercialnoticebody2", locale)
	return en_onboarding_commercialnoticebody2(inputs)
};
export { onboarding_commercialnoticebody2 as "onboarding_commercialNoticeBody" }