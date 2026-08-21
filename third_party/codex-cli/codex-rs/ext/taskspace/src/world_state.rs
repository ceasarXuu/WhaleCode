use codex_extension_api::PreviousWorldStateSection;
use codex_extension_api::RenderedWorldStateFragment;
use codex_extension_api::WorldStateSectionContribution;

use crate::runtime::TaskSpaceMapRecord;

const WORLD_STATE_ID: &str = "taskspace_map";
const START_MARKER: &str = "<taskspace_map>";
const END_MARKER: &str = "</taskspace_map>";

pub(crate) fn section(record: TaskSpaceMapRecord) -> WorldStateSectionContribution {
    let snapshot = serde_json::to_value(&record.map).unwrap_or(serde_json::Value::Null);
    let body = format!(
        "Canonical TaskSpace Map (store revision {}):\n{}",
        record.store_revision,
        serde_json::to_string(&record.map).unwrap_or_else(|_| "null".into())
    );
    WorldStateSectionContribution::new(WORLD_STATE_ID, snapshot.clone(), move |previous| {
        if matches!(previous, PreviousWorldStateSection::Known(value) if value == &snapshot) {
            return None;
        }
        Some(RenderedWorldStateFragment::new(
            "developer",
            (START_MARKER, END_MARKER),
            body.clone(),
        ))
    })
    .with_legacy_matcher(|role, text| {
        role == "developer"
            && text.trim_start().starts_with(START_MARKER)
            && text.trim_end().ends_with(END_MARKER)
    })
    .with_retained_fragment_matcher(|role, text| {
        role == "developer"
            && text.trim_start().starts_with(START_MARKER)
            && text.trim_end().ends_with(END_MARKER)
    })
}
