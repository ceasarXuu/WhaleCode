# R8 TaskSpace 全局约束

- Created: 2026-07-31
- Updated: 2026-08-06
- Status: Active
- Scope: 产品、设计、实现、测试和评测

## 1. 核心产品模型

1. TaskSpace 是自然工作上下文的图化、状态机化再组织，不是替 Agent 工作的编排器。
2. Map 是独立持久化、全局唯一的 canonical 有向依赖图。
3. Root 是唯一起点，Finish 是唯一终点；除 Root 外节点至少一个入边，允许多个前置节点。
4. Root 始终保持进行中，直到 Agent 显式 `finish_map`；Finish 是 Agent 手动闭合并总结的明确最终节点。
5. 多个未闭合节点可以同时 Ready 或 InFlight，不存在 current node、next node 或 singleton main node。
6. 状态机是 Agent 必须使用、不可绕过的 Tool。它只维护事实和硬规则，不拥有更高的语义地位。
7. 用户在 Map 关闭后反馈任务未完成时，Agent 通过 `reopen_map` 继续同一个 Map；历史完成事实不倒退。

## 2. Agent 与 Runtime 边界

1. Agent 负责目标、拆解、依赖、节点选择、动作归属、完成判断、重规划和总结。
2. Runtime 只验证图结构、revision、节点可执行状态、Agent 声明的动作对应、原子性和底线安全规则。
3. Runtime 不自动初始化、不代选节点、不补动作、不修改参数、不解释任务语义，也不因 Agent 可能犯错增加语义约束。
4. TaskSpace 下 Agent 只通过 Function Call 形态的 `taskspace_exec` 提交 client/map 动作；普通 client Tool 和
   `taskspace_control` 都由该工具内部声明，不再作为 TaskSpace 顶层 sibling Tool 暴露。
5. Provider-hosted Tool 保持 provider 原生能力和执行路径；Agent 在同一响应的 `taskspace_exec` 中为每项 Hosted 动作
   分别声明节点归属，Runtime 逐项核对声明与事实的结构错配、漏项、重复和未知引用，但不判断节点的业务语义是否合适，
   也不重执行或猜配。同一响应可归属多个节点。
6. Agent 在 `taskspace_exec` 外层 invocation metadata 中为每个 client/provider 动作显式声明 `node_id`；Runtime 只
   解析和校验，不推断或选择归属，`node_id` 不得进入普通 Tool 原生参数。
7. `taskspace_exec` 只负责承载合法序列和节点绑定；`taskspace_control` 只提供 Map 操作和读取能力，在内部执行路径中
   地位不高于普通 Tool。
8. 普通 Tool 的 schema、参数、handler、权限、sandbox、hook 和原生结果对 TaskSpace 完全无感；内部 Tool 合同必须
   从同一原生 ToolSpec 机械派生，不手写第二套协议。
9. Client Tool 能力合同在一次请求中只能向 Agent 暴露一次：Standard 在顶层暴露原生 Tool schema；TaskSpace
   必须移除顶层普通 client Tool 和 `taskspace_control`，改由 `taskspace_exec` 从同一 ToolSpec 快照在内部暴露。禁止
   顶层与内部双重暴露，也禁止在 L1/L2、Tool description 或其他消息中重复完整 Tool 合同。
10. 内部 Tool catalog、Function/Freeform/Namespace 转换、嵌套调用和原 Router dispatch 必须直接复用或中性抽取 Codex
    `exec/code-mode` 已有基建；这些能力不作为 TaskSpace 的第二套协议、额外架构复杂度或模型能力风险重新设计。
11. `taskspace_exec` schema 必须是静态能力合同，只能由确定排序的 ToolSpec 能力快照和协议版本机械生成。Map revision、
    node、调用计划、Provider output、Session 状态及其他运行时数据只能进入 Function Call 参数、Tool result、自然上下文或
    canonical Store，严禁写入 Tool schema/description。相同能力集合和协议版本必须生成逐字稳定的 Tool declaration。
12. Tool schema 入侵、独立顶层序列容器和 control manifest + sibling calls 均为封存候选，不得与主方案双轨实现；
   只有主方案被证据否定且用户重新决策后才能恢复评估。
13. Hosted 绑定不是可选记账。任一事实缺少唯一、合法的 Agent 节点声明时，整个 TaskSpace 响应不被接受；原始结果只
    保留为失败证据，不得以 `unbound`、默认 Root owner 或其他默认节点形式进入 canonical Map 后继续推进。Agent 若
    显式声明 Root 节点，是否合法仍只由 canonical Map 规则判断。

## 3. 动作与状态硬约束

1. 初始化、reopen 和非终态节点完成必须与至少一个真实后续动作位于同一 Agent response。
2. 一个 response 可以包含多个无结果依赖的普通工具动作，并可推进多个节点。
3. Runtime 在已定义的真实动作副作用边界前验证 `taskspace_exec` 计划的成员、Map 边界、节点归属和硬规则；完整批次
   预检边界必须先由 TX-03/TX-04 证明确立，不能用执行后的惩罚式拒绝代替。
