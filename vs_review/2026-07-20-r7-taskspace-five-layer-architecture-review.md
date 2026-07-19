# Subagent VS Review: R7 TaskSpace 五层交互架构

- Created: 2026-07-20T04:40:44+08:00
- Updated: 2026-07-20T06:38:42+08:00
- Report schema: adversarial-v1
- Task: 对 R7 TaskSpace 五层交互架构执行独立、可追踪的设计对抗性审查
- Report path: `vs_review/2026-07-20-r7-taskspace-five-layer-architecture-review.md`
- Review mode: fresh internal subagent
- Source session policy: no inherited main-agent context
- Status: complete - Round 8 PASS; frozen implementation-target baseline, production pending

## Round 1: 架构职责、暴露层次与最佳实践审查

### Review Input

#### Objective

判断 R7 TaskSpace 五层交互架构是否足够简洁、职责清楚、没有平行事实源或语义越界，并判断各类信息的详略、
暴露时机和载体是否合理。审查必须寻找反例，不以确认设计为目标。

#### Review Target

架构设计文档、提示词/协议/Skill/Tool/Runtime 分层、迁移与验证方案。

#### Target Locations

- `docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md`
- `docs/v0.0.5/build-R7/00-r7-three-projection-policy-charter.md`
- `docs/v0.0.5/build-R7/20-r7-dual-base-instructions-design.md`
- `docs/v0.0.5/build-R7/22-whalecode-taskspace-base-instructions.zh-CN.md`
- `third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool_simple_actions.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_output.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs`

#### Change Introduction

设计提出五层 Agent 交互架构：L1 Base Instructions、L2 Core Working Protocol、L3 Advanced Skills、
L4 Tool Contract、L5 Runtime and Factual Feedback。Agent 位于五层之外并拥有语义决策权。三种 TaskSpace
projection policy 共享所有能力，只允许 projection emission 不同。文档尚未实施，部分 Tool 调整明确为候选实验。

#### Risk Focus

- 五层是否确有五种不同变化频率和责任，还是人为拆层、命名重叠或隐藏了第六层。
- L1 与 L2 均常驻时是否仍会重复、冲突或产生错误的“优先级”理解。
- L3 Skill 的触发、加载、版本与退出是否可执行，是否把关键正确性藏进可选内容。
- L4 是否真正只承担能力合同；领域语义、使用条件和工作方法的边界是否可操作。
- L5 把 canonical state、validator、feedback、projection 合为一层是否过粗，或让 projection/Runtime 再次越界。
- 单一语义所有权与“L4 定义、L5 执行”等跨层协作是否自洽。
- 读写 Tool 拆分、output schema、strict mode、`required_next_call` 简化等候选方向是否符合真实 provider 和
  Agent 工具设计经验。
- 暴露时机、固定前缀、Skill 目录、动态 projection 与工具结果是否符合缓存和上下文成本规律。
- 迁移顺序能否真正保持单变量、避免中间状态语义缺失或生产协议互相矛盾。

#### User-Perspective Review Focus

- 新 Agent 是否能理解 TaskSpace 的价值、默认工作方式和正确行动顺序。
- 正常任务是否被过多常驻说明淹没；复杂任务是否能发现高级方法。
- 失败反馈是否足以自行恢复，同时没有 Runtime 指挥或惩罚式纠正。
- 文档作为未来实施依据是否容易理解，是否存在名称、层次、权威来源或验收口径歧义。

#### Implementation Completeness Focus

- 本轮目标是 proposed design，不把未实施内容计为完成。
- 核对文档中的“当前实现”描述是否与生产路径一致。
- 核对每个 FLA 阶段是否有明确生产入口、测试、日志和回退边界。
- 识别只写了版本名、schema 或 Skill 名但没有可执行交付合同的部分。

#### Target Benefit Focus

- 声称的架构清晰度、语义保真、固定上下文降本、可归因性和 Agent 使用质量是否有基线、测量方法和回归门禁。
- 区分已证明收益、设计假设和未测量目标。

#### Assumptions To Attack

- Base 的宏观模型与 Core Protocol 的常规方法可以稳定分开且不重复。
- 可选 Advanced Skill 不会成为普通任务正确性的隐性依赖。
- Tool schema 的约束强度足以表达主要机械合同。
- Runtime 可以执行调用顺序硬规则而不进入语义控制。
- Projection 可与 feedback 归于同层而仍保持纯事实、唯一事实源和三策略一致。
- 静态 Base/Protocol/Tool 前缀天然有利于缓存，动态内容位置不会破坏前缀。
- 读写双 Tool 是最小且可能有益的能力边界。
- FLA-0 至 FLA-6 可以逐项实施且不与 R7 总计划发生状态冲突。

#### Adversarial Lenses

- requirements
- architecture
- state and failure
- usability and comprehension
- maintenance
- implementation-completeness
- target-benefit
- testing and observability

#### Verification Status

- 目标文档已通过 Markdown whitespace、本地链接和外部链接可达性检查。
- 五层架构尚未实施，没有运行样本或成本收益结果。
- 读写 Tool 拆分、output schema、strict mode 和 `required_next_call` 简化均为待验证候选。

#### Reviewer Instructions

