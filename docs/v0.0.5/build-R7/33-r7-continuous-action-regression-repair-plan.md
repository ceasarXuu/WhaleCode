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

### 4.1 真实动作拥有轻量交接前缀

优先候选是由共享 Tool builder 在 TaskSpace profile 下，为可执行真实动作增加一个静态、可选的
`taskspace_transition` 参数。该参数只承载小型状态交接，不承载普通 Tool 的参数或输出。原 Tool 参数继续由
原 schema 校验和原 handler 消费。

示意形态：

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

这不是新的 Tool，也不是第二层调度架构。它是现有真实 Tool schema 的统一装饰规则：

- `apply_patch.input` 仍是顶层原生 Patch 字符串；
- `exec_command`、文件工具和 MCP Tool 保持原名称、原业务参数、原 handler；
- 装饰器、解析器和剥离逻辑只有一份，按 capability set 机械生成并计入 `tools_hash`；
- schema 在会话内静态，不按 revision 或状态动态改写；
- reserved 字段碰撞必须在 Tool 注册时确定性失败，不能静默改名或透传给第三方服务。

### 4.2 结构保证

目标 L4 不再向 Agent 暴露 `initialize_map`、`bind_node`、`complete_then_continue` 这三个可独立调用的
`taskspace_control` 分支。它们只存在于真实动作的 `taskspace_transition` 分支中，因此“状态交接但没有真实
动作”在 provider-visible schema 中不可表达。

`taskspace_transition` 对普通连续动作保持可选：同一 active Work 内执行第二个及后续动作不需要重复交接。
Runtime 只根据 canonical state 做机械校验：空 Map 的第一个真实动作必须携带初始化；尚未 binding 的 Ready
节点必须先由该节点真实动作携带绑定；从 active Work 切换到后继必须携带完成交接。校验失败零提交、零执行。

### 4.3 执行与反馈

单个 carrier call 的固定顺序是：

1. 解析并独立校验交接参数和原 Tool 参数。
2. 在执行任何副作用前完成 sequence、权限、sandbox 和 approval preflight。
3. 原子提交 Agent 明确给出的 Map 交接。
4. 使用剥离交接字段后的原始参数调用现有 Tool handler。
5. 返回交接事实与未经摘要、改写或裁剪的原 Tool 结果。

若交接提交后普通 Tool 失败，Map 不回滚；反馈必须同时保留“交接已提交”和“Tool 失败”两个事实。文本、图片、
MCP `structuredContent`、截断引用和 `apply_patch` 结果都必须有逐载体保真测试。若现有通用 ToolCallOutput 无法在
不损失原结果的前提下承载两类事实，FLA-3.5 必须先补齐通用事实 envelope，不能通过丢弃交接反馈或包装成
自由文本建议来绕过。

### 4.4 多动作与顺序

Agent 可在同一 assistant response 中继续发出其他独立 Tool calls。携带交接的真实动作是该序列的机械 barrier：
交接和该动作先完成，再按现有并行/串行规则处理后续调用。依赖该动作未知结果的后续步骤自然进入下一次
provider request；Runtime 不判断语义依赖。

现有“一次 response 最多一个 Patch”合同保持不变。本阶段不得借连续动作修复放宽多 Patch，也不得把后续
普通动作强制串行化。

## 5. 分阶段实施

### CA-0：冻结回归基线与验收口径

实施：

- 固定当前生产 commit、L1-L5 identity、Tool schema hash 和 H-003 trace。
- 从历史 artifact 重算 R5、D.2、D.4 与 FLA-3 的 standalone、拒绝、request、Patch exact 指标。
- 冻结 simple、complex 和 held-out 样本，不为触发 carrier 改写任务答案或操作步骤。

完成证据：一份机器基线 JSON 和逐 request 对账；当前行为不变。

### CA-1：Provider 与载体能力探针

只建 probe，不改生产路径。使用真实 DeepSeek endpoint 和生产 adapter 分别验证：

1. `taskspace_transition + exec_command` 的单调用生成、参数合法和执行顺序。
2. `taskspace_transition + apply_patch` 的合法 JSON、逐字 Patch exact 和大 Patch 稳定性。
3. `taskspace_transition + MCP Tool` 的 schema 合并、reserved 字段剥离、原始参数与结果保真。
4. 交接成功 + Tool 成功、交接成功 + Tool 失败、参数失败零提交三类反馈。
5. 多 Tool response 中 carrier barrier、独立 sibling 并行和结果依赖留到下一请求。

