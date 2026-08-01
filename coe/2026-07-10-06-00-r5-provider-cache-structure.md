# Problem P-001: R5 E4 provider cache prefix collapse
- Status: diagnosed
- Created: 2026-07-10 06:00
- Updated: 2026-07-10 07:02
- Objective: 在不预设 dynamic projection 是根因的前提下，定位 R5 E4 相对 standard 缓存命中从 83.8% 降至 9.7% 的实际 provider payload 结构断点。
- Symptoms:
  - `count-call-stack` 同轮 standard 6 次请求累计 cached input 53888/64295，R5 E4 9 次请求累计 cached input 9728/100365。
  - standard 第 2 次后单请求缓存命中约 91%-99.5%，R5 仅约 2%-14%。
- Expected behavior:
  - TaskSpace 额外动态状态不应破坏与 standard 共享的稳定 system/tool/history 前缀；实际变动范围应可被精确测量。
- Actual behavior:
  - 当前 trace 只有整个 messages hash、稳定前缀 hash 和 payload hash，不能指出实际序列化消息在哪个 index/role 首次变化。
- Impact:
  - R5 E4 uncached input 是 standard 的 8.71 倍，直接影响成本和 provider latency；未经证据不能选择 projection 重排或压缩方案。
- Reproduction:
  - 对比 `target/r5e4-projection-latest-only/count-call-stack/20260710-051931-572/pair-001` 两侧 retained rollout 和 R5 provider cache trace。
- Environment:
  - branch `whalecode-alpha`，commit `d699f88`，DeepSeek `deepseek-v4-flash`，ChatCompletions/native tools。
- Known facts:
  - R5 每请求 input 规模与 standard 接近，主要差异是 cache hit 和请求数。
  - R5 exact scan 已证明每个 provider payload 只有一份 active projection。
  - 当前所谓 exact scan 实际扫描 `ResponsesApiRequest`，不是 ChatCompletions 转换后的 wire body；它能证明 projection 唯一，不能证明 wire message 的首个前缀断点。
- Ruled out:
  - standard/R5 cached token 统计口径差异。
  - provider warm-up 或左右运行顺序是主要原因。
- Fix criteria:
  - 从 exact serialized request 证明首个公共前缀断点及其 owner；controlled rerun 排除 warm-up/偶然 cache 行为；修复需保持 projection 语义和工具反馈原文不变。
- Current conclusion: 已确认问题不是“projection 文本动态变化”这么简单，而是 latest-only replacement 破坏了 provider history 的单调追加结构。上一轮已发送的 projection 被从自然历史中删除，新 projection 被追加到本轮末尾，使本轮不再包含上一轮完整 input/output prefix；standard 则保持 append-only。该机制足以解释 provider 只能命中更早、更短的 prefix unit。当前 scanner 不是实际 Chat wire scan，因此首个 token 级断点和 tools schema 稳定性仍需 Phase G0 补测，不能把 H-001 扩大成唯一根因。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - H-001 通过 live rollout 顺序、history composer、Chat message converter 和 provider cache 行为形成直接因果链。
  - H-003/H-004 已由 controlled rerun 和同源 usage 审计排除。
- Close reason:
  - not closed

## Hypothesis H-001: dynamic projection item 是首个公共前缀断点
- Status: confirmed
- Parent: P-001
- Claim: latest projection 在序列化 ChatCompletions messages 中位于稳定自然历史之前或中间，其内容/位置变化导致后续大部分 messages 无法命中 prefix cache。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - R5 使用 latest-only projection，且 ChatCompletions 把 developer role 转为 system role；但当前没有 exact message-index 证据。
- Falsifiable predictions:
  - If true: 相邻请求 exact serialized messages 的首个差异 message 是 active projection，且后续本应稳定的历史位于它之后。
  - If false: 首个差异发生在 projection 之前，例如 base instructions、tools schema、环境消息或排序变化。
