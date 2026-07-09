# R5 Phase D Ledger Deactivation

Status: D1/D2 landed

Date: 2026-07-09

## 1. 目标

Phase D 的目标是让 TaskSpace 回到“图化/状态机化的任务 map 工具”，而不是语义工作流 runtime：

1. `start_task initial_*` 不再自动提升为 canonical truth。
2. `problem_ledger/cognitive_state` 不再进入 active projection 或 active closeout/final gate。
3. runtime 不再替 Agent 做 result validity / success criterion / broad delegation strategy 采纳。
4. 保留状态机硬底线：task/map/node/lease、node kind 能力、成功 read/search/edit/test/build 等机械证据。

## 2. 代码收敛

### D1: start_task 降级

`start_task_for_main_with_kind_and_criteria` 仍接受 `initial_success_criteria`、
`initial_output_contracts`、`initial_fact_sources`，但只作为 control schema 输入兼容项；runtime 不再：

- 自动生成 `sc-user-request` / `oc-user-request` / `fs-user-request`。
- 从 objective、criteria 或 fact source 文本派生 output contract。
- 把 initial sections 写入 `problem_ledger` 或 `cognitive_state`。

Agent 如果需要记录这些内容，必须显式调用 `state_commit` 或 focused `record_*` action；这些记录是 Agent-authored note，不是 runtime canonical truth。

### D2: active gate / projection 降级

已从 active 路径移除：

- provider-visible cognitive protocol 文案。
- fallback developer context 中的 `Task problem-state ledger` / `Task cognitive state` block。
- inspect closeout 的 fact-source coverage gate。
- validation closeout 的 satisfied success criterion gate。
- successful validation 的 runtime auto-accept / auto success criterion。
- final response 的 success criteria、open question、decision dependency、result validity、post-edit accepted validation gate。
- broad delegation debt gate。

仍保留的 hard baseline：

- ordinary tools 需要 active task path、current node binding、lease。
- live node kind 只允许 `inspect_code_context`、`implement_solution`、`smoke_test`、`regression_test`、`final_synthesis`。
- `inspect_code_context` finish 需要成功 read/search。
- `implement_solution` finish 需要成功 edit。
- `smoke_test` / `regression_test` finish 需要成功 test/build。
- final answer 仍拒绝泄露内部 TaskSpace / orchestration protocol terms。

## 3. 关键反向测试

新增或改写的边界测试：

- `start_task_initial_inputs_do_not_seed_problem_ledger_or_cognitive_state`
- `start_task_does_not_seed_missing_scaffold_from_user_request`
- `start_task_initial_scaffold_is_not_active_preflight_input`
- `start_task_does_not_derive_output_contracts_from_objective_or_initial_records`
- `validation_finish_does_not_require_success_criterion_or_auto_acceptance`
- `final_response_does_not_require_ledger_readiness_or_decision_records`
- `start_task_allows_agent_to_bypass_broad_delegation_debt`
- `route_task_allows_agent_to_bypass_broad_delegation_debt`

## 4. 验证

已通过：

```text
cargo fmt --all
cargo check -p codex-core
cargo build -p codex-cli --bin whale
cargo test -p codex-core start_task_ -- --nocapture
cargo test -p codex-core developer_context_uses_active_projection_replacement_after_task_start -- --nocapture
cargo test -p codex-core validation_finish_does_not_require_success_criterion_or_auto_acceptance -- --nocapture
cargo test -p codex-core final_response_does_not_require_ledger_readiness_or_decision_records -- --nocapture
cargo test -p codex-core taskspace_action_contract -- --nocapture
cargo test -p codex-core gate_recovery -- --nocapture
```

说明：`cargo fmt --all` 在当前 stable toolchain 下会打印 `imports_granularity = Item` 的稳定性 warning；格式化命令执行成功。

## 5. 旧测试套件处置