4. preflight 失败时整批普通工具零执行、Map 零提交。
5. 普通 Tool 失败保持普通 Tool 失败；Map 拒绝保持 Map 拒绝，二者不得互相伪装。
6. `apply_patch` 作为 `taskspace_exec` 内部成员时仍保持原生 freeform 文本输入形态；一个 response 最多实际执行一个
   Patch。
7. `finish_map` 是 Agent 显式终态事务；它必须能够同时完成最后 Work、Finish、Root 和总结。
8. client 事实读取也必须通过 `taskspace_exec` 提交；存在结果依赖时允许只包含一个读取 Tool 的单项调用，但不存在
   TaskSpace client Tool 的序列外入口。

## 4. 上下文与反馈

1. 语义必须忠实透传，不得丢失、残缺、扭曲、重复包装、注入建议或重新解释。
2. Agent 出现低级错误时，第一优先级检查 provider context、Tool result 和 projection，而不是增加 Runtime 约束。
3. 同一机械事实只有一个 Agent-visible 权威表达。
4. map-request 的自然历史原则上与 Standard 一样持续追加，Map 只在 Agent 请求时读取。
5. map-always、map-append、map-request 只允许在 projection 如何进入 context 上不同，不能拥有不同 Runtime、
   Map、Tool、状态机或反馈实现。
6. projection 必须保留全局路径；局部细节可以按距离和证据效用调整，但不能把旧节点直接变成不可见。
7. 不得为了缓存或 token 指标删除 Agent 完成正确工作所需的事实。

## 5. 工程与评测

1. Standard 与 TaskSpace 共用唯一原生 Tool 定义和执行能力；Standard 保持原生顶层多 Tool 调用，TaskSpace 的
   序列入口不得污染 Standard，也不得修改序列内部 Tool 的原生业务合同。
2. 不保留旧 wire、旧 parser、旧状态或旧 Map 数据兼容；错误抽象直接删除。
3. 每次只改变一个主要策略，简单和复杂样本都要检查，不能用聚合均值掩盖异常 run。
4. 统一使用 Docker benchmark，Standard 与 TaskSpace 使用相同模型、Skills、二进制、环境和验证器。
5. 所有功能和修复必须补充可定位成功、失败与原因的日志，并执行相关测试。
6. 未经用户明确授权，单次计划不得执行超过 3 个真实 Whale Agent sample；需要大规模运行时先申请预算。
7. 每次真实运行必须写入 `benchmarks/whale-agent-run-ledger.json`，失败和重试也不得覆盖历史。
8. 成本报告至少包含 request、input、cached/uncached input、output、wall time 和费用。
9. Tool 成本比较必须使用同一能力集合并区分“原有 Tool 合同”和“TaskSpace 新增 metadata”。Client Tool schema 从
   Standard 顶层迁移到 `taskspace_exec` 内部是替换暴露，不得把整份内部 Tool 合同误计为 TaskSpace 新增 input；只有
   `node_id`、合法序列、Hosted binding 和必要容器字段属于协议增量。Provider-hosted Tool 的完整 schema 只保留在
   provider 原生顶层，Exec 内仅表达逐项绑定，不得复制其完整合同。
10. 单一 `taskspace_exec` 入口是 Codex `exec/code-mode` 已验证的 Tool 暴露与嵌套执行形态，不得预设它会降低 Agent
    对原生 Tool 的理解或多 Tool 能力。行为测试用于发现具体实现缺陷，不得把未发生的模型退化列为方案固有坏处。
11. 涉及用户体验或重大技术路线时暂停实施，给出源码证据、外部依据和方案代价后由用户决策。
12. Prompt、context、projection、provider payload 或 Tool declaration 的缓存敏感变更必须先被免费指纹门禁阻断；
    Agent 说明变更原因并获得专项预算后，才能运行真实缓存回归。该门禁审计的是发布时静态前缀的一次性变化，不得
    据此推导 schema 会随运行时状态变化或形成持续缓存劣化。失败结果不得晋升或绕过。

## 6. R8 不预设的设计

以下内容不是 R8 硬约束，必须在具体问题中重新证明：

- 五层或其他固定层数的 Prompt/Tool/Runtime 架构；
- Final Receipt、factual carrier 或其他额外消息类型；
- 为所有问题服务的统一 observer 大模型；
- 预先固定的完整 Phase DAG；
- 为提高 Agent 遵循度而新增的 Runtime 行为惩罚；
- 任何 projection policy 专属执行分支。

## 7. 每次变更的停止条件

出现以下任一情况立即停止当前实现：

- 为解决 Agent 行为问题增加 Runtime 语义判断；
- 普通 Tool schema 或执行结果被 TaskSpace 装饰；
- 同一 Client Tool 合同同时出现在 TaskSpace 顶层和 `taskspace_exec` 内部，或被多个 Prompt/Tool 层完整复述；
- 同一事实出现第二个权威来源；
- Standard 路径发生非必要变化；
- Map Store 与 Session/rollout 形成双事实源；
- 一个改动同时改变反馈、Tool schema、projection 和状态机，无法单独归因；
- 缓存改善伴随语义缺失、业务回归或连续动作退化；
- 为通过测试加入针对 sample 内容的特殊规则。
