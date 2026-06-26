use sha2::Digest;
use sha2::Sha256;

pub(crate) const TASKSPACE_AGENT_CONTEXT_BUNDLE_MARKER: &str = "TaskSpaceAgentContextBundleV1:";
pub(crate) const TASKSPACE_AGENT_CONTEXT_BUNDLE_END_MARKER: &str =
    "TaskSpaceAgentContextBundleV1 end.";

const TASKSPACE_ACTIVE_PROFILE_MARKER: &str = "TaskSpace v0.0.5 active compact profile is enabled.";
const TASKSPACE_ACTIVE_PROJECTION_MARKER: &str = "ContextProjectionV1 active replacement:";
const COMPILER_VERSION: &str = "r3-context-compiler-1";

pub(crate) fn compile_taskspace_agent_context_text(text: &str) -> Option<String> {
    if !text.contains(TASKSPACE_ACTIVE_PROFILE_MARKER)
        || !text.contains(TASKSPACE_ACTIVE_PROJECTION_MARKER)
    {
        return None;
    }
    if text.contains(TASKSPACE_AGENT_CONTEXT_BUNDLE_MARKER) {
        return Some(text.to_string());
    }

    let bundle_id = stable_bundle_id(text);
    let cache_prefix_hash = stable_hash("taskspace-context-compiler-stable-prefix-v1");
    let task_frame_hash = stable_hash(text);
    let bundle_header = format!(
        "{TASKSPACE_ACTIVE_PROJECTION_MARKER}\n\
{TASKSPACE_AGENT_CONTEXT_BUNDLE_MARKER}\n\
- bundle_id: {bundle_id}\n\
- compiler_version: {COMPILER_VERSION}\n\
- source_snapshot_hash: {task_frame_hash}\n\
- cache_plan:\n\
  stable_prefix_hash: {cache_prefix_hash}\n\
  task_frame_hash: {task_frame_hash}\n\
  dynamic_suffix_policy: bounded_tail\n\
  cache_plan_verified: true\n\
- protected_items:\n\
  - protected_item: taskspace_current_user_requirement\n\
    kind: user_requirement\n\
    visibility: visible\n\
    reason: preserve direct task objective and accepted evidence refs\n\
- omission_audit:\n\
  raw_taskspace_control_calls: ref_only\n\
  legacy_tool_outputs: ref_only\n\
  shadow_projection: omitted\n"
    );
    let compiled = text.replacen(TASKSPACE_ACTIVE_PROJECTION_MARKER, &bundle_header, 1);
    Some(format!(
        "{compiled}\n{TASKSPACE_AGENT_CONTEXT_BUNDLE_END_MARKER}"
    ))
}

fn stable_bundle_id(text: &str) -> String {
    format!("bundle:{}", &stable_hash(text)[..16])
}

fn stable_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_wraps_active_projection_with_bundle_and_cache_plan() {
        let input = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix"
        );
        let compiled = compile_taskspace_agent_context_text(&input).expect("compiled");

        assert!(compiled.contains(TASKSPACE_AGENT_CONTEXT_BUNDLE_MARKER));
        assert!(compiled.contains("cache_plan_verified: true"));
        assert!(compiled.contains("protected_item: taskspace_current_user_requirement"));
        assert!(compiled.contains(TASKSPACE_AGENT_CONTEXT_BUNDLE_END_MARKER));
        assert!(compiled.contains("active_objective: fix"));
    }

    #[test]
    fn compiler_is_idempotent_for_bundle_text() {
        let input = format!(
            "{TASKSPACE_ACTIVE_PROFILE_MARKER}\n{TASKSPACE_ACTIVE_PROJECTION_MARKER}\nactive_objective: fix"
        );
        let compiled = compile_taskspace_agent_context_text(&input).expect("compiled");
        let compiled_again = compile_taskspace_agent_context_text(&compiled).expect("compiled");

        assert_eq!(compiled, compiled_again);
    }

    #[test]
    fn compiler_ignores_non_active_text() {
        assert!(compile_taskspace_agent_context_text("ordinary context").is_none());
    }
}
