/* eslint-disable */
import { getLocale, trackMessageCall, experimentalMiddlewareLocaleSplitting, isServer } from '../runtime.js';
/** @typedef {import('../runtime.js').LocalizedString} LocalizedString */

const en_daemons_config_usenpcaparphelp3 = /** @type {(inputs: {}) => LocalizedString} */ () => {
	return /** @type {LocalizedString} */ (`Enable faster ARP scanning on Windows by using broadcast ARP via Npcap instead of native SendARP, which doesn't support broadcast. **Requires Npcap installation**. Ignored on Linux/macOS`)
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
const daemons_config_usenpcaparphelp3 = (inputs = {}, options = {}) => {
	if (experimentalMiddlewareLocaleSplitting && isServer === false) {
		return /** @type {any} */ (globalThis).__paraglide_ssr.daemons_config_usenpcaparphelp3(inputs) 
	}
	const locale = options.locale ?? getLocale()
	trackMessageCall("daemons_config_usenpcaparphelp3", locale)
	return en_daemons_config_usenpcaparphelp3(inputs)
};
export { daemons_config_usenpcaparphelp3 as "daemons_config_useNpcapArpHelp" }