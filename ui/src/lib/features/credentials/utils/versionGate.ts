import { coerce, lt } from 'semver';

/**
 * Whether a daemon at `daemonVersion` is too old to receive a credential type
 * whose minimum-daemon-version floor is `minVersion`.
 *
 * `coerce` tolerates partial/odd version strings; a missing daemon version or a
 * missing floor means there is nothing to gate on (returns false). Used by the
 * discovery credential picker and wizard to disable too-new credential types.
 */
export function daemonTooOldForCredential(
	minVersion: string | undefined | null,
	daemonVersion: string | undefined | null
): boolean {
	if (!minVersion || !daemonVersion) return false;
	const dv = coerce(daemonVersion);
	return dv ? lt(dv, minVersion) : false;
}
