//! Naming the neighbours that could not be placed — for the log, and for the scan record the
//! operator reads.
use super::*;

impl HostService {
    /// Name the far ends that could not be placed, so the counters can be checked rather than
    /// inferred.
    ///
    /// Two lines, because the two populations call for different actions: a device we have never
    /// discovered is the operator's to scan, whereas a device we *have* discovered whose port we
    /// cannot name is ours to explain. Each is capped like the daemon's scan warnings and says how
    /// many were elided — a list that simply stops reads as though that was all of them.
    ///
    /// Returned as well as logged: the log lines carry the full per-neighbour evidence for whoever
    /// has container access, and the returned lines go onto the scan record so a self-hosted
    /// operator sees the same outcomes where they already read scan results.
    pub(super) async fn report_unresolved(
        &self,
        network_id: Uuid,
        unmatched: &[UnmatchedNeighbour],
        unresolved_ports: &[UnresolvedPort],
    ) -> Vec<String> {
        if unmatched.is_empty() && unresolved_ports.is_empty() {
            return Vec::new();
        }

        // One fetch for both lines: this runs after every scan, and the lists are dominated by a
        // handful of local devices reporting many far ends each. Remote hosts are included because
        // the port line names both ends.
        let host_ids: Vec<Uuid> = unmatched
            .iter()
            .map(|u| u.host_id)
            .chain(
                unresolved_ports
                    .iter()
                    .flat_map(|p| [p.host_id, p.remote_host_id]),
            )
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let names: HashMap<Uuid, String> = match self
            .get_all(StorableFilter::<Host>::new_from_entity_ids(&host_ids))
            .await
        {
            Ok(hosts) => hosts
                .into_iter()
                .map(|h| (h.id, h.base.name.to_string()))
                .collect(),
            // The identifiers below are the point of the lines; losing a host's name makes them
            // harder to read, not useless.
            Err(e) => {
                tracing::debug!(network_id = %network_id, error = %e, "Could not name the hosts for unresolved LLDP neighbours");
                HashMap::new()
            }
        };

        let mut warnings = Vec::new();

        if !unmatched.is_empty() {
            let listed: Vec<String> = unmatched
                .iter()
                .take(MAX_LISTED_UNMATCHED)
                .map(|u| u.describe(names.get(&u.host_id)))
                .collect();
            let elided = unmatched.len().saturating_sub(listed.len());

            tracing::warn!(
                network_id = %network_id,
                unmatched = unmatched.len(),
                elided,
                neighbours = %listed.join("; "),
                "LLDP/CDP neighbours identify devices this network has not discovered, so they draw \
                 no links. Expected where the far end is an endpoint or unmanaged device; a device \
                 that should have been scanned means its identifier is not one we hold."
            );

            warnings.push(format!(
                "{} LLDP/CDP neighbour{} identify devices this network has not discovered, so they \
                 draw no links. This is expected where the far end is an endpoint or unmanaged \
                 device; a device that should have been scanned means the identifier it advertises \
                 is not one this network holds. {}",
                unmatched.len(),
                if unmatched.len() == 1 { "" } else { "s" },
                describe_sample(&listed, elided),
            ));
        }

        if !unresolved_ports.is_empty() {
            let listed: Vec<String> = unresolved_ports
                .iter()
                .take(MAX_LISTED_UNMATCHED)
                .map(|p| p.describe(names.get(&p.host_id), names.get(&p.remote_host_id)))
                .collect();
            let elided = unresolved_ports.len().saturating_sub(listed.len());

            tracing::warn!(
                network_id = %network_id,
                unresolved_ports = unresolved_ports.len(),
                elided,
                neighbours = %listed.join("; "),
                "LLDP/CDP neighbours resolved to a device but not to one of its ports, so they draw \
                 a device-level link instead of a port-to-port one. Each entry names the port id \
                 that was tried and why it did not identify a single port."
            );

            warnings.push(format!(
                "{} LLDP/CDP neighbour{} resolved to a device but not to one of its ports, so \
                 Physical Topology draws a dashed device-level link instead of a port-to-port one. \
                 Each entry names the port id that was tried and why it did not identify a single \
                 port. {}",
                unresolved_ports.len(),
                if unresolved_ports.len() == 1 { "" } else { "s" },
                describe_sample(&listed, elided),
            ));
        }

        warnings
    }
}

/// `Examples: a; b; c (and 4 more).` — always says how many were left out, because a list that
/// simply stops reads as though that was all of them.
fn describe_sample(listed: &[String], elided: usize) -> String {
    let more = match elided {
        0 => String::new(),
        n => format!(" (and {n} more)"),
    };
    format!("Examples: {}{more}.", listed.join("; "))
}