每臂至少 6 次定向 probe。准入条件为：结构合法率 100%、Patch exact 100%、原 Tool 参数与反馈逐字/结构等价、
无第三方参数泄漏、无额外 provider request。任一核心载体失败即停止 CA-2，不用提示词或 Runtime fallback 掩盖。

### CA-2：冻结候选机器合同

- 新建候选 L4 schema、transition schema、result envelope 和 lifecycle oracle v2；旧 v2 artifact 作为回归证据保留。
- authority manifest 指向唯一候选；`required_next_call` 从目标合同、错误码和 oracle 中删除。
- 为所有可见 Tool 生成 capability/collision 清单；不支持 carrier 的 Tool 必须在启用候选前解决，不能静默降级。
- 冻结状态提交、Tool 失败、MCP、图片、截断读取、Patch 和多调用顺序的完整 fixtures。

完成证据：schema 可从同一 builder 重算，所有负例证明 standalone 非终态交接不可表达，尚不改变生产行为。

### CA-3：接入生产执行链

修改范围：

```text
third_party/codex-cli/codex-rs/tools/src/taskspace_tool*.rs
third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_*.rs
third_party/codex-cli/codex-rs/core/src/tools/sequence*.rs
third_party/codex-cli/codex-rs/core/src/tools/router*.rs
third_party/codex-cli/codex-rs/codex-api/src/endpoint/responses.rs
MCP Tool schema 装配与 ToolCallOutput 共享入口
```

- 用一个共享 decorator/parser 接入所有真实 Tool，不复制业务 schema 或 handler。
- 删除生产 `required_next_call`、跨 sibling missing preflight 和对应 parser 分支，不保留兼容模式。
- 保留 terminal control、纯 Map/read actions、原 router/approval/sandbox/hook 和 Tool 输出处理。
- 先完成 deterministic tests，再启用生产 schema；不得出现半新半旧 wire。

### CA-4：测试与日志

定向测试至少覆盖：

- 三种非终态交接 valid/invalid fixtures；standalone schema negative fixture。
- 空 Map 普通动作无初始化、无 binding 普通动作、stale revision、非法 DAG、载体参数错误均零提交零执行。
- 交接提交后 Tool 失败不回滚，反馈同时保留两类事实。
- direct/freeform `apply_patch`、function Tool、MCP、图片和截断输出保真。
- router、approval、sandbox、hook、one-Patch-per-response 与并行 Tool 行为不回归。
- Standard schema、wire、Tool 行为和 cache identity 零变化。

新增结构化日志：`transition_carrier_tool`、`transition_kind`、`transition_commit`、`tool_execution_status`、
`original_args_hash`、`stripped_args_hash`、`original_output_hash`、`delivered_output_hash`、barrier 序号和
`standalone_nonterminal_transition`。日志不得记录 Patch 正文、密钥或私有 Tool 内容。

### CA-5：Docker 三臂验证

统一使用 bundled skills 已启用的 Docker harness，对 simple、complex、held-out 各运行：

1. Standard；
2. 当前 R7 `required_next_call` 回归基线；
3. FLA-3.5 候选。

每个样本每臂先跑 3 次接线诊断；无环境异常后按冻结评估合同扩展。报告 solved、公开/隐藏验证、Map、逐 request
动作、standalone、拒绝、Patch exact、input/output/reasoning token、cache、wall/provider/tool time 和长尾。

硬门禁：

- 候选 correctness 不低于 Standard 和 R7 基线；
- 非终态 standalone 与 H-003 拒绝均为 0；
- 交接 + 首个真实动作合并率 100%；
- Patch/Tool 输入输出保真 100%；
- 不因交接增加 provider request；
- Standard wire hash 不变；TaskSpace schema 在会话内静态；
- request、token、cache、wall time 满足 `25` 号规格的非劣阈值。

### CA-6：晋级与后续阶段解锁

- 对生产 diff、trace 和三臂证据执行新的空白上下文对抗性审查。
- 阻塞发现关闭后，候选一次性替换当前合同，更新 L4/L5 authority 和文档。
- CA-5 任一硬门禁失败则 revert 候选生产 commit，保留 probe/结果，不进入 FLA-4。
- 只有 CA-6 标记 `active_verified` 后，FLA-4 才能以新合同做 action-local 描述与 schema 收敛。

