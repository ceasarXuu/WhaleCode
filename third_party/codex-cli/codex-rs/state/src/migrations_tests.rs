use codex_utils_absolute_path::test_support::PathExt;
use pretty_assertions::assert_eq;
use sqlx::Connection;
use sqlx::Row;
use sqlx::SqlSafeStr;
use sqlx::migrate::MigrateError;
use sqlx::migrate::Migration;
use sqlx::migrate::MigrationType;
use sqlx::migrate::Migrator;
use std::borrow::Cow;

use super::STATE_MIGRATOR;
use super::THREAD_HISTORY_MIGRATOR;
use super::repair_legacy_recency_migration_version;
use super::repair_legacy_whale_taskspace_migrations;
use super::repair_whale_0147_taskspace_migration_versions;
use crate::PINNED_THREAD_SECTION_ID;
use crate::PINNED_THREAD_SECTION_NAME;

const CUSTOM_THREAD_SECTION_ID: &str = "01984de2-8f74-7c91-a3b2-5c5e937cf317";

fn migrator_through(version: i64) -> Migrator {
    Migrator {
        migrations: Cow::Owned(
            STATE_MIGRATOR
                .migrations
                .iter()
                .filter(|migration| migration.version <= version)
                .cloned()
                .collect(),
        ),
        ignore_missing: STATE_MIGRATOR.ignore_missing,
        locking: STATE_MIGRATOR.locking,
        table_name: STATE_MIGRATOR.table_name.clone(),
        create_schemas: STATE_MIGRATOR.create_schemas.clone(),
        no_tx: STATE_MIGRATOR.no_tx,
    }
}

fn legacy_whale_taskspace_migrator() -> Migrator {
    let mut migrations = STATE_MIGRATOR
        .migrations
        .iter()
        .filter(|migration| migration.version <= 29)
        .cloned()
        .collect::<Vec<_>>();
    migrations.push(Migration::new(
        30,
        Cow::Borrowed("taskspace maps"),
        MigrationType::Simple,
        include_str!("fixtures/legacy_whale_0030_taskspace_maps.sql").into_sql_str(),
        false,
    ));
    migrations.push(Migration::new(
        31,
        Cow::Borrowed("taskspace canonical maps"),
        MigrationType::Simple,
        include_str!("fixtures/legacy_whale_0031_taskspace_canonical_maps.sql").into_sql_str(),
        false,
    ));
    Migrator::with_migrations(migrations)
}

fn whale_0147_taskspace_migrator() -> Migrator {
    let mut migrations = STATE_MIGRATOR
        .migrations
        .iter()
        .filter(|migration| migration.version <= 46)
        .cloned()
        .collect::<Vec<_>>();
    for (version, description, sql) in [
        (
            47,
            "taskspace canonical store",
            include_str!("../migrations/0051_taskspace_canonical_store.sql"),
        ),
        (
            48,
            "taskspace relational store",
            include_str!("../migrations/0052_taskspace_relational_store.sql"),
        ),
    ] {
        migrations.push(Migration::new(
            version,
            Cow::Borrowed(description),
            MigrationType::Simple,
            sql.into_sql_str(),
            false,
        ));
    }
    Migrator::with_migrations(migrations)
}

async fn open_legacy_whale_taskspace_pool() -> (
    scopeguard::ScopeGuard<std::path::PathBuf, impl FnOnce(std::path::PathBuf)>,
    crate::SqliteConfig,
    sqlx::SqlitePool,
) {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("legacy Whale state database should open");
    legacy_whale_taskspace_migrator()
        .run(&pool)
        .await
        .expect("legacy Whale migrations should apply");
    (cleanup, sqlite, pool)
}

