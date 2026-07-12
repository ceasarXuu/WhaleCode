# R5-J7.6 TaskSpace Control 契约忠实性修复计划

## 1. 元数据

- Created: 2026-07-13
- Updated: 2026-07-13
- Version: v0.0.5 build-R5 J7.6
- Status: In Progress
- Owner / Responsible: WhaleCode core runtime
- Related Systems: `taskspace_control` ToolSpec、typed args、Action Map handler、sequence aggregate、benchmark observer
- Related Links: `18-r5-single-patch-carrier-contract-plan.md`、`38-r5-j7-phase5-docker-benefit-result.md`、
  `coe/2026-07-10-22-56-r5-request-amplification.md`
- Risk Level: High
- Plan Type: Standard bug fix / contract refactor

## 2. 问题定义与证据门禁

J7.5 order 样本中，Action Map 初始化真实成功，六个 Agent 声明的节点也已创建，但成功反馈只返回
`task_id/map_id`。后续两节点 finish 真实提交后，成功反馈又只返回 `result_id/binding_status`，没有返回
`finished_node_id/next_node_id/current_node_id`。Agent 下一次请求因此再次声明已完成节点，收到正确的
`lifecycle_target_already_completed` 硬拒绝；随后用 draft node 重建已存在阶段，最终留下四个 open node。

同时，active tool schema 使用无判别字段的 `anyOf` 平铺 existing/create next 形状；terminal 又要求把终态节点
包进 `terminal_finish`，而 `preceding_finishes` 仍只能是非终态 finish。J7.5 trace 中依次出现：把终态节点放进
`preceding_finishes`、把 `__end__` 当作节点、空 terminal finish 被 open-node 硬规则拒绝。错误反馈均正确；缺陷在
成功事实被过度裁剪和输入形状不唯一，不在状态机底线，也不构成 Runtime 自动纠正 Agent 的理由。

根因修复证据门禁已满足：

1. 初始化 outcome 本来就含 `node_ids/current_node_id`，仅在 formatter 中被删掉。
2. finish session API 本来返回实际 finished node ID 与 outcome next node ID，handler 主动忽略了前者。
3. 原始 control call/output 均进入 canonical history，失败语义没有丢失；缺的是成功提交后的身份事实。
4. order 中五个失败 control-bearing requests 消耗 29.3 秒和 70,868 gross input，且失败路径紧随弱成功回执。

## 3. 目标与非目标

### 3.1 目标

1. 每个成功状态变更忠实返回 Runtime 已提交的机械身份与状态，不要求 Agent回放整段 journal 来推断。
2. existing next、created next 和 terminal target 在 tool schema 中各有唯一形状，schema 与 Rust parser 一致。
3. 不兼容旧 finish 形状，不引入 adapter、fallback 或双解析。
4. 保留所有 Action Map 硬规则；Runtime 不自动 finish、bind、create、dedupe 或选择下一节点。
5. trace 能直接审计成功身份覆盖率、重复 finish 是否发生、Map 是否闭合和成本是否出现新回退。

### 3.2 非目标

- 不修改 Agent reasoning，不解析思考文本，不增加任务语义提示。
- 不把失败后的建议、下一步或“正确动作”写入 tool output。
- 不动态重写 projection，不引入 semantic reducer 或状态恢复策略。
- 不放宽 dependency、ready、current、in-flight、open-node terminal 等硬规则。
- 不以单次读取、pytest 或节点数量差异新增行为限制。
- 不在本阶段执行 R5-K、G3 或 H。

## 4. 目标契约

### 4.1 非终态 finish 输入

旧形状删除：

```json
{"node_id":"inspect","next_node_id":"test"}
```

新形状使用嵌套判别联合：

```json
{
  "node_id": "inspect",
  "next": {"kind": "existing", "node_id": "test"}
}
```

```json
{
  "node_id": "inspect",
  "next": {
    "kind": "create",
    "node_kind": "smoke_test",
    "goal": "Run focused tests",
    "dependency_node_ids": ["inspect"]
  }
}
```

`node_id` 仍可省略，表示 Agent 明确选择当前节点；`next` 不可省略。`kind` 是唯一判别字段，两个 variant
均 `additionalProperties=false`。create 的 ID 仍由现有 Action Map substrate 机械生成，但必须在成功反馈返回。

### 4.2 terminal 输入

删除无信息增益的 `terminal_finish` wrapper：

