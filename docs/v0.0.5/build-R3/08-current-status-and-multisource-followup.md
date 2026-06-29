# R3 当前状态与 multi-source 后续记录

本文记录 2026-06-29 晚间继续推进 R3 时，`multi-source-data-merger` focused 样本暴露出的剩余问题、已完成修复、真实收益证明和当前外部 blocker。

## 1. 当前结论

R3 还不能声明全部完成。已经完成并验证的收益是：多个导致 TaskSpace 卡死或无效恢复的机制问题被打断；但 `multi-source-data-merger` 仍未 solved，最后一轮代码修复后还缺一次真实 rerun。

当前外部 blocker 是 D:\ 可用空间不足。最后检查约 13.57 GiB，可低于临时 focused run 门槛 14 GiB，也明显低于推荐 16-20 GiB。

## 2. 已证明的真实收益

### 2.1 rework blocker deadlock 被移除

对比现场：

```text
before:
  RunRoot = target\r3-multisource-after-diff-attribution-gib15
  outcome_taskspace = agent_exec_timeout
  exec_timed_out = True
  right_validation_lifecycle_stage = unknown
  open_leaf_nodes = 1

after:
  RunRoot = target\r3-multisource-after-rework-blocker-input-gib15
  RunDir  = target\r3-multisource-after-rework-blocker-input-gib15\runs\terminal_bench__multi-source-data-merger\20260629-201802-132
  exec_timed_out = False
  right_validation_lifecycle_stage = tests_completed
  tests_started_seen = True
  tests_completed_seen = True
  open_leaf_nodes = 0
```

解释：

```text
1. failed validation -> implement_solution rework 的路径不再被 origin validation blocker 的 unreviewed 状态卡死。
2. 真实样本从 900s agent_exec_timeout 推进到验证实际完成。
3. 该收益证明的是“卡死类故障被移除”，不是证明该样本业务已解出。
```

### 2.2 control normalization 初步收益

```text
RunRoot = target\r3-multisource-after-control-normalization-gib14
RunDir  = target\r3-multisource-after-control-normalization-gib14\runs\terminal_bench__multi-source-data-merger\20260629-204947-673

right / taskspace:
  engineering_unclean = False
  active_sentinel_warning_count = 0
  validation_lifecycle_stage = tests_completed
  tests_started_seen = True
  tests_completed_seen = True
  outcome_taskspace = wrong
  failure_taxonomy = agent_no_patch, audit_unclean
```

解释：

```text
1. 第一轮 control normalization 已经让样本从工程不干净/超时类失败推进到干净完成的 wrong。
2. 但该 run 仍出现未覆盖 alias：missing field title/context_summary/node_id。
3. 后续代码已补这些 alias，但因磁盘空间不足尚未执行最终真实 rerun。
```

## 3. 本轮已修复的机制问题

### 3.1 local infra validation 不再吞掉 changed artifact 证明义务

当 validation node 因本地 host shell 或 validator infrastructure 失败，但该失败绑定了尚未被证明的 changed artifacts 时，runtime 不再把它当作纯 infra block 关闭，而是进入 implement_solution rework。

验证：

```text
cargo test -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib -- --nocapture = PASS
```

### 3.2 implement rework prompt 按当前节点职责特化

`session/turn.rs` 的 action-contract prompt 现在会传入 current node kind。若当前是 `implement_solution`，local infra 失败提示不再要求重复 state_commit/block，而是要求 patch 或用平台兼容语法执行 changed artifact。

验证：

```text
cargo test -p codex-core action_contract_prompt_guides_platform_compatible_rework_after_recorded_local_infra --lib -- --nocapture = PASS
```

### 3.3 旧 diff 不再误归因给新 rework node

`record_taskspace_observed_implement_edit` 增加 active-map 级别的 successful edit guard，避免把旧 working-tree diff 记录到新 rework node 上。

验证：

```text
cargo test -p codex-core action_contract_prompt_guides --lib -- --nocapture = PASS
```

### 3.4 validation blocker 不再阻断自己的 active rework

