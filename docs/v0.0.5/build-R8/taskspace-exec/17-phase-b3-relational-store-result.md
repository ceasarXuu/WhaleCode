# Phase B3 MS-01～MS-02 关系化 Map Store 结果

- 日期：2026-08-07
- 状态：离线验证完成
- 真实 Whale Agent run：0

## 1. 已完成

SQLite 中的 canonical Map 已由以下关系化事实直接组成：

- Map head：身份、owner、schema、Map revision、Store revision 和语义哈希；
- Node：Root、Work、Finish 的目标、状态、内容和稳定顺序；
- Parent relation：Agent 声明的父节点关系；
- Node Action：归属节点、真实 action identity、Tool 名和机械 outcome。

物理层不再保存 `canonical_json`，也没有 Event Store、delta replay、并行镜像或旧 shape reader。状态库版本切换到
新库，实验数据不迁移。读取在同一 SQLite transaction 中组装完整 canonical Map，再校验语义哈希和 Map identity。

写入仍以 Map head 的 `store_revision` 做全图 CAS，但只同步候选与当前 Map 之间发生变化的实体行。单个 Action outcome
变化只替换对应 Action 行，不更新 Node 或 parent；Tool 执行不因此串行化。

## 2. 验证

| 检查 | 结果 |
|---|---|
| 关系表、外键、唯一 Root/Finish 和 Action identity 约束 | PASS |
| `canonical_json` 与 Map event table 不存在 | PASS |
| create/load/CAS/conflict/idempotency/rollback | PASS |
| fork/child binding 与同一 Map 原子提交 | PASS |
| 重启后从关系事实恢复同一 canonical Map | PASS |
| 损坏关系事实导致语义哈希拒绝 | PASS |
| 单 Action outcome 变化不重写 Node/parent | PASS |
| `cargo test -p codex-state --lib` | 127 passed |
| `cargo test -p codex-core taskspace_store --lib` | 8 passed |

## 3. 后续边界

本单元只提供低成本、可并发保护的固化事实层。MS-03 才会把 Exec 预检候选、client Action `Pending` 以及每个
Tool 的完成结果按最低延迟接入该 Store；Store 本身不根据 Tool 结果推导 Node 状态，也不替 Agent 做 Map 决策。
