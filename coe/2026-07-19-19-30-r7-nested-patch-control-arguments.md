# Problem P-001: R7 大型嵌套 patch control 参数畸形
- Status: open
- Created: 2026-07-19 19:30
- Updated: 2026-07-22 00:16
- Objective: 证明复杂样本中 `complete_then_continue` 参数尾随字符和下一次空参数调用的真实产生层，并在不让 Runtime 猜测语义的前提下确定修复点。
- Symptoms:
  - `subscription-billing-repair` 首次大型 `patch_then_actions` control 参数末尾多一个 `}`，随后一次 `taskspace_control` 参数为 `{}`。
- Expected behavior:
  - provider 生成符合公开 Tool Schema 的完整 JSON；Runtime 忠实解析，合法时原子提交，非法时零提交并把明确错误返回 Agent。
- Actual behavior:
  - Runtime 先拒绝 trailing characters；Agent 随后直接 patch、测试和读取，之后又生成一次空 control；最终改用不含 patch 的合法 handoff 完成任务。
- Impact:
  - 本次增加 2 个 provider request，复杂嵌套 continuation 的稳定性未知；状态正确性未受损。
- Reproduction:
  - 使用 `26814f3f4` 构建的 whale，以 `map-request` 运行 Docker scenario `subscription-billing-repair` 一次。
- Environment:
  - Linux/Docker，branch `whalecode-alpha`，DeepSeek V4 Flash，artifact `target/r7-atomic-completion-v102/complex/subscription-billing-repair/20260719-185323-282`。
- Known facts:
  - rollout 保存的第一次参数在第 2128 列后有尾随 `}`；Runtime 返回 `protocol_failed/state_commit=false/partial_commit=0`。
  - 下一次 control 参数为 `{}`，Runtime 返回 missing field `action` 且零提交。
  - 第三次合法 `complete_then_continue` 与最终 `complete_then_end` 均提交，样本 solved，公开/隐藏验证通过。
  - 历史 54 次真实 `patch_then_actions` carrier 有 15 次 JSON 不可解析，其中 14 次为 trailing characters。
  - 非流式生产 schema probe 可复现 trailing characters，排除 Whale SSE assembler。
  - 同一大型 patch 直接调用顶层 `apply_patch` 为 6/6 JSON 合法且正文逐字节一致；当前 carrier 为 4/6 合法，减少一层包装为 5/6 合法。
  - continuation 直接携带 `patch_input` 和 control 顶层携带 `patch_input` 都达到 6/6 JSON 合法，但正文均为 0/6 逐字节一致；前者有 1 次只返回 3 字节。
  - 生产 `required_next_call` probe 为 6/6 `control -> apply_patch` 且 6/6 patch exact，证明修复没有拆分合并 request。
  - v1.0.4 simple/complex Docker 都 solved 且 Map 闭合，但各有 2 次首次 sibling 遗漏；字段改名不是该采用问题的充分修复。
  - FLA-3.5 carrier 修复计划经十五轮空白上下文审查后以 `ACCEPT` 收口；当前仍为 `selected_not_implemented`，不能视为修复完成。
  - CA-0 executable-v2 工具链已完成预锚定实现与本地回归，仍须空白上下文审查和独立 anchor 后才可进入 CA-1。
- Ruled out:
  - Runtime 在解析失败后部分推进 Map 状态。
  - Whale SSE 流式聚合重复追加 arguments 尾部。
  - 首次失败反馈在下一 provider request 中缺失、错配或被改写。
  - 单纯减少一层 `arguments` 包装即可解决问题。
- Fix criteria:
  - 证明畸形字符首次出现的层；证明第二次空参数是否收到完整失败反馈；对可复现根因实施单点修复，并在 simple/complex Docker 样本中验证无 correctness、request 或缓存负回归。
- Current conclusion: 原始大型 nested patch 根因已修复，但自然 coding 的 sibling 首次采用率仍未通过。FLA-3.5 已选定由普通 Tool 携带机械 `taskspace_transition` 的单 provider-call 方案，Runtime 不推断后续动作且复用唯一 Tool 执行链；方案审查已通过，生产修复仍须完成 CA-0 至 CA-6 并取得自然样本验证证据。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
  - H-008
  - H-009
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: Provider 已生成畸形的完整 arguments
- Status: confirmed
- Parent: P-001
- Claim: 多余 `}` 已存在于 DeepSeek 最终 tool-call arguments，Whale 仅忠实转发。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - rollout 的 function_call 已包含畸形字符串，但该 artifact 位于本地流式聚合之后。
- Falsifiable predictions:
  - If true: 原始 SSE arguments delta 按顺序拼接后与 rollout 字节一致，且没有本地重复追加。
  - If false: 原始 SSE 拼接合法，畸形只在本地转换或聚合后出现。
- Diagnostic evidence plan:
  - Prediction or clause under test: 畸形字符首次出现于 provider stream。
  - Signal: 原始 SSE delta、完成事件 arguments 与 rollout arguments 的逐字节 hash/长度/尾部。
  - Capture method: 检查现有 wire artifact；不足时增加只记录长度、hash 和 JSON parse 状态的诊断日志后重跑一次 complex。
  - Event name or marker:
    - provider.tool_arguments_completed
  - Correlation keys:
    - call_00_mOSPYNmgFnuv7M7Cb8Gr8310
  - Differentiates from:
    - H-002
  - Supports if:
    - provider 完成态 arguments 与 rollout 都在同一偏移出现尾随字符。
  - Refutes if:
    - provider 完成态合法而 rollout 畸形。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 若现有 artifact 不足则增加，确诊后删除原始内容记录，只保留安全的 hash/parse 观测。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: confirmed；非流式 Chat Completions 在完全绕过 SSE assembler 时仍生成同类 trailing characters。
- Repair design readiness: ready
- Next step: 修复应改变 provider-visible carrier，不得让 Runtime 容错删除尾随字符。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 本地 SSE 聚合重复追加了 arguments 尾部
- Status: refuted
- Parent: P-001
- Claim: Chat Completions SSE assembler 对 delta/index/finish 事件处理错误，导致合法 provider arguments 在本地多追加一个闭括号。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 当前 rollout 只证明解析入口收到畸形字符串，不能证明 provider 发送的原始分片也畸形。
- Falsifiable predictions:
  - If true: 给 assembler 输入捕获的 delta 可稳定重现多余尾部，或完成事件内容被重复并入增量。
  - If false: assembler 单测和真实分片拼接都逐字节保持 provider 数据。
