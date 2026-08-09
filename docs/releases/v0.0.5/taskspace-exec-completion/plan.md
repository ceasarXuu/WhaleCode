# TaskSpace Exec 完整协议与生产闭环计划

- Release: v0.0.5
- Status: planned
- Plan Validity: valid-with-qualifications
- Baseline: `dc876b30e`
- Codex Reference: OpenAI Codex `646f7c0a91b8e327d263335da68ae8ef212895ce` (2026-08-09)
- Product Authority: ./decisions.md
- Applicable Decisions: D1-D13

## Execution Contract

- `decisions.md` 是本专题 active 决策的唯一用户权威；修改任何 active 决策必须获得用户对该项变化的明确批准，Agent 不得自批。
- 已验证的源码、测试和运行证据可以修订本计划的工程顺序与假设，但不得静默改写产品决策。
- 新的重大产品选择必须延后、局部标记 provisional，或先由用户确认；存在 material `provisional`/`conflict` 时不得继续依赖工作。
- 每个 material phase 完成后只审计该阶段的 Product Decision Delta，并标记为 `covered`、`engineering-only`、`provisional` 或 `conflict`。
- 本计划获确认前不继续生产实现；任何真实 Whale Agent run 仍单独遵守预算审批和全局账本规则。

## 1. 目标与非目标

### 1.1 目标

1. 让 Agent 只看到一个结构化 `taskspace_exec`，但在其中完整理解并调用当前有效的普通 Tool、Map 操作和 Hosted 绑定。
2. 让模型可见声明、预检 catalog、原生 Router dispatch 和结果合同来自同一能力快照，消除漂移。
3. 保留普通 Tool 的原生参数、权限、sandbox、hook、并行和结果语义；TaskSpace 只增加序列边界和节点归属。
4. 使失败在正确层表达：结构/硬规则由 Exec 拒绝，普通 Tool 失败保持普通 Tool 失败，Provider-hosted 事实不被伪装成可回滚动作。
5. 用离线确定性门禁先证明协议闭环，再申请最小真实预算验证 DeepSeek 是否稳定遵循。

### 1.2 非目标

- 不实现 Codex JavaScript/V8 runtime、`wait`、cell、yield、store/load、helper globals 或进程 host。
- 不让 Runtime 在 outer Function 返回后代替 Agent 根据结果生成新调用；有结果依赖的下一步必须回到下一次 Provider 推理。
- 不增加 TaskSpace 专属 planning/reasoning 解析、自动节点选择、语义纠错或 prompt 惩罚。
- 不重做 canonical Map、projection policy、持久化或 benchmark；只有发现本方案必需的直接缺口时另立小单元。
- 不为未知未来 Tool 类型预建抽象、兼容层或第二套 Registry。
- 不在本计划编写阶段运行真实 Whale Agent。

## 2. 当前行为与期望行为

| 维度 | 当前基线 | 期望行为 |
|---|---|---|
| 顶层暴露 | TaskSpace 顶层仅 `taskspace_exec` + Provider-hosted Tool，普通 client Tool 已隐藏 | 保持；Standard 完全不变 |
| 输入能力 | Catalog 从 `ToolSpecCapability` 生成 `calls[]` variant，支持 Function/Freeform/Namespace/Tool Search | 从“当前有效能力快照”生成，精确区分 enabled/deferred/hosted，名称无歧义 |
| 外层协议 | `protocol.rs` 集中描述入口、包装、边界和示例 | 保留单一权威，但修正 client 顺序含义并补足 capability/result 合同 |
| Map 操作 | 五个内部 Function variant，描述为一句摘要 | 每项只陈述自身用途、输入、硬前后条件和机械结果，不加入工作建议 |
| 输出合同 | Capability 保存 `output_schema`，但 outer declaration 的 `output_schema` 为 `None` | 同一 catalog 机械生成 Agent 可见 outer result schema；未知输出明确为 unknown，而不是静默丢失 |
| 结果反馈 | `client_results[].response` 序列化 `ResponseInputItem` 传输 envelope | 复用原生 `ToolOutput::code_mode_result()` 的语义值，TaskSpace envelope 只补身份、owner 和 outcome |
| Deferred | Deferred MCP 只有 handler/search source 而不在 catalog；deferred dynamic 反而在 prompt 过滤前进入 outer schema | 与 Standard 的有效暴露/Tool Search 生命周期一致；修复“不可达”和“提前展开”两种相反错误 |
| Namespace | 内层 public name 通过单下划线展平，冲突时整体失败 | 使用无歧义、可逆、由原生 `ToolName` 派生的身份；不沿用 JS identifier 限制 |
| 执行 | 先全批预检，再复用原 Router；client calls 按原生并行策略执行 | 保持；明确 `calls[]` 顺序只约束 Map boundary，不声明普通 work 依赖 |
| 结果依赖 | Outer Function 参数在执行前已完整生成，批内不能根据 A 的真实结果动态追加 B | 无结果依赖动作同批；有结果依赖时返回忠实结果，由 Agent 在下一次请求决定后续动作 |
| Hosted | Response scope 记录真实 item ID/index/type，Exec 逐项声明 `node_ids[]` | 保持并补齐 final-wire/失败矩阵验证；不重执行、不默认归属 |
| 协议重复 | Base instructions 有负向测试，详细合同集中在 Tool description | 扩展为“所有 active provider-visible 固定层”门禁，历史文档不进入扫描 |
| 观测归属 | 内层调用复用 `ToolCallSource::Direct`，trace 把 synthetic call 记成模型顶层直调 | 增加纯机械 TaskSpace requester identity；不改变 Tool 执行、hook 或反馈 |

