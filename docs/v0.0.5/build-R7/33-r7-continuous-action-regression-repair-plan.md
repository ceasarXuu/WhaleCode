# R7 连续动作合同回归修复计划

- Created: 2026-07-21
- Version: 1.5
- Status: `selected_not_implemented`
- Phase: FLA-3.5，阻塞 FLA-4 及后续阶段
- Scope: TaskSpace 非终态生命周期交接、真实动作 Tool schema、执行顺序与事实反馈
- Risk Level: Critical
- Compatibility: 不保留 `required_next_call` 兼容路径；候选通过后一次性替换
- Production baseline: 当前 `required_next_call + top-level sibling + sequence preflight`

## 1. 决策与目标

连续动作不是提示词偏好或性能实验，而是 TaskSpace 的产品合同：

1. 初始化 Map、首次绑定或完成当前 Work 并继续时，Agent 必须在同一个结构化工具调用中同时提交状态交接和
   至少一个真实后续动作。
2. 除最终 `complete_then_end` / `finish_end` 外，Agent 不能表达“只完成状态交接、没有后续动作”的调用形态。
3. Runtime 只校验并执行 Agent 明确提交的结构，不推断、补齐、改写或选择后续动作。
4. 后续动作仍走原有 Tool router、权限、sandbox、approval、hook、MCP 和原始反馈链。
5. Patch 正文保持原生 `apply_patch` 输入形态，不重新嵌入大型 control JSON。

本阶段修复 H-003。当前 `required_next_call` 只是对另一个顶层调用的声明，JSON Schema 无法保证 sibling
存在。自然样本反复出现先提交单独 control、被 Runtime 拒绝、再下一轮补动作，说明 D.4 修复了 Patch
保真，却回归了 D.2 已建立的连续动作结构保证。

## 2. 历史证据与根因

| 版本 | 形态 | 已验证收益 | 已知问题 |
|---|---|---|---|
| R5 J6 | `initialize_then_actions` / `finish_then_actions`，`actions minItems=1` | 非终态 standalone 在 schema 中不可表达；稳定样本无 control failure | 中央 carrier 承担普通 Tool 参数和结果，扩展性与反馈保真需要重新审视 |
| R6 / R7 D.2 | `complete_then_continue` 内含 continuation | 两个样本均为 `standalone complete=0`，完成本身不增加 request | 共用大型 nested Patch carrier 出现 JSON 和正文保真问题 |
| R7 D.4 至当前 | 小型 control + 顶层原生 Tool sibling | Patch probe 6/6 JSON 合法且 6/6 正文一致 | sibling 不是同一 schema 的组成部分；自然样本持续遗漏，形成拒绝和额外 request |

根因不是 Agent 缺少更多惩罚，也不是 Runtime 需要理解语义。根因是工具合同把一个不可分割的产品动作拆成
两个独立 provider tool calls，同时试图用第一个调用中的声明字段约束第二个调用。该约束跨越 JSON Schema
边界，天然只能事后检查。

## 3. 冻结边界

### 3.1 必须结构化合并的动作

| 状态动作 | 合法承载方式 | 可否单独调用 |
|---|---|---|
| `initialize_map` | 第一个真实动作携带初始化交接 | 否 |
| `bind_node` | 该节点第一个真实动作携带绑定交接 | 否 |
| `complete_then_continue` | 后继节点第一个真实动作携带完成与绑定交接 | 否 |
| `complete_then_end` | `taskspace_control` 终态调用，含 Agent 最终总结 | 是 |
| `finish_end` | `taskspace_control` 终态调用，含 Agent 最终总结 | 是 |

`mutate_graph`、`block_node`、`unblock_node`、`rework_node`、`expand_nodes`、`read_map` 和
`read_output_ref` 不是“完成后继续”的替代形态，可继续由 `taskspace_control` 独立表达。它们不能绕过
当前 binding/lease 的普通 Tool 硬门禁，也不能被 Runtime 自动转成生命周期交接。

### 3.2 明确不采用

1. 不把连续动作降级成 L1/L2 提示、Tool description 建议或 observer 指标。
2. 不保留 `required_next_call` 后再叠加更强错误、重试提示或 Runtime 自动补调用。
3. 不恢复包含任意 Tool 完整 schema 和大型 Patch 正文的中央 nested action union。
4. 不为每个 Tool 手写一份 `taskspace_*` 复制品，不建立第二套 router、权限或执行器。
5. 不根据 reasoning、命令内容、Patch 内容或任务语义判断下一步。
6. 不以 provider 不稳定为前提；能力结论必须来自定向 probe。

