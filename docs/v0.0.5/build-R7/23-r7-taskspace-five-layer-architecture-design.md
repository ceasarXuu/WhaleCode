# R7 TaskSpace 五层交互架构设计

- Created: 2026-07-20
- Version: 1.4
- Status: Proposed - concrete contract review required
- Scope: TaskSpace instructions、working protocol、skills、tools、Runtime、projection 与反馈链
- Compatibility: 不保留旧协议兼容分支；迁移必须分阶段验证
- Related: [R7 三种 Projection 策略共享架构宪章](00-r7-three-projection-policy-charter.md)、
  [R7 双基础提示词设计](20-r7-dual-base-instructions-design.md)、
  [R7 五层具体合同评审稿](24-r7-taskspace-five-layer-concrete-contract-draft.md)
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

本设计保留 R7 已建立的“每个 profile 只选择一份完整 Base”、唯一 canonical Map、Rooted DAG、Event Store、
共享 Runtime、共享工具链和三种 projection policy。L2 是从 TaskSpace Base 提取出的独立协议 artifact，不是
第二份 Base；它对旧“双 Base”文档中“不得存在附加协议”的限制构成有条件后续修订，只有通过本设计的迁移
门禁后才替代当前实现。三种策略之间唯一允许的差异仍是同一份 projection 如何进入 provider context；不得
因五层重构而产生三套提示词、工具、状态机或反馈链。

本文件只定义架构与内容所有权，不能单独作为实施依据。L1/L2 的逐字提示词、L3 Skill 样例、L4 Tool schema、
L5 result/projection 以及 TaskSpace 相关 provider payload 结构示例统一放在
`24-r7-taskspace-five-layer-concrete-contract-draft.md`。具体合同未经过用户逐项审阅前，不得把本设计视为产品
内容已冻结，也不得开始 FLA-2 之后的提示词、Skill 或 Tool 实施。

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

同一产品规则只能有一个权威层。其他产品层可以引用其名称或版本，不得复制后换一种措辞重新解释。

该所有权只约束 WhaleCode 自己维护的版本化 artifact，包括 L1、L2、bundled L3、L4 和 L5 模板；不扫描或
裁决用户输入、仓库 `AGENTS.md`、外部 Tool result 等动态自然语言。产品 artifact 之间相互矛盾是发布缺陷，
必须在装配前失败或由合同测试阻止；动态内容仍按 provider 的角色/顺序忠实传递，Composer 不做语义检测。

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

同一 TaskSpace profile 和同一可见 capability set 下的 Base、Core Protocol 和 tools schema 应保持稳定。
不能按 Map revision 动态删改 tool schema 或固定前缀。若权限、provider 能力或普通工具集合改变 schema，
必须形成新的 `capability_set_hash + tools_hash` 并进入 request 身份，不能仍声称合同未变。三种 projection
policy 共享相同五层合同，只有 Layer 5 的 projection emission 行为按已冻结的 session policy 变化。

### 3.7 一次只验证一个策略变更

提示词、协议、Skill、Tool schema、Runtime gate 和 projection 不得在同一实验中同时改动。每个阶段都以
Standard、冻结基线和单变量候选进行可归因对比。

## 4. 五层目标架构

五层是 **Agent 可见信息的五种权威面**，不是五个按顺序调用的代码模块。Provider Context Composer 是共享、
无语义的传输基础设施：它按显式 manifest 把五层内容放入 provider payload，但不拥有 TaskSpace 规则、事实或
工作方法，因此不是隐藏的第六个语义层。Renderer、emission policy、history reducer 和 composer 可以是不同
代码模块，不能因为产品上统称 L5 就混淆各自状态所有权。

