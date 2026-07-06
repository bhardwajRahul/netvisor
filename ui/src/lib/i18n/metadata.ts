/**
 * Translation utilities for backend-metadata fixture strings.
 *
 * Fixture files in $lib/data/*.json ship English names/descriptions generated
 * from the backend. `scripts/generate-meta-messages.js` syncs those strings
 * into messages/en.json as `meta_<fixtureKey>_<id>_*` keys; this module
 * resolves them dynamically (mirroring $lib/i18n/errors.ts), falling back to
 * the raw fixture string when no translation exists.
 */

import * as m from '$lib/paraglide/messages';

/** Minimal structural shape shared by credential and scan-setting field definitions. */
interface TranslatableField {
	id: string;
	label: string;
	placeholder?: string;
	help_text?: string;
	options?: { value: string; label: string }[];
}

function resolveMeta(key: string, fallback: string): string {
	const messageFn = m[key as keyof typeof m] as (() => string) | undefined;

	if (typeof messageFn === 'function') {
		try {
			return messageFn();
		} catch {
			// If resolution fails, fall through to fallback
		}
	}

	return fallback;
}

/** Resolve a fixture item's translated name, falling back to the fixture string. */
export function metaName(fixtureKey: string, id: string | null, fallback: string): string {
	if (!id) return fallback;
	return resolveMeta(`meta_${fixtureKey}_${id}_name`, fallback);
}

/** Resolve a fixture item's translated description, falling back to the fixture string. */
export function metaDescription(fixtureKey: string, id: string | null, fallback: string): string {
	if (!id) return fallback;
	return resolveMeta(`meta_${fixtureKey}_${id}_description`, fallback);
}

/**
 * Resolve translated label/placeholder/help_text/option labels for field
 * definitions. `ownerId` namespaces nested fields (credential type id);
 * pass null for flat field-definition fixtures like scan-settings.
 *
 * Key shape mirrors scripts/generate-meta-messages.js:
 *   meta_<fixtureKey>[_<ownerId>]_<fieldId>_label / _placeholder / _helpText
 *   meta_<fixtureKey>[_<ownerId>]_<fieldId>_option_<value>
 */
export function translateFieldDefinitions<F extends TranslatableField>(
	fixtureKey: string,
	ownerId: string | null,
	fields: F[]
): F[] {
	const namespace = ownerId ? `meta_${fixtureKey}_${ownerId}` : `meta_${fixtureKey}`;
	return fields.map((field) => {
		const prefix = `${namespace}_${field.id}`;
		return {
			...field,
			label: resolveMeta(`${prefix}_label`, field.label),
			placeholder: field.placeholder
				? resolveMeta(`${prefix}_placeholder`, field.placeholder)
				: field.placeholder,
			help_text: field.help_text
				? resolveMeta(`${prefix}_helpText`, field.help_text)
				: field.help_text,
			options: field.options?.map((option) => ({
				...option,
				label: resolveMeta(`${prefix}_option_${option.value}`, option.label)
			}))
		};
	});
}
