# BaseMap 拆解方法论注入设计

日期：2026-05-31

## 2026-06-04 修订状态

本文件保留为第一版 BaseMap 拆解方法论背景材料，但 E3 外部 benchmark 后已经证明：仅把方法论放进 prompt / developer context 不足以稳定改变 agent 行为。

最新设计基线见：
[TaskSpace E3 负收益后问题状态与模型管理重构基线](./2026-06-04-taskspace-cognitive-state-runtime-after-e3.md)。

后续实现应以“主 agent 作为问题状态与模型管理者”和“task/map/node/result 结构承载方法论”为准。本文件中的 prompt 注入仍可复用，但不能再被视为充分方案。

## 背景

TaskSpace 的收益不来自“有一张 map”，而来自主 agent 能把复杂任务拆成有边界、有依赖、可调度、可检查的工作图。

如果没有拆解方法论，主 agent 很容易退化成：

- 把线性计划改写成几个空泛节点。
- 创建多个标题不同但输入范围高度重叠的 inspect node。
- subagent 重复阅读同一批文件。
- 主 agent 不使用 node result，重新完整扫描。
- implementation node 不依赖调查结论，直接凭当前上下文修改。

因此 BaseMap 第一版不应扩展成多领域 map 模板库，而应内置一套通用 Map Decomposition Methodology，在主 agent 创建和更新 map 时强注入。

## 设计原则

### 方法论强注入

主 agent 每次创建、更新、reborn map，或者在宽任务上继续生长 map 时，都必须看到拆解方法论。

这不是 skill 弱提示，而是 TaskSpace developer context 的一部分，是主 agent 运行在 TaskSpace 下的工作纪律。

注入必须按场景裁剪：

- L1/低复杂度：只注入轻量规则，强调不要过度拆解、不要无意义 subagent。
- L2/L3：注入完整拆解方法论，强调事实面、风险面、产物面、边依赖和结果利用。
- L4/长程任务：额外注入 task routing、上下文边界、压缩后结构保留要求。

方法论是工作纪律，不是长篇模板。注入块应保持短而稳定，避免为了教 agent 做 map 而显著挤占业务上下文。

### 结构弱约束

第一版不要求所有 node 都有严格 schema，如 `scope/excluded_scope/expected_result`。

原因：

- 复杂任务开放度高，过硬 schema 容易让 agent 为填字段而失去判断。
- 不同任务的事实面差异很大，强行结构化会导致伪精确。
- 当前更重要的是压住线性退化和重复阅读，而不是提前定义所有返回形态。

节点描述可以采用自然语言，但必须表达边界、目的和结果期待。

### 退化轻检测

runtime 不做语义评判，但可以做轻量结构检查：

- 宽任务不能长期停留在一个巨大 inspect node。
- implementation 前必须存在上游事实/边界节点。
- validation 必须依赖 implementation。
- subagent-owned inspect track 应当是独立轨道，而不是彼此依赖。
- final synthesis 不能悬空。

runtime 负责阻止明显违反结构规则的行动，不负责判断“拆得好不好”。

## 主 agent 的角色定位

TaskSpace 下主 agent 不是第一个冲上去线性干活的 worker，而是全局负责人。

它的职责：

```text
理解用户目标
  -> 识别任务复杂度
  -> 创建或选择 task
  -> 初始化或更新 map
  -> 按事实面/风险面/产物拆 node
  -> 必要时派 subagent 调查
  -> 等待并读取 node result
  -> 综合判断
  -> 创建实施节点
  -> 创建验证节点
  -> 总结并沉淀结果
```

subagent 的职责：

- 只处理绑定 node 的局部事实面。
- 把结果写回 node。
- 不拥有全局决策权。

Task graph 的职责：

- 保留任务结构。
- 表达依赖。
- 承载节点上下文和结果。
- 让过程可观察、可检查。

## 复杂度识别

map 初始化前，主 agent 先粗分任务复杂度。

| 复杂度 | 特征 | 期望图形态 |
|---|---|---|
| 低 | 单文件、明确错误、明确验收 | 单节点或少量节点，不强制 subagent |
| 中 | 多文件、多事实源、测试/实现可能冲突 | 边界节点 + 多个 inspect + 实施 + 验证 |
| 高 | 架构、重构、跨模块 debug、目标含糊 | scope + 多事实/风险轨道 + synthesis + 分阶段实施/验证 |
| 长程 | 多轮、多 task、插话、压缩后继续 | task routing + map 更新 + 上下文边界维护 |

复杂度识别不是 runtime 语义判断，由主 agent 执行。runtime 只暴露当前 task manifest、已有 map、node 状态和可用 BaseMap 方法论。

