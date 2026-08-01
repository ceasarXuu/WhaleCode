# R7 五层架构 FLA-2 结果

- 日期：2026-07-20
- 状态：`historical_acceptance_blocked / superseded_by_active_verified_repair`
- 生产提交：`2ea8b4d24`
- 候选二进制 SHA256：`90bdaba699e58b80806fb3c200c03166f668001314bb6de225be4a7da4e2586f`
- Projection policy：`map-request`
- 机器结果：[`five-layer-fla2-result.json`](../../../benchmarks/taskspace/r7/five-layer-fla2-result.json)
- 阻塞调查：[`29-r7-fla2-blockers-and-control-path-investigation.md`](29-r7-fla2-blockers-and-control-path-investigation.md)
- 修复结果：[`30-r7-fla2-blocker-repair-result.md`](30-r7-fla2-blocker-repair-result.md)

> 更正：本文件最初把 FLA-2 标为 `active_verified`。后续请求级审查确认存在第三条静态 system Map handle、L2 与
> 当前 Tool result 能力不一致，以及观测器漏报 preflight reject/真实 commit。以下运行数据继续保留，但原验收结论
> 已撤回；在阻塞项关闭前不得将 FLA-2 作为 FLA-3 的已验证前置阶段。

> 2026-07-21 更新：B1、B2、旧 L4 discriminator、binding 事实反馈和观测少报已经完成修复，并通过一个简单
> 样本和一个复杂样本的 Docker 配对冒烟。原始数据与撤回结论作为历史证据保留；最新状态以 30 号修复结果为准。
> `H-003` 的跨 top-level sibling 结构问题仍未关闭。

> 2026-07-21 最终更新：Base 2.0.1 / manifest 1.0.2 的 current-identity simple/complex smoke、证据新鲜度与 raw-count
> gate、四轮独立对抗性闭环均已完成，Round 4 verdict 为 `pass_reacceptance`。FLA-2 已恢复 `active_verified`；本文件
> 后续表格继续作为最初 FLA-2 候选的历史数据，不替代 30 号修复结果。

## 1. 实施结果

1. TaskSpace Base 升级到 `2.0.0`，只保留 Map 的价值、图模型和 Agent/Runtime 宏观责任边界。
2. 具体工作循环提取为逐字冻结的 `taskspace-core-v2`，作为现有 developer bundle 的第一段；没有新增
   composer、session state、注入分支或动态提示。
3. Standard 不装配 L2；三种 TaskSpace projection policy 共用同一 L1/L2 和既有工具、状态机、反馈链。
4. provider wire trace v6 增加 L2 的 count、位置、role、section order、版本、hash、bytes 和匹配状态。
5. Base 中文审阅稿、authority manifest、生产 manifest 和合同测试同步到同一版本。

## 2. Wire 与固定成本

| 检查项 | 结果 |
|---|---:|
| TaskSpace provider 请求 | 65 |
| L1 `2.0.0` 精确匹配 | 65/65 |
| L2 唯一、第二条 system 第一 section、精确 hash | 65/65 |
| Standard provider 请求 | 56 |
| Standard 无 L2 | 56/56 |
| TaskSpace L2 重复、缺失或错位 | 0 |

| 固定 system 内容 | Bytes / request | 估算 token / request | 相对 FLA-0 TaskSpace |
|---|---:|---:|---:|
| FLA-0 TaskSpace | 22,700 | 5,675 | 基线 |
| FLA-2 TaskSpace | 21,666 | 5,417 | -1,034 bytes / -258 token（约 -4.6%） |
| FLA-2 Standard | 21,534 | 5,384 | 不适用 |

L2 虽然新增 1,597 bytes，但 Base 删除了被 L2 接管的重复教程，最终 TaskSpace 固定 system 内容净减少；
当前 TaskSpace 只比 Standard 多 132 bytes、约 33 个估算 token。

## 3. 当前配对结果

每个样本 3 个 pair；所有 12 个模式侧均通过公开与隐藏验证，`engineering_unclean_count=0`。

