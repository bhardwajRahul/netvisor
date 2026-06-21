//! Inactivity standby detection, notifications, and loopback credential scoping.
use super::*;

impl DaemonService {
    // ========================================================================
    // Inactivity standby
    // ========================================================================

    /// Check all active daemons for inactivity (no completed discovery in 30 days)
    /// and put them on standby, sending notification emails if email service is available.
    pub async fn check_daemon_inactivity(
        &self,
        email_service: Option<&EmailService>,
    ) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::days(30);

        // Get all non-standby daemons created more than 30 days ago
        let active_daemons = self
            .get_all(StorableFilter::<Daemon>::new_for_active_daemons().created_before(cutoff))
            .await?;

        for mut daemon in active_daemons {
            // Skip daemons still within their post-reactivation grace window
            // so a freshly-restarted daemon has time for a scheduled
            // discovery to complete before inactivity is re-evaluated.
            if is_within_standby_grace(daemon.base.standby_cleared_at, Utc::now()) {
                tracing::debug!(
                    daemon_id = %daemon.id,
                    cleared_at = ?daemon.base.standby_cleared_at,
                    "Skipping inactivity check (within standby-clear grace)"
                );
                continue;
            }

            // Check for historical discoveries completed by this daemon
            let filter = StorableFilter::<Discovery>::new_from_uuid_column("daemon_id", &daemon.id)
                .historical_discovery();
            let discoveries = self.discovery_service.get_all(filter).await?;

            // Find the most recent finished_at from Historical discoveries
            let last_finished = discoveries
                .iter()
                .filter_map(|d| {
                    if let RunType::Historical { ref results } = d.base.run_type {
                        results.finished_at
                    } else {
                        None
                    }
                })
                .max();

            let should_standby = match last_finished {
                Some(finished) => finished < cutoff,
                None => true, // No historical records at all
            };

            if should_standby {
                daemon.base.standby = true;
                self.update(&mut daemon, AuthenticatedEntity::System)
                    .await?;
                tracing::info!(
                    daemon_id = %daemon.id,
                    "Set daemon to standby (inactive for 30+ days)"
                );

                // Send notification emails if email service is available
                if let Some(email_service) = email_service
                    && let Err(e) = self.send_standby_notification(&daemon, email_service).await
                {
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        error = %e,
                        "Failed to send daemon standby notification email"
                    );
                }
            }
        }

        Ok(())
    }

    /// Send standby notification email to org owner and daemon installer
    async fn send_standby_notification(
        &self,
        daemon: &Daemon,
        email_service: &EmailService,
    ) -> Result<()> {
        let network = self
            .network_service
            .get_by_id(&daemon.base.network_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network not found"))?;

        let org_id = network.base.organization_id;
        let network_name = &network.base.name;
        let daemon_name = &daemon.base.name;

        // Send to org owner
        let owners = email_service
            .user_service
            .get_organization_owners(&org_id)
            .await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        email_service
            .send_daemon_standby_email(owner.base.email.clone(), daemon_name, network_name)
            .await?;

        // Also send to daemon installer if different from owner
        if daemon.base.user_id != owner.id
            && let Some(user) = email_service
                .user_service
                .get_by_id(&daemon.base.user_id)
                .await?
        {
            email_service
                .send_daemon_standby_email(user.base.email, daemon_name, network_name)
                .await?;
        }

        Ok(())
    }

    /// Send unreachable notification email to org owner and daemon installer
    pub(crate) async fn send_unreachable_notification(
        &self,
        daemon: &Daemon,
        email_service: &EmailService,
    ) -> Result<()> {
        let network = self
            .network_service
            .get_by_id(&daemon.base.network_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network not found"))?;

        let org_id = network.base.organization_id;
        let network_name = &network.base.name;
        let daemon_name = &daemon.base.name;

        // Send to org owner
        let owners = email_service
            .user_service
            .get_organization_owners(&org_id)
            .await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        email_service
            .send_daemon_unreachable_email(owner.base.email.clone(), daemon_name, network_name)
            .await?;

        // Also send to daemon installer if different from owner
        if daemon.base.user_id != owner.id
            && let Some(user) = email_service
                .user_service
                .get_by_id(&daemon.base.user_id)
                .await?
        {
            email_service
                .send_daemon_unreachable_email(user.base.email, daemon_name, network_name)
                .await?;
        }

        Ok(())
    }

    /// After discovery creates a host with a loopback ip_address, scope any localhost-targeted
    /// credentials to specifically the loopback interface instead of all ip_addresses.
    pub(crate) async fn scope_loopback_credentials(
        &self,
        host_id: &Uuid,
        host_service: &HostService,
    ) -> Result<()> {
        // Find the loopback interface for this host
        let ip_addresses = host_service.get_ip_addresses_for_host(host_id).await?;
        let loopback_interface = ip_addresses
            .iter()
            .find(|i| i.base.ip_address.is_loopback());

        let Some(loopback_iface) = loopback_interface else {
            return Ok(()); // No loopback interface on this host
        };
        let loopback_id = loopback_iface.id;

        // Get current credential assignments
        let assignments = self
            .credential_service
            .get_credential_assignments_for_host(host_id)
            .await?;

        let mut updated = false;
        let mut new_assignments = Vec::new();
        for assignment in assignments {
            if assignment.ip_address_ids.is_some() {
                new_assignments.push(assignment);
                continue;
            }

            // Check if this credential targets localhost
            let is_loopback_cred = match self
                .credential_service
                .get_by_id(&assignment.credential_id)
                .await
            {
                Ok(Some(cred)) => cred
                    .base
                    .target_ips
                    .as_ref()
                    .is_some_and(|ips| ips.iter().any(|ip| ip.is_loopback())),
                _ => false,
            };

            if is_loopback_cred {
                new_assignments.push(CredentialAssignment {
                    credential_id: assignment.credential_id,
                    ip_address_ids: Some(vec![loopback_id]),
                });
                updated = true;
            } else {
                new_assignments.push(assignment);
            }
        }

        if updated {
            self.credential_service
                .set_host_credentials(host_id, &new_assignments)
                .await?;
            tracing::debug!(
                host_id = %host_id,
                loopback_interface_id = %loopback_id,
                "Scoped loopback credentials to loopback ip_address"
            );
        }

        Ok(())
    }
}
