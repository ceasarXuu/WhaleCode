# R8 已知问题唯一账本

- Created: 2026-07-31
- Updated: 2026-08-05
- Authority: R8 当前问题状态的唯一事实源
- Historical evidence: `docs/v0.0.5/build-R7/47-r7.1-global-issue-register-legacy.md`

> **推进暂停（2026-08-05 更新）**：当前发现的问题共同依赖更底层的 TaskSpace 顶层动作承载方式。原执行序暂停，
> 先完成 [`taskspace-exec/`](taskspace-exec/README.md) 主方案。此前的 Tool schema 入侵、顶层结构化容器和 sibling
> 配对路线已降级并封存在 [`tool-sequence-protocol/`](tool-sequence-protocol/README.md)。主方案实施并验证后重新盘点
> 本表；现有问题不得因暂停自动关闭，也不得按旧根因或旧方案继续实施。

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

| 执行序 | ID | 层级 | 严重度 | 产品问题 | 用户或 Agent 的实际影响 | 产品应有表现 | 状态 | Source |
|---:|---|---:|---:|---|---|---|---|---|
| 1 | R8-I09 | F0 | P0 | 恢复旧任务时，产品可能接受一份内部关系已经损坏的任务地图 | Agent 看到的任务、依赖和完成状态不再可信，后续所有推进都可能建立在错误事实之上 | 只恢复结构完整的任务地图；损坏时明确停止，且不改变当前任务或子 Agent 关系 | [closed](I09/01-i09-store-hydrate-repair-result.md) | GI-009 |
| 2 | R8-I01 | F1 | P0 | 一轮工作完成后，Agent 可能同时收到新旧两个任务进度版本 | Agent 容易拿旧版本继续提交，明明刚完成的工作却被判定为过期，引发额外读取和重试 | 每轮推进结束只给 Agent 一个可继续使用的最新任务版本；中间版本不能与最终版本竞争 | [verifying](I01/00-i01-response-final-revision-repair-plan.md) | GI-001 |
| 3 | R8-I06 | F2 | P0 | 通过组合工具间接发起的动作，可能绕过 TaskSpace 对动作归属和修改次数的统一检查 | 某些真实操作可能没有归属任务节点，或在同一轮执行多个代码补丁，破坏可追踪性和原子边界 | 无论动作从哪一层工具发起，都经过同一套请求级硬检查；普通工具本身保持原生 | queued | GI-006 |
| 4 | R8-I05 | F3 | P1 | 动作被拒绝时，Agent 收到的原因可能重复、被包在字符串里，或混淆“尝试中的状态”和“真正保存的状态” | Agent 无法确认哪份状态已经生效，容易重复同一错误或额外读取任务地图自证 | 一次、结构化、忠实地返回拒绝原因，并明确区分已保存状态与失败尝试中的临时状态 | queued | GI-005 |
| 5 | R8-I02 | F3 | P1 | TaskSpace 的工具结果会被再次包装成高优先级上下文消息 | 同一事实重复占用上下文、破坏缓存，还可能让包装内容与原始工具结果产生竞争 | 工具事实通过原工具反馈完整传递一次；不得为了强调而另造 system/developer 消息副本 | queued | GI-002 |
| 6 | R8-I10 | F4 | P1 | 会话中可用工具发生变化时，产品没有一个稳定、可识别的能力版本 | Agent、缓存和性能报告无法判断是任务变化还是工具能力变化，跨请求结果难以可靠比较 | 只有实际可用工具集合变化时才切换能力版本，并让执行、缓存和观测使用同一身份 | queued | GI-010 |
| 7 | R8-I07 | F4 | P1 | 性能观察工具可能漏掉执行记录、把一次失败重复统计，或把过期证据当成当前结果 | 团队可能基于错误报告判断根因、关闭问题或选择版本，测试数字看似完整但不可复算 | 每个请求和失败只计算一次；证据缺失、身份不一致或已过期时明确判为不可比较 | [queued](I07/00-i07-observability-trust-repair-plan.md) | GI-007 |
| 8 | R8-I03 | F5 | P2 | Agent 不能稳定地把任务地图操作和实际工作动作组织在同一轮正确提交 | 初始化、推进或完成任务时频繁触发协议拒绝，本来一次能完成的工作被拆成多次请求 | Agent 能稳定使用少量明确的合法动作组合，如初始化并执行、完成并继续、完成并结束 | queued | GI-003 |
| 9 | R8-I04 | F5 | P2 | Agent 有时会提前执行依赖尚未满足的任务，或继续操作已经完成的节点 | 状态机只能拒绝这些动作，造成无效请求；严重时 Agent 会误判当前可做的工作 | Agent 能准确识别当前可执行节点，并稳定完成“结束前一步后继续下一步”的合法连续动作 | queued | GI-004 |
| 10 | R8-I08 | F6 | P3 | TaskSpace 完成同类任务所需的请求、输入、时间和未缓存成本仍明显高于 Standard | 即使任务质量有收益，也可能因成本过高而缺乏商业可用性 | 在不删减语义和硬约束的前提下，把额外成本收敛到可解释、稳定且与产品收益匹配的范围 | queued | GI-008 |

