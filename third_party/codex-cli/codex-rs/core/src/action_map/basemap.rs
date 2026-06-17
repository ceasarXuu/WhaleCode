#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BaseMapCandidateNode {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) when_to_use: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BaseMap {
    pub(crate) version: &'static str,
    pub(crate) candidate_nodes: &'static [BaseMapCandidateNode],
}

pub(crate) const BASE_MAP: BaseMap = BaseMap {
    version: "base-map-v1",
    candidate_nodes: &[
        BaseMapCandidateNode {
            id: "define_scope",
            title: "确定边界",
            when_to_use: "明确用户目标、非目标、风险边界和验收口径。",
        },
        BaseMapCandidateNode {
            id: "inspect_code_context",
            title: "梳理代码上下文",
            when_to_use: "定位真实代码路径、已有抽象、调用链和可复用基建。",
        },
        BaseMapCandidateNode {
            id: "research_external_context",
            title: "搜索外部资料",
            when_to_use: "需要最新事实、官方文档、社区案例或竞品行为证据。",
        },
        BaseMapCandidateNode {
            id: "identify_constraints",
            title: "识别约束",
            when_to_use: "梳理工程约束、兼容性、权限、性能、隐私和发布边界。",
        },
        BaseMapCandidateNode {
            id: "design_solution",
            title: "方案设计",
            when_to_use: "把目标和代码现实转成可执行技术方案。",
        },
        BaseMapCandidateNode {
            id: "design_logging",
            title: "日志设计",
            when_to_use: "新增或调整行为需要可观测性、诊断线索和失败定位能力。",
        },
        BaseMapCandidateNode {
            id: "design_tests",
            title: "测试设计",
            when_to_use: "定义冒烟、回归、单元或集成验证路径。",
        },
        BaseMapCandidateNode {
            id: "review_solution",
            title: "方案审查",
            when_to_use: "检查方案是否过度设计、遗漏约束或偏离项目目标。",
        },
        BaseMapCandidateNode {
            id: "implement_solution",
            title: "方案实施",
            when_to_use: "按已确认方案修改代码、配置或文档。",
        },
        BaseMapCandidateNode {
            id: "review_code",
            title: "代码审查",
            when_to_use: "检查改动质量、回归风险、边界条件和维护成本。",
        },
        BaseMapCandidateNode {
            id: "smoke_test",
            title: "冒烟测试",
            when_to_use: "快速证明主路径可运行，优先覆盖真实执行入口。",
        },
        BaseMapCandidateNode {
            id: "regression_test",
            title: "回归测试",
            when_to_use: "验证相关旧行为没有被破坏。",
        },
        BaseMapCandidateNode {
            id: "final_synthesis",
            title: "最终合成",
            when_to_use: "汇总结果、残余风险、验证结论和下一步建议。",
        },
    ],
};

pub(crate) fn base_map_metadata_prompt() -> String {
    let mut prompt = format!(
        "BaseMap metadata version: {}\nRuntime node_kind values for hard gate: inspect_code_context, implement_solution, smoke_test, regression_test, final_synthesis. `custom` is reserved only for restored legacy nodes and must not be used for live node creation.\n{}\nCandidate nodes:\n",
        BASE_MAP.version,
        node_kind_selection_prompt()
    );
    for node in BASE_MAP.candidate_nodes {
        prompt.push_str("- ");
        prompt.push_str(node.id);
        prompt.push_str(": ");
        prompt.push_str(node.title);
        prompt.push_str(" - ");
        prompt.push_str(node.when_to_use);
        prompt.push('\n');
    }
    prompt.push_str("Use these candidates as a task decomposition menu, not as a checklist. Start with the minimum sufficient map for the user's task. Simple single-file or single-failure work should usually stay on a narrow main-agent chain instead of expanding many candidate nodes. For taskspace_control(start_task/create_node), choose one runtime node_kind value. BaseMap candidates outside the hard-gated values are guidance for node titles and decomposition, not separate runtime kinds. Do not create a generic plan/implement/summary map.");
    prompt.push('\n');
    prompt.push_str(cognitive_state_protocol_prompt());
    prompt
}

pub(crate) fn node_kind_selection_prompt() -> &'static str {
    "Node kind selection rules:
