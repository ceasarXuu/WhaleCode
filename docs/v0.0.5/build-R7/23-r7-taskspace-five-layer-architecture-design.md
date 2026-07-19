# R7 TaskSpace 五层交互架构设计

- Created: 2026-07-20
- Version: 1.0
- Status: Proposed
- Scope: TaskSpace instructions、working protocol、skills、tools、Runtime、projection 与反馈链
- Compatibility: 不保留旧协议兼容分支；迁移必须分阶段验证
- Related: [R7 三种 Projection 策略共享架构宪章](00-r7-three-projection-policy-charter.md)、
  [R7 双基础提示词设计](20-r7-dual-base-instructions-design.md)
- Risk Level: High

## 1. 执行摘要

TaskSpace 需要一套完整但不重叠的五层 Agent 交互架构：

1. **Base Instructions**：建立稳定的 Agent 工作框架和 TaskSpace 宏观认知。
2. **Core Working Protocol**：说明日常如何使用 Map 工作，只保留完成正常任务必需的方法。
3. **Advanced Skills**：按需提供复杂任务经验，不进入每个请求的固定上下文。
4. **Tool Contract**：准确描述可调用能力、输入输出和机械约束，不承担工作方法教学。
5. **Runtime and Factual Feedback**：维护唯一事实、执行硬规则并忠实反馈，不参与语义决策。

Agent 不是第六层。Agent 是五层能力的唯一语义使用者：任务拆解、节点目标、依赖、完成标准、行动选择、
失败恢复和最终总结全部由 Agent 决定。五层架构的目的不是管理 Agent 的思考上限，而是让 Agent 获得清晰的
工作模型、足够的能力、无歧义的机械合同和未经扭曲的事实。

本设计保留 R7 已建立的双 Base、唯一 canonical Map、Rooted DAG、Event Store、共享 Runtime、共享工具链和
三种 projection policy。三种策略之间唯一允许的差异仍是同一份 projection 如何进入 provider context；
不得因五层重构而产生三套提示词、工具、状态机或反馈链。

## 2. 为什么需要重构

### 2.1 已解决的问题

R7 双 Base 改造纠正了两个基础错误：

1. Standard 重新继承 Codex 已验证的完整 Agent 工作框架，不再使用功能不足的 Whale 极简提示词。
2. TaskSpace 不再只是一个孤立、低显著性的附加 developer message，而成为明确的默认工作方式。

这些结论继续有效，不回退。

### 2.2 当前结构的新问题

当前 TaskSpace Base 同时承担宏观认知、图模型、日常操作方法、失败恢复、部分调用时序和工具规则。它增强了
TaskSpace 的可见性，却把原本应该分层的信息重新集中到固定 Base 中：

- 每次请求都携带正常任务不一定需要的操作细节，增加固定输入成本。
- Base、工具顶层描述和 Runtime 错误可能用不同措辞表达同一规则，形成冲突和版本漂移。
- 高级经验没有合适位置：放入 Base 或 developer message 会永久占用上下文；放入 Tool 会污染机械合同；
  放入 Runtime 则会越界替 Agent 决策。
- 工具 schema 同时承担“告诉 Agent 为什么这样工作”和“限定 JSON 调用形状”，两种责任相互挤压。
- 反馈层曾多次出现裁剪、再解释或后置纠正倾向。Agent 异常容易被误判为智能不足，而真实原因可能是工具
  结果没有完整进入上下文、状态提交语义不清或 projection 过期。

问题不是 TaskSpace 方法本身没有价值，而是不同稳定性、不同成本和不同约束强度的信息缺少清晰归属。

## 3. 设计原则

### 3.1 单一语义所有权

同一规则只能有一个权威层。其他层可以引用其名称或版本，不得复制后换一种措辞重新解释。

### 3.2 Agent 负责语义，Runtime 负责底线

Runtime 是不可绕过的状态工具，不是 Agent 的上级。Runtime 可以拒绝违反图结构、revision、readiness、
binding、terminal、事务原子性或明确调用顺序的操作；不能判断一个命令“是否有用”、Patch 属于哪个节点、
测试是否充分，或替 Agent 选择下一步。

### 3.3 语义透传优先