`cargo test -p codex-core validation -- --nocapture` 会命中大量旧 R4/R5 语义控制断言，例如：

- runtime 自动 accept successful validation result。
- runtime 自动满足 success criteria。
- final response 必须等待 ledger readiness。
- validation failure 自动 route 到 runtime 选择的 rework path。

这些断言与 R5-D 的边界原则冲突，不能作为 Phase D active path 验收标准。Phase D 只保留/新增反向测试来证明这些语义控制已退出 active path。R5-E/F 需要继续清理或重分组 legacy validation/rework 测试，避免旧测试名称误导后续判断。

## 6. 收益验证口径

Phase D 的收益不是“让 runtime 更会纠错”，而是减少上下文污染和 runtime 越界控制。样本验证重点看：

1. provider-visible payload 是否不再出现 ledger/cognitive protocol。
2. taskspace 是否不再因 `initial_*` 或 ledger readiness 固化局部错误。
3. request/tool count 是否没有负向放大。
4. 失败时优先检查上下文传递，而不是新增 semantic gate。

## 7. 样本收益验证

本阶段选择 `count-call-stack`，因为它覆盖普通工具读取、编辑、验证和反馈闭环；同时它是
R4/R5 前序阶段反复使用过的正向回归样本。

命令：

```text
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 \
  -Scenario count-call-stack \
  -Repeats 1 \
  -RunSide both \
  -RunRoot target/r5d-ledger-deactivation \
  -WhaleBin /home/zhangxu/whalecode-alpha/third_party/codex-cli/codex-rs/target/debug/whale \
  -TimeoutSeconds 900
```

运行结果：

```text
RunDir: target/r5d-ledger-deactivation/count-call-stack/20260710-002316-050
PairReport: target/r5d-ledger-deactivation/count-call-stack/20260710-002316-050/pair-001/pair-report.md
reported_evidence_level: E2-candidate
utility_direction: both_success
failure_taxonomy: none
engineering_unclean: False
```

横向对照：

| 版本 | 来源 | outcome | wall time | tool calls | provider request |
|---|---|---:|---:|---:|---:|
| standard 当前 | 同次 left | solved | 26197ms | 14 | outer exec 1 |
| R4 历史基线 | `target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136` 文档记录 | solved | 154525ms | 11 | 未记录同口径 |
| R5-D 当前 | 同次 right | solved | 17346ms | 11 | outer exec 1；rollout 内部 7 |

收益信号：

- TaskSpace 当前样本 business success、public validation、hidden oracle 均通过。
- TaskSpace tool call ratio 为 0.79，wall time ratio 为 0.66；没有出现 Phase D 引入的负向放大。
- `taskspace_control_count=0`、`state_commit_count=0`，说明本样本只依赖机械空 map 和普通工具反馈即可完成，未依赖 ledger/cognitive 语义控制。
- provider-visible forbidden scan 无命中：

```text
TaskSpace cognitive protocol
Task problem-state ledger
Task cognitive state
final_synthesis closeout requires
state_machine_allowed_actions
next_valid_actions
problem-state and model manager
cognitive preflight
result validity is required
final answer cannot be emitted until every success criterion
allowed action classes
hard action-class constraints
success criterion
```

口径说明：

- standard side 没有 `rollout.jsonl`，只能用外层 `whale-exec` token/request 汇总；TaskSpace side 同时有外层汇总和内部 rollout provider lifecycle。
- 因此本阶段不声明 request parity，只记录：外层 exec 口径双方均为 1；TaskSpace 内部 provider lifecycle 为 7。
- R4 历史基线来自既有文档记录，不是本轮同机重跑；只作为回归方向参考。

## 8. 结论

Phase D 的 D1/D2 已完成：TaskSpace active path 不再依赖旧 semantic ledger。`problem_ledger` /
`cognitive_state` 仍可作为 Agent-authored 可选记录存在，但不再是 runtime projection、closeout、final response 或 broad strategy gate 的依据。