问题总数：**10**；Open：**9**；Closed：**1**。当前专题：**TaskSpace Exec 主方案**；原问题队列暂停。

I01 暂停前计划：
[`I01/00-i01-response-final-revision-repair-plan.md`](I01/00-i01-response-final-revision-repair-plan.md)。

I01 暂停前进展：

- W0-W8 工程实现和离线验证完成：`3fbfbe6dc`、`ae36f0cbe`、`dbce3402e`、`d46b19479`、
  `9e64a3ddc`、`ad117ce24`、`cb91900c3`、`d2be70030`、`cec426afd`；
- 三种 TaskSpace policy 只保留原 control call 的唯一最终结果，Standard 和普通 Tool final wire 未变；
- 正式缓存快照未自行晋升；W9 真实行为验证和 W10 真实缓存证据仍需分别获得用户预算授权，因此 I01 保持 open。
- W9 的 `map-always` repeat 1 未复现 stale，成功 revision 链为 `2 -> 4 -> 6 -> 8 -> 10`；代码和两层验证均
  通过，但 5 次零执行协议/状态拒绝耗尽请求预算，Agent 未提交 `finish_map`。该运行停止并结算，阻塞归入既有
  I03，详见 [`I01/02-i01-w9-map-always-repeat1-result.md`](I01/02-i01-w9-map-always-repeat1-result.md)。

I03 当前新增证据：

- `map-always` 单次简单样本中先后出现 1 次缺少 control、2 次 control manifest 缺少 sibling、1 次初始化后
  普通动作仍缺少 control，以及 1 次完成节点与动作归属冲突；所有错误均被零执行拒绝，但使 Agent 在业务验证通过后
  仍无法在 10 次上游请求内闭合 Map。

I05/I07 当前新增证据：

- 同一 preflight/state rejection 同时以 Tool pairing output 和 developer factual message 暴露，属于 I05 待收敛的
  重复反馈；rollout observer 将 10 次 completed provider 请求统计为 19 次并近似双计 token，provider boundary
  verifier 又把 1 次本地 pre-dispatch reject 当成 upstream mismatch，属于 I07 待修复的证据口径问题。

I07 最新确定性根因证据（2026-08-05）：

- `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002` 中，provider boundary 与 final-wire 均证明实际完成
  8 次请求，但 Harness `request-summary.json` 和 `metrics.json` 报告 15 次；原始 rollout 恰好是前 7 个请求各有
  2 条带 usage 的 `token_count`，最终请求只有 1 条，即 `7 × 2 + 1 = 15`。
- 每个重复对由一条带唯一 `provider_request_id` 的 response-completed token 事件，和一条不带请求 ID、重复携带
  `last_token_usage` 的 rate-limit 状态事件组成。后者由提交 `e9d705a235` 增加；调用处注释声明应延迟发送以避免
  重复，但 `update_rate_limits()` 实际仍发送了第二条 `TokenCount`。
- Harness `New-TaskspaceRolloutRequestTraceSummary` 当前把每条带 `last_token_usage` 的 `token_count` 都计作请求，
  不检查 `provider_request_id`，也不按请求身份去重。因此 rollout input 被重复累计为 `213,460`，而 sample 聚合
  又用正确累计 input `114,476` 除以错误请求数 15，派生的平均每请求 token 同样失真。
- 现有合成测试只构造“一次请求对应一条 token 事件”，没有覆盖同一次请求同时出现 provider usage 事件与无 ID
  rate-limit 广播的真实形态，因此未能阻止该回归。
- 本轮费用账本不受影响：结算使用 reconciled provider boundary 和 8 个唯一 final-wire terminal，而不是错误的
  Harness 聚合值。完整证据见
  [`WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002.json`](../../../benchmarks/taskspace/r8/evidence/WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002.json)。

I07 独立修复状态（2026-08-05）：

- `I07-W0`～`W8`、`W10` 已完成，基础提交为 `6ad058e10`～`63b0336d3`，对抗性收敛提交截至
  `8acd79b76`，结果见
  [`I07/01-i07-independent-repair-result.md`](I07/01-i07-independent-repair-result.md)；
- 8/15 双计已修正为 8 completed/usage + 7 snapshots，10/11 阶段误判已修正为 11 local attempts +
  10 boundary requests + 1 local-only failure；