`block_main_node` 自动创建 rework node 时记录 `origin_node_id`。普通 rework 工具允许把 origin validation blocker 视为输入证据继续执行，但 final response gate 仍保持严格。

验证：

```text
cargo test -p codex-core blocked_validation_rework_can_edit_without_reviewing_blocker_result --lib -- --nocapture = PASS
```

### 3.5 action-contract lifecycle args normalization 扩展

`normalize_taskspace_action_contract_control_args` 已覆盖：

```text
block_node:
  reason | summary | result -> blocker_summary
  missing node_id -> current snapshot node_id

create_node:
  node_kind | child_kind -> kind
  node_title | label | name | child_name -> title
  description | summary | objective -> context_summary
  missing kind -> inspect_code_context
  missing title/context_summary -> safe defaults
  existing task with no active node -> bind_current=true

bind_node:
  no node_id but contains node creation fields -> rewrite to create_node
```

验证：

```text
cargo test -p codex-core action_contract_control_ --lib -- --nocapture = PASS, 4 tests
```

## 4. 当前验证清单

已通过：

```text
cargo fmt -p codex-core = PASS
cargo test -p codex-core action_contract_control_ --lib -- --nocapture = PASS, 4 tests
cargo test -p codex-core action_contract_prompt_guides --lib -- --nocapture = PASS, 5 tests
cargo test -p codex-core validation_node --lib -- --nocapture = PASS, 16 tests
cargo test -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib -- --nocapture = PASS
cargo test -p codex-core blocked_validation_rework_can_edit_without_reviewing_blocker_result --lib -- --nocapture = PASS
cargo build -p codex-cli --bin whale --profile dev-small = PASS
```

## 5. 剩余未收敛点

### 5.1 multi-source-data-merger 仍未 solved

最后一个真实 rerun 状态：

```text
outcome_taskspace = wrong
failure_taxonomy = agent_no_patch, audit_unclean
changed_paths = empty
open_leaf_nodes = 1
```

这说明 R3 机制已推进到更深层：不再是超时、schema missing 或 validation lifecycle unknown，而是模型在 implementation_needs_edit 后没有稳定 emit apply_patch。

### 5.2 expanded control normalization 缺真实 rerun

真实日志暴露的 alias 问题已经修复，但修复后尚未跑新的 benchmark。原因是 D:\ 空间已经降到约 13.57 GiB。

## 6. 下一步恢复步骤

释放 D:\ 到至少 16 GiB，推荐 20 GiB 后执行：

```powershell
$env:TASKSPACE_DOCKER_BACKEND_PROBE_TIMEOUT_SECONDS='120'
$env:TASKSPACE_MIN_FREE_GIB='16'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\run-taskspace-external-benchmark.ps1 `
  -Benchmark terminal-bench `
  -TaskDir 'target\terminal-bench-pinned-1a6ffa9\original-tasks\multi-source-data-merger' `
  -SampleId multi-source-data-merger `
  -SourceVersion 1a6ffa9674b571da0ed040c470cb40c4d85f9b9b `
  -Repeats 1 `
  -RunRoot 'target\r3-multisource-after-expanded-control-normalization' `
  -WhaleBin 'target\phase-r3-current-cargo-target\dev-small\whale.exe' `
  -Model deepseek-v4-flash `
  -TimeoutSeconds 900 `
  -ValidationTimeoutSeconds 420 `
  -ValidationPretestTimeoutSeconds 120 `
  -ValidationTestTimeoutSeconds 420 `
  -SandboxMode full-auto `
  -ConfigOverride 'model_reasoning_effort="max"' `
  -AllowStaleWhaleBin
```

通过条件：

```text
1. 不再出现 missing field title/context_summary/node_id。
2. active node 能进入 implementation edit。
3. changed_paths 至少包含 merge_users.py 或最终输出。
4. validation lifecycle 达到 tests_completed。
5. 若仍 wrong，按 public validation 日志继续定位业务/提示问题，而不是继续修 schema gate。
```
