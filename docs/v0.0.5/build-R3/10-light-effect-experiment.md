# R3 轻量效果实验记录

本文记录 2026-06-30 在 R3 工程层收口后追加的一轮轻量效果实验。该实验只用于观察当前 HEAD 的方向性效果，不作为 formal E3 或发布级收益结论。

## 1. 实验配置

```text
RunRoot = target\r3-light-effect-single-file-20260630-024154
RunDir = target\r3-light-effect-single-file-20260630-024154\single-file-fast-fix\20260630-024155-720
PairReport = target\r3-light-effect-single-file-20260630-024154\single-file-fast-fix\20260630-024155-720\pair-001\pair-report.md
Scenario = single-file-fast-fix
Repeats = 1
Model = deepseek-v4-flash
WhaleBin = D:\BuildCache\whalecode\cargo-target\dev-small\whale.exe
Mode = standard vs taskspace paired run
Formal E3 = false
ScoringMode = false
```

执行命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-benchmark.ps1 `
  -Scenario single-file-fast-fix `
  -Repeats 1 `
  -RunRoot target\r3-light-effect-single-file-20260630-024154 `
  -WhaleBin D:\BuildCache\whalecode\cargo-target\dev-small\whale.exe `
  -Model deepseek-v4-flash `
  -SandboxMode full-auto `
  -AllowNonE2Result
```

## 2. 结果摘要

```text
script_exit_code = 0
reported_evidence_level = E1
included_in_utility_aggregate = false
valid_pair = true
utility_direction = standard_better
failure_taxonomy = agent_no_patch
engineering_unclean = false
```

标准模式：

```text
outcome_standard = solved
business_success = true
exec_exit_code = 0
public_validation_exit_code = 0
hidden_oracle_exit_code = 0
wall_time_ms = 47208
tool_call_count = 7
changed_paths = src/tax_calc.py
open_leaf_nodes = 0
```

TaskSpace 模式：

```text
outcome_taskspace = wrong
business_success = false
exec_exit_code = 1
public_validation_exit_code = 1
hidden_oracle_exit_code = 1
wall_time_ms = 76848
tool_call_count = 5
changed_paths = none
open_leaf_nodes = 1
taskspace_tool_call_ratio = 0.71
taskspace_wall_time_ratio = 1.63
```

验证输出显示 TaskSpace 未实际修改 `src/tax_calc.py`：

```text
FAILED tests/test_tax_calc.py::test_calculate_tax_rounds_to_cents
assert 1.4 == 1.45

FAILED tests/test_tax_calc.py::test_calculate_total_uses_tax_amount
assert 21.39 == 21.44
```

## 3. 已验证仍正常的 R3 能力

本次负向结果不是上下文替换或 cache hit 退化导致：

```text
exact_payload_scan_passed = true
context_bundle_present = true
exact_context_bundle_verified = true
cache_plan_verified = true
replacement_confirmed = true
legacy_taskspace_history_present = false
raw_taskspace_control_history_tokens = 0
protected_items_present = true
request_2_plus_hit_rate = 0.985235
request_phase_attribution_coverage = 100
wait_attribution_status = complete
model_request_duration_ms = 66682
model_queue_wait_ms = 4036
```

## 4. 发现的问题

TaskSpace 已经正确定位 bug，但 patch 没有成功落地：

```text
Bug: calculate_tax rounds to 1 decimal but should round to 2 decimals.
Expected fix: change round(..., 1) to round(..., 2).
```

模型发出了两次 `apply_patch`：

1. 第一次缺少 `*** Begin Patch`，工具拒绝。
2. 第二次包含 begin/end，但目标路径写成 `tax_calc.py`，实际文件在 `src/tax_calc.py`，context 匹配失败。

随后 runtime 触发 implement-needs-edit recovery，但最终仍然停止：

```text
TaskSpace stopped this turn because the model repeatedly requested read/search/list actions
after implementation evidence was sufficient and no edit was recorded
(2/2 implement-needs-edit recoveries spent).
```

## 5. 结论

这轮轻量实验的方向性结论：

1. R3 的 context replacement、cache planner、payload proof、timing attribution 在当前 HEAD 仍然有效。
2. TaskSpace 在该轻量样本上没有带来解题收益；standard solved，taskspace wrong。
3. 当前主要问题不是“看不懂任务”，而是 patch recovery 没能把模型已经识别出的正确一行修改转成成功文件编辑。
4. 这说明下一轮优化应集中在 apply_patch 失败恢复和目标路径归一化，而不是继续扩大上下文或放宽预算。

该实验不能用于发布级收益声明，因为：

1. `Repeats=1`，只是一对样本。
2. `included_in_utility_aggregate=false`。
3. `reported_evidence_level=E1`。
4. oracle isolation gate 未满足，且不是 formal E3。
