# TaskSpace Exec 产品合同

- Created: 2026-08-05
- Status: Phase B0～B2 verified offline / Phase B3 completed-qualified / Phase B4 ready
- Authority: R8 TaskSpace 顶层动作协议主方案
- Supersedes: 普通 Tool schema 入侵、顶层结构化序列容器、control manifest + sibling calls 作为目标产品模型

## 1. 产品目标

TaskSpace 需要让 Agent 用 Map 组织工作，同时保留模型原生的多 Tool 工作能力。顶层协议必须直接表达两件事：

1. 本轮 Map 操作和工作动作组成合法的 TaskSpace 序列；
2. 每个真实工作动作归属于 Agent 明确声明的节点。

此前让多个顶层 Tool 互相复述或配对，合法性只能在 Agent 生成后被动拒绝。`taskspace_exec` 将 TaskSpace 的行动边界
收进一个模型可调用的 Function Tool，使 Agent 在一个调用中完成序列和归属声明，Runtime 再机械验证和执行。

## 2. Agent 看到的能力

### 2.1 Standard

Standard 保持 Codex 原生行为：Agent 直接看到并调用各个顶层 Tool，TaskSpace 不修改其 schema、Tool choice、结果或
执行路径。

### 2.2 TaskSpace

TaskSpace 的请求面包含：

```text
model-visible tools
  - taskspace_exec                 # Function Call，唯一 client/map 入口
  - provider-hosted capabilities   # provider 必须原生识别和执行的能力
```

`taskspace_exec` 内部说明从本轮实际可用的原生 ToolSpec 机械派生，向 Agent 暴露：

- 普通 Function Tool；
- Freeform Tool，例如 `apply_patch`；
- Namespace/MCP Tool；
- canonical Map 操作：`initialize_map`、`update_map`、`read_map`、`reopen_map`、`finish_map`；
- 延迟加载后实际可用的 client Tool；
- provider-hosted Tool 的结果登记与节点绑定语法，但不伪装为本地可执行 Tool。

普通 client Tool 在 TaskSpace 顶层不再重复暴露。Map 操作从 canonical Action Map transaction 原语直接生成，与 client call
同为 `calls[]` 中的平级 variant；它们不注册旧控制 Tool、不复用旧 handler，也不拥有 Runtime 控制器地位。

## 3. 一次调用表达什么

`taskspace_exec` 的外层是一个结构化 Function Call。静态 schema 固定顶层字段、数组元素、可用 Tool 及各 Tool 原生参数
的合法形状；它不固定一次响应实际调用哪些 Tool、调用数量、数组顺序或节点归属。

每个调用实例完全由 Agent 构造：

- `calls[]` 中实际出现的 Tool、数量、原生参数和数组顺序；
- 每个 client/map call 的节点归属；
- `hosted_bindings[]` 中本响应每项 Provider-hosted 事实的节点归属。

Runtime 在 Agent 响应产生前不生成、不预测、不补全或重排这些实例数据。收到 Function Call 后，Runtime 才解析 Agent
声明，对结构化 schema 之外的 TaskSpace 硬规则执行 preflight，并将通过的 client/map call 机械交给原生 Router。

协议版本、能力快照身份和内部调用传输身份不是 Agent 的工作内容。Runtime 已经持有本次 request 使用的协议和 ToolSpec
快照，并可用 outer `call_id + calls[] index` 生成稳定的内部调用身份，因此最终 Agent-visible 参数不得要求 Agent 回显
`version`、`capability_id` 或 `item_id`。这些机械身份只进入 response-local envelope、日志、持久化关联和 outer result。

结构化 Function Call 必须包含三类事实：

| 类别 | Agent 声明 | Runtime 权限 | 权威执行事实 |
|---|---|---|---|
| Map call | Map operation variant、canonical 参数和序列位置；不声明外层 owner | 调用 canonical transaction validator/commit | canonical Map transaction |
| Client call | 原生 Tool 名、原生输入、`node_id` | 预检后调用原 Router 一次 | 原生 Tool result |
| Hosted binding | 每项 Hosted 动作的非空 `node_ids[]` 与可机械核对的逐项声明 | 从原始 output item 读取真实身份、逐项核对并登记，不执行 | provider 原始 output item |

