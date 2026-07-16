use std::collections::BTreeMap;

use serde_json::json;

use super::JsonSchema;
use super::object_variant;

pub(super) fn simple_action_schemas() -> Vec<JsonSchema> {
    vec![
        object_variant(
            "expand_nodes",
            BTreeMap::from([(
                "node_ids".into(),
                JsonSchema::array(
                    JsonSchema::string(None),
                    Some(
                        "Currently folded node identifiers whose hidden event refs must be restored atomically."
                            .into(),
                    ),
                )
                .with_min_items(1),
            )]),
            vec!["node_ids".into()],
        ),
        object_variant(
            "read_output_ref",
            BTreeMap::from([
                ("output_ref".into(), JsonSchema::string(None)),
                (
                    "mode".into(),
                    JsonSchema::string_enum(
                        vec![
                            json!("head"),
                            json!("tail"),
                            json!("line_range"),
                            json!("grep"),
                        ],
                        None,
                    ),
                ),
                ("start_line".into(), JsonSchema::integer(None)),
                ("end_line".into(), JsonSchema::integer(None)),
                ("pattern".into(), JsonSchema::string(None)),
                ("max_bytes".into(), JsonSchema::integer(None)),
            ]),
            vec!["output_ref".into(), "mode".into()],
        ),
    ]
}