- Diagnostic evidence plan:
  - Prediction or clause under test: 相邻请求首个 message/token prefix 断点属于 active projection。
  - Signal: exact request 的 message index、role、content hash/bytes、tools schema hash、相邻请求最长公共前缀。
  - Capture method: 增加不记录正文的 diagnostic structural trace，并对同一样本 right-only 重跑。
  - Event name or marker:
    - TaskSpaceProviderPayloadStructureV1
  - Correlation keys:
    - request_id
    - provider_payload_sha256
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - first_changed_message 分类为 active_projection，且固定 system/tools 均在其前保持一致。
  - Refutes if:
    - first_changed_message 在 projection 之前或 projection 之后仍存在更早结构变化。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 根因确认后转为最小 common-prefix observability 或删除详细结构字段。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
  - E-004
  - E-005
- Conclusion: confirmed as a causal root mechanism, but not yet proven to be the only wire-level instability
- Repair design readiness: ready for cache-preserving history design; wire trace remains required before claiming complete closure
- Next step: Phase G0 在 `build_chat_completions_body` 之后记录脱敏 message-role/hash 序列和相邻请求 LCP；Phase G1 验证 append-only map delta/snapshot-at-compaction 方案。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: TaskSpace 请求结构在 projection 之前已经不同或不稳定
- Status: unverified
- Parent: P-001
- Claim: base instructions、system/developer message 组合、tools schema 或请求字段顺序在 R5 请求间发生变化，才是 cache prefix 提前失效的主因，projection 变化不是首个断点。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 当前 `stable_prefix_hash` 固定但只代表自定义摘要，不证明 provider 实际消费的完整前缀稳定。
- Falsifiable predictions:
  - If true: structural trace 在 projection 之前发现 message/tool/request-shape hash 变化。
  - If false: projection 之前所有序列化结构逐字节稳定。
- Diagnostic evidence plan:
  - Prediction or clause under test: projection 前存在更早的 serialized request 结构变化。
  - Signal: message layout hash、tools schema hash、base instruction hash、first changed JSON path。
  - Capture method: 与 H-001 共用 structural trace，按 provider 实际序列化顺序比较。
  - Event name or marker:
    - TaskSpaceProviderPayloadStructureV1
  - Correlation keys:
    - request_id
    - provider_payload_sha256
  - Differentiates from:
    - H-001
  - Supports if:
    - first changed path 位于 base/system/tools 或 projection 之前的 message。
  - Refutes if:
    - projection 之前全量结构稳定。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 同 H-001。
- Evidence gate: pending
- Related evidence:
  - E-001
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 与 H-001 同步验证。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: cache gap 主要来自 provider warm-up 或运行顺序
- Status: refuted
- Parent: P-001
- Claim: standard 先运行、R5 后运行或 provider cache 的短时波动造成单次样本命中差异，而不是稳定的请求结构缺陷。
- Layer: environment
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 当前只有一个 paired sample，cache 是 provider 外部状态。
- Falsifiable predictions:
  - If true: 同一 R5 right-only 连续重复时后一次 cache hit 显著回升，或差异不稳定。
  - If false: R5 重跑仍在相同 message 边界后持续低命中。
- Diagnostic evidence plan:
  - Prediction or clause under test: 重复同结构请求可改变 R5 cache hit 结论。
  - Signal: per-request cached/uncached input 与 structural trace。
  - Capture method: instrumentation 生效后同样本 right-only 连续运行两次。
  - Event name or marker:
    - TaskSpaceProviderCacheTraceV1
  - Correlation keys:
    - run_id
    - request_id
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - 第二次运行在结构不变时命中率显著接近 standard。
  - Refutes if:
    - 两次 R5 均稳定低命中且断点一致。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留现有 cache trace。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-004
- Conclusion: refuted；两次 TaskSpace 围绕一次 standard 运行仍保持低命中，standard 同时保持高命中
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: standard/R5 cache 统计口径不可比
- Status: refuted
- Parent: P-001
- Claim: standard 的 retained token_count 与 R5 provider lifecycle trace 对 cached tokens 的解析口径不同，造成表面 cache gap。
- Layer: diagnostic
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - benchmark 没有把 standard rollout/provider request telemetry 正式归档，当前 standard 数值来自 retained session rollout。
- Falsifiable predictions:
  - If true: 同一 response.completed usage 经两条 extractor 得到不同 cached token。
  - If false: 原始 provider usage 和两条 extractor 数值一致。
