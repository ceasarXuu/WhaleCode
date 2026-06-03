# TaskSpace E3 负收益后 Planner 化重构基线

日期：2026-06-04

## 结论摘要

本轮 E3 外部 benchmark 没有证明 TaskSpace 收益，反而暴露出当前设计的核心偏差：

```text
当前已经做到：agent 必须绑定 task/map/node 行动。
尚未做到：主 agent 必须以长期 planner 身份，通过 task map 组织问题解决。
```

因此下一阶段不能继续把重点放在“更多 gate”或“更复杂状态机”上，而要把 TaskSpace 从行动记录系统升级为 planner runtime。

新的核心定位：

```text
主 agent = 长期 Planner / Commander / Orchestrator
subagent = node executor / investigator / implementer
task map = 持久化、可观察、可增长、可审查的任务模型
node = 高内聚、可委派的主题任务
node context/result = 主 agent 理解战场情况的主要信息来源
```

## 背景证据

本设计基线来自 `2026-06-03` 的完整 E3 run：

- 外部 Terminal-Bench paired run：20 pair。
- `standard` 成功率：14/20。
- `taskspace` 成功率：9/20。
- valid E3 pair 中：`taskspace_better = 0`，`standard_better = 5`，`no_clear_delta = 9`。
- TaskSpace 平均 walltime、tool call、node 数量明显更高。
- `jsonl-aggregator` 是最关键负例：TaskSpace 平均 node 数 13.2，成功率 3/5；standard 成功率 5/5。

测试结论不是“TaskSpace 方向被证伪”，而是：

```text
TaskSpace 已具备机制可运行性和可观察性，但当前 map 行为模式没有转化为产品收益。
```

## 负收益后的关键讨论结论

### 固定成本不是首要矛盾

TaskSpace 启动和调度天然有成本。复杂任务在执行前往往无法可靠判断复杂度，因此不能把“简单任务必须不更慢”作为第一优化门槛。

当前更重要的目标是：

- 中高复杂任务是否提升成功率。
- 更弱模型是否因 TaskSpace 获得能力上限提升。
- TaskSpace 是否减少上下文混乱、错误前提扩散、重复阅读和无序探索。

因此下一阶段优先关注成功率和行为质量，而不是先压低启动成本。

### 当前 map 是行动台账，不是任务模型

当前 TaskSpace 的实际协作方式更接近：

```text
主 agent 自己读文件、写代码、跑命令
  -> runtime 要求这些动作绑定到 node
  -> map 记录行动和结果
```

这让 map 很容易退化成行动日志。agent 为了继续调用工具而创建 node，为了继续推进而完成 node，node 粒度自然下沉到行动步骤。

预期协作方式应该是：

```text
主 agent 理解用户目标
  -> 创建或选择 task
  -> 维护 task map
  -> 创建高内聚 node
  -> 委派 subagent 执行 node
  -> 从 node context/result 获取反馈
  -> 质疑、采信或废弃结果
  -> 更新 map
  -> 直到问题解决
```

### node 粒度应该是高内聚主题任务

node 不是非原子操作的拆分，也不是每次工具调用的容器。

合理 node 示例：

- 分析配置加载链路。
- 确认 JSONL 输入数据来源。
- 实现并验证聚合逻辑。
- 审查输出契约和编码兼容性。
- 定位失败根因并给出证据链。

不合理 node 示例：

- 读取 README。
- 写一个文件。
- 执行 pytest。
- 总结当前结果。

如果 node 小到只能描述一个工具动作，它就不是 task map 的子任务，而是执行轨迹。

### encoding 失败暴露输出契约缺失

`hello-world` 和 `heterogeneous-dates` 的 TaskSpace 失败主要来自 PowerShell 写文件编码差异：

- 一个输出带 UTF-8 BOM。
- 一个输出成 UTF-16 LE。
- validator 按严格文本读取失败。

这不是 TaskSpace 天然导致编码错误，而是 task/map 层没有持有输出契约。

Task 级或 node 级应该能持有：

- 输出文件路径。
- 编码要求。
- validator 读取方式。
- 禁止 BOM / 必须 UTF-8 的约束。
- 最终验收条件。

否则 subagent 或主 agent 会在局部工具选择上引入不可见失败。

### jsonl 幻觉暴露事实来源约束不足

`jsonl-aggregator` 中，TaskSpace agent 在误解数据来源后自行生成 JSONL，并基于自己生成的数据完成自检。该现象属于 LLM 自欺和环境模型幻觉，单次样本不能证明 TaskSpace 天然更容易幻觉。

