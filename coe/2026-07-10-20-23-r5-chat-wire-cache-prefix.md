# Problem P-001: R5 TaskSpace 最终 Chat wire 前缀与缓存命中失去可审计性
- Status: open
- Created: 2026-07-10 20:23
- Updated: 2026-07-10 20:40
- Objective: 在不改变 provider message 内容、顺序或 Agent 决策的前提下，定位 R5 request-2+ 缓存命中偏低的最终 wire 首差异，并以最终 Chat body 与 provider usage 证明根因。
- Symptoms:
  - `subscription-billing-repair` R5 实跑总 input 501347、cached input 61184，累计命中约 12.2%。
  - 现有 `provider-cache-trace-summary.json` 报告 0 requests；48MB rollout 超过 32MB 后被整体跳过。
  - standard 没有逐请求 provider trace，无法与 TaskSpace 使用同一观测点对照。
- Expected behavior:
  - standard/TaskSpace 在 `build_chat_completions_body` 后使用同一无正文 trace，记录 message role/hash/bytes、tools hash、相邻 LCP、首差异路径和 request 级 cache usage。
  - 无 compaction 时，忠实自然历史应只追加；TaskSpace map 更新不得删除、替换或重排已经发送的 provider message。
- Actual behavior:
  - `provider_payload_digest_for_wire` 在调用 `stream_request` 前直接序列化 `ResponsesApiRequest`；DeepSeek Chat body 随后才在 `codex-api` 中构造。
  - 观测字段把 pre-wire `input` hash 命名为 `messages_hash`，把 pre-wire `instructions` hash 命名为 `stable_prefix_hash`。
  - provider budget trace 只在 TaskSpace snapshot 存在时进入 rollout，standard 无逐请求覆盖。
- Impact:
  - 当前数据不能证明最终 Chat message 的首差异位置，也不能区分 projection replacement、system/tools 变化和 provider best-effort 波动。
  - 缓存修复若基于 pre-wire 推断，可能修改错误层级并再次引入语义重写。
- Reproduction:
  - 检查 `core/src/client.rs::provider_payload_digest_for_wire` 与 `codex-api/src/endpoint/responses.rs::build_chat_completions_body` 的调用顺序。
  - 检查 `target/r5-e5-feedback-validation/subscription-billing-repair/20260710-201038-830/pair-001/right/artifacts`。
- Environment:
  - Linux/bash，branch `whalecode-alpha`，commit `88a3df8`，DeepSeek ChatCompletions，R5 G0。
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
- Ruled out:
  - 不能把低缓存继续归因于 E4 的多份 active projection 累积；E4 已保证单请求中 active projection 唯一。
  - 不能用 Responses pre-wire hash 证明最终 Chat wire 前缀关系。
- Fix criteria:
  - 最终 Chat wire 观测对 standard/TaskSpace 覆盖 100%，不记录正文。
  - 每个 request 可关联最终 body hash、message shapes、tools hash、相邻 LCP、首差异路径与 token usage。
  - G0 诊断不改变 message 内容、顺序、tool schema 或 provider 请求语义。
  - 根因经最终-wire 实跑证据确认后，才进入 G1 history 修复。
  - G1 后无 compaction 的相邻请求保持严格前缀，request-2+ cache hit 达到计划门禁且 correctness 不回退。
