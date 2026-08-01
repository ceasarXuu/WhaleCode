# R7 原子完成交接修复结果

## 1. 结论

R5 已经建立的“完成节点时必须同时声明下一步”在 R6 Rooted DAG 改造时发生了合同回归：内部状态机的
`Complete` 被直接暴露成 provider 可调用 action，而 `complete + bind/action` 又被拆成同一响应中的 sibling
调用。工作协议虽然鼓励合并，工具 schema 却允许并显著展示单独完成，最终 Agent 稳定选择了后者。

修复后，provider 不再看到独立 `complete`。Agent 只能选择：

- `complete_then_continue`：完成当前 Work，同时绑定明确的下一 Work 并携带后续动作；
- `complete_then_end`：完成当前 Work，同时由 Agent 给出最终总结并闭合 Finish/Root；
- `finish_end`：仅用于当前已经没有 Running Work、Finish 已 Ready 的图。

这次修改只收敛 `taskspace_control` 合同和对应的机械事务，没有增加 projection 策略分叉，没有解析
reasoning，也没有让 Runtime 推断下一节点或修补 Agent 参数。

## 2. 工程实现

| 层 | 结果 |
|---|---|
| Tool schema | 移除 provider 可见的独立 `complete`，新增两个原子 action |
| Rooted DAG | completion、readiness、bind/terminal 进入同一 revision EventBatch |
| Runtime | candidate graph + lease 原子切换，失败不安装部分状态 |
| Session | `complete_then_end` 与 terminal persistence 原子提交 |
| Replay | 识别两种新 transaction，拒绝拆分提交形成的 crash window |
| Feedback | 成功只返回 committed delta；失败固定零提交，不返回主观建议 |
| Protocol | `v1.0.2` 与 tool schema 对齐，不再要求 sibling lifecycle calls |
| Logging | 增加 handoff/terminal committed/rejected 结构化事件 |
| Observer | 独立统计 handoff、terminal 和 standalone complete |

实现提交：`26814f3f4 fix(taskspace): restore atomic completion handoffs`。

## 3. 测试

通过的机械验证：

- `codex-tools`：141 passed，1 ignored；
- `taskspace_control`：28 passed；
- Runtime：10 passed；
- replay：20 passed；
- working protocol：2 passed；
- sequence：15 passed；
- terminal integration：2 passed；
- performance observer、K0 map budget observer、skill quick validation：通过；
- `cargo build -p codex-cli --bin whale`：通过并生成 binary attestation。

`codex-core --lib` 全量测试仍被本变更外的既有环境/并发用例和一个 Agent control stack overflow 阻断；
本次涉及的 TaskSpace 套件已独立全部通过。

## 4. Docker 同期对比

两组均为单次 `E2-candidate`，用于机制验收，不作为三重复统计结论。基准脚本因 `repeats_lt_3` 返回非零，
但 pair 均 `valid_pair=True`、`engineering_unclean=False`，Standard 与 TaskSpace 都通过公开和隐藏验证。

| Sample | Mode | 结果 | Requests | Tools | Wall | Input | Uncached | Output | Req 2+ cache |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| simple | Standard | solved | 7 | 9 | 22.84s | 52,008 | 1,704 | 2,279 | 96.55% |
| simple | TaskSpace | solved | 8 | 11 | 23.43s | 84,096 | 11,264 | 2,145 | 96.07% |
| complex | Standard | solved | 10 | 19 | 61.12s | 112,086 | 5,334 | 6,276 | 95.03% |
| complex | TaskSpace | solved | 12 | 18 | 53.16s | 159,794 | 6,450 | 5,155 | 95.85% |

TaskSpace 两组 Map 都是 4 nodes / 3 edges / 0 open，并且控制节奏一致：

| Action | simple | complex |
|---|---:|---:|
| `initialize_map` | 1 | 1 |
| `complete_then_continue` | 1 | 1 |
| `complete_then_end` | 1 | 1 |
| provider-visible standalone `complete` | 0 | 0 |
| `finish_end` | 0 | 0 |

simple 的实际路径是 `initialize_map + ls` → 调查与修复 → `complete_then_continue + pytest` →
`complete_then_end`。因此节点完成不再占用一个无后续工作的独立 provider request。

## 5. 新观察

complex 首次尝试把大型四文件 patch 嵌入 `complete_then_continue` 时，模型生成的 JSON 末尾多了一个 `}`；
下一次又发出空 `{}`。Runtime 两次都返回 `protocol_failed`，且
`state_commit=false/partial_commit=0`，Agent 随后自行改用合法 handoff 完成任务。

这是“大型嵌套工具参数生成稳定性”问题，不是 completion handoff 状态合同失效。后续专项诊断已经证明：
问题由 provider 在 TaskSpace 复合 carrier 中生成长 patch 时产生，历史 54 次 carrier 中有 15 次 JSON
非法；失败反馈完整进入后续上下文。仅扁平化字段虽能改善可解析性，却不能保证 patch 正文忠实。
详细结论见 `12-r7-nested-patch-control-root-cause.md`。

## 6. 判定

H-009 的回归修复门关闭：工具、Runtime、事件、持久化、replay、反馈、协议和 observer 已一致支持原子完成交接，
两个真实 Docker 样本均采用新 action，独立完成动作未再出现。Phase D.2 完成，Phase E 仍未启动。