- 使用全新内部 subagent session，不继承主 Agent 上下文。
- 直接读取目标文件，不依赖本报告对实现的概括。
- 只读审查，不修改任何文件。
- 优先寻找会导致职责冲突、上下文污染、错误暴露、Runtime 越界或不可实施迁移的反例。
- 独立核对主流 Agent/Tool/Skill/协议的官方一手资料；资料结论须附官方链接，不以博客汇总替代。
- 每项 finding 使用稳定编号 `F1`、`F2`；区分 blocking 与 non-blocking。
- 每项 finding 必须包含 broken assumption、failure scenario、trigger、impact、proof needed 和文件行号。
- 单独评价结构清晰度、职责正确性、详略、冲突、暴露时机、Tool 最佳实践、实施完整性和收益证据。
- 输出 Required Fixes、Missing Tests、Missing Logs / Observability；没有则明确 `none`。

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User-recommended agent command: n/a
- User-recommended agent verification: n/a
- User approval requested: n/a
- User-approved CLI command: n/a
- User decision: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | one 10-minute extension if alive | 2 | accepted blocking finding requires a fresh closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| architecture-adversary | 目标是跨提示词、Skill、Tool、Runtime 与 feedback 的架构职责设计 | 分层、依赖方向、重复、暴露时机、长期维护与迁移 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019f7c1d-947f-7892-bd76-e1fffcd93e4a` (`Pascal`) | spawn tool call and completion notification | `fork_context=false` | Round 1 Review Input | main-agent history、reasoning、drafts、conclusions | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1-architecture | architecture-adversary | 1 | `019f7c1d-947f-7892-bd76-e1fffcd93e4a` | about 3 minutes | completed | independent review returned | triage and revise |

### Reviewer Outputs

#### R1-architecture（verbatim）

**审查结论**

结论：**不建议将该设计作为可实施基线冻结**。发现 6 项 blocking、4 项 non-blocking。核心问题不是“五层”
数量，而是 provider 指令载体、projection composer、Skill 生命周期和结果合同仍没有可执行边界。

**Blocking Findings**

**F1：L2 的 provider 载体不可执行，并重新引入已否定的双提示词结构**

- Broken assumption：`versioned developer message` 可作为独立、稳定、具有明确优先级的层。
- Failure scenario：DeepSeek 原生 Chat API 只定义 system/user/assistant/tool，没有 developer role；适配器只能
  把 L2 合并进 system 或伪装为 user。前者破坏 Base 独立身份，后者改变权限和历史语义；同时重新引入双 Base
  设计明确删除的附加协议。
- Trigger：FLA-1 启用 L2，或切换 DeepSeek 原生/Anthropic 兼容 provider。
- Impact：L1/L2 权威顺序、缓存前缀、版本哈希和恢复行为均不可证明一致。
- Proof needed：逐 provider 的最终 payload 角色、字节顺序、合并规则和冲突矩阵；明确废止或修订双 Base 合同。
- Evidence：[五层设计:103](../docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md)、
  [双 Base:14](../docs/v0.0.5/build-R7/20-r7-dual-base-instructions-design.md)、
  [DeepSeek Chat API](https://api-docs.deepseek.com/api/create-chat-completion/)。

**F2：L5 隐藏了 projection policy 与 Provider Context Composer 这一实际架构层**

- Broken assumption：canonical state、validator、renderer、emitter、历史持久化和 provider composer 可合为一个
  “纯事实”层。
- Failure scenario：retry、resume、compaction 时，composer 决定替换、追加、去重、位置和历史持久化；这些不是
  canonical Map 事实，也不能仅由 renderer 重放。
- Trigger：任何 policy emission、重试、压缩、恢复或分叉。
- Impact：若 composer 在 L5 外，则存在隐藏第六层；若在 L5 内，则 L5 不再只有事实职责，版本与回放边界不清。
- Proof needed：分别定义 renderer、emission policy、context-history reducer、provider composer 的输入输出、状态
  所有权、版本和重放不变量。
- Evidence：[五层设计](../docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md)、
  [架构宪章](../docs/v0.0.5/build-R7/00-r7-three-projection-policy-charter.md)。

**F3：`map-append` 合同在同一宪章内自相矛盾**

- Broken assumption：策略定义已经冻结且唯一。
- Failure scenario：正文要求每个 request 追加最新 projection，表格却要求仅 revision 变化时追加；同 revision
  请求无法同时满足两者。
- Trigger：Map 未变化但产生新 provider request，尤其 ordinary tool result、retry、恢复首轮。
- Impact：上下文尾部、缓存成本、历史重放和验收测试没有唯一预期。
- Proof needed：选择唯一规则并统一定义、表格、bug 分类和测试 oracle。
- Evidence：[架构宪章](../docs/v0.0.5/build-R7/00-r7-three-projection-policy-charter.md)。

**F4：共享 Tool description 对 `map-request` 陈述了错误事实**

- Broken assumption：所有策略中“最后可见 projection 就是当前状态”。
- Failure scenario：Agent 在 revision 1 执行 `read_map`，随后控制调用提交 revision 2；`map-request` 不自动注入
  projection，最后可见 projection 仍是 revision 1，却被工具描述称为 current。
- Trigger：`map-request` 中读取后发生任意 Map 变更。
- Impact：Agent 使用旧 frontier/status 做决策，产生 stale revision、错误绑定和恢复循环。
- Proof needed：改为“仅当 projection revision 等于最新已知 canonical revision 时才是 current”，并覆盖
  read→mutate→no-read 场景。
- Evidence：[Tool description](../third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs)、
  [架构宪章](../docs/v0.0.5/build-R7/00-r7-three-projection-policy-charter.md)。

**F5：L3 只有名称，没有可执行的发现、加载和恢复合同**

- Broken assumption：Skill catalog 总能暴露，描述总能正确触发，且高级 Skill 不会成为正确性依赖。
- Failure scenario：Skill 描述被目录预算截短或省略；compaction/resume 恢复内容又被放入 L3；`map-request`
  仅保留 handle，Runtime 还禁止提醒，Agent 无法获得恢复方法。
- Trigger：Skill 数量增长、描述歧义、Skill 被禁用、compaction/resume/fork。
- Impact：复杂任务静默退化；仅记录 `loaded_skill_names` 无法判断未加载原因，实验不可归因。
- Proof needed：仓库路径、resolver、目录预算、冲突规则、版本锁定、加载失败、上下文驻留/压缩恢复、显式与
  隐式触发合同。
- Evidence：[五层设计](../docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md)、
  [OpenAI Skills](https://developers.openai.com/codex/skills)、
  [Anthropic Agent Skills](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview)。

**F6：FLA-4/FLA-5 的依赖顺序无法保持单变量**

- Broken assumption：L4 `output_schema` 可先独立稳定，L5 再补全反馈。
- Failure scenario：FLA-4 声明稳定输出 schema，但 preflight、参数错误和状态机错误仍使用不同 envelope；FLA-5
  随后再次改变这些结果。
- Trigger：任意 invalid arguments、sequence preflight、state rejection 或截断读取。
- Impact：schema 与线上结果不一致，provider/client 校验失败，回滚基线也不唯一。
- Proof needed：先冻结完整 result algebra，再原子实现所有结果分支，最后暴露 `output_schema`；或用明确
  feature flag/version 双轨实验。
- Evidence：[五层设计](../docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md)、
  [Preflight](../third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs)、
  [Args](../third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args.rs)。

**Non-blocking Risks**

**F7：Tool schema 并非 profile-static。** `apply_patch` 可见性改变会修改 enum 和 hash；需要把 capability-set
纳入身份或移除动态 schema。

**F8：`oneOf` 与 DeepSeek strict 候选不兼容。** DeepSeek strict 只列出 `anyOf`，要求所有属性 required，且
不支持 `minItems`；需要真实 V4 parallel/strict provider probe 与兼容 schema。

**F9：读写拆 Tool 不自动形成权限边界。** 两个 Tool 仍可能走同一 router/approval policy；需要授权矩阵、
审批 UI、router enforcement 和审计测试。MCP annotations 只是提示。

**F10：收益实验不足以支持决策。** 每臂 3 次不能证明不退化和因果收益；需要预注册非劣阈值、配对重复、
置信区间、失败率和 held-out 样本量依据。

**Required Fixes**

- 解决 F1，定义 DeepSeek 可执行的指令装配合同，并处理双 Base 文档的正式 supersession。
- 解决 F2，将 canonical runtime、renderer、emission policy、history reducer、provider composer 分别建模。
- 解决 F3/F4，冻结唯一 projection freshness 与 append 规则。
- 解决 F5，补齐 Skill 发现、版本、加载、压缩恢复和失败合同。
- 解决 F6，先冻结完整结果代数，再安排 `output_schema` 实验。
- 在冻结设计前处理 F7-F9 的 capability identity、strict 兼容和真实权限边界。

**Missing Tests**

- `map-append` 同 revision、retry、resume、compaction 的尾部与去重测试。
- `map-request` read→mutate→no-read 的 stale projection 测试。
- L1/L2/user/AGENTS/tool 描述冲突及各 provider payload 测试。
- Skill catalog 截短、省略、冲突、禁用、compaction/resume 和触发误判测试。
- 全 action、全错误分支、preflight、截断读取的 output-schema conformance 测试。
- Tool capability-set 变化、DeepSeek strict/parallel、阶段回滚和用户理解测试。

**Missing Logs / Observability**

- 最终 provider 指令角色、顺序、来源、字节哈希和 prefix hash。
- Skill catalog hash、截断/省略、候选匹配、加载原因、失败、版本、字节及压缩后状态。
- latest-visible 与 canonical revision 差值、projection age 和 stale-call 因果。
- schema variant 原因、visible tool set、strict endpoint、结果 schema 校验失败和 preflight sibling 身份。

审查全程只读，未修改文件，也未运行可能产生构建产物的测试。未解决 blocking：`F1-F6`。

### Main Agent Response

| Finding | 决定 | 裁决与处理 |
|---|---|---|
| F1 | 接受核心问题；驳回“不可执行”和“必然成为第二 Base”两个推论 | DeepSeek 适配器实际把逻辑 developer 映射为 system，因此 carrier 可执行但没有独立角色优先级。设计 v1.1 定义 L1 第一 system、L2 为聚合 developer bundle 首段/第二 system，并以构造期去重消除冲突；双 Base 文档增加有条件 supersession 说明。 |
| F2 | 接受 | 五层改定义为 Agent 可见权威面；store、validator、renderer、policy、history reducer、Composer 分别定义输入输出、状态所有权和重放方向。Composer 被限定为无语义装配基础设施，不是隐藏的第六语义层。 |
| F3 | 接受 | 冻结 `map-append` 为每个有效 request 末尾追加当时最新 projection，仅同 payload retry 去重；修正文档表格，保持正文、bug 分类和总验收一致。 |
| F4 | 接受 | Tool description 改为只有 projection revision 等于最新 canonical revision 才是 current；设计、宪章和定向单测同步覆盖 stale 语义。 |
| F5 | 接受 | 补齐 bundled Skill 路径、profile gate、resolver 冲突、预算、版本锁定、触发、载体、驻留、compaction/resume、失败和日志合同；硬恢复步骤禁止进入可选 Skill。 |
| F6 | 接受 | 迁移改为先冻结并全分支实现 result algebra，再实验 MCP outputSchema；DeepSeek Chat 只发送模型可读 JSON，不宣称 provider 约束。 |
| F7 | 接受 | Tool 身份改为 `profile + provider_schema_profile + capability_set_hash + tools_hash`。 |
| F8 | 接受 | input schema 方案改用 discriminator + `anyOf`；strict 前置 adapter 转发、Beta endpoint、全部可见 Tool 兼容和 parallel probe。 |
| F9 | 接受 | 明确读写拆分只改善意图/副作用合同，不形成权限边界；权限收益必须由 router/approval enforcement 证明。 |
| F10 | 接受 | 三次降级为 smoke；基线后、候选前预注册阈值与样本量，采用配对运行、置信区间和 held-out 样本。 |

主线程额外验证了两项 reviewer 没有展开的 wire 事实：当前 Chat adapter 只向 DeepSeek 转发 Tool 的 name、
description、parameters，不转发 `strict` 或 `output_schema`；Skill catalog 是逻辑 developer/system 内容，而已加载
Skill body 是 user history。修订据此区分 provider-visible schema、本地结果合同和模型实际读取的 Tool result。

## Round 2: Accepted Blocking Closure Review

### Closure Review Input

- Objective: 只判断 F1-F6 是否已在设计层闭合，并核对 F7-F10 修订没有制造新冲突。
- Reviewer: 全新 architecture-adversary session，不继承 Round 1 或主线程上下文。
- Scope: 修订后的五层设计、projection 宪章、双 Base 说明、Tool description/test 及实际 DeepSeek/Skill adapter。
- Constraint: proposed migration 未实施本身不算缺陷；设计不可执行、职责仍冲突或 wire 假设错误才算阻断。

### Closure Reviewer Launch Record

| Reviewer | Internal Mechanism | Session / Job ID | Context Forked | Input Packet | Read-only |
|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019f7c2d-de1b-7591-88ed-99ef2ce6d072` (`Ptolemy`) | `fork_context=false` | Round 2 closure-only packet | yes |

