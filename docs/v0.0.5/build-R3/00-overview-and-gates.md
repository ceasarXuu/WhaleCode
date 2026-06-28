# build-R3 总览和门禁

> build-R3 是 build-R2 Phase H 之后的收敛工作集。目标不是重新做 A-H，而是把
> R2 暴露出的真实 blocker 归并到一个可实现、可验证、可回滚的工程计划里。

## 0.1 元数据

```text
Created: 2026-06-26
Updated: 2026-06-28
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

R3-E third timing fix landed locally:
  benchmark timing parser now derives model_queue_wait_ms from provider lifecycle
  started_at_ms -> stream_opened.createdAtMs and records model_retry_backoff_ms=0
  when no retry attempts are observed.
  E3 score-validity and harness guardrails self-tests passed.
  B-tier `target\phase-r3-btier-smoke-20260627-041043` proved:
    wait_attribution_status=complete
    runtime_optimization_status=ready
    model_request_duration_ms=166112
    model_queue_wait_ms=9952
    model_retry_backoff_ms=0

B-tier correctness / graph / context / cache / timing gates now have real evidence:
  business_success=true
  exec_exit_code=0
  public_validation_exit_code=0
  hidden_oracle_exit_code=0
  open_leaf_nodes=0
  exact_context_bundle_verified=true
  replacement_confirmed=true
  raw_taskspace_control_history_tokens=0
  request_2_plus_hit_rate=0.986813
  cache_usage_missing_count=0
  native_tools_schema_hot_path_count=0

Remaining non-convergence:
  the same B-tier pair outcome is both_success_taskspace_cost_higher.
  taskspace_wall_time_ratio=4.87 and taskspace_tool_call_ratio=1.38.
  Therefore R3 can claim instrumentation and lifecycle correctness, but cannot claim speedup/cost saving.
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

## 0.10 2026-06-27 Current Status

R3 已在 B-tier `single-file-fast-fix` 上证明：

```text
business_success = true
exec_exit_code = 0
open_leaf_nodes = 0
exact_context_bundle_verified = true
replacement_confirmed = true
raw_taskspace_control_history_tokens = 0
request_2_plus_hit_rate = 0.986813
cache_usage_missing_count = 0
native_tools_schema_hot_path_count = 0
wait_attribution_status = complete
```

低内存脚本级门禁已复跑通过：

```text
test-release-decision.ps1 = PASS
test-cost-instrumentation.ps1 = PASS
test-v005-non-agent-gates-builder.ps1 = PASS
test-e3-start-gate.ps1 = PASS
```

仍未完成：

```text
current-HEAD build-v005-non-agent-gates.ps1 formal run
targeted diagnostic terminal-bench_E3-P0_1_1
code-complete marker
explicit user approval marker
formal E3 start gate
formal terminal-bench_E3-P0_3_5
```

当前暂停原因：

```text
FreePhysicalMemory ~= 2.27GB
formal non-agent gates include cargo test
continuing under this RAM state risks pagefile pressure and host instability
```

下一步先释放本机内存，或确认允许结束无关高占用进程，再执行正式 non-agent gates。

## 0.11 2026-06-28 Targeted Diagnostic Current Status

R3 继续推进到 terminal-bench targeted diagnostic 后，发现 B-tier 已收敛的
context/cache/graph/timing 能力仍不足以覆盖 E3-like 外部任务中的两类真实问题：

```text
target sample:
  terminal-bench processing-pipeline
  source_version = 1a6ffa9

new blockers observed:
  implement_solution 在已经有高信号证据后仍可能继续 read/search，而不是立刻 patch
  Windows 本机 Bash/WSL validator infra 失败会被当成普通测试失败，导致重复诊断或 turn.failed
```

已落地的 R3-D/F 延伸修复：

```text
implementation_needs_edit:
  当 implement_solution 节点已有 dependency-working / mandatory-evidence 证据，
  且没有成功 edit 时，后续 read/search 会被收敛为 apply_patch 或 blocked。

mandatory evidence target:
  如果已发现的高信号证据指向具体文件，例如 generate_report.sh，
  后续 apply_patch 必须覆盖该文件，否则被拒绝为 missing mandatory evidence target。

local validator infra terminalization:
  Bash/Service/CreateInstance/E_ACCESSDENIED、UTF-16/NUL 形式的同类输出、
  PowerShell InvalidEndOfLine 等本机 validator infra 失败不再驱动重复测试。
  它们会被提升为 validation node blocked / invalid validation evidence。

terminal action final gate:
  当本轮已经观察到 terminal TaskSpace action，例如 blocked，
  final-response gate 不再把 runtime 生成的 blocked final candidate 当成普通无动作回答拒绝。
```

最新真实验证：

```text
run:
  target\phase-r3-targeted-diagnostic-20260628-110353\runs\terminal_bench__processing-pipeline\20260628-110410-426

