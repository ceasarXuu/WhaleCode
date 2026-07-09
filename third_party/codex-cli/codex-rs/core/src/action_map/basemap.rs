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
    prompt.push_str("Candidate nodes are a task decomposition menu, not a checklist. taskspace_control(start_task/create_node) accepts one runtime node_kind value. BaseMap candidates outside the hard-gated values are node title/decomposition labels, not separate runtime kinds. Runtime rejects live custom nodes except restored legacy nodes.");
    prompt.push('\n');
    prompt
}

pub(crate) fn node_kind_selection_prompt() -> &'static str {
    "Node kind capability facts:
- inspect_code_context permits file reading, search, scope understanding, design investigation, and subagent investigation evidence.
- inspect_code_context can record conflicts among README/spec docs, tests, implementation, and validator output before edit work begins.
- implement_solution is the node kind that permits modifying code, tests, configuration, or docs.
- smoke_test and regression_test record post-implementation test/build/lint validation evidence.
- pre-fix diagnostic failures are evidence facts; post-edit validation results are validation facts.
- final_synthesis is answer-only/read-only synthesis. State baseline rejects edits, tests, builds, ordinary tools, and subagent spawn from final_synthesis.
- final answer output contract: user-visible phases, files, tests, outcomes, and residual risk; internal TaskSpace terms are hidden unless the user explicitly asks to debug TaskSpace.
- validation failure evidence can support later implementation rework; rerun validation evidence belongs on a validation node chosen by the Agent.
- multiple ready inspect_code_context nodes and explorer subagents are available for independent evidence tracks. The Agent chooses whether that structure is useful.
- path correction, known-file reads, and single-evidence follow-up reads are ordinary inspect evidence; the Agent chooses the task structure.
- node result packages are expected to be cohesive enough to carry useful claims, evidence refs, changed artifacts, uncertainty, and blockers.
- live TaskSpace node_kind values are hard-gated to inspect_code_context, implement_solution, smoke_test, regression_test, and final_synthesis; custom nodes are reserved for restored legacy nodes.

Typed node finish contracts:
- inspect_code_context/discover finish requires successful read/search evidence.
- implement_solution/patch finish requires a successful edit action, unless the node is blocked with an explicit no-edit reason.
- smoke_test/regression_test/validate finish requires a successful test/build result.
- final_synthesis is answer-only synthesis; it does not require ledger criteria or decisions.
- unmet hard state is represented by blocker evidence or a correctly typed follow-up node."
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
        assert!(!prompt.contains("TaskSpace cognitive protocol (MVP)"));
        assert!(!prompt.contains("problem-state and model manager"));
        assert!(prompt.contains("Typed node finish contracts"));
        assert!(prompt.contains("finish requires successful read/search evidence"));
        assert!(prompt.contains("does not require ledger criteria or decisions"));
        assert!(!prompt.contains("promote_taskspace"));
        assert!(!prompt.contains("promotion_not_in_mvp"));
    }
}
