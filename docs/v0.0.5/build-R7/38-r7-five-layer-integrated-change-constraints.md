# R7 五层架构整体变更约束

- Created: 2026-07-24
- Version: 1.0
- Status: Active change gate
- Machine contract:
  [`five-layer-integrated-change-constraints-v1.json`](../../../benchmarks/taskspace/r7/five-layer-integrated-change-constraints-v1.json)
- Architecture source:
  [`23-r7-taskspace-five-layer-architecture-design.md`](23-r7-taskspace-five-layer-architecture-design.md)
- Current L4 repair:
  [`37-r7-lightweight-tool-binding-repair-plan.md`](37-r7-lightweight-tool-binding-repair-plan.md)

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
| L4 Tool Contract | 能力、action 语义、参数、结果、副作用和机械调用形状 | schema 明确、静态、可解析；连续动作可结构表达 | 教授规划、推断任务语义、按 Map 状态动态改 schema |
| L5 Runtime and Factual Feedback | canonical Map、硬规则、原子提交、忠实结果和纯 projection | 单一事实、失败零提交、原始原因、明确 revision/commit | 代选节点、补参数、自动初始化、语义建议、隐藏或改写失败 |

Agent 是唯一语义决策者，不是第六层。Provider Context Composer 只是无语义传输设施，也不是第六层。

## 3. 不可互相牺牲的架构原则

### C-01 单一语义所有权

同一规则只有一个权威层。其他层只能引用其身份，不得复制后换一种表述再次解释。产品 artifact 冲突必须在发布前
失败，Composer 不扫描用户输入或 Tool result 做自然语言裁决。

### C-02 Agent 负责语义，Runtime 只守底线

任务拆解、目标、依赖、行动、完成判断和总结由 Agent 决定。Runtime 只能验证图结构、revision、readiness、
binding、terminal、事务原子性和明确的调用顺序。

### C-03 语义忠实透传

Tool result、control result、失败、commit 状态和 projection 不得丢失、残缺、扭曲、重复包装或注入建议。出现
Agent 异常时，先审查 provider payload、Tool feedback 和 projection，再讨论 Agent 能力或增加约束。

### C-04 内容按稳定性与必要性分层

所有请求都需要的宏观框架放 L1；所有普通 TaskSpace 任务都需要的方法放 L2；低频经验放 L3；机械合同放 L4；
动态事实放 L5。迁移位置本身不算成本收益，只有删除重复或减少固定内容才算。

### C-05 Tool 领域语义准确、策略语义克制

Tool 必须说清能力、成功条件、输入、输出和副作用；不能变成无含义字段表，也不能承担完整工作方法、提示词或
Runtime 的事后纠正。

### C-06 immutable capability epoch

同一 profile、provider 能力和可见 Tool 集合形成一个 immutable capability epoch。该 epoch 内 Base、L2 和 Tool
schema 字节固定。Map revision、空/非空状态和 lifecycle transition 不得改变 Tool schema。合法 Tool discovery
只能在请求之间建立带新 hash 的 epoch。

### C-07 三种 projection 策略只在 L5 emission 不同

`map-always`、`map-append`、`map-request` 共享 Base、L2、L3、Tool、Map、Runtime、反馈和状态机。不得为某个
projection policy 建立平行架构或专用行为分支。

### C-08 Map 是唯一 canonical rooted DAG

Root 是唯一起点并保持 Open，直到 Agent 显式闭合唯一 Finish。除 Root 外每个节点至少有一个入边，所有节点从
Root 可达，所有非 Finish 节点能到达 Finish；允许多父依赖。Runtime 维护事实和硬不变量，不设计任务结构。

### C-09 连续动作是必须保留的 L4/L5 合同

初始化必须与首个真实普通 Tool 同一 provider response；`bind_node` 和 `complete_then_continue` 必须与后继首个
真实 Tool 同一 response。Runtime 只做机械配对和原子提交，不生成后续动作。

### C-10 Patch 保真与单 Patch

`apply_patch` 保持原生顶层文本输入，不进入通用嵌套 dispatcher。单个 provider response 最多一个 Patch；该约束
不能阻止同一 response 中合法的 lifecycle control 与该 Patch 配对。

### C-11 Standard 隔离

TaskSpace 的普通 Tool 装饰、schema、硬门和反馈不得进入 Standard。Standard 的原生 Tool schema、业务参数和执行
路径保持不变；内置 Skills 属于共享能力，不能为 benchmark 人为禁用。

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

## 4. 历史回归总账

