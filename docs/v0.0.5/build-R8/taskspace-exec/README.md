# R8 TaskSpace Exec 主方案

- Created: 2026-08-05
- Status: Phase B0～B5 engineering complete / Phase B6 LS-01～LS-08 verified offline / LS-09 Hosted same-response pairing remains unresolved; first-turn combined example failed online
- Priority: Foundation / blocks the existing R8 issue queue
- Scope: TaskSpace 的唯一 client Tool 入口、合法动作序列、节点归属与 Hosted 结果核对

> [`00-product-contract.md`](00-product-contract.md) 是唯一产品决策基线，
> [`12-phase-b-zero-base-plan.md`](12-phase-b-zero-base-plan.md) 是唯一活动工程计划。
> Phase B6 的闭集序列实施单元和停点均直接维护在该计划中，不另建平行路线。

## 1. 路线决策

R8 从本专题起以 `taskspace_exec` 作为 TaskSpace 顶层动作协议的唯一主方案：

1. `taskspace_exec` 是一个 Function Call 形态的单一外层 Tool，复用 Codex `exec/code-mode` 的内部 Tool 暴露、嵌套
   调用和原 Router 复用方式，但不照搬其 Freeform Tool wire。
2. TaskSpace 请求不再向 Agent 顶层暴露普通 client Tool。普通 client Tool 的能力说明由 `taskspace_exec` 从原生
   `ToolSpec` 派生；Map 操作从 canonical Action Map transaction 原语直接定义并作为平级内部 variant 暴露。
3. Agent 在 `taskspace_exec` 内选择一个明确合法的 Map/Tool 顺序形状，并在统一 `tools[]` 中声明 client 或 Provider Tool、
   原生调用内容和节点归属；纯 `update_map` 继续作为受限 Map 动作合法存在。
4. Runtime 对 client Tool 执行机械预检、解析和原生 Tool dispatch；对 Provider Tool 不重执行，也不拆分
   `search/open_page/find_in_page` 等 Provider 内部步骤。每种实际发生的 Hosted capability 在同一响应内只形成一个逻辑
   action；Agent 声明节点归属，Runtime 只核对 capability 集合和节点合法性。
5. `taskspace_exec` 只增加两个 TaskSpace 职责：合法序列和节点绑定。它不规划任务、不选择节点、不解释 Tool 结果，
   也不根据 Tool 成败推进节点状态。
6. `taskspace_exec` schema 是静态能力合同，只提供一组带稳定判别值、有场景证据的合法顺序形状。Agent 不再自由拼装任意
   `calls[]`，但仍自主决定具体序列、节点、Tool、参数和归属；Runtime 收到调用后只机械归一化、预检和路由。
7. 普通 Client Tool 合同从 Standard 顶层迁移到 Exec 内部，只暴露一次；运行时 Map、node、plan、Provider output 和
   Session 状态不进入 Tool declaration。
8. Agent 不回显协议版本、能力快照身份或内部调用 ID；Runtime 从 request-local ToolSpec、outer `call_id` 和数组位置
   机械维护这些关联信息。
9. Agent 在 Ready 节点声明 Tool action 后，Runtime 只机械转为 InFlight；Tool outcome 不自动完成节点。无正向运行证据的
   `blocked` 状态和相关规则从目标模型删除。

旧的普通 Tool schema 入侵、顶层结构化序列容器和 `taskspace_control.actions[] + sibling calls` 三条路线只保留历史
文档证据，active code 直接删除。新方案不维护旧 TaskSpace 可运行性、不增加 adapter 或兼容分支，也不从旧
`taskspace_control` schema/parser/handler 派生新合同。

## 2. 当前事实

- 最新 Codex 主线仍使用一个 `exec` 入口，将 Function、Freeform 和 Namespace Tool 从原 `ToolSpec` 派生为内部
  ToolDefinition，并把嵌套调用送回统一 Tool runtime。
- Codex 主线 `exec` 是 Freeform JavaScript Tool；Whale 已通过本地改造和一次 DeepSeek V4 Flash 真实编码闭环证明
  `{source: string}` Function Call 形态能够进入相同嵌套 Tool 路径。
- 现有证据证明“Function 外层 Tool + 原 Router”可行；Phase B2 已建立结构化 Function 合同、request-local envelope
  和零副作用预检，但尚未接入原 Router 或生产 Provider response lifecycle。
- 2026-08-06 决策取消“旧协议保持运行直到原子切换”的迁移方案。Phase B 先删除旧 sibling/control/response-gate
  影响，再从 Standard 与 canonical Action Map 原语零基础建设新入口。