#[tokio::test]
async fn repairs_legacy_whale_collision_then_archives_v2_data_for_r8() {
    let (_cleanup, sqlite, pool) = open_legacy_whale_taskspace_pool().await;
    let canonical_json = r#"{"schema_version":"taskspace-canonical-map-v2","map_id":"legacy-map"}"#;
    sqlx::query(
        r#"
INSERT INTO taskspace_maps (
    map_id, owner_thread_id, canonical_schema_version, canonical_json,
    canonical_sha256, store_revision, map_revision, terminal,
    created_at_ms, updated_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind("legacy-map")
    .bind("legacy-owner")
    .bind("taskspace-canonical-map-v2")
    .bind(canonical_json)
    .bind("legacy-sha")
    .bind(1_i64)
    .bind(0_i64)
    .bind(0_i64)
    .bind(1_i64)
    .bind(1_i64)
    .execute(&pool)
    .await
    .expect("legacy canonical map should insert");
    pool.close().await;

    let runtime = crate::StateRuntime::init(sqlite.clone(), "deepseek".to_string())
        .await
        .expect("current runtime should repair and migrate the legacy database");
    assert!(
        runtime
            .load_taskspace_map("legacy-map")
            .await
            .expect("R8 store should remain readable")
            .is_none()
    );

    let verification_pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("migrated database should open");
    let preserved = sqlx::query_scalar::<_, String>(
        "SELECT canonical_json FROM taskspace_v2_maps WHERE map_id = 'legacy-map'",
    )
    .fetch_one(&verification_pool)
    .await
    .expect("legacy canonical JSON should be archived losslessly");
    assert_eq!(preserved, canonical_json);
    let applied = sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT version, checksum FROM _sqlx_migrations WHERE version IN (30, 31) ORDER BY version",
    )
    .fetch_all(&verification_pool)
    .await
    .expect("migration metadata should be readable");
    let expected = STATE_MIGRATOR
        .migrations
        .iter()
        .filter(|migration| matches!(migration.version, 30 | 31))
        .map(|migration| (migration.version, migration.checksum.to_vec()))
        .collect::<Vec<_>>();
    assert_eq!(applied, expected);
    verification_pool.close().await;
    runtime.close().await;
}

#[tokio::test]
async fn legacy_whale_taskspace_repair_is_noop_for_partial_or_unknown_history() {
    for mutation in ["missing-31", "unknown-31", "partial-schema"] {
        let (_cleanup, _sqlite, pool) = open_legacy_whale_taskspace_pool().await;
        match mutation {
            "missing-31" => {
                sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 31")
                    .execute(&pool)
                    .await
                    .expect("migration row should be removed");
            }
            "unknown-31" => {
                sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = 31")
                    .execute(&pool)
                    .await
                    .expect("migration checksum should be changed");
            }
            "partial-schema" => {
                sqlx::query("ALTER TABLE threads ADD COLUMN thread_source TEXT")
                    .execute(&pool)
                    .await
                    .expect("partial schema should be constructed");
            }
            _ => unreachable!(),
        }

        repair_legacy_whale_taskspace_migrations(&pool, &STATE_MIGRATOR)
            .await
            .expect("unsupported history should remain untouched");
        let version_30_checksum = sqlx::query_scalar::<_, String>(
            "SELECT lower(hex(checksum)) FROM _sqlx_migrations WHERE version = 30",
        )
        .fetch_one(&pool)
        .await
        .expect("legacy version 30 should remain present");
        assert_eq!(
            version_30_checksum,
            "e8b7fd4f72e583f648a6eac076ae000e63b3ffc7fca86e64321b7a60293beb9c9441d8cab044fd92a298d8cb90c387eb"
        );
        let err = STATE_MIGRATOR
            .run(&pool)
            .await
            .expect_err("SQLx should reject unsupported migration history");
        assert!(
            matches!(
                err,
                MigrateError::VersionMismatch(30)
                    | MigrateError::VersionMismatch(31)
                    | MigrateError::VersionMissing(31)
            ),
            "unexpected error for {mutation}: {err}"
        );
        pool.close().await;
    }
}

#[tokio::test]
async fn legacy_whale_taskspace_repair_is_noop_for_fresh_and_current_databases() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("fresh state database should open");

    repair_legacy_whale_taskspace_migrations(&pool, &STATE_MIGRATOR)
        .await
        .expect("fresh database repair should be a no-op");
    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("current migrations should apply");
    let before = sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT version, checksum FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(&pool)
    .await
    .expect("current migration metadata should be readable");

    repair_legacy_whale_taskspace_migrations(&pool, &STATE_MIGRATOR)
        .await
        .expect("current database repair should be a no-op");
    let after = sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT version, checksum FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(&pool)
    .await
    .expect("current migration metadata should remain readable");
    assert_eq!(after, before);
    pool.close().await;
}

