# Problem P-001: 0.147 rebase 丢失 TaskSpace Map 调试导出命令
- Status: open
- Created: 2026-08-21 05:47
- Updated: 2026-08-21 06:08
- Objective: 恢复 benchmark 可观测性依赖的内部 `whale debug taskspace-map` Store 导出能力。
- Symptoms:
  - 真实 TaskSpace Agent 完成修复且公开/隐藏测试均通过，但 runner 因 Map Store export 失败将业务结果判为 false。
- Expected behavior:
  - `whale debug taskspace-map --thread-id ... --output ...` 从当前 SQLite Store 导出 canonical Map 与 thread binding。
- Actual behavior:
  - 0.147 CLI 返回 `unrecognized subcommand 'taskspace-map'`。
- Impact:
  - 正确的 TaskSpace 运行被标记为 engineering unclean，阻断真实缓存验收与基线晋升。
- Reproduction:
  - 记录 `WAR-20260821-054054-CACHE-REGRESSION-BD2A9444`；Agent 修改正确、两组验证 exit 0，但 `observability_availability=map_store_failed`。
- Environment:
  - Linux；提交 `8cc1fef67b472ff2a28327750dc2884675f91dec`；workspace Whale SHA256 `aba16224dccac66412da5b9e34f5496c7ef62b979f67ed6af6ec029db8871549`。
- Known facts:
  - 失败命令是内部开发/验收可观测性接口，不是 Agent 产品逻辑。
  - 历史提交 `f8e3f67fb` 已有实现和 CLI 测试，rebase 提交 `f881e71b2` 删除了命令与测试，但 benchmark consumer 保留。
- Ruled out:
  - Agent 任务失败：Agent complete、目标文件修改正确、public/hidden validation 均为 0。
  - Store 没有 Map：rollout 包含 52 个 map runtime events，6 个成功 TaskSpace Exec 与连续 revision。
- Fix criteria:
  - 恢复隐藏 debug 子命令，适配 0.147 `SqliteConfig`，不改变自然语言或 Agent 产品路径。
  - CLI 测试覆盖成功导出、无 binding 和非法 thread id。
  - 使用失败 run 的真实 SQLite home 离线导出成功。
  - 新授权真实 map-request 复验业务、usage、trace 与 Map Store observability 全部通过。
- Current conclusion: H-001 confirmed；历史 exporter 已按 0.147 API 最小恢复，离线验证通过，等待新授权真实 map-request 关闭最后一项标准。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-001 confirmed by E-001/E-002；repair 已通过 E-003/E-004 的 CLI 与失败运行 SQLite 离线验证。
- Close reason:
  - not closed

## Hypothesis H-001: rebase 删除 producer 但保留 benchmark consumer
- Status: confirmed
- Parent: P-001
- Claim: 0.147 迁移遗漏了 Whale 私有 debug exporter，脚本仍调用原合同。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - CLI 明确报告未知子命令，且历史实现可定位。
- Falsifiable predictions:
  - If true: 当前 DebugSubcommand 无 TaskspaceMap，历史提交有实现/测试，consumer 仍调用。
  - If false: 当前命令存在但因参数或配置加载失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: producer、consumer 与迁移差异。
  - Signal: CLI help/error、当前 enum、历史 diff、PowerShell invocation。
  - Capture method: 真实日志与 Git/源码只读比较。
  - Event name or marker:
    - `map_store_export_failed`
  - Correlation keys:
    - `WAR-20260821-054054-CACHE-REGRESSION-BD2A9444-CACHE-001`
  - Differentiates from:
    - H-002
  - Supports if:
    - consumer 存在而 CLI variant/handler 被删除。
  - Refutes if:
    - handler 存在且被实际调用。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
- Conclusion: producer/consumer 迁移不完整。
- Repair design readiness: ready
- Next step: 使用新授权运行一次 map-request，确认 runner 端到端得到 measured Map Store observability。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Store 存在但缺少 thread binding
- Status: refuted
- Parent: P-001
- Claim: 命令实际存在，导出因当前 thread 没有持久化 binding 而失败。
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Map Store 导出也可能在有效运行后找不到绑定。
- Falsifiable predictions:
  - If true: CLI 进入 handler 并报告 no binding。
  - If false: clap 在 handler 前报告 unknown subcommand。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败所处 CLI 层级。
  - Signal: stderr 精确错误与 exit code。
  - Capture method: 真实 observability artifact。
  - Event name or marker:
    - `unrecognized subcommand`
  - Correlation keys:
    - thread `01a0211e-df51-75f1-b976-667ec8b441e1`
  - Differentiates from:
    - H-001
  - Supports if:
    - handler 返回 no binding。
  - Refutes if:
    - clap 拒绝未知命令。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: 命令未注册，尚未访问 Store，故 refuted。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - E-001 refuted

