# R7 Phase B 共享核心与 map-always 纵向切换结果

- Created: 2026-07-18
- Updated: 2026-07-18
- Status: Complete / Phase C Ready
- Production Commit: `e9fa52d12`
- Observer Commit: `8b2580698`
- Machine Result: `benchmarks/taskspace/r7/phase-b-result.json`

## 1. 阶段结论

Phase B 已完成。R7 现在只有一个 projection policy 核心、一个 renderer 和一个 provider composer；
生产配置只开放 `map-always`。R6 的 epoch cache、anchor、scope、session epoch state 和对应生产 marker
已经删除，没有保留迁移、双读、fallback 或隐藏第四模式。

`map-always` 的正确性门禁通过：两个 Docker 样本共 36 次 TaskSpace provider request，每次都只有
一份 projection，且 `map_id + revision + canonical hash + projection hash` 与发送前期望完全一致。
两组 Standard/TaskSpace 均通过 public/hidden validator，Map 均正常闭合。

成本结果也符合该策略的已知取舍：最新 projection 每轮替换会破坏上一轮动态位置之后的精确缓存
前缀。Phase B 不尝试用裁剪、旧 projection 累加或 Runtime 语义提示掩盖这个特征。

## 2. 工程改造

| 区域 | 结果 |
|---|---|
| Policy | 新增 `TaskSpaceProjectionPolicy`、trigger、cursor、emission 纯决策 |
| Renderer | 改为 `TaskSpaceMapProjectionR7V1`，不含 epoch 产品语义 |
| Composer | provider request 前移除旧自动 projection，在自然历史末尾放一份当前 projection |
| Identity | 对账 policy、Map ID、revision、canonical hash、projection hash |
| Session | 新 session 从配置冻结；resume/fork 只从 rollout metadata 恢复 |
| Invalid lifecycle | TaskSpace 无显式 policy 时机械拒绝；R6 session 不迁移 |
| Observer | wire、exact payload、成本报表均识别 R7 identity |
| 删除 | `taskspace_projection_epoch.rs`、epoch session state、anchor/scope 和专属日志 |

Renderer 和 policy 没有状态机决策能力。它们只读取 canonical Map、构造 projection、决定机械的
context emission；没有 next action、节点优先级、任务摘要或 Agent 建议。

## 3. Freshness 与反馈门禁

| 指标 | Simple | Complex | 合计 |
|---|---:|---:|---:|
| TaskSpace provider request | 18 | 18 | 36 |
| 每 request 最大 projection 数 | 1 | 1 | 1 |
| exact scan failure | 0 | 0 | 0 |
| identity unconfirmed | 0 | 0 | 0 |
| replacement unconfirmed | 0 | 0 | 0 |
| policy | map-always | map-always | 单一值 |
| observed revision | 2-7 | 2-7 | 当前值逐请求对账 |

同一 graph revision 下 canonical hash 仍可能变化，因为节点证据与结果可更新而不一定修改图结构；
因此 freshness 不能只看 revision。当前 scanner 使用完整四元身份逐请求对账，没有把“同 revision”
错误等价为“同 projection”。

复杂样本中 Agent 连续提交过格式损坏的大 patch carrier。Runtime 返回了明确的
`protocol_failed / invalid_arguments`，没有提交状态；随后一次 patch 本身失败时，control result 也
忠实返回 `success:false, state_commit:true` 和 committed delta。该 trace 说明反馈没有被吞掉或改写，
但也留下一个独立的 control 大参数稳定性观察项，不能归因为 projection policy。

## 4. 三臂快速对照

Frozen R6 来自 Phase A 同日冻结基线；Standard 和 R7 是本阶段新成对运行。三者 scenario、模型、
reasoning effort 和 Docker hard boundary 一致，但不是同一随机采样轮，因此只作诊断对照，不作效用
或默认策略结论。

### 4.1 Simple：single-file-fast-fix