## 3. Codex 完整职责映射

Codex 参考的是完整 code-mode 链，不是仅参考 `exec` 文案。

| Codex 职责 | 上游位置 | 可复用原则 | TaskSpace 处理 |
|---|---|---|---|
| 有效能力选择 | `core/src/tools/spec_plan.rs::register_code_mode_executors`、`ToolExposure` | 从同一 Registry 的 effective exposure 选择 prompt 与 runtime 能力 | 必须补齐；不得直接把所有 raw specs 等同为首轮 Agent 可见能力 |
| 单一协议渲染 | `code-mode-protocol/src/description.rs` | outer Tool 自包含调用方式，并从当前能力动态生成内层声明 | 已部分具备；继续补结果合同、deferred/namespace 和准确边界语义 |
| Function/Freeform/Namespace 投影 | `tools/src/code_mode.rs` | 机械投影名称、描述、输入和输出，不手写第二份 Tool 清单 | 复用现有 `ToolSpecCapability`，但修正 exposure、命名和结果消费 |
| 名称冲突 | `spec_plan.rs` normalized-name selection | prompt identity 与 dispatch identity 必须一致且确定 | TaskSpace JSON 不受 JS identifier 限制；建立可逆身份并 fail closed 于真实冲突 |
| Deferred 工具 | `ToolExposure::Deferred`、Tool Search guidance | 首轮不展开所有 schema，发现机制与 runtime 可调用集合一致 | 参考原则，不复制上游当前 ToolSearch omission；先验证 Whale Standard 生命周期 |
| 原生分发 | `code_mode/delegate.rs::call_nested_tool` | 嵌套调用回到同一 `ToolCallRuntime`/Router | 已具备主体；继续验证权限、hook、cancel、parallel 与 Standard 一致 |
| 结果转换 | `tools/src/tool_output.rs::code_mode_result` | 由原 Tool output 自己转换模型可见值，避免 transport envelope 泄漏 | 当前明确缺口；改为复用现有同名机制，不新建 TaskSpace 结果语义 |
| 输出类型 | `description.rs::render_code_mode_sample_for_definition` | 同一 `output_schema` 同时参与模型合同 | 当前明确缺口；为 outer Function 机械生成结果 schema |
| Provider/MCP 类型 | MCP shared types、namespace description | 结构化结果和 namespace 说明只在相关能力存在时暴露 | 按当前实际能力条件生成，不写固定清单 |
| 并行 | JS `Promise.all` + 原生 Tool parallel policy | Agent 声明可并行动作，Runtime 仍服从每个 Tool 的原生并行能力 | 只保留原生策略；不把数组先后变成业务依赖 |
| 同 cell 动态续调 | JavaScript 可 `await A` 后根据结果构造 B | 结果依赖必须由 Agent 再推理；Runtime 不拥有续调智能 | 不照搬；结构化 Function batch 只承载预先声明的无结果依赖动作 |
| 兼容性告警 | model `tool_mode` + `code_mode_warning.rs` | 不让明显不支持的模型静默进入不匹配模式 | 不照搬：TaskSpace 是普通 Function Tool，无独立 V8 host；只验证 Provider Function 能力和 final-wire |
| host/cell/wait/yield | Code Mode service、process host、`wait` | 服务长时 JavaScript runtime 的生命周期 | 不适用，不进入 TaskSpace Exec |
| 端到端测试 | `core/tests/suite/code_mode.rs` | 覆盖暴露、嵌套调用、结果、并行、MCP、deferred、fallback | 参考测试维度；注意上游大量测试 mock 模型 call，不能替代 DeepSeek 遵循验证 |

