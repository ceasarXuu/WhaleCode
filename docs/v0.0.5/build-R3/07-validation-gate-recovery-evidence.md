# R3 validation gate recovery 证据补充

本文记录 2026-06-29 在 `recover-accuracy-log` focused rerun 中继续推进 R3 时发现并修复的 validation recovery 问题。

## F.33 validation gate recovery 与 no-action advisory 化

第五轮真实 rerun 暴露出两个相邻问题：

```text
RunRoot = target\r3-validation-rework-recover-accuracy-log
RunDir  = target\r3-validation-rework-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-114416-834

现象：
  - 成功 edit: recover_logs.py
  - validation 先请求 python3 process_logs.py
  - validation coverage gate 正确拦截，reason=validation_test_missing_changed_artifact_coverage
  - 随后 generic no-action recovery 丢失了 gate 的具体 next_valid_actions
  - 下一轮又请求 find . -name '*.py'
  - turn 以 no-action recovery cap 结束
```

根因拆解：

```text
1. coverage gate 本身是正确的，能够识别“验证命令没有执行 changed artifact”。
2. 但 action-contract 工具返回的是 Ok(response_input)，其中 body 是 blocked tool output；
   session/turn.rs 没有把这个 blocked output 提升为 last_agent_message。
3. 因此下一轮 TaskSpaceNoActionRecoveryV1 只看到上一条 agent rationale，
   看不到 TaskSpaceGateRecoveryV1 的 blocking_items / next_valid_actions。
4. smoke_test 的 no-action cap=1 又把一次 policy violation 放大成硬失败。
```

本轮修复：

```text
runtime.rs:
  - validation_test_coverage_block 的 recovery message 增加明确约束：
    不要用 find/ls/rg/Get-ChildItem 重新发现 changed artifact。
  - next_valid_actions 改为可执行命令建议：
    run_test with command `python <artifact>` to execute changed artifact `<artifact>`

turn.rs:
  - 新增 TASKSPACE_GATE_RECOVERY_MARKER。
  - 从 FunctionCallOutput / CustomToolCallOutput 中提取 TaskSpaceGateRecoveryV1。
  - 如果 action-contract 工具输出被 gate blocked，则把 gate recovery 提升为 last_agent_message。
  - TaskSpaceNoActionRecoveryV1 会显式重放最近 TaskSpaceGateRecoveryV1，并要求优先 obey `next_valid_actions`。
  - no-action recovery cap 改为 advisory threshold；超过后继续插入 recovery，不再 return None 硬停机。
```

定向验证：

```text
cargo test -p codex-core validation_node_blocks_vacuous_test_after_changed_artifact --lib -- --nocapture = PASS
cargo test -p codex-core no_action_recovery --lib -- --nocapture = PASS, 3 tests
cargo test -p codex-core extracts_gate_recovery_from_blocked_tool_output --lib -- --nocapture = PASS
cargo test -p codex-core validation_node --lib -- --nocapture = PASS, 16 tests
cargo test -p codex-core active_context_replacement --lib -- --nocapture = PASS, 113 tests
cargo build -p codex-cli --bin whale --profile dev-small = PASS
```

## F.34 focused rerun 真实收益：recover-accuracy-log 已解出

本轮先遇到一次本地 harness 预检失败：

```text
RunRoot = target\r3-gate-recovery-recover-accuracy-log
abort_phase = external_materialization_preflight
reason = Free disk space below TaskSpace preflight minimum on D:\: 20 GiB available, 20 GiB required
free_bytes = 21471506432
required_free_bytes = 21474836480
```

该失败只比阈值少约 33 KiB，Docker/WSL 存储检查还有约 942 GiB 可用。为了避免清理文件带来额外风险，后续 rerun 使用：

```text
TASKSPACE_MIN_FREE_GIB=19
```

第一轮 gate recovery rerun 证明桥接有效但仍未解出：

```text
RunRoot = target\r3-gate-recovery-recover-accuracy-log-rerun
RunDir  = target\r3-gate-recovery-recover-accuracy-log-rerun\runs\terminal_bench__recover-accuracy-log\20260629-120857-814

right / taskspace:
  outcome_taskspace = wrong
  changed_paths = recover.py
  tool_call_count = 7
  open_leaf_nodes = 1

关键证据：
  - line 276 gate output 明确给出：
    next_valid_actions = run_test with command `python /app/recover.py`
  - line 284 no-action recovery 已经重放 TaskSpaceGateRecoveryV1
  - 但下一轮模型仍发 list_files，说明 no-action cap=1 仍会把一次 policy violation 放大成失败
```

no-action advisory 化后 rerun：

```text
RunRoot = target\r3-noaction-advisory-recover-accuracy-log
RunDir  = target\r3-noaction-advisory-recover-accuracy-log\runs\terminal_bench__recover-accuracy-log\20260629-122726-092

pair-001:
  standard = solved
  taskspace = solved
  taskspace business_success = True
  taskspace exec_exit_code = 0
  taskspace public_validation_exit_code = 0
  taskspace hidden_oracle_exit_code = 0
  taskspace changed_paths = recover_accuracy.py, recovered_logs/results.json, recovered_logs/run_1_generator.jsonl, recovered_logs/run_1_judge.jsonl, recovered_logs/run_2_generator.jsonl, recovered_logs/run_2_judge.jsonl, recovered_logs/run_3_generator.jsonl, recovered_logs/run_3_judge.jsonl
  taskspace tool_call_count = 6
  taskspace open_leaf_nodes = 0
  taskspace_control_count = 4
```

真实收益：

```text
1. recover-accuracy-log 从 formal E3 中的 all wrong / agent_no_patch，推进到单样本 solved。
2. no-patch、missing-target patch、vacuous validation、gate recovery 丢失、no-action 硬停机这几类失败链路均被逐层打断。
3. TaskSpace 侧最终生成了 required outputs，并通过 public + hidden oracle。
4. active context replacement 在 rerun 中保持通过：
   exact_payload_scan_passed=true
   replacement_confirmed=true
   legacy_taskspace_history_present=false
```

仍未收敛的问题：

```text
1. 成功后仍出现多轮冗余 validation recovery，直到 request_count 23/20 才 final。
2. no-action cap 已从硬停机改为 advisory，但这只是避免错误终止；真正需要的是成功验证后的 terminal action 更早收敛。
3. 当前结果是 E2-candidate / focused real-task evidence，不是 formal E3：
   - repeats 不足；
   - human review 未完成；
   - Terminal-Bench official protocol source hashes 不可用，validator E3 eligibility 仍被降级。
```
