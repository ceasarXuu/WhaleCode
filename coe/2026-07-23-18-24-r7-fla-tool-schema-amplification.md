# Problem P-001: R7 FLA TaskSpace Tool Schema 固定成本异常放大
- Status: open
- Created: 2026-07-23 18:24
- Updated: 2026-07-24 01:36
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
  - H-003
  - H-004
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

## Hypothesis H-003: 零侵入会把连续动作从 Tool 合同退化成单边协议
- Status: confirmed
- Parent: P-001
- Claim: 若普通 Tool 完全不暴露 TaskSpace 关系，边界 `taskspace_control` 的单个 JSON Schema
  无法要求同一 response 中存在并紧邻另一个普通 Tool；Runtime 只能拒绝错误序列，Agent 在选取普通
  Tool 时看不到对称的配对合同。
- Layer: repair-boundary
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - function arguments schema 只能约束本调用，不能约束 sibling call；
  - 普通 Tool 零字段时，只有 control description 和 L2 单方面说明配对关系。
- Falsifiable predictions:
  - If true: 零侵入版本可以在执行阶段校验正确顺序，但自然样本仍会出现普通 Tool 先于初始化或单独
    boundary control。
  - If false: 零侵入版本在三个 TaskSpace policy 的首次响应都稳定产生 boundary + ordinary Tool。
- Diagnostic evidence plan:
  - Prediction or clause under test: 普通 Tool 零扩展是否保留模型可见的连续动作结构。
  - Signal: 三臂首三个 provider response 的 Tool 序列与 preflight/gate 结果。
  - Capture method: 对已回退提交 `f0197e819` 的 Docker 自然样本 trace 逐 request 解析。
  - Event name or marker:
    - `taskspace.main_tool_call_rejected`
    - `tool.response_preflight_rejected`
  - Correlation keys:
    - `/tmp/wc-r7-fla9-v4/a1`
    - `/tmp/wc-r7-fla9-v4/a2`
    - `/tmp/wc-r7-fla9-v4/a3`
  - Differentiates from:
    - 生命周期参数 shape 不明确；
    - sequence executor 不支持多个调用；
    - projection policy 特有行为。
  - Supports if:
    - 三臂均先出现空 Map 普通 Tool gate，随后出现单独 boundary preflight。
  - Refutes if:
    - 三臂首次 boundary 都与普通动作正确配对。
  - Instrumentation status: captured
  - Instrumentation lifecycle:
    - 保留序列配对与零执行拒绝观测。
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: confirmed；零侵入解决了成本，但删除了普通 Tool 一侧的模型可见机械关系，且自然样本
  稳定暴露连续动作采用退化。
- Repair design readiness: ready
- Next step: 用必填两值 `taskspace_binding` 恢复轻量双边合同，完整生命周期参数只在
  `taskspace_control` 暴露一次。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-003: 零侵入三臂首次连续动作退化
- Related hypotheses:
  - H-003
- Direction: supports
- Type: natural-sample-and-wire-trace
- Source: `/tmp/wc-r7-fla9-v4/{a1,a2,a3}/single-file-fast-fix/**`
- Prediction or plan link:
  - H-003：验证单边协议能否稳定产生跨 Tool sibling。
- Matched signal:
  - map-always、map-append、map-request 首响应都先提交普通命令并被空 Map gate 拒绝；
  - 三臂随后各出现一次单独 `initialize_map`，被 sequence preflight 零执行拒绝；
  - 第三次才正确组合 boundary 和普通动作；
  - 后续 handoff 证明 executor 支持同 response 顺序执行，因此失败不来自执行能力缺失。
- Correlation keys:
  - map-always run `20260723-233104-251`
  - map-append run `20260723-233104-211`
  - map-request run `20260723-233104-211`
- Raw content:
  ```text
  request 1: ordinary Tool -> no_task_path
  request 2: initialize_map alone -> taskspace_boundary_action_requires_follow_up
  request 3: initialize_map + ordinary Tool -> executed
  ```