内部语法不是第二份业务 Tool schema：Tool 名、描述、输入和输出合同都从原 ToolSpec 派生；`node_id` 和序列位置属于
外层 TaskSpace invocation metadata，不能写回普通 Tool 参数。

## 4. 合法序列

`taskspace_exec` 只表达 Map 边界，不建立第二份 Work DAG。普通 Work B、C 的前置关系来自 Map 中的节点依赖；它们在
同一批次中可以并行，也可以按结果依赖拆到后续请求。

合法形状沿用已确认的产品规则：

| 形状 | 用途 | 规则 |
|---|---|---|
| `map-prelude + work` | 初始化、reopen 或先完成前置节点后继续工作 | prelude 必须位于首个 client call 之前 |
| `work list` | 在现有多个可执行节点上推进 | 每个 call 都有 Agent 声明的 `node_id` |
| `work + map-epilogue + next work` | 完成节点并继续独立后续工作 | epilogue 与后续 prelude 的边界必须可机械判定 |
| `work + terminal map` | 完成最后工作并显式关闭 Map | `finish_map` 只能位于终态边界 |
| `read-only work` | 读取事实，等待结果后再决定下一步 | 允许单个读取或多个无结果依赖读取 |
| `hosted_bindings[] + 其他合法形状` | 逐项登记同一 provider response 已完成的 hosted 动作 | 每项可供一个或多个节点使用；声明不改变 client/map 的先后关系 |

以下行为非法：

- `taskspace_exec` 调用自身或递归嵌套；
- client Tool 绕过 `taskspace_exec` 顶层调用；
- client call 缺少 `node_id`、绑定未知节点、绑定 Root/Finish，或把节点写入原生 Tool 参数；
- Hosted 事实缺少节点声明、Provider ID 缺失或同一真实 ID 重复；
- Map 操作出现在不合法边界；
- `read_map` 与任何其他 client/map call 或 Hosted binding 混合；
- 一个 exec 实际提交多个 `apply_patch`；
- Runtime 根据 Tool 内容、结果或自然语言推断节点归属。

## 5. 执行与状态边界

### 5.0 响应级执行边界

`taskspace_exec` 在 Provider 请求中是标准 Function Tool；Provider 返回的 outer Function Call 仍使用原生 `call_id` 与
FunctionCallOutput 配对。它与普通业务 Tool 的差异只发生在 Runtime 接收响应之后：它需要同时看到同一响应内已经完成的
Hosted facts，才能完成 TaskSpace 归属结算。

Runtime 复用现有 `session/turn` 生命周期建立一个仅在当前响应存活的 envelope：

```text
OutputItemDone(Web/Image)       -> 收集原始 Hosted ResponseItem
OutputItemDone(taskspace_exec)  -> 收集唯一 outer Function Call
response.completed(response_id) -> 冻结 envelope 并开始 TaskSpace Exec 处理
```

该 envelope 不是 Map、Session 全局状态或第二份事件存储。它不跨响应、不写入 Agent context、不通过重放重建；完成事件后
直接消费并释放。`response_id` 只作为 Runtime 日志关联字段，不进入 Agent 输入和 Provider fact 的权威身份。

TaskSpace response 只允许一个 outer `taskspace_exec`。Hosted facts 可以是 0～N 项；顶层普通 client Tool 和顶层 Map
operation 均非法。Provider 返回完成前不执行任何尚未发生的 client/map 调用。

### 5.1 Client Tool

Runtime 负责：

1. 解码 Agent 声明；
2. 在副作用发生前检查可判定的结构、Tool 身份、节点、请求关联的 Map revision、序列边界和单 Patch 规则；
3. 将内部调用机械还原为原生 ToolCall；
4. 交给现有 ToolRouter、权限、sandbox、hook 和 handler；
5. 原样收集结果并以对应内部调用身份返回。

Runtime 不负责决定应调用什么、选择哪个节点、补调用、改参数、重试或解释结果。

整个 Exec 的结构、Map、DAG、节点、Tool 参数和 Hosted 声明必须在任何未发生的 client/Map
副作用前一次性完成预检。预检通过后，Runtime 先将候选 Map 与 client action 的 `Pending` 归属持久化；
持久化成功后立即按原生 Tool 并行能力 dispatch，不为 Work 调用额外推导依赖。每个 Tool 结果一旦返回，
立即独立结算该 action outcome，不等待同批其他 Tool。持久化只串行化短事务，不用数据库锁串行化 Tool 执行。

