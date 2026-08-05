# TaskSpace Exec 正式工程计划

- Created: 2026-08-05
- Refined: 2026-08-06 after Phase A
- Status: Phase A verified / Phase B ready
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
  -> OutputItemDone(taskspace_exec call 1)  -- Agent 声明 plan + node
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
5. Agent 只声明一个 `hosted_node_id`；Runtime 从同响应真实 output item 读取 `id/item_id`。
6. 同响应 Hosted facts 只能归属一个节点；多个节点的 Hosted 工作拆分响应。
7. Provider Tool outcome、client Tool outcome和节点生命周期互不推导。
8. Agent 合同/Map preflight 失败时，尚未发生的 client/map 零执行；Provider 已发生事实始终保存。
9. Provider 身份缺失或重复只影响对应事实的绑定，不取消其他合法 client/map 动作。
10. 每个 Tool 结果只进入 Agent context 一次，不新增 developer/system factual carrier。

## 3. 工作单元

`Execution Status` 使用 `verified-isolated` 表示实现和定向测试已通过、但尚未接入生产路径；它不等于 integrated。

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Execution Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| TX-00 | 修正请求/usage 双计 | observability | I07-W0～W3 | canonical request facts | 按 completed request identity 聚合，排除无 ID rate-limit snapshot | 8 个请求稳定报告为 8 个 | 后续成本证据可复算 | Complexity: 修正共享消费口径；Reach/Cost: 历史错误数字变化 | 历史 8/15 重放为 8 | 身份缺失时 fail closed | verified |
| TX-01 | 固定当前生产与上游 seam | discovery | `core/src/tools`、最新 Codex main | ToolSpec/Router/response lifecycle | 记录可复用入口和旧路径删除清单 | 后续实现有唯一落点 | 避免沿过期架构继续开发 | Complexity: 文档；Reach/Cost: 零运行时影响 | 源码引用逐项复查 | 证据不足停在 discovery | verified |
| TX-02 | 冻结 Function Tool 与能力身份 | API | `taskspace_exec/catalog.rs` | ToolSpec snapshot/capability ID | 从同一 ToolSpec 快照派生 outer 描述、内部 catalog 和身份 | 声明、执行、缓存、观测共享能力版本 | 关闭 I10 的根源入口 | Complexity: 候选 catalog；Reach/Cost: 未接生产 | 5 个 catalog tests | 不注册 ToolSpec | verified-isolated |
| TX-03 | 形成唯一 typed plan | API/internal | `taskspace_exec/decoder.rs` | `TaskspaceExecPlan` | 只接受 `taskspace.plan(<strict JSON>)` | 副作用前获得完整计划 | 不解析 reasoning，不边执行边发现非法 | Complexity: 一个严格 decoder；Reach/Cost: Agent 生成合同增加 | 正反 decoder fixtures | 动态表达需求出现时回到产品决策 | verified-isolated |
| TX-04 | 冻结纯输入预检 | internal | `taskspace_exec/preflight.rs` | structural preflight | 校验版本、能力、调用身份、边界、递归和单 Patch | 明确错误在 client dispatch 前发生 | 建立 I06 零执行底线 | Complexity: 候选 validator；Reach/Cost: canonical Map 尚未接入 | 结构矩阵 tests | 不复制 Map 状态机 | verified-isolated |
| TX-05 | 验证 Hosted 身份与节点声明 | provider | `taskspace_exec/provider_reconcile.rs` | Provider fact collection | Agent 只声明节点，Runtime 读取真实 Provider ID | Hosted 事实无需 Agent 复写身份 | 消除第二事实源和延迟绑定 | Complexity: 候选 reconciler；Reach/Cost: 尚未接 response lifecycle | A2 probe、replay/Store、25 个 exec tests | Provider 无稳定 ID 时保持 unbound | verified-isolated |
| TX-06 | 接通唯一内部 Tool catalog | internal/cache | 上游 `spec_plan/ToolExposure` seam、code-mode helper | neutral ToolSpec conversion | 对齐最新上游 seam，让 code-mode 与 taskspace_exec 共用转换和 capability ID | 两个超级工具不复制 Tool 合同 | 防止 schema 三重暴露和能力漂移 | Complexity: 移动一个共享 helper，无新 registry；Reach/Cost: code-mode snapshot、缓存指纹受影响 | code-mode 0-diff；catalog/Router 一一对应；缓存门禁 | 需要第二 registry 时停止 | not-started |
| TX-07 | 收集响应级事实而不建全局状态 | response lifecycle | `session/turn.rs` | response-local `TaskspaceExecEnvelope` | 在 `OutputItemDone` 收集 Hosted items 和唯一 outer call，在 `response.completed` 冻结并消费 | executor 同时得到真实 Provider facts 和 Agent plan | 解决 Hosted/Function sibling 数据归属，不重放重建 | Complexity: +1 局部 envelope 和完成事件参数；Reach/Cost: response loop、stream fixtures 受影响，零持久状态 | 0/1/N Hosted、乱序完成、缺/多 outer call、stream abort；Standard 0-diff | envelope 需要写 Session 或跨响应时停止 | not-started |
| TX-08 | 建立 outer response executor | internal | `taskspace_exec/response_executor.rs`、现有 response sequence入口 | outer call decode/output pairing | 消费 TX-07 envelope，解码一次、拒绝多个 outer call、保留原 outer `call_id`，禁止递归 | `taskspace_exec` 有单一 Runtime 入口且能返回原生配对 output | 把特殊性限制在必要的响应边界 | Complexity: +1 response executor，不注册第二业务 Router；Reach/Cost: tool sequence入口和取消路径受影响 | outer call pairing、decode failure、cancel/timeout、零内部 dispatch | 需要普通 Tool handler读取全局 envelope 时回退 | not-started |
| TX-09 | 接入 canonical Map admission | state | `taskspace_exec/map_admission.rs`、现有 Action Map validator/transaction | prepared Map transaction | 将 typed plan 的 Map calls、node bindings、revision 和 client reservations一次交给现有 validator 准备 | Agent 合同和 DAG/状态硬规则在任何 client dispatch 前确定 | 不复制状态机并关闭 I06 绕过 | Complexity: +1 adapter，净复用 canonical transaction；Reach/Cost: Map revision、reservation tests 扩大 | init/reopen/complete/finish、stale、unknown/not-ready node、零提交拒绝 | 任一规则需复制到 exec 时停止 | not-started |
| TX-10 | 机械执行 client/map 计划 | internal | `taskspace_exec/dispatch.rs`、`ToolRouter` | admitted nested `ToolCall` | 从 TX-09 prepared plan 还原原生 payload，携带 outer/item/node/capability metadata，经原 Router 执行 | Function/Freeform/MCP/Namespace/ToolSearch/LocalShell 保持原权限、hook 和结果 | 普通 Tool 无 TaskSpace 侵入且所有入口统一过门 | Complexity: +1 invocation adapter，无第二 executor；Reach/Cost: 并行、取消、Patch、全部 Tool 类型测试受影响 | 类型矩阵、单 Patch、并行独立 calls、结果依赖拆批、plan 外调用零执行 | 任一 Tool 需修改原生 args 时停止 | not-started |
| TX-11 | 按失败矩阵结算 Hosted facts | provider | `taskspace_exec/provider_reconcile.rs`、TX-07 envelope、TX-09 admission | response-scope reconciliation | 用真实 item ID 和 Agent `hosted_node_id` 生成 bound/unbound/missing/duplicate 事实；不读取 outcome 推进节点 | 已发生 Provider 动作得到忠实归属或明确未绑定状态 | Provider 能力进入 Map，不重执行、不猜配 | Complexity: 扩展现有 reconciler；Reach/Cost: Web/Image response fixtures 和错误反馈增加 | 0/1/N、failed、缺节点、缺/重复 ID、invalid plan、prelude-created node | 无法直接取得 ID 时仅受影响事实 unbound | not-started |
| TX-12 | 复用 Event Store 持久化与幂等 | data | `action_map/event_store.rs`、现有 canonical persistence | provider item event owner | 增加按 `provider_item_id` 查重和 owner 冲突检查；bound 记 Node，unbound 记 Root，不建新表 | restart/replay 直接读取固化事实，不重建 Map 或 envelope | 保持全局唯一 Map 和单一持久事实源 | Complexity: 扩展现有 Event Store API，无新数据库；Reach/Cost: snapshot/replay/compaction tests 受影响 | same-owner replay 幂等、cross-owner conflict、Web/Image round-trip、SQLite restore | 现有 Store 无法表达时先停下，不加旁路账本 | not-started |
| TX-13 | 返回唯一无损结果 | feedback | `taskspace_exec/result.rs`、provider final wire | outer FunctionCallOutput | 组合内部原生结果、Hosted settlement、失败来源和唯一 final revision；删除额外 factual carrier依赖 | Agent 一次看到完整事实和可继续 revision | 承接 I01/I02/I05，减少歧义与缓存污染 | Complexity: +1 result builder，切换后删除旧 receipt；Reach/Cost: context/final-wire snapshots受影响 | 每个结果一次、原文/结构保真、状态/Tool/Provider错误不互换、Standard 0-diff | 任一原生结果丢失时不接 projection | not-started |
| TX-14 | 建设可复算日志 | observability | I07-W9～W11、response/dispatch trace | exec response correlation | 记录 provider request/response、outer call、item、node、provider item、capability、dispatch和settlement身份 | 一次失败可定位在生成、解码、Map、Provider、dispatch或反馈 | 完成 I07 新协议证据链 | Complexity: 复用现有 trace schema并增加关联字段；Reach/Cost: rollout 和报告脚本同步 | 合成 trace 重算；local reject provider delta=0；无重复 request/token | 需要第二 observer 数据源时停止 | not-started |
| TX-15 | 原子切换 TaskSpace projection | integration/cache | request Tool projection、provider builder | TaskSpace effective Tool list | TaskSpace 一次改为 `taskspace_exec + native hosted`，Standard 不变；同批启用 TX-07～14 | Agent 生成入口天然符合新协议 | 停止旧 sibling 的事后惩罚式配对 | Complexity: +1 mode projection，替换旧分支；Reach/Cost: 所有 TaskSpace 请求和缓存受影响 | 缓存门禁、payload snapshots、Standard exact equality、TaskSpace 无 client泄漏 | 门禁阻断后说明并申请专用预算 | not-started |
| TX-16 | 删除旧生产协议 | cleanup | old `taskspace_tool`、sequence/sibling、prompt/observer | actions manifest/pairing/decoration | 删除旧 schema 入侵、control manifest、sibling preflight/executor、receipt 和兼容 parser | 生产代码只有一个 TaskSpace 动作协议 | 防止双事实和修 A 回归 B | Complexity: 净删除分支；Reach/Cost: 旧测试和 benchmark parser 迁移 | `rg` 删除清单、全量 core/tool tests、无旧 wire snapshot | TX-15 未通过前不执行；原子提交可整体回退 | not-started |
| TX-17 | 确定性集成验收 | test | core/tool/provider/session integration tests | legal/illegal response matrix | 覆盖初始化并工作、多节点独立调用、完成并继续/结束、read-only、单 Patch、Hosted mixed、取消、replay | 付费运行前证明硬合同与失败边界 | 降低真实样本只用于模型行为的成本 | Complexity: 增加聚焦 fixtures；Reach/Cost: CI 时间增加，零 API 费用 | 指定矩阵全绿、Store/日志可复算、Standard 0-diff | 任一不变量失败即停止真实运行 | not-started |
| TX-18 | 小预算真实行为与成本验证 | validation | Docker benchmark、run ledger | Standard + TaskSpace policies | 获批后先跑简单/复杂各一次；异常先归因，不自动扩大 repeat | 验证 Agent 能生成 exec、任务完成、缓存和成本可观测 | 判断可行实现是否形成真实产品收益 | Complexity: 不增加生产代码；Reach/Cost: 产生 DeepSeek token、费用和时间 | 逐 request trace、Map、request/token/cache/time/cost 表 | 未获预算不运行；失败不以重试覆盖 | not-started |
| TX-19 | 重评 R8 问题全集 | planning | `01-r8-known-issues.md` | I01～I10 | 只依据新生产 trace 更新关闭、改方案或新增状态 | R8 回到唯一证据化问题队列 | 防止旧根因继续污染路线 | Complexity: 文档状态变更；Reach/Cost: 改变后续优先级 | 每项有新协议证据链接 | 无 E2/E3 证据保持 open | not-started |

