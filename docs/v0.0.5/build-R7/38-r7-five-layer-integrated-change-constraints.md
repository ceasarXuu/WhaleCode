# R7 五层架构整体变更约束

- Created: 2026-07-24
- Updated: 2026-07-27
- Version: 1.4
- Status: Active change gate
- Machine contract:
  [`five-layer-integrated-change-constraints-v1.json`](../../../benchmarks/taskspace/r7/five-layer-integrated-change-constraints-v1.json)
- Architecture source:
  [`23-r7-taskspace-five-layer-architecture-design.md`](23-r7-taskspace-five-layer-architecture-design.md)
- Current L4 redesign:
  [`42-r7.1-a2-nonterminal-action-ownership-design.md`](42-r7.1-a2-nonterminal-action-ownership-design.md)

## 1. 目的

此前若干修复只验证了一个局部目标，造成“解决 A、重新引入 B；再解决 B、又恢复 A”的往复。本合同把五层架构
原则、已经关闭的回归和当前未关闭问题放入同一个准入面。后续任何候选必须同时满足全部硬约束，不能用局部收益
抵消既有能力，也不能把历史已关闭问题重新解释为可接受的设计特征。

本合同是变更治理合同，不是新的语义层。它不进入 Agent provider context，不增加提示词，不参与 Runtime 决策，
只约束 WhaleCode 开发、评审、测试和发布。

## 2. 五层职责

| 层 | 唯一职责 | 必须保留 | 明确禁止 |
|---|---|---|---|
| L1 Base Instructions | 通用工程框架、TaskSpace 价值、Map 宏观模型和 Agent/Runtime 边界 | Codex 成熟工作框架；每个 profile 仅一份完整 Base | Tool 字段、JSON 示例、动态 Map 状态、逐 action 时序 |
| L2 Core Working Protocol | 普通任务都需要的 Map 工作循环、基础恢复和反模式 | 初始化后工作、真实边界同步、依据事实恢复 | 第二份 Base、参数全集、动态事实、高级 playbook |
| L3 Advanced Skills | 复杂 DAG、重规划、长任务和证据冲突的按需经验 | 内置 Skill 可被 Standard 与 TaskSpace 正常发现；正文按需加载 | 成为正确性前提、Runtime 自动加载、固定进入每个请求 |
| L4 Tool Contract | 普通 Tool 的原生能力合同与唯一 `taskspace_control` 的 Map action 合同 | schema 明确、静态、可解析；普通 Tool 不携带 TaskSpace 编排字段 | 教授规划、推断任务语义、按 Map 状态动态改 schema、把 response grammar 复制到普通 Tool |
| L5 Map Store, Runtime and Factual Feedback | 独立持久化 canonical Map、响应级 action manifest 硬门、原子提交、忠实结果和纯 projection | 单一事实、Agent 显式动作归属、失败零提交、原始原因、明确 revision/commit | 以 Session/rollout 代替 Map Store、维护 current node、代选节点、补参数、自动初始化、语义建议、读取普通 Tool 内容判断序列 |

Agent 是唯一语义决策者，不是第六层。Provider Context Composer 只是无语义传输设施，也不是第六层。

## 3. 不可互相牺牲的架构原则

### C-01 单一语义所有权

同一规则只有一个权威层。其他层只能引用其身份，不得复制后换一种表述再次解释。产品 artifact 冲突必须在发布前
失败，Composer 不扫描用户输入或 Tool result 做自然语言裁决。

### C-02 Agent 负责语义，Runtime 只守底线

任务拆解、目标、依赖、行动归属、完成判断和总结由 Agent 决定。Runtime 只能验证图结构、revision、派生
readiness、Agent 声明的 node/action reservation、terminal、事务原子性和明确的调用顺序。

### C-03 语义忠实透传

Tool result、control result、失败、commit 状态和 projection 不得丢失、残缺、扭曲、重复包装或注入建议。出现
Agent 异常时，先审查 provider payload、Tool feedback 和 projection，再讨论 Agent 能力或增加约束。

### C-04 内容按稳定性与必要性分层

所有请求都需要的宏观框架放 L1；所有普通 TaskSpace 任务都需要的方法放 L2；低频经验放 L3；机械合同放 L4；
动态事实放 L5。迁移位置本身不算成本收益，只有删除重复或减少固定内容才算。

### C-05 Tool 领域语义准确、策略语义克制

