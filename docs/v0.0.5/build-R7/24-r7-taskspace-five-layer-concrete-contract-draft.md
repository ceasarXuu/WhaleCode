# R7 TaskSpace 五层具体合同评审稿

- Created: 2026-07-20
- Document Version: 1.0
- Status: L1-L3 production active; FLA-3.5 action-carried lifecycle repair active
- Architecture Source: [R7 TaskSpace 五层交互架构设计](23-r7-taskspace-five-layer-architecture-design.md)
- Scope: Agent 实际可见的提示词、Skill、Tool schema、反馈和 projection 示例
- Implementation Status: FLA-0 至 FLA-3.5 已实施并验证；FLA-4 为下一阶段
- Authority: [R7 五层架构可执行规格](25-r7-five-layer-executable-spec.md) 与
  [`five-layer-contract-authority-v1.json`](../../../benchmarks/taskspace/r7/five-layer-contract-authority-v1.json)

## 1. 这份文档解决什么问题

架构文档说明了信息应该归属于哪一层，但仅靠抽象职责无法判断 Agent 最终看到的内容是否：

- 足够理解 TaskSpace，却没有被开发者设计背景污染；
- 在 Base、协议、Tool 和反馈之间重复或冲突；
- 过长、过细，或者关键操作不够显著；
- 把 Runtime 的机械底线写成了对 Agent 的语义指导；
- 在正确时机暴露，并符合 DeepSeek 的真实 wire 结构。

本文件把五层展开成可以逐字审阅的选定内容。它不是第二份架构真相：五层职责以 `23` 号文档为准，逐字文本、
完整 schema 和实施验收以 authority manifest 与 `25` 号规格为准。若发生冲突，阶段必须停止并修正文档或
权威 artifact，不能让实施者自行选择。

> 2026-07-22 supersession：第 6 节的 `required_next_call + top-level sibling` schema 和调用示例仅保留为
> H-003 历史复现材料，不代表当前生产合同。当前合同是普通动作 Tool 携带必填 `taskspace_action`；旧字段、
> missing-sibling preflight 和非终态独立 control 已删除。实现与结果见
> [R7 连续动作合同回归修复](33-r7-continuous-action-regression-repair-plan.md)。

## 2. Agent 实际看到的总体结构

### 2.1 固定和动态内容

| 内容 | 暴露时机 | DeepSeek wire | 是否每请求存在 | 大致目的 |
|---|---|---|---|---|
| L1 TaskSpace Base | session/profile 建立后 | 第一条 `system` | 是 | 建立默认工作方式和 Map 宏观模型 |
| L2 Core Working Protocol | TaskSpace session 建立后 | 第二条 `system` 的首段 | 是 | 给出普通任务的简洁工作循环 |
| L3 Skill catalog entry | Skill 可用时 | 第二条 `system` 的 Skill 目录段 | 是 | 让 Agent 知道何时可加载高级方法 |
| L3 Skill body | 用户点名或 Agent 主动读取时 | `<skill>` user item 或普通 Tool result | 否 | 提供复杂任务经验和示例 |
| L4 Tool definitions | Tool set 构造后 | 顶层 `tools` | 是 | 给出精确能力、参数和副作用合同 |
| L5 Tool result | Tool 执行后 | `tool` message | 按调用出现 | 返回本次执行事实和失败事实 |
| L5 projection | 按 session policy | system/tool/history tail | 依策略 | 暴露当前 Map 的确定性视图 |

### 2.2 DeepSeek 请求骨架

下面是结构示例，不省略层次，但省略本设计不修改的 Codex 通用 Base 正文和普通工具定义。
TaskSpace 新增或改写的文字在后续章节逐字给出，不用占位符代表待定语义：

```json
{
  "messages": [
    {
      "role": "system",
      "content": "<完整 Codex-derived Base，其中包含本文件第 3 节的 TaskSpace 段落>"
    },
    {
      "role": "system",
      "content": "<本文件第 4 节 L2>\n\n<permissions>...</permissions>\n\n<AGENTS instructions>...\n\n<skills catalog>..."
    },
    {
      "role": "user",
      "content": "修复订阅状态更新后缓存没有失效的问题，并补充测试。"
    },
    {
      "role": "system",
      "content": "<按 projection policy 产生的 bootstrap/current projection 或 Map handle>"
    }
  ],
  "tools": [
    "<taskspace_control definition>",
    "<普通编码工具>"
  ]
}
```

这里没有 DeepSeek 原生 `developer` role。第二条 `system` 是 WhaleCode 内部 developer bundle 的 wire 映射；L2
只是该 bundle 的第一个稳定 section，不获得额外模型权限。

### 2.3 精确装配位置

