# Problem P-001: R7 FLA TaskSpace Tool Schema 固定成本异常放大
- Status: open
- Created: 2026-07-23 18:24
- Updated: 2026-07-23 23:35
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
- Current conclusion: 固定成本的根因已经确认。它不是 wire 观测误差，也不是三种 projection
  policy 的固有成本；它是 lifecycle 合同侵入普通 Tool 并按 Tool 数量重复导致的结构性放大。
  紧凑嵌套 transition 虽降低了字节数，但自然样本证明通用 `arguments` 隐藏了精确参数合同，且仍未
  消除侵入。最终修复改为唯一 `taskspace_control` 精确承载边界动作，并与下一普通动作在同一
  provider response 中连续声明。普通 Tool 去侵入、状态硬门和固定 Tool 成本修复已验证；自然样本同时
  证明 bootstrap sibling 采用仍不稳定，需要单独的产品机制决策，不能靠继续重复提示词收敛。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-003: active binding 已足以为普通 Tool 提供机械归属
- Status: confirmed
- Parent: P-001
- Claim: 删除每次普通 Tool 的 `continue_current` 不会让动作失去节点归属，因为 ActionMap runtime
  已在业务 Tool 执行前使用 canonical active node 和 lease 建立 call reservation，并在结果返回后
  写回该 reservation。
- Layer: repair-boundary
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 如果普通 Tool 的归属还依赖 Agent 每次提交 revision/node，去掉 required carrier 会绕过状态机；
    如果 runtime 已独立完成绑定校验和 reservation，则该重复提交只增加 schema 与协议成本。
- Falsifiable predictions:
  - If true: 无 carrier 的普通 Tool 仍会经过 Map、binding、lease 校验，并使用 canonical binding
    预留和记录结果。
  - If false: carrier 是生成 node reservation 的唯一来源，或无 carrier 时 Tool 可在空 Map 下执行。
- Diagnostic evidence plan:
  - Prediction or clause under test: 普通 Tool 的节点归属是否由 canonical runtime state 独立完成。
  - Signal: `prepare_main_tool_call` 的 gate、reservation 字段来源及结果归档路径。
  - Capture method: 对普通 Tool 执行链进行静态调用链核对，并由修复后的定向测试验证空 Map 拒绝、
    active binding 接受和结果归属。
  - Event name or marker:
    - `taskspace.main_tool_call_reserved`
    - `taskspace.main_tool_result_recorded`
  - Correlation keys:
    - call id
    - map id
    - node id
    - lease id
  - Differentiates from:
    - Runtime 根据 Tool 语义推断节点的越界方案。
  - Supports if:
    - reservation 的 map/node/lease 全部来自 runtime canonical state，且 carrier 不参与字段选择。
  - Refutes if:
    - reservation 依赖 Agent 提交的 current node 或 revision。
  - Instrumentation status: existing-runtime-path
  - Instrumentation lifecycle:
    - 保留现有 reservation/result 事件，并新增 provider transition schema 观测。
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: confirmed；普通 Tool 的 active binding 归属是既有 runtime 硬基线，
  `continue_current` 只是在 provider schema 中重复声明 canonical state。
- Repair design readiness: ready
- Next step: 把普通 Tool carrier 收敛为只在初始化、绑定和 handoff 时出现的可选紧凑 transition。
- Blocker:
  - none
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

## Evidence E-003: 普通 Tool 已按 canonical active binding 独立预留和归档
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-path
- Source: `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs:181`、
  `third_party/codex-cli/codex-rs/core/src/session/mod.rs:947`、
  `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1797`、
  `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1874`
- Prediction or plan link:
  - H-003：确认无 `continue_current` 时仍存在不可绕过的机械归属。
- Matched signal:
  - `prepare_main_tool_call` 先校验 routing、maintenance barrier 和 main binding，再从 runtime
    `active_map_id/current_main_node_id/current_main_lease_id` 创建 call reservation；结果归档按该
    reservation 写回，不读取 Agent 提交的 binding 字段。
- Correlation keys:
  - call id
- Raw content:
  ```text
  self.validate_main_binding(owner_session_id)?;
  let map_id = self.active_map_id.clone()?;
  let node_id = self.current_main_node_id.clone()?;
  let lease_id = self.current_main_lease_id.clone()?;
  self.reserve_main_tool_call(call_id, MainToolReservation { map_id, node_id, lease_id, ... });
  ```