| 层 | 载体 | 生命周期 | 唯一职责 | 明确禁止 |
|---|---|---|---|---|
| L1 Base Instructions | TaskSpace 专用完整 base | profile 固定；每请求存在 | Agent 通用工程框架、TaskSpace 价值与宏观模型、责任边界 | 字段全集、动态状态、复杂案例、逐动作时序 |
| L2 Core Working Protocol | versioned stable instruction section | TaskSpace 会话固定；每请求存在 | 正常任务必需的 Map 工作循环、基础恢复方法和常见反模式 | 重复 Base、枚举参数、动态事实、高级 playbook |
| L3 Advanced Skills | 内置 versioned Skill | 仅目录描述常驻；正文按需加载 | 复杂 DAG、长任务、重规划、证据冲突等高级经验 | 成为正确性前提、覆盖硬合同、被 Runtime 强制加载 |
| L4 Tool Contract | provider-visible tool definition + result algebra | profile/capability set 固定；每请求暴露 | 能力、action 语义、参数、返回值、副作用、机械调用形状 | 教授完整方法、推断工作语义、动态拼接 Map 状态 |
| L5 Runtime and Factual Feedback | canonical state、validator、result、projection | 运行时动态 | 保存事实、执行硬约束、原子提交、忠实反馈、纯渲染 | 建议下一步、修复 Agent 参数、解释任务语义、隐藏失败 |

### 4.0 Provider 暴露与装配合同

架构中的“Base”“developer”“Skill”“Tool”首先是 provider-neutral 的逻辑载体，不能假设所有 provider 都有
相同角色。DeepSeek Chat Completion 当前只公开 `system`、`user`、`assistant`、`tool`；WhaleCode 适配器会把
内部 `developer` item 序列化为 `system`。因此 L1 与 L2 的职责差异不能依赖 provider 角色强弱，而必须依赖
内容去重、固定顺序、版本身份和冲突测试。

DeepSeek Chat 的目标 wire 顺序固定为：

| 顺序 | 逻辑来源 | DeepSeek wire | 持久化/变化 | 约束 |
|---:|---|---|---|---|
| 1 | L1 TaskSpace Base | 第一条 `system` | profile 固定 | 每个请求恰好一份完整 Base |
| 2 | L2 Core Protocol | 聚合 developer bundle 的首个稳定 section，最终为第二条 `system` | protocol 固定 | 独立 artifact/hash；不得与 L1 重复 |
| 3 | 其他 developer sections 与 L3 Skill catalog | 同一聚合 bundle 的后续稳定 section，最终仍为第二条 `system` | 配置/目录固定 | 顺序确定；目录截断可观测 |
| 4 | 用户输入、自然历史、已加载 L3 Skill body | `user`/`assistant`/`tool` 自然历史 | 追加；受 compaction | Skill body 使用明确边界标签，不获得更高权限 |
| 5 | L4 Tool definitions | 顶层 `tools` | capability set 固定 | schema hash 与实际发送字节一致 |
| 6 | L5 tool result / projection emission | 相邻 `tool` result 或策略指定的尾部 item | 动态 | 只含事实；位置由已冻结 policy 决定 |

Composer 只接受已经渲染并带身份的 section，不读取任务语义。它负责固定顺序、载体映射、历史持久化、
projection 替换/追加/不注入、retry 去重和 payload 观测。跨层冲突在进入 Composer 前由 ownership manifest 与
合同测试消除；Composer 不做自然语言冲突判定。

### 4.1 L1：Base Instructions

TaskSpace Base 继续是 Codex 成熟 Base 的完整同构版本，而不是在 Standard 后追加的一段附件。TaskSpace 部分
只回答四个宏观问题：

1. Map 是什么：它是任务目标、工作节点、依赖关系、当前进度和完成路径的全局工作视图。
2. Root、Work、dependency edge、Finish 和 active binding 分别代表什么。
3. 如何使用 Map：按 Map 组织和推进工作，并在真实工作边界同步其结构与状态。
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

