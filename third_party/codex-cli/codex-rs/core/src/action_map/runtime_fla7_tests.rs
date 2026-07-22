use super::*;
use serde_json::Value;
use serde_json::json;
use std::fs;

const LIFECYCLE_CONTRACT: &str =
    include_str!("../../../../../../benchmarks/taskspace/r7/five-layer-lifecycle-oracles-v1.json");
const LIFECYCLE_GOLDEN: &str =
    include_str!("../../../../../../benchmarks/taskspace/r7/five-layer-lifecycle-golden-v1.json");

fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn runtime_for_map(map: TaskSpaceMap) -> ActionMapRuntimeState {
    let owner = ThreadId::new();
    let map_id = map.id.clone();
    let current_binding = map.current_binding.clone();
    let mut state = ActionMapRuntimeState::default();
    state.mode = MapRuntimeMode::Experiment;
    state.bootstrap_required = false;
    state.routing_required = false;
    state.active_map_id = Some(map_id.clone());
    state.current_main_node_id = current_binding;
    state.maps.insert(
        map_id,
        ActionMapInstance::from_graph(map, Vec::new(), Some(owner)),
    );
    state
}

fn rendered_fixture(contract: &Value, revision_name: &str) -> Value {
    let fixture = &contract["fixture_maps"][revision_name];
    let map: TaskSpaceMap = serde_json::from_value(fixture["canonical_state"].clone())
        .expect("fixture canonical state must deserialize as TaskSpaceMap");
    let canonical_json = serde_json::to_string(&map).expect("serialize canonical map");
    let canonical_sha256 = sha256(&canonical_json);
    assert_eq!(canonical_sha256, fixture["canonical_sha256"]);

    let map_id = map.id.clone();
    let mut state = runtime_for_map(map);
    let current_projection = state
        .build_developer_context_for_map(&map_id, ProjectionEnvelope::CurrentProjection)
        .expect("current projection");
    let request_snapshot = state
        .build_developer_context_for_map(&map_id, ProjectionEnvelope::RequestSnapshot)
        .expect("request snapshot");
    let map_handle = state.build_map_handle_context().expect("map handle");

    json!({
        "canonical_sha256": canonical_sha256,
        "map_always": {
            "carrier": "ephemeral_current_projection",
            "sha256": sha256(&current_projection),
            "text": current_projection
        },
        "map_append": {
            "carrier": "persisted_request_snapshot",
            "sha256": sha256(&request_snapshot),
            "text": request_snapshot
        },
        "map_request": {
            "carrier": "ephemeral_map_handle",
            "sha256": sha256(&map_handle),
            "text": map_handle
        }
    })
}

fn production_golden() -> Value {
    let contract: Value = serde_json::from_str(LIFECYCLE_CONTRACT).expect("lifecycle contract");
    json!({
        "schema_version": 1,
        "contract_id": "r7-five-layer-lifecycle-golden-v1",
        "source_contract_id": contract["contract_id"],
        "fixture_outputs": {
            "revision_4": rendered_fixture(&contract, "revision_4"),
            "revision_5": rendered_fixture(&contract, "revision_5")
        }
    })
}

#[test]
fn fla7_fixture_maps_render_through_the_shared_production_carriers() {
    let actual = production_golden();
    if let Ok(output_path) = std::env::var("R7_FLA7_GOLDEN_OUT") {
        let bytes = serde_json::to_vec_pretty(&actual).expect("serialize generated golden");
        fs::write(output_path, bytes).expect("write generated golden");
        return;
    }

    let expected: Value = serde_json::from_str(LIFECYCLE_GOLDEN).expect("committed golden");
    assert_eq!(actual, expected);
}