- Interpretation: 删除逐调用 `continue_current` 不会降低状态机硬基线；空 Map、无 binding 或无 lease
  仍在真实 Tool dispatch 前失败。
- Time: 2026-07-23 19:42

## Hypothesis H-004: 通用嵌套 arguments 会隐藏边界 action 的精确参数合同
- Status: confirmed
- Parent: P-001
- Claim: 仅在普通 Tool 中暴露
  `taskspace_transition: { action, arguments: object }`，会让 Agent 看不到不同边界 action 的
  必填字段和嵌套形状，从而稳定产生可避免的初始化协议错误。
- Layer: repair-validation
- Factor relation: single
- Depends on:
  - H-001
  - H-003
- Rationale:
  - 紧凑 schema 通过不复制 root、edge 和 finish 结构降低成本，但代价是把正确调用合同移出
    provider Tool schema。L2 文字不能替代 JSON Schema 对字段结构的直接约束。
- Falsifiable predictions:
  - If true: 自然样本会在业务动作前反复猜测 root、finish identity 或 node id 的形状，且错误
    集中于 transition 解析。
  - If false: Agent 能依靠 L2 稳定生成一次通过的嵌套 transition，或失败与参数 shape 无关。
- Diagnostic evidence plan:
  - Prediction or clause under test: 紧凑 schema 是否保留足够的可操作合同。
  - Signal: 首次 Map 初始化前的调用参数、结构化错误码、重试次数和最终 wire Tool bytes。
  - Capture method: 用同一构建在 Standard、map-always、map-append、map-request 四臂各运行一次
    `single-file-fast-fix`，逐 request 检查 function call 与 output。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
    - `task_context_event_recorded`
  - Correlation keys:
    - run root `/tmp/wc-r7-fla9`
    - policy arm `a0` 至 `a3`
  - Differentiates from:
    - 模型随机业务幻觉；
    - ActionMap 状态机拒绝正确输入；
    - projection policy 差异。
  - Supports if:
    - 三个 TaskSpace policy 均在初始化字段上发生多次 shape 错误。
  - Refutes if:
    - TaskSpace 首次 transition 一次成功，或错误仅出现在某个 policy。
  - Instrumentation status: captured
  - Instrumentation lifecycle:
    - 保留结构化参数失败与最终 Tool section 观测；删除判废实现。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed；通用 `arguments` schema 不能承担严格边界合同。该中间方案判废，不保留
  fallback。
- Repair design readiness: ready
- Next step: 把三个边界 action 以精确 schema 集中到唯一 `taskspace_control`，由 Tool sequence
  preflight 保证同响应后续实际动作。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-004: 紧凑 transition Docker 四臂自然样本
- Related hypotheses:
  - H-004
- Direction: supports
- Type: diagnostic-log
- Source: `/tmp/wc-r7-fla9/{a0,a1,a2,a3}/single-file-fast-fix/**`
- Prediction or plan link:
  - H-004：验证通用嵌套对象是否仍能让 Agent 一次生成正确边界动作。
- Matched signal:
  - Standard 首请求 Tool section：12 tools、21,669 bytes、约 5,418 estimated tokens。
  - 紧凑 transition TaskSpace：13 tools、28,711 bytes、约 7,178 estimated tokens。
  - 三个 TaskSpace policy 分别出现 3 至 5 次无效初始化尝试，包括把 `root` 作为字符串、把
    `finish_identity` 作为字符串，以及混淆 `node_id` 与 `id`。
- Correlation keys:
  - Standard run `20260723-222909-711`
  - map-always run `20260723-222952-622`
  - map-append run `20260723-223102-571`
  - map-request run `20260723-223213-052`
- Raw content:
  ```text
  compact nested transition:
    tools_count=13
    tools_bytes=28711
    estimated_tokens=7178
    taskspace initialization attempts before success=3..5
  ```
- Interpretation: 中间方案确实消除了大部分 schema 重复，但以隐藏精确合同为代价，直接制造了
  多次协议重试；这不是可接受的成本优化。
- Time: 2026-07-23 22:45

## Hypothesis H-005: 单个 Tool schema 不能结构化保证跨 Tool sibling
- Status: confirmed
- Parent: P-001
- Claim: 普通 Tool schema 零扩展后，`taskspace_control` 自身的 JSON Schema 只能约束本调用参数，
  不能保证 Agent 在同一 provider response 中继续声明另一个普通 Tool sibling；L1/L2 和 description
  可以说明协议，但不能提供结构保证。
