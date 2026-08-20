use std::borrow::Cow;

use sqlx::SqlitePool;
use sqlx::migrate::Migrator;

pub(crate) static STATE_MIGRATOR: Migrator = sqlx::migrate!("./migrations");
pub(crate) static LOGS_MIGRATOR: Migrator = sqlx::migrate!("./logs_migrations");
pub(crate) static GOALS_MIGRATOR: Migrator = sqlx::migrate!("./goals_migrations");
pub(crate) static MEMORIES_MIGRATOR: Migrator = sqlx::migrate!("./memory_migrations");
pub(crate) static QUEUE_MIGRATOR: Migrator = sqlx::migrate!("./queue_migrations");
pub(crate) static THREAD_HISTORY_MIGRATOR: Migrator = sqlx::migrate!("./thread_history_migrations");

/// Allow an older Codex binary to open a database that has already been
/// migrated by a newer binary running in parallel.
///
/// We intentionally ignore applied migration versions that are newer than the
/// embedded migration set. Known migration versions are still validated by
/// checksum, so this only relaxes the "database is ahead of me" case.
fn runtime_migrator(base: &'static Migrator) -> Migrator {
    Migrator {
        migrations: Cow::Borrowed(base.migrations.as_ref()),
        ignore_missing: true,
        locking: base.locking,
        no_tx: base.no_tx,
        table_name: base.table_name.clone(),
        create_schemas: base.create_schemas.clone(),
    }
}

pub(crate) fn runtime_state_migrator() -> Migrator {
    runtime_migrator(&STATE_MIGRATOR)
}

pub(crate) fn runtime_logs_migrator() -> Migrator {
    runtime_migrator(&LOGS_MIGRATOR)
}

pub(crate) fn runtime_goals_migrator() -> Migrator {
    runtime_migrator(&GOALS_MIGRATOR)
}

pub(crate) fn runtime_memories_migrator() -> Migrator {
    runtime_migrator(&MEMORIES_MIGRATOR)
}

pub(crate) fn runtime_queue_migrator() -> Migrator {
    runtime_migrator(&QUEUE_MIGRATOR)
}

// The paginated history projector will call this when it takes ownership of opening the database.
#[allow(dead_code)]
pub(crate) fn runtime_thread_history_migrator() -> Migrator {
    runtime_migrator(&THREAD_HISTORY_MIGRATOR)
}

const LEGACY_WHALE_TASKSPACE_MIGRATIONS: [(i64, &str); 2] = [
    (
        30,
        "e8b7fd4f72e583f648a6eac076ae000e63b3ffc7fca86e64321b7a60293beb9c9441d8cab044fd92a298d8cb90c387eb",
    ),
    (
        31,
        "edf1e09cca8f6c1340648cba32c0dd11e2129fa6763c846304aa9c40eb8341fb1aace4b9c0b4b3b6ba6e86691c7fe7f1",
    ),
];

const WHALE_0147_TASKSPACE_MIGRATIONS: [(i64, i64, &str); 2] = [
    (
        47,
        51,
        "98f170c12ba97ea33b511af5ed58fab58913c8da007bc93e69d8673584e9984ab471c15d39f30bcb74bbc3d84ee8bb3b",
    ),
    (
        48,
        52,
        "dd29b7f828e39019bb5ba684d11573b399ecf7b7bcb0988adcc925493b8e8b4045991ef0eb7a402634e61df92b6ed573",
    ),
];

