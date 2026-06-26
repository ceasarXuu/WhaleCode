# Phase R3-E. Timing Attribution and Observability

## E.1 目标

补齐 Phase H 暴露的 timing blocker：

```text
wait_attribution_status = missing
wait_attribution_missing_fields includes model_request_duration_ms
taskspace_wall_time_ratio = 3.07
```

R3-E 不负责让所有任务一定更快。它负责让慢在哪里可证明，且 formal E3 不会在缺少
关键 timing 字段时通过。

## E.2 Required timing model

每个 provider request 至少要能拆出：

```text
request_id
logical_request_id
attempt_seq
queued_at_ms
dispatched_at_ms
stream_opened_at_ms
first_token_at_ms
completed_at_ms
model_request_duration_ms
stream_wait_ms
tool_wait_ms when applicable
client_overhead_ms
retry_or_fallback_duration_ms
```

聚合层至少要能输出：

```text
model_time_total_ms
tool_time_total_ms
runtime_overhead_total_ms
cache_wait_or_provider_wait_ms
unknown_time_ms
unknown_time_ratio
bottleneck_classification
wait_attribution_status
```

## E.3 实施任务

| Task | Production Code Path | Expected Behavior |
|---|---|---|
| provider lifecycle timing fields | `client.rs` provider lifecycle events | completed/error/cancelled 都有 duration |
| model request duration | client event finalization | `model_request_duration_ms` present |
| wait attribution summary | benchmark timing scripts | missing fields fail diagnostic |
| walltime release gate | release decision | high walltime requires complete bottleneck report |
| trace correlation | runtime/client events | request_id/logical_request_id stable across attempt |

## E.4 完成证据矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Provider duration fields | timing emitted for every terminal request | client lifecycle | provider request | client tests | provider_request_budget tags | none | planned |
| Timing summary | benchmark computes attribution | scripts | benchmark run | script tests | sample-timing.json | none | planned |
| Release blocker | missing timing blocks release | release script | release decision | release fixture | release-decision.json | none | planned |

## E.5 日志和观测

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| request queued | queued | queued timestamp | missing queue timestamp | `missing_queued_at` | `request_id` | info | timing summary |
| request dispatched | started | dispatch timestamp | missing dispatch | `missing_dispatched_at` | `request_id` | info | timing summary |
| stream opened | streaming | stream opened timestamp | stream never opens | `stream_open_error` | `logical_request_id` | warn | diagnostics |
| request terminal | completed/error/cancelled | terminal duration | missing duration | `missing_model_request_duration_ms` | `request_id` | error | release gate |
| attribution aggregated | validated | unknown_time_ratio within threshold | missing fields | `wait_attribution_missing_fields` | `sample_id` | error | release gate |

## E.6 验证

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | terminal events have duration | unit tests | completed/error/cancelled include duration |
| Correctness | retry attempts preserve IDs | fixture | logical request id stable, attempt seq increments |
| Benefit | walltime diagnosis complete | B-tier timing summary | wait_attribution_status=complete |
| Benefit | formal E3 blocked on missing timing | release fixture | missing duration fails |
| Observability | bottleneck report exists | artifact inspection | model/tool/runtime/unknown split present |

## E.7 Exit criteria

```text
sample-timing.json has wait_attribution_status=complete.
model_request_duration_ms is present for terminal provider events.
unknown_time_ratio is below configured threshold or release remains blocked.
taskspace_wall_time_ratio > threshold has a complete bottleneck report.
```

## E.8 当前实现状态

已落地 provider lifecycle duration 标准字段链路：

```text
runtime provider_request_budget trace:
  latency_ms
  model_request_duration_ms

benchmark timing parser:
  provider_lifecycle terminal events preferred
  responsesapi.websocket_timing retained as fallback

cost instrumentation artifact:
  started_at_ms
  completed_at_ms
  latency_ms
  model_request_duration_ms
```

这解决的是 Phase H 中 `model_request_duration_ms missing` 的主要工程缺口：DeepSeek / TaskSpace 路径不应依赖 `responsesapi.websocket_timing` 私有事件，只要 provider lifecycle terminal event 存在，就能归因模型请求耗时。

已验证：

```text
cargo test -p codex-core provider_request_budget --lib
10 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-score-validity.ps1
PASS

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
PASS
```

尚未完成的真实收益证明：需要重新跑 B-tier / targeted diagnostic，确认真实 `sample-timing.json` 中 `model_request_duration_ms` 非空，并使 `wait_attribution_missing_fields` 不再包含该字段。
