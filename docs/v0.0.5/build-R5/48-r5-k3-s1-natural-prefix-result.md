# R5-K3 S1 自然 Active Prefix 正式结果

- 日期：2026-07-14
- 策略：`S1 = completed_inactive_leaf_batch_archive_projection`
- 样本合同：`benchmarks/taskspace/map-compression/samples/subscription-billing-active-prefix/sample.json`
- 实验合同：`benchmarks/taskspace/map-compression/active-prefix-experiment.json`
- 样本合同 SHA-256：`2965931f04804ddd4be77641550e7cb53b86989aedbb9b0e6fb84b9c76f3c348`
- 候选：`a66fb22`，app-server SHA-256 `b2317a46eeefdb5eb9ff2cd827c69d74b1dd49209ec967a214873f563613c156`
- 匹配前版：`1a02490` + 共享 compaction 修复 `a66fb22`，app-server SHA-256 `57dcada2e57d7126dd41039d958aa64d8fa98be0e3d5835ea51d9d08773d959f`
- 判定：`REJECTED`
- 后续：S1/S2/S3 archive方向均已废弃；替代设计为S4，见`49-r5-k3-s4-distance-fold-design.md`

> 2026-07-14后续决策：本报告继续作为S1历史实验事实保留，但S1不得修订、复用或作为fallback保留。
> S2/S3未实施即废弃。S4不归档节点，只折叠远端节点的局部详情；Agent展开写入canonical事件且不可撤销。
> S4只优化详情成本，不解决Map骨架最终超过上下文限制的问题。

## 1. 判定

S1 在同一 canonical Map snapshot 上把 active projection 从 `1963B` 降到 `856B`，减少 `56.4%`，
可逆 archive 和读取引用均有效。但复杂样本三次运行中，候选相对匹配前版的后续请求中位数从 8 增至 12，
input token 从 80,866 增至 136,247，耗时从 27,720ms 增至 41,848ms。全部运行最终通过 validator，
但没有证明压缩带来工程收益或至少无负收益。

简单样本 Standard、P1、C1 均为 `3/3` solved，且没有 compaction、S1 激活为 0。C1 的请求中位数仍为 8，
P1 和 Standard 均为 6。由于策略未激活，这组差异不能归因为 archive 选择本身；它只说明当前候选没有达到
预登记的简单样本成本非劣门槛，不能拿功能通过代替性能通过。

因此拒绝当前 S1 选择规则。未来若保留 active frontier 附近的 completed 节点详情，只归档更远历史，必须登记为
新的独立策略并重新执行同一套门禁，不能修改样本或把新策略继续记为 S1 成功。

主要原始 artifact：

- 复杂样本合并指标：`target/r5-map-compression/S1-natural-prefix-formal-combined/records.json`
- 复杂样本中位数：`target/r5-map-compression/S1-natural-prefix-formal-combined/summary.json`
- 复杂首轮：`target/r5-map-compression/S1-natural-prefix-formal-smoke-{STD,P1,C}-20260714-*`
- 复杂补充两轮：`target/r5-map-compression/S1-natural-prefix-formal-additional-r2`
- 简单 Standard/C1 三轮：`target/r5-map-compression/S1-simple-STD-C-3x`
- 简单 P1 三个有效 TaskSpace case：`target/r5-map-compression/S1-simple-P1-3x` 的 repeat 1/3，及
  `target/r5-map-compression/S1-simple-P1-extra-1x` 的 repeat 1

每个 live case 均保留 container inspect/stdout/stderr、app-server events、provider wire trace、final rollout、validator
exit code 和 analyzer metrics。简单样本另生成通用 `performance-observation.{json,md}`，未运行侧明确标为 `skipped`，
没有把占位 0 计入中位数。

## 2. 样本客观性

该样本不是按 S1 预期答案构造的状态，而是从早于 fixture 建设的一次真实 Agent 运行中机械截取。截点选择明确
针对 S1 的可观测前提，否则无法测试激活；但没有按后续 Agent 是否成功、动作多少或成本高低筛选轨迹：

| 证据 | 值 |
|---|---|
| source rollout SHA-256 | `aefeadad376524dadc862ba3717c1ae024be853b11d5f838343f6861f785618d` |
| source repo HEAD | `fd24886833f725e072b0410760deed5772b199f2` |
| 截点 | rollout 第 324 行，task context sequence 68 |
| 截点规则 | 第一个已提交 control 结果，Task/Map active，3 completed nodes，`run_tests` 为唯一 open/current node |
| prefix 解压 SHA-256 | `ad588cd40e71511232a81aa1d5627eee758a944a6f172cb287ee78241683c228` |
| continuation prompt | `继续当前工作，完成剩余验证和必要修复。运行测试确认最终结果，并简要报告。` |
| 初始 validator | exit 1，1 failed / 7 passed |
| 最终 validator | 所有 9 个复杂样本 case 均 exit 0 |

反迎合约束实际生效：