### Closure Reviewer Output

#### R2-architecture-closure

**Verdict: FAIL.** F1-F4 and F6-F10 closed; F5 open; one new blocking finding N1.

| ID | Status | Reviewer rationale |
|---|---|---|
| F1 | CLOSED | L1/L2 are logical artifacts rather than wire roles; DeepSeek mapping, order, identity and conditional supersession are explicit. |
| F2 | CLOSED | Five layers are information-authority surfaces; Composer, renderer, policy, reducer and store have separate ownership. |
| F3 | CLOSED | Oracle is request-tail freshness; same revision may appear on separate requests and exact retry is deduplicated. |
| F4 | CLOSED | Read results are current only at their own revision; Tool description and regression assertion match. |
| F5 | OPEN | Agent path reads produce ordinary Tool results while explicit mention uses `<skill>` injection; the design conflated these carriers. A mutable catalog path also cannot preserve a session-pinned version after compaction. |
| F6 | CLOSED | Result algebra precedes carrier exposure; DeepSeek JSON, MCP outputSchema and strict input are separate stages. |
| F7 | CLOSED | Provider schema profile, capability-set hash and tools hash cover visible-schema changes. |
| F8 | CLOSED | Adapter forwarding, Beta endpoint, supported subset and parallel probe are explicit strict prerequisites. |
| F9 | CLOSED | Tool naming/annotations are non-authoritative; router/approval/executor must enforce permissions. |
| F10 | CLOSED | Three runs are smoke only; formal tests require paired/held-out/preregistered statistical design. |

**N1: dynamic instruction conflicts are outside the manifest model.** The revised design asked ownership lint to reject conflicts
among L1/L2/AGENTS/user/Tool while also forbidding Composer from interpreting natural language. A user or repository instruction
can contradict TaskSpace text; satisfying that rule would require semantic conflict detection or rejection of ordinary input. The
manifest must be restricted to product-owned versioned artifacts, while dynamic instructions remain representable and hard Runtime
invariants continue to apply only at execution.

Remaining non-blocking points: MCP `outputSchema` needs conforming `structuredContent`; formal evaluation should state confidence,
power, sample floor and multiple-candidate handling; one FLA cross-reference was stale; DeepSeek strict schema should prefer singleton
`enum` because published support does not list `const`.

### Main Agent Closure Response

| Finding | 决定 | 修订 |
|---|---|---|
| F5 | 接受 | 明确两条真实载体：显式 mention 由宿主注入 `<skill>` user item；Agent 自主选择通过普通文件 Tool 读取，内容作为 Tool result。两者锁定并校验同一内容寻址 snapshot/hash；resume/fork/compaction 只重读会话快照，缺失时不 fallback 最新版。 |
| N1 | 接受 | ownership manifest 限定为 L1-L5 的 WhaleCode 自有版本化 artifact；user、AGENTS、外部 Tool result 忠实透传，不做语义冲突检测。只有实际违反硬不变量的调用由 Runtime 机械拒绝。 |
| MCP structured result | 接受 | FLA-6 要求 `outputSchema` 与符合它的 `structuredContent` 同时发送。 |
| 统计口径 | 接受 | 增加置信水平、power、样本下限和多候选校正的预注册要求。 |
| FLA 引用 | 接受 | `FLA-4` 修正为 `FLA-6`。 |
| strict discriminator | 接受 | strict 候选改用 DeepSeek 公开支持的单值 `enum`，不依赖未声明的 `const`。 |

Round 2 reviewer 全程只读，未运行测试。

## Round 3: Final Blocking Closure Review

### Launch Record

| Reviewer | Internal Mechanism | Session / Job ID | Context Forked | Scope | Read-only |
|---|---|---|---|---|---|
| architecture-adversary | `multi_agent_v1.spawn_agent` | `019f7c38-1a25-7cb2-af0f-63ffbb6415e9` (`Wegener`) | `fork_context=false` | F5 Skill carrier/version and N1 dynamic-instruction ownership only | yes |

### Output

**Verdict: PASS.**

**B1/F5 CLOSED.** The design now pins `name + skill_version + body_sha256 + snapshot_path` to an immutable session
snapshot; explicit host injection and Agent-selected reads use separate, accurate carriers; compaction keeps identity without auto
reload; resume/fork restore the pinned identity; a missing snapshot fails without latest-version fallback. This matches current
implementation separation: explicit mentions build `<skill>` user items, while Agent file reads only produce ordinary Tool results
and implicit invocation telemetry.

**B2/N1 CLOSED.** Ownership checks now cover only versioned WhaleCode artifacts. User input, `AGENTS.md` and external Tool
results remain outside the manifest and pass through without Runtime/Composer semantic analysis. Composer remains mechanical, and
the projection charter independently prohibits natural-language inference and automatic semantic correction.

New blocking findings: none.

Remaining non-blocking implementation verification: persist pinned Skill identity outside compactable history; atomically create
and hash-check immutable snapshots; test explicit/Agent-read carriers, compaction/resume/fork, missing snapshots and hot updates;
do not treat the whitelist-based implicit detector as an integrity boundary. Round 3 reviewer was read-only and ran no tests.

### Closure Status

