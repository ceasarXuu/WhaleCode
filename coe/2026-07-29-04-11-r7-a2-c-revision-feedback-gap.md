# Problem P-001: 普通 Tool 结果提交后的 canonical revision 未进入 Agent 反馈
- Status: open
- Created: 2026-07-29 04:11
- Updated: 2026-08-01
- Objective: 让一次合法 TaskSpace response 完成后，Agent 忠实获得下一次控制操作所需的最终 canonical revision
- Symptoms:
  - `taskspace_control` 成功返回 `revision_after=N`
  - 同 response 的每个普通 Tool 结果释放 reservation 时继续提交 Map，使实际 revision 变成 `N+K`
  - 普通 Tool 输出只保留原生结果，不携带最终 revision；下一次 `execute/finish_map` 稳定提交旧 revision
  - A2-C 51 个状态失败 request 中 36 个是 `stale_revision`
- Expected behavior:
  - 原生普通 Tool 结果保持完整、无扭曲
  - Runtime 自己产生的 Map 状态变化有对应的机械事实反馈
  - 下一 provider request 能看到完整 response 执行后的唯一 canonical revision
- Actual behavior:
  - control result 只描述 reservation prepare 后的中间 revision
  - result attribution 的后续 revision 只进入 Store/日志，没有进入 provider-visible feedback
- Impact:
  - 每次成功的工作 response 很容易紧跟一次 stale reject 和一次纠正重试
  - request、input、output 和 wall time 被结构性放大
  - Agent 被误判为不能管理 revision，实际是反馈缺失
- Reproduction:
  - 检查 A2-C 任一成功 `initialize_and_execute/execute + ordinary sibling` response
  - 对照 control output、普通 Tool outputs、下一次 submitted revision 和 Store revision
- Environment:
  - source commit `abe2b872b6708e666293d0018ecd3654bf5a65cc`
  - run root `target/r7-five-layer-matrix/a2-c/abe2b872b/20260729-0315`
- Known facts:
  - `prepare_response_for_main` 先提交 reservations 并生成 `TaskSpaceResponseCommitV1`
  - `release_main_action_result` 对每个 ordinary result 再提交一次 canonical Map
  - `record_taskspace_bound_tool_result` 返回 `Result<(), String>`，不把最终 revision 返回 sequence
  - ordinary Tool 的 model-visible output 不包含 TaskSpace revision
- Ruled out:
  - Agent 忽略普通 Tool 输出中的 revision；输出中不存在该字段
  - 单一 projection policy；三种 policy 都复现，且一个 user turn 内 projection 不随每次 provider loop 重建
  - Store revision 未推进；事务测试和导出 Map 均证明 result attribution 会推进 revision
- Fix criteria:
  - control prepare revision 与 response-final canonical revision 语义明确分离
  - ordinary Tool 原生输出字节和语义保持不变
  - response 完成后 Agent 可直接获得最终 canonical revision，无需先触发 stale reject
  - 三种 projection policy 共用同一反馈实现
  - A2-C repeat-3 中由隐藏 result attribution 引起的 stale retry 为零
- Current conclusion: 根因修复已进入当前生产路径。prepare revision 只保留为内部结算事实；原
  `taskspace_control` call 在 sibling attribution 全部结束后只返回一个 `TaskSpaceResponseResultV2`，其中
  `canonical_revision` 是唯一 continuation revision。独立 developer/system receipt 已删除，普通 Tool result
  保持原生。确定性测试和免费 final-wire 检查已通过预期影响面验证；真实 Agent stale 消失与真实缓存接受仍待
  分别授权，因此问题保持 open/verifying
- Related hypotheses:
  - H-001
- Resolution basis:
  - Store-backed `TaskSpaceResponseFinalReceiptV1` 工程修复已实施
  - prepare revision、普通 Tool 输出和 response-final revision 保持三个独立事实
  - 定向事务与 sequence 回归通过
  - A2-C live rerun 证明独立 developer/system receipt carrier 确定性破坏缓存，产品修复失败
  - R8 I01 W0-W8 将最终事实收敛回原 control Tool result，并完成三 policy、Standard、普通 Tool 和 observer 的
    离线验证；见 E-008