本设计不重写整份 Codex-derived Base。它的装配边界是可机械检查的：

1. Standard 继续使用现有 `whalecode_standard.md`，不注入 L1-L5 任何 TaskSpace 内容。
2. TaskSpace 继续使用完整
   [`whalecode_taskspace.md`](../../../third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md)；
   只将其 `## TaskSpace work map` 到下一个同级 `## Task execution` 之前的区间替换为第 3.1 节。
3. 第 4.1 节原样放入第二条 `system` 的最前面，后面依次是 permissions、AGENTS instructions
   和 Skill catalog。
4. L3 Skill 正文只在加载时进入历史；L4 只在顶层 `tools` 中；L5 只出现在 Tool result 或
   当前 projection policy 规定的尾部位置。
5. 上述区间以外的 Base 文字不因 TaskSpace 方法迁移而改写，但发现跨层合同泄漏时必须在 Standard 和
   TaskSpace 两份 Base 中同步修正。现行完整中文对照见
   [TaskSpace Base 中文审阅稿](22-whalecode-taskspace-base-instructions.zh-CN.md)。

因此，评审 L1/L2 时可以明确看到：原 Base 的通用编码能力、AGENTS、执行、验证、进度和最终
回复规则仍在；TaskSpace 宏观模型不再在 Base 内同时承担完整操作教程。

整份 WhaleCode Base 另受一条全局所有权规则约束：可以描述通用工具行为，但不得出现 JSON Tool 参数、
provider 调用模板、patch 正文文法或其他 Tool wire 字节。这些内容只属于 L4 schema；从 Codex 默认 Base
继承的内容不构成例外。Standard 与 TaskSpace 必须由同一合同测试检查该边界。

## 3. L1 Base Instructions 逐字选定文本

### 3.1 英文 wire 权威文本

以下文字计划替换当前 TaskSpace Base 中过细的 Map 操作说明。它只讲工作方式、Map 的作用、核心概念和责任
边界，不解释线性上下文的缺陷，也不列 action 字段。

```markdown
## TaskSpace work map

Use the TaskSpace Map as the default way to organize and advance work. The Map is the global work view for the user's goal, work nodes, dependencies, current progress, and the path to completion.

- The Root is the Map's unique source, represents the user's task, and remains open while the task is in progress.
- Work nodes represent meaningful units of work with clear goals and completion boundaries.
- Directed dependency edges express which work must be completed before other work becomes ready. A Work node may depend on more than one predecessor.
- The Finish is the Map's unique sink and explicit endpoint. Every Work node belongs to at least one directed path from Root to Finish. Finish is closed only when the Agent has completed and verified the task and is ready to provide the final summary.
- The active binding identifies the Work node currently served by ordinary tool calls.

Keep the Map aligned with the work you are actually doing. Create or revise its structure when your understanding of the task changes, and update lifecycle state at meaningful work boundaries rather than after every minor tool result.

You decide how to decompose the task, which dependencies are meaningful, what evidence is sufficient, and when work is complete. The Runtime maintains the Map, enforces its mechanical invariants, and reports exact state changes or failures. It does not choose your plan, interpret task meaning, or decide the next action for you.
```

### 3.2 中文语义审阅稿

本段只用于中文评审，实施时以确认后的英文稿为 wire 权威文本。

> 使用 TaskSpace Map 作为组织和推进工作的默认方式。Map 是用户目标、工作节点、依赖关系、当前进度和完成
> 路径的全局工作视图。
>
> Root 是 Map 唯一起点，表示用户任务，在任务进行期间保持打开。Work 节点表示目标和完成边界清楚的工作
> 单元。有向依赖边表示哪些工作必须先完成，一个 Work 可以依赖多个前置节点。Finish 是 Map 唯一终点和明确的
> 结束位置；每个 Work 节点都必须位于至少一条从 Root 到 Finish 的有向路径上。只有 Agent 完成并验证任务、准备
> 提交最终总结时才闭合 Finish。active binding 表示当前普通工具调用所服务的 Work 节点。
>
> 让 Map 与真实工作保持一致。当你对任务的理解发生变化时，创建或修订 Map 结构；在有意义的工作边界更新
> 生命周期状态，不需要在每个细小工具结果后更新。
>
> 任务如何拆解、哪些依赖有意义、证据是否充分以及工作何时完成，都由你决定。Runtime 维护 Map、执行机械
> 不变量并准确报告状态变化或失败；它不替你选择计划、解释任务含义或决定下一步行动。

### 3.3 L1 与完整 Base 明确不出现的内容

