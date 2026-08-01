# TaskSpace Tool 序列协议产品定义

- Created: 2026-08-01
- Updated: 2026-08-02
- Status: Product baseline confirmed / feasibility validation pending
- Source: 2026-08-01 关于普通 Tool 零侵入、连续动作和顶层序列容器的多轮讨论
- Supersedes: 当前 `taskspace_control.actions[] + sibling Tool calls` 作为目标产品模型的假设
- Does not contain: 工程架构、数据结构、schema、代码路径、迁移或测试计划

## 1. 问题定义

TaskSpace 希望 Agent 在推进任务地图时，把 Map 变化与真正的工作动作组织成一个连续、合法的行动单元。例如：

- 创建 Map 后立即读取代码；
- 完成探索节点后立即在实现节点修改代码；
- 完成实现节点后立即在验证节点运行测试；
- 完成最后工作后明确关闭 Map 并给出总结。

当前产品却让 Agent 分别调用多个顶层 Tool：一个 `taskspace_control` 描述预期动作，若干普通 Tool 作为 sibling
calls 真正执行工作。两边必须依靠名称、数量、顺序和节点编号再次配对。单个 Tool 的合同无法表达“另一个 sibling
必须同时出现”，因此 Agent 可以轻易生成结构完整但整体非法的响应，再由 Runtime 事后整批拒绝。

这不是普通 Tool 能力不足，也不应通过修改每个 Tool、让 Runtime 猜测下一步或不断增加惩罚来修复。缺失的是一个
Agent 可以直接生成的、TaskSpace 专用的顶层行动单位。

## 2. 产品目标

TaskSpace 下，Agent 每次提交的是一个 **Tool 序列**，而不是若干彼此独立的顶层 Tool call。序列完整表达本轮要做的
Map 操作、真实工具动作、顺序和动作归属。Runtime 只接受合法序列，并把其中的每个原生 Tool 交回既有执行能力。

产品目标是：

1. Agent 在生成时就面对完整行动合同，而不是先生成零散调用、再接受事后纠正。
2. 初始化、完成并继续、reopen 并继续等连续动作是一等行为，不需要额外请求补齐。
3. 普通 Tool 保持行业通用的原生能力，不知道 TaskSpace、Map 或节点。
4. `taskspace_control` 也是序列中的普通 Tool，不拥有特殊的顶层协议地位。
5. Runtime 维护不可绕过的机械底线，不替 Agent 拆任务、选节点、补动作或判断工作意义。
6. Standard 保持原来的原生 Tool 调用方式，不为 TaskSpace 付出协议复杂度。

## 3. 产品概念

### 3.1 Tool 序列

Tool 序列是 TaskSpace 下 Agent 唯一的顶层行动载体。它表达“这一轮由哪些 Tool 调用组成，以及它们以什么顺序共同
构成一次合法推进”。

序列本身不承担业务能力，不读取文件、不修改代码，也不修改 Map。它只承载和约束其中的调用。

### 3.2 原生 Tool

原生 Tool 是实际提供能力的 Tool，例如读取文件、执行命令、修改代码、调用 MCP，以及 `taskspace_control`。

原生 Tool 只理解自己的业务参数和结果。它不理解序列、`node_id`、Map 状态或其他 Tool 的存在。

### 3.3 `taskspace_control`

`taskspace_control` 是操作和读取 Map 的原生 Tool。它与读取文件、执行命令等 Tool 使用相同的序列调用地位和通用
执行路径；差别只来自它本身提供的是 Map 能力。

它不再兼任“其他 sibling Tool 的动作清单”。序列已经是顺序和归属的唯一表达，control 内不得再维护一份重复的
Tool 名称、数量和顺序副本。

### 3.4 动作归属

每个真实工作动作归属于哪个节点，由 Agent 显式声明。归属属于序列中的这一次动作，不属于 Tool 的业务参数，也不
属于 Runtime 的隐式状态。

因此：

- 不存在单独的 bind 动作；
- 不存在 current node、默认节点或 Runtime 选定的 next node；
- 同一序列可以包含属于不同未闭合节点的动作；
- 普通 Tool 即使被复用于不同节点，其原生调用仍完全相同。

动作归属在工程上如何编码留到下一阶段，但必须位于原生 Tool 外层，并保持 Agent 声明、Runtime 只解析这一责任关系。

