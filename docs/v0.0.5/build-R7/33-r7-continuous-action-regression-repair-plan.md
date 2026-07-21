# R7 连续动作合同回归修复计划

- Created: 2026-07-21
- Version: 1.0
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
| Code mode | 顶层 code-mode Tool 携带 transition；cell 内 nested calls 继承提交后的 lease | nested call 自行提交 transition；绕过 turn barrier |
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
失败零状态提交、零 Tool 副作用。终态 action 继续由 control 单独表达。

### 4.4 `prepare -> commit -> execute` 机械协议

不能在现有 handler 外简单前置 Map commit。FLA-3.5 选择一个共享 `PreparedToolCall` 协议，把现有 Tool 的准备
和执行阶段显式拆开，但不复制 handler：

1. **parse**：解析 carrier，剥离 transition，校验原业务参数。
2. **prepare**：运行现有 PreToolUse、参数改写、权限、sandbox 与 approval 流程，产出不可变
   `PreparedToolCall`；此阶段不产生 Tool 业务副作用，不提交 Map。
3. **commit + reserve**：在同一 action-map 临界区重新校验 revision/lease，原子提交 transition，并为目标新 lease
   建立该 prepared call 的 reservation。旧 lease 不接收本次调用。
4. **execute**：原 handler 的执行部分消费 prepared call，不再次请求 approval，也不重新解释参数。
5. **post**：PostToolUse 只观察原 Tool outcome；随后 context mapper 附加 transition fact。

取消与失败边界固定为：prepare/approval/hook/sandbox 拒绝或 commit 前取消均零提交零执行；commit 后、Tool 启动前
取消记作“transition committed + tool cancelled”，不回滚；执行失败同样不回滚。任何无法安全拆出无副作用
prepare 的 Tool 在候选中不能标记 carrier-capable，且 TaskSpace capability epoch 不能带着未知缺口激活。

### 4.5 Carrier-neutral typed outcome

一个 provider call 只能有一个 call id，但必须保留两个独立事实。内部唯一类型为：

```text
TaskSpaceCarrierOutcome {
  transition_fact: TaskSpaceTransitionFact,
  tool_output: Opaque<ToolCallOutput>
}
```

`transition_fact` 只含 action、revision、commit/lease 和 factual error；`tool_output` 是 hooks 完成后的原始
ToolCallOutput 子载体。不得把 Tool 输出塞入 `TaskSpaceControlResultV2`，也不得把两类事实压成“整体成功/失败”。
provider mapper 按第 4.2 节载体生成一个合法 output：支持 content items 时增加独立 transition fact item 并原序
保留 Tool items；只有 text output 时使用版本化 factual frame，但其 `tool_output` 子载体必须可逆恢复。

保真门禁比较的是 frame 解码后的 `tool_output` 子载体与原 outcome，而不是比较必然增加 transition fact 的整个
provider payload。文本字节、图片 URL/顺序、MCP structured content、截断引用、exit status 和 error class 分别
计算 hash/conformance。任一 carrier 无法无损映射时阻塞。FLA-3.5 拥有 transport 实现；FLA-5 只冻结并验证其
result conformance，不再次实现 envelope。

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
- 冻结第 4.4 节 prepare/commit/execute 状态表，覆盖 hooks、approval、sandbox、取消和 reservation 归属。
- 冻结 typed outcome 各 provider wire 映射与子载体 hash 公式。
- 新建 authority JSON Schema，统一实施状态枚举；记录当前 production commit、source/wire hashes。
- 预注册 FLA-3.5 专用评估合同的开发样本与独立 carrier-validation 样本；FLA-8 held-out 继续封存且本阶段不可见。

完成证据：机器基线、状态表、wire/capability 矩阵和评估预注册均可独立检查；当前生产行为不变。

### CA-1：本地可行性与真实 Provider 探针

只建隔离 probe，不接生产：

1. 对全部 wire/ToolSpec/source 组合验证 schema 装饰、freeform Patch function 投影、reserved 字段剥离和
   Standard 零变化。
2. 用 fake Tool 验证 prepare denial、approval denial、sandbox denial、commit 前后取消、commit 后执行失败和
   新 lease reservation；任何 side effect 都有计数器。
3. 验证 typed outcome 在 text、content items、image、MCP structured content、截断和错误结果上的可逆映射。
4. 验证 code-mode outer carrier、nested attribution、`Promise.all` barrier 和 turn-wide one-Patch gate。
5. 使用真实 DeepSeek endpoint 对 exec、direct/freeform Patch、MCP 和多 Tool response 每臂至少 6 次 probe。

