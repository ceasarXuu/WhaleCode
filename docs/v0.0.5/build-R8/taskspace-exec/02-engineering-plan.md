# TaskSpace Exec 正式工程计划

- Created: 2026-08-05
- Status: Planned
- Risk depth: Full
- Prerequisite: [`00-product-contract.md`](00-product-contract.md)
- Evidence: [`01-upstream-and-feasibility-evidence.md`](01-upstream-and-feasibility-evidence.md)
- Issue dependency review: [`03-global-issue-prerequisite-review.md`](03-global-issue-prerequisite-review.md)
- I07 repair authority: [`../I07/00-i07-observability-trust-repair-plan.md`](../I07/00-i07-observability-trust-repair-plan.md)
- Integrates: I10 能力身份、I06 统一 admission、I01/I02/I05 结果合同、I07 新协议观测
- Excludes: projection 三模式、Map 压缩、未列入映射的旧协议独立修复、旧数据兼容

## 1. 目标数据流

```text
TaskSpace provider request
  -> top-level Function taskspace_exec
       -> catalog mechanically derived from native ToolSpec
       -> Agent-authored sequence + node bindings
  -> provider-native hosted capability descriptors

provider response
  -> hosted output facts already executed by provider
  -> taskspace_exec Function call

Runtime
  -> decode exec source into one typed TaskspaceExecPlan
  -> reconcile hosted declarations against raw provider facts
  -> preflight client/map portion before its defined side-effect boundary
  -> dispatch nested client/map calls through the existing ToolRouter
  -> settle canonical Map transaction and binding records
  -> return native results once with mechanical settlement metadata
```

目标不是新建第二套 Tool runtime。新增代码只应包含 TaskSpace 外层 ToolSpec、计划解码、合法性验证、节点绑定元数据和
hosted reconciliation；业务 Tool 定义、权限、sandbox、hook、handler 和结果保持共用。