## 拆解方法

### 优先按事实面拆

事实面是能独立调查并产生可用结论的证据来源。

例子：

```text
README / 产品规则
现有测试
parser 行为
pricing 行为
invoice 集成行为
日志与复现
配置与环境
```

适合 subagent 的事实面应满足：

- 输入边界明确。
- 与其他事实面尽量低重叠。
- 结果能被主 agent 整合。
- 不需要全局决策权。

### 按风险面拆

风险面用于架构、重构、质量治理。

例子：

```text
正确性风险
可维护性风险
性能风险
安全/权限风险
测试覆盖风险
可观测性风险
迁移/兼容风险
```

风险面适合高复杂度任务，但不能被滥用为固定岗位角色。主 agent 应按任务实际风险选择少数关键面。

### 按工作产物拆

产物面用于实施和验证阶段。

例子：

```text
方案设计
实现补丁
日志补强
测试补强
冒烟验证
回归验证
最终说明
```

产物节点通常依赖事实面或风险面节点。

### 不推荐的拆法

空泛阶段拆法：

```text
分析 -> 实施 -> 测试
```

这类拆法只比线性 todo 多了几个标题，不能降低上下文混乱。

重复调查拆法：

```text
Agent A: 检查项目质量
Agent B: 检查项目问题
Agent C: 分析代码
```

如果三个节点都读取同一批文件且产出目标相同，就是成本放大。

## 节点描述要求

第一版不强制 schema，但 BaseMap 方法论要求 inspect node 的 title/description 至少自然表达：

- 这个节点调查什么。
- 主要输入范围是什么。
- 明确不负责什么。
- 期望沉淀什么结果。

推荐写法：

```text
Inspect parser behavior against README requirements.
Input: README parser rules, src/order_pipeline/parser.py, tests/test_parser.py.
Not responsible for pricing or invoice totals.
Result should identify implementation bugs, wrong tests, and evidence paths.
```

中文等价：

```text
检查 parser 行为是否符合 README。
输入范围：README 中 parser 规则、parser.py、test_parser.py。
不负责 pricing 和 invoice。
结果需要指出实现 bug、错误测试预期和证据位置。
```

不推荐：

```text
分析代码
检查项目
调查问题
```

## Edge 设计方法

边表达决策依赖，不表达简单排列。

规则：

- 没有上游结论就不该开始下游工作时，创建有向边。
- 可并行的 inspect 节点之间不建边。
- implementation 依赖相关 inspect/boundary/synthesis。
- validation 依赖 implementation。
- final synthesis 依赖 validation 或最后一个有效结果节点。

示例：

```text
scope/boundary
  -> parser inspect
  -> pricing inspect
  -> invoice inspect

parser inspect + pricing inspect + invoice inspect
  -> implementation

implementation
  -> regression test
  -> final synthesis
```

不推荐：

```text
parser inspect -> pricing inspect -> invoice inspect
```

除非 pricing 的调查必须等待 parser 结论，否则这会把可并行事实面错误串行化。

## Subagent 委派方法

subagent 不是为了“人多”，而是为了降低主 agent 的上下文混杂。

适合委派：

- 独立事实面。
- 独立风险面。
- 明确文件集合。
- 明确产出问题。
- 不需要全局权衡。

不适合委派：

- 单文件简单修复。
- 任务尚未定界。
- 多个节点输入范围高度重叠。
- 需要主 agent 综合权衡的最终决策。

委派时主 agent 应提供：

```text
node_id
task objective
node scope
excluded scope
expected result
relevant source refs
```

subagent 返回结果不限具体格式，但必须沉淀到 node result，至少包含：

- 看了什么。
- 发现了什么。
- 证据在哪里。
- 哪些问题不确定或未覆盖。

## 重复阅读治理

重复阅读不是指同一个文件只能读一次。合理复核是必要的。

健康重复：

- 主 agent 对 subagent 发现做抽样复核。
- 下游 implementation 读取即将修改的文件。
- validation 读取测试结果或相关输出。
- review 节点复核关键风险。

不健康重复：

- 多个 subagent 无边界地读取同一批文件。
- 主 agent 无视 node result，全量重扫所有文件。
- 每个 inspect node 都以“检查项目”为目标。
- node result 过弱，导致主 agent 无法利用。

轻量检测：

- 记录每个 node 的 source refs。
- E2E 中统计 inspect node 的 source overlap。
- 当多个并行 inspect 节点标题相似且 source overlap 高时，标记为 decomposition smell。
- 该 smell 不一定硬失败，但应进入报告。

## Map 创建注入点

### Initial Bootstrap

进入 TaskSpace 后，主 agent 第一次需要为用户请求建立 task map。

