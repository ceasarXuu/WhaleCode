# Problem P-001: R7 大型嵌套 patch control 参数畸形
- Status: open
- Created: 2026-07-19 19:30
- Updated: 2026-07-19 20:07
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
- Ruled out:
  - Runtime 在解析失败后部分推进 Map 状态。
  - Whale SSE 流式聚合重复追加 arguments 尾部。
  - 首次失败反馈在下一 provider request 中缺失、错配或被改写。
  - 单纯减少一层 `arguments` 包装即可解决问题。
- Fix criteria:
  - 证明畸形字符首次出现的层；证明第二次空参数是否收到完整失败反馈；对可复现根因实施单点修复，并在 simple/complex Docker 样本中验证无 correctness、request 或缓存负回归。
- Current conclusion: 根因已确认在 provider 生成层：DeepSeek V4 Flash 对“TaskSpace 多分支复合 function arguments 中嵌入大型多行 patch 字符串”的结构闭合和正文保真均不稳定；不是 patch 内容本身、SSE 拼接、Runtime parser 或反馈丢失。当前 Runtime 拒绝语义正确。仅扁平化字段可改善 JSON 合法率，但不能作为语义保真的生产修复；独立 `apply_patch` 是当前唯一同时通过结构与正文门禁的形态。
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