工具结果、控制结果、失败原因和 projection 必须忠实进入上下文。允许机械封装、引用、分页和明确标注的
裁剪，不允许改变结论、抹去失败、混淆是否提交，或注入下一步建议。

Agent 出现重复、低级错误或异常成本时，诊断顺序固定为：先检查 provider payload、tool result、feedback、
projection 是否丢失、残缺、扭曲、重复或过期，再评估 Agent 能力；不得先增加 Runtime 语义限制。

### 3.4 固定内容按必要性分层

只有所有请求都需要的内容才进入 Base；只有所有 TaskSpace 正常任务都需要的操作方法才进入 Core Working
Protocol；低频复杂经验进入 Skill；机械调用合同进入 Tool；当前事实进入 Runtime feedback/projection。

### 3.5 结构约束不等于语义空白

Tool 必须准确说明一个 action 会做什么、何时成功、返回什么以及会产生什么副作用。否则 Agent 无法正确
选用能力。Tool 不应解释如何规划任务、怎样判断完成或采用何种工程策略。目标是“领域语义准确、策略语义
克制”，不是把 Tool 写成没有含义的字段表。

### 3.6 静态合同优先，避免破坏缓存

同一 TaskSpace profile 下的 Base、Core Protocol 和 tools schema 应保持稳定。不能按 Map revision 动态删改
tool schema 或固定前缀。三种 projection policy 共享相同五层合同，只有 Layer 5 的 projection emission 行为
按已冻结的 session policy 变化。

### 3.7 一次只验证一个策略变更

提示词、协议、Skill、Tool schema、Runtime gate 和 projection 不得在同一实验中同时改动。每个阶段都以
Standard、冻结基线和单变量候选进行可归因对比。

## 4. 五层目标架构

| 层 | 载体 | 生命周期 | 唯一职责 | 明确禁止 |
|---|---|---|---|---|
| L1 Base Instructions | TaskSpace 专用完整 base | profile 固定；每请求存在 | Agent 通用工程框架、TaskSpace 价值与宏观模型、责任边界 | 字段全集、动态状态、复杂案例、逐动作时序 |
| L2 Core Working Protocol | versioned developer message | TaskSpace 会话固定；每请求存在 | 正常任务必需的 Map 工作循环、基础恢复方法和常见反模式 | 重复 Base、枚举参数、动态事实、高级 playbook |
| L3 Advanced Skills | 内置 versioned Skill | 仅目录描述常驻；正文按需加载 | 复杂 DAG、长任务、重规划、证据冲突等高级经验 | 成为正确性前提、覆盖硬合同、被 Runtime 强制加载 |
| L4 Tool Contract | tools schema + output schema | profile 固定；每请求暴露 | 能力、action 语义、参数、返回值、副作用、机械调用形状 | 教授完整方法、推断工作语义、动态拼接 Map 状态 |
| L5 Runtime and Factual Feedback | canonical state、validator、result、projection | 运行时动态 | 保存事实、执行硬约束、原子提交、忠实反馈、纯渲染 | 建议下一步、修复 Agent 参数、解释任务语义、隐藏失败 |

### 4.1 L1：Base Instructions

TaskSpace Base 继续是 Codex 成熟 Base 的完整同构版本，而不是在 Standard 后追加的一段附件。TaskSpace 部分
只回答四个宏观问题：

1. Map 为什么存在，它如何补足线性上下文对复杂任务全局结构的表达不足。
2. Root、Work、dependency edge、Finish 和 active binding 分别代表什么。
3. Map 与自然上下文如何分工：Map 保存全局任务结构和状态，自然上下文保存详细交互与证据。
4. Agent 与 Runtime 的责任边界是什么。

Base 可以说明“Map 应随真实工作同步推进”，但不列出每种 action、字段、组合响应或错误恢复步骤。Base 中
保留成熟的编码、验证、沟通、工具使用和持久推进规则；TaskSpace 不应抛弃这些通用能力。

**目标体积**不是先验字数，而是“删除后会让所有 TaskSpace 请求失去共同认知”的最小稳定集合。任何新增
内容必须证明对所有 TaskSpace 请求都必要。

### 4.2 L2：Core Working Protocol

