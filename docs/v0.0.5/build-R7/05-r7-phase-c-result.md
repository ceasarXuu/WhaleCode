# R7 Phase C map-append 接入结果

- Created: 2026-07-18
- Updated: 2026-07-18
- Status: Complete / Phase D Ready
- Production Commit: `5f25aad60`
- Observer Commit: `9e3debba1`
- Startup Fix Commit: `e753ea864`
- Machine Result: `benchmarks/taskspace/r7/phase-c-result.json`

## 1. 阶段结论

Phase C 已完成。`map-append` 已接入 Phase B 建立的同一 policy、renderer 和 provider composer，
没有复制第二条 context 构造路径。每次 canonical revision 提交后，只追加一份不可变 snapshot；同一
revision 的重试不重复追加，旧 revision 明确标记为历史，最高 revision 是唯一当前状态。

两个 Docker 样本共 40 次 TaskSpace provider request，exact scanner 未发现 revision 顺序、重复、
identity 或 supersession 违规。Simple 最终持久历史为 revision `2..8`，Complex 为 `2..10`；两组
canonical Map 均闭合，Standard 与 R7 两臂均通过 public/hidden validator。

`map-append` 的缓存表现明显高于 Phase B 的 `map-always`，但仍低于当轮 Standard；同时旧 snapshot
会持续增加输入。该结果符合策略定义，不作为默认策略结论。本阶段每臂只有 1 次运行，冻结 R6 也是
同日历史样本，因此只用于机制验收和诊断，不用于效用排序。

## 2. 工程改造

| 区域 | 结果 |
|---|---|
| Envelope | 共享 envelope 增加 `revision_snapshot`、`supersedes_through_revision`、`highest_revision_only` |
| Policy | committed revision 触发 `AppendRevision`；同 revision 抑制，旧 revision 机械拒绝 |
| Transaction | 同一 control carrier 的多个 event 只捕获最终 revision；状态与 projection 原子安装 |
| Ordering | snapshot 在顶层 tool output 后刷新，保持 tool call/output 相邻关系 |
| Composer | `map-append` 保留不可变历史，provider request 不再额外注入动态 projection |
| Resume/compaction | 从可见历史机械恢复 cursor；新 context epoch 只补当前 revision 一次 |
| Scanner | 校验严格递增、最新 identity、Map 生命周期分段；不再套用 always 的唯一数量规则 |
| Schema | 明确同一 `map_id` 下只有最高 revision 是当前状态，不增加 Agent 工作建议 |
| Logs | 增加 emission decision、revision append、projection kind/revision 成本字段 |
| Session | Phase C 开放 `map-always` 与 `map-append`；`map-request` 继续机械拒绝 |

Renderer 仍只忠实构造 canonical Map；policy 只决定 projection 如何进入 context。Runtime 没有增加
节点优先级、下一步动作、任务摘要或纠错建议。

## 3. Revision 与反馈门禁

| 指标 | Simple | Complex | 合计 |
|---|---:|---:|---:|
| TaskSpace provider request | 12 | 28 | 40 |
| provider request 内最大 snapshot 数 | 6 | 8 | 8 |
| 最终持久 snapshot revision | `2..8` | `2..10` | 严格递增 |
| 最终持久 snapshot 数 | 7 | 9 | 16 |
| terminal canonical revision | 8 | 10 | 全部对齐 |
| exact scan failure | 0 | 0 | 0 |
| identity unconfirmed | 0 | 0 | 0 |
| duplicate / order violation | 0 / 0 | 0 / 0 | 0 / 0 |
| Map nodes / edges / open | 5 / 4 / 0 | 6 / 5 / 0 | 全部闭合 |

Simple 的一次无 binding ordinary action 和一次空 continuation 被硬规则正确拒绝。Complex 中，格式
损坏的 init、保留 target、失败 patch 以及无 binding complete 均返回明确原始错误；nested tool
output 紧邻对应 control output，Agent 随后能够重试。未观察到 projection 导致的反馈丢失、扭曲或
错误提交。

## 4. 三臂快速对照

Frozen R6 来自 Phase A 冻结基线；Standard 和 R7 是本阶段新成对运行。模型均为
`deepseek-v4-flash`、reasoning effort 为 `max`，执行边界均为 Docker hard boundary。

### 4.1 Simple：single-file-fast-fix

| 指标 | Current Standard | Frozen R6 | R7 map-append |
|---|---:|---:|---:|
| 结果 | solved | solved | solved |
| provider request | 10 | 19 | 12 |
| runtime tool / control | 12 / 0 | 18 / 9 | 9 / 11 |
| wall time | 24.21s | 46.55s | 29.58s |
| input token | 77,664 | 231,221 | 136,043 |
| cached input | 74,752 | 207,744 | 61,824 |
| uncached input | 2,912 | 23,477 | 74,219 |
| output token | 2,041 | 4,499 | 2,711 |
| request 2+ cache hit | 96.10% | 89.74% | 46.51% |
| message prefix preserved | 100.00% | 88.89% | 81.82% |
| Map nodes / edges / open | 0 / 0 / 0 | 5 / 4 / 0 | 5 / 4 / 0 |

