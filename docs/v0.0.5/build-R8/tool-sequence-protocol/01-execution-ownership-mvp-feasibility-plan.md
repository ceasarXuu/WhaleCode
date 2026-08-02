# Tool 序列执行归属最小可行性测试计划

- Created: 2026-08-02
- Status: Executing；MVT-0、MVT-1 completed，下一项 MVT-2
- Scope: 只验证“单一序列容器 + client/provider 执行归属”基础路线
- Excludes: 完整生产 schema、旧协议迁移、真实 Agent 行为收益和全量 provider 兼容

## 1. 要提前消除的风险

TaskSpace 要求 Agent 只提交一个有序 Tool 序列，Runtime 在任何动作发生前检查完整序列，然后把每个合法序列项交给
原有 Tool 能力。普通 Tool 可由本地 Runtime 执行，但 `image_generation` 等 provider-hosted Tool 会在 provider
生成响应时直接执行；若仍在主请求顶层暴露，Runtime 收到结果时已经无法执行前拦截。

本计划不先构建完整方案，而是用一个最小垂直切片回答六个会否决路线的问题：

1. 单一容器中的普通 Tool 能否无损回到现有 `ToolRouter`，而不是维护第二套 handler？
2. 非法序列能否保证本地和 hosted 动作都为零执行？
3. hosted 动作能否只由已通过预检的 Work 项触发，而不是由主 Agent 响应提前触发？
4. 本地与 hosted Work 能否保持唯一声明、唯一节点归属和唯一结果配对，并只按 Map 的 ready frontier 调度？
5. TaskSpace 的变化能否不改变 Standard 的 Tool schema、`tool_choice` 和 provider 请求结构？
6. hosted 子请求能否与主 Agent turn 隔离，不重放主上下文、不写入额外 reasoning/message，也不破坏主请求缓存链？

任何一项需要 sibling manifest、shadow call、reasoning 解析、Runtime 猜节点或修改普通 Tool 参数才能通过，均视为路线失败，
不能用补偿逻辑掩盖。

## 2. 已有事实与可复用基建

| 能力 | 现有位置 | 当前事实 | Spike 用法 |
|---|---|---|---|
| Tool 目录 | `tools/src/tool_spec.rs`、`tool_registry_plan.rs` | `ToolSpec` 是原生能力事实源 | 从相同 spec 生成测试序列项，不手写第二份普通 Tool schema |
| 嵌套调用 | `tools/src/code_mode.rs`、`core/src/tools/code_mode/mod.rs` | Code Mode 已把 Function、Freeform、Namespace/MCP 还原到原 Router | 复用转换和 payload 构造，不复用 V8/JS 运行时 |
| 序列预检 | `core/src/tools/sequence_preflight.rs` | 能在分派前拒绝当前 sibling 序列 | 用容器中的单一数组替代 manifest 配对输入 |
| 序列调度 | `core/src/tools/sequence.rs` | 已支持 barrier、并行段、失败后跳过 | 保留调度语义，只替换单项执行入口 |
| 本地执行 | `core/src/tools/parallel.rs`、`registry.rs` | `ToolCallRuntime` 最终进入 handler、hook、权限和 sandbox | Client 测试项必须走这条真实路径 |
| Provider 传输 | `codex-api/src/endpoint/responses.rs` | 已有 Auth、HTTP/SSE 和请求记录测试基建 | 用 mock Responses endpoint 验证 hosted wire |
| Hosted 结果 | `protocol/src/models.rs` | 已能解析 `WebSearchCall`、`ImageGenerationCall` | 不新造结果语义，只增加序列项关联 |
| Web Search | `core/src/web_tools/handlers.rs` | 已有 client-managed `WebSearchHandler` | Web Search 不用于证明 hosted adapter 必要性 |

已确认缺口：当前 `ToolCallRuntime` 将序列项硬编码到 native Tool；`ToolChoice` 不能表达 hosted selector 或
`allowed_tools`；`codex-api` 没有 Images endpoint。Spike 只补足能验证路线的最小部分，不提前建设通用 hosted 插件系统。

## 3. 最小测试对象

测试使用一个固定动作批次，避免为了测试目标编造复杂业务。Tools 容器不是 Work 的第二套 DAG：它只保证 Map
操作位于批次边界，Work 的依赖和并行关系只来自 Map。

