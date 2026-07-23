# Problem P-001: R7 FLA TaskSpace Tool Schema 固定成本异常放大
- Status: open
- Created: 2026-07-23 18:24
- Updated: 2026-07-23 18:24
- Objective: 证明 FLA 首轮 TaskSpace Tool schema 从 Standard 的约 5.4K tokens 放大到约 15.2K tokens 的真实构造机制，并区分必要合同与异常重复。
- Symptoms:
  - 三个 TaskSpace policy 的首请求 Tool schema 均为约 15,186 estimated tokens，Standard 为约 5,418。
- Expected behavior:
  - TaskSpace 可让普通工具原子携带机械状态动作，但固定 Tool schema 成本不应随普通工具数量线性复制完整 DAG/lifecycle 合同。
- Actual behavior:
  - 同一份四分支 `taskspace_action` schema 被内联进每一个普通工具，同时额外暴露 `taskspace_control`。
- Impact:
  - 每个 TaskSpace provider request 固定增加约 9,768 estimated input tokens；该成本独立于 projection policy 和任务复杂度。
- Reproduction:
  - 读取 FLA-8 四臂 repeat-3 首请求 `provider-wire-trace.jsonl`，对比 Standard 与任一 TaskSpace 臂的 `tools` section。
- Environment:
  - Linux/Docker，DeepSeek V4 Flash，subject commit `f2baea6d13caef02f15e1a3c6938a3fa05a3d315`，run `20260723-073642-091`。
- Known facts:
  - Standard 首请求 `tools_count=12`、`tools_bytes=21669`、`estimated_tokens=5418`。
  - TaskSpace 首请求 `tools_count=13`、`tools_bytes=60743`、`estimated_tokens=15186`。
  - TaskSpace provider 可见性路径对所有普通工具调用 `decorate_taskspace_carrier_tool`。
  - 装饰器把完整 `taskspace_action_schema()` 插入每个普通工具并标为 required。
- Ruled out:
  - projection 正文造成该首请求固定差额。
  - provider token usage 统计异常。
  - `taskspace_control` 再次暴露 `initialize_map`、`bind_node` 或 `complete_then_continue`；当前 control schema 不含这些 action。
- Fix criteria:
  - 在不破坏普通动作与机械状态原子绑定的前提下，消除完整 lifecycle schema 随普通工具数线性复制，并通过最终 provider wire payload、自然样本与状态原子性回归验证。
- Current conclusion: 固定成本的根因已经确认。它不是 wire 观测误差，也不是三种 projection policy 的固有成本；它是当前 carrier 表达方式导致的结构性重复。原子 carrier 是必要能力，但把完整四分支 DAG/lifecycle schema 内联到 12 个普通工具属于异常的实现形态。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 完整 taskspace_action schema 被按普通工具数量重复内联
- Status: confirmed
- Parent: P-001
- Claim: TaskSpace Tool schema 放大的主要机制是 `decorate_taskspace_carrier_tool` 向每个普通工具复制同一份四分支 `taskspace_action_schema()`。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 三个 TaskSpace policy 的 Tool section 完全相同，且代码存在统一装饰路径。
- Falsifiable predictions:
  - If true: TaskSpace 可见性路径应遍历全部普通 ToolSpec 并内联 action schema，wire Tool bytes 应显著高于仅增加一个 control tool 的规模。
  - If false: action schema 只出现一次，或固定差额主要来自 projection/system message。
- Diagnostic evidence plan:
  - Prediction or clause under test: Tool registry 构造是否按普通工具复制 lifecycle schema。
  - Signal: 最终 wire Tool bytes/count、装饰器调用路径和 schema variant 数。
  - Capture method: 对 FLA 首请求 wire trace 与生产 schema builder 源码逐项对账。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - subject commit `f2baea6d13caef02f15e1a3c6938a3fa05a3d315`
    - run `20260723-073642-091`
  - Differentiates from:
    - H-002
  - Supports if:
    - 每个普通工具都包含 action schema，且 Tool section 固定放大与 policy 无关。
  - Refutes if:
    - wire payload 只有一个 action schema 或差额来自其他 section。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 section bytes/hash；后续增加脱敏 per-tool schema bytes 观测。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed；当前 12 个普通工具各复制一份包含 4 个 variant 的完整 action schema，再加一个独立 `taskspace_control`。
