# Problem P-001: I07 请求、用量和 Provider 边界事实被混同
- Status: open
- Created: 2026-08-05 07:59
- Updated: 2026-08-05 07:59
- Objective: 让性能观察从唯一身份忠实还原 logical request、local attempt、boundary request、completed response 和 usage
- Symptoms:
  - 8 个 completed Provider responses 被 rollout summary 统计为 15 个请求并近似双计 token
  - 10 个 supervisor boundary requests 与 11 个本地 payload attempts 被判为 upstream mismatch
- Expected behavior:
  - no-ID TokenCount 只作为状态快照，不产生请求或 usage 记录
  - 本地 attempt、实际 boundary request 和 completed response 分开计数并按身份关联
  - 缺失、冲突或过期证据明确不可比较
- Actual behavior:
  - rollout summary 按带 `last_token_usage` 的事件条数聚合
  - boundary verifier 要求全部 `payload_captured` 与 boundary claims 列表严格相等
- Impact:
  - request、token、cache 和成本报告不可复算，可能误导版本与根因判断
- Reproduction:
  - `pwsh -File scripts/taskspace-benchmark/test-i07-characterization.ps1`
- Environment:
  - Linux；branch `whalecode-alpha`；起始 commit `6e3d78a3e`
- Known facts:
  - response-completed TokenCount 带完整 request identity；rate-limit snapshot 无身份但重复携带 last usage
  - ProviderWireTrace 在 `client.stream_request()` 前记录 `payload_captured`
  - Provider supervisor 的 boundary claims 是实际受监督边界证据
- Ruled out:
  - Provider 实际发送了 15 个请求；boundary/final-wire 只证明 8 个 completed requests
  - 必须删除 no-ID TokenCount；该事件仍服务 UI/rate-limit 状态
- Fix criteria:
  - 8/15 fixture 离线重放为 8 completed/usage 和 7 snapshots
  - 10/11 fixture报告 10 boundary、11 attempts、1 local-only failed attempt
  - 所有请求数消费者使用同一规范化事实，身份冲突 fail closed
  - TaskSpace Exec 接入前完成 I07-W0～W8，不运行真实 Whale Agent
- Current conclusion: 两个根因均已由源码和真实 trace 交叉确认，进入已授权修复
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: no-ID 状态快照被按请求完成事件消费
- Status: confirmed
- Parent: P-001
- Claim: `New-TaskspaceRolloutRequestTraceSummary` 忽略 Provider identity，把 rate-limit snapshot 重复携带的 last usage 当成新请求
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 8/15 差值恰好等于前 7 个请求的 paired no-ID snapshots
- Falsifiable predictions:
  - If true: 脱敏 8-request fixture 会被旧 analyzer 计为 15，按完整 identity 聚合后为 8
  - If false: 排除 no-ID snapshots 后仍不是 8，或 snapshots 携带另一请求 identity
- Diagnostic evidence plan:
  - Prediction or clause under test: 旧 analyzer 对真实事件形态稳定复现 8/15
  - Signal: `model_request_count` 与 TokenCount identity 分布
  - Capture method: 脱敏 fixture 调用当前 `New-TaskspaceRolloutRequestTraceSummary`
  - Event name or marker:
    - `token_count`
  - Correlation keys:
    - `provider_request_id`
    - `provider_logical_request_id`
    - `provider_attempt_seq`
  - Differentiates from:
    - Provider 实际发出额外请求
  - Supports if:
    - 8 个有身份 completed usage + 7 个无身份 snapshots 被计为 15
  - Refutes if:
    - 当前 analyzer 已按身份去重或 fixture 不复现
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 将 snapshot 排除数保留为 observer 自诊断
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: confirmed by source and real trace
- Repair design readiness: ready
- Next step: I07-W2/W3
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: payload capture 被误解释为 boundary-accepted request
- Status: confirmed
- Parent: P-001
- Claim: Provider wire tracer 在 transport 调用前记录 local attempt，而 verifier 将全部 attempts 与 supervisor boundary claims 强制一一相等
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 10/11 run 的额外 payload attempt 以 response_failed 结束，supervisor 没有对应 claim
- Falsifiable predictions:
  - If true: 旧 verifier 对 10 claims + 11 attempts 报 mismatch；第 11 attempt 不在 boundary 且未 completed
  - If false: record_request 发生在 transport 接收后，或 boundary 中存在第 11 个 claim
