# 11. Clean E3 Audit 与 Failure Taxonomy 设计

## 1. 背景

0.0.3 的 E3 diagnostic 可以分析，但 clean utility aggregate 不成立，因为 artifact audit review gate 未闭环。0.0.4 必须建立机械可解释的 pair inclusion/exclusion 规则。

## 2. Audit Manifest

每个 pair 输出 `audit.yaml`：

```yaml
audit_version: taskspace-e3-audit-v1
pair_id:
sample_name:
standard:
  success:
  exec_exit_code:
  public_validation_exit_code:
  wall_time_ms:
  changed_files:
  diff_ref:
  validator_stdout_ref:
  validator_stderr_ref:
  cleanup_ok:
taskspace:
  success:
  exec_exit_code:
  public_validation_exit_code:
  wall_time_ms:
  changed_files:
  diff_ref:
  validator_stdout_ref:
  validator_stderr_ref:
  cleanup_ok:
  graph_ref:
  graph_health_ref:
  result_validity_summary:
  decision_summary:
classification:
  included_in_utility:
  exclusion_reason:
  failure_taxonomy:
  utility_direction:
  audit_status:
proof:
  oracle_isolation_ok:
  remote_asset_ok:
  cleanup_ok:
  validator_equivalence_ok:
  human_review_required:
  human_review_completed:
```

## 3. Inclusion 规则

进入 clean utility aggregate 的 pair 必须满足：

```text
standard artifact 完整；
taskspace artifact 完整；
validator evidence 完整；
cleanup ok；
remote asset 不 taint；
diff/changed files 可读取；
audit_status != missing；
failure taxonomy != unknown。
```

## 4. Utility direction

| Direction | 条件 |
|---|---|
| taskspace_better | taskspace_success=true, standard_success=false, no environment taint |
| standard_better | standard_success=true, taskspace_success=false, no environment taint |
| both_success | both success=true |
| both_failed | both success=false |
| inconclusive | artifact/audit/environment/validator taint |

## 5. Failure taxonomy

```text
agent_patch_wrong
agent_no_patch
agent_validation_loop
taskspace_overhead_timeout
subagent_noise_or_unused
node_overfragmentation
result_not_synthesized
validator_slow_or_flaky
environment_noise
docker_run_failure
remote_asset_unavailable
remote_asset_equivalence_unproven
audit_unclean
unknown
```

## 6. 自动分类规则

| Signal | Failure class |
|---|---|
| no changed files and failed | agent_no_patch |
| validator repeatedly fails after multiple patches | agent_validation_loop |
| taskspace walltime high + timeout | taskspace_overhead_timeout |
| high nodes + high blocked ratio | node_overfragmentation |
| spawn_count > 0 and adopted_subagent_results = 0 | subagent_noise_or_unused |
| public validation 124 and both sides timeout | validator_slow_or_flaky |
| remote asset preflight fail-closed | remote_asset_equivalence_unproven |
| audit manifest missing | audit_unclean |
| no rule matches | unknown |

## 7. Aggregate 输出

0.0.4 aggregate 应包含：

```json
{
  "valid_utility_pairs": 0,
  "taskspace_better": 0,
  "standard_better": 0,
  "both_success": 0,
  "both_failed": 0,
  "inconclusive": 0,
  "excluded_by_reason": {},
  "failure_taxonomy_summary": {},
  "graph_health_summary": {}
}
```

## 8. Manual review

0.0.4 可以允许 manual review，但不能让 aggregate 只依赖人工解释。状态必须明确：

```text
not_required
required_pending
completed_accepted
completed_rejected
```

## 9. 验收

```text
每个 pair 有 audit.yaml。
每个 failed/timeout pair 至少一个 non-unknown failure class。
aggregate 能解释 included/excluded/inconclusive。
valid_utility_pairs 不因 audit missing 全部为 0。
```
