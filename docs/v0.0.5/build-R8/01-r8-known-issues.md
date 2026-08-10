# R8 已知问题唯一账本

- Created: 2026-07-31
- Updated: 2026-08-10
- Authority: R8 当前问题状态的唯一事实源
- Historical evidence: `docs/v0.0.5/build-R7/47-r7.1-global-issue-register-legacy.md`

> **VA-04A 离线重映射（2026-08-09）**：TaskSpace Exec Phase B4 已完成观测、固定离线门禁和当前源码重映射。
> 此前的 Tool schema 入侵、顶层结构化容器和 sibling 配对路线已封存在
> [`tool-sequence-protocol/`](tool-sequence-protocol/README.md)。I01/I02/I05/I06 的旧根因已由新架构删除，列为静态关闭候选；
> I10 的 Runtime-only capability identity 已完成离线闭环；I03/I04/I07/I08 与 I10 的生产验收必须等待获批的
> Phase B5 真实证据。离线候选不等于问题关闭。

> **VA-02 当前生产证据（2026-08-09）**：首次获批的正式 `map-request` 请求中，模型没有调用顶层
> `taskspace_exec`，而是把内部 `exec_command` 提升为顶层 call；Runtime 在零副作用边界正确拒绝。该事实归入 I03，
> VA-03 已暂停。运行同时暴露 v11 wire producer 与旧 consumer 的 I07 漂移，已由 `cca76e921` 修复并从原始 trace
> 恢复 usage；详见 [`taskspace-exec/24-phase-b5-va02-first-result.md`](taskspace-exec/24-phase-b5-va02-first-result.md)。
> 后续 VA-02R 已参考最新 Codex `exec` 将 outer Tool 操作合同收敛为一份 catalog-owned description，并通过同一示例的
> decoder/preflight 离线验证；真实 Agent 遵循仍待新预算复验，详见
> [`taskspace-exec/25-phase-b5-protocol-authority-repair.md`](taskspace-exec/25-phase-b5-protocol-authority-repair.md)。

> **VA-02 第二轮生产证据（2026-08-10）**：模型已稳定选择顶层 `taskspace_exec`，合法第二响应可初始化
> `root -> inspect -> fix -> verify -> finish` 并原生执行 client Tool；但两轮首响应都在无 Hosted output 时的必填
> `hosted_bindings: []` 邻近位置生成不同的非法 JSON。I03 因首次参数稳定性继续 verifying，VA-03 保持阻断。I07 的
> provider boundary 范围修复已在新 run 中在线结算 2 provider requests、3 local attempts 和完整 usage；request 2+
> cache hit 为 96.20%。详见
> [`taskspace-exec/39-phase-b5-va02-revalidation-result.md`](taskspace-exec/39-phase-b5-va02-revalidation-result.md)。

> **VA-02 零 Hosted 复验（2026-08-10）**：首响应已省略 `hosted_bindings` 并一次完成 Map 初始化与嵌套
> `exec_command`，证明局部合同修复在线成立。第二响应却生成了未声明的顶层 `exec_command`；两次 Provider 请求实际
> 声明的顶层 Tool 均只有 `taskspace_exec + web_search`，因此不是 Runtime 重新暴露普通 Tool。Runtime 在副作用前拒绝
> 符合硬边界。后续对照两次历史 Function Exec 和非法调用来源指纹后，主根因收敛为 TaskSpace 内层 `calls[]` wire 与
> Provider 顶层 Function Call 过于同形：模型把内层 `tool=exec_command` 提升为顶层，并把 wrapper-only `node_id` 扁平写入
> 原生参数。复用 Standard base 是放大因素而非充分根因；同一模型/base/Function outer 使用 JavaScript `tools.*` 内层语法
> 时连续 15 次保持正确 outer `exec`。适配层和 Runtime 没有改写名称，也没有反馈丢失。I03 继续 verifying，VA-03 继续阻断。

> **I03 离线修复（2026-08-10）**：`calls[]` 已破坏性替换为互斥的 `map` / `client` envelope：Map 使用
> `operation + input`，Client 使用 `name + node_id + input`。旧 `tool + arguments` wire 不再兼容，并有负向测试阻止回流。
> 内部 plan、原生 Tool 输入、Router、Map transaction、Hosted binding 和 Standard 均未改变；TaskSpace Exec 70 项测试通过。
> 该结果证明工程修复完整，不等于目标模型在线稳定性已通过；VA-02 仍需另行批准最小复验，VA-03 继续阻断。

