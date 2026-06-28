# Phase R3-F. Validation and Closeout

## F.1 目标

把 R3-A 到 R3-E 的修复转化为可发布前证据。R3-F 的重点是证明真实收益：

```text
不是只证明测试通过。
不是只证明 payload 有 hash。
不是只证明 B-tier 解题成功。
必须证明上下文污染消失、cache 命中稳定、graph closeout 完成、timing 可归因。
```

## F.2 验证顺序

必须按顺序执行：

```text
1. focused unit tests
2. PowerShell fixture tests
3. current-HEAD non-agent gates
4. B-tier single-file-fast-fix smoke
5. targeted diagnostic terminal-bench_E3-P0_1_1
6. code-complete marker
7. explicit user approval marker
8. formal E3 start gate
9. formal terminal-bench_E3-P0_3_5
```

不得跳过 B-tier 直接跑 formal E3。

## F.3 Commands

```powershell
cargo test -p codex-core taskspace --lib
cargo test -p codex-core active_context_replacement --lib
cargo test -p codex-core provider_request_budget --lib
cargo test -p codex-core budget --lib

pwsh -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1
pwsh -File scripts/taskspace-benchmark/test-release-decision.ps1
pwsh -File scripts/taskspace-benchmark/test-e3-start-gate.ps1

pwsh -File scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1 `
  -RunRoot target\phase-r3-non-agent-gates `
  -TaskListHash <task-list-hash> `
  -ProfileHash <profile-hash> `
  -SourceVersion <source-version>
```

B-tier smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 `
  -Scenario single-file-fast-fix `
  -Repeats 1 `
  -RunRoot target\phase-r3-btier-smoke `
  -TimeoutSeconds 900 `
  -ValidationTimeoutSeconds 180 `
  -ValidationPretestTimeoutSeconds 60 `
  -ValidationTestTimeoutSeconds 180 `
  -SandboxMode workspace-write `
  -EnableAggregate `
  -AllowNonE2Result `
  -WhaleBin D:\BuildCache\whalecode\cargo-target\debug\whale.exe
```

Targeted diagnostic only after B-tier passes all R3 gates:

```powershell
pwsh -File scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1 `
  -SampleSet terminal-bench_E3-P0_1_1 `
  -EvidenceTarget diagnostic-only `
  -Profile taskspace-v005-active
```

Formal E3 only after start gate:

```powershell
pwsh -File scripts/taskspace-benchmark/lib/e3-start-gate.ps1 `
  -ExpectedSampleSetId terminal-bench_E3-P0_3_5 `
  -V005NonAgentGatesPath <run-root>\v005-non-agent-gates.json `
  -V005CodeCompletePath <run-root>\v005-code-complete.json `
  -V005UserApprovalPath <run-root>\v005-user-approval.json
```

## F.4 Benefit gates

| Benefit | Baseline | Target | Evidence |
|---|---|---|---|
| Active replacement proof | Phase H `replacement_confirmed=false` | `replacement_confirmed=true` and `exact_context_bundle_verified=true` | active-context-replacement-report |
| Raw TaskSpace history removal | Phase H `raw_taskspace_control_history_tokens=917` | `raw_taskspace_history_tokens=0` | exact payload scan |
| Protected evidence preserved | Phase H `protected_items_present=false` | `protected_items_verified=true` | bundle manifest |
| Cache hit maintained | Phase H `request_2_plus_hit_rate=0.989783` | `>= 0.95` | provider-cache summary |
| Context cost controlled | Phase H `provider_direct_input_output_ratio=12.9726` | below agreed threshold or blocked with diagnosis | cost diagnostics |
| Graph closeout | Phase H `open_leaf_nodes=1` | `open_leaf_nodes=0` | graph health |
| Timing attribution | Phase H missing `model_request_duration_ms` | `wait_attribution_status=complete` | sample timing |
| Business correctness | Phase H B-tier passed | remains passed | public/hidden validation |

## F.5 Release blockers

Release-like claims remain blocked if any of these are true:

```text
context bundle missing
exact_context_bundle_verified != true
provider payload hash mismatch
raw_taskspace_history_tokens > 0
protected_items_verified != true
cache_plan_verified != true
request_2_plus_hit_rate < 0.95
trace_coverage < 0.99
cache_usage_missing_count > 0
native_tools_schema_hot_path_count > 0
open_leaf_nodes > 0
wait_attribution_status != complete
model_request_duration_ms missing
business_success != true
public or hidden validation exit code != 0
current code-complete marker missing
explicit user approval marker missing
```

## F.6 完成证据矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Non-agent gates | current HEAD evidence package | benchmark scripts | gate builder | builder tests | v005-non-agent-gates.json | none | planned |
| B-tier smoke | real benefit proof | CLI benchmark | whale binary | benchmark run | pair artifacts | none | planned |
| Targeted diagnostic | E3-like blocker check | E3 suite | diagnostic sample | suite output | diagnostic ledger | none | planned |
| Formal start gate | prevents premature formal E3 | start gate script | release process | start gate tests | gate-decision.json | none | planned |

## F.7 Exit criteria

```text
R3-A through R3-E exit criteria are all satisfied.
All focused and script gates pass on current HEAD.
B-tier smoke passes business, cache, context, graph, timing gates.
Targeted diagnostic passes or records a new blocker without release claim.
Code-complete marker exists only after all current blockers are closed.
Formal E3 is not run until explicit user approval marker exists.
```

## F.8 Closeout note

If targeted diagnostic finds a new blocker, create `build-R3/07-<blocker-name>.md`
instead of expanding this closeout file indefinitely.

## F.9 当前执行状态

2026-06-27 current-HEAD non-agent gates 首次执行结果：

```text
status = fail
failed_gate = start_gate_fixture
reason = wrapper timeout after 240 seconds
```

复核结果：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1
PASS
duration ~= 202 seconds
```

