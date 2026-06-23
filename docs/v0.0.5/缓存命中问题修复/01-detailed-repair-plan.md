# TaskSpace DeepSeek 缓存命中问题详细修复方案

- Created: 2026-06-22
- Updated: 2026-06-23
- Version: v0.0.5 cache-hit repair plan, scope corrected
- Status: In Progress
- Owner / Responsible: WhaleCode v0.0.5 runtime
- Related Systems: TaskSpace runtime, DeepSeek official ChatCompletions provider, provider request construction, cache trace, benchmark cache verifier
- Related Links:
  - `docs/v0.0.5/缓存命中问题修复/README.md`
  - `docs/v0.0.5/缓存命中问题修复/02-implementation-status.md`
  - `coe/2026-06-22-15-24-taskspace-deepseek-cache-hit-rate.md`
  - `scripts/taskspace-benchmark/verify-deepseek-cache-fix.ps1`
  - `scripts/taskspace-benchmark/test-deepseek-cache-verifier.ps1`
- Risk Level: Critical
- Plan Type: Standard
- Task Classification: bug fix + cache-performance optimization
- Recommended AI Agent reasoning level: xhigh

## 1. Problem Definition

### Current Behavior

TaskSpace 在 DeepSeek 官方 ChatCompletions 路径中会发起多轮 provider 请求。旧路径反复把 provider-native tools schema 放入请求热路径，并且前面混有动态 TaskSpace 状态。DeepSeek 的 input cache 对稳定前缀敏感；动态前缀变化会导致后续大块稳定 schema 无法稳定复用。

已观测 baseline：

| Mode | Requests | Input | Cached Input | Uncached Input | Hit Rate |
|---|---:|---:|---:|---:|---:|
| Standard | 1 | 94,353 | 75,264 | 19,089 | 0.797685 |
| TaskSpace old path | 8 | 127,528 | 15,104 | 112,424 | 0.118437 |

### Expected Behavior

TaskSpace request 2+ 应复用稳定 provider prefix，并在 DeepSeek 官方 usage 字段中稳定体现 `>= 0.95` 的缓存命中率。

### Gap

旧路径的 native tools schema 和动态消息排列方式不适合作为 DeepSeek TaskSpace 多轮请求热路径。必须改变 provider-visible 请求形态，而不是调整非缓存策略。

## 2. Goals

| Goal | Expected Benefit | Baseline | Target | Measurement |
|---|---|---:|---:|---|
| Stable provider prefix | request 2+ 复用稳定输入 | TaskSpace hit rate `0.118437` | `request_2_plus_hit_rate >= 0.95` | provider usage fields |
| Remove native tools schema churn | 热路径不再反复发送大 tools schema | old path present | `native_tools_schema_hot_path_count == 0` | cache trace |
| Complete cache observability | 每次 provider 请求可判定命中与请求形态 | partial/manual | `trace_coverage >= 0.99` | provider cache trace |
| Enforce cache-only gate | 缓存验收不被非缓存指标污染 | mixed gates | cache verifier ignores non-cache pass/fail | verifier selftest |

## 3. Non-Goals

本项目明确不处理、不验收、不继续追踪以下事项：

- TaskSpace 节点预算策略；
- TaskSpace 任务完成结果；
- 非缓存执行链路问题；
- TaskSpace 与 standard 的总 input token 比例；
- request count 与 standard 的比例；
- aggregate total token 下降目标。

这些事项即使在缓存验证过程中出现，也不属于本项目的修复范围。

## 4. Constraints And Assumptions

| Assumption | Verification Method | If Assumption Fails |
|---|---|---|
| DeepSeek 官方 cache usage 字段可信 | official no-tool cache probe | 标记 provider/account/model 层问题 |
| request 2+ 比整体 hit rate 更适合验收 | 冷启动请求单独排除 | 若 provider 不返回逐请求字段，则缓存项目不能关闭 |
| 移除 native tools schema 是必要条件 | request shape trace | 若 native tools schema 仍出现，cache gate fail |
| 缓存验证可以独立于任务是否完成 | verifier selftest | 若脚本再次绑定非缓存结果，视为回归 |

