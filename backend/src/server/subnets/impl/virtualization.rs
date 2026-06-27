use serde::{Deserialize, Serialize};
use strum_macros::VariantNames;
use utoipa::ToSchema;
use uuid::Uuid;

/// Virtualization metadata for subnets that belong to a virtual infrastructure.
/// Consistent with HostVirtualization and ServiceVirtualization patterns.
/// Points to the service that provides the virtualization (e.g., Docker daemon).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames, ToSchema)]
#[serde(tag = "type")]
pub enum SubnetVirtualization {
    /// Docker bridge network — host-scoped, same CIDR on different hosts are distinct subnets.
    #[schema(title = "Docker")]
    Docker(DockerSubnetVirtualization),
    /// Podman bridge network — host-scoped like Docker; same CIDR on different daemons are distinct subnets.
    #[schema(title = "Podman")]
    Podman(PodmanSubnetVirtualization),
}

impl SubnetVirtualization {
    pub fn service_id(&self) -> Option<Uuid> {
        match self {
            Self::Docker(d) => Some(d.service_id),
            Self::Podman(p) => Some(p.service_id),
        }
    }

    pub fn set_service_id(&mut self, id: Uuid) {
        match self {
            Self::Docker(d) => d.service_id = id,
            Self::Podman(p) => p.service_id = id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct DockerSubnetVirtualization {
    /// The Docker daemon service that owns this bridge network.
    /// Different Docker daemons on different hosts = distinct bridge subnets.
    pub service_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct PodmanSubnetVirtualization {
    /// The Podman daemon service that owns this bridge network.
    /// Different Podman daemons on different hosts = distinct bridge subnets.
    pub service_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_subnet_virtualization_round_trip() {
        let id = Uuid::new_v4();
        let virt = SubnetVirtualization::Docker(DockerSubnetVirtualization { service_id: id });
        let json = serde_json::to_string(&virt).unwrap();
        assert_eq!(
            serde_json::from_str::<SubnetVirtualization>(&json).unwrap(),
            virt
        );
        assert_eq!(virt.service_id(), Some(id));
    }

    #[test]
    fn podman_subnet_virtualization_round_trip() {
        let id = Uuid::new_v4();
        let virt = SubnetVirtualization::Podman(PodmanSubnetVirtualization { service_id: id });
        let json = serde_json::to_string(&virt).unwrap();
        // Tagged as "Podman" so it is distinct from the Docker variant on the wire.
        assert!(json.contains("\"Podman\""));
        assert_eq!(
            serde_json::from_str::<SubnetVirtualization>(&json).unwrap(),
            virt
        );
        assert_eq!(virt.service_id(), Some(id));
    }

    #[test]
    fn set_service_id_updates_either_variant() {
        let new_id = Uuid::new_v4();
        let mut docker = SubnetVirtualization::Docker(DockerSubnetVirtualization {
            service_id: Uuid::nil(),
        });
        docker.set_service_id(new_id);
        assert_eq!(docker.service_id(), Some(new_id));

        let mut podman = SubnetVirtualization::Podman(PodmanSubnetVirtualization {
            service_id: Uuid::nil(),
        });
        podman.set_service_id(new_id);
        assert_eq!(podman.service_id(), Some(new_id));
    }
}
