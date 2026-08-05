# TaskSpace Exec 正式工程计划

- Created: 2026-08-05
- Refined: 2026-08-06 after A2 reopening
- Status: Phase A evidence-complete / Phase B1 ready
- Risk depth: Full
- Product contract: [`00-product-contract.md`](00-product-contract.md)
- Upstream evidence: [`01-upstream-and-feasibility-evidence.md`](01-upstream-and-feasibility-evidence.md)
- Issue dependency review: [`03-global-issue-prerequisite-review.md`](03-global-issue-prerequisite-review.md)
- Integrates: I10 能力身份、I06 统一 admission、I01/I02/I05 结果合同、I07 新协议观测
- Excludes: projection 三模式重构、Map 压缩、旧数据兼容、Provider Tool proxy

## 1. Phase A 后的目标数据流

```text
TaskSpace provider request
  -> top-level Function taskspace_exec
  -> provider-native hosted capability descriptors

provider response streaming
  -> OutputItemDone(hosted fact 0..N)       -- provider 已执行
  -> OutputItemDone(taskspace_exec call 1)  -- Agent 声明 plan + per-item nodes
  -> response.completed(response_id)

existing session/turn response coordinator
  -> freeze one response-local TaskspaceExecEnvelope
  -> decode the outer call into one typed plan
  -> validate the complete Agent contract and canonical Map transition
  -> persist hosted facts with their real provider item IDs
  -> dispatch admitted client/map calls through the existing ToolRouter
  -> return one outer FunctionCallOutput with native results and final revision
```

最小必要增量只有四类：一个 response-local envelope、一个 outer response executor、TaskSpace invocation metadata、Hosted
事实结算。ToolSpec、Router、权限、sandbox、hook、业务 handler、Map Store 和原始 Tool result 都继续复用。

明确不采用：Session 全局暂存、重放重建 envelope、Agent 回显 Provider ID、第二 Router、Provider proxy、独立 binding
database、reasoning 解析或旧协议兼容分支。

## 2. 已冻结的不变量

1. Standard 的 ToolSpec、请求 payload、dispatch 和结果必须保持逐字/结构等价。
2. TaskSpace 顶层只暴露 `taskspace_exec + provider-native hosted capabilities`。
3. 一次 Provider response 只允许一个 outer `taskspace_exec`；内部不得递归调用 `taskspace_exec` 或原 `exec`。
4. 每个 client call 的 `node_id` 由 Agent 声明，普通 Tool schema 和 handler 对 TaskSpace 无感。
5. Agent 为每项 Hosted 动作分别声明非空 `node_ids[]`；Runtime 从同响应真实 output item 读取 `id/item_id` 并逐项核对。
6. 同响应 Hosted facts 可以归属不同节点；响应边界不是 Map 归属边界。
7. Provider Tool outcome、client Tool outcome和节点生命周期互不推导。
8. Agent 合同、Hosted reconciliation 或 Map preflight 失败时，尚未发生的 client/map 零执行、Map 零提交。
9. 漏绑、结构错配、歧义、非法节点或 Provider 身份缺失/重复使整个 TaskSpace 响应不被接受；不得默认写 Root 或未绑定池后继续。
10. 每个 Tool 结果只进入 Agent context 一次，不新增 developer/system factual carrier。
11. Client Tool 合同只暴露一次：Standard 使用原生顶层 Tool schema；TaskSpace 移除这些顶层 schema，并从同一
    ToolSpec 快照在 `taskspace_exec` 内机械暴露。Provider-hosted Tool 完整合同仍只在 provider 原生顶层，Exec 内不复制。
12. 成本比较不得把迁移后的内部 Client Tool 合同整体算作新增 input；必须分别报告原有 Tool 合同、TaskSpace
    metadata 和序列化形式差值。协议固有增量仅包括 `node_id`、合法序列、Hosted binding 和必要容器字段。

### 2.1 Phase A 边界

Phase A 只负责用理论分析、上游/官方资料、源码静态盘点、历史 trace、fixture 和隔离小型候选代码回答：

1. 产品合同是否自洽，是否与全局边界冲突；
2. Provider wire 和 Codex 上游 seam 是否提供继续实施所需的基础事实；
3. 是否已发现足以否定某个候选的轻量证据；
4. 是否存在至少一条不违反产品约束、值得进入实施的路径。