## 4. 目标工程形态

### 4.1 一个 registry 能力合同，一个 decorator

Tool registry 新增唯一机械元数据 `TaskSpaceCarrierCapability`。它只回答“这个 provider-visible Tool 调用是否
进入共享 prepare/execute 链、能否携带 transition 并保真返回结果”，不判断命令、Patch 或 Tool 对任务是否有用。
Runtime 不识别 no-op、不维护按 Tool 名推断的第二份 allowlist，也不评价 Agent 选择的动作质量。

共享 Tool builder 在 TaskSpace profile 下为 carrier-capable Tool 增加可选 `taskspace_transition`。原 Tool 名、
业务参数、handler、router 和权限保持唯一；transition 字段在进入 handler 前机械剥离。`taskspace_control`、
宿主内部控制项或不能进入共享执行链的调用明确标记为 non-carrier。候选激活前，所有 TaskSpace 可见 ordinary
Tool 必须有确定元数据；未知状态、字段碰撞或无法保真装饰都阻止 capability epoch 激活。

“真实动作”的机器含义止于一个实际执行并返回结果的 carrier-capable Tool call。Runtime 不阻止 Agent 用
`exec_command(true)` 或等待类 Tool，因为判断其工作价值会越过语义边界。

### 4.2 Wire 与 ToolSpec 矩阵

CA-0 必须冻结并穷举 `WireApi × ToolSpec × invocation source`，至少覆盖：

| 载体 | 目标形态 | 禁止的降级 |
|---|---|---|
| DeepSeek Chat function Tool | 原参数对象增加 `taskspace_transition` | 动态 tool choice、另一个 sibling 声明 |
| Responses function Tool | 与 Chat 同一逻辑 schema | provider 专属第二套 transition 语义 |
| Responses/custom freeform `apply_patch` | TaskSpace wire 机械投影为同名 function，顶层字段仅 `input + taskspace_transition`；剥离后仍进入原 Patch handler | 把 transition 或 Patch 塞入 freeform 文本；复制 Patch handler |
| Code mode | TaskSpace wire 将同名 freeform `exec` 投影为 function `{source, taskspace_transition}`；剥离后同一 handler 接收 byte-exact source，cell 内 nested calls 继承新 lease | 把 transition 写进 pragma/source；复制 code handler；绕过 turn barrier |
| MCP / dynamic Tool | 在 immutable capability epoch 中由同一 decorator 合并 reserved 字段，调用前剥离 | 把 reserved 字段发给 MCP server；延迟加载后漏装饰 |

示意 Patch 形态：

```json
{
  "taskspace_transition": {
    "action": "complete_then_continue",
    "expected_revision": 3,
    "current_node_id": "inspect",
    "next_node_id": "implement"
  },
  "input": "*** Begin Patch\n...\n*** End Patch"
}
```

Patch 正文仍是顶层 `input`，不得进入 transition。Standard 继续使用原 ToolSpec 和 wire，字节必须不变。
如果任一生产可达 wire 无法在不损失 Patch/Tool 输入的前提下完成投影，本方向直接阻塞，不以“DeepSeek Chat
当前能用”掩盖其他已启用入口。

### 4.3 结构保证

目标 L4 不再向 Agent 暴露 `initialize_map`、`bind_node`、`complete_then_continue` 三个独立
`taskspace_control` 分支；它们只存在于 carrier 的 `taskspace_transition` 中。因此状态交接没有 carrier action
时在 provider-visible schema 中不可表达。

同一 active Work 内第二个及后续动作不重复 transition。Runtime 只检查 canonical state：空 Map 的第一个
ordinary call 必须携带初始化；未 binding 的 Ready Work 第一个 call 必须携带绑定；切换后继必须携带完成交接。
未成功 commit 的调用零 Map 提交、零 ordinary Tool handler 执行；PreToolUse 可能发生的外部效果按第 4.4 节独立
记录，不能伪装成零副作用。终态 action 继续由 control 单独表达。

### 4.4 `pre-hook -> prepare -> commit -> handoff -> execute` 机械协议

不能在现有 handler 外简单前置 Map commit。FLA-3.5 选择一个共享 `PreparedToolCall` 协议，把现有 Tool 的准备
和执行阶段显式拆开，但不复制 handler。现有 PreToolUse hook 可以执行任意用户命令，因此不是无副作用的 prepare；
本阶段不改变其能力或另造 TaskSpace-only hook 分支，而是把它作为明确的 pre-commit 外部步骤并忠实记录：

