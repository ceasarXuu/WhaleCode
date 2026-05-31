# TaskSpace 分层 E2E 与 Benchmark 设计

日期：2026-05-31

## 背景

当前 TaskSpace 已经通过两类真实 E2E：

- 自然用户请求不暴露 `taskspace/map/node/subagent/parallel` 等内部概念。
- Whale 在真实 `whale exec --taskspace` 路径中创建任务图、派生 subagent、写回 node result、修改沙盒代码库并运行真实测试。
- E2E 不再只统计 node 数量，而是校验 edge、依赖顺序、并行调查轨道、实施与验证依赖、最终节点闭合。

这证明了第一阶段工程可行性，但还不能证明 TaskSpace 在更复杂、更脏、更长的真实开发任务中有稳定净收益。

下一阶段测试目标是：用复杂度分层 benchmark 持续施压，观察 TaskSpace 是否能在中高复杂度任务中优于标准线性模式，并确保低复杂度任务不被拖累。

## 证据等级

必须区分“机制能跑”和“产品有收益”。不同等级的测试只能支持不同强度的结论，不能越级宣传。

| 等级 | 名称 | 需要证据 | 允许结论 |
|---|---|---|---|
| E0 | Mechanism Smoke | 单次真实 `whale exec --taskspace`、真实工具、真实测试、基础 map/node/edge 观测 | 机制路径可运行 |
| E1 | Constructed Regression | 自建沙盒、多种变体、隐藏 oracle、图健康硬校验 | 机制在构造场景中稳定 |
| E2 | Paired Utility | 同题 standard/taskspace 对照、统一 oracle、成本与漏项统计、多次运行 | TaskSpace 在该类任务上出现可测净收益 |
| E3 | Real-world Utility | 历史真实失败样本或外部 benchmark、paired 对照、重复运行统计、人工复核 | TaskSpace 对真实复杂任务有产品收益证据 |

约束：

- E0/E1 不能宣称 TaskSpace 已证明真实复杂任务净收益。
- 只有达到 E2，才能说“在某类构造任务上优于 standard”。
- 只有达到 E3，才能说“对真实复杂任务有收益证据”。
- 文档、报告、发布说明必须标注当前证据等级。

## 产品假设

TaskSpace 的定位不是替代所有简单执行路径，而是面向中高复杂度问题解决的非线性工作组织层。

核心假设：

```text
低复杂度任务：
  TaskSpace 不应明显慢于 standard，不强制复杂图，不强制 subagent。

中复杂度任务：
  TaskSpace 应减少漏项、误改和上下文混杂。

高复杂度任务：
  TaskSpace 应形成健康任务图，让主 agent 能调度调查、整合、实施、验证。

长程复杂度任务：
  TaskSpace 应在多轮追问、插话、目标变化、上下文增长下维持 task 边界和工作结构。
```

当前优先级：

- 可观察：用户和开发者能看到任务如何推进。
- 可检查：E2E 能判断图结构和执行顺序是否健康。
- 暂不优先做可恢复/时空回溯。相关能力未来可在 viewer、历史 map、reborn 链路上发展，但不应牵引第一阶段复杂度。

## 分层测试矩阵

### L1 低复杂度任务

目标：证明 TaskSpace 不拖累简单任务。

典型场景：

- 单文件小 bug 修复。
- 简单测试断言修正。
- 小型文案或配置变更。
- 明确错误信息定位到单个函数。

用户输入要求：

- 像真实用户一样描述问题，不出现内部概念。
- 不要求拆分、并行、subagent、task、node。
- 验收条件清晰，例如“修完后跑这个测试”。

期望 TaskSpace 行为：

```text
task -> 1 到 3 个 node -> validation -> final
```

不要求：

- subagent。
- 多个并行 inspect node。
- 复杂 edge 网络。

硬指标：

| 指标 | 期望 |
|---|---|
| task created | true |
| ordinary_before_binding | false |
| completed_nodes | >= 1 |
| open_leaf_nodes | 0 |
| validation_passed | true |
| edit_owned_by_implementation_node | true |
| unexpected_taskspace_gate_failures | 0 |

软指标：

| 指标 | 观察目标 |
|---|---|
| wall_time_ratio_vs_standard | 初始阈值 <= 1.5，超过则标记成本劣化 |
| tool_call_ratio_vs_standard | 初始阈值 <= 1.5，超过则标记成本劣化 |
| token_ratio_vs_standard | 可采集时初始阈值 <= 1.5 |
| node_count | 不追求多，过多反而是负面信号 |
| subagent_count | 通常为 0 |

失败信号：

- 简单任务被拆成大量 node。
- 没有必要却 spawn subagent。
- 为维护图而重复读取大量文件。
- standard 一步能完成，TaskSpace 绕很久才完成。

