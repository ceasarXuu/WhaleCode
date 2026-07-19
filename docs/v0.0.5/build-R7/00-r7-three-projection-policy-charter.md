# build-R7 三种 Projection 策略共享架构宪章

> R7 不再改变 R6 已建立的 Rooted DAG 状态机，而是把“canonical Map 如何进入 provider
> context”收敛为一个可配置策略点。`map-always`、`map-append`、`map-request` 共用全部
> TaskSpace 基建，差异只能发生在 projection 的触发时机、持久方式和 context 位置。

## 0.1 元数据

```text
Created: 2026-07-17
Updated: 2026-07-18
Version: v0.0.5 build-R7
Status: Charter Frozen / Phase C Complete / Phase D Ready
Owner / Responsible: WhaleCode core runtime / TaskSpace
Related Systems: canonical ActionMap, taskspace_control, Event Store, provider context,
  projection renderer, compaction, replay/resume, Docker benchmark, Web Viewer
Related Links:
  docs/v0.0.5/build-R6/00-r6-rooted-dag-state-machine-charter.md
  docs/v0.0.5/build-R6/01-r6-phased-implementation-plan.md
  docs/v0.0.5/build-R6/16-r6-phase-f-context-cost-plan.md
  docs/v0.0.5/build-R6/17-r6-phase-f-result.md
Risk Level: Critical
Plan Type: Shared architecture / three policy strategies
Change Type: Breaking cutover / no compatibility
R6 Frozen Baseline: e29810158
```

## 0.2 为什么停止 R6、进入 R7

R6 已经完成了正确且可复用的状态机底座：唯一 Root、唯一 Finish、全节点位于 Root 到 Finish
路径上、Agent 显式推进、Runtime 机械校验、事件可回放、终结原子提交。这些不是 R7 要推翻的对象。

R6 后期为了同时获得全局 Map 和严格前缀缓存，引入了固定 `epoch baseline + canonical delta
journal`。它改善了缓存，却也暴露出当前 provider 的线性上下文与动态全景 Map 之间存在不可消除的
产品取舍：

1. 每轮替换最新全景最忠实地保持当前 Map，但会破坏动态位置之后的缓存前缀。
2. 持续追加最新全景保持线性追加和缓存，却保留过时 projection 并增加输入成本。
3. 把 Map 作为按需读取的外部事实最接近 Standard 的成本形态，但会降低 Map 的持续显著性。

这三种行为都可能有产品价值，且代价来自不同设计目标，不应继续在单一 R6 路径中互相折中。R7
因此冻结 R6 开发，把差异显式建模为三个可切换策略，并通过同一基建公平验证。

## 0.3 产品定义

TaskSpace 仍然是 Standard 自然上下文的图化、状态机化再组织。R7 只增加以下配置维度：

```text
taskspace_projection_policy =
  map-always
  | map-append
  | map-request
```

策略由用户或 benchmark 在创建 TaskSpace session 时选择，并在该 session 生命周期内保持不变。
Agent 不能通过普通输入、tool call 或 Map 状态变更切换策略。恢复、分叉和回放必须保留原策略；需要
更换策略时新建 session，不实现中途迁移。

Standard 是横向对照，不是第四种 TaskSpace projection 策略。R6 的 `epoch baseline + delta
journal` 是历史实现，也不是第四种产品模式。

## 0.4 唯一架构

### 0.4.1 单一数据流

```text
Agent taskspace_control / ordinary tools
                 |
                 v
      shared Runtime hard invariants
                 |
                 v
      canonical Rooted DAG + Event Store
                 |
                 v
        shared Projection Renderer
                 |
                 v
         ProjectionPolicy::emit()
        /             |             \
 map-always      map-append       map-request
 replace latest   append request   explicit read result
        \             |             /
                 v
       shared Provider Context Composer
```

三种策略必须共用：

- 同一 canonical Rooted DAG、validator、reducer、revision 和 lease；
- 同一 Event Store、snapshot、resume、fork 和 replay；
- 同一 `taskspace_control` schema、parser、handler 和 sequence executor；
- 同一 ordinary tool router、权限、沙箱、hook 和原始反馈链；
- 同一 projection renderer、字段定义、排序、折叠和引用规则；
- 同一 provider serializer、compaction 基建、日志与 benchmark observer；
- 同一系统提示词主体和 TaskSpace 硬约束。

唯一允许变化的生产决策是：同一份 rendered projection 是否进入 context、何时进入、作为可替换
当前视图还是不可变历史项进入。不得为某个策略复制 Runtime、Map、tool schema、事件类型或反馈层。

### 0.4.2 禁止的架构分叉

R7 禁止：