- Diagnostic evidence plan:
  - Prediction or clause under test: 两侧 cached token 源字段或算法不同。
  - Signal: raw token_count usage、provider terminal event、extractor source。
  - Capture method: 代码审计加标准 rollout 重提取，不改 provider 行为。
  - Event name or marker:
    - token_count
    - TaskSpaceProviderCacheTraceV1
  - Correlation keys:
    - session_id
    - request_id
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - 相同原始 usage 被不同算法解释。
  - Refutes if:
    - 两侧都直接使用 provider `cached_input_tokens` 且加总一致。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: refuted；两侧数值均直接来自 provider `TokenUsage.cached_input_tokens`，逐请求和累计值一致
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 单样本显示请求规模接近但 cache hit 巨大分叉
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Direction: supports
- Type: observation
- Source: `target/r5e4-projection-latest-only/count-call-stack/20260710-051931-572/pair-001` 与 standard retained rollout
- Prediction or plan link:
  - Problem symptoms
- Matched signal:
  - standard input/cached 64295/53888；R5 100365/9728；最后一轮 input 11967 vs 12297，单轮规模接近。
- Correlation keys:
  - standard session `019f48c0-0982-72f0-b4f7-6dbf548e22b4`
  - R5 request logical-1..9
- Raw content:
  ```text
  standard: requests=6 input=64295 cached=53888 uncached=10407
  R5 E4:   requests=9 input=100365 cached=9728 uncached=90637
  ```
- Interpretation: cache gap 真实存在，但该证据不能定位首个结构断点。
- Time: 2026-07-10 06:00

## Hypothesis H-005: 现有 exact payload telemetry 不是实际 Chat wire 结构
- Status: confirmed
- Parent: P-001
- Claim: `provider_payload_digest_for_wire` 在 ChatCompletions 转换前序列化 `ResponsesApiRequest`，因此 `messages_hash` 实际是 `input` hash，`stable_prefix_hash` 仅是 `instructions` hash，无法观察 `build_chat_completions_body` 生成的 messages/tools 最终布局。
- Layer: diagnostic
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - trace 标注 `provider_wire_api=ChatCompletions`，容易被误读为已扫描 wire body。
- Falsifiable predictions:
  - If true: digest 调用发生在 `ApiResponsesClient::stream_request` 之前，参数仍是 `ResponsesApiRequest`；Chat body 在 `codex-api` 内随后构造。
  - If false: telemetry 接收的是 `build_chat_completions_body` 的最终 `Value` 或序列化 bytes。
- Diagnostic evidence plan:
  - Prediction or clause under test: scanner owner 位于 wire conversion 前。
  - Signal: digest 调用点、参数类型和 Chat body builder 调用路径。
  - Capture method: 静态调用链审计。
  - Event name or marker:
    - taskspace-exact-payload-scan-event-v1
  - Correlation keys:
    - request_id
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - scanner 只看到 Responses request。
  - Refutes if:
    - scanner 看到最终 Chat JSON。
  - Instrumentation status: permanent observability defect
  - Instrumentation lifecycle:
    - Phase G0 将结构 trace 下移到 wire body 构造后，正文保持不可记录。
- Evidence gate: satisfied
- Related evidence:
  - E-006
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: Phase G0
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-002: 两侧 cache usage 使用同一 provider TokenUsage 字段
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: code-and-runtime-audit
- Source: `codex-api/src/sse/chat_completions.rs`、standard retained rollout、R5 rollout
- Prediction or plan link:
  - H-004 cached token source comparison
- Matched signal:
  - standard 与 R5 的 `token_count.info.last_token_usage.cached_input_tokens` 均来自 ChatCompletions usage；逐请求求和等于各自累计 `total_token_usage`。
- Correlation keys:
  - standard session `019f48c0-0982-72f0-b4f7-6dbf548e22b4`
  - R5 request logical-1..9
- Raw content:
  ```text
  standard sum: input=64295 cached=53888 uncached=10407
  R5 sum:       input=100365 cached=9728 uncached=90637
  ```