根因：`build-v005-non-agent-gates.ps1` 对 `start_gate_fixture` 的 240 秒超时在 Windows 上过紧，fixture 单独运行已经接近该边界；wrapper 叠加进程启动、输出捕获和系统调度后会误判为 timeout。修复为将该 gate timeout 调整到 420 秒。该修复不改变通过标准，仍要求 start gate fixture exit code 为 0 且不超时。

## F.10 2026-06-27 B-tier blocker 修复状态

本轮 B-tier smoke 暴露两个 release blocker：

```text
wait_attribution_status = missing
missing field = model_request_duration_ms
open_leaf_nodes = 1
```

已确认并修复：

```text
Timing:
  runner 之前读取 whale-exec.jsonl，里面没有 provider_lifecycle timing。
  rollout.jsonl 同一 run 可解析 16 个 terminal provider events，
  model_request_duration_ms = 117196。
  现已改为优先读取 artifact rollout.jsonl。

Graph:
  explicit taskspace_control(action=finish_node) 被 session/turn.rs
  在 successful validation 分支里替换成 final_answer，导致没有执行 taskspace_control。
  现已删除该替换分支，显式 lifecycle action 必须先落 runtime state。
```

已验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-score-validity.ps1
PASS

cargo test -p codex-core active_context_replacement --lib
83 passed

cargo test -p codex-core taskspace --lib
93 passed
```

下一步验证顺序：

```text
1. cargo build -p codex-cli --bin whale
2. rerun B-tier single-file-fast-fix
3. inspect pair artifacts:
   - business_success=true
   - exact_context_bundle_verified=true
   - replacement_confirmed=true
   - request_2_plus_hit_rate >= 0.95
   - open_leaf_nodes=0
   - model_request_duration_ms present
   - wait_attribution_status=complete
```

## F.11 2026-06-27 第二轮 B-tier 结果

使用 fresh target binary：

```text
D:\whalecode-alpha\target\phase-r3-current-cargo-target\debug\whale.exe
```

运行：

```text
target\phase-r3-btier-smoke-20260627-012652\single-file-fast-fix\20260627-012654-869
```

已证明收益：

```text
open_leaf_nodes = 0
model_request_duration_ms = 503738
model_timing_source_status = provider_lifecycle_timing
model_timing_source_path = pair-001\right\artifacts\rollout.jsonl
public_validation_exit_code = 0
hidden_oracle_exit_code = 0
```

仍未通过：

```text
TaskSpace exec_exit_code = 1
TaskSpace business_success = false
pair outcome = taskspace_worse
wait_attribution_status = missing
missing fields = model_queue_wait_ms, model_retry_backoff_ms
```

根因拆分：

```text
exec_exit_code=1:
  graph closeout 后没有 active node，但 action-contract prompt 仍诱导继续创建/读取节点。
  修复为 no-active-node completed-task final_answer prompt + runtime guard。

wait_attribution_status=missing:
  model_request_duration_ms 已补齐；剩余缺口是 provider queue/retry wait telemetry 未实现。
  这不影响 graph closeout 证明，但仍阻塞 speedup/release claim。
```

下一步：

```text
1. 提交 no-active-node final-answer 修复
2. fresh build 当前 commit
3. 重跑 B-tier，要求 business_success=true 且 open_leaf_nodes=0
4. 单独处理 queue/retry wait attribution，不能把 timing release gate 标成 complete
```

## F.12 2026-06-27 第三轮 B-tier 验证阻塞

已提交 no-active-node final-answer 修复：

```text
76e0b96e Close TaskSpace no-active-node final answer path
```

但第三轮 B-tier 尚未启动。原因不是当前修复出现源码编译错误，而是本机无法产出
该 commit 之后的 fresh `whale.exe`：

```text
现有 binary:
  D:\whalecode-alpha\target\phase-r3-current-cargo-target\debug\whale.exe
  LastWriteTimeUtc = 2026-06-26T17:26:35.1780363Z

当前 HEAD:
  76e0b96eee254cad6ced962958e633732e3d1796
  commit time = 2026-06-27T02:05:21+08:00
```

构建阻塞现场：

```text
fresh target build:
  rustc-LLVM ERROR: out of memory

existing target incremental build:
  reached codex-cli final binary compile
  failed with memory allocation of 2097152 bytes failed

dev-small profile build:
  failed with 页面文件太小，无法完成操作。 (os error 1455)

host snapshot:
  FreePhysicalMemory ~= 2.3GB before retry, then ~= 1.6GB
  PageFile AllocatedBaseSize = 49152 MB
  PageFile CurrentUsage = 22207 MB
```

判断：

```text
blocked_by = local_windows_commit_or_pagefile_pressure
not_blocked_by = rust_source_compile_error
not_yet_proven = 76e0b96e B-tier business_success/open_leaf_nodes benefit
```

恢复条件：

```text
1. 释放本机内存或提高 Windows commit limit/pagefile 可用量
2. 重跑:
   cargo build -j1 --profile dev-small -p codex-cli --bin whale
3. 使用 post-commit whale.exe 重跑 B-tier single-file-fast-fix
4. 要求同时证明:
   - business_success=true
   - exec_exit_code=0
   - open_leaf_nodes=0
   - model_request_duration_ms present
5. timing release gate 仍需单独补齐 queue/retry wait telemetry
```

## F.13 2026-06-27 第三轮 B-tier 结果

释放本机内存后，使用低内存 profile 成功构建 post-commit binary：

```text
cargo build -j1 --profile dev-small -p codex-cli --bin whale