- `AlwaysRuntime`、`AppendRuntime`、`RequestRuntime` 三套执行器；
- 三套 projection renderer 或字段不同的 projection schema；
- 根据策略改变 Map 不变量、节点生命周期、工具权限或终结规则；
- 根据策略增删 `taskspace_control` action；
- 为某一策略维护独立 Map、副本状态或 completion ledger；
- 为 R6 session 写 migration、adapter、dual reader 或 silent fallback；
- 用策略专属提示词改变 Agent 的任务拆分、工具选择或完成判断；
- 把缓存、压缩或 observer 的实现复制进每个策略分支。

## 0.5 共享 Projection 合同

Projection 是 canonical Map 的确定性纯构造结果，不是第二份事实：

```text
RenderedProjection {
  schema_version
  map_id
  revision
  root
  finish
  nodes[]
  edges[]
  frontier[]
  current_binding
  source_refs[]
  result_refs[]
  folding_facts[]
  canonical_sha256
}
```

同一 canonical Map revision、同一 renderer 版本和同一机械预算必须生成字节一致的 projection。
renderer 必须：

1. 忠实保留 Root、Finish、全部节点和边的全局骨架。
2. 原样保留状态、引用、失败和裁剪事实，不注入下一步建议或重新解释工具反馈。
3. 只执行共享的、确定性的详情折叠；不得因当前策略改变 Map 内容。
4. 输出 `map_id`、`revision` 和 hash，使 context、Event Store 与 canonical Map 可机械对账。
5. 对过长局部详情提供精确 ref；Map 骨架最终超限属于独立压缩专项，不在三策略中分叉解决。

## 0.6 三种策略合同

### 0.6.1 `map-always`

定义：每次 provider request 都从最新 canonical Map revision 重新渲染一份 projection，并在最终
provider payload 中只保留这一份当前 projection。前一轮 projection 不作为自然历史继续携带。

不变量：

- active Map 时每个 provider request 恰好一个 current projection；
- `emitted_revision == canonical_revision`，map id 和 hash 均一致；
- payload 中不存在旧 revision projection；
- retry、resume 和 compaction continuation 也必须读取最新 canonical Map；
- projection 置于稳定自然历史之后的动态区域，不能覆盖或重排原始消息。

已知产品特征：动态 projection 高频变化会降低 DeepSeek 自动前缀缓存命中，并增加 uncached input
成本。这是该策略为持续全局视野支付的已知代价，不作为实现 bug；缺失、重复、陈旧或 hash 不一致才是
bug。

### 0.6.2 `map-append`

定义：每次 provider request 构造时，把当时最新的完整 projection 作为不可变消息持久追加到自然
context 的末尾。下一轮保留旧 projection 和新增自然历史，再在末尾追加最新 projection。该行为不依赖
control carrier 或 revision commit；provider retry 若末项已经是同一 projection，则不重复追加。

每个追加项必须带有机械版本标记：

```text
projection_kind: request_snapshot
map_id: <id>
revision: <n>
supersedes_all_prior_projections: true
current_state_rule: last_projection_only
```

不变量：

- 每轮有效 provider request 的最后一条 message 是最新 projection；
- 同一 revision 可因连续 request 重复，Map revision 必须非递减；
- 最后一份 projection 是当前状态的唯一权威；
- 更早 projection 只作为历史证据，不能用于 current/frontier/status 或后续 tool 参数；
- 不得在不可变旧消息中写入永久性的 `authoritative_current=true`；
- 使用旧 revision 发起状态调用时返回机械 `stale_revision`，Runtime 不自动改写参数。

系统提示词和 tool description 只声明上述末项选择规则，不添加策略建议。旧 projection 累积、输入
增长和陈旧历史干扰是该策略换取线性追加与缓存的已知代价；缺少 supersession 标记、request 末项
不是 projection、revision 回退或最新 projection 与 Map 不一致才是 bug。

### 0.6.3 `map-request`

定义：普通 provider request 不自动注入完整 projection。Agent 通过共享
`taskspace_control.read_map` 或既有精确读取 action 主动获取最新 Map；读取结果作为忠实 tool result
自然追加到 context。多次读取形成由 Agent 选择时机的稀疏 projection 历史。

Map 在该策略下仍是不可绕过的工作状态机，不是可选第三方账本。共享 Runtime 必须继续执行：

- TaskSpace 开始后必须由 Agent 初始化合法 Map；空 Map 时普通工具和 subagent 不可执行；
- ordinary tool 必须绑定有效 Work node/lease，call/result 机械归属到该节点；
- Map 初始化、图变更、bind、transition、finish 只能通过 `taskspace_control`；
- `update_plan` 在 TaskSpace 中继续隐藏；
- Root 保持 OPEN，直到 Agent 显式 `finish_end` 闭合 Root 和 Finish；
- Map 未合法终结时不能绕过状态机直接结束 TaskSpace；
- subagent 继续受 node/lease 约束，策略不改变并发与归属规则。