以下工作不得作为 Phase A 全局门禁：需要跨 response lifecycle、Router、Map transaction、Event Store、生产
projection 或缓存链路才能得出结论的工作；需要大量候选代码、完整集成矩阵或多轮付费样本的工作。这些结论应分配到
最早能以真实实施证据验证它们的后续 TX 单元。“尚需较重实施才能确认”是后续风险，不等于 Phase A 失败。

## 3. 工作单元

`Execution Status` 使用 `verified-isolated` 表示实现和定向测试已通过、但尚未接入生产路径；它不等于 integrated。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Execution Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| TX-00 | 修正请求/usage 双计 | observability | I07-W0～W3 | canonical request facts | 按 completed request identity 聚合，排除无 ID rate-limit snapshot | 8 个请求稳定报告为 8 个 | 后续成本证据可复算 | Complexity: 修正共享消费口径；Reach/Cost: 历史错误数字变化 | 历史 8/15 重放为 8 | 身份缺失时 fail closed | verified |
| TX-01 | 固定当前生产与上游 seam | discovery | `core/src/tools`、最新 Codex main | ToolSpec/Router/response lifecycle | 记录可复用入口和旧路径删除清单 | 后续实现有唯一落点 | 避免沿过期架构继续开发 | Complexity: 文档；Reach/Cost: 零运行时影响 | 源码引用逐项复查 | 证据不足停在 discovery | verified |
| TX-02 | 冻结 Function Tool 与能力身份 | API | `taskspace_exec/catalog.rs` | ToolSpec snapshot/capability ID | 从同一 ToolSpec 快照派生 outer 描述、内部 catalog 和身份 | 声明、执行、缓存、观测共享能力版本 | 关闭 I10 的根源入口 | Complexity: 候选 catalog；Reach/Cost: 未接生产 | 5 个 catalog tests | 不注册 ToolSpec | verified-isolated |
| TX-03 | 形成唯一 typed plan | API/internal | `taskspace_exec/decoder.rs` | `TaskspaceExecPlan` | 只接受 `taskspace.plan(<strict JSON>)` | 副作用前获得完整计划 | 不解析 reasoning，不边执行边发现非法 | Complexity: 一个严格 decoder；Reach/Cost: Agent 生成合同增加 | 正反 decoder fixtures | 动态表达需求出现时回到产品决策 | verified-isolated |
| TX-04 | 冻结纯输入预检 | internal | `taskspace_exec/preflight.rs` | structural preflight | 校验版本、能力、调用身份、边界、递归和单 Patch | 明确错误在 client dispatch 前发生 | 建立 I06 零执行底线 | Complexity: 候选 validator；Reach/Cost: canonical Map 尚未接入 | 结构矩阵 tests | 不复制 Map 状态机 | verified-isolated |
| TX-05 | 验证 Hosted 逐项多节点合同是否值得实施 | provider/API | `taskspace_exec/plan.rs`、`provider_reconcile.rs`、provider response fixtures、小预算 probe | per-item Hosted binding contract | 确认真实 Provider ID/顺序存在、多节点语义可表达、Runtime 可无语义核对，并用真实 trace 识别 source-only 候选的局限 | Phase A 得到“合同自洽、基础事实存在、source-only 不应直接落地”的足够证据 | Complexity: 不再扩建生产 carrier 或继续付费试错；Reach/Cost: 实施稳定性后移 | V1～V3 静态/离线矩阵；V4 两次有界 probe 和原始 trace | 不在 Phase A 设计/实施结构化 carrier 或 receipt 链路 | phase-a-evidence-complete |
| TX-06 | 实施并验证结构化 outer contract 与唯一内部 Tool catalog | API/internal/cache | 上游 `spec_plan/ToolExposure` seam、code-mode helper、TaskSpace Tool projection | structured Function carrier + neutral ToolSpec conversion | 对齐上游 seam，从同一 ToolSpec 快照机械派生 outer contract、内部 catalog 和 capability ID；TaskSpace 删除顶层 client schema，只在 outer contract 内暴露一次；边实施边验证 DeepSeek 的结构化承载能力 | 不保留 source-only 与结构化两套生产协议，不复制 Tool 合同；工具迁移本身不形成新增 input | 将必须依赖真实 schema 和上游转换的结论放回实施环境验证 | Complexity: B1 承担 carrier 与共享 catalog 的完整小主题；Reach/Cost: provider payload、code-mode snapshot、缓存指纹受影响 | schema 派生 fixtures、code-mode 0-diff、catalog/Router 一一对应、顶层 client 零泄漏、单次合同暴露检查、Standard/TaskSpace 静态 Tool payload 体积拆分、缓存门禁；真实模型只做获批的小预算验证 | 需要第二 registry、修改普通 Tool schema、双重暴露或两套 carrier 时停止 | not-started |
| TX-07 | 收集响应级事实而不建全局状态 | response lifecycle | Responses SSE decoder、`session/turn.rs` | response-local `TaskspaceExecEnvelope` | 保留每个 Hosted item 的原始 `output_index`，收集唯一 outer call，在 `response.completed` 冻结并消费 | executor 按 Provider 顺序获得真实 facts 和 Agent plan，不受并行完成顺序影响 | 解决 Hosted/Function sibling 数据归属，不重放重建 | Complexity: +1 局部 envelope 和 output-index 元数据；Reach/Cost: response loop、stream fixtures 受影响，零持久状态 | 0/1/N Hosted、乱序 done、重复/缺失 index、缺/多 outer call、stream abort；Standard 0-diff | 无法保留原始 `output_index` 或 envelope 需跨响应时停止 | not-started |
| TX-08 | 建立 outer response executor | internal | `taskspace_exec/response_executor.rs`、现有 response sequence入口 | outer call decode/output pairing | 消费 TX-07 envelope，解码一次、拒绝多个 outer call、保留原 outer `call_id`，禁止递归 | `taskspace_exec` 有单一 Runtime 入口且能返回原生配对 output | 把特殊性限制在必要的响应边界 | Complexity: +1 response executor，不注册第二业务 Router；Reach/Cost: tool sequence入口和取消路径受影响 | outer call pairing、decode failure、cancel/timeout、零内部 dispatch | 需要普通 Tool handler读取全局 envelope 时回退 | not-started |
| TX-09 | 接入 canonical Map admission | state | `taskspace_exec/map_admission.rs`、现有 Action Map validator/transaction | prepared Map transaction | 将 typed plan 的 Map calls、node bindings、revision 和 client reservations一次交给现有 validator 准备 | Agent 合同和 DAG/状态硬规则在任何 client dispatch 前确定 | 不复制状态机并关闭 I06 绕过 | Complexity: +1 adapter，净复用 canonical transaction；Reach/Cost: Map revision、reservation tests 扩大 | init/reopen/complete/finish、stale、unknown/not-ready node、零提交拒绝 | 任一规则需复制到 exec 时停止 | not-started |
| TX-10 | 机械执行 client/map 计划 | internal | `taskspace_exec/dispatch.rs`、`ToolRouter` | admitted nested `ToolCall` | 从 TX-09 prepared plan 还原原生 payload，携带 outer/item/node/capability metadata，经原 Router 执行 | Function/Freeform/MCP/Namespace/ToolSearch/LocalShell 保持原权限、hook 和结果 | 普通 Tool 无 TaskSpace 侵入且所有入口统一过门 | Complexity: +1 invocation adapter，无第二 executor；Reach/Cost: 并行、取消、Patch、全部 Tool 类型测试受影响 | 类型矩阵、单 Patch、并行独立 calls、结果依赖拆批、plan 外调用零执行 | 任一 Tool 需修改原生 args 时停止 | not-started |
| TX-11 | 原子核对 Hosted facts | provider | `taskspace_exec/provider_reconcile.rs`、TX-07 envelope、TX-09 admission | response-scope reconciliation | 用真实 item ID 与 Agent 逐项 `node_ids[]` 生成唯一 fact-node relation set；任一漏项、歧义、非法节点或身份冲突拒绝整个响应 | 只有完整归属的 Provider 事实能进入 Map | Provider 能力进入 Map但不重执行、不猜配、不接受半结算 | Complexity: 扩展现有 reconciler；Reach/Cost: Web/Image fixtures 和整批失败反馈增加 | 0/1/N、多 owner、failed、缺/重复 ID/node、invalid plan、prelude-created node；失败时零 client/map | 无法唯一核对任一项时停止，不生成 unbound settlement | not-started |
| TX-12 | 复用 Event Store 持久化与幂等 | data | `action_map/event_store.rs`、现有 canonical persistence | provider item fact + node reference set | 增加按 `provider_item_id` 查重和 node set 冲突检查；只允许 Agent 显式声明且通过 TX-11/canonical admission 的引用集合进入 Store，不建新表或未绑定池 | restart/replay 直接读取一份固化事实及其完整节点引用，不重建 Map 或 envelope | 保持全局唯一 Map 和单一持久事实源 | Complexity: 扩展现有 Event Store API，无新数据库；Reach/Cost: snapshot/replay/compaction tests 受影响 | same-relation-set replay 幂等、conflicting-set reject、Web/Image round-trip、SQLite restore、拒绝默认 Root 引用 | 现有 Store 无法原子拒绝不完整归属时先停下，不加旁路账本 | not-started |
| TX-13 | 返回唯一无损结果 | feedback | `taskspace_exec/result.rs`、provider final wire | outer FunctionCallOutput | 组合内部原生结果、Hosted settlement、失败来源和唯一 final revision；删除额外 factual carrier依赖 | Agent 一次看到完整事实和可继续 revision | 承接 I01/I02/I05，减少歧义与缓存污染 | Complexity: +1 result builder，切换后删除旧 receipt；Reach/Cost: context/final-wire snapshots受影响 | 每个结果一次、原文/结构保真、状态/Tool/Provider错误不互换、Standard 0-diff | 任一原生结果丢失时不接 projection | not-started |
| TX-14 | 建设可复算日志 | observability | I07-W9～W11、response/dispatch trace | exec response correlation | 记录 provider request/response、outer call、item、node set、provider item、capability、dispatch 和 settlement 身份 | 一次失败可定位在生成、解码、Map、Provider、dispatch或反馈 | 完成 I07 新协议证据链 | Complexity: 复用现有 trace schema并增加关联字段；Reach/Cost: rollout 和报告脚本同步 | 合成 trace 重算；local reject provider delta=0；无重复 request/token | 需要第二 observer 数据源时停止 | not-started |
| TX-15 | 原子切换 TaskSpace projection | integration/cache | request Tool projection、provider builder | TaskSpace effective Tool list | TaskSpace 一次改为 `taskspace_exec + native hosted`，Standard 不变；同批启用 TX-07～14 | Agent 生成入口天然符合新协议，Client Tool 能力从顶层迁移而非叠加 | 停止旧 sibling 的事后惩罚式配对 | Complexity: +1 mode projection，替换旧分支；Reach/Cost: 所有 TaskSpace 请求和缓存受影响 | 缓存门禁、payload snapshots、Standard exact equality、TaskSpace 顶层 client 零泄漏、每份 client/hosted 完整合同恰好一次 | 门禁阻断后说明并申请专用预算 | not-started |
| TX-16 | 删除旧生产协议 | cleanup | old `taskspace_tool`、sequence/sibling、prompt/observer | actions manifest/pairing/decoration | 删除旧 schema 入侵、control manifest、sibling preflight/executor、receipt 和兼容 parser | 生产代码只有一个 TaskSpace 动作协议 | 防止双事实和修 A 回归 B | Complexity: 净删除分支；Reach/Cost: 旧测试和 benchmark parser 迁移 | `rg` 删除清单、全量 core/tool tests、无旧 wire snapshot | TX-15 未通过前不执行；原子提交可整体回退 | not-started |
| TX-17 | 确定性集成验收 | test | core/tool/provider/session integration tests | legal/illegal response matrix | 覆盖初始化并工作、多节点独立调用、完成并继续/结束、read-only、单 Patch、同响应 Hosted 多节点、取消、replay | 付费运行前证明硬合同与失败边界 | 降低真实样本只用于模型行为的成本 | Complexity: 增加聚焦 fixtures；Reach/Cost: CI 时间增加，零 API 费用 | 指定矩阵全绿、任何 Hosted 绑定错误整批拒绝、Store/日志可复算、Standard 0-diff | 任一不变量失败即停止真实运行 | not-started |