## 4. 阶段与停点

### Phase 0：观测基线

- Units: TX-00。
- Exit: 已 verified；历史 8/15 双计已可重放为 8。

### Phase A：可行性与合同

- Units: TX-01～TX-05。
- A1: typed plan 能在副作用前完整形成，已通过。
- A2: Runtime 直接复用 Provider `id/item_id`，Agent 只声明节点，已通过。
- Exit: 候选组件 verified-isolated；未接生产。

### Phase B1：共享能力与响应入口

- Entry: Phase A 完成。
- Units: TX-06～TX-08。
- Exit: 单一 catalog、response-local envelope 和 outer executor 均有确定性测试；没有 Session 全局临时状态。
- Cross-unit side effects: code-mode helper 和 response loop 受影响，但 Standard wire 不变。

### Phase B2：Map、执行与 Hosted 持久化

- Entry: B1 通过。
- Units: TX-09～TX-12。
- Exit: canonical Map admission、原 Router dispatch、Hosted settlement 和 Event Store restore 形成离线闭环。
- Cross-unit side effects: Map transaction、event owner 和全部 nested Tool 类型测试扩大；不新增数据库。

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
| B1-S2 | response envelope 必须写 Session 全局状态或跨响应恢复 | 停止 TX-07，不接受隐式生命周期 |
| B2-S1 | exec 需要复制 Map 状态规则 | 停止 TX-09，只允许 canonical validator adapter |
| B2-S2 | 某类 client Tool 必须修改原生参数才能携带 node | 停止 TX-10，不做 Tool schema 入侵 |
| B2-S3 | Provider fact 没有真实稳定 ID | 仅该事实保持 unbound；不得生成 ordinal/内容指纹 |
| B2-S4 | Event Store 不能表达幂等 owner | 停止 TX-12，不增加旁路 binding database |
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