Tool 必须说清能力、成功条件、输入、输出和副作用；普通 Tool 只能描述自身能力，不得携带 TaskSpace
binding、生命周期或 response 编排字段。`taskspace_control` 只描述 Map action，不能承担完整工作方法、提示词
或 Runtime 的事后纠正。

### C-06 immutable capability epoch

同一 profile、provider 能力和可见 Tool 集合形成一个 immutable capability epoch。该 epoch 内 Base、L2 和 Tool
schema 字节固定。Map revision、空/非空状态和 lifecycle transition 不得改变 Tool schema。合法 Tool discovery
只能在请求之间建立带新 hash 的 epoch。

### C-07 三种 projection 策略只在 L5 emission 不同

`map-always`、`map-append`、`map-request` 共享 Base、L2、L3、Tool、Map、Runtime、反馈和状态机。不得为某个
projection policy 建立平行架构或专用行为分支。

### C-08 Map 是唯一 canonical rooted DAG

Root 是唯一起点，Finish 是唯一终点。除 Root 外每个节点至少有一个入边，所有节点从 Root 可达，所有非 Finish
节点能到达 Finish；允许多父依赖。Root 与 Finish 只由 Agent 显式 `finish_map` 闭合；用户反馈后，同一个已关闭
Task Map 只由 Agent 显式 `reopen_map` 恢复。Runtime 维护事实和硬不变量，不设计任务结构。

### C-09 连续动作是必须保留的 L4/L5 合同

初始化、reopen 和所有非终态 Map mutation 必须与至少一个真实普通 Tool 位于同一 provider response。Agent 在唯一
`taskspace_control` action manifest 中为每个 sibling 普通 Tool 声明 `node_id + tool`；Runtime 只做数量、名称、
顺序、图事实、reservation 和 revision 的机械校验、原子提交与原生 dispatch，不读取普通 Tool 业务参数判断动作
意义，也不生成后续动作。

### C-10 Patch 保真与单 Patch

`apply_patch` 保持原生顶层文本输入，不进入通用嵌套 dispatcher。单个 provider response 最多一个 Patch；该约束
不能阻止同一 response 中合法的 lifecycle control 与该 Patch 配对。

### C-11 普通 Tool 保真与 Standard 隔离

Standard 与 TaskSpace 的 shared ordinary Tool schema、业务参数和原生执行路径必须相同。TaskSpace 只额外暴露
中央 `taskspace_control`，并在 L5 对完整 response 执行序列硬门；不得装饰、包装或重解释普通 Tool。内置 Skills
属于共享能力，不能为 benchmark 人为禁用。

### C-12 原子、事实型失败

preflight 失败必须零业务执行、零 Map 提交；业务 Tool 失败不得伪装为 Map 失败；同一失败只生成一份事实结果。
Runtime 不得修改参数后重试或以建议形式重写错误。

### C-13 固定成本与缓存是发布约束

provider 实际收到的完整 `tools`、messages、input、cache hit 和 request 路径都要观测。schema 成本按每请求和整轮
累计报告；不能只看业务成功率，也不能把已知结构放大长期标记为“可接受特征”而不设收敛门。

### C-14 评估必须可归因

每次只改变一个策略。比较臂必须使用同一 commit、binary、image、model、capability set、样本和验证器；简单与复杂
样本都要覆盖。Agent 错误不得从结果中删除，基础设施无效运行必须单独标记。

### C-15 无兼容债务

产品无值得保留的旧 TaskSpace 数据。不得保留旧 wire、静默 fallback、双 parser 或行为兼容分支；迁移必须一次性
更新 schema、parser、测试、日志和文档。

### C-16 机械角色必须由 Tool 结构表达

当 Root、Work、Finish 等机械角色决定解析和硬校验时，L4 必须用 `root`、`work_nodes[]`、`finish` 等不同字段
或不同 schema 分支表达角色；不得为了缩短 schema 把角色抹平成通用集合，再依赖跨字段说明让 Agent 自行维持
互斥、唯一性或成员关系。初始化允许多个首批 Work 节点，不得恢复单数 current/initial work 游标。

### C-17 canonical Map 必须独立持久化

Map 从初始化 commit 起就是独立持久化、全局唯一的数据。Map Store 是 Map、node、edge、action reservation、
result 和 revision
的唯一事实源；Session、Runtime、resume、fork 和 child agent 只能按身份访问同一份 Map。Runtime 可以维护按
revision 校验、随时可丢弃的缓存，但不能拥有 authoritative Map 副本。rollout 只记录对话、Tool 结果、审计事件
和 Map revision 引用，不能作为 Map snapshot/delta 的恢复权威，也不能在 Map Store 缺失时静默重建 Map。

