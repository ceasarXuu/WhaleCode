# R7 双基础提示词设计

- Created: 2026-07-20
- Version: 1.0
- Status: Implemented and diagnostic validated
- Scope: Standard / TaskSpace provider base instructions
- Compatibility: none

> 后续设计说明：`23-r7-taskspace-five-layer-architecture-design.md` 提议在仍保持“每个 profile 只有一份完整
> Base”的前提下，从 TaskSpace Base 等价提取一个独立、版本化的 Core Working Protocol artifact。该提议尚未
> 实施；通过 FLA 迁移门禁后，它将 supersede 本文对“附加 developer protocol 一律不存在”的限制，但不会
> 恢复旧“极简 Base + 点状附加规则”结构。当前生产状态仍以本文为准。

当前提示词版本：Standard `1.0.2`，TaskSpace `2.0.1`。Standard `1.0.2` 与 TaskSpace `2.0.1` 删除了从
Codex Base 继承的具体 Tool wire 调用示例，只保留通用文件编辑行为；参数、调用语法和补丁文法只由
provider 可见 Tool schema 负责。旧版本 Docker 结果继续作为历史基线保留。

## 1. 决策

WhaleCode 不再使用“极简 Whale base + TaskSpace developer message”的组合。每个 provider request 必须且
只能选择一份完整的 `base_instructions`：

| 会话工作方式 | 完整基础提示词 | 计划工具 |
|---|---|---|
| Standard | WhaleCode Standard Base | `update_plan` |
| TaskSpace | WhaleCode TaskSpace Base | `taskspace_control` |

三种 TaskSpace projection policy 只决定 Map projection 如何进入上下文，全部共享同一 TaskSpace base、工具、
状态机、事件存储、校验器和请求管线。

## 2. 根因

旧 Whale base 只有一段通用身份说明，抛弃了 Codex 已经长期验证的 Agent 工作框架。TaskSpace 再以独立
developer message 追加点状规则，产生三个问题：

1. 基础工作方法缺失，Agent 只看到局部调用规则，不理解完整的软件工程工作方式。
2. TaskSpace 的设计意图和 Map 方法位于较弱、较孤立的附加消息中，容易被更完整的基础行为框架淹没。
3. base、developer protocol 和 tool schema 平行描述同一工作方式，容易重复、冲突和版本漂移。

根因属于提示词架构错误，不应通过 Runtime 增加语义 gate 修复。

## 3. 两份 Base

### 3.1 Standard

Standard 直接继承 Codex 原生 `default.md` 的章节、规则和表达。当前允许两处品牌级文本差异，以及一处
跨层合同修正：把 Codex 产品身份改为 WhaleCode，移除与实际工作无关的产品背景说明，并把 Base 中具体
Tool wire 示例替换为通用工具行为。其余工作方式保持不变。

源文件：
`third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_standard.md`

`models.json` 中 DeepSeek 项只保留空 seed，所有模型目录入口统一通过 models-manager 装配这份权威文本；
显式 `config.base_instructions` 覆盖在装配后应用。

### 3.2 TaskSpace

TaskSpace 不是在 Standard 后追加一段协议，而是另一份完整 base。它保留 Codex 原生提示词的成熟框架，
只在与工作组织直接相关的位置进行有机改造：

1. 产品能力说明加入 TaskSpace Map。
2. `Planning` 章节替换为 `TaskSpace work map`，系统讲清楚 Map 的价值、图模型、使用方法和责任边界。
3. Tool Guidelines 中移除 `update_plan`，改为 `taskspace_control` 的简洁使用边界。
4. 工具字段、动态 Map 状态和 projection 内容不复制进 base，分别由 schema、feedback 和 projection 负责。

两份完整 Base 都不得嵌入 JSON 参数对象、patch 正文模板或其他 provider 调用字节。该约束作用于整份 Base，
不只作用于 TaskSpace Map 段；从 Codex 上游继承的内容也必须通过同一机器边界检查。

源文件：
`third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md`

## 4. 选择边界

模式、base profile 和计划工具可见性必须来自同一次 SessionState 快照：

```text
Standard  -> WhaleCode Standard Base -> update_plan visible
TaskSpace -> WhaleCode TaskSpace Base -> taskspace_control visible
```

禁止出现 TaskSpace base 配 Standard 工具集，或 Standard base 配 TaskSpace 工具集。每个 provider request
从一个状态快照选择两者，因此单次请求内部不会错配；模式切换自身继续沿用现有会话操作时序。

启动预热必须在 resume 恢复模式后进行。首轮前发生模式切换时，旧预热取消，并按新 profile 重建。

## 5. 配置与子 Agent

`config.base_instructions` 继续作为 Standard 的显式高级覆盖，并在日志中标记为不匹配内置合同。TaskSpace
base 是当前产品工作方式合同，不叠加该 Standard 覆盖。