### 3.1 上游已知限制

- 最新 Codex 当前仍有 Tool Search 不能作为 nested code-mode Tool 暴露的公开问题；Whale 已有结构化 `tool_search` variant，不能为“对齐”而删除。
- Codex 的 process-host fallback 与 `code_mode_only` fail-closed 解决的是 JS runtime 可用性，不是 TaskSpace Function Tool 的问题。
- Codex 的 JS 名称归一化、全局 `tools` 对象和 TypeScript 声明是 Freeform runtime 载体，不是 TaskSpace wire；只复用其单一事实源与完整合同原则。

## 4. 已确认缺口全集

| ID | 缺口 | 证据 | 影响 | 归属阶段 |
|---|---|---|---|---|
| G01 | Catalog 从 `client_router.specs()` 建立，尚未证明与当前 effective exposure 完全一致 | `router.rs::into_taskspace` | deferred/隐藏能力可能被首轮展开或出现 prompt/runtime 集合差异 | A, B |
| G02 | `ToolSpecCapability.output_schema` 进入 identity，却不进入 outer declaration | `catalog.rs` 中 capability identity 与 `output_schema: None` | Agent 知道怎么调用，但不知道结构化结果合同 | C |
| G03 | 内层结果以 `ResponseInputItem` 传输 envelope 返回，而不是原 Tool 的 nested result value | `handler.rs::ClientResult.response` | 反馈包含 call transport 结构，且可能弱化 MCP/structured output 的原生语义 | D |
| G04 | Namespace public name 使用单下划线展平 | `nested_tool_public_name` | 身份不可天然反解，合法名称组合可能冲突；这是 JS 约束的错误类比 | B |
| G05 | Deferred MCP 与 deferred dynamic 的 TaskSpace 行为不对称 | Deferred MCP 只注册 handler；dynamic specs 全部进入 base router，TaskSpace 在 prompt 过滤前构建 catalog | MCP 搜索后仍无法通过 decoder；dynamic schema 首轮被提前展开 | A, B |
| G06 | 协议文案把整个 `calls[]` 描述为 Agent-declared order，但 Runtime 不把普通 work calls 顺序当依赖 | `protocol.rs` 与 `dispatch_client_calls` | Agent 可能错误串行化，或误以为数组顺序保证结果依赖 | C |
| G07 | Map operation 描述只有一句摘要 | `map_operations.rs::map_operation_capabilities` | schema 字段存在，但操作角色、硬边界和机械结果不够自包含 | C |
| G08 | 唯一协议权威门禁主要覆盖 Base instructions，尚未覆盖全部 active provider-visible 固定层 | `base_instructions_profile.rs` 测试 | 后续可能在 developer/context 层再次复制详细 wire 合同 | E |
| G09 | 当前离线集成测试证明 Runtime/wire，不证明 DeepSeek 会稳定生成 outer Exec | VA-02 与现有 mock tests | 不能仅凭 60 个离线测试宣布产品行为通过 | F |
| G10 | Code Mode 共启时普通 Tool description 已被追加 JS `exec` 声明，TaskSpace catalog 会原样复制 | `ToolRegistryPlan::push_spec` 先 augment，随后 `into_taskspace` 投影 | TaskSpace 内层 variant 混入另一套调用语法，破坏单一协议权威 | A, B |
| G11 | 内层调用 trace 被记为 `Direct` model call | `handle_tool_call_with_status` 默认 `ToolCallSource::Direct` | rollout/observer 无法结构化区分顶层模型 call 与 outer Exec 内部 call | D |
| G12 | `LocalShell` 使 catalog 构建直接失败，且未证明目标 DeepSeek 配置不会选择它 | `TaskSpaceExecCatalogError::UnsupportedToolSpec` | 某些模型 Tool 配置可能在进入 Provider 前直接终止 TaskSpace | A，必要时 B |
| G13 | 序列规则、Map serde/schema、Hosted 类型、反馈写入/恢复各有手写双点 | `protocol.rs`/`preflight.rs`、`map_operations.rs`、catalog/response scope、handler/settlement recovery | 字段或规则演进可能单边漂移 | C-E |

## 5. 设计

### 5.1 单一能力快照