- Close reason:
  - not closed

## Hypothesis H-001: control output 暴露了中间 revision，result attribution 的最终 revision 未暴露
- Status: confirmed
- Parent: P-001
- Claim: sequence 在 sibling 执行前构造 control output，而 sibling result release 继续修改 canonical Map；最终 revision 没有返回 model-visible result
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - stale submitted revision 等于前一 control output，canonical revision 等于该 response 全部结果提交后的 revision
- Falsifiable predictions:
  - If true: 同 response 中 control `revision_after=N`，K 个 sibling 完成后 Store revision 至少为 `N+K`，下一请求仍提交 N
  - If false: 普通 Tool feedback 已携带最终 revision，或 result release 不推进 Map
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐一次完整 response 的 control、siblings、Store 和下一 request
  - Signal: revision_before/revision_after、reservation count、result release commits、submitted_expected_revision
  - Capture method: raw rollout + canonical Store export + production call graph
  - Event name or marker:
    - `TaskSpaceResponseCommitV1`
    - `taskspace_native_tool_result_attributed`
    - `taskspace_response_preflight_rejected`
  - Correlation keys:
    - control call_id
    - reservation_id
    - map_id
  - Differentiates from:
    - Agent 自行选择错误 revision
  - Supports if:
    - 最终 revision 只存在于 Store/日志
  - Refutes if:
    - provider-visible feedback 已提供最终 revision
  - Instrumentation status: existing-observability-sufficient
  - Instrumentation lifecycle:
    - 修复后保留 response-final revision 和 attribution count
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: production call graph 和 live trace 一致确认
- Repair design readiness: implemented
- Next step: 保留 Store-backed final revision，重构为同一 control call 的最终 Tool result，再重跑 A2-C
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 成功初始化后的下一控制请求使用了不可避免的旧 revision
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: A2-C `single-file-fast-fix / map-always / repeat 1`
- Prediction or plan link:
  - H-001 的完整 response revision 预测
- Matched signal:
  - control 返回 revision 1，一个 ordinary result 后 canonical revision 为 2；下一 execute 提交 1 并被拒绝
- Correlation keys:
  - map `map-019faa26-f0f3-7732-abf2-68615563a309`
- Raw content:
  ```text
initialize_and_execute: revision_before=0, revision_after=1
ordinary result: native output only
next execute: expected_revision=1
reject: current_revision=2, violation=stale_revision
  ```
- Interpretation: Agent 使用了它最后被明确告知的 revision
- Time: 2026-07-29 04:11

## Evidence E-002: production result attribution 推进 Map 但不返回 revision
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source:
  - `core/src/tools/sequence.rs`
  - `core/src/tools/parallel.rs`
  - `core/src/session/taskspace_response.rs`
  - `core/src/action_map/runtime/transactions.rs`
- Prediction or plan link:
  - H-001 的 call graph 预测
- Matched signal:
  - control output 在 sibling dispatch 前由 `prepared.model_visible_result()` 构造
  - 每个 sibling 通过 `release_main_action_result` 提交 result ref
  - record API 仅返回 `Result<(), String>`
- Correlation keys:
  - `ActionMapPreparedResponse.revision_after`
  - `release_action_reservation`
- Raw content:
  ```text
outputs = [control_output]
handle_taskspace_bound_tool_call_for_sequence(...)
record_taskspace_bound_tool_result(...) -> Result<(), String>
release_reservation(expected_revision = current map revision)
  ```
- Interpretation: Runtime 产生了 Agent 不可见的后续 canonical revision
- Time: 2026-07-29 04:11

## Evidence E-003: response-final canonical revision 已作为独立事实返回
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source:
  - `core/src/action_map/response.rs`
  - `core/src/action_map/runtime/transactions.rs`
  - `core/src/tools/sequence.rs`
  - `core/src/tools/sequence_taskspace_tests.rs`
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - sibling result attribution 全部结束后从 persistent Map Store 读取 canonical revision
  - developer factual message 追加 `TaskSpaceResponseFinalReceiptV1`
  - 下一次 execute 使用回执 revision 可直接提交
  - 普通 Tool 原生 output 不被包装或改写