注入内容：

- 当前用户请求。
- session 中与当前请求相关的近期上下文摘要。
- TaskSpace manifest。
- BaseMap candidate nodes。
- BaseMap 拆解方法论。
- 当前复杂度识别要求。

主 agent 输出：

```text
TaskMapDraft:
  title
  objective
  complexity
  initial nodes
  initial edges
  current main node
```

runtime 校验：

- 至少一个 node。
- current main node 存在。
- edge 引用有效。
- 无环。
- node title/summary 非空。

### Map Growth

执行过程中发现新任务、遗漏事实面、验证失败、用户补充目标时，主 agent 可以更新 map。

注入内容：

- 当前 active task。
- 当前 map digest。
- open/ready/running/completed node 摘要。
- 最近 node results。
- BaseMap 拆解方法论。
- 当前生长原因。

主 agent 应判断：

- 是补一个新事实面，还是扩展已有 node。
- 是创建 validation，还是先补 implementation。
- 是并行调查，还是必须等待上游。
- 是 task 内更新，还是新建 task。

runtime 校验：

- 新 node 的 edge 引用必须存在且无环。
- 非低复杂度 implementation node 至少有一个来自 `inspect_code_context`、boundary、synthesis 或上游 result-producing node 的入边。
- validation/test node 必须有来自 implementation 或上游 result-producing node 的入边。
- subagent 必须绑定 ready/running inspect node。

注意：runtime 不判断“哪些事实面是必要的”。必要事实面由主 agent 在 map draft 和 node description 中表达；runtime 只检查结构上是否存在入边、kind/status/lease 是否匹配、是否违反执行顺序。

### Reborn

`/task-reborn` 触发时，主 agent 重新生成当前 task 的 active map。

注入内容：

- task objective。
- durable facts。
- failure lessons。
- previous map digest。
- user reborn reason。
- BaseMap candidate nodes。
- BaseMap 拆解方法论。

主 agent 应：

- 不复制旧 map。
- 不继承旧噪声。
- 复用 durable facts 和失败教训。
- 重新生成一条更清晰路径。

## Prompt 注入草案

以下内容应进入 TaskSpace developer context 或 BaseMap metadata 注入块。

```text
You are operating inside TaskSpace. Your role is the global coordinator of the task, not a linear worker.

When creating or updating a task map:
- First classify task complexity: low, medium, high, or long-running.
- Keep low-complexity tasks lightweight. Do not create subagents or many nodes unless the task actually needs them.
- For medium/high-complexity tasks, create a boundary or fact-finding node before implementation.
- Decompose inspect work by independent evidence surfaces, risk surfaces, or work products.
- Do not create multiple inspect nodes that read the same broad source set for the same purpose.
- Parallel inspect nodes should be independent; do not add edges between them unless there is a real dependency.
- Implementation nodes must depend on the relevant inspect, boundary, or synthesis nodes.
- Validation nodes must depend on implementation nodes.
- Final synthesis must depend on the last validation or result-producing node.
- Before implementing, read and use relevant node results. Do not ignore completed node results and restart the same investigation from scratch.
- If node boundaries overlap heavily, merge or rewrite the nodes before executing more work.

Each inspect node description should state:
- what it investigates
- primary input scope
- excluded scope
- expected result

For important dependency edges, include a short reason:
- what upstream result the downstream node needs
- whether the dependency is boundary, evidence, implementation, validation, or synthesis
```

实际落地时可以翻译为中文或中英混合，但系统 prompt 中必须避免让用户看到这些内部概念。

## Runtime 约束边界

runtime 应做：

- 强制 agent 在 task/map/node binding 下行动。
- 阻止普通工具在未绑定 node 时执行。
- 记录 node result。
- 维护 edge、lease、status。
- 对明显结构错误做 gate。
- 导出 observability。

runtime 不应做：

- 基于关键词替主 agent 选择 task。
- 判断任务语义是否完成。
- 给 node 质量打分。
- 自动决定哪个领域 map 更适合。
- 自动生成复杂 map 替代主 agent 判断。

语义拆解仍由主 agent 负责。runtime 提供结构约束和工作模式压力。

结构 gate 示例：

| Gate | 允许 runtime 判断吗 | 依据 |
|---|---|---|
| 未绑定 task/map/node 就调用普通工具 | 是 | binding 状态 |
| subagent 绑定到 completed node | 是 | node status |
| validation node 没有上游 implementation/result-producing 入边 | 是 | edge + node kind |
| implementation 是否遗漏了某个业务事实面 | 否 | 语义判断，应由主 agent/review/E2E smell 处理 |
| 某个 node 是否“质量高” | 否 | 不做质量分 |
| 用户真实意图属于哪个 task | 否 | 由主 agent routing |