- Blocking findings found: F1-F6 in Round 1; F5 and N1 remained/opened in Round 2
- Accepted blocking findings fixed: yes, all design-level blockers
- Blocking re-review completed: yes, Round 2 and Round 3
- Blocking re-review passed: yes, Round 3
- Blocking re-review round links: Round 2 `R2-architecture-closure`; Round 3 `Final Blocking Closure Review`
- Blocking re-review launch records: `019f7c2d-de1b-7591-88ed-99ef2ce6d072`, `019f7c38-1a25-7cb2-af0f-63ffbb6415e9`
- Rejected findings backed by evidence: F1 的“carrier 不可执行/必然第二 Base”子推论被本地 DeepSeek role mapping 反证
- Deferred findings documented: none
- Implementation completeness gaps resolved or accepted by user: proposed implementation remains unimplemented by design; all gaps have explicit phases/tests/logs
- Target benefit warnings recorded: yes; three-run smoke cannot establish benefit or non-regression
- Blocked reason: none
- Allowed to proceed: yes, as a proposed implementation baseline

## Final Conclusion

R7 TaskSpace five-layer design v1.2 passed fresh blocking closure review. It may be used as a proposed implementation baseline,
but no FLA implementation or performance benefit is claimed complete. The remaining risks are explicitly assigned to phase gates:
wire composition, immutable Skill snapshots, result conformance, provider capability probes and statistically defensible evaluation.

### Post-review Amendment

- 2026-07-20: design v1.3 根据用户明确要求收敛 4.1 的 Agent 可见表述。删除“线性上下文表达不足”等开发者侧
  设计动机，改为直接说明 Map 的作用和默认使用方式。该修订没有改变五层职责、wire carrier、Runtime 边界或
  已闭合的 blocking contracts，因此不重新打开架构审查。
- 2026-07-20: design v1.4 根据用户对“抽象设计无法判断实际内容”的反馈，新增
  [`24-r7-taskspace-five-layer-concrete-contract-draft.md`](../docs/v0.0.5/build-R7/24-r7-taskspace-five-layer-concrete-contract-draft.md)，
  逐字展示 L1/L2、L3 Skill、L4 action description/schema、L5 result/projection 与端到端 trace。架构职责的 blocking
  closure 仍有效，但产品内容改为 `concrete contract review required`；用户审阅前不得启动 FLA-2 之后的内容实施。

## Round 4: 具体合同的可实施性与可评估性审查

### Review Input

#### Objective

判断 R7 五层设计和具体合同是否已经达到“新实现者不依赖主 Agent 聊天背景，即可按文档完成生产路径实施、
测试、日志和收益评估”。审查目标是寻找反例和隐藏决策，不是确认架构表述是否自洽。

#### Review Target

TaskSpace 五层提示词与工具链设计、具体 Agent-visible 合同、分阶段实施方案、验收与性能评测方案。

#### Target Locations

- `docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md`
- `docs/v0.0.5/build-R7/24-r7-taskspace-five-layer-concrete-contract-draft.md`
- `docs/v0.0.5/build-R7/00-r7-three-projection-policy-charter.md`
- `docs/v0.0.5/build-R7/01-r7-phased-implementation-plan.md`
- `third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool_simple_actions.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_output.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs`
- `third_party/codex-cli/codex-rs/core/src/provider_wire_sections.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/projection.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/projection_policy.rs`
- `.agents/skills/observe-taskspace-performance/SKILL.md`

#### Change Introduction

设计把 TaskSpace 的 Agent 交互拆为 L1 Base、L2 常驻工作协议、L3 按需 Skill、L4 Tool contract 和
L5 Runtime/事实反馈。新增的具体合同稿展示 L1/L2 逐字文本、Skill 正文、Tool action/schema 骨架、
result/projection 样例和端到端调用路径。当前状态是尚未实施的设计候选。

#### Risk Focus

- 具体稿是否仍以“候选、骨架、示例”回避了实施必须做出的产品或数据合同决策。
- 每个 FLA 阶段是否指定真实源码入口、产物、删除项、测试、日志、回滚基线和完成证据。
- L1/L2/Skill/Tool/result 的逐字内容是否能直接进入版本化 artifact，还需要实现者补完隐藏语义。
- Tool 读写拆分、action 改名、`required_next_call` 移除、strict 和 output schema 等待 A/B 项是否与主实施路径混在一起。
- 数据结构、revision、partial commit、retry、resume/fork/compaction 和三种 projection 是否有唯一、可测的 oracle。
- 评测是否有固定 sample ID、对照臂、主指标、阈值、重复数决策、失败归类、缓存计算和逐 request trace 证据。
- 文档是否能阻止“只改提示词、只加 schema、只加测试脚手架”被误报为完成。

#### User-Perspective Review Focus

- Agent 能否从实际可见内容中理解 Map 价值、普通工作循环、高级 Skill 触发、Tool 参数与失败恢复。
- 新实现者能否明确哪些文本是已选合同，哪些是独立实验，哪些不得进入当前阶段。
- 失败结果是否给出足够事实供 Agent 自主修正，同时不会被 Runtime 诱导。

#### Implementation Completeness Focus

- 将 FLA-0 至 FLA-8 逐项对应到生产源码模块、集成入口、需新增或删除的 artifact、定向/端到端测试、trace/log 与可重现验收命令。
- 检查具体合同中的所有 action/result/projection 形状是否覆盖完整，还是只展示了局部分支。
- 检查计划是否显式要求生产路径连通，防止 protocol-only、schema-only、test-only 或 mock-only 产物被判定为完成。
- 本轮不要求代码已实施，但要求文档能让实施者无需自行重新设计。

#### Target Benefit Focus

- 已声明的目标包括更低固定上下文、更高首次正确调用率、更少空转 request、不退化的任务正确性与缓存表现。
- 检查每项是否有可重现基线、可预注册阈值、测量脚本/日志来源、对照证据和回归处理。
- 未实施因此不要求已有收益结果，但“验收标准不可执行”应被视为设计缺口。

#### Assumptions To Attack

- 实施者可以从“骨架”和“候选”自行推导完整合同，而不引入新语义。
- 现有 Runtime/Tool/projection 模块的边界恰好对应文档层次。
- 将 L2 放在第二条 system 的首段不需要更具体的 composer 改动和 wire 回归测试。
- 同一 action 同时参与 schema、parser、preflight、handler、result、projection 时，现有阶段顺序仍能保持单变量。
- “后续用方差决定样本数”在缺少最小数值和决策公式时足够可执行。

#### Adversarial Lenses

- comprehension
- implementation-completeness
- requirements
- state
- failure
- maintenance
- testing
- observability
- target-benefit

#### Verification Status

- 具体合同的 13 个 JSON 示例已进行语法解析，Markdown 本地链接和代码块已检查。
- 五层候选尚未进入产品代码，没有行为或性能收益证据。
- 旧架构职责已通过 Round 1-3 审查，但本轮不得以旧结论替代对新增具体合同和实施细节的审查。

#### Reviewer Instructions

- 使用全新内部 subagent session，`fork_context=false`。
- 只读目标文档和代码，不修改任何文件。
- 不使用主 Agent 的聊天历史、推理、结论或完整 diff。
- 以“新实施者能否直接执行”为主标准，不接受只有抽象层次的修复建议。
- 每个 blocking/major finding 必须给出 broken assumption、failure scenario、trigger、impact、proof needed，
  并引用文件与行号。
- 输出 summary、blocking findings、non-blocking risks、user-perspective checks、implementation completeness table、
  target benefit table、required fixes、missing tests、missing logs/observability 和 evidence。
- 若发现 blocking，说明最小的设计级闭合产物，不以“实施时再决定”作为修复。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | one 10-minute extension if alive | 2 | accepted blockers require a fresh closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| documentation-skill-adversary | 具体合同必须能在无主线聊天背景的情况下指导新实施者和新 Agent，当前最高风险是隐含知识、触发不明和可执行性缺口 | 提示词、Skill、Tool 合同、文档可执行性、实施与验收完整性 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| documentation-skill-adversary | `multi_agent_v1.spawn_agent`, `gpt-5.5` low, explorer | `019f7c60-3af4-7351-87fd-e2023c487667` (`Pasteur`) | spawn tool result and completion notification/transcript | `fork_context=false` | Round 4 Review Input plus the same neutral scope and output contract | main-agent history, hidden reasoning, prior conclusions, full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R4-concrete-contract | documentation-skill-adversary | 1 | `019f7c60-3af4-7351-87fd-e2023c487667` | about 64 seconds | completed | independent review returned | triage and revise |

