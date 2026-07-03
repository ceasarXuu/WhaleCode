# R4 接手进度盘点

报告日期：2026-07-03  
接手范围：`v0.0.5 build-R4` tools chain 专项、R4 文档、机器可读门禁、public-10 证据、当前 `whalecode-alpha` HEAD  
接手基线 HEAD：`7ecbeb5 R4 restore durable evidence gates`

## 1. 总体判断

| 维度 | 当前状态 | 完成度 | 判断依据 |
|---|---:|---:|---|
| R4 phase 流程 | 已推进到 R4-H，并进入 post-closeout 修复 | 8/8 phase 已走完 | `06-r4-engineering-closeout.md` 已记录 closeout 和 E3 no-go |
| 工具链工程治理 | 可继续作为 R4 已交付资产 | 约 90% | tool path coverage、sample ledger、public-10 plan、usage accounting gate 当前均可在 fresh checkout 通过 |
| TaskSpace utility parity | 未完成 | public-10 closeout 为 3/10 solved；post-closeout 已修复 `heterogeneous-dates` 执行层问题，但未覆盖全部 public-10 失败项 |
| E3 readiness | 阻塞 | 0% | key 已解锁真实复验，但 `organization-json-generator` 仍只能产出 E1 诊断证据；负收益和 timeout/wrong 样本仍未收敛 |

## 2. Phase 状态

| Phase | 主题 | 当前状态 | 证据 |
|---|---|---|---|
| R4-A | Tool path inventory | 已关闭 | `r4-tool-path-coverage.json` 10 paths 全 canonical；`test-r4-tool-path-coverage.ps1` pass |
| R4-B | Field evidence ledger | 已恢复 fresh-checkout 可验证 | `r4-sample-evidence-ledger.json` 12 samples；必需 evidence 改为仓库内 durable docs/CoE，原 run path 保留为 archived path |
| R4-C | Direct tool feedback contract | 已关闭 | direct success/error preview parity 已纳入 coverage manifest 和 focused tests |
| R4-D | Action-contract internal tool parity | 已关闭工程门禁，仍有 utility 后续 | `count-call-stack`、`large-output-ref-smoke` 等样本显示反馈链路修复有效；后续 public 样本仍暴露 long-flow 问题 |
| R4-E | Projection/output-ref safeguards | 已关闭工程门禁 | large output rollout 从约 491MB 降到 360KB 的证据已记录；pair-safe projection 有 focused coverage |
| R4-F | Non-direct tool coverage | 已关闭 | CodeMode/MCP/multi-agent paths 均 canonical |
| R4-G | Public-10 benchmark/report gate | 工程验收机制完成；utility 负收益 | public-10 计划和 snapshot 当前可验证；closeout 记录 TaskSpace 3/10 solved 和请求放大 |
| R4-H | Closeout | 已完成但需维护 evidence durability | closeout 存在；本次修复补齐 fresh-checkout gate reproducibility |

## 3. 本次接手修复

### 3.1 发现的问题

当前 checkout 上 R4 证据门禁一开始不能完整复现：

1. `test-r4-sample-ledger.ps1` 失败，因为 ledger 的必需 evidence 指向未提交的 `target/...` 历史运行产物。
2. `test-r4-public-10-usage-accounting-gate.ps1` 失败，因为它默认从 `C:\WhaleRunCache` 生成 good report；Linux fresh checkout 没有这些外部 run roots，报告为 `0/10 found`。

CoE：`coe/2026-07-03-05-03-r4-durable-evidence-gates.md`

### 3.2 修复内容

| 文件 | 变更 |
|---|---|
| `docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json` | 必需 `primary_evidence` / `secondary_evidence` 改为仓库内 durable docs/CoE；原始 `target/...` 路径保留在 `archived_run_path(s)` |
| `docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json` | 新增 public-10 durable snapshot，用于 fresh checkout 验证 report shape、accounting availability 和 request amplification 字段 |
| `scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1` | 默认读取 durable snapshot 作为 good report，再执行原有正向/负向 mutation gate |
| `coe/2026-07-03-05-03-r4-durable-evidence-gates.md` | 记录问题、假设、诊断证据和 fix-validation |

### 3.3 sqlite/organization 后续假设复核

接手继续检查 `coe/2026-07-01-04-05-r4-repeated-blocked-action.md` 时发现：

| 假设 | 当前复核结论 | 验证 |
|---|---|---|
| H-035 mixed native/unified apply_patch headers | 当前代码已修复并有聚焦测试覆盖 | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_apply_patch_strips_unified_file_headers_inside_native_update --lib` PASS |
| H-036 validation rework summaries lose high-signal output | 当前代码已修复并有聚焦测试覆盖 | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_failure_summary_preserves_error_after_warning_noise --lib` PASS |

这两项已在 CoE 中从 active 更新为 `repaired-and-validated-by-focused-test`，并新增 E-016/E-017。它们说明 `organization-json-generator` 后续问题不应继续归因到这两个已验证的工具链缺陷；下一步应回到真实 public sample 重新跑证据，确认剩余失败是否来自 request/timeout envelope、patch recovery convergence 或模型解题策略。

### 3.4 organization-json-generator Linux 复验入口

接手继续执行 `organization-json-generator` 时，先修复了 Linux harness 前置问题，而不是直接声称 utility 进展：

| 层 | 结论 |
|---|---|
| pinned source | 已稀疏 checkout `terminal-bench` `91e10457b5410f16c44364da1a34cb6de8c488a5`，任务目录为 `tasks/organization-json-generator` |
| plan-only | 已通过：`PromptInvalid=False`、`PromptManualReview=False` |
| Linux harness | 已修 Windows-only primitive：`WindowsIdentity`、`curl.exe`、`cmd.exe` validator launcher、`subst`、null `USERPROFILE` rollout lookup |
| provider preflight | 已新增 `provider-credential-preflight-health.json`；当前主机缺 `DEEPSEEK_API_KEY` 时以 `provider_credential_missing` fail-fast；该行为已纳入 external wrapper focused harness |
| validator env | 已修复 native Docker loopback proxy build：`pip install jsonschema` 可通过；直接 validator run 会进入预期业务断言失败 |

CoE：`coe/2026-07-01-04-05-r4-repeated-blocked-action.md` H-037/H-038/H-039。

### 3.5 keyed organization-json-generator 复验结论

用户将 `DEEPSEEK_API_KEY` 放入 `.env.local` 后，R4 readiness gate 已可通过 provider credential 检查；key 不写入仓库、不写入文档。随后执行 `organization-json-generator` 真实 keyed rerun，得到两个结论：