D:\whalecode-alpha\target\phase-r3-current-cargo-target\dev-small\whale.exe
LastWriteTimeUtc = 2026-06-26T19:56:31.2306704Z
```

随后重跑 B-tier：

```text
target\phase-r3-btier-smoke-20260627-035703\single-file-fast-fix\20260627-035705-541
```

已证明 no-active-node final-answer 修复的直接收益：

```text
TaskSpace exec_exit_code = 0
TaskSpace business_success = true
public_validation_exit_code = 0
hidden_oracle_exit_code = 0
open_leaf_nodes = 0
exact_context_bundle_verified = true
replacement_confirmed = true
request_2_plus_hit_rate = 0.987422
cache_usage_missing_count = 0
```

仍未通过 timing release gate：

```text
wait_attribution_status = missing
missing fields = model_queue_wait_ms, model_retry_backoff_ms
```

## F.14 2026-06-27 第四轮 B-tier 结果

修复 benchmark timing parser 后，重新跑 B-tier：

```text
target\phase-r3-btier-smoke-20260627-041043\single-file-fast-fix\20260627-041044-436
```

已通过的 R3 gate：

```text
business_success = true
exec_exit_code = 0
public_validation_exit_code = 0
hidden_oracle_exit_code = 0
open_leaf_nodes = 0

exact_payload_scan_passed = true
exact_context_bundle_verified = true
cache_plan_verified = true
replacement_confirmed = true
legacy_taskspace_history_present = false
raw_taskspace_control_history_tokens = 0
protected_items_present = true

request_2_plus_hit_rate = 0.986813
cache_usage_missing_count = 0
native_tools_schema_hot_path_count = 0

wait_attribution_status = complete
runtime_optimization_status = ready
model_request_duration_ms = 166112
model_queue_wait_ms = 9952
model_retry_backoff_ms = 0
```

仍不能宣称的收益：

```text
pair outcome = both_success_taskspace_cost_higher
taskspace_wall_time_ratio = 4.87
taskspace_tool_call_ratio = 1.38
```

结论：

```text
R3 的 correctness / graph closeout / context replacement / cache-hit / timing attribution
在 B-tier single-file-fast-fix 上已有真实证据。

但 speedup/cost saving 不能宣称。当前真实证据显示 TaskSpace 在该样本上仍更慢、更贵。
下一阶段应把优化目标从“补齐观测字段”切换到“降低 TaskSpace 请求数、模型时长和重复动作”。
```

## F.15 2026-06-27 脚本级门禁复跑与资源状态

已在当前 HEAD 复跑低内存脚本级门禁：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1
PASS
duration ~= 88 seconds

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
PASS

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-v005-non-agent-gates-builder.ps1
PASS

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1
PASS
duration ~= 165 seconds
```

正式 current-HEAD non-agent gates 暂缓执行。原因不是门禁脚本失败，而是本机资源风险：

```text
FreePhysicalMemory ~= 2.27GB
FreeVirtualMemory ~= 25.68GB

Top memory holders included:
  vmmemWSL ~= 959MB working set
  Codex processes ~= 827MB combined working set in top two entries
  MsMpEng ~= 460MB working set
  multiple opencode processes ~= 300MB working set each
```

`build-v005-non-agent-gates.ps1` 的正式模式会串行触发多组 `cargo test`。在当前 RAM 状态下继续执行存在较高页面文件压力和系统失稳风险，因此按资源门禁暂停。下一步需要先释放本机内存，或确认允许结束无关高占用进程；随后再执行正式 non-agent gates。

注意：递归扫描整个 `target` 查找 marker 在当前目录体量下发生超时，因此后续 marker 检查应只针对已知 run root，不再全量递归扫构建产物目录。

## F.16 2026-06-28 targeted diagnostic 修复与收益证明

在 B-tier gates 已经通过后，继续执行 targeted diagnostic：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-external-benchmark.ps1 `
  -Benchmark terminal-bench `
  -TaskDir target\terminal-bench-pinned-1a6ffa9\original-tasks\processing-pipeline `
  -SampleId processing-pipeline `
  -SourceVersion 1a6ffa9 `
  -Repeats 1 `
  -RunRoot target\phase-r3-targeted-diagnostic-20260628-110353 `
  -WhaleBin target\phase-r3-current-cargo-target\dev-small\whale.exe `
  -Model deepseek-v4-flash `
  -TimeoutSeconds 900 `
  -ValidationTimeoutSeconds 420 `
  -ValidationPretestTimeoutSeconds 120 `
  -ValidationTestTimeoutSeconds 420 `
  -SandboxMode full-auto `
  -ConfigOverride 'model_reasoning_effort=max' `
  -AllowStaleWhaleBin `
  -EnableAggregate
```

本轮验证的修复范围：

```text
1. implementation_needs_edit:
   防止 implement_solution 在证据充分但尚未 edit 时继续 rediscovery。

2. mandatory evidence target:
   如果 inspect evidence 指向 generate_report.sh，patch 必须覆盖该目标。

3. local validator infra detection:
   识别普通文本和 UTF-16/NUL 形态的 Bash/Service/CreateInstance/E_ACCESSDENIED。

4. compact state_commit / blocked terminalization:
   接受 DeepSeek 常见的 top-level state_commit，
   并把 local validator infra blocker 终止为显式 blocked validation node。

5. terminal action final gate skip:
   已观察到 terminal TaskSpace action 的请求，不再被普通 final-response gate
   误判为无动作回答。
```

验证结果：

```text
run:
  target\phase-r3-targeted-diagnostic-20260628-110353\runs\terminal_bench__processing-pipeline\20260628-110410-426

