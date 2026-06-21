//! LLDP/FDB neighbor-resolution filters.
use super::*;

impl<T: Storable> StorableFilter<T> {
    // =========================================================================
    // LLDP resolution filters
    // =========================================================================

    /// Filter by IP address (for ip_addresses table)
    pub fn ip_address(mut self, ip: std::net::IpAddr) -> Self {
        let col = self.qualify_column("ip_address");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::IpAddr(ip));
        self
    }

    /// Filter by if_descr (for interfaces table)
    pub fn if_descr(mut self, descr: &str) -> Self {
        let col = self.qualify_column("if_descr");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(descr.to_string()));
        self
    }

    /// Filter by if_name (for interfaces table)
    pub fn if_name(mut self, name: &str) -> Self {
        let col = self.qualify_column("if_name");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(name.to_string()));
        self
    }

    /// Filter by chassis_id (for hosts table)
    pub fn chassis_id(mut self, chassis_id: &str) -> Self {
        let col = self.qualify_column("chassis_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(chassis_id.to_string()));
        self
    }

    /// Filter by sys_name (for hosts table)
    pub fn sys_name(mut self, sys_name: &str) -> Self {
        let col = self.qualify_column("sys_name");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(sys_name.to_string()));
        self
    }

    /// Filter by ip_address_id FK (for interfaces table)
    pub fn ip_address_id(mut self, ip_address_id: &Uuid) -> Self {
        let col = self.qualify_column("ip_address_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*ip_address_id));
        self
    }

    /// Filter interfaces with unresolved LLDP/CDP neighbors in a network.
    /// Matches entries that have LLDP or CDP data but no neighbor (neither interface nor host).
    pub fn unresolved_lldp_in_network(mut self, network_id: Uuid) -> Self {
        let network_col = self.qualify_column("network_id");
        let lldp_chassis_col = self.qualify_column("lldp_chassis_id");
        let cdp_device_col = self.qualify_column("cdp_device_id");
        let cdp_addr_col = self.qualify_column("cdp_address");
        let neighbor_if_entry_col = self.qualify_column("neighbor_interface_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        self.conditions
            .push(format!("{} = ${}", network_col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(network_id));

        // Has LLDP or CDP data but not yet resolved (no neighbor of either type)
        self.conditions.push(format!(
            "({} IS NOT NULL OR {} IS NOT NULL OR {} IS NOT NULL)",
            lldp_chassis_col, cdp_device_col, cdp_addr_col
        ));
        self.conditions
            .push(format!("{} IS NULL", neighbor_if_entry_col));
        self.conditions
            .push(format!("{} IS NULL", neighbor_host_col));

        self
    }

    /// Filter interfaces with unresolved single-MAC FDB data in a network.
    /// Matches entries that have exactly 1 learned MAC, no existing neighbor,
    /// and no LLDP/CDP data (FDB is lower-priority than protocol-based discovery).
    pub fn unresolved_fdb_in_network(mut self, network_id: Uuid) -> Self {
        let network_col = self.qualify_column("network_id");
        let fdb_col = self.qualify_column("fdb_macs");
        let neighbor_if_entry_col = self.qualify_column("neighbor_interface_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");
        let lldp_chassis_col = self.qualify_column("lldp_chassis_id");
        let cdp_device_col = self.qualify_column("cdp_device_id");

        self.conditions
            .push(format!("{} = ${}", network_col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(network_id));

        // Has single-MAC FDB data, no neighbor, no LLDP/CDP
        self.conditions.push(format!(
            "{} IS NOT NULL AND jsonb_array_length({}) = 1",
            fdb_col, fdb_col
        ));
        self.conditions
            .push(format!("{} IS NULL", neighbor_if_entry_col));
        self.conditions
            .push(format!("{} IS NULL", neighbor_host_col));
        self.conditions
            .push(format!("{} IS NULL", lldp_chassis_col));
        self.conditions.push(format!("{} IS NULL", cdp_device_col));

        self
    }

    /// Filter interfaces that have any resolved neighbor (full or partial resolution)
    pub fn has_neighbor(mut self) -> Self {
        let neighbor_if_entry_col = self.qualify_column("neighbor_interface_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        self.conditions.push(format!(
            "({} IS NOT NULL OR {} IS NOT NULL)",
            neighbor_if_entry_col, neighbor_host_col
        ));

        self
    }

    /// Filter interfaces with full neighbor resolution (specific remote port known)
    pub fn has_neighbor_if_entry(mut self) -> Self {
        let col = self.qualify_column("neighbor_interface_id");
        self.conditions.push(format!("{} IS NOT NULL", col));
        self
    }

    /// Filter interfaces connected to a specific host (either resolution type)
    pub fn neighbor_host(mut self, host_id: Uuid) -> Self {
        let neighbor_if_entry_col = self.qualify_column("neighbor_interface_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        // Either directly connected to host (partial resolution)
        // Or connected to an interface on that host (full resolution)
        // For full resolution, we need a subquery
        self.conditions.push(format!(
            "({} = ${} OR {} IN (SELECT id FROM interfaces WHERE host_id = ${}))",
            neighbor_host_col,
            self.values.len() + 1,
            neighbor_if_entry_col,
            self.values.len() + 1
        ));
        self.values.push(SqlValue::Uuid(host_id));

        self
    }
}