- Diagnostic evidence plan:
  - Prediction or clause under test: 本地 assembler 改变了 arguments 字节序列。
  - Signal: 每个 delta 的 index/length/hash、聚合前后长度/hash、既有 assembler 测试。
  - Capture method: 审计 `codex-api/src/sse/chat_completions.rs` 并用捕获分片构造定向测试。
  - Event name or marker:
    - provider.tool_arguments_assembled
  - Correlation keys:
    - call_00_mOSPYNmgFnuv7M7Cb8Gr8310
  - Differentiates from:
    - H-001
  - Supports if:
    - 聚合结果比原始 delta 顺序拼接多出字节或重复消费完成态内容。
  - Refutes if:
    - 聚合结果与原始 delta 拼接逐字节一致。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 诊断 hash 可转为永久观测，禁止记录敏感完整参数。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: refuted；同类错误在 `stream=false` 的 provider 原始 message.tool_calls arguments 中复现，本地 assembler 未参与。
- Repair design readiness: not applicable
- Next step: 不修改 SSE assembler。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: TaskSpace 复合 carrier 嵌入大型 patch 降低结构生成稳定性
- Status: confirmed
- Parent: P-001
- Claim: 大型多行 patch 作为字符串嵌入 TaskSpace 多分支复合 function carrier 时，模型在 patch 结束后稳定出现 JSON 闭合错误；单独的大 patch function 或复合 carrier 中的短 patch 不发生该错误。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - H-001
- Rationale:
  - simple 的短 `actions` continuation 合法；complex 首次把四文件 patch 深层嵌入时畸形，缩小为普通 action 后合法。
- Falsifiable predictions:
  - If true: 当前/扁平大 patch carrier 的合法率都低于直接大 patch和当前短 patch；扁平化一层不能根治。
  - If false: 直接大 patch 同样失败，或短/大 carrier 的合法率无可辨差异。
- Diagnostic evidence plan:
  - Prediction or clause under test: 大 patch 与 TaskSpace 复合 envelope 的交互，而非 patch 本身或单层嵌套导致失败。
  - Signal: 同 patch 的 current carrier、flat carrier、direct apply_patch，以及短 patch current carrier 的 parse/shape valid rate。
  - Capture method: 先检索历史真实 run；证据不足时执行最小 provider schema probe，不执行 patch。
  - Event name or marker:
    - provider.tool_arguments_shape_probe
  - Correlation keys:
    - protocol v1.0.2
  - Differentiates from:
    - 单次随机生成错误、transport 聚合错误、输出截断。
  - Supports if:
    - 两种大型复合 carrier 都复现错误，direct large 和 current short 均保持合法。
  - Refutes if:
    - 三臂合法率相当或错误只出现在 transport 聚合层。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed；历史真实 carrier 15/54 不可解析；最新非流式 probe 中 current large=4/6、flat large=5/6、current short=6/6、direct large=6/6，且只有 direct large 6/6 patch 正文逐字节一致。
- Repair design readiness: ready
- Next step: 设计不把大型 patch 正文放入 TaskSpace 复合 arguments 的工具合同，先做 provider probe 再改 Runtime。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: 首次失败反馈没有完整进入下一请求
- Status: refuted
- Parent: P-001
- Claim: `{}` 重试是因为第一次 `invalid_arguments` 的错误语义在 provider context 中丢失、截断或错配 call_id。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - Agent 在收到明确错误后仍发出空 control，可能是反馈链路问题，也可能只是模型恢复失败。
- Falsifiable predictions:
  - If true: 下一 provider payload 缺少对应 call/result，call_id 错配，或错误正文与 Runtime 输出不一致。
  - If false: 下一 payload 包含完整、正确关联的失败结果，空调用只能归于 Agent 生成行为。
- Diagnostic evidence plan:
  - Prediction or clause under test: 下一请求是否完整携带首次失败反馈。
  - Signal: provider payload message shapes、tool call/result call_id、错误正文 hash 和上下文顺序。
  - Capture method: 从 rollout 与 provider cache trace 对账；必要时读取安全落盘的 final-wire payload。
  - Event name or marker:
    - taskspace_control function_call_output
  - Correlation keys:
    - call_00_mOSPYNmgFnuv7M7Cb8Gr8310
  - Differentiates from:
    - Agent 在完整反馈下仍生成错误动作。
  - Supports if:
    - 错误结果缺失、错配或被改写。
  - Refutes if:
    - 下一 request 中 call/result 和错误内容逐字节正确。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-004
- Conclusion: refuted；失败输出的 JSON-string SHA-256 与 request 7 message index 27 完全一致，并持续保留到 request 11；空 `{}` 出现在 request 10，其间 Agent 已直接 patch、测试和读取。
- Repair design readiness: not applicable
- Next step: 不修改 projection 或 feedback；空调用归为完整上下文下的 Agent 生成错误。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-005: 只扁平化 patch 字段即可形成生产修复
- Status: refuted
- Parent: P-001
- Claim: 把 patch 正文从 `continuation.patch.arguments.input` 移到 `continuation.patch_input` 或 control 顶层，就能同时修复 JSON 闭合和补丁语义保真。
- Layer: fix-validation
- Factor relation: any_of
- Depends on:
  - H-003
- Rationale:
  - 两种扁平形态都减少了长字符串结束后的结构闭合数量，可能消除 trailing characters。
- Falsifiable predictions:
  - If true: 扁平形态应同时达到 6/6 JSON/shape 合法和 6/6 patch hash 一致。
  - If false: 即使 JSON 合法，patch 正文仍发生截断、改写或占位符替换。
- Diagnostic evidence plan:
  - Prediction or clause under test: 结构合法率改善是否等价于 patch 语义保真。
  - Signal: JSON parse、expected shape、patch bytes/hash exact match。
  - Capture method: 非流式 provider 六臂 probe，不执行 patch，不持久化原始 arguments 或正文。
  - Event name or marker:
    - provider.tool_arguments_shape_probe
  - Correlation keys:
    - `continuation_patch_input_large`
    - `control_top_level_large`
  - Differentiates from:
    - 仅检查 JSON 合法率的假修复。
  - Supports if:
    - 两种扁平形态均 6/6 JSON 合法且 6/6 patch exact。
  - Refutes if:
    - 任一扁平形态出现 patch hash 不一致或明显截断。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - probe 保留为 provider 能力回归工具；不进入生产请求。
