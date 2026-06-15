# TaskSpace v0.0.4 E3 运行研究报告

日期：2026-06-16

运行根目录：
`D:\whalecode-alpha\target\e3-v004-proof-20260615\serial-clean-v1\suite-20260616-020714`

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

## 2. 执行摘要

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

## 3. 执行效果明细

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

Pair 级明细：

| Task | Pair | 结果 | Standard 耗时 | TaskSpace 耗时 | Standard tokens | TaskSpace tokens | TS token ratio |
|---|---:|---|---:|---:|---:|---:|---:|
| analyze-access-logs | 001 | TaskSpace only | 22.4s | 463.8s | 57,921 | 5,439,623 | 93.91x |
| analyze-access-logs | 002 | Both | 33.2s | 131.1s | 74,865 | 831,299 | 11.10x |
| analyze-access-logs | 003 | Both | 35.3s | 160.6s | 104,901 | 1,280,017 | 12.20x |
| analyze-access-logs | 004 | Both | 33.9s | 192.2s | 422,971 | 4,389,679 | 10.38x |
| analyze-access-logs | 005 | Both | 57.4s | 374.7s | 141,046 | 12,993,596 | 92.12x |
| count-call-stack | 001 | Neither | 57.2s | 151.6s | 144,994 | 1,047,578 | 7.22x |
| count-call-stack | 002 | Neither | 94.4s | 287.8s | 242,183 | 2,804,769 | 11.58x |
| count-call-stack | 003 | Neither | 72.4s | 251.9s | 230,269 | 2,240,984 | 9.73x |
| count-call-stack | 004 | Neither | 101.9s | 237.6s | 314,889 | 2,511,386 | 7.98x |
| count-call-stack | 005 | Neither | 134.7s | 254.8s | 431,759 | 2,552,022 | 5.91x |
| log-summary | 001 | TaskSpace only | 33.3s | 377.8s | 86,133 | 4,152,217 | 48.21x |
| log-summary | 002 | Both | 28.4s | 328.5s | 86,074 | 3,974,568 | 46.18x |
| log-summary | 003 | Standard only | 32.6s | 238.3s | 84,563 | 2,163,497 | 25.58x |
| log-summary | 004 | TaskSpace only | 26.6s | 263.8s | 69,243 | 2,652,195 | 38.30x |
| log-summary | 005 | Standard only | 28.2s | 237.7s | 72,544 | 2,039,857 | 28.12x |

## 4. 耗时分析

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

## 5. Token 成本分析

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

## 6. TaskSpace 行为模式

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

## 7. 失败模式：count-call-stack

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

## 8. 本次运行对 v0.0.4 的含义

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

## 9. 瓶颈假设

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

## 10. 建议

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

## 11. 结论

本次 v0.0.4 E3 运行之所以有价值，是因为它终于是工程 clean 的；但产品信号是混合的：

- 正确性：小幅正向，+1/15。
- 耗时：负向，约 5x agent time。
- Token 成本：强负向，约 20x direct tokens。
- 行为质量：混合偏负，持续存在 unreviewed/blocked graph warnings。

下一阶段不应只追求 solved count 提升。TaskSpace 必须在成本约束下证明价值：更少 unreviewed results、更强 decision adoption、自适应升级/降级，以及明确的 token/runtime budgets。
