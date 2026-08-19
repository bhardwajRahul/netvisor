//! Custom migration runner that handles `-- no-transaction` migrations
//! containing `CREATE INDEX CONCURRENTLY` and similar statements that
//! PostgreSQL forbids inside a transaction block.
//!
//! Background: sqlx's `Migrator::run` calls `conn.execute(&migration.sql)`
//! which sends the entire migration as one PG simple_query. PG bundles
//! multi-statement simple_query messages into an implicit transaction —
//! even when the migration has the `-- no-transaction` header, because the
//! header only controls whether sqlx wraps the call in `BEGIN`/`COMMIT`,
//! not how PG itself handles the wire-level Query message. For DDL like
//! `CREATE INDEX CONCURRENTLY` that explicitly errors inside any
//! transaction (even an implicit one), the only fix is to send each
//! statement as its own simple_query.
//!
//! This runner is used in both server startup (`StorageFactory::new`) and
//! test setup so production deploys and CI agree on migration application
//! semantics.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Context;
use sha2::{Digest, Sha384};
use sqlx::PgPool;

/// Apply pending migrations from `migrations_dir` to the database `pool`.
///
/// For each migration file (sorted by `<version>` prefix):
/// - If the file starts with `-- no-transaction`, split on `;` and send
///   each statement as its own `pool.execute()` call. Guards against
///   `$$`-quoted blocks (would break naive splitting); migrations in this
///   path must be plain DDL.
/// - Otherwise, wrap the whole file in a transaction and execute as one.
///
/// Records each successfully applied migration in `_sqlx_migrations`
/// using sqlx's bookkeeping schema, so re-applies are idempotent and
/// state stays compatible with sqlx-cli.
pub async fn apply_migrations(pool: &PgPool, migrations_dir: &Path) -> anyhow::Result<()> {
    ensure_migrations_table(pool).await?;
    let applied = applied_versions(pool).await?;

    let mut entries: Vec<(i64, String, PathBuf)> = std::fs::read_dir(migrations_dir)
        .with_context(|| format!("reading migrations dir {}", migrations_dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| parse_migration_file(&entry.path()))
        .collect();
    entries.sort_by_key(|(version, _, _)| *version);

    for (version, description, path) in entries {
        if applied.contains(&version) {
            continue;
        }

        let sql = std::fs::read_to_string(&path)
            .with_context(|| format!("reading migration {}", path.display()))?;

        if sql.starts_with("-- no-transaction") {
            apply_no_tx_migration(pool, &path, &sql).await?;
        } else {
            apply_tx_migration(pool, &sql)
                .await
                .with_context(|| format!("applying migration {}", path.display()))?;
        }

        record_migration(pool, version, &description, &sql).await?;
    }

    Ok(())
}

async fn apply_no_tx_migration(pool: &PgPool, path: &Path, sql: &str) -> anyhow::Result<()> {
    for stmt in split_statements(sql) {
        // `raw_sql` uses PG's simple_query protocol, bypassing the prepared-
        // statement path that `sqlx::query()` uses. Required because
        // `CREATE INDEX CONCURRENTLY` and similar can't be prepared, and
        // because we want each statement to run as its own implicit
        // transaction (the whole point of the no-tx path).
        sqlx::raw_sql(&stmt)
            .execute(pool)
            .await
            .with_context(|| format!("executing statement from {}", path.display()))?;
    }
    Ok(())
}

async fn apply_tx_migration(pool: &PgPool, sql: &str) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    // `raw_sql` uses simple_query so multi-statement migrations work; the
    // outer transaction provides the BEGIN/COMMIT envelope.
    sqlx::raw_sql(sql).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Split SQL into individual statements on `;` boundaries, dropping `--` line
/// comments as it goes. Each returned statement has its trailing `;` re-attached.
///
/// A single scan rather than a line-wise strip followed by `split(';')`, because a
/// semicolon only ends a statement when it is not inside something: a `'...'` literal,
/// a `--` comment, or a `$tag$ ... $tag$` dollar-quoted block. The dollar-quote case is
/// the one that matters — a batched backfill's `DO $$ ... ; ... $$` body is full of
/// semicolons, and splitting on them yields fragments that are not valid SQL. This
/// runner used to refuse such migrations outright; a batched backfill is exactly what
/// the migration guidelines ask for on a table with data, so it has to be able to run one.
fn split_statements(sql: &str) -> Vec<String> {
    let chars: Vec<char> = sql.chars().collect();
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut i = 0;

    while i < chars.len() {
        // `--` to end of line.
        if chars[i] == '-' && chars.get(i + 1) == Some(&'-') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // '...', with '' as the escape for a literal quote.
        if chars[i] == '\'' {
            current.push(chars[i]);
            i += 1;
            while i < chars.len() {
                current.push(chars[i]);
                if chars[i] == '\'' {
                    if chars.get(i + 1) == Some(&'\'') {
                        current.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // $tag$ ... $tag$ — copied through verbatim, semicolons and all.
        if chars[i] == '$'
            && let Some(tag) = dollar_tag_at(&chars, i)
        {
            current.push_str(&tag);
            i += tag.chars().count();
            while i < chars.len() {
                if chars[i] == '$' && dollar_tag_at(&chars, i).as_deref() == Some(tag.as_str()) {
                    current.push_str(&tag);
                    i += tag.chars().count();
                    break;
                }
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }

        if chars[i] == ';' {
            push_statement(&mut statements, &mut current);
            i += 1;
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    push_statement(&mut statements, &mut current);
    statements
}

fn push_statement(statements: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        statements.push(format!("{trimmed};"));
    }
    current.clear();
}

/// The dollar-quote delimiter starting at `start` (`$$`, `$body$`, …), or `None` when
/// this `$` is something else — a positional parameter, or part of an identifier.
fn dollar_tag_at(chars: &[char], start: usize) -> Option<String> {
    let mut end = start + 1;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    if chars.get(end) != Some(&'$') {
        return None;
    }
    Some(chars[start..=end].iter().collect())
}

fn parse_migration_file(path: &Path) -> Option<(i64, String, PathBuf)> {
    let file_name = path.file_name()?.to_str()?;
    if !file_name.ends_with(".sql") {
        return None;
    }
    let stem = file_name.trim_end_matches(".sql");
    let (version_str, description) = stem.split_once('_')?;
    let version = version_str.parse::<i64>().ok()?;
    Some((version, description.to_string(), path.to_path_buf()))
}

async fn ensure_migrations_table(pool: &PgPool) -> anyhow::Result<()> {
    // Schema mirrors sqlx-postgres's `_sqlx_migrations` so cross-tooling
    // (sqlx-cli, future use of `sqlx::migrate!`) reads consistent state.
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            success BOOLEAN NOT NULL,
            checksum BYTEA NOT NULL,
            execution_time BIGINT NOT NULL
        )"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn applied_versions(pool: &PgPool) -> anyhow::Result<HashSet<i64>> {
    let rows: Vec<(i64,)> =
        sqlx::query_as("SELECT version FROM _sqlx_migrations WHERE success = TRUE")
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(v,)| v).collect())
}

async fn record_migration(
    pool: &PgPool,
    version: i64,
    description: &str,
    sql: &str,
) -> anyhow::Result<()> {
    let checksum = Sha384::digest(sql.as_bytes()).to_vec();
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES ($1, $2, TRUE, $3, -1)
         ON CONFLICT (version) DO NOTHING",
    )
    .bind(version)
    .bind(description)
    .bind(checksum)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_statements_basic() {
        let sql = "CREATE INDEX a ON t (x);\nCREATE INDEX b ON t (y);";
        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].starts_with("CREATE INDEX a"));
        assert!(stmts[1].starts_with("CREATE INDEX b"));
        // Each statement re-attaches its trailing semicolon.
        assert!(stmts[0].ends_with(';'));
        assert!(stmts[1].ends_with(';'));
    }

    #[test]
    fn split_statements_drops_comment_only_segments() {
        let sql = "-- header\n-- another\nCREATE INDEX a ON t (x);\n-- trailing comment\n";
        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 1);
        assert!(stmts[0].contains("CREATE INDEX a"));
    }

    #[test]
    fn split_statements_strips_inline_comments() {
        let sql = "CREATE INDEX a ON t (x)\n-- this comment should not appear\n  WHERE x IS NULL;";
        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 1);
        assert!(stmts[0].contains("CREATE INDEX a"));
        assert!(stmts[0].contains("WHERE x IS NULL"));
        assert!(!stmts[0].contains("this comment"));
    }

    #[test]
    fn split_statements_does_not_split_on_semicolon_inside_comment() {
        // Regression: an SQL line-comment containing `;` would cause naive
        // split-on-semicolon to start a new statement mid-comment, which PG
        // would parse as garbage. Strip comments before splitting.
        let sql = "-- The existing index lives on under the same name; we create the new one.\nCREATE INDEX a ON t (x);";
        let stmts = split_statements(sql);
        assert_eq!(
            stmts.len(),
            1,
            "comment-only segments must not produce statements: {stmts:?}"
        );
        assert!(stmts[0].contains("CREATE INDEX a"));
    }

    #[test]
    fn split_statements_handles_multiline_statement() {
        let sql =
            "CREATE TABLE foo (\n    id UUID,\n    name TEXT\n);\nCREATE INDEX b ON foo (id);";
        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].contains("CREATE TABLE foo"));
        assert!(stmts[1].contains("CREATE INDEX b"));
    }

    #[test]
    fn parse_migration_file_extracts_version_and_description() {
        let p = Path::new("/tmp/migrations/20260502000004_scd2_partial_unique_indexes.sql");
        let parsed = parse_migration_file(p).expect("should parse");
        assert_eq!(parsed.0, 20260502000004);
        assert_eq!(parsed.1, "scd2_partial_unique_indexes");
    }

    #[test]
    fn parse_migration_file_rejects_non_sql() {
        let p = Path::new("/tmp/migrations/README.md");
        assert!(parse_migration_file(p).is_none());
    }

    #[test]
    fn split_statements_keeps_a_dollar_quoted_block_whole() {
        // A batched backfill's DO block is full of semicolons. Splitting on them would hand
        // the server fragments that are not valid SQL, which is why this used to be refused
        // outright rather than mis-executed.
        let sql = "SET lock_timeout = '5s';\n\
                   DO $$\n\
                   BEGIN\n\
                     UPDATE hosts SET name = 'a';\n\
                     COMMIT;\n\
                   END $$;\n\
                   ALTER TABLE hosts ADD COLUMN x TEXT;";
        let stmts = split_statements(sql);
        assert_eq!(stmts.len(), 3, "got {stmts:?}");
        assert!(stmts[1].starts_with("DO $$"));
        assert!(stmts[1].contains("UPDATE hosts SET name = 'a';"));
        assert!(stmts[1].contains("COMMIT;"));
        assert!(stmts[2].starts_with("ALTER TABLE"));
    }

    #[test]
    fn split_statements_does_not_split_inside_a_string_literal() {
        let stmts = split_statements("INSERT INTO t VALUES ('a;b'); SELECT 1;");
        assert_eq!(stmts.len(), 2, "got {stmts:?}");
        assert!(stmts[0].contains("'a;b'"));
    }
}