| ID | 曾出现的问题 | 当前不可回退的结论 | 状态 |
|---|---|---|---|
| R-01 | Whale 极简 Base 丢失 Codex 成熟工程框架 | Standard 与 TaskSpace 各使用一份完整、同构的专用 Base | closed |
| R-02 | TaskSpace 只以低显著性散点 developer 文本出现 | L1 讲宏观模型，L2 讲普通工作协议，二者版本化且不重复 | closed |
| R-03 | Docker benchmark 禁用内置 Skills | Standard 与 TaskSpace 使用相同内置 Skills 基建 | closed |
| R-04 | L2 恢复协议依赖 Tool result 中不存在的字段 | 所有执行入口统一返回 `TaskSpaceControlResultV2` 可见事实 | closed |
| R-05 | observer 漏报 preflight reject、handler、gate 和真实 commit | 四阶段分别记录，provider 成功与业务成功分开 | closed |
| R-06 | nested lifecycle discriminator 导致 action 选择和解析漂移 | lifecycle 使用直接 action，禁止恢复旧嵌套 transition | closed |
| R-07 | 初始化成功结果未明确报告当前 binding | 初始化结果显式报告 node binding 和 commit 事实 | closed |
| R-08 | schema 只能描述 control 内部，无法保证 top-level sibling | 连续动作由普通 Tool binding 加 response sequence preflight 表达 | closed |
| R-09 | 普通 Tool 未携带 carrier 时静默解释为继续当前节点 | TaskSpace 普通 Tool binding 必填且分支可判别 | closed |
| R-10 | 单独 complete/bind 被后置拒绝，增加请求和恢复 | 边界 control 与 `after_boundary` 真实 Tool 必须同 response | closed |
| R-11 | 过早或错误终态 action、多个 terminal 分支歧义 | 只有 `finish_map`；Agent 提交 terminal，Runtime 验证 canonical frontier | closed |
| R-12 | Patch 被嵌套 JSON 转义破坏，或同 response 多 Patch 部分写入 | 原生顶层 Patch 保真；单 response 最多一个 Patch | closed |
| R-13 | TaskSpace 装饰侵入 Standard 普通 Tool | 只在 TaskSpace provider visibility 阶段装饰 | closed |
| R-14 | preflight 与 router 对同一失败重复包装 | 未 dispatch 时只返回一份事实失败 | closed |
| R-15 | provider-native 不可承载 Tool 混入 TaskSpace，或不完整 Tool 响应抢占 | 不可承载能力隐藏；未完整响应不执行、不持久化 | closed |
| R-16 | 完整 lifecycle 联合复制到每个普通 Tool，固定 schema 约 60.7 KB | 后续 lifecycle 仅在中央 control；普通 Tool 只承载绑定 | closed |
| R-17 | 初始化成为独立 provider request，随后才执行真实 Tool | 初始化对象由首个真实普通 Tool 携带并原子执行 | closed |
| R-18 | `string \| object` binding 联合让模型稳定选择短而错误的分支 | `initialize_map`、`active`、`after_boundary` 使用同形可判别对象 | closed |
| R-19 | 完整初始化图 schema 被复制到每个普通 Tool，固定 schema 约 55.6 KB | 必须在固定 schema 内机械收敛，且不能回退 R-08/R-09/R-17/R-18 | open |

## 5. 当前 R-19 候选的整组准入门

任何实现候选必须一次通过以下全部条件：

1. **职责门**：不把初始化、节点选择或图设计交给 Runtime。
2. **静态门**：空 Map 与已初始化 Map 的同一 capability epoch 使用完全相同 Tool schema。
3. **连续动作门**：初始化与首个真实 Tool 仍为一个 response、一个普通 Tool call。
4. **可判别门**：三个 binding 分支保持对象形态和唯一 action discriminator。
5. **反馈门**：初始化失败、业务失败和 Map commit 事实分别忠实返回。
6. **Patch 门**：顶层原生 Patch、单 Patch 和 control + Patch 合法配对均保持。
7. **Standard 门**：Standard Tool schema 与请求路径字节不变。
8. **projection 门**：三个 TaskSpace policy 的 L1-L4 与 Runtime 行为一致。
9. **成本门**：同 capability set 的 `tools` section 明确低于当前 55,578 bytes/request，并报告总 Input、缓存和请求数。
10. **行为门**：简单、复杂样本不增加独立初始化、事后补 Map、单独边界 control、multi-patch 或业务失败。
11. **观测门**：schema profile、carrier outcome、preflight/dispatch/commit 和 provider wire trace 均可对账。
12. **迁移门**：不保留旧初始化 wire parser 或 fallback。

## 6. 已淘汰方向

| 方向 | 淘汰原因 |
|---|---|
| 按 Map 空/非空动态切换普通 Tool schema | 违反 C-06，破坏 immutable capability epoch 和缓存身份 |
| Runtime 自动创建 Root/Work/Finish 或自动选择节点 | 违反 C-02、C-08，替 Agent 做语义决策 |
| 恢复独立 `taskspace_control.initialize_map` 请求 | 回退 R-17，并重新产生初始化 request |
| 恢复每个普通 Tool 的完整 lifecycle action 联合 | 回退 R-16 |
| 把原生 Tool 封进通用 nested dispatcher | 回退 R-12，并重新引入解析/转义面 |
| 让 binding 可选并用当前状态补默认值 | 回退 R-09 |
| 只靠 L1/L2 提示词要求初始化或连续动作 | 无法提供 L4 结构合同，回退 R-08/R-10 |
| 放宽 rooted DAG、revision、binding 或 terminal 硬规则换成功率 | 违反 C-02、C-08、C-12 |
| 在失败反馈中加入下一步动作建议 | 违反 C-03、C-12 |

## 7. 变更流程

1. 先在 COE 中写出根因假设和能证伪它的证据。
2. 对候选逐项填写机器合同中的全部 gate，不通过即淘汰。
3. 先做 schema/协议探针，确认固定成本和可生成性，再改生产路径。
4. 代码、parser、schema、日志和测试作为一个 wire 版本原子迁移，不保留兼容。
5. 运行定向单元、合同、Standard 隔离、连续动作、Patch 和生命周期回归。
6. 使用相同 Docker harness 跑 simple/complex 四臂；报告结果、动作、Map、request、input、output、cache 和时间。
7. 只有全部门通过才更新 authority 与生产 manifest；局部通过不得标记完成。