### 3.1 TX-05 / A2 重新验证单元

详细合同和执行方法见 [`07-a2-multi-node-binding-validation-plan.md`](07-a2-multi-node-binding-validation-plan.md)。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A2-V1 | 找到逐项关联的真实 wire 能力 | discovery | provider response fixtures、Responses decoder、既有 Hosted probe artifact | Hosted output identity/order/model-visible facts | 盘点 Agent 可声明字段与 Runtime 可见字段的交集，排除内容和语义猜配 | 冻结 `output_index` 排序 + Agent 有序声明，不要求 Agent 回显 Provider ID | Complexity: 仅证据矩阵；Reach/Cost: 零生产影响、零 API 费用 | Web/Image、同类多项、乱序 done fixture 可恢复唯一顺序 | TX-07 无法保留原始 index 时停止 | verified |
| A2-V2 | 冻结逐项多节点 Agent 合同 | API/internal | `taskspace_exec/plan.rs`、`decoder.rs`、`preflight.rs` | `hosted_bindings[]` typed plan | 使用有序 `{tool,node_ids[]}`，每项节点集合非空且无重复，计划直接升级 v3、不兼容 v2 | 一个 Hosted 事实可由 Agent 绑定给一个或多个节点，且不修改 provider Tool schema | Complexity: 修改未接生产候选 schema；Reach/Cost: Phase A snapshots/tests 更新，缓存生产面不变 | strict decode、旧单 node 字段拒绝、空/重复节点拒绝、多 owner fixtures | 需要第二业务 schema或语义匹配时停止 | verified-isolated |
| A2-V3 | 证明错误整批拒绝 | internal | `provider_reconcile.rs` | atomic Hosted reconciliation report | 按 output index、数量、Tool 类型和非空节点集合完整核对；任一 finding 返回空绑定集合 | 下游没有部分成功、重复事实、unbound settlement 或默认 owner | Complexity: 扩大候选 reconciler 和结构化 finding；Reach/Cost: 零生产接线、零 Provider 费用 | 多 owner 成功；缺/多/乱序/重复 ID/index/node、类型错配均整批空 bindings | 完整 Map/Store/Router 零副作用在 TX-09/11/12/17 接线时复验 | verified-isolated |
| A2-V4 | 用有界真实样本识别候选风险 | provider validation | 专用 Hosted probe、全局 run ledger | same-response multi-node declaration | 用不预设 Hosted 数量的 v3 协议验证同响应多节点绑定，通过 trace 分离可见性、合同暴露和模型执行 | 证明 Agent 可见全部动作，同时 source-only 候选不足以直接进入生产落地 | Complexity: 两次 probe 均首败即停，不扩大 repeat；Reach/Cost: 稳定性结论移交 TX-06/17/18 | 原始 trace 和费用可复算 | Phase A 不再重试；结构化 carrier 随 TX-06 实施验证 | evidence-complete-risk-deferred |
| TX-18 | 小预算真实行为与成本验证 | validation | Docker benchmark、run ledger | Standard + TaskSpace policies | 获批后先跑简单/复杂各一次；异常先归因，不自动扩大 repeat；成本按原有 Tool 合同、TaskSpace metadata、自然上下文和动态结果拆分 | 验证 Agent 能生成 exec、任务完成、缓存和成本可观测，并避免把 Tool 迁移误报为新增成本 | 判断可行实现是否形成真实产品收益 | Complexity: 不增加生产代码；Reach/Cost: 产生 DeepSeek token、费用和时间 | 逐 request trace、Map、request/token/cache/time/cost 表；静态 payload 拆分与 Provider usage 可相互解释 | 未获预算不运行；失败不以重试覆盖 | not-started |
| TX-19 | 重评 R8 问题全集 | planning | `01-r8-known-issues.md` | I01～I10 | 只依据新生产 trace 更新关闭、改方案或新增状态 | R8 回到唯一证据化问题队列 | 防止旧根因继续污染路线 | Complexity: 文档状态变更；Reach/Cost: 改变后续优先级 | 每项有新协议证据链接 | 无 E2/E3 证据保持 open | not-started |

