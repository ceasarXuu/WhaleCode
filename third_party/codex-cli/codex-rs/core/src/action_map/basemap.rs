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
    prompt.push_str("Use these candidates as a task decomposition menu. For taskspace_control(start_task/create_node), choose one runtime node_kind value. BaseMap candidates outside the hard-gated values are guidance for node titles and decomposition, not separate runtime kinds. Do not create a generic plan/implement/summary map.");
    prompt
}

pub(crate) fn node_kind_selection_prompt() -> &'static str {
    "Node kind selection rules:
- Use inspect_code_context for reading files, searching, understanding scope, design investigation, and subagent investigation nodes.
- Use implement_solution only when the node will modify code, tests, configuration, or docs.
- Use smoke_test or regression_test before running test/build/lint commands; baseline failing validation and final passing validation are test nodes.
- Use final_synthesis only for final wrap-up.
- If validation fails and edits are needed, leave the test node with its result, switch to implement_solution for the fix, then switch back to a test node for the rerun.
- Do not create custom nodes in live TaskSpace work. If work does not fit a kind, choose the closest concrete kind and explain the scope in the node title/context."
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
    }
}
