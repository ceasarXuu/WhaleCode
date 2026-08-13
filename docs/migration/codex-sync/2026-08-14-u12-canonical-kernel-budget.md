# U12：TaskSpace canonical kernel 移植清单与代码预算

- 日期：2026-08-14
- 来源边界：当前仓库 `ca7a0b505^` 中切换前 Whale TaskSpace；未访问其他工作空间
- 目标边界：Codex 0.147 `ext/taskspace` 独立领域 crate
- 状态：`in-progress`（预算已批准；U12a/U12b verified）
- 真实模型请求：0

## 1. 最小目标

U12 只恢复 TaskSpace Rooted DAG 的事实模型、机械不变量、状态派生、原子 transaction、domain event/reducer 和确定性 replay。它不接数据库、session、provider、工具、extension lifecycle、RPC、TUI、projection 或 telemetry。

领域 crate 保持单一 canonical schema `taskspace-canonical-map-v2`，不恢复 R5 的 `TaskStatus/MapStatus`、可选依赖、外置 Finish、语义 NodeKind、自动推进或兼容解析。U11 的旧数据库兼容只解决 migration 编号碰撞，不授权 U12 接触持久化。

## 2. 精确保留清单

| 旧来源 | U12 目标 | 动作 | 生产行预算上限 |
| --- | --- | --- | ---: |
| `protocol/src/taskspace.rs` 的前 114 行 | `ext/taskspace/src/model.rs` | 保留 strict serde canonical types/schema constant；不恢复 protocol 模块和其中测试壳 | 125 |
| `action_map/rooted_dag/model.rs` | `ext/taskspace/src/model.rs` | 保留 Root/Work/Finish、canonicalize、hash、node lookup；与 canonical types 合并去除 `codex_protocol` 依赖 | 185 |
| `action_map/rooted_dag/invariants.rs` | `ext/taskspace/src/invariants.rs` | 保留 DAG、record、reservation、terminal 等机械校验与稳定 violation code | 400 |
| `action_map/rooted_dag/transitions.rs` 的生产部分 | `ext/taskspace/src/transitions.rs` | 保留 node state/frontier 的纯派生函数 | 90 |
| `action_map/rooted_dag/events.rs` | `ext/taskspace/src/events.rs` | 保留 canonical facts、batch、reducer、replay 和 corruption 拒绝 | 480 |
| `action_map/rooted_dag/transactions.rs` | `ext/taskspace/src/transactions.rs` | 保留 initialize/execute/release/finish/reopen 的候选构造与原子提交结果 | 470 |
| `action_map/rooted_dag/mod.rs` | `ext/taskspace/src/lib.rs` | 只公开上述领域 API，不导出 host adapter | 80 |
| workspace 与新 crate manifest | workspace `Cargo.toml`、`ext/taskspace/Cargo.toml` | 复用 workspace 的 `serde`、`serde_json`、`sha2`、`petgraph`；测试使用 `proptest`、`pretty_assertions` | 40 |
| **合计硬上限** | | 超过即停止并重新审批 | **1,870** |

预期实际生产代码约 1,700–1,800 行；硬上限包含 public visibility、crate 文档和 0.147 lint 适配余量。每个手写生产文件仍小于 500 行。

## 3. 测试保留清单

以下测试不计入生产代码预算，预计 1,300–1,500 行：

- canonical JSON strict round-trip、禁用旧状态字段和 canonical hash fixture；
- `fixture_tests.rs` 的 fork/join、无效图和顺序稳定性；
- `phase_d_tests.rs` 的动态图、block/unblock、reservation、Finish 与 stale revision；
- `property_tests.rs` 的固定 seed 256-case DAG property；
- `replay_tests.rs` 的 20-cycle replay、损坏 batch、revision/hash 等价；
- `transitions.rs` 原内联 readiness/frontier 测试。

验收命令限定为新 crate 的 fmt/check/test，以及既有 sync metadata/cache index gate；U12 不运行真实 Whale Agent，不生成 provider 请求。

## 4. 明确不恢复

| 旧区域 | U12 处理 | 后续归属或原因 |
| --- | --- | --- |
| `action_map/map.rs` | 淘汰 | 旧 facade/host 数据结构，不应成为第二领域模型 |
| `checkpoint_refs.rs`、`event_codec.rs`、`event_store.rs` | 延后 | session/context event 接线属于 U13/U14 |
| `runtime.rs`、`runtime/**`、`store_handle.rs` | 延后并重写 adapter | store/CAS 属于 U13；lifecycle 属于 U14；不复制旧 core runtime |
| `projection.rs`、`projection_policy.rs`、`detail_fold.rs` | 延后 | WorldState/provider projection 与 viewer 属于 U14/U16 |
| `response.rs` | 延后且不原样恢复 | tool response/atomic handoff 由 U14 extension tool seam 重新接入 |
| `context/**`、`session/taskspace_*`、`tools/handlers/taskspace_*` | 不进入 U12 | 禁止恢复 core/session/tool-router 侵入；U14 使用 extension contributors |
| `state/**taskspace*` | 不进入 U12 | U13 复用现有 `StateRuntime`，以新 migration 号接 store/CAS/replay |
| app-server/protocol RPC/schema | 不进入 U12 | U15 extension-owned service/API |
| TUI、viewer、final-wire/cache snapshots | 不进入 U12 | U16 用户闭环与缓存合同 |
| 旧兼容 parser、旧 schema alias、provider trace、语义 ledger、自动策略 | 淘汰 | 与单一 Rooted DAG、最小宿主侵入和 Agent-authored 语义边界冲突 |

## 5. 实施拆分与安全停止点

批准 1,870 行生产代码硬上限后，U12 仍按三个可验证提交实施，但共同属于唯一计划中的同一个 U12，不新增平行计划：

1. U12a：crate scaffold + canonical model + invariants；
2. U12b：transitions + events/replay；
3. U12c：transactions + 完整 fixture/property 回归与 U12 收口。

任一子提交发现需要依赖 `codex-core`、`codex-state`、`codex-protocol` 的 TaskSpace 专用类型，或需要修改 session/provider/tool router 才能让领域测试通过，立即停止并报告 seam 缺口；不以兼容层或 feature flag 绕过。

## 6. 授权记录

用户已明确批准：U12 可在上述精确范围内新增最多 1,870 行手写生产代码、测试另计，并按 3 个原子提交逐步验证和推送。该授权不覆盖 U13–U16，也不允许扩大到 store、session、provider、工具路由、RPC 或 TUI。

## 7. 执行进度

| 子单元 | 结果 | 生产代码累计 | 验证 |
| --- | --- | ---: | --- |
| U12a：crate + canonical model + invariants | verified | 约 684 行 | 7 passed；无 `codex-core/state/protocol/tools` 依赖 |
| U12b：transitions + events/replay | verified | 约 1,234 行 | 11 passed；event wire、deterministic replay、revision/empty rejection |
| U12c：transactions + 完整 property/fixture | next | — | 待执行 |