## 退化信号

第一版关注可解释、易判断的信号。

处理等级：

| 等级 | 含义 | 是否阻止行动 |
|---|---|---|
| hard gate | 结构上无法安全继续，例如未绑定 node、edge 无效、subagent 绑定 completed node | 是 |
| soft barrier | 可能正在退化，要求主 agent 先拆分、解释或更新 map | 暂停当前普通工具行动，允许通过 taskspace control 修正 |
| smell report | 可疑但不确定，只进入报告和 viewer | 否 |
| cost warning | 成本可能劣化，进入 benchmark/report | 否 |

第一版信号：

| 信号 | 数据来源 | 含义 | 处理 |
|---|---|---|---|
| single_huge_inspect | node kind + main tool call count + elapsed time + source refs count | 一个 inspect node 内持续线性扫描 | soft barrier |
| no_implementation_dependency | node kind + incoming edges | implementation 没有结构性上游入边 | hard gate |
| validation_without_implementation | node kind + incoming edges | validation/test 没有 implementation 或 result-producing 上游 | hard gate |
| repeated_broad_overlap | inspect node source refs overlap + title similarity | 多个 inspect 节点高度重叠 | smell report |
| ignored_node_results | completed node results + downstream result consumption event | 有 completed result 但实施前未记录使用 | smell report，后续可升级 |
| graph_as_todo_list | edge shape + node kind diversity | edge 只有简单串行，且无事实面拆分 | smell report |
| over_decomposition_low_task | complexity + node count + subagent count + cost ratio | 低复杂度任务节点过多或 spawn subagent | cost warning |

初始阈值应保守：

- `single_huge_inspect` 先沿用现有宽 inspect 工具预算，结合 tool call count 触发。
- `repeated_broad_overlap` 第一版只报告，不阻止。
- `ignored_node_results` 第一版只报告，不阻止。
- 任何需要判断业务必要性的信号不得做 hard gate。

## 结果消费与边理由

只创建边还不够。健康 map 需要证明下游节点确实利用了上游结果，否则 edge 可能只是为了通过图健康指标而伪造的外观。

第一版增加两个观测概念：

```text
EdgeReason:
  from_node_id
  to_node_id
  dependency_kind
  reason
  expected_upstream_result

NodeResultConsumed:
  consumer_node_id
  consumed_result_id
  consumed_node_id
  consumed_at
  usage_summary
```

落地策略：

- 主 agent 创建关键 edge 时，提示它写短 reason。
- 主 agent 进入 implementation/validation 前，提示它读取并引用相关 completed node result。
- runtime 可以记录显式读取 result 的事件；如果第一版没有专门读取 API，也可以在 taskspace control 调用、node result summary 或 final synthesis 中记录等价证据。
- E2E 报告统计 key edge reason 覆盖率和 result consumed 覆盖率。

这仍然不是质量分。它只回答“下游是否声明并记录使用了上游结果”，不判断使用得是否完美。

## E2E 验收

方法论注入的 E2E 不只看最终代码是否正确，还要检查 map 是否体现拆解质量。

最低验收：

- 自然用户 prompt 不泄漏内部概念。
- 中高复杂度任务产生 boundary/inspect/implementation/validation/final 基本结构。
- 多个 inspect node 的标题和 source refs 有区分。
- implementation 依赖相关 inspect node。
- validation 依赖 implementation。
- subagent result 写回对应 node。
- 最终业务测试通过。

增强验收：

- 与 standard 模式对照，TaskSpace 漏项更少。
- inspect source overlap 可解释。
- 主 agent 在实施前读取或引用 completed node results。
- 关键 edge 有 reason 或等价依赖说明。
- report 导出 per-node source refs、source overlap、NodeResultConsumed 或等价证据。
- viewer 能清楚展示事实面、实施、验证关系。

## 当前非目标

- 不设计多套领域 map 模板。
- 不引入复杂角色岗位体系。
- 不强制结构化 result schema。
- 不引入质量分。
- 不做 runtime 语义路由。
- 不做完整时空回溯。

## 近期实施建议

1. 把 BaseMap 拆解方法论加入 TaskSpace developer context。
2. 给 `inspect_code_context` node description 增加强提示：scope、excluded scope、expected result。
3. 在 runtime 中保留现有宽 inspect 工具预算 barrier，并把提示文案改成拆解方法论语言。
4. 在 observability export 中加入 node source refs 和 source overlap 统计。
5. 在 E2E 中新增 decomposition smell 报告，不急于全部硬失败。
6. 用 L1/L2/L3 benchmark 验证：简单任务不拖累，中高复杂度任务图更健康。