建立一个 request-local、不可变的 `TaskSpaceExecCatalog` 作为 TaskSpace Exec 唯一能力快照。它必须直接消费原生 Registry 的
effective entries，而不是维护新的注册系统。每项 capability 只包含机械事实：原生 `ToolName`、transport kind、未被其他
surface 改写的原生 description、input schema、output schema 和 exposure/deferred 状态。

同一快照同时驱动：

1. Provider-visible `taskspace_exec` Function declaration；
2. plan decoder 与 preflight lookup；
3. internal ToolName/payload 恢复；
4. capability identity；
5. final-wire 与缓存指纹测试。

不增加第二个 runtime registry。现有 `ToolRouter` 仍是唯一执行注册表。
`capability_identity` 定义为“本次 Agent 可见并可由 Exec 解码的静态合同身份”，不尝试哈希所有 Runtime 行为。parallel policy
继续由同一个 request-local Router 执行，因此不因其未进入 schema identity 而复制执行配置。

### 5.2 名称与 Tool 类型

- Plain Function/Freeform 使用原生名称。
- Namespace Tool 使用从原生 `(namespace, name)` 可逆生成的 structured identity；具体编码先由 A02 用现有 Provider wire 和
  `ToolName` API 证明，不能沿用只为 JavaScript identifier 服务的单下划线规则。
- Tool Search 保留为 client Function capability，并服从 Standard 的 deferred 生命周期。
- Web Search/Image Generation 继续是 Provider-hosted，只在 Provider 顶层声明；Exec 内只出现绑定类型，不复制完整 schema。
- `taskspace_exec`、Codex `exec`/`wait` 和任何会递归进入自身的入口从内部集合中机械排除。
- `LocalShell` 先由 A05 判断是否属于目标 DeepSeek 的实际有效 Tool surface；若存在，复用其原生 payload 构建，不允许静默
  fallback 为另一种 shell Tool，也不因未来可能使用而提前建设。

### 5.3 输入与序列合同

`taskspace_exec` 仍只有：

```text
calls[]
  map operation:    { tool, arguments }
  client function:  { tool, node_id, arguments }
  client freeform:  { tool, node_id, input }

hosted_bindings[]
  { tool, node_ids[] }
```

`calls[]` 的数组位置用于稳定 identity、Map prelude/finish boundary 和结果关联；它不声明普通 client calls 之间的业务依赖。
Runtime 在所有副作用前验证完整批次，但执行 client calls 时只服从原 Tool parallel policy 和 Map 节点可执行状态。

Outer Function Call 在执行开始前已经封闭，因此 Runtime 不可能也不应该在得到内层结果后“继续构造同一个 Exec”。如果 B 的
参数或是否执行取决于 A 的结果，本批只执行 A 并忠实返回结果，Agent 在下一次 Provider 推理中决定 B。只有 Agent 在发起
本批时已判断不存在结果依赖的多个动作，才可同时声明并按原生 parallel policy 执行。

### 5.4 输出合同与反馈

Outer `output_schema` 从同一 catalog 生成固定 envelope：

- `kind/status/outer_call_id/map_id/map_revision_at_dispatch`；
- `reads[]`，其 `map` 使用 canonical Agent-visible Map schema；
- `client_results[]`，包含 call identity、Tool、node、outcome 和原生 nested `result` 或明确 `error`；
- `hosted_results[]`，只包含真实 provider identity、Tool、outcome 和 Agent 声明的 node owners。

`client_results[].result` 必须调用现有 `ToolOutput::code_mode_result()` 或等价的同一原生转换点；不得序列化
`ResponseInputItem` 作为业务结果。Tool 没有结构化 `output_schema` 时，schema 中该 result 为 unconstrained/unknown，实际值仍忠实返回。
大输出、图片、MCP metadata、裁剪和 output reference 继续完全服从 Standard 的 ToolOutput/context 路径。

内层调用在 rollout/trace 中使用纯机械 `TaskSpaceExec` requester identity，携带 outer call、call index 和 node owner。它只修正
观测归属，不参与 Router 授权、执行排序或结果转换。

### 5.5 协议内容分层

- Base instructions：只讲 coding agent 的宏观工作方式，不出现 TaskSpace wire。
- `taskspace_exec` description：唯一详细操作合同，讲 outer entry、包装、序列边界、owner 和最小示例。
- Variant description/schema：只讲该 Tool/Map operation 自身的原生参数、结果和硬条件。
- Runtime error：只报告实际违反项、位置和零执行/已发生事实，不给 Agent 注入下一步策略。

示例必须由生产构造器生成，并反向通过同一 decoder/preflight；不得写 sample 专属命令或无法执行的静态 JSON。

## 6. 最小前置验证

