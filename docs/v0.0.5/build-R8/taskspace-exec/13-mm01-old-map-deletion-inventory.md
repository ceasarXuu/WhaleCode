# MM-01 旧 Map 生产调用链与删除清单

- Date: 2026-08-07
- Status: Verified
- Scope: `codex-protocol`、`core/action_map`、session Store、`codex-state`、CLI/TUI 和活动测试
- Paid Whale Agent run: 未执行

## 1. 判定规则

本清单只按当前生产职责判断，不按代码是否“可能以后有用”判断：

1. 新最简 Map 明确需要且存在生产调用链的职责列为 `keep-and-rebuild`；
2. 与新模型冲突、只有旧字段消费者、只有测试消费者或没有消费者的实现列为 `delete`；
3. `keep-and-rebuild` 只保留职责，不承诺保留旧类型、字段、函数或文件；
4. 不建立 legacy、deprecated、adapter、fallback、双写或候选保留区；
5. 历史文档和 benchmark evidence 可追溯，但不得进入编译、注册、Prompt 或活动 fixture。

## 2. 保留并重建的生产职责

| 职责 | 当前生产证据 | 新边界 | 落地单元 |
|---|---|---|---|
| Map 唯一身份与 Runtime mode | `core/state/session.rs` 持有 `ActionMapRuntimeState`；session 模式切换和投影入口读取它 | 保留 `mode`、active Map identity 和当前 canonical Map；删除旧事件集合和间接事实推导 | MM-04、MM-07 |
| canonical Map 持久化 | `core/session/taskspace_store.rs` 调用 `codex-state` create/load/CAS；session fork/resume 使用线程绑定 | SQLite 只保存最简 canonical JSON、hash、Store CAS 和线程绑定；Map revision 只在 canonical JSON 内 | MM-07 |
| 图硬规则 | Store hydrate 和事务提交调用 rooted DAG validation | 从 Node `parents[]` 建图，校验唯一 Root/Finish、端点、重复、自环、环和双向可达；`children[]` 只作可见派生 | MM-03 |
| 节点状态事务 | Runtime、snapshot 和 Viewer 需要节点当前状态 | 状态直接保存在 Node；Runtime 只校验允许转换，不从 completion/block/action ledger 推导 | MM-04 |
| Action 到 Node 的机械归属 | 后续 Exec 的 client/Hosted 核对需要持久节点归属 | Action 直接保存在所属 Node 的 `actions[]`，只含真实身份、Tool 名和机械 outcome | MM-05 |
| 三种 projection policy | `session/mod.rs` 生产请求构建调用 `decide_projection_emission`，分别支持 always/append/request | 保留策略差异，只重建 payload；策略不得拥有第二份 Map 事实 | MM-08 |
| CLI/TUI/观测消费 | `session/handlers.rs`、CLI debug、TUI Viewer 和 benchmark parser 读取 snapshot/protocol 字段 | 全部直接读取同一最简 Node view，禁止 alias 或旧 shape 拼装器 | MM-09 |
| Store 结构化日志 | Store commit/conflict/hydrate 已有生产 tracing | 保留机械 identity/revision/hash/operation 事实；删除旧 Map 语义字段 | MM-07、OB-01 |

## 3. 必须删除的旧实现

