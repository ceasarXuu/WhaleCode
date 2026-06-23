# TaskSpace 证据等级与样本集

- 状态：可执行
- 创建日期：2026-06-18
- 更新日期：2026-06-18
- 范围：TaskSpace 基准测试与发布证据治理

## 1. 目的

本文档用于修复一次流程问题：v0.0.5 收口讨论中，内部矩阵接近 100% 的结果被表述成容易误解为 Terminal-Bench E3 正确率的结论。这种口径不能用于发布判断。

从现在开始，每一次 TaskSpace 实验都必须同时标明：

1. 证据等级：E1、E2、E3、E4 或 E5。
2. 样本集：本次运行实际采用的、已经登记的任务或场景名称。

证据等级不是任务难度。`L1`、`L2`、`L3` 这类标签描述场景形态；`E1`、`E2`、`E3` 这类标签描述结果能够支撑多强的结论。

## 2. 证据等级

| 等级 | 名称 | 来源 | 最低门禁 | 允许结论 | 禁止结论 |
|---|---|---|---|---|---|
| E1 | 机制冒烟 | 内部测试夹具或聚焦本地场景 | 可复现提示词、测试夹具、验证器或确定性人工检查 | 某条机制路径可以执行并产出产物 | 版本正确率、效用、发布或外部基准测试结论 |
| E2 | 内部工程回归 | 内部构造场景 | Standard/TaskSpace 成对运行、重复次数 >= 3、提示词门禁干净、模型 provider 参数完整、聚合统计已启用、判定器隔离未失败 | 指定内部测试夹具集上的效用和回归就绪判断 | Terminal-Bench、DeepSWE、外部基准测试或产品级正确率结论 |
| E3-candidate | 外部效用候选 | 外部基准测试或脱敏历史 Whale 失败样本 | E3 源数据和成对产物已存在，但至少一个 E3 门禁仍未完成 | 工程 clean 的外部候选证据，等待审计或证明闭环 | 最终 E3 分数、发布通过或公开产品结论 |
| E3 | 外部效用证据 | Terminal-Bench、DeepSWE 或已审计历史 Whale 失败样本 | 重复次数 >= 5、固定源版本、原始提示词校验和、验证器校验和、官方或等价验证器证明、验证器/源码隔离证明、成对产物、已完成人工审计 | 声明范围和样本集内的外部效用结论 | 超出样本范围的泛化产品结论 |
| E4 | 发布校准证据 | E3 与必要内部回归的登记组合 | 当前运行器尚未实现；必须包含与上一发布版本的同口径对比、配置档身份、成本门禁、分数有效性和明确发布决策 | 指定发布配置档下的版本发布就绪判断 | 在 E4 工具和样本集登记前做任何 E4 结论 |
| E5 | 产品基准测试看板 | 冻结的长期基准测试看板 | 当前尚未实现；必须包含独立审查、固定周期、稳定样本板、统计阈值、成本核算和竞品或基线策略 | 产品级基准测试趋势或外部定位结论 | 当前任何 v0.0.x 发布结论 |

当前代码实际实现了 `E1`、`E2`、`E2-candidate`、`E3-candidate` 和 `E3`。`E4`、`E5` 目前只是治理定义，必须等运行器支持和样本板登记完成后才能使用。

## 3. 候选证据规则

候选证据低于目标证据等级，不能偷换口径。

| 报告等级 | 含义 | 能否满足目标等级 |
|---|---|---|
| `E1` | 冒烟或降级证据 | 只能满足 E1 |
| `E2-candidate` | 仍有 E2 门禁未解决的内部效用候选 | 不能满足 E2 |
| `E2` | 内部效用证据 | 可以满足 E2 |
| `E3-candidate` | 仍有 E3 门禁未解决的外部效用候选 | 不能满足 E3 |
| `E3` | 已闭环的外部效用证据 | 可以满足 E3 |

如果报告里写着 `requested_evidence_target: E3`，但 `reported_evidence_level: E3-candidate`，这次结果就不是 E3。

## 4. 已登记样本集

### 4.1 E1：内部冒烟集

样本集标识：`taskspace-internal_E1-smoke_2_5`

