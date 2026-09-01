# U20 Codex CLI 0.151 发布收口报告

## 结论

当前 `whalecode-codex` vendor 已追赶至官方稳定版 `rust-v0.151.0`，固定 peeled commit 为 `78c290807ce710180111df227df3b7a4fe845452`、tree 为 `68d61fd9886a749a78487d8ce950e3cb9309a3d7`。W1–W6 的实现与发布证据已经闭合，当前只等待聚焦对抗性收口复审；0.152 alpha 未进入本主题。

本次保留 DeepSeek Flash 默认、Pro 可见、Responses API、三访问路由与 TaskSpace 单一 relational state authority。Codex 原生 task UI 继续隐藏，没有为上游测试全绿改变 Whale 产品逻辑。

## 合入内容

- 导入 0.150/0.151 的权限、安全、PTY shutdown、sandbox、unified-exec、model-aware ToolRouter、MCP result 和会话基础修复。
- 恢复 Whale identity、`WHALE_HOME`、DeepSeek provider/catalog/cache/compaction 与 TaskSpace/Extension seam。
- 修复 OpenAI API PAT endpoint 与 header 组合、跨 provider content-kind 投影和 0.151 test fixture。
- 隔离测试 runner 现在清理宿主 Codex 环境变量，并在会启动 CLI 子进程的 scope 重建 Whale binary。
- 保留 cutover replay 工件不可变；新增相对固定 0.151 substrate 的 current overlay inventory，最终 index 共 883 路径。

## 验证矩阵

| 范围 | 结果 |
| --- | --- |
| W1 pristine 资格 | core 3808/3815；7 个失败均记录为上游基线/时序签名 |
| Provider / DeepSeek | login 163/163、model-provider-info 30/30、models-manager 55/55、DeepSeek API 2/2；route/history/transition/compaction 定向通过 |
| TaskSpace / Extension | state 210/210、TaskSpace Extension 41/41、core TaskSpace 75/75、app-server schema 299/299（1 ignored）、fork→reload 1/1 |
| W6 定向回归 | PAT 2/2、Guardian integration 2/2、Guardian unit 5/5、websocket 42/42、spec plan 54/54、executor MCP 4/4、cache final-wire 2/2 |
| 当前 core 隔离全量 | 受控 `-j 4`：3969 项中 3948 passed（1 flaky）、7 failed、14 timed out、9 skipped；7 failed 与 1 个 turn-state timeout 为批准延期精确集合，另 13 个是当前宿主 zsh-fork/exec-wrapper timeout |
| 同步元数据 | current overlay 883 路径；generator check、validator 与 56 个脚本测试通过 |

## 非绿边界

剩余 TaskSpace 非绿集中在 Cyber metadata 的 child/fork、remote compaction、recover、pending-input 和 websocket reuse 继承，websocket turn-state reset 超时，以及 executor-skill opt-in 请求计数。它们与此前批准延期的 TaskSpace/W9 生命周期问题同簇，逐项签名、pristine 对照和授权映射见[当前 vendor core 非绿清单](../../releases/v0.0.7/codex-upstream-sync/evidence/current-vendor/core-regression-manifest.md)，根因证据见 [`coe/2026-09-01-06-00-codex-0151-release-closeout-regressions.md`](../../../coe/2026-09-01-06-00-codex-0151-release-closeout-regressions.md)。

当前宿主另有 13 个 DotSlash zsh + exec-wrapper intercept 测试稳定超时；相同生产代码的先前隔离运行没有该集合，单项复跑仍超时。本报告保留原始 JUnit 并将其明确列为宿主验证限制，不把它们归入 TaskSpace 延期，也不宣称通过。

本主题不修改 TaskSpace 状态机来消除这些上游测试差异。Windows 与 zsh-fork 平台专项验证也继续延期。上述边界不视为通过，但不阻塞 Linux + DeepSeek + TaskSpace 当前已验证矩阵。

## 缓存与成本

用户批准本轮不超过 1 CNY 的总包后，最小 `deepseek-v4-flash` Standard + map-request 双臂完成两组成功资格运行。R2 `WAR-20260902-070644-CACHE-REGRESSION-BA4DEF1B` 业务成功，但其门禁报告位于临时目录，因此只保留为审计证据；持久化 R3 `WAR-20260902-071009-CACHE-REGRESSION-BBE4EBE4` 使用仓库内门禁报告再次通过并用于晋升。两组累计 26 个 Provider 请求，输入 334,386 tokens（其中 cached 294,144、uncached 40,242），输出 6,416 tokens，估算费用 0.05895688 CNY。R3 的 Standard 后续请求缓存命中率 96.696%，TaskSpace 为 91.5114%，两臂 trace coverage 均为 100%、usage gap 为 0、business success 为 true。当前 `e39d5bd4…` 敏感面已成为 accepted baseline，[持久门禁报告](../../../benchmarks/cache-regression/gate-reports/20260902-codex-0151-w6-package-r3.json)及发布级 live gate 均通过。

## Provenance 与回滚

- 固定上游：`rust-v0.151.0` / `78c290807ce710180111df227df3b7a4fe845452`。
- current overlay：[`current-overlay-inventory.json`](../../releases/v0.0.7/codex-upstream-sync/current-overlay-inventory.json)。
- 唯一执行计划：[`plan.md`](../../releases/v0.0.7/codex-upstream-sync/plan.md)。
- W3、W4、W5 已形成独立提交；W6 作为发布证据与 fixture 收口提交，可独立 revert，不需要破坏性 reset。
