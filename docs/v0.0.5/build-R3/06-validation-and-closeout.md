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