| 样本 | 定义 | 当前清单目标 | 用途 |
|---|---|---|---|
| `count-call-stack` | 内部解析器/格式敏感测试夹具，使用 `scripts/validate.py` 判定器 | E1 | 验证轻量路径和验证优先在精确输出格式任务上的行为。 |
| `large-output-ref-smoke` | 内部大输出回放测试夹具，使用 pytest 判定器 | E1 | 验证输出引用创建和大输出回放控制。 |

允许结论：这些场景只能说明特定机制是否跑通，不能证明 TaskSpace 效用或发布就绪。

### 4.2 E2：v0.0.5 内部回归矩阵

样本集标识：`taskspace-internal_E2-v005-regression_3_5`

| 样本 | 定义 | 场景等级 | 当前清单目标 | 用途 |
|---|---|---:|---|---|
| `single-file-fast-fix` | 内部单文件 Python 税费计算修复，使用 pytest 判定器 | L1 | E2 | 确认 TaskSpace 不会破坏简单确定性修复。 |
| `multi-file-order-pipeline` | 内部多文件订单处理修复，覆盖 README/test 冲突处理 | L2 | E2 | 验证多文件推理和本地回归验证。 |
| `subscription-billing-repair` | 内部订阅计费修复，编辑面更宽 | L3 | E2 | 验证更宽编辑范围和成本告警行为。 |

v0.0.5 收口矩阵还包含上面的 E1 冒烟样本，所以该组合运行是内部 E1/E2 混合工程矩阵，不是 Terminal-Bench E3。

### 4.3 E3 候选：Terminal-Bench 原始四样本

样本集标识：`terminal-bench_E3-original_4_5`

| 样本 | 定义 | 历史用途 |
|---|---|---|
| `hello-world` | Terminal-Bench 入门文件/任务验证样本 | 用于早期完整基准测试和 E3 测试框架证明。 |
| `heterogeneous-dates` | Terminal-Bench 数据和日期规范化任务 | 用于早期外部效用探索。 |
| `jsonl-aggregator` | Terminal-Bench JSONL 聚合任务 | 暴露过 TaskSpace 节点增长和成本放大问题。 |
| `log-summary` | Terminal-Bench 日志摘要任务 | 暴露过 TaskSpace 效用与成本混合表现。 |

只有当该样本集内所有纳入统计的配对运行都报告为 `E3` 时，才可以支撑 E3 结论。早期存在被排除项或候选配对运行的结果，只能描述为诊断证据或 E3-candidate 证据。

### 4.4 E3 候选：Terminal-Bench P0 可比范围

样本集标识：`terminal-bench_E3-P0_3_5`

此样本集的可执行样本数按 3 个计算。`query-optimize` 不属于执行样本集；它只是历史 P0 运行中的封闭失败预检记录，不能计入成功率、耗时或 token 的执行样本数。

| 样本 | 定义 | 计划重复策略 | 历史执行口径 | 统计处理 |
|---|---|---:|---|---|
| `processing-pipeline` | Terminal-Bench processing pipeline 修复任务 | 5 个 Standard/TaskSpace 配对 | v0.0.3/v0.0.4 P0 运行均执行 5 个配对 | 当前可比 P0 样本。 |
| `multi-source-data-merger` | Terminal-Bench 多源数据合并和冲突报告任务 | 5 个 Standard/TaskSpace 配对 | v0.0.3/v0.0.4 P0 运行均执行 5 个配对 | 当前可比 P0 样本。 |
| `recover-accuracy-log` | Terminal-Bench recovery/accuracy log 任务 | 5 个 Standard/TaskSpace 配对 | v0.0.3/v0.0.4 P0 运行均执行 5 个配对 | 当前可比 P0 样本。 |

这是 v0.0.3/v0.0.4 的主要 P0 可比范围。用于 agent 能力统计的实际执行样本是 3 个，共 15 个配对。`query-optimize` 因远程资产等价性未证明，历史运行中只保留封闭失败预检记录，不进入该表，也不参与任何执行样本数、正确率、耗时或 token 对比。该样本集结果不能和 v0.0.5 内部测试夹具结果直接比较。

