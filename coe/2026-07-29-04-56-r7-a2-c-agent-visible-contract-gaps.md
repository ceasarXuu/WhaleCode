# Problem P-001: A2-C 三项机械规则未完整进入 Agent 可见合同
- Status: open
- Created: 2026-07-29 04:56
- Updated: 2026-07-29 04:56
- Objective: 让 `mutations` 可选语义、完成节点动作归属和单 Patch 硬规则在 L2/L4 与 Runtime 一致
- Symptoms:
  - Agent 省略文案称为 optional 的 `mutations` 时，wire 仍以缺字段拒绝
  - Agent 在同一 transaction 完成节点并把 sibling action 归属给该节点，Runtime 正确拒绝但 Tool 合同未说明
  - Agent 在同一 response 生成多个 Patch，Runtime 正确零执行拒绝但 Agent 可见合同未说明
- Expected behavior:
  - 可选字段在 schema、wire 和 Runtime 中同为可选
  - 无法由单 Tool schema 表达的 response-wide 机械规则在固定 L2 与中央 L4 同步可见
  - Runtime 保持硬校验，不解释 Patch 意图、不选择节点、不修复 Agent 参数
- Actual behavior:
  - L4/wire 与说明存在 `mutations` 必填冲突
  - 两个已有硬门只有后置拒绝，没有同等显著的事前合同
- Impact:
  - 产生可以在生成前避免的参数和 reservation reject
  - Agent 从失败反馈才首次得知底层规则，放大 request 和上下文
- Fix criteria:
  - `execute.mutations` 可省略且省略时等价于空列表
  - L2/L4 明确 completed/blocked node 不能拥有同 transaction sibling action
  - L2/L4 明确每 response 最多一个 `apply_patch`
  - 普通 Tool schema 不增加 TaskSpace 字段，Runtime 不新增语义决策
  - schema、parser、合同和 observer 回归通过
- Current conclusion: 三项均为 Agent 可见机械合同缺口；权威应在 L4，固定普通工作协议在 L2 引用同一规则，不依赖可选 L3
- Related hypotheses:
  - H-001
- Resolution basis:
  - 工程修复与聚焦回归通过
  - 同口径 A2-C live rerun pending
- Close reason:
  - not closed

## Hypothesis H-001: Agent 只能从后置拒绝学习已有 Runtime 硬规则
- Status: confirmed
- Parent: P-001
- Claim: 三项错误不是 Runtime 缺少硬门，而是 L2/L4 的事前合同与实际 parser/validator 不完整或冲突
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Falsifiable predictions:
  - If true: Runtime 已有 completed/blocked ownership 和单 Patch reject，L2/L4 没有对应文本；mutations schema/parser 与 optional 文案冲突
  - If false: L2/L4 已完整一致表达三项合同，Agent 只是偶发违反
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: 源码与 A2-C trace 同时满足预测
- Repair design readiness: implemented
- Next step: A2-C live rerun
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: L2/L4/wire 已对齐且普通 Tool 保持原生
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source:
  - `core/src/context/prompts/taskspace_core_protocol_v3.md`
  - `tools/src/taskspace_tool.rs`
  - `core/src/tools/handlers/taskspace_control_args_wire.rs`
  - `tools/src/taskspace_tool_tests.rs`
  - `core/src/tools/handlers/taskspace_control_args_tests.rs`
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - `mutations` 从 execute required 列表删除并由 serde default 为空列表
  - L2/L4 包含完成/阻塞节点归属与单 Patch 文本
  - `codex-tools` 147 passed，TaskSpace args 13 passed
- Correlation keys:
  - taskspace-core-v3.5
  - manifest 1.0.40
- Raw content:
  ```text
codex-tools: 147 passed, 1 ignored
tools::handlers::taskspace_control_args::tests: 13 passed
  ```
- Interpretation: 三项 Agent 可见合同已工程闭合；行为稳定性仍由 live rerun 验证
- Time: 2026-07-29 04:56