### L2 中复杂度任务

目标：证明 TaskSpace 能处理多文件关联问题，并比线性模式更少漏项。

典型场景：

- 多文件关联 bug，有明确测试。
- README、测试、实现之间有轻微冲突。
- 一个行为由 parser、service、formatter、tests 共同决定。
- 前端组件、状态管理、接口 mock 三处共同影响一个问题。

用户输入要求：

- 正常描述项目症状和验收。
- 可以说“先理解再改”“区分业务规则和测试预期”，但不能提示内部协作策略。

期望 TaskSpace 行为：

```text
boundary/事实源识别
  -> 2 到 4 个独立 inspect node
  -> synthesis 或 baseline validation
  -> implementation
  -> regression/smoke
  -> final
```

硬指标：

| 指标 | 期望 |
|---|---|
| edge_count | >= 关键节点数 - 1 |
| edge_order_violations | 0 |
| key_edges_have_reason | true |
| implementation_consumes_upstream_result | true |
| implementation_has_incoming_edge | true |
| test_depends_on_implementation | true |
| direct_test_depends_on_implementation | true |
| nodes_with_results | 与 completed_nodes 接近 |
| validation_passed | true |
| unexpected_failed_collab_tool_calls | 0 |

按场景启用的硬指标：

| 条件 | 额外要求 |
|---|---|
| 存在多个独立事实面 | 至少 2 个 inspect track |
| 使用 subagent | subagent result 必须写回 node |
| README 与测试冲突 | 最终 diff 必须体现对产品真相的选择 |

软指标：

- 是否把事实源、实施、验证拆成不同节点。
- 是否存在明确的调查结果整合。
- 是否避免把所有读取塞进一个超大 inspect node。
- 是否避免多个 subagent 读取高度重叠文件。

### L3 高复杂度任务

目标：证明 TaskSpace 在混乱信息环境下仍能维持结构。

典型场景：

- 架构质量分析并提出治理方案。
- 跨模块重构，涉及接口、测试、日志、文档。
- Debug 场景中日志、复现、代码路径、配置互相矛盾。
- README 过时，测试缺失，代码行为隐式依赖历史约定。
- 多种修复路径，需要先比较方案再实施。

用户输入要求：

- 可以表达复杂目标，例如“检查架构质量并优化”“定位这个跨模块问题”“不要只修表面”。
- 不出现 task/node/map/subagent/parallel/delegate。
- 不直接告诉 agent 怎样拆任务。

期望 TaskSpace 行为：

```text
scope/boundary
  -> evidence tracks
  -> risk tracks
  -> synthesis/decision
  -> staged implementation
  -> smoke/regression/review
  -> final
```

硬指标：

| 指标 | 期望 |
|---|---|
| has_boundary_node | true |
| parallel_inspect_tracks | >= 2 |
| parallel_inspect_tracks_independent | true |
| key_edges_have_reason | true |
| implementation_consumes_upstream_result | true |
| implementation_depends_on_parallel_inspect_tracks | true |
| direct_implementation_depends_on_parallel_inspect_tracks | true |
| validation_node_has_real_command | true |
| edit_outside_implementation | 0 |
| edge_order_violations | 0 |
| open_leaf_nodes | 0 |
| open_final_synthesis_nodes | 0 |

效用型指标：

- 与 standard 模式对照时，是否少漏关键文件。
- 是否能识别错误测试、错误 README、错误实现中的至少一种冲突。
- 是否能解释采用方案和放弃方案。
- 是否减少无依据修改。
- 是否更容易从 viewer 看出当前卡点和下一步。

失败信号：

- 只有一个巨大 inspect node。
- 边退化成顺序流水账，无法表达真实依赖。
- subagent 都扫同一批文件，没有独立事实面。
- 主 agent 不读取 node result，直接线性重扫和修改。
- 实施节点没有依赖调查节点。

### L4 长程复杂度任务

目标：验证 TaskSpace 在多轮会话和上下文增长中是否保持 task 边界。

典型场景：

- 用户先要求架构检查，中途插入小 bug，再回到架构任务。
- 用户修改目标，从“修 bug”变成“先设计方案，不动代码”。
- 用户否定上一轮方案，要求换思路。
- 上下文压缩后继续同一个 task。
- 同一 session 中存在多个 active/pending task。

用户输入要求：

- 多轮自然对话。
- 用户不理解 TaskSpace 内部概念。
- 用户可能含糊、插话、改变优先级。

期望 TaskSpace 行为：

```text
turn 1: 建立 task A
turn 2: 更新 task A map
turn 3: 识别插话为 task B 或 task A 的子问题
turn 4: 回到 task A，不污染 task B
turn 5: 压缩后继续，task/map/node/result 结构仍可用
```