## 4. 已确认的产品规则

### 4.1 TaskSpace 与 Standard

1. Standard 继续允许 Agent 按原生方式调用一个或多个顶层 Tool。
2. TaskSpace 下，Agent 不再直接提交顶层原生 Tool，只提交一个 Tool 序列。
3. 两种模式共用同一份原生 Tool 定义、能力、权限、sandbox、hook、执行器和结果语义。
4. TaskSpace 不能通过给每个普通 Tool 增加 Map 字段来实现序列要求。
5. 不维护两套手写的 Tool 业务协议；原生 Tool 始终是能力定义的唯一事实源。

### 4.2 Agent 的责任

Agent 决定并显式提交：

- 本轮要调用哪些 Tool；
- 调用顺序；
- 哪些动作可以处于同一轮；
- 每个真实工作动作归属于哪个节点；
- 要创建、修改或完成哪些 Map 节点和依赖；
- 何时证据充分、何时关闭 Map，以及最终总结。

序列不能成为 Runtime 的计划。Runtime 不能根据 Map 或 Tool 结果替 Agent 追加序列项。

### 4.3 Runtime 的责任

Runtime 只负责：

- 确认 TaskSpace 请求使用了序列入口；
- 解析 Agent 已声明的序列、顺序和节点归属；
- 在任何真实动作开始前检查结构、revision、DAG、节点状态、归属、单 Patch 等硬规则；
- 非法时拒绝整个序列，不执行部分普通 Tool，也不提交部分 Map 变化；
- 合法时把每个原生调用交给既有 Tool 执行路径；
- 忠实返回每个 Map 操作和原生 Tool 的结果。

Runtime 不负责：

- 判断某个动作对任务是否聪明、充分或高效；
- 推断缺失的节点归属；
- 自动选择当前节点或下一节点；
- 自动初始化、补齐、重排或改写序列；
- 因 Agent 误解反馈而增加语义控制；
- 修改原生 Tool 参数或把失败伪装成其他类型的失败。

### 4.4 `taskspace_control` 的地位

1. `taskspace_control` 必须位于 Tool 序列内部，不能作为序列外的特殊兄弟调用。
2. 它只声明 Map 操作、目标 Map revision 和相关 Map 事实。
3. 它不再通过 `actions[]` 复述同一序列中普通 Tool 的名称、数量、顺序和节点归属。
4. 序列规则可以要求 control 位于某些动作之前或之后，但这只是序列合法性，不使 control 成为特殊执行架构。
5. control 结果与普通 Tool 结果都必须保持各自真实语义，不互相包装或替代。

### 4.5 节点模型

1. Map 是全局唯一、独立持久化的 canonical 有向依赖图，也是状态机本身。
2. Root 是唯一起点，Finish 是唯一终点；除 Root 外节点至少一个入边，允许多父依赖。
3. 多个节点可以同时 Ready 或 InFlight，不存在单一 current node。
4. 每个普通动作的节点归属由 Agent 在该动作外层明确声明。
5. Runtime 只检查声明的节点是否机械可执行，不判断 Agent 选择是否最优。

### 4.6 执行归属与 Provider 托管 Tool

Tool 序列是唯一动作事实，但序列成员的实际执行位置可以不同：

1. Client-managed Tool 继续由现有 `ToolRouter`、handler、hook、权限和 sandbox 路径执行。
2. Provider-hosted Tool 只能由 Runtime 在完整序列预检通过后，通过受控的 provider 执行能力触发。
3. 执行位置是 Runtime 内部事实，不改变 Agent 声明的 Tool 身份、序列位置或节点归属。
4. TaskSpace 主请求不得在序列容器之外同时暴露同一个 provider-hosted Tool；否则 provider 可能在预检前执行动作。
5. 不允许用“序列内声明 + 顶层真实调用”的双写，也不允许根据事后结果反推它属于哪个序列项。
6. 某个 hosted capability 若没有可确定触发、可核验结果的执行能力，则它在 TaskSpace 中不可用；不得静默退回序列外调用。
7. Standard 继续使用 provider 原生 hosted Tool，TaskSpace 的执行归属机制不得改变 Standard 行为。

