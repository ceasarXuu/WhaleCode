# Problem P-001: R6 TaskSpace 嵌套参数错误丢失字段路径
- Status: verifying
- Created: 2026-07-16 07:47
- Updated: 2026-07-16 08:03
- Objective: 让 `taskspace_control` 参数错误忠实指出非法嵌套字段路径，避免 Agent 因定位信息缺失在互斥修正之间循环。
- Symptoms:
  - `subscription-billing-repair` 的一个 R6 live arm 在 Map 初始化前连续调用 `initialize_map`。
  - 反馈在 `unknown field goal, expected node_id` 与 `missing field goal` 之间摆动，但没有说明分别来自 `finish.goal` 和 Root/Work 的 `goal`。
- Expected behavior:
  - 参数反馈应包含 serde 定位到的嵌套对象路径，原样区分 `finish`、`root`、`initial_work_node` 和数组元素。
- Actual behavior:
  - handler 直接使用 `serde_json::from_str`，错误文本只保留叶层字段名和该对象的 expected fields。
- Impact:
  - Agent 无法判断哪个同名字段非法，删除所有 `goal` 后又违反 Root/Work 合同；Map 长时间保持空白，普通工具受 `no_task_path` 硬规则拒绝，请求与 token 持续放大。
- Reproduction:
  - `target/r6-phase-e/e6-final-current/subscription-billing-repair/20260716-074100-236/pair-002/left`。
- Environment:
  - Linux、Docker hard boundary、`deepseek-v4-flash`、R6 terminal candidate `019ad0745`、harness `bb817f397`。
- Known facts:
  - 第一次初始化参数中的 Finish 正确不含 `goal`，但一条 edge 错把字段名 `initial_work_node` 当成节点 ID；Runtime 忠实返回图 violations。
  - Agent 第二次修正 edge 时自行给 Finish 增加 `goal`，这是 Agent 操作错误。
  - parser 对第二次调用返回无路径的 `unknown field goal, expected node_id`。
  - 下一次调用删除 Root 的 `goal` 后，parser 返回无路径的 `missing field goal`。
  - provider-visible schema 与 Rust parser 都规定 Root/Work 有 `node_id+goal`、Finish 只有 `node_id`。
- Ruled out:
  - schema 与 parser 对 Finish/Root/Work 字段合同不一致。
  - Runtime 应自动修改 Agent 的 Map 声明。
- Fix criteria:
  - `finish.goal`、`root.goal`、`additional_work_nodes[i].goal` 等错误包含稳定嵌套路径。
  - 错误继续由原 `taskspace_control` handler 以单层 `TaskSpaceControlResultR6V1` 返回。
  - 合法参数、state-machine violations 和 nested action preflight 行为不变。
  - 原复杂样本不再因同名字段定位缺失形成 bootstrap 循环。
- Current conclusion: H-001 已修复并通过 parser、handler、sequence 与工具 schema 定向回归；等待新二进制复杂样本复验 live 循环是否消失。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 非 path-aware serde 解析丢失嵌套对象身份
- Status: confirmed
- Parent: P-001
- Claim: `serde_json::from_str::<TaskSpaceControlArgs>` 只返回局部 serde 错误，无法告诉 Agent 同名 `goal` 位于 Finish 还是 Root/Work，导致一次错误修正后无法定向恢复。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 多种嵌套节点对象复用字段名 `goal`，局部错误文本对整个 control payload 不具唯一定位能力。
- Falsifiable predictions:
  - If true: provider-visible output没有对象路径；代码直接调用普通 `serde_json::from_str`；path-aware deserializer 可对同一 payload 产生区分路径。
  - If false: 当前反馈已经含 `finish`/`root` 路径，或循环中的调用不是在同名字段之间切换。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对 live trace 中第二、三次参数分别运行普通和 path-aware serde，比较 typed error。
  - Signal: `finish`/`root`/数组索引路径与 function call output。
  - Capture method: rollout 对齐、代码路径检查和定向 parser 测试。
  - Event name or marker:
    - `task_context_event_recorded`
  - Correlation keys:
    - function call ID
  - Differentiates from:
    - H-002
  - Supports if:
    - path-aware 解析唯一定位对象，而现有输出没有。
  - Refutes if:
    - 两种解析均无法产生路径。
  - Instrumentation status: permanent regression
  - Instrumentation lifecycle:
    - 保留 parser 路径测试。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready; Phase E continued implementation is already authorized