```text
batch S1
  map prelude：初始化 root、N1、N2、N3、finish
  work item-1 / node-N1 / client test tool：写入事件 local-A
  work item-2 / node-N2 / hosted image stub：向 mock provider 请求一次并写入事件 hosted-B
  work item-3 / node-N3 / client test tool：写入事件 local-C

map
  root -> N1 -> finish
  root -> N2 -> finish
  root -> N3 -> finish

negative batch S2
  map：root -> N1 -> N2 -> finish
  同批声明 N1、N2 Work；N2 尚未 ready，整批零执行拒绝
```

S1 中 N1/N2/N3 属于同一 ready frontier，可并发执行；容器不得额外声明 `N1 -> N2` 等 Work 顺序。S2 用于证明
Runtime 不会因容器位置绕过 Map readiness，也不会根据 N1 的 Tool 结果自动替 Agent 完成节点并解锁 N2。

这不是在证明图像质量，也不要求模型理解任务。它只客观验证控制流：唯一声明、执行前预检、执行归属、Map readiness、
失败传播和结果配对。hosted stub 返回协议合法的固定结果；本地项使用现有测试 Tool 或最小记录型 handler，不得调用 shell
制造无关环境变量。

测试结构暂放在现有 Rust 测试体系：

- 序列和本地分派：`third_party/codex-cli/codex-rs/core/src/tools/` 下的相邻 `#[cfg(test)]` 模块；
- provider wire：`third_party/codex-cli/codex-rs/core/tests/suite/`，复用 `core_test_support::responses` 和 WireMock；
- provider 请求类型：`third_party/codex-cli/codex-rs/codex-api/src/common.rs` 相邻单元测试。

Spike 不新建 crate、不引入依赖、不增加配置开关，也不接入生产 CLI。

## 4. 待验证假设与判定阈值

| ID | 假设 | 最小证据 | 通过阈值 | 否决信号 |
|---|---|---|---|---|
| H1 | 普通 Tool 可从容器还原到原生执行路径 | `item-1`、`item-3` 进入真实 `ToolRouter`，保留原参数和 handler 结果 | 无第二 handler、无 Tool 参数 decoration、现有 hook 测试继续通过 | 必须复制 handler 或修改普通 Tool schema |
| H2 | 完整预检发生在所有执行之前 | 对 S1 制造非法 revision、非法 node 和第二个 Patch 变体，并执行依赖后继未 ready 的 S2 | 每个非法变体均为本地 0 次、provider 0 次、Map 0 提交 | 任一动作先发生再被拒绝 |
| H3 | 混合 Work 只有一个声明和依赖事实源 | 合法 S1 的事件日志、provider 请求和结果数组均带 `sequence_id/item_id`；调度读取 Map ready frontier | 三项均执行且结果逐项一一对应；不要求 Work 完成顺序；S2 的依赖只来自 Map | 容器再声明 Work 依赖、按名称/位置猜配，或忽略 Map readiness |
| H4 | Hosted 失败不扭曲后续状态 | mock 分别返回明确失败、断流后结果未知 | item-2 分别为 `failed`、`outcome_unknown`；item-3 为 `not_executed`；无自动重试 | 把未执行伪装成失败，或自动重试收费动作 |
| H5 | 主 Agent 请求不会提前触发 hosted Tool | 捕获 TaskSpace 主请求和 hosted 子请求 | 主请求只含序列入口；hosted Tool 仅出现在预检后的专用请求 | 主请求仍暴露 `image_generation` 或出现 shadow call |
| H6 | Standard wire 不受影响 | 对 spike 前冻结的同一 Standard fixture 比较序列化请求 | tools、tool schema、tool choice、parallel flag 和输入保持相同 | Standard 引入容器、TaskSpace 字段或 provider 分支 |
| H7 | Hosted 子请求不成为新的 Agent turn | 比较执行前后的主 history、request chain 和 mock 子请求 body | 子请求不带主 `previous_response_id`，不产生额外 assistant reasoning/message；主 history 只收到对应序列结果 | 复用完整 `ModelClientSession::stream`、重放主上下文或改变主缓存链 |