Canonical Map 在存储中按 Map head、Node、Node parents 和归属 Node 的 Action 细粒度持久化。这些表只是
同一 Map 产品模型的物理展开，不是平行事实源；读取时直接组装为同一 canonical Map，不依赖 rollout
或 delta replay 重建。高频 Action outcome 只更新对应行和 Map head revision，不重写整份 Map JSON。

Client Tool 只能归属于 Work node。Ready、InFlight 和 Blocked Work node 可执行；Waiting 与 Completed Work node 不可执行。
Blocked 是 Agent 声明的工作阻碍，不是 Tool 权限冻结，Agent 可以继续调用 Tool 收集事实或解除阻碍。Root 和 Finish 只表达
任务边界，不能承载实际 Tool action。

### 5.2 Provider-hosted Tool

Provider-hosted Tool 由 provider 在响应生成过程中原生执行。它的原始输出是唯一执行事实，不能被
`taskspace_exec` 回滚、重执行或替换。

Agent 在同一响应的 `taskspace_exec` 中为每项 Hosted 动作分别声明非空 `node_ids[]`。同一个 Provider 事实可以同时供多个
节点使用，多个事实也可以指向同一节点；事实本身仍只保存一份。Runtime 使用 provider 原始输出逐项核对：

- 绑定最小单位是一个带独立 Provider 身份的原始 hosted output item，不是整个响应或 Runtime 推断的语义任务组；
- 从每个真实 hosted output item 直接读取唯一 `id/item_id` 和 Tool 类型；
- 将每项 Hosted 事实绑定到该项 Agent 声明的节点；
- 允许同一响应中的不同 Hosted 事实归属不同节点，也允许多项事实归属同一节点；
- 检查声明与事实是否一一对应、节点是否存在、Provider ID 是否缺失或重复；
- 保留 Provider 状态，但不以成功或失败改变节点状态。

Agent 不得回显、复制或另造 Provider 传输身份。`hosted_bindings[]` 按 Provider 原始 `output_index` 排序后的 Hosted item
顺序逐项声明；每项只包含模型可见的 Hosted Tool 名和 Agent 选择的非空 `node_ids[]`。Runtime 使用 `output_index` 恢复顺序，
再核对数量和 Tool 类型，不得用事件完成顺序、URL、结果内容或语义相似度猜配。缺少声明、无法唯一对应、节点非法、
Provider ID/`output_index` 缺失或冲突时，该响应不被 TaskSpace 接受，不能把事实标记为已结算的 `unbound`，也不能
默认写到 Root。
Hosted action 也只能归属于 Work node。其执行已经发生，因此 Tool outcome 不决定节点生命周期；Runtime 仍必须拒绝将
Hosted action 记到 Root、Finish、未知节点或空归属中。

### 5.3 失败与结算矩阵

| 响应事实 | Hosted 事实 | 尚未发生的 client/map | 原因 |
|---|---|---|---|
| 缺少 outer exec 或 outer plan 无法解码 | 保留在 provider 原始响应和诊断日志中，不写 Map | 整批拒绝，零执行、Map 零提交 | 没有可信的 Agent 节点与动作声明 |
| plan 可解码但违反 Agent 合同或 canonical Map 硬规则 | 保留在 provider 原始响应和诊断日志中，不写 Map | 整批拒绝，零执行、Map 零提交 | 不从被拒绝的计划中选择性接受状态或归属声明 |
| plan 合法，全部 Hosted 声明一一对应，节点存在或由合法 prelude 创建 | 用真实 Provider ID 逐项记录为各自节点事件 | 按预检计划执行 | Agent 逐项决定节点，Runtime 机械核验落账 |
| 声明漏项、多项、歧义、节点非法，或 Provider fact 缺失/重复 ID | 保留在 provider 原始响应和诊断日志中，不默认写 Root 或未绑定池 | 整批拒绝，零执行、Map 零提交 | 未完整归属的响应不是可接受的 TaskSpace 事务 |
| Provider fact 状态为 failed/cancelled | 仍按声明节点保存原状态 | 其他合法 client/map 正常执行 | Tool outcome 与节点生命周期正交 |
| client Tool 执行失败 | 已结算 Hosted 事实不回滚 | 原结果按内部调用身份返回，Map 节点状态不自动改变 | 不把已发生的 Provider 事实伪装成同一事务可回滚动作 |