- Current conclusion: H-001 已由代码路径和实跑 artifact 确认；H-002/H-003 仍需 G0 最终-wire trace 区分，当前禁止直接修改 history。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 当前 exact payload 观测实际位于 Chat 转换前
- Status: confirmed
- Parent: P-001
- Claim: `provider_payload_digest_for_wire` 哈希的是 `ResponsesApiRequest` 而非最终 DeepSeek Chat body，因此现有 messages/stable-prefix 字段无法定位 wire 前缀断点。
- Layer: diagnostic
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - digest 调用发生在 `ApiResponsesClient::stream_request` 之前，Chat body 只在后者内部构造。
- Falsifiable predictions:
  - If true: digest 的 JSON 包含 `input`/`instructions`，而网络 body 包含 `messages`；现有 `messages_hash` 来自 `input`。
  - If false: digest 应接收 `build_chat_completions_body` 的最终 `Value`，且 standard/TaskSpace 都有同构记录。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对账请求构造调用顺序与 hash 字段来源。
  - Signal: digest 入参类型、最终 body 字段和 standard trace 覆盖。
  - Capture method: 静态代码路径审计与现有 artifact 检查。
  - Event name or marker:
    - `payload_captured`
  - Correlation keys:
    - provider request id
  - Differentiates from:
    - provider cache best-effort 波动。
  - Supports if:
    - digest 在 Chat 转换前运行且 standard 没有 provider budget event。
  - Refutes if:
    - 最终 Chat body 已被直接观测。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 由 G0 最终-wire 安全 trace 取代错误命名字段
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready for diagnostic-only G0 instrumentation; not ready for history repair
- Next step: 在最终 Chat body 构造后记录共享无正文 trace。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 每轮替换历史中段 projection 是最终 wire 首个断点
- Status: unverified
- Parent: P-001
- Claim: composer 删除上轮已发送的 active projection 并在末尾追加新 projection，使 request N 的 Chat message 序列不是 request N+1 的前缀。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - E4 latest-only 策略明确执行 `stale_active_projection_replaced`，理论上会改变历史中段位置。
- Falsifiable predictions:
  - If true: 相邻最终 Chat request 的首差异位于旧 active projection message，后续自然 assistant/tool history 发生位移。
  - If false: 首差异应更早出现在 system/tools，或 history 本身保持前缀。
- Diagnostic evidence plan:
  - Prediction or clause under test: 最终 Chat message shape 的相邻 LCP 与首差异 JSON path。
  - Signal: message index/role/hash/bytes、tools hash、first_diff_path、prefix_preserved。
  - Capture method: G0 shared post-conversion trace 加一次 paired sample。
  - Event name or marker:
    - `provider.chat_wire_prefix_broken`
  - Correlation keys:
    - epoch id
    - previous/current request id
  - Differentiates from:
    - H-003 system/tools 不稳定。
  - Supports if:
    - tools/system 稳定且首差异落在 active projection message。
  - Refutes if:
    - 首差异位于 projection 之前或相邻序列已是严格前缀。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - LCP/usage 聚合保留为永久缓存审计
- Evidence gate: pending
- Related evidence:
  - E-002
- Conclusion: unverified
- Repair design readiness: blocked until G0 runtime evidence
- Next step: 执行 G0 并采集同一样本相邻请求 trace。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: system、tools 或 action-contract 表面在相邻请求间变化
- Status: unverified
- Parent: P-001
- Claim: projection 位置之外的 system message、tool schema 或 action-contract transport 变化更早破坏最终 wire 前缀，动态 projection 不是主要断点。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - H-001
- Rationale:
  - 当前 pre-wire trace 无法证明 Chat 转换后的 system/tools 稳定；缓存 anchor 也可能被动态 instructions 布局包围。
- Falsifiable predictions:
  - If true: 相邻请求 tools hash 或 system message hash 变化，first_diff_path 位于 projection 之前。
  - If false: tools/system hash 稳定，首差异仅落在 active projection 位置。
- Diagnostic evidence plan:
  - Prediction or clause under test: 最终 Chat body 的 system/tools shape 稳定性。
  - Signal: message[0] hash、tools hash、first_diff_path。
  - Capture method: G0 shared post-conversion trace。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - epoch id
    - request id
  - Differentiates from:
    - H-002 projection replacement。
  - Supports if:
    - system/tools 在相邻请求间变化。
  - Refutes if:
    - system/tools 保持稳定。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 保留 hash，不保留正文
- Evidence gate: pending
- Related evidence:
  - E-001