Core Working Protocol 是 Base 与 Tool 之间的常驻方法层。它不是全量手册，只说明 Agent 完成普通
TaskSpace 任务必须掌握的工作循环：

1. 在普通工具工作前建立与当前已知信息一致的 Map，并绑定首个可执行 Work。
2. 在绑定节点内执行服务于该目标的普通工具；独立动作可以并行，有依赖的动作等待结果。
3. 当真实工作目标切换时，先提交当前生命周期边界，再绑定 Ready 后继并携带第一个真实动作。
4. 当证据改变任务结构时，先由 Agent 修订 Map，再按新结构继续。
5. 在 Work 节点内完成验证；Agent 证据充分后，显式创建并闭合唯一 Finish，提交最终总结。
6. 按 `state_commit`、revision 和原始错误恢复，不推断未发生的提交或自动回滚。

协议说明行为次序和恢复原则，但不复制字段名全集。诸如 `expected_revision` 的类型、哪个 action 要求哪些
字段、合法 JSON 分支和返回 schema 全部以 L4 为准。

Core Protocol 独立维护 `protocol_version + sha256`。它在三种 projection policy 中字节一致，不能根据
policy 加入不同建议。当前 Base 中“使用 Map”的详细操作段落应在迁移期提取到这里，Base 只保留宏观内容。

把同一段文字从 Base 移到 developer message 本身不会节省 token：两者都属于每个请求的稳定前缀。L2 的
直接收益是职责隔离、独立版本和可归因实验。固定输入成本只有在删除跨层重复、缩短正常任务协议，或把低频
高级内容迁入 L3 后才会下降；不得把单纯搬迁报告为性能收益。

### 4.3 L3：Advanced Skills

高级经验放入内置 `taskspace-advanced` Skill。Provider 固定上下文只看到简短、稳定的 Skill 名称与触发
描述，完整正文仅在 Agent 判断任务匹配时加载。

第一版可覆盖：

- 多父依赖、并行分支与汇聚验证的 Map 设计。
- 长会话中的节点修订、废弃工作处理、折叠节点展开和 Map 全局视图恢复。
- Map 过扁、过碎、事后补账、节点目标与真实动作错位的诊断方法。
- Debug 中的竞争假设、证据冲突、blocked/rework 和独立复核。
- 复杂 Create 工作中脚手架、实现、集成、验证的依赖组织。
- compaction、resume、fork 后如何依据当前事实恢复工作，而不是重放旧动作。

Skill 只提供经验和示例，不新增 Runtime 规则。未加载 Skill 时，Agent 仍必须能依靠 L1、L2 和 L4 正确完成
普通任务。Runtime 不得根据任务复杂度自动注入 Skill 正文；是否加载属于 Agent 的语义选择。

### 4.4 L4：Tool Contract

Tool contract 是 Agent 可执行能力的权威接口。每个 action 分支必须独立说明：

- 该 action 对 Map 或读取结果产生的确切变化。
- 必填参数、互斥参数和边界条件。
- 是否只读、是否修改状态、是否幂等、是否可能失败。
- 成功与失败时稳定的结构化输出。
- 只有机械规则需要时才说明与同一响应中其他 tool call 的顺序关系。

Tool 顶层 description 只描述工具整体用途，具体语义下沉到对应 `oneOf` action 分支。禁止用一段超长顶层
文本重复所有 action，也禁止使用“Mechanical action variant”这类无法帮助选择的占位描述。

目标工具面采用最小的职责拆分：

| Tool | 职责 | Action 范围 |
|---|---|---|
| `taskspace_control` | 修改 canonical Map 或节点可见状态 | initialize、mutate、complete/continue、complete/end、finish/end、expand |
| `taskspace_read` | 只读获取当前 Map 或被引用的原始输出 | read_map、read_output_ref |

拆分依据不是“每个 action 一个 Tool”，而是读写权限、副作用和输出合同存在本质差异。两个 Tool 继续共享同一
TaskSpace service、Map、validator、result envelope 和日志，不得形成两套架构。该拆分必须作为独立候选验证；
若实测增加选择错误或成本且不能改善合同清晰度，则保留单 Tool，但仍必须在 schema 内清晰区分读写分支。

