# R6 Phase E2 Canonical Replay 实施结果

- Created: 2026-07-16
- Updated: 2026-07-16
- Status: Completed
- Scope: Phase E2 only
- Related Design: `10-r6-terminal-replay-convergence-design.md`

## 1. 结论

Phase E2 已通过独立退出门禁，可以进入 E3。生产恢复与离线观测不再各自解释
checkpoint/delta：两者统一调用 Rust canonical replay，并对 loader、存活 segment、delta 链、snapshot
恢复不变量和最终 hash 给出相同 verdict。

本阶段不修改 observer 的最终状态来源，不修改 terminal provider 行为，也不修改 Hook。Phase E 尚未完成。

## 2. 已落地能力

1. 新增单次读取的 rollout loader；原始字节同时用于 SHA256 和 JSONL 解析，避免读取与校验对象漂移。
2. 新增 rollback/compaction aware 的统一 reducer，明确区分 parsed、surviving 和 active chain 三类计数。
3. delta sequence、base id/hash、previous/result hash、JSON Patch 和 snapshot restore 使用稳定 typed error code。
4. production session reconstruction 已删除私有 replay 分支，改为调用同一个 canonical reducer。
5. 新增 `whale debug taskspace-replay --rollout ... --output ...`，成功只输出 proof + snapshot，失败输出无
   partial snapshot 的稳定错误 envelope。
6. resume 遇到 JSONL parse error 时明确 fatal，不再静默跳过坏行后恢复旧状态。

## 3. 语义忠实性修复

严格 restore round-trip 暴露了两处原有语义丢失，均修正于 canonical 恢复路径：

| 问题 | 原行为 | 修复后 |
|---|---|---|
| trace tags 被重解释 | restore 使用固定白名单删除未知但合法的持久化 tags | 原样保留 snapshot 中的 tags |
| 机械空 Map 被改写 | restore 丢失已有 task/map identity，并重新初始化 routing state | 根据持久化机械初始化事实恢复同一 identity/state |

校验保持严格：只有 `restore(snapshot)` 后重新生成的 snapshot 与输入完全相等才通过。没有通过兼容分支、字段
忽略或校验降级掩盖差异。

## 4. 测试结果

| 验证 | 结果 | 覆盖重点 |
|---|---:|---|
| `cargo test -p codex-core taskspace_replay --lib` | 15 passed | loader、链校验、rollback、compaction、restore |
| `cargo test -p codex-core rollout_reconstruction --lib` | 31 passed | production resume 与共享 reducer |
| `cargo test -p codex-rollout get_rollout_history_rejects_tail_parse_errors` | 1 passed | 尾部 parse error fatal |
| `cargo test -p codex-cli debug_taskspace_replay` | 2 passed | success/error envelope、原子覆盖输出 |
| `cargo test -p codex-core action_map::runtime --lib` | 13 passed | snapshot restore 与 runtime 不变量 |
| `cargo build -p codex-cli --bin whale` | passed | CLI 构建 |
| `just fix -p codex-core/codex-rollout/codex-cli` | passed | lint 自动修复 |
| `just fmt` | passed | Rust 格式化 |

未执行全 workspace test；本阶段按改动边界执行了 targeted regression，完整 workspace 测试依项目规则需要用户
单独授权。

## 5. 真实失败样本 Replay Proof

输入为 R6-E-OBS-01 冻结 rollout：

`target/r6-phase-e/finish-boundary/subscription-billing-repair/20260715-232923-210/pair-001/right/artifacts/rollout.jsonl`

| 事实 | Canonical proof |
|---|---|
| raw rollout SHA256 | `fe4aba73fd99632c2c96b35aca7f7bd0858e144cbbf532c38626b5c8266daddd` |
| parse errors | 0 |
| parsed checkpoint / delta | 2 / 72 |
| surviving checkpoint / delta | 2 / 72 |
| active checkpoint | `map-checkpoint-33a9a6fd152e1d43` |
| active chain delta | 70，last sequence 70 |
| final snapshot SHA256 | `a4fab287c5bd34e6ec3724ee7561e34da31fe066766cc7bc79fc4ba9fa77f07a` |
| final map revision | 7 |
| graph state | 3 个 Work 均 COMPLETED，Finish READY，Root OPEN，map complete=false |
| final collections | 4 edges，0 leases，3 results |

该结果确认 OBS-01 是 observer checkpoint-only read model 分叉：rollout 中完整 delta 链存在，canonical replay
可以准确恢复最新状态。

## 6. 阶段判定

| Gate | 判定 |
|---|---|
| resume/offline 对同一 rollout 的 hash 与 verdict 一致 | PASS |
| corruption 使用稳定 typed code 且无 partial snapshot | PASS |
| restore 不扭曲、丢失或静默裁剪 snapshot 语义 | PASS |
| E3 observer 切换的 canonical proof 输入可用 | PASS |

下一阶段 E3 只做 observer 纵向切换：PowerShell 保留 timeline/count/report 构造，最终 Map collections 必须只从
本阶段 proof snapshot 构造。
