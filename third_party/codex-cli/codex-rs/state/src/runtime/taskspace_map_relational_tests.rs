use super::StateRuntime;
use super::test_support::unique_temp_dir;
use crate::CommitTaskSpaceMapRequest;
use crate::CreateTaskSpaceMapRequest;
use codex_protocol::ThreadId;
use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceActionOutcome;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_protocol::taskspace::TaskSpaceNodeAction;
use codex_protocol::taskspace::TaskSpaceNodeState;
use sqlx::Row;

async fn runtime() -> (std::path::PathBuf, std::sync::Arc<StateRuntime>) {
    let home = unique_temp_dir();
    let runtime = StateRuntime::init(home.clone(), "test-provider".to_string())
        .await
        .expect("state runtime");
    (home, runtime)
}

fn canonical_map(map_id: &str, revision: u64) -> TaskSpaceCanonicalMap {
    TaskSpaceCanonicalMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.to_string(),
        map_id: map_id.to_string(),
        root: TaskSpaceMapNode {
            node_id: "root".to_string(),
            goal: "deliver".to_string(),
            state: TaskSpaceNodeState::InFlight,
            content: String::new(),
            parents: Vec::new(),
            actions: Vec::new(),
        },
        work_nodes: vec![
            TaskSpaceMapNode {
                node_id: "implement".to_string(),
                goal: "implement".to_string(),
                state: TaskSpaceNodeState::InFlight,
                content: String::new(),
                parents: vec!["root".to_string()],
                actions: vec![TaskSpaceNodeAction {
                    action_id: "call-1".to_string(),
                    tool_name: "read_file".to_string(),
                    outcome: TaskSpaceActionOutcome::Pending,
                }],
            },
            TaskSpaceMapNode {
                node_id: "verify".to_string(),
                goal: "verify".to_string(),
                state: TaskSpaceNodeState::Waiting,
                content: String::new(),
                parents: vec!["implement".to_string()],
                actions: Vec::new(),
            },
        ],
        finish: TaskSpaceMapNode {
            node_id: "finish".to_string(),
            goal: "finish".to_string(),
            state: TaskSpaceNodeState::Waiting,
            content: String::new(),
            parents: vec!["verify".to_string()],
            actions: Vec::new(),
        },
        revision,
    }
}

