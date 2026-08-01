# R4 历史 sample 现场账本

> 本文把 R3 后已经观察到的真实样本现场整理成 R4 的证据账本。R4 后续每次修复都要回到这些样本，
> 证明问题类型被消除，而不是只证明新代码能通过单元测试。

## 2.1 样本总表

| Sample / Run | Standard Result | TaskSpace Result | Main Tool Symptom | R4 Interpretation |
|---|---|---|---|---|
| `single-file-fast-fix` rerun `target/r3-tool-feedback-rerun-single-file-20260630-0343/.../pair-001` | solved, 42s, 7 tools | solved, 114s, 5 tools | 修复后能改 `src/tax_calc.py`，但 wall time 约 2.7x | positive control：反馈/路径修复可带来 correctness，但性能仍需专项 |
| `count-call-stack` `target/r3-tools-result-sweep-20260630-3samples/.../pair-001` | solved, 57s, 11 tools | wrong, 325s, 8 tools | action-contract 内部 `apply_patch` 目标路径错，stderr 有 verification failed，但普通 tool result 回灌不清晰 | P0：internal tool failure parity 未完成 |
| `multi-file-order-pipeline` `target/r3-tools-result-sweep-20260630-3samples/.../pair-001` | solved, 158s, 13 tools | wrong, 346s, 10 tools | strict JSON / unknown action / node policy violation 循环，最终 no patch | P1：schema/control feedback 和 node policy 反馈不足 |
| `large-output-ref-smoke` `target/r3-tools-result-sweep-20260630-3samples/.../right/artifacts` | outer run timeout | right side timeout | `rollout.jsonl` 约 491MB，stderr 显示 900s timeout，测试仍失败 | P1：large output/ref、policy loop、日志膨胀需要治理 |
| `single-file-fast-fix` first light experiment `target/r3-light-effect-single-file-20260630-024154/.../pair-001` | solved, 47s, 7 tools | wrong, 77s, 5 tools | TaskSpace 知道 `round(..., 2)`，但 patch 未落盘 | R3 context/cache 健康，问题转移到 tool feedback/recovery |
| historical invalid tool-call history `coe/2026-06-20-02-00-taskspace-tool-call-history.md` | not applicable | provider protocol failure | assistant tool_calls 后未跟齐 tool messages | P0：projection/pairing safety 是协议正确性问题 |

## 2.2 关键现场细节

### 2.2.1 positive control：`single-file-fast-fix` 修复后通过

```text
RunDir: target\r3-tool-feedback-rerun-single-file-20260630-0343\single-file-fast-fix\20260630-034332-008
outcome_standard = solved
outcome_taskspace = solved
failure_taxonomy = none
TaskSpace changed_paths = src/tax_calc.py
validation = 3 passed in 0.03s
taskspace_wall_time_ratio = 2.7
taskspace_tool_call_ratio = 0.71
```

结论：

1. `apply_patch` normalization 和部分 tool feedback 对齐确实能修复一个 known-bad path。
2. correctness 通过不等于性能通过，TaskSpace 仍显著慢于 standard。
3. R4 benchmark 必须同时报告 solved、wall time、tool count、token/cache/log 指标。

### 2.2.2 `count-call-stack`：internal apply_patch failure 没有稳定进入普通反馈链路

```text
outcome_standard = solved
outcome_taskspace = wrong
failure_taxonomy = agent_no_patch
standard changed_paths = src/call_stack_counter.py
taskspace changed_paths = none
TaskSpace open_leaf_nodes = 1
taskspace_wall_time_ratio = 5.69
```

已知现场：

1. 模型两次尝试 `apply_patch`，目标路径类似 `src/call_stack_counter/__main__.py`。
2. stderr 有 `apply_patch verification failed: Failed to read file to update ...`。
3. `whale-exec.jsonl` 里没有稳定观察到同等普通 tool result 项。

设计结论：

1. 这不是“模型不会修题”的单纯问题，因为 standard solved。
2. 这也不能只归因于路径猜错，因为失败反馈应促使模型下一步纠正路径。
3. R4-D 必须证明 action-contract internal tool 失败和 standard tool 失败在下一轮 payload 中等价可见。

### 2.2.3 `multi-file-order-pipeline`：schema/control loop 和 node policy feedback 问题

