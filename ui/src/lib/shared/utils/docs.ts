/**
 * The public documentation site. Metadata fixtures carry docs links as root-relative paths
 * (`/docs/guides/integrations/snmp/`) because the website consumes them that way; the app runs on
 * a different origin and composes them onto this one.
 */
export const DOCS_ORIGIN = 'https://scanopy.net';

/** Absolute URL for a root-relative docs path taken from metadata. */
export function docsUrl(path: string): string {
	return `${DOCS_ORIGIN}${path}`;
}