- Correlation keys:
  - map_id
  - control_call_id
  - reservation_revision_after
  - canonical_revision
- Raw content:
  ```text
tools::sequence::taskspace_tests: 5 passed
response_final_receipt_revision_is_accepted_by_the_next_execute: passed
taskspace_response_final_receipt_emitted
codex-core --lib with repository .env.local: 1915 passed, 3 ignored
  ```
- Interpretation: 反馈缺失已在工程路径补齐，但本证据只验证值可用，没有验证 carrier 对 Provider 缓存和 wire
  角色的影响；后续 live evidence 已证明当前 carrier 不合格
- Time: 2026-07-29 04:56

## Hypothesis H-002: 独立 developer receipt 在 DeepSeek wire 上变成中途 system 消息并破坏缓存
- Status: confirmed
- Parent: P-001
- Claim: sequence 在 Tool siblings 后追加 developer factual message；DeepSeek wire 将其表示为历史中途的 system
  message，导致下一请求只复用基础缓存前缀
- Layer: repair-regression
- Factor relation: dependent_on
- Depends on:
  - H-001
- Rationale:
  - 修复前 `map-append` 保持约 95% request-2+ cache；修复后 exact message prefix 仍保持，但每个 receipt 后缓存
    稳定退回约 7K 以下
- Falsifiable predictions:
  - If true: 紧跟 receipt 的请求稳定低缓存，不紧跟 receipt 的同一 run 请求恢复高缓存
  - If false: 缓存低点与 receipt 无时间关联，或 Tool hash/tool choice/message prefix 同时变化
- Diagnostic evidence plan:
  - Prediction or clause under test: 在保持 projection policy、Tool hash、tool choice 和消息前缀不变时，对齐 receipt
    与下一 token_count
  - Signal: receipt-before、input、cached input、wire role、prefix preservation
  - Capture method: A2-C repair rerun raw rollout + provider wire trace
  - Event name or marker:
    - `TaskSpaceResponseFinalReceiptV1`
    - `token_count`
    - `provider_wire_request`
  - Correlation keys:
    - run path
    - request index
    - control_call_id
  - Differentiates from:
    - `map-append` projection 累加的正常 input 成本
    - Tool schema/tool_choice 动态变化
    - 单次 Provider 冷启动
  - Supports if:
    - receipt-before 请求系统性低缓存且无 receipt 请求高缓存
  - Refutes if:
    - 两组命中无显著路径差异
  - Instrumentation status: existing-raw-evidence-sufficient / aggregate-report-gap
  - Instrumentation lifecycle:
    - 后续矩阵报告永久增加 receipt-before cache 分组
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-005
- Conclusion: 35/35 与 1/39 的分组结果及 wire role 共同确认
- Repair design readiness: ready
- Next step: 设计同一 control call 的最终 Tool result carrier；实施前保持五层职责和普通 Tool 零侵入
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-004: map-append receipt-before 与缓存坍缩完全对齐
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: A2-C repair rerun，全部 6 个 `map-append` runs
- Prediction or plan link:
  - H-002 的 receipt 时间关联预测
- Matched signal:
  - receipt-before：35 requests，35 个 cached input <= 7,000，命中率 23.64%
  - no-receipt-before：39 requests，仅 1 个 cached input <= 7,000，命中率 93.69%
- Correlation keys:
  - run root `target/r7-five-layer-matrix/a2-c-repair/445499582/20260729-0546`
- Raw content:
  ```text
receipt_before=false requests=39 collapse=1 input=860042 cached=805760 hit=93.69%
receipt_before=true  requests=35 collapse=35 input=901614 cached=213120 hit=23.64%
  ```
- Interpretation: 同一 policy、同一 binary、同一矩阵内的请求级对照排除了普通 projection 累加和随机离群
- Time: 2026-07-29 06:20

## Hypothesis H-003: final revision 已可见，但中间 revision 仍是更强的竞争事实
- Status: confirmed
- Parent: P-001
- Claim: 独立 receipt 没有替换 control prepare 的 `revision_after`；Agent 面对两个 revision 时稳定沿用前者，
  因此原始“缺失”转化为“歧义”