## 2. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| TX-00 | 修复真实请求与 usage 双计 | observability | I07-W0～W3，详见 [I07 专题计划](../I07/00-i07-observability-trust-repair-plan.md) | normalized request facts 与 request summary | 只按完整请求身份和 completed usage 聚合；无 ID state snapshot 不计请求；重复 ID 一致时去重、冲突时 fail closed | 8 个真实 provider 请求稳定报告为 8 个，状态广播不再伪装请求 | 后续 TaskSpace Exec request/token/cache/cost 证据可复算 | Complexity: 建立最小共享事实分类并修正消费口径，不删除 Runtime/UI 事件；Reach/Cost: 错误历史数字会纠正 | 合成 pair 1->1；历史 8/15 证据重放为 8；身份冲突阻断 | 若历史事件缺少可用 provider ID则 fail closed，不回退为按事件数估算 | planned |
| TX-01 | 固定当前生产与上游 seam | discovery | `core/src/tools`、`tools/src/code_mode.rs`、latest Codex main | ToolSpec、ToolRouter、code-mode nested dispatch、provider output parser | 画出 TaskSpace/Standard 请求、内部 client dispatch 和 hosted response 的当前调用链，并记录可复用函数和必须删除的旧入口 | 后续每个实现点都有唯一落点，不凭旧文档猜测 | 避免覆盖最新上游演进或再建平行 Router | Complexity: 只增加调查文档；Reach/Cost: 阅读 core/tools/provider/session，零运行时影响 | 源码引用、调用图和删除清单可逐项复查 | 证据不完整时停在 discovery | planned |
| TX-02 | 冻结 Function Tool 与 I10 能力身份合同 | API | `codex-tools` ToolSpec builder、TaskSpace Tool projection、provider wire identity | `taskspace_exec` Function ToolSpec 与 canonical capability identity | 生成 `{source:string}` 候选 ToolSpec；内部说明和 capability identity 都从同一 effective ToolDefinitions 快照派生，冻结哪些字段变化必须改变身份 | TaskSpace 只有一个 client/map 顶层入口，声明能力、执行能力、缓存和观测拥有同一版本语义；Standard ToolSpec 不变 | 先消除 I10，防止 Agent 看到的 catalog 与 Runtime 实际执行集合漂移 | Complexity: +1 TaskSpace ToolSpec builder 和一个权威 identity 值，不增加第二 catalog；Reach/Cost: provider payload、缓存指纹、dispatch trace 和 schema tests 受影响 | JSON snapshot、identity mutation matrix、schema size/hash、Standard 0-diff、provider acceptance fixture | 不接生产 projection；任何消费方需要独立重算不同 hash 时停止设计 | planned |
| TX-03 | 证明 source 能无歧义产生 I06 typed plan | API/internal | 新建 Whale 自有 `core/src/tools/taskspace_exec/decoder.rs`，复用 code-mode protocol | `TaskspaceExecPlan` 与 source decoder | 比较受限 JS 声明子集和最小结构化表达，只实现能在副作用前确定 Map/client/provider 三类记录、node binding 和 Tool identity 的最小方案 | Runtime 在执行 Tool 前得到完整、可验证的唯一计划 | 从结构上消除 I06 的内部调用绕过和“边执行边发现序列非法”风险 | Complexity: +1 decoder 与 typed plan；Reach/Cost: 增加模型生成合同和解析测试，不执行真实 Tool | 正反 fixture 覆盖递归、未知 Tool、动态分支、缺绑定和合法并行；同一 source 结果可复算 | 任一方案需要 reasoning 解析、双 manifest 或执行后补计划时停止并回到产品决策 | planned |
| TX-04 | 冻结 I06 副作用前预检边界 | internal | `taskspace_exec/preflight.rs` 与现有 sequence/map validators | client/map sequence validator | 用 TX-03 typed plan 验证 prelude/work/epilogue、revision、节点、单 Patch、Tool identity 和结果依赖拆批规则，明确哪些失败保证整批零执行 | 非法 client/map 序列在定义边界内不会产生半执行状态，任何内部 Tool 都不能绕过同一 admission | 保留 TaskSpace 硬底线，同时不让 Runtime 参与任务语义 | Complexity: +1 preflight 入口并复用现有 validators；Reach/Cost: Map、Patch、全部 nested Tool 类型和 dispatch 合同测试扩大 | 每条规则一正一负 fixture；拒绝时 Tool trace=0、Map revision 不变；不存在 unplanned nested call | 无法在不限制合法结果依赖的情况下预检时暂停，不降级为执行后惩罚 | planned |
| TX-05 | 证明 Hosted 双写身份可逐项核对 | provider contract | provider response fixtures 与 `taskspace_exec/provider_reconcile.rs` | `ProviderFactRef`、reconciliation result | 盘点 Web/Image 等真实 output identity，定义模型可声明且 Runtime 可复算的最小引用，并覆盖 mixed/completed/failed 多项结果 | 每个 provider 事实可判定 exact/missing/duplicate/conflict/unbound | Hosted 动作可进入 Map 而不伪装成 client Tool 或让 Runtime 猜配 | Complexity: +1 reconciliation 数据类型；Reach/Cost: provider adapter、rollout/replay fixture 和日志受影响 | 无网络 fixture 重放；真实 ID 可见性不足时用获批单探针证伪 | 若任何关键 Hosted 类型没有稳定可见身份，停止生产接线并请求产品决策 | planned |
| TX-06 | 接通 I10 单一内部 Tool catalog | internal | `codex-tools`、code-mode descriptor helpers、TaskSpace capability context | shared ToolSpec-to-nested-definition conversion 与 capability identity | 抽取/更新中性派生函数供 code-mode 和 taskspace_exec 共用，让 description、nested Router、provider trace 和 cache gate 携带 TX-02 的同一 identity | 两个超级工具读取同一原生 Tool 合同；TaskSpace 的声明、执行和观测集合可逐身份核对 | 避免 Tool schema 三重暴露、协议漂移和 I10 的跨请求不可比较 | Complexity: 移动一个现有 helper、增加第二调用方和 identity 透传，无新 registry；Reach/Cost: code-mode、ToolSpec、trace 和缓存测试受影响 | code-mode snapshot 不回归；TaskSpace catalog 与 Router 一一对应；任一 identity mismatch fail closed | helper 不能保持 code-mode 行为或需复制 catalog 时回退 | planned |
| TX-07 | 建立独立 TaskSpace Exec handler | internal | `core/src/tools/taskspace_exec/` | Tool executor、request context、recursion guard | 新增 Function handler，接收 source、调用 decoder/preflight/reconciler，并禁止自身和原 `exec` 递归进入 catalog | TaskSpace 协议有单一生产入口，业务 Tool handler 不变 | 将 TaskSpace 复杂性限制在明确边界 | Complexity: +1 handler 模块和一种 Tool runtime path；Reach/Cost: core/tool routing、telemetry 和测试构建时间增加 | handler contract tests 覆盖 payload、取消、超时、递归和空计划 | 未接 provider projection前可整体删除 | planned |
| TX-08 | 通过 I06 统一 admission 执行 client/map 计划 | internal | `taskspace_exec/dispatch.rs`、现有 ToolRouter | bound nested ToolCall metadata | 只接受 TX-04 已预检 plan item，将其机械还原为原 ToolCall，通过同一 Router 执行并保留 `exec_call_id/item_id/node_id/capability_id` trace | 普通 Tool 获得原生权限、hook、sandbox 和结果；节点归属可审计；不存在 plan 外内部 dispatch | 不维护第二套 executor，并关闭 I06 所描述的组合工具绕过入口 | Complexity: +1 dispatch adapter 和 invocation metadata；Reach/Cost: Router、Map transaction、并行/取消及全部 Tool 类型测试受影响 | Function/Freeform/MCP/Namespace/ToolSearch/LocalShell 类型矩阵；plan 外调用零执行；原结果逐字/结构相等 | 任一类型需要修改业务 Tool 参数或绕过 plan 时停下，不加兼容例外 | planned |
| TX-09 | 结算 Hosted 双写 | provider | `taskspace_exec/provider_reconcile.rs`、provider response collector | response-scope fact set 与 Agent records | 对 TX-05 引用做 exact set reconciliation，登记成功绑定并保留冲突/未绑定原事实 | Runtime 可发现错绑、漏绑、重复和伪造，不重执行 provider Tool | Provider 能力进入 TaskSpace 但执行所有权不被改写 | Complexity: +1 response-scope reconciliation pass；Reach/Cost: Web/Image 使用、rollout size 和诊断日志增加少量元数据 | mixed response fixture 覆盖 0/1/N、失败结果、重复、replay 幂等 | 不能 exact reconcile 时保持 unbound 并阻止生产切换，不猜测 | planned |
| TX-10 | 承接 I01/I02/I05 唯一结果合同 | feedback | `taskspace_exec/result.rs` 与 provider final wire | outer FunctionCallOutput | 组合内部原生结果、机械 binding/preflight 状态和唯一最终 revision，不新增 developer carrier，不复用旧 pairing receipt | Agent 一次看到完整执行事实和可继续使用的 Map 版本；已保存状态、失败尝试和普通 Tool 失败互不伪装 | 在新协议中同时消除 stale revision、重复上下文和拒绝语义竞争，不浪费一次旧路径修复 | Complexity: +1 outer result builder并随 TX-13 删除旧 receipt；Reach/Cost: provider wire、上下文、缓存和 I01/I02/I05 测试受影响 | final-wire tests 断言每个 call/result 一次、唯一 continuation revision、错误分类不互换、无 developer 副本、Standard 0-diff | 新结果未满足无损前不切换生产，也不据单测自动关闭三项问题 | planned |
| TX-11 | 建设新协议日志并承接 I07 剩余口径 | observability | I07-W9～W11，详见 [I07 专题计划](../I07/00-i07-observability-trust-repair-plan.md) | exec plan/preflight/dispatch/reconciliation events | 复用 I07 单一 Provider 事实合同，记录 outer call、内部 item、node、原 Tool call ID、provider fact ID、capability ID、零执行范围和结算状态；本地 preflight reject 的 provider delta=0 | 每次失败可定位在生成、预检、执行、绑定或反馈阶段，且 local attempt、boundary request 与 completed response 不再混同 | 支撑问题复盘并完成 I07 在新协议下的观测语义，不建立长期 Observer 专项 | Complexity: 增加少量稳定关联字段但不新增服务或第二套计数规则；Reach/Cost: rollout 存储和分析脚本需同步 | 合成 trace 可复算；同一请求不重复计数；local reject provider delta=0；Standard provider facts 0-diff | 日志身份不能稳定关联时不进入真实评测 | planned |
| TX-12 | 原子切换 TaskSpace projection | integration/cache | turn Tool projection、provider request builder | TaskSpace vs Standard Tool list | TaskSpace 改为 `taskspace_exec + native hosted`，隐藏顶层 client/control；Standard 保持原列表 | Agent 无法绕过超级工具，同时仍可使用 provider 原生能力 | 让协议约束发生在生成入口，而不是事后配对惩罚 | Complexity: +1 mode 分支但删除旧 TaskSpace projection；Reach/Cost: capability hash、provider cache 和所有 TaskSpace 请求受影响 | 缓存指纹门禁、payload snapshots、Standard exact equality、TaskSpace 无 client 顶层泄漏 | 门禁阻断后先说明并申请预算；不得 `--no-verify` | planned |
| TX-13 | 删除旧生产协议 | cleanup | `taskspace_tool.rs`、旧 sequence/sibling preflight/executor、prompt/observer | actions manifest、sibling pairing、Tool decoration | 删除旧入侵字段、旧顶层容器试验入口、control manifest/sibling 配对和兼容 parser，只保留历史文档/fixture | 当前代码只有一个 TaskSpace 动作协议 | 降低维护和回归成本，防止修 A 回归 B | Complexity: 净删除旧分支和状态；Reach/Cost: 旧测试、文档链接和 benchmark parser 需要迁移 | `rg` 删除清单、全量 core/tool tests、无旧 wire snapshot | TX-12 未验证前不执行；失败可回退整个原子切换提交 | planned |
| TX-14 | 确定性集成验收 | test | core/tool/provider/session integration tests | legal/illegal sequence matrix | 覆盖初始化并工作、多节点并行、完成并继续、完成并结束、read-only、一个 Patch、Hosted mixed、取消和 replay | 无付费请求即可证明主要硬合同和失败边界 | 降低真实样本只用于模型行为的成本 | Complexity: 增加聚焦 fixtures；Reach/Cost: CI 时间增加，零 API 费用 | 指定 test filters 全绿，日志与 canonical Store 可复算 | 任一不变量失败即停止真实运行 | planned |
| TX-15 | 小预算真实行为与成本验证 | validation | Docker benchmark + run ledger | Standard / TaskSpace policy arms | 经用户批准后先跑 1 个简单和 1 个复杂样本 repeat=1；只在无新异常时申请扩大重复 | 验证 Agent 能生成 exec、业务完成、缓存和成本可观测 | 判断可行实现是否形成真实产品收益 | Complexity: 不增加生产代码；Reach/Cost: 产生 DeepSeek token、时间和账本记录 | 逐 request trace、map、request/token/cache/time/cost 表；异常逐例归因 | 未获预算不运行；失败不以重试覆盖 | planned |
| TX-16 | 重评 R8 问题全集 | planning | `01-r8-known-issues.md` | I01～I10 | 按新生产 trace 判断哪些自动消失、需改方案、仍独立或新增，不自动关闭 | 后续 R8 回到唯一、证据化的问题队列 | 防止旧根因和旧计划污染新架构 | Complexity: 只更新问题账本；Reach/Cost: 改变后续优先级，无运行时成本 | 每个状态有新协议证据路径 | 没有 E2/E3 证据的问题保持 open | planned |