- `expected_revision`、`required_next_call` 等字段名。
- `initialize_map`、`complete_then_continue` 等 action 枚举。
- “线性上下文不适合复杂任务”等开发者设计动机。
- 完整错误恢复步骤、JSON 示例和 Tool 调用模板。
- 复杂 DAG、竞争假设、节点折叠等高级方法。

其中 JSON 示例、Tool 调用模板和 patch 正文文法禁止出现在整份 WhaleCode Base；其余条目描述 TaskSpace
专属 L1 段的边界。

## 4. L2 Core Working Protocol 逐字选定文本

### 4.1 英文 wire 权威文本

```markdown
<taskspace_core_protocol version="taskspace-core-v2">
## Working with the Map

Use this loop for ordinary TaskSpace work:

1. At the start of substantive work, create a Map that reflects what is currently known, bind the first Ready Work node, and begin a real action for that node in the same response.
2. Perform ordinary tool calls under the active binding. Independent calls may run together; calls that depend on earlier results wait for those results.
3. Keep one Work node focused on one coherent goal. Do not update the Map after every minor result, but do revise it when the real work structure, dependencies, or active goal changes.
4. When the active Work node is complete and work continues, complete it, bind an Agent-selected Ready successor, and begin the successor's first real action in the same response.
5. Include validation inside the Work graph. When all Work is complete and the evidence is sufficient, explicitly close the unique Finish and provide the final summary.

## Reading results and recovering

- Treat each control result as the exact statement of whether state was committed. Do not infer success from intent or silently assume rollback.
- On rejection, read the returned action, submitted values, observed canonical values, revision, and state_commit fields, then choose your own correction.
- A previously read projection is current only when its revision matches the latest canonical revision visible in TaskSpace feedback or the Map handle.
- If evidence changes the plan, revise the Map before continuing under the new structure.
</taskspace_core_protocol>
```

### 4.2 中文语义审阅稿

本段只用于中文评审，实施时以确认后的英文稿为 wire 权威文本。

> 普通 TaskSpace 工作使用以下循环：
>
> 1. 开始实质工作时，根据当前已知信息建立 Map，绑定第一个 Ready Work，并在同一次回应中开始该节点的
>    真实动作。
> 2. 在 active binding 下执行普通工具调用。相互独立的调用可以同时执行；依赖早先结果的调用等待结果后再执行。
> 3. 每个 Work 聚焦一个连贯目标。不在每个细小结果后都更新 Map；真实工作结构、依赖或当前目标改变时才修订。
> 4. 当前 Work 完成且任务继续时，在同一次回应中完成它、绑定 Agent 选择的 Ready 后继，并开始后继的第一个
>    真实动作。
> 5. 验证作为 Work graph 中的真实工作。所有 Work 完成且证据充分时，显式闭合唯一 Finish 并提供最终总结。
>
> 读取结果与恢复时：把每次 control result 视为状态是否提交的精确陈述，不从调用意图推测成功，也不默认已回滚。
> 调用被拒绝时，读取 action、提交值、Runtime 观测到的 canonical 值、revision 和 state_commit，然后自主选择修正。
> 已读 projection 的 revision 只在等于最新 canonical revision 时才表示当前状态。证据改变计划时，先修订 Map，
> 再按新结构继续。

### 4.3 L2 为什么保留这些句子

| 句子 | 原因 | 不放到哪里 |
|---|---|---|
| 初始化并开始真实动作 | 普通任务都需要的工作顺序 | L1 不讲动作时序；Tool 只讲调用合同 |
| 独立调用可一起执行 | 避免把 TaskSpace 退化成一步一请求 | Runtime 不判断工具语义依赖 |
| 完成并继续时携带后继真实动作 | 避免单独 transition request | L4 由普通动作 Tool 的 transition 字段保证同一调用 |
| 验证属于 Work graph | 防止把 verify 与 Finish 混为一体 | Runtime 不判断测试是否充分 |
| 按 state_commit 和 revision 恢复 | 所有普通失败都需要 | Tool result 提供实际值，不教重规划 |

### 4.4 L2 明确不出现的内容

- 每个 action 的完整字段和 JSON Schema。
- 多父依赖应该如何设计、何时拆分节点等高级经验。
- 根据命令、Patch 或测试内容给出的下一步建议。
- 当前 Map revision、Ready 节点或 binding 等动态事实。

## 5. L3 Advanced Skill 选定内容

### 5.1 Catalog 中始终可见的条目

```text
- taskspace-advanced: Use for complex TaskSpace work that needs multi-branch DAG design, convergence of multiple prerequisites, long-session replanning, competing debug hypotheses, or recovery after major context compaction. Do not load for small linear tasks whose Map and recovery are already clear. (file: <session-pinned-snapshot>/taskspace-advanced/SKILL.md)
```

