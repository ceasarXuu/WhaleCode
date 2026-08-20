# Base 状态机显著性五轮结果

- Date: 2026-08-17
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Repeat: 5 个独立 `repeat=1`，并行执行
- Subject commit: `a8fd76e54`
- Binary SHA-256: `150f99047b860fef049db8858c6e13e74c91c824c808e2ec9fe7399fb8eeb1b8`
- Ledger: `WAR-20260817-031407-BASE-LIFECYCLE-R5`
- Baseline: [`72-state-machine-protocol-repeat5-result.md`](72-state-machine-protocol-repeat5-result.md)

## 1. 唯一变量与生效证明

相对基线，本轮只把简洁的节点状态机工作模型加入 TaskSpace Base instructions。Runtime、`taskspace_exec`
description/schema、合法序列、反馈、Map projection、普通 Tool 和 Standard Base 均未改变。

五轮 final-wire trace 都只识别到一份 TaskSpace Base `3.0.4`：

- SHA-256：`a783705f320504306fc9fca591cb1b15246b73482201a916b511f8d5cc49ec33`；
- `matches_current_contract=true`；
- wire bytes 固定为 `20,019`。

Rollout `session_meta.base_instructions` 仍保存会话创建时的 Standard 初始快照，不能用来判断实际 Provider
instructions；本轮以 final-wire payload scanner 为权威。

## 2. 逐轮结果

| Run | 业务/Map | Requests | Input | Cached | Uncached | Output | Agent wall | Map | Waiting 误选 | 其他拒绝 |
|---:|---|---:|---:|---:|---:|---:|---:|---|---:|---|
| 1 | passed | 6 | 89,982 | 82,304 | 7,678 | 1,848 | 16.441s | 4 nodes / 3 edges | 0 | none |
| 2 | passed | 8 | 128,598 | 107,008 | 21,590 | 2,442 | 22.001s | 5 nodes / 4 edges | 0 | JSON syntax 1 |
| 3 | passed | 7 | 105,102 | 85,248 | 19,854 | 1,902 | 18.385s | 4 nodes / 3 edges | 0 | none |
| 4 | passed | 7 | 111,419 | 104,320 | 7,099 | 2,683 | 22.500s | 5 nodes / 4 edges | 0 | none |
| 5 | passed | 7 | 109,803 | 87,552 | 22,251 | 2,218 | 20.246s | 5 nodes / 4 edges | 0 | none |
| **Total** | **5/5** | **35** | **544,904** | **466,432** | **78,472** | **11,093** | **99.573s** | - | **0/5** | **1** |
| **Mean** | - | **7.0** | **108,980.8** | **93,286.4** | **15,694.4** | **2,218.6** | **19.915s** | - | - | - |
| **Median** | - | **7** | **109,803** | **87,552** | **19,854** | **2,218** | **20.246s** | - | - | - |

全量缓存命中率为 `85.60%`；Request 2+ 加权命中率为 `91.21%`。按冻结价格估算费用为
`0.10998664 CNY`。

五个 runner 都因只执行 right side 而以 pair 评分退出码 `1` 结束；side artifacts 证明五轮业务验证、公开测试、
隐藏 oracle、Agent completion 和 Map 闭合全部通过，因此不能把该 harness 退出码解释为 Agent 失败。

## 3. 状态机行为

- Run 2、4、5 使用 `root -> inspect -> fix -> verify -> finish`。三轮都先完成 `fix`，再在同一合法
  `update_and_work` 序列中执行 `verify`，没有直接选择 Waiting frontier。
- Run 1、3 使用 `root -> inspect -> fix -> finish`，节点 goal 明确包含“修复并跑测试确认”，测试在 `fix`
  中完成后再关闭 Map。这是自洽的任务分解，不是跳过依赖或提前结束。
- 五轮没有 `TransitionInvalid`，没有对 Waiting owner 的独立 `work`。
- Run 2 有一次 malformed JSON；Runtime 没有执行对应 Map 或 Tool 动作，Agent 下一请求正确重发。该异常没有
  状态机因果证据。

## 4. 与前一基线比较

| 指标 | Tool description 基线 | Base 候选 | 变化 |
|---|---:|---:|---:|
| 业务/Map 通过 | 5/5 | 5/5 | 持平 |
| Waiting 误选 | 2/5 | 0/5 | -2 runs |
| TransitionInvalid | 3 | 0 | -3 |
| JSON syntax rejects | 2 | 1 | -1，非目标指标 |
| Requests | 41 | 35 | -14.63% |
| Input | 639,497 | 544,904 | -14.79% |
| Uncached input | 112,009 | 78,472 | -29.94% |
| Output | 11,932 | 11,093 | -7.03% |
| Agent wall | 116.676s | 99.573s | -14.66% |
| Request 2+ cache | 87.46% | 91.21% | +3.75 pp |
| 估算费用 | 0.14642276 CNY | 0.10998664 CNY | -24.88% |

## 5. 结论边界

本轮支持 H-005：Agent 需要在选择动作前从 Base 获得稳定的依赖状态机工作模型；只把完整硬合同放在 Tool
description 中，信息虽完整但显著性不足。候选在当前简单样本上同时保持正确性并消除已观测 Waiting 误选，且没有成本回归。

这仍然是单一样本的五轮证据，不证明所有复杂 DAG 都已稳定，也不能把所有成本下降都归因于 Base。当前可以保留
Base `3.0.4`，后续在自然出现复杂依赖样本时继续观察，不为扩大结论立即追加付费运行。

本批使用 TaskSpace 行为 benchmark runner，而不是缓存基线专用 promotion runner。缓存数据可以作为观测证据，
但不能据此晋升受保护的缓存基线；该门禁仍保持独立阻断，直到另行授权并执行专用缓存回归。

## 6. 证据

- `target/r8-base-lifecycle/repeat5-{1..5}/single-file-fast-fix/20260817-031641-*`
- Provider 前凭据缺失的零请求尝试：`target/r8-base-lifecycle/repeat5-{1..5}/single-file-fast-fix/20260817-031612-*`