- Layer: feedback-semantics
- Factor relation: dependent_on
- Depends on:
  - H-001
- Falsifiable predictions:
  - If true: 最新 stale 的 submitted revision 等于前一 control `revision_after`，同时历史中存在更高的 receipt
    `canonical_revision`
  - If false: stale 前没有 final receipt，或 Agent 已提交 receipt revision
- Evidence gate: satisfied
- Related evidence:
  - E-006
- Conclusion: 14 个 response stale 和 3 个 finish stale 全部符合预测，且只出现在 map-always
- Repair design readiness: ready
- Next step: 延迟构造 control Tool result，完整 sibling attribution 后一次性返回
  `reservation_revision_after` 与唯一 `canonical_revision/next_expected_revision`
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-006: 17 次 stale 全部选择 prepare revision 而非 final revision
- Related hypotheses:
  - H-003
- Direction: supports
- Type: reproduction
- Source: A2-C repair rerun raw rollout
- Prediction or plan link:
  - H-003 revision 竞争预测
- Matched signal:
  - 14 个 response commit stale
  - 3 个 `finish_map` stale
  - submitted revision 均等于前一 control `revision_after`
  - 同一历史的 receipt/projection 已包含更高 canonical revision
- Correlation keys:
  - run root `target/r7-five-layer-matrix/a2-c-repair/445499582/20260729-0546`
  - control call ID
  - request index
- Raw content:
  ```text
control revision_after=3
final receipt canonical_revision=5
current projection revision=5
next execute expected_revision=3
reject current_revision=5
  ```
- Interpretation: final revision 不再缺失，但反馈没有形成唯一权威语义；单纯“再追加一条更晚消息”不足以修复
- Time: 2026-07-29 19:18

## Evidence E-005: wire 保持前缀和 Tool 身份，但 receipt 作为 system 历史出现
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: A2-C repair rerun provider wire trace
- Prediction or plan link:
  - H-002 的 role 与 prefix 判别
- Matched signal:
  - `prefix_preserved=true`
  - `message_prefix_preserved=true`
  - `tool_choice_changed=false`
  - `tools_hash` 不变
  - `TaskSpaceResponseFinalReceiptV1` 对应 message role 为 `system`
- Correlation keys:
  - `single-file-fast-fix / map-append / repeat 1`
  - receipt control call IDs
- Raw content:
  ```text
request 4: prefix_preserved=true, cached=5888
message[14].role=system, content=TaskSpaceResponseFinalReceiptV1
request 5 without new receipt: cached=15744
request 6 after new receipt: cached=5888
  ```
- Interpretation: 缓存回归来自 receipt carrier，而不是 Tool schema 或自然历史前缀重写
- Time: 2026-07-29 06:20

## Evidence E-007: R8 当前 HEAD 确定性复现两个 Agent-visible continuation revision
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: reproduction
- Source:
  - source commit `90389c9f9`
  - `core/src/tools/sequence_taskspace_tests.rs`
- Prediction or plan link:
  - R8 I01-W0 当前版本 characterization
  - H-001 的 prepare 与 final revision 分离预测
  - H-003 的两个成功 revision 竞争预测
- Matched signal:
  - 原 `taskspace_control` FunctionCallOutput 使用 schema `TaskSpaceResponseCommitV1` 并返回
    `revision_after=1`
  - 同一 output vector 末尾的 developer message 使用 schema `TaskSpaceResponseFinalReceiptV1` 并返回
    `canonical_revision=2`
  - 两个结果都表示成功，且 final revision 大于 prepare revision
- Capture method:
  - 当前生产 `execute_response_tool_sequence()` 的确定性集成 fixture
- Correlation keys:
  - control call ID `control`
  - ordinary call ID `initial-work`
- Raw content:
  ```text
  cargo test -p codex-core \
    current_response_exposes_prepare_and_final_revision_as_distinct_authorities \
    -- --nocapture

  result: 1 passed; 0 failed
  prepare schema=TaskSpaceResponseCommitV1 revision_after=1
  final schema=TaskSpaceResponseFinalReceiptV1 canonical_revision=2
  Agent-visible continuation revision count=2
  ```
