# Phase B1 最简 Canonical Map 重建结果

- Date: 2026-08-07
- Status: Verified offline
- Product commit: `f8dc23612`
- Gate commit: `67a7e7a1b`
- Paid Whale Agent run: 未执行

## 1. 结论

Phase B1 的 `MM-00～MM-10` 已完成。TaskSpace Map 现在只有一份 SQLite canonical 当前态：

1. Map 只包含 `schema_version/map_id/root/work_nodes/finish/revision`；
2. Node 只包含 `node_id/goal/state/content/parents/actions`；
3. Agent 只声明 `parents`，Runtime 机械反算并在所有 Agent-visible Node 中输出 `children`；
4. Action 只记录真实调用身份、Tool 名和机械 outcome，同一 action 可以归属多个 Node；
5. Tool outcome 不改变 Node state，完整 Tool 输出继续保留在 Standard 历史；
6. Root/Finish 只由显式 finish 闭合，reopen 继续使用同一 Map；
7. SQLite 只保存 canonical JSON/hash、Store CAS 和线程绑定，不保存重复 Map revision/terminal；
8. projection 三模式共享同一 R8 Node view，仅进入 context 的方式不同。

旧 v3 edges、Map refs、completion/block/result/evidence/terminal ledger、event replay、detail-fold、旧 snapshot/UI 类型和
过渡 migration 已从活动实现删除，没有 adapter、fallback、dual read/write 或 dormant 别名。

## 2. 工作单元

| Unit | Result | Evidence |
|---|---|---|
| MM-02 | canonical protocol 切换到 v4 最简结构，旧字段反序列化失败 | `protocol/src/taskspace.rs`；Protocol 195 tests |
| MM-03 | parent-only DAG 校验和 children 派生完成；支持 fork/join | `rooted_dag/{invariants,transitions,tests}.rs`；15 tests |
| MM-04 | Node state/content 直接事务、显式 finish/reopen、stale revision 拒绝 | `rooted_dag/transactions.rs` |
| MM-05 | 最小 Node actions 完成；多节点归属合法，冲突身份拒绝 | `one_action_can_belong_to_multiple_nodes_without_conflict` |
| MM-06 | event/replay/detail-fold 与旧专属 tests 净删除 | `f8dc23612` 删除清单 |
| MM-07 | Store 合同和迁移收敛为单一 canonical 状态 | State 7 tests；Core Store 8 tests；CLI restart/export tests |
| MM-08 | projection/snapshot 直接从 canonical Node view 构造 | Core Action Map、三模式和 provider wire tests |
| MM-09 | CLI、TUI、App Server schema、观测导出切换到 R8 | CLI 5 tests；Viewer 3 tests；schema fixtures 3 tests |
| MM-10 | 扩展零残留门禁并区分 Standard output refs | Python 6 tests；zero-base PASS |

## 3. 工程收益

| Metric | Before | After | Verification |
|---|---:|---:|---|
| 本阶段实现 diff | 旧 Map 多层结构 | `1512` insertions / `5611` deletions | commit `f8dc23612` |
| canonical 关系事实源 | Node + top-level edges 双表达 | 仅 Node parents；children 机械派生 | DAG tests |
| Store Map 派生列 | canonical + map revision + terminal | canonical + Store CAS | migration/State tests |
| App Server snapshot 旧辅助类型 | Edge/Result/Evidence/Maintenance/Sentinel/Trace 共 9 类 | 0 | generated schema diff/test |
| Standard final wire | 受保护 baseline | byte-normalized unchanged | cache gate report |
| 真实 API 成本 | 不适用 | `$0` | 未运行 Whale Agent |

## 4. 验证

| Verification | Result |
|---|---|
| `cargo check -p codex-core -p codex-state -p codex-cli --tests` | PASS |
| `cargo test -p codex-protocol --lib` | 195 passed |
| `cargo test -p codex-core action_map::` | 15 passed |
| `cargo test -p codex-state taskspace_map` | 7 passed |
| `cargo test -p codex-core taskspace_store` | 8 passed |
| projection/provider map/provider wire targeted suites | PASS |
| `cargo test -p codex-cli --test debug_taskspace_map` | 5 passed |
| App Server schema fixtures | 3 passed |
| TUI Action Map Viewer | 3 passed |
| cache payload contract | 3 passed |
| TaskSpace zero-base gate | 6 tests + repository scan PASS |
| cache regression gate | PASS，surface `41bb2280...`；Standard final wire unchanged |

缓存门禁政策文件发生了独立变更，因此发布级 live baseline 仍保持阻断；这不阻断 Phase B2 离线实施。任何真实 Provider
验证仍必须在 VA-02 前单独申请预算。

## 5. 下一入口

Phase B2 从 `EX-01` 开始：在本阶段唯一 canonical transaction 之上定义最小 initialize/update/read/reopen/finish
Map 操作合同，不增加 edge/ref/binding Tool，也不让 Agent 回填 revision。
