# TaskSpace DeepSeek 缓存命中问题修复立项

- Created: 2026-06-22
- Updated: 2026-06-22
- Version: v0.0.5 blocker project
- Status: Active blocker / Draft engineering plan
- Owner / Responsible: WhaleCode v0.0.5 runtime
- Related Systems: TaskSpace runtime, DeepSeek official ChatCompletions provider, provider request construction, action-map runtime, benchmark harness
- Related Links:
  - `coe/2026-06-22-15-24-taskspace-deepseek-cache-hit-rate.md`
  - `scripts/taskspace-benchmark/verify-deepseek-cache-fix.ps1`
  - `docs/v0.0.5/build-R2/00-overview-and-gates.md`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/codex-api/src/endpoint/responses.rs`
- Risk Level: Critical
- Plan Type: Full
- Recommended AI Agent reasoning level: xhigh

## 0. Executive Decision

TaskSpace DeepSeek 缓存命中问题必须列为 v0.0.5 正式阻塞项。未解决前，不允许把 TaskSpace benchmark、E2/E3 实验或成本结论作为 release 证据。

原因很直接：当前 TaskSpace live 验证中，DeepSeek 官方 no-tool cache probe 可以达到约 `99.8%` 命中，但 TaskSpace 真实请求只有约 `11-12%` 命中。成本差异主要来自 uncached input 暴涨，而不是单纯总 token 多一点。

v0.0.5 必须完成一个 cache-optimized TaskSpace transport，使 TaskSpace provider 请求满足以下目标：

```text
steady_state_provider_cache_hit_rate_for_requests_2_plus >= 0.95
taskspace_uncached_input_tokens <= 1.2x standard_uncached_input_tokens on comparable diagnostic samples
taskspace_direct_input_plus_output_tokens <= 2.0x standard on formal gates
business_success must not regress against existing TaskSpace gates
```

如果最终证明 DeepSeek 官方 ChatCompletions 在带 tools schema 的请求上无法达到这个目标，v0.0.5 不能继续依赖该请求形态。

## 1. Problem Definition

### Current Behavior

TaskSpace 当前在一个用户任务内拆成多轮 provider 请求。每轮请求都可能带有：

- 大量工具 schema；
- TaskSpace projection；
- provider budget guidance；
- recovery item；
- inspect / implement / validation 节点状态；
- 工具输出历史或其替换项；
- action-map runtime 动态上下文。

DeepSeek ChatCompletions 的请求体中，工具 schema 位于 messages 之后。只要 messages 早期内容变化，后面即使工具 schema 完全相同，也不再处在同一个 provider prefix 上。

### Expected Behavior

TaskSpace 应该把稳定 system/developer/protocol/action contract 前缀固定下来，让第二次及后续 provider 请求复用绝大部分稳定输入。

### Evidence Baseline

最近 live 验证样本：

```text
standard:
  model_request_count = 1
  input_tokens = 94353
  cached_input_tokens = 75264
  uncached_input_tokens = 19089
  hit_rate = 0.797685

taskspace:
  model_request_count = 8
  input_tokens = 127528
  cached_input_tokens = 15104
  uncached_input_tokens = 112424
  hit_rate = 0.118437