TaskSpace right side:
  exec_exit_code = 0
  business_success = true
  public_validation_exit_code = 0
  hidden_oracle_exit_code = 0
  open_leaf_nodes = 0
  turn.failed = absent
  tool_call_count = 30
  rollout_trace_model_request_count = 35

cache:
  trace_coverage = 1
  cache_usage_missing_count = 0
  native_tools_schema_hot_path_count = 0
  request_2_plus_hit_rate = 0.984414
  request_2_plus_cached_input_tokens = 3912576
  request_2_plus_uncached_input_tokens = 61946
```

当前仍未完全收敛：

```text
outcome_taskspace = engineering_unclean
engineering_unclean_reasons:
  active_sentinel_warning:validator_failure
  e3_external_validator_fidelity_unproven
  e3_external_validator_not_e3_eligible

graph-health warning:
  high_blocked_node_ratio
```

解释：

```text
本轮已经证明 targeted diagnostic 的业务正确性、graph closeout、terminalization
和 cache hit 都成立；但 benchmark 工程清洁度仍把本机 Bash E_ACCESSDENIED
归为 validator_failure，且该 run 仍是 diagnostic，不是 E3 eligible formal run。

因此 R3 可以继续声明“targeted diagnostic blocker 已从 turn.failed/open leaf
收敛为显式 blocked validation node”，但仍不能声明 formal E3 完成或速度/成本收益完成。
```

## 0.12 2026-06-28 Targeted Diagnostic Sentinel Clean Follow-up

针对 0.11 的 `active_sentinel_warning:validator_failure` 残留，继续定位后确认：

```text
sentinel source:
  node-3 / result-33 / trace-448
  call_id = taskspace-action-contract-32-run_test
  action_class = test
  body = Tool call failed before producing a result.

actual whale-exec command output:
  Bash/Service/CreateInstance/E_ACCESSDENIED
```

根因不是 runtime local-infra detector 失效，而是 tool failure 持久化路径过度脱敏：
`FunctionCallError::RespondToModel` 被写成固定占位语，ActionMap 看不到可分类的稳定错误码。

修复后只保留脱敏后的 canonical infra signal，不写入任意 raw error：

```text
Tool call failed before producing a result.
local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED
```

最新真实验证：

```text
run:
  target\phase-r3-targeted-diagnostic-20260628-114800\runs\terminal_bench__processing-pipeline\20260628-114818-716

TaskSpace right side:
  outcome_taskspace = solved
  exec_exit_code = 0
  business_success = true
  public_validation_exit_code = 0
  hidden_oracle_exit_code = 0
  open_leaf_nodes = 0
  active_sentinel_warning_count = 0
  turn.failed = absent

efficiency movement on this diagnostic sample:
  previous provider_request_count = 34
  current provider_request_count = 16
  previous tool_call_count = 30
  current tool_call_count = 10
  taskspace_tool_call_ratio = 0.18
  taskspace_wall_time_ratio = 0.95

cache:
  request_2_plus_hit_rate = 0.982693
  cache_usage_missing_count = 0
  native_tools_schema_hot_path_count = 0
```

仍未完成：

```text
engineering_unclean_reasons:
  e3_external_validator_fidelity_unproven
  e3_external_validator_not_e3_eligible

这两个原因来自 targeted diagnostic 的外部 validator fidelity / E3 eligibility，
不是 TaskSpace runtime graph、cache、sentinel 或业务正确性失败。
```

## 0.13 2026-06-28 Non-Agent Gates

current-HEAD formal non-agent gates 已通过：

```text
artifact:
  target\phase-r3-non-agent-gates-20260628-120740\v005-non-agent-gates.json

status = pass
git_commit = 00121c5fd516c543312836d132954debac8b915c
task_list_hash = terminal-bench-processing-pipeline@1a6ffa9
profile_hash = taskspace-v005-active__deepseek-v4-flash__reasoning-max
source_version = terminal-bench@1a6ffa9
generated_at = 2026-06-28T12:07:41.3219985+08:00
```

各 gate：

```text
provider_request_hook      pass  4199ms
runtime_budget_response    pass  4199ms
budget_quality_impact      pass  2007ms
active_context_replacement pass  3885ms
state_commit_displacement  pass  3739ms
spawn_node_budget          pass  4407ms
request_phase_attribution  pass  2007ms
release_decision_fixture   pass  88213ms
start_gate_fixture         pass  171287ms
```

当前 R3 可以继续推进到 release marker / formal E3 start-gate 设计，但还不能自动进入
formal E3，因为 `explicit user approval marker` 按 R3-F 规则必须由用户确认后才能生成。

## 0.14 2026-06-28 Formal E3 Preflight Update

用户已在聊天中批准继续 formal E3，但正式启动前的 plan-only calibration 暴露
multi-sample runner 参数契约问题：

```text
Cannot bind parameter because parameter 'SampleNames' is specified more than once.
```

已修复范围：

```text
scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1
scripts\taskspace-benchmark\run-taskspace-external-benchmark.ps1
scripts\taskspace-benchmark\run-taskspace-benchmark.ps1
scripts\taskspace-benchmark\adapters\external-benchmark-common.ps1
scripts\taskspace-benchmark\adapters\terminal-bench-adapter.ps1
```

验证：

```text
test-e3-start-gate.ps1 = PASS
test-external-wrapper-harness.ps1 = PASS
test-terminal-bench-adapter-harness.ps1 = PASS