right / TaskSpace:
  exec_exit_code = 0
  business_success = true
  public_validation_exit_code = 0
  hidden_oracle_exit_code = 0
  wall_time_ms = 268848
  tool_call_count = 30
  rollout_trace_model_request_count = 35
  taskspace_control_count = 3
  open_leaf_nodes = 0
  turn.failed = absent

provider cache:
  provider_request_count = 34
  trace_coverage = 1
  cache_usage_missing_count = 0
  tool_free_action_contract_count = 34
  native_tools_schema_hot_path_count = 0
  request_2_plus_hit_rate = 0.984414
  request_2_plus_cached_input_tokens = 3912576
  request_2_plus_uncached_input_tokens = 61946
```

真实收益结论：

```text
targeted diagnostic blocker 已从:
  turn.failed + repeated validator diagnostics / open lifecycle uncertainty

收敛为:
  exec_exit_code=0
  business_success=true
  public/hidden validator pass
  open_leaf_nodes=0
  explicit local-validator-infra blocked evidence
  cache hit >= 0.95
```

仍阻塞 release / formal E3 的问题：

```text
outcome_taskspace = engineering_unclean
engineering_unclean_reasons:
  active_sentinel_warning:validator_failure
  e3_external_validator_fidelity_unproven
  e3_external_validator_not_e3_eligible

需要后续处理：
  benchmark 指标层把 local-validator-infra blocker 从 validator_failure 中拆出；
  targeted diagnostic 通过后，仍需要正式 current-HEAD non-agent gates；
  code-complete marker 与 explicit user approval marker 仍不存在；
  formal terminal-bench_E3-P0_3_5 仍未运行。
```

## F.17 2026-06-28 sentinel clean targeted rerun

针对 F.16 的 `active_sentinel_warning:validator_failure` 残留，新增脱敏错误分类修复后，
重建 binary：

```text
cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS

target\phase-r3-current-cargo-target\dev-small\whale.exe
SHA256 = 9E5B08528D6B11C5BAA742374CFBA193FFDE5F4EAB632384EE447AB04A777CEA
```

重跑同一个 targeted diagnostic：

```text
target\phase-r3-targeted-diagnostic-20260628-114800\runs\terminal_bench__processing-pipeline\20260628-114818-716
```

结果：

```text
outcome_taskspace = solved
engineering_unclean_reasons =
  e3_external_validator_fidelity_unproven
  e3_external_validator_not_e3_eligible

TaskSpace right side:
  exec_exit_code = 0
  business_success = true
  public_validation_exit_code = 0
  hidden_oracle_exit_code = 0
  wall_time_ms = 288513
  tool_call_count = 10
  rollout_trace_model_request_count = 17
  taskspace_control_count = 3
  state_commit_count = 1
  active_sentinel_warning_count = 0
  open_leaf_nodes = 0

provider cache:
  provider_request_count = 16
  trace_coverage = 1
  cache_usage_missing_count = 0
  tool_free_action_contract_count = 16
  native_tools_schema_hot_path_count = 0
  request_2_plus_hit_rate = 0.982693
  request_2_plus_cached_input_tokens = 1776000
  request_2_plus_uncached_input_tokens = 31278

relative movement vs previous targeted run:
  provider_request_count: 34 -> 16
  tool_call_count: 30 -> 10
  active_sentinel_warning_count: 1 -> 0
  outcome_taskspace: engineering_unclean -> solved
```

结论：

```text
R3 targeted diagnostic 当前已证明：
  business correctness pass
  graph closeout pass
  active sentinel clean
  cache hit maintained
  request/tool count materially reduced on the diagnostic sample

仍不能声明 formal E3 完成，因为该 targeted run 仍带有:
  e3_external_validator_fidelity_unproven
  e3_external_validator_not_e3_eligible
```

## F.18 2026-06-28 current-HEAD non-agent gates

执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\build-v005-non-agent-gates.ps1 `
  -RunRoot target\phase-r3-non-agent-gates-20260628-120740 `
  -TaskListHash 'terminal-bench-processing-pipeline@1a6ffa9' `
  -ProfileHash 'taskspace-v005-active__deepseek-v4-flash__reasoning-max' `
  -SourceVersion 'terminal-bench@1a6ffa9'
```

结果：

```text
status = pass
git_commit = 00121c5fd516c543312836d132954debac8b915c
artifact = target\phase-r3-non-agent-gates-20260628-120740\v005-non-agent-gates.json
```

gate 明细：

```text
provider_request_hook      pass  exit=0  timeout=false  duration_ms=4199
runtime_budget_response    pass  exit=0  timeout=false  duration_ms=4199
budget_quality_impact      pass  exit=0  timeout=false  duration_ms=2007
active_context_replacement pass  exit=0  timeout=false  duration_ms=3885
state_commit_displacement  pass  exit=0  timeout=false  duration_ms=3739
spawn_node_budget          pass  exit=0  timeout=false  duration_ms=4407
request_phase_attribution  pass  exit=0  timeout=false  duration_ms=2007
release_decision_fixture   pass  exit=0  timeout=false  duration_ms=88213
start_gate_fixture         pass  exit=0  timeout=false  duration_ms=171287
```

当前剩余 R3-F blocker：

```text
code-complete marker 尚未生成
explicit user approval marker 尚未生成
formal E3 start gate 尚未针对 terminal-bench_E3-P0_3_5 放行
formal terminal-bench_E3-P0_3_5 尚未运行
```

注意：本次 non-agent gates 绑定的是 targeted diagnostic 身份
`terminal-bench-processing-pipeline@1a6ffa9`，不是 formal E3 sample set 身份。
formal E3 之前必须生成匹配 formal sample set 的 marker，且需要显式用户批准。

