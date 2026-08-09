# R8 TaskSpace 全局约束

- Created: 2026-07-31
- Updated: 2026-08-07
- Status: Active
- Scope: 产品、设计、实现、测试和评测

## 1. 核心产品模型

1. TaskSpace 是自然工作上下文的图化、状态机化再组织，不是替 Agent 工作的编排器。
2. Map 是独立持久化、全局唯一的 canonical 有向依赖图。
3. Root 是唯一起点，Finish 是唯一终点；除 Root 外节点至少声明一个 `parent`，允许多个前置节点。
4. Root 始终保持进行中，直到 Agent 显式 `finish_map`；Finish 是 Agent 手动闭合并总结的明确最终节点。
5. 多个未闭合节点可以同时 Ready 或 InFlight，不存在 current node、next node 或 singleton main node。
6. 状态机是 Agent 必须使用、不可绕过的 Tool。它只维护事实和硬规则，不拥有更高的语义地位。
7. 用户在 Map 关闭后反馈任务未完成时，Agent 通过 `reopen_map` 继续同一个 Map；历史完成事实不倒退。
8. Canonical Map 的最简结构只包含 `map_id`、Root、Work nodes、Finish 和 Runtime 管理的 `revision`；不得另建顶层
   `edges[]`、action/result/evidence/completion/block/terminal ledger 或其他平行事实表。
9. 每个 Agent 可见 Node 必须同时包含 `node_id`、`goal`、`state`、`content`、`parents[]`、`children[]` 和
   `actions[]`。不得在 Tool、projection、snapshot 或 Viewer 中省略 `children[]`。
10. Agent 直接声明每个节点的 `parents[]`，每个元素只有父节点 `node_id`；`children[]` 由 Runtime 对全图
    `parents[]` 做无语义的反向计算，并作为同一 Node 的一等可见字段展示。Agent 决定全部关系，Runtime 不增加、删除或
    猜测关系。
11. Root 的 `parents[]` 必须为空；除 Root 外每个节点至少一个 parent。Finish 的派生 `children[]` 必须为空；除 Finish
    外每个节点至少一个 child。允许多 parent join 和多 child fork，禁止缺失端点、重复关系、自环、环、Root 不可达节点和
    无法到达 Finish 的节点。
12. `goal` 直接表达节点要完成的工作；`content` 由 Agent 直接维护当前应长期保留的节点语义。Map 不预设
    summary、result、evidence、reason、source 或 handoff condition 等语义模块，也不由 Runtime 摘要、分类或解释内容。
13. `actions[]` 只保存节点与真实 Tool action 的必要归属和机械执行事实；完整 Tool 参数、原始输出、长日志、裁剪、压缩和
    大输出保存完全复用 Standard 路径。Map 不建设任何 `*_ref`、冷存储、渐进读取或结果复制机制。
14. `revision` 是 Runtime 管理的乐观并发版本，不是 Agent 工作语义。Runtime 将请求所见 revision 与提交机械关联、成功后
    递增；Agent 不创建、修改或回显 `expected_revision`。
15. Map 至少包含一个 Work node。只有 Work node 可以拥有 `actions[]` 或作为 Tool action 的归属节点；Root 与 Finish
    只表达任务边界和最终闭合，不承载实际 Tool action。
16. Agent 创建 Work node 时不声明初始状态。Runtime 先把同批新节点纳入完整候选 DAG，再基于全部 `parents[]` 一次性
    机械推导 Waiting/Ready；推导结果不得依赖节点在请求数组中的声明顺序，并必须正确处理同批 fork、chain 和 join。

## 2. Agent 与 Runtime 边界

1. Agent 负责目标、拆解、依赖、节点选择、动作归属、完成判断、重规划和总结。
2. Runtime 只验证图结构、请求关联的 revision、节点可执行状态、Agent 声明的动作对应、原子性和底线安全规则。
3. Runtime 不自动初始化、不代选节点、不补动作、不修改参数、不解释任务语义，也不因 Agent 可能犯错增加语义约束。
4. TaskSpace 下 Agent 只通过 Function Call 形态的 `taskspace_exec` 提交 client/map 动作；普通 client Tool 和
   canonical Map operations 都由该工具内部声明，不再作为 TaskSpace 顶层 sibling Tool 暴露。
5. Provider-hosted Tool 保持 provider 原生能力和执行路径；Agent 在同一响应的 `taskspace_exec` 中为每项 Hosted 动作
   分别声明节点归属，Runtime 逐项核对声明与事实的结构错配、漏项、重复和未知引用，但不判断节点的业务语义是否合适，
   也不重执行或猜配。同一响应可归属多个节点。