L2 在源码和版本管理上是独立 artifact，在 provider payload 中不承诺存在原生 `developer` role。当前
DeepSeek Chat 接线把它固定放在聚合 developer bundle 的第一段，随后整个 bundle 映射为 `system`。这仍然只有
一份 L1 Base；L2 不包含产品身份、通用编码框架或另一套 TaskSpace 心智模型。未来 provider 若原生支持
developer role，只允许改变机械 carrier，L2 字节、顺序身份和语义所有权保持不变，并通过 provider payload
snapshot 单独验证。

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

第一版 Skill 使用现有 bundled system skill 管线，但 session catalog 不直接指向可热更新的 `current` 文件。
启动时把已验证正文放入内容寻址快照
`$CODEX_HOME/skills/.system/.snapshots/<bundle_sha>/taskspace-advanced/SKILL.md`，TaskSpace session 锁定
`name + skill_version + body_sha256 + snapshot_path`。只在 TaskSpace profile 的 available-skills catalog 中暴露该
不可变路径。新 session 可以选择新 bundle；存量 session、resume 和 fork 不静默换版。

现有宿主有两条不同加载路径，设计不得把它们混称为同一种注入：

1. **用户显式点名**：结构化 Skill input 或明确 `$taskspace-advanced` mention 在 provider request 前由宿主
   resolver 命中 session 快照，并把正文作为带 `<skill>` 边界的 `user` item 注入。
2. **Agent 自主选择**：Agent 根据 catalog 决定使用后，通过已有文件读取能力打开 catalog 的 snapshot path；
   正文作为普通工具 result 忠实进入自然历史。现有 implicit invocation detector 只记录 telemetry，不额外注入
   第二份 `<skill>` 正文。

两条路径的 carrier 不同，但必须读取同一 snapshot bytes 并报告同一 body hash。禁止新增 Runtime 自动加载、
根据任务语义选择 Skill，或在 Agent 读取后再次注入正文。生命周期合同：

| 项目 | 合同 |
|---|---|
| 发现 | catalog 暴露稳定 `name + description + path`；描述只写适用条件、非适用条件和能力范围 |
| 预算 | 沿用全局 Skill metadata budget；必须记录 kept/omitted/truncated、原始/发送字节和 catalog hash |
| 冲突 | `taskspace-advanced` 名称保留给 bundled skill；同名用户/仓库 Skill 不能静默覆盖，resolver 返回显式冲突 |
| 版本 | catalog 锁定内容寻址 snapshot；显式注入和 Agent 读取都校验 body hash；热更新只对新 session 生效 |
| 触发 | 用户显式点名走宿主注入；Agent 根据 catalog 主动读取走普通文件 Tool；Runtime/policy/error 不自动加载 |
| 载体 | 显式路径为 `<skill>` user item；Agent 路径为原始 tool result；两者记录同一 canonical skill identity |
| 驻留 | 读取后的正文是自然历史的一部分，在当前 context epoch 中持续可见；不得每轮重新注入 |
| 压缩/恢复 | 不保证正文永久存在；记录 identity/裁剪事实但不自动重载。Agent 需要时重读锁定 snapshot path |
| resume/fork | 恢复父 session 的 snapshot identity；快照缺失返回 `skill_snapshot_missing`，不得解析为最新版 |
| 失败 | 缺失、禁用、hash 不匹配或读取失败作为机械事实反馈；普通任务继续，不静默补内联手册或 fallback 最新版 |

高级 Skill 不得承载 compaction/resume 的最低正确恢复步骤；这些步骤必须在 L2 和 L4 足够完成。L3 只能改善
复杂 Map 设计质量。评测必须区分 `catalog_not_visible`、`description_truncated`、`not_selected`、`load_failed` 和
`loaded`，不能只记录最终 Skill 名称。

### 4.4 L4：Tool Contract

Tool contract 是 Agent 可执行能力的权威接口。每个 action 分支必须独立说明：

- 该 action 对 Map 或读取结果产生的确切变化。
- 必填参数、互斥参数和边界条件。
- 是否只读、是否修改状态、是否幂等、是否可能失败。
- 成功与失败时稳定的结构化输出。
- 只有机械规则需要时才说明与同一响应中其他 tool call 的顺序关系。

