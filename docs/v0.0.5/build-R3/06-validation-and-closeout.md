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