1. **parse**：解析 carrier，剥离 transition，校验原业务参数。
2. **pre-hook**：按现有语义运行 PreToolUse allow/block hook。它可能产生文件、网络或进程副作用；Runtime 不推断
   这些副作用，也不声称可回滚，而是冻结 `PreHookFact {not_run|allowed|blocked|failed, hook_identity,
   exit/error, external_effects_possible}`。block/failure 或随后 stale/cancel 都必须把该事实返回。
3. **prepare**：完成无普通 Tool handler 副作用的参数解析/规范化，计算权限、初始 sandbox、潜在升级 sandbox、
   网络权限和所有可能的 approval。当前 hook schema 不支持参数改写，本阶段不得虚构该能力；文件上传、MCP
   attachment materialization 或其他有副作用转换都归入 execute。
4. **pre-authorize**：若初始 sandbox 失败后可能升级，必须在 commit 前一次性申请并冻结升级 grant。Agent/user
   拒绝升级不必拒绝初始 sandbox 执行，但 commit 后若初始 sandbox denial，只能按 Tool failure 返回，禁止再次
   弹出 approval。网络和其他可预测审批遵守同一规则。
5. **commit + reserve**：在同一 action-map 临界区重新校验 revision/lease、effective args hash、permission
   snapshot、approval grant scope/expiry 和 cancellation generation；原子提交 transition，并为目标新 lease
   建立该 prepared call 的 reservation。旧 lease 不接收本次调用。
6. **execution handoff**：orchestrator 在调用任何 handler 代码前，以一个原子事件裁决 cancellation generation 并
   接管 prepared call。handoff 成功是唯一 `execution_started` 边界，与 Tool 是否只读、是否已经产生副作用无关。
7. **execute**：原 handler 的执行部分消费 prepared call，执行必要上传和业务副作用；初始 sandbox denial 只有
   预授权 grant 存在时才自动升级，否则直接形成 Tool failure。commit 后不得再次请求 approval。
8. **post**：PostToolUse 只观察不可变的原 Tool outcome；hook failure 作为独立 factual field 返回，不能替换或
   丢弃 Tool output。随后 context mapper 附加 transition fact。

`PreparedToolCall` 至少冻结：effective args/source hash、capability epoch、Tool/source identity、目标 revision/lease、
permission snapshot、initial/escalated sandbox plan、approval grant id/scope/expiry、network grant、`PreHookFact`、
cancellation generation。任一字段在 commit 前失效则回到 rejected-before-commit，不能静默重新 prepare。

取消与失败边界固定为：prepare policy/approval/grant 校验拒绝或 commit 前取消均零 Map 提交、零 ordinary Tool
handler 执行；PreToolUse 已发生的外部效果不伪装成零副作用，始终随 `PreHookFact` 保留。commit 后的实际 sandbox
denial 属于 Tool failure，不再触发审批；commit 后、handoff 前取消记作“transition committed + tool cancelled”，
不回滚。任何无法安全拆出无 ordinary Tool handler 副作用 prepare 的 Tool 不能标记 carrier-capable，TaskSpace
capability epoch 也不能带着未知缺口激活。

### 4.5 Carrier-neutral typed outcome

一个 provider call 只能有一个 call id。`execution_started` 的唯一边界是第 4.4 节的 runtime handoff 事件：它在
任何 handler 工作前发生，并与是否已有业务副作用解耦。内部唯一和类型为：

```text
TaskSpaceCarrierOutcome =
  RejectedBeforeCommit {
    stage: Parse | PreHook | Prepare | PreAuthorize | Commit,
    pre_hook_fact,
    failure: FactualError
             | CommitFailedNoState(TransactionFailureFact)
             | Cancelled(CancellationFact),
    tool: NotDispatched
  }
  | CommittedNotExecuted {
      transition_fact,
      pre_hook_fact,
      tool: CancelledBeforeStart(CancellationFact)
            | StartFailure(FunctionCallErrorFact)
    }
  | Executed {
      transition_fact,
      pre_hook_fact,
      execution: Returned(Opaque<ToolCallOutput>)
                 | Failed(FunctionCallErrorFact)
                 | CancelledAfterStart(CancellationFact),
      post_hook_fact: NotRun(reason) | Succeeded | Failed(factual_error),
      retention_fact: NotRequired | Stored(ref, hash) | Failed(factual_error),
      delivery_fact: Pending | Delivered(wire_hash) | Failed(factual_error)
    }
}
```

