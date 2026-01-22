import type { components } from '$lib/api/schema';

// Re-export generated types
// DaemonResponse includes computed version_status from the API
export type Daemon = components['schemas']['DaemonResponse'];
export type DaemonBase = components['schemas']['DaemonBase'];
export type DaemonMode = components['schemas']['DaemonMode'];
export type DaemonCapabilities = components['schemas']['DaemonCapabilities'];

// Version-related types
export type DaemonVersionStatus = components['schemas']['DaemonVersionStatus'];
export type VersionHealthStatus = components['schemas']['VersionHealthStatus'];
export type DeprecationWarning = components['schemas']['DeprecationWarning'];
export type DeprecationSeverity = components['schemas']['DeprecationSeverity'];

// Provisioning types (for ServerPoll mode)
export type ProvisionDaemonRequest = components['schemas']['ProvisionDaemonRequest'];
export type ProvisionDaemonResponse = components['schemas']['ProvisionDaemonResponse'];
