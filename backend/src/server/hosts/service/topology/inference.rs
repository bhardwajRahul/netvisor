//! Turning far ends the LLDP pass could not place into subnets and hosts.
//!
//! The *rule* for what range an address implies lives in
//! [`crate::server::subnets::r#impl::inference`], because a controller-reported address and a
//! neighbour-advertised one are placed by the same rule. What is here is the part only this pass
//! has: the far ends it collected, the evidence it can attach to a warning, and the hosts it mints
//! for them.
use cidr::IpCidr;

use crate::daemon::discovery::types::warnings::ProvisionalSubnet;
use crate::server::ip_addresses::r#impl::base::IPAddressBase;
use crate::server::networks::r#impl::Network;
use crate::server::subnets::r#impl::{
    base::SubnetBase,
    inference::{UnplacedFarEnd, infer_ranges},
    types::{SubnetCidrSource, SubnetType},
};

use super::*;

impl HostService {
    /// Create a subnet for every range these far ends imply, and report each one.
    ///
    /// Runs after the resolution pass rather than inside it: the ranges are a property of the whole
    /// network's unplaced far ends, not of any one interface, and pooling them is the entire reason
    /// this lives on the server (see the module docs).
    ///
    /// A failure to create one range never fails the pass. Link resolution is the caller's actual
    /// job, and losing every resolved link because a subnet insert raced another session would be a
    /// far worse outcome than one missing range that the next scan re-proposes anyway.
    pub(super) async fn infer_far_end_subnets(
        &self,
        network_id: Uuid,
        far_ends: Vec<UnplacedFarEnd>,
        limit_ctx: Option<&HostLimitContext>,
    ) -> Result<InferenceOutcome> {
        // No early return on an empty far-end list: the standing report below covers ranges the
        // ingest path inferred, and a scan that placed every neighbour is exactly when those are
        // the only ones left to tell an operator about.
        let live = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
            .await?;
        let live_cidrs: Vec<IpCidr> = live.iter().map(|s| s.base.cidr).collect();

        let mut warnings = Vec::new();
        // Evidence per range this pass created, folded into the standing report below.
        let mut evidence: HashMap<Uuid, ProvisionalSubnet> = HashMap::new();
        for range in infer_ranges(far_ends, &live_cidrs) {
            let mut subnet = Subnet::new(SubnetBase {
                cidr: range.cidr,
                // The whole point: a range nothing read, only inferred, so the row asks to be
                // confirmed rather than asserting itself.
                cidr_source: SubnetCidrSource::Inferred,
                network_id,
                name: range.cidr.to_string(),
                description: None,
                // Not `Management` even though a management address is what usually produces this:
                // on a flat network that address is simply the device's LAN address, and typing the
                // subnet would be a second guess stacked on the first.
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Discovery,
                tags: Vec::new(),
            });
            subnet.last_seen_at = Utc::now();

            let created = match self
                .subnet_service
                .create(subnet, AuthenticatedEntity::System)
                .await
            {
                Ok(created) => created,
                Err(e) => {
                    tracing::warn!(
                        network_id = %network_id,
                        cidr = %range.cidr,
                        error = %e,
                        "Could not create inferred subnet; leaving its far ends unplaced"
                    );
                    continue;
                }
            };

            tracing::info!(
                network_id = %network_id,
                cidr = %range.cidr,
                subnet_id = %created.id,
                addresses = range.far_ends.len(),
                widened_by_vlan = range.widened_by_vlan,
                "Inferred a subnet from far-end addresses"
            );

            self.mint_far_end_hosts(network_id, &created, &range.far_ends, limit_ctx)
                .await;

            evidence.insert(
                created.id,
                ProvisionalSubnet {
                    cidr: range.cidr.to_string(),
                    subnet_id: created.id,
                    addresses: range
                        .far_ends
                        .iter()
                        .map(|f| f.address.to_string())
                        .collect(),
                    sys_names: range
                        .far_ends
                        .iter()
                        .filter_map(|f| f.sys_name.clone())
                        .collect(),
                    seen_by_host_ids: range.far_ends.iter().map(|f| f.host_id).collect(),
                    widened_by_vlan: range.widened_by_vlan,
                },
            );
        }

        // Only ranges *created here* make a re-resolution worth running; the standing report below
        // says nothing new about what this pass can place.
        let minted = !evidence.is_empty();
        warnings.extend(
            self.provisional_subnet_warnings(network_id, evidence)
                .await?,
        );
        Ok(InferenceOutcome { minted, warnings })
    }

    /// Report every range on this network still waiting to be confirmed, not only the ones this
    /// pass created.
    ///
    /// A provisional CIDR is a standing state, not a per-scan delta — the same reasoning that makes
    /// `LldpNeighbourNotFound` a standing population. Reporting only new ones would mean an
    /// operator who did not act on the scan that proposed a range never hears about it again, and
    /// would leave ranges inferred on the ingest path silent entirely: nothing there has a
    /// `session_id`, and `append_historical_warnings` needs one.
    ///
    /// Far-end evidence is attached where this pass has it and omitted where it does not, which is
    /// the honest shape: a range inferred from a controller-reported address has no neighbour that
    /// advertised it.
    async fn provisional_subnet_warnings(
        &self,
        network_id: Uuid,
        mut evidence: HashMap<Uuid, ProvisionalSubnet>,
    ) -> Result<Vec<DiscoveryWarning>> {
        let live = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
            .await?;

        Ok(live
            .into_iter()
            .filter(|s| s.base.cidr_source == SubnetCidrSource::Inferred)
            .map(|s| {
                let detail = evidence.remove(&s.id).unwrap_or(ProvisionalSubnet {
                    cidr: s.base.cidr.to_string(),
                    subnet_id: s.id,
                    addresses: Vec::new(),
                    sys_names: Vec::new(),
                    seen_by_host_ids: Vec::new(),
                    widened_by_vlan: false,
                });
                DiscoveryWarning::ProvisionalSubnetInferred(detail)
            })
            .collect())
    }
}