`transition_fact` 只含 action、revision、commit/lease。`Returned` 在 PostToolUse 前冻结原 ToolCallOutput；
`Failed` 忠实保存 handler/upload/orchestrator 的结构化 error，不伪造 Tool output；`CancelledAfterStart(CancellationFact)` 明确可能
已有部分副作用。PostToolUse 没有运行必须是 `NotRun`，legacy hook 的替换/丢弃结果只能作为独立 hook fact，
不能覆盖冻结 execution outcome。retained store 或 provider mapper 失败也不能覆盖 Tool outcome；delivery 失败时
该内部事实进入 session event/trace，并由现有 provider retry/error 路径暴露，不能伪造已送达 output。不得把这些
事实塞入 `TaskSpaceControlResultV2` 或压成整体 verdict。

`TransactionFailureFact` 至少包含 expected/observed revision、reservation 创建状态和 transaction error；该分支必须
证明 Map revision 与 reservation 均未改变。`CancellationFact` 至少包含 cancellation generation、观测点
`before_commit|before_handoff|after_handoff`、`handler_started` 与 `partial_effects: none_observed|possible|unknown`。
同一次调用只能命中一个取消或 commit failure variant，不能同时记录“未执行”和“已执行”。
provider mapper 按第 4.2 节载体生成一个合法 output：支持 content items 时增加独立 transition fact item 并原序
保留 Tool items；只有 text output 时使用版本化 factual frame，但其 `tool_output` 子载体必须可逆恢复。

保真门禁比较 frame 解码后的 execution 子载体与冻结 outcome，不比较整个 payload。普通 Tool 逐字段覆盖
text/items、图片 URL/顺序、success、截断引用、exit status 和 error class。任一 carrier 无法无损映射时阻塞。

MCP 必须先冻结独立版本化子载体，不能依赖当前二选一 mapper：

```text
McpToolOutputV1 {
  content: ordered policy-visible raw JSON blocks with source_index,
  structured_content: Absent | Present(JSON including explicit null),
  is_error: Absent | Present(bool),
  meta: Absent | Present(JSON including explicit null),
  sanitization_facts: [...]
}
```

顺序固定为：接收包含未知 block 的原始有序 JSON 并计算私密 hash；执行既有安全/图片支持策略并逐项记录 source
index、preserve/transform/remove fact；冻结 policy-visible `McpToolOutputV1` 和 presence-aware 字段 hash；完整写入
retained output store；再做 context 截断并给 output ref；最后映射 provider wire。安全策略允许的未知 block 必须按
原 JSON 保留；被移除的内容只保留安全 fact/hash，不绕过策略泄露原文。支持 content items 的 wire 以版本化 metadata
item 加有序 text/image/items 表达，并用稳定 index 重建 policy-visible `content`；text-only wire 使用可逆 JSON frame。
`structured_content/is_error/meta` 的 absent、显式 null、false/true 不得合并，也不得因存在图片而丢失。CA-0 冻结
每一步 schema/hash，CA-1 从原始 MCP fixture 跨阶段 round-trip；retention/delivery fault 必须与已冻结 Tool outcome
同时观测。TaskSpace 层保证的是安全策略批准后的完整子载体，不绕过全局安全策略，也不静默摘要。FLA-3.5
拥有 transport；FLA-5 只验证 conformance。

### 4.6 Capability epoch、code mode 与多动作

Tool schema 不要求整个 session 永不变化，而是要求在一个 immutable `capability_epoch` 内稳定。注册、MCP
deferred load、refresh 或 provider capability 变化只能在 provider request 之间创建新 epoch；Map revision、
transition 状态或 projection policy 不得触发 epoch。每个 epoch 记录全量 Tool 元数据、reserved collision、
schema bytes/token、`capability_set_hash` 和 `tools_hash`。Code mode 枚举 nested tools 时必须使用同一 epoch snapshot。

Code mode 的顶层 call 是 carrier；其 nested calls 全部归属新 lease。cell 内 sequence 必须复用 turn barrier，
`Promise.all` 不能让 nested call 越过 carrier prepare/commit，Patch 计数是 turn-wide 而不是 top-level-call-wide。

Agent 可在同一 response 中发出更多独立 Tool calls。carrier call 是机械 barrier：它完成 transition 与第一个 Tool
后，再按现有规则处理后续调用。依赖未知结果的步骤自然进入下一 request，Runtime 不判断语义依赖。一次 response
最多一个 Patch 的合同保持不变。

## 5. 分阶段实施

### CA-0：冻结基线与前置设计合同