硬指标：

| 指标 | 期望 |
|---|---|
| task_routing_required | true |
| no_ordinary_before_task_binding | true |
| task_count | 能随用户主题自然增长 |
| active_task_switch_logged | true |
| node_results_preserved_after_compaction | true |
| current_binding_valid_after_compaction | true |
| no_cross_task_result_pollution | true |

暂缓指标：

- 完整时空回溯。
- 任意历史节点重新执行。
- 自动 reborn。

这些属于未来可恢复能力，不作为当前 benchmark 的第一优先级。

## 两类指标

### 工程可行性指标

这类指标回答“机制有没有按设计运行”。

| 类别 | 指标 |
|---|---|
| 启动 | taskspace_enabled、viewer_url、task created |
| 绑定 | ordinary_before_binding、current node lease |
| 图结构 | node_count、edge_count、edge_order_violations |
| 节点生命周期 | completed_nodes、open_leaf_nodes、open_final_synthesis_nodes |
| subagent | spawn_agent_calls、subagent_results、unexpected_failed_collab_tool_calls |
| 工具归属 | edit_owned_by_implementation、test_owned_by_validation |
| 验证 | pytest/command exit code、hidden oracle、git diff |
| 稳定性 | crash events、timeout、invalid_request_error |

工程指标用于硬失败判断。只要工程指标失败，就不能声称 TaskSpace 机制健康。

### 效用指标

这类指标回答“TaskSpace 是否比标准线性模式更有价值”。

| 类别 | 指标 |
|---|---|
| 漏项 | 是否检查到关键文件、关键事实源、关键测试 |
| 误改 | 是否修改了无关文件、是否为了错误测试扭曲产品规则 |
| 幻觉 | 是否引用不存在的行为、文件、命令结果 |
| 调度 | 是否主动拆分独立事实面、是否利用 node result |
| 成本 | wall time、tool calls、token、subagent count |
| 可观察 | viewer 是否能解释当前状态、依赖、卡点 |
| 对照收益 | 与 standard 同题对照的成功率、漏项率、耗时、误改率 |

效用指标不应全部变成硬门槛。复杂任务存在自然不确定性，benchmark 需要长期积累统计，而不是单次运行绝对判断。

## Standard 对照策略

从 E2 开始，每个 benchmark 场景必须有两条运行路径：

```text
standard:
  whale exec ...

taskspace:
  whale exec --taskspace ...
```

对照时不要要求 TaskSpace 在所有维度胜出。合理目标是：

- L1：TaskSpace 不明显更差。
- L2：TaskSpace 在漏项和误改上更稳。
- L3：TaskSpace 在结构化调查、验证闭环、可观察性上明显更好。
- L4：TaskSpace 在多主题和压缩后结构保留上优于 standard。

对照报告至少包含：

- 两种模式的最终业务验证结果。
- 两种模式的 git diff。
- 两种模式的命令执行记录。
- TaskSpace 的 observability artifact。
- standard 的线性 transcript 摘要。
- 统一 oracle 输出：关键文件覆盖、禁止修改文件、产品真相选择、隐藏测试、漏项分类。
- 失败分类：工程失败、业务失败、调度失败、成本劣化、观测不足。

对照运行策略：

- 每个 E2 场景初始至少重复 3 次，避免单次模型随机性误判。
- 每个 E3 场景初始至少重复 5 次，并保留人工复核摘要。
- 统计时分开记录 pass rate、business success、graph health、cost、leak/mis-edit。
- 如果 standard 和 taskspace 都失败，不能算 TaskSpace 负收益；应归入任务难度或模型能力失败，再看失败形态是否不同。
- 如果 TaskSpace 成功但成本超过 L1 阈值，低复杂度场景仍判成本劣化。

## Benchmark 场景库

第一阶段自建场景：

| 场景 | 层级 | 目的 |
|---|---|---|
| single-file-fast-fix | L1 | 验证低复杂度不拖累 |
| config-test-fix | L1 | 验证小范围配置/测试修复 |
| order-pipeline-natural | L2 | README、测试、实现冲突 |
| order-pipeline-growth | L2/L3 | 多 inspect track、subagent、edge health |
| logging-regression-debug | L3 | 日志、复现、代码路径三源调查 |
| architecture-quality-review | L3 | 架构检查、治理建议、部分落地 |
| interrupted-session | L4 | 用户插话和回到旧任务 |
| compaction-continue | L4 | 压缩后继续 task |

第二阶段引入外部 benchmark：

- Terminal-Bench：适合观察真实终端任务、长命令链、环境问题和任务完成率。
- SWE-bench 风格任务：适合观察修复正确性、测试闭环和跨文件定位。
- 自定义 Whale regression corpus：保存历史真实失败样本，避免只测人工构造的理想场景。

