# TaskSpace Exec 产品合同

- Created: 2026-08-05
- Status: Phase A A1 passed / A2 reopened / production integration blocked
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
- `taskspace_control`；
- 延迟加载后实际可用的 client Tool；
- provider-hosted Tool 的结果登记与节点绑定语法，但不伪装为本地可执行 Tool。

普通 client Tool 在 TaskSpace 顶层不再重复暴露。`taskspace_control` 在内部仍是普通 Tool，只因其业务能力是读写 Map
而产生不同结果，不拥有超级 Tool 或 Runtime 控制器地位。

## 3. 一次调用表达什么

`taskspace_exec` 的外层是 Function Call。已确认的首个候选参数是：

```json
{
  "source": "<TaskSpace exec source>"
}
```

`source` 的最终内部语法在工程阶段 TX-03 冻结。无论采用受限 JavaScript 还是等价的声明式子集，它编译出的唯一
Runtime 计划必须包含以下三类事实：

| 类别 | Agent 声明 | Runtime 权限 | 权威执行事实 |
|---|---|---|---|
| Map call | `taskspace_control` 原生参数和序列位置 | 预检后调用原 handler | canonical Map transaction |
| Client call | 原生 Tool 名、原生输入、`node_id` | 预检后调用原 Router 一次 | 原生 Tool result |
| Hosted binding | 每项 Hosted 动作的 `node_id` 与可机械核对的逐项声明 | 从原始 output item 读取真实身份、逐项核对并登记，不执行 | provider 原始 output item |

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
| `hosted_bindings[] + 其他合法形状` | 逐项登记同一 provider response 已完成的 hosted 动作 | 每项可绑定不同节点；声明不改变 client/map 的先后关系 |

以下行为非法：

- `taskspace_exec` 调用自身或递归嵌套；
- client Tool 绕过 `taskspace_exec` 顶层调用；
- client call 缺少 `node_id`、绑定未知节点或把节点写入原生 Tool 参数；
- Hosted 事实缺少节点声明、Provider ID 缺失或同一真实 ID 重复；
- Map 操作出现在不合法边界；
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

TaskSpace response 只允许一个 outer `taskspace_exec`。Hosted facts 可以是 0～N 项；顶层普通 client Tool 和顶层
`taskspace_control` 仍非法。Provider 返回完成前不执行任何尚未发生的 client/map 调用。

### 5.1 Client Tool

Runtime 负责：

1. 解码 Agent 声明；
2. 在副作用发生前检查可判定的结构、Tool 身份、节点、Map revision、序列边界和单 Patch 规则；
3. 将内部调用机械还原为原生 ToolCall；
4. 交给现有 ToolRouter、权限、sandbox、hook 和 handler；
5. 原样收集结果并以对应内部调用身份返回。

Runtime 不负责决定应调用什么、选择哪个节点、补调用、改参数、重试或解释结果。

### 5.2 Provider-hosted Tool

Provider-hosted Tool 由 provider 在响应生成过程中原生执行。它的原始输出是唯一执行事实，不能被
`taskspace_exec` 回滚、重执行或替换。

Agent 在同一响应的 `taskspace_exec` 中为每项 Hosted 动作分别声明 `node_id`。Runtime 使用 provider 原始输出逐项核对：

- 绑定最小单位是一个带独立 Provider 身份的原始 hosted output item，不是整个响应或 Runtime 推断的语义任务组；
- 从每个真实 hosted output item 直接读取唯一 `id/item_id` 和 Tool 类型；
- 将每项 Hosted 事实绑定到该项 Agent 声明的节点；
- 允许同一响应中的不同 Hosted 事实归属不同节点，也允许多项事实归属同一节点；
- 检查声明与事实是否一一对应、节点是否存在、Provider ID 是否缺失或重复；
- 保留 Provider 状态，但不以成功或失败改变节点状态。

Agent 不得回显、复制或另造 Provider 传输身份。Agent 声明与 Provider 事实之间采用何种非语义、可唯一核验的关联键，
仍是 A2 必须验证并冻结的工程合同；不得用 URL、结果内容或语义相似度猜配。缺少声明、无法唯一对应、节点非法、
Provider ID 缺失或冲突时，该响应不被 TaskSpace 接受，不能把事实标记为已结算的 `unbound`，也不能默认写到 Root。
Agent 显式声明 Root 节点时是否合法，仍只由 canonical Map validator 判断；A2 不新增节点业务语义。

### 5.3 失败与结算矩阵

| 响应事实 | Hosted 事实 | 尚未发生的 client/map | 原因 |
|---|---|---|---|
| 缺少 outer exec 或 source 无法解码 | 保留在 provider 原始响应和诊断日志中，不写 Map | 整批拒绝，零执行、Map 零提交 | 没有可信的 Agent 节点与动作声明 |
| plan 可解码但违反 Agent 合同或 canonical Map 硬规则 | 保留在 provider 原始响应和诊断日志中，不写 Map | 整批拒绝，零执行、Map 零提交 | 不从被拒绝的计划中选择性接受状态或归属声明 |
| plan 合法，全部 Hosted 声明一一对应，节点存在或由合法 prelude 创建 | 用真实 Provider ID 逐项记录为各自节点事件 | 按预检计划执行 | Agent 逐项决定节点，Runtime 机械核验落账 |
| 声明漏项、多项、歧义、节点非法，或 Provider fact 缺失/重复 ID | 保留在 provider 原始响应和诊断日志中，不默认写 Root 或未绑定池 | 整批拒绝，零执行、Map 零提交 | 未完整归属的响应不是可接受的 TaskSpace 事务 |
| Provider fact 状态为 failed/cancelled | 仍按声明节点保存原状态 | 其他合法 client/map 正常执行 | Tool outcome 与节点生命周期正交 |
| client Tool 执行失败 | 已结算 Hosted 事实不回滚 | 原结果返回并按原 reservation 结算 | 不把已发生的 Provider 事实伪装成同一事务可回滚动作 |

