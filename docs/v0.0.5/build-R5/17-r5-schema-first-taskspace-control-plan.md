# R5-J6 Schema-first TaskSpace Control 设计与实施计划

- Created: 2026-07-12
- Updated: 2026-07-12
- Version: 1.0
- Status: Implemented; structural benefit verified, total-cost parity not reached
- Owner / Responsible: WhaleCode TaskSpace
- Related Systems: `taskspace_control` tool schema、ToolRouter、native tool scheduler、TaskSpace transition notice、benchmark observer
- Related Links: `14-r5-native-control-cadence-plan.md`、`01-r5-phased-simplification-plan.md`
- Risk Level: High
- Plan Type: Full

## 1. 背景与问题定义

R5-J5 已证明 provider、transport 和 native scheduler 能在一个 response 中承载多个 control 或
control + ordinary tool call，但真实样本仍稳定产生单独的非终态 `finish_node`：

```text
Agent 已经决定 finish 后的下一动作
  -> 只生成一个 finish_node tool call
  -> runtime 成功提交 finish
  -> 再发一次 provider request
  -> Agent 才生成已经决定的下一动作
```

当前契约存在三层不一致：

| 层 | 当前表达 | 实际约束力 | 问题 |
|---|---|---:|---|
| Tool schema | `action=finish_node`，后续动作不在参数中 | 强，但只约束单次 control 参数 | schema 天然允许单独 finish |
| Tool description / transition notice | “Prefer chaining”，同时声明 standalone valid | 软 | 同时鼓励和允许相反行为 |
| Runtime | 成功后记录 cadence inefficiency；旧方案曾后置拒绝 | 只能在生成后处理 | 不能改变已生成形态；拒绝会制造重试和无意义 no-op |

根因不是 Agent 缺少下一步语义，也不是 native multi-tool 能力缺失，而是
`taskspace_control` 的机器可读契约没有把“非终态 finish 必须携带继续动作”表达成一个完整调用。
顶层 function schema 只能约束自身参数，无法约束 provider response 中是否还存在兄弟 tool call。

## 2. 设计原则

1. **Schema-first**：期望行为必须首先成为 `taskspace_control` 参数的合法形状，而不是提示词建议。
2. **单一工具**：不新增公开 action-frame tool，不新增独立协议层；只演进现有 `taskspace_control`。
3. **Agent 完整声明**：所有状态迁移、普通工具及最终答复都由 Agent 在参数中明确给出。
4. **Runtime 机械执行**：runtime 只校验 schema、状态机硬规则、权限和资源边界，并按声明顺序执行。
5. **忠实反馈**：每个内部工具的原始成功、失败、正文或 output ref 都进入结果，不摘要、不改写、不推断。
6. **无后置惩罚**：删除对 response 兄弟调用形态的 cadence reject/violation 逻辑；非法形态在 schema 中不可表达。
7. **不做兼容**：实验性产品无保留旧 `finish_node` 调用形态的需求，迁移时直接删除旧形态及其分支。
8. **不粗化 Map**：请求收益不得来自减少节点、自动合并节点或 runtime 自动推进状态。

## 3. 目标与非目标

### 3.1 目标

1. 非终态 finish 只能表示为“finish 后继续”，并在同一个 tool call 中携带至少一个后续动作。
2. 终态 finish 只能表示为“finish 后结束”，并携带非空 Agent final candidate。
3. 空 Map 初始化可以在同一个 tool call 中携带初始化后立即可执行的普通动作。
4. 一个调用可以按 Agent 声明连续 finish 多个已满足硬状态的节点。
5. 内部 ordinary actions 复用现有 ToolRouter、handler、权限、沙箱、取消和日志链。
6. tool schema、tool description、TaskSpace transition notice 和 runtime 行为表达同一份机械契约。

### 3.2 非目标

1. 不解析或约束 `reasoning_content`、assistant 自然语言或思考过程。
2. 不由 runtime 选择 next node、生成 ordinary action、补写参数、自动重试或自动恢复。
3. 不把所有原生 tools 收编为新的通用 action runtime。
4. 不改变普通 `actions list` 的原生调用方式；没有 TaskSpace 状态迁移时继续使用顶层 native tools。
5. 不保证预先声明的后续动作能够消费同批前一步的未知输出；存在结果依赖时自然进入下一次 provider request。
6. 不把单次样本的随机 request 下降直接宣布为稳定性能收益。

