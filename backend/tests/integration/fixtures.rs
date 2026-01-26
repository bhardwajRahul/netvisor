use scanopy::server::bindings::r#impl::base::Binding;
use scanopy::server::daemon_api_keys::r#impl::base::DaemonApiKey;
use scanopy::server::daemons::r#impl::base::Daemon;
use scanopy::server::discovery::r#impl::base::Discovery;
use scanopy::server::groups::r#impl::base::Group;
use scanopy::server::hosts::r#impl::base::Host;
use scanopy::server::if_entries::r#impl::base::IfEntry;
use scanopy::server::interfaces::r#impl::base::Interface;
use scanopy::server::invites::r#impl::base::Invite;
use scanopy::server::networks::r#impl::Network;
use scanopy::server::organizations::r#impl::base::Organization;
use scanopy::server::ports::r#impl::base::Port;
use scanopy::server::services::definitions::ServiceDefinitionRegistry;
use scanopy::server::services::r#impl::base::Service;
use scanopy::server::services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt};
use scanopy::server::shared::entity_metadata::EntityCategory;
use scanopy::server::shared::storage::traits::{Entity, Storable};
use scanopy::server::shared::types::metadata::EntityMetadataProvider;
use scanopy::server::shares::r#impl::base::Share;
use scanopy::server::snmp_credentials::r#impl::base::SnmpCredential;
use scanopy::server::subnets::r#impl::base::Subnet;
use scanopy::server::tags::r#impl::base::Tag;
use scanopy::server::topology::types::base::Topology;
use scanopy::server::user_api_keys::r#impl::base::UserApiKey;
use scanopy::server::users::r#impl::base::User;
use serde::Serialize;

/// Generate all fixtures (requires Docker containers to be running, except OpenAPI)
pub async fn generate_fixtures() {
    generate_db_fixture()
        .await
        .expect("Failed to generate db fixture");

    generate_daemon_config_fixture()
        .await
        .expect("Failed to generate daemon config fixture");

    generate_services_json()
        .await
        .expect("Failed to generate services json");

    generate_billing_plans_json()
        .await
        .expect("Failed to generate billing and features json");

    generate_schema_mermaid()
        .await
        .expect("Failed to generate schema mermaid");

    generate_entity_metadata_json()
        .await
        .expect("Failed to generate entity metadata json");

    // OpenAPI generation - public spec only (excludes internal endpoints)
    let openapi_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static/openapi-public.json");
    super::openapi_gen::generate_public(&openapi_path).expect("Failed to generate OpenAPI spec");

    println!("✅ Generated test fixtures");
}

async fn generate_db_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("docker")
        .args([
            "exec",
            "scanopy-postgres-dev-1",
            "pg_dump",
            "-U",
            "postgres",
            "-d",
            "scanopy",
            "--clean",
            "--if-exists",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "pg_dump failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let fixture_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/scanopy-next.sql");
    std::fs::write(&fixture_path, output.stdout)?;

    println!("✅ Generated scanopy-next.sql from test data");
    Ok(())
}

async fn generate_daemon_config_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let find_output = std::process::Command::new("docker")
        .args([
            "exec",
            "scanopy-daemon-1",
            "find",
            "/root/.config",
            "-name",
            "config.json",
            "-type",
            "f",
        ])
        .output()?;

    if !find_output.status.success() {
        return Err(format!(
            "Failed to find daemon config: {}",
            String::from_utf8_lossy(&find_output.stderr)
        )
        .into());
    }

    let config_path = String::from_utf8_lossy(&find_output.stdout)
        .trim()
        .to_string();

    if config_path.is_empty() {
        return Err("No config.json found in container".into());
    }

    println!("Found daemon config at: {}", config_path);

    let output = std::process::Command::new("docker")
        .args(["exec", "scanopy-daemon-1", "cat", &config_path])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to read daemon config: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let fixture_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/daemon_config-next.json");
    std::fs::write(&fixture_path, output.stdout)?;

    println!("✅ Generated daemon_config-next.json from test daemon");
    Ok(())
}

