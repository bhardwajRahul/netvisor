//! UniFi Network Application (controller) credential types for discovery dispatch.
//!
//! Both UniFi transports — API key and local admin — reach the same controller endpoint with
//! the same host, port and site; only the auth material differs. So they collapse to a single
//! wire payload carrying a [`UnifiAuth`] discriminator, the same way SnmpV1/V2c/V3 collapse to
//! one `SnmpQueryCredential` carrying a version.

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, ResolvableSecret,
};
use serde::{Deserialize, Serialize};

/// Default controller port: 443, a UniFi OS console (UDM / Cloud Key / Cloud Gateway).
/// Self-hosted UniFi OS Server is 11443 and the legacy Network Application is 8443.
pub const DEFAULT_UNIFI_PORT: u16 = 443;

/// UniFi's default site. This is the *internal* site name (the `<name>` in the controller URL
/// `/manage/site/<name>`), not the site's display name — they differ on any renamed site.
pub const DEFAULT_UNIFI_SITE: &str = "default";

pub fn default_unifi_port() -> u16 {
    DEFAULT_UNIFI_PORT
}

pub fn default_unifi_site() -> String {
    DEFAULT_UNIFI_SITE.to_string()
}

/// How the daemon authenticates to the controller.
///
/// Kept as an enum rather than a bag of optional fields so illegal combinations (an API key
/// *and* a username, or neither) cannot be represented on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(tag = "mode")]
pub enum UnifiAuth {
    /// `X-API-KEY` header. Stateless, but **UniFi OS only** — the legacy self-hosted Network
    /// Application on 8443 does not support API keys at all.
    ApiKey { api_key: ResolvableSecret },
    /// Local-admin username/password, exchanged for a session cookie. Works on every
    /// controller type, including legacy self-hosted.
    LocalAdmin {
        username: String,
        password: ResolvableSecret,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct UnifiQueryCredential {
    pub port: u16,
    pub site: String,
    pub auth: UnifiAuth,
}

impl UnifiQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        let mut lines = vec![
            BannerField {
                label: "Port",
                value: BannerFieldValue::Plain(self.port.to_string()),
            },
            BannerField {
                label: "Site",
                value: BannerFieldValue::Plain(self.site.clone()),
            },
        ];
        match &self.auth {
            UnifiAuth::ApiKey { api_key } => lines.push(BannerField {
                label: "API key",
                value: api_key.banner_value(),
            }),
            UnifiAuth::LocalAdmin { username, password } => {
                lines.push(BannerField {
                    label: "Username",
                    value: BannerFieldValue::Plain(username.clone()),
                });
                lines.push(BannerField {
                    label: "Password",
                    value: password.banner_value(),
                });
            }
        }
        lines
    }
}