| 指标 | Current Standard | Frozen R6 | R7 map-always |
|---|---:|---:|---:|
| 结果 | solved | solved | solved |
| provider request | 6 | 19 | 18 |
| provider outer tool / control | 9 / 0 | 18 / 9 | 25 / 7 |
| wall time | 15.78s | 46.55s | 42.72s |
| input token | 40,686 | 231,221 | 207,247 |
| cached input | 38,912 | 207,744 | 3,584 |
| uncached input | 1,774 | 23,477 | 203,663 |
| output token | 1,576 | 4,499 | 3,147 |
| request 2+ cache hit | 95.24% | 89.74% | 1.76% |
| message prefix preserved | 100.00% | 88.89% | 0.00% |
| Map nodes / edges / open | 0 / 0 / 0 | 5 / 4 / 0 | 5 / 4 / 0 |

R7 相对当前 Standard：request `3.00x`、wall `2.71x`、input `5.09x`。相对冻结 R6，R7 的
request、wall 和 input 略低，但 uncached input 为 `8.68x`；收益不足以抵消 cache 形态差异。

### 4.2 Complex：subscription-billing-repair

| 指标 | Current Standard | Frozen R6 | R7 map-always |
|---|---:|---:|---:|
| 结果 | solved | solved | solved |
| provider request | 11 | 16 | 18 |
| provider outer tool / control | 21 / 0 | 22 / 7 | 36 / 11 |
| wall time | 58.14s | 58.93s | 77.78s |
| input token | 129,168 | 209,772 | 279,004 |
| cached input | 120,192 | 184,704 | 81,920 |
| uncached input | 8,976 | 25,068 | 197,084 |
| output token | 6,778 | 5,767 | 9,384 |
| request 2+ cache hit | 92.77% | 87.87% | 28.59% |
| message prefix preserved | 100.00% | 86.67% | 0.00% |
| Map nodes / edges / open | 0 / 0 / 0 | 4 / 3 / 0 | 5 / 4 / 0 |

R7 相对当前 Standard：request `1.64x`、wall `1.34x`、input `2.16x`。相对冻结 R6：request
`1.13x`、wall `1.32x`、input `1.33x`，且 uncached input 为 `7.86x`。

## 5. 缓存根因

这不是 projection 重复注入。Wire trace 显示每次都只有一份最新 projection，旧自动 projection
数量为零。

`map-always` 中，第 N 次请求的末项是 ephemeral projection；它不进入持久自然历史。第 N+1 次
请求时，新 assistant/tool 消息占据上一轮 projection 所在的索引，当前 projection 再移动到新的末尾。
因此 provider 的精确 message prefix 在旧 projection 位置结束，后续内容无法复用。Simple 的
request 2+ hit 降至 1.76%，Complex 为 28.59%，属于宪章已声明的策略特征。

## 6. 测试结果

```text
cargo check --workspace --all-targets                         PASS
cargo fmt --all -- --check                                   PASS
cargo test -p codex-core projection -- --nocapture           18 passed
R7 projection policy contract                                PASS
cost instrumentation selftest                                PASS
performance observation selftest                             PASS
TaskSpace benchmark harness self-test                        PASS
Docker simple/complex Standard + R7                           all solved
```

完整 `codex-core --lib`：`1883 passed / 25 failed / 3 ignored`。没有 Phase B projection/config
失败。两个 TaskSpace 子代理 spawn/replay 失败已在 Phase A commit `c4f3c3c57` 上以相同错误复现，
记录在 `coe/2026-07-18-03-15-r7-subagent-map-restore.md`；其余失败来自共享 `/tmp`、file watcher、
guardian 测试未注入 key、临时 diff 路径归一化等既有环境或非本阶段范围。

## 7. Phase C 准入

Phase B 退出门禁已满足，可以进入 Phase C，但当前先暂停：

1. R6 epoch production symbol 为零；
2. 36/36 TaskSpace request 的 projection 唯一且 identity 精确；
3. Standard 未注入 projection，TaskSpace Map/Root/Finish 正常闭合；
4. session policy 创建、持久化、resume/fork 恢复合同已接通；
5. 缓存下降按 `map-always` 产品特征记录，没有通过 Runtime 语义干预修饰结果。

Phase C 只能在同一 policy/renderer/composer 上增加 `AppendSnapshot`，不得恢复 epoch baseline 或复制
第二套 provider context 路径。