但它证明当前 TaskSpace 没有阻止错误前提扩散：

- 错误前提进入 map 后继续生长。
- blocked/completed 不能表达“这个前提可能是错的”。
- node result 被主 agent 采信过快。
- 缺少数据 provenance 和输入合法性验证。

TaskSpace 应该成为反幻觉结构，而不是错误前提的放大器。

### BaseMap 方法论没有生效的原因

之前已经设计过 BaseMap candidate nodes 和拆解方法论 prompt，但它没有稳定转化为行为。

原因：

- 它只是 developer context / prompt，仍是弱注入。
- 它没有进入 task/map/node 的数据结构。
- `taskspace_control` 的字段以自由文本为主，无法承载强语义契约。
- runtime 只校验是否存在 task/map/node/lease，不校验是否形成任务模型。
- 当前 node kind 更接近行动阶段，容易诱导 `inspect -> implement -> test -> summarize`。
- 某些 maintenance barrier 会提示创建 narrower follow-up node，可能反向推动 node 过细。
- 旧测试主要验证机制可运行，没有验证 map 建模质量。

结论：

```text
方法论不能只写在 prompt 里，必须进入 task/map/node 的结构、工具协议和 runtime 工作流。
```

## 当前 agent-map 协作机制

### 当前链路

```mermaid
flowchart TD
  U["用户自然请求"] --> A["主 agent"]
  A --> C["taskspace_control"]
  C --> R["TaskSpace runtime"]
  R --> T["Task"]
  R --> M["Map"]
  R --> N["Current node"]
  N --> L["Execution lease"]
  L --> Tool["普通工具调用或 spawn_agent"]
  Tool --> NR["Node result / tool result"]
  NR --> N
  N --> A
```

runtime 当前负责：

- 要求普通工具调用前存在 active task/map/node。
- 要求当前 action 符合 node kind。
- 要求主 agent 或 subagent 持有合法 lease。
- 记录 node、edge、result、event。
- 暴露 viewer snapshot。

runtime 当前不负责：

- 判断任务语义。
- 判断 node 是否高内聚。
- 判断 map 是否表达了正确问题模型。
- 判断数据来源是否真实。
- 判断 node result 是否可信。
- 选择 task 或 node。

### 目标链路

```mermaid
flowchart TD
  U["用户目标"] --> P["主 agent: Planner"]
  P --> TM["Task Map: 持久任务模型"]
  TM --> N1["Node A: 高内聚主题任务"]
  TM --> N2["Node B: 高内聚主题任务"]
  TM --> N3["Node C: 验证或审查任务"]
  N1 --> S1["subagent 执行"]
  N2 --> S2["subagent 执行"]
  N3 --> S3["subagent 执行"]
  S1 --> R1["Node A context/result"]
  S2 --> R2["Node B context/result"]
  S3 --> R3["Node C context/result"]
  R1 --> P
  R2 --> P
  R3 --> P
  P --> Q["采信 / 质疑 / 废弃 / 派生新任务"]
  Q --> TM
```

目标状态下，主 agent 的默认动作不是一线工具调用，而是 map operation：

- route task。
- start task。
- create high-cohesion node。
- assign node to subagent。
- inspect node result。
- mark result validity。
- update map。
- synthesize final answer。

## 新设计原则

### Planner 优先

TaskSpace 模式下，主 agent 默认身份是长期 Planner。

它可以在极小任务或必要复核时亲自执行有限工具，但默认不应作为一线 worker 持续读写跑。

第一版不引入绝对禁令，避免把系统做死；但 runtime 和 prompt 都要把一线执行视为例外，而不是默认路径。

### Map 是任务模型

task map 不等同于 todo list，也不等同于执行日志。

task map 至少要表达：

- 用户目标。
- 当前成功标准。
- 已知事实。
- 输入和事实来源。
- 输出契约。
- 未解决问题。
- node 之间的依赖。
- 哪些结果被采信。
- 哪些结果被质疑。
- 哪些路径被替代或废弃。

### Node 是委派单元

node 的最小合格标准：

- 能被一个 subagent 独立执行。
- 有明确输入边界。
- 有明确排除范围。
- 有明确期望产出。
- 产出能被主 agent 用于更新全局判断。

node 不应该对应单次工具调用或单个动作步骤。

### Node context 是主要信息源

subagent 的观察、命令、证据、失败、假设、产出，都应沉淀在 node context/result 中。

