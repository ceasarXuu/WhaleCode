# R5-J6.7.7 上下文残留收敛结果

- Date: 2026-07-12
- Status: Engineering/live gate complete after repair; adversarial review pending authorization
- Scope: J6.7.7-A 到 J6.7.7-G engineering/live gate
- Repair commits: `0032a38`, `9e30128`
- R4: historical/unavailable，未补造request、token或cache数据

## 1. 结论

J6.7.7-A-F已完成。G阶段后续3-repeat复验曾暴露严重的Runtime final-rejection provider loop；
`0032a38`删除plain final拒绝注入和自动follow-up，`9e30128`补充open Map确定性集成测试，修复后的
3-repeat engineering/live gate已通过。TaskSpace fresh会话不再平行维护一份持续变化的
Map projection：Agent提交的initialize/control及其原始反馈就是自然上下文；只有resume、compaction或
new epoch需要重建上下文时，才构造一次完整全局Map projection。该结构同时关闭了旧状态矛盾和DeepSeek
prefix缓存断裂。

snapshot改为“生命周期full checkpoint + 相邻状态delta链”。固定每8个provider response写入不断变大
full checkpoint的路径已删除。最终两个样本的checkpoint/rollout均低于30%，相对J6.7.6 full snapshot
bytes下降超过96%。

J6.7尚未正式关闭：engineering/live已完成，仍需用户授权执行对抗性审查。J7继续blocked。

## 1.2 修复后3-repeat复验（2026-07-13 00:21）

有效artifacts：

- focused：`target/r5-final-loop-fix-repeat3/count-call-stack/20260713-002149-383`；
- complex：`target/r5-final-loop-fix-repeat3/subscription-billing-repair/20260713-002149-397`。

| Sample | Mode | Solved | Requests | Tools | Wall | Input | Uncached | Request 2+ cache |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| focused | Standard | 3/3 | 20 | 34 | 64.37s | 140,777 | 7,017 | 94.57% |
| focused | R5 | 3/3 | 26 | 34 | 88.67s | 200,267 | 9,035 | 95.28% |
| complex | Standard | 3/3 | 47 | 71 | 224.56s | 556,639 | 21,471 | 96.03% |
| complex | R5 | 3/3 | 28 | 55 | 152.08s | 285,800 | 15,848 | 94.32% |

| Sample | Request ratio | Tool ratio | Wall ratio | Input ratio | Uncached ratio | Cache delta |
|---|---:|---:|---:|---:|---:|---:|
| focused | 1.30x | 1.00x | 1.38x | 1.42x | 1.29x | +0.71pp |
| complex | 0.60x | 0.77x | 0.68x | 0.51x | 0.74x | -1.71pp |

六个R5 runs全部solved，合计54 requests；`TaskSpaceFinalAnswerRejectedV1`、`final_rejected`、zero cache hit和
same-shape zero均为0。focused三轮及complex repeat-2由Agent直接给出plain final且Map未闭合；Runtime正常
交付回答并忠实保留active Map，没有自动产生recovery request。其他complex两轮由Agent显式`finish_then_end`
完成Map。该差异是Agent是否正确使用账本工具的结果，Runtime不再替Agent纠正或推进。

缓存异常同时闭环：旧批次三个非异常same-shape zero分别发生在final rejection后的50ms、81ms和88ms即时
follow-up；DeepSeek缓存构建为秒级且best-effort。修复删除了这些无业务请求，新六轮没有零命中，因此没有
增加缓存等待、重试或Runtime缓存控制。证据见
`coe/2026-07-13-00-28-r5-same-shape-zero-cache.md`。

## 1.1 三次重复复验（2026-07-12 23:12）

有效artifacts：

- focused：`target/r5-j6-7-7-repeat3-final/count-call-stack/20260712-225957-221`；
- complex：`target/r5-j6-7-7-repeat3-final/subscription-billing-repair/20260712-225957-211`。