- Phase A 已证明完整 typed plan、零副作用 preflight 和 Provider response envelope 的可行性。当时的
  Provider `id/item_id` 逐项对账结论已被 LS-09 在线证据取代；现行模型仅按逻辑 Hosted capability 对账。`source:string`
  只保留为被淘汰候选的历史证据；Phase A 已完成，Phase B1 从结构化 Function
  schema 开始实施。
- Phase B0 后确认 `taskspace-canonical-map-v3` 的顶层 edges、action/result/evidence/completion/block/terminal ledger、
  间接状态推导和 Map 专属 ref 均不属于目标模型。最简 Map 已冻结为 Node goal/state/content/parents/children/actions；Agent
  只声明 parents，Runtime 机械反算并始终展示 children，Tool 过程完全复用 Standard。
- Phase B1 `MM-00～MM-10` 已完成。canonical schema、Store、projection、snapshot、CLI/TUI/App Server 和观测消费者
  已统一到最简 Node 模型；旧 v3、edges/ref/ledger/event-replay/detail-fold 和无消费者代码已归零。
- Phase B2 `EX-01～EX-04` 已完成。Map 五项操作、静态 Exec catalog、请求级 revision/identity 和整批预检均有离线证据。
- Phase B3 的 client 原生 dispatch、关系化 canonical Store、Hosted response 对账、唯一 outer 反馈和正式 Router
  入口已经落地。提交 `aba41ff04`、`4d7387a86` 已按 Session producer tracking + 现有 FIFO barrier 修复首轮审查发现的
  cancellation、graceful shutdown 和组合持久化生产链缺口；`24c54333b` 进一步关闭 admission-before-abort、shutdown
  error submission-loop exit 和 pending-turn restart。Focused review 确认 B01/B02 PASS；B03 以三层确定性证据
  qualified closure，不为单体 mega-test 增加生产 hook。未新增持久化队列或产品语义。
- Phase B4 已完成现有事件关联审计、最小字段补齐、缓存敏感面、性能消费、固定离线验收和 I01～I10 离线重映射。
  I10 的 Runtime-only 能力身份已完成离线闭环；I01/I02/I05/I06 仅为静态关闭候选。Phase B5 只使用正式生产路径进行 Provider shape 与
  四臂测量，旧 A2 source-only probe 不得复用。
- Phase B5 的首次获批 VA-02 已在首个响应按结构门禁停止：模型把 Exec 内部 `exec_command` 作为非法顶层 Tool call，
  Runtime 零副作用拒绝。随后对照 OpenAI Codex 2026-08-09 最新主线，确认缺口不是 base prompt 提醒不足，而是 outer
  Tool declaration 只有 schema、没有自包含的操作合同。当前已新增由同一 catalog 生成的唯一 `taskspace_exec`
  protocol description，并用同一首次示例反向通过 decoder 与 preflight；未修改 base instructions、普通 Tool、Router 或
  Runtime 语义。VA-03 仍未启动，VA-02 真实复验需重新申请预算。同期发现的 wire v11 consumer 漂移已由
  `cca76e921` 修复，运行 usage 已从原始 trace 完整恢复。
- Phase B5 CP-01～CP-13 已完成当前依赖顺序中的离线建设和总验收：Catalog、原生 Tool identity、deferred 生命周期、输入/输出合同、
  中性 nested result 和内层请求来源已统一到生产事实源。CP-11 当时的 Hosted 逐项对账已被 LS-09 逻辑 capability
  模型取代。由于 Provider Function Tool 不发送 output
  schema，返回合同按最新 Codex 做法由同一 schema 渲染进唯一 outer Tool description；当前 TaskSpace Tool final-wire 已进入
  免费门禁，Standard 与普通 Tool 不变。冻结的 Core、State、CLI、Viewer、App Server Protocol、workspace、zero-base 和缓存
  合同已整体通过。两轮 VA-02 都证明合法的第二响应可完成 Map 初始化和原生 client dispatch，但首个响应都在空
  `hosted_bindings` 邻近位置生成非法 JSON；第二轮在线观测与账本已正确结算，request 2+ cache hit 为 96.20%。
  当前已将无 Hosted output 时的 `hosted_bindings` 收敛为可省略字段，原有 Hosted 漏绑硬拒绝保持不变；最小真实复验的
  首响应已合法执行。第二响应仍生成了 Provider 未声明的顶层 `exec_command`，Runtime 在零副作用边界拒绝；两次请求的
  顶层 Tool 集合始终只有 `taskspace_exec + web_search`。VA-03 继续阻断，详见
  [`39-phase-b5-va02-revalidation-result.md`](39-phase-b5-va02-revalidation-result.md)。
