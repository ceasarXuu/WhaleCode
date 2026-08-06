use codex_protocol::taskspace::TaskSpaceNodeView;
use serde::Serialize;

use super::rooted_dag::TaskSpaceMap;
use super::rooted_dag::derive_node_views;
use super::rooted_dag::is_complete;
use super::rooted_dag::state_sha256;

pub(crate) const TASKSPACE_MAP_HANDLE_MARKER: &str = "TaskSpaceMapHandleR8V1:";
pub(crate) const TASKSPACE_MAP_PROJECTION_END: &str = "TaskSpaceMapProjectionR8V1 end.";
pub(crate) const TASKSPACE_MAP_PROJECTION_MARKER: &str = "TaskSpaceMapProjectionR8V1:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionEnvelope {
    CurrentProjection,
    RequestSnapshot,
}

impl ProjectionEnvelope {
    pub(crate) fn kind(self) -> &'static str {
        match self {
            Self::CurrentProjection => "current_projection",
            Self::RequestSnapshot => "request_snapshot",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct TaskSpaceMapView {
    pub(crate) map_id: String,
    pub(crate) revision: u64,
    pub(crate) canonical_sha256: String,
    pub(crate) root_node_id: String,
    pub(crate) finish_node_id: String,
    pub(crate) complete: bool,
    pub(crate) nodes: Vec<TaskSpaceNodeView>,
}

pub(super) struct RenderedProjection {
    pub(super) body: String,
}

pub(super) fn render_projection(
    input: TaskSpaceMapView,
    envelope: ProjectionEnvelope,
) -> RenderedProjection {
    let map_json = serde_json::to_string_pretty(&input)
        .expect("TaskSpace projection contains only serializable canonical facts");
    let body = format!(
        "{TASKSPACE_MAP_PROJECTION_MARKER}\n- schema_version: taskspace-map-projection-r8-v1\n- projection_kind: {}\n- map_id: {}\n- revision: {}\n- canonical_sha256: {}\n- map:\n{}\n{TASKSPACE_MAP_PROJECTION_END}\n",
        envelope.kind(),
        input.map_id,
        input.revision,
        input.canonical_sha256,
        indent(&map_json, 2),
    );
    RenderedProjection { body }
}

pub(crate) fn taskspace_map_view(
    map: &TaskSpaceMap,
) -> Result<TaskSpaceMapView, serde_json::Error> {
    Ok(TaskSpaceMapView {
        map_id: map.map_id.clone(),
        revision: map.revision,
        canonical_sha256: state_sha256(map)?,
        root_node_id: map.root.node_id.clone(),
        finish_node_id: map.finish.node_id.clone(),
        complete: is_complete(map),
        nodes: derive_node_views(map),
    })
}

pub(super) fn render_empty_projection(map_id: &str, envelope: ProjectionEnvelope) -> String {
    format!(
        "{TASKSPACE_MAP_PROJECTION_MARKER}\n- schema_version: taskspace-map-projection-r8-v1\n- projection_kind: {}\n- map_id: {map_id}\n- map: none\n- bootstrap_required: true\n{TASKSPACE_MAP_PROJECTION_END}\n",
        envelope.kind(),
    )
}

fn indent(value: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::taskspace::TaskSpaceNodeState;

    #[test]
    fn projection_contains_full_node_view_without_old_map_layers() {
        let body = render_projection(
            TaskSpaceMapView {
                map_id: "map-1".into(),
                revision: 2,
                canonical_sha256: "abc".into(),
                root_node_id: "root".into(),
                finish_node_id: "finish".into(),
                complete: false,
                nodes: vec![TaskSpaceNodeView {
                    node_id: "work".into(),
                    goal: "Implement".into(),
                    state: TaskSpaceNodeState::Ready,
                    content: "Keep this fact".into(),
                    parents: vec!["root".into()],
                    children: vec!["finish".into()],
                    actions: vec![],
                }],
            },
            ProjectionEnvelope::CurrentProjection,
        )
        .body;

        for expected in [
            "\"parents\"",
            "\"children\"",
            "\"content\"",
            "Keep this fact",
        ] {
            assert!(body.contains(expected), "missing {expected} from {body}");
        }
        for forbidden in ["edges", "_ref", "node_details", "terminal_history"] {
            assert!(
                !body.contains(forbidden),
                "old field {forbidden} leaked into {body}"
            );
        }
    }
}