## 6. Phase Gate Matrix

| Phase | 独立验证 | 禁止依赖后续补证 | 退出条件 |
|---|---|---|---|
| CA-0 | 历史 artifact 与当前 trace 重算 | CA-1 probe | 回归基线可复现 |
| CA-1 | 真实 provider 多载体 probe | 生产 sample | 全载体达到准入门槛 |
| CA-2 | schema/oracle/golden/lint | CA-3 Runtime 容错 | standalone 在结构上不可表达 |
| CA-3 | Rust/unit/integration/wire tests | CA-5 自然样本纠错 | 单一生产链完整接通 |
| CA-4 | fault、反馈保真、日志自测 | CA-5 人工 trace 猜测 | 全分支可机器观测 |
| CA-5 | Docker 三臂重复运行 | CA-6 审查意见补结果 | 所有硬门禁与非劣门禁通过 |
| CA-6 | 空白 reviewer + 关闭阻塞项 | FLA-4 | promote 或 revert 决策落地 |

## 7. 与既有后续计划的关系

| 既有阶段 | 调整 | 冲突处理 |
|---|---|---|
| FLA-4 L4 input schema | 被 FLA-3.5 阻塞 | 不再正式化当前 sibling 合同；只在新 carrier 上优化 description/discriminator |
| FLA-5 result algebra | 保留 | 将 carrier 的交接事实 + 原 Tool 结果纳入同源结果合同，删除 missing-sibling 分支 |
| FLA-6-E1 读写拆分 | 保留为后续实验 | 不改变 carrier、router 或权限基线 |
| 原 FLA-6-E2 移除 `required_next_call` | 删除 | 这不是实验，而是 H-003 回归修复的一部分 |
| FLA-6 MCP output schema | 保留 | CA-1/CA-4 只保证 carrier 透传；是否暴露 MCP `outputSchema` 仍独立评估 |
| FLA-6 DeepSeek strict | 保留 | 不把 strict 当作连续动作成立的前提 |
| FLA-7 projection/recovery | 保留 | carrier 不改变 canonical Map、renderer 或三种 emission policy |
| FLA-8 正式评估 | 保留 | 使用 FLA-3.5 晋级后的唯一 L4 基线 |
| R7 Phase E-H | 顺延 | FLA-3.5 未晋级前不得宣称工具链或生命周期收口 |

历史 D.2、D.3、D.4、FLA-2 和 FLA-3 结果文档保持原样，分别作为当时版本的事实证据；不得回写成当前设计。

## 8. 风险、回滚与完成定义

| 风险 | 早期信号 | 控制 |
|---|---|---|
| decorator 使 Tool schema 膨胀 | 固定 input/tool schema token 明显上升 | CA-1 记录每 Tool 增量；CA-5 执行成本非劣门禁 |
| Patch 再次被 JSON 结构破坏 | JSON 非法或 Patch hash 不等 | Patch 保持顶层 `input`；100% exact 硬门禁 |
| Tool 结果包装导致语义丢失 | 原始与 delivered hash/结构不等 | 按载体 conformance；任一不等阻塞 |
| MCP reserved 字段泄漏或冲突 | server 收到字段、注册冲突被忽略 | 注册期确定性失败 + router 剥离测试 |
| 生命周期提交与权限顺序错误 | approval 拒绝后 Map 已提交 | 权限/sandbox preflight 必须先于状态提交 |
| 中央 decorator 演变成第二 router | 分支按 Tool 名复制执行逻辑 | 只做 schema 装饰、字段剥离和交接；原 handler 唯一 |
| 动态 schema 破坏缓存 | 同 capability 下 tools hash 逐 request 改变 | schema 仅由 profile/capability set 决定并在会话内冻结 |

回滚单位是 CA-3/CA-4 的候选生产提交；不使用 feature flag、双 parser 或兼容 session。计划、probe 和失败证据
保留，生产回到 CA-0 冻结 baseline。

本修复只有同时满足以下条件才能称为完成：结构上不存在 standalone 非终态交接；所有真实 Tool 继续走原执行链；
Patch 和反馈完全保真；自然样本不再出现 H-003；成本不劣；对抗性审查无未关闭 blocking finding；FLA-4 及
后续计划已经以新合同为唯一基线。