- Conclusion: unverified
- Repair design readiness: blocked until G0 runtime evidence
- Next step: 执行 G0 shared final-wire trace。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: digest 与最终 Chat body 构造位于不同层级
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/client.rs:2255`、`third_party/codex-cli/codex-rs/codex-api/src/endpoint/responses.rs:130`
- Prediction or plan link:
  - H-001 If true
- Matched signal:
  - core 先对 `ResponsesApiRequest` 调用 `provider_payload_digest_for_wire`，随后 `stream_request` 才调用 `build_chat_completions_body`；digest 的 `messages_hash` 明确取 JSON `input` 字段。
- Correlation keys:
  - `provider_payload_digest_for_wire`
  - `build_chat_completions_body`
- Raw content:
  ```text
  provider_payload_digest_for_wire(&request, provider_wire_api)
  client.stream_request(request, options)
  let body = build_chat_completions_body(request)
  messages_hash: json_field_hash(&value, "input")
  ```
- Interpretation: 现有 exact 命名不成立，必须先移动到共享最终 wire 观测点。
- Time: 2026-07-10 20:23

## Evidence E-002: E5 复杂样本缓存命中仍低
- Related hypotheses:
  - H-002
  - H-003
- Direction: neutral
- Type: reproduction
- Source: `target/r5-e5-feedback-validation/subscription-billing-repair/20260710-201038-830/pair-001/right/artifacts/token-summary.json`
- Prediction or plan link:
  - P-001 Symptoms
- Matched signal:
  - 总 input 501347，cached input 61184；rollout 可观察到 31 个 logical request，但 token summary 只保留一次累计 usage。
- Correlation keys:
  - pair-001/right
- Raw content:
  ```text
  input_tokens=501347
  cached_input_tokens=61184
  rollout_bytes=48367921
  rollout_scan_mode=skipped_large_rollout
  ```
- Interpretation: 缓存问题真实存在，但累计 usage 不能区分 H-002/H-003。
- Time: 2026-07-10 20:23

## Evidence E-003: 现有逐请求 trace 对 standard 缺位且大 rollout 被跳过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: observation
- Source: E5 paired artifacts 与 `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- Prediction or plan link:
  - H-001 Supports if
- Matched signal:
  - TaskSpace provider events只经 action-map budget trace 导出；standard 无 action-map snapshot。超过 32MB 时 `rollout_effective_scan_path` 直接置空，TaskSpace 汇总也写成 0 requests。
- Correlation keys:
  - `provider-cache-trace-summary.json`
  - `taskspace-cost-scan-policy-v1`
- Raw content:
  ```text
  provider_request_count=0
  trace_coverage=0.0
  rollout_scan_mode=skipped_large_rollout
  ```
- Interpretation: G0 必须建立独立于 TaskSpace map 和 rollout 文件大小的共享 transport trace。
- Time: 2026-07-10 20:23

## Evidence E-004: G0 共享最终-wire 诊断已通过 focused 与 harness 测试
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: `core/src/provider_wire_trace.rs`、`codex-api/src/endpoint/responses.rs`、benchmark cost instrumentation tests
- Prediction or plan link:
  - P-001 Fix criteria
  - H-002/H-003 Diagnostic evidence plan
- Matched signal:
  - standard/TaskSpace 共用 `WHALE_PROVIDER_WIRE_TRACE_PATH`，trace 独立于 action-map budget 与 rollout 大小。
  - `build_chat_completions_body` 成为共享 builder；最终 Chat `messages/tools` 计算 message role/hash/bytes、相邻 LCP、首差异路径，pre-wire hash 单独命名。
  - request terminal 记录 provider 返回的 input/cached/output usage；artifact extractor 优先使用 final-wire trace。
  - trace schema 不含 message content 字段，未改变 request message 或排序。
- Correlation keys:
  - epoch id
  - provider wire request id
- Raw content:
  ```text
  codex-api Chat body tests: 5 passed
  provider_wire_trace comparison tests: 2 passed
  provider payload focused tests: passed
  cost instrumentation selftest: passed
  benchmark harness: PASS
  external wrapper harness: PASS
  whale build: passed
  ```
- Interpretation: H-001 的错误观测点已被可运行的诊断替代；H-002/H-003 仍须真实 paired sample，不得仅凭单测确认。
- Time: 2026-07-10 20:40