## F.19 2026-06-28 formal E3 preflight blocker and fix

正式 E3 预检创建了 formal task list：

```text
task_list = target\phase-r3-formal-e3-20260628-170557\tasks-terminal-bench_E3-P0_3_5.jsonl
sample_set_id = terminal-bench_E3-P0_3_5
samples = processing-pipeline, multi-source-data-merger, recover-accuracy-log
source_version = 1a6ffa9674b571da0ed040c470cb40c4d85f9b9b
```

旧 HEAD 上首次 identity 计算得到：

```text
task_list_hash = de1c223db57ea05e0c87839bb9d13677eb4faa84d3a3830df2b36d7e0ecac5a2
profile_hash = 261ea8335c6ebcb27223e093d9bda58217b539e495a1f2686a820c7e50cd844c
```

正式身份绑定的 non-agent gates 在旧 HEAD 上通过：

```text
artifact = target\phase-r3-formal-e3-20260628-170557\non-agent-gates\v005-non-agent-gates.json
status = pass
git_commit = aad32edfe90698c73bddc47fa00ab29a534c2467
```

但随后 formal plan-only calibration 暴露一个真实 E3 harness blocker：

```text
Cannot bind parameter because parameter 'SampleNames' is specified more than once.
```

修复 `SampleNames` 后，下一层 wrapper 继续暴露 provenance 参数契约漂移：

```text
A parameter cannot be found that matches parameter name 'SuiteReceiptPath'.
```

根因：

```text
run-taskspace-e3-suite.ps1 对每个样本重复发出 -SampleNames <name>
run-taskspace-external-benchmark.ps1 也对下游 run-taskspace-benchmark.ps1 重复发出 -SampleNames <name>
PowerShell string[] 参数应当一次绑定，多值跟随同一个参数名
external wrapper 漏声明 SuiteReceiptPath / SuiteReceiptSha256，但 suite runner 已经传递，
downstream run-taskspace-benchmark.ps1 也已经支持这两个 provenance 参数
```

修复：

```text
两个 wrapper 均改为：
  -SampleNames <name1> <name2> <name3>
而不是：
  -SampleNames <name1> -SampleNames <name2> -SampleNames <name3>
external wrapper 补齐 SuiteReceiptPath / SuiteReceiptSha256 声明和透传
跨 powershell.exe -File 进程边界时，SampleNames 使用 CSV 单值传递，
入口脚本 normalize 回数组，避免 PowerShell string[] 参数绑定歧义
external materialization 改用 target\external-materialized\<hash> 短路径根，
run root 写 materialized-scenarios-pointer.json 保留证据指针
external common copy/hash helper 支持 Windows long path
```

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1
result = PASS
RunRoot = target\e3-start-gate-selftest\20260628-171620-334

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-external-wrapper-harness.ps1
result = PASS
RunRoot = target\external-wrapper-selftest\20260628-172201-777

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1
result = PASS
RunRoot = target\terminal-bench-adapter-selftest\20260628-175207-395

powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1 ...
mode = PlanOnly
SuiteRoot = target\phase-r3-formal-e3-20260628-170557\plan-after-short-materialization-root\suite-20260628-175509
status = completed
suite_score_valid = true
score_valid_child_runs = 3
score_invalid_child_runs = 0
sample_set_id = terminal-bench_E3-P0_3_5
task_list_hash = de1c223db57ea05e0c87839bb9d13677eb4faa84d3a3830df2b36d7e0ecac5a2
profile_hash = c04582a682c487647ffea44b9f6a2010a23619c0724a1d8a1a09c538b01f0bd4
```

影响：

```text
该修复会改变 run-taskspace-e3-suite.ps1 的脚本 SHA
profile_hash 必须重新计算
formal v005-non-agent-gates 必须在新 HEAD 上重跑
之前旧 HEAD formal non-agent gates 只能作为“发现问题前的证据”，不能继续用于 final start gate
```

## F.20 2026-06-28 start gate calibration semantics

formal E3 start gate 原本把 calibration evidence 作为 full E3 启动前硬条件。
这在工程上形成循环依赖：

```text
start gate 需要 formal terminal-bench_E3-P0_3_5 的 calibration evidence 才放行；
formal terminal-bench_E3-P0_3_5 又需要 start gate 放行才能产生该 evidence。
```

修复后的语义：

```text
calibration gate controls speed/cost claims, not the first identity-bound formal run.

当 current-HEAD non-agent gates、code-complete marker、user approval marker、
task_list_hash、profile_hash、source_version 全部通过时，
start gate 允许 calibration_gate=skipped_allowed 的正式 E3 运行。