这意味着“所有 Tool 都在序列内”描述的是 Agent 的唯一行动合同，不要求所有能力都在同一个进程执行。Runtime 只根据
已声明的序列项选择既有 client 执行路径或受控 provider 执行路径，不根据任务语义替 Agent 选择 Tool。

## 5. 期望行动形状

以下是产品语义，不是最终 schema。

### 5.1 初始化并开始工作

```text
Tool 序列(
  创建 Map，
  节点 A：读取代码，
  节点 B：搜索相关测试
)
```

创建空 Map 不能单独占用一轮。Agent 必须同时开始至少一个真实工作动作。

### 5.2 在现有节点继续工作

```text
Tool 序列(
  节点 A：读取文件，
  节点 A：搜索引用，
  节点 B：读取测试
)
```

当 Map 不需要变化时，不应为了形式完整而制造一次空的 control 操作。节点归属由各动作自身的外层声明表达。

### 5.3 完成节点并继续

```text
Tool 序列(
  完成节点 A，
  节点 B：修改代码，
  节点 C：读取验证条件
)
```

非终态完成不是单独停顿点。完成之后必须在同一序列中存在真实后续动作；后续动作可以服务一个或多个可执行节点。

### 5.4 Reopen 并继续

```text
Tool 序列(
  重新打开已关闭的同一个 Map 并增加后续工作，
  新节点 D：调查用户反馈
)
```

Reopen 只由用户的新反馈触发 Agent 决策，但 Runtime 不理解反馈语义。Reopen 必须伴随实际工作，不能成为单独空转。

### 5.5 完成任务

```text
Tool 序列(
  完成最后一批 Work，
  关闭唯一 Finish 与 Root，
  提交最终总结
)
```

终态由 Agent 显式提交，不要求再附带无意义的后续 Tool。成功后该 turn 结束。

### 5.6 事实读取

事实读取同样严格使用 Tool 序列，不存在序列外的直接 Tool 调用。当 Agent 必须先取得 Map 或渐进折叠输出，才能
决定后续动作时，它提交一个只包含事实读取 Tool 的**单项 Tool 序列**：

```text
Tool 序列(
  读取 Map
)
```

“单项”只表示该 Tool 序列只有一个成员，不表示存在第二种调用入口。读取结果返回后，Agent 根据事实决定下一步，
并在下一轮提交新的 Tool 序列。事实读取不引导 Agent，也不自动改变节点。

## 6. 明确禁止的产品形状

- TaskSpace 下绕过序列直接调用顶层原生 Tool；
- 以“事实读取”为理由绕过序列入口直接调用读取 Tool；
- 序列外单独放置 `taskspace_control`；
- 普通 Tool 参数中出现 `node_id`、Map mutation、TaskSpace lifecycle 或 binding；
- control 内再维护一份与序列重复的普通 Tool manifest；
- 单独 bind、current node、next node、默认节点或 Runtime 自动归属；
- 初始化、reopen 或非终态完成后没有任何真实工作动作；
- 为通过协议而执行无业务价值的占位命令；
- Runtime 从自然语言、reasoning、Tool 参数或 Tool 结果推断并补写 Agent 意图；
- 非法序列先执行一部分，再用后置拒绝纠正；
- 为旧 wire、旧 manifest 或旧 Map 数据保留兼容分支；
- 让 Standard 使用 TaskSpace 序列或承担 TaskSpace 的固定上下文成本。

## 7. 反馈与失败的产品底线

本专题暂不规定 Runtime 最终以“一个序列结果”还是“多个原生 Tool 结果”返回，因为该选择本身不决定 TaskSpace
工作模式。无论采用哪种外形，都必须满足：

1. 每个结果能唯一对应 Agent 提交的序列项；
2. 原生 Tool 的成功、失败和内容忠实保留，不被 control 或 Runtime 再解释；
3. Map 接受或拒绝的事实只表达一次，并给出唯一可继续使用的 canonical revision；
4. 一个失败不能同时伪装为 Tool 失败、Map 失败和 developer 提示；
5. 因前序失败而未执行的动作必须明确标记为未执行，不能伪装成已调用失败；
6. 反馈不能建议 Agent 下一步做什么，只报告已经发生或未发生的机械事实。

## 8. 为什么该设计是基建问题

当前若继续逐题修复，会在错误的顶层动作模型上反复增加补偿：

