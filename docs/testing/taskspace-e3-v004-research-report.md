# TaskSpace v0.0.4 E3 运行研究报告

日期：2026-06-16

运行根目录：
`D:\whalecode-alpha\target\e3-v004-proof-20260615\serial-clean-v1\suite-20260616-020714`

综合结论：本次 v0.0.4 E3 运行的主要价值是终于取得了工程 clean 的可分析样本，证明 v0.0.4 在可审计、可诊断、区分工程污染与 agent 解题失败方面明显前进；但 TaskSpace 的产品收益仍未成立。原始结果只比 Standard 多解决 1/15 个 pair，却消耗约 5x agent time 和约 20x direct input+output tokens，并持续暴露 `high_unreviewed_result_ratio`、`high_blocked_node_ratio`、subagent 采纳不足等行为质量问题。下一阶段重点不应继续堆结构，而应转向 decision adoption、TaskSpace budget guardrails、task-shape routing，以及明确的 token/runtime budget 约束。

## 1. 范围与证据

本报告分析 v0.0.4 在 Terminal-Bench 校准任务上的一次工程 clean 的 E3 串行运行。

- 任务：`analyze-access-logs`、`log-summary`、`count-call-stack`
- 每个任务重复次数：5
- 对比模式：`standard` vs `taskspace`
- Pair 总数：15
- 工程 clean 状态：本次运行 clean
  - `invalid_harness_sample_count=0`
  - `signature_count=0`
  - `engineering_unclean_pairs=0`
  - `suite_score_valid=true`

重要 caveat：

- 本次运行在 sample 层仍处于 `audit_required`，也就是 E3 人工审计模板尚未完成。下面的 solved/wrong 是来自 `metrics.json` 和 `audit.json` 的 public validator 原始结果，不是经过人工审计后的最终 benchmark claim。

## 2. v0.0.4 版本目标回顾与影响分析

v0.0.4 的原始定位不是“让 TaskSpace 立刻全面超过 Standard”，而是把 TaskSpace 从可运行的 task/map/node 结构，升级成可审计的问题状态管理 runtime。也就是说，0.0.4 首先要证明结构化活动能稳定沉淀目标、事实、问题、决策、结果采纳和审计证据；只有在工程证据 clean 的前提下，才能继续判断它是否改善了 0.0.3 暴露的 utility 问题。

因此本次评估需要分三层看：

- 工程机制层：v0.0.4 的 ledger、adoption、graph health、audit、gate 是否真的落到 runtime/harness。
- 执行有效性层：E3 是否能排除 Docker、validator、path、disk、report、audit 等工程污染。
- 产品收益层：TaskSpace 是否以可接受成本带来更高 solved rate、更好决策质量或更强 subagent ROI。

### 2.1 原始 v0.0.4 目标与对应工程