合法计划的 Map prelude、Hosted 事件 owner 和 client reservation 必须通过现有 canonical Map transaction 接缝一次准备；
Hosted 节点由 prelude 创建时，只有 transaction 成功后才能落为 Node owner。Runtime 不增加独立 binding database，继续
复用 `TaskSpaceEventStore` 的 `provider_item_id + owner` 和现有 Map 持久化路径完成幂等恢复与冲突检查。

### 5.4 Tool 与节点状态正交

Tool 的成功、失败、进行中或完成不自动改变节点状态。节点完成、阻塞、Map 关闭和 reopen 均只能来自 Agent 的显式
`taskspace_control` 操作，并受 canonical Map 规则验证。

## 6. 反馈合同

- 每个内部 client call 返回其原生结果，不做 TaskSpace 语义重写。
- `taskspace_exec` 只汇总调用身份、节点归属、机械校验状态和原始结果引用。
- 同一事实只出现一个 Agent-visible 权威表达；不得再注入 developer factual carrier。
- preflight 拒绝必须指出具体条目、违反的硬规则和零执行范围，不加入下一步建议。
- provider reconciliation 失败必须区分“provider 动作已发生”和“TaskSpace 绑定未成立”。
- 结果裁剪沿用原生 Tool 与上下文底线，不因 TaskSpace 额外摘要、改写或隐藏关键失败。

## 7. 不做什么

- 不修改普通 Tool 的原生 schema、参数或 handler；
- 不保留单独 node bind Tool、current node、Runtime 自动绑定或控制 manifest；
- 不把 exec 内调用顺序当作 Work DAG；
- 不解析 reasoning 或自然语言来恢复动作；
- 不为旧 wire、旧 parser 或实验数据做兼容；
- 不让旧候选方案与主方案长期双轨运行；
- 不因 Agent 偶发错误增加任务语义判断或惩罚式重试。

## 8. 已确认与待证明

| 项目 | 状态 | 说明 |
|---|---|---|
| Function Call 外层 | 已确认 | DeepSeek 不使用 Codex Freeform wire，采用 Function 参数承载 source |
| 内部 ToolSpec 派生 | 已确认 | 复用 Codex 主线机制，不手写第二份 Tool 合同 |
| Client 原 Router 执行 | 已确认 | 复用现有 ToolRouter/registry/handler/hook |
| 合法序列 + node binding | 已确认 | `taskspace_exec` 仅有的 TaskSpace 新职责 |
| Hosted 原生执行 + 双写核对 | 已确认 | provider 事实不可回滚；Runtime 只核对绑定 |
| 内部 source 语法 | A1 离线通过 | `taskspace.plan(<strict JSON>);` 在副作用前生成唯一 typed plan；Agent 生成稳定性仍待获批真实验证 |
| 完整批次预检边界 | A1 离线通过 | 结构、能力、node 声明、Map 边界和单 Patch 在 dispatch 前判定；canonical Map 合法性由后续原 validator 接入 |
| Hosted 稳定 Provider 身份 | A2 部分证据成立 | Runtime 可直接读取 Provider `id/item_id`，不要求 Agent 回显传输身份 |
| Hosted 逐项多节点归属 | A2 未完成 | 必须证明同响应 0～N 项 Hosted 事实可由 Agent 逐项声明并唯一核对；任何不完整归属整批拒绝 |

## 9. 验收标准

1. TaskSpace 请求只顶层暴露 `taskspace_exec` 和 provider 必需的 hosted capability；Standard payload 无变化。
2. Agent 可在一个 `taskspace_exec` 中提交初始化并工作、多个独立 work、完成并继续、完成并结束。
3. 每个 client call 的 `node_id` 由 Agent 声明，但原 Tool schema 和 handler 完全不知道 TaskSpace。
4. 非法 client/map 序列在明确边界内零执行、Map 零提交；边界由 TX-04 的可证伪结果冻结。
5. provider 原始 output 的真实 ID 与 Agent 逐项节点声明一一核对；同响应可绑定多个节点，任何缺失、歧义、错配、
   非法节点或身份冲突均整批拒绝且不使用默认 Root/未绑定池降级。
6. Tool 结果完整进入 Agent context 一次；失败语义、节点状态和 provider reconciliation 不互相伪装。
7. 旧入侵、旧容器和 sibling 生产路径在原子切换后删除，当前源码不保留兼容分支。
8. 确定性测试、日志、缓存门禁和获批真实样本共同证明正确性；真实样本不以一次成功宣称稳定。
