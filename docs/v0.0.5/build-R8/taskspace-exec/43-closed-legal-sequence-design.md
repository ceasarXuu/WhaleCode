# TaskSpace Exec 闭集合法序列设计

- Status: Approved product design / implementation not started
- Created: 2026-08-12
- Updated: 2026-08-12
- Product Authority: [`00-product-contract.md`](00-product-contract.md#confirmed-product-decisions)
- Applicable Decisions: PD1～PD7
- Scope: 只重构 Agent-visible TaskSpace Exec 动作序列，并收敛无证据的 `blocked` 状态；不改变普通 Tool 原生合同或 Standard

## 1. 产品定义

闭集规定的是一次 `taskspace_exec` 中 **Map 动作与 Tool 动作的合法顺序**，不是用一组高层语义命令
取代 Agent 的判断。

Agent 仍然决定：

- 本轮要提交哪种合法序列；
- `update_map` 要如何修改 Map；
- 调用哪些 Tool、使用什么原生参数、归属哪些节点；
- 何时完成节点、调整 Map、重开或结束整个 Map。

Runtime 只负责：

1. 解码 Agent 选择的合法序列形状；
2. 将其中的 Map 动作机械交给 canonical Map transaction；
3. 将 client Tool 机械交给原 Router 执行；
4. 将 Provider 已执行 Tool 与顶层原始事实核对后记录，不重复执行；
5. 在副作用前校验结构、Map/DAG、Tool 参数、节点归属和单 Patch 等硬规则。

## 2. 为什么要改

当前 `calls[]` 允许 Agent 任意排列 Map call 和 client call，再由 Runtime 事后判断排列是否合法。这导致：

- 合法的“完成前置并继续”只是 description 中的规则，不是 schema 中可直接选择的形状；
- Agent 经常同时写“完成父节点”和“将子节点改为 `in_flight`”，在 readiness 尚未派生时触发拒绝；
- 数组顺序对 Map call 有先后意义，对 client calls 却不构成第二份 DAG，同一字段承载了两种规则；
- 非法组合只能在 Agent 已生成后拒绝，形成额外请求和上下文噪声。

目标是把已知合法顺序写入 Tool schema，而不是让 Runtime 增加语义决策。

## 3. 证据准入规则

合法序列不允许凭空创建。每个序列必须满足下列任一条：

1. **E1：真实运行证据**：当前生产 trace 中已出现且行为合法；
2. **E2 + E3：确定性工程证据 + 已确认产品需要**：当前 canonical transaction 和生产预检已证明能力成立，
   且产品合同或用户决策明确要求保留。

不能只因为“未来可能有用”就增加序列。后续新类型也必须按同一门槛逐项加入。

## 4. 七个核心场景的证据

| 产品场景 | 真实证据 | 确定性/产品证据 | 结论 |
|---|---|---|---|
| 初始化并工作 | `initialize_map + exec_command` 在最新可解码 trace 中出现 10 次 | 生产 preflight 要求 initialize 同批带真实 work | 纳入 |
| 继续工作 | 单组 client Tool 出现 15 次 | client Tool 已复用原 Router | 纳入 |
| 完成前置并继续 | `completed + apply_patch` 5 次，`completed + exec_command` 4 次 | 父节点 completion 后 readiness 由 parents 机械派生 | 纳入 |
| 完成并关闭 Map | `completed + finish_map` 6 次；多轮真实任务已闭合 Map | `finish_map` 是已确认显式终态 | 纳入 |
| 读取 Map | 可解码 trace 中出现 1 次 | `read_map` 已有独占批次合同和完整视图测试 | 纳入 |
| 重开并继续 | 当前简单样本没有真实在线轨迹 | 用户已确认“用户反馈未完成时重开同一 Map”；`reopen_requires_and_accepts_real_followup_work` 已验证生产路径 | 按 E2+E3 纳入，不伪称在线验证 |
| 调整 Map 并继续 | 当前简单样本未形成独立统计 | canonical `update_map` 已验证新增节点、修改 parents/goal/content/state；复杂任务不能失去 Map 演化能力 | 按 E2+E3 纳入，不创建新的修改语义 |

真实次数来自 R8 现有 rollout 的可解码 TaskSpace Exec 调用汇总；只用于证明场景存在，不用于声称稳定率。

可复核证据入口：

- [`39-phase-b5-va02-revalidation-result.md`](39-phase-b5-va02-revalidation-result.md) 记录初始化、连续 Tool 工作和 waiting 拒绝；
- [`40-va02-source-structured-ab-plan.md`](40-va02-source-structured-ab-plan.md) 记录初始化、handoff、最终闭合和错误组合；
- [`42-phase-b5-schema-compression-result.md`](42-phase-b5-schema-compression-result.md) 记录完成前置后继续测试、完成 verify 后 finish；
- [`taskspace_exec_preflight_tests.rs`](../../../../third_party/codex-cli/codex-rs/core/src/tools/taskspace_exec_preflight_tests.rs)
  中 `reopen_requires_and_accepts_real_followup_work` 验证 reopen 必须带真实后续工作；
- [`taskspace_exec_tests.rs`](../../../../third_party/codex-cli/codex-rs/core/src/tools/taskspace_exec_tests.rs)
  中 `update_changes_parents_and_state_through_canonical_transaction` 验证纯 Map 演化能力。

## 5. 原子动作模型

### 5.1 Map 动作

Map 动作仍是现有五个受限 canonical operation：

- `initialize_map`；
- `update_map`；
- `read_map`；
- `reopen_map`；
- `finish_map`。

`update_map` 不是万能 Tool。它只能修改 canonical Map：新增 Work node，修改节点 `goal/content/parents/state`，
并继续接受完整 DAG、转移和 revision 硬校验。它不能执行 Tool、不能修改文件、不能绕过节点依赖，
也不能关闭 Map。

因此闭集必须保留纯 `update_map` 序列。它可用于只更新节点内容、目标、依赖或生命周期，不强制捆绑 Tool 动作。
这会取代当前“完成节点的 update 必须同批带后续 work/finish”的过度性规则。

### 5.2 Tool 动作

Exec 中只有一种平级 Tool 动作位置，不再分成 `client_work[]` 和 `hosted_work[]`，也不保留序列外
`hosted_bindings[]`。

每个 Tool 动作都表达：

- Tool 原生身份；
- Tool 的原生调用内容；
- Agent 声明的节点归属。

对 Agent 而言，`exec_command`、MCP Tool、`web_search`、`image_generation` 都在同一 `tools[]` 位置中，受同一序列顺序和
节点归属规则约束。不向 Agent 暴露“Hosted 是例外通道”这种平行概念。

Runtime 只在执行适配时机械区分：

| Tool 来源 | Runtime 行为 | 不变的合同 |
|---|---|---|
| Client Tool | 预检后交给原 Router 执行一次 | 原生 schema、权限、sandbox、hook、result |
| Provider Tool | 与本响应顶层已执行事实逐项核对，记录并绑定节点，不再执行 | Provider 原始 ID、状态、结果和执行事实 |

这是执行归属适配，不是 Agent-visible 序列分叉。Provider Tool 也不会因为已经执行就绕过 Map 顺序、节点归属或
漏绑/错绑检查。

## 6. 合法序列闭集

七个产品场景不需要七套重复的高层命令。“完成前置并继续”和“调整 Map 并继续”在动作顺序上都是
`update_map -> tools`，差异由 Agent 声明的 `update_map` 内容忠实表达。

目标闭集为：

| ID | 顺序形状 | 覆盖场景 | 关键硬规则 |
|---|---|---|---|
| L1 | `initialize_map -> tools+` | 初始化并工作 | initialize 为首动作；至少一个 Tool |
| L2 | `tools+` | 继续一个或多个已可执行节点 | Tools 不表达第二份 DAG；结果依赖动作留到下一请求 |
| L3 | `update_map` | 纯 Map 更新 | 非空、有效 canonical update；不执行 Tool |
| L4 | `update_map -> tools+` | 完成前置并继续；调整 Map 并继续 | Tool 只能使用 update 后候选 Map 中真实可执行的节点 |
| L5 | `update_map -> finish_map` | 完成最后节点并关闭 Map | finish 为末动作；不预留无证据的终态 Tool 混合 |
| L6 | `read_map` | 单独读取完整 Map | 独占批次 |
| L7 | `reopen_map -> update_map -> tools+` | 用户反馈未完成后重开并继续 | Agent 显式新增/调整后续 Work 与 Finish 依赖；Runtime 不生成返工节点 |
| L8 | `finish_map` | Map 已 Ready 时显式关闭，包括中断后恢复结束 | 只在 Finish 已 Ready 时合法 |

L8 不是凭空扩展：`finish_map` 本就是已确认的独立显式终态 transaction。如果只保留 L5，一旦 Agent 在
前一请求已完成最后 Work 却在 finish 前中断，就会被迫提交无意义 update 才能关闭 Map。

`tools+` 中可同时包含 client 和 Provider Tool。它们在序列中地位相同；Provider Tool 的真实执行已发生，
但 Runtime 仍以该序列的候选 Map 检查节点归属并完成记录。

当前没有“新增 Tool action 后立即关闭 Map”的独立合法证据，因此 L5 不包含 `tools`。若未来真实 trace 证明该动作组合必要，
必须按第 3 节重新准入，不能以 `tools*` 预留未证明能力。

## 7. `work -> in_flight`

用户已批准：Agent 在 `ready` Work node 上声明 Tool 动作，就是对“启动该节点”的显式声明。Runtime 在
dispatch/reconcile 前只机械归一化 `ready -> in_flight`。

边界：

- Agent 仍显式选择 Tool 和 `node_id/node_ids`；Runtime 不选节点；
- 已是 `in_flight` 的节点保持不变；
- Tool 成功、失败或取消不自动完成节点；
- completion 仍只来自 Agent 的 `update_map`；
- Agent 仍可以用纯 L3 `update_map` 先更新 Map，不被强制同批调用 Tool。

## 8. 移除 `blocked` 的证据结论

### 8.1 已发现证据

- 当前源码、schema、projection、preflight 和确定性测试中存在 `blocked`；
- 历史文档声明 blocked 节点仍可以调用 Tool，Tool 成败不自动 block/unblock；
- 在当前 R8 真实 benchmark 产物、Provider trace 和运行结果中，没有找到 Agent 实际创建/使用 blocked 节点并由此
  改善推理、避免错误或完成任务的证据。

因此现有证据只证明“blocked 被实现了”，不证明“blocked 对 Agent 有产品收益”。

### 8.2 目标模型

目标 Node state 收敛为：

```text
waiting -> ready -> in_flight -> completed
```

- `waiting/ready` 继续由 parents 是否 completed 机械派生；
- `in_flight` 由 Agent 选择在节点上工作，或显式纯 Map update 表达；
- `completed` 只由 Agent 显式 update 表达；
- 外部条件暂时不满足时，Agent 可将事实写入节点 `content`，继续其他 Ready 节点，或在 Map 保持未闭合时向用户
  说明当前缺少的外部条件；不再为此增加一套节点限制状态和转移规则。

不保留兼容读取或 migration：产品是实验性产品，没有需要保留的 TaskSpace 数据。若未来出现可复现的正向证据，
再按 PD2 作为新产品决策评估，不预留死代码。

## 9. Agent-visible Schema 原则

1. `sequence` 是 exact discriminator + disjoint `anyOf`；没有 generic/raw/custom/other 分支。
2. 每个 branch 只展示对应顺序的 Map 槽位和统一 `tools[]` 槽位。
3. Map operation 输入直接复用 canonical operation schema；`update_map` 不被重命名或拆成多套语义命令。
4. 全部 Tool 共用一份 TaskSpace Tool action catalog；client/Provider 只在 Runtime execution adapter 中分流。
5. 原生 Tool catalog 在 final declaration 中只定义一次，由各含 Tool 序列引用；禁止按序列数重复展开。
6. 同一 `tools[]` 中的顺序只用于稳定身份与反馈，不表达 Tool B 依赖 Tool A 的结果。
7. Standard final wire 逐字不变。

DeepSeek 官方 strict Tool schema 支持 `anyOf` 以及 `$ref` + 可复用 definition，可以用于闭集和 Tool catalog 去重。
Provider 实际接受的 `$def/$defs` 形状仍必须由 final-wire fixture 冻结，不影响产品模型。

## 10. Runtime 归一化与预检

```text
AgentLegalSequence
  -> NormalizedExecPlan
       pre_map_transactions[]
       tool_actions[]
       terminal_map_transaction?
```

`NormalizedExecPlan` 只是内部执行值，不进入 schema、上下文或持久化。预检顺序：

1. decode 唯一合法 sequence branch；
2. 将 Agent 声明的 pre-Map 动作应用到候选 Map；
3. 根据 Agent 的 Tool 节点声明机械归一化 `ready -> in_flight`；
4. 用 resulting candidate Map 校验全部 Tool 归属和 client Tool 参数；
5. 对 Provider Tool 与顶层已执行事实做逐项对账；
6. 应用该 branch 允许的 terminal Map operation；
7. 校验完整候选 Map 与单 Patch 等全局硬规则；
8. 通过后复用现有持久化、Router、Action settlement 和 outer result 链。

Provider Tool 的执行事实不可回滚；若 Exec 序列或绑定非法，Runtime 不重执行也不默认归属，保留 Provider 原始
事实作为失败证据，但不将其结算到 canonical Map。

## 11. Feedback

- 结构错误只返回未知/缺失 sequence type 或该 type 不允许的字段；
- 动态错误返回 sequence type、字段位置、节点状态和违反的 DAG/Tool 硬规则；
- 不推荐下一步、不把非法序列改写为另一合法序列；
- client Tool 保留原生结果，Provider Tool 保留原始执行事实，Map read 返回完整 Map；
- 不增加 TaskSpace 语义摘要或二次解释。

## 12. 不采用的方向

| 方向 | 不采用原因 |
|---|---|
| 保留任意 `calls[]`，只增加文字规则 | 已证明合法组合不能稳定进入动作生成 |
| 为七个场景创建七套重复的 Map 修改语义 | 会重复 `update_map`，增加概念和 schema 体积 |
| 删除纯 `update_map` | 会让 Map 独立修改被迫捆绑 Tool，改变 Agent 对记账本的支配权 |
| 单独 `hosted_work[]` / `hosted_bindings[]` | 把 Provider Tool 变成序列外例外，与统一 Tool 工作模型冲突 |
| Runtime 自动选 sequence、节点或 Map 变更 | 替 Agent 做语义决策，越过责任边界 |
| 保留 `blocked` 但等待未来使用 | 当前无收益证据，却会持续增加状态、转移、schema、预检和投影分支 |

## 13. 实施验收

1. Agent-visible schema 只能选择 L1～L8，不存在 generic/raw/custom 逃生分支。
2. 七个核心场景全部有上表中的证据关联；没有证据的新场景不进入 schema。
3. `update_map` 保持一份 canonical 合同，同时被 L3/L4/L5/L7 复用，不保留多套高层改名版本。
4. client 与 Provider Tool 使用同一 Agent-visible Tool action 槽位；不存在序列外 Hosted 字段。
5. Provider Tool 只对账、绑定和记录，绝不重复执行；漏绑/错绑仍 fail closed。
6. Agent 在 Ready 节点上声明 Tool 后，该节点机械进入 InFlight；Tool outcome 不自动 completion。
7. Node schema、Map transaction、projection、Store、CLI/Viewer、feedback 和 tests 中不再存在 TaskSpace `blocked`。
8. 普通 Tool 原生 input、Router、权限、sandbox、hook 和结果语义不变。
9. Standard final wire 逐字不变；TaskSpace declaration 变更通过缓存敏感面门禁后才申请真实预算。
10. 旧 `RawPlan.calls`、`hosted_bindings`、blocked 规则和无消费者 helper 直接删除，不做兼容、双写或 migration。

## 14. 参考依据

1. [DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls/)：Function Tool、strict schema、`anyOf` 与可复用 definition。
2. [DeepSeek Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion)：Tool call 参数仍需 Runtime 校验。
3. [JSON Schema composition](https://json-schema.org/understanding-json-schema/reference/combining)：`anyOf`/`oneOf` 的结构语义。
4. [OpenAI Structured Outputs](https://openai.com/index/introducing-structured-outputs-in-the-api/)：结构受约束不等于任务语义由 Runtime 判断。
