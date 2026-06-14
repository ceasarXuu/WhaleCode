# 11. Clean E3 Audit 与 Failure Taxonomy 设计

## 1. 背景

0.0.3 的 E3 diagnostic 可以分析，但 clean utility aggregate 不成立，因为 artifact audit review gate 未闭环。0.0.4 必须建立机械可解释的 pair inclusion/exclusion 规则。

## 1.1 硬性执行有效性契约

0.0.4 之后的 E3 成绩只允许三类 agent outcome：

| Outcome | 解释 | 判分 |
|---|---|---|
| `solved` | agent 完成解题，validator/oracle 干净运行并通过 | 成功 |
| `wrong` | agent 完成解题，validator/oracle 干净运行并判定业务失败 | 失败 |
| `agent_exec_timeout` | agent 没有在规定解题时间内完成 | 允许的 timeout failure |

其他任何异常都不是 agent outcome，而是 `engineering_unclean`。范围包括 Docker/WSL/container 失败、validator timeout/crash、fixture/materialization/source/path/cache/disk/proof/report/parser/lifecycle marker 异常、artifact 缺失、cleanup/proof 不可验证。

硬规则：

- `public_validation_timeout` 属于 validator infrastructure failure，不等于 `agent_exec_timeout`。
- 任一 pair 出现 `engineering_unclean` 时，本次 E3 run 的 `score_valid=false`。
- `score_valid=false` 时 aggregate 只能输出 diagnostic taxonomy，不得输出 Standard vs TaskSpace score、better/worse 或版本收益结论。
- 只有所有 comparable pairs 都是 `solved` / `wrong` / `agent_exec_timeout`，且 validator/oracle 运行干净，才能计算 clean utility score。

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
  run_score_valid:
  outcome_standard:
  outcome_taskspace:
  engineering_unclean:
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
| both_failed | both success=false，且两侧失败都是 `wrong` 或 `agent_exec_timeout` |
| run_invalid_engineering_unclean | artifact/audit/environment/validator taint；整次 run 不得算分 |

## 5. Failure taxonomy

```text
agent_patch_wrong
agent_no_patch
agent_validation_loop
agent_exec_timeout
subagent_noise_or_unused
node_overfragmentation
result_not_synthesized
validator_slow_or_flaky
environment_noise
engineering_unclean
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
| agent process reaches configured solve timeout | agent_exec_timeout |
| high nodes + high blocked ratio | node_overfragmentation |
| spawn_count > 0 and adopted_subagent_results = 0 | subagent_noise_or_unused |
| public validation 124, validator crash, or validator dependency missing | `validator_slow_or_flaky` + `engineering_unclean` |
| remote asset preflight fail-closed | remote_asset_equivalence_unproven |
| audit manifest missing | audit_unclean |
| no rule matches | unknown |

## 7. Aggregate 输出

0.0.4 aggregate 应包含：

```json
{
  "score_valid": false,
  "score_invalid_reason": "engineering_unclean",
  "engineering_unclean_count": 0,
  "agent_exec_timeout_count": 0,
  "clean_comparable_pair_count": 0,
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

当 `score_valid=false` 时，`taskspace_better`、`standard_better`、`both_success`、`both_failed` 只能作为 diagnostic raw counts 输出，不能作为成绩或版本结论。

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
每个 failed/timeout pair 必须区分 agent_exec_timeout、wrong 和 engineering_unclean。
aggregate 能解释 included/excluded/inconclusive/run_invalid_engineering_unclean。
任一 engineering_unclean 会使 score_valid=false，且报告不能输出有效 better/worse。
valid_utility_pairs 只能来自 clean comparable pairs。
```