- Repair design readiness: ready
- Next step: 修复设计必须保留单次普通工具调用与状态动作的原子绑定，不能退回 Runtime 推断或拆分 provider request。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Tool section 差额来自 projection 或 wire 统计误差
- Status: refuted
- Parent: P-001
- Claim: 约 9,768 estimated tokens 差额不是 Tool schema 本身，而是 projection 被错误计入 Tool section或 provider usage 估算异常。
- Layer: diagnostic
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 首请求同时包含 bootstrap projection，需要排除分区归属错误。
- Falsifiable predictions:
  - If true: section scanner 应把 projection bytes 计入 tools，或三种 policy 的 Tool bytes 随 projection 内容变化。
  - If false: tools section hash/bytes 在三个 TaskSpace policy 中固定，projection 独立计量。
- Diagnostic evidence plan:
  - Prediction or clause under test: 最终 provider payload 的 `tools` JSON 是否被独立、稳定计量。
  - Signal: section count/bytes/hash 与 policy 间一致性。
  - Capture method: 对 24 个 run 的 `provider-wire-trace.jsonl` 聚合。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - request index
    - projection policy
  - Differentiates from:
    - H-001
  - Supports if:
    - Tool bytes 随 projection 改变或分区重叠。
  - Refutes if:
    - 三种 TaskSpace policy 每 request 的 Tool section均为同一 bytes/hash。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain as permanent observability
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: refuted；Tool section 直接读取最终 wire JSON 的 `tools` 字段，三种 policy 均稳定为 60,743 bytes。
- Repair design readiness: not applicable
- Next step: 不修改 projection renderer 或 provider usage accounting 来处理该成本。
- Blocker:
  - none
- Close reason:
  - hypothesis refuted

## Evidence E-001: FLA 最终 provider payload Tool section 对账
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports H-001; refutes H-002
- Type: diagnostic-log
- Source: `target/r7-five-layer-eval-data/f2baea6d13caef02f15e1a3c6938a3fa05a3d315/20260723-073642-091/raw/**/provider-wire-trace.jsonl`
- Prediction or plan link:
  - H-001/H-002：区分固定 schema 放大与 projection/统计误差。
- Matched signal:
  - Standard: 12 tools、21,669 bytes、5,418 estimated tokens。
  - TaskSpace: 13 tools、60,743 bytes、15,186 estimated tokens。
- Correlation keys:
  - run `20260723-073642-091`
- Raw content:
  ```text
  Standard tools: count=12 bytes=21669 estimated_tokens=5418
  TaskSpace tools: count=13 bytes=60743 estimated_tokens=15186
  delta: bytes=39074 estimated_tokens=9768
  ```
- Interpretation: 证明差额位于最终 provider payload 的 Tool section，且首请求尚无长历史，不能归因于 projection 累积。
- Time: 2026-07-23 18:24

## Evidence E-002: 生产 schema builder 按普通工具复制 action schema
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/session/turn.rs:1146-1164`、`third_party/codex-cli/codex-rs/tools/src/taskspace_carrier.rs:12-87`、`third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs:244-283`
- Prediction or plan link:
  - H-001：确认最终 Tool section 的构造机制。
- Matched signal:
  - TaskSpace 可见性路径过滤 `update_plan` 后，对每个 ToolSpec 调用装饰器；装饰器执行 `properties.insert("taskspace_action", taskspace_action_schema())`。
- Correlation keys:
  - none
- Raw content:
  ```text
  .filter(|spec| spec.name() != "update_plan")
  .map(codex_tools::decorate_taskspace_carrier_tool)

  properties.insert(ACTION_FIELD.into(), taskspace_action_schema());
  required.push(ACTION_FIELD.into());
  ```
- Interpretation: 证明放大是确定性的生产构造行为，不是 Agent 运行差异。当前 action schema 包含 `continue_current`、`initialize_map`、`bind_node`、`complete_then_continue` 四个完整 variant。
- Time: 2026-07-23 18:24