## 5. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| MVT-0 | 冻结可比较的请求基线 | API/cache | `core/tests/suite/` provider request fixture | Standard 与当前 TaskSpace 请求 body | 在任何生产代码变化前，用现有 request recorder 固化 Tool 列表、schema hash、tool choice、parallel flag、instructions/input hash | 后续每项请求变化都有明确基线 | 防止用改动后的快照自证“Standard 没变化” | Complexity: 复用现有 4 份本地 fixture，无运行时状态；Reach/Cost: usage 与单臂退出合同由 `0076e720a`、`c2246a6f1` 修复 | 双臂业务/usage 完整，用户接受当前三份 TaskSpace final-wire；Standard 不变，见 9.1～9.4 | accepted baseline 已绑定完整证据；后续变化重新走缓存门禁 | completed |
| MVT-1 | 证明原 Router 可承接容器内普通 Tool | internal | `core/src/tools/nested_call.rs`、`code_mode/mod.rs`、`router_tests.rs` | nested payload 构造与 `ToolRouter` | 抽取不依赖 Code Mode 的原生嵌套调用构造器，并让 Function/Freeform 两项记录型 Tool 依次走真实 Router | 容器项可复用现有 registry、handler、hook 和结果转换；普通 Tool schema 不变 | 排除“序列容器必然要求第二套 Tool 实现”的基础风险 | Complexity: 移动一个已有 helper 并增加测试；Reach/Cost: 无 provider wire、prompt 或运行时网络变化 | H1 通过；构造器 1/1、Router 7/7、dispatch trace 3/3、缓存门禁通过，见 9.5 | 若必须复制协议或 handler，回退 helper 变更并判定路线暂停 | completed |
| MVT-2 | 证明执行归属可以位于 ready Work 的单项分派边界 | internal | `core/src/tools/sequence.rs`、`router.rs` 相邻测试 | 记录型 client/hosted adapter 与真实序列调度器 | 用测试专用 adapter 将 S1 三项 ready Work 交给同一 Router/调度器；用 S2 验证依赖后继不会因容器位置提前执行 | Map prelude 先提交；同一 ready frontier 可混合 client/hosted 并发；Work 结果按 call/item identity 配对 | 排除新建平行调度器和第二套 Work DAG 的必要性 | Complexity: 只增加测试构造入口和 fake adapter，不进入 CLI；Reach/Cost: 只增加 core 测试编译时间 | H3；S1 三项执行且结果一一配对，不断言 Work 完成全序；S2 零 adapter 调用、Map 0 提交 | 若必须复制 scheduler、修改普通 Tool schema，或容器必须再声明 Work 依赖，删除 spike 并暂停路线 | planned |
| MVT-3 | 证明非法序列对两类执行均零副作用 | state | `core/src/tools/sequence_*tests.rs` | preflight 到 dispatcher 的调用边界 | 为 revision、node、Patch 规则各造一个非法 S1，使用共享事件计数器和 mock 请求计数 | 所有拒绝都发生在 client/provider 分派之前 | 证明 hosted adapter 不会削弱状态机底线 | Complexity: +3 负例 fixture，无新生产状态；Reach/Cost: TaskSpace 序列回归测试时间增加 | H2；三个 fixture 均断言事件 0、HTTP 0、revision 不变 | 任一测试出现先执行后拒绝，停止后续 hosted 设计 | planned |
| MVT-4 | 证明受限 hosted provider wire 可以机械构造和解析 | provider API | `codex-api/src/common.rs`、`core/tests/suite/` | hosted tool choice、Responses request、SSE parser | 仅实现 mock 所需的 provider-neutral hosted selector，将 WireMock executor 接入 MVT-2 同一 dispatcher，发出只含一个 hosted Tool 的请求并解析固定结果 | hosted 调用由序列项触发，provider 请求无其他可选 Tool | 提前发现当前 API 类型、认证/传输或返回 parser 是否阻断路线 | Complexity: 扩展一个通用请求枚举和测试，不增加 Images endpoint；Reach/Cost: Responses 序列化测试受影响，无真实 API 费用 | H5 的 hosted 子请求部分；匹配 `tools`、`tool_choice`、`parallel_tool_calls=false` 并解析一个 `ImageGenerationCall` | 若 provider wire 只能通过重进完整 Agent turn 或主请求顶层调用，回退并判定该 hosted 路径不可行 | planned |
| MVT-5 | 证明 hosted 执行不污染主 Agent 会话 | context/cache | `core/tests/suite/` request recorder 与 history fixture | hosted executor 的 provider client 边界 | 用独立机械请求执行 item-2，比较主 history、response chain 和子请求字段 | hosted 执行只产出 item-2 结果，不制造额外 Agent turn 或动态主上下文 | 避免序列执行重新退化成隐藏的线性 Agent 请求并破坏缓存 | Complexity: +1 history/request-chain fixture，不新增产品状态；Reach/Cost: provider client 构造与缓存测试受影响，无真实 token 成本 | H7；断言无主 `previous_response_id`、无额外 reasoning/message、主请求基线仍可追加 | 若只能复用完整 Agent turn，停止 hosted 方案并评估直接能力 API | planned |
| MVT-6 | 证明失败和未知结果不会触发重复副作用 | failure | `core/tests/suite/` hosted mock fixture | hosted execution outcome | 分别模拟确定失败和响应中断，记录尝试次数与后续 item 状态 | 失败忠实返回，未知保持未知，后续动作跳过且调用次数固定为 1 | 防止重复收费和错误的成功/失败语义污染 Map | Complexity: +2 provider 故障 fixture 和一个 outcome 枚举原型；Reach/Cost: 错误处理测试增加，无生产重试策略 | H4；每个 fixture provider 请求恰好 1 次、item-3 未执行 | 若现有 transport 强制自动重试非幂等调用，暂停并先解决 retry ownership | planned |
| MVT-7 | 证明 TaskSpace 隔离且 Standard 不回归 | API/cache | provider request fixture、缓存敏感面门禁 | TaskSpace/Standard model-visible Tool 集合 | 捕获两种模式请求，断言 TaskSpace 只有容器、Standard 与 MVT-0 基线相同，并执行缓存敏感面门禁 | 模式差异只位于 TaskSpace Tool 投影和分派入口 | 避免为新协议再次牺牲 Standard 行业兼容与缓存稳定性 | Complexity: +2 request snapshot；Reach/Cost: provider request 和缓存门禁测试增加，无真实 token 成本 | H5、H6；`check_cache_regression_gate.py --source index` 通过或准确阻断待授权验证 | Standard fixture 有变化时回退所有共享请求构造变更 | planned |
| MVT-8 | 形成可执行的路线决策 | documentation | 本专题目录 | spike 结果文档 | 汇总每项证据、未证明事项和代码净增量，按第 7 节矩阵作出单一结论 | 完整工程设计只基于已验证边界展开 | 防止把局部 mock 成功误报为 provider/产品全链路成功 | Complexity: +1 结果文档，不增加运行时概念；Reach/Cost: 后续设计必须引用该结论 | 所有 H1-H7 有证据路径且无模糊的“基本通过” | 证据不足时状态保持 blocked，不进入正式实施 | planned |

