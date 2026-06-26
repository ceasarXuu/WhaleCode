# Phase R3-A. TaskSpace Context Compiler and Cache Planner

## A.1 目标

建立一个专门模块负责 TaskSpace 的 agent-visible context 处理：

```text
拼接
裁剪
压缩
分层摘要
完整轨迹引用
DeepSeek cache-friendly layout
provider-visible assembly
结构化验证元数据
```

这个模块不是 formatter，也不是事后 scanner。它是 TaskSpace 到 provider request
之间的唯一上下文编译边界。

## A.2 背景

当前相关逻辑分散在多处：

| Current Area | Current Responsibility | Problem |
|---|---|---|
| `action_map/runtime.rs` | 生成 active/shadow projection 文本 | 只负责 projection，不负责最终 provider-visible payload |
| `session/turn.rs` | 过滤 provider-visible history，另有 action-contract prompt 精简路径 | 普通路径和 action-contract 路径不统一 |
| `client.rs` | exact provider payload scan | 只能事后扫描，无法决定上下文结构 |
| cost/release scripts | 聚合 scan/cache/budget 证据 | 容易验证字符串现象，而不是验证上下文 contract |

R3-A 要把这些职责归并到一个生产入口：

```text
TaskSpace full state + current turn + provider mode
  -> TaskSpaceContextCompiler
  -> TaskSpaceAgentContextBundleV1
  -> provider-visible prompt items
  -> verification manifest and cache plan
```

## A.3 设计原则

| Principle | Requirement |
|---|---|
| 单一入口 | 所有 TaskSpace provider-visible context 必须来自 compiler |
| 完整轨迹保留 | full event/map/result history 仍保留在 runtime/artifacts，不默认发给 agent |
| 语义完整 | Agent 可见 snapshot 必须能说明任务目标、路径、当前节点、证据和下一步 |
| 分层细节密度 | 当前节点最详细，临近节点次之，历史节点摘要，原始正文引用化 |
| Cache 友好 | 稳定内容放前缀，动态内容集中后缀，字段排序稳定，避免时间戳污染稳定区 |
| 可验证 | 输出 bundle、omission audit、cache plan 和 exact payload join |
| 可回退 | 保留旧 projection path 的 fallback，但 release gate 不允许 fallback claim 为完成 |

## A.4 输入输出边界

### 输入

```text
TaskSpace task state
Action map instance
Current node binding
Problem ledger
Cognitive state
Result/evidence store
Current user turn
Recent tool results
Provider profile: ordinary / action-contract / subagent / bootstrap
Route mode and advisory profile hints
```

### 输出

```text
TaskSpaceAgentContextBundleV1
ProviderVisiblePromptItems
RetrievalRefIndex
OmissionAudit
CachePlan
VerificationManifest
ContextCompilerTraceEvent
```

## A.5 Cache planner 责任

Context compiler 必须同时产出 cache plan。目标不是简单缩短上下文，而是保证高命中率。

| Cache Region | Contents | Change Policy |
|---|---|---|
| Stable prefix | system/tool protocol, TaskSpace schema, action-contract schema, fixed rules | 版本不变时 hash 不变 |
| Semi-stable task frame | task id, map skeleton, accepted criteria/facts/decisions, completed path summaries | 只有状态语义变化才变 |
| Dynamic suffix | current user turn, current node focus, recent evidence deltas, next action hint | 每轮允许变化，但 bounded |
| Hidden refs | output refs, result refs, trace refs | 默认不展开，只放稳定引用 |

Cache plan 至少包含：

```text
stable_prefix_hash
task_frame_hash
dynamic_suffix_hash
stable_prefix_tokens_estimate
task_frame_tokens_estimate
dynamic_suffix_tokens_estimate
expected_cacheable_tokens
expected_dynamic_tokens
cache_break_positions
cache_hit_risk_reasons
canonical_ordering_version
```

## A.6 实施任务

