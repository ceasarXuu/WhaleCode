# R5-K2.F 损坏恢复结构化终止结果

- 日期：2026-07-13
- 状态：COMPLETE
- Candidate code commit：`73226bea0c59c98bf051f207af4166ee74f6e6bc`
- Live artifact：`target/r5-map-compression/K2.F-smoke`

## 1. 实施结果

rollout 恢复由 panic/`expect` 改为带稳定 `phase` 和 `message` 的 `RolloutReconstructionError`。session 初始化在
恢复校验失败时记录 `taskspace.rollout_reconstruction_fatal`、发送 `ErrorEvent`，并终止 session 构造；rollback
恢复失败时同样先返回错误，不继续重算 token 或持久化。

恢复校验在取得 session state 写锁前完成，`next_turn_is_first` 也只在成功恢复后更新。因此损坏 checkpoint、delta
或 TaskSpace event 不会留下部分 history、Map 或 session 标记。本阶段没有接入 archive codec，也没有改变 projection、
tool schema、provider 消息或 Map 状态推进规则。

## 2. 故障矩阵

| 故障 | 稳定 phase | 结果 |
|---|---|---|
| delta 缺少 checkpoint | `map_checkpoint_delta_chain` | fatal，Map/history 不变 |
| 中间 delta 缺失 | `map_checkpoint_delta_chain` | fatal，Map/history 不变 |
| checkpoint hash 损坏 | `map_checkpoint_delta_chain` | fatal，Map/history 不变 |
| TaskSpace ownership 序列非法 | `taskspace_ownership_checkpoint` | fatal，Map/history 不变 |
| TaskSpace event 序列非法 | `taskspace_event_sequence` | fatal，Map/history 不变 |

五类故障全部显式失败，不提供兼容或静默 fallback。

## 3. 测试证据

| 验证 | 结果 |
|---|---|
| corruption matrix | 5/5 PASS |
| `session::rollout_reconstruction_tests` | 30/30 PASS |
| `record_initial_history` 相关测试 | 17/17 PASS |
| rollback 相关测试 | 20/20 PASS |
| K0 long replay | 2/2 PASS |
| `codex-core --lib` | 1835 PASS，11 FAIL，3 ignored |
| Docker normal-path smoke | simple/complex 的 B0/C 共 4/4 PASS |

整库 11 个失败不在本次恢复变更路径：2 个 file-watcher 环境/时序失败，多数 guardian/MCP/session guardian
测试缺少 `DEEPSEEK_API_KEY`，另有 1 个 thread manager 失败。本次涉及的 52 个恢复、初始化、rollback 和 long
replay 用例全部通过。

## 4. 正常路径观察

| Sample | Arm | Requests | Input tokens | Cached tokens | Wall ms |
|---|---|---:|---:|---:|---:|
| simple | B0 | 8 | 62059 | 59520 | 26845 |
| simple | C | 8 | 67007 | 62592 | 29828 |
| complex | B0 | 11 | 119571 | 114304 | 56358 |
| complex | C | 16 | 235392 | 227200 | 85433 |

4 个运行均正确完成，未出现 `taskspace.rollout_reconstruction_fatal`。K2.F 没有正常路径调用点，单次候选轨迹的
额外 request 是 Agent 普通动作差异，不能归因为恢复机制，也不作为压缩收益或回归结论。S1 仍按冻结合同执行每臂
3 次正式对照。

## 5. 阶段结论

K2.F 达到进入 S1 的门禁：损坏链路 structured fatal 100%，partial restore 为 0，正常路径没有新增 production
行为。下一步只允许接入 `S1 = completed_inactive_leaf_batch_archive_projection`，完成独立验收后暂停，不叠加 S2。