## Evidence E-001: 正确 Agent 结果仅因未知 debug 子命令被判失败
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `WAR-20260821-054054-CACHE-REGRESSION-BD2A9444` pair report、metrics、observability stderr
- Prediction or plan link:
  - P-001 症状与失败层级。
- Matched signal:
  - Agent complete，public/hidden exit 0，正确 patch 将 round 精度从 1 改为 2；Map export exit 2，错误为 unknown `taskspace-map`。
- Correlation keys:
  - `01a0211e-df51-75f1-b976-667ec8b441e1`
- Raw content:
  ```text
  external_validation_status=passed
  error: unrecognized subcommand 'taskspace-map'
  observability_map_store_error_code=map_store_export_failed
  ```
- Interpretation: 业务失败是开发验收 exporter 缺失造成的 false negative。
- Time: 2026-08-21 05:47

## Evidence E-002: 历史 exporter 与测试被迁移提交删除
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `f8e3f67fb`、`f881e71b2^` 与当前 `cli/src/main.rs`
- Prediction or plan link:
  - H-001 的 producer/consumer 不对称。
- Matched signal:
  - 历史 DebugSubcommand、handler 和 `debug_taskspace_map.rs` 存在；当前均缺失；`export-action-map-observability.ps1` 仍调用该命令。
- Correlation keys:
  - `f8e3f67fb04db96d4db8eb4506ced3a258fc2e8a`
  - `f881e71b2f79bdcafbe800b1bfd9500edc940e58`
- Raw content:
  ```text
  historical: TaskspaceMap(DebugTaskspaceMapCommand)
  current: no TaskspaceMap variant
  consumer: whale debug taskspace-map --thread-id ... --output ...
  ```
- Interpretation: 根因是 rebase 遗漏既有私有开发命令，不需要改动 TaskSpace 产品行为。
- Time: 2026-08-21 05:47

## Evidence E-003: 0.147 CLI exporter 定向测试通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `codex-cli::debug_taskspace_map` nextest run `16b817de-288a-4960-9c06-7d8cd27bcc2f`
- Prediction or plan link:
  - P-001 CLI 合同覆盖成功导出、无 binding 与非法 thread id。
- Matched signal:
  - 三项测试全部通过；`cargo fmt --all -- --check` 通过。
- Correlation keys:
  - nextest run `16b817de-288a-4960-9c06-7d8cd27bcc2f`
- Raw content:
  ```text
  3 tests run: 3 passed, 0 skipped
  ```
- Interpretation: 恢复后的隐藏命令已正确适配 0.147 ConfigBuilder 与 SqliteConfig。
- Time: 2026-08-21 06:07

## Evidence E-004: 失败运行的真实 SQLite Store 可离线导出并被观测脚本消费
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `WAR-20260821-054054-CACHE-REGRESSION-BD2A9444` 右臂 SQLite home 与当前 debug binary
- Prediction or plan link:
  - P-001 离线真实 Store 标准。
- Matched signal:
  - 导出 `TaskSpaceMapExportR8V1`，map store revision 12、canonical revision 18、owner binding；完整 PowerShell consumer 返回 `MapStoreAvailability: measured`、5 个 nodes。
- Correlation keys:
  - thread `01a0211e-df51-75f1-b976-667ec8b441e1`
  - map `map-01a0211e-df51-75f1-b976-667ec8b441e1`
- Raw content:
  ```text
  schema_version=TaskSpaceMapExportR8V1
  store_revision=12; canonical_revision=18; binding_relation=owner
  MapStoreAvailability: measured; nodes=5
  ```
- Interpretation: 原失败运行无需重新执行 Agent 即可证明导出修复正确；仍保留一次新真实运行作为 runner 端到端关闭证据。
- Time: 2026-08-21 06:08