该状态只表示“可以运行正式 E3 生成证据”，不表示“可以声明速度/成本收益”。
```

保持不变的 release blocker：

```text
speed_claim_allowed 必须为 true 才能声明 speedup / cost saving
calibration_gate_passed 必须为 true 才能发布最终 release-like claim
test-release-decision.ps1 仍覆盖该约束
```

验证：

```text
git diff --check = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 = PASS
```

## F.21 2026-06-28 optional child args fix

在 start gate calibration 语义修复后，formal plan-only 再次运行时暴露：

```text
run-taskspace-external-benchmark.ps1 : Missing an argument for parameter 'ApprovalMarkerSha256'.
```

该问题不是 TaskSpace runtime、解题能力或 DeepSeek provider 问题，而是 suite runner 的
child process argument contract 漏洞。空 marker hash/path 在 plan-only 阶段合法，
但不应作为没有值的命名参数传给下游脚本。

修复规则：

```text
optional string args are emitted only when non-empty.
required identity/provenance args are still emitted unconditionally.
```

验证结果：

```text
RunRoot = target\phase-r3-formal-e3-20260628-170557\plan-current-head-after-optional-arg-fix
SuiteRoot = suite-20260628-181726
status = completed
suite_score_valid = true
score_valid_child_runs = 3
score_invalid_child_runs = 0
profile_hash = 2aebff6baaf60a71367f9c999e93a1fd01a140257d48e4cee8378fccb0cbc013
```

影响：

```text
该修复再次改变 run-taskspace-e3-suite.ps1 SHA。
commit 后必须以最终 HEAD 重新生成 formal non-agent gates 和 marker。
```

## F.22 2026-06-28 formal E3 full-run state semantics

在 current-HEAD gates / markers 放行后，full formal E3 第一次启动暴露两个新的
harness closeout 问题。

### F.22.1 Windows deep run-root Git materialization

长 run root 下的两次 full formal E3 都在 workspace materialization 阶段失败：

```text
RunRoot = target\phase-r3-formal-e3-20260628-170557\formal-run-final-head-correct-source
RunRoot = target\phase-r3-formal-e3-20260628-170557\formal-run-final-head-retry-1
failure = invalid object 100644 83544132e76f2c3e3f5cee636e8e0ca0cabb5faf for 'Dockerfile'
failed loose-object temp path length = 281
```

短路径验证：

```text
RunRoot = target\e3f-final
SuiteRoot = target\e3f-final\suite-20260628-184253
result = workspace materialization passed; processing-pipeline entered real agent execution
```

结论：这是 Windows Git loose-object 路径预算问题，不是 task fixture 内容损坏。
当前正式运行必须使用短 run root；长期应由 suite runner 自动选择 Windows-safe
short run root。

### F.22.2 pending audit must not be invalid_harness

短路径 full formal E3 的第一项样本产生了可审查证据：

```text
sample = processing-pipeline
attempted_pairs = 5
completed_pairs = 5
run_validity = valid
phase = audit_required
engineering_unclean_count = 0
audit_required_count = 5
score_block_reason = audit_required
score_invalid_reason =
```

同时已经产生真实收益信号，但尚未完成 E3 人审计分：

```text
cost gate = PASS
direct_input_output_ratio = 1.1877
walltime_ratio = 0.6001
provider request 2+ cache hit rate = 0.984232
semantic_replacement_rate = 0.5299
protected_miss_count = 0
```

旧 suite runner 的错误：

```text
只要 aggregate.score_valid=false 就升级为 invalid_harness。
这会把 score_block_reason=audit_required 错误写成
harness_materialization_failure/score_invalid，并跳过剩余样本。
```

修复后的状态机：

```text
score_block_reason=audit_required
score_invalid_reason=<empty>
=> score_status=pending_audit
=> child run_validity 保持 valid
=> suite status = audit_required
=> suite_score_ready = false
=> suite_score_valid = false
=> score_pending_audit_child_runs 计数
=> emit suite_score_pending_audit
```

真正的 engineering-unclean 或 score-invalid 仍然保持 invalid_harness 熔断。

验证：

```text
git diff --check = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-score-validity.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 = PASS
```

下一步：

```text
1. commit/push 当前 state-machine fix
2. 重新计算 formal profile_hash
3. 重跑 current-HEAD non-agent gates
4. 重新生成 code-complete / user-approval marker
5. 重跑 formal start gate
6. 使用短 run root 重跑 formal terminal-bench_E3-P0_3_5
```

下一轮 full formal E3 的正确预期：

```text
如果样本执行完成但缺少人工审查，suite 应结束为 audit_required，而不是 invalid_harness。
只有完成 audit-review.json 并通过 score validity 后，才能声明 formal E3 score / speed / cost claim。
```

## F.23 2026-06-28 terminal-bench build network proof

pending-audit 状态机修复后重新启动 full formal E3，`processing-pipeline`
正确保留为 `audit_required`，suite 继续执行到 `multi-source-data-merger`。
该样本第一对在 validator Docker build 阶段失败：

```text
SuiteRoot = target\e3f-after-pending-audit-fix\suite-20260628-202449
sample = terminal_bench__multi-source-data-merger
pair = pair-001
reason = docker_build_environment_failure
infra_signature = harness_materialization_failure/docker_build_environment_failure
engineering_unclean_reasons =
  docker_build_environment_failure
  e3_external_validator_fidelity_unproven
  e3_external_validator_not_e3_eligible
  no_tests_started_marker
```

关键证据：

```text
Docker build stderr:
  Unable to connect to deb.debian.org:http
  Package 'tmux' has no installation candidate
  Unable to locate package asciinema

Validator stdout:
  docker_backend = wsl
  proxy_env_skipped_loopback = HTTP_PROXY / HTTPS_PROXY / http_proxy / https_proxy
  proxy_env_count = 0
  docker_cache_enabled = False
  docker_cache_bypass_reason = dockerfile_base_image_not_digest_pinned

Host:
  HTTP_PROXY / HTTPS_PROXY = http://127.0.0.1:7890
  127.0.0.1:7890 listening

WSL Docker probe:
  docker run --rm --network host python:3.11-slim ...
  proxy_connect = ok
```

结论：这是 harness build-network contract 问题。旧 adapter 在 WSL host
network 可以访问 Windows loopback proxy 的情况下仍跳过 proxy，并且未把 proxy
显式传入 Docker build；因此 apt 走直连失败。

修复：

```text
terminal-bench-adapter.ps1:
  WSL backend 保留 loopback proxy，记录 proxy_env_preserved_loopback
  Docker build 添加 --build-arg <proxyName>=<proxyValue>
  WSL Docker build 使用 --network host
  docker-build-result.json 记录 proxy_env_count / proxy_build_arg_count