Runtime 不得为了提高 Map 使用率而：

- 自动调用 `read_map`、规定读取频率或在每轮追加提醒；
- 自动选择、创建、连接或完成节点；
- 从自然语言推断 Agent 是否“忘记了 Map”；
- 修复 stale revision、错误 node id 或错误工具参数；
- 因 Agent 没有读取最新 Map 而拒绝本来符合硬约束的 ordinary action。

该策略可能降低 Map 对 Agent 的持续影响，这是待验证的产品假设，不是预先确认的缺陷。若 Agent 不读
Map 但仍遵守所有硬约束，Runtime 不得额外干预；若 Agent 能绕过初始化、binding 或显式终结，则属于
实现 bug。

## 0.7 Compaction、恢复与分叉

canonical Map 和 Event Store 独立于 provider context，不能被 context compaction 删除或改写。三种策略
共用 compaction 管线，只在新 context epoch 的 projection emission 上遵守各自策略：

| 场景 | map-always | map-append | map-request |
|---|---|---|---|
| 普通有效 request | 替换为最新 projection | 每次在末尾追加当时最新 projection；仅同 payload retry 去重 | 不自动注入 |
| compaction 后首轮 | 注入最新 projection | 追加一份当前 revision 作为新 epoch 起点 | 仅保留机械 Map handle，Agent 按需读取 |
| resume 后首轮 | 注入最新 projection | 若恢复历史已含当前 revision 则不重复，否则追加一次 | 仅保留机械 Map handle |
| fork | 子 session 使用同一 canonical fork snapshot 和原策略 | 同左 | 同左 |

`map-request` 的 Map handle 只能包含 `map_id`、当前 revision、TaskSpace active、可用读取 action 等机械
身份信息，不得变成缩小版 projection，也不得包含 next action。它用于保持状态机存在性和工具参数可用性，
不是对 Agent 的语义提醒。

`map-request` 中某次 `read_map` 结果只证明读取当时的 revision；后续控制提交使其成为历史 projection。只有
读取结果 revision 与 Map handle 或最新控制反馈报告的 canonical revision 相等时，才可称为 current。Runtime
不得因视图过期自动读 Map 或拒绝本来符合硬约束的 ordinary action。

R7 不把旧 projection 永久保留视为 compaction 的硬要求。compaction 可以按共享上下文机制淘汰历史消息，
但必须记录裁剪事实，且不得影响 canonical Map。`map-append` 在新 epoch 从一份当前 snapshot 继续，旧
revision 的 provider 历史被压缩是上下文生命周期事实，不是 Map 数据丢失。

## 0.8 Agent 与 Runtime 边界

| 事项 | Agent | Runtime / Projection 基建 |
|---|---:|---:|
| 选择 projection 策略 | 用户配置 | 校验并冻结 session 参数 |
| 定义节点、边和 goal | 决定 | 原样保存、机械校验 |
| 判断语义完成与下一步 | 决定 | 不判断、不提示 |
| 生成 projection 内容 | 不手工维护 | 从 canonical Map 纯构造 |
| 决定 map-request 读取时机 | 决定 | 执行并忠实返回 |
| 决定 always/append emission | 策略合同决定 | 机械执行 |
| 缓存命中 | 不保证 | 观测，不伪造、不补偿 |
| 状态机硬约束 | 发起合法动作 | 不可绕过地校验 |
| 工具错误纠正 | 决定 | 忠实反馈，不替 Agent 修正 |

继续遵守已冻结的诊断优先级：Agent 出现重复、低级错误或异常成本时，先检查 context、tool result、
projection 和 provider payload 是否发生丢失、残缺、扭曲、重复或过期，再评估 Agent 能力；不得默认
增强 Runtime 语义控制。

## 0.9 参数与持久化合同

目标配置只保留一个 canonical 字段：

```text
taskspace_projection_policy = "map-always" | "map-append" | "map-request"
```

实现阶段可以同时提供 CLI 对该字段的直接覆盖，但 CLI、配置文件、protocol 和 session metadata 必须
解析为同一个 enum，不得各自维护字符串分支。要求：

- 无值时使用一个明确、可观测的实验默认值；默认值必须在正式四臂结果后冻结；
- 非法值启动失败，不 silent fallback；
- policy 写入 session/rollout metadata，resume/fork 精确恢复；
- Agent tool schema 不暴露切换 action；
- benchmark artifact 必须记录 policy、来源和最终解析值；
- 不接受 R6 epoch baseline 别名，不迁移旧 session。