Tool 顶层 description 必须用足够但不重复的文字说明工具做什么、何时使用/不使用、读写范围、主要副作用和
结果形态。复杂 Tool 以约 3 至 4 个高信息密度句子为起点，再把每个 action 的参数、前置条件、返回和边界
下沉到对应分支。禁止用一段超长顶层文本复述全部 action，也禁止使用“Mechanical action variant”这类无法
帮助选择的占位描述。结构敏感且文字仍不足以消歧的分支可以给最小输入示例，但示例必须计入固定 token 成本。

目标工具面采用最小的职责拆分：

| Tool | 职责 | Action 范围 |
|---|---|---|
| `taskspace_control` | 修改 canonical Map 或节点可见状态 | initialize、mutate、complete/continue、complete/end、finish/end、expand |
| `taskspace_read` | 只读获取当前 Map 或被引用的原始输出 | read_map、read_output_ref |

拆分依据不是“每个 action 一个 Tool”，而是读写意图、副作用和输出合同存在本质差异。命名或 MCP annotation
本身不构成权限边界；只有 ordinary tool router、approval policy 和执行校验真正隔离写权限时，才能对用户声称
read-only。两个 Tool 继续共享同一 TaskSpace service、Map、validator、result algebra 和日志，不得形成两套
架构。该拆分必须作为独立候选验证；若实测增加选择错误或成本且不能改善合同清晰度，则保留单 Tool，但仍必须
在 schema 内清晰区分读写分支。

`complete_continue` 本身应表达“提交当前边界并继续”。若产品硬规则要求它不能成为响应末项，response
preflight 可以在执行任何调用前检查后续 sibling 是否实际存在；不再要求 Agent 同时填写一个与真实 sibling
重复的 `required_next_call` 声明。JSON Schema 无法单独约束另一个顶层 tool call 必须存在，不能假装该问题
已经被 schema 解决。是否移除当前字段必须做单变量 A/B，不与 Tool 拆分同时实施。

`read_output_ref` 的不同读取模式应使用带明确 discriminator 的 `anyOf` 分支表达各自必填字段，而不是把所有
字段设为 optional 后交给 Runtime 猜测。选择 `anyOf` 是因为 DeepSeek strict 当前公开支持集合不包含
`oneOf`；每个分支使用 DeepSeek 支持的单值 `enum` discriminator 保证只有一个合法形状。`transition_node` 之类的二级判别字段
如果与 action 重复，也应由唯一 action 名取代。

所有 Tool schema 在同一 `profile + provider_schema_profile + capability_set` 内保持静态，记录
`tool_contract_version + capability_set_hash + tools_hash`。只有明确需要状态变化的参数值来自 projection 或
反馈，不通过每轮修改 schema 表达状态。

L4 还拥有 provider-neutral 的结果代数定义，但不假设 provider 能看到 `output_schema`。DeepSeek Chat 当前工具
定义只接收 name、description 和 input parameters，WhaleCode 适配器也只转发这些字段；模型实际看到的是后续
`tool` message 内容。完整 result algebra 必须先由 Runtime 全分支实现并做本地 schema conformance，再决定：

- DeepSeek Chat：发送稳定、模型可读的 JSON result；`output_schema` 仅用于本地校验和观测，不虚报 provider 约束。
- MCP 或未来原生支持的 carrier：可以暴露同一份 `outputSchema`，不得复制一套不同结果语义。
- strict input：只有适配器真实转发 `strict`、使用 DeepSeek Beta endpoint、所有同时暴露的函数都兼容其 schema
  子集，并通过 parallel tool probe 后，才可进入单变量候选。

### 4.5 L5：Runtime and Factual Feedback

L5 是动态事实和机械执行的权威面，内部不能压成一个职责不清的“大 Runtime”。共享实现至少拆清以下数据流：

