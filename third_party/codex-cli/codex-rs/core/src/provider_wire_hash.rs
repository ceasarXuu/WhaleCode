use serde_json::Map;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;

pub(super) fn section_hashes(
    message_values: [Vec<Value>; 4],
    base_instructions_value: Value,
    tools_value: Value,
    tool_choice_value: Value,
    other_value: Map<String, Value>,
) -> [String; 8] {
    let [system, natural, projection, ordinary] = message_values;
    [
        json_hash(&Value::Array(system)),
        json_hash(&Value::Array(natural)),
        json_hash(&Value::Array(projection)),
        json_hash(&Value::Array(ordinary)),
        json_hash(&base_instructions_value),
        json_hash(&tools_value),
        json_hash(&tool_choice_value),
        json_hash(&Value::Object(other_value)),
    ]
}

pub(super) fn json_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_default()
}

pub(super) fn json_hash(value: &Value) -> String {
    byte_hash(&json_bytes(value))
}

pub(super) fn byte_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