## 6. 执行顺序与安全停止点

### Phase A：本地结构可行性

- Entry condition: 仅运行本地 Rust 单测，不需要 provider 凭据。
- Work units: MVT-0、MVT-1、MVT-2、MVT-3。
- Phase-local evidence: 原 Router 复用、唯一顺序、非法序列零执行。
- Next-phase condition: H1-H3 全部通过；否则停止，不建设 hosted wire。

### Phase B：Mock Provider 可行性

- Entry condition: Phase A 通过。
- Work units: MVT-4、MVT-5、MVT-6、MVT-7。
- Phase-local evidence: 捕获的 provider 请求体、SSE 结果、主会话隔离、失败尝试次数和 Standard snapshot。
- Cross-unit side effects: provider 请求类型的扩展可能进入缓存敏感面，必须执行现有门禁；不进行真实 Whale Agent run。
- Next-phase condition: H4-H7 全部通过且 Standard 无变化。

### Phase C：结论

- Entry condition: Phase A/B 都有完整证据，或其中一项触发否决信号。
- Work units: MVT-8。
- Phase-local evidence: 单一结论、代码净增量、剩余 provider-specific 未知项。
- Next-phase condition: 只有“结构可行”才能开始完整工程设计。

## 7. 决策矩阵

| 结果 | 结论 | 后续动作 |
|---|---|---|
| H1-H7 全部通过 | 基础路线可行 | 基于已验证 seam 设计正式序列 schema、持久化结算和 hosted binding |
| H1-H7 通过，但后续最小真实 probe 被特定 provider 拒绝 | 序列架构可行，该 provider 的 hosted capability 暂不可用 | 保留 TaskSpace 能力缺失的明确事实，不退回顶层调用；其他 Tool 继续设计 |
| H1 或 H2 失败 | 基础路线不可行 | 停止本专题工程扩展，重新讨论容器与预检边界 |
| H3 需要第二份 Work 依赖或归属事实 | 产品约束被破坏 | 否决该实现，不接受 sibling manifest、shadow call、容器内 Work DAG 或事后猜配 |
| H6 失败 | 隔离设计不成立 | 回退共享请求构造改动，先重新划清 Standard/TaskSpace 边界 |
| H7 失败 | Hosted 执行会污染主 Agent turn | 不接受受限 Responses 子请求，优先评估直接能力 API；没有直接 API 时隐藏该能力 |