async fn generate_services_json() -> Result<(), Box<dyn std::error::Error>> {
    let services: Vec<serde_json::Value> = ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .filter_map(|s| {
            if s.can_be_manually_added() {
                Some(serde_json::json!({
                    "logo_url": s.logo_url(),
                    "name": s.name(),
                    "description": s.description(),
                    "discovery_pattern": s.discovery_pattern().to_string(),
                    "category": s.category(),
                    "color": s.color(),
                    "logo_needs_white_background": s.logo_needs_white_background()
                }))
            } else {
                None
            }
        })
        .collect();

    let json_string = serde_json::to_string_pretty(&services)?;
    let json_path = std::path::Path::new("../ui/static/services-next.json");
    tokio::fs::write(json_path, json_string).await?;

    Ok(())
}

async fn generate_billing_plans_json() -> Result<(), Box<dyn std::error::Error>> {
    use scanopy::server::billing::plans::get_website_fixture_plans;
    use scanopy::server::billing::types::features::Feature;
    use scanopy::server::shared::types::metadata::{MetadataProvider, TypeMetadata};
    use strum::IntoEnumIterator;

    let plans = get_website_fixture_plans();
    let plan_metadata: Vec<TypeMetadata> = plans.iter().map(|p| p.to_metadata()).collect();
    let feature_metadata: Vec<TypeMetadata> = Feature::iter().map(|f| f.to_metadata()).collect();

    let json_string = serde_json::to_string_pretty(&plan_metadata)?;
    let path = std::path::Path::new("../ui/src/lib/data/billing-plans-next.json");
    tokio::fs::write(path, json_string).await?;

    let json_string = serde_json::to_string_pretty(&feature_metadata)?;
    let path = std::path::Path::new("../ui/src/lib/data/features-next.json");
    tokio::fs::write(path, json_string).await?;

    println!("✅ Generated billing-plans-next.json and features-next.json in ui/src/lib/data/");
    Ok(())
}

/// Entity metadata entry for documentation generation
#[derive(Serialize)]
struct EntityMetadataEntry {
    /// Unique identifier (e.g., "host")
    id: &'static str,
    /// Singular name (e.g., "Host")
    name_singular: &'static str,
    /// Plural name (e.g., "Hosts")
    name_plural: &'static str,
    /// Description for documentation
    description: &'static str,
    /// Category key (e.g., "network_infrastructure")
    category: &'static str,
    /// Human-readable category name (e.g., "Network Infrastructure")
    category_display: &'static str,
    /// Database table name (e.g., "hosts")
    table_name: &'static str,
}

impl EntityMetadataEntry {
    fn new<E: Entity + Storable>(id: &'static str) -> Self {
        let category = E::entity_category();
        Self {
            id,
            name_singular: E::ENTITY_NAME_SINGULAR,
            name_plural: E::ENTITY_NAME_PLURAL,
            description: E::ENTITY_DESCRIPTION,
            category: category_to_snake_case(category),
            category_display: category.display_name(),
            table_name: E::table_name(),
        }
    }
}

fn category_to_snake_case(category: EntityCategory) -> &'static str {
    match category {
        EntityCategory::OrganizationsAndUsers => "organizations_and_users",
        EntityCategory::NetworkInfrastructure => "network_infrastructure",
        EntityCategory::DiscoveryAndDaemons => "discovery_and_daemons",
        EntityCategory::Visualization => "visualization",
        EntityCategory::Metadata => "metadata",
    }
}