- 固定当前 sibling 生产 commit、L1-L5 identity、schema/source/wire hash 和 H-003 trace。
- 从 R5、D.2、D.4、FLA-3 artifact 重算 standalone、拒绝、request 和 Patch exact。
- 冻结第 4.2 节 wire matrix、`TaskSpaceCarrierCapability` 正负矩阵、reserved namespace 和 capability epoch 规则。
- 冻结第 4.4 节完整状态机，覆盖 PreToolUse 外部副作用事实、参数解析/上传、AfterToolUse、初始 sandbox、
  升级/network 预授权、grant 失效、commit 前后取消、handoff/start failure 和 reservation 归属；不得留“由实现
  决定”的分支。
- 冻结 typed outcome 全分支、runtime handoff 边界、McpToolOutputV1 presence/未知 block/安全处理各阶段、retention/
  delivery 失败、provider wire 映射与 hash 公式。
- 生成而非手写 `r7-carrier-entry-closure-v1.json`：从全部生产可达 `ToolSpec`、`ToolPayload`、router/alias、deferred
  MCP、dynamic registry 与 code-mode nested router 枚举 schema decorator、parser、capability epoch、handler 和
  outcome mapper；任何可达入口无且仅无一条完整链即失败。
- 扩展 authority/production-manifest JSON Schema：candidate manifest 是独立 candidate namespace 中的实际实体，
  active authority 在 CA-6 前保持字节不变；状态机使用 `evaluation_candidate -> promotion_pending|rejected ->
  promoted|rejected`，并允许 post-promotion 全量复测失败时 `promoted -> reverted`。实现跨文件 linter，强制 candidate
  id 等于由 active-authority snapshot 与八个具名 artifact 内容 hash 规范化计算的 SHA-256，另设真实
  `candidate_commit`；contract/path/hash/source-authority/active-authority snapshot 双向一致、ID 唯一、
  最多一个 pending/promoted、active pointer 与状态一致。artifact 使用具名且全部必需的角色：L4 schema、transition
  schema、typed outcome、lifecycle oracle v2、capability matrix、rollback manifest、continuous-action evaluation、FLA-8
  evaluation v2；每个文件必须位于该 candidate namespace、存在于 candidate commit 且当前 hash 匹配。linter 通过
  first-parent diff 重放 manifest 状态历史：新记录只能从 evaluation 开始，后续只能按状态表迁移。加入 mismatch、
  duplicate promoted、伪 backlink、伪 commit、direct promoted/reverted、路径逃逸、缺角色/文件和非法 revert 负例。
  补齐 production commit/source/wire hashes。CA-0 同时实现 candidate manifest generator、唯一 transition command 和
  schema/linter tests，未完成不得进入 CA-1。
- 冻结 `r7-phase-ownership-v1.json`，将 carrier transport、L4 schema、L5 conformance、Tool experiments、
  lifecycle/recovery、产品面和正式评估各映射到唯一 owner phase；重复 owner 或无 owner 机器失败。
- 在任何 CA-1 probe 前完整生成并冻结 `continuous-action-evaluation-v1.json`：样本、重复、seed、顺序、全部阈值、
  指标和 SHA-256 均不可留到 CA-2；同时生成 FLA-8 v2 的机械指标迁移候选。FLA-8 held-out 只复制 identity/hash，
  CA-0/CA-1 容器不挂载其内容路径，并用负向 mount assertion 证明不可见。

完成证据：机器基线、状态表、wire/capability 矩阵和评估预注册均可独立检查；当前生产行为不变。

### CA-1：本地可行性与真实 Provider 探针

只建隔离 probe，不接生产：

1. 对全部 wire/ToolSpec/source 组合验证 schema 装饰、freeform Patch 与 code-mode 同名 function 投影、reserved
   字段剥离、Patch input/source byte exact 和 Standard 零变化。
2. 用 fake Tool、transaction fault injector、可控 latch 与真实 hook command 验证 pre-hook 写文件/网络 allow/block/
   failure、prepare/approval denial、commit 原子失败、sandbox denial、commit 前/后取消、只读 handler 立即失败、
   dispatch/start failure、handoff 两侧及 handler 进入后取消、commit 后执行失败和新 lease reservation；pre-hook 与
   ordinary Tool handler side effect 分别计数。每个取消/commit fault 只允许命中一个 variant，并逐字段校验 revision、
   reservation、generation、handler-start 与 partial-effects。
3. 验证 typed outcome 的 start/cancellation facts、execution `Returned/Failed/CancelledAfterStart(CancellationFact)`、post-hook、retention、
   delivery 全状态，以及 text/items、image、McpToolOutputV1、截断和 hook error 的可逆映射。MCP fixture 必须从原始
   wire 覆盖 absent/false/true、显式 null、未知 block、图片 preserve/remove、store 写失败和 provider mapper 失败。