## 8. 暂不进行的真实 Provider Probe

Mock 测试能证明 Whale 的结构和 wire 构造正确，但不能证明某个线上 provider、模型或认证方式实际接受 hosted selector。
Phase A/B 通过后，如正式范围确实需要该 provider-hosted capability，再单独申请一次最小真实 API probe：

- 只测一个 provider、一个 hosted Tool、一次请求；
- 不启动 Whale Agent sample，不进入自然语言任务循环；
- 启动前说明模型、预计 token/费用、最长耗时和停止条件；
- 只验证请求是否被接受并返回一个预期 hosted item，不评价生成质量；
- 失败只说明该 provider binding 不可用，不推翻已经通过的 client sequence 架构。

当前 DeepSeek 主路径没有暴露原生 `image_generation`，因此真实图像 probe 不是 Phase A/B 的前置条件。

## 9. 执行记录

### 9.1 MVT-0 请求基线

2026-08-02 在未修改生产代码前复用现有最终 wire fixture 检查基线。Standard 基线稳定；TaskSpace
当前实现与受保护 fixture 不一致，MVT-0 尚未完成。

| 模式 | 当前 accepted fixture SHA-256 | 本地 mock 候选 SHA-256 |
|---|---|---|
| Standard | `d5808bdd792f343716cab7e79cf902ddeff7e155161d9e36e956cad9ee264b86` | 无变化 |
| Map Always | `2cf6090311979b38089c2402a48617c1df7cebdab516bc88216678e6afe1f9d5` | `e355ee1d827604ef3e826e5235f9bd2843ca9d12c4d58b549013d8af2cad933a` |
| Map Append | `76fc55757bfb9df4bd03fa32e1ed753abdae25ba8820425d57c5dec3ba849d47` | `6dc4c18cd9d22f838e59c83ef53de7cf6540e2b18ead19cbd4e383c0011ec61b` |
| Map Request | `8ad4f318e1d9886656cc2b3675e294ee6efe622bdf69c0437959e39d8a8df2f1` | `7fde706ebfee95158ef421132f67271caad3e2e451a1f9f813782f3d19a2f733` |

验证结果：

- `standard_request_pair_preserves_the_complete_prefix`：通过，原 fixture 无变化。
- `taskspace_projection_policies_have_independent_request_pairs`：本地 mock 候选通过；候选使用单一
  `TaskSpaceResponseResultV2` 反馈，替代 accepted fixture 中的 commit + developer receipt 双反馈。
- 缓存门禁：`BLOCKED`。expected surface 为
  `9651121f653277a6919dd97d549ecbfd5f812ef64267d2561cd873de95888bdc`，候选 actual surface 为
  `204978af2218017fe58f2322973b2a605cce6ec16e1e91f92ef4386aa1e3926b`；基线状态为
  `live_regression_failed`，必须先取得专用真实回归预算并走独立晋升流程。
- 候选快照已回退，受保护的 accepted fixture 保持不变。
- 本阶段未启动真实 Whale Agent 或外部 provider 请求。

### 9.2 MVT-0 首次真实运行与测量修复

用户批准 Standard 与 map-request 各一次。运行 `WAR-20260802-165454-CACHE-REGRESSION-2723DE14` 只完成
Standard：业务验证通过，5 次 provider 请求的真实 usage 为 60,617 input、48,128 cached、810 output，request 2+
命中率为 97.0812%。旧 runner 把 rollout 中 9 条重复累计快照当成 token 真值，证据合同拒绝后按停止条件没有执行
map-request。

提交 `0076e720a` 完成两项修复：provider terminal 成为 usage 唯一事实源，并在 provider-route 前复用现有
binary-health。原始 artifact 离线重算和 219 项缓存回归均通过，staged cache gate 的 provider 上下文指纹未变。
本次授权已消费，MVT-0 仍保持 blocked；取得新预算前不得补跑或进入 MVT-1。详细结果见
[`../cache-regression/19-provider-terminal-usage-repair.md`](../cache-regression/19-provider-terminal-usage-repair.md)。

### 9.3 MVT-0 第二次真实运行与单臂退出合同修复

第二次获批运行 `WAR-20260802-180016-CACHE-REGRESSION-2E8B3F50` 完成 Standard：业务通过，6 次 provider
请求的真实 usage 为 74,555 input、72,960 cached、1,293 output，request 2+ 命中率为 97.5422%。usage、boundary、
预算和清理证据均完整。

