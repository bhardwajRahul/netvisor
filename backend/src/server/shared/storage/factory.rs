use anyhow::Result;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Pool, Postgres};
use std::str::FromStr;
use std::sync::Arc;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

use crate::server::dependencies::dependency_members::DependencyMemberStorage;
use crate::server::tags::entity_tags::EntityTagStorage;
use crate::server::user_api_keys::r#impl::network_access::UserApiKeyNetworkAccessStorage;
use crate::server::users::UserNetworkAccessStorage;
use crate::server::vlans::r#impl::subnet_vlans::SubnetVlanStorage;
use crate::server::{
    bindings::r#impl::base::Binding, credentials::r#impl::base::Credential,
    daemon_api_keys::r#impl::base::DaemonApiKey, daemons::r#impl::base::Daemon,
    dependencies::r#impl::base::Dependency, discovery::r#impl::base::Discovery,
    hosts::r#impl::base::Host, interfaces::r#impl::base::Interface, invites::r#impl::base::Invite,
    ip_addresses::r#impl::base::IPAddress, networks::r#impl::Network,
    organizations::r#impl::base::Organization, ports::r#impl::base::Port,
    services::r#impl::base::Service, shared::storage::generic::GenericPostgresStorage,
    shares::r#impl::base::Share, subnets::r#impl::base::Subnet, tags::r#impl::base::Tag,
    topology::types::base::Topology, user_api_keys::r#impl::base::UserApiKey,
    users::r#impl::base::User, vlans::r#impl::base::Vlan,
};

pub struct StorageFactory {
    pub pool: PgPool,
    pub sessions: SessionManagerLayer<PostgresStore>,
    pub daemon_api_keys: Arc<GenericPostgresStorage<DaemonApiKey>>,
    pub user_api_keys: Arc<GenericPostgresStorage<UserApiKey>>,
    pub users: Arc<GenericPostgresStorage<User>>,
    pub networks: Arc<GenericPostgresStorage<Network>>,
    pub hosts: Arc<GenericPostgresStorage<Host>>,
    pub ip_addresses: Arc<GenericPostgresStorage<IPAddress>>,
    pub dependencies: Arc<GenericPostgresStorage<Dependency>>,
    pub daemons: Arc<GenericPostgresStorage<Daemon>>,
    pub subnets: Arc<GenericPostgresStorage<Subnet>>,
    pub services: Arc<GenericPostgresStorage<Service>>,
    pub organizations: Arc<GenericPostgresStorage<Organization>>,
    pub invites: Arc<GenericPostgresStorage<Invite>>,
    pub shares: Arc<GenericPostgresStorage<Share>>,
    pub discovery: Arc<GenericPostgresStorage<Discovery>>,
    pub topologies: Arc<GenericPostgresStorage<Topology>>,
    pub tags: Arc<GenericPostgresStorage<Tag>>,
    pub ports: Arc<GenericPostgresStorage<Port>>,
    pub bindings: Arc<GenericPostgresStorage<Binding>>,
    pub credentials: Arc<GenericPostgresStorage<Credential>>,
    pub interfaces: Arc<GenericPostgresStorage<Interface>>,
    pub vlans: Arc<GenericPostgresStorage<Vlan>>,
    // Junction tables
    pub entity_tags: Arc<EntityTagStorage>,
    pub user_api_key_network_access: Arc<UserApiKeyNetworkAccessStorage>,
    pub user_network_access: Arc<UserNetworkAccessStorage>,
    pub dependency_members: Arc<DependencyMemberStorage>,
    pub subnet_vlan: Arc<SubnetVlanStorage>,
}

pub async fn create_session_store(
    db_pool: Pool<Postgres>,
    use_secure: bool,
) -> Result<SessionManagerLayer<PostgresStore>> {
    let session_store = PostgresStore::new(db_pool.clone());

    session_store.migrate().await?;

    Ok(SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)))
        .with_name("session_id")
        .with_secure(use_secure)
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax))
}

impl StorageFactory {
    pub async fn new(database_url: &str, use_secure_session_cookies: bool) -> Result<Self> {
        let connect_options = PgConnectOptions::from_str(database_url)?.statement_cache_capacity(0);
        let pool = PgPoolOptions::new().connect_with(connect_options).await?;

        crate::server::shared::storage::migration_runner::apply_migrations(
            &pool,
            std::path::Path::new("./migrations"),
        )
        .await?;

        let sessions = create_session_store(pool.clone(), use_secure_session_cookies).await?;

        Ok(Self {
            pool: pool.clone(),
            sessions,
            discovery: Arc::new(GenericPostgresStorage::new(pool.clone())),
            organizations: Arc::new(GenericPostgresStorage::new(pool.clone())),
            invites: Arc::new(GenericPostgresStorage::new(pool.clone())),
            shares: Arc::new(GenericPostgresStorage::new(pool.clone())),
            daemon_api_keys: Arc::new(GenericPostgresStorage::new(pool.clone())),
            user_api_keys: Arc::new(GenericPostgresStorage::new(pool.clone())),
            users: Arc::new(GenericPostgresStorage::new(pool.clone())),
            networks: Arc::new(GenericPostgresStorage::new(pool.clone())),
            hosts: Arc::new(GenericPostgresStorage::new(pool.clone())),
            ip_addresses: Arc::new(GenericPostgresStorage::new(pool.clone())),
            dependencies: Arc::new(GenericPostgresStorage::new(pool.clone())),
            daemons: Arc::new(GenericPostgresStorage::new(pool.clone())),
            subnets: Arc::new(GenericPostgresStorage::new(pool.clone())),
            services: Arc::new(GenericPostgresStorage::new(pool.clone())),
            topologies: Arc::new(GenericPostgresStorage::new(pool.clone())),
            tags: Arc::new(GenericPostgresStorage::new(pool.clone())),
            ports: Arc::new(GenericPostgresStorage::new(pool.clone())),
            bindings: Arc::new(GenericPostgresStorage::new(pool.clone())),
            credentials: Arc::new(GenericPostgresStorage::new(pool.clone())),
            interfaces: Arc::new(GenericPostgresStorage::new(pool.clone())),
            vlans: Arc::new(GenericPostgresStorage::new(pool.clone())),
            // Junction tables
            entity_tags: Arc::new(EntityTagStorage::new(pool.clone())),
            user_api_key_network_access: Arc::new(UserApiKeyNetworkAccessStorage::new(
                pool.clone(),
            )),
            dependency_members: Arc::new(DependencyMemberStorage::new(pool.clone())),
            subnet_vlan: Arc::new(SubnetVlanStorage::new(pool.clone())),
            user_network_access: Arc::new(UserNetworkAccessStorage::new(pool.clone())),
        })
    }
}