## 4. 阶段与停点

### Phase 0：观测基线

- Units: TX-00。
- Exit: 已 verified；历史 8/15 双计已可重放为 8。

### Phase A：可行性与合同

- Units: TX-01～TX-05。
- A1: typed plan 能在副作用前完整形成，已通过。
- A2: V1～V3 证明多节点产品合同和无语义核对在离线成立；V4 排除 Hosted 子动作不可见，并否定 source-only 作为可直接落地的承载方式。
- Exit: 已完成。产品合同自洽、Provider/上游基础事实存在、明确排除默认 Root/unbound 和 source-only 直接落地；结构化 carrier、完整 Hosted 链路和模型稳定性按责任移交后续阶段。

### Phase B1：结构化承载、共享能力与响应入口

- Entry: Phase A 轻量证据已完成。
- Units: TX-06～TX-08。
- Exit: 结构化 outer contract、单一 catalog、response-local envelope 和 outer executor 均有确定性测试；TaskSpace
  顶层没有普通 client Tool，内部每份 Client Tool 合同只暴露一次；静态成本报告能把原有 Tool 合同与 TaskSpace
  metadata 分开；没有 source-only 生产平行协议或 Session 全局临时状态。
- Cross-unit side effects: code-mode helper 和 response loop 受影响，但 Standard wire 不变。

