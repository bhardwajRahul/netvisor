//! Self-report phase: daemon reports itself as a host on the network.
//!
//! Runs on first discovery only. Creates the daemon host with its ip_addresses,
//! Scanopy service, and bindings on bound subnets.

use std::net::{IpAddr, Ipv4Addr};

use anyhow::{Error, Result};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::discovery::service::base::DiscoveryRunner;
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::DaemonUtils;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::ip_addresses::r#impl::base::{ALL_IP_ADDRESSES_IP, IPAddress};
use crate::server::ports::r#impl::base::Port;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::scanopy_daemon::ScanopyDaemon;
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::MatchDetails;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::Subnet;

impl DiscoveryRunner {
    /// Self-report phase: detect ip_addresses, create daemon host with Scanopy service.
    /// Only runs on first discovery (is_first_run check in caller).
    pub(super) async fn run_self_report_phase(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        let network_id = self
            .service
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let utils = &self.service.utils;
        let host_id = self.host_id;

        let binding_address = self.service.config_store.get_bind_address().await?;
        let binding_ip = IpAddr::V4(binding_address.parse::<Ipv4Addr>()?);

        // Get ip_addresses
        let interface_filter = self.service.config_store.get_interfaces().await?;
        let (ip_addresses, _, _) = utils
            .get_own_interfaces(network_id, &interface_filter)
            .await?;

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Filter ip_addresses to those with matching created subnets
        let ip_addresses: Vec<IPAddress> = ip_addresses
            .into_iter()
            .filter_map(|mut i| {
                if let Some(subnet) = created_subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&i.base.ip_address))
                {
                    i.base.subnet_id = subnet.id;
                    return Some(i);
                }
                None
            })
            .collect();

        let daemon_bound_subnet_ids: Vec<Uuid> =
            if binding_address == ALL_IP_ADDRESSES_IP.to_string() {
                created_subnets.iter().map(|s| s.id).collect()
            } else {
                created_subnets
                    .iter()
                    .filter(|s| s.base.cidr.contains(&binding_ip))
                    .map(|s| s.id)
                    .collect()
            };

        let own_port = Port::new_hostless(PortType::new_tcp(
            self.service.config_store.get_port().await?,
        ));
        let own_port_id = own_port.id;
        let local_ip = utils.get_own_ip_address()?;
        let hostname = utils.get_own_hostname();

        let host_base = HostBase {
            name: hostname.clone().unwrap_or(format!("{}", local_ip)),
            hostname,
            network_id,
            description: Some("Scanopy daemon".to_string()),
            tags: Vec::new(),
            source: EntitySource::Discovery,
            hidden: false,
            virtualization: None,
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments: vec![],
        };

        let mut host = Host::new(host_base);
        host.id = host_id;

        let daemon_service_definition = ScanopyDaemon;
        let daemon_service_bound_interfaces: Vec<&IPAddress> = ip_addresses
            .iter()
            .filter(|i| daemon_bound_subnet_ids.contains(&i.base.subnet_id))
            .collect();

        let daemon_service = Service::new(ServiceBase {
            name: ServiceDefinition::name(&daemon_service_definition).to_string(),
            service_definition: Box::new(daemon_service_definition),
            tags: Vec::new(),
            network_id,
            bindings: daemon_service_bound_interfaces
                .iter()
                .map(|i| Binding::new_port_serviceless(own_port_id, Some(i.id)))
                .collect(),
            host_id: host.id,
            virtualization: None,
            source: EntitySource::DiscoveryWithMatch {
                details: MatchDetails::new_certain("Scanopy Daemon self-report"),
            },
            position: 0,
        });

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        ops.create_host(
            host,
            ip_addresses.clone(),
            vec![own_port],
            vec![daemon_service],
            vec![],
            vec![],
            // Self-report carries no ifTable; empty interface set, nothing to prune.
            true,
            cancel,
        )
        .await?;

        Ok(())
    }
}