外部 benchmark 适配原则：

- 不把 benchmark prompt 改写成 TaskSpace 指令。
- 不泄露内部 map/node/subagent 概念。
- 保留原始验收方式。
- TaskSpace 额外导出 observability artifact。
- 同一题尽量跑 standard 与 taskspace 对照。

实施顺序约束：

- E1 自建场景用于保护机制回归。
- E2 paired 对照不应长期后置；L1/L2 自建场景稳定后立刻补 standard/taskspace 对照。
- E3 外部 benchmark 和历史真实失败 corpus 是收益声明前置条件，不是锦上添花。

## Prompt Guard

所有拟真用户 prompt 默认禁止出现：

```text
taskspace
action map
map
node
subagent
spawn_agent
taskspace_control
parallel
parallelize
concurrent
simultaneously
delegate
delegation
multiple agents
multi-agent
split ... agents
fan out
```

允许出现：

- “先理解再修改”
- “区分产品规则和测试预期”
- “不要只修表面”
- “跑测试验证”
- “说明你怎么组织工作”

这些是普通用户可自然表达的工作要求，不属于内部协作机制泄漏。

## 报告格式

每个 E2E run 生成：

```text
target/real-user-e2e/<scenario>/<timestamp>/
  repo/
  artifacts/
    user-prompt.txt
    whale-exec.jsonl
    whale-exec.stderr.log
    last-message.md
    git-diff.patch
    validation.stdout.log
    validation.stderr.log
    hidden_oracle.py
    hidden-oracle.stdout.log
    observability/
      action-map-observability.json
      action-map-observability.md
      action-map-observability.html
    report.md
```

`report.md` 必须包含：

- scenario id。
- whale binary path、version、sha256。
- model。
- thread id。
- prompt leak 检查结果。
- 工程可行性指标。
- 效用指标。
- 证据等级。
- key edge reason 覆盖率。
- node result consumed 事件或等价证据。
- per-node source refs 与 source overlap 摘要。
- failures。
- artifact 路径。

关键 edge 报告字段：

```text
edge_id
from_node_id
to_node_id
dependency_kind
edge_reason
created_at
created_by
from_result_id
consumed_by_node_id
```

第一版如果 runtime 尚未持久化全部字段，E2E report 必须标注字段缺失，不得把 edge_count 单独解释为健康依赖证据。

## 失败分类

| 类型 | 含义 | 处理方式 |
|---|---|---|
| Harness failure | 脚本、路径、环境、依赖失败 | 修测试基础设施 |
| Runtime failure | TaskSpace gate、binding、lease、tool call 协议失败 | 修 runtime |
| Decomposition failure | 图没有健康生长，主 agent 退回线性模式 | 修方法论注入和调度约束 |
| Business failure | 最终代码行为不满足验收 | 分析 agent 能力和任务上下文 |
| Observability failure | 任务完成但无法解释过程 | 修 viewer/export |
| Cost regression | 简单任务被明显拖慢或工具调用过多 | 调整复杂度识别和轻量路径 |

## 近期实施顺序

1. 把现有 natural/growth E2E 归入 L2/L3。
2. 补 L1 简单任务基准，证明 TaskSpace 不拖累。
3. 给 L1/L2 增加 standard/taskspace paired 对照，建立 E2 证据。
4. 补 L3 架构质量/Debug 压力任务，观察主 agent 是否主动调度。
5. 补 L4 多轮任务和上下文压缩样本。
6. 建立历史真实失败 corpus，接入少量 Terminal-Bench/SWE-bench 风格任务，冲击 E3 证据。

## L4 Harness 草案

L4 不在第一阶段追求回溯，但需要能证明 task 结构没有被压缩或插话破坏。

最小 harness：

```text
turn 1: 用户提出 task A，中复杂度任务。
turn 2: 用户补充 task A 约束，触发 map growth。
turn 3: 用户插入 task B，要求快速处理。
turn 4: 用户回到 task A，要求继续。
turn 5: 人为触发或构造上下文压缩边界。
turn 6: 用户继续 task A。
```

验收：

- task A/B 有不同 task identity。
- task B 的 node result 不写入 task A。
- 回到 task A 时 current binding 指向 task A 的有效 node。
- 压缩后 task manifest 仍保留 task id、active map、open node、关键 completed result summary。
- 不要求恢复旧完整上下文，不要求重放历史节点。

## 当前非目标

- 不用 E2E 做主观质量打分。
- 不要求低复杂度任务强行拆成漂亮 graph。
- 不把用户 prompt 写成内部流程指令。
- 不在第一阶段追求完整回溯、历史节点重放或自动恢复。
- 不为了通过 benchmark 写固定关键词回复或绕过模型路径。
