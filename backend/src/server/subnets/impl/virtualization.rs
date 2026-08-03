//! Subnet virtualization used to be a JSONB blob whose entire content was the owning service's
//! id, tagged `Docker` or `Podman`.
//!
//! Both halves are now carried better elsewhere: the id is `subnets.virtualization_service_id`,
//! a real foreign key that a dangling reference cannot survive, and the runtime is already named
//! by `SubnetType::DockerBridge` / `PodmanBridge`. Nothing was left for this type to hold, so it
//! was removed rather than kept as an empty shell (GH #650).