| 版本目标 | 对应工程 | 本次运行中的具体作用和影响 | 判断 |
|---|---|---|---|
| 引入 `ProblemStateLedgerV1`，复用现有 `action_map` runtime，不新建并行 TaskSpace 存储 | `third_party/codex-cli/codex-rs/core/src/action_map/ledger.rs`、`map.rs` 的 `TaskState.problem_ledger`、`runtime.rs` 的 snapshot/restore/context 注入、`protocol/src/protocol.rs` 的 `problem_ledger` snapshot 字段 | 任务目标、成功标准、开放问题、决策和下一步动作进入 first-class state，不再只埋在自然语言 result 里。它让 final synthesis/readiness 可以被结构化检查，也让 graph health 能读取问题状态。但这也增加了每轮上下文负载，是 TaskSpace 高 token 成本的可能贡献因素之一。 | 机制已落地；收益未单独证明。 |
| 扩展 `taskspace_control` 为 v2 action set，同时兼容 v1 | `tools/src/taskspace_tool.rs` schema、`core/src/tools/handlers/taskspace_control.rs`、`session/mod.rs`，新增/扩展 `initial_success_criteria`、`record_success_criteria`、`record_open_question`、`record_decision`、`record_next_best_action`、`adopt_result` 等动作 | 模型获得了显式记录问题状态和决策依赖的工具入口。本次 E3 能产出更丰富的 audit/graph 信号，但 15/15 TaskSpace run 仍出现 `high_unreviewed_result_ratio`，说明工具可用不等于模型稳定使用；v2 schema 改善了可审计性，还没有自动转化为更高决策质量。 | 工程达成；行为采纳不足。 |
| 引入 `ResultAdoptionV1`，把 result validity 升级为“是否进入问题模型或决策” | `core/src/action_map/cognitive.rs` 的 `NodeResultAdoption` / `ResultAdoptionState`、`runtime.rs` 的 `adopt_result_for_main` 和 `record_decision` 依赖校验、`graph-health.ps1` 的 adoption 指标 | 这项工程让“subagent/result 产生了但没被用”变成可测问题。本次运行暴露 `subagent_no_decision_yield=7/15`、`high_unreviewed_result_ratio=15/15`，说明 0.0.4 确实看见了 adoption 瓶颈；但它没有解决该瓶颈，尤其 `log-summary` 大量 subagent 输出没有稳定转成净收益。 | 可观测性达成；质量目标未达成。 |
| 引入 `GraphHealthReportOnly`，先报告不阻断 | `scripts/taskspace-benchmark/lib/graph-health.ps1`、`metrics-extractor.ps1` 写出 `graph-health.json`、`aggregate-report.ps1` 聚合 `graph-health-summary.json` | 本次报告能量化 `high_unreviewed_result_ratio`、`high_blocked_node_ratio`、`low_decision_density`、`synthesis_not_ready`，正是该目标的直接产物。Report-only 的副作用是：即使 graph health 很差，run 仍会继续消耗时间和 token，所以它适合诊断，不适合成本控制。 | 达成诊断目标；尚未形成运行期节流。 |
| 引入 `CleanE3AuditManifest` 和 `FailureTaxonomyV1`，修正 E3 证据链 | `scripts/taskspace-benchmark/lib/audit-manifest.ps1`、`failure-taxonomy.ps1`、`audit-report.ps1`、`aggregate-report.ps1`，每个 pair 输出 `audit.json/yaml`、failure taxonomy、included/excluded/inconclusive、`score_valid`/`engineering_unclean` | 这是本次能称为“工程 clean run”的核心前提：`invalid_harness_sample_count=0`、`signature_count=0`、`engineering_unclean_pairs=0`、`suite_score_valid=true`。它也明确保留 `audit_required` caveat，避免把 public validator 原始结果包装成最终 benchmark claim。 | 工程有效性显著改善；人工审计闭环仍未完成。 |
| 只对高确定性结构错误加 hard gate | `runtime.rs` 中 success criteria、invalid/questioned result、final_synthesis readiness、validate-after-edit、spawn plan 等检查；`basemap.rs` 中 node kind 和 final synthesis 协议提示 | gate 防止明显不安全的结构路径进入最终回答，并保持 final answer 不暴露内部 TaskSpace 术语。本次 E3 没有因为 gate 产生工程 unclean，但 `count-call-stack` 0/5 说明结构 gate 不等于任务语义成功；它主要防止坏证据进入最终决策，不能替代 format/validator 导向的解题策略。 | 安全边界达成；正确率收益有限。 |
| 保持 TaskSpace 对用户不可见，viewer 作为调试面而非用户前置操作 | `basemap.rs` 的 final answer 语言约束、viewer/report 侧 graph health/adoption 展示、E3 artifact 包 | 对 benchmark 来说，TaskSpace 仍作为内部 orchestration 运行，任务输入没有要求用户理解 map/node/subagent。影响是正面的：用户可见行为没有被 TaskSpace 概念污染；但 debug/研究仍依赖 artifact 和报告。 | 用户边界基本达成。 |

### 2.2 执行中追加的 E3 guardrails 目标