- Evidence gate: satisfied
- Related evidence:
  - E-005
- Conclusion: refuted；两种扁平形态虽均为 6/6 JSON 合法，但 patch exact 都是 0/6，`continuation.patch_input` 还出现一次 3 字节正文。
- Repair design readiness: blocked on sibling-tool probe
- Next step: 验证同一 provider response 中由小型 TaskSpace 状态 action 与独立 `apply_patch` sibling tool call 组成的序列；不得把扁平 carrier 直接投入生产。
- Blocker:
  - 尚未证明 sibling 调用顺序、patch exact、失败反馈和 request 数在真实 Runtime 中满足门禁。
- Close reason:
  - not closed

## Hypothesis H-006: 多工具同响应本身导致 patch 正文损坏
- Status: refuted
- Parent: P-001
- Claim: 只要同一 provider response 同时生成 lifecycle control 和 direct apply_patch，patch 正文就会失真；control schema 复杂度不是独立因素。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - H-003
- Rationale:
  - 首轮 sibling probe 的 patch exact 只有 2/6，可能由多工具生成本身导致。
- Falsifiable predictions:
  - If true: 最小 control schema 与 direct patch 的双调用仍显著低于 direct-only。
  - If false: 最小 control 双调用可达到与 direct-only 相同的顺序和正文保真。
- Diagnostic evidence plan:
  - Prediction or clause under test: 多调用与 schema 复杂度哪个因素决定正文保真。
  - Signal: direct-only、完整 control 可见、完整 control 双调用、最小 control 双调用的 call order 和 patch hash。
  - Capture method: 非流式 provider 消融，每臂 6 次。
  - Event name or marker:
    - r7.sibling_patch_sequence_observed
  - Correlation keys:
    - `sibling_minimal_control`
    - `direct_only`
  - Differentiates from:
    - H-003 中未分离的复合 schema 交互。
  - Supports if:
    - 最小 control 双调用仍出现正文失真。
  - Refutes if:
    - 最小 control 双调用和 direct-only 都达到 6/6 patch exact。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - probe 保留为 provider 能力回归工具，不进入生产请求。
- Evidence gate: satisfied
- Related evidence:
  - E-006
- Conclusion: refuted；direct-only 与最小 control 双调用均为 6/6 调用形状正确、6/6 patch exact。多工具同响应可以可靠工作，完整 TaskSpace schema 的认知负载才是独立风险因素。
- Repair design readiness: not applicable
- Next step: 修复应同时移除 patch 嵌套和普通工具 schema 的重复嵌套，不得只改执行顺序。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-007: 精简 control 可保留同 request 且避免正文改写
- Status: confirmed
- Parent: P-001
- Claim: 保留完整状态字段、删除 nested ordinary/patch payload，并用轻量 continuation 枚举声明顶层后续调用后，实际生成出的 direct patch 可保持正文完整；遗漏 sibling 的风险可由执行前机械 preflight 零执行拒绝。
- Layer: fix-validation
- Factor relation: all_of
- Depends on:
  - H-006
- Rationale:
  - 最小 control 证明双调用可行，但生产仍需要完整 DAG 状态参数。
- Falsifiable predictions:
  - If true: lean production-shaped control 下所有实际生成出的 patch 都 hash 一致，且 control 参数稳定合法。
  - If false: lean control 仍生成 patch 改写、截断或非法 JSON。
- Diagnostic evidence plan:
  - Prediction or clause under test: 删除 nested tool payload 是否足以让完整状态 schema 不再改写 patch。
  - Signal: control shape、调用顺序、patch JSON 与 hash；单独统计未生成 sibling 的响应。
  - Capture method: `sibling_lean_control` 非流式 6 次 probe。
  - Event name or marker:
    - r7.sibling_patch_sequence_observed
  - Correlation keys:
    - `sibling_lean_control`
  - Differentiates from:
    - 仅用最小 control 得出的不可生产结论。
  - Supports if:
    - 所有实际生成 patch 均 exact，control shape 全部合法；遗漏 sibling 单独暴露。
  - Refutes if:
    - 任一实际生成 patch 的 hash 不一致或 control 参数非法。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - provider probe 保留；生产新增 preflight/segment 结构化日志，不记录 patch 正文。
- Evidence gate: satisfied
- Related evidence:
  - E-006
