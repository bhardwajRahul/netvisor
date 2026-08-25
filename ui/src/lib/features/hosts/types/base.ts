// Re-export generated types from OpenAPI schema
import type { components } from '$lib/api/schema';

// Entity primitive types
export type Host = components['schemas']['Host'];
export type HostVirtualization = components['schemas']['HostVirtualization'];
export type ProxmoxVirtualization = components['schemas']['ProxmoxVirtualization'];
export type IPAddress = components['schemas']['IPAddress'];
export type Interface = components['schemas']['Interface'];
export type Port = components['schemas']['Port'];
export type Service = components['schemas']['Service'];
export type TransportProtocol = components['schemas']['TransportProtocol'];

// API response type (host with hydrated children)
export type HostResponse = components['schemas']['HostResponse'];

// API request types - consolidated input types (used for both create and update)
export type CreateHostRequest = components['schemas']['CreateHostRequest'];
export type UpdateHostRequest = components['schemas']['UpdateHostRequest'];
export type IPAddressInput = components['schemas']['IPAddressInput'];
export type PortInput = components['schemas']['PortInput'];
export type ServiceInput = components['schemas']['ServiceInput'];
export type BindingInput = components['schemas']['BindingInput'];

// SNMP types
export type IfAdminStatus = components['schemas']['IfAdminStatus'];
export type IfOperStatus = components['schemas']['IfOperStatus'];

// Credential assignment for a host, optionally limited to specific IP addresses
export interface CredentialAssignment {
	credential_id: string;
	/** IP address IDs to limit this credential to. null = all host IP addresses. */
	ip_address_ids: string[] | null;
}

// Form state type for creating/editing hosts
// Includes children arrays for form editing - distinct from HostResponse (API response type)
export interface HostFormData {
	// Host primitive fields
	id: string;
	created_at: string;
	updated_at: string;
	name: string;
	network_id: string;
	hostname: string | null;
	description: string | null;
	source: components['schemas']['EntitySource'];
	virtualization_metadata: HostVirtualization | null;
	virtualization_service_id: string | null;
	hidden: boolean;
	tags: string[];

	// SNMP fields (populated by discovery, read-only in UI)
	sys_descr: string | null;
	sys_object_id: string | null;
	sys_location: string | null;
	sys_contact: string | null;
	management_url: string | null;
	chassis_id: string | null;
	sys_name: string | null;

	// Hardware identity (ENTITY-MIB or a controller integration, read-only in UI)
	manufacturer: string | null;
	model: string | null;
	serial_number: string | null;

	// Credential assignments (user-editable, from junction table)
	credential_assignments: CredentialAssignment[];

	// Children for form editing (managed separately from host in stores)
	ip_addresses: IPAddress[];
	ports: Port[];
	services: Service[];

	// Interface list (populated by discovery, read-only)
	interfaces: Interface[];
}

// Request type for creating a host (needs form data with children)
export interface CreateHostWithServicesRequest {
	host: HostFormData;
	services: Service[] | null;
}

// Request type for updating a host with children
export interface UpdateHostWithServicesRequest {
	host: Host;
	/** IP addresses to sync - if provided, will create/update/delete to match */
	ip_addresses: IPAddress[] | null;
	/** Ports to sync - if provided, will create/update/delete to match */
	ports: Port[] | null;
	/** Services to sync - if provided, will create/update/delete to match */
	services: Service[] | null;
}

// Frontend-specific types
export interface AllIPAddresses {
	id: null;
	name: string;
}

export const ALL_IP_ADDRESSES: AllIPAddresses = {
	id: null,
	name: 'All IP Addresses'
};