> **I03 map-client wire 在线复验（2026-08-10）**：目标模型连续两次正确生成顶层 `taskspace_exec`，分别执行
> `initialize_map + client(exec_command)` 和后续 `client(exec_command)`；旧顶层 client Tool 提升与旧同形 wire 均未复现。
> 第二请求缓存命中 94.69%，Tool shape 保持稳定。运行未完成业务修复的直接原因是批准的两请求上限：Agent 读取完实现与测试后，
> 第三次请求在 Provider 执行前被预算代理以 429 截止，因此没有 patch。I03 的新 wire 在线可用性已通过，但端到端动作闭环仍待新预算；
> VA-03 继续阻断。详见 [`taskspace-exec/39-phase-b5-va02-revalidation-result.md`](taskspace-exec/39-phase-b5-va02-revalidation-result.md)。

TaskSpace Exec Phase B4 已完成正式生产链、可靠 Action 结算、跨层观测、缓存/性能消费和固定离线验收。该结果证明工程
不变量成立，但尚未证明目标 Provider 下的 Agent 行为、三种 projection 的效果和不可约成本；最终关闭仍按
VA-04B 使用 Phase B5 当前 trace 重评。

TaskSpace Exec 与全局问题的处理边界统一记录在
[`taskspace-exec/03-global-issue-prerequisite-review.md`](taskspace-exec/03-global-issue-prerequisite-review.md)：I07 已确认的
请求/usage 双计子问题前置为 TX-00；I10、I06、I01/I02/I05 分别融入新方案对应单元；I03/I04/I08 等生产接入后
重评。该映射不改变本表任何问题状态，也不新增 I07 子问题编号。

## 1. 使用规则

本账本迁移 R7.1 已观测到的问题，不迁移旧根因和旧修复方案。`Source` 只用于追溯历史证据。

新增问题必须满足以下之一：

- 当前源码或确定性测试证明独立缺陷；
- 当前有效 trace 证明新的用户可见或 Agent 可见异常；
- 当前问题深挖后发现无法归入既有问题的独立根因。

不得把一次失败中的多个日志表现重复登记为多个问题，也不得把计划、验收或发布步骤登记为产品问题。

## 2. 影响分层

| 层级 | 责任面 | 该层失败的影响 | 对应问题 |
|---:|---|---|---|
| F0 | canonical Map Store | Runtime 读取到非法或错误事实，所有上层判断失去基础 | I09 |
| F1 | Runtime 事务与 revision | 同一动作出现竞争状态或提交身份，导致 stale、重复提交和错误恢复 | I01 |
| F2 | Tool admission 与 dispatch | TaskSpace 硬约束可被入口绕过，出现未绑定或多 Patch 真实执行 | I06 |
| F3 | Tool feedback 与 context | Agent 收到丢失、重复、歧义事实，缓存前缀也可能被破坏 | I05、I02 |
| F4 | capability 与观测身份 | 可见能力和证据边界不稳定，行为与成本结论无法准确归因 | I10、I07 |
| F5 | Agent 协议行为 | Agent 生成低效或非法动作，但底层仍应守住正确性 | I03、I04 |
| F6 | 成本与晋升 | 衡量修复后的不可约产品成本，不能反向决定底层语义 | I08 |

## 3. 当前全集与优先级

`P0` 表示 canonical 正确性或不可绕过边界；`P1` 表示语义、能力身份或证据可信性；`P2` 表示 Agent 行为；
`P3` 表示修复后的成本和发布验收。执行序优先处理更底层责任面。

本表只描述产品问题，不在问题名称中预设技术根因。具体机制、证据和修复方案进入各问题专项文档。