- I03 可能继续依赖提示词或拒绝反馈提高 sibling 配对率；
- I06 可能继续侵入各种普通 Tool 以保证动作过门；
- I01、I02、I05 会围绕重复 control 与普通结果设计更多 carrier；
- I08 会把重复 schema、拒绝重试和额外请求当作不可约成本。

Tool 序列先统一“Agent 到底提交什么”，这些问题的边界才有稳定基础。专题完成后，旧问题可能被解决、合并、重新
表述或保留，但必须重新用新协议的事实盘点，不能沿用旧根因直接实施。

## 9. 讨论如何收敛到当前结论

### 9.1 只增强 Prompt 不足以承担硬合同

Prompt 可以解释 TaskSpace 的价值和工作方法，却无法让一个 Tool call 的结构自动要求另一个 sibling call 必须存在。
真实运行中，Agent 已看到工作协议，仍连续生成单独普通 Tool 和缺少 sibling 的 control。继续增加强调文本会污染上下文，
也仍然只能在模型遵循度上碰运气。

结论：Prompt 负责工作方法，不能作为跨 Tool 顺序的唯一硬保证。

### 9.2 只做 response 后置拒绝能守底线，但不能形成好入口

当前 preflight 正确阻止了未绑定动作和非法 Map 提交，因此硬边界本身没有放松的理由。但 Agent 已经生成了错误形状，
Runtime 才告诉它重来，造成大量零推进请求。

结论：保留执行前硬检查，但 Agent 一开始就应生成与检查对象同形的完整序列。

### 9.3 把 TaskSpace 字段加入每个普通 Tool 会污染能力层

早期方案曾把 binding 或完整 lifecycle 联合投影到普通 Tool。这会使所有 Tool 重复携带 TaskSpace schema，放大固定
上下文，改变 Freeform、MCP 和延迟 Tool 的形状，并让 Standard 与 TaskSpace 产生两套普通 Tool 合同。

结论：节点归属和顺序必须存在于调用外层，普通 Tool 的原生定义不能被修改。

### 9.4 control manifest 与 sibling calls 是双重表达

后续方案把普通 Tool 恢复原生形状，但让 control 的 `actions[]` 复述 sibling Tool 的名称、顺序和节点。它避免了
普通 Tool 侵入，却让 Agent 同时维护“预期清单”和“实际调用”两份结构，任何数量、名称或归属差异都会整批失败。

结论：序列本身应成为唯一动作清单；`taskspace_control` 回归纯 Map Tool。

### 9.5 顶层 Tool 序列是当前产品结论

序列把 Agent 原本的“调用 A、调用 B、调用 C”改为“提交序列 A、B、C”。它没有替 Agent 决策，也没有改变 A、B、
C 的能力，只是让 TaskSpace 的合法行动单位在生成时可见、在执行前可验证。

这条结论同时满足连续动作、普通 Tool 零侵入、Agent 声明节点归属和 Runtime 只守硬边界四个长期约束。

### 9.6 为什么事后记录 Provider Tool 仍不成立

Provider-hosted Tool 在 provider 生成响应期间已经执行。Runtime 收到 `web_search_call` 或
`image_generation_call` 时，只能观察已发生的结果，无法再保证执行前的 Map、节点和序列规则。若另外提交一份顺序
记录，实际调用 `A,B,C` 与记录 `B,C,A` 仍可能不一致；若同时在序列和顶层各写一次，又会恢复双重事实。

结论：TaskSpace 主请求只能暴露序列入口。hosted 动作必须先作为序列项通过整体预检，再由 Runtime 按该唯一序列位置
触发 provider 执行。事后结果只能结算已经获准的同一序列项，不能创建或修正动作归属。

### 9.7 最小工程方向

现有本地 Tool 执行、序列预检、Map reservation、串并行屏障和 provider 通信继续复用。需要补充的最小连接点是：

```text
已预检的序列项
  -> client-managed：现有 ToolRouter
  -> provider-hosted：受控 Hosted Tool Executor
```

该连接点只决定已声明动作在哪里执行，不新增规划、节点选择、重排、重试建议或语义判断。完整工程设计必须先通过
[`01-execution-ownership-mvp-feasibility-plan.md`](01-execution-ownership-mvp-feasibility-plan.md) 的最小可行性测试，
避免在 provider 边界尚未证实时扩建协议。