`complete_continue` 本身应表达“提交当前边界并继续”。若产品硬规则要求它不能成为响应末项，response
preflight 可以在执行任何调用前检查后续 sibling 是否实际存在；不再要求 Agent 同时填写一个与真实 sibling
重复的 `required_next_call` 声明。JSON Schema 无法单独约束另一个顶层 tool call 必须存在，不能假装该问题
已经被 schema 解决。是否移除当前字段必须做单变量 A/B，不与 Tool 拆分同时实施。

`read_output_ref` 的不同读取模式应使用互斥 `oneOf` 分支表达各自必填字段，而不是把所有字段设为 optional
后交给 Runtime 猜测。`transition_node` 之类的二级判别字段如果与 action 重复，也应由唯一 action 名取代。

所有 Tool schema 在同一 profile 内保持静态，记录 `tool_contract_version + tools_hash`。只有明确需要状态变化
的参数值来自 projection 或反馈，不通过每轮修改 schema 表达状态。

### 4.5 L5：Runtime and Factual Feedback

L5 包含三个紧密相连但责任清楚的子部件：

1. **Canonical state**：唯一 Rooted DAG、节点状态、边、revision、binding、Finish、Event Store 和引用数据。
2. **Hard validator/executor**：校验图连通性、状态迁移、revision、readiness、binding、terminal、权限、
   原子性及明确的工具调用顺序。
3. **Factual feedback/projection**：返回提交结果和原始工具事实，从 canonical Map 纯构造全局 projection。

控制结果使用稳定 envelope，至少包含：

```text
schema_version
status
success
state_commit
partial_commit
committed_revision
delta
steps
error { class, code, message, actual, expected }
```

错误必须作为 tool result 进入 Agent 上下文，使 Agent 能看到并自行纠正。`message` 只描述违反的机械合同、
实际值和期望值，不给出“你应该先修改哪个文件”之类的工作建议。对于事务预检失败，必须明确本批次是否一个
调用都未执行；对于控制已提交但后续普通工具失败，必须分别保留两个事实。

读取结果可以用稳定 envelope 携带 revision、范围、截断和 continuation reference，但 `content` 必须保持原始
字节语义。Projection 是 canonical Map 的确定性视图，不是第二份事实，也不是提示词：它只呈现节点、边、
状态、引用和明确的裁剪事实，不加入下一步建议、重要性判断或对工具结果的再解释。

`map-always`、`map-append`、`map-request` 只在 projection emission 上不同。它们共享完全相同的 canonical
state、renderer、Tool contract、Runtime gate 和反馈格式。

## 5. Agent 的主权边界

五层提供的是工作环境，不是决策流水线。以下事项只能由 Agent 决定：

- 用户目标如何拆解为 Work 节点。
- 一个节点是否应该依赖一个或多个前置节点。
- 一组代码改动应合并为一个连贯节点，还是拆为多个有独立完成标准的节点。
- 哪些工具可并行，哪些必须等待证据。
- 失败意味着修改假设、修订 Map、重试工具还是选择其他路径。
- 验证证据是否足够，何时显式进入 Finish 并总结。
- 是否加载高级 Skill，是否展开已折叠节点，何时主动读取 Map。

Runtime 可以指出“节点尚未 Ready”，不能指出“先修测试再修实现”；可以拒绝无效 revision，不能代替 Agent
改成最新 revision；可以拒绝未闭合图的 `finish_end`，不能自动完成剩余节点。

## 6. 信息流与权威关系

```text
User request
   |
   v
L1 Base + L2 Core Protocol + available Skill catalog
   |
   +---- Agent optionally loads L3 Advanced Skill
   |
   v
Agent semantic decision
   |
   v
L4 Tool call contract
   |
   v
L5 Runtime hard validation -> canonical commit/tool execution
   |
   v
L5 exact result + current projection according to session policy
   |
   +----> Agent reads facts and decides again
```

冲突优先级不按“哪段提示词更强”决定，而按职责决定：

1. 当前事实以 L5 canonical state 和已提交结果为准。
2. 合法调用形状以 L4 schema 为准。
3. 正常工作方法以 L2 为准。
4. 宏观工作模型与责任边界以 L1 为准。
5. L3 只能补充经验，不能覆盖 L1、L2、L4 或 L5。