0.0.4 原始 Phase 6 曾暴露 uv cache 相对路径、Docker/WSL、磁盘空间、suite 状态传播等工程问题。后续 guardrails 升级不是原始产品机制目标，但已经成为判断 v0.0.4 是否可信的硬前置。

| 追加目标 | 对应工程 | 本次运行中的具体作用和影响 | 判断 |
|---|---|---|---|
| 硬性执行有效性契约：除 agent 执行超时外，其他 Docker、validator、path、disk、materialization、report/audit 异常都使本次执行无效 | `suite-status.ps1`、`run-state.ps1`、`aggregate-report.ps1`、`test-e3-score-validity.ps1`、`test-e3-harness-guardrails.ps1` | 这直接解决了之前“docker/validator fail 还被拿来算成绩”的问题。本次 suite health 明确 `suite_score_valid=true`，且没有工程异常签名，所以 public validator 结果至少不再被场外因素污染。 | 达成，且是本次报告可信的前提。 |
| 提前发现工程异常，避免数小时无效 E3 | `harness-health.ps1`、`e3-start-gate.ps1`、`calibration-gate.ps1`、disk preflight、workspace path budget、validator probe、child invalid circuit breaker | D 盘空间、相对 uv cache、深路径写入、child status 传播等问题都被前移成 preflight/probe/start-gate/circuit-breaker 检查。本次没有触发早停，说明 run 通过了这些护栏；但这些护栏的价值主要是让坏 run 便宜，而不是让 clean run 更快。 | 工程护栏有效；不等同于性能优化。 |
| 让 E3 runtime 自解释，定位 15-task 为什么要跑数小时 | `timing.ps1`、`runtime-bottleneck-report.ps1`、`runtime-reconstruction.ps1`、`resource-governor.ps1`、`parallel-diff.ps1`、`calibration-selection.ps1` | 本次能拆出 suite 级 `agent_duration_ms=4744306`、`public_validation_duration_ms=3061684`、`docker_run_duration_ms=2727821`，并给出 `validator_bound` 分类。但 `model_request_duration_ms` 缺失，`runtime_optimization_status=blocked`，所以不能宣称已完成速度优化或能稳定大幅提速。 | 诊断框架达成；速度结论仍受 instrumentation 限制。 |
| 保持 v0.0.4 comparable profile，不用改 timeout/model/budget 来制造速度或分数改善 | `calibration-selection.json`、profile hash、task list hash、source version、start gate identity checks、parallel equivalence gate | 本次报告可以把“TaskSpace agent 时间 4.99x、direct tokens 19.92x”作为同 profile 下的成本信号，而不是另一个实验配置的结果。这也意味着如果要通过缩短 TaskSpace 预算、降低 reasoning、减少上下文等方式提速，必须另起 profile/version，不能算 v0.0.4 同口径提升。 | 可比性边界达成。 |

### 2.3 目标达成总评

v0.0.4 在“工程机制”和“证据链清洁度”上比 0.0.3 明显前进：它能产出 ledger/adoption/graph health/audit/timing 证据，也能把工程 unclean 和 agent 解题失败区分开。本次运行的最大价值就是终于让 E3 结果不再被 Docker、validator 或路径错误污染。

但 v0.0.4 的产品收益没有成立：

- 正确率只从 Standard 的 7/15 到 TaskSpace 的 8/15，净增 1 个 solved pair。
- TaskSpace agent 耗时是 Standard 的 4.99x。
- TaskSpace direct input+output tokens 是 Standard 的 19.92x。
- graph health 显示所有 TaskSpace run 都有高比例 unreviewed results，subagent 结果采纳也不稳定。
- `count-call-stack` 双方都是 0/5，TaskSpace 消耗更多但没有带来新的解题路径。

因此，最准确的版本结论是：v0.0.4 大体完成了“可审计、可诊断、可区分工程污染”的工程目标，但还没有完成“以可接受成本稳定提升 agent 能力”的产品目标。下一阶段应该把重点从继续堆结构，转向 decision adoption、TaskSpace budget guardrails、task-shape routing 和 token/runtime budget 约束。

## 3. 执行摘要