| ID | Critical Assumption | Decision Unlocked | Cheapest Credible Method | Enough Evidence / Not Proven | Budget / Isolation | Stop / Cleanup | Status |
|---|---|---|---|---|---|---|---|
| A01 | `client_router.specs()` 与 effective exposure 在 enabled/deferred/hidden 上是否一致 | Catalog 应消费哪个既有视图 | 静态追踪 Registry plan + 参数化单测，不调用模型 | 能列出每类 Tool 的 provider-visible、nested-visible、runtime-callable 集合；不证明模型遵循 | 离线 | 若需要新 Registry，停止并改为抽取现有 effective view | planned |
| A02 | Namespace 在当前 Responses API 中的原生 identity 能否无损投影进 Exec | 确定可逆编码，不引入 JS normalization | 用现有 Namespace ToolSpec/ToolName fixture 做 encode/decode/collision 测试 | 全部合法 name round-trip 且无歧义；不证明 Provider 偏好 | 离线 | 若 scalar string 不可逆，局部改用 structured identity，不改普通 Tool schema | planned |
| A03 | Standard deferred Tool Search 的真实生命周期能否被 outer Function schema复用 | 确定首轮/后续 declaration 策略 | 静态调用链 + 当前 deferred MCP/Tool Search 测试 | 证明何时 schema 可见、何时 handler 可调用；不证明 DeepSeek 主动搜索 | 离线 | 若同轮动态扩展不可行，保持多轮发现，不提前展开全部 schema | planned |
| A04 | 现有 `code_mode_result()` 能否覆盖所有当前 TaskSpace client output | 决定是否可零新增结果转换器 | 对 Function/Freeform/MCP/Tool Search/image-bearing output 做 table test | 值与 Standard/Code Mode 已有语义一致；不证明长上下文效果 | 离线 | 任一类型缺口先修公共 ToolOutput，不建 TaskSpace 私有语义 | planned |
| A05 | Code Mode augmentation 与 LocalShell 是否会进入目标 TaskSpace surface | 决定是否需要隔离 description 或支持 payload | 组合构建 TaskSpace+CodeMode 与目标 DeepSeek model Tool config | 精确列出实际 declaration 和失败路径；不证明模型行为 | 离线 | 若需要改变目标 Tool 产品面，先停下请用户决策 | planned |