| 执行序 | ID | 层级 | 严重度 | 产品问题 | 产品应有表现 | VA-04A 离线结论 | 状态 | Source |
|---:|---|---:|---:|---|---|---|---|---|
| 1 | R8-I09 | F0 | P0 | 恢复旧任务时可能接受内部关系损坏的任务地图 | 只恢复结构完整的地图；损坏时停止且不改变当前事实 | 当前关系 Store、hydrate 校验和 State 回归继续成立 | [closed](I09/01-i09-store-hydrate-repair-result.md) | GI-009 |
| 2 | R8-I01 | F1 | P0 | 一轮工作后 Agent 可能收到互相竞争的新旧进度 | 每轮只有一个可继续使用的结果，revision 不成为 Agent 填表负担 | 旧 receipt/control 双版本链已删除；Exec 只返回一个 outer 结果，request revision 由 Runtime 内部维护。静态关闭候选，待 E3 排除 stale 重试 | [verifying](I01/00-i01-response-final-revision-repair-plan.md) | GI-001 |
| 3 | R8-I06 | F2 | P0 | 组合工具内部动作可能绕过归属和单 Patch 硬门 | 所有 TaskSpace client 动作先过同一请求级预检，普通 Tool 保持原生 | 生产顶层仅 Exec+Hosted；完整 plan 在副作用前校验，顶层绕过和多 Patch 有确定性拒绝。静态关闭候选 | verifying | GI-006 |
| 4 | R8-I05 | F3 | P1 | 拒绝原因可能重复或混淆临时候选与已保存事实 | 忠实返回一次失败；未提交候选不得表现为已保存状态 | 旧 pairing/developer 双反馈已删除；preflight 拒绝零提交，单一 Tool pairing 返回原始阶段错误。静态关闭候选，待 E3 检查模型可见效果 | verifying | GI-005 |
| 5 | R8-I02 | F3 | P1 | Tool 事实可能被另造高优先级消息重复包装 | 原 Tool/outer Tool 反馈只进入上下文一次，不建立 system/developer 副本 | 旧 carrier 与专属 Event Store 已由 zero-base 删除；Exec 源码不存在额外 developer 注入。静态关闭候选，待 final-wire trace 复核 | verifying | GI-002 |
| 6 | R8-I10 | F4 | P1 | 工具能力变化没有跨执行、缓存和报告共用的身份 | 实际工具集合变化才切换身份，各消费面引用同一值 | 同一 Catalog 快照机械生成 Runtime-only SHA-256，并由 dispatch、request scope、Provider/Exec trace 和性能报告共用；缺失或冲突时报告不可比较。离线实现已验证，待当前生产 trace 验收 | [verifying](I10/00-i10-capability-identity-repair-plan.md) | GI-010 |
| 7 | R8-I07 | F4 | P1 | 观察工具可能漏计、重复计数或使用过期证据 | 请求和失败逐身份计一次；身份不一致时明确不可比较 | 新 wire 运行已直接结算 2 requests、usage、缓存、费用和账本，旧计数漂移未复现；但 section cost unavailable、base-instructions identity unrecognized 仍待收敛 | [verifying](I07/00-i07-observability-trust-repair-plan.md) | GI-007 |
| 8 | R8-I03 | F5 | P2 | Agent 不能稳定组织 Map 与工作动作的同轮提交 | 稳定生成初始化并执行、完成并继续、完成并结束等合法组合 | 新 `map/client` wire 在线连续两次保持 outer Exec，初始化与两个 client Action 均成功；两请求预算在 patch 前截止，尚未验证完成并结束 | [verifying](taskspace-exec/39-phase-b5-va02-revalidation-result.md) | GI-003 |
| 9 | R8-I04 | F5 | P2 | Agent 可能选择依赖未满足或已完成的节点 | Agent 准确使用可执行 frontier；Runtime 只守硬规则 | 当前 DAG/readiness 硬规则确定性通过；是否仍有错误选择只能由 E3 判断 | queued | GI-004 |
| 10 | R8-I08 | F6 | P3 | TaskSpace 的请求、输入、时间和未缓存成本可能高于 Standard | 额外成本可解释、稳定并与产品收益匹配 | 新 wire 单臂第二请求缓存命中 94.69%，证明此前 54.69% 不是当前稳定必现；尚无同 commit 四臂证据，不能评价相对 Standard 成本 | queued | GI-008 |

问题总数：**10**；Open：**9**；Closed：**1**。当前专题：**TaskSpace Exec Phase B5 端到端在线验收**。

## 4. VA-04A 证据边界