```

关键事实：

- TaskSpace input 只约为 standard 的 `1.35x`。
- TaskSpace uncached input 约为 standard 的 `5.9x`。
- 因此成本爆炸点是 cache miss，不只是上下文变大。

## 2. Root Cause

### Confirmed Root Cause

DeepSeek 官方 cache 对稳定 no-tool prefix 正常工作；但 TaskSpace 当前的多轮 ChatCompletions 请求形态会反复把大 tools schema 放在动态 messages 之后，导致稳定的大块工具/协议内容无法作为同一 prefix 被复用。

这不是单一 prompt 顺序 bug。此前已经修过稳定 developer section 和 TaskSpace 动态 section 混在同一个 message 的问题，也修过 `apply_patch` custom tool 到 ChatCompletions function tool 的映射问题；这些是正确的局部修复，但不能从根本上解决 tools schema 反复 miss 的成本问题。

### Why Standard Mode Is Different

Standard 模式通常是少量请求，且请求形态更稳定。它也可能带 tools，但没有 TaskSpace 的多轮状态机 projection、节点预算、recovery item 和动态 action-map 上下文反复插入在工具 schema 前面。

TaskSpace 的问题是组合效应：

```text
many provider requests
+ dynamic early messages
+ large tools schema serialized after messages
= repeated uncached large input
```

## 3. Goals

| Goal | Expected Benefit | Measurement |
|---|---|---|
| G1: Stable cache prefix | DeepSeek 请求 2+ 能复用大部分固定协议输入 | `prompt_cache_hit_tokens / (hit + miss)` for requests 2+ |
| G2: Remove tools-schema churn from hot path | 降低 uncached input 成本 | TaskSpace uncached input ratio vs Standard |
| G3: Preserve TaskSpace correctness | 成本优化不能牺牲任务完成率 | existing benchmark business success and oracle gates |
| G4: Make cache behavior observable | 后续不会再次凭感觉判断缓存问题 | request-shape hash, prefix hash, per-request cache trace |
| G5: Make rollout reversible | 如果新 transport 降低正确率，可以回退 | config flag and side-by-side benchmark |

## 4. Non-Goals

- 不把降低模型能力作为主要手段，例如简单缩短 prompt 但破坏 TaskSpace 约束。
- 不通过隐藏固定自然语言答复绕过 Agent/Model 路径。
- 不把 benchmark 阈值调低来伪造成本改善。
- 不依赖尚未验证的 provider 行为，例如假设 DeepSeek 会缓存 tools schema 后缀。
- 不在没有 live provider 证据时声明问题修复。

## 5. Root Solution Direction

### Recommended Direction: Cache-Optimized Tool-Free Provider Transport

把 TaskSpace 的 provider-visible 请求从“模型直接调用工具”改为“模型输出受约束动作契约，runtime 本地解析并执行”。

目标形态：

```text
Stable provider prefix:
  system/developer policy
  TaskSpace action contract
  allowed action classes
  output grammar / JSON schema description
  examples

Dynamic suffix:
  compact task state
  active node summary
  minimal recent evidence refs

No provider tools schema in hot path.

Runtime:
  parse model action contract
  validate against active node and permissions
  execute local tools / taskspace_control / apply_patch / shell
  record result into action-map ledger
  project compact state for next model turn