TaskSpace 带来了很小的原始正确率提升，但耗时和 token 成本非常高。

| 指标 | Standard | TaskSpace | TaskSpace / Standard |
|---|---:|---:|---:|
| 解决 pair 数 | 7/15 | 8/15 | +1 solved pair |
| Agent 耗时 | 791.9s | 3952.2s | 4.99x |
| Public validation 耗时 | 1623.8s | 1437.8s | 0.89x |
| Docker run 耗时 | 1441.1s | 1286.4s | 0.89x |
| 直接 input+output tokens | 2,564,355 | 51,073,287 | 19.92x |
| 非缓存 input tokens | 185,043 | 660,749 | 3.57x |
| Output tokens | 39,728 | 293,242 | 7.38x |
| Tool calls | 103 | 124 | 1.20x |

主要结论：

- TaskSpace 目前还不是广义上更好的模式。它在部分日志处理任务上有选择性收益，在 `log-summary` 上整体持平，在 `count-call-stack` 上无效。
- 成本结构主要由大量 cached input 复用驱动，不只是可见的 subagent JSONL 额外开销。
- 运行耗时瓶颈由 TaskSpace agent 耗时和 public validation 共同构成。suite 级 timing 把整体分类为 `validator_bound`，但模式级对比显示，TaskSpace agent 执行耗时才是 Standard 和 TaskSpace 之间最大的差异来源。
- TaskSpace 图结构质量信号偏弱：15/15 个 TaskSpace run 都出现 `high_unreviewed_result_ratio`，13/15 出现 `high_blocked_node_ratio`。

## 4. 执行效果明细

| Task | Standard | TaskSpace | 解读 |
|---|---:|---:|---|
| analyze-access-logs | 4/5 | 5/5 | TaskSpace 明确多赢 1 个 pair。 |
| count-call-stack | 0/5 | 0/5 | 两种模式都稳定失败。TaskSpace 花费更多但没有解决。 |
| log-summary | 3/5 | 3/5 | 总分相同，但 pair 级胜负不同。 |
| Total | 7/15 | 8/15 | 净收益为 +1 solved pair。 |

Pair 级结果分类：

| 分类 | 数量 |
|---|---:|
| 双方都解决 | 5 |
| 只有 TaskSpace 解决 | 3 |
| 只有 Standard 解决 | 2 |
| 双方都未解决 | 5 |

任务级效果与成本汇总：

口径说明：
- `agent` 是 agent 执行墙钟时间，不含 public validator。
- `validation` 是 public validator 时间。
- `tokens` 是 direct `whale-exec.jsonl` 的 `input+output`，括号内是 `uncached input`。该表不含嵌套 subagent JSONL 的额外 token。

| Task | Standard solved | TaskSpace solved | Standard agent | TaskSpace agent | Standard validation | TaskSpace validation | Standard tokens | TaskSpace tokens | TS/Std tokens |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| analyze-access-logs | 4/5 | 5/5 | 182.2s | 1322.4s | 659.4s | 636.2s | 801,704 (98,887) | 24,934,214 (318,682) | 31.10x |
| count-call-stack | 0/5 | 0/5 | 460.7s | 1183.7s | 503.7s | 366.5s | 1,364,094 (78,080) | 11,156,739 (189,392) | 8.18x |
| log-summary | 3/5 | 3/5 | 149.1s | 1446.2s | 460.9s | 435.1s | 398,557 (8,076) | 14,982,334 (152,675) | 37.59x |

Pair 级详细对比：