- Diagnostic evidence plan:
  - Prediction or clause under test: 对照 record_request/stream_request 顺序并重放 10/11 事件关系
  - Signal: 源码调用顺序、payload digest 集合、terminal status
  - Capture method: 读取 `client.rs` 和 `provider_wire_trace.rs`；运行脱敏 verifier fixture
  - Event name or marker:
    - `provider.chat_wire_request_terminal`
    - `provider_request_claimed`
  - Correlation keys:
    - `request_id`
    - `provider_payload_sha256`
  - Differentiates from:
    - 上游存在未被 supervisor 记录的成功请求
  - Supports if:
    - 第 11 个 attempt 位于 boundary 集合之外且 terminal=response_failed
  - Refutes if:
    - 第 11 个 digest 存在于 boundary 或 terminal=response_completed
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 attempt/boundary/completion 三阶段计数
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-004
- Conclusion: confirmed by source and real trace
- Repair design readiness: ready
- Next step: I07-W2/W4
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 真实 rollout 的 8 个完成事件与 7 个无身份快照
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002/.../rollout.jsonl`
- Prediction or plan link:
  - H-001 的 paired event 数量预测
- Matched signal:
  - 8 full-identity TokenCount + 7 no-ID TokenCount with repeated last usage
- Correlation keys:
  - run `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002`
- Raw content:
  ```text
completed identity events = 8
no-id last-usage snapshots = 7
legacy rollout request summary = 15
  ```
- Interpretation: 15 来自事件语义混淆，不是 Provider 请求放大
- Time: 2026-08-05 07:59

## Evidence E-002: provider wire 在 transport 调用前记录 payload
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/client.rs:2903-2919`
- Prediction or plan link:
  - H-002 的调用顺序预测
- Matched signal:
  - budget admission -> `record_request()` -> `client.stream_request()`
- Correlation keys:
  - logical request id
  - attempt sequence
- Raw content:
  ```text
provider_request_budget.before_dispatch(...)
provider_wire_trace.record_request(...)
client.stream_request(request, options).await
  ```
- Interpretation: `payload_captured` 是 local attempt 证据，不足以证明 boundary accepted
- Time: 2026-08-05 07:59

## Evidence E-003: W0 usage characterization fixture 复现旧错误
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `scripts/taskspace-benchmark/test-i07-characterization.ps1`
- Prediction or plan link:
  - H-001 的旧 analyzer 复现预测
- Matched signal:
  - legacy analyzer returned `model_request_count=15` for 8 identified completions
- Correlation keys:
  - `usage-double-count-rollout.jsonl`
- Raw content:
  ```text
  I07 characterization: PASS (legacy 8/15 and 10/11 defects reproduced)
  usage model_request_count = 15
  ```
- Interpretation: 脱敏最小输入独立复现同一双计机制，H-001 证据门保持 satisfied
- Time: 2026-08-05 07:59

## Evidence E-004: W0 boundary characterization fixture 复现旧错误
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: `scripts/taskspace-benchmark/test-i07-characterization.ps1`
- Prediction or plan link:
  - H-002 的 10/11 mismatch 预测
- Matched signal:
  - verifier returned exit 3 with boundary=10、wire attempts=11 and `provider_dispatch_trace_mismatch`
- Correlation keys:
  - `attempt-boundary-events.jsonl`
  - `attempt-boundary-wire.jsonl`
- Raw content:
  ```text
  I07 characterization: PASS (legacy 8/15 and 10/11 defects reproduced)
  boundary_request_count = 10
  wire_request_count = 11
  errors = [provider_dispatch_trace_mismatch]
  ```
- Interpretation: 脱敏最小输入独立复现 attempt/boundary 阶段混淆，H-002 证据门保持 satisfied
- Time: 2026-08-05 07:59

## Evidence E-005: W1 消费面 inventory 门禁
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/check-request-fact-consumers.py`
- Prediction or plan link:
  - I07-W1 全量消费面分类
- Matched signal:
  - 10 个生产 reader/producer 均登记其原始来源和目标事实
  - 新增未登记 reader 的负例被拒绝
  - 测试 support 不触发误报
- Correlation keys:
  - `whalecode-request-fact-consumers-v1`
- Raw content:
  ```text
request fact consumer gate: PASS
Ran 3 tests ... OK
  ```
- Interpretation: 后续 W2-W7 的迁移范围可由机器清单约束，不再依赖手工记忆
- Time: 2026-08-05 08:15