## 4. 外部依据与本地事实

### 4.1 外部依据

1. [DeepSeek Function Calling](https://api-docs.deepseek.com/guides/function_calling/)：strict mode 会校验
   function JSON schema，但属于 beta endpoint，且只接受 provider 支持的 schema 子集。因此必须先对生产目标
   endpoint 和模型执行能力探针，不能直接假设 strict/`anyOf`/`minItems` 可用。
2. [OpenAI API Function Tool Reference](https://platform.openai.com/docs/api-reference/assistants)：function
   参数由 JSON Schema 描述；`strict=true` 只支持 JSON Schema 子集。Whale 的 Codex-derived tool model 也应先验证
   provider 子集再冻结 schema。
3. [JSON Schema Combining](https://json-schema.org/understanding-json-schema/reference/combining)：`anyOf`
   可表达互斥的调用形态；各变体应使用不同 action 枚举和 `additionalProperties=false`，避免一个实例同时匹配多个变体。
4. [JSON Schema Array Validation](https://json-schema.org/draft/2020-12/json-schema-validation#name-validation-keywords-for-arrays)：
   `minItems` 才能机器化保证后续动作列表非空；省略时等价于允许0项。

### 4.2 已确认的本地事实

| Fact | Evidence | Design Impact |
|---|---|---|
| 当前 `taskspace_control` 为单一宽对象 + `action` enum | `tools/src/taskspace_tool.rs` | required 字段无法按 action 精确约束 |
| 当前 `strict: false` | `create_taskspace_control_tool()` | schema 主要是生成引导和本地解析契约，不能假设 provider 强校验 |
| 当前 `JsonSchema` 支持 `anyOf`，不支持 `minItems` | `tools/src/json_schema.rs` | J6.0 必须先补最小 schema 表达能力并做 wire probe |
| 当前 ToolRouter 已统一解析和分发普通工具 | `core/src/tools/router.rs` | 内部动作必须复用，不复制 tool handler |
| 当前 sequence executor 已有 control barrier、首错停止和 skipped output | `core/src/tools/sequence.rs` | 复用其机械顺序语义，不新建语义调度器 |
| J5 hard reject 产生重试和无意义 echo | J5 Docker/CoE evidence | 禁止 response 生成后的 cadence 惩罚 |

### 4.3 复杂度与依赖

该改动只演进一个公开 tool，但会跨越 schema 生成、provider wire、native tool dispatch、权限和反馈链，属于高风险
核心工作流变更。风险来自工具执行边界而不是业务数据；没有数据迁移和向后兼容要求，可以按独立 commit 整组回退。

| Dependency | Type | Current Status | Blocking Risk | Handling Plan |
|---|---|---|---|---|
| DeepSeek 目标 endpoint 的 schema 子集 | third-party | Unknown | `anyOf/minItems` 被拒绝或未按预期生成 | J6.0 真实 wire probe，未通过则暂停 |
| 本地 `JsonSchema` 的 `minItems` 支持 | system | Complete | 无法机器化表达非空 actions | J6.0 已补关键字并通过 roundtrip/provider probe |
| model-visible ToolSpec 集合 | system | Ready | nested action 暴露隐藏工具或 schema 递归 | J6.1 从同一 request 的可见集合机械派生并排除 control |
| ToolRouter/ToolCallRuntime | system | Ready | composite 路径绕过权限、沙箱或取消 | J6.2 复用生产路径并用负例门禁 |
| Docker benchmark/log observer | environment | Ready | 无法归因 request、cache 和 Map health | J6.4 使用现有统一 runner |

## 5. 最小工具模型

### 5.1 对 Agent 暴露的行为形态

TaskSpace 下只新增三种组合形态；普通 action list 不经过 control tool：

| Agent 行为 | Tool 表达 |
|---|---|
| 初始化 Map 后立即工作 | `taskspace_control(action=initialize_then_actions, ..., actions=[...])` |
| 完成节点后继续工作 | `taskspace_control(action=finish_then_actions, finishes=[...], actions=[...])` |
| 完成节点后结束 | `taskspace_control(action=finish_then_end, finishes=[...], final_candidate=...)` |
| 仅执行普通工具 | 现有顶层 native tool calls |

`create_node`、`bind_node`、`block_node`、`read_output_ref` 继续作为纯机械 map 操作保留。旧
`initialize_map`、`finish_node` 直接删除，不增加兼容 alias、normalizer 或 fallback。

### 5.2 参数草案

以下是行为契约草案，不代表在 J6.0 provider probe 之前冻结具体 JSON Schema 子集：

```json
{
  "action": "finish_then_actions",
  "finishes": [
    {
      "node_id": "node-2",
      "result_summary": "Implemented the fix",
      "next_node_id": "node-3"
    }
  ],
  "actions": [
    {
      "tool_name": "exec_command",
      "payload_type": "function",
      "arguments": {
        "cmd": "cargo test -p codex-core focused_test"
      }
    }
  ]
}
```

```json
{
  "action": "finish_then_end",
  "finishes": [
    {
      "node_id": "node-4",
      "result_summary": "Validation passed"
    }
  ],
  "final_candidate": "Implemented and verified the fix."
}
```

核心约束：

1. 根 schema 使用 `anyOf`，每个变体拥有唯一的单值 action enum、独立 required 字段和
   `additionalProperties=false`。
2. `initialize_then_actions.actions` 与 `finish_then_actions.actions` 必须 `minItems=1`。
3. `finish_then_end.final_candidate` 必须非空；若 provider 子集不支持 `minLength`，由 typed parser 做普通参数
   合法性校验。不得退回 response-level cadence gate。
4. `finishes` 必须 `minItems=1`，按数组顺序串行提交；每一步都显式包含 Agent 生成的 result summary 和 next binding。
5. `actions` 只允许当前 request 已向 Agent 暴露的 ordinary tools；禁止嵌套 `taskspace_control` 和已隐藏的
   `update_plan`，避免递归协议和线性 plan 回流。
6. function/custom/MCP payload 使用现有 ToolPayload 类型区分；J6.0 根据 provider 可接受的最小 schema 子集决定
   使用精确 `anyOf` 还是 `tool_name enum + typed payload`。不得使用无约束自由文本 action。

### 5.3 为什么必须把 actions 放进 tool 参数

单个 function 的 JSON Schema 不能要求同一 provider response 中再出现一个兄弟 tool call。因此以下方案都不能
达到本目标：

```text
finish_node 参数要求 next_node_id        -> 只能保证 next binding，不能保证 next action
description 写 MUST emit another call    -> 仍是提示词
runtime 检查 response 尾部是否有 call    -> 仍是生成后的拒绝
```

只有把后续动作变成同一 `taskspace_control` 参数的 required 字段，schema 才能在 Agent 生成调用时表达完整形态。
这不是新增架构层，而是把现有 tool 从“单个状态动作”收敛为“状态动作及其已决定的直接后续调用”。

## 6. Runtime 机械执行契约

### 6.1 执行顺序

```text
parse and validate outer taskspace_control arguments
  -> validate every nested tool is visible and ordinary
  -> submit initialize/finish steps in Agent-declared order
  -> after every state commit, preflight the next step against latest state
  -> execute nested ordinary actions through existing ToolRouter
  -> collect ordered raw results
  -> on first failure, preserve failure and mark undeployed tail as skipped
  -> for finish_then_end, release exact Agent final_candidate only after all finishes succeed
```

状态提交与 ordinary tools 不组成跨工具回滚事务。若 finish 已成功而后续 ordinary tool 失败，finish 保持已提交，
结果中必须明确逐步 success/failure；runtime 不撤销、不重试、不改选下一动作。

### 6.2 必须复用的现有能力

| Capability | Required Reuse | Forbidden Duplication |
|---|---|---|
| tool lookup/visibility | ToolRouter model-visible registry | handler 内自建工具名白名单 |
| argument parsing | 对应原生 ToolSpec/handler | taskspace_control 重写每个工具 schema parser |
| permissions/approval/sandbox | ToolCallRuntime 原路径 | composite call 绕过审批或沙箱 |
| cancellation | 现有 child cancellation token | 不可取消的内部执行 |
| output refs/truncation | 原工具输出与 OutputReference | 二次摘要或丢弃原文 |
| error/skip semantics | sequence executor first-error stop | 自动 retry、fallback 或成功伪装 |

允许新增的 runtime 代码仅限：把 Agent 已声明的 nested action 机械转换为现有 `ToolCall`，生成稳定的派生 call id，
并把执行结果重新装回 outer tool output。不得增加 action selection、semantic reducer 或 recovery planner。

### 6.3 忠实反馈格式

Outer tool 返回一个结构化结果，按执行顺序保留每一步：

```json
{
  "schema_version": "TaskSpaceControlBatchResultV1",
  "status": "completed_or_partial",
  "steps": [
    {
      "index": 0,
      "kind": "finish",
      "call_id": "outer:0",
      "success": true,
      "output": "原始状态工具结果"
    },
    {
      "index": 1,
      "kind": "ordinary_tool",
      "tool_name": "exec_command",
      "call_id": "outer:1",
      "success": false,
      "output": "原始工具失败输出或 output_ref"
    }
  ]
}
```

`output` 不做语义摘要。若原输出过长，沿用原工具已经生成的 output reference；不由 composite handler 再裁剪一次。
日志只记录 action 类型、tool name、状态、字节数、hash 和关联 id，不复制命令输出、final candidate 或 secret。

## 7. Tool、Prompt 与 Runtime 一致性

| Surface | 应保留内容 | 必须删除内容 |
|---|---|---|
| JSON Schema | 三种组合 action 的 required/互斥/非空结构 | 宽对象中可单独出现的 `finish_node` |
| Tool description | 每种 action 的机械效果、顺序、首错停止、反馈结构 | “Prefer chaining”“standalone remains valid”等软硬矛盾 |
| Transition notice | TaskSpace 已启用、空 Map 必须用 control、普通工具需 binding | cadence 建议、策略提示、重复工具手册 |
| Runtime validation | schema、tool visibility、状态机、权限、资源底线 | response sibling-shape reject、cadence correction、自动补动作 |
| Telemetry | 实际 action count、batch size、failure/skip、request delta | 把低效调用标记成状态错误 |

工具 schema 是唯一完整契约；tool description 解释 schema 的机械含义；transition notice 只声明模式和硬状态；runtime
执行并验证同一契约。三处不得各自发明额外行为规则。

## 8. 分阶段实施

### J6.0：Schema 与 Provider 能力门禁

**Entry:** 本文完成评审，尚未修改生产调用形态。

**Tasks:**

1. 给本地 `JsonSchema` 增加最小 `min_items` 表达和序列化测试，不扩展无关关键字。
2. 构造三个最小 provider probes：判别 `anyOf`、`minItems=1`、nested ordinary action。
3. 分别记录当前生产 endpoint 的 `strict=false` 行为，以及目标部署允许时 beta strict endpoint 的 schema 接受结果。
4. 验证 thinking 模式、stream parser 和 cache wire 不丢失 nested arguments。

**Independent verification:** schema serialization tests + 原始 wire artifact + provider response parser fixture。

**Exit:** 生产目标路径能接受选定 schema；三种 action 均至少成功生成和解析一次；空 actions 负例能被 provider 或本地
typed parser 明确拒绝。任何 schema 子集不支持必须回到本阶段简化 schema，不能进入 runtime 实现后再用提示词补洞。

**Fallback:** 回退本地 schema keyword commit，J6 暂停；J5 当前合法行为保持不变。

**实施结果（2026-07-12）：** J6.0 已通过。共享 `JsonSchema` 已增加可选 `minItems` 并完成 round-trip
测试；probe artifact 为 `target/r5-j6-schema-probe/provider-capability.json`。DeepSeek stable endpoint 在
`strict=false` 下接受 `type: object + anyOf + minItems`，并分别生成了正确的
`initialize_then_actions`、`finish_then_actions`、`finish_then_end`，每个 response 均只有一个
`taskspace_control` call，actions/finishes 非空。beta strict endpoint 虽返回 HTTP 200，但三次 arguments
均退化为空对象，因此本轮不采用 strict beta；生产设计使用 stable schema 作为生成契约，并由本地 typed parser
校验同一参数 schema。该校验只处理当前 tool 参数是否合法，不检查或处罚 response 中的兄弟调用形态。

### J6.1：冻结 TaskSpace Control Schema

**Entry:** J6.0 100% 通过。

**Tasks:**

1. 将 tool schema 改为 discriminated `anyOf` 变体。
2. 新增 `initialize_then_actions`、`finish_then_actions`、`finish_then_end`。
3. 删除 `initialize_map`、`finish_node` 旧形态，不保留兼容解析。
4. 基于当前 model-visible ToolSpecs 生成 nested ordinary action 的合法工具集合。
5. 更新 tool registry/schema hash tests，确认 Standard 仍不暴露 `taskspace_control`。

**Independent verification:** schema snapshot、合法/非法参数表驱动测试、Standard/TaskSpace visibility test。

**Exit:** standalone nonterminal finish 在 model-visible schema 中不可表示；所有组合 action 均能 typed parse；旧形态明确失败。

**Fallback:** 整体回退 J6.1 commit，不同时保留新旧 schema。

### J6.2：复用 Native Router 的机械执行

**Entry:** J6.1 100% 通过。

**Tasks:**

1. 把 nested actions 转换为现有 ToolCall/ToolPayload，并通过 ToolRouter 执行。
2. 保留 latest-state preflight、first-error stop、skipped tail、权限、沙箱、取消和 output refs。
3. 实现 `TaskSpaceControlBatchResultV1` 忠实聚合结果。
4. terminal candidate 仅在所有 finish 成功且 lifecycle hard gate 通过后原样发布。

**Independent verification:** order、state visibility、permission denial、sandbox denial、cancellation、first failure、large output ref、
terminal provenance 集成测试。

**Exit:** 生产 router 路径被真实调用；无 mock/stub 替代；每个内部结果与原生独立调用逐字节或等价结构一致。

**Fallback:** 回退 J6.2 和 J6.1，恢复 J5；不得留下 schema 已承诺但 runtime 未执行的半成品。

### J6.3：删除矛盾引导与后置 cadence 逻辑

**Entry:** J6.2 100% 通过。

**Tasks:**

1. 精简 tool description，只描述 schema 已表达的机械效果。
2. 精简 transition notice，删除 cadence guidance 和 standalone-valid 文案。
3. 删除 `finish_cadence_violation`、trailing finish observation 及旧 cadence rejection 遗留测试。
4. 保留纯观测指标：batch size、nested action count、request count、failure/skip。

**Independent verification:** provider-visible text forbidden scan + session fixtures + telemetry schema tests。

**Exit:** schema、description、notice、runtime 没有相互矛盾；不存在 response 生成后的 cadence 拒绝或纠正。

**Fallback:** 文案与 runtime cleanup 与 J6.1/J6.2 同组回退，禁止回到“新 schema + 旧提示/旧 gate”的混合状态。

### J6.4：正确性与收益验证

**Entry:** J6.0-J6.3 100% 通过，locked binary attestation 通过。

**Samples:**

1. `count-call-stack` focused sample：观察 init + action、finish + patch、finish + test、finish + end。
2. 一个固定复杂 Map sample：观察多个 finish、依赖节点、失败停止及 Map health。

每个 sample 各执行一次 Standard、R4 历史基线和 R5-J6 Docker run；如历史环境不可重放，R4 只引用同口径已存 artifact，
不得伪造实时对比。

**Correctness gate:**

- Standard/R5 最终结果正确；
- R5 Map node/edge/result 数量不低于预设固定拓扑；
- protocol/state control failure、反馈丢失、权限绕过、terminal provenance error 均为0；nested ordinary action failure 独立统计；
- 无旧 `finish_node`、无 `update_plan` 回流。

**Benefit gate:**

- invalid actionless nonterminal finish = 0；
- `finish_then_actions` 均携带并真实执行 nested action；
- terminal extra provider request = 0；
- no-op follow-up = 0；
- 每个 `*_then_actions` 至少真实执行1个 ordinary action；
- 相比 J5 同类 trace，能直接归因于 control-only 边界的 provider request 全部消除；
- cache、token、wall time 分账报告，且收益不来自 Map 坍缩。

**Exit:** correctness 和行为收益同时通过才关闭 J6。单次总 request 因模型额外探索未下降时，可以声明结构性断点消除，
但不得声明总成本 parity；需保留具体新增 request 的 trace 解释。

## 9. Phase Gate Matrix

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Completion Required Before Next Phase | Proceed Decision |
|---|---|---|---|---|---|
| J6.0 | schema tests、provider wire probe、parser fixture | 不依赖 handler 实现 | provider 接受且生成/解析三种形态 | 100% | proceed J6.1 |
| J6.1 | schema snapshot、typed parse、visibility tests | 不依赖 router delegation | standalone finish 不可表示 | 100% | proceed J6.2 |
| J6.2 | router/permission/sandbox/output integration | 不依赖 prompt cleanup | 原生能力复用和忠实结果 | 100% | proceed J6.3 |
| J6.3 | forbidden scan、session/log tests | 不依赖 live sample | 四层契约一致、无后置 gate | 100% | proceed J6.4 |
| J6.4 | Docker paired samples、performance observer | 无后续 phase 补证 | correctness + benefit report | 100% | close or pause |

## 10. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| schema keywords | `minItems` 可进入 wire | `tools/src/json_schema.rs` | ToolSpec serialization | tools 139 pass/1 ignored | stable endpoint probe accepted `anyOf/minItems` | provider probe only | landed |
| discriminated control schema | 三种组合形态，旧 finish 不可表示 | `tools/src/taskspace_tool.rs` | model-visible tools | schema positive/negative tests | action variants and deterministic tool ordering | none | landed |
| typed control parsing | 参数精确映射，无兼容 normalizer | `handlers/taskspace_control_args.rs` | function handler | parser tests 9/9 | protocol reason code | none | landed |
| nested native dispatch | 内部 action 复用原 ToolRouter | `tools/router.rs`、`tools/sequence.rs` | provider function call | scenario integration 7/7 | derived call ids + step states | no mock at exit | landed |
| faithful batch output | 每步原始结果完整可追踪；nested 参数 schema 原样透传 | TaskSpace output type/tool schema | provider history | raw output equality + original parameter schema test | raw nested response and split failure classes | none | landed |
| terminal release | exact Agent final 在硬 gate 后发布 | turn completion | `finish_then_end` | terminal/no-extra-request fixture | candidate source and terminal carrier | none | landed |
| contract cleanup | schema、description、notice、runtime 一致 | taskspace tool/runtime/sequence | first TaskSpace turn | context replacement 24/24 | cadence reject event removed | none | landed |
| behavior benefit | 独立 finish 断点消失且 Map 不坍缩 | Docker benchmark | CLI `--taskspace` | focused + complex samples | latest R5 requests 8/12；terminal extra=0 | none | structural benefit passed; cost parity not reached |

## 11. Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation / Trace Field | Log Level | Consumer |
|---|---|---|---|---|---|---|---|
| outer control parse | received/validated | `taskspace.control_batch_validated` | invalid schema/payload | `reason_code` | `request_id/outer_call_id` | info/warn | runtime/debug |
| finish step | preflighted/committed | `taskspace.control_step_committed` | hard-state rejection | `gate_class/reason` | `outer_call_id/step_id/node_id` | info/warn | TaskSpace owner |
| nested tool lookup | visible/resolved | `taskspace.nested_tool_resolved` | hidden/unknown/recursive | `reason_code` | `outer_call_id/step_id/tool_name` | debug/warn | runtime/security |
| nested tool execution | started/completed | existing native success event | native failure/cancel | existing tool reason | `outer_call_id/derived_call_id` | info/warn | tool owner |
| tail skip | skipped | structured skipped result | skip attribution missing | `prior_failed_step_id` | `outer_call_id/step_id` | warn | runtime/debug |
| batch feedback | assembled/persisted | `taskspace.control_batch_recorded` | output pairing failure | `reason_code` | `outer_call_id/result_hash` | info/error | context audit |
| terminal release | staged/released | existing terminal release event | lifecycle rejection | `gate_reason` | `turn_id/outer_call_id/node_id` | info/warn | lifecycle audit |

## 12. 风险、替代方案与回退

| Risk | Probability | Impact | Mitigation | Fallback |
|---|---:|---:|---|---|
| provider 不支持目标 schema 子集 | Medium | High | J6.0 先做真实 endpoint probe | 暂停 J6，不用提示词或 runtime gate 伪补 |
| nested schema 复制使 tools payload 变大、破坏 cache | Medium | High | 优先 `tool_name enum + typed payload` 最小形态；记录 wire/hash/LCP | 回退 J6.1，重新简化 schema |
| composite 调用绕过权限或沙箱 | Low | Critical | 强制复用 ToolRouter/ToolCallRuntime，权限负例为硬门禁 | 回退 J6.2 |
| 聚合输出丢失单工具语义 | Medium | Critical | 原始输出等价测试、output ref 复用、逐步 call id | 回退 J6.2 |
| 预声明 action 错误消费未知前序结果 | Medium | Medium | 文档明确只承载无结果依赖的已决定动作；首错停止 | Agent 下一 request 重规划，不自动 retry |
| 多 finish 导致 Map 坍缩 | Low | High | 固定拓扑/依赖和 result gate | 阻止收益声明 |
| 新旧形态并存增加技术债务 | Low | High | 直接删除旧 schema、parser、tests、telemetry | 整组 commit 回退，不保留兼容 |

明确拒绝的替代方案：

| Alternative | Decision | Reason |
|---|---|---|
| 只加强系统提示词 | Rejected | schema 仍允许 standalone finish，且提示词与工具契约继续分裂 |
| 后置检查 sibling calls 并拒绝 | Rejected | J5 已证明产生重试、无意义 no-op 和上下文污染 |
| runtime 自动选择/执行 next action | Rejected | 越过 Agent 决策边界 |
| 只强制 `next_node_id` | Rejected | 只能保证 binding，不能消除下一 ordinary action 的 provider request |
| 新增独立 action-frame tool/协议层 | Rejected | 扩散架构，和“聚焦修改现有 tool”目标不符 |

## 13. Open Questions

| Question | Resolution Phase | Blocking Rule |
|---|---|---|
| 目标 DeepSeek endpoint 是否接受 `anyOf + minItems` | J6.0 | 未确认不得冻结 schema |
| nested function/custom/MCP 的最小统一 payload schema 是什么 | J6.0 | 必须覆盖本阶段样本实际工具；不得用无约束文本代替 |
| nested ToolSpec 是否复制完整参数 schema | J6.0 | 以 provider 接受、token/cache 成本和参数正确率共同决定 |
| 单个 outer output 如何保持 MCP/custom 原始结果类型 | J6.2 | 等价反馈测试未通过不得进入 J6.3 |

## 14. Decision Log

| Decision | Status | Reason |
|---|---|---|
| 聚焦演进现有 `taskspace_control` | Accepted | 不增加公开工具和架构层 |
| schema 内承载 finish 后续 actions | Accepted | function schema 只能约束自身参数；这是消除 standalone 合法形态的必要条件 |
| 普通 actions list 保持 native | Accepted | 不把 TaskSpace 变成通用动作 runtime |
| 内部 action 复用 ToolRouter | Accepted | 保持权限、沙箱、反馈和工具实现单一来源 |
| 旧 `finish_node`/`initialize_map` 不兼容 | Accepted | 实验性产品无历史数据迁移价值 |
| provider/schema probe 前不实施 handler | Accepted | 避免实现后再用 prompt/gate 修补 provider 缺口 |

## 15. Plan Quality Checklist

- [x] 目标限定为一个现有 tool 的 schema-first 演进。
- [x] Tool、runtime、transition notice 的单一契约和删除项明确。
- [x] 不解析 reasoning，不由 runtime 做语义决策。
- [x] 不使用后置 cadence 拒绝、自动 retry 或 no-op 满足规则。
- [x] provider/schema 高风险前置验证，失败时暂停而非 fallback。
- [x] 每个 phase 有独立证据、退出门禁和整组回退路径。
- [x] 权限、沙箱、取消、失败停止、反馈完整性和 terminal provenance 有测试门禁。
- [x] correctness、Map health、request、cache、token 和 wall time 分账验收。
- [x] Standard/R4/R5 Docker 样本规则已登记。

## 16. 实施与验收结果

### 16.1 工程结果

1. `taskspace_control` 仅保留 `initialize_then_actions`、`finish_then_actions`、`finish_then_end` 三种组合生命周期形态及少量机械辅助动作；旧 `initialize_map/finish_node` 不兼容。
2. nested action 通过现有 ToolRouter/ToolCallRuntime 执行，权限、沙箱、取消、事件归属和原始输出沿用同一路径；首错后剩余动作明确标记 skipped。
3. Agent 初始化时直接声明稳定 `node_id`，依赖、current binding 和后续 finish 全程复用同一标识，不再维护 `node_key -> runtime id` 双轨映射。
4. nested function action 直接嵌入原工具 `parameters` schema，不摘要、不改写；复杂样本发现并修复了 `send_message.target` 丢失问题。
5. observer 分开统计 provider outer calls、nested actions、Runtime tools，以及 protocol/state/nested-action 三类失败。

关键 commits：`b7052ab`、`59173c7`、`1f9cb1a`、`2ed47b7`、`81d2702`。

### 16.2 测试与构建

| Gate | Result |
|---|---|
| `codex-tools` | 139 passed, 1 ignored |
| TaskSpace handler | 9 passed |
| ActionMap runtime | 11 passed |
| native sequence | 6 passed |
| ActionMap scenarios | 7 passed |
| active context replacement | 24 passed |
| multi-agent ActionMap | 11 passed |
| benchmark cost/observer/harness self-tests | passed |
| locked Whale build + attestation | passed |
| full `codex-core --lib --test-threads=1` | 1782 passed, 2 unrelated file-watcher baseline failures, 3 ignored |

### 16.3 Docker sample

| Sample / Mode | Result | Requests | Runtime tools | Carriers | Nested actions | Protocol / state / nested failures | Wall | Request 2+ cache |
|---|---|---:|---:|---:|---:|---|---:|---:|
| `count-call-stack` Standard | solved | 6 | 11 | 0 | 0 | 0 / 0 / 0 | 16.64s | 93.69% |
| `count-call-stack` R5 latest | solved | 8 | 14 | 3 | 3 | 0 / 0 / 0 | 21.62s | 90.11% |
| `multi-file-order-pipeline` Standard | solved | 10 | 18 | 0 | 0 | 0 / 0 / 0 | 44.88s | 95.19% |
| `multi-file-order-pipeline` R5 latest | solved | 12 | 16 | 5 | 9 | 0 / 0 / 1 | 77.62s | 91.85% |
| `count-call-stack` R4 historical | solved | N/A | 11 | N/A | N/A | N/A | 154.53s | N/A |

Evidence：

- focused paired Standard：`target/j6-contract-b/count-call-stack/count-call-stack/20260712-041303-257`。
- focused final R5：`target/j6-contract-c/count-call-stack/count-call-stack/20260712-041525-466`。
- complex paired Standard：`target/j6-complex-a/order-pipeline/multi-file-order-pipeline/20260712-041646-435`。
- complex final R5：`target/j6-complex-b/order-pipeline/multi-file-order-pipeline/20260712-042255-022`。
- R4 historical：`target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136`；复杂样本无同口径 R4 artifact，不补造数据。

### 16.4 结论边界

- **已通过**：旧独立 finish 合法形态消失；初始化/finish/terminal carrier 均真实出现；终态额外 provider request 为0；最终样本 protocol/state failure 为0；原始 nested 反馈和参数能力语义均完整保留。
- **结构收益成立**：focused control 从历史 J5 的5个独立生命周期调用收敛到3个 carrier；最新 R5 与 Standard 的请求差在两个样本中均为2。
- **总成本 parity 未成立**：R5 wall/input/output 仍高于 Standard，复杂样本还有1次普通 patch 失败；不得把当前结果描述为整体性能优于 Standard。
- **Map 未坍缩**：focused/complex 分别保留4/5个节点并全部完成；Agent 未声明依赖边，因此 edge=0 只作为结构观察，不由 runtime 自动补边或推断依赖。