| 删除面 | 代码证据 | 删除原因 | 删除单元 |
|---|---|---|---|
| canonical v3 schema | `protocol/src/taskspace.rs` 的 `edges`、`source_refs`、completion/block/action/result/evidence/terminal ledger | 与已确认最简 Node 模型直接冲突 | MM-02 |
| 顶层 edge 表及 graph mutation | `rooted_dag/model.rs`、`invariants.rs`、`transactions.rs`、`transitions.rs` | Agent 只声明 Node parents；独立 edge 和 connect/disconnect 不是产品合同 | MM-02、MM-03、MM-04 |
| 事件事实与 replay | `rooted_dag/events.rs`、`graph_events`、replay tests | canonical SQLite state 已是事实源；聊天或事件重放重建 Map 属于旧设计 | MM-06 |
| completion/block/terminal 间接状态 | `transactions.rs`、`transitions.rs`、`invariants.rs` | 节点状态应直接可读，不能由多个子账本推导 | MM-04、MM-06 |
| result/evidence/source/summary/reason refs | protocol、snapshot、projection、Viewer fixtures | Map 不建设独立渐进暴露或冷存储；完整 Tool 输出复用 Standard | MM-02、MM-06、MM-08、MM-09 |
| `node_events` 与 trace ledger | `action_map/map.rs`、runtime projection/snapshot、protocol event 字段 | 复制 Standard Tool 历史且无独立生产事实价值 | MM-06、MM-09 |
| detail-fold/archive | `action_map/detail_fold.rs` 及 projection 调用 | 对旧 refs/events 做二次语义分类和折叠，违反全局 projection 原则 | MM-06、MM-08 |
| 旧 ActionMapInstance 聚合字段 | `graph_events`、`task_id`、`node_events` 及 refs 查询 helper | 只服务已删除账本；不得改名搬入新实例 | MM-04、MM-06 |
| Store 重复派生列 | SQLite `map_revision`、`terminal` 及 codec 一致性检查 | 与 canonical JSON 重复形成平行事实；Store CAS 已由 `store_revision` 独立承担 | MM-07 |
| 旧 projection/snapshot 字段 | event/result/evidence/source/terminal-history/sentinel/maintenance/detail-fold 字段 | 复制旧模型并高频进入上下文或调试面 | MM-08、MM-09 |
| 只验证旧 shape 的 fixtures/tests | rooted DAG replay、v3 protocol、Store、CLI/TUI snapshot fixtures | 测试奖励已废弃结构，不是生产消费者 | 对应实现单元同步删除 |

## 4. 文件级处置

| Path | Disposition |
|---|---|
| `protocol/src/taskspace.rs` | 原地替换为最简 Map/Node/Action/View；旧类型全部删除 |
| `core/src/action_map/rooted_dag/events.rs` | 删除文件及 module/export/tests |
| `core/src/action_map/rooted_dag/replay_tests.rs` | 删除文件 |
| `core/src/action_map/detail_fold.rs` | 删除文件及调用者 |
| `core/src/action_map/rooted_dag/{model,invariants,transactions,transitions}.rs` | 按新模型重写；禁止保留旧 ledger/edge API |
| `core/src/action_map/map.rs` | 收敛为当前 Map Runtime 实例；删除 events/refs/task ledger |
| `core/src/action_map/{projection,runtime/projection,runtime/snapshot}.rs` | 从最简 canonical Map 直接构造 Agent-visible view |
| `core/src/session/taskspace_store*.rs` | 保留 session 持久化边界，更新新 Map 合同并删除旧字段测试 |
| `state/src/runtime/taskspace_map*.rs` | 保留 canonical JSON/hash/Store CAS/binding；删除重复 Map revision/terminal 事实 |
| `state/migrations/0031_taskspace_canonical_maps.sql` | 按无历史数据产品约束改为最简 Store schema；不增加旧数据迁移或兼容读取 |
| CLI/TUI/benchmark active consumers | 直接切换到新 view；删除旧 alias 和旧 fixture |

## 5. 依赖与实施顺序

1. MM-02 先替换 protocol schema，使编译错误精确暴露全部旧消费者；
2. MM-03～MM-05 建立新图、状态和 Action 原语；
3. MM-06 删除事件重放、detail-fold 和间接账本文件；
4. MM-07 替换 Store，不做 dual-read/write；
5. MM-08～MM-09 重建 context、snapshot、CLI/TUI 和活动 fixture；
6. MM-10 用结构化门禁阻止旧符号回流。

任何旧实现只有在能指出新产品模型中的当前生产职责和调用者时才允许保留；否则按上表直接删除。

## 6. 验证

- `rg` 逐符号检查 protocol、core、state、CLI/TUI 的生产和测试调用者；
- 确认 `graph_events` 仅在 `ActionMapInstance` 内自循环保存，未进入 canonical Store；
- 确认 `node_events`、detail-fold 和全部 Map refs 只服务旧 projection/snapshot；
- 确认 SQLite canonical JSON、Store CAS、线程绑定和 projection policy 存在当前生产调用链；
- TaskSpace zero-base gate 与 cache regression gate 在盘点前均为 PASS。