- Layer: residual-design-boundary
- Factor relation: single
- Depends on:
  - H-003
  - H-004
- Rationale:
  - JSON Schema 的约束边界是一个 function 的 arguments，不覆盖同一 assistant response 中其他
    function call 的存在与顺序。
- Falsifiable predictions:
  - If true: 精确 control schema 会消除字段 shape 猜测，但自然样本仍可能先调用普通 Tool 或单独提交
    boundary control；后续看到明确失败后可以正确组合。
  - If false: L2 和 control schema 足以让三种 policy 从首响应稳定产生
    `initialize_map + ordinary action`。
- Diagnostic evidence plan:
  - Prediction or clause under test: 非侵入方案能否同时提供 bootstrap sibling 的结构保证。
  - Signal: 首三个 request 的工具序列、普通 gate、sequence preflight 与后续 handoff 序列。
  - Capture method: 在最终 control schema 上运行三个 TaskSpace policy；补强 L1/L2/description 后再次
    运行，逐 request 解析 rollout。
  - Event name or marker:
    - `taskspace.main_tool_call_rejected`
    - `tool.response_preflight_rejected`
  - Correlation keys:
    - `/tmp/wc-r7-fla9-v4/a1`
    - `/tmp/wc-r7-fla9-v4/a2`
    - `/tmp/wc-r7-fla9-v4/a3`
  - Differentiates from:
    - Tool 参数 shape 不明确；
    - 动态 `tool_choice` 或 tools hash 变化；
    - sequence executor 不支持多个调用。
  - Supports if:
    - 首轮仍出现普通 gate 或单独 boundary，但后续 handoff 能稳定同 response 组合。
  - Refutes if:
    - 三臂首响应都直接组合初始化和实际动作。
  - Instrumentation status: captured
  - Instrumentation lifecycle:
    - 保留 ordinary gate、boundary preflight 和 request path 观测。
- Evidence gate: satisfied
- Related evidence:
  - E-005
- Conclusion: confirmed；该缺口不能通过继续增加同义提示词获得结构保证。动态工具可见性、机械预初始化、
  control 内嵌动作或接受首轮硬门各有明确产品代价，需要单独决策。
- Repair design readiness: product-decision-required
- Next step: 用户确认 bootstrap 连续动作的目标优先级和可接受代价后，再设计单变量方案。
- Blocker:
  - bootstrap ownership / cache-shape / nested dispatch tradeoff
- Close reason:
  - not closed

## Evidence E-005: 最终去侵入方案与 bootstrap 复跑
- Related hypotheses:
  - H-003
  - H-005
- Direction: supports
- Type: natural-sample-and-wire-trace
- Source: `/tmp/wc-r7-fla9-v4/{a1,a2,a3}/single-file-fast-fix/**`
- Prediction or plan link:
  - H-003：普通 Tool 无 carrier 时仍经过 canonical state gate。
  - H-005：单 Tool schema 是否能保证首轮 sibling。
- Matched signal:
  - 当前 TaskSpace Tool section 为 13 tools、25,394 bytes、约 6,349 estimated tokens；
    Standard 为 12 tools、21,669 bytes、约 5,418 estimated tokens。
  - 三臂业务均通过，Map 均为 5 nodes、4 edges、0 open。
  - 三臂首响应都先提交普通命令并被 `no_task_path` 拒绝；随后各有一次单独
    `initialize_map` 被 `taskspace_boundary_action_requires_follow_up` 拒绝。
  - 后续所有 `complete_then_continue + patch/test` 均在同一 response 正确执行。
- Correlation keys:
  - map-always run `20260723-233104-251`
  - map-append run `20260723-233104-211`
  - map-request run `20260723-233104-211`
- Raw content:
  ```text
  Standard: tools=12 bytes=21669 estimated_tokens=5418 requests=6
  map-always: tools=13 bytes=25394 estimated_tokens=6349 requests=7
  map-append: tools=13 bytes=25394 estimated_tokens=6349 requests=8
  map-request: tools=13 bytes=25394 estimated_tokens=6349 requests=8
  ordinary_tool_schema_extension_count=0
  ```
- Interpretation: 普通 Tool 去侵入和固定成本修复成立，状态硬门没有弱化；但 bootstrap 的同响应连续动作
  仍不是单个 Tool schema 能保证的结构属性。
- Time: 2026-07-23 23:35