这段 description 的任务只有两个：说明什么时候值得加载，以及什么时候不要加载。它不包含任何硬规则。

### 5.2 `SKILL.md` 正文样例

```markdown
---
name: taskspace-advanced
description: Use for complex TaskSpace work that needs multi-branch DAG design, convergence of multiple prerequisites, long-session replanning, competing debug hypotheses, or recovery after major context compaction. Do not load for small linear tasks whose Map and recovery are already clear.
---

# Advanced TaskSpace Work

Use these methods only when they improve the current task. They are planning heuristics, not Runtime rules.

## Design a useful graph

- Prefer one Work node for one coherent deliverable or decision boundary.
- Keep tightly coupled edits in one node when separating them would create artificial handoffs.
- Use separate branches when work is genuinely independent or when independent evidence should converge before implementation.
- Add multiple incoming edges when a node truly requires several completed prerequisites. Do not create edges only to make the graph look more complex.

## Replan from evidence

- When evidence invalidates the current structure, revise the graph explicitly instead of preserving obsolete nodes as if they were still required.
- Mark blocked or rework state from observed facts. The choice of a new path remains yours.
- Preserve high-value evidence references when old node details are folded.

## Recover a long task

1. Read the current Map and identify the active binding, Ready frontier, blocked work, and latest canonical revision.
2. Inspect only the evidence needed to understand the active and nearest predecessor nodes.
3. Revise the Map if its structure no longer matches the remaining work.
4. Continue from a Ready node; do not replay old actions merely because they appear in history.

## Example: converging prerequisites

For a subscription-cache bug, a useful graph may be:

root -> reproduce
root -> inspect-invalidation-path
reproduce -> identify-root-cause
inspect-invalidation-path -> identify-root-cause
identify-root-cause -> implement-fix
implement-fix -> verify-regression
verify-regression -> finish

If reproduction and code inspection are tightly coupled in the actual task, combining them into one investigation node is also valid. Choose the graph that matches the work rather than optimizing for node count.
```

### 5.3 两种加载路径示例

用户显式点名时，宿主在请求前注入：

```xml
<skill>
<name>taskspace-advanced</name>
<path>/.../.snapshots/<sha>/taskspace-advanced/SKILL.md</path>
<完整正文>
</skill>
```

Agent 自主判断需要时，调用现有文件读取工具打开 catalog path，正文作为普通 Tool result 返回。Runtime 不因为
“任务看起来复杂”而自动加载，也不在 Agent 读取后再次注入 `<skill>`。

## 6. L4 当前生产 carrier 与历史回归

### 6.1 当前 `taskspace_control` 顶层 description

```text
Use taskspace_control to initialize and change the canonical TaskSpace Map, bind Work nodes, commit lifecycle transitions, expand folded node details, and read retained TaskSpace facts. Each call selects one action schema. Successful calls return the committed revision and exact delta or an exact read result; rejected calls return a structured error and whether any state was committed. Use it only for Map state and retained TaskSpace data, not to wrap ordinary tool names, commands, patch content, or reasoning. The Runtime validates mechanical graph and state invariants but never chooses nodes, repairs arguments, or decides the next action.
```

### 6.2 后续独立实验：拆出 `taskspace_read`

```text
Use taskspace_read to retrieve the current rendered Map or exact retained output referenced by TaskSpace. It never changes canonical Map state. Results identify the Map revision, returned range, truncation, and continuation reference when applicable. Read results are factual snapshots: a projection is current only while its revision matches the latest canonical revision reported by TaskSpace.
```

主线不暴露这个 Tool；`read_map` 和 `read_output_ref` 都属于 `taskspace_control`。只有 FLA-6-E1 独立 A/B
接受后，才使用上面的逐字 description 拆分读 Tool。实验不得与 action 改名、result V2 或其他 Tool 候选叠加。

### 6.3 当前 action-local 描述和字段

下表记录当前生产 action。`initialize_map`、`bind_node`、`complete_then_continue` 只作为普通动作 Tool 的
`taskspace_action` 出现；普通动作继续服务当前节点时使用 `continue_current`，其余独立 Map 操作仍属于
`taskspace_control`。