## 7. 可执行工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| U01 | 冻结完整 Codex 对照 | docs | 本 plan | Codex mapping | 固化上游 commit、职责、不适用项和已知限制 | 后续不能用“参考 Codex”笼统扩张范围 | 避免单点模仿 | Complexity: 文档；Reach: 后续计划 | 链接/commit/路径检查 | 文档提交可独立回退 | drafted |
| U02 | 建立 effective capability 事实表 | discovery/tests | Registry/ToolSpec tests | exposure matrix | 执行 A01/A03/A05，只记录当前事实 | enabled/deferred/hidden/hosted/client/LocalShell 分类明确 | 解锁最小 catalog 改造 | Complexity: 测试 fixture；Reach: Tool exposure | 参数化离线测试 | 结论不清即停在 discovery | planned |
| U03 | 证明可逆 Tool identity | discovery/tests | `codex-tools` capability tests | identity projection | 执行 A02 | Namespace/plain 名称无歧义且可回到原生 ToolName | 消除错误展平 | Complexity: 局部编码；Reach: schema/decoder | round-trip + collision tests | 未证明前不改 wire | planned |
| U04 | 证明公共结果转换覆盖 | discovery/tests | ToolOutput/context tests | `code_mode_result` | 执行 A04 | 每类内层结果都有既有公共转换 | 避免 TaskSpace 专属反馈层 | Complexity: 仅测试优先；Reach: output | table tests | 公共缺口另拆小修复 | planned |
| C01 | 让 Catalog 消费 effective view | catalog | `router.rs`, `catalog.rs` | `TaskSpaceExecCatalog::build` | 从现有 Registry effective entries 中性投影，不新建 Registry | declaration/decoder/dispatch 集合一致 | 关闭 G01/G05 基础 | Complexity: 删除 raw-spec 假设；Reach: TaskSpace schema/cache | exposure matrix + Standard zero-diff | 需要架构分叉即回退并停 | planned |
| C02 | 修正 Tool identity | catalog/decoder | capability projection, `plan.rs` | public identity | 用 U03 已证明的可逆编码替换单下划线展平 | Agent identity 与原生 dispatch identity一一对应 | 关闭 G04 | Complexity: wire 破坏性变化、无兼容；Reach: fixtures/cache | encode/decode/collision/final-wire | 不保留旧 alias；失败回退整单元 | planned |
| C03 | 收敛 deferred declaration | catalog | catalog builder | deferred capability selection | 只展开当前有效能力；保留 Tool Search 发现路径 | 首轮不重复暴露 deferred schema | 降低成本并保持可发现性 | Complexity: exposure分支来自现有状态；Reach: MCP/apps | deferred lifecycle tests | 发现能力丢失则停止，不以全展开掩盖 | planned |
| C04 | 隔离 surface-specific Tool description | catalog | Registry projection seam | native capability description | TaskSpace 只消费未追加 Code Mode JS 示例的原生 description | 内层 Tool 只出现一套调用语法 | 关闭 G10 | Complexity: 中性抽取现有字段；Reach: CodeMode+TaskSpace | 组合 declaration snapshot | 若需要手写清洗字符串，停止并回到事实源设计 | planned |
| C05 | 处理目标配置中的 LocalShell | catalog/dispatch | 由 A05 定位 | LocalShell capability | 仅在 A05 证明目标配置实际需要时机械投影与构建原生 payload | TaskSpace 不因目标模型 Tool 类型启动即 fatal | 关闭 G12 | Complexity: 条件性小单元；Reach: shell safety | payload/permission/sandbox parity | A05 证明不适用则标记 not-applicable，不实施 | blocked-on-discovery |
| P01 | 修正序列边界文字 | protocol | `protocol.rs` | sequence contract | 明确数组位置不表达普通 work 依赖，只表达 Map boundary/identity | Agent 不被错误引导串行化 | 关闭 G06 | Complexity: 文案；Reach: cache/tool schema | description snapshot + preflight/parallel tests | 缓存门禁阻断后申请预算，不绕过 | planned |
| P02 | 完善 Map operation 合同 | map capability | `map_operations.rs` | five capability descriptions | 从 canonical operation 定义生成用途、硬条件和机械结果 | Agent 能独立理解每个 Map call | 关闭 G07 | Complexity: 无新语义；Reach: schema token | schema snapshots + operation tests | 出现工作建议/重复 outer rules 即回退 | planned |
| P03 | 生成 outer output schema | catalog/output | `catalog.rs` + schema helpers | `ResponsesApiTool.output_schema` | 从固定 envelope、Map view 和 capability output schema生成 | 输入与结果合同同源 | 关闭 G02 | Complexity: schema构造；Reach: provider payload/cache | per-tool output change drives declaration identity; schema validation | Provider 不支持则停止并拿 wire 证据，不静默删除 | planned |
| P04 | 统一示例构造 | protocol/tests | `protocol.rs` | canonical examples | 示例从 catalog 可用能力选择，并通过正式 decode/preflight | 示例永不描述不存在能力 | 提升协议可靠性 | Complexity: 删除硬编码假设；Reach: tests | fixture capabilities absent/present | 不为 sample 特判 | planned |
| P05 | 建立 schema/规则一致性合同测试 | tests | Map operations/protocol/preflight | parity fixtures | 用同一有效/无效用例同时验证 serde decode、JSON schema 和 preflight；不新建规则 DSL | 手写实现点发生漂移会在离线门禁失败 | 收敛 G13 的必要部分 | Complexity: tests only；Reach: Map wire | field matrix + boundary matrix | 不为追求“单一文件”重构稳定代码 | planned |
| R01 | 保留原 ToolOutput 到 nested result 边界 | dispatch | `parallel.rs`, `taskspace_exec/dispatch.rs` | internal dispatch result | 在不改变 Router/hook 的前提下延后 `into_response`，复用 `code_mode_result` | client result 是原生语义值 | 关闭 G03 | Complexity: 局部返回类型；Reach: CodeMode/Standard必须零差异 | Function/Freeform/MCP/ToolSearch parity | 若需修改普通 Tool handler，停止重新设计 | planned |
| R02 | 固定 outer feedback envelope | handler | `taskspace_exec/handler.rs` | result structs | 用 typed structs 输出 `result/error/outcome`，匹配 P03 schema | 成功、失败、取消、结算错误不歧义 | 稳定反馈层 | Complexity: 破坏旧 TaskSpace wire、无兼容；Reach: observer/tests | schema round-trip + failure matrix | 不改变 Map/node lifecycle | planned |
| R03 | 验证原生执行不变量 | verification | Router/TaskSpace tests | permissions/hooks/cancel/parallel | 对照最新 Codex 测试维度补最小缺口，不重写实现 | TaskSpace nested call 与 Standard 使用同一安全和执行路径 | 防止漏协议 | Complexity: tests；Reach: core | side-by-side deterministic tests | 发现公共缺陷另立单元 | planned |
| R04 | 修正内层调用观测归属 | trace | `ToolCallSource`, dispatch trace | TaskSpace requester | 增加只含 outer/call/node identity 的 TaskSpace source，执行仍走同一 Router | trace 不再把 synthetic call 当模型顶层直调 | 关闭 G11 | Complexity: 一个 enum variant及消费者；Reach: rollout observer | direct/code-mode/taskspace trace fixtures | 不让 source 参与授权或执行逻辑 | planned |
| H01 | 完成 Hosted final-wire 证明 | verification | response scope tests | hosted reconciliation | 覆盖多 item、多 node、失败/取消、漏绑/错绑和 response order | 已发生事实与 Agent owner 声明可机械核对 | 关闭 hosted 不确定性 | Complexity: tests；Reach: provider response | table/fault tests | 不新增 hosted 存储或 fallback | planned |
| H02 | 收敛 Hosted 类型识别 | cleanup | catalog/router/response scope | hosted kind classifier | 从现有 ToolSpec/ResponseItem 建一个最小共享分类边界，三处不再各写名单 | 新增真实 hosted variant 时不会单边漏协议 | 收敛 G13 | Complexity: 删除重复 match；Reach: provider tools | Web/Image正反 fixtures | 不为未知 provider 类型建插件系统 | planned |
| G01 | 建立协议唯一权威门禁 | gate | cache-regression/static checks | active provider-visible sources | 扫描 base/developer/context/tool declaration 构建链的 active 输出源，禁止详细 wire 重复 | 后续不再出现多层协议漂移 | 关闭 G08 | Complexity: 精确 allowlist；Reach: dev loop | 正例/反例门禁，历史 docs 不报警 | 误报普通词汇即收窄，不扩大到全仓文档 | planned |
| G02 | 固定 final-wire/cache 门禁 | gate | cache regression + provider wire fixtures | declaration identity | 对 enabled/deferred/namespace/output 改变建立指纹测试 | 敏感变更被立即发现且原因可定位 | 防缓存回归 | Complexity: fixture更新需解释；Reach: CI | gate source=index + Standard exact diff | 不用 `--no-verify` | planned |
| V01 | 离线总验收 | validation | existing Docker/Rust suites | TaskSpace Exec production chain | 运行最小相关全链，不运行模型 | 设计与 Runtime 生产链闭环 | 进入真实验证前止损 | Complexity: test time；Reach: workspace | focused → workspace → gates | 任一失败不申请真实预算 | planned |
| V02 | DeepSeek 单样本协议复验 | validation | approved Whale runner | VA-02 | 另行登记账本并申请 map-request complex sample repeat=1 | 证明首次 outer Exec、初始化+工作和反馈可用 | 关闭 G09 第一层 | Cost: 真实 API；Reach: 1 sample | trace + request/token/cache/time/cost | 首个结构失败即停，不自动重试 | blocked-on-budget |
| V03 | 多模式行为与成本验收 | validation | Docker benchmark | always/append/request + Standard | 仅在 V02 通过后另行设计并申请预算 | 判断协议在三种 projection policy 下的收益和不可约成本 | 发布决策证据 | Cost: 待单独预算；Reach: benchmark | 预先确认 arm/sample/repeat/stop | 未获批不执行 | deferred |

