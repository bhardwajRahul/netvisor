//! HTTP transport for the HPE Networking Instant On cloud portal.
//!
//! HPE publishes no API for Instant On — this speaks the same undocumented API the portal's own
//! web client uses. Two consequences shape the whole module:
//!
//! 1. **Nothing here is a contract.** Endpoints and field names come from the portal bundle and
//!    can change without notice, so every response is parsed leniently and every failure explains
//!    itself in terms an operator can act on.
//! 2. **Endpoints are discovered, not hard-coded.** The portal publishes `settings.json` with its
//!    current API base, SSO host and OAuth client id. Reading it at runtime is what let this keep
//!    working across the HPE rebrand, when the API base moved hosts and the old one began
//!    answering 308.
//!
//! Auth is OAuth2 authorization-code with PKCE, but with the credential POSTed by us rather than
//! typed into a login page — which is why the account must have MFA disabled. There is nowhere to
//! answer a second factor.

use std::time::Duration;

use anyhow::{Error, Result, anyhow, bail};
use base64ct::{Base64UrlUnpadded, Encoding};
use rand::RngCore;
use reqwest::{Client, StatusCode, redirect::Policy};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use crate::daemon::discovery::service::warnings::AttemptOutcome;
use crate::server::credentials::r#impl::mapping::InstantOnQueryCredential;

use super::types::InstantOnEnvelope;

/// Label used in credential-resolution error messages.
const CREDENTIAL_LABEL: &str = "Instant On portal connection";

/// Where the portal publishes its own configuration. The one URL that has to be hard-coded;
/// everything else is read from what it returns.
const PORTAL_SETTINGS_URL: &str = "https://portal.instant-on.hpe.com/settings.json";

/// API version the portal's client asks for. Sent on every API request; the API answers
/// differently (or not at all) without it.
const API_VERSION_HEADER: &str = "x-ion-api-version";
const API_VERSION: &str = "7";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The probe runs without an outer timeout wrapper (`dispatch::probe_integrations` only wraps
/// `execute`), so the client bounds its own requests. Four round trips to a cloud service, so
/// this is more generous than a LAN controller would need.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The portal's self-description. Field names are the portal's, not ours.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortalSettings {
    /// Base for every `/api/...` call.
    rest_api_url: String,
    /// SSO host that issues the token.
    sso_fqdn: String,
    /// OAuth client id the portal's own web app uses.
    sso_client_id_auth_z: String,
    /// Redirect URI registered for that client. The authorization code comes back on it; we read
    /// the code out of the `Location` header rather than ever following it.
    sso_redirect_url: String,
}

/// Step 1 of the exchange: the credential POST returns a session token, not the bearer.
///
/// Two accepted spellings because this is a reverse-engineered response and the portal's client
/// has used both; taking whichever is present costs one `or` and avoids a total auth failure over
/// a renamed field. If neither is there, that is worth failing loudly on — it means the shape
/// changed in a way we cannot paper over.
#[derive(Debug, Deserialize)]
struct SessionTokenResponse {
    access_token: Option<String>,
    session_token: Option<String>,
}

impl SessionTokenResponse {
    fn token(self) -> Option<String> {
        self.access_token.or(self.session_token)
    }
}

#[derive(Debug, Deserialize)]
struct BearerTokenResponse {
    access_token: String,
}

/// An authenticated connection to the Instant On cloud portal.
pub struct InstantOnClient {
    client: Client,
    api_base: String,
    bearer: String,
    /// Site name the operator restricted this credential to, if any.
    site_filter: Option<String>,
}

impl InstantOnClient {
    pub fn site_filter(&self) -> Option<&str> {
        self.site_filter.as_deref()
    }

    /// Read the portal's settings, run the PKCE exchange, and hold the bearer token.
    pub async fn connect(credential: &InstantOnQueryCredential) -> Result<Self, Error> {
        // Redirects are never followed: step 3 *needs* the unfollowed 302 to read its `code`, and
        // on the API calls a redirect means HPE moved the endpoint out from under us — which
        // should surface as a clear failure rather than be silently chased.
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .build()
            .map_err(|e| anyhow!("Failed to build Instant On HTTP client: {e}"))?;

        let settings: PortalSettings = client
            .get(PORTAL_SETTINGS_URL)
            .send()
            .await
            .map_err(|e| Error::new(e).context("Could not reach the Instant On portal"))?
            .json()
            .await
            .map_err(|e| anyhow!("Could not read the Instant On portal's settings.json: {e}"))?;

        let bearer = Self::exchange_token(&client, &settings, credential).await?;

        Ok(Self {
            client,
            api_base: settings.rest_api_url.trim_end_matches('/').to_string(),
            bearer,
            site_filter: credential.site.clone(),
        })
    }

    /// How a [`Self::connect`] failure should be reported to the operator.
    ///
    /// Same approach as the UniFi client: read the transport error out of the chain rather than
    /// matching our own message text, so rewording a `bail!` cannot change the classification.
    /// Everything raised here directly is a credential or protocol problem, hence the `Rejected`
    /// fallback; genuine transport failures carry a `reqwest::Error` and classify from it.
    pub fn classify_connect_error(error: &Error) -> AttemptOutcome {
        error
            .chain()
            .find_map(|cause| cause.downcast_ref::<reqwest::Error>())
            .map(AttemptOutcome::from)
            .unwrap_or(AttemptOutcome::Rejected)
    }