| Tool | Action | Agent 可见描述 | 核心必填字段 | 成功副作用 |
|---|---|---|---|---|
| ordinary carrier | `initialize_map` | Create the initial rooted DAG and execute this first real action under its initial binding. | root、initial_work_node、finish_identity、additional_work_nodes、edges | 初始化 Map；当前 ordinary Tool 同 call 执行 |
| control | `mutate_graph` | Atomically add Work nodes or dependency edges and remove eligible edges from the current Map. It does not choose or bind a node. | expected_revision、add_nodes、add_edges、remove_edges | revision +1；图结构原子变更 |
| ordinary carrier | `bind_node` | Bind one Agent-selected Ready Work node and execute this node's first real action. | expected_revision、node_id | 目标节点进入 running；当前 ordinary Tool 同 call 执行 |
| control | `block_node` | Mark a Work node blocked. It does not select an alternative path. | expected_revision、node_id | 节点进入 blocked |
| control | `unblock_node` | Return a blocked Work node to the mechanically derived lifecycle state after its blocker is cleared. | expected_revision、node_id | 节点回到 pending/ready |
| control | `rework_node` | Reopen a completed Work node because the Agent has decided more work is required. | expected_revision、node_id | 节点进入 rework/ready |
| ordinary carrier | `complete_then_continue` | Atomically complete the active Work node, bind one Agent-selected Ready successor, and execute its first real action. | expected_revision、current_node_id、next_node_id | 当前 completed；后继 running；当前 ordinary Tool 同 call 执行 |
| control | `finish_map` | Close the Map from one explicitly declared terminal lifecycle state. | expected_revision、terminal_state、terminal_node_id、final_summary | `last_running_work` 完成最终 Work 后闭合；`no_active_work_ready_finish` 直接闭合已 Ready 的 Finish |
| control | `expand_nodes` | Mark previously folded node details for full inclusion in future projections. It does not change graph lifecycle state. | node_ids | 更新显示状态，不改变任务判断 |
| control | `read_map` | Return the current full rendered Map and its canonical revision. | action | 无写入 |
| control | `read_output_ref` | Return an exact retained output range by reference. Select one discriminator branch for `head`、`tail`、`line_range` 或 `grep`. | output_ref + mode 对应字段 | 无写入 |

### 6.4 历史 sibling schema 摘录

完整、可直接生成 provider Tool definition 的权威文件是
[`five-layer-taskspace-control-v2.schema.json`](../../../benchmarks/taskspace/r7/five-layer-taskspace-control-v2.schema.json)。
它内联展开全部 action 和 `read_output_ref` 四种 mode，不使用 `$ref`，并冻结 capability profile 的唯一机械
变换。以下两支只用于文档阅读，不是待实施者补全的“骨架”：

```json
{
  "type": "function",
  "function": {
    "name": "taskspace_control",
    "description": "<6.1 的逐字文本>",
    "parameters": {
      "type": "object",
      "anyOf": [
        {
          "properties": {
            "action": { "type": "string", "enum": ["initialize_map"] },
            "root": {
              "type": "object",
              "properties": {
                "node_id": { "type": "string" },
                "goal": { "type": "string" }
              },
              "required": ["node_id", "goal"],
              "additionalProperties": false
            },
            "initial_work_node": {
              "type": "object",
              "properties": {
                "node_id": { "type": "string" },
                "goal": { "type": "string" }
              },
              "required": ["node_id", "goal"],
              "additionalProperties": false
            },
            "finish_identity": {
              "type": "object",
              "properties": { "id": { "type": "string" } },
              "required": ["id"],
              "additionalProperties": false
            },
            "additional_work_nodes": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "node_id": { "type": "string" },
                  "goal": { "type": "string" }
                },
                "required": ["node_id", "goal"],
                "additionalProperties": false
              }
            },
            "edges": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "from": { "type": "string" },
                  "to": { "type": "string" }
                },
                "required": ["from", "to"],
                "additionalProperties": false
              }
            },
            "required_next_call": { "type": "string", "enum": ["ordinary_tool", "apply_patch"] }
          },
          "required": ["action", "root", "initial_work_node", "finish_identity", "additional_work_nodes", "edges", "required_next_call"],
          "additionalProperties": false
        },
        {
          "properties": {
            "action": { "type": "string", "enum": ["complete_then_continue"] },
            "expected_revision": { "type": "integer" },
            "current_node_id": { "type": "string" },
            "next_node_id": { "type": "string" },
            "required_next_call": { "type": "string", "enum": ["ordinary_tool", "apply_patch"] }
          },
          "required": ["action", "expected_revision", "current_node_id", "next_node_id", "required_next_call"],
          "additionalProperties": false
        }
      ]
    }
  }
}
```

以上是 H-003 的历史回归 schema，仅用于解释根因。当前生产让真实动作自身携带轻量
`taskspace_action`；该字段必须显式选择 `continue_current` 或生命周期动作。`required_next_call` 和 sibling
preflight 已删除，不保留兼容路径。

### 6.5 历史 sibling 组合调用示例

```json
[
  {
    "name": "taskspace_control",
    "arguments": {
      "action": "complete_then_continue",
      "expected_revision": 3,
      "current_node_id": "investigate",
      "next_node_id": "implement",
      "required_next_call": "apply_patch"
    }
  },
  {
    "name": "apply_patch",
    "arguments": "*** Begin Patch\n...\n*** End Patch"
  }
]
```