- Next step: 重跑 `subscription-billing-repair`，确认 Agent 收到对象级路径且不再形成 bootstrap 循环。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 工具 schema 与 parser 对节点字段要求不一致
- Status: refuted
- Parent: P-001
- Claim: provider schema 要求所有节点都有 `goal`，但 parser 单独禁止 Finish 的 `goal`，因此 Agent 无法提交合法参数。
- Layer: alternative
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - `unknown field goal` 和 `missing field goal` 交替出现，表面上可能来自合同自相矛盾。
- Falsifiable predictions:
  - If true: `finish_node_schema` 暴露 `goal`，或 `graph_node_schema` 不要求 Root/Work 的 `goal`。
  - If false: schema 与 parser 分别对 Finish 和 Root/Work 完全一致。
- Diagnostic evidence plan:
  - Prediction or clause under test: 逐字段比较工具 schema builder 与 Deserialize structs。
  - Signal: properties、required、additionalProperties 与 Rust fields。
  - Capture method: 代码检查及既有 schema tests。
  - Event name or marker:
    - none
  - Correlation keys:
    - none
  - Differentiates from:
    - H-001
  - Supports if:
    - 任一字段合同不一致。
  - Refutes if:
    - Finish 均为 node_id-only，Root/Work 均为 node_id+goal。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: closed as alternative.
- Blocker:
  - none
- Close reason:
  - schema and parser contracts match

## Evidence E-001: Live trace 在无路径错误后执行全局 goal 删除
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: R6 complex pair-002 left rollout
- Prediction or plan link:
  - H-001 同名字段定位预测。
- Matched signal:
  - 第二次调用的 `finish.goal` 返回 `unknown field goal, expected node_id`；第三次调用同时删除 Root goal 后返回 `missing field goal`，两次输出均无对象路径。
- Correlation keys:
  - `call_00_18bTGedQ0Qab0Lvbjya10486`
  - `call_00_btFN77haYvoTROVflc3o2978`
- Raw content:
  ```text
  finish={node_id:finish, goal:Finish task}
  -> invalid taskspace_control arguments: unknown field `goal`, expected `node_id`
  root={node_id:root}, finish={node_id:finish}
  -> invalid taskspace_control arguments: missing field `goal`
  ```
- Interpretation: 反馈内容真实但残缺，不能唯一定位非法对象；Agent 的后续全局删除与该缺口一致。
- Time: 2026-07-16 07:47

## Evidence E-002: Schema 合同一致但 parser 使用普通 serde 入口
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: code-location
- Source: `tools/src/taskspace_tool.rs`; `core/src/tools/handlers/taskspace_control_args.rs`
- Prediction or plan link:
  - H-001/H-002 parser 与 schema 比较。
- Matched signal:
  - `finish_node_schema` 与 `TaskSpaceFinishNodeArgs` 都只有 `node_id`；Root/Work schema 与 struct 都要求 `goal`；解析入口是 `serde_json::from_str`，没有 path-aware wrapper。
- Correlation keys:
  - none
- Raw content:
  ```text
  serde_json::from_str::<TaskSpaceControlArgs>(arguments)
  finish_node_schema -> required [node_id], additionalProperties=false
  graph_node_schema -> required [node_id, goal], additionalProperties=false
  ```
- Interpretation: 合同本身一致，缺陷位于反馈定位信息，而不是 schema 字段定义。
- Time: 2026-07-16 07:47

## Evidence E-003: 两段式 path-aware wire parser 保留嵌套路径
- Related hypotheses:
  - H-001
- Direction: supports
- Type: repair-verification
- Source: `core/src/tools/handlers/taskspace_control_args_wire.rs`; targeted Rust tests
- Prediction or plan link:
  - H-001 路径保真修复。
- Matched signal:
  - 直接包裹原 internally tagged enum 仍只返回根路径 `.`；改为先解析 `action`、再解析具体 payload 后，错误分别包含 `finish`、`root`、`additional_work_nodes[0]`。
  - `cargo test -p codex-core taskspace_control_args --lib`: 13/13。
  - `cargo test -p codex-core taskspace_control --lib`: 21/21。
  - `cargo test -p codex-core tools::sequence::tests --lib`: 11/11。
  - `cargo test -p codex-tools taskspace_tool --lib`: 3/3。
  - `just bazel-lock-check` 与 `cargo build -p codex-cli --bin whale --locked` 通过。
- Correlation keys:
  - none
- Raw content:
  ```text
  invalid taskspace_control arguments at finish: unknown field `goal`, expected `node_id`
  invalid taskspace_control arguments at root: missing field `goal`
  invalid taskspace_control arguments at additional_work_nodes[0]: missing field `goal`
  ```
- Interpretation: 修复只补全参数错误的对象身份；typed result 外壳、合法参数、state-machine validation 和 nested action 路径保持不变。
- Time: 2026-07-16 08:03