6. Agent 在 `taskspace_exec` 外层 invocation metadata 中为每个普通 client call 显式声明单个 `node_id`，为每项
   Provider-hosted fact 显式声明非空 `node_ids[]`。Map call 不声明外层 owner，其节点引用只来自对应 canonical Map
   operation 参数。Runtime 只解析和校验，不推断或选择归属，TaskSpace metadata 不得进入普通 Tool 原生参数。
7. `taskspace_exec` 只负责承载合法序列和节点绑定；canonical Map operations 只提供 Map 操作和读取能力，在内部执行路径中
   地位不高于普通 Tool。
8. 普通 Tool 的 schema、参数、handler、权限、sandbox、hook 和原生结果对 TaskSpace 完全无感；内部 Tool 合同必须
   从同一原生 ToolSpec 机械派生，不手写第二套协议。
9. Client Tool 能力合同在一次请求中只能向 Agent 暴露一次：Standard 在顶层暴露原生 Tool schema；TaskSpace
   必须移除顶层普通 client Tool 和独立 Map Tool，改由 `taskspace_exec` 从同一 ToolSpec 快照在内部暴露。禁止
   顶层与内部双重暴露。`taskspace_exec` 自身的 description 是外层调用方式、序列规则和节点归属规则的唯一模型可见
   操作合同；L1/L2、developer message 和其他 context 不得再复制 JSON 形状、合法序列或普通 Tool 参数合同。
10. 内部 Tool catalog、Function/Freeform/Namespace 转换、嵌套调用和原 Router dispatch 必须直接复用或中性抽取 Codex
    `exec/code-mode` 已有基建，TaskSpace 只增加合法序列和节点绑定 metadata。外层 description、内部 schema、Runtime
    catalog 和 dispatch 必须从同一确定性能力快照生成或消费；不得维护平行的手写能力清单。
11. `taskspace_exec` schema 必须是静态能力合同，只能由确定排序的 ToolSpec 能力快照和协议版本机械生成。Map revision、
    node、调用计划、Provider output、Session 状态及其他运行时数据只能进入 Function Call 参数、Tool result、自然上下文或
    canonical Store，严禁写入 Tool schema/description。相同能力集合和协议版本必须生成逐字稳定的 Tool declaration。
12. 静态 schema 只定义 `calls[]`、`hosted_bindings[]` 及各 Tool 参数的合法结构。每次调用中实际使用的 Tool、数量、参数、
    数组顺序和节点归属全部由 Agent 构造；Runtime 在收到调用前不预设这些实例数据，收到后只解析、验证硬规则并机械执行。
13. 协议版本、能力快照身份和内部调用传输身份由 Runtime 从本次 request、outer `call_id` 与数组位置机械维护，不要求
    Agent 回显。它们可以进入内部 envelope、日志和结果关联字段，但不得成为 Agent-visible 必填参数。
14. Tool schema 入侵、独立顶层序列容器和 control manifest + sibling calls 均为封存候选，不得与主方案双轨实现；
   只有主方案被证据否定且用户重新决策后才能恢复评估。
15. Hosted 绑定不是可选记账。任一事实缺少唯一、合法的 Agent 节点声明时，整个 TaskSpace 响应不被接受；原始结果只
    保留为失败证据，不得以 `unbound`、默认 Root owner 或其他默认节点形式进入 canonical Map 后继续推进。Root 与
    Finish 不是合法 action owner。
16. TaskSpace Exec 从 Standard 的原生 ToolSpec、ToolRouter、Provider response lifecycle 和 canonical Action Map
    原语零基础建设。旧 `taskspace_control.actions[] + sibling calls` 的 schema、parser、handler glue、sequence、context、
    response gate、feedback carrier 和测试不得作为过渡层、adapter 或兼容路径保留。
17. 旧 `taskspace_control` 不是新协议的原生合同。新 Map Tool 合同必须从 canonical Action Map 操作重新建立，Map 操作中
    不得包含普通 Tool manifest、sibling 位置、普通 Tool 名复述或外层节点归属；这些只属于 `taskspace_exec`。
18. 重建期间不维持旧 TaskSpace 可运行性，也不以 Standard fallback 冒充 TaskSpace。每个提交只需保持代码库构建与
    Standard 回归成立；TaskSpace 只有在新入口达到对应阶段门禁后才恢复可运行状态。

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
9. Ready、InFlight 和 Blocked Work node 可以承载 client Tool action；Waiting 与 Completed node 不可执行。Blocked
   表示工作遇到阻碍，不剥夺 Agent 调用 Tool 调查或解除阻碍的能力。
10. `read_map` 必须作为独立 `taskspace_exec` 调用出现，不得与 client Tool、Hosted binding 或其他 Map operation 混合；
    返回值必须是完整 Agent-visible Map，包含全局节点路径、状态、内容、动作以及机械派生的 `children[]`。