## 0.10 可观测性

共用 observer 至少记录以下结构化事件，不保存 API key、完整用户输入或未经授权的原始 provider payload：

```text
taskspace.projection_emitted
  policy
  trigger: provider_request | explicit_read
  map_id_hash
  canonical_revision
  emitted_revision
  projection_sha256
  bytes / estimated_tokens
  context_position
  persisted_in_history
  projection_is_message_tail
  same_revision_as_previous
  supersedes_all_prior_projections
  freshness_verdict

taskspace.projection_read_requested
taskspace.projection_read_completed
taskspace.projection_policy_restored
```

provider wire observer 必须按策略解释 projection 数量，不能继续把“每 request 恰好一个 active
projection”作为三种策略共用断言。缓存报告同时保留 provider 返回的 hit/miss tokens、请求级 LCP、
projection token 占比和策略字段。

## 0.11 已知特征与实现 Bug

| 策略 | 已知产品特征，不作为 bug | 实现 bug |
|---|---|---|
| map-always | 自动缓存偏低、uncached 成本偏高 | projection 缺失、重复、陈旧、hash/revision 不一致 |
| map-append | 每个 request 的旧 projection 累积、总 input 增长、可能干扰注意力；map 未变化时 revision 会重复 | request 末尾缺 projection、revision 倒退、无 supersession、末项 identity 错误 |
| map-request | 无持续全景、读取次数可能低、Map 影响力可能下降 | Map 可绕过、read 返回错误状态、binding/terminal 失效 |

“已知特征”仍必须持续量化；它只意味着不能用修 bug 的名义改变策略定义，不意味着产品一定接受该
代价。三种策略都保留为一等实验策略，不因单轮结果自动删除。

## 0.12 R7 总验收

R7 完成必须同时满足：

1. 只有一个 canonical Map、一个 renderer、一个 provider composer 和一个工具/反馈链。
2. 三种策略由同一 enum/参数切换，除 projection emission 外行为逐事件一致。
3. `map-always` 每轮只暴露最新 projection，并明确量化缓存代价。
4. `map-append` 在每轮 provider request 末尾持久追加当时最新完整 projection，旧版本明确失效且
   最后版本唯一权威。
5. `map-request` 不自动注入完整 Map，但初始化、binding、归属和显式终结均不可绕过。
6. resume、fork、retry 和 compaction 对每种策略都有确定、可回放的结果。
7. R6 epoch baseline 生产路径和专属状态被删除，不保留兼容分支。
8. Standard + 三策略在同一 Docker 基建、模型、样本和 observer 下完成正式四臂对照。
9. correctness、Map 完整性和反馈保真不得因换策略而变化；成本和 Agent 行为差异可归因到 policy。
10. 文档、配置、日志、benchmark artifact 和最终代码使用同一策略名称与合同。

## 0.13 明确非目标

- R7 不重做 R6 Rooted DAG 领域模型、终结状态机或工具能力合同。
- R7 不为三种策略设计不同 Agent、提示词人格、工具集或 Runtime 权限。
- R7 不保证某一策略同时取得最高 Map 影响力、最低输入和最高缓存。
- R7 不把 provider 自动缓存包装为可由用户标记的 cache unit。
- R7 不在本轮完成 Map 骨架最终超限的通用压缩方案。
- R7 不兼容 R6 session/snapshot，不保留旧 epoch baseline 产品路径。
- R7 不在正式证据前指定唯一默认策略或宣称某臂获胜。

## 0.14 外部设计依据

1. DeepSeek 缓存按已持久化的完整前缀单元匹配，线性追加可以复用前一轮前缀，而中部替换只能命中
   更早的共同前缀。R7 因此把缓存差异作为三策略的产品特征显式验证，而不是用 Runtime 猜测补偿。
   [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/)
2. DeepSeek Anthropic API 当前忽略 message、tool 和 tool result 上的 `cache_control`，产品不能依赖
   用户标记 cache unit 来消除三策略成本差异。
   [DeepSeek Anthropic API compatibility](https://api-docs.deepseek.com/guides/anthropic_api/)
3. JSON Schema 的条件结构可以把配置和 tool action 约束为互斥分支；R7 只用它表达机械输入合同，
   不把策略差异复制成三套工具 schema。
   [JSON Schema conditional validation](https://json-schema.org/understanding-json-schema/reference/conditionals)
4. R6 采用的事件历史可恢复原则在 R7 继续成立：projection 是可重建视图，不能成为新的事实源。
   [Temporal history service architecture](https://github.com/temporalio/temporal/blob/main/docs/architecture/history-service.md)
