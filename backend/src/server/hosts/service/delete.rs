//! Host deletion.
use super::*;

impl HostService {
    /// Delete a host (children cascade via FK)
    pub async fn delete_host(&self, id: &Uuid, authentication: AuthenticatedEntity) -> Result<()> {
        // Can't delete host with daemon
        if self
            .daemon_service
            .get_one(StorableFilter::<Daemon>::new_from_host_ids(&[*id]))
            .await?
            .is_some()
        {
            return Err(ValidationError::new(
                "Can't delete a host with an associated daemon. Delete the daemon first.",
            )
            .into());
        }

        let host = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Host {} not found", id))?;

        let lock = self.get_host_lock(id).await;
        let _guard = lock.lock().await;

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Host)
                .await?;
        }

        // Delete host - children cascade via ON DELETE CASCADE
        self.storage().delete(id).await?;

        let trigger_stale = host.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            host.id(),
            host.clone().into(),
            self.get_network_id(&host),
            self.get_organization_id(&host),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Deleted, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(())
    }
}