| Task | Pair | Standard 结果 | TaskSpace 结果 | Pair 结果 | Standard agent | TaskSpace agent | Standard validation | TaskSpace validation | Standard tokens | TaskSpace tokens | TS/Std tokens | Pair total |
|---|---:|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| analyze-access-logs | 001 | 失败 | 通过 | 仅 TaskSpace | 22.4s | 463.8s | 76.2s | 66.6s | 57,921 (2,033) | 5,439,623 (102,627) | 93.91x | 664.0s |
| analyze-access-logs | 002 | 通过 | 通过 | 双方通过 | 33.2s | 131.1s | 60.9s | 99.7s | 74,865 (1,981) | 831,299 (6,846) | 11.10x | 348.1s |
| analyze-access-logs | 003 | 通过 | 通过 | 双方通过 | 35.3s | 160.6s | 239.0s | 83.8s | 104,901 (2,524) | 1,280,017 (8,094) | 12.20x | 542.1s |
| analyze-access-logs | 004 | 通过 | 通过 | 双方通过 | 33.9s | 192.2s | 89.7s | 288.0s | 422,971 (89,324) | 4,389,679 (96,094) | 10.38x | 627.2s |
| analyze-access-logs | 005 | 通过 | 通过 | 双方通过 | 57.4s | 374.7s | 193.5s | 98.1s | 141,046 (3,025) | 12,993,596 (105,021) | 92.12x | 775.3s |
| count-call-stack | 001 | 失败 | 失败 | 双方失败 | 57.2s | 151.6s | 99.2s | 43.3s | 144,994 (9,019) | 1,047,578 (58,803) | 7.22x | 377.1s |
| count-call-stack | 002 | 失败 | 失败 | 双方失败 | 94.4s | 287.8s | 86.7s | 99.2s | 242,183 (17,546) | 2,804,769 (28,450) | 11.58x | 592.6s |
| count-call-stack | 003 | 失败 | 失败 | 双方失败 | 72.4s | 251.9s | 157.5s | 72.0s | 230,269 (13,811) | 2,240,984 (23,364) | 9.73x | 589.8s |
| count-call-stack | 004 | 失败 | 失败 | 双方失败 | 101.9s | 237.6s | 40.6s | 102.3s | 314,889 (19,098) | 2,511,386 (17,092) | 7.98x | 505.7s |
| count-call-stack | 005 | 失败 | 失败 | 双方失败 | 134.7s | 254.8s | 119.5s | 49.8s | 431,759 (18,606) | 2,552,022 (61,683) | 5.91x | 583.0s |
| log-summary | 001 | 失败 | 通过 | 仅 TaskSpace | 33.3s | 377.9s | 134.9s | 81.6s | 86,133 (1,703) | 4,152,217 (93,238) | 48.21x | 665.3s |
| log-summary | 002 | 通过 | 通过 | 双方通过 | 28.4s | 328.5s | 79.3s | 87.1s | 86,074 (1,636) | 3,974,568 (17,962) | 46.18x | 550.0s |
| log-summary | 003 | 通过 | 失败 | 仅 Standard | 32.6s | 238.3s | 99.1s | 63.4s | 84,563 (1,685) | 2,163,497 (13,884) | 25.58x | 458.5s |
| log-summary | 004 | 失败 | 通过 | 仅 TaskSpace | 26.6s | 263.8s | 83.6s | 107.4s | 69,243 (1,213) | 2,652,195 (14,371) | 38.30x | 543.3s |
| log-summary | 005 | 通过 | 失败 | 仅 Standard | 28.2s | 237.7s | 64.1s | 95.5s | 72,544 (1,839) | 2,039,857 (13,220) | 28.12x | 454.2s |

## 5. 耗时分析

Suite 级 timing：

- Total pair duration：8276212ms，约 137.9 分钟
- Agent execution：4744306ms，约 79.1 分钟
- Public validation：3061684ms，约 51.0 分钟
- Docker run：2727821ms，约 45.5 分钟
- Suite bottleneck classification：`validator_bound`

模式级 timing 给出的结论更尖锐：

- Standard agent 总耗时：13.2 分钟
- TaskSpace agent 总耗时：65.9 分钟
- TaskSpace 额外消耗约 52.7 分钟 agent runtime，但净收益只有 +1 solved pair。
- Public validation 和 Docker 是很大的绝对成本，但不能解释 TaskSpace 和 Standard 的差距。本次运行里 TaskSpace 的 validation 耗时甚至略低于 Standard。

按任务的平均 agent 耗时：