### Reviewer Output

#### R4-concrete-contract（完整 finding transcript；重复路径列表做排版归一）

**Summary**

Round 4 does not pass. A new implementer cannot deterministically implement, test, instrument, and evaluate the five-layer design
from the current documents without making new contract decisions. The main blockers are contract authority conflicts, incomplete
versioned artifacts, incomplete FLA-to-production mapping, and non-executable evaluation rules. Current production code also shows
R7 work already exists while the concrete contract document still says all text/schema are unimplemented candidates.

**Blocking Findings**

**B4.1 - Contract authority is unresolved between “candidate/unimplemented” and already-completed implementation path**

- Broken assumption: the documents identify one selected baseline contract.
- Failure scenario: an implementer treats `24` as authoritative and implements `bind_node` / `taskspace_read` /
  `TaskSpaceControlResultV2`, while production currently exposes `transition_node`, a single `taskspace_control`, and
  `TaskSpaceControlResultR6V1`.
- Trigger: starting FLA-4/FLA-5 from the concrete contract draft.
- Impact: incompatible tool schema, parser, tests, logs, and benchmark reports can all be “correct” against different documents.
- Proof needed: a frozen contract index marking each item as `selected`, `experimental`, `obsolete`, or `not implemented`, with
  source artifact paths and hashes.
- Affected phase/production path: FLA-4, FLA-5; `taskspace_tool.rs`, `taskspace_control_args.rs`, result formatter.
- Evidence: `24...contract-draft.md:5-8`, `24...contract-draft.md:287-294`, `24...contract-draft.md:399-400`,
  `01...plan.md:13`, `01...plan.md:478-498`, `taskspace_tool.rs:186-245`, `taskspace_tool.rs:343-349`,
  `taskspace_control_args.rs:9`.

**B4.2 - L4 action contract is not complete enough to become a versioned artifact**

- Broken assumption: samples plus an action table are sufficient for implementation.
- Failure scenario: implementer must invent missing `anyOf` branches, per-action descriptions, idempotency, boundary conditions,
  read/write split behavior, and structured output coverage for all variants.
- Trigger: implementing provider-visible schema from section 6.4.
- Impact: parser/schema/preflight/handler drift; conformance tests cannot know whether failures are contract violations.
- Proof needed: full generated provider schema artifact covering every action/result variant, plus conformance fixtures for every branch.
- Affected phase/production path: FLA-4/FLA-5; tool schema, parser, preflight, handlers.
- Evidence: `24...contract-draft.md:311-315`, `24...contract-draft.md:296-309`, `taskspace_tool.rs:331-340`,
  `taskspace_control_args.rs:12-61`.

**B4.3 - Result algebra is contradictory and incomplete**

- Broken assumption: result shape is fixed and mechanically testable.
- Failure scenario: tests assert `partial_commit: false` and `TaskSpaceControlResultV2`, while production emits
  `partial_commit: 0` and `TaskSpaceControlResultR6V1`; rejected errors lack the draft’s `actual` / `expected` oracle in many paths.
- Trigger: implementing FLA-5 result conformance.
- Impact: Agent recovery semantics, observer classification, and benchmark eligibility become ambiguous.
- Proof needed: one result schema version, typed `partial_commit`, complete status/error enum, and golden results for parse failure,
  preflight failure, state-machine failure, resource failure, committed transition, and ordinary-tool-after-control failure.
- Affected phase/production path: FLA-5; `taskspace_control_output.rs`, `taskspace_control_args.rs`, performance observer.
- Evidence: `24...contract-draft.md:437-453`, `24...contract-draft.md:460-478`, `24...contract-draft.md:485-509`,
  `taskspace_control_args.rs:337-352`, `taskspace_control_output.rs:53-75`, `taskspace_control_output.rs:209-224`.

**B4.4 - FLA-0 through FLA-8 are not mapped to concrete implementation entries and reproducible commands**

- Broken assumption: the FLA plan is executable by a new implementer.
- Failure scenario: implementer cannot tell which modules to edit, which artifacts to add/remove, which commands to run, or what
  exact evidence closes each FLA item.
- Trigger: starting FLA implementation without prior chat context.
- Impact: protocol-only, schema-only, mock-only, or prompt-only work can be counted as complete.
- Proof needed: per-FLA checklist with production module, integration entry, artifact path, removal list, test command, log event,
  benchmark command, rollback baseline, and completion evidence.
- Affected phase/production path: all FLA phases.
- Evidence: `23...architecture-design.md:33-36`, `23...architecture-design.md:530-613`, `01...plan.md:70-89`,
  `01...plan.md:103-115`.

**B4.5 - Evaluation contract is not executable enough to prove benefits or non-regression**

- Broken assumption: “sample categories” and “at least 3 runs” are enough for evaluation.
- Failure scenario: two implementers pick different `simple`, `complex`, `branch-join`, or held-out tasks and reach incompatible
  default-policy conclusions.
- Trigger: Phase G formal matrix.
- Impact: default recommendation and benefit claims are not reproducible.
- Proof needed: fixed sample IDs/prompts/repos, frozen arms, run order, seeds or pairing rule, primary metric, non-inferiority
  thresholds, minimum sample-size formula, failure classification, cache price artifact, and anti-overfitting held-out set.
- Affected phase/production path: FLA-0, FLA-8, Phase G, benchmark skill.
- Evidence: `01...plan.md:131-175`, `01...plan.md:544-574`, `23...architecture-design.md:608-611`,
  `23...architecture-design.md:650-655`, `.agents/skills/observe-taskspace-performance/SKILL.md:12-23`.

**B4.6 - Resume/fork/compaction/retry oracles remain policy-level prose, not unique production tests**

- Broken assumption: lifecycle edge cases are deterministic from the current docs.
- Failure scenario: `map-append` retry dedup, compaction after old projections, forked session policy restoration, or
  `map-request` stale read behavior are implemented differently but still defensible from prose.
- Trigger: Phase E implementation.
- Impact: cross-policy equivalence cannot be established; regressions may hide as allowed projection differences.
- Proof needed: scripted event fixtures with expected canonical event hash, provider payload diff, policy cursor state, projection
  sequence, and resume/fork/compaction outcomes.
- Affected phase/production path: projection policy, composer, session state, provider observer.
- Evidence: `00...policy-charter.md:223-241`, `01...plan.md:500-520`, `projection_policy.rs:90-160`,
  `provider_wire_sections.rs:256-339`.

**Non-blocking Risks**

**N4.1 - Agent-visible L1/L2 comprehension is plausible but not proven after D.5 removed the separate developer protocol path**

- Counterexample: `24` says L2 is second `system` first section, while D.5 says the independent Working Protocol developer
  message was deleted.
- Impact: readers may expect a distinct L2 artifact that current production no longer sends.
- Proof needed: provider payload snapshot showing exact L1/L2 placement after D.5.
- Evidence: `24...contract-draft.md:29-35`, `24...contract-draft.md:73-84`, `01...plan.md:485-492`.

**N4.2 - Runtime semantic-neutrality is well stated but not enforced by a lintable rule set**

- Counterexample: errors could add “please read_map” or “run tests next” without failing a contract test.
- Impact: Runtime may gradually become semantic-guiding.
- Proof needed: artifact ownership manifest and lints for forbidden advisory phrases in L5/tool results.
- Evidence: `23...architecture-design.md:74-86`, `24...contract-draft.md:456-483`, `24...contract-draft.md:753-754`.

**Implementation Completeness Table**