## 4. 上下文与反馈

1. 语义必须忠实透传，不得丢失、残缺、扭曲、重复包装、注入建议或重新解释。
2. Agent 出现低级错误时，第一优先级检查 provider context、Tool result 和 projection，而不是增加 Runtime 约束。
3. 同一机械事实只有一个 Agent-visible 权威表达。
4. map-request 的自然历史原则上与 Standard 一样持续追加，Map 只在 Agent 请求时读取。
5. map-always、map-append、map-request 只允许在 projection 如何进入 context 上不同，不能拥有不同 Runtime、
   Map、Tool、状态机或反馈实现。
6. projection 必须保留全局路径；局部细节可以按距离和证据效用调整，但不能把旧节点直接变成不可见。
7. 不得为了缓存或 token 指标删除 Agent 完成正确工作所需的事实。
8. Standard 如何保留、裁剪、压缩和持久化 Tool 过程，TaskSpace 就如何处理；Map 只额外保存 Agent 明确写入的节点
   `content` 和必要 action 归属，不增加 TaskSpace 专属 ref 或渐进暴露协议。

## 5. 工程与评测

1. Standard 与 TaskSpace 共用唯一原生 Tool 定义和执行能力；Standard 保持原生顶层多 Tool 调用，TaskSpace 的
   序列入口不得污染 Standard，也不得修改序列内部 Tool 的原生业务合同。
2. 不保留旧 wire、旧 parser、旧状态或旧 Map 数据兼容；错误抽象直接删除。
3. 旧 Map 设计中任何不符合最简模型、已经失去作用或没有生产消费者的 schema、字段、类型、transaction、event/replay、
   projection/detail-fold、snapshot、Store 列、Viewer 字段、测试 fixture 和辅助函数必须连同调用链与测试一起删除。禁止用
   rename、deprecated、legacy 模块、dormant 分支、adapter、fallback、双写或“暂时保留”掩盖残留；未来可能需要不构成
   保留理由。
4. 每次只改变一个主要策略，简单和复杂样本都要检查，不能用聚合均值掩盖异常 run。
5. 统一使用 Docker benchmark，Standard 与 TaskSpace 使用相同模型、Skills、二进制、环境和验证器。
6. 所有功能和修复必须补充可定位成功、失败与原因的日志，并执行相关测试。
7. 未经用户明确授权，单次计划不得执行超过 3 个真实 Whale Agent sample；需要大规模运行时先申请预算。
8. 每次真实运行必须写入 `benchmarks/whale-agent-run-ledger.json`，失败和重试也不得覆盖历史。
9. 成本报告至少包含 request、input、cached/uncached input、output、wall time 和费用。
10. Tool 成本比较必须使用同一能力集合并拆分原有 Tool 合同、TaskSpace metadata 和序列化形式差值。Standard 在顶层
   暴露 Client Tool；TaskSpace 将同一合同迁移到 `taskspace_exec` 内部，并只增加 `node_id`、合法序列、Hosted binding
   和必要容器字段。Provider-hosted Tool 的完整 schema 只保留在 provider 原生顶层，Exec 内仅表达逐项绑定。
11. 单一 `taskspace_exec` 入口沿用 Codex `exec/code-mode` 的 Tool 暴露和嵌套执行形态，并完整保留原生 Tool 名称、
    描述、参数、结果及多 Tool 组合能力。行为测试用于验证具体实现和 Provider 兼容性。
12. 涉及用户体验或重大技术路线时暂停实施，给出源码证据、外部依据和方案代价后由用户决策。
13. Prompt、context、projection、provider payload 或 Tool declaration 的缓存敏感变更必须先被免费指纹门禁阻断；
    Agent 说明变更原因并获得专项预算后，才能运行真实缓存回归。Tool declaration 在能力集合或协议版本发布变更时更新，
    相同版本的运行请求保持逐字稳定。失败结果不得晋升或绕过。

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
- Map 重新出现顶层 `edges[]`、任何 `*_ref`、独立语义分类账本，或要求 Agent 同时双写 parent/child；
- 为旧 Map 代码增加改名、兼容、转接、保留注释或 dormant 分支，而不是删除无效设计及其消费者；
- Standard 路径发生非必要变化；
- Map Store 与 Session/rollout 形成双事实源；
- 一个改动同时改变反馈、Tool schema、projection 和状态机，无法单独归因；
- 缓存改善伴随语义缺失、业务回归或连续动作退化；
- 为通过测试加入针对 sample 内容的特殊规则。
- 为维持旧 TaskSpace 可运行而新增 adapter、fallback、双 schema、双 parser 或条件兼容分支。