- TaskSpace 专用完整 base instructions 已在 `standard/structured/source × repeat=3` 历史矩阵中完成在线验证：六次 TaskSpace
  运行没有再次出现顶层 client Tool 逃逸，说明 Standard/TaskSpace 合同冲突修复成立。但 Structured 与 Source 都是
  0/3 业务成功。用户已决定停止 Source，active code 只保留 Structured。父子节点交接合同、当前 Structured observer
  和 Responses 顶层 `instructions` identity 已完成离线修复；outer arguments 的目标模型稳定性仍待新的真实预算验证。完整
  历史逐臂成本与 trace 见
  [`40-va02-source-structured-ab-plan.md`](40-va02-source-structured-ab-plan.md)。
- 最新 `standard/map-request × repeat=1` 复验中，Standard 6 请求成功；TaskSpace 首次 JSON 少一个闭合括号，随后因旧反馈
  未区分 syntax 与 top-level contract，连续四次使用错误 `arguments` wrapper，第 6～8 请求恢复并准确定位业务根因，但在
  patch 前触及请求上限。当前 parser 已离线拆分错误类型并明确 direct `calls` 恢复合同，observer 也能同时计量有效动作与
  协议拒绝；没有 JSON 修补或 Runtime 语义接管。
- 修复后的 `map-request × repeat=1` 在线复验再次在首请求少一个闭合括号，但 Agent 收到新 syntax/direct-calls/零执行反馈后，
  下一请求立即纠正，未再产生 wrapper 放大，I05 在线关闭。Agent 随后完成正确 patch、3 项测试和完整 Map，并在线使用
  canonical parent handoff；最终自然语言回复所需的第 9 次本地请求被批准的 8-request 边界截断，因此端到端结果仍为 partial。
  I03 首请求 JSON、I04 waiting-node 误选与 I07 nested patch 漏计继续开放。
- SR-04 / FF-01 后续预算包完成 3 次有效 `map-request` 复验，全部正确修复并闭合 Map；没有 syntax、wrapper 或顶层
  client 逃逸。三轮参数均原生合法，故只证明生产路径 3/3 稳定，不宣称真实自愈事件已触发。第三轮的两次 waiting
  拒绝准确返回未完成父节点并由 Agent 下一请求纠正，I04 行为仍开放。
- LS-09 Run A 与 Run B 均完成业务、隐藏验证和 Map 闭合，且全部顶层 client 动作保持在 `taskspace_exec` 内；但 Run B
  一次长 `apply_patch` 参数生成了含裸换行的非法 JSON，并两次用 L2 `work` 选择仍为 Waiting 的后继节点，均在零副作用
  边界被拒绝后才恢复。最终 Map 为 5 节点线性链，没有实际覆盖预定的 fork/join 或 Map 调整。Run B 请求 3+ 缓存命中
  92.28%，同一 Tool schema 与 `tool_choice` 全程稳定，因此不构成缓存结构回归。按批准的异常停点暂停 Run C；LS-09
  不能据业务通过宣称协议验收完成。
- Run B 后确认实现缺口不在 Runtime 的八分支解码或 DAG 硬门，而在每个 `anyOf` 分支没有自包含地说明适用条件。
  当前 L1～L8 均已在自身 schema 分支声明选择边界；L2 明确既有 Tool outcome 不会完成 owner，L4 明确只有前置
  Map update 能解锁本批 Tool owner，且本批 Tool outcome 不会解锁后继。72 项 TaskSpace Exec 测试、zero-base 和缓存
  敏感面门禁通过。该记录只证明 Agent-visible 合同已完整落线，不把尚未执行的真实 Provider 复验写成行为收益。
- Run C 暴露的 Hosted action 文字合同已经补入最终 Provider-visible schema，并由 Run D 证明真实生效；但 Agent 仍把
  Hosted action 套用为 client Tool 的执行请求，连续产生字段、生命周期和同响应对账错误。当前证据把根因推进到统一
  `tools[]` 表达模型：client 是执行前请求，Hosted 是执行后归属凭据，两者位于同一动作槽却具有相反生命周期。Run D
  12 requests 后仍未生成业务文件，正式验收失败。用户随后明确 `web_search` 是不可拆分的逻辑 Tool，Provider 内部
  `search/open_page` 不得成为 TaskSpace action。当前已删除逐 output 数量、顺序、ID 和结果暴露，按 capability 形成一次
  声明、一次绑定和一次结果；真实收益等待单次复验。详见
  [`45-ls09-hosted-contract-revalidation-result.md`](45-ls09-hosted-contract-revalidation-result.md)。
