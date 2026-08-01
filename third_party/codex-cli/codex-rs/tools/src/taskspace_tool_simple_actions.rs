use std::collections::BTreeMap;

use serde_json::json;

use super::JsonSchema;
use super::object_variant;

pub(super) fn read_action_schemas() -> Vec<JsonSchema> {
    let mut read_map = object_variant("read_map", BTreeMap::new(), Vec::new());
    read_map.description = Some(
        "Return the current rendered Map and canonical revision without changing state.".into(),
    );

    vec![
        read_map,
        read_output_ref_schema(
            "head",
            "Return the beginning of exact retained output by reference.",
            BTreeMap::new(),
            Vec::new(),
        ),
        read_output_ref_schema(
            "tail",
            "Return the end of exact retained output by reference.",
            BTreeMap::new(),
            Vec::new(),
        ),
        read_output_ref_schema(
            "line_range",
            "Return an exact retained output line range by reference.",
            BTreeMap::from([
                (
                    "start_line".into(),
                    JsonSchema::integer(None).with_minimum(1),
                ),
                ("end_line".into(), JsonSchema::integer(None).with_minimum(1)),
            ]),
            vec!["start_line".into(), "end_line".into()],
        ),
        read_output_ref_schema(
            "grep",
            "Return exact matching retained output ranges by reference and pattern.",
            BTreeMap::from([("pattern".into(), JsonSchema::string(None))]),
            vec!["pattern".into()],
        ),
    ]
}

fn read_output_ref_schema(
    mode: &str,
    description: &str,
    mut properties: BTreeMap<String, JsonSchema>,
    mut required: Vec<String>,
) -> JsonSchema {
    properties.insert("output_ref".into(), JsonSchema::string(None));
    properties.insert(
        "mode".into(),
        JsonSchema::string_enum(vec![json!(mode)], None),
    );
    properties.insert(
        "max_bytes".into(),
        JsonSchema::integer(None).with_minimum(1),
    );
    required.insert(0, "output_ref".into());
    required.insert(1, "mode".into());
    required.push("max_bytes".into());
    let mut schema = object_variant("read_output_ref", properties, required);
    schema.description = Some(description.into());
    schema
}
