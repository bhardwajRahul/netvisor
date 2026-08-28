/**
 * Range arithmetic on CIDRs, for the places the UI has to reason about one range against another.
 *
 * `ipaddr.js` is already a dependency and already used for the address validators; this only gives
 * its `parseCIDR`/`match` pair the shape the callers want, so nobody hand-rolls prefix maths.
 */

import { parseCIDR, isValidCIDR } from 'ipaddr.js';

/**
 * Whether `outer` covers every address in `inner`. A range contains itself.
 *
 * Both halves are needed: `match` alone only says the inner network address falls inside `outer`,
 * which is also true when `inner` is the *wider* of the two and merely starts in the same place —
 * `10.20.30.0/23` "matches" `10.20.30.0/24` in both directions. The prefix comparison is what makes
 * the answer directional.
 *
 * Returns `false` rather than throwing for anything unparseable or for a v4/v6 pair, since callers
 * are asking a yes/no question about data they did not author.
 */
export function cidrContains(outer: string, inner: string): boolean {
	if (!isValidCIDR(outer) || !isValidCIDR(inner)) return false;
	try {
		const [outerAddr, outerPrefix] = parseCIDR(outer);
		const [innerAddr, innerPrefix] = parseCIDR(inner);
		if (outerAddr.kind() !== innerAddr.kind()) return false;
		return outerPrefix <= innerPrefix && innerAddr.match([outerAddr, outerPrefix]);
	} catch {
		return false;
	}
}