- Conclusion: confirmed；lean control 6/6 参数合法，5/6 生成声明的 sibling patch，且生成出的 patch 5/5 exact。剩余 1/6 是调用遗漏，不是正文改写，必须在任何状态提交前被 preflight 拒绝。
- Repair design readiness: ready
- Next step: 实施 lean continuation + top-level patch barrier + full-response preflight，并用真实 Docker 样本验证遗漏率是否造成不可接受的 request 放大。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Complex rollout 中连续两次参数协议失败且零提交
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Direction: neutral
- Type: reproduction
- Source: `target/r7-atomic-completion-v102/complex/subscription-billing-repair/20260719-185323-282/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - 确认症状、调用顺序和 Runtime 状态提交语义；尚不区分畸形产生层。
- Matched signal:
  - 首次 arguments 第 2128 列后有 trailing character；下一次 arguments 为 `{}`；两次失败均零状态提交。
- Correlation keys:
  - `call_00_mOSPYNmgFnuv7M7Cb8Gr8310`
  - `call_00_7zpccrCZMGU53jt5c6Nf7085`
- Raw content:
  ```text
  invalid taskspace_control arguments at .: trailing characters at line 1 column 2128
  state_commit=false, partial_commit=0
  next arguments={}
  invalid taskspace_control arguments at .: missing field `action` at line 1 column 2
  state_commit=false, partial_commit=0
  third complete_then_continue committed revision=3
  complete_then_end committed revision=4
  ```
- Interpretation: 状态机和失败反馈输出本身工作正确；仍需定位畸形参数与空重试首次产生在哪一层。
- Time: 2026-07-19 19:30

## Evidence E-002: 54 次历史真实 patch carrier 失败分布
- Related hypotheses:
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: `target/**/rollout.jsonl` 中 43 份包含 `patch_then_actions` 的真实 TaskSpace rollout
- Prediction or plan link:
  - H-003 对大型复合 carrier 具有可重复失败率的预测。
- Matched signal:
  - 54 次 carrier 中 39 次合法、15 次不可解析；14/15 为 trailing characters，1/15 为缺少分隔符。
- Correlation keys:
  - `patch_then_actions`
- Raw content:
  ```text
  carrier count: 54
  valid: 39
  invalid: 15 (27.78%)
  trailing characters: 14
  expected comma or closing object: 1
  valid arguments bytes mean: 1159.92
  invalid arguments bytes mean: 2012.13
  invalid bytes range: 1726..2402
  maximum valid bytes: 2475
  ```
- Interpretation: 这是跨 R6/R7 多轮真实运行重复出现的 provider 参数问题，不是 `v1.0.2` 或单次采样偶发；长度增加显著相关但不是单一阈值。
- Time: 2026-07-19 19:35

## Evidence E-003: 非流式四臂生产 schema probe
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports H-001 and H-003; refutes H-002
- Type: provider-experiment
- Source: `scripts/taskspace-benchmark/probe-r7-nested-patch-control.ps1` 与 `target/r7-nested-patch-control-probe/20260719-114549-025/provider-capability.json`
- Prediction or plan link:
  - H-001 非流式仍复现；H-002 只有 SSE 聚合才复现；H-003 复合大 carrier 劣于 direct large/current short。
- Matched signal:
  - 24/24 HTTP 200 且各返回一个工具调用；stream=false，完全绕过 Whale SSE assembler。
  - current large 与 flat large 都有 2/6 trailing；current short 和 direct large 都为 6/6 JSON/shape 合法。
  - direct large 6/6 patch 正文逐字节一致；扁平化没有消除复合 carrier 问题。
- Correlation keys:
  - schema `r7-nested-patch-control-probe-v1`
  - production schema SHA-256 `f4aedeaf3ae905346f7910e883298142aa0e10e3fda7eadc7af792377328320b`
- Raw content:
  ```text
  arm             JSON valid   expected shape   trailing   patch exact
  current_large      4/6           4/6             2/6        0/6
  flat_large         4/6           4/6             2/6        1/6
  current_short      6/6           6/6             0/6        3/6
  direct_large       6/6           6/6             0/6        6/6

  transport: non_streaming_chat_completions
  raw arguments persisted: false
  patch content persisted: false
  API key persisted: false
  ```
- Interpretation: 大型 patch 字符串本身可以被 DeepSeek 可靠放入简单 function arguments；失败来自它与 TaskSpace 多分支复合 carrier 的交互。不能修 SSE，也不能仅减少一层对象。
- Time: 2026-07-19 19:48

## Evidence E-004: 失败反馈逐字节进入后续 provider context
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: diagnostic-log
- Source: complex rollout 与 `provider-wire-trace.jsonl` 的 call/result/hash/request 对账
- Prediction or plan link:
  - H-004 要求下一 request 中错误结果缺失、错配或改写。
- Matched signal:
  - Runtime 失败输出按 JSON string 编码后的 SHA-256 与 request 7 新增 tool message content hash 完全一致，并在 request 7-11 持续存在。
- Correlation keys:
  - call `call_00_mOSPYNmgFnuv7M7Cb8Gr8310`
  - request 7 message index 27
- Raw content:
  ```text
  runtime failure output JSON-string sha256:
    38ddd5bc7b09be3cd166665657333bfdf14a492a1c8b738862f69b578c8ad9c7
  request 7 message[27] content_sha256:
    38ddd5bc7b09be3cd166665657333bfdf14a492a1c8b738862f69b578c8ad9c7
  matching message count request 6..11:
    0, 1, 1, 1, 1, 1
  empty taskspace_control arguments occurred in request 10
  intervening requests executed direct patch, pytest, and file read
  ```
- Interpretation: feedback 层没有丢失、歧义或扭曲。空 `{}` 是 Agent 在完整错误历史下后续生成的另一次工具参数错误，不应通过 projection 注入或 Runtime 纠正处理。
- Time: 2026-07-19 19:38

## Evidence E-005: 六臂 probe 区分结构合法与正文保真
- Related hypotheses:
  - H-003
  - H-005
- Direction: supports H-003; refutes H-005
- Type: provider-experiment
- Source: `target/r7-nested-patch-control-probe/20260719-120146-077/provider-capability.json`
- Prediction or plan link:
  - H-005 要求扁平 patch 字段同时通过 JSON 和逐字节正文门禁。
- Matched signal:
  - 36/36 HTTP 200；所有请求使用 `stream=false`。
  - current large 为 4/6 JSON 合法，flat large 为 5/6；两者仍出现 trailing characters。
  - continuation 直接 `patch_input` 和 control 顶层 `patch_input` 都为 6/6 JSON 合法，但 patch exact 均为 0/6。
  - continuation 直接 `patch_input` 的返回长度为 `1455,1455,1455,3,1441,1455`，预期为 1471 字节。
  - direct `apply_patch` 为 6/6 JSON 合法且 6/6 patch exact。
- Correlation keys:
  - schema `r7-nested-patch-control-probe-v1`
  - production schema SHA-256 recorded per event
- Raw content:
  ```text
  arm                              JSON valid   trailing   patch exact
  current_large                       4/6          2/6         0/6
  flat_large                          5/6          1/6         0/6
  current_short                       6/6          0/6         0/6
  direct_large                        6/6          0/6         6/6
  continuation_patch_input_large      6/6          0/6         0/6
  control_top_level_large             6/6          0/6         0/6
  ```
- Interpretation: 参数可解析不等于 patch 语义忠实。当前可证明的修复边界是让 patch 保持独立工具参数形态；如何与状态交接保持同一 request，需要下一阶段单独验证。
- Time: 2026-07-19 20:05

## Evidence E-006: 同响应 sibling patch 的 schema 复杂度消融
- Related hypotheses:
  - H-006
  - H-007
- Direction: refutes H-006; supports H-007
- Type: provider-experiment
- Source: `scripts/taskspace-benchmark/probe-r7-sibling-patch-sequence.ps1` 与 `benchmarks/taskspace/r7/sibling-patch-sequence-probe-result.json`
- Prediction or plan link:
  - H-006 预测最小 control 双调用仍损坏 patch；H-007 预测 lean control 的已生成 patch 保持 exact。
- Matched signal:
  - direct-only 为 6/6 形状正确、6/6 patch exact。
  - 完整 control schema 可见但只调用 patch 为 6/6 调用正确、5/6 patch exact。
  - 完整 control 双调用为 5/6 顺序正确、2/6 patch exact。
  - patch-first 完整 control 双调用为 6/6 顺序正确、0/6 patch exact，调整顺序不能修复。
  - 最小 control 双调用为 6/6 顺序正确、6/6 patch exact。
  - lean production-shaped control 为 6/6 control 合法、5/6 双调用；实际生成 patch 5/5 exact，1/6 只生成 control。
- Correlation keys:
  - `direct_only`
  - `direct_with_control_visible`
  - `sibling_control_first`
  - `sibling_patch_first`
  - `sibling_minimal_control`
  - `sibling_lean_control`
- Raw content:
  ```text
  arm                           expected calls   patch JSON   patch exact
  direct_only                       6/6              6/6          6/6
  direct_with_control_visible       6/6              6/6          5/6
  sibling_control_first             5/6              5/6          2/6
  sibling_patch_first               6/6              6/6          0/6
  sibling_minimal_control           6/6              6/6          6/6
  sibling_lean_control              5/6              5/6          5/6
  ```
- Interpretation: 同 response 多工具不是根因。生产修复必须把 TaskSpace 从 nested tool orchestrator 收敛为状态工具，保留轻量 continuation 声明，并对遗漏的 sibling 在执行前机械拒绝；不能通过调整调用顺序或只移动 patch 字段解决。
- Time: 2026-07-19 20:51

## Hypothesis H-008: continuation 字段歧义是 sibling 遗漏的充分原因
- Status: refuted
- Parent: P-001
- Claim: 模型把 `continuation` 标量误解为“control 已经安排下一动作”是首次 sibling 遗漏的充分原因；改成只声明种类的 `required_next_call` 后，自然 coding 样本应不再遗漏。
- Layer: provider-contract
- Factor relation: any_of
- Depends on:
  - H-007
- Rationale:
  - v1.0.3 simple/complex 都在 control 单独返回后，经明确失败反馈立即纠正；字段名与历史 nested carrier 语义可能形成错误先验。
- Falsifiable predictions:
  - If true: 使用 `required_next_call` 的 v1.0.4 自然 simple/complex 中 required call violation 为 0。
  - If false: 字段和说明已明确“只声明、同响应 sibling”后，仍出现同类首次遗漏。
- Diagnostic evidence plan:
  - Prediction or clause under test: 字段语义改名是否足以消除自然采用失败。
  - Signal: 定向 provider probe 的调用形状；Docker trace 中 declaration、satisfied、violation 和下一请求恢复路径。
  - Capture method: 生产 schema 6 次非流式 probe；同 commit simple/complex 各一次 Standard/TaskSpace Docker pair。
  - Event name or marker:
    - r7.sibling_patch_sequence_observed
    - taskspace.response_required_next_call_validated
    - tool.response_preflight_rejected
  - Correlation keys:
    - protocol 1.0.4
    - implementation `12e7f8e3e`
  - Differentiates from:
    - H-009 只判断 patch fidelity 与合并 request 能力，不判断自然首次采用率。
  - Supports if:
    - 两个自然样本 violation 均为 0。
  - Refutes if:
    - 任一样本继续出现字段合法但 sibling 缺失。
  - Instrumentation status: permanent-observability
  - Instrumentation lifecycle:
    - observer 同时读取当前 `required_next_call` 和历史 artifact 的 `continuation`；产品 parser 不兼容旧字段。
- Evidence gate: satisfied
- Related evidence:
  - E-007
  - E-008
  - E-009
- Conclusion: refuted；定向 probe 从 v1.0.3 的 5/6 提升到 6/6，但自然 simple/complex 仍各有 2 次首次遗漏。字段歧义存在，但不是充分根因；单个 function schema 不能结构化约束另一个顶层 tool call。
- Repair design readiness: not ready
- Next step: 把首次 sibling 采用率作为独立 provider-visible 协议问题设计，不允许 Runtime 推断或补调用。
- Blocker:
  - 尚无经自然任务 probe 证明的低成本调用合同。
- Close reason:
  - hypothesis refuted

## Hypothesis H-009: 顶层原生 patch 可修复正文并保留合并 request
- Status: confirmed
- Parent: P-001
- Claim: 删除 nested tool payload 后，`taskspace_control`、direct `apply_patch` 和后续普通工具仍可由一个 provider response 承载，且 patch 正文和原生反馈不被 control 改写。
- Layer: fix-validation
- Factor relation: all_of
- Depends on:
  - H-007
- Rationale:
  - 用户要求不能用 patch fidelity 换取每个动作一个 provider request。
- Falsifiable predictions:
  - If true: 生产 probe 与 Docker 都观察到同响应 control + patch；patch exact；sequence 单测保持后续普通工具同响应执行。
  - If false: Runtime 或 provider 把 control、patch 强制拆成请求，或 patch 再次经 control carrier 改写。
- Diagnostic evidence plan:
  - Prediction or clause under test: 修复是否破坏合并 request 设计。
  - Signal: provider tool call 数组顺序、patch hash、response batch、barrier trace、原生 patch output。
  - Capture method: 生产 schema probe、sequence regression test、Docker rollout 逐 request 对账。
  - Event name or marker:
    - r7.sibling_patch_sequence_observed
    - taskspace.response_required_next_call_validated
    - tool.barrier_started
    - tool.barrier_completed
  - Correlation keys:
    - protocol 1.0.4
    - binary SHA-256 `d7f996618551d18aae9e66f6c208c39734e760d3e3af86d2b88c0a622456eea4`
  - Differentiates from:
    - H-008 的首次采用概率。
  - Supports if:
    - 6/6 probe 同响应且 exact，Docker 至少一次真实 control + patch 同响应成功。
  - Refutes if:
    - 任一已生成 patch 失真，或执行器强制新增 provider request。
  - Instrumentation status: permanent-observability
  - Instrumentation lifecycle:
    - 不记录 patch 正文，只记录 hash、长度、调用顺序和 reason code。
- Evidence gate: satisfied
- Related evidence:
  - E-008
  - E-009
- Conclusion: confirmed；生产 probe 6/6 同响应且 6/6 exact，simple/complex 都成功执行同响应 `control -> patch`，sequence 回归还覆盖同响应后续普通工具。修复没有破坏合并 request。
- Repair design readiness: implemented
- Next step: 保持该边界，后续采用率优化不得恢复 nested carrier 或 Runtime 自动补调用。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-007: v1.0.3 生产实现保真但自然样本各有两次遗漏
- Related hypotheses:
  - H-007
  - H-008
- Direction: supports H-007; supports H-008 candidate
- Type: fix-validation
- Source: `benchmarks/taskspace/r7/working-protocol-v1.0.3-result.json`
- Prediction or plan link:
  - 分开观察已生成 patch 的正文保真与声明后 sibling 的自然采用率。
- Matched signal:
  - provider probe 5/6 生成预期调用，已生成 patch 5/5 exact。
  - simple、complex 都 solved，分别为 5 次声明 3 次满足、8 次声明 6 次满足，各有 2 次 violation。
  - complex 三次重试 patch 的 SHA-256 完全相同，失败反馈未导致 patch 语义漂移。
- Correlation keys:
  - implementation `839832c5f`
  - protocol 1.0.3
- Raw content:
  ```text
  simple: 5 declarations, 3 satisfied, 2 violations
  complex: 8 declarations, 6 satisfied, 2 violations
  emitted provider patches: exact
  ```
- Interpretation: 顶层 patch 修复有效，但 `continuation` 字段的首次 sibling 采用不足，触发稳定的请求重试。
- Time: 2026-07-19 21:25

## Evidence E-008: required_next_call 定向生产 schema probe 为 6/6
- Related hypotheses:
  - H-008
  - H-009
- Direction: supports
- Type: provider-experiment
- Source: `benchmarks/taskspace/r7/sibling-required-next-call-production-result.json`
- Prediction or plan link:
  - 验证新字段下 provider 是否具备同响应双调用和大型 patch exact 能力。
- Matched signal:
  - 6/6 HTTP 200、6/6 `taskspace_control -> apply_patch`、6/6 control shape、6/6 patch JSON、6/6 patch exact。
  - 第 2 次及以后每次缓存 4096/4128 input tokens。
- Correlation keys:
  - arm `sibling_control_first`
  - protocol 1.0.4 schema
- Raw content:
  ```text
  expected_call_names_match=6/6
  patch_exact=6/6
  ```
- Interpretation: 合并 request 能力和 patch 保真已确认；该 probe 明确要求 exactly two tools，不能单独证明自然采用率。
- Time: 2026-07-19 21:43

## Evidence E-009: v1.0.4 Docker 验证保留合并 request 但未消除首次遗漏
- Related hypotheses:
  - H-008
  - H-009
- Direction: refutes H-008; supports H-009
- Type: fix-validation
- Source: `benchmarks/taskspace/r7/working-protocol-v1.0.4-result.json`
- Prediction or plan link:
  - H-008 预测自然样本 violation 为 0；H-009 预测真实 control + patch 同响应且任务正确。
- Matched signal:
  - simple/complex 的 Standard 与 TaskSpace 均 solved，公开/隐藏验证通过；两个 TaskSpace Map 都闭合。
  - simple 为 5 次声明、3 次满足、2 次 violation；complex 为 6/4/2。
  - 每次 violation 后 Agent 都明确读取到“declaration does not execute or schedule”反馈，并在下一请求正确合并 sibling。
  - simple 与 complex 均实际成功执行同响应 `complete_then_continue + apply_patch`；patch 使用原生输出。
  - complex 的 25 请求还包含错误 bind、patch 上下文失败、三 patch preflight 和过早 terminal，不能全部归因于字段改名。
- Correlation keys:
  - simple `20260719-214716-333`
  - complex `20260719-214822-447`
  - binary SHA-256 `d7f996618551d18aae9e66f6c208c39734e760d3e3af86d2b88c0a622456eea4`
- Raw content:
  ```text
  simple Standard/TaskSpace requests: 6/11
  complex Standard/TaskSpace requests: 11/25
  TaskSpace req2+ cache: 96.44% / 97.88%
  same-shape zero: 0 / 0
  ```
- Interpretation: feedback 忠实且合并 request 设计保留；新字段不是首次采用问题的充分修复，P-001 的 request 效率退出门仍未满足。
- Time: 2026-07-19 22:05

## Evidence E-010: FLA-3.5 carrier 修复计划通过空白上下文审查
- Related hypotheses:
  - H-008
  - H-009
- Direction: supports repair design; does not satisfy fix validation
- Type: design-review
- Source: `docs/v0.0.5/build-R7/33-r7-continuous-action-regression-repair-plan.md`、`vs_review/2026-07-21-r7-continuous-action-repair-plan-review.md`
- Prediction or plan link:
  - 首次采用问题必须由一个 provider-visible Tool schema 内的连续动作合同解决，不能由 Runtime 自动补 sibling。
- Matched signal:
  - 第十五轮审查为 `ACCEPT`，Blocking/High/Medium/Low 均为 none，Phase conflict 为 PASS。
  - 计划要求 ordinary Tool 携带 transition、复用现有 router/permission/sandbox/hook/MCP/handler/result pipeline。
  - 计划状态仍为 `selected_not_implemented`，没有把 scaffold 或 baseline 门禁当成生产完成。
- Correlation keys:
  - target commit `2165b5065`
  - review launch `019f8337-0ce1-78d3-84eb-ef83e071048c`
- Interpretation: repair design gate 已满足；Problem 仍保持 open，直到 CA-3 实现和 CA-5 自然样本验证通过。
- Time: 2026-07-21 23:40

## Evidence E-011: CA-0 启动前 projection ownership 门禁复现遗漏项
- Related hypotheses:
  - H-009
- Direction: neutral to root cause; blocks clean implementation baseline
- Type: regression-test
- Source: `scripts/taskspace-benchmark/test-r7-projection-policy-contract.ps1`
- Prediction or plan link:
  - CA-0 应从全套确定性门禁通过的干净基线开始。
- Matched signal:
  - 测试稳定失败并指出 `scripts/taskspace-benchmark/sync-r7-five-layer-contract-manifest.ps1` 未被 ownership inventory 覆盖。
  - 该脚本只生成共享五层 manifest，包含 projection 状态枚举文本但不拥有 projection Runtime。
- Correlation keys:
  - baseline commit `48ca37597`
  - missing inventory path `scripts/taskspace-benchmark/sync-r7-five-layer-contract-manifest.ps1`
- Interpretation: 应登记为独立五层 manifest generation gate；不得把它误归为 projection 语义实现。
- Time: 2026-07-21 23:45

## Evidence E-012: projection ownership inventory 恢复完整覆盖
- Related hypotheses:
  - H-009
- Direction: neutral to root cause; satisfies clean implementation baseline gate
- Type: fix-validation
- Source: `benchmarks/taskspace/r7/phase-a-ownership-inventory.json`
- Prediction or plan link:
  - 把后续新增的五层 generator、production manifest 和 machine gate 按真实职责登记后，marker coverage 与五层合同应同时通过。
- Matched signal:
  - `test-r7-projection-policy-contract.ps1` PASS，34 个 inventory item 覆盖 25 个 marker 文件。
  - `test-r7-five-layer-contracts.ps1 -Phase All` 同次 PASS。
  - 新增项均为 `retain_shared` 的五层生成/门禁职责，没有归入 projection Runtime。
- Correlation keys:
  - inventory `r7-phase-a-projection-policy-ownership`
  - paths `sync-r7-five-layer-contract-manifest.ps1`、`test-r7-five-layer-contracts.ps1`、production manifest
- Interpretation: CA-0 可以从确定性 ownership 与 active baseline 门禁均通过的状态开始。
- Time: 2026-07-21 23:52

## Evidence E-013: CA-0 executable-v2 工具链通过预锚定回归
- Related hypotheses:
  - H-009
- Direction: supports implementation readiness; does not satisfy production repair
- Type: fix-validation
- Source: `scripts/taskspace-benchmark/test-r7-continuous-action-toolchain.ps1`、Rust closure generator、required-check workflow
- Prediction or plan link:
  - CA-0 必须先形成可执行 schema、严格 parser、候选 generator/transition/verifier、源码生成 closure、completion bootstrap 和固定评估合同，再单独审查并锚定。
- Matched signal:
  - 预锚定自测通过 8 个 artifact role、302 条 closure entry、3 个严格 JSON 负例和 8 个 PowerShell 脚本语法检查。
  - closure 由编译后的四类 registry profile 与 Rust AST source binding 生成，覆盖 33 个 `ToolHandlerKind`、两种 provider wire，并对 relevant source inventory 绑定 path/hash。
  - `codex-tools` tool-registry 定向测试 41 passed、1 ignored；`codex-core` sequence 定向测试 16 passed。
  - `test-r7-five-layer-contracts.ps1 -Phase All`、FLA-3.5 scaffold、projection ownership contract 均通过。
  - workflow 由 Docker image `rhysd/actionlint@sha256:b1934ee5f1c509618f2508e6eb47ee0d3520686341fec936f3b79331f9315667` 验证通过。
  - 首次全 bin 测试发现 `src/bin` 辅助模块被误识别为独立 binary；已迁入专属模块目录并由同一全量命令验证修复。
- Correlation keys:
  - toolchain self-test `target/r7-toolchain/self-test/toolchain-test-result.json`
  - closure entries `302`
  - ownership items/marker files `35/27`
- Interpretation: CA-0 代码实现已具备提交和独立审查条件；active authority/production 未修改，工具链 anchor 尚未创建，因此 FLA-3.5 仍未完成。
- Time: 2026-07-22 00:16

## Evidence E-014: CA-0 artifact 与冻结评测合同可由原始事实独立重算
- Related hypotheses:
  - H-009
- Direction: supports implementation readiness; does not satisfy production repair
- Type: fix-validation
- Source: scripts/taskspace-benchmark/test-r7-continuous-action-toolchain.ps1、test-r7-continuous-action-evaluator.ps1
- Prediction or plan link:
  - Round 1 finding 4、8、9 要求 artifact 不再是字段外形，评测结论不能由 candidate 自签，也不得借用 FLA-8 held-out 样本。
- Matched signal:
  - L4 schema、transition、typed outcome 与 carrier oracle 均携带实际 instance/value/trace，并由 verifier 重算 hash、schema decision 和 payload exactness。
  - 冻结评测合同绑定三个 ca0_dev_only 非 held-out 样本的 scenario、完整目录、prompt、fixture 目录和 oracle 身份。
  - pinned evaluator 忽略 summary 布尔值，从 requests/tools/map/verdict/cache 事件重算 36 个 run、12 个 pair，并执行固定种子 10,000 次分层 bootstrap 与 Holm 校正。
  - duplicate、missing、artifact hash drift、cache unavailable、held-out 五类负例均被拒绝。
  - 总工具链回归通过：462 closure entries、6 strict JSON negatives、5 evaluator negatives、17 scripts/libraries。
- Correlation keys:
  - evaluation id r7-fla3-5-continuous-action-v1
  - contract digest 635e1bab4f2d9f7c4ef571281ae0ba3344a5ab297d341ab991dde383e21da4de
  - fixture directory identity cb6d38348f592908e0e3c6bf897ecb0213198bb5519a210e24e7cb65c2ed4a55
- Raw content:
  ```text
  evaluator: passed=true runs=36 negative_cases=5
  toolchain: passed=true closure_entries=462 strict_negative_cases=6 scripts_parsed=17
  ```
- Interpretation: Round 1 的 hollow artifact、self-reported evaluation 和 held-out 污染路径已被机械关闭；completion required-check 仍需单独绑定 evaluator 结果与 GitHub run identity。
- Time: 2026-07-22 01:50

## Evidence E-015: CA-0 全生命周期在独立 clone 中完成 pinned replay 与合法 revert
- Related hypotheses:
  - H-009
- Direction: supports implementation readiness; does not satisfy production repair
- Type: fix-validation
- Source: scripts/taskspace-benchmark/test-r7-continuous-action-integration.ps1
- Prediction or plan link:
  - Round 1 finding 7、8、10 要求 prospective 原子发布、completion 原始事实重算、GitHub run identity 和合法 revert 路径形成端到端证据。
- Matched signal:
  - 独立 clone 从 source commit 4c6876d5e4b2ed7921817189e007afd1b846a1a2 创建临时 toolchain anchor，随后完成 candidate 生成、promotion_pending、promoted、reverted 全状态链。
  - promotion completion 从 anchor 父提交导出 pinned launcher/verifier/schema/evaluator，重算 36-run evaluation result，并在第二个临时 clone 中真实执行 promoted 到 reverted rollback drill。
  - 正式 revert commit 再由同一 pinned launcher 独立验证，authority 与 production 恢复 baseline，required-check 不再把合法 revert 判失败。
  - promotion/revert attestation 分别绑定 repository、workflow ref/SHA、event、run id、run attempt、target commit 和完整导出 artifact hash。
  - 黑盒过程中先后暴露并修复 Git 单行输出展开、directory manifest 排序、staged delete 漏计和工具链固定输入漏 pin；修复后从头重跑通过。
- Correlation keys:
  - candidate a1c273de9382c8b334581dfd0327bbfad08e7a3b58f32e442f0d8f4a7a5549bb
  - synthetic anchor d4719ff5be42862f0d7ca4ce741e8d6bdc9d05d7
  - promoted 544cdaff320ffb3ca575e3683c9bc643dfbc6451
  - reverted cf32729b2118f3d0e1600b74396dea7861df0903
  - promotion attestation 7e486d9b29cda4d8d7ff40c9d2d899b7c8cfbda152da659c1649fee79700f12e
  - revert attestation 38baba7a9754226e60cff8b1a07cc0100abd197a2fcd2b6a44dd467c49e99038
- Raw content:
  ```text
  r7_continuous_action_integration passed=true
  promotion attestation verified=true event_kind=promotion
  revert attestation verified=true event_kind=revert
  ```
- Interpretation: 原子 candidate 状态链、pinned completion 重算和合法 revert required-check 已具备可重放黑盒证据；正式 anchor 仍取决于 Round 2 空白上下文审查。
- Time: 2026-07-22 02:25

## Evidence E-016: Round 2 否定 CA-0 自认证证据链
- Related hypotheses:
  - H-009
- Direction: contradicts E-014/E-015 对 production correctness 的解释；保留其 Git 生命周期证据
- Type: adversarial-review
- Source: `vs_review/2026-07-22-r7-ca0-toolchain-review.md`
- Prediction or plan link:
  - 正式 anchor 只能锚定能够独立验证后续 candidate production code 的工具链，不能锚定 candidate 自己提供的 verdict、fixture 或路径。
- Matched signal:
  - Round 2 以 `REJECT` 识别 8 个 blocking finding，其中 R2-1 至 R2-4 均指向“生成者定义并证明自身”。
  - 四类 executable artifact 的 verifier 只重算 schema、hash 和标签，没有执行 production parser、transition、outcome 或 oracle。
  - closure 只记录 binding source 子集，profile 与 DeepSeek rows 存在手工合成；rollback 未覆盖真实 add/modify/delete/mode。
  - shared index、successor authority、anchor path、transitive environment 和 GitHub workflow identity 另有边界缺口。
- Correlation keys:
  - review session `019f85f0-915f-71d0-bcbb-d9ffa793c2a0`
  - review target `54165032e`
  - verdict `REJECT`
- Interpretation: E-014/E-015 只能证明 artifact/Git 生命周期的内部一致性，不能证明 carrier production correctness。正式 anchor 继续禁止创建。
- Time: 2026-07-22 02:50

## Evidence E-017: R2-3 收敛后 R2-2 暴露前实现工具链矛盾
- Related hypotheses:
  - H-009
- Direction: narrows remaining blocker; requires architecture decision
- Type: root-cause-analysis
- Source: Rust production source、R7 closure generator、固定容器 lifecycle
- Prediction or plan link:
  - R2-3 应由 production config 与真实 provider mapper 生成完整 closure；R2-2 executor 必须在 anchor 父提交固定，并在后续 candidate commit 上执行真实生产路径。
- Matched signal:
  - commit `918a49897` 将 source inventory 扩展为全部 production Rust 文件，并用新增/修改/删除回归证明 digest fail-closed。
  - commit `dd2c8af4f` 使用 `ModelInfo + Features + ToolsConfig::new` 生成 7 个 production profile，并通过 `build_chat_completions_body` 的真实 DeepSeek mapper 生成 466 条双 wire entry；487 个 Rust source、48 个关键 binding 进入 inventory。
  - 3 个 Rust 专项测试与 PreAnchor 工具链通过；R2-3 的手工 profile/mapper 缺口已关闭。
  - 全仓搜索确认当前 production source 不存在 `taskspace_transition`、`TaskSpaceCarrierOutcome` 或 carrier executor；现有 L4/transition/outcome/oracle 由 PowerShell 手工构造，描述的是 anchor 之后才会实现的 candidate 行为。
  - 因而在当前提交上增加“读取 fixture 后再输出相同结构”的 Rust binary 仍是自认证；此前 synthetic candidate lifecycle PASS 只能证明 Git 状态机，不可升级为 production executor 证据。
  - digest-pinned Rust 容器的冷构建同时因 Cargo Git dependency `crossterm` 拉取超时失败，说明执行环境还需要 locked fetch + offline replay 或等价依赖供给协议；该次运行不得记为 PASS。
- Correlation keys:
  - R2-3 commits `918a49897`, `dd2c8af4f`
  - closure entries/source/bindings `466/487/48`
  - failed container `aba3d65dfe20`
  - failure `R7_TEST_CLOSURE_GENERATION_FAILED` / locked crossterm fetch timeout
- Interpretation: R2-2 的正确目标不是把合成 fixture 换一种语言重放，而是建立 pre-anchor 固定、可对 post-anchor candidate 使用的 production black-box runner。下一步必须在“完整 Agent mock-provider 黑盒 runner”与“预先冻结 production probe 接口”之间选择；前者架构更忠实、工作量更大，后者需要提前引入未激活的 carrier probe contract。
- Time: 2026-07-22 03:49