async fn generate_entity_metadata_json() -> Result<(), Box<dyn std::error::Error>> {
    let metadata: Vec<EntityMetadataEntry> = vec![
        // Organizations & Users
        EntityMetadataEntry::new::<Organization>("organization"),
        EntityMetadataEntry::new::<User>("user"),
        EntityMetadataEntry::new::<Invite>("invite"),
        EntityMetadataEntry::new::<UserApiKey>("user_api_key"),
        // Network Infrastructure
        EntityMetadataEntry::new::<Network>("network"),
        EntityMetadataEntry::new::<Host>("host"),
        EntityMetadataEntry::new::<Subnet>("subnet"),
        EntityMetadataEntry::new::<Interface>("interface"),
        EntityMetadataEntry::new::<Port>("port"),
        EntityMetadataEntry::new::<Service>("service"),
        EntityMetadataEntry::new::<Binding>("binding"),
        EntityMetadataEntry::new::<IfEntry>("if_entry"),
        // Discovery & Daemons
        EntityMetadataEntry::new::<Daemon>("daemon"),
        EntityMetadataEntry::new::<DaemonApiKey>("daemon_api_key"),
        EntityMetadataEntry::new::<Discovery>("discovery"),
        EntityMetadataEntry::new::<SnmpCredential>("snmp_credential"),
        // Visualization
        EntityMetadataEntry::new::<Group>("group"),
        EntityMetadataEntry::new::<Topology>("topology"),
        EntityMetadataEntry::new::<Share>("share"),
        // Metadata
        EntityMetadataEntry::new::<Tag>("tag"),
    ];

    let json_string = serde_json::to_string_pretty(&metadata)?;
    let json_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static/entity-metadata-next.json");
    tokio::fs::write(&json_path, json_string).await?;

    println!("✅ Generated entity-metadata-next.json");
    Ok(())
}

async fn generate_schema_mermaid() -> Result<(), Box<dyn std::error::Error>> {
    // Check if tbls is available (graceful skip for local dev without tbls)
    let which = std::process::Command::new("which").arg("tbls").output();
    if which.is_err() || !which.unwrap().status.success() {
        println!("⚠️  tbls not found, skipping schema generation");
        return Ok(());
    }

    let temp_dir = std::env::temp_dir().join("tbls-schema");
    let _ = std::fs::remove_dir_all(&temp_dir);

    // tbls runs on host, connects to exposed port 5435
    let output = std::process::Command::new("tbls")
        .args([
            "doc",
            "postgres://postgres:password@localhost:5435/scanopy?sslmode=disable",
            temp_dir.to_str().unwrap(),
            "--er-format",
            "mermaid",
            "--exclude",
            "sqlx_migrations",
            "--force",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!("tbls failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    // Extract mermaid block from README.md
    let readme_path = temp_dir.join("README.md");
    let readme_content = std::fs::read_to_string(&readme_path)?;

    let mermaid = readme_content
        .lines()
        .skip_while(|line| *line != "```mermaid")
        .skip(1) // skip the ```mermaid line
        .take_while(|line| *line != "```")
        .collect::<Vec<_>>()
        .join("\n");

    let _ = std::fs::remove_dir_all(&temp_dir);

    // Full schema with all columns
    let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static/schema-next.mermaid");
    std::fs::write(&schema_path, &mermaid)?;
    println!("✅ Generated schema-next.mermaid");

    // Simplified ER diagram (relationships only, no columns)
    let simplified_er = generate_simplified_er(&mermaid);
    let er_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static/schema-er-next.mermaid");
    std::fs::write(&er_path, simplified_er)?;
    println!("✅ Generated schema-er-next.mermaid");

    Ok(())
}

/// Generate a simplified ER diagram from the full tbls mermaid output.
/// This strips the attribute blocks (columns) and keeps only table names and relationships.
fn generate_simplified_er(full_mermaid: &str) -> String {
    let mut result = Vec::new();
    let mut in_table_block = false;

    for line in full_mermaid.lines() {
        let trimmed = line.trim();

        // Keep the erDiagram declaration
        if trimmed == "erDiagram" {
            result.push(line.to_string());
            continue;
        }

        // Detect start of table block (table name followed by {)
        if trimmed.ends_with('{') {
            in_table_block = true;
            // Extract table name and add it without attributes
            let table_name = trimmed.trim_end_matches('{').trim();
            result.push(format!("  {}", table_name));
            continue;
        }

        // End of table block
        if trimmed == "}" {
            in_table_block = false;
            continue;
        }

        // Skip attribute lines inside table blocks
        if in_table_block {
            continue;
        }

        // Keep relationship lines (contain || or }o or |{ etc)
        if trimmed.contains("||")
            || trimmed.contains("}o")
            || trimmed.contains("|{")
            || trimmed.contains("o{")
        {
            result.push(line.to_string());
        }
    }

    result.join("\n")
}
