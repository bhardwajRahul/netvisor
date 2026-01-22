use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio::time::Instant;
use uuid::Uuid;

use crate::{
    daemon::runtime::state::BufferedEntities,
    server::{
        hosts::r#impl::{api::DiscoveryHostRequest, api::HostResponse, base::Host},
        subnets::r#impl::base::Subnet,
    },
};

/// Entity state in the buffer - tracks lifecycle from discovery to server confirmation.
#[derive(Clone, Debug)]
pub enum BufferedEntity<T> {
    /// Discovered by daemon, not yet confirmed by server.
    Pending(T),
    /// Confirmed by server with actual data (may have different ID after deduplication).
    Created { pending_id: Uuid, actual: T },
}

impl<T> BufferedEntity<T> {
    pub fn is_pending(&self) -> bool {
        matches!(self, BufferedEntity::Pending(_))
    }

    pub fn is_created(&self) -> bool {
        matches!(self, BufferedEntity::Created { .. })
    }

    pub fn get_data(&self) -> &T {
        match self {
            BufferedEntity::Pending(t) => t,
            BufferedEntity::Created { actual, .. } => actual,
        }
    }
}

/// Thread-safe buffer for accumulating discovered entities with lifecycle tracking.
///
/// In both modes, discovery adds entities to this buffer. The flush mechanism differs:
/// - **DaemonPoll**: Entities are immediately sent to server and marked as Created
/// - **ServerPoll**: Server polls pending entities and responds with Created confirmations
pub struct EntityBuffer {
    /// Subnets keyed by daemon-generated ID for lookup
    subnets: Arc<RwLock<HashMap<Uuid, BufferedEntity<Subnet>>>>,
    /// Hosts keyed by daemon-generated ID
    hosts: Arc<RwLock<HashMap<Uuid, BufferedEntity<DiscoveryHostRequest>>>>,
}