普通子 Agent 当前不持有父 TaskSpace Map runtime 和控制工具，因此继续使用 Standard base。只有未来明确
建立子 Agent 的独立 TaskSpace runtime、Map ownership 和工具合同后，才能选择 TaskSpace base；不能只继承
提示词而缺失能力。

## 6. 版本与日志

两份 base 分别维护 `version + sha256`。任何字节变化都必须更新对应版本、哈希、机器合同和测试，禁止静默
修改。每次 provider request 记录：

- `profile`
- `version`
- `sha256`
- `bytes`
- `matches_current_contract`

provider wire trace v5 记录最终线上消息中的 `base_instructions_identity`；性能报告聚合为
`base_instructions_identity_summary`。预热使用单独事件记录相同身份。

## 7. Runtime 边界

本设计不扩大 Runtime 的语义责任。Runtime 只负责：

1. 按明确会话模式选择完整 base。
2. 保证 base 与计划工具可见性一致。
3. 校验 TaskSpace 图、revision、Ready、binding、Finish 和调用形状等硬规则。
4. 忠实传递 Map、工具结果和错误。

任务拆解、节点目标、依赖关系、完成判断、恢复方式和下一动作仍全部由 Agent 决定。projection 不负责补充
提示、解释反馈或替 Agent 纠错。

## 8. 验收

静态合同：

1. Standard 与 Codex 原生 prompt 行数一致，且只有两处品牌行和一处 Tool wire 边界差异。
2. 两份文件哈希与版本合同匹配。
3. TaskSpace base 不包含 `update_plan`，Standard 不教授 TaskSpace Map。
4. 两份 Base 都不包含 JSON Tool 参数、patch 正文模板或其他具体调用字节。
5. 旧 developer Working Protocol 注入和身份字段从生产代码、脚本中消失。

运行合同：

1. Standard 每个请求只出现 Standard base，且只暴露 `update_plan`。
2. TaskSpace 每个请求只出现 TaskSpace base，且只暴露 `taskspace_control`。
3. resume、首轮预热、模式切换、retry 和 compaction 不发生 profile/tool 错配。
4. Standard 与三种 TaskSpace policy 均完成 simple、complex Docker sample；报告 request、token、cache、耗时、
   Map 和动作路径。

本阶段先证明架构一致性和语义完整性。两份完整 base 带来的固定输入成本必须在正式样本中单独测量，不能
在没有行为证据时假定收益已经超过成本。

## 9. 实施结果

实现提交为 `5ecfefbcd`，观测提交为 `de7a8f547`。定向 Rust、模型目录、provider payload、PowerShell
合同和完整 CLI 构建均通过。`single-file-fast-fix` 在同一 Docker、模型和二进制下完成一次双臂诊断：

| 模式 | 结果 | 请求 | 输入 | 未缓存输入 | Req2+ 缓存 | Base 匹配 | Base 估算/请求 |
|---|---|---:|---:|---:|---:|---:|---:|
| Standard | solved | 6 | 65,949 | 11,293 | 97.46% | 6/6 | 5,313 |
| TaskSpace | solved | 7 | 95,545 | 14,009 | 96.95% | 7/7 | 5,503 |

两侧公开与隐藏验证均通过。所有线上请求都只有一份完整 base，profile、版本和哈希没有错配。TaskSpace
相对 Standard 的 base 固定差值约 190 estimated tokens/request；总输入差异还包含额外请求、工具 schema、
控制反馈和自然历史，不能全部归因于 base 文本。单次样本只作为接线与成本诊断证据。

## 10. Tool wire 边界修复

2026-07-21 将两份 Base 中继承自 Codex 默认提示词的具体 `apply_patch` JSON/patch 模板替换为通用文件编辑
行为。调用参数、调用语法和 patch 文法继续由 provider 可见 Tool schema 唯一负责。Rust 单测、双 Base 合同、
五层完整合同、TaskSpace manifest 身份测试和终端合同回归均通过。

Docker `single-file-fast-fix` 一次双臂冒烟结果如下；该结果只证明生产接线和基本完成性，不作为行为稳定性或
效用统计结论：

| 模式 | 结果 | 请求 | Patch | Base 身份匹配 | Manifest 身份匹配 |
|---|---|---:|---:|---:|---:|
| Standard `1.0.2` | solved | 5 | 1 次且一次成功 | 5/5 | N/A |
| TaskSpace `2.0.1` | solved | 11 | 1 次且一次成功 | 11/11 | 11/11（`1.0.2`） |

运行证据位于
`target/r7-five-layer/base-tool-wire-boundary-smoke/single-file-fast-fix/20260721-030950-043/`。本轮仍观察到
既有的 3 次 required-sibling preflight 拒绝；它不属于 Base Tool wire 泄漏修复范围，也没有被本轮变更隐藏。
