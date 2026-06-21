//! Service interface-binding reassignment on host change.
use super::*;

impl ServiceService {
    /// Update bindings to match ports and ip_addresses available on new host
    /// `original_interfaces` and `updated_interfaces` are the ip_addresses for the respective hosts
    /// `original_ports` and `updated_ports` are the ports for the respective hosts
    #[allow(clippy::too_many_arguments)]
    pub async fn reassign_service_interface_bindings(
        &self,
        service: Service,
        original_host: &Host,
        original_interfaces: &[IPAddress],
        original_ports: &[Port],
        updated_host: &Host,
        updated_interfaces: &[IPAddress],
        updated_ports: &[Port],
        interface_id_remap: &std::collections::HashMap<Uuid, Uuid>,
    ) -> Service {
        let lock = self.get_service_lock(&service.id).await;
        let _guard = lock.lock().await;

        tracing::trace!(
            "Preparing service {:?} for transfer from host {:?} to host {:?}",
            service,
            original_host,
            updated_host
        );

        let mut mutable_service = service.clone();

        let service_name = service.base.name.clone();
        let service_id = service.id;

        mutable_service.base.bindings = mutable_service
            .base
            .bindings
            .iter_mut()
            .filter_map(|b| {
                // Look up original interface from the provided slice
                let original_interface = b
                    .ip_address_id()
                    .and_then(|id| original_interfaces.iter().find(|i| i.id == id));

                match &mut b.base.binding_type {
                    BindingType::IPAddress { ip_address_id } => {
                        if let Some(original_ip_address) = original_interface {
                            let new_ip_address: Option<&IPAddress> =
                                updated_interfaces.iter().find(|i| *i == original_ip_address);

                            if let Some(new_ip_address) = new_ip_address {
                                *ip_address_id = new_ip_address.id;
                                return Some(*b);
                            }
                        }
                        // Structural match failed — try direct ID remap (handles subnet_id
                        // mismatch on second scan where scanner subnet UUIDs differ from DB)
                        if let Some(&new_id) = interface_id_remap.get(ip_address_id) {
                            *ip_address_id = new_id;
                            return Some(*b);
                        }
                        // Interface binding couldn't be matched - this can happen during consolidation
                        // when the source host's interface doesn't exist on the destination host.
                        // We drop the binding and warn.
                        tracing::warn!(
                            service_id = %service_id,
                            service_name = %service_name,
                            original_interface_id = ?b.ip_address_id(),
                            "Dropping ip_address binding during reassignment: \
                             no matching ip_address found on destination host"
                        );
                        None::<Binding>
                    }
                    BindingType::Port {
                        port_id,
                        ip_address_id,
                    } => {
                        if let Some(original_port) =
                            original_ports.iter().find(|p| p.id == *port_id)
                            && let Some(new_port) =
                                updated_ports.iter().find(|p| *p == original_port)
                        {
                            let new_ip_address: Option<Option<IPAddress>> = match original_interface
                            {
                                // None interface = listen on all ip_addresses, assume same for new host
                                None => Some(None),
                                Some(original_ip_address) => {
                                    match updated_interfaces
                                        .iter()
                                        .find(|i| *i == original_ip_address)
                                    {
                                        Some(found_ip_address) => {
                                            Some(Some(found_ip_address.clone()))
                                        }
                                        None => {
                                            // Structural match failed — try direct ID remap
                                            // (handles subnet_id mismatch on second scan)
                                            if let Some(&new_id) = interface_id_remap.get(&original_ip_address.id) {
                                                if let Some(found) = updated_interfaces.iter().find(|i| i.id == new_id) {
                                                    Some(Some(found.clone()))
                                                } else {
                                                    tracing::warn!(
                                                        service_id = %service_id,
                                                        service_name = %service_name,
                                                        port_number = %new_port.base.port_type.config().number,
                                                        remapped_interface_id = %new_id,
                                                        "Port binding ip_address remap target not found - \
                                                         falling back to 'all ip_addresses'"
                                                    );
                                                    Some(None)
                                                }
                                            } else {
                                                tracing::warn!(
                                                    service_id = %service_id,
                                                    service_name = %service_name,
                                                    port_number = %new_port.base.port_type.config().number,
                                                    original_interface_ip = %original_ip_address.base.ip_address,
                                                    "Port binding ip_address not found on destination host - \
                                                     falling back to 'all ip_addresses'"
                                                );
                                                Some(None)
                                            }
                                        }
                                    }
                                }
                            };

                            match new_ip_address {
                                None => return None,
                                Some(new_ip_address) => {
                                    *port_id = new_port.id;
                                    *ip_address_id = match new_ip_address {
                                        Some(new_ip_address) => Some(new_ip_address.id),
                                        None => None,
                                    };
                                    return Some(*b);
                                }
                            }
                        }
                        // Port not found on destination host - drop the binding
                        tracing::warn!(
                            service_id = %service_id,
                            service_name = %service_name,
                            original_port_id = %port_id,
                            "Dropping port binding during reassignment: \
                             no matching port found on destination host"
                        );
                        None::<Binding>
                    }
                };

                None
            })
            .collect();

        mutable_service.base.host_id = updated_host.id;

        mutable_service.base.network_id = updated_host.base.network_id;

        tracing::trace!(
            "Reassigned service {:?} bindings for from host {:?} to host {:?}",
            mutable_service,
            original_host,
            updated_host
        );

        mutable_service
    }
}