#[tokio::test]
async fn repairs_whale_0147_taskspace_versions_before_applying_upstream_0149() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("state database should open");
    whale_0147_taskspace_migrator()
        .run(&pool)
        .await
        .expect("Whale 0.147 migrations should apply");
    pool.close().await;

    let runtime = crate::StateRuntime::init(sqlite.clone(), "deepseek".to_string())
        .await
        .expect("0.149 runtime should repair and migrate a Whale 0.147 database");
    let verification_pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("migrated database should open");
    let applied = sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT version, checksum FROM _sqlx_migrations WHERE version BETWEEN 47 AND 52 ORDER BY version",
    )
    .fetch_all(&verification_pool)
    .await
    .expect("migration metadata should be readable");
    let expected = STATE_MIGRATOR
        .migrations
        .iter()
        .filter(|migration| (47..=52).contains(&migration.version))
        .map(|migration| (migration.version, migration.checksum.to_vec()))
        .collect::<Vec<_>>();
    assert_eq!(applied, expected);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('rollout_migration_state', 'projects', 'taskspace_map_nodes')",
        )
        .fetch_one(&verification_pool)
        .await
        .expect("0.149 and TaskSpace tables should be queryable"),
        3
    );
    verification_pool.close().await;
    runtime.close().await;
}

#[tokio::test]
async fn whale_0147_taskspace_version_repair_rejects_unknown_history() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("state database should open");
    whale_0147_taskspace_migrator()
        .run(&pool)
        .await
        .expect("Whale 0.147 migrations should apply");
    sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = 48")
        .execute(&pool)
        .await
        .expect("unknown history should be constructed");

    repair_whale_0147_taskspace_migration_versions(&pool, &STATE_MIGRATOR)
        .await
        .expect("unsupported history should remain untouched");
    let err = STATE_MIGRATOR
        .run(&pool)
        .await
        .expect_err("SQLx should reject unknown 0.147 history");
    assert!(matches!(err, MigrateError::VersionMismatch(47)));
    pool.close().await;
}