## 7. 内容归属判定表

| 内容 | 唯一归属 | 理由 |
|---|---|---|
| TaskSpace 为什么有价值 | L1 | 所有请求都需要的心智模型 |
| Root/Work/edge/Finish 的概念 | L1 | 稳定领域模型，不是调用字段 |
| 初始化后开始真实工作 | L2 | 常规工作循环 |
| 节点切换时携带下一真实动作 | L2 | 日常方法；L4 只描述合法形状 |
| `expected_revision` 类型和必填条件 | L4 | 机械输入合同 |
| `complete_continue` 后必须有 sibling | L4 | L4 定义合同，L5 只机械执行原子预检 |
| 当前 revision、binding、Ready frontier | L5 | 动态事实 |
| 多父依赖的设计经验 | L3 | 低频高级方法 |
| stale revision 错误的实际值和期望值 | L5 | 本次执行事实 |
| stale revision 后应如何重规划 | Agent，可参考 L3 | 语义决策 |
| projection 的字段和裁剪标记 | L5 | canonical state 的纯视图 |
| 当前 session 的 projection policy | L5 session state | 用户配置后冻结，不由 Agent 临时切换 |

## 8. 当前实现审计

### 8.1 Base 与协议

当前 `whalecode_taskspace.md` 的 TaskSpace 章节已经系统说明价值、图模型、使用方法、失败恢复和 Runtime
边界，方向正确，但 L1 与 L2 尚未真正分开。原独立 Working Protocol 因与 Base 平行重复而被移除；这并不
意味着中间层不需要，而是旧实现缺少单一所有权。

优化方向：从 Base 提取“使用 Map”的操作循环到独立、短小、版本化的 L2；Base 继续保留宏观模型。提取时
先保持语义等价，不同时改写 Tool 或 Runtime。

### 8.2 Tool schema

当前 `taskspace_control` schema 约 10 KB，描述文本约 4.5 KB；顶层 description 约 1.6 KB，并集中复述多个
action。与此同时，若干 action 分支只有通用占位描述，具体 action 的局部可发现性不足。当前还存在：

- 一个 Tool 同时返回控制事务 JSON、原始 Map 文本和原始 output slice，输出合同不统一。
- `strict: false` 且没有 `output_schema`。
- `read_output_ref` 的模式相关字段主要由 Runtime 二次校验。
- action 与 `transition_node` 存在重复判别。
- `required_next_call` 是声明值，真正的 sibling 是另一项顶层调用，两者可能不一致或缺失。

优化方向：先重写 action-local 描述和条件 schema，再独立验证读写拆分、output schema、
`required_next_call` 简化和 strict mode；不得一次混改。

### 8.3 Runtime 与反馈

当前控制失败已经通过 `RespondToModel` 进入上下文，并包含 `state_commit: false` 等稳定事实；成功批次也返回
revision、delta 和逐步结果。这是正确基础，应保留。

待改进部分是让读结果和控制结果具备可声明、可验证的输出合同，并在截断时提供范围和 continuation reference。
任何封装都必须保留原始内容，不能用摘要替代结果。Runtime preflight 应只执行明确硬规则，离线 benchmark
可以标记 filler、事后补账和动作错位，但生产 Runtime 不得据此拒绝语义行为。

### 8.4 版本与观测

Base 已有独立版本、SHA-256 和 wire identity；五层尚未形成统一的 effective contract identity。缺少该身份时，
行为变化很难归因于 Base、协议、Skill、Tool 或 Runtime 中的哪一项。

每个 provider request 和 benchmark artifact 应记录：

```text
base_profile/version/sha256
core_protocol_version/sha256
loaded_skill_names/versions/sha256
tool_contract_version/tools_hash
runtime_contract_version/result_schema_version/renderer_version
projection_policy/projection_revision/projection_sha256
```

## 9. 外部 Tool 设计经验与反思

### 9.1 共同结论

主流 Agent 工具没有采用单一的“越原子越好”或“越聚合越好”规则。合理边界由意图、副作用、权限、输出
合同和真实高频工作流共同决定：