4. 验证 code-mode outer carrier、nested attribution、`Promise.all` barrier 和 turn-wide one-Patch gate。
5. 使用真实 DeepSeek endpoint 对 exec、direct/freeform Patch、MCP 和多 Tool response 每臂至少 6 次 probe。

准入：结构合法、Patch exact、原参数、typed 子载体、lease 归属均为 100%；拒绝/取消时 commit/side-effect 符合
状态表；无 reserved 泄漏、无额外 provider request。任一生产可达组合失败即停止 CA-2。

### CA-2：冻结候选机器合同，不提升 active authority

- 候选 artifact 放入独立 candidate namespace：L4 schema、transition schema、typed outcome、lifecycle oracle v2、
  capability matrix、rollback manifest、CA-0 已冻结的 `continuous-action-evaluation-v1.json` 和 FLA-8 v2。
- FLA-8 v2 只把旧 `combined_control_plus_next_rate` 机械替换为 transition carrier 指标；样本 identity、sealed
  held-out hash、重复和统计规则不变，生成过程中不得读取 held-out 内容或结果。
- active authority 与 production manifest 保持 sibling 回归基线且字节不变；CA-2 先生成全部具名 candidate artifact，
  由 generator 用 active-authority snapshot + 排序后的 `role=content_sha256`（不含路径和 commit）计算 candidate id，
  写入对应 namespace 并提交；随后 manifest 单独记录 `candidate_commit`、逐角色 hash、source/active-authority snapshot
  与 `evaluation_candidate`。ID 与 commit 分离以避免“目录名依赖包含自身目录的 commit hash”自引用。Runtime 不把
  candidate 宣称为 active。
- `required_next_call`、missing-sibling error/oracle 只从候选合同删除；历史 v1/v2 artifact 不覆盖。
- lifecycle v2 用 standalone-schema-negative、参数失败零提交、commit+Tool failure、code-mode 和恢复 fixtures
  替换 missing-sibling 场景。

完成证据：同一 builder 可重算全部 schema；candidate 没有 placeholder；active production hash 不变。

### CA-3：接入单一候选执行链

修改范围至少包括：

```text
tools/src/tool_spec.rs, tool_config.rs, apply_patch_tool.rs, taskspace_tool*.rs
tools/src/tool_registry_plan*.rs
core/src/tools/registry.rs, parallel.rs, sequence*.rs, code_mode/mod.rs
core/src/tools/router.rs, spec.rs, orchestrator.rs, hook_runtime.rs, hook_names.rs
core/src/tools/handlers/apply_patch*.rs, taskspace_control_*.rs
core/src/action_map/runtime.rs and reservation/lease paths
core/src/mcp_tool_call.rs, mcp_openai_file.rs
hooks/src/events/*.rs, hooks/src/engine/output_parser.rs
protocol/src/models.rs and ToolCallOutput types
codex-api/src/endpoint/responses.rs and every enabled provider wire mapper
MCP/dynamic Tool registry and ToolCallOutput provider mappers
```

- 实现一个 metadata source、decorator/parser、PreparedToolCall 协议和 typed outcome mapper。
- 删除或降级所有参与 TaskSpace gate、attribution、reservation 的 Tool 名称/source/命令内容 classifier；这些路径
  只能读取 registry capability metadata。若旧 classifier 仅供执行后 observer，必须有静态证明其结果不流入 gate/lease。
- 在候选代码中删除 `required_next_call`、missing-sibling preflight 和三个独立非终态 control 分支；无双 parser。
- 原业务 handler 只按 prepare/execute 拆分，不复制实现；Standard 不经过 transition decorator。
- 候选 runtime manifest 使用独立 contract id，不能伪装成 active authority。

### CA-4：确定性测试、日志与回滚演练

测试必须覆盖：

- 全 wire/ToolSpec/capability epoch property matrix，未知 Tool 和 reserved collision 阻止 epoch 激活。
- approval/grant 失效及 commit 前取消均零 Map 提交、零 ordinary Tool handler 执行；真实 PreToolUse 写文件/网络
  副作用必须形成 pre-hook fact，不得被零副作用断言掩盖；升级未预授权时 sandbox denial 不再请求 approval。
- 所有上传只能出现在 handoff 后；commit 原子失败、只读立即失败、dispatch/start failure、commit/handoff 两侧及
  handler 进入后取消有唯一类型和完整 factual payload；用 latch/fault injection 证明每例只命中一个 variant，并
  校验 revision/reservation/generation/handler-start/partial-effects。commit 后的 Returned/Failed/Cancelled 不回滚；
  PostToolUse NotRun/Failed 不替换冻结 outcome。