- 性能、成本、缓存、freshness 和 provenance 已共用 canonical request facts，严格 mode-map 合同在四条路径一致生效，
  24-run 离线报表通过，最终空白复审无 blocking finding；
- 全局 I07 不关闭，仍等待 TaskSpace Exec item/node 身份接入（W9）和经授权生产验收（W11）。

已关闭问题：
[`I09/00-i09-store-hydrate-repair-plan.md`](I09/00-i09-store-hydrate-repair-plan.md)。
[`I09/01-i09-store-hydrate-repair-result.md`](I09/01-i09-store-hydrate-repair-result.md)。

I09 提交索引：

- 核心修复：`e92241ed6`、`6a31eeb96`
- 生命周期回归：`c7ec19d0b`
- 拒绝日志：`923e8c945`
- 问题关闭与结果归档：`6cd61face`

I02 当前缓存证据：

- [缓存回归门禁首次验证](cache-regression/01-first-validation-result.md)：同一最简样本中 Standard request 2+
  命中率为 96.62%，map-request 为 35.79%；两臂业务均通过，provider usage 覆盖率均为 100%。
- [缓存命中回归门禁子主题](cache-regression/README.md)：门禁自身的覆盖、误报、证据身份和修复计划独立维护，
  不重复增加 R8 产品问题编号。
- [MVT-0 accepted baseline](cache-regression/21-mvt0-accepted-baseline-result.md)：最新同批次 Standard/map-request
  request 2+ 命中率为 97.90%/67.85%，两臂业务与 usage 均通过；一次 state rejection 仍被复制到 control output、
  普通 Tool output 和 developer message，因此 I02/I05 保持 open。

## 4. 依赖与重评关系

| 上游问题 | 关闭后必须重评 | 原因 |
|---|---|---|
| I09 旧任务恢复可信性 | I01、I04 | 先确保恢复出的任务地图可信，才有资格评价后续进度版本和节点选择 |
| I01 唯一最新进度 | I02、I03、I08 | Agent 先能获得唯一最终状态，才能安全删除重复消息并评价动作与成本 |
| I06 所有动作统一过门 | I03、I08 | 先保证任何入口都不能绕过硬规则，行为和成本统计才完整 |
| I05 清晰的拒绝反馈 | I02、I03、I04 | 先让 Agent 准确知道失败事实，再删除副本并评价它是否会正确纠错 |
| I02 工具事实只传递一次 | I03、I04、I08 | 先消除重复上下文和缓存干扰，再评价 Agent 行为与不可约成本 |
| I10 稳定的工具能力版本 | I07、I08 | 性能报告必须能区分“工具变了”和“任务变了” |
| I03 稳定的动作组合 | I04 | 先解决通用动作组织问题，再判断节点顺序错误是否仍是独立问题 |
| I01～I07、I09～I10 | I08 | 成本是最终验收，不作为底层设计的先验优化目标 |

I07 不作为所有问题的整体前置。每个底层问题先建设自身所需的最小、可重算证据；I07 随后只负责收敛跨问题
共用的观测身份和报告口径，避免再次形成长期 Observer 专项。

例外仅限 I07 中已经由同一真实 trace 和当前源码同时证明的 request/usage 双计：它作为 TaskSpace Exec TX-00 在
Phase A 前修复。I07 的 local attempt/boundary 对账由 [专题计划](I07/00-i07-observability-trust-repair-plan.md) 收敛，
新协议 item/node 关联再进入 TX-11；TX-00 通过不关闭完整 I07。

## 5. 已知但不作为独立问题迁移

| 事项 | R8 处理方式 |
|---|---|
| R7.1 的 20 个 Phase | 不迁移；其中包含调查、实现、评测和发布动作，不是问题全集 |
| 五层架构 | 不作为预设答案；相关职责边界按 R8 全局约束重新验证 |
| 三种 projection 的固有差异 | 保留为产品模式，不把已声明差异当成缺陷 |
| 旧 candidate 的完成度与晋升门 | 不迁移；R8 重新建立自己的实现与验收证据 |
| 历史兼容与旧 Map 数据 | 无保留价值，不建立兼容工作 |
| 未证明的“Agent 智能不足” | 不登记；优先检查上下文和 Tool 反馈 |

## 6. 关闭要求

每个问题关闭时必须在本表更新状态，并链接一份问题结果文档。结果文档必须包含：

- 实际根因和被否定的假设；
- 修改与删除的代码路径；
- 确定性测试、日志和回归结果；
- 对 Standard、连续动作、普通 Tool、Map Store、缓存和成本的影响；
- 若使用真实 Agent，关联全局运行账本；
- 对全局约束逐项检查的结论。
