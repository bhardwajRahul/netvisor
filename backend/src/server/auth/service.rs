use crate::server::shared::events::types::TelemetryOperation;
use crate::server::{
    auth::{
        r#impl::{
            api::{LoginRequest, RegisterRequest},
            base::{LoginRegisterParams, PendingSetup, ProvisionUserParams},
        },
        middleware::auth::AuthenticatedEntity,
    },
    email::traits::EmailService,
    organizations::{
        r#impl::base::{Organization, OrganizationBase},
        service::OrganizationService,
    },
    shared::{
        events::{
            bus::EventBus,
            types::{AuthEvent, AuthOperation, TelemetryEvent},
        },
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Storable},
    },
    users::{
        r#impl::{
            base::{User, UserBase},
            permissions::UserOrgPermissions,
        },
        service::UserService,
    },
};
use anyhow::{Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use email_address::EmailAddress;
use std::{collections::HashMap, net::IpAddr, sync::Arc, time::Instant};
use tokio::sync::RwLock;
use uuid::Uuid;
use validator::Validate;

pub struct AuthService {
    pub user_service: Arc<UserService>,
    organization_service: Arc<OrganizationService>,
    email_service: Option<Arc<EmailService>>,
    login_attempts: Arc<RwLock<HashMap<EmailAddress, (u32, Instant)>>>,
    /// Rate limiting for verification email resend (not token storage - tokens stored in DB)
    verification_resend_cooldown: Arc<RwLock<HashMap<EmailAddress, Instant>>>,
    event_bus: Arc<EventBus>,
    public_url: String,
}

impl AuthService {
    const MAX_LOGIN_ATTEMPTS: u32 = 5;
    const LOCKOUT_DURATION_SECS: u64 = 15 * 60; // 15 minutes
    const VERIFICATION_TOKEN_EXPIRY_HOURS: i64 = 24;
    const PASSWORD_RESET_TOKEN_EXPIRY_HOURS: i64 = 1;
    const RESEND_COOLDOWN_SECS: u64 = 60;

    pub fn new(
        user_service: Arc<UserService>,
        organization_service: Arc<OrganizationService>,
        email_service: Option<Arc<EmailService>>,
        event_bus: Arc<EventBus>,
        public_url: String,
    ) -> Self {
        Self {
            user_service,
            organization_service,
            email_service,
            login_attempts: Arc::new(RwLock::new(HashMap::new())),
            verification_resend_cooldown: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
            public_url,
        }
    }

    /// Check if email service is configured
    pub fn has_email_service(&self) -> bool {
        self.email_service.is_some()
    }

    /// Register a new user with password
    pub async fn register(
        &self,
        request: RegisterRequest,
        params: LoginRegisterParams,
        pending_setup: Option<PendingSetup>,
        billing_enabled: bool,
    ) -> Result<User> {
        let LoginRegisterParams {
            org_id,
            permissions,
            ip,
            user_agent,
            network_ids,
        } = params;

        request
            .validate()
            .map_err(|e| anyhow!("Validation failed: {}", e))?;

        // Check if email already taken
        let all_users = self
            .user_service
            .get_all(StorableFilter::<User>::new_from_email(&request.email))
            .await?;

        if !all_users.is_empty() {
            return Err(anyhow!("Email address already taken"));
        }

        let terms_accepted_at = if request.terms_accepted {
            Some(Utc::now())
        } else {
            None
        };

        // Provision user with password
        let mut user = self
            .provision_user(
                ProvisionUserParams {
                    email: request.email,
                    password_hash: Some(hash_password(&request.password)?),
                    oidc_subject: None,
                    oidc_provider: None,
                    org_id,
                    permissions,
                    network_ids,
                    terms_accepted_at,
                    billing_enabled,
                    marketing_opt_in: request.marketing_opt_in,
                },
                pending_setup,
            )
            .await?;

        // Handle email verification based on email service availability
        if self.email_service.is_some() {
            // Email service configured: send verification email
            if let Err(e) = self.send_verification_email_internal(&mut user).await {
                tracing::warn!("Failed to send verification email: {}", e);
                // Don't fail registration if email fails - user can resend later
            }
        } else {
            // No email service (self-hosted): auto-verify user
            user.base.email_verified = true;
            self.user_service
                .update(&mut user, AuthenticatedEntity::System)
                .await?;
        }

        let authentication: AuthenticatedEntity = user.clone().into();
        self.event_bus
            .publish_auth(AuthEvent {
                id: Uuid::new_v4(),
                user_id: Some(user.id),
                organization_id: Some(user.base.organization_id),
                timestamp: Utc::now(),
                operation: AuthOperation::Register,
                ip_address: ip,
                user_agent,
                metadata: serde_json::json!({
                    "method": "password",
                    "email_verified": user.base.email_verified
                }),

                authentication,
            })
            .await?;

        Ok(user)
    }

    /// Core user provisioning logic - handles both password and OIDC registration
    /// If pending_setup is provided, uses setup.org_name and marks OnboardingModalCompleted
    /// If billing_enabled is false (self-hosted), sets default billing plan
    pub async fn provision_user(
        &self,
        params: ProvisionUserParams,
        pending_setup: Option<PendingSetup>,
    ) -> Result<User> {
        let ProvisionUserParams {
            email,
            password_hash,
            oidc_subject,
            oidc_provider,
            org_id,
            permissions,
            network_ids,
            terms_accepted_at,
            billing_enabled,
            marketing_opt_in,
        } = params;

        let mut is_new_org = false;

        // If being invited, use provided org ID, otherwise create a new one
        let organization_id = if let Some(org_id) = org_id {
            org_id
        } else {
            is_new_org = true;

            // Use org name from setup if provided, otherwise default
            let org_name = pending_setup
                .as_ref()
                .map(|s| s.org_name.clone())
                .unwrap_or_else(|| "My Organization".to_string());

            // Mark OnboardingModalCompleted if setup was provided (pre-registration setup flow)
            let onboarding = if pending_setup.is_some() {
                vec![TelemetryOperation::OnboardingModalCompleted]
            } else {
                vec![]
            };

            // Set billing plan if billing is disabled (self-hosted)
            let plan = if !billing_enabled {
                Some(crate::server::billing::types::base::BillingPlan::default())
            } else {
                None
            };

            // Create new organization for this user
            let organization = self
                .organization_service
                .create(
                    Organization::new(OrganizationBase {
                        stripe_customer_id: None,
                        name: org_name,
                        plan,
                        plan_status: None,
                        onboarding,
                        has_payment_method: false,
                        trial_end_date: None,
                        brevo_company_id: None,
                    }),
                    AuthenticatedEntity::System,
                )
                .await?;
            organization.id
        };

        // If being invited, will have permissions (default to Viewer in case permissions were lost for some reason); otherwise, new user and should be owner of org
        let permissions = if is_new_org {
            UserOrgPermissions::Owner
        } else {
            permissions.unwrap_or(UserOrgPermissions::Viewer)
        };

        // Create user based on auth method
        let user = if let Some(hash) = password_hash {
            Ok(self
                .user_service
                .create(
                    User::new(UserBase::new_password(
                        email,
                        hash,
                        organization_id,
                        permissions,
                        network_ids,
                        terms_accepted_at,
                    )),
                    AuthenticatedEntity::System,
                )
                .await?)
        } else if let Some(oidc_subject) = oidc_subject {
            Ok(self
                .user_service
                .create(
                    User::new(UserBase::new_oidc(
                        email,
                        oidc_subject,
                        oidc_provider,
                        organization_id,
                        permissions,
                        network_ids,
                        terms_accepted_at,
                    )),
                    AuthenticatedEntity::System,
                )
                .await?)
        } else {
            Err(anyhow!("Must provide either password or OIDC credentials"))
        }?;

        if is_new_org {
            let authentication: AuthenticatedEntity = user.clone().into();

            // Include org_name and onboarding data in metadata for Brevo sync
            let org_name = pending_setup
                .as_ref()
                .map(|s| s.org_name.clone())
                .unwrap_or_else(|| "My Organization".to_string());
            let use_case = pending_setup.as_ref().and_then(|s| s.use_case.clone());
            let company_size = pending_setup.as_ref().and_then(|s| s.company_size.clone());
            let job_title = pending_setup.as_ref().and_then(|s| s.job_title.clone());

            let mut metadata = serde_json::json!({
                "org_name": org_name,
                "marketing_opt_in": marketing_opt_in
            });
            if let Some(use_case) = use_case {
                metadata["use_case"] = serde_json::json!(use_case);
            }
            if let Some(company_size) = company_size {
                metadata["company_size"] = serde_json::json!(company_size);
            }
            if let Some(job_title) = job_title {
                metadata["job_title"] = serde_json::json!(job_title);
            }

            self.event_bus
                .publish_telemetry(TelemetryEvent {
                    id: Uuid::new_v4(),
                    organization_id: user.base.organization_id,
                    operation: TelemetryOperation::OrgCreated,
                    timestamp: Utc::now(),
                    metadata,
                    authentication,
                })
                .await?;
        }

        Ok(user)
    }

    /// Login with username and password
    pub async fn login(
        &self,
        request: LoginRequest,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        request
            .validate()
            .map_err(|e| anyhow!("Validation failed: {}", e))?;

        // Check if account is locked due to too many failed attempts
        self.check_login_lockout(&request.email).await?;

        // Attempt login
        let result = self.try_login(&request).await;

        // Update login attempts based on result
        match result {
            Ok(user) => {
                // Success - clear attempts
                self.login_attempts.write().await.remove(&request.email);

                let authentication: AuthenticatedEntity = user.clone().into();
                self.event_bus
                    .publish_auth(AuthEvent {
                        id: Uuid::new_v4(),
                        user_id: Some(user.id),
                        organization_id: Some(user.base.organization_id),
                        timestamp: Utc::now(),
                        operation: AuthOperation::LoginSuccess,
                        ip_address: ip,
                        user_agent,
                        metadata: serde_json::json!({
                            "method": "password",
                        }),

                        authentication,
                    })
                    .await?;

                Ok(user)
            }
            Err(e) => {
                // Failure - increment attempts

                self.event_bus
                    .publish_auth(AuthEvent {
                        id: Uuid::new_v4(),
                        user_id: None,
                        organization_id: None,
                        timestamp: Utc::now(),
                        operation: AuthOperation::LoginFailed,
                        ip_address: ip,
                        user_agent,
                        metadata: serde_json::json!({
                            "method": "password",
                            "email": request.email
                        }),
                        authentication: AuthenticatedEntity::Anonymous,
                    })
                    .await?;

                let mut attempts = self.login_attempts.write().await;
                let entry = attempts
                    .entry(request.email.clone())
                    .or_insert((0, Instant::now()));
                entry.0 += 1;
                entry.1 = Instant::now();
                Err(e)
            }
        }
    }

    /// Check if user is locked out due to too many login attempts
    async fn check_login_lockout(&self, email: &EmailAddress) -> Result<()> {
        let attempts = self.login_attempts.read().await;
        if let Some((count, last_attempt)) = attempts.get(email)
            && *count >= Self::MAX_LOGIN_ATTEMPTS
        {
            let elapsed = last_attempt.elapsed().as_secs();
            if elapsed < Self::LOCKOUT_DURATION_SECS {
                let remaining = (Self::LOCKOUT_DURATION_SECS - elapsed) / 60;
                return Err(anyhow!(
                    "Too many failed login attempts. Try again in {} minutes.",
                    remaining + 1
                ));
            }
        }
        Ok(())
    }

    /// Attempt login without rate limiting
    async fn try_login(&self, request: &LoginRequest) -> Result<User> {
        // Get user by email
        let user = self
            .user_service
            .get_one(StorableFilter::<User>::new_from_email(&request.email))
            .await?
            .ok_or_else(|| anyhow!("Invalid email or password"))?;

        // Check if user has a password set
        let password_hash = user
            .base
            .password_hash
            .as_ref()
            .ok_or_else(|| anyhow!("User has no password set. Please register first."))?;

        // Verify password
        verify_password(&request.password, password_hash)?;

        // Check if email is verified
        if !user.base.email_verified {
            return Err(anyhow!("Please verify your email before logging in"));
        }

        Ok(user.clone())
    }

    pub async fn update_password(
        &self,
        user_id: Uuid,
        password: Option<String>,
        email: Option<EmailAddress>,
        ip: IpAddr,
        user_agent: Option<String>,
        authentication: AuthenticatedEntity,
    ) -> Result<User> {
        let mut user = self
            .user_service
            .get_by_id(&user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found".to_string()))?;

        if let Some(password) = password {
            user.set_password(hash_password(&password)?);
        }

        if let Some(email) = email {
            user.base.email = email
        }

        self.event_bus
            .publish_auth(AuthEvent {
                id: Uuid::new_v4(),
                user_id: Some(user.id),
                organization_id: Some(user.base.organization_id),
                timestamp: Utc::now(),
                operation: AuthOperation::PasswordChanged,
                ip_address: ip,
                user_agent,
                metadata: serde_json::json!({}),

                authentication: authentication.clone(),
            })
            .await?;

        self.user_service.update(&mut user, authentication).await
    }

    /// Initiate password reset process - generates a token stored in database
    pub async fn initiate_password_reset(
        &self,
        email: &EmailAddress,
        url: String,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<()> {
        let email_service = self
            .email_service
            .as_ref()
            .ok_or_else(|| anyhow!("Email service not configured"))?
            .clone();

        let mut user = match self
            .user_service
            .get_one(StorableFilter::<User>::new_from_email(email))
            .await?
        {
            Some(user) => user,
            None => {
                // User doesn't exist - but we still return Ok to prevent enumeration
                tracing::info!("Password reset requested for non-existent email");
                return Ok(());
            }
        };

        self.event_bus
            .publish_auth(AuthEvent {
                id: Uuid::new_v4(),
                user_id: Some(user.id),
                organization_id: Some(user.base.organization_id),
                timestamp: Utc::now(),
                operation: AuthOperation::PasswordResetRequested,
                ip_address: ip,
                user_agent,
                metadata: serde_json::json!({}),
                authentication: AuthenticatedEntity::Anonymous,
            })
            .await?;

        // Generate token and store in database
        let token = Uuid::new_v4().to_string();
        let expires = Utc::now() + Duration::hours(Self::PASSWORD_RESET_TOKEN_EXPIRY_HOURS);
        user.base.password_reset_token = Some(token.clone());
        user.base.password_reset_expires = Some(expires);
        self.user_service
            .update(&mut user, AuthenticatedEntity::System)
            .await?;

        email_service
            .send_password_reset(user.base.email.clone(), url, token)
            .await?;

        Ok(())
    }

    /// Reset password using token from database
    pub async fn complete_password_reset(
        &self,
        token: &str,
        new_password: &str,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        // Find user by password reset token
        let mut user = self
            .user_service
            .get_one(StorableFilter::<User>::new_from_password_reset_token(token))
            .await?
            .ok_or_else(|| anyhow!("Invalid or expired password reset token"))?;

        // Check if token is expired
        if let Some(expires) = user.base.password_reset_expires {
            if Utc::now() > expires {
                // Clear expired token
                user.base.password_reset_token = None;
                user.base.password_reset_expires = None;
                self.user_service
                    .update(&mut user, AuthenticatedEntity::System)
                    .await?;
                return Err(anyhow!("Password reset token has expired"));
            }
        } else {
            return Err(anyhow!("Invalid password reset token"));
        }

        let authentication: AuthenticatedEntity = user.clone().into();
        self.event_bus
            .publish_auth(AuthEvent {
                id: Uuid::new_v4(),
                user_id: Some(user.id),
                organization_id: Some(user.base.organization_id),
                timestamp: Utc::now(),
                operation: AuthOperation::PasswordResetCompleted,
                ip_address: ip,
                user_agent,
                metadata: serde_json::json!({}),

                authentication,
            })
            .await?;

        // Update password and clear token
        let hashed_password = hash_password(new_password)?;
        user.set_password(hashed_password);
        user.base.password_reset_token = None;
        user.base.password_reset_expires = None;
        self.user_service
            .update(&mut user, AuthenticatedEntity::System)
            .await?;

        Ok(user.clone())
    }

    pub async fn logout(
        &self,
        user_id: Uuid,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<()> {
        if let Ok(Some(user)) = self.user_service.get_by_id(&user_id).await {
            let authentication: AuthenticatedEntity = user.into();
            self.event_bus
                .publish_auth(AuthEvent {
                    id: Uuid::new_v4(),
                    user_id: authentication.user_id(),
                    organization_id: authentication.organization_id(),
                    timestamp: Utc::now(),
                    operation: AuthOperation::LoggedOut,
                    ip_address: ip,
                    user_agent,
                    metadata: serde_json::json!({}),

                    authentication,
                })
                .await?;
        }

        Ok(())
    }

    /// Internal helper to generate verification token and send email
    async fn send_verification_email_internal(&self, user: &mut User) -> Result<()> {
        let email_service = self
            .email_service
            .as_ref()
            .ok_or_else(|| anyhow!("Email service not configured"))?;

        // Generate token and expiry
        let token = Uuid::new_v4().to_string();
        let expires = Utc::now() + Duration::hours(Self::VERIFICATION_TOKEN_EXPIRY_HOURS);

        // Store token in user record
        user.base.email_verification_token = Some(token.clone());
        user.base.email_verification_expires = Some(expires);
        self.user_service
            .update(user, AuthenticatedEntity::System)
            .await?;

        // Send verification email
        email_service
            .send_verification_email(user.base.email.clone(), self.public_url.clone(), token)
            .await?;

        Ok(())
    }

    /// Verify email using token
    pub async fn verify_email(
        &self,
        token: &str,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        // Find user by verification token
        let mut user = self
            .user_service
            .get_one(StorableFilter::<User>::new_from_email_verification_token(
                token,
            ))
            .await?
            .ok_or_else(|| anyhow!("Invalid verification token"))?;

        // Check if token is expired
        if let Some(expires) = user.base.email_verification_expires {
            if Utc::now() > expires {
                return Err(anyhow!(
                    "Verification token has expired. Please request a new one."
                ));
            }
        } else {
            return Err(anyhow!("Invalid verification token"));
        }

        // Mark as verified and clear token
        user.base.email_verified = true;
        user.base.email_verification_token = None;
        user.base.email_verification_expires = None;

        self.user_service
            .update(&mut user, AuthenticatedEntity::System)
            .await?;

        self.event_bus
            .publish_auth(AuthEvent {
                id: Uuid::new_v4(),
                user_id: Some(user.id),
                organization_id: Some(user.base.organization_id),
                timestamp: Utc::now(),
                operation: AuthOperation::EmailVerified,
                ip_address: ip,
                user_agent,
                metadata: serde_json::json!({}),
                authentication: user.clone().into(),
            })
            .await?;

        Ok(user)
    }

    /// Resend verification email with rate limiting
    pub async fn resend_verification_email(&self, email: &EmailAddress) -> Result<()> {
        // Check rate limiting
        {
            let cooldowns = self.verification_resend_cooldown.read().await;
            if let Some(last_sent) = cooldowns.get(email)
                && last_sent.elapsed().as_secs() < Self::RESEND_COOLDOWN_SECS
            {
                let remaining = Self::RESEND_COOLDOWN_SECS - last_sent.elapsed().as_secs();
                return Err(anyhow!(
                    "Please wait {} seconds before requesting another verification email",
                    remaining
                ));
            }
        }

        // Find user
        let mut user = self
            .user_service
            .get_one(StorableFilter::<User>::new_from_email(email))
            .await?
            .ok_or_else(|| anyhow!("User not found"))?;

        // Check if already verified
        if user.base.email_verified {
            return Err(anyhow!("Email is already verified"));
        }

        // Send verification email
        self.send_verification_email_internal(&mut user).await?;

        // Update cooldown
        self.verification_resend_cooldown
            .write()
            .await
            .insert(email.clone(), Instant::now());

        Ok(())
    }

    /// Cleanup old login attempts (called periodically from background task)
    pub async fn cleanup_old_login_attempts(&self) {
        let mut attempts = self.login_attempts.write().await;

        attempts.retain(|_, (_, last_attempt)| {
            last_attempt.elapsed().as_secs() < Self::LOCKOUT_DURATION_SECS
        });

        tracing::debug!("Cleaned up old login attempts");
    }

    /// Cleanup old verification resend cooldowns
    pub async fn cleanup_old_verification_cooldowns(&self) {
        let mut cooldowns = self.verification_resend_cooldown.write().await;

        cooldowns
            .retain(|_, last_sent| last_sent.elapsed().as_secs() < Self::RESEND_COOLDOWN_SECS * 2);

        tracing::debug!("Cleaned up old verification cooldowns");
    }
}

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Password hashing failed: {}", e))?
        .to_string();

    Ok(hash)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<()> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow!("Invalid password hash: {}", e))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| anyhow!("Invalid username or password"))
}