```json
{
  "action": "finish_then_end",
  "preceding_finishes": [],
  "terminal_node_id": "final",
  "final_candidate": "Exact Agent-authored answer"
}
```

`terminal_node_id` 可省略，表示 Agent 声明当前节点为终态目标。`preceding_finishes` 继续只接受带 next 的非终态
transition；这不是语义判断，而是 Action Map 的生命周期类型约束。

### 4.3 成功输出

所有初始化和 finish 成功输出升级为单一 `TaskSpaceControlResultV2`，不保留 V1 success 兼容：

```json
{
  "schema_version": "TaskSpaceControlResultV2",
  "status": "committed",
  "success": true,
  "steps": [{
    "kind": "map_initialized",
    "task_id": "task-1",
    "map_id": "map-1",
    "created_node_ids": ["inspect", "test"],
    "current_node_id": "inspect"
  }]
}
```

```json
{
  "schema_version": "TaskSpaceControlResultV2",
  "status": "committed",
  "success": true,
  "steps": [{
    "kind": "state_transition",
    "index": 0,
    "finished_node_id": "inspect",
    "result_id": "result-1",
    "next": {"kind": "existing", "node_id": "test"},
    "current_node_id": "test"
  }]
}
```

created next 只把 `next.kind` 改为 `created` 并返回 Runtime 生成的 `node_id`。terminal step 返回
`finished_node_id/result_id/map_status/task_status/current_node_id:null`。字段仅陈述已提交事实，不包含总结、评价、
建议、重试方向或恢复动作。

失败输出同步使用 V2 envelope，但完整保留原始 `error.class/code/message` 和已完成 steps；不能因为统一 schema
而改写、压缩或删除失败正文。

## 5. 分阶段实施

### J7.6-A：契约冻结与 fixture

- Entry：J7.5 trace 与代码路径证据齐全。
- Tasks：先写 schema/parser/formatter 失败测试；fixture 覆盖 existing、create、terminal、旧形状拒绝。
- Validation：测试必须先证明旧输出缺少 IDs、旧 schema 允许近似形状。
- Exit：目标 JSON 与 Rust 类型一一对应，无 Unknown 字段。
- Fallback：不进入生产 handler 修改。

### J7.6-B：ToolSpec 与 typed parser 原子替换

- Entry：A 通过。
- Tasks：引入 tagged `next` enum；删除旧平铺字段与 `terminal_finish`；更新 tool description 的机械说明。
- Validation：ToolSpec snapshot、serde parser、unknown-field、cross-variant fixture。
- Exit：每个 JSON 只能匹配一个 variant；旧形状全部拒绝。
- Fallback：按本 phase commit 整体回退，不保留双 schema。

### J7.6-C：忠实 V2 成功/失败反馈

- Entry：B 通过。
- Tasks：使用 session 已返回的 finished ID、next ID 和 initialize outcome；统一 step kind/index；保留 nested
  ordinary event refs 的 aggregate 逻辑。
- Validation：handler 单测与 sequence aggregate 测试证明 IDs 未被 outer aggregate 删除，failure 原文 hash 不变。
- Exit：初始化、existing/create finish、terminal 的必要机械身份覆盖率 100%。
- Fallback：B/C 必须同组回退，禁止新输入配旧输出。

### J7.6-D：日志与工程回归

- Entry：C 通过。
- Tasks：增加不含任务正文的 `taskspace.control_state_committed` tracing event，记录 call/action/step count 与
  identity coverage；observer fixture 统计 V2 success、identity-missing 和 committed 后重复 finish。
- Validation：tools/core/action-map/sequence focused suites、`cargo check`、PowerShell observer tests。
- Exit：日志能区分 parse failure、state reject、success identity missing 和 Agent repeat；正文/secret 不入日志。
- Fallback：observer 可单独回退；生产契约不得依赖 observer 才正确。

### J7.6-E：Docker sample 与 trace 门禁

- Entry：locked binary attestation 与 D 全部通过。
- Samples：`multi-file-order-pipeline` 为主复现；`subscription-billing-repair` 为交叉回归。各跑一次
  Standard/R5，同容器、同模型、同 side contract；R4 沿用 unavailable 结论，不补造。
- Trace：逐 provider request 记录 control action、输入 variant、成功/失败、returned IDs、ordinary tools、patch、
  pytest、Map current/open/result、input/cached/uncached/output tokens、wall time。