| 子部件 | 输入 | 输出 | 可拥有状态 | 禁止拥有 |
|---|---|---|---|---|
| Canonical store/reducer | 已验证事件 | Rooted DAG snapshot、revision、Event Store | Map、事件、引用数据 | provider message、工作建议 |
| Hard validator/executor | command、canonical snapshot、capability/permission snapshot | commit events 或机械 rejection | 无第二份 Map；只持事务临时态 | 任务意图、测试充分性 |
| Projection renderer | canonical snapshot、renderer version | 确定性 projection bytes/hash | 无 | emission 时机、历史消息 |
| Projection policy | session policy、projection identity、context epoch metadata | replace/append/no-auto-emission directive | 冻结 policy、最近 emission identity | Map 内容、自然语言改写 |
| Context-history reducer | 已提交自然历史、compaction/resume/fork 事件 | 当前不可变历史 epoch | provider history 与裁剪事实 | canonical Map |
| Provider Context Composer | L1-L4 已渲染 sections、history、L5 directive/result | 有序 provider payload 与 wire identity | 无领域事实 | 冲突消解、摘要、下一步建议 |

以上是 L5 背后的实现部件，不是新增 Agent 教学层。依赖方向固定为 canonical store -> renderer -> emission
directive -> composer；history reducer 与 canonical store 分别维护 provider 历史和 Map 事实，不能互相替代。
retry、resume、compaction 和 fork 必须能够从 canonical snapshot、事件与冻结 policy 重放出相同 emission
decision；Composer 不能通过读取旧 projection 推断当前 Map。

L4 先冻结完整 result algebra，L5 的 preflight、参数解析、状态拒绝、成功提交、普通工具失败和读取截断全部原子
实现这一合同。控制结果的共同 envelope 至少包含：

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
state、renderer、Tool contract、Runtime gate、result algebra、history reducer 和 Composer。

`map-request` 下“最后一次读取到的 projection”不自动等于当前状态。任何模型可见 projection 必须携带自身
revision；只有 `visible_projection_revision == latest_known_canonical_revision` 时才能称为 current。控制调用
提交新 revision 后，先前读取结果立即成为历史事实，Runtime 不因 Agent 尚未重读而拒绝符合硬约束的 ordinary
action，但 Tool description、Base 和反馈都不得把旧 projection 描述成当前。必须覆盖
read(rev N) -> mutate(rev N+1) -> no read 的合同测试。

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
Provider Context Composer mechanically assembles
L1 Base + L2 Core Protocol + available Skill catalog + history + L4 tools
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
L5 exact result + projection emission directive according to session policy
   |
   v
Composer emits the result/projection without reinterpretation
   |
   +----> Agent reads facts and decides again
