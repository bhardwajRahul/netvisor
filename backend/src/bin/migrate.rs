//! Standalone migration runner used by `make migrate-db` / `make dev-fresh`.
//!
//! Uses the same `apply_migrations` path the server's startup uses so that
//! `-- no-transaction` migrations (e.g. `CREATE INDEX CONCURRENTLY`) work
//! identically in both contexts. The stock `sqlx migrate run` CLI does not
//! handle these correctly — see the comment at the top of
//! `server/shared/storage/migration_runner.rs` for the underlying PG
//! simple_query / implicit-transaction issue.

use std::path::PathBuf;

use clap::Parser;
use scanopy::server::shared::storage::migration_runner::apply_migrations;
use sqlx::postgres::PgPoolOptions;

#[derive(Parser, Debug)]
#[command(about = "Apply pending Scanopy migrations using the custom runner")]
struct Args {
    /// Postgres connection string.
    #[arg(long)]
    database_url: String,

    /// Directory containing the migration SQL files.
    #[arg(long, default_value = "./migrations")]
    migrations_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&args.database_url)
        .await?;

    apply_migrations(&pool, &args.migrations_dir).await?;

    pool.close().await;
    Ok(())
}
