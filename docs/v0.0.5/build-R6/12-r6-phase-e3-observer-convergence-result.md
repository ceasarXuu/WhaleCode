# R6 Phase E3 Observer 纵向切换结果

- Created: 2026-07-16
- Updated: 2026-07-16
- Status: Completed
- Scope: Phase E3 only
- Related Design: `10-r6-terminal-replay-convergence-design.md`
- Prerequisite: `11-r6-phase-e2-canonical-replay-result.md`

## 1. 结论

Phase E3 已通过独立退出门禁，可以进入 E4。PowerShell observer 不再解释 checkpoint/delta，也不再由
full/large 两条事件扫描路径维护最终 Map 状态。最终 task/map/node/edge/lease/result/sentinel collections 只从
本轮 attested `whale debug taskspace-replay` 生成的 canonical proof snapshot 一次构造。

事件扫描仅保留 timeline、event count 和 collab tool call 统计；它不能覆盖 canonical final state。Hook 未修改。

## 2. 结构变化

```text
attested whale + rollout
  -> Rust canonical replay proof
       -> one snapshot-to-collections conversion
            -> full report
            -> large-rollout report

rollout/jsonl scan
  -> timeline + counts + tool-call statistics only
```

具体变化：

1. exporter 新增必填 `WhalePath`，benchmark 显式传入本轮已完成健康校验的 binary。
2. proof source 记录 binary SHA、rollout SHA、checkpoint/delta 计数、active chain 和 final snapshot hash。
3. report 不嵌入原始 proof snapshot，避免 final collections 之外再次暴露整份 Map。
4. full/large 共用同一个 proof consumer；large 只限制 timeline 物化和采样数量。
5. replay failure 仍输出成本/timeline 元数据，但 final collections 全部为空，observer 进程非零退出。
6. benchmark 将 `replay_failed` 记为 evidence/harness failure，不能形成成功结论。

## 3. 实施中发现并修复的问题

| 问题 | 表现 | 修复 |
|---|---|---|
| proof snapshot 二次暴露 | large report 通过 `source.replay.snapshot` 泄露完整大结果 | source 只保存 proof 元数据/hash |
| collection 顺序不稳定 | 同一 proof 的 full/large 内容相同但 node 数组顺序不同 | 对 canonical id/from/to 显式稳定排序 |
| timeline 覆盖 final state | 旧 sentinel 测试允许 clear event 改写旧 checkpoint 状态 | 改为断言 timeline 不得覆盖 replay snapshot |

这些修复没有新增状态解释器，也没有让 observer 猜测缺失状态。

## 4. 真实 OBS-01 对账

冻结输入：

`target/r6-phase-e/finish-boundary/subscription-billing-repair/20260715-232923-210/pair-001/right/artifacts/rollout.jsonl`

large 策略输入只在同一 JSONL 每行前增加空白，以跨过 export policy 体积阈值；事件内容、顺序和语义均未改变。

| 事实 | full | large | 判定 |
|---|---:|---:|---|
| export policy | `full` | `summary_only_large_rollout` | 两条策略均覆盖 |
| final snapshot SHA256 | `a4fab287...f07a` | `a4fab287...f07a` | 相同 |
| final collections | canonical | canonical | 完全相同 |
| map revision | 7 | 7 | 相同 |
| Work 状态 | 3 completed | 3 completed | 相同 |
| Root / Finish | OPEN / READY | OPEN / READY | 相同 |
| complete | false | false | 相同 |
| nodes / edges | 5 / 4 | 5 / 4 | 相同 |
| leases / results | 0 / 3 | 0 / 3 | 相同 |
| large timeline dropped | N/A | 344 | 仅 timeline 成本策略生效 |

该结果收敛 R6-E-OBS-01：observer 不再停留在初始化 checkpoint，而是忠实展示 revision 7。

## 5. 失败语义

破坏第一条 delta 的 `previousSnapshotSha256` 后：

| 字段 | 结果 |
|---|---|
| observer exit code | 非零 |
| availability | `replay_failed` |
| error code | `previous_hash` |
| tasks/maps/nodes/edges | 0/0/0/0 |
| partial snapshot in report | false |
| timeline | 仍保留 424 条可读事件元数据 |

Standard-only rollout 返回 `not_applicable` 且正常退出，不记为 TaskSpace replay failure。

## 6. 回归结果

| 测试 | 结果 |
|---|---|
| PowerShell modified-file parser | PASS |
| `test-r6-action-map-observability.ps1` | PASS |
| `test-action-map-observability-summary-export.ps1` | PASS |
| `test-action-map-observability-lib.ps1` | PASS |
| `test-action-map-sentinel-clearance.ps1` | PASS |
| `test-metrics-extractor-harness.ps1` | PASS |
| real OBS-01 full/large strict collection comparison | PASS |
| corrupted delta no-fallback check | PASS |
| Standard not-applicable check | PASS |

## 7. 阶段判定

| Gate | 判定 |
|---|---|
| observer final state 只来自 canonical Rust proof | PASS |
| full/large final hash、revision 和 collections 一致 | PASS |
| OBS-01 恢复 revision 7 / Finish READY / Root OPEN | PASS |
| replay failure 无旧 checkpoint/partial snapshot fallback | PASS |
| benchmark 使用 attested binary 并传播失败 | PASS |

E4 将只处理 Finish READY 的 provider action surface：从 canonical control state 派生 named
`taskspace_control` hard state，不改变 observer、replay 或 Hook。