## 10. 非目标

本专题不试图：

- 让 Runtime 成为工作流编排器；
- 规定 Agent 应如何拆分业务任务；
- 把所有动作强制串行，或让 Runtime 推断依赖；
- 解决 Map 最终超过上下文上限的专用压缩问题；
- 改变 map-always、map-append、map-request 的 projection 产品差异；
- 用 Prompt 替代结构合同，或完全删除 Agent 的 TaskSpace 工作方法说明；
- 在产品定义阶段决定 provider、schema、内部对象、执行调度或反馈 wire；
- 通过兼容旧实现降低迁移成本。

## 11. 后续工程阶段必须回答但当前不预设的事项

以下事项受上述产品规则约束，但仍属于后续工程设计：

1. 如何从唯一原生 Tool 定义生成序列内部可调用的能力，而不形成第二套手写 schema。
2. 如何覆盖 Function、Freeform Patch、Namespace/MCP、ToolSearch 和延迟出现的 Tool。
3. 动作归属在序列项中的最小表达，以及 Map 级 Tool 为什么不需要伪造节点归属。
4. 如何区分声明顺序、执行屏障和可以并行的独立动作，而不让 Runtime 推断业务依赖。
5. 合法序列开始执行后，control 成功、普通 Tool 失败时的事务和 reservation 事实边界。
6. 如何返回序列结果并保持 provider pairing、原生结果保真、唯一 revision 和缓存稳定。
7. 如何确保 Standard 的 provider wire、Tool schema、Tool choice 和执行行为逐字或语义不变。
8. 如何删除旧 manifest、旧 sibling 配对、普通 Tool decoration 和兼容 parser，而不是保留双路径。
9. provider-hosted capability 的受控调用描述如何保持足够明确，同时不复制普通 Tool schema 或形成通用插件框架。

## 12. 产品验收标准

进入工程设计前，需确认：

- Agent 在 TaskSpace 下只面对一个顶层 Tool 序列入口；
- 所有原生 Tool，包括 `taskspace_control`，都位于该序列内部；
- control 不再复制普通动作清单；
- 普通 Tool 对 TaskSpace 完全无感，Standard 行为不变；
- 节点归属由 Agent 对每个动作声明，且不产生单独 bind 或 current node；
- 初始化、完成并继续、reopen 并继续、并行多节点工作和最终关闭都有有限合法路径；
- Runtime 只守机械底线，不生成、修复或优化 Agent 的工作序列；
- 反馈外形可由工程选择，但语义保真、唯一事实和失败边界不可妥协；
- R8 旧问题队列保持暂停，直到本专题实施完成并重新盘点。

## 13. 决策记录

| 主题 | 当前结论 | 状态 |
|---|---|---|
| TaskSpace 顶层入口 | 只允许一个 Tool 序列，而不是多个独立顶层 Tool | 已确认 |
| 序列成员 | 所有 Tool 均在容器内，包括 `taskspace_control` | 已确认 |
| 普通 Tool | 原生定义、参数、handler 和结果对 TaskSpace 无感 | 已确认 |
| control 地位 | 是提供 Map 能力的普通 Tool，不是外置 manifest 或特殊 sibling | 已确认 |
| 节点选择 | Agent 逐动作声明；Runtime 不推断 | 已确认 |
| 节点归属位置 | 属于原生调用外层的序列动作，不进入普通 Tool 参数 | 产品责任已确认，字段设计待后续 |
| current node / bind | 均不存在 | 已确认 |
| 结果外形 | 可以整体返回或逐调用返回 | 工程可选，但受语义底线约束 |
| Standard | 保持原生多 Tool 调用路径，不使用 TaskSpace 序列 | 已确认 |
| 执行归属 | 序列项是唯一事实；Runtime 只在预检后选择既有 client 路径或受控 provider 路径 | 已确认，待工程验证 |
| Provider-hosted Tool | 不在 TaskSpace 主请求中顶层暴露；无确定执行能力时在 TaskSpace 中不可用 | 已确认，待 provider 能力验证 |
| 事后记录 / 双写 | 不作为动作来源，不允许用于补建、猜配或修正序列 | 已确认禁止 |
| 旧问题队列 | 暂停；专题完成后重新盘点 | 已确认 |