```

This changes the provider contract from API-level tool calls to runtime-owned action envelopes.

### Action Contract Candidates

The model should emit one of a small set of envelopes:

```json
{
  "type": "taskspace_action",
  "version": "v1",
  "action": "read_file",
  "node_id": "node-123",
  "args": {
    "path": "src/example.rs"
  },
  "rationale": "Need evidence for the active inspect node."
}
```

Candidate actions:

| Action | Runtime Execution | Allowed Node Kinds |
|---|---|---|
| `read_file` | existing shell/read path or safer read tool | inspect, validation |
| `search` | `rg` wrapper | inspect, validation |
| `apply_patch` | existing apply_patch handler | implement |
| `run_test` | shell command under validation policy | smoke_test, regression_test |
| `taskspace_control` | existing action-map control path | route/finish/bind nodes |
| `final_answer` | final synthesis path | final_synthesis |
| `blocked` | structured blocker | any node |

The provider no longer sees the full tools schema; it sees the stable action contract text.

## 6. Alternatives And Tradeoffs

| Option | Description | Pros | Cons | Recommendation |
|---|---|---|---|---|
| A. Tool-free action contract | Model emits structured action JSON/text; runtime executes tools locally | Best cache shape; provider-agnostic; debuggable | Requires parser, validator, migration work | Recommended |
| B. Split planner and tool executor | DeepSeek plans without tools; a local executor or smaller model translates to tool calls | Keeps DeepSeek prefix clean | More moving parts; possible correctness drift | Consider after A if needed |
| C. Responses-native DeepSeek path | Use a provider path where tools might be serialized/cached differently | Could retain native tool calls | Must be proven; may not exist or may behave similarly | Discovery only |
| D. Prompt ordering and tool filtering only | Keep ChatCompletions tools but shrink/move dynamic text | Lower implementation cost | Already insufficient in live validation | Not enough |
| E. Cache warmup requests | Send synthetic warmup requests before real work | May improve benchmark numbers | Adds latency/cost and may not help unique tasks | Not primary solution |

## 7. Phased Execution Plan

### Phase 0: Baseline And Gate Lock

#### Objective

Freeze the current failure as an explicit v0.0.5 blocker and prevent accidental release claims.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Latest failure artifact exists | Read verification report | `post-edit-drain-force/deepseek-cache-fix-verification.md` | runtime |
| Official cache probe works | Run verification script no-tool probe | hit rate around 99% | runtime |
| TaskSpace cache miss reproduced | Run TaskSpace benchmark probe | TaskSpace hit rate around 11-12% | runtime |

#### Implementation Tasks

- Add this project document under `docs/v0.0.5/缓存命中问题修复/`.
- Add release gate wording that v0.0.5 cannot use TaskSpace experimental evidence until cache gates pass.
- Keep COE linked as the canonical evidence record.

#### Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Documentation exists and links evidence | file review | root cause and acceptance gates are explicit |
| Benefit | No false positive release claim | release checklist review | cache gate blocks v0.0.5 closeout |

#### Exit Criteria

- Project is documented.
- Gate is visible to future implementers.
- Open questions are recorded.

### Phase 1: Provider Request Shape Observatory

#### Objective

Make provider cache behavior explainable per request, not only after aggregate benchmark reports.

#### Implementation Tasks

- Add `TaskSpaceProviderCacheTraceV1` artifact with:
  - logical mode;
  - model request index;
  - request phase;
  - node kind;
  - provider wire API;
  - tools count;
  - messages hash;
  - stable prefix hash;
  - dynamic suffix hash;
  - input tokens;
  - cached input tokens;
  - uncached input tokens;
  - hit rate.
- Add exact request-shape comparison for Standard vs TaskSpace.
- Mark whether the request used native tools schema or tool-free action contract.

#### Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Trace coverage | benchmark artifact scan | >= 99% provider requests have cache trace |
| Benefit | Root-cause observability | compare failing run | can identify first divergent prefix segment |

#### Exit Criteria

- No future cache investigation relies on manual log scraping.
- The trace can prove whether a request is tools-schema hot path or tool-free path.

### Phase 2: Tool-Free Action Contract Prototype

#### Objective

Build a narrow TaskSpace transport mode that removes provider tools schema from the hot path for a small scenario.

#### Implementation Tasks

- Add `TaskspaceProviderTransportMode`:
  - `NativeTools` for current behavior;
  - `CacheOptimizedActionContract` for the new path.
- Add provider prompt contract for `TaskSpaceActionV1`.
- Add parser and validator for the action envelope.
- Map `read_file`, `search`, `apply_patch`, `run_test`, and `taskspace_control` to existing runtime/tool handlers.
- Keep all permission and node-kind checks in runtime; do not trust model-emitted action type alone.
- Feature-flag the mode for DeepSeek official provider only at first.

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Transport mode config | `core/src/session/turn.rs`, provider config | TaskSpace request build | unit tests | trace field `transport_mode` | none | planned |
| Action parser | new parser module or existing session path | model output handling | parser tests | invalid-action warning events | none | planned |
| Runtime executor bridge | session/tool runtime handlers | tool execution path | integration tests | tool result ledger entries | none | planned |
| Cache trace | metrics/artifact writer | benchmark harness | fixture tests | `TaskSpaceProviderCacheTraceV1` | none | planned |

#### Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Single-file fast fix solves | live DeepSeek diagnostic | public and hidden validation exit 0 |
| Benefit | Cache hit improves | same script as current failure | requests 2+ hit rate >= 0.95 |
| Safety | Invalid action rejected | unit/integration tests | invalid envelope cannot bypass node policy |

#### Exit Criteria

- The prototype solves at least one L1 diagnostic in TaskSpace.
- Provider requests in the new mode contain no native tools schema.
- Cache trace shows stable prefix reuse after request 1.

### Phase 3: Expand Action Contract Coverage

#### Objective

Make the tool-free transport cover the normal TaskSpace workflow, not only one smoke scenario.

#### Implementation Tasks

- Support inspect, implement, smoke_test, regression_test, and final_synthesis nodes.
- Support structured blockers and budget-recovery actions.
- Support output references instead of replaying large tool outputs.
- Ensure `apply_patch` uses existing patch verification and permission checks.
- Ensure shell/test commands remain constrained by node kind.

#### Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | E2 diagnostic matrix | benchmark harness | no regression vs NativeTools on solved count |
| Benefit | Cache hit | provider cache trace | steady-state hit rate >= 0.95 |
| Budget | Request count | token summary | model_request_count ratio <= 2.0x standard |

#### Exit Criteria

- New transport passes E2 diagnostic without engineering unclean failures.
- Existing native tools path remains available as fallback.

### Phase 4: Cost Gate Integration

#### Objective

Promote cache metrics from diagnostic-only to release gate inputs.

#### Implementation Tasks

- Extend release decision script to read cache trace.
- Add hard gates:
  - requests 2+ hit rate >= 0.95;
  - TaskSpace uncached input <= 1.2x Standard on comparable samples;
  - unknown cache trace coverage <= 1%.
- Add soft warning:
  - aggregate sample hit rate below 0.85, because cold-start request can dominate very small samples.
- Add failure taxonomy:
  - `cache_prefix_unstable`;
  - `native_tools_schema_hot_path`;
  - `cache_trace_missing`;
  - `provider_cache_regression`;
  - `tool_free_transport_correctness_regression`.

#### Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Gate rejects current failing artifact | fixture from `post-edit-drain-force` | fail with `native_tools_schema_hot_path` or `cache_prefix_unstable` |
| Benefit | Gate accepts synthetic good fixture | generated fixture | pass with hit rate >= thresholds |

#### Exit Criteria

- v0.0.5 release checks cannot pass while cache is in the current failure state.

### Phase 5: Formal v0.0.5 Validation

#### Objective

Prove the cache-optimized transport is suitable for v0.0.5 experiments.

#### Validation Matrix

| Stage | Sample | Required Result |
|---|---|---|
| L1 smoke | `single-file-fast-fix` | business success, requests 2+ hit >= 0.95 |
| E2 diagnostic | current TaskSpace diagnostic set | no correctness regression, uncached input <= 1.2x Standard |
| E3 readiness | v0.0.5 readiness sample | existing R2 gates plus cache gates pass |
| Formal E3 | approved sample set only | release evidence accepted only if cache gates pass |

#### Exit Criteria

- TaskSpace cost is no longer dominated by uncached repeated stable input.
- Cache trace artifacts are archived with benchmark results.
- NativeTools fallback is either retained as debug-only or explicitly deprecated for DeepSeek official TaskSpace.

## 8. Acceptance Criteria

### Must Pass Before v0.0.5 Experimental Claims

- Given a TaskSpace DeepSeek official run, when request 2+ reuses the same stable TaskSpace action contract, then provider usage reports cache hit rate >= `0.95` for the stable prefix-dominated request.
- Given a comparable Standard vs TaskSpace diagnostic sample, when token summary is generated, then TaskSpace uncached input tokens are <= `1.2x` Standard uncached input tokens.
- Given the current failing artifact, when release gates evaluate it, then the gate fails with a cache-specific taxonomy.
- Given the cache-optimized transport, when native provider tools are disabled, then model actions are still executed through the existing runtime permission and node-kind checks.
- Given malformed or disallowed model action output, when runtime parses it, then no tool executes and the action-map ledger records a structured rejection.
- Given an implementation node, when the model emits `run_test`, then runtime rejects it until an appropriate validation node is active.
- Given a validation node, when the model emits `apply_patch`, then runtime rejects it.

### Stretch Targets

- Aggregate TaskSpace sample cache hit rate >= `0.90` on E2+ samples.
- Direct input+output token ratio <= `1.5x` Standard on L1/L2 diagnostics.
- No more than one cold-start provider request per user task unless explicitly marked.

## 9. Rollback And Fallback Strategy

- Keep `NativeTools` transport as a feature-flagged fallback during development.
- Default DeepSeek TaskSpace to `CacheOptimizedActionContract` only after Phase 3 passes.
- If correctness regresses, fallback to `NativeTools` for that scenario and keep cache gate failed.
- If DeepSeek changes official cache behavior, rerun Phase 1 probes before changing thresholds.

## 10. Observability Requirements

Every TaskSpace provider request must expose:

- request index;
- task id / map id;
- active node id and kind;
- request phase;
- transport mode;
- tools schema present or absent;
- prompt prefix hash;
- dynamic suffix hash;
- provider usage cache hit/miss fields;
- cache hit rate;
- reason for any gate failure.

These fields must be visible in benchmark artifacts without manual parsing of raw provider payloads.

## 11. Open Questions

| Question | Why It Matters | Proposed Handling |
|---|---|---|
| Should the release gate require aggregate hit >= 0.95 or steady-state hit >= 0.95? | Very small tasks have cold-start miss that can dominate aggregate rate | Use steady-state >= 0.95 as hard gate, aggregate as warning until enough samples exist |
| Can DeepSeek official expose a Responses-compatible API path with better tool schema caching? | Could preserve native tool calls | Phase 1 discovery only; do not assume |
| How strict should the action grammar be? | More strict improves runtime safety but may reduce model compliance | Start with JSON envelope plus recovery parser; reject ambiguous actions |
| Should Standard mode also use cache trace? | Needed for comparable cost gates | Yes, include Standard as baseline |
| What is the minimum acceptable E2 sample count? | Avoid overfitting to one smoke task | Align with existing v0.0.5 E2/E3 policy |

## 12. Decision Log

| Topic | Decision | Rationale | Date |
|---|---|---|---|
| v0.0.5 status | Cache miss is a formal blocker | Experiments cannot continue with uncontrolled uncached input cost | 2026-06-22 |
| Root solution | Build tool-free action-contract transport | Removes repeated native tools schema from provider hot path | 2026-06-22 |
| Validation | Use DeepSeek official usage fields | `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens` are the relevant provider truth source | 2026-06-22 |
| Fallback | Keep NativeTools while validating | Avoid blocking local debugging if new transport regresses correctness | 2026-06-22 |

## 13. Immediate Next Steps

1. Add Phase 1 cache trace artifact and exact request-shape comparison.
2. Add `TaskspaceProviderTransportMode` behind a flag.
3. Implement the minimal `TaskSpaceActionV1` parser and executor for `single-file-fast-fix`.
4. Run the existing verification script against both `NativeTools` and `CacheOptimizedActionContract`.
5. Promote passing cache thresholds into release-decision gates.

## 14. Project Close Criteria

This project can close only when:

- v0.0.5 cache gates pass on live DeepSeek official API;
- TaskSpace correctness gates do not regress;
- benchmark artifacts include cache trace evidence;
- the old native-tools DeepSeek TaskSpace path is either disabled by default or explicitly marked non-release for cost-sensitive experiments;
- COE is updated with final acceptance evidence.
