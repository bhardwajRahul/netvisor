import type { components } from '$lib/api/schema';
import { credentialTypes, entities } from '$lib/shared/stores/metadata';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

export type Credential = components['schemas']['Credential'];
export type CredentialBase = components['schemas']['CredentialBase'];
export type CredentialType = components['schemas']['CredentialType'];
export type CredentialOrderField = components['schemas']['CredentialOrderField'];
/** Which integrations run on a daemon, and against which hosts. */
export type IntegrationTarget = components['schemas']['IntegrationTarget'];
/** Release maturity of a credential type's integration. Derived from the backend enum. */
export type CredentialStability = components['schemas']['CredentialStability'];

// Re-export SNMP types still used by other features (Interface display, etc.)
export type Interface = components['schemas']['Interface'];
export type IfAdminStatus = components['schemas']['IfAdminStatus'];
export type IfOperStatus = components['schemas']['IfOperStatus'];

import type { Color } from '$lib/shared/utils/styling';
import type { TagProps } from '$lib/shared/components/data/types';
import {
	common_beta,
	common_network,
	common_testing,
	common_unknown,
	credentials_betaTooltip,
	credentials_targetNetworkTooltip,
	credentials_targetDaemonHost,
	credentials_targetDaemonHostTooltip,
	credentials_targetHost,
	credentials_targetHostTooltip,
	snmp_adminStatusDown,
	snmp_adminStatusUp,
	snmp_operStatusDormant,
	snmp_operStatusDown,
	snmp_operStatusLowerLayerDown,
	snmp_operStatusNotPresent,
	snmp_operStatusUp
} from '$lib/paraglide/messages';

/**
 * Create a default credential with the given organization ID.
 * Defaults to SNMPv2c type.
 */
export function createDefaultCredential(organization_id: string): Credential {
	return {
		name: '',
		credential_type: {
			type: 'SnmpV2c',
			community: { mode: 'Inline' as const, value: '' }
		},
		organization_id,
		tags: [],
		assigned_network_ids: [],
		host_assignments: [],
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel
	};
}

/**
 * Get the type discriminant from a credential's credential_type.
 */
export function getCredentialTypeId(credential: Credential): string {
	return credential.credential_type.type;
}

/**
 * Get a summary of non-secret fields for display on cards.
 */
export function getCredentialSummary(credential: Credential): string {
	const ct = credential.credential_type;
	switch (ct.type) {
		case 'SnmpV2c':
			return '161/udp';
		case 'DockerProxy':
		case 'PodmanProxy':
			return `Port ${ct.port ?? 2376}`;
		default:
			return '';
	}
}

/**
 * Get the associated service name for display in credential lists and popovers.
 * Returns the service name from the associated ServiceDefinition (e.g. "SNMP", "Docker").
 */
export function getCredentialDescription(credential: Credential): string {
	const ct = credential.credential_type;
	return credentialTypes.getDescription(ct.type);
}

/**
 * Get human-readable labels for SNMP admin status
 */
export function getAdminStatusLabels(): Record<IfAdminStatus, string> {
	return {
		Up: snmp_adminStatusUp(),
		Down: snmp_adminStatusDown(),
		Testing: common_testing()
	};
}

/**
 * Get human-readable labels for SNMP operational status
 */
export function getOperStatusLabels(): Record<IfOperStatus, string> {
	return {
		Up: snmp_operStatusUp(),
		Down: snmp_operStatusDown(),
		Testing: common_testing(),
		Unknown: common_unknown(),
		Dormant: snmp_operStatusDormant(),
		NotPresent: snmp_operStatusNotPresent(),
		LowerLayerDown: snmp_operStatusLowerLayerDown()
	};
}

/**
 * Single source of truth for target display properties (color, label, tooltip).
 * Targets are the unified replacement for scope models.
 *
 * A scope chip, coloured by the entity it actually reaches.
 *
 * Each scope names a real entity, so it borrows that entity's colour rather
 * than an arbitrary one: a network scope reads as a network, a daemon-host
 * scope as a daemon, and a remote-host scope as a host. That way a scope means
 * the same colour here as the thing it points at does everywhere else.
 */
export function getTargetTagProps(target: string): TagProps {
	if (target === 'Network') {
		return {
			label: common_network(),
			color: entities.getColorHelper('Network').color,
			title: credentials_targetNetworkTooltip()
		};
	}
	if (target === 'DaemonHost') {
		return {
			label: credentials_targetDaemonHost(),
			color: entities.getColorHelper('Daemon').color,
			title: credentials_targetDaemonHostTooltip()
		};
	}
	return {
		label: credentials_targetHost(),
		color: entities.getColorHelper('Host').color,
		title: credentials_targetHostTooltip()
	};
}

/**
 * Tag marking a credential type whose integration is still in beta, or `null` for stable
 * types. Beta is presentation only — the type stays selectable and usable; it is flagged so
 * users know the integration is unvalidated and its fields may still change.
 *
 * Lives here rather than in the template so the i18n lookup sits next to `getTargetTagProps`,
 * the other producer of credential-type tags.
 */
export function getStabilityTagProps(stability: CredentialStability | undefined): TagProps | null {
	if (stability !== 'Beta') return null;
	return {
		label: common_beta(),
		color: 'Amber' as Color,
		title: credentials_betaTooltip()
	};
}

/**
 * A credential type that applies *only* to the daemon's own host (e.g. the Docker/Podman
 * socket). Derived from the target scheme — this replaces the former `is_local_auto` flag.
 * Such types target only the daemon host (a `<uuid>@127.0.0.1` token) — no remote/network scope.
 */
export function isDaemonHostOnly(targets: string[] | undefined): boolean {
	return targets?.length === 1 && targets[0] === 'DaemonHost';
}

/**
 * Human-readable labels for SNMP admin status
 * @deprecated Use getAdminStatusLabels() instead for proper i18n support
 */
export const ADMIN_STATUS_LABELS: Record<IfAdminStatus, string> = {
	Up: 'Admin Up',
	Down: 'Admin Down',
	Testing: 'Testing'
};

/**
 * Human-readable labels for SNMP operational status
 * @deprecated Use getOperStatusLabels() instead for proper i18n support
 */
export const OPER_STATUS_LABELS: Record<IfOperStatus, string> = {
	Up: 'Up',
	Down: 'Down',
	Testing: 'Testing',
	Unknown: 'Unknown',
	Dormant: 'Dormant',
	NotPresent: 'Not Present',
	LowerLayerDown: 'Lower Layer Down'
};
