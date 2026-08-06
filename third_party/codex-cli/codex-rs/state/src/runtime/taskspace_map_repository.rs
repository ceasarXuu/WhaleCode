use std::collections::BTreeMap;

use super::taskspace_map_codec::action_outcome_name;
use super::taskspace_map_codec::canonical_sha256;
use super::taskspace_map_codec::from_i64;
use super::taskspace_map_codec::node_state_name;
use super::taskspace_map_codec::parse_action_outcome;
use super::taskspace_map_codec::parse_node_state;
use super::taskspace_map_codec::parse_thread_id;
use super::taskspace_map_codec::to_i64;
use super::taskspace_map_codec::validate_map_identity;
use crate::TaskSpaceMapRecord;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use codex_protocol::taskspace::TaskSpaceMapNode;
use codex_protocol::taskspace::TaskSpaceNodeAction;
use sqlx::Row;
use sqlx::Sqlite;
use sqlx::Transaction;

#[derive(Clone)]
struct StoredNode {
    role: &'static str,
    position: usize,
    node: TaskSpaceMapNode,
}

pub(super) async fn load_map_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    map_id: &str,
) -> anyhow::Result<Option<TaskSpaceMapRecord>> {
    let Some(head) = sqlx::query(
        r#"
SELECT map_id, owner_thread_id, schema_version, map_revision,
       canonical_sha256, store_revision, created_at_ms, updated_at_ms
FROM taskspace_maps
WHERE map_id = ?
        "#,
    )
    .bind(map_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };

    let map_id: String = head.try_get("map_id")?;
    let owner_thread_id = parse_thread_id(head.try_get("owner_thread_id")?, "owner_thread_id")?;
    let schema_version: Option<String> = head.try_get("schema_version")?;
    let map_revision: Option<i64> = head.try_get("map_revision")?;
    let canonical_map = match (schema_version, map_revision) {
        (None, None) => None,
        (Some(schema_version), Some(map_revision)) => Some(
            load_canonical_map(
                tx,
                &map_id,
                schema_version,
                from_i64(map_revision, "map_revision")?,
            )
            .await?,
        ),
        _ => anyhow::bail!("TaskSpace map `{map_id}` has an incomplete relational head"),
    };
    validate_map_identity(&map_id, canonical_map.as_ref())?;
    let expected_sha256: String = head.try_get("canonical_sha256")?;
    let actual_sha256 = canonical_sha256(&canonical_map)?;
    if actual_sha256 != expected_sha256 {
        anyhow::bail!("TaskSpace map `{map_id}` canonical hash mismatch");
    }
    Ok(Some(TaskSpaceMapRecord {
        map_id,
        owner_thread_id,
        canonical_map,
        canonical_sha256: expected_sha256,
        store_revision: from_i64(head.try_get("store_revision")?, "store_revision")?,
        created_at_ms: head.try_get("created_at_ms")?,
        updated_at_ms: head.try_get("updated_at_ms")?,
    }))
}