准入：结构合法、Patch exact、原参数、typed 子载体、lease 归属均为 100%；拒绝/取消时 commit/side-effect 符合
状态表；无 reserved 泄漏、无额外 provider request。任一生产可达组合失败即停止 CA-2。

### CA-2：冻结候选机器合同，不提升 active authority

- 候选 artifact 放入独立 candidate namespace：L4 schema、transition schema、typed outcome、lifecycle oracle v2、
  capability matrix、rollback manifest、`continuous-action-evaluation-v1.json` 和 FLA-8 evaluation contract v2。
- FLA-8 v2 只把旧 `combined_control_plus_next_rate` 机械替换为 transition carrier 指标；样本 identity、sealed
  held-out hash、重复和统计规则不变，生成过程中不得读取 held-out 内容或结果。
- active authority 继续指向 sibling 回归基线；candidate registry 记录 artifact/hash/commit 与
  `evaluation_candidate`，Runtime 不把它宣称为 active。
- `required_next_call`、missing-sibling error/oracle 只从候选合同删除；历史 v1/v2 artifact 不覆盖。
- lifecycle v2 用 standalone-schema-negative、参数失败零提交、commit+Tool failure、code-mode 和恢复 fixtures
  替换 missing-sibling 场景。

完成证据：同一 builder 可重算全部 schema；candidate 没有 placeholder；active production hash 不变。

### CA-3：接入单一候选执行链

修改范围至少包括：

```text
tools/src/tool_spec.rs, tool_config.rs, apply_patch_tool.rs, taskspace_tool*.rs
core/src/tools/registry.rs, parallel.rs, sequence*.rs, code_mode/mod.rs
core/src/tools/handlers/apply_patch*.rs, taskspace_control_*.rs
core/src/action_map/runtime.rs and reservation/lease paths
codex-api/src/endpoint/responses.rs and every enabled provider wire mapper
MCP/dynamic Tool registry and ToolCallOutput provider mappers
```

- 实现一个 metadata source、decorator/parser、PreparedToolCall 协议和 typed outcome mapper。
- 在候选代码中删除 `required_next_call`、missing-sibling preflight 和三个独立非终态 control 分支；无双 parser。
- 原业务 handler 只按 prepare/execute 拆分，不复制实现；Standard 不经过 transition decorator。
- 候选 runtime manifest 使用独立 contract id，不能伪装成 active authority。

### CA-4：确定性测试、日志与回滚演练

测试必须覆盖：

- 全 wire/ToolSpec/capability epoch property matrix，未知 Tool 和 reserved collision 阻止 epoch 激活。
- approval、PreToolUse、sandbox 拒绝及 commit 前取消均零提交；commit 后取消/失败不回滚；PostToolUse 看到原 outcome。
- reservation 原子归属新 lease，旧 lease 不记录 carrier；stale revision 发生在 commit 前。
- code-mode cell barrier、nested attribution、并行、turn-wide one-Patch 和 nested output。
- typed outcome 子载体的文本/图片/MCP/截断/error conformance；不比较整个 framed payload hash。
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

simple/complex 每臂 3 次仅做接线诊断；独立 carrier-validation 样本的重复数、顺序、seed、门槛在看到候选结果前
由评估合同冻结，不能根据前三次结果追加或停止。FLA-8 held-out 在整个 CA-5 保持 sealed，本阶段结果不得用于
FLA-8 正式决策。

专用指标：`transition_required_count`、`transition_carrier_count/rate`、standalone-schema-negative、H-003、
prepare rejection、Patch/typed-output exact、request、token、cache by capability epoch、wall/provider/tool time。
旧 `combined_control_plus_next_rate` 不适用于候选，不得作为门禁。

硬门禁：correctness 不劣；所有 required transition carrier rate=100%；standalone/H-003=0；输入和 typed 子载体
保真=100%；不因交接增加 request；Standard wire hash 不变；同 epoch tools hash 稳定；成本满足专用合同预注册
非劣阈值。

### CA-6：审查、晋级或完整回滚

- 对候选生产 diff、rollback drill、trace 和三臂证据执行新的空白上下文对抗性审查。
- 通过后一个 promotion commit 同时切换 active L4/L5 authority、production manifest、schema/parser、FLA-8
  evaluation contract v2 和文档；
  `required_next_call` 在 active 合同中彻底消失。
- 失败则 revert CA-3/CA-4 候选代码与 runtime manifest；active authority 始终保持 sibling baseline。candidate
  artifact 保留为 `rejected` 证据，不创建兼容生产路径。
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
| FLA-7 / R7 Phase F | FLA-6/7 共同产出单架构审计证据；Phase F 只汇总，不形成另一套 acceptance |
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