    /// The four-step OAuth2 PKCE exchange the portal's web client performs.
    async fn exchange_token(
        client: &Client,
        settings: &PortalSettings,
        credential: &InstantOnQueryCredential,
    ) -> Result<String, Error> {
        let sso = settings.sso_fqdn.trim_end_matches('/');
        let verifier = random_urlsafe_token();
        let challenge = Base64UrlUnpadded::encode_string(&Sha256::digest(verifier.as_bytes()));
        let state = random_urlsafe_token();

        // 1. Validate the credential, receiving a session token (not yet the bearer).
        let password = credential.password.resolve("password", CREDENTIAL_LABEL)?;
        let response = client
            .post(format!("{sso}/aio/api/v1/mfa/validate/full"))
            .form(&[
                ("username", credential.username.as_str()),
                ("password", password.expose_secret()),
            ])
            .send()
            .await
            .map_err(|e| Error::new(e).context("Could not reach Instant On sign-in"))?;

        match response.status() {
            s if s.is_success() => {}
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => bail!(
                "Instant On rejected the account or password. If the credentials are right, check \
                 that multi-factor authentication is disabled on this account — the sign-in \
                 cannot answer an MFA prompt."
            ),
            s => bail!("Instant On sign-in failed with HTTP {s}"),
        }

        let session_token = response
            .json::<SessionTokenResponse>()
            .await
            .map_err(|e| anyhow!("Could not parse the Instant On sign-in response: {e}"))?
            .token()
            .ok_or_else(|| {
                anyhow!(
                    "Instant On accepted the sign-in but returned no session token. The portal's \
                     sign-in response has changed shape; this integration needs an update."
                )
            })?;

        // 2. Trade the session token for an authorization code. The code is in the `Location`
        //    header of the redirect, which is why this client never follows redirects.
        let authorize = client
            .get(format!("{sso}/as/authorization.oauth2"))
            .query(&[
                ("client_id", settings.sso_client_id_auth_z.as_str()),
                ("redirect_uri", settings.sso_redirect_url.as_str()),
                ("response_type", "code"),
                ("scope", "profile openid"),
                ("state", state.as_str()),
                ("code_challenge", challenge.as_str()),
                ("code_challenge_method", "S256"),
                ("sessionToken", session_token.as_str()),
            ])
            .send()
            .await
            .map_err(|e| Error::new(e).context("Could not reach Instant On authorization"))?;

        let location = authorize
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                anyhow!(
                    "Instant On authorization did not redirect with an authorization code \
                     (HTTP {}). This usually means the account requires multi-factor \
                     authentication.",
                    authorize.status()
                )
            })?;

        let code = url::Url::parse(location)
            .ok()
            .and_then(|u| {
                u.query_pairs()
                    .find(|(k, _)| k == "code")
                    .map(|(_, v)| v.into_owned())
            })
            .ok_or_else(|| {
                anyhow!("Instant On authorization redirect carried no authorization code")
            })?;

        // 3. Exchange the code for the bearer token, proving we hold the PKCE verifier.
        let token_response = client
            .post(format!("{sso}/as/token.oauth2"))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code.as_str()),
                ("redirect_uri", settings.sso_redirect_url.as_str()),
                ("client_id", settings.sso_client_id_auth_z.as_str()),
                ("code_verifier", verifier.as_str()),
            ])
            .send()
            .await
            .map_err(|e| Error::new(e).context("Could not reach Instant On token exchange"))?;

        if !token_response.status().is_success() {
            bail!(
                "Instant On token exchange failed with HTTP {}",
                token_response.status()
            );
        }

        Ok(token_response
            .json::<BearerTokenResponse>()
            .await
            .map_err(|e| anyhow!("Could not parse the Instant On token response: {e}"))?
            .access_token)
    }

    /// GET a site-scoped resource (e.g. `inventory`) and decode its `elements` envelope.
    pub async fn get_site<T: DeserializeOwned>(
        &self,
        site_id: &str,
        resource: &str,
    ) -> Result<InstantOnEnvelope<T>, Error> {
        self.get(&format!("api/sites/{site_id}/{resource}")).await
    }

    /// GET any path under the portal's API base and decode its `elements` envelope.
    pub async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<InstantOnEnvelope<T>, Error> {
        let url = format!("{}/{}", self.api_base, path);

        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.bearer)
            .header(API_VERSION_HEADER, API_VERSION)
            .send()
            .await
            .map_err(|e| Error::new(e).context("Could not reach the Instant On portal"))?;

        match response.status() {
            s if s.is_success() => {}
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                bail!("Instant On rejected the session for {path}")
            }
            // A redirect here means the API moved hosts, as it did at the HPE rebrand. Say that,
            // rather than reporting an opaque status: the fix is a Scanopy update, not anything
            // the operator can change.
            s if s.is_redirection() => bail!(
                "The Instant On API redirected {path} (HTTP {s}). HPE has moved the endpoint; \
                 this integration needs an update."
            ),
            s => bail!("The Instant On API returned HTTP {s} for {path}"),
        }

        response
            .json()
            .await
            .map_err(|e| anyhow!("Could not parse the Instant On {path} response: {e}"))
    }
}

/// 256 bits of CSPRNG, base64url without padding — the encoding PKCE requires for the verifier,
/// and the same construction `AuthService::generate_secure_token` uses for session tokens.
fn random_urlsafe_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    Base64UrlUnpadded::encode_string(&bytes)
}