- Claude Code、Gemini CLI 使用独立的读、搜索、编辑、Shell 和任务工具，强调可预测的能力边界。
- GitHub MCP 对同一资源使用 `method` 聚合相关操作，同时用 toolsets 控制工具数量和权限面。
- Playwright MCP 对独立浏览器动作拆分 Tool，对同类 tab 操作使用 action 枚举。
- OpenCode Read 通过 offset/limit、大小上限、继续读取提示和可行动错误渐进暴露内容。
- Cline 同时提供聚合读取和独立 Patch/Search/Bash，说明粒度应服务于工作流而非教条。

因此 TaskSpace 不应拆成十几个 action Tool，也不应把读取、状态写入、方法教学和动态事实全部塞进一个超长
Tool。读写双 Tool 是值得验证的最小边界，而不是新增架构分支。

### 9.2 描述与 schema

Anthropic 和 VS Code 的官方经验都强调：Tool 描述应准确说明用途、适用条件、返回值、限制和参数含义；复杂
输入应给出结构化示例。反思是 Tool 不能“无语义”，但语义必须局限于能力合同，不扩张为 Agent 工作手册。

MCP 将 Tool 的 `inputSchema`、可选 `outputSchema`、结构化结果及 read-only/destructive/idempotent 等注解
作为协议能力。TaskSpace 应利用这些结构表达机械事实，而不是把所有约束塞进自然语言顶层 description。

OpenAI Structured Outputs 证明严格 schema 可以提升参数形状一致性，但 DeepSeek 当前 strict tool calls 位于
Beta 路径，并要求对象属性全部 required、`additionalProperties: false` 等约束。R7 不能直接把现有复杂 schema
切到 strict；必须先做 provider 兼容、并行工具、缓存和错误行为的独立实验。

### 9.3 错误反馈

MCP SEP-1303 的关键经验是：模型可纠正的输入校验失败应作为 Tool 执行结果返回给模型，而不是只作为协议层
错误被宿主吞掉。TaskSpace 当前 `RespondToModel` 方向正确。下一步不是让 Runtime 解释“应该怎么工作”，而是
补齐 action、actual、expected、state_commit 和 revision 等可操作事实。

### 9.4 渐进暴露与工具数量

Anthropic、OpenCode 和 OpenAI Agents SDK 都建议通过范围、分页、过滤、截断引用、tool search 或 namespace
降低上下文和选择成本。TaskSpace 可对超长 output 使用渐进读取，但 Map projection 本身承担全局视图职责，
不能简单分页到 Agent 看不见全局。Projection 可降低远端节点细节，并提供精确引用；连骨架都超限时需要另立
Map 专用压缩专项，不能假装普通分页已经根治。

### 9.5 评测而不是凭直觉优化

Anthropic 明确建议用真实 Agent 任务评测工具，包括结果质量、tool calls、token、Runtime 和错误。TaskSpace
此前已经证明单次样本容易被模型波动或异常 request 误导。任何层的变更都要通过重复、held-out、逐 request
trace 验证，不能为了让样本触发目标机制而构造自问自答式任务。

## 10. 分阶段迁移方案

本节使用 `FLA`（Five-Layer Architecture）编号，是 R7 内部的专项迁移序列，不重编号或覆盖
`01-r7-phased-implementation-plan.md` 中已有的 R7 Phase。

### FLA-0：冻结五层基线

- 记录当前 Base、Tool、Runtime、projection 的字节身份和完整 provider payload。
- 把当前缺失的 Core Protocol 和 Advanced Skill 标记为 `absent`，不伪造版本。
- 跑 Standard + 当前 TaskSpace simple/complex 各 3 次，保存 request、token、cache、耗时、动作和 Map。

验收：任一结果都能关联到完整五层身份；不改变生产行为。

### FLA-1：提取 L2，收敛 L1

- 从 TaskSpace Base 提取日常工作循环和基础恢复到独立 Core Protocol。
- Base 只保留价值、图模型、Map/对话分工和 Agent/Runtime 边界。
- 做内容归属扫描，确保同一句机械规则不在 Base、Protocol 和 Tool 三处重复。

验收：语义覆盖不减少；Standard 零注入；TaskSpace 行为不退化；只把搬迁记录为架构变化，不虚报降本。