| Sample | Mode | Solved | Requests | Wall | Input | Uncached | Request 2+ cache |
|---|---|---:|---:|---:|---:|---:|---:|
| focused | Standard | 3/3 | 27 | 55.95s | 203,923 | 7,827 | 95.97% |
| focused | R5 | 3/3 | 25 | 79.06s | 194,868 | 20,276 | 89.01% |
| complex | Standard | 3/3 | 39 | 204.43s | 469,280 | 23,200 | 94.88% |
| complex | R5 | 3/3 | 152 | 515.30s | 3,674,526 | 1,446,814 | 60.51% |

focused三组correctness稳定，R5总requests/input低于Standard，但wall、uncached和cache受一次same-shape
zero-hit影响，不能声明稳定成本收益。complex的聚合被pair-002异常主导：R5单组达到120 requests、15 nodes、
56 controls、385.69s和3.28M input；另外两组为20/12 requests，说明这是history-dependent机制失控，
不是稳定常态开销。

异常组有50次`final_rejected`，Runtime每次都把拒绝设为`needs_follow_up=true`并自动继续provider sampling；
rollout记录52份`TaskSpaceFinalAnswerRejectedV1` developer feedback。119次相邻请求中118次保持message prefix，
因此不是projection结构再次破坏缓存，也不是provider transport retry。根因与证据见
`coe/2026-07-12-23-17-r5-final-rejection-provider-loop.md`。

## 2. 最终有效证据

| Sample | Artifact | Eligibility |
|---|---|---|
| focused | `target/r5-j6-7-7-final-v4/count-call-stack/20260712-224119-094` | valid paired diagnostic, both solved |
| complex | `target/r5-j6-7-7-final-v4/subscription-billing-repair/20260712-224119-054` | valid paired diagnostic, both solved |

两组均为单次diagnostic，不声明统计显著性。complex runner因E2要求`repeats>=3`且未开启aggregate返回1，
但pair本身`valid_pair=True`、`engineering_unclean=False`、两侧validator通过，不是业务或工程失败。

## 3. 结果、动作与成本

| Sample | Mode | Result | Requests | Runtime tools | Controls | Input | Cached | Uncached | Output | Wall |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| focused | Standard | solved | 10 | 14 | 0 | 78,946 | 76,032 | 2,914 | 1,919 | 26.60s |
| focused | R5 | solved | 11 | 12 | 4 | 78,624 | 75,904 | 2,720 | 1,443 | 22.09s |
| complex | Standard | solved | 12 | 20 | 0 | 125,955 | 118,784 | 7,171 | 5,262 | 53.74s |
| complex | R5 | solved | 12 | 20 | 3 | 135,938 | 129,792 | 6,146 | 4,888 | 50.29s |

| Sample | Request ratio | Tool ratio | Input ratio | Uncached ratio | Wall ratio | Request 2+ cache delta |
|---|---:|---:|---:|---:|---:|---:|
| focused | 1.10x | 0.86x | 1.00x | 0.93x | 0.83x | +0.26pp |
| complex | 1.00x | 1.00x | 1.08x | 0.86x | 0.94x | +1.36pp |

本轮不能推断R5必然更快；可以确认correctness、uncached input和warm cache均无负收益。复杂样本R5与
Standard request/tool数相同，说明TaskSpace机制本身没有必然制造额外模型轮次。

## 4. Projection与缓存根因闭环

首次latest-only实现把projection放在每次请求末尾，下一请求先删除旧projection、追加新tool反馈，再重建
projection。wire trace显示上一请求完整message list不再是下一请求前缀；v2 focused/complex request-2+
cache仅2.89%/10.51%。该实现被判定为负收益并替换，没有作为最终方案保留。

最终合同：

1. blank/fresh active自然历史不注入平行projection，`active_projection_count=0`合法；
2. resume/compaction/new epoch由initial context构造一次projection并持久化，后续事件只追加；
3. scanner接受fresh的0或epoch的1，仍拒绝`>1`，禁止旧projection累积；
4. projection构造保持root、nodes/goals/edges/frontier全局骨架及D1-D3/P0-P3机械详情分层；
5. 骨架超预算显式返回`map_skeleton_over_budget`，不分页、不生成Runtime摘要。