formal plan-only terminal-bench_E3-P0_3_5:
  status = completed
  suite_score_valid = true
  score_valid_child_runs = 3
  score_invalid_child_runs = 0
  profile_hash = c04582a682c487647ffea44b9f6a2010a23619c0724a1d8a1a09c538b01f0bd4
```

当前 gating 影响：

```text
修复改变 suite runner script SHA
formal profile_hash 必须重新计算
formal v005-non-agent-gates 必须在修复后的新 HEAD 上重跑
formal E3 start gate 仍需 calibration evidence + 当前 HEAD markers 同时通过
```

## 0.15 2026-06-28 Formal E3 Start-Gate Semantics

formal start gate 的 calibration 语义已调整为：

```text
允许正式 E3 先运行并生成真实 calibration / timing evidence；
不允许在 calibration gate 未通过时宣称 speedup / cost saving。
```

调整原因：

```text
formal terminal-bench_E3-P0_3_5 的 serial calibration evidence 需要由正式 suite
或同等身份绑定的 suite run 产生。
旧规则要求 start gate 先看到 calibration evidence 才允许 formal E3，
但 calibration evidence 又依赖 formal E3 才能生成，形成循环依赖。
```

当前规则：

```text
当 task list、profile、source version、current-HEAD non-agent gates、
code-complete marker 和 explicit user approval marker 全部匹配时，
start gate 可以在 calibration_gate=skipped_allowed 的情况下放行 full_e3。

此时：
  full_e3_allowed = true
  speed_claim_allowed = false
  calibration_gate_passed = false
  calibration_gate_skipped_allowed = true

release decision / speed claim 仍必须等待 calibration_gate_passed=true。
```

验证：

```text
git diff --check = PASS
test-e3-start-gate.ps1 = PASS
test-release-decision.ps1 = PASS
```

## 0.16 2026-06-28 Optional Wrapper Args

current HEAD plan-only formal E3 继续暴露 suite child runner 的可选参数契约问题：

```text
Missing an argument for parameter 'ApprovalMarkerSha256'.
```

根因：

```text
run-taskspace-e3-suite.ps1 无条件把可选 marker hash/path 加入 child process args。
当 marker 尚未生成或 plan-only 不需要 marker 时，值为空字符串。
Windows PowerShell 跨 native process 调用时空字符串参数不稳定，
下游脚本看到 -ApprovalMarkerSha256，但没有收到有效参数值。
```

修复：

```text
suite child args 对所有可选 string 参数统一执行 non-empty guard：
  ApprovalMarkerSha256
  CodeCompleteMarkerSha256
  V005NonAgentGatesPath
  V005CodeCompleteMarkerPath
  V005UserApprovalMarkerPath

必需身份字段仍保持强传递：
  TaskListHash
  ProfileHash
  SourceVersion
  SampleSetId
  SuiteReceiptPath / SuiteReceiptSha256
```

验证：

```text
formal plan-only terminal-bench_E3-P0_3_5:
  status = completed
  suite_score_valid = true
  score_valid_child_runs = 3
  score_invalid_child_runs = 0
  task_list_hash = de1c223db57ea05e0c87839bb9d13677eb4faa84d3a3830df2b36d7e0ecac5a2
  profile_hash = 2aebff6baaf60a71367f9c999e93a1fd01a140257d48e4cee8378fccb0cbc013