- reservation 原子归属新 lease，旧 lease 不记录 carrier；stale revision 发生在 commit 前。
- code-mode cell barrier、nested attribution、并行、turn-wide one-Patch 和 nested output。
- typed outcome 全 handoff/执行/Hook/retention/delivery 分支及文本/图片/McpToolOutputV1 presence/未知 block/截断/error
  conformance；原始 MCP fixture 跨阶段比较，不比较整个 frame hash。
- 静态/动态断言 commit 后不存在 approval 调用，prepare 阶段不存在上传/materialization；相关 owner 文件必须
  全部登记在 phase ownership contract。
- 对 `r7-carrier-entry-closure-v1.json` 每个入口证明 decorator/parser/epoch/outcome mapper 精确命中一次；手写清单、
  alias、deferred/dynamic/nested 漏项均失败。
- 运行 candidate 跨文件/first-parent history linter 全部反例；promotion/revert drill 必须证明 active authority/pointer
  与 candidate 状态不可能同时指向两个生产合同；candidate id 可机械重算，candidate commit 和全部具名 artifact
  均可从 Git 独立重建。
- 静态审计 action-map/lease gate 不读取 Tool 名、source 或参数内容；capability metadata 是唯一资格/归属输入。
- 运行 phase ownership lint，Phase E/F/G 只能引用 FLA evidence，不能拥有 production target 或独立 gate。
- Standard schema/wire/handler/cache identity 零变化；TaskSpace schema 只在 capability epoch 边界改变。
- 在隔离 Docker worktree 中执行一次完整 rollback drill：代码、parser、runtime manifest、schema/hash 恢复
  sibling baseline，再重建候选；只记录 `rollback_drill_passed/failed`，不改变真实 candidate 状态或 active authority。

日志增加：epoch id/hash/schema bytes/token、carrier capability/source、prepare/approval/hook/sandbox 状态、transition
commit、reservation lease、Tool status、原/剥离参数 hash、typed 子载体各分量 hash、barrier/Patch 序号。禁止记录
Patch 正文、密钥和私有 Tool 内容。

### CA-5：独立 Docker 三臂验证

只使用 CA-2 冻结的 `continuous-action-evaluation-v1.json`：

1. Standard；
2. CA-0 冻结的当前 sibling 回归基线；
3. FLA-3.5 候选。

simple/complex 每臂 3 次仅做接线诊断；独立 carrier-validation 的重复数、顺序、seed、门槛已在 CA-0 冻结，
不能根据 probe 或前三次结果修改、追加或停止。CA-1/CA-5 容器都不挂载 FLA-8 held-out；本阶段结果不得用于
FLA-8 正式决策。candidate report/schema 一旦出现旧 `combined_control_plus_next_rate` 即机器失败。

专用指标：`transition_required_count`、`transition_carrier_count/rate`、standalone-schema-negative、H-003、
prepare rejection、Patch/typed-output exact、request、token、cache by capability epoch、wall/provider/tool time。
旧 `combined_control_plus_next_rate` 不适用于候选，不得作为门禁。

硬门禁：correctness 不劣；所有 required transition carrier rate=100%；standalone/H-003=0；输入和 typed 子载体
保真=100%；不因交接增加 request；Standard wire hash 不变；同 epoch tools hash 稳定；成本满足专用合同预注册
非劣阈值。

### CA-6：审查、晋级或完整回滚

- 对候选生产 diff、rollback drill、trace 和三臂证据执行新的空白上下文对抗性审查。
- 通过后先由唯一 transition command 将 candidate 变为 `promotion_pending`；同一个 promotion commit 同时切换 active
  L4/L5 authority、production manifest active pointer、schema/parser、FLA-8 evaluation contract v2 和文档，并将
  candidate 原子变成 `promoted`。`required_next_call` 在 active 合同中彻底消失。
- 失败则 revert CA-3/CA-4 候选代码与 runtime manifest；active authority 始终保持 sibling baseline。candidate
  artifact 保留为 `rejected` 证据，不创建兼容生产路径。若 promotion commit 后全量复测失败，必须用单个 revert
  commit 同时恢复旧 active pointer/authority/runtime，并把 candidate 标记 `reverted`；禁止只回滚代码或留下
  promoted 双权威。
- promote 或 rollback 后重跑全量 hash/contract tests；只有 promotion 成功才将 FLA-3.5 标记 `active_verified`。

## 6. Phase Gate Matrix