1. 前缀中没有未来 pytest 输出、未知 plan 根因或后续修复；
2. 没有 synthetic TaskSpace event、低 token 阈值或 Runtime 激活触发器；
3. prompt 不提 S1、节点数、目标文件、错误类型或预期答案；
4. 工作区按固定 Git commit 和真实的截点前 patch 重建；
5. Standard、P1、C1 共用 prefix、workspace、prompt、模型、容器和 validator；
6. hash、Git tree、RPC、初始失败状态任一不匹配即判 harness invalid。

这保证样本观察的是“Agent 面对真实未完成工作时如何利用不同 projection”，而不是测试脚本先写答案再要求 Agent
复述答案。

## 3. 匹配对照

原始 B0 存在 compaction 本地推理事件误入 canonical history、但未持久化的问题，跨进程恢复会出现 event sequence
gap。直接使用 B0 会把恢复缺陷混入 S1 比较，因此正式实验使用 P1：从 S1 前源码构建，只移植双方共同需要的
compaction persistence 修复。P1 与 C1 的 `compact.rs` 字节一致。

正式三次运行中，P1/C1 生成 projection 前的 canonical snapshot SHA-256 均为：

`67929041350c74da17c12654f742cc252a48ad4c52c831085927128ad4b1cfbc`

这证明 `1963B -> 856B` 来自同一 Map 输入上的 S1 单变量，而不是不同最终 Map 的事后比较。

## 4. 复杂样本结果

以下只统计 continuation epoch；每臂均为 `3/3` validator 通过，RPC error 和 terminal provider error 均为 0。
表格单元格统一为`总和 / 均值 / 中位数`，耗时单位为 ms。

| 指标 | Standard | P1 | C1 |
|---|---:|---:|---:|
| Requests | 28 / 9.33 / 8 | 27 / 9.00 / 8 | 36 / 12.00 / 12 |
| Wall | 168,152 / 56,050.67 / 35,192 | 83,427 / 27,809.00 / 27,720 | 139,565 / 46,521.67 / 41,848 |
| Input tokens | 346,152 / 115,384.00 / 77,486 | 279,525 / 93,175.00 / 80,866 | 412,247 / 137,415.67 / 136,247 |
| Cached input | 321,664 / 107,221.33 / 73,600 | 255,104 / 85,034.67 / 71,424 | 386,816 / 128,938.67 / 126,720 |
| Uncached input | 24,488 / 8,162.67 / 7,752 | 24,421 / 8,140.33 / 9,442 | 25,431 / 8,477.00 / 9,527 |
| Output tokens | 19,219 / 6,406.33 / 3,850 | 8,493 / 2,831.00 / 2,458 | 14,588 / 4,862.67 / 4,590 |
| Commands | 38 / 12.67 / 12 | 32 / 10.67 / 11 | 54 / 18.00 / 16 |
| Failed commands | 4 / 1.33 / 1 | 4 / 1.33 / 1 | 6 / 2.00 / 2 |
| Projection bytes | N/A | 5,889 / 1,963 / 1,963 | 2,568 / 856 / 856 |

命中率不能相加，因此下表的`总计`是`sum(cached) / sum(input)`加权命中率，均值和中位数按三次运行各自的
命中率计算。

| 缓存指标 | 统计口径 | Standard | P1 | C1 |
|---|---|---:|---:|---:|
| Full cache hit | 加权总计 | 92.93% | 91.26% | 93.83% |
| Full cache hit | 算术均值 | 90.67% | 91.61% | 94.01% |
| Full cache hit | 中位数 | 94.25% | 90.64% | 93.01% |

| C1 / P1 | 总和比 | 均值比 | 中位数比 |
|---|---:|---:|---:|
| Requests | `1.33x` | `1.33x` | `1.50x` |
| Wall | `1.67x` | `1.67x` | `1.51x` |
| Input | `1.47x` | `1.47x` | `1.68x` |
| Cached input | `1.52x` | `1.52x` | `1.77x` |
| Uncached input | `1.04x` | `1.04x` | `1.01x` |
| Output | `1.72x` | `1.72x` | `1.87x` |
| Commands | `1.69x` | `1.69x` | `1.45x` |

复杂样本 C1/P1 的加权、均值和中位 cache hit 分别高 `2.57pp`、`2.40pp`、`2.37pp`，但总 input 仍是
`1.47x`。input 增量主要来自更多请求重复携带可缓存前缀，而不是单轮 uncached input 激增；缓存命中率不能替代
总请求和总 token 指标。Standard 有一次 14-request 高值，所以均值明显高于中位数；C1 三次为 13/11/12，
高请求不是由单个离群 case 独立造成。

## 5. 动作归因与限制

不能把 C1 的全部额外成本简单归因给 S1：

1. P1/C1 六次运行中有四次 compactor handoff 只留下待运行 pytest 命令，两次留下较完整工作摘要；summary 本身有
   模型随机性；
2. C1 repeat 1 在首次失败后额外执行 10 个 Git status/log/diff/show 命令，用来重新确认哪些修改已经存在；
3. C1 repeat 2 首次 patch 产生缩进错误，三次 pytest 失败后才修正；这是 Agent patch 质量问题；
4. C1 repeat 3 在 pytest 通过后又运行两段手工业务验证，其中一次 shell quoting 失败；
5. P1 repeat 1 也出现缩进修复和直接覆写文件，说明低级 patch 错误不是 C1 独有。