合法计划的 Map prelude、Hosted action-node attribution 和 client action-node declarations 必须先通过 canonical Map
transaction 接缝验证；Hosted 节点由 prelude 创建时，只有 transaction 成功后才能把必要 action 事实写入所选节点的
`actions[]`。Provider 原始结果继续只存在于原生 response、Standard history 和通用持久化路径，Map 不复制结果，也不创建
result/evidence ref。一个 Hosted action 被 Agent 归属到多个节点时，各节点只保存同一真实 action identity 的必要归属事实；
不得为此增加旁路 binding database、默认 Root owner 或未绑定池。

### 5.4 Tool 与节点状态正交

Tool 的成功、失败、进行中或完成不自动改变节点状态。节点完成、阻塞、Map 关闭和 reopen 均只能来自 Agent 的显式
Map operation，并受 canonical Map 规则验证。

### 5.5 Canonical Map 最简模型

Canonical Map 是节点中心的当前工作状态，不是 Tool 历史、结果仓库或需要 Runtime 在顶层 join 的平行账本。产品模型为：

```text
map
  map_id
  root
  work_nodes[]
  finish
  revision                 # Runtime 管理

node
  node_id
  goal
  state
  content
  parents[]                # Agent 直接声明的 node_id
  children[]               # Runtime 机械反算，Agent 始终可见
  actions[]
```

Canonical 写入只保存 `parents[]`；Agent 可见 projection、snapshot、CLI debug 和 Viewer 必须在同一个 Node 中同时展示
`parents[]` 与机械派生的 `children[]`。这不是两份关系：Agent 通过目标节点的 `parents[]` 创建、修改和删除全部依赖，Runtime
只做反向索引，不要求 Agent 把 `A.children=B` 和 `B.parents=A` 重复声明。Map 不再持久化顶层 `edges[]`。

`goal` 说明节点要完成什么，`content` 保存 Agent 认为当前应长期保留的直接语义。Map 不预设 summary、result、evidence、
reason、source 或 handoff condition；这些信息若重要，由 Agent 直接写入 `content`，若只是 Tool 过程则继续留在 Standard
历史。`actions[]` 只承载必要的 action identity、Tool 名、机械 outcome 和节点归属；不得保存完整 Tool 参数或输出，也不得
增加任何 `*_ref`。

Map 必须至少包含一个 Work node。Agent 创建 Work node 时只声明 `node_id`、`goal`、`content` 和 `parents[]`，不声明
初始状态。Runtime 将同批全部新节点放入完整候选 DAG 后，一次性机械推导 Waiting/Ready；推导与数组顺序无关，并覆盖同批
fork、chain 和 join。Root 与 Finish 的生命周期只由明确 Map operation 改变。

节点状态直接表达 waiting、ready、in-flight、blocked 和 completed 等生命周期事实，不再用 completion/block/terminal
子账本间接推导。Tool outcome 不推进节点状态；Root 与 Finish 只能由 Agent 的显式终态操作原子完成，最终说明直接写入
Finish 的 `content`。Runtime 只根据 `parents[]` 检查 DAG 硬规则，并在父节点全部 completed 后机械更新 readiness。

`revision` 只用于乐观并发。Runtime 把 Provider 请求所见 revision 与返回的 `taskspace_exec` 机械关联，成功提交后递增；
Agent 不填写、修改或回显 revision。

本项目没有需要保留的旧 TaskSpace 数据。该结构直接升级 canonical schema 并拒绝旧 shape，不建设 migration、dual-write、
fallback 或兼容读取。

## 6. 反馈合同

- `read_map` 只能作为一次独立 `taskspace_exec` 出现；其结果是完整 Agent-visible Map，并与 projection 共用同一构造器，
  包含所有节点、全局路径、状态、内容、动作及派生 `children[]`。
- 每个内部 client call 返回其原生结果，不做 TaskSpace 语义重写。
- `taskspace_exec` 只汇总调用身份、节点归属、机械校验状态和原生 Tool 结果。
- 同一事实只出现一个 Agent-visible 权威表达；不得再注入 developer factual carrier。
- preflight 拒绝必须指出具体条目、违反的硬规则和零执行范围，不加入下一步建议。
- provider reconciliation 失败必须区分“provider 动作已发生”和“TaskSpace 绑定未成立”。
- 结果裁剪沿用原生 Tool 与上下文底线，不因 TaskSpace 额外摘要、改写或隐藏关键失败。