| 事项 | 结论 |
|---|---|
| provider credential | `provider_credential_preflight_completed status=pass`，说明 `.env.local` 配置已经被 harness 子进程继承 |
| metrics extractor durability | 首次 keyed run 暴露 `.python-version` 消失导致的 metrics post-processing crash；已修复为 `hash_status=missing` 并由 focused harness 与第二次真实 run 验证 |
| real pair report | 第二次 keyed run 成功写出 `run-summary.md`、`pair-report.md`、左右 metrics |
| evidence level | `reported_evidence_level=E1`，`included_in_utility_aggregate=False`，不能作为 R4 验收通过证据 |
| standard path | `exec_exit_code=0`，但 `public_validation_exit_code=1`，业务结果 wrong |
| TaskSpace path | `exec_exit_code=124`、`exec_timed_out=True`、`wall_time_ms=900088`、`tool_call_count=92`，业务结果 `agent_exec_timeout` |
| convergence signal | TaskSpace request budget 进入 `request_count=89->90 max=20 state=over_profile_hint`，`TaskSpaceNoActionRecoveryV1` 到 recovery attempt 32；同时出现 `bwrap: loopback: Failed RTM_NEWADDR` |

Run root：

```text
target/r4-org-json-real-keyed-20260703b/runs/terminal_bench__organization-json-generator/20260703-155610-406
```

当前判断：R4 的 provider 前置阻塞已经解除；新的 P0 阻塞是 TaskSpace 在沙箱/工具失败条件下不能快速收敛到 bounded blocked-with-evidence，导致 900s timeout 和请求预算放大。该问题应作为 R4 utility-convergence 继续处理，不能把 R4 标记为完成。

### 3.6 bwrap/tool-runtime bootstrap failure 收录与修复

本轮将 keyed `organization-json-generator` 中的 `bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`
正式收录为 R4-D P0 path：`tool-runtime-bootstrap-failure`。

结论：

| 层 | 结论 |
|---|---|
| 能力层 | `exec.rs` 明确识别 bwrap loopback/RTM_NEWADDR bootstrap signature，且 `SandboxType::None` 不误判 |
| 反馈层 | `ActionMapRuntime` 对普通工具节点记录 `tool_runtime_bootstrap_failure`、`failure_kind=sandbox_bootstrap_failed`、task-level blocker，并释放 current node |
| 验证节点 | 同类失败仍归入 local validator infra invalidation，但不会生成可重试 rework node |
| 下一轮 contract | 无 active node 且存在该 blocker 时，只允许 `final_answer` 或 `blocked`；runtime rewrite 会拦截继续 `create_node`、read/search/test/edit 等普通工具动作 |
| R4 manifest | `r4-tool-path-coverage.json` 增加第 11 条 canonical path，并绑定 focused Rust coverage tests |

当前状态：该 feedback-layer case 已完成 focused 修复和轻量门禁验证；还不能替代真实 utility 复验。下一步仍需要重跑 `organization-json-generator`，验证 900s timeout 是否消失，以及是否进入可聚合的 E3 utility evidence。

### 3.7 tools feedback 子类型继续收敛

继续接手 R4 后，`organization-json-generator` 又暴露出一组同源 tools 链路问题。它们不是“工具没返回任何错误”，而是
raw signal 存在但语义没有正确进入下一轮 tool contract：

| Case | 结论 | 状态 |
|---|---|---|
| `tool-platform-command-mismatch` | recovery payload 现在按 host platform 生成 Unix `sed` 或 Windows `Get-Content` | focused fixed |
| `duplicate-successful-evidence-loop` | inspect 重复 read/search 现在会被 named gate 拦截并携带 previous result / repeat state | focused fixed |
| `inspect-data-artifact-evidence-gap` | `.json` / `.csv` 这类 input data artifacts 已计入 inspect working evidence | focused fixed |
| `validation-changed-artifact-coverage-feedback` | 不覆盖 changed artifact 的 validation gate 现在回传 required command / next action | focused fixed |
| `validation-command-missing-script-feedback` | 不存在的 validation script 留在 validation node，不再误转 implement rework | focused fixed |
| `duplicate-inspect-premature-fact-source-convergence` | 本次新增收录 case；缺少 `employees.csv` / `projects.csv` 这类声明 fact sources 时，duplicate gate 不再给 `finish_node`，manual/forced finish 都会被 coverage guard 拦截 | focused fixed |
| `provider-budget-advisory-runaway` | keyed rerun `20260703d` 证实 `request_count` 达到 `max_requests` 后仍继续 provider sampling；已升级为 pre-dispatch hard gate，插入 `TaskSpaceProviderBudgetHardStopV1` 并结束当前 turn，保留一次明确 budget recovery grace | focused fixed / real rerun pending |
| `provider-node-budget-premature-inspect-stop` | keyed rerun `20260704-000713-854` 证实 hard stop 消除 timeout 后，固定 per-node limit 又在 fact-source evidence floor 前终止 inspect；已改为按声明 fact-source artifacts 扩展 inspect effective node limit，并在边界 recovery 时标记 `budget_recovery` | focused fixed / real rerun pending |
| `implementation-rework-feedback-evidence-join` | keyed rerun `20260704-001749-411` 证实 inspect 已读全 fact sources 后，implement rework 仍逐行修 `IndentationError` 并凭空使用不存在的 `salary` 字段；已改为合并 validation failure 与上游 inspect CSV/schema evidence，并强化 recovery next-action contract | focused fixed / real rerun pending |
| `inspect-projection-finish-before-fact-source-coverage` | keyed rerun `20260704-003459-046` 证实底层 fact-source guard 已知道缺 `projects.csv`，但 projection `next_valid_actions` 仍广告 `finish_node -> implement_solution`；已改为 projection 复用 TaskState fact-source coverage guard，缺 artifact 时只提示读取缺失事实源 | focused fixed / real rerun pending |
| `implementation-editable-validation-failure-misblocked` | keyed rerun `20260704-004643-993` 证实 projection guard 后 TaskSpace 已读全 fact sources 并进入 implement/validation，但 `IndentationError` rework 可被 `block_node` 误关成 infra/closed validation；已改为拒绝 editable validation failure blocker，并输出 structured recovery | focused fixed / real rerun pending |
| `validation-closeout-output-contract-coverage-gap` | generator-only validation 被 forced closeout 当成 schema/output contract success；已要求同一次 validation 覆盖 output/schema contract，并重开引用 generator-only result 的 success criteria | focused fixed / real rerun pending |
| `validation-output-contract-schema-fact-source-gap` | `schema.json` 只作为 fact source 时，弱 `json.load` validation 被误接受；已把 fact sources 中的 schema/validator artifacts 纳入 output-contract coverage | focused fixed / real rerun pending |
| `validation-recovery-next-action-projection-dilution` | gate recovery 已给出 exact `jsonschema` 命令，但 projection 又退化成泛化 test action；已让 validation projection 保留最新 exact recovery next actions | focused fixed / real rerun pending |
| `validation-rework-target-artifact-read-gap` | schema failure 无 traceback 时 rework 不能读取变更代码 artifact；已从 blocked validation dependency changed artifacts 推导命名 rework target | focused fixed / real rerun pending |
| `validation-jsonschema-module-missing-rework-misroute` | host 缺 `jsonschema` module 被误当成 implementation failure；已留在 validation node 并投影 CLI `python -m jsonschema -i ...` recovery | focused fixed / real rerun pending |
| `implementation-rework-repeat-read-budget-drain` | Unix `sed` 读文件结果缺 artifact ref，duplicate rework gate 失明；已给 `sed -n ... -- path` read 记录 artifact ref，并拦截重复 target read | focused fixed / real rerun pending |
| `validation-blocker-manual-rework-origin-loss` | validation recovery 中手动 create rework node 丢失 blocked validation origin，patch 被 unreviewed-result gate 拦截；已记录 origin 并在 origin blocked 后刷新对应 rework node | focused fixed / real rerun pending |
| `validation-stale-failure-block-without-current-test` | 新 validation node 未运行当前 test/build 就复用旧失败 blocker 关闭 graph；已要求声称 validation/test failure 前必须有同节点验证 tool result，infra blocker 例外 | focused fixed / real rerun pending |
| `validation-rework-duplicate-read-projection-loop` | rework target 已读且 duplicate gate 已生效，但 projection 仍广告 read/search、未展示 target read result，模型重复读到 node hard stop；已把 target read result 作为 critical evidence 并将 next action 收窄到 apply_patch | focused fixed / real rerun pending |

