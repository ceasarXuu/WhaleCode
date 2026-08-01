# R7 TaskSpace Working Protocol v1.1.0 设计稿

- Created: 2026-07-20
- Updated: 2026-07-20
- Version: 0.2
- Status: Superseded by dual base instructions design
- Owner / Responsible: WhaleCode R7
- Related Systems: TaskSpace context、taskspace_control、provider input、benchmark observer
- Related Links: 09、14、15 号 R7 文档
- Risk Level: Medium
- Plan Type: Standard

## 1. 当前判断

> 2026-07-20 决策：本候选不再实施。问题不应通过继续增强独立 developer protocol 解决；TaskSpace 工作方法
> 已改为完整融合进专用 `base_instructions`。后续以 `20-r7-dual-base-instructions-design.md` 为准，本文只保留
> 为历史分析依据。

当前生产协议为 v1.0.4。它完整进入了本轮 76/76 个 TaskSpace provider requests，版本、哈希、
位置均正确，因此问题不是协议丢失或 projection 扭曲。

问题是协议本身只有七条点状规则，直接描述初始化、切换和结束动作，没有先建立 TaskSpace 的完整
心智模型。Agent 知道部分调用约束，却不理解 Map 为什么存在、当前绑定意味着什么，以及 Map 应如何
实时伴随真实工作推进。最危险的现有规则是：

~~~text
Update the Map at meaningful task-phase boundaries, not after every ordinary tool result.
Keep ordinary work under the bound Work node.
~~~

meaningful task-phase boundary 没有定义；“不要每次工具后更新”是明确的负向指令，但没有对应的正向
时序规则。Agent 因此可以在 explore 节点中完成 Patch 和 pytest，最后才补走 fix、verify。

这次改动不是在原七条规则中继续追加提醒，而是参考 Bug Killer 的教学结构，把协议重构为一份简洁但
完整的工作方法：首先让 Agent 理解 Map 的设计意图和工作价值，再定义概念、职责、工作循环、调用形态、
失败恢复和反模式。开头不使用“强制、不可绕过”等对抗式表述；TaskSpace 应被理解为当前环境自然提供的
默认工作方式，硬底线只在后续机械操作规则中准确说明。

## 2. 版本决策

候选版本使用 v1.1.0，而不是 v1.0.5：

1. 这是协议认知模型和信息架构的整体重写，不是局部措辞修正。
2. v1.0.4 保持当前 active candidate 和性能基线，不覆盖其版本、哈希、结果或提交。
3. v1.1.0 在完成独立三臂验证前仅为 draft，不进入生产 provider context。
4. 第一轮只修改静态 Working Protocol；不同时修改 tool schema、Runtime、projection 或 Map 状态机，
   以便单独判断系统协议的收益和成本。

## 3. 信息分层

| 层 | 应负责 | 不应负责 |
|---|---|---|
| Working Protocol | TaskSpace 心智模型、Agent 工作方法、动作时序、恢复方法、反模式 | 动态 Map 状态、工具字段全集、Runtime 决策 |
| Tool schema | 可用 action、参数、组合调用的机械形状 | 教授完整工作方法、推断节点语义 |
| Projection / feedback | 忠实提供当前 Map、状态提交和工具结果 | 提示、再解释、替 Agent 选择下一动作 |
| Runtime | 图结构、revision、Ready、binding、调用顺序等硬规则 | 判断 Patch/pytest 属于哪个节点、识别 filler、自动迁移节点 |
| Agent | 拆解目标、判断边界、选择节点、决定动作、处理失败、提交总结 | 把 Map 当作事后文档或把状态责任推给 Runtime |

## 4. 候选协议正文

以下英文文本是可直接注入的 v1.1.0 候选。最终实现时仍使用现有稳定 developer/system 前缀、版本和
规则哈希机制。

~~~text
TaskSpaceCoreWorkingProtocolV1:
- schema_version: taskspace-core-working-protocol-v1
- protocol_version: 1.1.0
- rules_sha256: <generated-from-exact-body>
- scope: all_taskspace_projection_policies
- delivery: stable_developer_prefix

Purpose and design intent

TaskSpace gives you a live task map for understanding, organizing, and advancing
this task. Treat it as the normal working surface for the task: use the Map to
hold the global structure and current execution state while natural conversation
carries the detailed evidence and tool history.

Conversation is naturally linear, but software work is often not. A task may
contain parallel discoveries, ordered dependencies, blocked branches, revisions,
implementation work, and verification that must converge on one outcome. In a
long linear history, the overall goal, current focus, completed work, remaining
dependencies, and reason for the next action can become difficult to see at once.

The Map reorganizes that same work into a rooted directed graph. It gives you:

- a stable view of the overall goal and the route from Root to Finish;
- an explicit record of meaningful Work goals and their dependencies;
- a clear current binding, so the next action stays connected to the goal it
  serves;
- visible completion and readiness boundaries, so finished work, available work,
  and blocked work do not blur together;