- Correctness gate：两侧外部验证通过；R5 protocol/state failure=0；Map task/map completed、open nodes=0；
  committed identity coverage=100%；旧形状调用=0。
- Benefit gate：order 不再出现 committed 后重复 finish；不新增 control-only 恢复 request；请求/token/cache/wall
  完整分账；收益不来自少读、少测、自动状态变更或 Map 坍缩。
- Exit：所有 correctness gate 通过才允许把 J7.5 重新标记 complete；否则记录新问题并暂停。

## 6. 实现完整性矩阵

| Plan Item | Production Path | Integration Entry | Test Evidence | Runtime Evidence | Status |
|---|---|---|---|---|---|
| tagged next schema | `tools/src/taskspace_tool.rs` | active `taskspace_control` ToolSpec | schema snapshot | provider call args | planned |
| typed parser | `taskspace_control_args.rs` | ToolRouter parse boundary | serde fixtures | protocol output | planned |
| V2 identity feedback | `taskspace_control.rs` | handler + sequence aggregate | handler/sequence tests | canonical output | planned |
| terminal simplification | schema + args + handler | `finish_then_end` | terminal fixture | terminal trace | planned |
| identity observer | benchmark instrumentation | Docker artifact | extractor tests | report JSON/table | planned |
| benefit proof | unified Docker runner | Standard/R5 pair | validators | rollout/map/cost/cache | planned |

只有 production path、测试和 runtime evidence 全部落地才可标记 `landed`。

## 7. 变更链日志

| Change Link | Success Signal | Failure Signal | Reason Field | Correlation | Privacy |
|---|---|---|---|---|---|
| schema parse | parsed tagged variant | V2 protocol failure | `error.code` | `call_id` | 不记录 goal/final 正文 |
| map initialize | task/map/nodes/current committed | state reject | `error.code` | `call_id/task_id/map_id` | 仅 ID 与计数 |
| nonterminal finish | finished/result/next/current committed | state reject | `error.code` | `call_id/node_id/result_id` | 仅 ID 与 variant |
| terminal finish | task/map completed | open/in-flight reject | `error.code` | `call_id/node_id/result_id` | 不记录 final candidate |
| observer join | identity coverage=100% | missing/repeat | metric name | run/request/call | 聚合计数 |

## 8. 风险、替代方案与回退

| Risk | Impact | Mitigation | Fallback |
|---|---|---|---|
| schema 体积增加 | input token 增长 | 删除旧平铺字段和 wrapper；记录 tools bytes/hash | 重新精简描述，不恢复歧义结构 |
| Agent 仍生成旧形状 | 一次 protocol failure | 无兼容；确认 provider 实际看到新 schema | 记录模型可用性问题并暂停 |
| 更多 IDs 造成上下文重复 | 小幅 token 增长 | 只返回提交事实，不复制 goal/summary/tool result | 保留 IDs，先删 envelope 冗余 |
| Agent 仍重复 finish | correctness gate 不过 | 先检查成功 output 是否进入下一请求及 schema 是否生效 | 记录新的上下文链路证据，不加 Runtime 纠错 |
| terminal 仍不闭合 | open node | trace 对照 Agent 声明与 Map hard state | 保留硬拒绝，不自动闭合 |

明确拒绝：自动 finish、自动 bind、自动选择 ready node、语义 dedupe、错误后建议、兼容旧字段、projection 注入。

## 9. 参考与决策依据

- JSON Schema 的 `oneOf` 表示恰好匹配一个分支，本计划进一步使用 `kind` 判别字段降低 provider 生成歧义：
  <https://json-schema.org/understanding-json-schema/reference/combining>
- DeepSeek function calling 的 strict 模式依赖受支持的 JSON Schema 子集，因此生产实现继续使用已验证的普通 object
  schema，不假设 strict 可消除兄弟 tool-call 问题：<https://api-docs.deepseek.com/guides/function_calling>
- MCP tool result/output schema 强调机器可验证的结构化输出；本计划把该原则用于内部 control 回执，但不引入新的
  MCP 架构层：<https://modelcontextprotocol.io/specification/2025-06-18/server/tools>
- Serde internally tagged enum 为 Rust parser 提供与 `kind` 一致的唯一分支：<https://serde.rs/enum-representations.html>

## 10. 暂停规则

J7.6-E 完成后必须暂停并汇报，不进入 R5-K、G3、H，也不自动执行对抗性审查。若 sample 暴露新问题，先记录
症状、因果证据与影响；只有用户确认后再进入后续修复。