### Phase B2：Map、执行与 Hosted 持久化

- Entry: B1 通过。
- Units: TX-09～TX-12。
- Exit: canonical Map admission、原 Router dispatch、Hosted settlement 和 Event Store restore 形成离线闭环。
- Cross-unit side effects: Map transaction、fact-node references 和全部 nested Tool 类型测试扩大；不新增数据库。

### Phase B3：反馈与观测

- Entry: B2 通过。
- Units: TX-13～TX-14。
- Exit: outer result 只传一次，trace 可从 provider request 复算到每个内部结果和 Hosted fact。
- Cross-unit side effects: final wire 和 benchmark parser 增加新协议字段，但生产 projection 尚未切换。

### Phase C：原子生产切换与清理

- Entry: B1～B3 全绿；缓存敏感面已被门禁识别。
- Units: TX-15～TX-16。
- Exit: TaskSpace 只有 `taskspace_exec` 主路径，Standard 0-diff，旧协议无生产残留。
- Cross-unit side effects: TaskSpace model-visible Tool 集合一次变化；回退必须整体回退原子切换提交。

### Phase D：验收与问题重排

- Entry: Phase C 完成；先执行 TX-17，只有其确定性矩阵通过后才允许进入 TX-18。
- Units: TX-17～TX-19。
- Exit: 真实质量、缓存、成本和问题状态均有可复算证据。
- Cross-unit side effects: TX-18 产生经授权的 API 成本；其他单元零 Provider 费用。