test-terminal-bench-adapter-harness.ps1:
  覆盖 build proxy args
  覆盖 WSL loopback proxy preservation
  防止旧 proxy_env_skipped_loopback 行为回归
```

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-terminal-bench-docker-cache-smoke.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-external-wrapper-harness.ps1 = PASS

target\r3-proxy-build-probe\20260628-2222:
  proxy_env_count = 4
  proxy_build_arg_count = 4
  docker build exit_code = 0
  docker build classification = ok
  docker run exit_code = 1
  docker run classification = docker_run_failure
```

`docker_run_failure` 来自未解题 fixture 缺少输出文件：

```text
/app/merged_users.parquet missing
/app/conflicts.json missing
```

因此修复已经证明原始 apt/Docker build blocker 被清除，下一轮 formal E3 可以继续
用真实 agent 生成这些文件。由于 adapter SHA 改变，必须重新生成 profile identity、
non-agent gates、markers 和 start gate，再重跑正式 E3。

### F.23.1 start-gate fixture contract sync

首次用 suite runner 重跑 formal E3 时，suite 内置 start gate 失败在 cheap self-tests：

```text
SuiteRoot = target\e3f-after-build-proxy-fix\suite-20260628-223434
abort_reason = e3_start_gate_failed/self_test_failed
failed command = .\scripts\taskspace-benchmark\test-harness.ps1
output = terminal-bench validator did not guard WSL loopback proxy injection
```

这是 `test-harness.ps1` 的旧断言未同步新契约：

```text
旧断言：必须出现 proxy_env_skipped_loopback
新契约：WSL host networking 下保留 loopback proxy，并传入 Docker build args
```

修复后的 fixture 断言：

```text
proxy_env_preserved_loopback exists
$proxyBuildArgs += @("--build-arg", "$proxyName=$proxyValue") exists
proxy_env_skipped_loopback absent
```

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 = PASS
git diff --check = PASS
```

该修复改变 git commit，需要再次刷新 non-agent gates 和 markers 后重跑正式 E3。

### F.24 cost instrumentation large-rollout memory guard

继续 formal E3 后，`multi-source-data-merger` 不再卡在 Docker build。真实验证路径证明：

```text
pair-001 / pair-002:
  docker_build_environment_mode = host-proxy-forwarded
  proxy_env_count = 4
  proxy_build_arg_count = 4
  docker build phase = ok
```

随后暴露新的 runner 级问题：pair-003 两侧 validation 文件已经写出，但样本状态仍停在
`completed_pairs=2`。进程现场：

```text
PID = 7872
process = run-taskspace-benchmark.ps1
CPU delta over 60s = 59.515625s
working set delta over 60s = 683671552 bytes
private memory ~= 3.4GB
right/artifacts/rollout.jsonl = 103255682 bytes
```

定位：

```text
right/artifacts/git-diff.patch 已写出
right/artifacts/graph-health.json 已写出
right/artifacts/metrics.json 缺失
```

因此卡点不是 agent、Docker validation、changed inventory 或 graph health，而是
`Get-TaskspaceBenchmarkMetrics` 内的 cost instrumentation。大 rollout 会被多个诊断函数
重复全量扫描并 materialize 事件，导致 PowerShell 内存膨胀。

修复：

```text
metrics-extractor.ps1:
  changed inventory 过滤 .tbench-testing / .venv / node_modules / __pycache__ 等运行时依赖树
  metrics.json 暴露 rollout_scan_mode / rollout_bytes / rollout_scan_max_bytes

cost-instrumentation.ps1:
  新增 cost-scan-policy.json
  rollout 超过 TASKSPACE_COST_ROLLOUT_SCAN_MAX_BYTES 或默认 32MiB 时，保留原文件但跳过多次全量 rollout 诊断
  大文件模式记录 rollout_scan_mode = skipped_large_rollout

e3-proof.ps1:
  validator source isolation proof 使用 bounded repo scan
  跳过 .tbench-testing 等运行时目录
  超过扫描上限时显式让 proof 不通过，而不是静默证明 absence
```

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-metrics-extractor-harness.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-proof-harness.ps1 = PASS

真实 pair-003 right/artifacts metrics extraction:
  elapsed_ms = 1951
  rollout_scan_mode = skipped_large_rollout
  rollout_bytes = 103255682
  changed_count = 0
  metrics.json written
```

结论：这是 runner observability 内存问题，不是 agent 解题或 Docker validation 问题。
修复不改变 agent session 预算、不改变 validation 语义，只限制诊断脚本对大 rollout 的
重复全量解析。

仍未收敛的问题：

```text
multi-source-data-merger pair-001 / pair-002:
  standard = solved
  TaskSpace = wrong
  engineering_unclean = False

TaskSpace stderr:
  rg: /data: IO error
  Get-Content: Cannot find path '/data/source_a/users.json'
  Get-Content: Cannot find path '/data/source_b/users.csv'
  apply_patch failed: W:\app\src\merge_users.py does not exist
```

这说明在该任务上 TaskSpace 的路径理解 / 编辑策略存在真实退化，后续 R3 需要继续从
上下文管理器、任务环境说明和工具反馈压缩层面处理。

### F.25 observability exporter large-rollout artifact guard

继续 formal E3 后，又暴露了第二层 runner 观测问题：

```text
SuiteRoot = target\e3f-current\suite-20260629-010004
sample = multi-source-data-merger
pair = pair-001
right/artifacts/rollout.jsonl = 243219874 bytes
observability/action-map-observability.json before fix ~= 985922523 bytes
observability/action-map-observability.html before fix ~= 985930832 bytes
export process working set ~= 5858.6 MB
```