### C-18 动作归属必须由 Agent 逐项声明

每个普通 Tool call 的 `node_id` 必须由 Agent 在同一 response 的 action manifest 中显式声明；同一 response 可
推进多个 Ready 节点。Runtime 只解析并建立外层 `TaskSpaceBoundCall`，不得推断、默认、修复或选择动作归属。
原生 ordinary Tool handler 对 `node_id` 完全无感。

### C-19 生命周期只保存不可重复的事实

canonical Map 不持久化 `Open`，也不存在 `current_node/current_binding/next_node`。Store 保存依赖、completion、
block、action reservation、result 和 revision；`Waiting/Ready/InFlight/Blocked/Completed` 由这些事实计算。
`open_nodes` 只可作为派生查询或指标，不得成为状态转换或第二事实源。

### C-20 Map 生命周期必须闭合且可继续

Map 生命周期只有三种转换：`initialize_and_execute` 负责不存在到进行中，`finish_map` 负责进行中到已关闭，
`reopen_map` 负责用户反馈后的已关闭到进行中。`finish_map` 必须允许 Agent 在同一终态事务中显式完成最后一批
Work，否则“非终态完成必须带下一动作”会使最后 Work 永远无法闭合。`reopen_map` 必须与新增 Work、edges 和
真实 actions 同 response，不得成为独立空转请求。

### C-21 历史工作事实不可倒退

用户反馈后继续任务时，既有 Work completion、result 和 evidence 保持不变；当前 terminal 移入 terminal history，
Agent 通过新增 Work 表达遗漏或补充工作。不得保留 `rework_node`、删除 completion 或把历史完成节点改回未完成。
Root/Finish 的关闭状态只由当前 terminal 派生，不与 Work completion 混存为第二事实源。

## 4. 历史回归总账