| Task | Standard avg | TaskSpace avg | Ratio |
|---|---:|---:|---:|
| analyze-access-logs | 36.4s | 264.5s | 7.27x |
| count-call-stack | 92.1s | 236.7s | 2.57x |
| log-summary | 29.8s | 289.2s | 9.70x |

解读：

- TaskSpace 有较高的固定 orchestration/context 开销。对于 Standard 能在 60 秒内解决的小任务，这个开销尤其昂贵。
- `count-call-stack` 说明更长运行时间不必然带来更强推理。TaskSpace 平均耗时是 Standard 的 2.6x，但 5 次全部失败。

## 6. Token 成本分析

直接模式用量，不包含嵌套 subagent JSONL 文件：

| Mode | Input | Cached input | Uncached input | Output | Reasoning output | Input+Output |
|---|---:|---:|---:|---:|---:|---:|
| Standard | 2,524,627 | 2,339,584 | 185,043 | 39,728 | 16,748 | 2,564,355 |
| TaskSpace | 50,780,045 | 50,119,296 | 660,749 | 293,242 | 82,420 | 51,073,287 |

所有 JSONL 用量，包含 TaskSpace 嵌套 subagent JSONL 文件：

| 范围 | Files | Input | Cached input | Uncached input | Output | Reasoning output | Input+Output |
|---|---:|---:|---:|---:|---:|---:|---:|
| Direct only | 30 | 53,304,672 | 52,458,880 | 845,792 | 332,970 | 99,168 | 53,637,642 |
| All JSONL | 45 | 53,982,666 | 52,920,192 | 1,062,474 | 341,964 | 100,764 | 54,324,630 |
| Nested extra | 15 | 677,994 | 461,312 | 216,682 | 8,994 | 1,596 | 686,988 |

关键观察：

- TaskSpace direct input 是 Standard 的 20.1x。
- TaskSpace direct input+output 是 Standard 的 19.9x。
- TaskSpace uncached input 是 Standard 的 3.6x，明显低于总 token 倍数，因为本次运行大量受益于 prompt cache。
- TaskSpace cached input rate 约 98.7%；Standard cached input rate 约 92.7%。
- 独立的嵌套 subagent JSONL 只额外增加约 0.69M tokens。这意味着 TaskSpace 的大部分 token 成本不在 standalone subagent 文件里，而在主 TaskSpace 执行上下文里。更可能的原因是 graph state、subagent summary、result、累积 context 被反复带入模型调用。

最大 token 异常点：

| Task | Pair | Mode | Outcome | Tokens | Agent time |
|---|---:|---|---|---:|---:|
| analyze-access-logs | 005 | TaskSpace | solved | 12,993,596 | 374.7s |
| analyze-access-logs | 001 | TaskSpace | solved | 5,439,623 | 463.8s |
| analyze-access-logs | 004 | TaskSpace | solved | 4,389,679 | 192.2s |
| log-summary | 001 | TaskSpace | solved | 4,152,217 | 377.8s |
| log-summary | 002 | TaskSpace | solved | 3,974,568 | 328.5s |

Outcome 与成本关系：

| Mode | Outcome | Runs | Avg agent time | Avg tokens | Avg tool calls |
|---|---|---:|---:|---:|---:|
| Standard | solved | 7 | 35.6s | 140,995 | 5.1 |
| Standard | wrong | 8 | 67.9s | 197,174 | 8.4 |
| TaskSpace | solved | 8 | 286.6s | 4,464,149 | 6.4 |
| TaskSpace | wrong | 7 | 237.1s | 2,194,299 | 10.4 |

这说明当前 TaskSpace 的成功更像是和更多 context、更多时间投入相关，而不是来自更精简、更高效的推理路径。

## 7. TaskSpace 行为模式

TaskSpace graph warnings：

| Warning | Count |
|---|---:|
| high_unreviewed_result_ratio | 15/15 |
| high_blocked_node_ratio | 13/15 |
| subagent_no_decision_yield | 7/15 |
| low_decision_density | 2/15 |
| synthesis_not_ready | 2/15 |