不允许把第二个调用嵌入 control 参数：

```json
{
  "action": "complete_then_continue",
  "next_tool": "apply_patch",
  "patch": "..."
}
```

## 7. L5 Factual Feedback 选定合同

完整结果权威是
[`five-layer-taskspace-result-v2.schema.json`](../../../benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json)。
下面示例必须与它一致；所有未展示分支也由该 schema 和生命周期 oracle 直接判定。

### 7.1 成功提交

```json
{
  "schema_version": "TaskSpaceControlResultV2",
  "action": "complete_then_continue",
  "status": "committed",
  "success": true,
  "state_commit": true,
  "partial_commit": false,
  "canonical_revision": 4,
  "submitted_expected_revision": 3,
  "committed_revision": 4,
  "delta": {
    "map_id": "map-42",
    "committed_revision": 4,
    "graph_event_refs": [
      {"revision": 4, "event_id": "event-4", "event_type": "complete_then_continue"}
    ],
    "node_detail_event_refs": []
  },
  "steps": [
    {
      "kind": "complete_then_continue",
      "map_id": "map-42",
      "current_node_id": "investigate",
      "next_node_id": "implement",
      "revision": 4
    }
  ],
  "read": null,
  "error": null
}
```

这里不出现“现在请修改文件”或“建议运行测试”。后续动作由 Agent 已提交的 carrier action 或下一次语义决策决定。

### 7.2 状态机拒绝

```json
{
  "schema_version": "TaskSpaceControlResultV2",
  "action": "complete_then_continue",
  "status": "state_machine_failed",
  "success": false,
  "state_commit": false,
  "partial_commit": false,
  "canonical_revision": 4,
  "submitted_expected_revision": 3,
  "committed_revision": null,
  "delta": null,
  "steps": [],
  "read": null,
  "error": {
    "class": "state_machine",
    "code": "TASKSPACE_STALE_REVISION",
    "message": "expected_revision does not match the current canonical revision",
    "actual": { "canonical_revision": 4 },
    "expected": { "submitted_expected_revision": 3 }
  }
}
```

错误只陈述哪个机械条件不成立，不补写可执行的下一步。
`actual` 始终是 Runtime 观测到的 canonical 事实，`expected` 是调用者在输入中声明的条件；
两者不得根据自然语言 message 的句式交换方向。

### 7.3 历史回归基线的 Response preflight 拒绝

```json
{
  "schema_version": "TaskSpaceControlResultV2",
  "action": "complete_then_continue",
  "status": "protocol_failed",
  "success": false,
  "state_commit": false,
  "partial_commit": false,
  "canonical_revision": 3,
  "submitted_expected_revision": 3,
  "committed_revision": null,
  "delta": null,
  "steps": [],
  "read": null,
  "error": {
    "class": "protocol",
    "code": "TASKSPACE_REQUIRED_SIBLING_MISSING",
    "message": "complete_then_continue requires a following top-level ordinary_tool call in the same response",
    "actual": {"next_call_kind": "response_end"},
    "expected": {"next_call_kind": "ordinary_tool"}
  }
}
```

整个 batch 在执行前被拒绝，所以 `state_commit=false`。该示例只记录旧 sibling 回归；当前 schema 不再允许
单独非终态 transition，因此没有 missing-sibling 分支。

### 7.4 carrier 的交接已提交、普通工具随后失败

这是同一个 provider call id 下的两个独立事实。当前实现先返回短 `TaskSpaceCarrierResultV2` 头，再原样附加
普通 Tool 输出：

```text
{"schema_version":"TaskSpaceCarrierResultV2",
 "action_result":{"status":"committed","state_commit":true,"canonical_revision":4},
 "tool_dispatched":true}
<原 Tool 失败输出保持不变>
```

Agent 能据此知道 binding 已切换，但普通动作失败。Runtime 不自动回滚 Map，也不替 Agent 决定重试或修订节点。
transition 失败时 `tool_dispatched=false`，普通 Tool 不进入执行链。FLA-5 再正式化全载体结果一致性，不在
FLA-3.5 增加 prepare、reservation 或新的结果代数层。

### 7.5 截断读取