R7 相对当前 Standard：request `1.20x`、wall `1.22x`、input `1.75x`；相对冻结 R6：request
`0.63x`、wall `0.64x`、input `0.59x`，但 uncached input 为 `3.16x`。

### 4.2 Complex：subscription-billing-repair

| 指标 | Current Standard | Frozen R6 | R7 map-append |
|---|---:|---:|---:|
| 结果 | solved | solved | solved |
| provider request | 16 | 16 | 28 |
| runtime tool / control | 23 / 0 | 22 / 7 | 35 / 11 |
| wall time | 57.76s | 58.93s | 92.42s |
| input token | 193,007 | 209,772 | 526,926 |
| cached input | 184,832 | 184,704 | 366,336 |
| uncached input | 8,175 | 25,068 | 160,590 |
| output token | 6,145 | 5,767 | 10,077 |
| request 2+ cache hit | 95.66% | 87.87% | 69.36% |
| message prefix preserved | 100.00% | 86.67% | 92.59% |
| Map nodes / edges / open | 0 / 0 / 0 | 4 / 3 / 0 | 6 / 5 / 0 |

R7 相对当前 Standard：request `1.75x`、wall `1.60x`、input `2.73x`；相对冻结 R6：request
`1.75x`、wall `1.57x`、input `2.51x`，uncached input 为 `6.41x`。本次放大主要来自 Agent 路径
增加，而不是 snapshot 重复：28 次 request 的 revision/identity 扫描均通过。

## 5. 输入与缓存解释

| 指标 | Simple | Complex |
|---|---:|---:|
| provider request 累计可见 projection | 62,326 B / 15,586 est. tokens | 235,412 B / 58,860 est. tokens |
| 累计自然历史 | 70,374 B / 17,598 est. tokens | 604,232 B / 151,068 est. tokens |
| 累计 tool schema | 321,912 B / 80,484 est. tokens | 751,128 B / 187,796 est. tokens |
| 最终持久 projection | 13,529 B / 3,385 est. tokens | 24,137 B / 6,038 est. tokens |

`map-append` 保持线性消息前缀，Phase B `map-always` 的 request 2+ cache hit 分别为 1.76% 和
28.59%，本阶段为 46.51% 和 69.36%。但 DeepSeek 缓存不是“message prefix 相同就保证全部命中”：
新 revision 后的首次请求仍可能低命中，tool choice 形态切换也会改变可缓存前缀。旧 snapshot 持续
可见造成的 input 增长是 `map-append` 的已知产品特征，不应伪装为实现缺陷，也不能据单次样本推断
稳定收益。

## 6. 测试结果

```text
cargo check -p codex-core --tests                           PASS
cargo fmt --all -- --check                                  PASS
cargo test -p codex-core projection -- --nocapture          25 passed
provider map-append / rollback / session policy tests        PASS
codex-tools taskspace control tests                          3 passed
R7 projection policy contract                               PASS
cost instrumentation selftest                               PASS
performance observation selftest                            PASS
TaskSpace benchmark harness self-test                        PASS
Docker simple/complex Standard + R7                          all solved
```

完整 `codex-core --lib`：`1894 passed / 25 failed / 3 ignored`。失败项与 Phase B 的 25 项集合一致；
新增 Phase C 测试全部进入通过项。两个既有 TaskSpace 子代理 spawn/replay 失败继续记录在
`coe/2026-07-18-03-15-r7-subagent-map-restore.md`，其余为共享 `/tmp`、file watcher、guardian key
注入、临时 diff 路径等非本阶段失败。

## 7. 无效运行与运行经验

两次预运行不进入效果证据：`051502` 因 benchmark 子进程不会自动加载仓库 `.env.local`，provider
preflight 在发出模型请求前失败；`051530` 暴露了 `Session::new` 遗留的 Phase B policy gate，导致
`map-append` 启动即被拒绝。后者已在 `e753ea864` 修复并增加 startup regression test。

以后从 shell 启动 benchmark 时，应使用 `set -a; source .env.local; set +a` 将环境显式导出，且不得
打印 secret。`run_score_valid=true` 的 `052006` 两组 run 才是本阶段有效样本。runner 因
`repeats=1` 且未启用 aggregate 返回非零，只表示不允许生成统计效用结论，不表示 pair 失败。

## 8. Phase D 准入

Phase C 的 6 项退出门禁全部满足，可以进入 Phase D，但当前暂停：

1. same revision duplicate 为 0；
2. revision order violation 为 0；
3. 最高 emitted revision 与 terminal canonical revision 一致；
4. 40/40 provider request 的最新 identity 可精确复核；
5. 输入增长和旧 snapshot 数量已完整量化；
6. shared renderer/composer 未产生策略架构分叉。

Phase D 只能在共享 `taskspace_control` 上增加 `read_map`，并令 `map-request` 改变 emission decision；
不得修改 Map 状态机、ordinary tool 权限或复制 renderer/provider context 路径。