S1 的结构性变化仍然清楚：三个刚完成、紧邻 current frontier 的节点详情被替换为 archive index 和引用。观察到的
Git 重查和过度验证与证据显著性下降一致，但三次样本不足以把每个动作建立为严格因果。验收不需要反向证明
“所有回归都由 S1 导致”；候选有责任证明无负收益，而当前证据没有做到，因此不能放行。

P1 和 C1 都只有 `1/3` 运行由 Agent 正确闭合 Map；其余运行虽通过外部 validator，仍保留 `run_tests` open。
两臂表现相同，记为既有 terminal lifecycle 观察，不计入 S1 差异。

## 6. 简单样本结果

`single-file-fast-fix` 使用原始场景，不增加压缩触发条件。三臂均 `3/3` solved、patch 计数均为每次 1，
compaction event 均为 0。表格单元格同样为`总和 / 均值 / 中位数`，耗时单位为 ms。

| 指标 | Standard | P1 | C1 |
|---|---:|---:|---:|
| Requests | 17 / 5.67 / 6 | 19 / 6.33 / 6 | 23 / 7.67 / 8 |
| Wall | 53,285 / 17,761.67 / 16,938 | 61,154 / 20,384.67 / 22,163 | 72,303 / 24,101.00 / 23,580 |
| Input tokens | 114,634 / 38,211.33 / 39,777 | 141,397 / 47,132.33 / 46,133 | 178,537 / 59,512.33 / 60,789 |
| Cached input | 109,952 / 36,650.67 / 38,144 | 133,248 / 44,416.00 / 43,520 | 168,960 / 56,320.00 / 58,112 |
| Uncached input | 4,682 / 1,560.67 / 1,633 | 8,149 / 2,716.33 / 2,613 | 9,577 / 3,192.33 / 2,847 |
| Output tokens | 4,740 / 1,580.00 / 1,405 | 5,808 / 1,936.00 / 1,952 | 6,653 / 2,217.67 / 2,167 |
| Runtime tools | 24 / 8.00 / 8 | 25 / 8.33 / 8 | 22 / 7.33 / 8 |
| Failed tools | 0 / 0.00 / 0 | 1 / 0.33 / 0 | 1 / 0.33 / 0 |

| 缓存指标 | 统计口径 | Standard | P1 | C1 |
|---|---|---:|---:|---:|
| Full cache hit | 加权总计 | 95.92% | 94.24% | 94.64% |
| Full cache hit | 算术均值 | 95.91% | 94.20% | 94.66% |
| Full cache hit | 中位数 | 95.89% | 94.34% | 94.77% |
| Request 2+ hit | 加权总计 | 95.53% | 93.69% | 94.24% |
| Request 2+ hit | 算术均值 | 95.53% | 93.61% | 94.26% |
| Request 2+ hit | 中位数 | 95.53% | 93.79% | 94.35% |

简单样本 C1/P1 的 request 总和/均值比为 `1.21x`，中位数比为 `1.33x`；input 总和/均值比为 `1.26x`，
中位数比为 `1.32x`，超过预登记 `1.10x` 非劣门槛。由于 S1 未激活，
该结果不用于解释 archive 语义好坏，只作为候选版本成本门失败和后续重复观察项。C1、P1 都是两次 Map closed、
一次 external solved 但 Map 仍 active，不存在单边新增的闭合回归。

## 7. 测试与缺陷修复

自然前缀首次跨进程恢复暴露了真实 compaction persistence 缺陷：本地 compactor 的 `OutputItemDone` 被写入内存
canonical history，但未持久化；checkpoint 覆盖这些事件后，重启看到 sequence gap。修复后 compactor 输出只作为
本地临时 `ResponseItem`，只有最终 checkpoint 进入 canonical history。

- 修复提交：`a66fb22 fix(taskspace): keep compact inference out of history`
- 新增回归：`taskspace_manual_compact_rollout_resumes_without_event_sequence_gap`
- 相关恢复测试：3 个 focused test 全部通过
- live 复验：sequence 连续 `1..120`，checkpoint 69 覆盖 `1..68`，无 gap

该缺陷修复同时进入 P1/C1，不算作 S1 收益。

## 8. Gate 结论

| Gate | 结果 | 判定 |
|---|---|---|
| 自然 active Map 实际激活 | C1 `3/3` | PASS |
| 同一 canonical snapshot | P1/C1 SHA完全相同 | PASS |
| projection ratio <= 0.90 | `0.4361` | PASS |
| canonical Map / archive可逆 | round-trip与hash验证通过 | PASS |
| complex correctness | STD/P1/C1均`3/3` | PASS |
| simple correctness | STD/P1/C1均`3/3` | PASS |
| simple candidate/previous成本 <= 1.10 | requests `1.33x`，input `1.32x` | FAIL |
| complex实际工程收益 | requests `1.50x`，input `1.68x`，wall `1.51x` | FAIL |

最终判定为 `REJECTED`。当前阶段停止，不实施 S2，不修改 S1 eligibility，不围绕该样本增加提示词或 Runtime
语义干预。