底层 benchmark 使用双臂 E2 资格生成最终退出码，因 `RunSide=left` 按计划跳过右臂而返回 1；缓存 runner 将其判为
run failure，按授权停止条件没有启动 map-request。提交 `c2246a6f1` 只让缓存专用单臂命令声明允许非 E2 结果，选中
arm 的业务、usage、预算和清理门禁保持不变。219 项缓存回归与 staged cache gate 通过。

本次授权已消费，不能在修复后补跑。MVT-0 继续 blocked，MVT-1 未开始。完整根因、费用和证据见
[`../cache-regression/20-single-arm-exit-contract-repair.md`](../cache-regression/20-single-arm-exit-contract-repair.md)。

### 9.4 MVT-0 双臂通过与基线接受

修复后运行 `WAR-20260802-181842-CACHE-REGRESSION-7A794B3A` 完成两臂：Standard 与 map-request 业务均通过，
provider 请求分别为 7/8，request 2+ 命中率分别为 97.90%/67.85%，usage 覆盖完整。用户明确接受该结果作为当前
MVT-0 基线；promotion 只更新三种 TaskSpace final-wire 快照，Standard 快照不变。

map-request 的一次零执行 state rejection 和重复反馈继续归入 R8-I04/I05/I02，不影响基线事实完整性，也不因
promotion 自动关闭。详细结果见
[`../cache-regression/21-mvt0-accepted-baseline-result.md`](../cache-regression/21-mvt0-accepted-baseline-result.md)。

MVT-0 状态更新为 completed；下一项为 MVT-1，且只运行本地测试。

### 9.5 MVT-1 原 Router 复用证明

提交 `228c68ff8` 把 Code Mode 内已有的 Function/Freeform payload 构造逻辑抽取到
`core/src/tools/nested_call.rs`。新入口只把已知 `ToolSpec`、原始名称、call id 和 JSON 输入构造成 `ToolCall`；它不执行
Tool、不解释 TaskSpace、不修改普通 Tool 参数，也不复制 handler。Code Mode 改为调用该入口后，仍通过原有
`ToolCallRuntime -> ToolRouter -> ToolRegistry` 链路执行。

记录型测试注册一个同时接受 Function/Custom payload 的原生 handler，将 `inspect({path})` 与 freeform `patch(text)`
依次交给真实 `ToolRouter`。断言得到的调用顺序、名称和 payload 分别为
`inspect/{"path":"README.md"}`、`patch/patch body`，证明未来容器项无需第二套 Tool 实现即可复用原分派路径。

验证结果：

- `nested_call_builder_preserves_native_tool_identity_and_payload_kind`：1/1 通过；
- `tools::router::tests`：7/7 通过；
- `dispatch_lifecycle_trace`：3/3 通过，原 dispatch trace 仍有效；
- 缓存门禁：通过，指纹 `a0e06b82dc2c7eab23ecbf4a07b980fd913971e54780dadce4e2af6154faf84c`，无需真实回归；
- 未运行真实 Whale Agent 或外部 provider 请求。

完整 `code_mode` 邻接筛选为 38/39：`code_mode_notify_injects_additional_exec_tool_output_into_active_context`
缺少 notify marker。该测试已在改动前基线 `b3913f965` 的 detached worktree 中以相同症状复现，因此不是 MVT-1
回归；问题如实保留，不计入 H1 通过证据，也不在本单元扩展修复。

MVT-1 状态更新为 completed；下一项为 MVT-2，只在测试范围验证同一调度器能按显式执行归属处理
client/provider-hosted 项，仍不进入生产 CLI。

## 10. 外部依据

1. [OpenAI Agents SDK：Hosted tools 与 local/runtime tools 的执行边界](https://openai.github.io/openai-agents-python/tools/)
2. [OpenAI Responses API：`tool_choice` 与 allowed tools 合同](https://platform.openai.com/docs/api-reference/responses-streaming/response/web_search_call?lang=curl)
3. [OpenAI 官方 OpenAPI 规格](https://github.com/openai/openai-openapi)
4. [wiremock-rs 官方仓库](https://github.com/LukeMathWalker/wiremock-rs)

这些资料只支持 provider 执行归属、请求约束和 mock 检查方法。TaskSpace 的 Map、节点归属和序列合法性仍由本项目产品
约束定义，不能借外部 SDK 的 Agent 编排语义替代。
