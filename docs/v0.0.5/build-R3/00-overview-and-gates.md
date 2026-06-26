# build-R3 总览和门禁

> build-R3 是 build-R2 Phase H 之后的收敛工作集。目标不是重新做 A-H，而是把
> R2 暴露出的真实 blocker 归并到一个可实现、可验证、可回滚的工程计划里。

## 0.1 元数据

```text
Created: 2026-06-26
Updated: 2026-06-26
Version: v0.0.5 build-R3
Status: Draft
Owner / Responsible: WhaleCode core runtime
Related Systems: TaskSpace runtime, session turn assembly, provider client, benchmark release gates
Related Links:
  docs/v0.0.5/build-R2/08-phase-h-e3-readiness.md
  docs/v0.0.5/build-R2/09-module-checklist-and-closeout.md
Risk Level: High
Plan Type: Full
AI Agent 推理程度: high
```

## 0.2 问题定义

build-R2 已经证明 TaskSpace 的业务路径、DeepSeek cache 命中、action-contract ABI、
budget advisory 语义、spawn review debt gate 在 B-tier smoke 中不是当前主阻塞。
但是 Phase H 同时证明 v0.0.5 仍不能进入 targeted/formal E3。

当前行为：

```text
active-context-replacement-report:
  exact_payload_scan_passed = false
  replacement_confirmed = false
  legacy_taskspace_history_present = true
  raw_taskspace_control_history_tokens = 917
  protected_items_present = false
graph-health / metrics:
  open_leaf_nodes = 1
pair report:
  taskspace_wall_time_ratio = 3.07
sample-timing:
  wait_attribution_status = missing
  wait_attribution_missing_fields includes model_request_duration_ms
cost-diagnostics:
  root_cause = fixed_taskspace_provider_context_surface_too_large
  provider_direct_input_output_ratio = 12.9726
  projection_token_share_of_taskspace_input = 0.0022
```

期望行为：

```text
TaskSpace 只通过一个上下文编译器生成 agent-visible context。
真实完整轨迹仍完整保留，但默认不全量进入 provider payload。
上下文编译器同时维护 DeepSeek cache-friendly layout。
active replacement proof 由结构化 bundle 和 exact provider payload join 证明。
graph closeout 和 timing attribution 独立收敛，不被上下文优化掩盖。
```

## 0.3 R3 目标

| Goal | Expected Benefit | Verification |
|---|---|---|
| 建立 TaskSpace Context Compiler + Cache Planner | 消除散落的 projection/filter/scanner 逻辑，降低上下文污染 | 单一路径生成 `TaskSpaceAgentContextBundleV1`，旧路径测试不能绕过 |
| 保留完整轨迹但默认只发送精简 map snapshot | Agent 知道任务走到哪、为什么、下一步做什么，同时避免全量历史进模型 | bundle 中有 current node、nearby nodes、completed path summary、evidence refs |
| 维护 DeepSeek cache hit | 成本管理从硬砍预算变成稳定前缀和小动态区 | `request_2_plus_hit_rate >= 0.95`，stable prefix hash 非预期变化为 0 |
| 修正 active replacement proof | 不再用全局字符串 grep 误判合法 `taskspace_control` 枚举 | `exact_context_bundle_verified=true`，`raw_taskspace_history_tokens=0` |
| 收敛 graph closeout | 真实完成路径不留下 open leaf | B-tier 和 targeted diagnostic 中 `open_leaf_nodes=0` |
| 补齐 timing attribution | walltime blocker 可解释，不再缺关键字段 | `model_request_duration_ms` 存在，`wait_attribution_status=complete` |
| 重跑 H/E3 前置门禁 | 证明收益真实，不把诊断失败包装成 release pass | non-agent gates、B-tier、targeted diagnostic、start gate 顺序通过 |

## 0.4 非目标

```text
不在 R3 中重新定义 TaskSpace 产品愿景。
不把 profile 重新变成 request/session 硬上限。
不删除完整轨迹或牺牲 replay/debug 能力换取短上下文。
不通过关键词白名单绕过模型或伪造自然语言回答。
不在 active replacement proof 中只依赖 hash-only 或 post-run synthetic evidence。
```

## 0.5 阶段总览

| Phase | Theme | Main Output | Exit Gate |
|---|---|---|---|
| R3-A | Context compiler and cache planner | 统一上下文编译器设计和生产入口 | 所有 provider-visible TaskSpace context 只能从 compiler 输出 |
| R3-B | Agent context bundle contract | `TaskSpaceAgentContextBundleV1` schema、refs、audit、cache plan | 结构化 bundle fixture 和 scanner fixture 通过 |
| R3-C | Provider integration and payload proof | session/client 接入、exact payload proof 重写 | B-tier 中 active replacement proof 通过 |
| R3-D | Graph closeout | open leaf 生命周期收敛 | B-tier 和 targeted diagnostic `open_leaf_nodes=0` |
| R3-E | Timing attribution | provider/model/wait timing 字段补齐 | wait attribution complete，walltime blocker 可解释 |
| R3-F | Validation closeout | 当前 HEAD 证据包、B-tier、targeted diagnostic、formal E3 start gate | targeted diagnostic 通过后才允许 formal E3 |

2026-06-26 implementation note:

```text
R3-A/B/C first vertical slice landed locally:
  TaskSpace context compiler module wraps active projection with bundle/cache/protected proof.
  ordinary provider-visible prompt and action-contract prompt both compile active context items.
  exact payload scanner ignores legal taskspace_control(action=...) guidance inside compiled bundle.
  runtime exact_payload_scan trace and benchmark scripts now carry context_bundle_present,
  exact_context_bundle_verified, and cache_plan_verified.
Focused gates passed, but B-tier benefit proof is still required before marking R3-A/B/C done.

R3-D first lifecycle fix landed locally:
  final_answer no longer blocks finish_node recovery when the current implement/test node already
  has successful required-action evidence.
  active_context_replacement and taskspace unit gates passed.
  B-tier / targeted diagnostic proof is still required before marking graph closeout done.

R3-D second lifecycle fix landed locally:
  explicit taskspace_control(action=finish_node) on smoke_test/regression_test is no longer
  rewritten into final_answer before tool conversion.
  This preserves lifecycle state commit before terminal response.
  active_context_replacement and taskspace unit gates passed with 83 / 93 tests.
  B-tier proof must still show open_leaf_nodes=0.

R3-D third lifecycle fix landed locally:
  closed graph with an existing task and no active bound node is now treated as final-answer state.
  The prompt tells the model to return final_answer when work is complete, and runtime guards
  no-active-node plus accepted successful validation by synthesizing final_answer for non-terminal
  follow-up work actions.
  active_context_replacement and taskspace unit gates passed with 84 / 94 tests.
  B-tier proof must now show business_success=true and open_leaf_nodes=0 together.

R3-E first timing fix landed locally:
  provider_request_budget trace now emits model_request_duration_ms beside latency_ms.
  benchmark timing parser prefers provider_lifecycle terminal durations and keeps websocket timing
  as fallback.
  provider_request_budget, E3 score-validity, and cost instrumentation self-tests passed.
  B-tier / targeted diagnostic proof is still required before marking wait attribution done.

R3-E second timing fix landed locally:
  benchmark runner now reads model timing from artifact rollout.jsonl when present and falls back
  to whale-exec.jsonl only when rollout is unavailable.
  Same B-tier artifact proved rollout.jsonl reports provider_lifecycle_timing while whale-exec.jsonl
  reports jsonl_without_timing.
  E3 score-validity self-test passed.
```

## 0.6 R2 blocker 到 R3 phase 映射

| R2 Blocker | R3 Owner Phase | Notes |
|---|---|---|
| `replacement_confirmed=false` | R3-A/B/C | 上下文编译器成为唯一 provider-visible TaskSpace context 入口 |
| `legacy_taskspace_history_present=true` | R3-A/C | 旧 taskspace control/call/output 只保留为 refs 或 audit |
| `raw_taskspace_control_history_tokens=917` | R3-B/C | 结构化验证区分合法 action enum 和旧历史正文 |
| `protected_items_present=false` | R3-B | bundle 显式包含 protected evidence refs，不靠自然语言扫描 |
| `provider_direct_input_output_ratio=12.9726` | R3-A/C/F | 拆出 stable/semi-stable/dynamic，证明动态区受控 |
| `projection_token_share=0.0022` | R3-A/C | 说明大头不在 projection，必须管整个 provider-visible payload |
| `request_2_plus_hit_rate` 需要持续维护 | R3-A/F | cache planner 产出 stable hash、dynamic hash、risk reason |
| `open_leaf_nodes=1` | R3-D | graph lifecycle blocker，不属于 context compiler 单独职责 |
| `taskspace_wall_time_ratio=3.07` | R3-A/E/F | 上下文和 cache 可降耗，但 timing 需要独立归因 |
| `model_request_duration_ms` missing | R3-E | telemetry blocker |
| code-complete/user approval marker missing | R3-F | release/start gate blocker |

## 0.7 总门禁

R3 不能被视为完成，除非以下条件同时满足：

```text
cargo test -p codex-core taskspace --lib passes
cargo test -p codex-core active_context_replacement --lib passes
cargo test -p codex-core provider_request_budget --lib passes
pwsh -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1 passes
pwsh -File scripts/taskspace-benchmark/test-release-decision.ps1 passes
pwsh -File scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1 status=pass
B-tier single-file-fast-fix business_success=true
B-tier public_validation_exit_code=0
B-tier hidden_oracle_exit_code=0
B-tier exact_context_bundle_verified=true
B-tier replacement_confirmed=true
B-tier raw_taskspace_history_tokens=0
B-tier protected_items_verified=true
B-tier request_2_plus_hit_rate >= 0.95
B-tier cache_usage_missing_count=0
B-tier native_tools_schema_hot_path_count=0
B-tier open_leaf_nodes=0
B-tier wait_attribution_status=complete
targeted diagnostic passes before formal E3
formal E3 start gate explicitly allows terminal-bench_E3-P0_3_5
```

## 0.8 依赖

| Dependency | Type | Current Status | Blocking Risk | Handling Plan |
|---|---|---|---|---|
| DeepSeek provider usage/cache fields | third-party | Available in B-tier evidence | Missing usage would block cache proof | Keep provider-cache summary as hard gate |
| Existing TaskSpace state/map/runtime schemas | system | Available | Schema drift can break compiler | R3-A must pin input/output structs and tests |
| Existing action-contract transport | system | Available after R2 ABI repair | Prompt path can drift from ordinary provider path | Route both through compiler profiles |
| Benchmark scripts | system | Available | Release gate may still accept weak evidence | R3-F updates gates before formal E3 |
| Formal user approval marker | person/process | Missing | Formal E3 must not run | R3-F keeps it explicit |

## 0.9 Review policy

R3 changes are high-risk core-runtime changes. Each implementation PR should include:

```text
design diff or doc pointer
focused unit tests
benchmark fixture update
runtime trace evidence
benefit validation result
release rollback/fallback note
```

After code changes land, ask whether to run adversarial review before formal E3.
