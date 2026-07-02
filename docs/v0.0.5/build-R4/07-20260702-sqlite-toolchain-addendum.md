# R4 sqlite-db-truncate 工具链收敛补充记录

日期：2026-07-02

本记录补充 `sqlite-db-truncate` public sample 在 R4 后续收敛中的真实现场。该样本业务难度较高，不能把一次 wrong 直接等同为工程失败；本记录只声明已被证据支持的工程收益。

## 1. timeout artifact streaming

失败现场：
```text
旧 Invoke-RealProcess 使用 ReadToEndAsync，超时 kill 时 stdout/stderr 可能无法落盘。
```

修复：
```text
scripts/action-map-real-user-e2e-lib.ps1
scripts/taskspace-benchmark/test-real-process-streaming.ps1
```

验证：
```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-real-process-streaming.ps1
PASS

TASKSPACE_MIN_FREE_GIB=15
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-external-wrapper-harness.ps1
PASS
```

收益判断：
- 真实收益成立：timeout 时 `whale-exec.jsonl`、stderr、timing、metrics 能保留，后续真实样本根因分析不再黑盒。
- 该收益是 observability / RAM-pressure 收敛收益，不直接声称解题成功率提升。

## 2. uv cache permission failure classification

失败现场：
```text
uv run pytest failed to open ...\uv\cache\sdists-v9\.git
os error 5 / Access is denied
```

修复：
```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
text_mentions_local_validator_infra_failure(...)
```

验证：
```text
cargo test -j1 -p codex-core uv_cache_access_denied_auto_blocks_validation_as_local_infra --lib
PASS

cargo test -j1 -p codex-core local_infra_tool_result_auto_blocks_validation_node --lib
PASS

cargo test -j1 -p codex-core action_contract_run_test_local_infra_result_auto_blocks_validation --lib
PASS

cargo test -j1 -p codex-core local_validator_infra_failure_does_not_raise_validator_failure --lib
PASS
```

收益判断：
- 工程收益成立：host-side uv cache 权限问题不会再被误归因成实现失败。

## 3. unanchored Update File rejection

失败现场：
```text
RunDir:
C:\WhaleRunCache\r4-uv-infra-sqlite-truncate-20260702\runs\terminal_bench__sqlite-db-truncate\20260702-210016-678

TaskSpace accepted an Update File patch with only + lines and no context/deletion.
The model treated the edit as successful while recover.py still contained broken content.
```

修复：
```text
third_party/codex-cli/codex-rs/core/src/session/turn.rs
TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1
taskspace_apply_patch_unanchored_update_targets(...)
```

验证：
```text
cargo test -j1 -p codex-core taskspace_action_contract_rejects_unanchored_update_patch --lib
PASS

cargo test -j1 -p codex-core taskspace_action_contract_allows_anchored_update_patch --lib
PASS

cargo test -j1 -p codex-core apply_patch_unanchored_update_recovery_does_not_count_as_no_action_retry --lib
PASS
```

收益判断：
- 工程收益成立：该坏 patch 形态在执行前被拒绝，不再制造误导性的成功 edit evidence。
- 后续真实 rerun 未再次触发此分支，因此真实样本收益只记为 branch-level gate pass。

## 4. validation rework duplicate read recovery

失败现场：
```text
Before:
C:\WhaleRunCache\r4-unanchored-patch-sqlite-truncate-20260702\runs\terminal_bench__sqlite-db-truncate\20260702-213917-258\pair-001

outcome_taskspace=agent_exec_timeout
taskspace_exec_timed_out=true
taskspace_wall_ms=420067
taskspace_tool_call_count=13
open_leaf_nodes=1
failure_taxonomy=engineering_unclean, taskspace_overhead_timeout, audit_unclean
```

根因：
```text
ActionMap 已经阻止重复读取：
validation rework node already read failure artifact recover.py and no successful edit

但 session 层把该拒绝落入 generic TaskSpaceNoActionRecoveryV1，
模型反复读同一 artifact，直到 timeout。
```

修复：
```text
third_party/codex-cli/codex-rs/core/src/session/turn.rs
taskspace_message_hit_implementation_needs_edit(...)
```

验证：
```text
cargo test -j1 -p codex-core validation_rework_duplicate_read_rejection_uses_edit_recovery --lib
PASS

cargo test -j1 -p codex-core taskspace_implementation_needs_edit_rejection_uses_specific_recovery --lib
PASS

cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

真实复验：
```text
After:
C:\WhaleRunCache\r4-edit-recovery-sqlite-truncate-20260702\runs\terminal_bench__sqlite-db-truncate\20260702-220208-680\pair-001

outcome_standard=solved
outcome_taskspace=engineering_unclean
failure_taxonomy=engineering_unclean, agent_patch_wrong, audit_unclean
taskspace_exec_timed_out=false
taskspace_wall_ms=196129
taskspace_tool_call_count=5
open_leaf_nodes=0
right_validation_lifecycle_stage=tests_completed
public_validation_exit_code_taskspace=1
hidden_oracle_exit_code_taskspace=0
```

收益判断：
- 真实工程收益成立：同一 public sample 从 `agent_exec_timeout` / open leaf 变成非超时 closed graph。
- 工具效率收益成立：TaskSpace tool calls 从 13 降到 5。
- 剩余失败是 `agent_patch_wrong`，属于模型解题或策略质量问题，不再是本分支的工具链循环。

## 5. 当前结论

`sqlite-db-truncate` 对 R4 的意义是工具链收敛证明，不是 utility parity 证明。当前状态：

| 维度 | 结论 |
|---|---|
| 工程超时 | 已从 timeout 收敛为非 timeout |
| graph 收尾 | open leaf 从 1 收敛为 0 |
| 工具调用 | TaskSpace 13 次降到 5 次 |
| 业务正确性 | 未通过，仍是 `agent_patch_wrong` |
| 下一步 | 把 wrong 作为模型策略/任务解法质量问题进入后续样本分析，不再归入本分支工具链循环 |