### FLA-2：建立 L3 Advanced Skill

- 创建 `taskspace-advanced`，只放复杂场景经验和示例。
- 定义清晰触发描述，正文不自动注入。
- 选择确实需要高级方法的复杂样本和不需要 Skill 的简单样本分别验证。

验收：简单任务不加载也正确；复杂任务由 Agent 主动加载后有可观察收益；Skill 不产生新硬规则。

### FLA-3：重构 L4 描述与 action schema

- 缩短顶层 description，补全 action-local 语义。
- 用 `oneOf` 表达 read mode 和 action 的互斥必填字段。
- 删除重复 discriminator，生成并记录 tools hash。
- 本阶段不改变 Runtime 行为或 Tool 数量。

验收：schema contract tests、provider wire identity、首次正确调用率和缓存均不退化。

### FLA-4：逐项验证 Tool 能力边界

按独立实验依次验证，不叠加：

1. `taskspace_control` / `taskspace_read` 读写拆分。
2. 稳定 `output_schema` 与 read result envelope。
3. 移除冗余 `required_next_call` 声明，由 action 合同加原子 preflight 验证真实 sibling。
4. DeepSeek strict mode 兼容实验。

每项失败都回到上一冻结基线，不保留兼容分支。

### FLA-5：L5 反馈与观测收口

- 为 success、protocol failure、ordinary tool failure、截断读取补全稳定结构和日志。
- 验证被拒批次、部分执行禁止、已提交控制加后续工具失败等边界。
- 静态审计 Runtime 不含命令内容分类、Patch 意图判断、测试充分性判断或 next-action 建议。

验收：反馈完整进入模型上下文，state commit 无歧义，projection 字节可复现。

### FLA-6：正式对照与决策

- 对每个接受的单变量版本运行 Standard、冻结 TaskSpace 基线和当前候选。
- simple、complex、held-out adversarial sample 各至少 3 次，Docker 环境统一并允许并行。
- 逐 request 审计，而不是只看总均值。

验收后再决定合并；没有证据的层次优化不得进入生产。

## 11. 测试与收益指标

### 11.1 正确性

- 用户任务公开与隐藏验证结果。
- Map 唯一 Root/Finish、全节点可追溯、依赖和生命周期合法。
- ordinary tool 始终归属有效 binding。
- 控制失败、工具失败和状态提交语义完整进入上下文。

### 11.2 行为质量

- 初始化前普通工具次数。
- 单独非终局 lifecycle transition 次数。
- 合法的 control + next action 合并率。
- 重复读取、重复测试、filler/no-op、事后补 Map 和动作/节点错位。
- Map 过扁、过碎、错误依赖及复杂任务中多父边使用情况。

这些指标用于观察和离线诊断，不自动升级为 Runtime 语义 gate。

### 11.3 成本

- requests 总和、均值、中位数和逐次分布。
- input、cached input、uncached input、output 和 reasoning tokens。
- provider 缓存命中率、request 级 LCP、Base/Protocol/Skill/Tool/projection 各自估算占比。
- wall time、provider time、tool time、control time。
- tool schema bytes、固定前缀 bytes、每轮 projection bytes。

### 11.4 实验纪律

- 每次只改变一层中的一个策略。
- 同一 commit、模型、配置、Docker image、样本和运行脚本。
- 结果同时给出总和、均值、中位数，不用单一异常运行代表整体。
- 失败样本先做 trace 根因分析，不能直接剔除。
- 用 held-out 样本防止提示词和 Tool 描述迎合已知测试。

## 12. 验收标准

五层重构完成必须同时满足：

1. 每条规则都有且只有一个权威层，并可通过内容清单追踪。
2. 不加载 Advanced Skill 也能完成普通 TaskSpace 工作。
3. Base 不包含 action 字段全集，Core Protocol 不复制 schema，Tool 不教授完整工作方法。
4. Tool 的每个 action 都有局部、准确、可选择的语义描述和明确输入输出合同。
5. Runtime 不做语义判断、不自动修正 Agent 决策、不注入下一步建议。
6. 所有失败都忠实进入 Agent 上下文，并准确说明是否提交状态。
7. Projection 保持全局性、确定性和事实性；任何裁剪都有明确标记与引用。
8. 三种 projection policy 除 emission 外共享完全相同的五层实现与版本。
9. Standard 不注入 TaskSpace Base、Protocol、Tool 或 Skill 正文。
10. simple 与 complex 正确性不退化，成本变化可归因，缓存没有因动态 schema 或前缀漂移受损。