- Interpretation: 删除普通 Tool 关系后，连续动作只剩 control 一侧的说明和 Runtime 后置拒绝；这不是
  原先轻量 carrier 的等价实现。
- Time: 2026-07-24 00:20

## Hypothesis H-004: 固定 next_call 声明不能把跨 Tool 关系变成结构约束
- Status: confirmed
- Parent: P-001
- Claim: 在三个边界 control 中增加必填固定值 `next_call="ordinary_tool"`，只能证明 Agent 知道
  “后续应有普通 Tool”，不能让单个 JSON Schema 要求同一 response 中实际存在该 sibling。
- Layer: repair-boundary
- Factor relation: single
- Depends on:
  - H-003
- Rationale:
  - Tool 参数 schema 只能约束当前 function call；
  - 固定字段与真实 sibling 的存在性没有结构关联。
- Falsifiable predictions:
  - If true: Agent 可以生成 schema 合法且带 `next_call` 的单独 `initialize_map`，随后仍被 sequence
    preflight 拒绝。
  - If false: 三种 TaskSpace policy 首次初始化均直接形成 control + ordinary Tool。
- Diagnostic evidence plan:
  - Prediction or clause under test: 固定声明是否能替代跨 Tool 结构关联。
  - Signal: provider 首响应中 control 参数和 sibling 调用数量。
  - Capture method: 使用同一 Docker image 并行运行四臂 `single-file-fast-fix`，逐 request 解析 rollout。
  - Event name or marker:
    - `tool.response_preflight_rejected`
  - Correlation keys:
    - `/tmp/whale-paired-bench-runs/f9-light-v3/a{0,1,2,3}`
  - Differentiates from:
    - 初始化 DAG 参数错误；
    - ordinary Tool schema 未携带 TaskSpace 字段；
    - executor 不支持顺序执行。
  - Supports if:
    - control 参数包含固定声明，但调用仍单独结束 response。
  - Refutes if:
    - 首次 control 始终与 ordinary Tool 配对。
  - Instrumentation status: captured
  - Instrumentation lifecycle:
    - 固定声明试验代码删除；保留序列 trace 与拒绝码观测。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed；固定字段是没有结构收益的重复表达，已从最终修复中删除。
- Repair design readiness: ready
- Next step: 保留普通 Tool 的单一两值 binding 和原子 sequence preflight；不得继续通过堆叠
  固定字段或提示词宣称已获得结构保证。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-004: next_call 四臂自然样本未消除单独初始化
- Related hypotheses:
  - H-004
- Direction: supports
- Type: natural-sample-and-wire-trace
- Source: `/tmp/whale-paired-bench-runs/f9-light-v3/a{0,1,2,3}/single-file-fast-fix/**`
- Prediction or plan link:
  - H-004：观察固定声明是否改变真实 sibling 生成。
- Matched signal:
  - 四臂业务均成功，三个 TaskSpace policy 的请求数均为 7；
  - map-append、map-request 首响应各有一次单独 `initialize_map`；
  - map-always 前两响应均为单独 `initialize_map`；
  - 后续 `complete_then_continue + action` 均可在同 response 正常顺序执行；
  - 初始化图不再遗漏 `root -> initial_work_node`，说明 edges 字段的机械描述修复有效，但与
    sibling 结构问题相互独立。
- Correlation keys:
  - map-always run `20260724-012710-433`
  - map-append run `20260724-012710-425`
  - map-request run `20260724-012710-382`
- Raw content:
  ```text
  map-always R1/R2: initialize_map alone -> taskspace_boundary_requires_after_boundary_action
  map-append R1: initialize_map alone -> taskspace_boundary_requires_after_boundary_action
  map-request R1: initialize_map alone -> taskspace_boundary_requires_after_boundary_action
  ```
- Interpretation: 固定 `next_call` 没有把 sibling 变为本次 Tool 调用的结构组成部分，不能作为
  轻量修复的有效机制。
- Time: 2026-07-24 01:36