- 原生 Hosted Tool 身份已从 TaskSpace 自建名称收敛为当前请求 `ToolSpec::name()` 的机械复用。获批的
  Standard/map-request smoke 均通过；专项 Web Search trace 也确认顶层声明与 Runtime mismatch 只使用 `web_search`，
  `search/open_page/find_in_page` 仅为 `web_search_call` 内部 action。然而专项样本的两个逻辑 Hosted 使用均未在同响应
  生成 Exec 归属，最终虽完成业务和 Map，成功 Hosted 归属仍为 0。身份修复在线成立，I03 同响应双写缺口未关闭；详见
  [`50-native-hosted-identity-live-result.md`](50-native-hosted-identity-live-result.md)。
- 首轮合并示例候选把 client work 与 `already_executed` 登记放入同一个 `initialize_and_work` JSON，Tool description
  比上一阶段减少 126 bytes，离线 75/75 通过；但真实首请求把应由 Provider 产生的原生 `web_search_call` 错写成携带
  `queries` 的顶层 client Function Call。Runtime 零副作用拒绝，计划 repeat=3 在 1/3 后停止。该候选不晋升缓存基线，
  也不继续用文字补丁扩大协议；详见
  [`52-hosted-first-turn-example-stage2-result.md`](52-hosted-first-turn-example-stage2-result.md)。

## 3. 文档

1. [`00-product-contract.md`](00-product-contract.md)：已确认的产品语义、Agent/Runtime/Provider 边界和非目标。
2. [`01-upstream-and-feasibility-evidence.md`](01-upstream-and-feasibility-evidence.md)：最新 Codex 主线事实、本地 Function
   exec 证据和可复用边界。
3. [`02-engineering-plan.md`](02-engineering-plan.md)：Phase A 后的历史计划；其兼容迁移顺序已被零基线决策取代。
4. [`03-global-issue-prerequisite-review.md`](03-global-issue-prerequisite-review.md)：Phase A 时 I01～I10 的历史依赖证据；
   旧 TX 顺序已失效。
5. [`04-phase-a-discovery.md`](04-phase-a-discovery.md)：当前生产、Codex 上游 seam 和旧协议删除清单。
6. [`05-phase-a-result.md`](05-phase-a-result.md)：TX-01～TX-05 实施结果和 A2 纠偏结论。
7. [`06-a2-revalidation-result.md`](06-a2-revalidation-result.md)：A2 既有证据、失效结论和重新打开原因。
8. [`07-a2-multi-node-binding-validation-plan.md`](07-a2-multi-node-binding-validation-plan.md)：A2 逐项多节点绑定的分步验证与完成门禁。
9. [`08-a2-v1-v3-result.md`](08-a2-v1-v3-result.md)：V1～V3 的 wire、候选合同和原子拒绝离线证据。
10. [`09-a2-v4-first-probe-result.md`](09-a2-v4-first-probe-result.md)：首次真实 V4 probe 的失败事实、测试混杂因素和复验前置修正。
11. [`10-a2-v4-v3-reprobe-result.md`](10-a2-v4-v3-reprobe-result.md)：v3 修正后复验、Agent 可见性证据与 source-only 合同承载阻塞。
12. [`12-phase-b-zero-base-plan.md`](12-phase-b-zero-base-plan.md)：当前唯一有效的 Phase B 工程计划、最简 Map 重建顺序和
    旧协议/旧 Map 净删除边界。
13. [`13-mm01-old-map-deletion-inventory.md`](13-mm01-old-map-deletion-inventory.md)：旧 Map 生产调用链、保留职责和逐文件净删除清单。
14. [`14-phase-b1-minimal-map-result.md`](14-phase-b1-minimal-map-result.md)：MM-02～MM-10 实施、测试、缓存门禁和工程收益证据。
15. [`15-phase-b2-exec-contract-result.md`](15-phase-b2-exec-contract-result.md)：EX-01～EX-04 的合同、预检、离线验收和剩余边界。
16. [`16-phase-b3-ex05-native-dispatch-result.md`](16-phase-b3-ex05-native-dispatch-result.md)：EX-05 原生 client dispatch 证据。
17. [`17-phase-b3-relational-store-result.md`](17-phase-b3-relational-store-result.md)：MS-01～MS-02 关系化 Store 证据。
18. [`18-phase-b3-execution-feedback-result.md`](18-phase-b3-execution-feedback-result.md)：MS-03、EX-06～EX-08 的生产执行、
    Hosted 对账、唯一反馈和 B3 总验收。
