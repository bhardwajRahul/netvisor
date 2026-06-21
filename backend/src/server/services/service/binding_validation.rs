//! Binding validation, conflict detection, and deduplication helpers.
use super::*;

impl ServiceService {
    /// Validate that all bindings reference ports/interfaces that belong to the service's host.
    /// Returns Ok(()) if all bindings are valid, Err with ValidationError if any are invalid.
    pub(crate) async fn validate_bindings_belong_to_host(
        &self,
        host_id: &Uuid,
        bindings: &[Binding],
    ) -> Result<()> {
        if bindings.is_empty() {
            return Ok(());
        }

        let host_service = self
            .host_service
            .get()
            .expect("host_service not initialized");

        // Get all ports and ip_addresses for this host
        let host_ports = host_service.get_ports_for_host(host_id).await?;
        let host_interfaces = host_service.get_ip_addresses_for_host(host_id).await?;

        let valid_port_ids: std::collections::HashSet<Uuid> =
            host_ports.iter().map(|p| p.id).collect();
        let valid_ip_address_ids: std::collections::HashSet<Uuid> =
            host_interfaces.iter().map(|i| i.id).collect();

        for binding in bindings {
            match &binding.base.binding_type {
                BindingType::IPAddress { ip_address_id } => {
                    if !valid_ip_address_ids.contains(ip_address_id) {
                        return Err(ValidationError::new(format!(
                            "IP address binding references ip_address {} which does not belong to this host",
                            ip_address_id
                        )).into());
                    }
                }
                BindingType::Port {
                    port_id,
                    ip_address_id,
                } => {
                    if !valid_port_ids.contains(port_id) {
                        return Err(ValidationError::new(format!(
                            "Port binding references port {} which does not belong to this host",
                            port_id
                        ))
                        .into());
                    }
                    if let Some(iface_id) = ip_address_id
                        && !valid_ip_address_ids.contains(iface_id)
                    {
                        return Err(ValidationError::new(format!(
                                "Port binding references ip_address {} which does not belong to this host",
                                iface_id
                            )).into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a new binding is already covered by existing bindings.
    /// A Port binding with a specific interface is covered if there's already
    /// a Port binding for the same port with ip_address_id = None (all ip_addresses).
    pub(crate) fn is_binding_covered_by_existing(
        new_binding: &Binding,
        existing_bindings: &[Binding],
    ) -> bool {
        match &new_binding.base.binding_type {
            // A Port binding with a specific interface is covered by an "all ip_addresses" binding for the same port
            BindingType::Port {
                port_id,
                ip_address_id: Some(_),
            } => existing_bindings.iter().any(|existing| {
                matches!(
                    &existing.base.binding_type,
                    BindingType::Port {
                        port_id: existing_port_id,
                        ip_address_id: None,
                    } if existing_port_id == port_id
                )
            }),
            // Other binding types are not covered by anything else
            _ => false,
        }
    }

    /// Validates that a binding doesn't conflict with existing bindings.
    /// Rules:
    /// - Interface binding conflicts with port bindings on same interface OR port bindings on all ip_addresses
    /// - Port binding (specific ip_address) conflicts with interface binding on same interface
    /// - Port binding (all ip_addresses) conflicts with ANY interface binding
    ///
    /// Returns None if valid, Some(error_message) if conflict found.
    pub(crate) fn validate_binding_no_conflict(
        new_binding: &BindingType,
        existing_bindings: &[Binding],
    ) -> Option<&'static str> {
        match new_binding {
            BindingType::IPAddress { ip_address_id } => {
                // Check for conflicting port bindings: same interface OR all-interfaces
                for existing in existing_bindings {
                    if let BindingType::Port {
                        ip_address_id: existing_iface,
                        ..
                    } = &existing.base.binding_type
                        && (*existing_iface == Some(*ip_address_id) || existing_iface.is_none())
                    {
                        return Some(
                            "Cannot add ip_address binding: service already has a port binding on this ip_address (or on all ip_addresses).",
                        );
                    }
                }
            }
            BindingType::Port {
                ip_address_id: Some(ip_address_id),
                ..
            } => {
                // Check for conflicting interface binding on same interface
                for existing in existing_bindings {
                    if let BindingType::IPAddress {
                        ip_address_id: existing_iface,
                    } = &existing.base.binding_type
                        && existing_iface == ip_address_id
                    {
                        return Some(
                            "Cannot add port binding: service already has an ip_address binding on this ip_address.",
                        );
                    }
                }
            }
            BindingType::Port {
                ip_address_id: None,
                ..
            } => {
                // Port binding on all ip_addresses: conflicts with ANY interface binding
                for existing in existing_bindings {
                    if matches!(existing.base.binding_type, BindingType::IPAddress { .. }) {
                        return Some(
                            "Cannot add port binding on all ip_addresses: service already has ip_address bindings.",
                        );
                    }
                }
            }
        }
        None
    }

    /// Deduplicate bindings in a list.
    /// - Removes exact duplicates (same binding_type)
    /// - When an all-interfaces port binding is present, removes specific-interface bindings for the same port
    pub(crate) fn deduplicate_bindings(bindings: Vec<Binding>) -> Vec<Binding> {
        use std::collections::HashSet;

        // First, collect all port_ids that have all-interfaces bindings
        let all_interface_port_ids: HashSet<Uuid> = bindings
            .iter()
            .filter_map(|b| {
                if let BindingType::Port {
                    port_id,
                    ip_address_id: None,
                } = &b.base.binding_type
                {
                    Some(*port_id)
                } else {
                    None
                }
            })
            .collect();

        // Track seen binding types for deduplication
        let mut seen_binding_types: HashSet<String> = HashSet::new();
        let mut result = Vec::new();

        for binding in bindings {
            // Skip specific-interface port bindings when an all-interfaces binding exists for the same port
            if let BindingType::Port {
                port_id,
                ip_address_id: Some(_),
            } = &binding.base.binding_type
                && all_interface_port_ids.contains(port_id)
            {
                tracing::debug!(
                    port_id = %port_id,
                    "Deduplicating specific-ip_address binding superseded by all-ip_addresses binding"
                );
                continue;
            }

            // Create a key for deduplication based on binding type
            let key = format!("{:?}", binding.base.binding_type);
            if seen_binding_types.contains(&key) {
                tracing::debug!(
                    binding_type = %key,
                    "Deduplicating duplicate binding"
                );
                continue;
            }

            seen_binding_types.insert(key);
            result.push(binding);
        }

        result
    }

    /// Validate all bindings in a list don't conflict with each other.
    /// Returns Ok(()) if all bindings are valid, Err with message if any conflict.
    pub(crate) fn validate_bindings_no_conflicts(bindings: &[Binding]) -> Result<()> {
        for (i, binding) in bindings.iter().enumerate() {
            // Check against all bindings before this one (to avoid duplicate checks)
            let preceding_bindings = &bindings[..i];
            if let Some(error_msg) =
                Self::validate_binding_no_conflict(&binding.base.binding_type, preceding_bindings)
            {
                return Err(ValidationError::new(error_msg).into());
            }
        }
        Ok(())
    }
}
