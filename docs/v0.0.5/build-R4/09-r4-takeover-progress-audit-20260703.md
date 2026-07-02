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
| E3 readiness | 阻塞 | 0% | R4 记录 E3 no-go；负收益和 timeout/wrong 样本仍未收敛 |

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

## 4. 本次验证

| 验证项 | 命令 | 结果 |
|---|---|---|
| JSON 结构 | `jq empty docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json` | PASS |
| Tool path coverage | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1` | PASS：10 paths |
| Sample ledger | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-sample-ledger.ps1` | PASS：12 samples |
| Public-10 plan + snapshot | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json` | PASS：10 planned samples |
| Usage accounting | `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1` | PASS：rejects ambiguous token usage |
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
| organization-json-generator direct validator | direct `external-validator.ps1` run on generated fixture | PASS：Docker build `classification=ok`; run reaches expected missing `organization.json` assertions |
| Whitespace | `git diff --check` | PASS |

## 5. 当前未完成项

| 优先级 | 未完成项 | 当前证据 | 下一步 |
|---:|---|---|---|
| P0 | TaskSpace utility parity | public-10 closeout 中 TaskSpace 仅 3/10 solved；post-closeout 只证明 `heterogeneous-dates` 已改善；`sqlite-db-truncate` 已收敛到非 timeout wrong；`organization-json-generator` 当前停在 provider credential 前置 | 配置 `DEEPSEEK_API_KEY` 后，继续真实样本复验 |
| P0 | Long-flow convergence | 多个样本有 timeout/request amplification；H-035/H-036 已排除为当前阻塞；H-037/H-038 修复 Linux harness/preflight | 建立 R5 或 R4-extension utility-convergence case，按样本闭环 |
| P0 | Provider timeout usage flush | timeout 行现在不会伪装成 0，但 exact usage 仍可能不可得 | 增加 timeout-safe provider usage flush 或回收路径 |
| P1 | 成本/token 放大 | `heterogeneous-dates` post-closeout 已改善，但 public-10 closeout 仍记录 6x-28x request amplification | 新二进制重跑 public-10 subset，更新 durable report snapshot |
| P1 | Release evidence bundle | raw paired run artifacts 仍在外部 run cache，不在仓库内 | 设计 release artifact policy：保留 summary snapshot、压缩关键 evidence，还是外链 run cache |

## 6. 下一步接手顺序

1. 先把本次 evidence durability 修复提交并推送，保持 R4-H 证据门禁可复核。
2. 建立 R4 utility-convergence 继续工作入口，优先选择一个 public-10 负样本做 bug-killer 闭环。
3. `sqlite-db-truncate` 当前适合作为已收敛工具链样本归档：状态是非 timeout、closed graph、`agent_patch_wrong`。
4. `organization-json-generator` 当前下一步不是 patch recovery，而是 provider 前置：补 `DEEPSEEK_API_KEY` 后重跑真实样本；Docker build 的 `pip install jsonschema` 代理问题已修，缺 key 的 fail-fast 也已有 focused harness 覆盖。
5. 环境前置满足后，重新观察剩余失败是否仍是 failed-patch recovery、request/timeout envelope 或模型解题策略。
6. 每完成一个样本，更新 public-10 snapshot 或生成新的 durable report artifact，避免再次依赖未提交 `target/` 缓存。