- Interpretation: 当前 HEAD 仍在一次 response 中暴露两个可竞争的成功 revision；问题不是仅存在于 R7.1
  历史 trace，也不是 projection policy 自身产生。
- Time: 2026-08-01

## Evidence E-008: R8 I01 W0-W8 收敛为原 control call 的唯一最终结果
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: repair-verification
- Source:
  - commits `3fbfbe6dc`、`ae36f0cbe`、`dbce3402e`、`d46b19479`、`9e64a3ddc`
  - commits `ad117ce24`、`cb91900c3`、`d2be70030`、`cec426afd`
  - `docs/v0.0.5/build-R8/I01/00-i01-response-final-revision-repair-plan.md`
- Prediction or plan link:
  - R8 I01-W1 至 I01-W8
  - H-003 的“同一 control call 唯一最终 revision”修复预测
- Matched signal:
  - sequence 只生成一个与原 control `call_id` 配对的 `TaskSpaceResponseResultV2`
  - 成功结果只暴露 `canonical_revision`，不暴露 `revision_after`
  - 独立 `TaskSpaceResponseFinalReceiptV1` developer message 从当前生产路径删除
  - 三种 projection policy 使用同一 feedback 路径
  - Standard、普通 Tool schema/result、instructions、tools 和 tool_choice 未发生变化
  - 免费 final-wire matrix 只有三个 TaskSpace 场景 changed；其余 7 个场景 unchanged，0 个 uncomparable
- Capture method:
  - Rust response、sequence、transaction 和 provider-wire 定向测试
  - PowerShell 当前 benchmark analyzer 合同
  - clean-HEAD 免费 final-wire 比较
- Correlation keys:
  - original control call ID
  - map ID
  - final canonical revision
- Raw content:
  ```text
  TaskSpace policies changed: 3/3
  Standard and other protected scenarios unchanged: 7/7
  first difference: /request_2/input/length
  old: prepare control output + developer final receipt
  new: one finalized control output
  ```
- Interpretation: 当前实现已消除已确认的双 revision 和独立 receipt carrier。该证据是确定性的工程验证，不能替代
  W9 的真实 Agent stale 复验，也不能自行晋升 W10 的真实缓存 accepted baseline。
- Time: 2026-08-01

## Evidence E-009: R8 map-always 单次真实运行未再出现 revision 竞争
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: repair-verification
- Source:
  - ledger `WAR-20260801-222316-R8-I01-W9-MA-1B64DB37`
  - `docs/v0.0.5/build-R8/I01/02-i01-w9-map-always-repeat1-result.md`
  - durable evidence `target/r8-i01-w9/WAR-20260801-222316-R8-I01-W9-MA-1B64DB37/map-always-r1`
- Prediction or plan link:
  - I01-W9：真实 Agent 不再因隐藏 result attribution 使用旧 revision
- Matched signal:
  - 5 次成功 control response 均只有一个与原 control call 配对的 `TaskSpaceResponseResultV2`
  - 唯一 continuation revision 链为 `2 -> 4 -> 6 -> 8 -> 10`
  - 下一次成功提交均使用前一最终 `canonical_revision`
  - `stale_revision` 为 0，旧 `TaskSpaceResponseFinalReceiptV1` 为 0
- Correlation keys:
  - mode `map-always`
  - sample `single-file-fast-fix`
  - product commit `9b49f6dc96ad553ab454fefc2c96c975a6838442`
- Raw content:
  ```text
  canonical revisions: 2, 4, 6, 8, 10
  stale_revision: 0
  TaskSpaceResponseFinalReceiptV1: 0
  public validation: passed
  hidden oracle: passed
  agent lifecycle: interrupted before finish_map
  ```
- Interpretation: I01 的双 revision 根因在 map-always 当前单次真实路径中未复现。运行整体失败来自既有 I03
  组合动作拒绝耗尽请求预算，不能据此反驳 I01 修复，也不能据此把 W9 或 I01 标记完成。
- Time: 2026-08-01