```text
outcome_standard = solved
outcome_taskspace = wrong
failure_taxonomy = agent_no_patch
standard changed_paths = src/order_pipeline/parser.py, src/order_pipeline/pricing.py, tests/test_invoice.py
taskspace changed_paths = none
TaskSpace open_leaf_nodes = 3
taskspace_wall_time_ratio = 2.2
```

已知现场：

1. 出现 `action_contract_output_not_strict_json`。
2. 出现 `node_policy_violation:unknown:read_file`。
3. 出现多次 node create/bind/control 行为，但最终未形成有效 patch。

设计结论：

1. action-contract parse rejection 和 node policy rejection 需要进入统一反馈契约。
2. 反馈不能只说“失败”，必须告诉模型当前 node 可执行 action、被拒原因、应如何继续。
3. R4 不能再用粗暴 recovery 次数耗尽作为主要收敛机制。

### 2.2.4 `large-output-ref-smoke`：timeout 和日志膨胀

```text
right/artifacts/whale-exec.jsonl = 0 bytes
right/artifacts/rollout.jsonl ~= 490,846,386 bytes
stderr = process timed out after 900 seconds
validation = 2 tests failed
observed request_count reached 134/8 in prior inspection
```

设计结论：

1. output-ref 不能只解决 provider payload 大小，还必须约束 runtime artifact 膨胀。
2. policy violation loop 需要可解释收敛，而不是继续写巨量重复日志。
3. R4-E 必须为 large output 建立 summary/ref/retrieve 的可审计路径。

### 2.2.5 historical invalid tool-call history：协议层失败

历史 CoE 记录过 assistant tool_calls message 未跟齐所有 tool messages，导致 provider 侧 invalid tool-call history。

设计结论：

1. projection 过滤不是纯性能优化，它会影响 provider 协议正确性。
2. tool call/result 必须作为 pair 或 group 被投影，不能单独过滤一侧。
3. R4-C/R4-E 需要 fixture 证明 omitted tool call 和 output 总是成组处理。

## 2.3 样本驱动的 bug 分类

| Class | Definition | Examples | Required Gate |
|---|---|---|---|
| tool_feedback_loss | tool 失败细节没有进入下一轮可见 payload | `count-call-stack` | exact payload scan 包含失败 path/stderr/failure type |
| action_contract_loop | action JSON / node policy 失败后重复无效动作 | `multi-file-order-pipeline` | recovery feedback 结构化，重复 violation 指标下降 |
| no_patch_after_known_fix | agent 已识别正确修改但未落盘 | early `single-file-fast-fix` | changed_paths 非空且 validation pass |
| log_bloat_timeout | 输出/ref/policy loop 导致日志和 wall time 失控 | `large-output-ref-smoke` | rollout size 和 request count 有合理上界及原因 |
| protocol_pairing_break | projection 打破 tool call/result pairing | historical CoE | provider history fixture 通过 |

## 2.4 后续证据采集要求

每个 R4 真实样本 rerun 必须至少采集：

```text
run_dir
head
whale binary attestation
outcome_standard / outcome_taskspace
failure_taxonomy
changed_paths
validation stdout/stderr
tool_call_count
wall_time_ms
input/output/cache tokens
request_2_plus_cache_hit_rate
TaskSpace node count / open leaf count
tool feedback loss count
projection omit count by reason
large output ref count
rollout size
provider-visible payload proof path
```

没有这些字段时，样本只能作为诊断线索，不能作为 R4 benefit proof。

## 2.5 R4-B 工程化账本门禁

2026-06-30 补充：

```text
Ledger:
  docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json
Validator:
  scripts/taskspace-benchmark/test-r4-sample-ledger.ps1
Gate integration:
  scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1
Gate name:
  r4_sample_ledger
```

该账本把 R3/R4 已知真实现场变成机器可读证据行。validator 会检查：

1. 至少 6 个样本证据行。
2. 每个样本有唯一 id、failure class、owner phase 和 required follow-up。
3. 每个 primary evidence 文件必须存在。
4. secondary evidence 如果声明也必须存在。
5. 必须覆盖 `solved_positive_control`、`tool_feedback_loss`、`action_contract_loop`、
   `log_bloat_timeout`、`no_patch_after_known_fix`、`protocol_pairing_break` 六类现场。

这关闭 R4-B 的“历史样本 scattered in target/CoE”管理缺口，但不等于这些样本已被修复；
修复和收益验证仍由 R4-D/R4-E/R4-G 负责。