| Task | Production Code Path | Expected Behavior |
|---|---|---|
| 新建 context compiler 模块 | `core/src/action_map/context_compiler.rs` or equivalent | 从 TaskSpace state 生成 bundle |
| 定义 cache plan struct | same module plus runtime trace exports | cache layout 可观测 |
| 抽离 projection renderer | 从 `runtime.rs` 迁移或包装 | active/shadow projection 不再各自拼字符串 |
| 统一普通 provider path | `session/turn.rs` | ordinary prompt 也走 compiler |
| 统一 action-contract path | `session/turn.rs` | action-contract 不再有独立上下文裁剪逻辑 |
| 输出 compiler trace | `runtime.rs` trace model | 每次 provider request 可追踪 bundle id |
| 保留 fallback | session/client | fallback 可诊断，但不能通过 release gate |

## A.7 完成证据矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Context compiler module | 生产 bundle | `core/src/action_map/context_compiler.rs` | provider request assembly | unit tests | `context_compiler` trace | none | planned |
| Cache planner | 输出 hash/token/risk | compiler cache plan | provider request budget event | cache plan tests | cache plan fields in artifacts | none | planned |
| Ordinary path integration | 普通模型请求使用 bundle | `session/turn.rs` | sampling request | active replacement tests | exact payload scan join | none | planned |
| Action-contract integration | tool-free action prompt 使用同一 bundle profile | `session/turn.rs` | action-contract transport | action-contract tests | request shape trace | none | planned |
| Fallback labeling | fallback 不冒充完成 | session/client/release scripts | release decision | release fixture | fallback reason field | none | planned |

## A.8 日志和观测

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| compiler invoked | started | bundle build starts | missing task/map/current node | `missing_context_reason` | `request_id`, `task_id`, `map_id` | info | benchmark |
| bundle rendered | validated | `bundle_schema_valid=true` | schema invalid | `schema_error` | `bundle_id` | error | release gate |
| cache plan built | validated | stable/dynamic hashes emitted | unstable prefix | `cache_hit_risk_reasons` | `bundle_id`, `request_id` | warn | cost diagnostics |
| provider prompt assembled | committed | prompt item count and token estimates emitted | raw history included | `omission_failure_reason` | `provider_payload_sha256` | error | release gate |
| fallback used | committed | fallback reason emitted | fallback hidden | `fallback_reason` | `request_id` | warn | release gate |

## A.9 测试和收益验证

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | compiler produces valid bundle | unit tests with task/map/result fixtures | schema valid, required sections present |
| Correctness | ordinary/action-contract paths share compiler | path tests | no independent prompt pruning path remains |
| Correctness | full trajectory remains retrievable | fixture with hidden result refs | original result body accessible via ref |
| Benefit | provider-visible raw history eliminated | B-tier exact scan | raw TaskSpace history tokens = 0 |
| Benefit | cache hit maintained | B-tier provider-cache summary | request_2_plus_hit_rate >= 0.95 |
| Benefit | dynamic suffix bounded | cache plan artifact | dynamic suffix below configured threshold |
| Observability | context chain logged | trace/artifact inspection | bundle id, cache hashes, omission audit present |

## A.10 风险和回退

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| compiler 漏掉关键事实 | Agent 解题质量下降 | business_success drops | protected facts/accepted evidence hard gate | disable release, use old path for diagnosis only |
| cache plan 把动态内容放进前缀 | 成本上升 | stable prefix hash changes unexpectedly | canonical ordering and prefix diff test | fallback to conservative suffix placement |
| refs 不可读 | Agent 无法渐进检索 | ref resolution errors | ref index unit tests | include short summary in dynamic suffix |
| 旧路径未完全替换 | false green or pollution | provider payload scan sees legacy markers | release gate blocks | keep diagnostic artifact |

## A.11 Exit criteria

```text
All TaskSpace provider-visible context has a compiler bundle id.
Ordinary and action-contract prompts both use compiler profiles.
Bundle includes cache plan, omission audit, retrieval refs, verification manifest.
Old projection-only path is no longer accepted as release-complete evidence.
Focused tests pass before R3-B.
```