v4的R5 request-2+ cache恢复为focused 96.42%、complex 95.41%，message prefix分别为9/10、10/11；
唯一一次shape变化是blank bootstrap的named tool choice切换为active auto tools。

## 5. 反馈与状态机

| Gate | Focused R5 | Complex R5 |
|---|---:|---:|
| protocol failures | 0 | 0 |
| state failures | 0 | 0 |
| nested ordinary failures | 0 | 0 |
| exact payload duplicate | 0 | 0 |
| orphan call/output | 0 / 0 | 0 / 0 |
| runtime forbidden marker | 0 | 0 |
| retention / salience | 100% / 100% | 100% / 100% |
| semantic replacement / protected miss | 0 / 0 | 0 / 0 |

v3诊断曾出现complex 2次state failure：Agent在`verify`仍开放时尝试terminal，随后在`fix`仍running时
尝试直接bind `verify`。Runtime分别返回原始`active_map_has_open_nodes`和`current_main_node_running`，Agent
读到反馈后改为`finish_nodes(fix -> verify)`并完成。这是正确硬约束与忠实反馈，不是Runtime语义纠正。

## 6. Replay存储

| Sample | Rollout | Full checkpoints | Checkpoint bytes | Ratio | Deltas | Delta bytes | Internal replay ratio |
|---|---:|---:|---:|---:|---:|---:|---:|
| focused | 707,410 B | 2 | 204,925 B | 28.97% | 47 | 233,431 B | 61.97% |
| complex | 826,604 B | 2 | 229,459 B | 27.76% | 59 | 265,995 B | 59.94% |

相对J6.7.6 focused 5.68 MB、complex 9.10 MB full snapshot基线，checkpoint bytes分别下降约96.4%和
97.5%。delta带`base checkpoint hash + previous snapshot hash + result hash`，replay逐段校验并恢复最终
Map/task/result exact hash。

internal replay仍约占rollout 60%，主要是delta内的trace/event refs与canonical runtime events并存；它不
进入provider上下文，也不是累计相对base造成的二次膨胀。长期Map/replay物理压缩已纳入R5-K的K0规模分账，
当前不以语义裁剪或兼容路径临时处理。

## 7. Map结果

focused和complex均为单Map、3 nodes、0 open nodes、task completed。Agent本轮没有声明dependency，因此
edges为0并触发observer机械warning `multi_node_map_without_edges`；Runtime未推断依赖或修改Agent建图。
该warning继续观察，不构成状态机失败。

## 8. 测试与开发环境

| Gate | Result |
|---|---|
| focused Rust tests / scanner / replay chain | passed |
| `action_map_scenario_evaluation` | 7 passed |
| benchmark cost/performance self-tests | passed |
| locked Whale build + binary attestation | passed |
| full `codex-core --lib` | 1817 passed, 2 failed, 3 ignored |

两项full-core失败为既有Linux file-watcher时序测试：
`recursive_registration_downgrades_to_non_recursive_after_drop`和
`unregister_holds_state_lock_until_unwatch_finishes`。它们与J6.7文件无依赖，且此前基线同样失败。

运行经验：benchmark runner不会自动读取repo `.env.local`；正式Docker sample必须在宿主进程中
`source .env.local`后启动runner。每次production commit后必须重新执行locked build和binary attestation，
否则binary freshness preflight会把样本正确标记为`invalid_harness`。

## 9. Gate状态

| Phase | Status |
|---|---|
| A lineage/owner | complete |
| B blank Map/mode | complete |
| C terminal owner | complete |
| D nested/ack owner | complete |
| E projection skeleton/detail | complete |
| F incremental replay/storage | complete |
| G engineering/live | complete；修复后focused/complex各3 repeats全部solved |
| G adversarial review | pending user authorization |

经用户授权完成对抗性审查并关闭critical/high finding后，才可正式关闭J6.7并解锁J7。