Subagent 使用情况：

| Task | Spawn agent calls | Subagent results | TaskSpace solved |
|---|---:|---:|---:|
| analyze-access-logs | 4 | 12 | 5/5 |
| count-call-stack | 0 | 0 | 0/5 |
| log-summary | 11 | 42 | 3/5 |

行为解读：

- 即使没有 spawn subagent，TaskSpace 也会使用大量结构化状态和上下文。
- 在 `count-call-stack` 上，尽管任务稳定失败，TaskSpace 没有 spawn subagent。它主要表现为一个更重的单 agent workflow，tool calls 更多，但没有形成新的解题路径。
- 在 `log-summary` 上，TaskSpace spawn 了很多 subagent，但总分只和 Standard 持平。graph 多次出现 `subagent_no_decision_yield`，说明 subagent 产物没有稳定转化为被采纳的决策。
- 在 `analyze-access-logs` 上，TaskSpace 确实带来了收益。它 5/5 解决，比 Standard 多赢一个 pair。但成本极高：5 个 TaskSpace run 直接消耗 24.9M tokens，而 Standard 只消耗 0.8M tokens。

当前 TaskSpace 最明显的模式是：昂贵的广度探索加上下文累积。它在冗余检查和综合能提高可靠性的任务上可能有帮助，但还不能稳定地把广度转化为决策，也不能识别广度不再产生收益的时刻。

## 8. 失败模式：count-call-stack

`count-call-stack` 的 5 次重复中，两种模式全部失败。这不是工程 clean 问题：

- Public validators 都完成了。
- 失败是正常 validator assertion failure，主要是 `FAILED ../tests/test_outputs.py::test_count_output`。
- 所有行都是 `engineering_unclean=false`。

观察到的行为：

- 两种模式都写了 `output.txt`，并且经常解压或解析 `log.stack`。
- Standard 在部分 repeat 中改写了解析辅助脚本。
- TaskSpace 在部分 repeat 中也创建了 helper script，但没有 spawn subagent。
- 在这个任务上，TaskSpace tool calls 多于 Standard：63 vs 55。
- TaskSpace 消耗约 11.16M tokens，Standard 消耗约 1.36M tokens，但双方都是 0/5。

可能原因：

- 这个任务对格式和 validator 预期非常敏感。agent 生成了看似合理的 stack analysis 输出，但没有满足精确的 expected structure/content。
- TaskSpace 没有引入明显不同的 verification 或 format-diff 策略，而是放大了同类本地探索循环。
- 因为没有 spawn subagent，最需要替代假设的任务反而没有真正启用 multi-agent primitive。

## 9. 本次运行对 v0.0.4 的含义

正向信号：

- 本次运行的工程 harness 是 clean 的。
- TaskSpace 在部分日志处理任务上确实能提升可靠性。
- Prompt caching 让巨大的上下文成本没有 raw token 总量看起来那么灾难性。

负向信号：

- 净效果提升只有 +1/15。
- TaskSpace runtime 约为 Standard agent runtime 的 5x。
- TaskSpace direct token cost 约为 Standard 的 20x。
- Graph hygiene 较差：所有 TaskSpace run 都有 high unreviewed result ratio。
- Subagent output adoption 较弱。多个 run 中存在 subagent，但没有转化为 decision yield。
- TaskSpace 还缺少良好的“停止继续花费”的机制，不能在路径无收益时及时降级或停止。

最准确的描述是：

> v0.0.4 TaskSpace 展现了 correctness signal，但成本效率不成立。它更像一个高上下文 orchestration 系统，偶尔带来可靠性收益，而不是一个稳定更强的 coding agent 模式。

## 10. 瓶颈假设

H1：Context bloat 是主要 token 瓶颈。

- 证据：TaskSpace direct input+output 为 51.1M tokens，是 Standard 的 19.9x。
- 证据：TaskSpace usage 中 cached input 占约 98.7%。
- 推断：即使任务本身很小，graph/context/subagent state 也被反复纳入模型调用。

