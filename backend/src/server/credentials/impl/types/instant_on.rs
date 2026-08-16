//! HPE Networking Instant On (cloud portal) credential type for discovery dispatch.
//!
//! Instant On switches expose SNMP only in local management mode, which means giving up the cloud
//! portal that is the reason to buy them. So the portal's own API is the only source of port and
//! neighbour data for a cloud-managed site.
//!
//! Unlike every other credential here, the endpoint is not on the operator's network — the daemon
//! authenticates to HPE's cloud. The credential is still bound to a host (the Instant On switch it
//! reports on), because the IP binding names *what the credential produces data about*, not where
//! the transport goes; `ContainerSocketQueryCredential` binds to the daemon host and then connects
//! to a Unix socket the same way.

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, ResolvableSecret,
};
use serde::{Deserialize, Serialize};

/// One transport, because there is exactly one way in: HPE publishes no API for Instant On, so the
/// portal account is the only credential the undocumented API accepts. An official route, if one
/// ever ships, becomes a second transport of this same integration.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct InstantOnQueryCredential {
    /// Portal account email address.
    pub username: String,
    /// Password for that account. MFA must be off — the token exchange posts these directly.
    pub password: ResolvableSecret,
    /// Restrict the fetch to one site by name. `None` ⇒ every site the account can see.
    pub site: Option<String>,
}

impl InstantOnQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        let mut lines = vec![
            BannerField {
                label: "Account",
                value: BannerFieldValue::Plain(self.username.clone()),
            },
            BannerField {
                label: "Password",
                value: self.password.banner_value(),
            },
        ];
        // Only worth a line when it narrows the fetch; "all sites" is the default and saying so
        // adds nothing an operator can act on.
        if let Some(site) = &self.site {
            lines.push(BannerField {
                label: "Site",
                value: BannerFieldValue::Plain(site.clone()),
            });
        }
        lines
    }
}