```json
{
  "schema_version": "TaskSpaceControlResultV2",
  "action": "read_output_ref",
  "status": "read_ok",
  "success": true,
  "state_commit": false,
  "partial_commit": false,
  "canonical_revision": 4,
  "submitted_expected_revision": null,
  "committed_revision": null,
  "delta": null,
  "steps": [],
  "read": {
    "kind": "output_range",
    "output_ref": "output://tool-call-91",
    "mode": "line_range",
    "range": {"start_line": 1, "end_line": 200},
    "truncated": true,
    "continuation": {
      "action": "read_output_ref",
      "output_ref": "output://tool-call-91",
      "mode": "line_range",
      "start_line": 201,
      "end_line": 400,
      "max_bytes": 16384
    },
    "content": "<原始第 1 到 200 行，未经总结或改写>"
  },
  "error": null
}
```

## 8. 三种 Projection 的具体暴露

假设 canonical Map 是 revision 4：

```text
root-task [OPEN]
  investigate [COMPLETED]
  implement [RUNNING, BOUND]
  verify [PENDING]
  finish [PENDING]

root-task -> investigate -> implement -> verify -> finish
```

### 8.1 `map-always`

每次请求末尾只有一份可替换 current projection：

```yaml
TaskSpaceMapProjectionR7V1:
  projection_kind: current_projection
  map_id: map-42
  revision: 4
  canonical_sha256: abc123
  root: { id: root-task, status: open }
  binding: implement
  ready: []
  nodes:
    - { id: investigate, status: completed }
    - { id: implement, status: running }
    - { id: verify, status: pending }
    - { id: finish, status: pending }
  edges:
    - [root-task, investigate]
    - [investigate, implement]
    - [implement, verify]
    - [verify, finish]
TaskSpaceMapProjectionR7V1 end.
```

### 8.2 `map-append`

每个有效 request 末尾追加不可变 snapshot：

```yaml
TaskSpaceMapProjectionR7V1:
  projection_kind: request_snapshot
  map_id: map-42
  revision: 4
  supersedes_all_prior_projections: true
  current_state_rule: last_projection_only
  canonical_sha256: abc123
  root: { id: root-task, status: open }
  binding: implement
  ready: []
  nodes:
    - { id: investigate, status: completed }
    - { id: implement, status: running }
    - { id: verify, status: pending }
    - { id: finish, status: pending }
  edges:
    - [root-task, investigate]
    - [investigate, implement]
    - [implement, verify]
    - [verify, finish]
TaskSpaceMapProjectionR7V1 end.
```

旧 projection 仍在历史中，但只有最后一份用于 current state。相同 revision 可以在不同有效 request 中重复；完全
相同 payload 的 provider retry 不重复追加。

### 8.3 `map-request`

普通请求只自动携带机械 handle：

```yaml
TaskSpaceMapHandleR7V1:
  active: true
  map_id: map-42
  canonical_revision: 4
  read_action: taskspace_control.read_map
TaskSpaceMapHandleR7V1 end.
```

Agent 调用 `read_map` 后，projection 作为 Tool result 追加。若随后控制提交 revision 5，之前读到的 revision 4
仍是历史事实但不再是 current；Runtime 不自动提醒或强迫重读。

## 9. 端到端正常路径示例

### 9.1 用户请求

```text
修复订阅状态更新后缓存没有失效的问题，并补充回归测试。
```

### 9.2 Agent 首次响应中的 carrier 调用

```json
{
  "name": "exec_command",
  "arguments": {
    "cmd": "rg -n \"subscription|cache|invalidate\" src tests",
    "workdir": "/workspace",
    "taskspace_action": {
      "action": "initialize_map",
      "root": { "node_id": "subscription-cache-fix", "goal": "修复订阅状态更新后的缓存失效并验证" },
      "initial_work_node": { "node_id": "investigate", "goal": "定位状态更新与缓存失效链路" },
      "finish_identity": { "id": "finish" },
      "additional_work_nodes": [
        { "node_id": "implement", "goal": "实施根因修复" },
        { "node_id": "verify", "goal": "执行回归验证" }
      ],
      "edges": [
        { "from": "subscription-cache-fix", "to": "investigate" },
        { "from": "investigate", "to": "implement" },
        { "from": "implement", "to": "verify" },
        { "from": "verify", "to": "finish" }
      ]
    }
  }
}
```

这体现三个关键行为：先建立 Map；初始化与第一个真实动作在同一请求；节点粒度按实际连贯工作划分，没有为了
展示 DAG 强行拆成大量节点。

### 9.3 调查完成后继续

```json
{
  "name": "apply_patch",
  "arguments": {
    "input": "*** Begin Patch\n<一个连贯 Patch>\n*** End Patch",
    "taskspace_action": {
      "action": "complete_then_continue",
      "expected_revision": 1,
      "current_node_id": "investigate",
      "next_node_id": "implement"
    }
  }
}
```

### 9.4 实现完成后进入验证