- Interpretation: gap 不是 extractor 口径制造的。
- Time: 2026-07-10 06:15

## Evidence E-003: DeepSeek cache 复用依赖从 token 0 开始的完整公共前缀
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: external-primary-source
- Source: `https://api-docs.deepseek.com/guides/kv_cache`
- Prediction or plan link:
  - H-001 prefix invalidation mechanism
- Matched signal:
  - provider 按公共前缀复用 cache，并在 user input/model output 边界持久化 cache unit；删除前轮中间项会让后续请求失去前轮完整边界。
- Correlation keys:
  - DeepSeek context caching
- Raw content:
  ```text
  Official behavior summarized: cache matching starts from token 0 and reuses an exact common prefix.
  ```
- Interpretation: 结构非单调比少量动态文本本身更能解释严重命中下降。
- Time: 2026-07-10 06:18

## Evidence E-004: controlled 三次 right-side 运行排除 warm-up/顺序主因
- Related hypotheses:
  - H-001
  - H-003
- Direction: refutes H-003 and supports structural cause
- Type: controlled-experiment
- Source: `target/r5-cache-control-three/count-call-stack/20260710-060705-568`
- Prediction or plan link:
  - H-003 repeated-run cache recovery prediction
- Matched signal:
  - TaskSpace、standard、TaskSpace 顺序运行；两次 TaskSpace hit 分别约 14.0% 和 9.9%，中间 standard 约 90.7%。
- Correlation keys:
  - pair-001 right TaskSpace
  - pair-002 right standard
  - pair-003 right TaskSpace
- Raw content:
  ```text
  TaskSpace #1: input=113111 cached=15872 uncached=97239
  standard:     input=117070 cached=106240 uncached=10830
  TaskSpace #2: input=129428 cached=12800 uncached=116628
  ```
- Interpretation: provider 已热身时 TaskSpace 仍低命中，不能归因于左右顺序或单次波动。
- Time: 2026-07-10 06:09

## Evidence E-005: 相邻请求首个 Chat message 差异由 stale projection 删除产生
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports H-001
- Type: rollout-and-code-path
- Source: E4 R5 rollout response item 顺序；`session/turn.rs::compose_provider_visible_history`；`codex-api/src/endpoint/chat_completions.rs`
- Prediction or plan link:
  - H-001 first changed message classification
- Matched signal:
  - E4 request 1 可见 items 为 permissions/environment/user/P1；request 2 删除 P1 后变成 permissions/environment/user/assistant-tool-history/P2。Chat converter 保序后，message index 4 从 `system(P1)` 变为 `assistant(tool_calls)`。
- Correlation keys:
  - R5 logical request 1
  - R5 logical request 2
- Raw content:
  ```text
  req1: H0 + P1
  req2: H0 + A1/T1 + P2
  standard req2: H0 + A1/T1 (append-only from req1 + output)
  ```
- Interpretation: R5 下一请求不包含上一请求的完整 input/output cache boundary；这属于 context layout 破坏，不是 projection 字段内容轻微变化。
- Time: 2026-07-10 06:45

## Evidence E-006: scanner 在 Chat wire conversion 前运行
- Related hypotheses:
  - H-002
  - H-005
- Direction: supports H-005; leaves H-002 partially unverified
- Type: code-path-audit
- Source: `core/src/client.rs:provider_payload_digest_for_wire`、`codex-api/src/endpoint/responses.rs:build_chat_completions_body`
- Prediction or plan link:
  - H-005 telemetry owner
- Matched signal:
  - core 先对 `ResponsesApiRequest` 调用 `serde_json::to_vec/to_value`，随后 `ApiResponsesClient` 才在 codex-api 内转换为 Chat messages/tools。
- Correlation keys:
  - provider_wire_api=ChatCompletions
- Raw content:
  ```text
  messages_hash = hash(value["input"])
  stable_prefix_hash = hash(value["instructions"])
  build_chat_completions_body(request) occurs downstream
  ```
- Interpretation: E4 uniqueness scan 有效，但“exact wire payload”命名过强；H-002 的实际 tools/message wire 稳定性尚未被当前 trace 直接证明。
- Time: 2026-07-10 06:52
