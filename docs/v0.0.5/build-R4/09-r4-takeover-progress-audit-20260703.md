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
| `provider-budget-advisory-runaway` | 仍未关闭；需要真实 rerun 判断是否升级为 hard gate / terminal blocked-with-evidence | open |

新增关键判断：

- 这是反馈层“语义缺失”多于“语义扭曲”：原始失败/证据信号进入了 trace 或 tool output，但缺少 failure kind、required command、missing artifact 或 phase completion guard。
- `taskspace runtime` 可以负责工具反馈分类和 phase gate，但不能超越状态机底线把“重复证据”解释成“inspect 已完成”。
- R4 当前实际进行位置：phase 流程已到 R4-H/post-closeout；工程上继续在 R4-D feedback layer 和 R4-G utility-convergence 做回补，不应重新标记 R4 已验收。

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
| organization-json-generator direct validator | direct `external-validator.ps1` run on generated fixture | PASS：Docker build `classification=ok`; run reaches expected missing `organization.json` assertions |
| Whitespace | `git diff --check` | PASS |

## 5. 当前未完成项

| 优先级 | 未完成项 | 当前证据 | 下一步 |
|---:|---|---|---|
| P0 | TaskSpace utility parity | public-10 closeout 中 TaskSpace 仅 3/10 solved；post-closeout 只证明 `heterogeneous-dates` 已改善；`sqlite-db-truncate` 已收敛到非 timeout wrong；keyed `organization-json-generator` 仍是 E1 diagnostic fail | 继续 R4 utility-convergence，不进入验收通过 |
| P0 | Long-flow convergence | keyed `organization-json-generator` 中 TaskSpace 92 tool calls、900s timeout、request_count 90/20、recovery attempt 32；bootstrap、validation-command、duplicate-read、fact-source coverage 等 focused feedback issues 已修，但尚未真实 rerun 验证 | 重跑该样本；若仍 timeout，再建立 request budget hard gate / repeated no-action recovery hard stop |
| P0 | Provider budget hard stop | 当前仍是 advisory/warned/over-profile-hint 语义，不会天然终止 provider turns | 基于下一次 real rerun 决定是否把 request budget hint 升级为 hard gate，或对 repeated no-action recovery 做 terminal blocked-with-evidence |
| P0 | Provider timeout usage flush | 报告层已能从 rollout token_count 恢复 timeout 前 partial usage，并标为 `recovered_from_rollout_trace`；如果进程被杀前没有任何 token_count/response.completed，exact usage 仍不可得 | 后续真实复验时检查 timeout 行是否有 rollout token_count；如无，再做 provider 退出/回收路径 |
| P1 | 成本/token 放大 | `heterogeneous-dates` post-closeout 已改善，但 public-10 closeout 仍记录 6x-28x request amplification | 新二进制重跑 public-10 subset，更新 durable report snapshot |
| P1 | Release evidence bundle | raw paired run artifacts 仍在外部 run cache，不在仓库内 | 设计 release artifact policy：保留 summary snapshot、压缩关键 evidence，还是外链 run cache |

## 6. 下一步接手顺序

1. 先运行 `scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1`，用 JSON 判断当前是 `blocked`、`fail`，还是 `ready_for_real_utility_rerun`。
2. 建立 R4 utility-convergence 继续工作入口，优先选择一个 public-10 负样本做 bug-killer 闭环。
3. `sqlite-db-truncate` 当前适合作为已收敛工具链样本归档：状态是非 timeout、closed graph、`agent_patch_wrong`。
4. `organization-json-generator` 当前下一步不再是 provider 前置；keyed run 已证明 provider preflight 通过，`bwrap` feedback-layer case 已收录并修复。
5. 先重跑 `organization-json-generator` 验证 `tool-runtime-bootstrap-failure` 修复是否消除持续 recovery；如仍 timeout，再把 request budget hint 升级为 hard gate，或在 repeated no-action recovery 超阈值时强制 blocked-with-evidence。
6. 每完成一个样本，更新 public-10 snapshot 或生成新的 durable report artifact，避免再次依赖未提交 `target/` 缓存。