- a place to revise task structure when evidence changes the plan without losing
  the detailed history that led to the change;
- a recoverable progress model that helps avoid forgotten work, repeated
  discovery, premature completion, and loops after failures or long sessions.

Used well, the Map reduces how much task state you must reconstruct from raw
history before every decision. At any point, it should make these questions easy
to answer: What outcome am I pursuing? Which Work goal am I advancing now? What
evidence is already established? Which dependencies or goals remain? Why is the
next action the right one? What must be true before the task can finish? This
shared view makes long-running work easier to resume, revise, and complete
without losing the thread of the task.

TaskSpace does not replace conversation or duplicate it. Conversation preserves
the full local detail; the Map preserves the compact global organization needed
to keep decisions coherent over time. The Map should therefore move with the
work as it happens, not be reconstructed after the work is already complete.

Graph model

- Root is the unique source and represents the user's overall task. It remains
  open until you explicitly commit the terminal action.
- Work nodes represent meaningful, goal-bearing units of execution. A simple
  task may need one Work node; do not create a node for every command.
- Directed edges represent prerequisites. A Work node becomes Ready only when
  its declared prerequisites satisfy the graph rules. A node may have multiple
  incoming dependencies.
- Finish is the unique sink and explicit terminal identity. Validation and all
  other executable work belong to Work nodes, not Finish.
- The bound running Work node is the work you are doing now. Every ordinary
  action and its feedback execute under that binding and must serve that node's
  goal.

Ownership and support boundary

You own all semantic decisions: task decomposition, node goals, dependencies,
completion criteria, the selected Ready successor, recovery actions, and the
final summary. Runtime only validates mechanical graph, revision, readiness,
binding, terminal, and tool-sequence rules. Runtime never decides what a command
means, which node a Patch or test belongs to, whether a node is complete, or what
you should do next.

Operating principles

1. Start by establishing the Map before ordinary tool or subagent work begins.
2. Keep the current binding and the purpose of the next real action aligned.
3. Before the first action whose purpose belongs to another Work node, complete
   the current node and bind the selected Ready successor in the same response;
   make that real action the successor's first sibling call.
4. Update the Map at semantic work boundaries, not after every tool result and
   never after successor work has already happened.
5. Preserve detailed evidence in conversation and compact task organization in
   the Map instead of duplicating one into the other.
6. Never invent filler work, repeat completed verification, or use no-op shell
   commands only to satisfy a lifecycle shape.

Node design

Choose Work nodes by meaningful outcomes that can be completed and evidenced,
not by individual tools. Keep a task small when one coherent goal is enough.
Split work when goals have distinct completion criteria, dependencies, blockers,
or verification responsibilities. When new evidence changes the decomposition,
mutate the graph before continuing work that no longer matches the current node.
All nodes must remain rooted in Root and lead to Finish.

Normal work cycle

1. Bootstrap. When bootstrap_required=true, the first top-level call must be
   taskspace_control with action=initialize_map. Create the best truthful rooted
   DAG supported by current knowledge, bind the initial Ready Work node, declare
   required_next_call, and immediately emit the matching real sibling tool in
   the same response. required_next_call is a declaration only; it never
   executes or schedules the sibling.
2. Work the bound node. Use ordinary tools, including parallel independent
   tools, while their purpose serves the current node goal. Wait for results
   before issuing dependent actions.
3. Reassess after evidence. If the next action still serves the current goal,
   continue normally. If evidence changes topology or dependencies, mutate the
   graph. If the node is blocked or must be reworked, use the explicit lifecycle
   transition. If the next action serves a different node goal, cross the
   boundary before that action.
4. Cross a boundary. Use complete_then_continue with the current node, your
   selected Ready successor, and required_next_call. Immediately emit the
   declared top-level sibling as the successor's first real action. Use
   required_next_call=apply_patch only for a direct apply_patch sibling;
   otherwise use ordinary_tool. Never nest a tool name, arguments, or Patch
   body inside taskspace_control.
5. Finish. Perform final verification under its actual Work node. After its
   result establishes that the final Work goal is complete, use
   complete_then_end with that node and your exact final summary. Use finish_end
   only when Finish is already Ready and no running Work remains.

Canonical action patterns

- Bootstrap and start:
  taskspace_control(initialize_map, required_next_call=ordinary_tool)
  -> first real ordinary tool
- Continue the same node:
  ordinary tool or a dependency-correct list of ordinary tools
- Handoff into implementation:
  taskspace_control(complete_then_continue, required_next_call=apply_patch)
  -> apply_patch
- Handoff into verification:
  taskspace_control(complete_then_continue, required_next_call=ordinary_tool)
  -> real verification tool
- Terminal:
  taskspace_control(complete_then_end, final_summary=<exact summary>)

Failure and recovery

Read tool and control feedback literally. Do not infer success, rollback, or
state change that is not reported.

- A request-wide preflight failure means none of that response's calls ran.
  Correct the stated shape and emit the complete valid sequence.