impl EntityBuffer {
    pub fn new() -> Self {
        Self {
            subnets: Arc::new(RwLock::new(HashMap::new())),
            hosts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ========================================================================
    // Subnet methods
    // ========================================================================

    /// Add a discovered subnet (pending state).
    pub async fn push_subnet(&self, subnet: Subnet) {
        let mut subnets = self.subnets.write().await;
        subnets.insert(subnet.id, BufferedEntity::Pending(subnet));
    }

    /// Mark subnet as created with actual server data.
    /// Returns the ID mapping if it changed (pending_id, actual_id).
    pub async fn mark_subnet_created(
        &self,
        pending_id: Uuid,
        actual: Subnet,
    ) -> Option<(Uuid, Uuid)> {
        let mut subnets = self.subnets.write().await;
        if let Some(entry) = subnets.get_mut(&pending_id) {
            let id_changed = pending_id != actual.id;
            *entry = BufferedEntity::Created {
                pending_id,
                actual: actual.clone(),
            };
            if id_changed {
                return Some((pending_id, actual.id));
            }
        }
        None
    }

    /// Get a subnet by its pending (daemon-generated) ID.
    /// Returns the actual server data if created, or pending data as fallback.
    pub async fn get_subnet(&self, pending_id: &Uuid) -> Option<Subnet> {
        let subnets = self.subnets.read().await;
        subnets.get(pending_id).map(|e| e.get_data().clone())
    }

    /// Wait for a subnet to be confirmed by server (with timeout).
    /// Returns None if timeout expires before confirmation.
    pub async fn await_subnet(&self, pending_id: &Uuid, timeout: Duration) -> Option<Subnet> {
        let deadline = Instant::now() + timeout;
        loop {
            {
                let subnets = self.subnets.read().await;
                if let Some(entry) = subnets.get(pending_id)
                    && entry.is_created()
                {
                    return Some(entry.get_data().clone());
                }
            }
            if Instant::now() > deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // ========================================================================
    // Host methods
    // ========================================================================

    /// Add a discovered host with its children (interfaces, ports, services).
    pub async fn push_host(&self, host: DiscoveryHostRequest) {
        let mut hosts = self.hosts.write().await;
        hosts.insert(host.host.id, BufferedEntity::Pending(host));
    }

    /// Mark host as created with actual server data.
    /// Accepts HostResponse (which includes children) and extracts the Host.
    pub async fn mark_host_created(&self, pending_id: Uuid, actual: HostResponse) {
        let mut hosts = self.hosts.write().await;
        if let Some(entry) = hosts.get_mut(&pending_id) {
            // Update the host in the request with the actual server data
            // Convert HostResponse to Host using the to_host() method
            if let BufferedEntity::Pending(req) = entry {
                let updated_req = DiscoveryHostRequest {
                    host: actual.to_host(),
                    // Use children from HostResponse if available, otherwise keep pending data
                    interfaces: if actual.interfaces.is_empty() {
                        req.interfaces.clone()
                    } else {
                        actual.interfaces
                    },
                    ports: if actual.ports.is_empty() {
                        req.ports.clone()
                    } else {
                        actual.ports
                    },
                    services: if actual.services.is_empty() {
                        req.services.clone()
                    } else {
                        actual.services
                    },
                };
                *entry = BufferedEntity::Created {
                    pending_id,
                    actual: updated_req,
                };
            }
        }
    }

    /// Get a host by its pending (daemon-generated) ID.
    pub async fn get_host(&self, pending_id: &Uuid) -> Option<DiscoveryHostRequest> {
        let hosts = self.hosts.read().await;
        hosts.get(pending_id).map(|e| e.get_data().clone())
    }

    /// Wait for a host to be confirmed by server (with timeout).
    pub async fn await_host(&self, pending_id: &Uuid, timeout: Duration) -> Option<Host> {
        let deadline = Instant::now() + timeout;
        loop {
            {
                let hosts = self.hosts.read().await;
                if let Some(entry) = hosts.get(pending_id)
                    && entry.is_created()
                {
                    return Some(entry.get_data().host.clone());
                }
            }
            if Instant::now() > deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // ========================================================================
    // Drain methods (for polling)
    // ========================================================================

    /// Drain all buffered entities and return them (clears the buffer).
    pub async fn drain(&self) -> BufferedEntities {
        let hosts = {
            let mut hosts = self.hosts.write().await;
            let drained: Vec<DiscoveryHostRequest> =
                hosts.drain().map(|(_, e)| e.get_data().clone()).collect();
            drained
        };

        let subnets = {
            let mut subnets = self.subnets.write().await;
            let drained: Vec<Subnet> = subnets.drain().map(|(_, e)| e.get_data().clone()).collect();
            drained
        };

        BufferedEntities { hosts, subnets }
    }

    /// Check if the buffer is empty.
    pub async fn is_empty(&self) -> bool {
        let hosts = self.hosts.read().await;
        let subnets = self.subnets.read().await;
        hosts.is_empty() && subnets.is_empty()
    }

    /// Get the count of buffered items without draining.
    pub async fn count(&self) -> (usize, usize) {
        let hosts = self.hosts.read().await;
        let subnets = self.subnets.read().await;
        (hosts.len(), subnets.len())
    }

    /// Get count of pending items only.
    pub async fn pending_count(&self) -> (usize, usize) {
        let hosts = self.hosts.read().await;
        let subnets = self.subnets.read().await;
        let pending_hosts = hosts.values().filter(|e| e.is_pending()).count();
        let pending_subnets = subnets.values().filter(|e| e.is_pending()).count();
        (pending_hosts, pending_subnets)
    }
}

impl Default for EntityBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        hosts::r#impl::base::{Host, HostBase},
        shared::types::entities::EntitySource,
    };

    #[tokio::test]
    async fn test_entity_buffer_push_and_drain() {
        let buffer = EntityBuffer::new();

        // Push a host
        let host = DiscoveryHostRequest {
            host: Host::new(HostBase {
                name: "test-host".to_string(),
                hostname: None,
                tags: vec![],
                network_id: Uuid::new_v4(),
                description: None,
                source: EntitySource::Manual,
                virtualization: None,
                hidden: false,
            }),
            interfaces: vec![],
            ports: vec![],
            services: vec![],
        };
        buffer.push_host(host).await;

        // Verify buffer has content
        assert!(!buffer.is_empty().await);
        assert_eq!(buffer.count().await, (1, 0));

        // Drain and verify
        let entities = buffer.drain().await;
        assert_eq!(entities.hosts.len(), 1);
        assert!(entities.subnets.is_empty());

        // Verify buffer is empty after drain
        assert!(buffer.is_empty().await);
    }

    #[tokio::test]
    async fn test_entity_buffer_concurrent_access() {
        let buffer = Arc::new(EntityBuffer::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let buf = buffer.clone();
                tokio::spawn(async move {
                    let host = DiscoveryHostRequest {
                        host: Host::new(HostBase {
                            name: format!("host-{}", i),
                            hostname: None,
                            tags: vec![],
                            network_id: Uuid::new_v4(),
                            description: None,
                            source: EntitySource::Manual,
                            virtualization: None,
                            hidden: false,
                        }),
                        interfaces: vec![],
                        ports: vec![],
                        services: vec![],
                    };
                    buf.push_host(host).await;
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        let entities = buffer.drain().await;
        assert_eq!(entities.hosts.len(), 10);
    }

    #[tokio::test]
    async fn test_entity_buffer_lifecycle() {
        use crate::server::subnets::r#impl::{base::SubnetBase, types::SubnetType};
        use chrono::Utc;
        use cidr::{IpCidr, Ipv4Cidr};
        use std::net::Ipv4Addr;

        let buffer = EntityBuffer::new();
        let network_id = Uuid::new_v4();
        let now = Utc::now();

        // Push a subnet
        let subnet = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                name: "test-subnet".to_string(),
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap()),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };
        let pending_id = subnet.id;
        buffer.push_subnet(subnet.clone()).await;

        // Verify it's pending
        assert_eq!(buffer.pending_count().await, (0, 1));

        // Mark as created (same ID)
        buffer.mark_subnet_created(pending_id, subnet.clone()).await;

        // Verify it's created
        assert_eq!(buffer.pending_count().await, (0, 0));
        assert_eq!(buffer.count().await, (0, 1));

        // Get the subnet
        let retrieved = buffer.get_subnet(&pending_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, pending_id);
    }

    #[tokio::test]
    async fn test_drain_returns_all_entities() {
        use crate::server::subnets::r#impl::{base::SubnetBase, types::SubnetType};
        use chrono::Utc;
        use cidr::{IpCidr, Ipv4Cidr};
        use std::net::Ipv4Addr;

        let buffer = EntityBuffer::new();
        let network_id = Uuid::new_v4();
        let now = Utc::now();

        // Push two subnets
        let subnet1 = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                name: "subnet-1".to_string(),
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap()),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };
        let subnet2 = Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                name: "subnet-2".to_string(),
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(192, 168, 2, 0), 24).unwrap()),
                network_id,
                description: None,
                subnet_type: SubnetType::Unknown,
                source: EntitySource::Manual,
                tags: vec![],
            },
        };

        buffer.push_subnet(subnet1.clone()).await;
        buffer.push_subnet(subnet2.clone()).await;

        // Mark one as created
        buffer
            .mark_subnet_created(subnet1.id, subnet1.clone())
            .await;

        // Drain should return all entities (both pending and created)
        let all = buffer.drain().await;
        assert_eq!(all.subnets.len(), 2);

        // Verify buffer is empty after drain
        assert!(buffer.is_empty().await);
    }
}