根因不是 agent 解题、Docker validation 或 DeepSeek API，而是
`export-action-map-observability.ps1` 对大 rollout 的全量物化：

```text
Read-JsonLines 将 rollout 全部读入 List[object]
timeline.details 保留 raw payload
snapshot_updated 将 result.body / evidencePackage 拷入 nodes
report lib 将完整 reduced JSON 写成 .json
report lib 又把同一份 JSON 嵌入 HTML trace-data
```

修复方式：

```text
action-map-observability-summary-lib.ps1:
  默认 rollout > 32MiB 时进入 summary_only_large_rollout
  line-stream 读取 rollout
  超过 TASKSPACE_OBSERVABILITY_EVENT_MAX_BYTES 的单行只提取类型和计数
  timeline 按 TASKSPACE_OBSERVABILITY_TIMELINE_SAMPLE_LIMIT 有界采样
  保留 runtimeEventCounts / topLevelEventCounts / largeLineEventCounts
  不保留 raw result body / raw snapshot payload

export-action-map-observability.ps1:
  写出 action-map-observability-policy.json
  小 rollout 保持原 full export
  大 rollout 输出小而完整的 observability JSON/Markdown/HTML

cost-instrumentation.ps1:
  summary_only_large_rollout 下从 summary.runtimeEventCounts 读取精确 runtime event count
  不把 bounded timeline 误当全量 timeline
```

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-action-map-observability-summary-export.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-action-map-observability-lib.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1 = PASS

真实 243MB rollout 单独导出:
  elapsed = 20.7s
  mode = summary_only_large_rollout
  json_bytes = 394428
  html_bytes = 402737
  markdown_bytes = 28642
  timeline_count = 240
  timeline_dropped = 1805
  parsed_lines = 1952
  largeLineSkippedCount = 95
  parse_errors = 0
  mapRuntimeEvents = 3067

下游 cost instrumentation:
  observability_source_status = summary_only_large_rollout
  taskspace_runtime_event_count = 3067
  runtime_state_commit_count = 6
```

结论：该问题是 runner observability artifact 放大，不是 TaskSpace 解题失败。修复只限制诊断报告的物化规模，
不改变 agent session 预算、provider payload、TaskSpace graph 语义或 validator 判定。由于脚本 SHA 再次变化，
后续必须刷新 current-HEAD profile/gates/markers 后再重跑 formal E3。

### F.26 pre-agent validator probe runner-private isolation

重跑 formal E3 后，`processing-pipeline` 的 pair-001 两侧都解题成功，但 E3 proof 被判为不干净：

```text
SuiteRoot = target\e3f-after-observability-guard\suite-20260629-023835
sample = processing-pipeline
pair = pair-001
outcome_standard = solved
outcome_taskspace = solved
proof_official_runner_or_equivalent = True
proof_agent_cannot_read_validator_source = False
proof_validator_e3_eligible = False
engineering_unclean_reason = e3_external_validator_source_not_isolated
```

证据拆分：

```text
actual validator source guard:
  protected file count = 5
  read denied after protect = true
  read restored after release = true

failing proof signal:
  left agent_artifact_validator_tokens = ["external-validator-source"]
  right agent_artifact_validator_tokens = []
```

根因是 runner 把 pre-agent validator probe 的 runtime manifest 写到了 agent 可读目录：

```text
old path = pair-001\left\artifacts\vprobe\terminal-bench-runtime-manifest.json
agent-visible path = W:\artifacts\vprobe\terminal-bench-runtime-manifest.json
standard transcript = Get-Content "W:\artifacts\vprobe\terminal-bench-runtime-manifest.json"
manifest field = validator_mount ... external-validator-source
```

这不是 validator 源码内容泄漏，而是 validator-source 路径元数据被放入 agent-visible artifacts。修复方式是将 pre-agent probe 输出整体迁移到 runner-private 分区：

```text
workspace.ps1:
  side.RunnerPrivateDir = pair-001\_runner-private\<side>

run-taskspace-benchmark.ps1:
  probeRoot = side.RunnerPrivateDir\vprobe
  validator-probe stdout/stderr/result/manifest all stay under probeRoot

test-harness.ps1:
  probe proof path budget checks RunnerPrivateDir
  asserts artifacts\vprobe does not exist
```

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 = PASS
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-proof-harness.ps1 -RunRoot target\r3-e3-proof-runner-private-test = PASS
git diff --check = PASS

focused processing-pipeline rerun:
  RunDir = target\r3-processing-pipeline-runner-private-proof\terminal_bench__processing-pipeline\20260629-025723-879
  command exit_code = 0
  engineering_unclean = False
  outcome_standard = solved
  outcome_taskspace = solved
  proof_agent_cannot_read_validator_source = True
  proof_validator_e3_eligible = True
  agent_artifact_validator_tokens(left) = []
  agent_artifact_validator_tokens(right) = []
  left\artifacts\vprobe exists = False
  right\artifacts\vprobe exists = False
  _runner-private validator-probe-result.json count = 2
```

结论：该问题是 runner artifact 分区错误，不是 agent 解题错误、DeepSeek API 问题或 Docker validation 问题。修复不弱化 E3 proof；相反，它把 proof-only metadata 从 agent 可见上下文中移除，同时保留 runner 自身可审计证据。

由于 `workspace.ps1`、`run-taskspace-benchmark.ps1` 和 `test-harness.ps1` 已变化，正式继续 E3 前必须重新刷新 current-HEAD profile/gates/markers/start gate。