## 5. Technical Design

### 5.1 Transport Mode

TaskSpace DeepSeek 使用显式 transport：

```rust
enum TaskspaceProviderTransportMode {
    NativeTools,
    CacheOptimizedActionContract,
}
```

`CacheOptimizedActionContract` 是缓存修复路径。`NativeTools` 只可用于对比或调试，不能作为 DeepSeek TaskSpace 缓存修复证据。

### 5.2 Provider Request Shape

目标形态：

```text
Stable prefix:
  system/developer policy
  DeepSeek cache anchor
  TaskSpace action contract
  stable action schema examples

Dynamic suffix:
  compact active task state
  bounded recent result references

Provider tools:
  none
```

Hard invariant:

```text
DeepSeek TaskSpace cache-optimized requests must omit native tools schema.
```

### 5.3 Action Contract

模型输出本地 runtime 可解析的 action envelope；provider 不再接收 native tools schema。

```json
{
  "schema_version": "taskspace-action-v1",
  "action": "read_file",
  "node_id": "node-123",
  "args": {
    "path": "src/example.rs"
  },
  "rationale": "Need direct evidence for the active node."
}
```

runtime 仍必须做基本安全校验，防止模型输出绕过本地工具权限。这是 transport 的必要安全边界，不是新增的业务验收目标。

### 5.4 Cache Trace

每个 TaskSpace provider request 记录：

- request index；
- transport mode；
- request shape classifier；
- tools present/count；
- input tokens；
- cached input tokens；
- uncached input tokens；
- hit rate；
- cache usage present/missing；
- request 2+ 聚合命中率。

## 6. Phased Execution Plan

### Phase 0: Scope Lock

#### Objective

把 v0.0.5 本项目收敛为缓存命中率修复，删除非缓存验收项。

#### Tasks

- 更新 `README.md`、本计划、实现状态文档。
- 明确禁止把任务完成率、预算策略、总 token 比例作为缓存 gate。
- 更新验证脚本，使 pass/fail 只由 cache usage 与 request shape 决定。

#### Validation

| Validation Type | Method | Passing Standard |
|---|---|---|
| Documentation | `rg` scope audit | 无非缓存验收项 |
| Script behavior | verifier selftest | 任务失败但缓存达标的 fixture 通过缓存验证 |

### Phase 1: Cache Trace And Request Shape

#### Objective

让缓存行为可观测、可复现、可作为 release gate 输入。

#### Tasks

- 生成 `provider-cache-trace.jsonl`。
- 生成 `provider-cache-trace-summary.json`。
- 分类请求形态：
  - `tool_free_action_contract`；
  - `native_tools_schema_hot_path`；
  - `unknown_or_unclassified`。
- 聚合 request 2+ cache hit rate。

#### Exit Criteria

- `trace_coverage >= 0.99`。
- provider cache usage 缺失数为 `0`。
- request 2+ 命中率可直接从 summary 读取。

### Phase 2: Cache-Optimized Transport

#### Objective

在 DeepSeek TaskSpace 热路径移除 native tools schema。

#### Tasks

- DeepSeek TaskSpace 默认选择 `CacheOptimizedActionContract`。
- provider request tools list 为空。
- 稳定 action contract 进入 provider prefix。
- 动态 TaskSpace 状态保持 bounded suffix。
- runtime 本地解析 action envelope 并调用已有执行路径。

#### Exit Criteria

- `native_tools_schema_hot_path_count == 0`。
- `tool_free_action_contract_count > 0`。
- request 2+ 命中率 `>= 0.95`。

### Phase 3: Cache Gate Integration

#### Objective

把缓存指标变成独立 gate。

#### Tasks

- `verify-deepseek-cache-fix.ps1` 使用 cache-only pass/fail。
- `write-release-decision.ps1` 暴露 `provider_cache_trace_gate_pass`。
- 反例 fixture 覆盖：
  - hit rate 低；
  - native tools schema 热路径出现；
  - cache trace 缺失；
  - provider cache usage 缺失。