## 8. Phase 顺序

### Phase A：事实闭环

- Entry: 本计划与 D1-D13 获确认。
- Units: U02-U04（U02 包含 A05）。
- Exit: effective exposure、namespace identity、deferred lifecycle、result conversion 均有离线事实。
- Product Decision Delta: 应全部为 `engineering-only`；任何 material 新选择立即暂停。

### Phase B：单一 Catalog

- Entry: Phase A 全部 direction-supported。
- Units: C01-C04；C05 仅在 A05 证明适用时进入。每个单元独立提交、独立验证。
- Exit: 一个 capability snapshot 同时驱动 declaration、decoder、identity 和 dispatch lookup。
- Product Decision Delta: 必须 covered by D3/D10/D12；若需要第二 Registry 为 `conflict`。

### Phase C：模型可见完整合同

- Entry: Phase B verified。
- Units: P01-P05。
- Exit: 输入、结果、deferred、namespace、Map operation 和示例都自包含且同源。
- Product Decision Delta: 必须 covered by D5/D10/D13；Tool schema 成本变化记录但不以成本删除语义。

### Phase D：结果与执行语义

- Entry: Phase C declaration 固定并通过缓存门禁。
- Units: R01-R04。
- Exit: nested result 不再泄漏 transport envelope，原 Router 安全与并行行为保持。
- Product Decision Delta: 必须 covered by D2/D3/D11；普通 Tool handler 变化视为高风险冲突检查。