| ID | 曾出现的问题 | 当前不可回退的结论 | 状态 |
|---|---|---|---|
| R-01 | Whale 极简 Base 丢失 Codex 成熟工程框架 | Standard 与 TaskSpace 各使用一份完整、同构的专用 Base | closed |
| R-02 | TaskSpace 只以低显著性散点 developer 文本出现 | L1 讲宏观模型，L2 讲普通工作协议，二者版本化且不重复 | closed |
| R-03 | Docker benchmark 禁用内置 Skills | Standard 与 TaskSpace 使用相同内置 Skills 基建 | closed |
| R-04 | L2 恢复协议依赖 Tool result 中不存在的字段 | 所有执行入口统一返回 `TaskSpaceControlResultV2` 可见事实 | closed |
| R-05 | observer 漏报 preflight reject、handler、gate 和真实 commit | 四阶段分别记录，provider 成功与业务成功分开 | closed |
| R-06 | nested lifecycle discriminator 导致 action 选择和解析漂移 | lifecycle 使用直接 action，禁止恢复旧嵌套 transition | closed |
| R-07 | 初始化成功结果未明确报告动作归属 | 初始化结果显式报告 Agent 声明的 action attribution、reservation 和 commit 事实 | closed |
| R-08 | schema 只能描述单个 control，无法保证 top-level sibling | 完整 response 必须经过 Tool 类型、顺序和数量 preflight；连续动作使用原生 sibling Tool calls | closed；实现不得回退到无 preflight |
| R-09 | 普通 Tool 归属缺失时曾静默漂移到错误节点 | 缺失或错位的动作归属必须在零执行 preflight 中失败，不得静默默认、推断或漂移 | closed；旧 current-binding 实现由 R-24 追踪替换 |
| R-10 | 非终态连续动作仍产生稳定拒绝和恢复；A2-C 进一步确认 Tool identity、final revision 和持续 control 遵循问题 | 非终态 boundary 与真实后继动作必须在同一 response；manifest 使用 provider-visible Tool identity；完整 response 执行后的 canonical revision 必须以唯一权威值忠实反馈；真实模型不得形成稳定额外请求 | open；identity 修复生效，但 final receipt 产生 revision 歧义和缓存回归；rerun 277 个 TaskSpace request 中 116 个零执行拒绝 |
| R-11 | 过早或错误终态 action、多个 terminal 分支歧义 | 只有 `finish_map` 负责关闭；Agent 同时声明最后完成的 Work 与 terminal，Runtime 验证 canonical frontier | closed；B2.5 扩充可达性合同 |
| R-12 | Patch 被嵌套 JSON 转义破坏，或同 response 多 Patch 部分写入 | 原生顶层 Patch 保真；单 response 最多一个 Patch | closed |
| R-13 | TaskSpace 装饰曾侵入 Standard 普通 Tool | shared ordinary Tool 在 Standard/TaskSpace 中保持原生 schema 与执行路径一致 | closed；新候选必须消除 TaskSpace 内装饰 |
| R-14 | preflight 与 router 对同一失败重复包装 | 未 dispatch 时只返回一份事实失败 | closed |
| R-15 | provider-native 不可序列化 Tool 混入客户端执行，或不完整 Tool 响应抢占 | 未完整响应不执行、不持久化；不可进入完整 manifest 的能力必须显式分类并在产品决策前暂停 | closed；能力策略继续作为 B0 硬门 |
| R-16 | 完整 lifecycle 联合复制到每个普通 Tool，固定 schema 约 60.7 KB | TaskSpace response grammar 不得复制到普通 Tool；唯一 control 集中拥有 Map action | closed；目标候选删除剩余 binding |
| R-17 | 初始化成为独立 provider request，随后才执行真实 Tool | `initialize_map` 与至少一个原生普通 Tool 必须位于同一 response，并按 barrier 顺序执行 | closed；不要求同一 Tool call |
| R-18 | `string \| object` binding 联合让模型稳定选择短而错误的分支 | provider-visible wire 只保留中央 control 的直接 action；删除普通 Tool binding 联合 | closed；目标候选删除旧联合 |
| R-19 | 初始化图和 binding schema 曾被复制到每个普通 Tool，TaskSpace Tool section 为 46,926 B/request | shared ordinary Tool 与 Standard 字节一致；TaskSpace 固定增量只允许来自唯一 control 和明确能力集差异 | open；A2-C 实测 26,822 B/request，held-out 待验证 |
| R-20 | 为降成本把初始化角色抹平成 `nodes + role ids`，Agent 将 Finish 重复放入 Work 集合并连续初始化失败 | Root、`work_nodes[]`、Finish 在 wire 中保持角色分区；允许多个首批 Work，成本优化不得依赖跨字段自然语言互斥 | closed |
| R-21 | 节点绑定的 TaskSpace 子代理在 child session 启动前从父 rollout 恢复了不一致的 assignment 状态 | child 按 Map 身份访问同一持久化状态；attach 失败原子回收，原失败与相邻 handoff 测试通过 | closed（R7.1-A1） |
| R-22 | 复杂样本仍产生多 Patch sibling reject；A2-C 在 18 次 TaskSpace run 中观测到 2 次 | 晋升证据不得保留重复 multi-Patch 协议拒绝或事后补账路径；L2/L4 明确每 response 最多一个 Patch；保持单 Patch 原子安全且不让 Runtime 代替 Agent 推进 Map | open；rerun 降为 1 次但未达到零，仍需逐 request 因果审计 |
| R-23 | canonical Map 被 Session-local Runtime 持有，并依赖 rollout checkpoint/delta 重建 | 独立持久化 Map Store 已成为唯一事实源；Session/Runtime 只持有引用或可丢弃缓存，rollout 不承担 Map 恢复 | closed（R7.1-A0） |
| R-24 | singleton `current_node/current_binding/main lease` 把多活跃节点 DAG 退化为 Runtime 驱动的线性游标 | Agent 为每个普通动作显式声明 `node_id`；Runtime 不维护 current/next，不代选节点，同一 response 可推进多个节点 | closed（R7.1-A2-B1X） |
| R-25 | `Open` 作为持久化状态与依赖、阻塞、执行中和完成事实重复，产生双重状态源 | Store 只保存不可重复事实；`Waiting/Ready/InFlight/Blocked/Completed/open_nodes` 全部由事实计算 | closed（R7.1-A2-B1X） |
| R-26 | `execute` 强制完成后携带下一动作，而 `finish_map` 又要求所有 Work 事先完成，导致最后 Work 没有有限合法闭合路径 | `finish_map` 接受 Agent 显式声明的最后 Work，并在一个终态事务中完成 Work、Finish、Root 和总结 | closed；A2-B2.5 verified |
| R-27 | Map 关闭后缺少用户反馈驱动的继续路径，或通过旧 `rework_node` 倒退既有完成事实 | `reopen_map` 恢复同一 Map 并携带新增 Work、edges、actions；旧 terminal 进入历史，旧 Work 事实不变 | closed；A2-B2.5 verified |