| Plan item | Expected behavior | Production path | Integration entry | Tests | Runtime/log evidence | Mock/stub exposure | Status | Finding |
|---|---|---|---|---|---|---|---|---|
| FLA-0 | Freeze baseline/contracts | benchmark + contract artifacts | not fully fixed | categories only | incomplete | historical-only risk | Blocked | B4.5 |
| FLA-1 | Ownership manifest | composer/contracts | unspecified | ownership lint described | unspecified | schema-only risk | Blocked | B4.4 |
| FLA-2 | L1/L2 extraction | base instructions/provider composer | conflicted by D.5 | payload snapshot needed | base hash planned | prompt-only risk | Blocked | B4.1 |
| FLA-3 | Advanced Skill lifecycle | bundled skills/snapshots | not mapped | lifecycle fixtures missing | catalog telemetry planned | doc-only risk | Blocked | B4.4 |
| FLA-4 | Tool schema | `taskspace_tool.rs` | exists but differs | partial | tool hash exists | schema drift | Blocked | B4.2 |
| FLA-5 | Result algebra | `taskspace_control_output.rs` | R6/V2 mismatch | incomplete | observer mentions R6 | result drift | Blocked | B4.3 |
| FLA-6 | Tool candidates | split/strict/outputSchema | unresolved A/B | not fixed | not fixed | optional counted complete | Blocked | B4.1 |
| FLA-7 | Projection lifecycle | policy/composer/observer | partial | unit tests partial | observer partial | lifecycle gaps | Blocked | B4.6 |
| FLA-8 | Formal matrix | benchmark scripts/skill | categories only | no fixed IDs/rule | reports old fields | non-reproducible | Blocked | B4.5 |

**Target Benefit Table**

| Claimed benefit | Baseline | Target | Method | Evidence | Result | Regression | Status | Finding |
|---|---|---|---|---|---|---|---|---|
| Lower fixed context | Frozen R6/Standard | not pre-registered | four-arm matrix | categories only | unavailable | relocation misreport risk | Blocked | B4.5 |
| Higher first correct call rate | Frozen R6 | not fixed | Docker samples | partial D.4/D.5 | not generalizable | overfit risk | Blocked | B4.5 |
| Fewer empty requests | Standard/R6 | not fixed | request metrics | phase evidence | not formal | standalone transitions | Blocked | B4.5 |
| No correctness regression | Standard + R6 | threshold missing | validators | categories | unavailable | failure reclassification | Blocked | B4.5 |
| Cache performance | provider cache trace | threshold missing | request-2+ cache/LCP | report skill | incomplete | price missing | Blocked | B4.5 |

**Required Fixes**

- Create a single R7 contract authority artifact that marks every L1/L2/L3/L4/L5 decision as selected, optional experiment,
  obsolete, or deferred.
- Replace sample-only L4/L5 sections with full versioned schema/result artifacts and golden fixtures for every action/result variant.
- Add an FLA-0 through FLA-8 implementation matrix with exact production paths, artifact paths, commands, logs, rollback baselines,
  and completion evidence.
- Freeze the formal evaluation packet: sample IDs, arms, run order, metrics, thresholds, sample-size rule, cache price artifact,
  trace sources, and failure taxonomy.
- Add deterministic lifecycle fixtures for retry, partial commit, ordinary-tool failure after committed control, resume, fork,
  compaction, and each projection policy.

**Missing Tests**

- Full provider schema conformance for all `taskspace_control` branches.
- Result algebra golden tests for every status/error class.
- Cross-policy differential test with canonical event hash and allowed provider-payload diffs.
- Payload snapshot tests proving exact L1/L2/Skill/tool/projection placement.
- Benchmark harness self-test for fixed R7 arms, sample IDs and failure classification.

**Missing Logs / Observability**

- `effective_contract_identity` per request for L1/L2/L3/L4/L5.
- Per-action result schema version and action/result variant counters.
- Projection lifecycle logs for retry/resume/fork/compaction, not only provider request emission.
- Explicit `unavailable` fields for missing cache, price, trace, validator, and Skill evidence.
- Runtime-neutrality lint or log classification for advisory vs factual feedback.

No files were edited.

### Main Agent Response

| Finding | 决定 | 设计级闭合 |
|---|---|---|
| B4.1 | 接受 | 新增唯一 authority manifest，逐项记录现行回滚基线、已选未实施目标、禁用实验、淘汰项、生产入口和 hash；主线明确保留单 Tool、`required_next_call`、`strict=false`。 |
| B4.2 | 接受 | 新增完整 L4 内联 schema，覆盖 12 个 action 与 `read_output_ref` 四个 mode；文档中的两分支代码只标为非权威阅读摘录。 |
| B4.3 | 接受 | 新增完整 `TaskSpaceControlResultV2` JSON Schema、固定错误码/消息合同和 `partial_commit=false` 原子性；旧 R6V1 明确到 FLA-5 同次删除。 |
| B4.4 | 接受 | 新增 `25-r7-five-layer-executable-spec.md`，逐 FLA 写明生产文件、集成入口、新增/删除、测试、日志、smoke、回滚和完成证据。 |
| B4.5 | 接受 | 新增机器评估合同，冻结 5 个 scenario 目录 hash、7/3 臂、固定 30 次配对、轮换顺序、95% paired bootstrap、阈值、失败分类和原始证据；金额在没有价格 artifact 时明确 unavailable，且不参与晋级。 |
| B4.6 | 接受 | 新增 12 个生命周期 oracle、两份完整 canonical Map、冻结 state/event hash、retry/resume/fork/compaction 与三 policy 的唯一判定。 |
| N4.1 | 接受 | `01` 号计划新增 supersession 说明：D.5 删除旧独立协议，FLA-2 新 L2 是现有 developer bundle 第一 section；`25` 号规格冻结两条 DeepSeek system 的准确位置。 |
| N4.2 | 接受风险；拒绝自然语言关键词分类方案 | 使用封闭错误码、固定事实模板、完整结果 schema、golden snapshot 和 Runtime 静态边界审计约束反馈。禁止引入 advisory phrase classifier，因为它会让 Runtime/Composer 解释自然语言并产生新的语义边界。 |

Required Fixes、Missing Tests 和 Missing Logs 全部接受，分别落入 `25` 号规格的 FLA-1 至 FLA-8 交付矩阵。
这些测试和脚本被明确标记为实施阶段必须新增的生产验收能力；本轮没有把“设计了测试”误报为“测试已存在”。
Round 4 reviewer 全程只读，未修改文件，也未运行构建或行为样本。

## Round 5: 具体合同阻断项闭环复审

### Closure Review Input

#### Objective

独立判断 Round 4 的 B4.1-B4.6 是否已在设计层形成唯一、可实施、可测试、可观测和可评估的闭环，并寻找修订
引入的新阻断问题。不得因为文档数量增加或存在 schema 文件就默认通过。

#### Target Locations

- `benchmarks/taskspace/r7/five-layer-contract-authority-v1.json`
- `benchmarks/taskspace/r7/five-layer-l1-taskspace-base-section-v2.md`
- `benchmarks/taskspace/r7/five-layer-l2-core-protocol-v2.md`
- `benchmarks/taskspace/r7/five-layer-l3-taskspace-advanced-v1.SKILL.md`
- `benchmarks/taskspace/r7/five-layer-taskspace-control-v2.schema.json`
- `benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json`
- `benchmarks/taskspace/r7/five-layer-lifecycle-oracles-v1.json`
- `benchmarks/taskspace/r7/five-layer-evaluation-contract-v1.json`
- `docs/v0.0.5/build-R7/01-r7-phased-implementation-plan.md`
- `docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md`
- `docs/v0.0.5/build-R7/24-r7-taskspace-five-layer-concrete-contract-draft.md`
- `docs/v0.0.5/build-R7/25-r7-five-layer-executable-spec.md`
- 当前生产 Tool/parser/result/projection/provider wire/Skill 路径，仅用于核对 baseline 与 production target。

#### Required Checks