### 4.4.1 Diagnostic-only：Terminal-Bench P0 低成本诊断变体

样本集标识：

```text
terminal-bench_E3-P0_1_1
terminal-bench_E3-P0_3_1
terminal-bench_E3-P0_3_2
```

这些标识沿用数据集名称、子集名称、sample 数量和 repeats 次数的命名规范，但证据等级不是正式 E3。它们必须在所有报告和 release artifacts 中写明：

```text
reported_evidence_level = diagnostic-only
not_release_proof = true
formal_sample_set_id = terminal-bench_E3-P0_3_5
```

允许用途：

- 在代码完成前后用最小成本检查 request/token/spawn/budget 是否还有数量级失控。
- 选择正式 `terminal-bench_E3-P0_3_5` 前的工程健康检查。
- 定位具体样本的 harness 或 runtime 问题。

禁止用途：

- 不得支撑 `release_pass`。
- 不得声明“正确率未下降已经坐实”。
- 不得与 `terminal-bench_E3-P0_3_5` 混表计算成功率、耗时或 token 倍率。
- 不得用 `_1_1`、`_3_1` 或 `_3_2` 结果替代 3 samples x 5 repeats 的正式 P0 结论。

### 4.5 E3 候选：v0.0.4 clean 15-run 可比范围

样本集标识：`terminal-bench_E3-v004-clean_3_5`

| 样本 | 定义 | 重复策略 | 历史结果 |
|---|---|---:|---|
| `analyze-access-logs` | Terminal-Bench access-log 分析任务 | 5 | v0.0.4 工程 clean 运行：Standard 4/5，TaskSpace 5/5。 |
| `log-summary` | Terminal-Bench 日志摘要任务 | 5 | v0.0.4 工程 clean 运行：Standard 3/5，TaskSpace 3/5。 |
| `count-call-stack` | Terminal-Bench call-stack 计数/格式任务 | 5 | v0.0.4 工程 clean 运行：Standard 0/5，TaskSpace 0/5。 |

如果 v0.0.5 要证明相对 v0.0.4 正确率未下降，这是必须使用的同口径基线。除非审计完成且每个纳入配对运行都报告 `E3`，否则只能称为 E3-candidate 或“工程 clean 的公开验证器证据”，不能称为最终 E3。

### 4.6 E3 Harness Proof：已审计 Terminal-Bench Hello World

样本集标识：`terminal-bench_E3-hello-audited_1_5`

| 样本 | 定义 | 用途 |
|---|---|---|
| `hello-world` | 单个 Terminal-Bench 样本；历史证明运行已完成人工审计，且没有 E3 门禁失败 | 证明简单外部样本上的测试框架闭环，不证明广泛 TaskSpace 效用。 |

### 4.7 未来 E3：历史 Whale 失败语料

样本集标识：`historical-whale_E3-failures_TBD_TBD`

定义：脱敏后的真实 Whale 用例、会话失败、运行时失败或产品回归。样本必须包含原始提示词哈希、脱敏测试夹具、验证器或审计路径、隐私审查和产物审计。

当前状态：语料规则已经存在，但还没有在本文档中登记具体活跃样本名。在具体样本标识登记之前，该样本集不能支撑发布结论。

### 4.8 未来 E3：DeepSWE Adapter 范围

样本集标识：`deepswe_E3-adapter-spike_TBD_TBD`

定义：通过 Whale 外部基准测试适配器接入的 DeepSWE 长程软件工程任务子集。

当前状态：适配器已有试验路径，但还没有在本文档中登记具体活跃样本名。在具体样本标识和验证器保真证明登记之前，该样本集不能支撑发布结论。

## 5. E4 与 E5 登记规则

E4 和 E5 当前刻意留空，直到项目补齐必要工具并冻结样本板。

| 等级 | 首次运行前必须登记的样本要求 |
|---|---|
| E4 | 一个发布校准集，必须列出所有纳入的 E3 样本集、所有必需 E2 回归样本、上一版本基线、配置档哈希策略、成本门禁和发布决策负责人。 |
| E5 | 一个产品基准测试看板，必须列出外部基准测试家族、样本选择策略、刷新节奏、统计阈值、竞品或基线策略和独立审查流程。 |