## 5. 当前 R-10/R-19/R-22 的整组准入门

R-21、R-23 至 R-27 已关闭，但其 handoff、持久化、无 current、无节点 Open、终态可达和 reopen 结论继续作为不可回退门。任何
实现候选必须一次通过以下全部条件：

1. **职责门**：不把初始化、节点选择或图设计交给 Runtime。
2. **静态门**：空 Map 与已初始化 Map 的同一 capability epoch 使用完全相同 Tool schema。
3. **连续动作门**：初始化和所有非终态 mutation 与至少一个真实 Tool 均在同一 response。
4. **普通 Tool 保真门**：shared ordinary Tool 不包含 TaskSpace 字段，Standard/TaskSpace schema byte-identical。
5. **反馈门**：初始化失败、业务失败、reservation prepare、ordinary result attribution 和完整 response
   执行后的最终 canonical revision 分别忠实返回；不得用中间 revision 冒充下一请求可用 revision。
6. **Patch 门**：顶层原生 Patch、单 Patch 和 control + Patch 合法配对均保持。
7. **Standard 门**：Standard Tool schema 与请求路径字节不变。
8. **projection 门**：三个 TaskSpace policy 的 L1-L4 与 Runtime 行为一致。
9. **成本门**：同 capability set 的 `tools` section 明确低于当前 46,926 bytes/request；shared ordinary Tool
   增量为零，并报告总 Input、缓存和请求数。
10. **行为门**：简单、复杂样本不增加独立初始化、事后补 Map、单独边界 control、multi-patch 或业务失败。
11. **观测门**：schema profile、response manifest、preflight/barrier/dispatch/commit 和 provider wire trace 均可对账。
12. **迁移门**：不保留 `taskspace_binding`、旧初始化 carrier、parser 或 fallback。
13. **角色结构门**：Root、`work_nodes[]`、Finish 由 schema 分区，valid fixture 一次解析，角色重复和旧通用
    `nodes + role ids` wire 被确定性拒绝。
14. **子代理恢复门**：节点绑定子代理 spawn、完成 watcher、resume/fork 按 Map 身份访问同一份持久化状态，
    不出现 `current binding and main lease are inconsistent`，并用 typed telemetry 区分 child attach 与
    session resume。
15. **响应级行为门**：逐 policy 报告 multi-Patch attempts、零 dispatch reject、Map lifecycle cadence 和事后
    补账；自然复杂样本不得稳定重复触发同一协议拒绝后再恢复。
16. **持久化所有权门**：进程重启、resume、fork 和 child agent 均从独立 Map Store 读取同一 `map_id` 与 revision；
    rollout 截断不丢 Map，Map Store 缺失时也不得从 rollout 静默重建。
17. **Agent 动作归属门**：每个 sibling 普通 Tool 都与 action manifest 中 Agent 声明的 `node_id + tool`
    逐项对应；缺失、错位或多余调用零执行零提交。
18. **无 current 门**：生产模型、Runtime、projection、subagent、event 和测试不保留 canonical
    `current_node/current_binding/next_node`、`bind_node`、`complete_then_continue` 或 singleton main lease。
19. **非重复状态门**：生产 Store 不持久化 `Open`；readiness、inflight、blocked、completed 和 open metrics
    都能从同一组事实确定性重建。
20. **终态可达门**：最后 Work 由 Agent 在 `finish_map` 中显式完成；单 Work、多末端 Work 和多父依赖图都能在
    有限请求内关闭，不允许 Runtime 根据 Tool 成功自动完成。
21. **用户反馈继续门**：已关闭 Map 只能通过 `reopen_map + 新 Work + edges + actions` 恢复；同一 `map_id`
    保持，旧 terminal 进入历史，生产 schema/domain 不存在 `rework_node`。

当前总计：`24 closed / 3 open`。R-10、R-19、R-22 仍阻止产品晋升。成本下降不能越过这些阻塞项单独晋升。

A2-C live evidence：24/24 业务成功、18/18 Map terminal，但 104 个 sequence failure request、51 个 state
failure request、0/18 首请求初始化提交。详细数据与根因见
[`46-r7.1-a2-c-repeat3-result.md`](46-r7.1-a2-c-repeat3-result.md)。