1. Authority 是否明确区分现行回滚基线、选定未实施目标、禁用实验和淘汰项，且 hash/生产入口没有冲突。
2. L4 是否完整覆盖全部 action/mode，schema、表格、示例和主线单 Tool 选择一致。
3. L5 是否有一个完整 V2 result algebra，`partial_commit`、失败分类、actual/expected、普通 Tool 后续失败语义唯一。
4. FLA-0 至 FLA-8 是否逐项给出生产入口、删除项、测试、日志、命令、回滚和完成证据，能阻止文档/schema-only
   被误报为完成。
5. 评估合同是否冻结真实 scenario 目录、arms、顺序、重复数、统计判定、阈值、失败分类、cache/price unavailable
   语义和 evidence，且运行者无需自行设计。
6. 生命周期 oracle 是否给出可重算 canonical/event hash，并唯一规定 retry/resume/fork/compaction 和三策略差异。
7. N4.1 的 D.5/L2 carrier 是否消歧；N4.2 是否在不引入 Runtime 自然语言解释器的前提下可回归验证。
8. 精确文本、schema 与 executable spec 是否仍互相矛盾，或存在不可执行命令被写成已有能力的情况。

#### Reviewer Instructions

- 使用 fresh session，`fork_context=false`；只读，不修改文件。
- 不依赖主 Agent 的聊天历史、推理或结论，直接读取上述 artifact 与必要源码。
- 对 B4.1-B4.6、N4.1-N4.2 分别给出 CLOSED / OPEN 与证据。
- 新 finding 必须给出 broken assumption、具体失败场景、触发条件、影响、需要的最小闭合产物和文件行号。
- 区分“设计合同可执行”与“生产尚未实施”；后者本身不算本轮缺陷，但任何把未实施写成已验证的声明算缺陷。
- 输出最终 PASS/FAIL、blocking findings、non-blocking risks、missing tests/logs 和是否允许作为实施基线。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | one 10-minute extension if alive | 2 | any open accepted blocker requires another fresh closure round |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-contract-adversary | 本轮只审计文档是否足以驱动真实生产实施和可重复验收，不复审宏观愿景 | 合同权威、完整 schema、生命周期、实施矩阵、评估可执行性 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-contract-adversary | `multi_agent_v1.spawn_agent`, `gpt-5.5` low, explorer | `019f7c7e-6fd6-7a13-8375-307d6dddf942` (`Sartre`) | spawn tool result and completion transcript | `fork_context=false` | Round 5 Closure Review Input | main-agent history, hidden reasoning, prior conclusions, full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R5-implementation-closure | implementation-contract-adversary | 1 | `019f7c7e-6fd6-7a13-8375-307d6dddf942` | about 75 seconds | completed | independent closure review returned | fix B5.1 and re-review |

### Reviewer Output

**Verdict: FAIL.**

| Item | Status | Evidence |
|---|---|---|
| B4.1 | CLOSED | Authority manifest selects one rollback baseline and one target set: rollback `48922ce9b`, single `taskspace_control`, `required_next_call` retained, `strict=false`, experiments disabled；hashes match disk. |
| B4.2 | OPEN | Executable spec said `read_output_ref` uses `byte/line/continuation/full`, but authoritative input/result schemas use `head/tail/line_range/grep`. An implementer following the spec would create rejected fixtures. |
| B4.3 | CLOSED | V2 result schema uniquely fixes boolean `partial_commit=false`, statuses, actual/expected and ordinary-tool-after-control semantics. |
| B4.4 | CLOSED | FLA-0 through FLA-8 have production entries, removals, tests, logs, smoke/evidence, rollback and completion rules; doc/schema-only completion is forbidden. |
| B4.5 | CLOSED | Evaluation freezes five scenario directories/hashes, arms, 30 repeats, order, seed, bootstrap, gates, unavailable semantics, artifacts and commands；reviewer independently verified hashes. |
| B4.6 | CLOSED | Lifecycle artifact has 12 fixtures, canonical/event hashes, retry/resume/fork/compaction outcomes and required logs；reviewer independently recomputed hashes. |
| N4.1 | CLOSED | D.5 removed the old independent protocol; FLA-2 L2 is explicitly the existing developer bundle first section and has exact DeepSeek placement. |
| N4.2 | CLOSED | Neutrality uses ownership, fixed result schema/messages and static boundary audit without a Runtime natural-language classifier. |

**B5.1 - `read_output_ref` mode contract is contradictory.**

- Broken assumption: executable spec and schema name the same four branches.
- Failure scenario: FLA-4 implementer writes `mode=full` or `mode=byte` fixtures from the spec; selected schema rejects them.
- Trigger: parser/schema conformance or golden fixture implementation.
- Impact: L4 still permits two incompatible implementations.
- Minimum closure: make executable spec use exactly schema modes and refresh the governing artifact hash.
- Evidence: executable spec line 99；input schema lines 229-278；result schema lines 166-175.

Non-blocking risks: authority manifest need not hash itself but freeze tooling should record its external hash；projection payload goldens
are correctly assigned to future implementation and do not yet exist. The four future scripts are explicitly marked “要求新增”，not
falsely claimed as current evidence. Freeze decision: do not freeze until B5.1 is corrected and re-reviewed.

### Main Agent Response

| Finding | 决定 | 修订 |
|---|---|---|
| B5.1 | 接受 | 可执行规格改为与两个机器 schema 完全一致的 `head`、`tail`、`line_range`、`grep`；authority manifest 新增 `governing_documents` 并冻结修订后 executable spec SHA256。 |
| Authority self-hash | 接受为非阻断观测要求 | authority 作为顶层索引不递归记录自身 hash；提交 hash 和后续 FLA-0 run manifest 负责记录其外部身份。 |
| Projection golden | 接受为未来实施证据 | 保持 FLA-7 activation gate；未生成前不声称 lifecycle production verified。 |

Round 5 reviewer 全程只读，未修改文件。B5.1 是接受的 blocking finding，因此必须启动全新 closure reviewer；
不能由主线程自行宣告闭合。

## Round 6: B5.1 最终闭环复审

### Closure Review Input

- Objective: 只判断 B5.1 是否闭合，并检查修订是否产生新的 L4/authority 矛盾。
- Scope: `five-layer-contract-authority-v1.json`、两个 L4/L5 schema、`24` 号具体合同和 `25` 号可执行规格。
- Required proof: 所有 Agent/implementer 可见位置只把 `read_output_ref` mode 定义为 `head`、`tail`、
  `line_range`、`grep`；governing document hash 可重算；禁用读写拆分实验不改变这四种 mode。
- Constraint: 生产尚未实施不算缺陷；本轮只审查设计合同唯一性。Reviewer 必须 fresh、`fork_context=false`、只读。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| contract-closure-adversary | B5.1 是一个窄而机械的跨 artifact 合同漂移，需要独立 hash/schema/doc 终检 | mode enum、文档表述、authority hash、新冲突 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Context Forked | Scope | Read-only |
|---|---|---|---|---|---|
| contract-closure-adversary | `multi_agent_v1.spawn_agent`, `gpt-5.5` low, explorer | `019f7c81-1469-72a3-843a-e93e2e2c343b` (`Heisenberg`) | `fork_context=false` | B5.1 and affected authority/schema/docs only | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R6-B5.1-closure | contract-closure-adversary | 1 | `019f7c81-1469-72a3-843a-e93e2e2c343b` | about 52 seconds | completed | independent closure review returned | fix remaining doc enum spelling and re-review |

### Reviewer Output

**Verdict: FAIL. B5.1 remains OPEN.** Input schema, result schema and executable spec all use `head`、`tail`、
`line_range`、`grep`，and the executable spec hash matches authority. However, `24` 号具体合同表格 used natural-language
`line range` instead of the exact enum `line_range`. An implementer could therefore write `mode: "line range"` and be rejected by
the selected schema. The one-Tool baseline and disabled split experiment remain correct. No new blocker was found；`continuation`
is correctly a result field rather than an input mode.