19. [`19-phase-b4-observability-audit.md`](19-phase-b4-observability-audit.md)：OB-01A～OB-01B 现有事件、身份断点和最小补齐结果。
20. [`20-phase-b4-cache-surface-result.md`](20-phase-b4-cache-surface-result.md)：OB-02A 最终声明构建链、缓存敏感面和正反门禁证据。
21. [`21-phase-b4-performance-observer-result.md`](21-phase-b4-performance-observer-result.md)：OB-02B R8 Exec 动作、I07 成本事实和跨层身份消费证据。
22. [`22-phase-b4-offline-acceptance.md`](22-phase-b4-offline-acceptance.md)：VA-01 Docker、Rust、CLI/Viewer、Standard 与门禁的固定离线验收证据。
23. [`23-phase-b4-issue-remap-result.md`](23-phase-b4-issue-remap-result.md)：VA-04A 对 I01～I10 的当前源码重映射、证据边界和 B5 观察点。
24. [`../I10/00-i10-capability-identity-repair-plan.md`](../I10/00-i10-capability-identity-repair-plan.md)：B5 前置的 Runtime-only 能力身份闭环。
25. [`24-phase-b5-va02-first-result.md`](24-phase-b5-va02-first-result.md)：首次正式 Provider shape 验证、实际成本、结构失败和停点。
26. [`25-phase-b5-protocol-authority-repair.md`](25-phase-b5-protocol-authority-repair.md)：最新 Codex `exec` 对照、协议单一权威修复和离线验证。
27. [`26-phase-b5-cp01-effective-surface-result.md`](26-phase-b5-cp01-effective-surface-result.md)：DeepSeek、deferred、Hosted、Code Mode 和 LocalShell 的 effective surface 事实矩阵。
28. [`27-phase-b5-cp02-tool-identity-result.md`](27-phase-b5-cp02-tool-identity-result.md)：原生 `ToolName` 往返、当前 Namespace 扁平别名碰撞证据和最小 wire 决策停点。
29. [`28-phase-b5-cp03-result-conversion.md`](28-phase-b5-cp03-result-conversion.md)：Function、Freeform、MCP、Tool Search、错误与大输出的公共结果转换覆盖和 CP-09 实施约束。
30. [`29-phase-b5-cp04-effective-capability-result.md`](29-phase-b5-cp04-effective-capability-result.md)：同一 Registry 条目的 Provider/native ToolSpec 双视图和 TaskSpace effective Catalog 接入证据。
31. [`30-phase-b5-cp06-tool-identity-result.md`](30-phase-b5-cp06-tool-identity-result.md)：结构化 Namespace wire、原生二元身份查找和旧扁平 alias 零兼容证据。
32. [`31-phase-b5-cp09-nested-result.md`](31-phase-b5-cp09-nested-result.md)：公共中性 nested result、失败透传、MCP/Tool Search/Patch/output-reference 忠实反馈证据。
33. [`32-phase-b5-cp05-deferred-lifecycle.md`](32-phase-b5-cp05-deferred-lifecycle.md)：首轮隐藏、自然历史恢复、dynamic/MCP 展开与失效能力 fail-closed 证据。
34. [`33-phase-b5-cp07-input-contract.md`](33-phase-b5-cp07-input-contract.md)：Map 边界与 Work 依赖分离、canonical 操作说明和可反解示例证据。
35. [`34-phase-b5-cp08-output-contract.md`](34-phase-b5-cp08-output-contract.md)：typed outer result、canonical Map read、原生 nested result 与模型可见返回合同的同源证据。
36. [`35-phase-b5-cp10-dispatch-requester.md`](35-phase-b5-cp10-dispatch-requester.md)：Direct、Code Mode 与 TaskSpace 内层 Tool requester 的机械身份和 rollout 回放证据。
37. [`36-phase-b5-cp11-hosted-reconciliation.md`](36-phase-b5-cp11-hosted-reconciliation.md)：Hosted 类型同源分类、真实 Provider output 与 Agent 多节点归属的逐项核对证据。
38. [`37-phase-b5-cp12-final-wire-gate.md`](37-phase-b5-cp12-final-wire-gate.md)：详细协议单一权威、TaskSpace Tool final-wire 和精确缓存敏感面证据。
39. [`38-phase-b5-cp13-offline-acceptance.md`](38-phase-b5-cp13-offline-acceptance.md)：CP-01～CP-12 生产链、workspace、zero-base 与缓存合同的离线总验收。
40. [`39-phase-b5-va02-revalidation-result.md`](39-phase-b5-va02-revalidation-result.md)：第二轮与零 Hosted 正式复验、顶层 Tool 越界、在线观测结算与缓存证据。
41. [`40-va02-source-structured-ab-plan.md`](40-va02-source-structured-ab-plan.md)：已结束并封存的 structured/source 单变量实验、选型证据与后续离线修复结果。
42. [`41-phase-b5-sr04-ff01-revalidation-result.md`](41-phase-b5-sr04-ff01-revalidation-result.md)：UTF-8 自愈与忠实反馈修复后的三轮在线稳定性、waiting 观察及预算结算。
43. [`42-phase-b5-schema-compression-result.md`](42-phase-b5-schema-compression-result.md)：TaskSpace Exec 固定合同单变量压缩、SC-01 两轮在线结果和预算结算。
44. [`43-closed-legal-sequence-design.md`](43-closed-legal-sequence-design.md)：Phase B6 闭集合法顺序的场景证据、L1～L8、
    统一 Tool action、Ready 启动、纯 Map update 和 blocked 移除决策。