## 3. 阶段与停点

### Phase 0：修正观测基线

- Entry: I07 的 8 个真实请求被统计为 15 个已有 E1/E2 证据。
- Units: TX-00。
- Boundary: 只修 benchmark 对 TokenCount 事件的请求/usage 消费语义；不修改 Tool、Map、prompt、provider 请求或 UI
  rate-limit 更新。
- Exit: paired completed usage + no-ID rate-limit snapshot 只计一个请求，历史证据可重放为 8 个请求。

### Phase A：消除协议可行性风险

- Entry: 产品合同已确认；TX-00 通过，后续指标不会沿用已知双计口径。
- Units: TX-01～TX-05。
- Stop point A1: TX-03 若无法在副作用前形成完整 typed plan，不能把 Codex 实时 exec 直接接入生产。
- Stop point A2: TX-05 若无法逐项稳定核对 Hosted 双写，不能用顺序、URL 或语义相似度猜配。
- Exit: Function wire、I10 capability identity 合同、I06 source/plan/preflight 边界和 provider identity 均有确定性证据。

### Phase B：构建未接线生产组件

- Entry: Phase A 两个停点均通过。
- Units: TX-06～TX-11。
- Cross-unit side effects: 增加一个 Whale 自有 handler、typed plan 和 reconciliation pass；不改变当前 provider payload。
- Exit: 组件测试证明 I10 单一 catalog identity、I06 统一 admission、I01/I02/I05 唯一反馈、I07 新 trace、原 Tool
  保真、Map 原子性和 Hosted 不重执行。