- Use inspect_code_context for reading files, searching, understanding scope, design investigation, and subagent investigation nodes.
- In inspect_code_context, reconcile README/spec docs, tests, and implementation before editing; explicit product docs can make an existing test expectation stale.
- Use implement_solution only when the node will modify code, tests, configuration, or docs.
- Use smoke_test or regression_test before post-implementation test/build/lint commands.
- Keep expected failing pre-fix diagnostic test runs inside inspect_code_context; do not create a separate smoke_test node just to prove the current bug fails.
- Use final_synthesis only for answer-only final wrap-up after accepted validation; do not edit, test, build, spawn agents, or call ordinary tools from final_synthesis. The user-facing final answer must describe work phases and outcomes without internal TaskSpace terms such as task, map, node, subagent, spawn, lease, final_synthesis, or taskspace_control unless the user explicitly asks to debug TaskSpace. If the user asks how work was organized, describe visible phases, files, tests, and outcomes only; never mention hidden execution roles or words such as subagent, explorer, agent, delegated, parallel, evidence track, fan-out, or spawn.
- If validation fails and edits are needed, leave the test node with its result, switch to implement_solution for the fix, then switch back to a test node for the rerun.
- Prefer the minimum sufficient node chain; create multiple ready inspect_code_context nodes only for independent evidence tracks with distinct source surfaces.
- If the user request names or implies two or more distinct functional surfaces, files, modules, or evidence classes before editing, treat them as independent investigation tracks until evidence proves otherwise. Create separate ready inspect_code_context nodes for those tracks and assign explorer subagents; the main agent should coordinate and integrate instead of reading every track itself.
- Keep path correction, known-file reads, and single-evidence follow-up reads inside the current inspect_code_context node; do not create another inspect node or spawn an explorer just to read one known file.
- Do not split work into micro-nodes. A node should be a cohesive theme of work with a useful result package, not one file read, one command, one trivial observation, or one small thought.
- Do not create custom nodes in live TaskSpace work. If work does not fit a kind, choose the closest concrete kind and explain the scope in the node title/context.

Typed node finish contracts:
- inspect_code_context/discover can finish only after successful read/search evidence or a problem-state update tied to the node, such as a fact, open question, or decision.
- implement_solution/patch can finish only after a successful edit action, unless the node is blocked with an explicit no-edit reason.
- smoke_test/regression_test/validate can finish only after a successful test/build result and a satisfied success criterion that cites this validation node's result.
- final_synthesis can finish only after at least one success criterion is satisfied or waived with evidence and at least one decision is recorded.
- If the contract is not met, do not write a stronger summary. Block the node or create/bind the correct follow-up node."
}

pub(crate) fn cognitive_state_protocol_prompt() -> &'static str {
    "TaskSpace cognitive protocol (MVP):
- The main agent is the task's problem-state and model manager, not a linear worker. Maintain the task map, assign bounded nodes, integrate evidence, and update the task's current model before acting.
- Ordinary work and subagent spawn require cognitive preflight: at start_task or after route_task and before the first non-control action, record at least one output contract and one fact source with non-empty evidence_refs.
- At task start or when requirements change, record user-stated acceptance criteria, required artifact/format/schema/validator/non-goals as output contracts with evidence_refs before relying on them. When starting a new task from a clear request, include initial_success_criteria, initial_output_contracts, and initial_fact_sources directly in start_task. Use artifact_ref for the current user request, README/spec/test/source paths, or validator_ref for observed checks when no result_id exists yet.
- Prefer taskspace_control(action=state_commit, schema_version=taskspace-state-commit-v1) at cognitive checkpoints when updating multiple problem-state or lifecycle records. Put related nodes, finished_nodes, blockers, result_validities, result_adoptions, success_criteria, output_contracts, fact_sources, facts, decisions, and next_best_action in one commit_id. Runtime accepts valid sections, rejects invalid sections with structured errors, and keeps legacy record_* actions available for focused corrections.
- Record fact sources for user-provided facts, observed environment facts, and test/validator outputs. Keep generated_for_test_only, inferred, and unknown provenance out of active task facts and final user claims unless they are explicitly rechecked against observed/provided evidence.
- Treat subagent and node results as evidence packages, not final truth. After finish_node/block_node or subagent completion, call mark_result_validity before any further ordinary work, spawn, or final answer. Accepted implementation results must include claims, evidence refs, and changed_artifacts for modified files.
- Active facts must cite accepted results or observed/provided fact sources. Questioned, invalid, unreviewed, generated, inferred, or unknown material may guide investigation but cannot anchor the final answer.
- Direct trace events are an internal audit log for observability and replay. Do not expose task/map/node/subagent terminology to the user unless the user is explicitly debugging TaskSpace.
- Final answers are user-facing product output. Collapse hidden orchestration into ordinary phrases such as investigation, implementation, and validation; do not mention subagent, explorer, agent, delegated, parallel, evidence track, fan-out, spawn, lease, taskspace_control, task, map, or node."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basemap_exposes_expected_candidate_nodes() {
        assert_eq!(BASE_MAP.version, "base-map-v1");
        assert!(BASE_MAP.candidate_nodes.len() >= 10);
        assert!(
            BASE_MAP
                .candidate_nodes
                .iter()
                .any(|node| node.id == "define_scope")
        );
        assert!(
            BASE_MAP
                .candidate_nodes
                .iter()
                .any(|node| node.id == "smoke_test")
        );
        let prompt = base_map_metadata_prompt();
        assert!(prompt.contains("TaskSpace cognitive protocol (MVP)"));
        assert!(prompt.contains("problem-state and model manager"));
        assert!(prompt.contains("Typed node finish contracts"));
        assert!(prompt.contains("Do not split work into micro-nodes"));
        assert!(prompt.contains("generated_for_test_only"));
        assert!(!prompt.contains("promote_taskspace"));
        assert!(!prompt.contains("promotion_not_in_mvp"));
    }
}