### Main Agent Response

| Finding | 决定 | 修订 |
|---|---|---|
| B5.1 residual | 接受 | `24` 号 action 表改为逐字枚举 `head`、`tail`、`line_range`、`grep`；不再用自然语言近义写法。 |
| Governing artifact identity | 接受增强 | authority 的 `governing_documents` 同时冻结 `23`、`24`、`25` 三份文档 SHA256，避免低优先级文档在后续漂移。 |
| continuation risk | 接受 | FLA-4/FLA-5 conformance 明确区分 input mode 与 read result continuation 字段；不新增 input mode。 |

Round 6 reviewer 全程只读，未修改文件。B5.1 仍是接受的 blocking finding，需再次由 fresh reviewer 闭环。

## Round 7: B5.1 精确枚举终检

### Closure Review Input

- Objective: 只判断所有权威和评审文本是否对 `read_output_ref` 使用唯一 input mode 集合
  `head`、`tail`、`line_range`、`grep`，并确认 `continuation` 只作为结果字段。
- Scope: authority manifest、L4/L5 schemas、`23`、`24`、`25` 三份治理文档。
- Required proof: 搜索不存在 `mode=full/byte/continuation` 或把 `line range` 当成枚举的表述；三份 governing document
  hash 可重算；单 Tool 主线与禁用拆分没有变化。
- Reviewer policy: fresh、`fork_context=false`、只读；输出 PASS/FAIL、B5.1 CLOSED/OPEN、新 blocker 和 freeze decision。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| enum-contract-adversary | 最后缺口只剩跨 schema/doc 的精确枚举一致性，适合窄范围机械终检 | enum spelling、result/input distinction、governing hashes |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Context Forked | Scope | Read-only |
|---|---|---|---|---|---|
| enum-contract-adversary | `multi_agent_v1.spawn_agent`, `gpt-5.5` low, explorer | `019f7c83-0abd-71e1-a65e-c5162bc66e93` (`Hume`) | `fork_context=false` | B5.1 exact enum closure only | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R7-B5.1-final | enum-contract-adversary | 1 | `019f7c83-0abd-71e1-a65e-c5162bc66e93` | about 74 seconds | completed | independent exact-enum review returned | accept PASS; close B5.1 |

### Reviewer Output

**Verdict: PASS. B5.1 CLOSED.** Input schema、result schema、`24` 号合同表格与 `25` 号规格均只使用
`head`、`tail`、`line_range`、`grep`。`continuation` 只位于读取结果 envelope，不是 input mode。Reviewer
独立重算 `23/24/25` governing document hashes 全部与 authority 一致；单 `taskspace_control` 主线、
`required_next_call`、`strict=false` 和禁用读写拆分保持不变。New blockers: none。Freeze decision: 可作为本轮
reviewed scope 的实施基线。

Non-blocking risk：result schema 当时只把 `continuation` 限定为 object/null，内部字段尚未进一步约束。

### Main Agent Response

| Finding | 决定 | 修订 |
|---|---|---|
| B5.1 | 接受 PASS | 标记为 CLOSED；不再修改四个 input mode。 |
| continuation shape | 接受非阻断改进 | Result V2 将非空 continuation 冻结为一份完整、可直接调用的 `read_output_ref line_range` 参数对象；无剩余行或其他 mode 时为 `null`。文档示例同步补全 action/output_ref/start/end/max_bytes。 |
| 文档状态 | 接受 | `23/24/25` 更新为 frozen implementation/product baseline，同时仍明确 production pending；authority 状态改为 `frozen_design_target_not_implemented` 并刷新全部 hash。 |

Round 7 reviewer 全程只读，未修改文件。由于 PASS 后又接受了非阻断合同补全并刷新 frozen 状态，启动最后一轮
fresh review，避免未审的 continuation/status 变更混入冻结基线。

## Round 8: 冻结基线最终一致性复审

### Closure Review Input

- Objective: 对最终冻结工件做一次全局机械一致性终检，不重新讨论已闭合的宏观架构。
- Scope: authority、L1/L2/L3 exact artifacts、L4/L5 schemas、lifecycle/evaluation contracts、`23/24/25` 文档。
- Required checks: authority 所有 baseline/selected/governing hashes 可重算；状态明确 frozen but production pending；
  continuation 是完整 `line_range` next-call arguments 且通过 result schema；所有 12 action 和四个 mode 唯一；
  没有新占位决策或把 future scripts 说成已实现；B4.1-B4.6/B5.1 不因最终改动重新打开。
- Reviewer policy: fresh、`fork_context=false`、只读。输出 PASS/FAIL、open/new blockers、nonblocking risks、freeze decision。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| final-contract-integrity-adversary | 冻结前检查最后的 hash、status 与 continuation 合同没有重开已闭合问题 | artifact identity、schema example conformance、implementation-status honesty |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Context Forked | Scope | Read-only |
|---|---|---|---|---|---|
| final-contract-integrity-adversary | `multi_agent_v1.spawn_agent`, `gpt-5.5` low, explorer | `019f7c86-95a6-7712-91ca-09e75d734b12` (`Bacon`) | `fork_context=false` | final frozen contract integrity | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R8-final-integrity | final-contract-integrity-adversary | 1 | `019f7c86-95a6-7712-91ca-09e75d734b12` | about 104 seconds | completed | final independent integrity review returned | accept PASS and freeze design baseline |

### Reviewer Output

**Verdict: PASS.** Open/new blockers: none. B4.1-B4.6 and B5.1 were not reopened.

Reviewer independently confirmed:

- Authority status is `frozen_design_target_not_implemented`；`23/24/25` all say production pending.
- Every baseline、selected target and governing document SHA256 in authority recomputes successfully.
- L4 compiles to one provider Tool、15 schema branches、12 unique actions and exactly four `read_output_ref` modes.
- L5 compiles；the documented truncated-read result validates；continuation is a complete `line_range` next-call argument object.
- Four FLA-6 experiments remain disabled，with single Tool、retained `required_next_call` and `strict=false` as baseline.
- Lifecycle contract contains LC-01 through LC-12；evaluation freezes five samples、arm sets、3-run smoke and 30-run formal repeats.
- Future benchmark/fixture scripts are not present and are accurately marked as implementation deliverables rather than current evidence.

Non-blocking risk: this is a design/product/executable baseline freeze，not production verification. Production activation evidence
remains assigned to FLA-0 through FLA-8.

Freeze decision: accepted as final Round 8 implementation-target baseline；production activation remains pending.

## Final Closure Status

- Blocking findings found in concrete-contract review: B4.1-B4.6 and B5.1.
- Accepted blocking findings fixed: yes.
- Fresh blocking closure reviews completed: Round 5、Round 6、Round 7、Round 8.
- Final blocking re-review passed: yes，Round 8.
- New blockers in final round: none.
- Rejected recommendation: advisory phrase classifier；replaced with typed contracts、fixed factual templates、goldens and static
  Runtime boundary audit，avoiding natural-language interpretation in Runtime.
- Deferred design decisions: none in the selected mainline.
- Disabled experiments: read/write Tool split、`required_next_call` removal、MCP output schema、DeepSeek strict.
- Missing production implementation: explicit and assigned to FLA-0 through FLA-8；not claimed complete.
- Target benefits proven: no；evaluation contract is frozen，but production runs have not occurred.
- Allowed to proceed: yes，as a frozen implementation-target baseline only.

## Final Conclusion After Concrete Review

R7 TaskSpace five-layer design is no longer only an abstract architecture description. The frozen baseline now includes exact L1/L2
text、an exact L3 Skill、a complete L4 input schema、a complete L5 result schema、deterministic lifecycle oracles、a reproducible
evaluation contract and an FLA-0 through FLA-8 production matrix. Independent Round 8 review passed with no open blockers.

This conclusion does not claim that the five-layer refactor is implemented or beneficial. Those claims require the future production
paths、tests、logs、Docker samples and formal matrix specified by the frozen executable contract.
