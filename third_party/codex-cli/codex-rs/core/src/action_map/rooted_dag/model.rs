use codex_protocol::taskspace::TASKSPACE_CANONICAL_SCHEMA_VERSION;
use codex_protocol::taskspace::TaskSpaceCanonicalMap;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) use codex_protocol::taskspace::TaskSpaceActionOutcome as ActionOutcome;
pub(crate) use codex_protocol::taskspace::TaskSpaceMapId as MapId;
pub(crate) use codex_protocol::taskspace::TaskSpaceMapNode as MapNode;
pub(crate) use codex_protocol::taskspace::TaskSpaceNodeAction as NodeAction;
pub(crate) use codex_protocol::taskspace::TaskSpaceNodeId as NodeId;
pub(crate) use codex_protocol::taskspace::TaskSpaceNodeState as NodeState;
pub(crate) use codex_protocol::taskspace::TaskSpaceNodeView as NodeView;
pub(crate) use codex_protocol::taskspace::TaskSpaceRevision as Revision;

pub(crate) type TaskSpaceMap = TaskSpaceCanonicalMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NodeRole {
    TaskRoot,
    Work,
    Finish,
}

impl NodeRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::TaskRoot => "task_root",
            Self::Work => "work",
            Self::Finish => "finish",
        }
    }
}

pub(crate) fn new_map(
    map_id: MapId,
    root: MapNode,
    work_nodes: Vec<MapNode>,
    finish: MapNode,
) -> TaskSpaceMap {
    let mut map = TaskSpaceMap {
        schema_version: TASKSPACE_CANONICAL_SCHEMA_VERSION.into(),
        map_id,
        root,
        work_nodes,
        finish,
        revision: 1,
    };
    canonicalize(&mut map);
    map
}

pub(crate) fn node<'a>(map: &'a TaskSpaceMap, node_id: &str) -> Option<&'a MapNode> {
    if map.root.node_id == node_id {
        return Some(&map.root);
    }
    if map.finish.node_id == node_id {
        return Some(&map.finish);
    }
    map.work_nodes
        .iter()
        .find(|candidate| candidate.node_id == node_id)
}

pub(crate) fn node_mut<'a>(map: &'a mut TaskSpaceMap, node_id: &str) -> Option<&'a mut MapNode> {
    if map.root.node_id == node_id {
        return Some(&mut map.root);
    }
    if map.finish.node_id == node_id {
        return Some(&mut map.finish);
    }
    map.work_nodes
        .iter_mut()
        .find(|candidate| candidate.node_id == node_id)
}

pub(crate) fn node_role(map: &TaskSpaceMap, node_id: &str) -> Option<NodeRole> {
    if map.root.node_id == node_id {
        return Some(NodeRole::TaskRoot);
    }
    if map.finish.node_id == node_id {
        return Some(NodeRole::Finish);
    }
    map.work_nodes
        .iter()
        .any(|candidate| candidate.node_id == node_id)
        .then_some(NodeRole::Work)
}

pub(crate) fn nodes(map: &TaskSpaceMap) -> impl Iterator<Item = (NodeRole, &MapNode)> {
    std::iter::once((NodeRole::TaskRoot, &map.root))
        .chain(map.work_nodes.iter().map(|node| (NodeRole::Work, node)))
        .chain(std::iter::once((NodeRole::Finish, &map.finish)))
}

pub(crate) fn node_ids(map: &TaskSpaceMap) -> BTreeSet<&str> {
    nodes(map).map(|(_, node)| node.node_id.as_str()).collect()
}

pub(crate) fn children_by_parent(map: &TaskSpaceMap) -> BTreeMap<NodeId, Vec<NodeId>> {
    let mut children = node_ids(map)
        .into_iter()
        .map(|node_id| (node_id.to_string(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (_, node) in nodes(map) {
        for parent in &node.parents {
            children
                .entry(parent.clone())
                .or_default()
                .push(node.node_id.clone());
        }
    }
    for node_children in children.values_mut() {
        node_children.sort();
        node_children.dedup();
    }
    children
}

pub(crate) fn canonicalize(map: &mut TaskSpaceMap) {
    map.work_nodes
        .sort_by(|left, right| left.node_id.cmp(&right.node_id));
    for node in std::iter::once(&mut map.root)
        .chain(map.work_nodes.iter_mut())
        .chain(std::iter::once(&mut map.finish))
    {
        node.parents.sort();
        node.actions.sort();
    }
}

pub(crate) fn state_sha256(map: &TaskSpaceMap) -> Result<String, serde_json::Error> {
    let mut canonical = map.clone();
    canonicalize(&mut canonical);
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(crate) fn is_complete(map: &TaskSpaceMap) -> bool {
    map.root.state == NodeState::Completed && map.finish.state == NodeState::Completed
}

pub(crate) fn map_node(
    node_id: impl Into<String>,
    goal: impl Into<String>,
    state: NodeState,
    content: impl Into<String>,
    parents: Vec<String>,
) -> MapNode {
    MapNode {
        node_id: node_id.into(),
        goal: goal.into(),
        state,
        content: content.into(),
        parents,
        actions: Vec::new(),
    }
}