| 分类 | 问题 | 当前可下结论 | 当前不能下结论 |
|---|---|---|---|
| 确定性关闭 | I09 | 关系 Store hydrate 仍拒绝非法图，State 134 项通过 | 无 |
| 静态关闭候选 | I01、I02、I05、I06 | 旧根因和旧生产路径为零，新 Exec 的唯一反馈、内部 revision、零副作用预检和不可绕过入口有确定性测试 | 目标模型是否仍产生 stale 重试、误读拒绝或非法组合 |
| 工程完成待生产验收 | I10 | catalog、dispatch、request scope、Provider/Exec trace 和报告共用同一 Runtime-only identity；Standard request 不变 | 当前 Provider trace 是否完整携带且逐 request 一致 |
| 工程修复后待补齐观测 | I07 | 新生产 run 已直接完成 request/usage/cache/cost/ledger 可信结算 | section cost 与 base-instructions identity 为什么仍不可用 |
| 新 wire 在线通过、端到端待验 | I03 | 目标模型连续两次正确生成 outer Exec，初始化与嵌套 client Action 均成功，旧提升未复现 | 允许足够请求后能否继续生成 patch、验证和 finish 组合；通过前不关闭 I03 |
| 行为/成本待验证 | I04、I08 | 当前 DAG 和测量工具已具备验证条件 | 节点选择、三种 projection 成本与业务收益；VA-03 尚未开始 |

本轮 B4 证据为：TaskSpace Exec 57、settlement/recovery 11、State 134、Core 1856/3、CLI 5、Viewer 4、App Server
Protocol 183、workspace、zero-base 和 cache gate 全部通过，详见
[`22-phase-b4-offline-acceptance.md`](taskspace-exec/22-phase-b4-offline-acceptance.md)。OB-01/OB-02 的身份链和报告消费见
[`19-phase-b4-observability-audit.md`](taskspace-exec/19-phase-b4-observability-audit.md) 与
[`21-phase-b4-performance-observer-result.md`](taskspace-exec/21-phase-b4-performance-observer-result.md)。完整重映射结论见
[`23-phase-b4-issue-remap-result.md`](taskspace-exec/23-phase-b4-issue-remap-result.md)。

I10 后续离线补证为 TaskSpace Exec 58、Core 1857/3、workspace、zero-base、性能观察 fixture 和缓存门禁通过，见
[`I10 修复计划与结果`](I10/00-i10-capability-identity-repair-plan.md)。

旧 control/sibling 真实运行、旧 developer carrier 缓存结果和旧请求放大数字只保留在各专题历史文档中，不再作为当前
问题的产品证据。VA-04B 只使用最终生产入口的获批 trace 更新状态。

## 5. 依赖与重评关系

| 上游问题 | 关闭后必须重评 | 原因 |
|---|---|---|
| I09 旧任务恢复可信性 | I01、I04 | 先确保恢复出的任务地图可信，才有资格评价后续进度版本和节点选择 |
| I01/I02/I05/I06 静态候选 | I03、I04、I08 | Phase B5 同一 trace 同时确认旧根因未复现，再评价 Agent 行为与成本 |
| I10 稳定的工具能力版本 | I07、I08 | 性能报告必须能区分“工具变了”和“任务变了” |
| I03 稳定的动作组合 | I04 | 先解决通用动作组织问题，再判断节点顺序错误是否仍是独立问题 |
| I01～I07、I09～I10 | I08 | 成本是最终验收，不作为底层设计的先验优化目标 |

I07 不作为所有问题的整体前置。其 request/usage 双计、local attempt/boundary 对账和 Exec 动作身份已完成离线修复；
后续只负责用当前生产 trace 验收，不再扩展为长期 Observer 专项。

## 6. 已知但不作为独立问题迁移

| 事项 | R8 处理方式 |
|---|---|
| R7.1 的 20 个 Phase | 不迁移；其中包含调查、实现、评测和发布动作，不是问题全集 |
| 五层架构 | 不作为预设答案；相关职责边界按 R8 全局约束重新验证 |
| 三种 projection 的固有差异 | 保留为产品模式，不把已声明差异当成缺陷 |
| 旧 candidate 的完成度与晋升门 | 不迁移；R8 重新建立自己的实现与验收证据 |
| 历史兼容与旧 Map 数据 | 无保留价值，不建立兼容工作 |
| 未证明的“Agent 智能不足” | 不登记；优先检查上下文和 Tool 反馈 |

## 7. 关闭要求

每个问题关闭时必须在本表更新状态，并链接一份问题结果文档。结果文档必须包含：

- 实际根因和被否定的假设；
- 修改与删除的代码路径；
- 确定性测试、日志和回归结果；
- 对 Standard、连续动作、普通 Tool、Map Store、缓存和成本的影响；
- 若使用真实 Agent，关联全局运行账本；
- 对全局约束逐项检查的结论。