```json
{
  "name": "exec_command",
  "arguments": {
    "cmd": "cargo test subscription_cache",
    "workdir": "/workspace",
    "taskspace_action": {
      "action": "complete_then_continue",
      "expected_revision": 2,
      "current_node_id": "implement",
      "next_node_id": "verify"
    }
  }
}
```

### 9.5 最终闭合

```json
{
  "name": "taskspace_control",
  "arguments": {
    "action": "finish_map",
    "expected_revision": 3,
    "terminal_state": "last_running_work",
    "terminal_node_id": "verify",
    "final_summary": "修复订阅状态提交后的缓存失效路径，并新增覆盖状态更新的回归测试；定向测试通过。"
  }
}
```

终局 action 可以单独出现，因为它直接结束 Map；非终局 complete/bind 不应形成空转请求。

## 10. 失败与自主恢复示例

Agent 使用旧 revision 3 调用 `complete_then_continue`，但 canonical revision 已经是 4：

```json
{
  "schema_version": "TaskSpaceControlResultV2",
  "action": "complete_then_continue",
  "status": "state_machine_failed",
  "success": false,
  "state_commit": false,
  "partial_commit": false,
  "canonical_revision": 4,
  "submitted_expected_revision": 3,
  "committed_revision": null,
  "delta": null,
  "steps": [],
  "read": null,
  "error": {
    "class": "state_machine",
    "code": "TASKSPACE_STALE_REVISION",
    "message": "expected_revision does not match the current canonical revision",
    "actual": { "canonical_revision": 4 },
    "expected": { "submitted_expected_revision": 3 }
  }
}
```

反馈没有写“请先 read_map 再重试”。Agent 可以根据已有 control feedback、Map handle 或主动读取来决定如何恢复。
只要下一次调用符合 revision 和状态机硬约束，Runtime 就允许执行，不判断这是不是最聪明的恢复方式。

## 11. 跨层重复检查

| 信息 | 唯一正文所有者 | 其他层如何出现 |
|---|---|---|
| Map 的作用和核心概念 | L1 | L2 直接使用这些名词，不重新解释价值 |
| 普通工作循环 | L2 | L1 只说默认使用；L4 描述单个 action |
| action 名、字段、必填条件 | L4 | L2 不列字段；L5 error 只回显本次 action/actual/expected |
| 当前 revision、binding、Ready frontier | L5 | L1-L4 不写动态值 |
| 复杂 DAG 和长任务恢复经验 | L3 | L1/L2 保证不加载 Skill 也能正确工作 |
| 是否提交、提交了什么、为什么拒绝 | L5 | L2 只要求忠实读取结果，不重述本次事实 |

存在必要的跨层概念引用，例如 L1 解释 Finish，L4 描述关闭 Finish 的 action。判断重复的标准不是相同单词只能
出现一次，而是同一规则不能在多个层以不同措辞分别成为权威。

## 12. 用户需要重点判断的问题

### 12.1 L1

- 这段是否只说明 Map 的作用和默认工作方式，没有暴露开发者设计动机？
- Root/Work/edge/Finish/binding 是否都属于每次请求需要的宏观概念？
- 是否仍然太长，或者缺少 Agent 开始工作前必须知道的概念？

### 12.2 L2

- 普通工作循环是否足够清楚？
- “同一响应携带第一个真实动作”是否应该属于常驻方法层？
- 失败恢复是否只教 Agent读事实，没有替它决定下一步？

### 12.3 L3

- Catalog description 能否让 Agent正确判断复杂任务才加载？
- Skill 正文是否确实是高级经验，而不是把普通正确性藏进可选内容？
- DAG 示例是否尊重 LLM 倾向于完成连贯工作，而不是追求节点数量？

### 12.4 L4

- 顶层 description 是否足以正确选 Tool，又没有重复全部 action？
- Action-local 描述、字段和组合调用是否比当前超长 Tool 更容易理解？
- 读写拆分是否真的有价值，还是应继续保持单 Tool？

### 12.5 L5

- 成功、拒绝和部分链路失败是否完整保留事实？
- Error 是否存在建议、诱导或语义再解释？
- 三种 projection 的暴露差异是否足够清楚，且没有改变同一份 Map？

## 13. 落地方式

L1-L5 当前生产内容和哈希记录在 authority manifest；L1-L3 与 FLA-3.5 carrier 已接入生产，旧 sibling 形态只作
历史回归材料。实施按 `25` 号规格的 FLA-0 至 FLA-3、FLA-3.5、FLA-4 至 FLA-8 逐阶段进行；每层的英文文本和 schema
进入对应版本化生产 artifact，本 Markdown 不作为 Runtime 读取源。
任何一层发生实质改写，都要先更新权威 artifact 与 hash，再作为独立变量测试，不能一次性整体替换。