A2-C repair evidence：provider/dispatch identity 已拆分；Store-backed response-final receipt 已接入；`mutations`
可省略；completed/blocked ownership 与单 Patch 合同已进入 L2/L4。定向 Rust、Tool schema 和 observer 回归通过，
但同口径 live rerun 未通过：identity mismatch 降为零，三臂仍有 82 个 sequence failure request、34 个 state
failure request、1 次 multi-Patch，首请求初始化提交仅 1/18。独立 developer final receipt 在 DeepSeek wire
上成为中途 system 消息；`map-append` 中紧跟 receipt 的 35/35 个请求缓存均退回约 7K 以下，未紧跟 receipt 的
请求为 1/39。该载体违反 C-03/C-13，不得以“最终 revision 已可见”为由晋升。

Repair trace 深入复审进一步确认：

- 277 个 TaskSpace request 中 116 个被流程或状态硬门零执行拒绝；
- 63 个 `taskspace_control_required` 中 41 个发生在 Map 初始化后，59/63 的下一 response 会携带 control，
  说明错误反馈可理解但跨 response 协议没有稳定形成；
- 17 个 stale 全部使用前一 control prepare revision，尽管 receipt/projection 已提供最终 revision，原缺失已
  转化为两个竞争事实的歧义；
- 14 个 `reservation_invalid` 中 12 个是 waiting 后继节点动作，2 个是 completed-owner；公开反馈只含
  reservation ID，缺少 Runtime 已知的节点状态和机械前置条件。

## 6. 已淘汰方向

| 方向 | 淘汰原因 |
|---|---|
| 按 Map 空/非空动态切换普通 Tool schema | 违反 C-06，破坏 immutable capability epoch 和缓存身份 |
| Runtime 自动创建 Root/Work/Finish 或自动选择节点 | 违反 C-02、C-08，替 Agent 做语义决策 |
| 允许 `taskspace_control.initialize_map` 成为无后继普通 Tool 的独立成功请求 | 回退 R-17，并重新产生初始化 request |
| 在任意普通 Tool 上增加 TaskSpace binding 或 lifecycle 联合 | 编排职责侵入能力层，违反 C-05/C-11 |
| 把原生 Tool 封进通用 nested dispatcher | 回退 R-12，并重新引入解析/转义面 |
| 从普通 Tool 参数、Store current pointer 或启发式规则推断/补全动作归属 | 恢复隐式归属；唯一来源必须是 Agent action manifest |
| 只靠 L1/L2 提示词要求初始化或连续动作 | 缺失 L5 response grammar 硬门，回退 R-08/R-10 |
| 放宽 rooted DAG、revision、action attribution/reservation 或 terminal 硬规则换成功率 | 违反 C-02、C-08、C-12 |
| 在失败反馈中加入下一步动作建议 | 违反 C-03、C-12 |
| 把有不同硬规则的初始化角色压成通用 nodes 集合和 role id 引用 | 违反 C-05、C-16，并重新引入 R-20 |
| 从 Session rollout、checkpoint 或 delta 重建 canonical Map，或保留 Session-local authoritative Map 副本 | 违反 C-01、C-08、C-17，并延续 R-23 |
| 维护 singleton current binding、next node 或 main lease，再由 Runtime 把普通动作归到该节点 | 违反 C-02、C-18，并延续 R-24 |
| 持久化 `Open` 或同时保存可由同一事实推导的多套节点状态 | 违反 C-01、C-19，并延续 R-25 |
| 用 `rework_node` 删除或反转既有 Work completion | 历史事实不可倒退；用户反馈通过 `reopen_map + 新 Work` 表达 |
| 要求最后 Work 先通过单独非终态 mutation 完成，再允许 `finish_map` | 与连续动作门形成不可达终态，延续 R-26 |
| 把每条新用户消息自动解释为 reopen | Runtime 无权判断反馈语义；是否继续同一 Task Map 由 Agent 决定 |

## 7. 变更流程

1. 先在 COE 中写出根因假设和能证伪它的证据。
2. 对候选逐项填写机器合同中的全部 gate，不通过即淘汰。
3. 先做 schema/协议探针，确认固定成本和可生成性，再改生产路径。
4. 代码、parser、schema、日志和测试作为一个 wire 版本原子迁移，不保留兼容。
5. 运行定向单元、合同、Standard 隔离、连续动作、Patch 和生命周期回归。
6. 使用相同 Docker harness 跑 simple/complex 四臂；报告结果、动作、Map、request、input、output、cache 和时间。
7. 只有全部门通过才更新 authority 与生产 manifest；局部通过不得标记完成。
