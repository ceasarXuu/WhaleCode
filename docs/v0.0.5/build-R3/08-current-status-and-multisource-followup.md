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
## 7. 2026-06-30 focused rerun 后的新增收敛

### 7.1 expanded control normalization 已有真实收益，但暴露 validation 假阳性

本轮清理 D:\ 后重跑了 `multi-source-data-merger` focused pair：

```text
RunRoot = target\r3-multisource-after-rework-chain-review-gate
RunDir  = target\r3-multisource-after-rework-chain-review-gate\runs\terminal_bench__multi-source-data-merger\20260629-235542-254

right / taskspace:
  exec_timed_out = False
  right_validation_lifecycle_stage = tests_completed
  right_tests_started_seen = True
  open_leaf_nodes = 0
  engineering_unclean = False
  outcome_taskspace = wrong
  changed_paths = merge_users.py
```

收益：

```text
1. missing field title/context_summary/node_id 不再复现。
2. rework-chain review gate 不再卡住验证节点。
3. previous agent_exec_timeout / lifecycle unknown 推进为 tests_completed。
```

新问题：

```text
node-3 smoke_test 中 `python merge_users.py` exit_code=0，
但输出包含：
  Warning: /data/source_a/users.json not found
  No source files found. Exiting.

runtime 仍触发 forced_validation_closeout，并生成 “Validation passed; final result is ready.”
外部 validator 随后证明 /app/merged_users.parquet 和 /app/conflicts.json 不存在。
```

根因是 `force_finish_validation_after_successful_tool` 和 `latest_successful_validation_result_id`
只看 `tool_success=true`，没有判断测试输出的失败语义。

### 7.2 semantic validation gate 修复与真实收益

修复：

```text
runtime.rs:
  node_result_is_successful_validation(result)
    = MainToolCall
    + action_class in {test, build}
    + tool_success=true
    + validation output does not contain strong failure markers

trace_tags_for:
  success=true 但输出包含 no source files found / FileNotFoundError / failed / no such file 等强失败语义时，
  标记 validator_failure，而不是 validator_success。

turn.rs:
  validation closeout 不再在调用前重复使用旧的 current_main_node_has_successful_action(test/build) 前置条件；
  统一交给 runtime 的 semantic validation 判断。
```

单测：

```text
cargo test -p codex-core force_finish_validation_ --lib -- --nocapture = PASS, 2 tests
cargo test -p codex-core action_contract_control_ --lib -- --nocapture = PASS, 5 tests
cargo build -p codex-cli --bin whale --profile dev-small = PASS
```

真实 rerun：

```text
RunRoot = target\r3-multisource-after-semantic-validation-gate
RunDir  = target\r3-multisource-after-semantic-validation-gate\runs\terminal_bench__multi-source-data-merger\20260630-002557-891

right / taskspace:
  exec_exit_code = 1
  exec_timed_out = False
  right_validation_lifecycle_stage = tests_completed
  right_tests_started_seen = True
  open_leaf_nodes = 1
  changed_paths = merge_users.py
  forced_validation_closeout = not observed for the false-positive `No source files found` run
```

收益解释：

```text
1. 假阳性 closeout 被挡住：runtime 不再把 “No source files found. Exiting.” 当作验证通过。
2. 失败从 “错误最终通过后被外部 validator 打脸” 变成 “内部验证失败后继续 rework，但未完成”。
3. 这证明 validation gate 的真实性提升，但不证明样本已 solved。
```

### 7.3 shell chain normalization 修复

semantic validation gate 后暴露新的系统性问题：

```text
agent 发出 run_test:
  python merge_users.py && python -c ...

Windows PowerShell 5.1 报错：
  The token '&&' is not a valid statement separator in this version.
```

这不是业务解题错误，而是 action-contract shell command 与宿主 shell 的适配缺口。

修复：

```text
turn.rs:
  normalize_taskspace_action_contract_test_command
    -> normalize_taskspace_host_shell_test_command
    -> Windows: normalize_taskspace_powershell_and_chain

转换规则：
  a && b
  =>
  a; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; b

并且只拆顶层 &&，引号内的 && 不拆。
```

验证：

```text
cargo test -p codex-core taskspace_action_contract_run_test_ --lib -- --nocapture = PASS, 3 tests
cargo test -p codex-core force_finish_validation_ --lib -- --nocapture = PASS, 2 tests
cargo test -p codex-core action_contract_control_ --lib -- --nocapture = PASS, 5 tests
cargo fmt -p codex-core = PASS
cargo build -p codex-cli --bin whale --profile dev-small = PASS
```

第二次真实 rerun：

```text
RunRoot = target\r3-multisource-after-shell-chain-normalization
status = incomplete
reason = outer command timeout after 20 minutes; residual benchmark PowerShell/validator processes were stopped manually.
```

该 rerun 不计入收益结论。下一次需要用更长外层 timeout 或更小的 focused harness timeout 重新验证 shell chain normalization 的真实收益。