### Phase C：原子生产切换与清理

- Entry: Phase B 全绿；缓存敏感面变更已被门禁识别并获准验证。
- Units: TX-12～TX-13。
- Cross-unit side effects: TaskSpace model-visible Tool 集合一次变化；Standard 无变化；旧协议同批次退休。
- Exit: 当前源码不存在双协议和兼容 parser，TaskSpace 只有 `taskspace_exec` 入口。

### Phase D：验收与问题重排

- Entry: Phase C 构建和确定性测试通过。
- Units: TX-14～TX-16。
- Cross-unit side effects: 真实验证产生授权成本；失败只形成证据，不自动扩展 Runtime 约束。
- Exit: 新方案的正确性、Agent 使用、反馈、缓存和成本均可复算，R8 问题全集完成重评。

## 4. 主要风险

| Risk | Trigger Signal | Mitigation | Safe Stop / Fallback |
|---|---|---|---|
| Function source 只是把旧容器换名 | source 仍要求第二份 Tool schema或 manifest | Tool catalog 只从 ToolSpec 派生，typed plan 只新增 sequence/binding metadata | 停在 TX-03，不接生产 |
| 实时 nested dispatch 破坏整批 preflight | 前序 Tool 已执行后才发现非法 epilogue | TX-03/TX-04 先冻结声明与执行分界 | 不接受执行后惩罚式校验 |
| Hosted 双写无法 exact match | 模型看不到稳定 ID 或同类多项歧义 | 先用原始 response fixture 和最小真实探针证明 identity | 保留 unbound 原事实并暂停主方案该能力 |
| Tool schema/说明膨胀 | exec description 占据异常 input 比例 | 复用 Codex deferred Tool 与稳定 catalog identity，不做语义压缩 | 以成本证据决定延迟暴露，不删合同 |
| Standard 被 TaskSpace 污染 | Standard payload/hash/test 变化 | projection 严格模式隔离，snapshot 0-diff | 回退 TX-12 原子切换 |
| 旧协议残留形成双事实 | actions/sibling/receipt 仍参与执行或反馈 | TX-13 删除清单和 `rg` 门禁 | 不发布双轨版本 |

## 5. 执行约束

- TX-00 通过前不产生新的 TaskSpace Exec 性能结论；其修复不等于关闭完整 I07。
- Phase A 完成前不写生产 handler，不用更多 prompt 强调替代协议设计。
- 任何真实 Whale Agent run 必须先登记账本并取得对应预算；Phase A 默认只做本地源码和 fixture 验证。
- 每个代码工作单元独立提交并推送，测试失败不得与下一单元混在同一提交中修复。
- 每次代码变更执行聚焦冒烟和邻接回归；生产代码批次完成后询问用户是否执行对抗性审查。
- 不为了保留实验数据实现旧 wire 兼容；主方案失败时回退代码并回到产品决策，而不是暗中启用候选路线。