## 7. 不做什么

- 不修改普通 Tool 的原生 schema、参数或 handler；
- 不保留单独 node bind Tool、current node、Runtime 自动绑定或控制 manifest；
- 不把 exec 内调用顺序当作 Work DAG；
- 不增加顶层 `edges[]`、Map 专属 ref、节点语义分类账本或 parent/child 双写合同；
- 不解析 reasoning 或自然语言来恢复动作；
- 不为旧 wire、旧 parser 或实验数据做兼容；
- 不让旧候选方案与主方案长期双轨运行；
- 不因 Agent 偶发错误增加任务语义判断或惩罚式重试。

## 8. 已确认与待证明

| 项目 | 状态 | 说明 |
|---|---|---|
| Function Call 外层 | 已确认 | DeepSeek 不使用 Codex Freeform wire，TaskSpace 采用 Function Call 外层 |
| 内部 ToolSpec 派生 | 已确认 | 复用 Codex 主线机制，不手写第二份 Tool 合同 |
| Client 原 Router 执行 | 已确认 | 复用现有 ToolRouter/registry/handler/hook |
| 合法序列 + node binding | 已确认 | `taskspace_exec` 仅有的 TaskSpace 新职责 |
| Hosted 原生执行 + 双写核对 | 已确认 | provider 事实不可回滚；Runtime 只核对绑定 |
| 静态 schema + Agent 动态实例 | EX-02 离线通过 | schema 固定合法形状；Agent 决定本次 Tool、数量、参数、顺序和节点归属 |
| 完整批次预检边界 | EX-04 离线通过 | 结构、能力、node 声明、Map/DAG 边界和单 Patch 在 dispatch 前判定；失败只返回机械错误且零副作用 |
| Hosted 稳定 Provider 身份 | A2 部分证据成立 | Runtime 可直接读取 Provider `id/item_id`，不要求 Agent 回显传输身份 |
| Hosted 逐项多节点归属 | Phase A direction-supported / 实施验收后移 | 产品语义和 Runtime 无语义核对离线成立；节点 `actions[]`、完整链路和集成行为由新 Phase B 单元验收 |
| Hosted 多节点持久化 | Phase B 待实施 | 同一真实 action identity 可出现在 Agent 声明的多个节点 `actions[]`；Provider 原始结果不进入 Map，也不创建 ref |
| 最简 canonical Map | Phase B1 已完成 | Node 直接包含 goal/state/content/parents/actions；children 始终可见但机械派生；无顶层 edges、平行 ledger 或 Map 自建 ref |

## 9. 验收标准

1. TaskSpace 请求只顶层暴露 `taskspace_exec` 和 provider 必需的 hosted capability；Standard payload 无变化。
2. 同一静态 schema 支持 Agent 构造可变数量和顺序的调用；Agent 可在一个 `taskspace_exec` 中提交初始化并工作、多个
   独立 work、完成并继续、完成并结束。
3. 每个 client call 的 `node_id` 由 Agent 声明，但原 Tool schema 和 handler 完全不知道 TaskSpace。
4. 非法 client/map 序列在明确边界内零执行、Map 零提交；边界由 TX-04 的可证伪结果冻结。
5. provider 原始 output 的真实 ID 与 Agent 逐项节点声明一一核对；同响应可绑定多个节点，任何缺失、歧义、错配、
   非法节点或身份冲突均整批拒绝且不使用默认 Root/未绑定池降级。
6. Tool 结果完整进入 Agent context 一次；失败语义、节点状态和 provider reconciliation 不互相伪装。
7. 旧入侵、旧容器和 sibling 生产路径已在 Phase B0 删除，后续源码不得恢复兼容分支。
8. 确定性测试、日志、缓存门禁和获批真实样本共同证明正确性；真实样本不以一次成功宣称稳定。
9. 每个 Agent 可见 Node 同时展示 `parents[]` 和 `children[]`；Agent 只声明 parents，Runtime 不推断新关系，只机械生成
   children。Map 序列化和所有消费面不存在顶层 `edges[]`、任何 `*_ref` 或旧生命周期 ledger。