| Phase | 独立验证 | 禁止依赖后续补证 | 退出条件 |
|---|---|---|---|
| CA-0 | 基线、wire/capability、时序、outcome、评估预注册 | CA-1 试错 | 前置合同无 placeholder |
| CA-1 | 本地 fault probe + 真实 provider 多载体 probe | 生产 sample | 所有生产可达组合通过 |
| CA-2 | candidate schema/oracle/eval/rollback lint | CA-3 Runtime 容错 | standalone 不可表达且 active authority 未变 |
| CA-3 | Rust/unit/integration/wire tests | CA-5 Agent 自纠 | 单一候选链完整接通 |
| CA-4 | fault、code-mode、反馈保真、日志、rollback drill | CA-5 人工解释 | 全分支可机器观测且可完整回滚 |
| CA-5 | 专用三臂预注册评估 | FLA-8 held-out | 所有硬门禁与非劣门禁通过 |
| CA-6 | 空白 reviewer + promotion/rollback 全量复测 | FLA-4 | 唯一 active 合同确定 |

## 7. 后续阶段单一 DAG

```text
FLA-0 -> FLA-1 -> FLA-2 -> FLA-3 -> FLA-3.5
FLA-3.5 -> FLA-4 -> FLA-5 -> FLA-6 -> FLA-7 -> FLA-8 -> R7 Phase H
                                               |          |
                                               |          +-- 包含 R7 Phase G 四臂 projection 子矩阵
                                               +-- 完成 R7 Phase E 生命周期等价与 Phase F L5 单架构审计
```

| 既有阶段 | 唯一所有权与冲突处理 |
|---|---|
| FLA-4 | 只在晋级 carrier 上优化 L4 description/discriminator；不实现 carrier，不正式化 sibling |
| FLA-5 | 冻结并验证 transition fact + opaque Tool output 的 conformance；transport 只由 FLA-3.5 实现 |
| FLA-6 | 只做读写拆分、MCP output schema、DeepSeek strict 三个独立实验 |
| FLA-7 / R7 Phase E | FLA-7 是 lifecycle/recovery/projection 唯一实现和验收阶段；Phase E 是其产品里程碑别名，不单独改代码或跑第二套 gate |
| FLA-7 / R7 Phase F | FLA-6/7 共同产出单架构审计证据；Phase F 只读引用并汇总，不形成另一套 acceptance |
| FLA-8 / R7 Phase G | Phase G 四臂是 FLA-8 七臂正式矩阵的 projection-policy 子集，共用 run/artifact；默认值与 promote 决策只由 FLA-8 给出 |
| R7 Phase H | 仅在 FLA-8 完成后做发布收口和经授权审查 |

历史 D.2-D.4、FLA-2/3 结果保持原样；FLA-8 held-out、旧评估 artifact 和历史拒绝不得回写。

## 8. 风险、回滚与完成定义

| 风险 | 早期信号 | 控制 |
|---|---|---|
| freeform/code-mode 无 carrier | wire 矩阵缺项、nested call 绕 barrier | CA-0 矩阵 + CA-1/4 硬门禁 |
| prepare 拆分复制 handler | 同一 Tool 出现两套业务执行逻辑 | PreparedToolCall 共享接口；源码重复审计 |
| approval/hook 后提交顺序错误 | denial 后 revision 变化或旧 lease reservation | fault state table + 原子 commit/reserve |
| Patch/Tool 结果被 frame 扭曲 | typed 子载体 hash/结构不等 | 按 carrier 子载体 conformance，任一失败阻塞 |
| schema 膨胀或缓存漂移 | epoch schema token 超限、同 epoch hash 改变 | 预注册 byte/token 上限；只在 epoch 边界变化 |
| MCP collision/泄漏 | server 收到 reserved 字段 | epoch 激活失败 + 剥离测试 |
| authority 半切换 | candidate 被称为 active、回滚后 hash 不一致 | candidate namespace + promotion commit + rollback drill |

回滚覆盖 CA-2 candidate registry、CA-3/4 代码、runtime manifest、schema/parser 和所有生成 hash。active authority
在 CA-6 promotion 前不切换；失败证据保留但不能被生产读取。不得使用 feature flag、双 parser 或兼容 session。

本修复只有同时满足以下条件才能完成：所有生产可达 carrier 结构成立；standalone 非终态交接不可表达；
prepare/commit/execute 时序可证明；Patch 与 typed Tool 子载体保真；code mode/MCP/权限链无绕过；自然样本 H-003
为零；成本非劣；rollback 可复现；对抗审查无 blocking finding；FLA-4 及后续只引用晋级合同。