| 样本 | 模式 | 成功 | Request（总/均值/中位） | 普通工具 | Control | Control 失败 | Map 节点/边/开放叶 |
|---|---|---:|---:|---:|---:|---:|---:|
| simple | Standard | 3/3 | 23 / 7.67 / 7 | 31 | 0 | 0 | 不适用 |
| simple | TaskSpace | 3/3 | 29 / 9.67 / 9 | 32 | 20 | 3 | 15 / 12 / 0 |
| complex | Standard | 3/3 | 33 / 11.00 / 11 | 54 | 0 | 0 | 不适用 |
| complex | TaskSpace | 3/3 | 36 / 12.00 / 13 | 45 | 25 | 4 | 16 / 13 / 0 |

| 样本 | 模式 | Input（总/均值/中位） | Cached（总/均值/中位） | Uncached（总/均值/中位） | Output（总/均值/中位） | Wall ms（总/均值/中位） | Cache hit 2+ |
|---|---|---:|---:|---:|---:|---:|---:|
| simple | Standard | 265,242 / 88,414 / 78,687 | 258,944 / 86,315 / 76,800 | 6,298 / 2,099 / 1,998 | 5,665 / 1,888 / 2,056 | 56,947 / 18,982 / 19,313 | 97.48% |
| simple | TaskSpace | 402,925 / 134,308 / 119,600 | 379,648 / 126,549 / 109,184 | 23,277 / 7,759 / 5,722 | 8,651 / 2,884 / 3,047 | 79,398 / 26,466 / 25,507 | 96.76% |
| complex | Standard | 473,016 / 157,672 / 153,941 | 456,960 / 152,320 / 149,120 | 16,056 / 5,352 / 5,225 | 15,110 / 5,037 / 4,352 | 137,775 / 45,925 / 44,387 | 96.42% |
| complex | TaskSpace | 575,247 / 191,749 / 202,106 | 552,448 / 184,149 / 196,480 | 22,799 / 7,600 / 7,476 | 16,343 / 5,448 / 4,972 | 139,678 / 46,559 / 45,453 | 96.93% |

简单样本第一次 TaskSpace 请求使用了新的 L1/L2 cache shape，记录为一次 `warmup candidate`；之后没有
`same-shape zero` 或 cache shape transition。因此 23,277 的 uncached 总量包含一次版本切换冷启动，持续缓存
表现应优先看 request 2+。

## 4. 相对效果

| 样本 | 版本 | Request 放大 | Input 放大 | Wall time 放大 |
|---|---|---:|---:|---:|
| simple | FLA-0 TaskSpace / 同轮 Standard | 1.364x | 1.649x | 1.314x |
| simple | FLA-2 TaskSpace / 同轮 Standard | 1.261x | 1.519x | 1.394x |
| complex | FLA-0 TaskSpace / 同轮 Standard | 1.106x | 1.263x | 1.238x |
| complex | FLA-2 TaskSpace / 同轮 Standard | 1.091x | 1.216x | 1.014x |

请求和 Input 放大在两个样本都缩小；complex 的耗时接近 Standard，simple 的耗时比例反而上升。FLA-2
TaskSpace 相对历史 TaskSpace 基线的绝对 Request 分别为 `29 vs 30`、`36 vs 52`，但同轮 Standard 也发生了
明显波动，因此不能把绝对下降全部归因于 L1/L2 重构。

## 5. 暴露的问题

跨 6 次 TaskSpace 运行，FLA-0 与 FLA-2 的 `taskspace_required_next_call_missing` 都是 11 次；普通工具在 Map
初始化前触发 `no_task_path` 从 6 次变为 7 次。FLA-2 没有证明 Agent 对生命周期组合调用的遵循度提高，说明
仅把方法从 Base 提取到 L2 并不足以解决现有 L4 调用形状问题。

这组数据不代表 FLA-2 验收通过。后续请求级调查证明生产 wire 与可观测性门禁没有完整通过；同时，当前 L2 的拒绝
恢复说明依赖尚未实施的 L5 结果字段。详细根因和口径对账见 `29` 号调查。L4 的结构问题仍应由既定 FLA-4 处理，
不能通过 Runtime 增加语义干预来掩盖。

## 6. 结论

本轮最初候选的完整 wire、L2/L5 一致性和观测门禁失败，因此其历史结论仍为 `acceptance_blocked`。这些 blocker
随后已由 30 号修复结果关闭并通过 current-identity 独立复验，当前 FLA-2 已是 FLA-3 的已验证基线。这里的固定
system 上下文减少约 4.6% 仍只是一项历史局部成本事实，不升级为总体行为收益或统计非劣声明。