#[tokio::test]
async fn relational_schema_has_no_canonical_json_or_parallel_map_store() {
    let (home, runtime) = runtime().await;
    let columns = sqlx::query("PRAGMA table_info(taskspace_maps)")
        .fetch_all(runtime.pool.as_ref())
        .await
        .expect("table info")
        .into_iter()
        .map(|row| row.try_get::<String, _>("name").expect("column name"))
        .collect::<Vec<_>>();
    assert!(columns.contains(&"schema_version".to_string()));
    assert!(columns.contains(&"map_revision".to_string()));
    assert!(!columns.contains(&"canonical_json".to_string()));

    let tables = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE 'taskspace_map%' ORDER BY name",
    )
    .fetch_all(runtime.pool.as_ref())
    .await
    .expect("table names");
    assert!(tables.contains(&"taskspace_map_nodes".to_string()));
    assert!(tables.contains(&"taskspace_map_node_parents".to_string()));
    assert!(tables.contains(&"taskspace_map_node_actions".to_string()));
    assert!(!tables.iter().any(|name| name.contains("event")));
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn action_outcome_commit_updates_only_the_matching_action_row() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    let mut initial = canonical_map(&map_id, 1);
    initial.work_nodes[0].actions.push(TaskSpaceNodeAction {
        action_id: "call-2".to_string(),
        tool_name: "exec_command".to_string(),
        outcome: TaskSpaceActionOutcome::Pending,
    });
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: Some(initial.clone()),
            commit_id: "create-relational".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");

    sqlx::query(
        "CREATE TABLE taskspace_write_audit (entity TEXT NOT NULL, operation TEXT NOT NULL, action_id TEXT)",
    )
    .execute(runtime.pool.as_ref())
    .await
    .expect("audit table");
    for (name, table, operation) in [
        ("audit_node_update", "taskspace_map_nodes", "UPDATE"),
        ("audit_node_insert", "taskspace_map_nodes", "INSERT"),
        ("audit_node_delete", "taskspace_map_nodes", "DELETE"),
        (
            "audit_parent_insert",
            "taskspace_map_node_parents",
            "INSERT",
        ),
        (
            "audit_parent_delete",
            "taskspace_map_node_parents",
            "DELETE",
        ),
        (
            "audit_action_insert",
            "taskspace_map_node_actions",
            "INSERT",
        ),
        (
            "audit_action_delete",
            "taskspace_map_node_actions",
            "DELETE",
        ),
        (
            "audit_action_update",
            "taskspace_map_node_actions",
            "UPDATE",
        ),
    ] {
        let entity = if table.ends_with("actions") {
            "action"
        } else if table.ends_with("parents") {
            "parent"
        } else {
            "node"
        };
        let action_id = if table.ends_with("actions") {
            if operation == "DELETE" {
                "OLD.action_id"
            } else {
                "NEW.action_id"
            }
        } else {
            "NULL"
        };
        let statement = format!(
            "CREATE TRIGGER {name} AFTER {operation} ON {table} BEGIN INSERT INTO taskspace_write_audit VALUES ('{entity}', '{operation}', {action_id}); END"
        );
        sqlx::query(&statement)
            .execute(runtime.pool.as_ref())
            .await
            .expect("audit trigger");
    }

    let mut candidate = initial;
    candidate.revision = 2;
    candidate.work_nodes[0].actions[0].outcome = TaskSpaceActionOutcome::Succeeded;
    runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: 1,
            canonical_map: Some(candidate),
            commit_id: "settle-call-1".to_string(),
            operation: "settle_action".to_string(),
            actor_thread_id: owner,
            binding: None,
            consumed_pending_action_ids: Vec::new(),
        })
        .await
        .expect("settle action");

    let events = sqlx::query(
        "SELECT entity, operation, action_id FROM taskspace_write_audit ORDER BY rowid",
    )
    .fetch_all(runtime.pool.as_ref())
    .await
    .expect("audit events")
    .into_iter()
    .map(|row| {
        (
            row.try_get::<String, _>("entity").expect("entity"),
            row.try_get::<String, _>("operation").expect("operation"),
            row.try_get::<Option<String>, _>("action_id")
                .expect("action_id"),
        )
    })
    .collect::<Vec<_>>();
    assert_eq!(
        events,
        vec![(
            "action".to_string(),
            "UPDATE".to_string(),
            Some("call-1".to_string()),
        )]
    );
    let loaded = runtime
        .load_taskspace_map(&map_id)
        .await
        .expect("load map")
        .expect("map");
    assert_eq!(
        loaded.canonical_map.expect("canonical").work_nodes[0].actions[0].outcome,
        TaskSpaceActionOutcome::Succeeded
    );
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn action_row_diff_handles_add_delete_and_reorder_without_reinserting_survivors() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    let mut initial = canonical_map(&map_id, 1);
    initial.work_nodes[0].actions.extend([
        TaskSpaceNodeAction {
            action_id: "call-2".to_string(),
            tool_name: "exec_command".to_string(),
            outcome: TaskSpaceActionOutcome::Pending,
        },
        TaskSpaceNodeAction {
            action_id: "call-3".to_string(),
            tool_name: "write_stdin".to_string(),
            outcome: TaskSpaceActionOutcome::Pending,
        },
    ]);
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: Some(initial.clone()),
            commit_id: "create-action-row-diff".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");

    sqlx::query(
        "CREATE TABLE taskspace_action_audit (operation TEXT NOT NULL, action_id TEXT NOT NULL)",
    )
    .execute(runtime.pool.as_ref())
    .await
    .expect("audit table");
    for (name, operation, action_id) in [
        ("audit_action_insert", "INSERT", "NEW.action_id"),
        ("audit_action_update", "UPDATE", "NEW.action_id"),
        ("audit_action_delete", "DELETE", "OLD.action_id"),
    ] {
        let statement = format!(
            "CREATE TRIGGER {name} AFTER {operation} ON taskspace_map_node_actions BEGIN INSERT INTO taskspace_action_audit VALUES ('{operation}', {action_id}); END"
        );
        sqlx::query(&statement)
            .execute(runtime.pool.as_ref())
            .await
            .expect("audit trigger");
    }

    let mut candidate = initial;
    candidate.revision = 2;
    candidate.work_nodes[0].actions = vec![
        candidate.work_nodes[0].actions[2].clone(),
        TaskSpaceNodeAction {
            action_id: "call-4".to_string(),
            tool_name: "apply_patch".to_string(),
            outcome: TaskSpaceActionOutcome::Succeeded,
        },
        candidate.work_nodes[0].actions[0].clone(),
    ];
    runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: 1,
            canonical_map: Some(candidate.clone()),
            commit_id: "reorder-action-rows".to_string(),
            operation: "update_actions".to_string(),
            actor_thread_id: owner,
            binding: None,
            consumed_pending_action_ids: Vec::new(),
        })
        .await
        .expect("update actions");

    let events =
        sqlx::query("SELECT operation, action_id FROM taskspace_action_audit ORDER BY rowid")
            .fetch_all(runtime.pool.as_ref())
            .await
            .expect("audit events")
            .into_iter()
            .map(|row| {
                (
                    row.try_get::<String, _>("operation").expect("operation"),
                    row.try_get::<String, _>("action_id").expect("action_id"),
                )
            })
            .collect::<Vec<_>>();
    assert_eq!(
        events
            .iter()
            .filter(|(operation, _)| operation == "DELETE")
            .map(|(_, action_id)| action_id)
            .collect::<Vec<_>>(),
        vec![&"call-2".to_string()]
    );
    assert_eq!(
        events
            .iter()
            .filter(|(operation, _)| operation == "INSERT")
            .map(|(_, action_id)| action_id)
            .collect::<Vec<_>>(),
        vec![&"call-4".to_string()]
    );

    let loaded = runtime
        .load_taskspace_map(&map_id)
        .await
        .expect("load map")
        .expect("map")
        .canonical_map
        .expect("canonical");
    assert_eq!(loaded, candidate);
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn node_row_diff_handles_reorder_without_unique_conflict_or_reinserting_survivors() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    let initial = canonical_map(&map_id, 1);
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: Some(initial.clone()),
            commit_id: "create-node-row-diff".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create map");

    sqlx::query(
        "CREATE TABLE taskspace_node_audit (operation TEXT NOT NULL, node_id TEXT NOT NULL)",
    )
    .execute(runtime.pool.as_ref())
    .await
    .expect("audit table");
    for (name, operation, node_id) in [
        ("audit_node_insert", "INSERT", "NEW.node_id"),
        ("audit_node_update", "UPDATE", "NEW.node_id"),
        ("audit_node_delete", "DELETE", "OLD.node_id"),
    ] {
        let statement = format!(
            "CREATE TRIGGER {name} AFTER {operation} ON taskspace_map_nodes BEGIN INSERT INTO taskspace_node_audit VALUES ('{operation}', {node_id}); END"
        );
        sqlx::query(&statement)
            .execute(runtime.pool.as_ref())
            .await
            .expect("audit trigger");
    }

    let mut candidate = initial;
    candidate.revision = 2;
    candidate.work_nodes.swap(0, 1);
    runtime
        .compare_and_swap_taskspace_map(CommitTaskSpaceMapRequest {
            map_id: map_id.clone(),
            expected_store_revision: 1,
            canonical_map: Some(candidate.clone()),
            commit_id: "reorder-node-rows".to_string(),
            operation: "update_nodes".to_string(),
            actor_thread_id: owner,
            binding: None,
            consumed_pending_action_ids: Vec::new(),
        })
        .await
        .expect("reorder nodes");

    let events = sqlx::query("SELECT operation, node_id FROM taskspace_node_audit ORDER BY rowid")
        .fetch_all(runtime.pool.as_ref())
        .await
        .expect("audit events")
        .into_iter()
        .map(|row| {
            (
                row.try_get::<String, _>("operation").expect("operation"),
                row.try_get::<String, _>("node_id").expect("node_id"),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        events.iter().all(|(operation, _)| operation == "UPDATE"),
        "retained nodes must not be deleted or reinserted: {events:?}"
    );
    assert!(events.iter().any(|(_, node_id)| node_id == "implement"));
    assert!(events.iter().any(|(_, node_id)| node_id == "verify"));

    let loaded = runtime
        .load_taskspace_map(&map_id)
        .await
        .expect("load map")
        .expect("map")
        .canonical_map
        .expect("canonical");
    assert_eq!(loaded, candidate);
    let _ = tokio::fs::remove_dir_all(home).await;
}

#[tokio::test]
async fn one_hosted_action_round_trips_under_multiple_nodes() {
    let (home, runtime) = runtime().await;
    let owner = ThreadId::new();
    let map_id = format!("map-{owner}");
    let mut canonical = canonical_map(&map_id, 1);
    canonical.work_nodes[0].actions[0] = TaskSpaceNodeAction {
        action_id: "hosted-1".to_string(),
        tool_name: "web_search".to_string(),
        outcome: TaskSpaceActionOutcome::Succeeded,
    };
    canonical.work_nodes[1].actions = canonical.work_nodes[0].actions.clone();
    runtime
        .create_taskspace_map(CreateTaskSpaceMapRequest {
            map_id: map_id.clone(),
            owner_thread_id: owner,
            canonical_map: Some(canonical.clone()),
            commit_id: "create-shared-action".to_string(),
            operation: "activate_taskspace".to_string(),
        })
        .await
        .expect("create shared action map");

    let loaded = runtime
        .load_taskspace_map(&map_id)
        .await
        .expect("load map")
        .expect("map")
        .canonical_map
        .expect("canonical");
    assert_eq!(loaded, canonical);
    assert_eq!(loaded.work_nodes[0].actions, loaded.work_nodes[1].actions);
    let _ = tokio::fs::remove_dir_all(home).await;
}