```

以下列表是内容所有权，不是让模型在冲突发生后执行的“优先级”：

1. 当前事实以 L5 canonical state 和已提交结果为准。
2. 合法调用形状以 L4 schema 为准。
3. 正常工作方法以 L2 为准。
4. 宏观工作模型与责任边界以 L1 为准。
5. L3 只能补充经验，不能覆盖 L1、L2、L4 或 L5。

实现必须维护机器可读的 product ownership manifest，至少为 WhaleCode 自有规范性规则记录稳定 rule id、owner
layer、artifact、version 和允许的跨层引用。构建期检查同一 rule id 只有一个正文所有者；provider payload
snapshot 检查 L1/L2 顺序、产品段落重复和禁止短语。用户输入、仓库 `AGENTS.md` 和外部 Tool result 不加入该
manifest，也不由 Runtime/Composer 做自然语言冲突检测；它们按既有 instruction hierarchy 和 wire 顺序原样
进入上下文。即使动态文本要求绕过 TaskSpace，Runtime 仍只在实际调用时执行既有硬不变量并忠实报错，不拒绝、
改写或解释用户文本。

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
- `strict: false` 且没有内部统一的 result schema；即使新增 `output_schema`，当前 DeepSeek Chat adapter 也不会
  把它发送给 provider。
- `read_output_ref` 的模式相关字段主要由 Runtime 二次校验。
- action 与 `transition_node` 存在重复判别。
- `required_next_call` 是声明值，真正的 sibling 是另一项顶层调用，两者可能不一致或缺失。

优化方向：先重写 action-local 描述和条件 schema；再冻结并实现全分支 result algebra；最后分别验证读写拆分、
`required_next_call` 简化、MCP `outputSchema` 暴露和 DeepSeek strict mode。不得一次混改，也不得把本地 schema
校验误报为 provider 能力。

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
developer_bundle_section_order/sha256
skill_catalog_hash/kept/omitted/truncated
skill_load_status/reason/names/versions/sha256/bytes
tool_contract_version/provider_schema_profile/capability_set_hash/tools_hash
runtime_contract_version/result_algebra_version/renderer_version
projection_policy/projection_revision/projection_sha256
visible_projection_revision/canonical_revision/projection_age
history_epoch/emission_directive/retry_deduplicated
wire_roles/wire_section_order/wire_prefix_hash
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
作为协议能力，但 annotations 只是提示，不能替代宿主授权。TaskSpace 应在 carrier 支持时利用这些结构表达机械
事实，而不是把所有约束塞进自然语言顶层 description；DeepSeek Chat 不支持的字段只用于本地合同，不能假装
已经暴露给模型。

OpenAI Structured Outputs 证明严格 schema 可以提升参数形状一致性，但 DeepSeek 当前 strict tool calls 位于
Beta 路径，并要求对象属性全部 required、`additionalProperties: false` 等约束，公开子集只列出 `anyOf` 而非
`oneOf`。当前 WhaleCode Chat adapter 还没有转发 `strict`。R7 不能直接把现有复杂 schema 切到 strict；必须
先完成 adapter wire probe，再做并行工具、缓存和错误行为的独立实验。

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

### FLA-0：冻结 wire 与行为基线

- 记录当前 Base、developer bundle、Skill catalog、Tool、Runtime、projection 和最终 provider payload 的字节身份。
- 把当前缺失的 Core Protocol 和 Advanced Skill 标记为 `absent`，不伪造版本。
- 跑 Standard + 当前 TaskSpace simple/complex 各 3 次，保存 request、token、cache、耗时、动作和 Map。
- 在看到候选结果前冻结正确性、失败率和成本非劣阈值；三次仅作为接线诊断，不作为统计充分的收益证明。

验收：任一结果都能关联到完整合同身份；不改变生产行为；为后续确定配对重复数和置信区间方法。

### FLA-1：建立装配合同与 ownership manifest

- 先实现 L1-L5 的 section identity、固定 wire 顺序、provider role mapping 和 payload snapshot，不移动正文。
- 建立 rule ownership manifest 和重复/冲突 lint；把 capability set 纳入 Tool 身份。
- 对 DeepSeek Chat 验证 base 第一条 system、developer bundle 第二条 system、tools 顶层字段和动态尾部位置。

验收：不改变模型可见语义；同一 payload 可按来源重建；跨层冲突在构建/测试期失败；缓存前缀无意外漂移。

### FLA-2：提取 L2，收敛 L1

- 从 TaskSpace Base 等价提取日常工作循环和最低恢复步骤到独立 Core Protocol artifact。
- Base 只保留价值、图模型、Map/对话分工和 Agent/Runtime 边界。
- L2 固定进入 developer bundle 第一段；正式标记其对双 Base 文档相关限制的 supersession。

验收：语义覆盖不减少；Standard 零注入；TaskSpace 行为不退化；只把搬迁记录为架构变化，不虚报降本。

### FLA-3：建立 L3 Advanced Skill 生命周期

- 通过现有 bundled system skill 管线创建 `taskspace-advanced`，只放复杂场景经验和示例。
- 生成内容寻址 snapshot，session catalog 锁定 snapshot path/hash；实现 TaskSpace profile gate、名称冲突、
  预算截断、缺失快照和 compaction/resume/fork 观测。
- 分别测试用户显式 mention 的 `<skill>` 注入与 Agent 自主文件读取，两者正文 hash 一致且不重复注入。
- 选择确实需要高级方法的复杂样本和不需要 Skill 的简单样本分别验证。

验收：简单任务不加载也正确；复杂任务主动加载后才讨论收益；hot update 不改变存量 session，缺失/截断可归因，
Skill 不产生新硬规则。

### FLA-4：重构 L4 描述与 input schema

- 将顶层 description 收敛为完整的工具选择信息，补全 action-local 语义。
- 用 provider 兼容的 discriminator + `anyOf` 表达互斥必填字段，删除重复 discriminator。
- 生成 `provider_schema_profile + capability_set_hash + tools_hash`；本阶段不改变 Runtime 行为、结果或 Tool 数量。

验收：schema contract tests、最终 DeepSeek wire、首次正确调用率和缓存均不退化；不声称 strict 已启用。

### FLA-5：冻结并原子实现 result algebra

- 先定义 success、preflight rejection、argument failure、state rejection、ordinary tool failure 和 truncated read 的
  完整共同 envelope 与分支。
- 在同一 feature version 内让 parser、preflight、handler、executor 和 read path 全部满足 conformance tests。
- 模型实际收到的 JSON 与本地 schema 同版；任何分支不合规都阻止启用，不保留半新半旧 envelope。

验收：所有结果进入上下文，state commit 无歧义；结果 schema 覆盖率 100%；本阶段不拆 Tool、不启用 strict。

### FLA-6：逐项验证 Tool 能力候选

按独立实验依次验证，每次从上一接受基线开始且不叠加未接受候选：

1. `taskspace_control` / `taskspace_read` 读写拆分；权限收益只在 router/approval enforcement 通过后成立。
2. 移除冗余 `required_next_call` 声明，由 action 合同加原子 preflight 验证真实 sibling。
3. 对 MCP carrier 暴露与 FLA-5 同源的 `outputSchema`，并同时发送符合该 schema 的 `structuredContent`；
   DeepSeek Chat 只验证模型可见 JSON，不做伪暴露。
4. 先让 adapter 转发 `strict`，再用 Beta endpoint 对全部并行可见工具做兼容 probe，最后才运行 strict 候选。

每项失败都回到上一冻结基线，不保留兼容分支。

### FLA-7：L5 数据流、projection freshness 与恢复收口

- 明确 store/reducer、validator/executor、renderer、policy、history reducer 和 Composer 的接口与重放不变量。
- 修正 `map-request` 的 projection freshness 描述，验证 read -> mutate -> no-read 不会把旧视图称为 current。
- 统一 `map-append` 每个有效 request 追加、retry 去重、compaction/resume 起点等唯一 oracle。
- 静态审计 Runtime 不含命令内容分类、Patch 意图、测试充分性判断或 next-action 建议。

验收：三策略共享实现；projection/历史/canonical state 不混淆；retry、resume、compaction、fork 可确定重放。

### FLA-8：正式对照与决策

- 对每个接受的单变量版本运行 Standard、冻结 TaskSpace 基线和当前候选。
- 使用配对 seed/运行顺序、simple、complex 和 held-out adversarial 样本；Docker 环境统一并允许并行。
- 报告成功率、非劣检验/置信区间、总和、均值、中位数、长尾和逐 request trace。
- 样本数由 FLA-0 方差和预注册最小效应决定；每臂 3 次只允许作为 smoke，不得证明收益或“不退化”。

验收后再决定合并；没有证据的层次优化不得进入生产。

## 11. 测试与收益指标

### 11.1 正确性

- 用户任务公开与隐藏验证结果。
- Map 唯一 Root/Finish、全节点可追溯、依赖和生命周期合法。
- ordinary tool 始终归属有效 binding。
- 控制失败、工具失败和状态提交语义完整进入上下文。
- L1/L2/bundled L3/L4/L5 产品 artifact 出现故意冲突时，ownership lint 在发送前阻止重复权威规则。
- 冲突性的 user/AGENTS 动态文本仍原样进入 payload；Composer 不解释它，实际非法调用只由硬不变量拒绝。
- `map-append` 同 revision、retry、resume、compaction 和 `map-request` read -> mutate -> no-read 均符合唯一 oracle。
- Skill catalog 截断、省略、同名冲突、禁用、读取失败、compaction/resume 均有明确行为和日志。
- result algebra 覆盖所有 action、参数错误、preflight、状态拒绝、普通工具失败和截断读取分支。

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
- 基线完成后、候选运行前预注册主指标、置信水平、power、样本下限、非劣阈值、失败/异常处理和多候选校正。
- 优先采用配对运行与 bootstrap 置信区间；三次 smoke 只能发现大回归，不能证明因果收益。

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
11. DeepSeek 最终 wire 角色、顺序和转发字段与设计一致；逻辑 developer 不被误认为 provider 原生权限层。
12. Provider Context Composer 没有 TaskSpace 语义分支，只执行带身份 section 的顺序、carrier 和 emission directive。
13. `visible_projection_revision` 与 canonical revision 的差值可观测，旧读取结果不会被描述成 current。
14. Tool schema 的 capability 变体、strict 支持和 result schema 可见范围均按真实 provider 能力声明。

## 13. 非目标

- 本设计不改变 R6/R7 Rooted DAG 领域模型。
- 本设计不选择 `map-always`、`map-append` 或 `map-request` 中的最终胜者。
- 本设计不让 Runtime 解析 reasoning、命令意图、Patch 语义或测试质量。
- 本设计不把所有 TaskSpace 知识放入一个永远加载的 developer message。
- 本设计不要求每个 action 拆成一个 Tool，也不预设读写拆分一定获益。
- 本设计不在同一阶段完成 Map 骨架最终超限的通用压缩方案。
- 本设计不为旧 Working Protocol、旧 schema 或旧 session 增加兼容分支。
- 本设计不把 Provider Context Composer 定义成新的语义层；它是可测试但无语义的共享装配基础设施。

## 14. 待独立验证的决策

以下方向有明确工程假设，但尚不能写成已验证事实：

1. L1/L2 去重及 L3 高级内容按需化后，能降低多少固定上下文成本；单纯提取 L2 不计为降本。
2. 内置 Skill 的触发描述能否让 Agent 在复杂任务主动加载，同时不污染简单任务。
3. 读写双 Tool 是否比单 Tool 更易选、更清晰，且不会增加 request 或选择错误；拆分本身不计为权限收益。
4. 移除 `required_next_call` 声明后，action-local 合同加 preflight 能否稳定保留合并 request。
5. 同源 result schema 用于本地 conformance 及 MCP `outputSchema` 后是否有收益；DeepSeek Chat 不预设支持
   provider-visible output schema。
6. WhaleCode adapter 转发 `strict` 后，DeepSeek Beta strict mode 是否适合当前 `anyOf` schema 和并行工具集。

这些问题必须按 FLA-6 的单变量顺序回答，不能在设计文档中用直觉提前宣布成功。

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
16. [DeepSeek Chat Completion API](https://api-docs.deepseek.com/api/create-chat-completion/)
    定义原生 Chat 的消息角色、Tool 输入字段和 non-strict 参数风险，是 wire carrier 判断的直接依据。
17. [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/)
    说明自动前缀缓存按共同前缀命中，支持固定 section 顺序和动态内容后置的成本约束。
18. [Claude Code Skills](https://code.claude.com/docs/en/slash-commands)
    说明 Skill metadata 常驻、正文按需加载以及加载正文对后续上下文成本的影响。
19. [OpenAI Agents SDK Context](https://openai.github.io/openai-agents-python/context/)
    建议把始终需要的信息放入 instructions，把按需数据通过工具获取。
20. [Anthropic Agent Skill authoring](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
    强调触发描述、渐进暴露、正文和引用材料的职责边界。