主 agent 从 node context 理解全局情况，但不盲信。它可以对结果做有效性标记：

- `accepted`：暂时采信。
- `questioned`：有疑点，需要复核。
- `superseded`：被新证据替代。
- `invalid`：确认错误，不再作为依据。

这不是质量分，也不是 runtime 做客观判断，而是主 agent 对信息可信度的显式管理。

### Runtime 不做语义选择

runtime 不用关键词、BM25 或语义检索选择 task/node。

runtime 的职责是：

- 暴露 task/map/node inventory。
- 校验结构协议。
- 管理 lease 和互斥。
- 记录 result。
- 维持可观察性。
- 阻止明显违反协议的执行路径。

task routing、map 生成、node 选择、结果采信都由主 agent 执行。

## Planner 化重构计划

### Phase P0：确认当前实现边界

目标：把现状和目标差距写清楚，防止继续在旧抽象上补丁式增强。

需要完成：

- 梳理 `taskspace_control` 当前 action 和字段。
- 梳理 runtime gate 当前校验项。
- 梳理 BaseMap prompt 当前注入内容。
- 梳理 subagent spawn 与 node binding 当前路径。
- 梳理 viewer snapshot 中可见的 task/map/node/result 信息。

输出：

- 当前实现边界清单。
- 与本文件目标模型的差距表。

验收：

- 能明确指出哪些能力已经存在，哪些只是 prompt，哪些完全没有工程承载。

### Phase P1：Task/Node 结构升级

目标：让方法论进入结构，而不是只停留在 prompt。

Task 级新增或强化字段：

- `objective`：用户目标。
- `success_criteria`：当前成功标准。
- `fact_sources`：输入和事实来源。
- `output_contracts`：输出契约，如文件、格式、编码、validator。
- `open_questions`：未解决问题。
- `risk_notes`：关键风险和禁止假设。

Node 级新增或强化字段：

- `theme`：主题任务，而不是动作标题。
- `scope`：输入边界。
- `excluded_scope`：明确不负责什么。
- `expected_result`：期望产出。
- `acceptance_hint`：主 agent 如何判断该 node 产出可用。
- `result_validity`：主 agent 对结果的当前采信状态。

约束：

- 字段可以是自然语言，不做过硬 schema。
- runtime 只校验非空和引用合法，不做语义评分。
- 仍允许简单任务使用轻量 task/node，但必须表达最小成功标准。

### Phase P2：主 agent Planner 工作流协议

目标：让主 agent 的默认工作模式从“自己执行”转成“调度执行”。

需要新增或改造的控制动作：

- `start_task`：创建 task 时必须带目标、成功标准、事实来源和首批高内聚 node。
- `create_node`：创建 node 时必须表达主题、scope、excluded scope、expected result。
- `assign_node`：显式把 node 委派给 subagent。
- `read_node_result`：主 agent 按需读取 node result，而不是全量重扫。
- `mark_result_validity`：标记 node result 的采信状态。
- `revise_map`：基于结果批量调整 node、edge、risk、open question。

第一版可以复用现有 `taskspace_control`，通过新增 action 或扩展参数实现；不新造并行 runtime。

### Phase P3：主 agent 一线执行降级

目标：减少 TaskSpace 退化成“主 agent 自己干活 + map 记账”。

策略：

- TaskSpace 模式下，主 agent 的普通工具调用默认应绑定到少数 planner 允许场景：
  - 读取 task/map/node inventory。
  - 轻量复核关键 node result。
  - 执行最终 synthesis 前的少量验证。
  - 简单任务的单节点直接执行。
- 中高复杂任务中，搜索、代码阅读、实现、测试优先通过 node 委派给 subagent。
- 如果主 agent 在同一 node 内连续进行大量普通工具调用，runtime 应触发 planner maintenance barrier，要求它先总结 node、更新 map、或委派子任务。

注意：

- 不做绝对禁止，否则会破坏小任务效率和异常恢复。
- barrier 的提示不能继续鼓励“创建更细 node”，而应鼓励“提升 node 主题边界、委派执行、或合并过细节点”。

### Phase P4：事实来源与输出契约

目标：降低错误前提扩散和局部工具选择造成的失败。

机制：

- task 初始化时要求主 agent 显式记录输入来源和输出契约。
- subagent 执行 node 时必须看到该 node 相关的 fact sources 和 output contracts。
- 对数据处理类任务，默认要求先确认输入数据来源，再生成输出。
- 禁止在未说明 provenance 的情况下把自造数据当成真实输入。
- 对文件输出，node context 应能携带编码、格式、路径约束。