## 5. 依赖与明确停点

| Stop | Trigger | Decision |
|---|---|---|
| B1-S1 | catalog 转换必须复制 registry 或修改普通 Tool schema | 停止 TX-06，重新对齐上游 seam |
| B1-S1a | 同一 Client Tool 合同必须在 TaskSpace 顶层与 Exec 内部同时暴露，或必须在多个 Prompt/Tool 层完整复述 | 停止 TX-06，不能用双重暴露换取模型遵循度 |
| B1-S2 | response envelope 必须写 Session 全局状态或跨响应恢复 | 停止 TX-07，不接受隐式生命周期 |
| B2-S1 | exec 需要复制 Map 状态规则 | 停止 TX-09，只允许 canonical validator adapter |
| B2-S2 | 某类 client Tool 必须修改原生参数才能携带 node | 停止 TX-10，不做 Tool schema 入侵 |
| A-S1 | 轻量证据已证明产品合同自相矛盾、Provider 缺少根本能力，或所有候选都必然违反全局约束 | 才停止 Phase A；仅因需要后续实施才能确认的未知不得阻断 |
| B1-S3 | 结构化 carrier 实施后仍无法表达完整 Tool/Hosted 合同，或必须回到 source-only 平行协议 | 停止 TX-06，基于实施证据回到产品决策 |
| B2-S3 | Provider fact 没有真实稳定 ID，或任一绑定无法唯一核对 | 整个 TaskSpace 响应拒绝，零 client/map 执行、零 Map/Store 写入 |
| B2-S4 | Event Store 不能表达一份 fact 对多个节点的幂等引用集合 | 停止 TX-12，不增加旁路 binding database |
| B3-S1 | outer result 无法保留某个原生 Tool 结果 | 停止 TX-13，不以摘要或引用替代事实 |
| C-S1 | 缓存门禁阻断 | 说明变更面并申请专用真实回归预算，不绕过 |
| D-S1 | 确定性矩阵失败 | 不启动任何真实 Whale Agent run |

## 6. 验证与执行约束

- 每个 TX 单元单独实现、测试、提交和推送；不得把相邻单元提前混入。
- TX-06、TX-07、TX-09、TX-12、TX-13、TX-15 是缓存/状态/反馈高风险面，完成后先做定向审计再进入依赖单元。
- 任何 Tool declaration、base instruction 或 provider payload 变化都先执行缓存门禁。
- TX-14 前不宣称新协议可观测；TX-17 前不申请真实样本预算。
- 真实 Whale Agent run 严格遵循全局账本和预算规则；阶段授权不替代付费运行授权。
- 不为实验数据或旧 wire 增加兼容；失败时回退当前单元，不恢复候选方案暗线。
- 外部依据沿用 [`01-upstream-and-feasibility-evidence.md`](01-upstream-and-feasibility-evidence.md) 的 Codex、OpenAI
  Function Calling/Web Search 和 MCP 官方资料；TaskSpace 状态与绑定规则以本产品合同和本地证据为准。