45. [`44-ls09-run-c-result.md`](44-ls09-run-c-result.md)：分支适用合同修复后的 Provider-hosted 真实验收、请求路径、成本、
    Map 中断状态和 Hosted action 操作合同缺口。
46. [`45-ls09-hosted-contract-revalidation-result.md`](45-ls09-hosted-contract-revalidation-result.md)：Hosted 最小操作合同修复后的
    真实复验、逐请求失败链、成本和统一 Tool action 表达冲突。
47. [`46-ls09-logical-hosted-revalidation-result.md`](46-ls09-logical-hosted-revalidation-result.md)：逻辑 Hosted Tool 聚合的真实
    通过证据、同响应归属遗漏后的不可恢复断层、成本与产品决策停点。
48. [`47-ls09-hosted-execution-direction-result.md`](47-ls09-hosted-execution-direction-result.md)：`already_executed` 执行方向
    在线生效、业务闭环和残余提前登记/漏登证据。
49. [`48-ls09-same-response-pairing-repeat5-result.md`](48-ls09-same-response-pairing-repeat5-result.md)：同响应双写合同修复后的
    五轮统计、缓存与成本、原生 Web 动作映射缺口和合法序列缺口。
50. [`49-ls09-indivisible-pairing-partial-result.md`](49-ls09-indivisible-pairing-partial-result.md)：不可拆分文字合同的三轮失败、
    协议误读和预算停点。
51. [`50-native-hosted-identity-live-result.md`](50-native-hosted-identity-live-result.md)：原生 Hosted ToolSpec 身份修复的三次
    获批真实运行、缓存成本、Web Search 原始 trace 与剩余同响应漏登结论。
52. [`51-hosted-error-priority-stage1-result.md`](51-hosted-error-priority-stage1-result.md)：Hosted mismatch 优先级的离线与在线阶段证据，以及确定性复合门禁。
53. [`52-hosted-first-turn-example-stage2-result.md`](52-hosted-first-turn-example-stage2-result.md)：首轮 client/Hosted 合并示例的
    单变量真实失败、成本和顶层 client Function Call 误读证据。

## 4. 推进规则

- R8 已知问题队列继续暂停，直到该主方案完成生产接入并重新盘点 I01～I10。
- 唯一允许在 Phase A 前实施的全局问题子范围是 TX-00：修复 I07 已坐实的 usage/request 聚合错误；它不改变 Tool、
  Map、prompt 或 provider 行为。
- 旧三类方案不得保留 active runtime、schema、parser、adapter 或兼容 fixture；历史文档只作证据，不得作为实现依赖。
- 每个阶段只验证一个主要不变量；涉及 provider/Agent 行为的真实运行必须重新申请预算。
- Tool declaration、prompt 或 provider payload 发生变化时，先运行缓存敏感面门禁，再说明变化并申请真实缓存回归。
- 生产代码变更完成后按项目规则另行申请对抗性审查；本次只建立路线合同和工程计划。