H2：Decision adoption 是主要质量瓶颈。

- 证据：`high_unreviewed_result_ratio` 出现在 15/15 个 TaskSpace run。
- 证据：`subagent_no_decision_yield` 出现在 7/15 个 run。
- 推断：TaskSpace 产生中间结果的速度超过了验证和采纳这些结果的能力。

H3：Task routing 过粗。

- 证据：`count-call-stack` 没有使用 subagent，且 0/5。
- 证据：`log-summary` 使用了很多 subagent，但只和 Standard 持平。
- 推断：TaskSpace 还不能根据任务形态和已观察到的失败模式决定是否使用 subagent、何时停止或何时切换策略。

H4：Validation 仍是 suite 级速度瓶颈。

- 证据：public validation 消耗约 51 分钟；Docker run 消耗约 45.5 分钟。
- 证据：validation 时间对两种模式都很大，所以即使 agent 侧优化完成，它仍会限制 E3 迭代速度。

## 11. 建议

1. 增加一等公民级 token summary artifact。

- 在 pair、sample、suite 层持久化 `token-summary.json`。
- 区分 direct agent usage 和 nested subagent usage。
- 跟踪 input、cached input、uncached input、output、reasoning output 和 cost estimate。
- 把 top-token outliers 写入 `suite-health.json` 或 companion report。

2. 增加 TaskSpace budget guardrails。

- 当 token ratio 超阈值但没有新增 accepted decisions 时，中断或降级 TaskSpace。
- 建议初始阈值：
  - `taskspace_total_tokens > 10x standard_tokens` 且 N 步内没有新增 accepted decision。
  - 首次 synthesis checkpoint 后同时出现 `high_unreviewed_result_ratio` 和 `subagent_no_decision_yield`。
  - 接近 final answer 时出现 `synthesis_not_ready`，即使 validator 后续通过，也应标记为诊断可疑。

3. 改进 decision adoption。

- 要求每个 subagent result 被 accepted、rejected 或 explicitly deferred。
- 跟踪“每 1M tokens 采纳的证据数”和“每个 subagent result 对应的 accepted result 数”。
- 惩罚不产生决策的 graph growth。

4. 增加 task-shape routing。

- 对小型 deterministic file task，先走轻量 Standard-like 路径，只在 validator failure 或 ambiguity 时升级。
- 对 `count-call-stack` 这类 parser/format-sensitive 任务，路由到 verification-first workflow：
  - 读取 expected output format。
  - 生成小型 parser。
  - 在 final 前自运行 output checks。
  - 对比 validator failure text 和实际输出。

5. 优化 validation runtime。

- 尽可能缓存或预构建 validator Python/uv 环境。
- 继续推进 Docker image cache，但要注意 Docker build 总共只有约 1 分 24 秒；Docker run 和 test environment setup 才是主因。
- 保持足够高的 validation timeout 以避免 invalid run，同时增加 preflight timing probe 来识别 validator setup 天生较慢的任务。

6. 给 E3 reporting 增加 TaskSpace value gate。

- 汇报 solved delta 时必须同时汇报 cost delta：
  - `extra_solved_pairs`
  - `extra_agent_minutes`
  - `extra_uncached_tokens`
  - `extra_total_tokens`
  - `extra_cost_per_additional_solved_pair`
- 对本次运行，TaskSpace 多解决 1 个 pair，但额外消耗约 52.7 agent minutes 和 48.5M direct input+output tokens。

## 12. 结论

本次 v0.0.4 E3 运行之所以有价值，是因为它终于是工程 clean 的；但产品信号是混合的：

- 正确性：小幅正向，+1/15。
- 耗时：负向，约 5x agent time。
- Token 成本：强负向，约 20x direct tokens。
- 行为质量：混合偏负，持续存在 unreviewed/blocked graph warnings。

下一阶段不应只追求 solved count 提升。TaskSpace 必须在成本约束下证明价值：更少 unreviewed results、更强 decision adoption、自适应升级/降级，以及明确的 token/runtime budgets。