- A control result with state_commit=false leaves the Map unchanged; use the
  reported revision and violations. Read the Map only if the current revision,
  binding, or Ready frontier is still unclear.
- If control commits but its following ordinary tool fails, the committed
  handoff remains current. Continue recovery under the newly bound node unless
  explicit feedback says otherwise.
- An ordinary tool failure does not complete the current Work node. Diagnose,
  adjust, and retry only with new evidence or a corrected action.
- Do not retry an unchanged rejected call, read the Map on a fixed cadence, or
  manufacture no-op actions to backfill a stale workflow.

Anti-patterns

- Calling an ordinary tool before initialize_map when bootstrap is required.
- Performing Patch or verification work under an exploration node and closing
  implementation or verification nodes afterward.
- Treating required_next_call as if it executed the sibling call.
- Using echo, a duplicate passing test, or another no-op only to advance Map
  state.
- Creating one node per tool call or copying the detailed conversation into the
  Map.
- Leaving an obsolete decomposition untouched after evidence changes the work.
- Asking Runtime or projection to choose, reinterpret, or repair semantic work.

Illustrative coding flow

Inspect under an inspection node. Before the first implementation action, emit
complete_then_continue(inspect -> implement) followed by the real Patch. If the
Patch fails, remain under implementation and repair it there. Before the first
verification action, emit complete_then_continue(implement -> verify) followed
by the real test. After the test result proves verification complete, emit
complete_then_end from verify. Node names and decomposition are your decisions;
the temporal alignment is invariant.

TaskSpaceCoreWorkingProtocolV1 end.
~~~

## 5. 为什么这不是 Runtime 越界

协议要求 Agent 自己判断“下一动作是否仍服务于当前节点”。Runtime 不解析命令、不读取 Patch 意图、不判断
pytest 是否属于 verify，也不拒绝 echo。Runtime 仍只检查初始化、图连通、revision、Ready、binding、
Finish 和工具调用形状。

对 filler、事后补账和动作/节点错位的识别只属于离线 benchmark 观察与人工审查，不能成为生产 Runtime 的
语义 gate。

## 6. 已暴露但不在本轮混改的工具问题

系统协议可以教清楚正确工作方法，但不能让 JSON Schema 跨两个顶层 tool calls 表达 sibling 必须存在。
required_next_call 当前仍是“参数声明 + prompt/description 引导 + response preflight 拒绝”，不是纯 schema
强约束。v1.0.4 六次运行有 10/29 次 sibling 遗漏，说明该工具表达问题继续存在。

为保持实验可归因，v1.1.0 第一轮不同时修改这个工具结构。若完整协议仍无法显著提高首次采用率，再把
跨调用表达能力作为独立工具方案处理；不能把 provider 首次生成问题交给 Runtime 自动补调用。

另一个潜在缺口是当前 Map 支持增加节点、增加/删除边，但没有明确的“退役尚未执行且已被证明不再需要的
Work 节点”动作。协议可以要求及时修正分解，却不能声称能完成 schema 不支持的恢复。该能力是否需要补充，
必须独立设计和验证，不能夹带在提示词版本中。

## 7. 验收设计

第一轮保持单变量：只替换 Working Protocol 文本、版本和哈希。

| 指标 | v1.0.4 基线 | v1.1.0 诊断目标 |
|---|---:|---:|
| TaskSpace 协议完整注入 | 76/76 | 100% |
| 公开/隐藏验证通过 | 6/6 | 6/6 |
| Map 完整闭合 | 6/6 | 6/6 |
| 初始化前普通工具 | 2/6 runs | 0/6 |
| required_next_call 遗漏 | 10/29 | 0/全部声明 |
| 含 sibling 遗漏的运行 | 6/6 | 0/6 |
| echo/no-op filler | 多次 | 0 |
| 无变更后的重复 passing pytest | 多次 | 0 |
| finish_not_ready | 2 次明确出现 | 0 |
| 实际动作与绑定节点时序一致 | 未验收 | 6/6 trace audit 通过 |
| Requests/Input/Wall | 已记录 | 不得在无行为收益时退化 |

比较臂为：

1. Standard；
2. 冻结的 v1.0.4 baseline；
3. v1.1.0 candidate。

simple、complex 各 3 次；运行顺序轮换并统一 Docker、模型、工具、Runtime 和样本。除总和、均值、中位数、
缓存和成本外，必须逐 request 审计节点绑定与实际动作顺序。三次只是工程诊断，不用于宣称统计稳定性。

## 8. 实施门禁

1. 用户先审阅并确认协议心智模型和完整正文。
2. 实现时更新 source、版本、规则哈希、contract history 和 contract tests，不覆盖 v1.0.4 结果。
3. 先跑协议结构、exactly-once、wire identity 和 Standard 零注入测试。
4. 再执行三臂 Docker sample，并生成统一 performance observation。
5. 若结果不满足行为收益，保留 v1.0.4 active，v1.1.0 标记 evaluated-not-accepted；不得用 Runtime 语义 gate
   掩盖协议失败。