/// What one inference step produced.
pub(super) struct InferenceOutcome {
    /// Whether this step created a range, and so whether re-resolving can place anything new.
    pub minted: bool,
    /// Every range on the network still waiting to be confirmed, not only the ones created here.
    pub warnings: Vec<DiscoveryWarning>,
}

impl HostService {
    /// The plan limit that applies to hosts on this network, or `None` where the plan sets none.
    ///
    /// Best-effort by design: a network or organization that cannot be read yields no context, and
    /// the mint proceeds ungated rather than the whole resolution pass failing over a lookup. The
    /// alternative — treating an unreadable plan as a full one — would silently stop drawing links
    /// on a healthy fleet.
    pub(super) async fn host_limit_context(&self, network_id: Uuid) -> Option<HostLimitContext> {
        let network = self.network_service.get_by_id(&network_id).await.ok()??;
        let org_id = network.base.organization_id;
        let plan = self
            .organization_service
            .get_by_id(&org_id)
            .await
            .ok()?
            .and_then(|o| o.base.plan)
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);

        let limit = plan.host_limit()?;
        let org_network_ids = self
            .network_service
            .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
            .await
            .unwrap_or_default()
            .iter()
            .map(|n| n.id)
            .collect();

        Some(HostLimitContext {
            limit,
            org_id,
            org_network_ids,
            plan,
        })
    }

    /// Mint a host for each far end now that there is a subnet to place it in.
    ///
    /// The same thing `ControllerIdentity::into_host` does for a device a controller reports but
    /// the sweep never scanned, and deliberately through the same pipeline: `create_with_children`
    /// runs `select_matching_host` first, so a far end whose address or chassis id this network
    /// already holds updates that host instead of duplicating it. Minting is only ever the
    /// *fallback*, which is what keeps a device from appearing twice.
    ///
    /// `limit_ctx` is what makes the plan's host limit apply here at all. Both existing gates sit
    /// on paths this one does not take, so without it minting would quietly outrun a limit while
    /// still counting towards the number a customer is shown.
    ///
    /// Nothing here fails the pass. Link resolution is the caller's job, and a far end that cannot
    /// be minted — because the plan is full, or because two sessions raced — is one missing host,
    /// not a reason to lose every link the pass resolved.
    async fn mint_far_end_hosts(
        &self,
        network_id: Uuid,
        subnet: &Subnet,
        far_ends: &[UnplacedFarEnd],
        limit_ctx: Option<&HostLimitContext>,
    ) {
        // One host per address: several ports naming the same far end is one device, and minting
        // per sighting would put a row on the map for every cable.
        let mut minted: HashSet<IpAddr> = HashSet::new();

        for far_end in far_ends {
            if !minted.insert(far_end.address) {
                continue;
            }

            let mut host = Host::new(HostBase {
                network_id,
                // Reported by something else and never contacted. Distinct from `Discovery` so a
                // host with no ports and no services is not read as a device that is merely down,
                // and promoted the moment a scan reaches it.
                source: EntitySource::Inferred,
                // The neighbour's advertised sysName is matched against this column by the
                // resolution ladder, so recording it is what lets the *next* pass place this far
                // end without re-deriving anything.
                sys_name: far_end.sys_name.clone(),
                chassis_id: Some(far_end.chassis_id.clone()),
                ..Default::default()
            });
            // Ranked, not assigned: a sysName is reverse-DNS-grade evidence, so a real scan's
            // hostname or a name a person types still outranks it.
            host.base.apply_name(match &far_end.sys_name {
                Some(name) => HostName::Hostname(name.clone()),
                None => HostName::Ip(far_end.address),
            });
            host.last_seen_at = Utc::now();

            let ip_address = IPAddress::new(IPAddressBase {
                network_id,
                host_id: Uuid::nil(), // Server assigns.
                subnet_id: subnet.id,
                ip_address: far_end.address,
                mac_address: None,
                name: None,
                position: 0,
            });

            if let Err(e) = self
                .create_with_children(
                    host,
                    vec![ip_address],
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    ConflictBehavior::Upsert,
                    AuthenticatedEntity::System,
                    limit_ctx,
                    // A neighbour sees an address and a name, never an ifTable. Claiming an
                    // authoritative empty one here would tear down interfaces a later SNMP walk of
                    // the same host collected.
                    false,
                    InterfaceDataComplete::none(),
                )
                .await
            {
                tracing::warn!(
                    network_id = %network_id,
                    address = %far_end.address,
                    error = %e,
                    "Could not mint a host for an unplaceable far end"
                );
            }
        }
    }
}