## 13. 非目标

- 本设计不改变 R6/R7 Rooted DAG 领域模型。
- 本设计不选择 `map-always`、`map-append` 或 `map-request` 中的最终胜者。
- 本设计不让 Runtime 解析 reasoning、命令意图、Patch 语义或测试质量。
- 本设计不把所有 TaskSpace 知识放入一个永远加载的 developer message。
- 本设计不要求每个 action 拆成一个 Tool，也不预设读写拆分一定获益。
- 本设计不在同一阶段完成 Map 骨架最终超限的通用压缩方案。
- 本设计不为旧 Working Protocol、旧 schema 或旧 session 增加兼容分支。

## 14. 待独立验证的决策

以下方向有明确工程假设，但尚不能写成已验证事实：

1. L1/L2 去重及 L3 高级内容按需化后，能降低多少固定上下文成本；单纯提取 L2 不计为降本。
2. 内置 Skill 的触发描述能否让 Agent 在复杂任务主动加载，同时不污染简单任务。
3. 读写双 Tool 是否比单 Tool 更易选、更清晰，且不会增加 request 或选择错误。
4. 移除 `required_next_call` 声明后，action-local 合同加 preflight 能否稳定保留合并 request。
5. `output_schema` 对 DeepSeek 的结果稳定性、tool loop、并行调用和缓存是否有正收益。
6. DeepSeek strict mode 是否适合当前复杂 union schema。

这些问题必须按 FLA-4 的单变量顺序回答，不能在设计文档中用直觉提前宣布成功。

## 15. 外部依据

1. [Anthropic: Define tools](https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools)
   强调详细、准确的用途、参数、限制和高信号结果。
2. [Anthropic: Writing effective tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents)
   主张用真实评测迭代工具边界、减少重叠、支持分页和可行动错误。
3. [Claude Code tools reference](https://code.claude.com/docs/en/tools-reference)
   展示读、编辑、搜索、Shell、任务和 Skill 的职责拆分。
4. [Claude Code features overview](https://code.claude.com/docs/en/features-overview)
   区分常驻项目约定、按需 Skill、外部工具和生命周期 Hook。
5. [Gemini CLI tools](https://google-gemini.github.io/gemini-cli/docs/tools/)
   展示独立文件、搜索、Shell、Todo 工具及工具结果回传模型的路径。
6. [OpenCode Read tool source](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/tool/read.ts)
   展示 offset/limit、大小上限、继续读取提示和错误建议。
7. [Cline SDK tools](https://docs.cline.bot/sdk/tools)
   展示聚合读取与独立 Patch、Search、Bash 工具并存的实际工具面。
8. [GitHub MCP Server](https://github.com/github/github-mcp-server)
   展示 toolsets、按资源聚合 method 和只读权限面的组合设计。
9. [Playwright MCP](https://github.com/microsoft/playwright-mcp)
   展示动作拆分、同类枚举、快照输出和上下文成本控制。
10. [MCP Tools specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
   定义 input/output schema、结构化内容、结果校验和错误反馈。
11. [MCP SEP-1303](https://modelcontextprotocol.io/seps/1303-input-validation-errors-as-tool-execution-errors)
    说明模型可纠正的校验错误应作为 Tool 结果进入模型上下文。
12. [VS Code Language Model Tool API](https://code.visualstudio.com/api/extension-guides/ai/tools)
    强调 Tool 命名、用途、返回、适用条件、限制和模型可读错误。
13. [OpenAI Structured Outputs](https://openai.com/index/introducing-structured-outputs-in-the-api/)
    说明严格 schema 与仅保证 JSON 合法之间的区别。
14. [OpenAI Agents SDK tools](https://openai.github.io/openai-agents-python/tools/)
    展示 tool namespace、按需工具搜索和 docstring/schema 生成。
15. [DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls)
    记录 strict mode 的 Beta 入口、支持范围和 schema 限制。