在这些条目登记到本文档之前，任何文档都不得宣称 E4 或 E5 证据。

## 6. 允许的版本对比结论

| 对比 | 是否允许 | 必须使用的表述 |
|---|---|---|
| v0.0.5 内部 E1/E2 矩阵 vs v0.0.4 Terminal-Bench E3 candidate | 不允许 | “不同口径，不能比较正确率。” |
| v0.0.5 在 `terminal-bench_E3-v004-clean_3_5` 上复跑 vs v0.0.4 clean 15-run | 允许，前提是同配置档和分数有效性都有记录 | “同口径 Terminal-Bench clean 15-run 对比。” |
| v0.0.5 E2 矩阵 vs 之前的 E2 矩阵 | 允许，前提是内部样本集和重复次数相同 | “内部工程回归对比。” |
| E3-candidate vs E3 | 不允许 | “候选证据仍在等待 E3 门禁。” |
| 任何 E1/E2 结果作为发布就绪证据 | 不允许 | “仅代表工程就绪。” |

## 7. 运行记录必填字段

每份实验报告都必须包含：

```text
experiment_level:
reported_evidence_level:
requested_evidence_target:
sample_set_id:
sample_names:
scenario_levels:
benchmark_family:
runner_entrypoint:
runner_profile_hash:
source_version:
repeats_per_sample:
mode_pairing:
run_root:
score_valid:
engineering_clean:
human_audit_status:
token_summary_status:
allowed_claim:
explicit_non_claims:
```

任一字段缺失时，该结果只能标记为 `diagnostic-only`，直到补齐记录。

## 8. 命名规则

实验样本集标识必须使用以下格式：

```text
数据集名称_子集名称_sample数量_repeats次数
```

示例：

```text
terminal-bench_E3-P0_3_5
```

字段规则：

| 字段 | 规则 |
|---|---|
| 数据集名称 | 使用数据来源或 benchmark 家族名，例如 `terminal-bench`、`taskspace-internal`、`deepswe`。 |
| 子集名称 | 使用证据等级和子集语义，例如 `E3-P0`、`E3-v004-clean`、`E2-v005-regression`。 |
| sample数量 | 只统计实际进入 agent 执行并可用于能力统计的样本数；预检失败、资产不可用、未执行项不计入。 |
| repeats次数 | 每个执行样本的配对重复次数；未登记或未来样本集使用 `TBD`。 |

以下术语必须精确使用：

| 术语 | 含义 |
|---|---|
| `internal matrix` | 基于 `benchmarks/taskspace/scenarios/*` 测试夹具的运行，通常是 E1/E2。 |
| `Terminal-Bench E3` | 通过外部基准测试适配器执行，且具备 Terminal-Bench 源数据元信息和 E3 门禁的运行。 |
| `clean public-validator evidence` | 工程测试框架有效且 public validator 结果可信，但尚未完成最终 E3 审计闭环的运行。 |
| `full E3` | 只有纳入统计的配对运行都报告 `E3`，审计已完成，且分数有效性为 true 的运行。 |
| `same-scope comparison` | 样本集标识、样本名、重复策略和运行器配置档实质相同的对比。 |

## 9. v0.0.5 口径修正

v0.0.5 的 `24/25` 结果必须描述为：

```text
v0.0.5 在内部 E1/E2 混合工程矩阵上取得 24/25 原始业务成功结果：
taskspace-internal_E2-v005-regression_3_5 加 taskspace-internal_E1-smoke_2_5。
```

不得描述为：

```text
v0.0.5 取得了接近 100% 的 Terminal-Bench E3 正确率。
```

v0.0.5 下一步正确的同口径正确性检查，是在 v0.0.5 配置档下复跑 `terminal-bench_E3-v004-clean_3_5`，并与 v0.0.4 clean 15-run 对比。

该检查只回答“相对 v0.0.4 clean 15-run 是否退化”，不能替代 v0.0.5 当前 P0 成本/正确率收口所需的 `terminal-bench_E3-P0_3_5`。