async fn load_canonical_map(
    tx: &mut Transaction<'_, Sqlite>,
    map_id: &str,
    schema_version: String,
    revision: u64,
) -> anyhow::Result<TaskSpaceCanonicalMap> {
    let parent_rows = sqlx::query(
        "SELECT node_id, parent_node_id FROM taskspace_map_node_parents WHERE map_id = ? ORDER BY node_id, position",
    )
    .bind(map_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut parents = BTreeMap::<String, Vec<String>>::new();
    for row in parent_rows {
        parents
            .entry(row.try_get("node_id")?)
            .or_default()
            .push(row.try_get("parent_node_id")?);
    }

    let action_rows = sqlx::query(
        "SELECT node_id, action_id, tool_name, outcome FROM taskspace_map_node_actions WHERE map_id = ? ORDER BY node_id, position",
    )
    .bind(map_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut actions = BTreeMap::<String, Vec<TaskSpaceNodeAction>>::new();
    for row in action_rows {
        let outcome: String = row.try_get("outcome")?;
        actions
            .entry(row.try_get("node_id")?)
            .or_default()
            .push(TaskSpaceNodeAction {
                action_id: row.try_get("action_id")?,
                tool_name: row.try_get("tool_name")?,
                outcome: parse_action_outcome(&outcome)?,
            });
    }

    let rows = sqlx::query(
        r#"
SELECT node_id, role, position, goal, state, content
FROM taskspace_map_nodes
WHERE map_id = ?
ORDER BY CASE role WHEN 'root' THEN 0 WHEN 'work' THEN 1 ELSE 2 END, position
        "#,
    )
    .bind(map_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut root = None;
    let mut work_nodes = Vec::new();
    let mut finish = None;
    for row in rows {
        let node_id: String = row.try_get("node_id")?;
        let role: String = row.try_get("role")?;
        let position = from_i64(row.try_get("position")?, "node position")? as usize;
        let state: String = row.try_get("state")?;
        let node = TaskSpaceMapNode {
            node_id: node_id.clone(),
            goal: row.try_get("goal")?,
            state: parse_node_state(&state)?,
            content: row.try_get("content")?,
            parents: parents.remove(&node_id).unwrap_or_default(),
            actions: actions.remove(&node_id).unwrap_or_default(),
        };
        match role.as_str() {
            "root" if position == 0 && root.is_none() => root = Some(node),
            "work" if position == work_nodes.len() => work_nodes.push(node),
            "finish" if position == 0 && finish.is_none() => finish = Some(node),
            _ => anyhow::bail!("TaskSpace map `{map_id}` has invalid node role ordering"),
        }
    }
    if !parents.is_empty() || !actions.is_empty() {
        anyhow::bail!("TaskSpace map `{map_id}` contains orphan node details");
    }
    Ok(TaskSpaceCanonicalMap {
        schema_version,
        map_id: map_id.to_string(),
        root: root.ok_or_else(|| anyhow::anyhow!("TaskSpace map `{map_id}` has no root node"))?,
        work_nodes,
        finish: finish
            .ok_or_else(|| anyhow::anyhow!("TaskSpace map `{map_id}` has no finish node"))?,
        revision,
    })
}

pub(super) async fn insert_map_head(
    tx: &mut Transaction<'_, Sqlite>,
    record: &TaskSpaceMapRecord,
) -> anyhow::Result<bool> {
    let (schema_version, map_revision) = canonical_head(record.canonical_map.as_ref())?;
    let inserted = sqlx::query(
        r#"
INSERT INTO taskspace_maps (
    map_id, owner_thread_id, schema_version, map_revision, canonical_sha256,
    store_revision, created_at_ms, updated_at_ms
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(map_id) DO NOTHING
        "#,
    )
    .bind(&record.map_id)
    .bind(record.owner_thread_id.to_string())
    .bind(schema_version)
    .bind(map_revision)
    .bind(&record.canonical_sha256)
    .bind(to_i64(record.store_revision, "store_revision")?)
    .bind(record.created_at_ms)
    .bind(record.updated_at_ms)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if inserted == 1 {
        sync_map_rows(tx, &record.map_id, None, record.canonical_map.as_ref()).await?;
    }
    Ok(inserted == 1)
}

pub(super) async fn compare_and_swap_map(
    tx: &mut Transaction<'_, Sqlite>,
    current: &TaskSpaceMapRecord,
    candidate: &Option<TaskSpaceCanonicalMap>,
    next_store_revision: u64,
    canonical_sha256: &str,
    now: i64,
) -> anyhow::Result<bool> {
    let (schema_version, map_revision) = canonical_head(candidate.as_ref())?;
    let updated = sqlx::query(
        r#"
UPDATE taskspace_maps
SET schema_version = ?, map_revision = ?, canonical_sha256 = ?,
    store_revision = ?, updated_at_ms = ?
WHERE map_id = ? AND store_revision = ?
        "#,
    )
    .bind(schema_version)
    .bind(map_revision)
    .bind(canonical_sha256)
    .bind(to_i64(next_store_revision, "store_revision")?)
    .bind(now)
    .bind(&current.map_id)
    .bind(to_i64(current.store_revision, "expected_store_revision")?)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if updated == 1 {
        sync_map_rows(
            tx,
            &current.map_id,
            current.canonical_map.as_ref(),
            candidate.as_ref(),
        )
        .await?;
    }
    Ok(updated == 1)
}

fn canonical_head(
    map: Option<&TaskSpaceCanonicalMap>,
) -> anyhow::Result<(Option<&str>, Option<i64>)> {
    map.map_or(Ok((None, None)), |map| {
        Ok((
            Some(map.schema_version.as_str()),
            Some(to_i64(map.revision, "map_revision")?),
        ))
    })
}

async fn sync_map_rows(
    tx: &mut Transaction<'_, Sqlite>,
    map_id: &str,
    current: Option<&TaskSpaceCanonicalMap>,
    candidate: Option<&TaskSpaceCanonicalMap>,
) -> anyhow::Result<()> {
    let current = flatten_nodes(current);
    let candidate = flatten_nodes(candidate);

    for node_id in current
        .keys()
        .filter(|node_id| !candidate.contains_key(*node_id))
    {
        sqlx::query("DELETE FROM taskspace_map_nodes WHERE map_id = ? AND node_id = ?")
            .bind(map_id)
            .bind(node_id)
            .execute(&mut **tx)
            .await?;
    }
    for stored in candidate.values() {
        let old = current.get(&stored.node.node_id);
        if old.is_none_or(|old| !same_node_fields(old, stored)) {
            sqlx::query(
                r#"
INSERT INTO taskspace_map_nodes (map_id, node_id, role, position, goal, state, content)
VALUES (?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(map_id, node_id) DO UPDATE SET
    role = excluded.role, position = excluded.position, goal = excluded.goal,
    state = excluded.state, content = excluded.content
                "#,
            )
            .bind(map_id)
            .bind(&stored.node.node_id)
            .bind(stored.role)
            .bind(i64::try_from(stored.position)?)
            .bind(&stored.node.goal)
            .bind(node_state_name(stored.node.state))
            .bind(&stored.node.content)
            .execute(&mut **tx)
            .await?;
        }
    }

    for stored in candidate.values() {
        let old = current.get(&stored.node.node_id);
        if old.is_none_or(|old| old.node.parents != stored.node.parents) {
            replace_parents(tx, map_id, &stored.node).await?;
        }
    }
    let changed_actions = candidate
        .values()
        .filter(|stored| {
            current
                .get(&stored.node.node_id)
                .is_none_or(|old| old.node.actions != stored.node.actions)
        })
        .collect::<Vec<_>>();
    for stored in &changed_actions {
        sqlx::query("DELETE FROM taskspace_map_node_actions WHERE map_id = ? AND node_id = ?")
            .bind(map_id)
            .bind(&stored.node.node_id)
            .execute(&mut **tx)
            .await?;
    }
    for stored in changed_actions {
        insert_actions(tx, map_id, &stored.node).await?;
    }
    Ok(())
}

fn flatten_nodes(map: Option<&TaskSpaceCanonicalMap>) -> BTreeMap<String, StoredNode> {
    let Some(map) = map else {
        return BTreeMap::new();
    };
    let mut nodes = BTreeMap::new();
    for (role, position, node) in std::iter::once(("root", 0, &map.root))
        .chain(
            map.work_nodes
                .iter()
                .enumerate()
                .map(|(i, node)| ("work", i, node)),
        )
        .chain(std::iter::once(("finish", 0, &map.finish)))
    {
        nodes.insert(
            node.node_id.clone(),
            StoredNode {
                role,
                position,
                node: node.clone(),
            },
        );
    }
    nodes
}

fn same_node_fields(left: &StoredNode, right: &StoredNode) -> bool {
    left.role == right.role
        && left.position == right.position
        && left.node.goal == right.node.goal
        && left.node.state == right.node.state
        && left.node.content == right.node.content
}

async fn replace_parents(
    tx: &mut Transaction<'_, Sqlite>,
    map_id: &str,
    node: &TaskSpaceMapNode,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM taskspace_map_node_parents WHERE map_id = ? AND node_id = ?")
        .bind(map_id)
        .bind(&node.node_id)
        .execute(&mut **tx)
        .await?;
    for (position, parent) in node.parents.iter().enumerate() {
        sqlx::query(
            "INSERT INTO taskspace_map_node_parents (map_id, node_id, parent_node_id, position) VALUES (?, ?, ?, ?)",
        )
        .bind(map_id)
        .bind(&node.node_id)
        .bind(parent)
        .bind(i64::try_from(position)?)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn insert_actions(
    tx: &mut Transaction<'_, Sqlite>,
    map_id: &str,
    node: &TaskSpaceMapNode,
) -> anyhow::Result<()> {
    for (position, action) in node.actions.iter().enumerate() {
        sqlx::query(
            "INSERT INTO taskspace_map_node_actions (map_id, node_id, action_id, position, tool_name, outcome) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(map_id)
        .bind(&node.node_id)
        .bind(&action.action_id)
        .bind(i64::try_from(position)?)
        .bind(&action.tool_name)
        .bind(action_outcome_name(action.outcome))
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