/// Move the TaskSpace migrations shipped by Whale's 0.147 substrate out of
/// the versions that Codex 0.149 subsequently assigned to upstream schema.
///
/// Both legacy checksums must match and both destination versions must be
/// absent. Unknown or partial histories remain untouched so SQLx rejects them.
pub(crate) async fn repair_whale_0147_taskspace_migration_versions(
    pool: &SqlitePool,
    migrator: &Migrator,
) -> anyhow::Result<()> {
    let migrations_table_exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await?
    .is_some();
    if !migrations_table_exists {
        return Ok(());
    }

    let applied = sqlx::query_as::<_, (i64, String)>(
        r#"
SELECT version, lower(hex(checksum))
FROM _sqlx_migrations
WHERE version IN (47, 48) AND success = 1
ORDER BY version
        "#,
    )
    .fetch_all(pool)
    .await?;
    let expected = WHALE_0147_TASKSPACE_MIGRATIONS
        .iter()
        .map(|(legacy_version, _, checksum)| (*legacy_version, (*checksum).to_string()))
        .collect::<Vec<_>>();
    if applied != expected {
        return Ok(());
    }

    let destination_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM _sqlx_migrations WHERE version IN (51, 52)",
    )
    .fetch_one(pool)
    .await?;
    if destination_count != 0 {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for (legacy_version, current_version, legacy_checksum) in WHALE_0147_TASKSPACE_MIGRATIONS {
        let current = migrator
            .migrations
            .iter()
            .find(|migration| migration.version == current_version)
            .ok_or_else(|| {
                anyhow::anyhow!("current state migration {current_version} is missing")
            })?;
        let result = sqlx::query(
            r#"
UPDATE _sqlx_migrations
SET version = ?, description = ?, checksum = ?
WHERE version = ? AND success = 1 AND lower(hex(checksum)) = ?
            "#,
        )
        .bind(current_version)
        .bind(current.description.as_ref())
        .bind(current.checksum.as_ref())
        .bind(legacy_version)
        .bind(legacy_checksum)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            anyhow::bail!("Whale 0.147 migration {legacy_version} changed during version repair");
        }
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn repair_legacy_whale_taskspace_migrations(
    pool: &SqlitePool,
    migrator: &Migrator,
) -> anyhow::Result<()> {
    let migrations_table_exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await?
    .is_some();
    if !migrations_table_exists {
        return Ok(());
    }

    let applied = sqlx::query_as::<_, (i64, String)>(
        r#"
SELECT version, lower(hex(checksum))
FROM _sqlx_migrations
WHERE version IN (30, 31) AND success = 1
ORDER BY version
        "#,
    )
    .fetch_all(pool)
    .await?;
    let expected = LEGACY_WHALE_TASKSPACE_MIGRATIONS
        .iter()
        .map(|(version, checksum)| (*version, (*checksum).to_string()))
        .collect::<Vec<_>>();
    if applied != expected {
        return Ok(());
    }

    let taskspace_table_count = sqlx::query_scalar::<_, i64>(
        r#"
SELECT count(*)
FROM sqlite_master
WHERE type = 'table'
  AND name IN ('taskspace_maps', 'taskspace_map_bindings', 'taskspace_map_commits')
        "#,
    )
    .fetch_one(pool)
    .await?;
    let thread_source_column_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM pragma_table_info('threads') WHERE name = 'thread_source'",
    )
    .fetch_one(pool)
    .await?;
    if taskspace_table_count != 3 || thread_source_column_count != 0 {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    sqlx::query("ALTER TABLE threads ADD COLUMN thread_source TEXT")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS device_key_bindings")
        .execute(&mut *tx)
        .await?;

    for (version, legacy_checksum) in LEGACY_WHALE_TASKSPACE_MIGRATIONS {
        let current = migrator
            .migrations
            .iter()
            .find(|migration| migration.version == version)
            .ok_or_else(|| anyhow::anyhow!("current state migration {version} is missing"))?;
        let result = sqlx::query(
            r#"
UPDATE _sqlx_migrations
SET description = ?, checksum = ?
WHERE version = ? AND success = 1 AND lower(hex(checksum)) = ?
            "#,
        )
        .bind(current.description.as_ref())
        .bind(current.checksum.as_ref())
        .bind(version)
        .bind(legacy_checksum)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            anyhow::bail!("legacy Whale migration {version} changed during repair");
        }
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn repair_legacy_recency_migration_version(
    pool: &SqlitePool,
    migrator: &Migrator,
) -> anyhow::Result<()> {
    let Some(recency_migration) = migrator
        .migrations
        .iter()
        .find(|migration| migration.version == 39)
    else {
        return Ok(());
    };
    let migrations_table_exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await?
    .is_some();
    if !migrations_table_exists {
        return Ok(());
    }

    let legacy_recency_needs_repair = sqlx::query_scalar::<_, i64>(
        r#"
SELECT 1
FROM _sqlx_migrations
WHERE version = ?
  AND checksum = ?
  AND NOT EXISTS (
      SELECT 1 FROM _sqlx_migrations WHERE version = ?
  )
        "#,
    )
    .bind(38_i64)
    .bind(recency_migration.checksum.as_ref())
    .bind(recency_migration.version)
    .fetch_optional(pool)
    .await?
    .is_some();
    if !legacy_recency_needs_repair {
        return Ok(());
    }

    sqlx::query(
        r#"
UPDATE _sqlx_migrations
SET version = ?, description = ?
WHERE version = ?
  AND checksum = ?
  AND NOT EXISTS (
      SELECT 1 FROM _sqlx_migrations WHERE version = ?
  )
        "#,
    )
    .bind(recency_migration.version)
    .bind(recency_migration.description.as_ref())
    .bind(38_i64)
    .bind(recency_migration.checksum.as_ref())
    .bind(recency_migration.version)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
#[path = "migrations_tests.rs"]
mod tests;