runtime 不判断“事实来源是否真的正确”，但可以要求相关字段存在，并让 viewer/audit 可见。

### Phase P5：结果采信与质疑机制

目标：让主 agent 对 node result 保持谨慎信任，避免错误 result 直接污染全局 map。

机制：

- 每个 node result 默认是 `unreviewed` 或 `pending_review`。
- 主 agent 可以标记为 `accepted`、`questioned`、`superseded`、`invalid`。
- 被 `questioned` 的 result 不应作为下游 implementation 的唯一依赖。
- 被 `invalid` 的 result 仍保留在历史中，但不得作为 active map 的有效事实来源。
- `superseded` result 必须指向替代它的新 result 或 node。

第一版不需要质量评分，也不需要复杂信任模型。

### Phase P6：Viewer 与 E3 审计升级

目标：让人能看出 TaskSpace 是否真的在 Planner 化。

viewer 需要展示：

- task 目标和成功标准。
- task fact sources。
- task output contracts。
- node 的主题、scope、excluded scope、expected result。
- node 依赖关系。
- node result validity。
- questioned/invalid/superseded 的原因。
- 主 agent 是自己执行还是委派执行。

E3 audit 需要新增判断项：

- node 是否是高内聚主题任务。
- map 是否表达了任务模型，而不是行动日志。
- 主 agent 是否主要在调度，而不是亲自线性执行。
- subagent result 是否进入 node context 并被主 agent 使用。
- 错误前提是否被及时质疑或隔离。
- 输出契约是否在执行前被显式建模。

## Benchmark 观察指标更新

下一轮 E3 不能只看 pass/fail 和成本，还要观察 Planner 化行为：

| 指标 | 目标 |
|---|---|
| `node_theme_cohesion` | node 是否是高内聚主题任务 |
| `atomic_node_ratio` | 过细行动节点占比 |
| `main_direct_tool_ratio` | 主 agent 直接一线工具调用占比 |
| `delegated_node_ratio` | 委派给 subagent 的 node 占比 |
| `result_reuse_rate` | 主 agent 是否利用 node result |
| `source_provenance_present` | 是否建模事实来源 |
| `output_contract_present` | 是否建模输出契约 |
| `questioned_result_count` | 是否出现谨慎质疑机制 |
| `wrong_premise_containment` | 错误前提是否被隔离而非扩散 |
| `map_growth_health` | map 生长是否跟任务复杂度匹配 |

这些指标可以先作为 audit 字段，不立即变成硬门槛。

## 设计取舍

### 不引入多角色岗位体系

继续复用 Codex/Whale 现有 subagent 类型：

- `default`
- `explorer`
- `worker`

不新增 Scout/Reviewer/Judge 等类人岗位角色。当前问题不是角色不够多，而是主 agent 没有稳定 Planner 化。

### 不让 runtime 做语义选择

runtime 不能根据关键词、BM25 或 embedding 自动选择 task/node。语义选择由主 agent 做。

runtime 只提供 inventory 和结构协议。

### 不把质量分引入 runtime

复杂 agent 任务没有客观质量分。结果有效性由主 agent 显式标记，并通过 audit 和测试观察。

### 不追求完全禁止主 agent 执行

主 agent 完全不能执行会带来过度僵硬。第一版目标是默认调度、有限复核、必要时亲自处理简单任务。

## 下一步工程入口

优先改造路径：

1. 更新 `taskspace_control` schema，让 task/node 创建携带任务模型字段。
2. 更新 TaskSpace developer context，把主 agent 明确定位为长期 Planner。
3. 更新 runtime maintenance barrier 文案，避免鼓励过细 node。
4. 更新 node/result 数据模型，支持 result validity。
5. 更新 viewer snapshot 和网页展示。
6. 更新 E3 audit 模板和 benchmark report 生成。
7. 重新以 `jsonl-aggregator` 和 `heterogeneous-dates` 做小样本回归，观察 map 是否健康生长。

第一阶段成功标准：

- 中高复杂任务中，主 agent 不再把 map 当行动日志。
- node 标题和上下文能表达高内聚主题任务。
- subagent result 能沉淀到 node context，并被主 agent 显式采信或质疑。
- 输出契约和事实来源进入 task/map 可观察状态。
- `jsonl-aggregator` 不再围绕错误前提无限生长。