新增关键判断：

- 这是反馈层“语义缺失”多于“语义扭曲”：原始失败/证据信号进入了 trace 或 tool output，但缺少 failure kind、required command、missing artifact 或 phase completion guard。
- `taskspace runtime` 可以负责工具反馈分类和 phase gate，但不能超越状态机底线把“重复证据”解释成“inspect 已完成”。
- R4 当前实际进行位置：phase 流程已到 R4-H/post-closeout；工程上继续在 R4-D feedback layer 和 R4-G utility-convergence 做回补，不应重新标记 R4 已验收。

### 3.8 provider budget hard stop

在 `organization-json-generator` keyed rerun `target/r4-org-json-real-keyed-20260703d/.../whale-exec.jsonl` 中，前置 feedback fixes 已经让 inspect 读到
`schema.json`、`departments.csv`、`employees.csv`，没有再触发 duplicate-read premature implement；但 `projects.csv` 未读时，
provider request budget 从 `19->20 max=20` 后继续到 `26->27`，全部处于 `over_profile_hint`。

本轮修复：

| 层 | 结论 |
|---|---|
| action map | `gate_provider_request_pre_dispatch` 对 rollout/node budget 超限返回 `allowed=false`，reason 分别为 `provider_request_hard_limit_exceeded` / `provider_node_request_hard_limit_exceeded` |
| session | `Session::action_map_gate_provider_request_pre_dispatch` 暴露 hard gate 给采样路径 |
| turn loop | `try_run_sampling_request` 在 `stream_with_provider_request_budget` 前阻断；外层 loop 记录 hard-stop item 后直接结束当前 turn |
| feedback | 新增 `TaskSpaceProviderBudgetHardStopV1`，携带 request/node/grace 计数、blocking items 和 next valid actions |
| grace | 只允许 `request_phase=budget_recovery` 且 grace 未消耗时继续一次恢复请求 |

第一次新二进制真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703e-hardgate/runs/terminal_bench__organization-json-generator/20260704-000713-854
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 5
TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=5/20 node_request_count=5/5
```

结论：provider budget runaway / 900s timeout 已被 hard stop 截断；但固定 per-node limit 过早终止 inspect，仍未读
`employees.csv` / `projects.csv`。因此本轮继续修复 `provider-node-budget-premature-inspect-stop`：inspect effective
`max_model_requests_per_node` 根据声明 fact-source artifacts 扩展，且 recovery item 到达预算边界时下一次 provider request 标记为
`budget_recovery`。

当前状态：focused 修复已完成；仍需用新二进制重跑 `organization-json-generator` 验证真实 wrong 是否继续推进到 solved 或新的 failure class。

### 3.9 implementation rework feedback evidence join

adaptive budget 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703f-adaptive-budget/runs/terminal_bench__organization-json-generator/20260704-001749-411
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 16
```

进展和新问题：

| 层 | 结论 |
|---|---|
| inspect | 已读取 `employees.csv`、`departments.csv`、`projects.csv` 和 schema，说明 adaptive fact-source budget 越过了上一层 blocker |
| validation rework | 失败信息进入 trace，但 recovery 没有把失败和上游 CSV/schema evidence 合并为同一 action contract |
| failure shape | 先是整文件顶层缩进导致 line 1 / line 2 `IndentationError` 被逐行修，后续 replacement 又引用不存在的 `salary` 字段并触发 `KeyError: 'salary'` |
| budget | provider hard stop 正常阻断在 `request_count=20/20`，但预算被低质量 rework 消耗完 |

本轮修复：

| 层 | 结论 |
|---|---|
| action map | `current_main_working_evidence_summary()` 使用当前节点的有界依赖闭包，并把 `validation_rework` summary 与上游 inspect working evidence 合并 |
| session recovery | `TaskSpaceImplementNeedsEditRecoveryV1` 明确 validation failure 是 primary target；Python 顶层 `IndentationError` 需要按整文件/整块修；`KeyError` 只能使用已观察到的 schema/CSV/JSON 字段 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_summary_merges_transitive_inspect_evidence_and_failure --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core implement_recovery_prioritizes_validation_failure_and_inspected_fields --lib
```

当前状态：focused 修复已完成；仍需重跑 `organization-json-generator` 验证是否越过 implement rework，或者按新 trace 收录下一层 R4-D/R4-G blocker。

### 3.10 inspect projection fact-source guard

rework evidence join 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703g-rework-evidence/runs/terminal_bench__organization-json-generator/20260704-003459-046
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 11
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| timeout / hard stop | 900s timeout 没有复发，provider node budget hard stop 在 `request_count=11/20`、`node_request_count=11/10` 截断 |
| inspect evidence | TaskSpace 已读 `schema.json`、`departments.csv`、`employees.csv`，但仍缺 `projects.csv` |
| feedback visibility | final projection 的 `verified_input_evidence` 明确只列前三个输入，说明缺失事实源可从 runtime 状态推导 |
| projection bug | 同一 projection 的 `next_valid_actions` 仍包含 `finish_node -> implement_solution`，没有复用底层 fact-source coverage guard |

根因判断：

这个 case 是反馈层“语义缺失”而不是“语义扭曲”。底层 duplicate/manual/forced finish guard 没有错，缺失的是
provider-visible `projection_next_valid_actions` 这一路没有拿到 `TaskState`，因此无法调用声明 fact-source coverage 判断。

本轮修复：

| 层 | 结论 |
|---|---|
| context projection | `append_context_projection_with_header` 调用 `projection_next_valid_actions(map, current_node_id, Some(task))` |
| next action policy | inspect 节点缺声明 fact-source artifacts 时，projection 只输出 `read_file declared fact-source artifact ... next` 和 `do not finish inspect_code_context...` |
| regression | 保留原先“已有证据且无缺失 fact source 时可进入 implement”的 projection 行为 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_blocks_inspect_finish_until_declared_fact_sources_read --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_prioritizes_inspect_to_implement_after_evidence --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources_block_manual_and_forced_finish_until_read --lib
```

当前状态：focused 修复已完成；仍需重跑 `organization-json-generator` 验证 projection 不再误导模型提前 finish，
并观察 TaskSpace 是否能读取 `projects.csv` 后进入 implement。