#### Exit Criteria

缓存 gate 可以独立回答“DeepSeek input cache 命中是否达标”，不受非缓存结果影响。

### Phase 4: Live DeepSeek Verification

#### Objective

用 DeepSeek 官方 API 证明修复有效。

#### Tasks

- 运行 official no-tool probe。
- 运行 TaskSpace cache-optimized probe。
- 归档 verification JSON/Markdown。
- 归档 provider cache trace。

#### Passing Standard

| Metric | Target |
|---|---:|
| official identical second request hit rate | available and high |
| TaskSpace request 2+ hit rate | `>= 0.95` |
| cache trace coverage | `>= 0.99` |
| cache usage missing count | `0` |
| native tools schema hot path count | `0` |
| tool-free action contract count | `> 0` |

## 7. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Status |
|---|---|---|---|---|---|---|
| Transport mode | DeepSeek TaskSpace uses cache-optimized transport | `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | TaskSpace provider request build | `taskspace_provider_transport_defaults_deepseek_to_action_contract` | `transport_mode` trace | landed |
| Tool-free prompt | Provider request omits native tools schema | `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | DeepSeek ChatCompletions request | action-contract tests | `native_tools_schema_hot_path_count=0` | landed |
| Cache trace | Provider requests emit cache hit/miss summary | `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1` | benchmark artifact aggregation | `test-cost-instrumentation.ps1` | `provider-cache-trace-summary.json` | landed |
| Cache release gate | Low/missing/native-tools cache trace fails | `scripts/taskspace-benchmark/write-release-decision.ps1` | release decision | `test-release-decision.ps1` | `provider_cache_trace_gate_pass` | landed |
| Cache verifier | Script validates cache only | `scripts/taskspace-benchmark/verify-deepseek-cache-fix.ps1` | cache verification command | `test-deepseek-cache-verifier.ps1` | verification JSON/Markdown | landed |

## 8. Acceptance Criteria

The project is accepted when all are true:

- DeepSeek official cache fields are present.
- `effective_taskspace_cache_hit_rate_source == provider_request_2_plus`。
- `effective_taskspace_cache_hit_rate >= 0.95`。
- `cache_trace_coverage >= 0.99`。
- `cache_usage_missing_count == 0`。
- `native_tools_schema_hot_path_count == 0`。
- `tool_free_action_contract_count > 0`。
- Cache verifier selftest proves non-cache task result fields do not affect cache pass/fail.

## 9. Test Plan

Required commands:

```powershell
cargo test -p codex-core taskspace_provider_transport_defaults_deepseek_to_action_contract --lib
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-deepseek-cache-verifier.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\verify-deepseek-cache-fix.ps1 -RunTaskspaceBenchmark -TaskspaceProviderTransport cache_optimized_action_contract -MinTaskspaceHitRate 0.95
```

## 10. Rollback And Fallback

- If cache-optimized transport fails cache gate, keep cache project open.
- If provider cache usage fields disappear, fail cache verification rather than infer from unrelated fields.
- If native tools schema reappears in DeepSeek TaskSpace hot path, fail cache verification.
- `NativeTools` remains a debug comparison path, but cannot close this cache project.

## 11. Observability And Success Metrics

| Metric | Target | Source |
|---|---:|---|
| request 2+ cache hit rate | `>= 0.95` | provider cache trace |
| cache trace coverage | `>= 0.99` | provider cache trace |
| cache usage missing count | `0` | provider cache trace |
| native tools schema hot path count | `0` | provider cache trace |
| tool-free action contract count | `> 0` | provider cache trace |

## 12. Change Log

| Date | Change | Reason |
|---|---|---|
| 2026-06-22 | Created cache blocker project | Formalize TaskSpace DeepSeek cache-hit issue |
| 2026-06-23 | Action-contract cache path live evidence recorded | request 2+ hit rate reached `0.989246` |
| 2026-06-23 | Scope corrected | Removed non-cache goals, operations, and acceptance criteria |
| 2026-06-23 | Cache verifier corrected | Script pass/fail no longer depends on task result fields |