```

## 0.17 2026-06-28 Formal E3 Full-Run State Fix

正式 E3 full run 继续暴露两个与 agent 解题无关的 harness 问题。

第一，深层 run root 在 Windows 上触发 Git loose-object 路径预算问题：

```text
deep root = target\phase-r3-formal-e3-20260628-170557\formal-run-*
failure = invalid object 100644 ... for 'Dockerfile'
failed loose-object temp path length = 281
short root = target\e3f-final
result = workspace materialization passed, real agent pairs executed
```

当前操作规则：

```text
formal E3 full run root 使用短路径，例如 target\e3f-*。
长期修复应把 Windows run-root path budget 纳入 suite runner 自动策略。
```

第二，短路径 formal run 的 `processing-pipeline` 跑完 5 对后进入人工审查状态，
但旧 suite 状态机把 `score_block_reason=audit_required` 错误升级为
`invalid_harness / score_invalid`，并熔断剩余样本。

真实现场：

```text
SuiteRoot = target\e3f-final\suite-20260628-184253
sample = processing-pipeline
run_validity = valid
phase = audit_required
attempted_pairs = 5
completed_pairs = 5
engineering_unclean_count = 0
audit_required_count = 5
score_block_reason = audit_required
cost gate = PASS
direct_input_output_ratio = 1.1877
walltime_ratio = 0.6001
provider request 2+ cache hit rate = 0.984232
semantic_replacement_rate = 0.5299
protected_miss_count = 0
```

修复后的状态语义：

```text
audit_required != invalid_harness

score_block_reason=audit_required 且 score_invalid_reason 为空时：
  child run_validity 保持 valid
  suite status = audit_required
  suite_score_ready = false
  suite_score_valid = false
  score_pending_audit_child_runs > 0
  emit event suite_score_pending_audit

真正 engineering_unclean / score_invalid 仍然保持 invalid_harness 熔断。
```

验证：

```text
git diff --check = PASS
test-e3-score-validity.ps1 = PASS
test-e3-harness-guardrails.ps1 = PASS
test-e3-start-gate.ps1 = PASS
test-release-decision.ps1 = PASS
```

影响：

```text
该修复改变 suite runner script SHA 和 formal profile_hash。
commit 后必须重新生成 formal non-agent gates、code-complete marker、user-approval marker、
并重新跑 formal start gate。
下一轮 formal E3 预期不再因为 pending human audit 熔断；
若所有样本执行完成但未人审，正确终态应为 audit_required。
```

## 0.18 2026-06-28 Terminal-Bench Build Network Contract

pending-audit 状态机修复后的正式 E3 继续推进到第二个样本：

```text
SuiteRoot = target\e3f-after-pending-audit-fix\suite-20260628-202449
sample = multi-source-data-merger
pair = pair-001
abort_signature = harness_materialization_failure/docker_build_environment_failure
```

根因不是 agent 解题失败，而是外部 validator 的 Docker build 网络契约不完整：

```text
Dockerfile:
  FROM python:3.11-slim
  RUN apt-get update && apt-get install -y tmux asciinema

host proxy:
  HTTP_PROXY / HTTPS_PROXY = http://127.0.0.1:7890

旧 validator:
  docker_backend = wsl
  proxy_env_skipped_loopback = HTTP_PROXY / HTTPS_PROXY / http_proxy / https_proxy
  proxy_env_count = 0
  docker build 未传 --build-arg proxy

失败现象:
  Unable to connect to deb.debian.org:http
  Package 'tmux' has no installation candidate
  Unable to locate package asciinema
```

验证性探针证明 WSL Docker 在 `--network host` 下可以访问该 loopback proxy：

```text
docker run --rm --network host python:3.11-slim ...
proxy_connect = ok
```

修复后的契约：

```text
WSL backend:
  run/build 均使用 host networking
  loopback proxy 不再跳过，记录 proxy_env_preserved_loopback

Docker build:
  对 HTTP_PROXY / HTTPS_PROXY / http_proxy / https_proxy 等变量传 --build-arg
  docker-build-result.json 记录 proxy_env_count / proxy_build_arg_count

native backend:
  仍把 localhost / 127.0.0.1 proxy 改写为 host.docker.internal
```

验证：

```text
test-terminal-bench-adapter-harness.ps1 = PASS
test-terminal-bench-docker-cache-smoke.ps1 = PASS
test-external-wrapper-harness.ps1 = PASS

no-agent multi-source-data-merger validator probe:
  proxy_env_count = 4
  proxy_build_arg_count = 4
  docker build phase = ok
  docker run phase = docker_run_failure
```

其中 `docker_run_failure` 是直接运行未解题 fixture 的预期测试失败，
缺少 `/app/merged_users.parquet` 和 `/app/conflicts.json`，不再是 apt/Docker build
环境失败。

影响：

```text
该修复改变 terminal-bench-adapter.ps1 SHA。
formal profile_hash 会变化；commit 后必须重新生成 current-HEAD gates / markers，
再以短 run root 重跑 formal E3。
```

后续 suite start gate 还暴露一个测试契约同步问题：

```text
failed gate = cheap_self_tests
failed command = .\scripts\taskspace-benchmark\test-harness.ps1
old expectation = proxy_env_skipped_loopback
new contract = proxy_env_preserved_loopback + proxyBuildArgs
```

已同步 `test-harness.ps1`：

```text
test-harness.ps1 = PASS
test-e3-start-gate.ps1 = PASS
git diff --check = PASS
```

该问题只影响 gate fixture，不代表正式样本或 agent 解题失败。