### 3.11 implementation editable validation failure misblock

projection guard 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703h-projection-factsource/runs/terminal_bench__organization-json-generator/20260704-004643-993
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 10
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| inspect | 已读 `schema.json`、`departments.csv`、`employees.csv`、`projects.csv`，说明 projection fact-source guard 的真实效果成立 |
| implement | 创建了 `generate_organization.py`，但 Add File patch 每行带一个额外前导空格 |
| validation | `python generate_organization.py` 先报 line 2 `IndentationError`，一行 patch 后又报 line 3 `IndentationError` |
| rework closeout | 新 rework node 接受 `block_node`，最后 blocked reason 称 closed validation prevents further editing，并将缩进错误归为 `infra-evidence-unresolved-indentation` |
| output contract | public validation 失败：`/app/organization.json` 不存在 |

根因判断：

这是 control/feedback 语义扭曲：validation failure 是可编辑实现错误，应该继续 patch 失败 artifact；runtime 不应接受
rework node 把它 block 成 infra/closed validation。现有守卫覆盖了 missing source、validator procedure、internal policy，
但没有覆盖“editable validation failure as blocker”。

本轮修复：

| 层 | 结论 |
|---|---|
| action map | `block_main_node` 在 implement rework 无 successful edit、依赖 validation evidence 明确是 `IndentationError` / `SyntaxError` / `KeyError` 等可编辑失败时拒绝 `block_node` |
| feedback | action-contract recent output 新增 `failure_kind: editable_validation_failure_blocker_rejected` |
| next action | recovery 明确要求 patch failed validation artifact；Python 顶层缩进/语法错误要按 whole file / block 修，不能 block for inspection |
| regression | 已保持 validator-procedure blocker、missing-source blocker 和 implement recovery 行为通过 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_editable_validation_failure_blocker_before_edit --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_editable_validation_failure_blocker_rejection --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_validator_procedure_blocker_before_edit --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_missing_current_artifact_visibility_blocker --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core implement_recovery_prioritizes_validation_failure_and_inspected_fields --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_validator_procedure_blocker_rejection --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_missing_source_blocker_rejection --lib
```

当前状态：focused 修复已完成；仍需重跑 `organization-json-generator` 验证 TaskSpace 是否继续 patch 缩进问题、生成
`organization.json`，并通过 public validation。

第二次复验：

```text
RunDir: target/r4-org-json-real-keyed-20260703i-editable-blocker/runs/terminal_bench__organization-json-generator/20260704-005922-113
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 8
```

进展：TaskSpace 仍无 900s timeout，并且已读全 fact sources、创建 `processor.py`、进入 validation。
未通过原因仍属于同一 `implementation-editable-validation-failure-misblocked` 类型，但 provider wording 变成：

```text
Test failed with IndentationError; cannot read files to diagnose because read actions are not allowed in current narrowed state
```

该 wording 最初没有命中 `blocker_claims_editable_validation_failure_as_blocker`，因此 runtime 接受了 `block_node`。
已把 `cannot read`、`read actions are not allowed`、`read restriction`、`insufficient information` 和
`current narrowed state` 纳入同一 detector，并用该真实文案更新 focused test。

### 3.12 validation closeout output contract coverage gap

editable blocker wording 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703j-editable-wording/runs/terminal_bench__organization-json-generator/20260704-010752-603
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 8
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| implement | TaskSpace 创建 `generate_json.py`，并成功生成 `organization.json` |
| validation | 只运行 `python generate_json.py`；该命令 exit 0 只证明 generator 执行成功 |
| closeout | runtime 触发 `TaskSpaceForcedValidationCloseoutV1 trigger=validation_success_after_tool_drain`，把 generator success 当成 validation success |
| final answer | 声称 `organization.json` follows `schema.json` |
| public validator | 失败于 `KeyError: 'members'` 和 `KeyError: 'averageDepartmentBudget'`；输出使用 `member_ids` 和 snake_case statistics |

根因判断：

这是 validation/feedback 语义缺失，不是工具执行失败。raw tool success 正确传递了 `exit_code=0`，但 runtime 把“脚本运行成功”
提升成“输出契约验证成功”。R4 tools 链路必须区分 execution success、generator success、output/schema contract success。

本轮修复：

| 层 | 结论 |
|---|---|
| validation gate | 对声明 output contract artifacts 的 validation command 增加覆盖检查；generator-only command 输出 `validation_test_missing_output_contract_coverage` |
| next action | recovery 保留 combined command，例如 `python generate_json.py && python -m jsonschema -i organization.json schema.json` |
| forced closeout | 若 generator-only successful result 已记录，closeout 前会重开引用该 result 的 satisfied success criteria，并将 result 标记 invalid |
| regression | 直接输出 artifact 的数值验证、local validator、changed-artifact coverage、validation infra/rework 路径保持通过 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks_generator_only_command_for_schema_output_contract --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation_rejects_generator_only_output_contract_success --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt_structures_output_contract_coverage_failure --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_output_contract_coverage_recovery_preserves_next_action --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 是否不再 generator-only closeout，并实际修正
`organization.json` 的 schema/public-test contract。

### 3.13 schema fact-source weak validation gap

output-contract coverage 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703k-output-contract/runs/terminal_bench__organization-json-generator/20260704-013819-201
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 9
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| validation gate | `python process.py` 被 `validation_test_missing_output_contract_coverage` 拒绝；H-014 guard 已在真实 run 中生效 |
| feedback | session 插入 `TaskSpaceValidationNeedsTestRecoveryV1`；模型收到失败语义并改变下一步 |
| weak validation | 模型改跑 `python process.py && python -c 'import json; data=json.load(open("organization.json")); print("Valid")'` |
| semantic gap | runtime 接受了 JSON parse，final answer 声称 schema validation 成功 |
| public validator | 仍失败于 `KeyError: 'members'` 和 `KeyError: 'averageDepartmentBudget'` |

根因判断：

H-014 修掉了“generator-only execution success 被 closeout 当成 validation success”。本轮不是同一个旧问题复发；
反馈传递是有效的，模型也响应了反馈。新的缺口是 coverage 语义仍然过宽：`schema.json` 在该任务中作为
`initial_fact_sources` / success criterion 出现，而不是 output contract 本身。runtime 只从 output contracts 抽
schema target，导致 `json.load` 这种弱检查被误判为 output contract validation。

本轮修复：

| 层 | 结论 |
|---|---|
| requirement extraction | 从 output contracts、success criteria、fact sources 一并提取 schema/validator artifacts |
| command semantics | 有 schema/validator target 时，命令必须包含 `jsonschema`、`validate`、`pytest`、`run-tests` 等真实 validator 语义；普通 `json.load` / `python -c` 不再足够 |
| next action | schema fact source 场景也会保留 `python process.py && python -m jsonschema -i organization.json schema.json` |
| regression | 直接输出 artifact 仍可用具体 `assert` 检查 JSON 结构；local validator、changed-artifact coverage、forced closeout 路径保持通过 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
cargo fmt --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 是否实际运行 schema/public-equivalent validation，
并修正 `organization.json` 的字段 contract。

### 3.14 validation recovery next-action projection dilution

schema fact-source guard 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703l-schema-factsource/runs/terminal_bench__organization-json-generator/20260704-014928-473
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 13
right_open_leaf_nodes: 1
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| validation gate | 多次弱 validation 被 `validation_test_missing_output_contract_coverage` 拒绝；schema fact-source guard 已生效 |
| gate recovery | `TaskSpaceGateRecoveryV1.next_valid_actions` 给出 `python process.py && python -m jsonschema -i organization.json schema.json` |
| recovery feedback | `TaskSpaceValidationNeedsTestRecoveryV1` 要求 obey `next_valid_actions`、use the named command exactly |
| active projection | 紧随其后的 `ContextProjectionV1 active replacement` 只展示 `run validator/test command` |
| budget | smoke node 在重复弱尝试后触发 `provider_node_request_hard_limit_exceeded request_count=14/20 node_request_count=6/5` |
| public validator | 仍失败于 `/app/organization.json does not exist`，说明未执行 schema-validating command |

根因判断：

这次不是 raw tool failure，也不是 gate recovery 缺失。精确失败语义在 `TaskSpaceGateRecoveryV1` 和
`TaskSpaceValidationNeedsTestRecoveryV1` 中已经存在；语义丢失发生在 provider-visible projection 层。active projection
重新根据 validation node kind 生成了泛化 `run validator/test command`，覆盖了前一条精确 recovery 的行动约束。

本轮修复：

| 层 | 结论 |
|---|---|
| runtime state | 记录 latest gate recovery `next_valid_actions`，keyed by map/node |
| projection | smoke/regression node projection 若存在 latest gate recovery，优先原样输出 exact recovery commands |
| feedback constraint | projection 追加 `do not substitute weaker validation; use the exact recovered command unless it cannot run` |
| cleanup | 当前节点记录新 main tool result、清理 blocked repeats 时同步清掉对应 recovery 状态，避免污染后续节点 |
| regression | schema fact-source 测试现在同时断言 active projection 包含 exact `jsonschema` 命令且不再含泛化 `run validator/test command` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 是否按 projection 中的 exact command 执行
`python process.py && python -m jsonschema -i organization.json schema.json`。

### 3.15 validation rework target artifact read gap

recovery projection 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703m-recovery-projection/runs/terminal_bench__organization-json-generator/20260704-020629-368
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 8
public_validation_exit_code: 1
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| exact validation | TaskSpace 执行了 `python generate_org.py && python -m jsonschema -i organization.json schema.json` |
| schema feedback | jsonschema 返回真实业务错误：project 缺 `members`，statistics 缺 `averageDepartmentBudget`、`totalEmployees`、`skillDistribution` 等 required keys |
| rework routing | runtime 插入 `TaskSpaceImplementNeedsEditRecoveryV1`，说明 validation failure 正确进入 implement rework |
| action contract | `implementation_needs_edit` 下 `read_file schema.json` 被拒绝为 `node_policy_violation`，泛读拦截仍正确 |
| target visibility | 模型随后 block：无法读取 `generate_org.py` 就不能做正确 patch |

根因判断：

这次不是 validation feedback 丢失，也不是 projection dilution。exact schema command 已经执行，schema failure 也正确进入 rework。
缺口在 rework 目标工件投影和 session action contract 之间：schema failure 没有 traceback path，旧逻辑只从 failure text
找目标文件，没有把 blocked validation dependency 的 changed artifact `generate_org.py` 作为可读取 rework target 暴露给
`implementation_needs_edit`。

本轮修复：

| 层 | 结论 |
|---|---|
| runtime dependency projection | `implement_node_dependency_validation_rework_artifact_refs` 合并 blocked validation dependency 的 changed artifacts |
| provider snapshot | 新增 `current_node_validation_rework_artifacts`，供 action contract state 和 projection 使用 |
| projection | implement rework 未编辑前，`next_valid_actions` 明确列出命名 target artifact read 和 patch |
| session action contract | `implementation_needs_edit` 下只允许读取命名 validation rework target artifact；`schema.json` 等泛读仍被拒绝 |
| regression | 新增 runtime 和 session focused tests 覆盖 schema failure 无 traceback、命名读取放行、泛读拒绝 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_allows_named_validation_rework_artifact_read --locked
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 是否读取或直接 patch `generate_org.py`，
并把 `organization.json` 修到 schema/public validator 通过。

### 3.16 validation jsonschema module missing rework misroute

target artifact read 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703n-rework-target-read/runs/terminal_bench__organization-json-generator/20260704-022632-418
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 9
right_open_leaf_nodes: 1
public_validation_exit_code: 1
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| previous fixes | TaskSpace 读齐 schema/CSV，创建 `organization.json`，并执行 schema validation 语义命令 |
| validation command | 使用 `python3 -c "import json, jsonschema; ... jsonschema.validate(...)"` |
| failure semantic | 失败是 `ModuleNotFoundError: No module named 'jsonschema'`，schema validation 未真正执行 |
| target read | rework 中成功读取 `organization.json`，证明 3.15 修复有效 |
| control loop | runtime 将该失败路由到 implement rework 后，模型反复 `finish_node`，最终 `provider_node_request_hard_limit_exceeded request_count=15/20 node_request_count=6/5` |

根因判断：

`jsonschema` module missing 不是 output schema mismatch，也不是可由 implementation patch 直接修复的 evidence。旧分类只把
E_ACCESSDENIED、bwrap、uv cache 等少数 case 视为 local infra，并把 `ModuleNotFoundError` 纳入 failed validation
noninfra path，导致 validation node 被 block 并创建 rework。正确边界是：这种失败应留在 validation node，并给出同一 schema
contract 的可执行替代命令，例如默认 Python 环境下的 `python -m jsonschema -i organization.json schema.json`。

本轮修复：

| 层 | 结论 |
|---|---|
| validation classification | 新增 `validation_failure_is_missing_jsonschema_dependency`，从 noninfra implementation rework 分类中排除 |
| projection recovery | validation node 看到该失败后，基于 output contract/schema requirements 输出 `python -m jsonschema -i organization.json schema.json` |
| local infra guard | E_ACCESSDENIED、uv cache、bwrap 等既有 local infra tests 保持通过 |
| regression | 新增 focused test 覆盖 missing jsonschema 不进入 rework、current node 保持 validation、projection 给 CLI recovery |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_missing_jsonschema_dependency_stays_on_validation_with_cli_recovery --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core local_infra --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 是否改用 `python -m jsonschema`，
并在真实 schema mismatch 后继续修正 `organization.json`。

### 3.17 implementation rework repeat-read budget drain

jsonschema module missing 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703o-jsonschema-recovery/runs/terminal_bench__organization-json-generator/20260704-024204-931
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| validation command | TaskSpace 使用 `python csv_processor.py && python -m jsonschema -i organization.json schema.json`，说明 H-018 recovery 已推动到可执行 schema validation |
| failure semantic | schema 输出缺 `members`、`averageDepartmentBudget`、`totalEmployees`、`skillDistribution` 等真实 implementation defects |
| recovery direction | runtime 插入 `TaskSpaceImplementNeedsEditRecoveryV1`，要求使用已有证据 patch target artifact，不要 rediscover |
| attribution gap | `read_file csv_processor.py` 在 Linux action-contract 下执行为 `sed -n '1,240p' -- csv_processor.py`，但 rollout trace 的 read `main_tool_result` 记录 `artifactRefs=[]` |
| control loop | 已存在的 `validation_rework_duplicate_artifact_read` gate 因 artifact ref 缺失未触发，模型重复读同一文件直到 `provider_node_request_hard_limit_exceeded` |

根因判断：

这不是新的 schema classification 问题，也不是 target-read permission 问题。`read_command_artifact_ref` 支持
PowerShell `Get-Content`、`cat` 和 `type`，但漏掉了 TaskSpace Unix `read_file` 的稳定命令形态
`sed -n '1,240p' -- path`。结果是工具反馈丢失 artifact identity，runtime 不能把“已经读过 target artifact”传递给 duplicate
rework gate。

本轮修复：

| 层 | 结论 |
|---|---|
| artifact attribution | `read_command_artifact_ref` 识别 `sed -n ... -- path` 并把 path 写入 successful read result evidence |
| duplicate rework gate | 第一次命名 validation rework target read 仍允许；第二次同 target read 在无 successful edit 前触发 `validation_rework_duplicate_artifact_read` |
| convergence feedback | recovery 明确要求 `apply_patch` 或返回 blocked，避免把节点请求预算耗尽在重复读取上 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 是否在首次读取 `csv_processor.py` 后进入 `apply_patch`，
并继续修正 schema/public validator 失败。

### 3.18 validation blocker manual rework origin loss

sed read attribution 修复后的 post-commit keyed rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703q-postcommit-attestation/runs/terminal_bench__organization-json-generator/20260704-030017-880
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| sed attribution | 第一次 `read_file process.py` 已记录 `artifactRef: process.py`，证明 3.17 修复有效 |
| duplicate gate | 后续两次 `read_file process.py` 被 `validation_rework_duplicate_artifact_read` 拦截 |
| manual rework path | 模型在 validation recovery 中先 `create_node(implement_solution)`，再把 validation node block 掉 |
| origin gap | action map 最终边为 `node-4 -> node-6`，而不是 `node-5 -> node-6`；`node-6` 没有 `origin_node_id=node-5` |
| lifecycle gate | `node-6` 绑定后发出的 `apply_patch` 被 `result-14 still unreviewed` 拦截，要求先 `state_commit` |
| control loop | provider 最终 `request_count=20/20` hard stop；这属于状态归因问题被转化成继续采样预算消耗 |

根因判断：

自动 validation rework 路径已经会设置 `origin_node_id`，但手动 `create_node` 路径没有。`create_node` 默认依赖选择只看
completed leaf nodes，因此在 validation node 仍 running 时创建的 detached rework node 会挂回最近 completed implementation。
随后 validation blocker 虽然语义存在，lifecycle gate 也正确要求 unreviewed result 不能被普通工作依赖，但因为 rework node
origin 丢失，runtime 无法识别这是 active validation rework input。

本轮修复：

| 层 | 结论 |
|---|---|
| create_node origin | detached `implement_solution` 若从 active `smoke_test` / `regression_test` 创建，会记录该 validation node 为 `origin_node_id` |
| DAG dependency | 同时加入 validation -> rework 依赖边；validation running 时 rework 保持 Pending |
| blocked refresh | validation node 变成 Blocked 后，只刷新匹配 origin 的 pending rework node 为 Ready，不放宽普通 blocked node 的下游解锁语义 |
| lifecycle gate | 绑定该 rework node 后，`apply_patch` 可使用 unreviewed blocker 作为 active rework input，不需要额外 state_commit 自救 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core manual_validation_rework_created_before_block_keeps_origin --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 是否能完整 patch `process.py`/生成 `organization.json`，
并继续通过 schema/public validator 或暴露下一层 R4 tools 问题。

### 3.19 validation stale failure block without current test

manual validation rework origin 修复后，最新 keyed rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703r-manual-rework-origin/runs/terminal_bench__organization-json-generator/20260704-032001-321
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 12
right_open_leaf_nodes: 0
public_validation_exit_code: 1
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| sed attribution | `sed -n '1,240p' -- generate_org.py` read 已记录 target artifact |
| duplicate gate | 第二次 `read_file generate_org.py` 被 `validation_rework_duplicate_artifact_read` 拦截 |
| implementation quality | 模型只移除了 line 1 的前导空格，line 2 起仍保留顶层缩进 |
| validation evidence | 新 smoke node 没有当前 `Build`/`Test` result，却用上一轮 `IndentationError` 文案执行 `block_node` |
| graph state | `open_leaf_nodes=0`，但 public validator exit 1，说明 graph closed 不等于 validation 真实完成 |

根因判断：

这是反馈层“语义缺失 + 跨节点复用”问题。旧 `IndentationError` 语义存在，并没有完全丢失；缺的是当前 validation node
必须先产生自己的 test/build evidence，才能把失败声明为该 node 的 validation result。没有这个底线，runtime 会允许模型把
上一轮可编辑失败复用成新 validation node 的 terminal blocker。

本轮修复：

| 层 | 结论 |
|---|---|
| validation block guard | smoke/regression node 若要用 failed validation/test blocker，必须已有同节点 `Build`/`Test` tool result |
| infra exception | `Local validator infrastructure failed`、`Cannot execute test commands` 等本地验证基础设施 blocker 不被当成 stale failure reuse |
| feedback | 拒绝 stale validation block 时，明确要求先在当前 node 运行 required validation command |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node_rejects_stale_failure_without_current_test --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 在 rework patch 后会重新运行 schema/public validation，
并继续 patch 未修完的顶层缩进，而不是复用上一轮失败关闭新的 validation node。

### 3.20 validation rework duplicate-read projection loop

stale validation block guard 修复后，最新 keyed rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703s-stale-validation-guard/runs/terminal_bench__organization-json-generator/20260704-033716-688
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 16
right_open_leaf_nodes: 1
public_validation_exit_code: 1
```

