use super::state::ActionMapRuntimeState;
use crate::action_map::projection::ProjectionEnvelope;
use crate::action_map::projection::TASKSPACE_MAP_HANDLE_MARKER;
use crate::action_map::projection::render_empty_projection;
use crate::action_map::projection::render_projection;
use crate::action_map::projection::taskspace_map_view;

impl ActionMapRuntimeState {
    pub(crate) fn build_developer_context(&self, envelope: ProjectionEnvelope) -> Option<String> {
        let map_id = self.active_map_id.as_deref()?;
        Some(
            self.build_developer_context_for_map(map_id, envelope)
                .unwrap_or_else(|| render_empty_projection(map_id, envelope)),
        )
    }

    pub(crate) fn build_map_handle_context(&self) -> Option<String> {
        let map_id = self.active_map_id.as_deref()?;
        let (revision, bootstrap_required, complete) =
            self.active_map()
                .map_or(("none".to_string(), true, false), |map| {
                    (
                        map.canonical_map().revision.to_string(),
                        false,
                        map.is_complete(),
                    )
                });
        Some(format!(
            "{TASKSPACE_MAP_HANDLE_MARKER}\n- taskspace_active: true\n- map_id: {map_id}\n- revision: {revision}\n- bootstrap_required: {bootstrap_required}\n- complete: {complete}\nTaskSpaceMapHandleR8V1 end.\n"
        ))
    }

    pub(crate) fn build_developer_context_for_map(
        &self,
        map_id: &str,
        envelope: ProjectionEnvelope,
    ) -> Option<String> {
        let map = self.maps.get(map_id)?;
        let input = taskspace_map_view(map.canonical_map()).ok()?;
        Some(render_projection(input, envelope).body)
    }
}