### Phase E：Hosted 与门禁

- Entry: Phase D verified。
- Units: H01-H02、G01-G02。
- Exit: Hosted reconciliation、唯一协议权威、final-wire 和缓存检测均闭环。
- Product Decision Delta: 必须 covered by D6/D7/D10。

### Phase F：验收

- Entry: Phase E 离线通过且无 provisional/conflict。
- Units: V01；随后单独申请 V02 预算。V03 不自动启动。
- Exit: V02 通过后才重新盘点 R8 I01-I10 和后续 benchmark。
- Product Decision Delta: 运行证据只能修订工程计划；若与 active 决策冲突必须回到用户决策。

## 9. Pending Product Decisions

当前没有必须在 Phase A 前新增的产品决策。以下工程发现会触发停点，但不得被实现默认为新产品行为：

| Trigger | Why Material | Required Action |
|---|---|---|
| Provider 不接受 outer `output_schema`，且无法保持完整结果合同 | 会在协议完整性与 Provider 兼容之间产生产品取舍 | 提供真实 wire 证据、成本和替代方案，请用户决策 |
| Standard deferred 生命周期无法在单一 outer Function 中复用 | 会在首轮 token 成本与工具可发现性之间产生取舍 | 完成 A03 后请用户决策，不默认全展开或隐藏能力 |
| 可逆 namespace identity 必须改变 Agent-visible结构而非仅编码 | 会改变 Exec public wire | 提供 A02 证据和最小选项，请用户决策 |
| 目标 DeepSeek 配置实际使用不可结构化嵌套的 LocalShell | 会在保持原生 Provider Tool 与单一 Exec 入口之间产生冲突 | 提供 A05 final-wire 证据，请用户决策，不自动 fallback |
| DeepSeek 仍稳定绕过 outer Exec | 可能否定 Function-super-tool 路线，而非一般提示词问题 | 停止实现扩张，提交 trace 后由用户决定主方案 |

## 10. 验证与提交纪律

1. 每个 work unit 一个原子提交并 push；不得把 Catalog、输出、prompt、Map 状态机混在同一提交。
2. 先跑最小相关测试；只有跨 Router/ToolOutput 单元才扩大 workspace 回归。
3. 任何 declaration/context 变化必须先运行：
   `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`。
4. 门禁阻断时按项目规则说明路径、前缀影响和验证理由，再申请真实回归预算。
5. 真实 run 启动前写 `benchmarks/whale-agent-run-ledger.json` planned 记录；结束后立即结算。
6. 本计划不授权任何真实 run，也不授权对抗性审查；两者均按项目规则另行批准。

## 11. Product Decision Delta 模板

| Phase | Decision Surface | Implemented / Observed Semantics | Authority Coverage | Classification | Required Action |
|---|---|---|---|---|---|
| A | discovery only | 待执行 | D3, D10, D13 | engineering-only expected | 记录事实，冲突即停 |
| B | capability exposure/identity | 待执行 | D3, D10, D12 | covered expected | 单元后审计 |
| C | Agent-visible contract | 待执行 | D5, D10, D13 | covered expected | 单元后审计 |
| D | result/dispatch semantics | 待执行 | D2, D3, D11 | covered expected | 单元后审计 |
| E | hosted/gates | 待执行 | D6, D7, D10 | covered expected | 单元后审计 |
| F | Provider behavior | 待执行 | D1-D11 | evidence only | 不以 evidence 静默改 authority |

## 12. 参考资料

- [Codex Exec 协议渲染](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/code-mode-protocol/src/description.rs)
- [Codex effective Tool 注册与 Code Mode 入口](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/core/src/tools/spec_plan.rs)
- [Codex ToolSpec 到内层定义的投影](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/tools/src/code_mode.rs)
- [Codex 嵌套调用委托](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/core/src/tools/code_mode/delegate.rs)
- [Codex 原生 Tool 结果转换](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/tools/src/tool_output.rs)
- [Codex 模型 Tool Mode 合同](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/protocol/src/openai_models.rs)
- [Codex Code Mode 集成测试](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/core/tests/suite/code_mode.rs)
- [Codex deferred Tool Search 当前公开缺口](https://github.com/openai/codex/issues/32101)
