import type { components } from '$lib/api/schema';

// Re-export generated types
// DaemonResponse includes computed version_status from the API
export type Daemon = components['schemas']['DaemonResponse'];
export type DaemonBase = components['schemas']['DaemonBase'];
export type DaemonMode = components['schemas']['DaemonMode'];

// Version-related types
export type DaemonVersionStatus = components['schemas']['DaemonVersionStatus'];
export type VersionHealthStatus = components['schemas']['VersionHealthStatus'];
export type DeprecationWarning = components['schemas']['DeprecationWarning'];
export type DeprecationSeverity = components['schemas']['DeprecationSeverity'];

// Provisioning types (for ServerPoll mode)
export type ProvisionDaemonRequest = components['schemas']['ProvisionDaemonRequest'];
export type ProvisionDaemonResponse = components['schemas']['ProvisionDaemonResponse'];

// Install-command builder types
export type InstallArtifacts = components['schemas']['InstallArtifacts'];
export type InstallCommandKind = components['schemas']['InstallCommandKind'];

/**
 * Stand-in the server emits for the daemon api key (it never mints, so it doesn't know the
 * plaintext). The frontend substitutes the key it holds from the provision response. Must match
 * `API_KEY_PLACEHOLDER` in backend/src/server/daemons/impl/install_artifacts.rs.
 */
export const API_KEY_PLACEHOLDER = '<API_KEY>';

/** Fill the api-key placeholder in every emitted command + the compose file. */
export function fillInstallArtifactsKey(
	artifacts: InstallArtifacts,
	apiKey: string
): InstallArtifacts {
	const sub = (s: string) => s.replaceAll(API_KEY_PLACEHOLDER, apiKey);
	return {
		...artifacts,
		commands: artifacts.commands.map((c) => ({ ...c, command: sub(c.command) })),
		docker_compose: artifacts.docker_compose
			? sub(artifacts.docker_compose)
			: artifacts.docker_compose
	};
}