#[tokio::test]
async fn relational_taskspace_migration_archives_v2_rows_without_activating_them() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("state database should open");
    migrator_through(51)
        .run(&pool)
        .await
        .expect("v2 migrations should apply");

    sqlx::query("INSERT INTO taskspace_maps VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind("legacy-map")
        .bind("legacy-owner")
        .bind("taskspace.v2")
        .bind(r#"{"schema_version":"taskspace.v2"}"#)
        .bind("legacy-sha")
        .bind(2_i64)
        .bind(4_i64)
        .bind(0_i64)
        .bind(10_i64)
        .bind(20_i64)
        .execute(&pool)
        .await
        .expect("legacy map should insert");
    sqlx::query("INSERT INTO taskspace_map_bindings VALUES (?, ?, ?, ?, ?, ?)")
        .bind("legacy-thread")
        .bind("legacy-map")
        .bind("owner")
        .bind(Option::<String>::None)
        .bind(10_i64)
        .bind(20_i64)
        .execute(&pool)
        .await
        .expect("legacy binding should insert");
    sqlx::query("INSERT INTO taskspace_map_commits VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind("legacy-commit")
        .bind("legacy-map")
        .bind(1_i64)
        .bind(2_i64)
        .bind("legacy-sha")
        .bind("legacy-request")
        .bind("update")
        .bind("legacy-thread")
        .bind(20_i64)
        .execute(&pool)
        .await
        .expect("legacy commit should insert");

    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("relational migration should apply");

    let archived = sqlx::query_as::<_, (String, String)>(
        "SELECT map_id, canonical_json FROM taskspace_v2_maps",
    )
    .fetch_one(&pool)
    .await
    .expect("archived map should remain readable");
    assert_eq!(archived.0, "legacy-map");
    assert_eq!(archived.1, r#"{"schema_version":"taskspace.v2"}"#);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM taskspace_v2_map_bindings")
            .fetch_one(&pool)
            .await
            .expect("archived bindings should remain readable"),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM taskspace_v2_map_commits")
            .fetch_one(&pool)
            .await
            .expect("archived commits should remain readable"),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM taskspace_maps")
            .fetch_one(&pool)
            .await
            .expect("new relational store should be readable"),
        0
    );
}

#[tokio::test]
async fn thread_section_migration_preserves_legacy_pin_compatibility() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let state_path = sqlite.state_db_path();
    let pool = sqlite
        .open_read_write_pool(&state_path)
        .await
        .expect("sqlite database should open");
    migrator_through(/*version*/ 44)
        .run(&pool)
        .await
        .expect("released thread migrations should apply");

    for thread_id in [
        "00000000-0000-0000-0000-000000000043",
        "00000000-0000-0000-0000-000000000044",
    ] {
        if thread_id.ends_with("44") {
            sqlx::query("UPDATE threads SET is_pinned = 1 WHERE id = ?")
                .bind("00000000-0000-0000-0000-000000000043")
                .execute(&pool)
                .await
                .expect("legacy pin should remain writable before section migration");
            STATE_MIGRATOR
                .run(&pool)
                .await
                .expect("section migration should apply");
        }
        sqlx::query(
            r#"
INSERT INTO threads (
    id,
    rollout_path,
    created_at,
    updated_at,
    created_at_ms,
    updated_at_ms,
    source,
    model_provider,
    cwd,
    title,
    sandbox_policy,
    approval_mode
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(thread_id)
        .bind("/tmp/legacy.jsonl")
        .bind(1_700_000_000_i64)
        .bind(1_700_000_000_i64)
        .bind(1_700_000_000_000_i64)
        .bind(1_700_000_000_000_i64)
        .bind("cli")
        .bind("openai")
        .bind("/tmp")
        .bind("")
        .bind("read-only")
        .bind("on-request")
        .execute(&pool)
        .await
        .expect("legacy thread insert should succeed");
    }

    let registered_sections = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT id, name, appearance FROM thread_sections ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .expect("independent thread sections should load");
    assert_eq!(
        registered_sections,
        vec![(
            PINNED_THREAD_SECTION_ID.to_string(),
            PINNED_THREAD_SECTION_NAME.to_string(),
            None,
        )]
    );

    let threads = sqlx::query_as::<_, (i64, Option<String>)>(
        "SELECT is_pinned, thread_section_id FROM threads ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .expect("legacy and section-aware thread metadata should load");
    assert_eq!(threads, vec![(1, None), (0, None)]);

    sqlx::query("INSERT INTO thread_sections (id, name) VALUES (?, ?)")
        .bind(CUSTOM_THREAD_SECTION_ID)
        .bind("Custom section")
        .execute(&pool)
        .await
        .expect("custom sections should have independent persisted identities");

    let thread_id = "00000000-0000-0000-0000-000000000043";
    sqlx::query("UPDATE threads SET thread_section_id = ? WHERE id = ?")
        .bind(CUSTOM_THREAD_SECTION_ID)
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("threads should reference independently persisted sections");
    sqlx::query("UPDATE threads SET is_pinned = 0 WHERE id = ?")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("released binaries should still update the legacy pin column");
    let thread = sqlx::query_as::<_, (i64, Option<String>)>(
        "SELECT is_pinned, thread_section_id FROM threads WHERE id = ?",
    )
    .bind(thread_id)
    .fetch_one(&pool)
    .await
    .expect("legacy pin updates should not overwrite the authoritative section");
    assert_eq!(thread, (0, Some(CUSTOM_THREAD_SECTION_ID.to_string())));

    sqlx::query("UPDATE threads SET thread_section_id = NULL WHERE id = ?")
        .bind(thread_id)
        .execute(&pool)
        .await
        .expect("threads should be removable from sections");

    let registered_sections =
        sqlx::query_as::<_, (String, String)>("SELECT id, name FROM thread_sections ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("empty sections should remain independently discoverable");
    assert_eq!(
        registered_sections,
        vec![
            (
                CUSTOM_THREAD_SECTION_ID.to_string(),
                "Custom section".to_string(),
            ),
            (
                PINNED_THREAD_SECTION_ID.to_string(),
                PINNED_THREAD_SECTION_NAME.to_string(),
            ),
        ]
    );

    let mut released_pin_migrator = migrator_through(/*version*/ 44);
    released_pin_migrator.ignore_missing = true;
    released_pin_migrator
        .run(&pool)
        .await
        .expect("released pin-capable binaries should tolerate newer migrations");

    pool.close().await;
}

#[tokio::test]
async fn thread_section_order_migration_backfills_stably() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("sqlite database should open");
    migrator_through(/*version*/ 45)
        .run(&pool)
        .await
        .expect("pre-ordering migrations should apply");

    sqlx::query("INSERT INTO thread_sections (id, name) VALUES (?, ?)")
        .bind(CUSTOM_THREAD_SECTION_ID)
        .bind("Custom section")
        .execute(&pool)
        .await
        .expect("custom section should exist before threads reference it");

    let older = "00000000-0000-0000-0000-000000000071";
    let newer = "00000000-0000-0000-0000-000000000072";
    let pinned = "00000000-0000-0000-0000-000000000073";
    let unsectioned = "00000000-0000-0000-0000-000000000074";
    for (thread_id, recency_at_ms, section) in [
        (older, 1_700_000_001_000_i64, Some(CUSTOM_THREAD_SECTION_ID)),
        (newer, 1_700_000_002_000, Some(CUSTOM_THREAD_SECTION_ID)),
        (pinned, 1_700_000_003_000, Some(PINNED_THREAD_SECTION_ID)),
        (unsectioned, 1_700_000_004_000, None),
    ] {
        sqlx::query(
            r#"
INSERT INTO threads (
    id, rollout_path, created_at, updated_at, recency_at,
    created_at_ms, updated_at_ms, recency_at_ms, source,
    model_provider, cwd, title, preview, sandbox_policy, approval_mode, thread_section_id
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(thread_id)
        .bind("/tmp/legacy.jsonl")
        .bind(recency_at_ms / 1000)
        .bind(recency_at_ms / 1000)
        .bind(recency_at_ms / 1000)
        .bind(recency_at_ms)
        .bind(recency_at_ms)
        .bind(recency_at_ms)
        .bind("cli")
        .bind("openai")
        .bind("/tmp")
        .bind("")
        .bind("preview")
        .bind("read-only")
        .bind("on-request")
        .bind(section)
        .execute(&pool)
        .await
        .expect("legacy section row should insert");
    }

    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("section ordering migration should apply");
    let custom_order = sqlx::query_scalar::<_, String>(
        "SELECT id FROM threads WHERE thread_section_id = ? ORDER BY section_position, id",
    )
    .bind(CUSTOM_THREAD_SECTION_ID)
    .fetch_all(&pool)
    .await
    .expect("backfilled custom order should load");
    assert_eq!(custom_order, vec![newer.to_string(), older.to_string()]);
    let positions =
        sqlx::query_scalar::<_, Option<i64>>("SELECT section_position FROM threads ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("section positions should load");
    assert_eq!(
        positions,
        vec![Some(2_000_000), Some(1_000_000), Some(1_000_000), None]
    );
    let entered = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT section_entered_at_ms FROM threads ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .expect("section entry timestamps should load");
    assert_eq!(
        entered,
        vec![
            Some(1_700_000_001_000),
            Some(1_700_000_002_000),
            Some(1_700_000_003_000),
            None,
        ]
    );

    let section_position_index = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'index' AND name = ?",
    )
    .bind("idx_threads_section_position")
    .fetch_optional(&pool)
    .await
    .expect("section position index should remain inspectable");
    assert_eq!(
        section_position_index,
        Some("idx_threads_section_position".to_string())
    );

    pool.close().await;
}

#[tokio::test]
async fn thread_item_update_ordinals_allow_older_writers() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pre_update_ordinal_migrator = Migrator {
        migrations: Cow::Owned(
            THREAD_HISTORY_MIGRATOR
                .migrations
                .iter()
                .filter(|migration| migration.version < 4)
                .cloned()
                .collect(),
        ),
        ignore_missing: THREAD_HISTORY_MIGRATOR.ignore_missing,
        locking: THREAD_HISTORY_MIGRATOR.locking,
        table_name: THREAD_HISTORY_MIGRATOR.table_name.clone(),
        create_schemas: THREAD_HISTORY_MIGRATOR.create_schemas.clone(),
        no_tx: THREAD_HISTORY_MIGRATOR.no_tx,
    };
    let pool = sqlite
        .open_thread_history_db(
            &pre_update_ordinal_migrator,
            /*telemetry_override*/ None,
        )
        .await
        .expect("pre-update-ordinal migrations should apply");
    sqlx::query(
        r#"
INSERT INTO thread_items (thread_id, turn_id, item_id, rollout_ordinal, created_at_ms, item_type, item_json) VALUES
    ('thread-1', 'turn-1', 'existing-item-1', 11, 1_100, 'userMessage', '{}'),
    ('thread-1', 'turn-1', 'existing-item-2', 12, 1_200, 'userMessage', '{}')
        "#,
    )
    .execute(&pool)
    .await
    .expect("pre-migration items should be inserted");
    THREAD_HISTORY_MIGRATOR
        .run(&pool)
        .await
        .expect("update-ordinal migration should apply");
    sqlx::query(
        r#"
INSERT INTO thread_items (thread_id, turn_id, item_id, rollout_ordinal, created_at_ms, item_type, item_json) VALUES
    ('thread-1', 'turn-1', 'old-writer-item-1', 13, 1_300, 'userMessage', '{}'),
    ('thread-1', 'turn-1', 'old-writer-item-2', 14, 1_400, 'userMessage', '{}')
        "#,
    )
    .execute(&pool)
    .await
    .expect("older writers should be able to append multiple items after migration");
    let ordinals = sqlx::query_as::<_, (i64, i64)>(
        "SELECT rollout_ordinal, updated_at_ordinal FROM thread_items WHERE thread_id = ? ORDER BY rollout_ordinal",
    )
    .bind("thread-1")
    .fetch_all(&pool)
    .await
    .expect("old-writer items should load");
    assert_eq!(ordinals, vec![(11, 11), (12, 12), (13, 0), (14, 0)]);

    pool.close().await;
}

#[tokio::test]
async fn agent_job_tables_are_dropped_when_upgrading() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let state_path = sqlite.state_db_path();
    let pool = sqlite
        .open_read_write_pool(&state_path)
        .await
        .expect("sqlite database should open");
    migrator_through(/*version*/ 15)
        .run(&pool)
        .await
        .expect("agent job migrations should apply");

    sqlx::query(
        r#"
INSERT INTO agent_jobs (
    id,
    name,
    status,
    instruction,
    input_headers_json,
    input_csv_path,
    output_csv_path,
    created_at,
    updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind("job-1")
    .bind("legacy job")
    .bind("running")
    .bind("process rows")
    .bind(r#"["path"]"#)
    .bind("/tmp/input.csv")
    .bind("/tmp/output.csv")
    .bind(1_700_000_000_i64)
    .bind(1_700_000_000_i64)
    .execute(&pool)
    .await
    .expect("legacy agent job should insert");
    sqlx::query(
        r#"
INSERT INTO agent_job_items (
    job_id,
    item_id,
    row_index,
    row_json,
    status,
    result_json,
    created_at,
    updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind("job-1")
    .bind("item-1")
    .bind(0_i64)
    .bind(r#"{"path":"secret.csv"}"#)
    .bind("completed")
    .bind(r#"{"result":"legacy"}"#)
    .bind(1_700_000_000_i64)
    .bind(1_700_000_000_i64)
    .execute(&pool)
    .await
    .expect("legacy agent job item should insert");

    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("current migrations should apply");

    let agent_job_tables = sqlx::query_scalar::<_, String>(
        r#"
SELECT name
FROM sqlite_master
WHERE type = 'table' AND name IN ('agent_jobs', 'agent_job_items')
ORDER BY name
        "#,
    )
    .fetch_all(&pool)
    .await
    .expect("remaining agent job tables should load");
    assert_eq!(agent_job_tables, Vec::<String>::new());

    pool.close().await;
}

#[tokio::test]
async fn recency_migration_backfills_and_seeds_old_binary_inserts() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let state_path = sqlite.state_db_path();
    let pool = sqlite
        .open_read_write_pool(&state_path)
        .await
        .expect("sqlite database should open");
    migrator_through(/*version*/ 37)
        .run(&pool)
        .await
        .expect("pre-recency migrations should apply");

    sqlx::query(
        r#"
INSERT INTO threads (
    id,
    rollout_path,
    created_at,
    updated_at,
    created_at_ms,
    updated_at_ms,
    source,
    model_provider,
    cwd,
    title,
    sandbox_policy,
    approval_mode
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind("00000000-0000-0000-0000-000000000001")
    .bind("/tmp/first.jsonl")
    .bind(1_700_000_000_i64)
    .bind(1_700_000_100_i64)
    .bind(1_700_000_000_123_i64)
    .bind(1_700_000_100_456_i64)
    .bind("cli")
    .bind("openai")
    .bind("/tmp")
    .bind("")
    .bind("read-only")
    .bind("on-request")
    .execute(&pool)
    .await
    .expect("legacy row should insert");

    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("recency migration should apply");

    let backfilled = sqlx::query(
        "SELECT updated_at, updated_at_ms, recency_at, recency_at_ms FROM threads WHERE id = ?",
    )
    .bind("00000000-0000-0000-0000-000000000001")
    .fetch_one(&pool)
    .await
    .expect("backfilled row should load");
    assert_eq!(backfilled.get::<i64, _>("recency_at"), 1_700_000_100);
    assert_eq!(backfilled.get::<i64, _>("recency_at_ms"), 1_700_000_100_456);

    sqlx::query(
        r#"
INSERT INTO threads (
    id,
    rollout_path,
    created_at,
    updated_at,
    created_at_ms,
    updated_at_ms,
    source,
    model_provider,
    cwd,
    title,
    sandbox_policy,
    approval_mode
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind("00000000-0000-0000-0000-000000000002")
    .bind("/tmp/second.jsonl")
    .bind(1_700_000_200_i64)
    .bind(1_700_000_300_i64)
    .bind(1_700_000_200_123_i64)
    .bind(1_700_000_300_456_i64)
    .bind("cli")
    .bind("openai")
    .bind("/tmp")
    .bind("")
    .bind("read-only")
    .bind("on-request")
    .execute(&pool)
    .await
    .expect("old-binary row should insert");

    let seeded = sqlx::query("SELECT recency_at, recency_at_ms FROM threads WHERE id = ?")
        .bind("00000000-0000-0000-0000-000000000002")
        .fetch_one(&pool)
        .await
        .expect("old-binary row should load");
    assert_eq!(seeded.get::<i64, _>("recency_at"), 1_700_000_300);
    assert_eq!(seeded.get::<i64, _>("recency_at_ms"), 1_700_000_300_456);

    pool.close().await;
}

#[tokio::test]
async fn repairs_recency_migration_that_was_applied_as_version_38() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let state_path = sqlite.state_db_path();
    let pool = sqlite
        .open_read_write_pool(&state_path)
        .await
        .expect("sqlite database should open");
    migrator_through(/*version*/ 37)
        .run(&pool)
        .await
        .expect("pre-recency migrations should apply");

    let recency_migration = STATE_MIGRATOR
        .migrations
        .iter()
        .find(|migration| migration.version == 39)
        .expect("recency migration should exist");
    let mut legacy_migrations = STATE_MIGRATOR
        .migrations
        .iter()
        .filter(|migration| migration.version <= 37)
        .cloned()
        .collect::<Vec<_>>();
    legacy_migrations.push(Migration::new(
        38,
        recency_migration.description.clone(),
        recency_migration.migration_type,
        recency_migration.sql.clone(),
        recency_migration.no_tx,
    ));
    let legacy_recency_migrator = Migrator::with_migrations(legacy_migrations);
    legacy_recency_migrator
        .run(&pool)
        .await
        .expect("legacy recency migration should apply as version 38");

    repair_legacy_recency_migration_version(&pool, &STATE_MIGRATOR)
        .await
        .expect("legacy migration history should be repaired");
    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("current migrations should apply after repair");

    let applied = sqlx::query(
        "SELECT version, checksum FROM _sqlx_migrations WHERE version >= 38 ORDER BY version",
    )
    .fetch_all(&pool)
    .await
    .expect("applied migrations should load")
    .into_iter()
    .map(|row| {
        (
            row.get::<i64, _>("version"),
            row.get::<Vec<u8>, _>("checksum"),
        )
    })
    .collect::<Vec<_>>();
    let expected = STATE_MIGRATOR
        .migrations
        .iter()
        .filter(|migration| migration.version >= 38)
        .map(|migration| (migration.version, migration.checksum.to_vec()))
        .collect::<Vec<_>>();
    assert_eq!(applied, expected);

    pool.close().await;
}

#[tokio::test]
async fn repair_recency_migration_succeeds_while_another_connection_holds_writer_slot() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let state_path = sqlite.state_db_path();
    let pool = sqlite
        .open_read_write_pool(&state_path)
        .await
        .expect("database should open");
    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("current migrations should apply");
    let read_pool = sqlite
        .open_read_only_pool(&state_path)
        .await
        .expect("read-only pool should open");
    let mut write_connection = pool.acquire().await.expect("write connection should open");
    let write_transaction = write_connection
        .begin_with("BEGIN IMMEDIATE")
        .await
        .expect("write transaction should acquire the writer slot");

    let repair_result = repair_legacy_recency_migration_version(&read_pool, &STATE_MIGRATOR).await;

    write_transaction
        .rollback()
        .await
        .expect("write transaction should roll back");
    drop(write_connection);
    read_pool.close().await;
    pool.close().await;
    repair_result.expect("current migration history should not need the writer slot");
}