本次进展和新问题：

| 层 | 结论 |
|---|---|
| validation | 已执行 `python process.py && python -m jsonschema -i organization.json schema.json`，到达真实 schema failure |
| failure semantic | schema 明确指出 `member_ids` 应为 `members`，statistics 需要 camelCase 字段 |
| target read | `node-4` 成功读取 `process.py`，`result-11` 带 artifact ref |
| duplicate gate | 后续 5 次 `read_file process.py` 全部被 `validation_rework_duplicate_artifact_read` 拦截 |
| projection gap | projection 仍广告 `read_file validation rework target artifact process.py only if current contents are not visible`，`critical_artifact_evidence` 为 none |
| budget | `provider_node_request_hard_limit_exceeded node_request_count=6/5`，rework node 保持 open |

根因判断：

这不是 sed attribution 失败，也不是 duplicate gate 未触发。问题在 provider-visible projection：target read result 已存在，
但 projection 没把它作为当前 critical evidence 展示，同时继续显示 read/search 可用。模型看到“内容可能不可见”的 read action，
反复请求同一 read；底层 gate 每次都正确拒绝，但拒绝反馈被下一轮 projection 冲淡。

本轮修复：

| 层 | 结论 |
|---|---|
| critical evidence | validation rework target read result 进入 `critical_artifact_evidence`，标记 `signal=validation_rework_target_read` 并附带 bounded excerpt |
| next action | target 已读且无 edit 时，不再广告该 target 的 read_file；改为 `use existing validation rework target read result` + `apply_patch` |
| current contract | target 已读后 allowed action 文案收窄到 edit/control，并明确 read/search visible rework target 会被 blocked |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
git diff --check
```

当前状态：focused 修复已完成；仍需再次 keyed rerun 验证 TaskSpace 是否在第一次读取 `process.py` 后直接 patch
`members` 和 statistics camelCase 字段，再重跑 schema/public validation。

## 4. 本次验证

| 验证项 | 命令 | 结果 |
|---|---|---|
| JSON 结构 | `jq empty docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json` | PASS |
| Tool path coverage | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1` | PASS：11 paths |
| Sample ledger | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-sample-ledger.ps1` | PASS：12 samples |
| Public-10 plan + snapshot | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json` | PASS：10 planned samples |
| Usage accounting | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1` | PASS：rejects ambiguous token usage；synthetic timeout row 可从 rollout token_count 恢复 partial usage |
| R4 acceptance readiness | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1` | EXPECTED BLOCKED：engineering gates pass；`provider_credential_missing` 阻断真实 utility 复验 |
| R4 acceptance readiness with `.env.local` key | `set -a; . ./.env.local; set +a; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1` | PASS：engineering gates pass；provider credential present；可进入真实 utility rerun |
| H-035 focused Rust unit | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_apply_patch_strips_unified_file_headers_inside_native_update --lib` | PASS：1 passed |
| H-036 focused Rust unit | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_failure_summary_preserves_error_after_warning_noise --lib` | PASS：1 passed |
| External wrapper harness | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-external-wrapper-harness.ps1` | PASS |
| Provider credential focused harness | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-external-wrapper-harness.ps1` | PASS：临时清空 `DEEPSEEK_API_KEY` 后稳定退出 `provider_credential_preflight` |
| Terminal-Bench uv cache | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-terminal-bench-uv-cache-harness.ps1` | PASS |
| Oracle runner | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-oracle-runner-harness.ps1` | PASS |
| Terminal-Bench adapter | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1` | PASS |
| Metrics extractor | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-metrics-extractor-harness.ps1` | PASS |
| Benchmark harness | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-harness.ps1` | PASS |
| organization-json-generator plan-only | `run-taskspace-external-benchmark.ps1 ... -PlanOnly` | PASS：prompt guard clean |
| organization-json-generator provider preflight | `run-taskspace-external-benchmark.ps1 ... deepseek-v4-flash` without `DEEPSEEK_API_KEY` | PASS：invalid_harness `provider_credential_missing` |
| organization-json-generator keyed rerun #1 | `run-taskspace-external-benchmark.ps1 ... deepseek-v4-flash` with `.env.local` key | FAIL：真实模型执行后在 metrics extractor post-processing crash；根因是 changed file 消失竞态 |
| Metrics extractor disappeared-file race | `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-metrics-extractor-harness.ps1` | PASS：vanished changed path 被记录为 `hash_status=missing` |
| organization-json-generator keyed rerun #2 | `run-taskspace-external-benchmark.ps1 ... deepseek-v4-flash` with `.env.local` key | DIAGNOSTIC FAIL：pair report/metrics 正常写出；standard wrong，TaskSpace 900s timeout；仅 E1，不进入 utility aggregate |
| bwrap bootstrap focused Rust suite | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core bootstrap_failure --lib` | PASS：4 tests |
| local infra regression Rust suite | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core local_infra --lib` | PASS：11 tests |
| duplicate fact-source coverage gate | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish --lib` | PASS：缺 `employees.csv` / `projects.csv` 时 duplicate feedback 不给 `finish_node` |
| inspect finish fact-source guard | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources_block_manual_and_forced_finish_until_read --lib` | PASS：manual/forced inspect finish 都等到声明 fact sources 覆盖后才允许进入 implement |
| duplicate read/search regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_node_blocks_repeated_successful_read_command --lib` | PASS |
| duplicate read/search forced transition regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core forced_inspect_transition_accepts_duplicate_read_search_gate_recovery --lib` | PASS |
| data artifact inspect evidence | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_data_artifact_read_counts_as_working_evidence --lib` | PASS |
| session duplicate recovery | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core duplicate_read_search_recovery_pushes_inspect_transition --lib` | PASS |
| inspect convergence regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_successful_diagnostic_and_working_evidence_marks_convergence_ready --lib` | PASS |
| provider budget focused suite after fact-source adaptive limit | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_budget --lib` | PASS：22 tests |
| active budget hard gate suite | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget --lib` | PASS：11 tests |
| Whale build after hard gate | `CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked` | PASS |
| organization-json-generator hard-stop rerun | `run-taskspace-external-benchmark.ps1 ... -RunRoot target/r4-org-json-real-keyed-20260703e-hardgate` | DIAGNOSTIC: standard solved；TaskSpace wrong；timeout eliminated；new premature inspect hard stop exposed |
| organization-json-generator adaptive-budget rerun | `run-taskspace-external-benchmark.ps1 ... -RunRoot target/r4-org-json-real-keyed-20260703f-adaptive-budget` | DIAGNOSTIC: inspect 已读全 fact sources；TaskSpace 进入 implement rework，暴露 `IndentationError` 逐行修复和 `KeyError: 'salary'` |
| implementation rework evidence join | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_summary_merges_transitive_inspect_evidence_and_failure --lib` | PASS：rework summary 同时包含 validation failure 和上游 CSV 字段证据 |
| implementation recovery action contract | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core implement_recovery_prioritizes_validation_failure_and_inspected_fields --lib` | PASS：recovery 明确 failure 优先、整文件缩进修复、字段不得发明 |
| inspect projection fact-source guard | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_blocks_inspect_finish_until_declared_fact_sources_read --lib` | PASS：缺 `projects.csv` 时 projection 不再提示 `finish_node` / implement transition |
| inspect projection normal transition regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_prioritizes_inspect_to_implement_after_evidence --lib` | PASS：无缺失声明 fact source 的正常 inspect evidence 仍可提示进入 implement |
| editable validation failure blocker guard | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_editable_validation_failure_blocker_before_edit --lib` | PASS：rework node 不能把 `IndentationError` 这类可编辑 validation failure block 成 infra/closed validation |
| editable validation failure feedback | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_editable_validation_failure_blocker_rejection --lib` | PASS：recent feedback 输出 `editable_validation_failure_blocker_rejected`，并要求 whole-file/block patch |
| editable validation failure real wording regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_editable_validation_failure_blocker_before_edit --lib` | PASS：使用真实 rerun 中的 `cannot read files ... read actions are not allowed ... current narrowed state` blocker 文案 |
| output contract validation gate | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks_generator_only_command_for_schema_output_contract --locked` | PASS：`python generate_json.py` generator-only validation 被拒绝，要求 schema/output contract check |
| output contract forced closeout guard | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation_rejects_generator_only_output_contract_success --locked` | PASS：generator-only successful result 被标记 invalid，引用它的 success criterion 被重开 |
| output contract feedback | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt_structures_output_contract_coverage_failure --locked` | PASS：recent feedback 输出 `validation_test_missing_output_contract_coverage` 和 combined next command |
| schema fact-source output contract guard | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked` | PASS：`schema.json` 只作为 fact source 时，弱 `json.load` validation 被拒绝并要求 `jsonschema` |
| validation recovery projection preservation | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked` | PASS：gate recovery 后 active projection 保留 exact `python process.py && python -m jsonschema -i organization.json schema.json`，并不再泛化为 `run validator/test command` |
| validation rework target artifact runtime | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked` | PASS：schema failure 无 traceback 时从 blocked validation dependency changed artifacts 推导 `generate_org.py`，并在 projection/recovery 中暴露命名 target |
| validation rework target artifact action contract | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_allows_named_validation_rework_artifact_read --locked` | PASS：`implementation_needs_edit` 下允许读取 `generate_org.py`，仍拒绝 `schema.json` 泛读 |
| validation rework regression suite | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked` | PASS：12 tests |
| implementation needs edit regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core implementation_needs_edit --locked` | PASS |
| validation jsonschema dependency recovery | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_missing_jsonschema_dependency_stays_on_validation_with_cli_recovery --locked` | PASS：missing `jsonschema` 不进入 rework；projection 输出 CLI schema validator |
| validation rework sed read attribution | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked` | PASS：Unix `sed` read 记录 artifact ref；第二次同 target read 被 duplicate rework gate 拒绝 |
| validation blocker manual rework origin | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core manual_validation_rework_created_before_block_keeps_origin --locked` | PASS：manual rework 继承 active validation origin，origin blocked 后 Ready，patch 不再被 unreviewed blocker gate 拦截 |
| validation stale failure block guard | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node_rejects_stale_failure_without_current_test --locked` | PASS：fresh validation node 无当前 test/build result 时不能用旧 `IndentationError` blocker 关闭 |
| validation block guard regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node --locked` | PASS：3 tests；有同节点 failed validator result 的 block 仍允许 |
| validation rework target-read projection | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked` | PASS：target 读完后 projection 使用已有 result、展示 critical evidence，并要求 apply_patch 而非继续 read |
| validation node output-contract regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked` | PASS |
| forced validation closeout regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked` | PASS |
| local infra regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core local_infra --locked` | PASS：11 tests |
| validation aggregate regression | `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked` | PASS：83 tests |
| latest R4-D fix build | `cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check`; `CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked`; `git diff --check` | PASS：format/build/whitespace 通过；仅有已知 stable rustfmt config warning |
| organization-json-generator direct validator | direct `external-validator.ps1` run on generated fixture | PASS：Docker build `classification=ok`; run reaches expected missing `organization.json` assertions |
| Whitespace | `git diff --check` | PASS |

## 5. 当前未完成项

| 优先级 | 未完成项 | 当前证据 | 下一步 |
|---:|---|---|---|
| P0 | TaskSpace utility parity | public-10 closeout 中 TaskSpace 仅 3/10 solved；post-closeout 只证明 `heterogeneous-dates` 已改善；`sqlite-db-truncate` 已收敛到非 timeout wrong；keyed `organization-json-generator` 仍是 E1 diagnostic fail | 继续 R4 utility-convergence，不进入验收通过 |
| P0 | Long-flow convergence | keyed `organization-json-generator` 的 900s timeout 已被 hard stop 消除；最新真实进展已越过 fact-source projection、editable blocker、generator-only closeout、schema fact-source weak validation、recovery projection dilution、validation rework target artifact read gap、jsonschema module recovery、sed read artifact attribution、manual validation rework origin、stale validation block guard，并 focused fixed 到 validation rework duplicate-read projection loop | 重跑该样本；若仍 wrong，再按新 trace 建立下一层 tool/control case |
| P0 | Provider budget hard stop real-run validation | hard gate 已真实生效；adaptive fact-source node limit 的真实 rerun 已越过固定 5-request stopper；当前失败从 stale validation block 推进到“target read 已可见但 projection 继续广告 read/search，导致重复 read 到 node hard stop” | 下一次 keyed rerun 同时验证 provider hard stop、adaptive inspect、rework evidence join、projection fact-source guard、editable validation blocker guard、output contract coverage guard、schema fact-source guard、recovery projection guard、validation rework target artifact read guard、jsonschema module recovery、sed read artifact attribution、manual rework origin、stale validation block guard、target-read projection guard 是否共同推进 utility |
| P0 | Provider timeout usage flush | 报告层已能从 rollout token_count 恢复 timeout 前 partial usage，并标为 `recovered_from_rollout_trace`；如果进程被杀前没有任何 token_count/response.completed，exact usage 仍不可得 | 后续真实复验时检查 timeout 行是否有 rollout token_count；如无，再做 provider 退出/回收路径 |
| P1 | 成本/token 放大 | `heterogeneous-dates` post-closeout 已改善，但 public-10 closeout 仍记录 6x-28x request amplification | 新二进制重跑 public-10 subset，更新 durable report snapshot |
| P1 | Release evidence bundle | raw paired run artifacts 仍在外部 run cache，不在仓库内 | 设计 release artifact policy：保留 summary snapshot、压缩关键 evidence，还是外链 run cache |

## 6. 下一步接手顺序

1. 先运行 `scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1`，用 JSON 判断当前是 `blocked`、`fail`，还是 `ready_for_real_utility_rerun`。
2. 建立 R4 utility-convergence 继续工作入口，优先选择一个 public-10 负样本做 bug-killer 闭环。
3. `sqlite-db-truncate` 当前适合作为已收敛工具链样本归档：状态是非 timeout、closed graph、`agent_patch_wrong`。
4. `organization-json-generator` 当前下一步不再是 provider 前置；keyed run 已证明 provider preflight 通过，`bwrap` feedback-layer case 已收录并修复。
5. 先重跑 `organization-json-generator` 验证 `tool-runtime-bootstrap-failure`、fact-source coverage、provider hard stop、adaptive inspect node limit、implementation rework evidence join、inspect projection fact-source guard、editable validation blocker guard、output contract coverage guard、schema fact-source guard、recovery projection guard、validation rework target artifact read guard、jsonschema module recovery、sed read artifact attribution、manual validation rework origin、stale validation block guard 和 target-read projection guard 是否让 TaskSpace 生成 schema/public-test 正确的 `organization.json`；如仍 wrong，按新 trace 建立下一层 case。
6. 每完成一个样本，更新 public-10 snapshot 或生成新的 durable report artifact，避免再次依赖未提交 `target/` 缓存。
