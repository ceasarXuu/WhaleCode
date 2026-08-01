# build-R5 TaskSpace 简洁模型收敛宪章

> R5 不继续沿 R4 的 tools 链路专项往下补复杂策略。R5 的主目标是把 TaskSpace 重新收敛为
> `standard` 自然上下文的图化/状态机化再组织：runtime 维护 map 和硬规则底线，Agent 保持
> 100% 语义决策权，context/projection 只做忠实传递和透明裁剪。

## 0.1 元数据

```text
Created: 2026-07-09
Updated: 2026-07-09
Version: v0.0.5 build-R5
Status: Draft
Owner / Responsible: WhaleCode core runtime
Related Systems: TaskSpace runtime, action_map runtime, context projection,
  taskspace_control, tool feedback recording, benchmark harness, CoE
Related Links:
  docs/v0.0.5/build-R4/00-r4-tools-chain-special-project.md
  docs/v0.0.5/build-R4/10-r4-request-convergence-engineering-plan.md
Risk Level: High
Plan Type: Full
Change Type: Architecture refactor / simplification
```

## 0.2 背景

R4 期间多个失败现场显示：TaskSpace 的主要风险已经不只是某个 tool result 丢失，而是当前
TaskSpace 结构逐步演化成了一个半语义工作流 runtime。它维护了 `problem_ledger`、
`cognitive_state`、`facts`、`decisions`、`fact_sources`、`output_contracts`、`result_validity`、
`adoption`、projection heuristics 和 next-action hints。

这些结构有一部分用于可观测性和恢复，但也把自然上下文里的局部细节提升成持久语义状态，
并让 runtime 开始影响 Agent 的判断路径。H-203/H-204 中 `/app` 路径细节被持久放大，是这个
方向的典型风险。

## 0.3 R5 总目标

R5 要把 TaskSpace 收敛到一个更小、更清晰的模型：

```text
standard 自然顺序上下文
  -> TaskSpace 按 root task 和 node 图进行再组织
  -> 每个 node 归档其工具调用、工具反馈、Agent 摘要和必要 refs
  -> 状态机维护节点生命周期、归属、依赖和硬约束
  -> Agent 自己理解语义、选择策略、推进状态
```

TaskSpace 的定位：

1. 是 Agent 必须使用、不可绕过的内建工具和规则化账本。
2. 地位不高于普通 tools，只是负责保存 map、归属上下文、执行状态机底线。
3. 不替 Agent 判断任务事实、业务结论、下一步策略、是否已经理解充分。
4. 不通过 projection 注入思考提示、策略建议、纠错指令或语义再解释。
5. 只在必要时对上下文做透明裁剪、渐进暴露和可追溯引用。

## 0.4 目标模型

### 0.4.1 最小持久结构

```text
TaskSpaceMap
  root_task
    id
    title
    objective
    status
  nodes[]
    id
    parent_id / dependency_ids
    kind
    objective
    status
    owner
    created_at
    updated_at
  node_events[]
    node_id
    event_id
    event_kind
    source
    success
    raw_ref
    visible_excerpt
    created_at
  edges[]
    from_node_id
    to_node_id
    relation
```

节点允许的少量结构化字段：

| Field | 允许原因 | 不允许承载 |
|---|---|---|
| `kind` | 状态机生命周期和工具硬基线 | 业务语义判断 |
| `objective` | Agent 给节点设定的工作目标 | runtime 改写后的任务理解 |
| `status` | pending/running/blocked/completed 等状态 | “已理解”“已解决”等语义结论 |
| `dependency_ids` | map 拓扑和执行先后 | runtime 自动规划策略 |
| `owner` | main/subagent 归属 | 语义权威 |
| `node_events` | 忠实归档工具和反馈 | runtime 决策摘要 |

### 0.4.2 上下文原则

TaskSpace 上下文应该是：

```text
map skeleton
+ current node local context
+ recent node events / tool feedback
+ bounded excerpts
+ refs for omitted raw bodies
+ hard state-machine errors when violated
```

它不应该是：

```text
facts/decisions/fact_sources/output_contracts 的 runtime 语义账本
next action recommendation engine
agent strategy prompt
validator closeout semantic controller
coverage inference engine
path correction controller
```

## 0.5 R5 设计原则

| Principle | Rule |
|---|---|
| 语义透传 | tool result、用户要求、Agent 摘要必须尽量原样保留；无法完整保留时给出裁剪范围和 ref |
| Agent 主权 | Agent 决定语义、策略、状态推进；runtime 不替 Agent “更聪明” |
| 状态机底线 | runtime 只维护 map、节点生命周期、归属、依赖、工具配对和协议硬规则 |
| 简洁优先 | 新结构必须证明比旧结构少、更透明、更可测，不能为了修单点失败继续堆语义层 |
| 渐进拆除 | 每个 phase 直接切断或删除一类过度设计，并用 paired/targeted 样本证明没有负收益 |
| 无兼容债 | 本产品为实验性产品，不保留历史 TaskSpace 数据兼容；风险控制依赖小提交、测试和 git 回退，不依赖 runtime 兼容层 |
| 日志驱动 | 记录 map 状态、node event 归属、projection 裁剪、硬规则拒绝，不记录主观策略判断 |

## 0.6 明确非目标

```text
不在 R5 继续修补 R4 的复杂 projection heuristics。
不让 runtime 自动纠正 Agent 的错误工具选择。
不新增更强 prompt 让 Agent “应该如何思考”。
不把 benchmark 任务文本中的局部细节提升成无 provenance 的 canonical truth。
不在当前运行中丢弃 Agent 需要的工具反馈和 raw refs。
不为了降低成本静默丢弃工具反馈。
不以 sample pass 证明架构正确，必须同时看负收益和语义传递质量。
```

## 0.7 R5 总验收

R5 完成时必须满足：

1. model-visible TaskSpace 上下文可解释为 `standard` 上下文的图化/状态机化再组织。
2. `problem_ledger` / `cognitive_state` / `facts` / `decisions` / `fact_sources` / `output_contracts`
   不再作为 active runtime 语义控制路径。
3. projection 不包含策略性 next-action hints，不把局部文本细节提升为强语义事实。
4. 每个工具反馈都能追溯到 node-local event、raw ref、visible excerpt。
5. 状态机拒绝只发生在硬规则底线：无 task/node 归属、状态非法、工具协议/配对非法、权限/沙箱/安全基线非法。
6. R4 已有正向样本不发生明确回退；已知失败样本的失败形态不能变成反馈丢失或上下文扭曲。
7. 成本指标不因“少结构”显著恶化：request count、tool count、input tokens、wall time 至少不出现无解释放大。
8. R5 closeout 必须列出仍保留的复杂结构、保留原因、后续删除条件；默认不为历史数据兼容保留复杂结构。
