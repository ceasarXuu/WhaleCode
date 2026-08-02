# TaskSpace Tool 序列容器正式工程计划

- Created: 2026-08-02
- Status: Drafted / plan authoring
- Scope: 将已验证的 Tool 序列执行部件接入生产 TaskSpace 请求与执行链
- Risk depth: Full，涉及 model-visible Tool API、Map 事务、provider 执行归属、反馈和缓存
- Prerequisite: [`00-product-definition.md`](00-product-definition.md) 与
  [`01-execution-ownership-mvp-feasibility-plan.md`](01-execution-ownership-mvp-feasibility-plan.md)
- Excludes: 旧 wire/旧数据兼容、Map 压缩策略、projection 三模式重构、Agent 业务任务拆分策略

## 1. 工程问题与目标

当前生产 TaskSpace 仍让模型生成一个顶层 `taskspace_control` 和若干顶层普通 Tool call，并由
`taskspace_control.actions[]` 重复声明普通调用的 Tool 名和节点归属。Runtime 收到响应后再配对两份事实。这条路径能
在执行前拒绝错误，却不能让模型直接生成同形的合法行动单位，并且仍把 provider-hosted Tool 暴露在预检之前。

目标路径是：

```text
TaskSpace model request
  -> model-visible tools 只有 taskspace_tools
  -> Agent 提交一个 taskspace_tools 外层调用
  -> Runtime 解码容器项并保留 Agent 声明的 item_id/node_id
  -> 完整批次结构与 canonical Map 预检
  -> client-managed item 进入现有 ToolRouter
  -> provider-hosted item 进入受控 hosted executor
  -> 原生结果逐项结算到 Map
  -> 一个容器结果返回逐项事实和唯一 canonical revision
```

Standard 不进入该路径，继续使用 Codex 原生顶层多 Tool 调用、Tool schema、`tool_choice`、并行执行和结果配对。

## 2. 已证事实与仍需发现的边界

| 主题 | 当前证据 | 工程结论 |
|---|---|---|
| Function/Freeform 原生调用 | MVT-1，`228c68ff8` | 可从外层输入还原为原生 `ToolCall` 并复用同一 Router/handler/hook |
| Map Ready frontier | MVT-2，`3791c873c` | Work 依赖只读取 canonical Map；容器不得增加第二份 Work DAG |
| 非法批次原子拒绝 | MVT-3，`e7dcf3c9d` | 完整预检必须位于所有 client/hosted dispatch 之前 |
| Hosted 机械请求 | MVT-4，`102b74dd7`、`b6168c782` | 测试 executor 已证明可构造隔离的单能力 Responses 请求；尚未证明生产 capability、auth 或兼容端点 |
| 主会话隔离 | MVT-5，`118b20c55` | Hosted 子请求无需成为 Agent turn，也不应携带主 history/response chain |
| Hosted 失败语义 | MVT-6，`749df97b8` | 测试 executor 在 `max_attempts=1` 下可区分明确失败与结果未知；生产逐请求 retry override 仍需 TS-03 证明 |
| Standard 隔离 | MVT-7 | Standard 两请求 final-wire 基线通过，生产 TaskSpace 容器入口尚未接入 |
| Namespace/MCP | 现有 Code Mode 只证明 schema 展开，未纳入 MVT 生产容器 | TS-01 必须确认名称、输入与 Router 还原路径 |
| ToolSearch/延迟 Tool | 未验证 | TS-02 必须确认能否作为 client-managed 单项容器调用；不能沿用 provider 顶层执行假设 |
| LocalShell/Web/Image | MVT 只证明测试专用 Image adapter 的 transport 可行 | 每类执行输入与 provider 支持必须单独确认；未确认时从 TaskSpace 容器能力集中隐藏 |
| DeepSeek Tool flags/schema | 官方 V4 文档显示不同兼容端点对 `tool_choice`、禁并行和 strict schema 的支持不同 | TS-03 必须按现有 model/provider capability 生成请求；Runtime 不能依赖 provider flag 代替完整预检 |
| `taskspace_control` 执行路径 | 当前 initialize/execute/reopen 由 response preflight 直接提交，handler 明确拒绝这些 action | TS-04 必须找出统一 Tool 分派 seam；不得把旧绕行路径直接搬进容器 |

MVT 的测试 adapter 不是生产协议。尤其 `mvt_hosted_image({prompt})` 是用于证明执行归属的测试能力，不能直接改名成为
`image_generation` 的产品输入合同。

## 3. 最小目标合同

### 3.1 Model-visible 外形

正式名称暂定为 `taskspace_tools`。它是 TaskSpace 主请求唯一的顶层 Tool；名称在 TS-06 合同冻结后不得在实施中随意
修改。

以下示例表达目标结构，不代表手写每个原生 Tool schema：

```json
{
  "items": [
    {
      "item_id": "map-1",
      "tool": "taskspace_control",
      "input": {
        "action": "execute",
        "expected_revision": 12,
        "mutations": [
          {"action": "complete_node", "node_id": "inspect"}
        ]
      }
    },
    {
      "item_id": "work-1",
      "node_id": "inspect-code",
      "tool": "exec_command",
      "input": {"cmd": "rg -n prepare_taskspace_response core/src"}
    },
    {
      "item_id": "work-2",
      "node_id": "inspect-tests",
      "tool": "exec_command",
      "input": {"cmd": "rg -n taskspace_control core/tests"}
    }
  ]
}
```

合同约束：

1. `item_id` 由 Agent 声明，在一个容器内唯一；Runtime 用 `outer_call_id/item_id` 形成内部调用身份，不猜测或重写归属。
2. `node_id` 对普通 Work 必填，对 `taskspace_control` 禁止；普通 Tool 的原生 `input` 中不出现 Map 字段。
3. `tool` 必须是当前容器 schema 中的精确原生名称；Namespace/MCP 使用现有 `ToolName` 的规范名称，不另建 alias 表。
4. Function 的 `input` 保持原参数 object，Freeform 的 `input` 保持 string。每个变体从现有 `ToolSpec` 机械生成，禁止
   手写第二份普通 Tool 参数 schema。
5. `taskspace_control` 删除 `actions[]`，只保留 Map 读取、图变更、节点状态和显式终态参数。
6. Map open 且请求仍需推进时，合法 Agent 行动必须包含容器；Map closed 后才允许提交无 Tool 的最终自然语言响应。TS-07
   必须先冻结这份请求状态表以及“terminal finish 与最终总结”唯一合同。支持强制指定函数的 provider 可按冻结合同选择
   `taskspace_tools`；不支持或拒绝 `tool_choice` 的 DeepSeek 路径必须省略字段，不能伪造 provider 保证，并由 L1/L2 协议、
   唯一 Tool 暴露和 Runtime 零执行硬合同共同守底线。该差异必须进入 final-wire/cache fixture，不在实现中临时切换。
7. 支持禁并行字段的 TaskSpace provider 请求发送 `parallel_tool_calls=false`；忽略或不支持该字段的路径不能假设它生效。
   Runtime 对同一 response 的多个 outer 容器整批零执行拒绝。容器内部 Work 并发由预检后的 Runtime 执行，Standard
   provider flags 保持原值。

### 3.2 合法批次形状

容器数组保留声明与结果对应顺序，但不构成第二份 Work 依赖或强制总执行顺序。Map 操作只允许出现在边界：

| 形状 | 用途 | 执行规则 |
|---|---|---|
| `[map_read]` | `read_map`、`read_output_ref` | 单项事实读取；结果返回后由 Agent 下一轮决策 |
| `[map_prelude, work+]` | 初始化、reopen、完成旧节点、增删节点/边后立即工作 | prelude 先提交；所有 Work 必须在提交后的同一 Ready frontier |
| `[work+]` | Map 无需变化时继续工作 | 所有 Work 在当前 Ready frontier；可属于多个节点 |
| `[work+, finish]` | 最后一批工作成功后关闭 Map | finish 是后置屏障；任一前置 Work 未成功则不执行 finish |
| `[map_prelude, work+, finish]` | 初始化/reopen 后在同一批完成一个有限任务 | prelude、Ready Work、terminal finish 三段执行 |

以下形状整批零执行拒绝：

- 初始化、reopen 或非终态节点完成后没有 Work；
- 非终态 Map mutation 位于末尾；
- Map 操作出现在 Work 中间；
- 同一批声明尚未 Ready 的依赖后继；
- 普通 Work 缺失 `node_id`，或 Map Tool 伪造 `node_id`；
- 超过一个 `apply_patch`；
- 未知、重复、空 `item_id` 或不存在的 Tool/节点；
- 序列外出现任何顶层原生 Tool，或同一 response 出现多个 outer 容器调用。

同一 Ready frontier 内的 Work 是否并发，只由既有 Tool 并行安全能力、Patch 屏障和 Runtime 资源约束决定。容器不表达
`B -> C`；若 C 需要 B 的结果，Agent 必须在 Map 中声明节点依赖，并在 B 结算后的下一请求调用 C。

### 3.3 结果合同

TaskSpace provider 只认识外层 `taskspace_tools` call id，因此下一轮只返回一个配对的 outer Tool result。现有
`FunctionCallOutputPayload` 支持 text/image content items；不能把多模态内容先转成 lossy text。目标结果统一使用
`ContentItems`：第一个 text item 是机械 manifest，记录每个 `item_id` 的状态及其在后续原生 content items 中的
`content_start/content_count`；其余 content items 按范围原样拼接，不摘要、不转写、不复制。

```json
{
  "schema_version": "TaskSpaceToolSequenceResultV1",
  "status": "completed",
  "canonical_revision": 14,
  "items": [
    {"item_id": "map-1", "status": "completed", "content_start": 0, "content_count": 1},
    {"item_id": "work-1", "status": "failed", "content_start": 1, "content_count": 2},
    {"item_id": "work-2", "status": "completed", "content_start": 3, "content_count": 1}
  ]
}
```

`content_start` 从 manifest 后的第一个原生 content item 按 `0` 计数，不把 manifest 自身计入范围。范围直接引用现有
`FunctionToolOutput.body`；文本、图片 URL/detail 和顺序逐值保留。协议错误、Tool 执行失败、`not_executed` 和
`outcome_unknown` 是不同状态。同一 Ready frontier 的独立 Work 不因列表前项失败而自动跳过；`cause_item_id` 只用于
已声明的 Map 屏障，例如 required Work 失败导致 terminal finish 未执行。

`canonical_revision` 在整个 outer result 中只出现一个可继续使用的最终值。新纯 Map Tool 的嵌套结果只返回该操作的
机械事实，不再返回第二个 revision；这是把 continuation 元数据提升到容器边界，不是裁剪 Tool 业务反馈。

### 3.4 事务和失败边界

“整批预检”不意味着外部 Tool 已执行后还能回滚。准确边界是：

1. 结构、已知 Map 状态、hypothetical prelude 后的 readiness 和 hypothetical success 后的 finish 合法性，在任何 dispatch
   前完成；这里失败必须 Map/Tool/HTTP 全为零副作用。
2. 合法 prelude 通过 `taskspace_control` 的 canonical Map 事务提交图变化并创建 Work reservation，然后才启动 Work。
3. Work 一旦启动，其真实副作用不可由 Runtime 伪装成事务回滚；每项按真实结果结算并释放 reservation。
4. 任一 required Work 失败、取消或未知时，后置 finish 不执行；已合法提交的 prelude 保留，Map 继续 open，并在唯一
   outer result 中报告最终 revision。
5. finish 只在前序 Work 全部满足 terminal 条件后由 Agent 已声明的 control item执行；Runtime 不自动补 finish。

## 4. 代码责任与复用边界

| 位置 | 目标责任 | 复用/变更 |
|---|---|---|
| `tools/src/tool_spec.rs` | 原生 Tool schema 唯一事实源 | 保持原生 Tool 定义不变 |
| `tools/src/code_mode.rs` | 当前 Function/Freeform/Namespace 展开逻辑 | 抽取无 Code Mode 语义的共享 descriptor，Code Mode 与容器共同使用 |
| `tools/src/taskspace_tool.rs` | Map Tool 参数 schema | 删除 `actions[]` 和 sibling 描述；不加入普通 Tool schema |
| `tools/src/taskspace_sequence_tool.rs`（候选） | 从共享 descriptor 生成唯一 `taskspace_tools` ToolSpec | 新增一个窄构造器，不新增 registry/plugin/config |
| `core/src/tools/nested_call.rs` | 将已验证的嵌套输入还原为原生 `ToolCall` | 扩展已确认类型；未知类型明确失败 |
| `core/src/tools/sequence_preflight.rs` | 容器结构、Map revision/readiness、边界和 Patch 硬校验 | 重写旧 sibling/action 配对，不保留兼容 parser |
| `core/src/tools/sequence.rs` | 预检后的批次分段、原 Router 调用、结算与结果构造 | 删除旧 manifest 分支；避免继续扩大当前大文件，必要时按 preflight/execution/result 职责拆分 |
| `core/src/session/taskspace_response.rs` | Map 事务支持 | 重构为协议中性的 handler 依赖；`taskspace_control` 仍经统一 item dispatcher、原 ToolRouter 和原 handler执行，preflight 不代执行、不合成 control 结果 |
| `core/src/session/turn.rs` | Standard/TaskSpace model-visible Tool 投影和 outer call 入口 | Standard 原样；TaskSpace 只投影容器 |
| `core/src/tools/provider_tool_declaration.rs` | provider 顶层 item 到调用声明 | Standard 保留；TaskSpace 不再用事后 `RejectedNative` 代替入口隔离 |

不新增第二个 Tool registry、工作流引擎、Map scheduler、provider client、持久化表、配置开关或旧协议兼容层。

## 5. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| TS-01 | 盘点普通嵌套 Tool | discovery | `tools/src/tool_spec.rs`、`tools/src/code_mode.rs`、`core/src/tools/nested_call.rs` | Function、Freeform、Namespace/MCP | 逐类记录 schema 来源、规范名称、输入类型、Router handler 和结果类型，给出 supported/deferred 结论 | 普通 Tool 进入容器的范围有源码证据 | 防止复制 schema 或遗漏 MCP 名称语义 | Complexity: +1 能力矩阵章节，无运行时代码；Reach/Cost: 只阻塞未证明的普通 Tool 类型 | 每类有 source/spec/handler/result 路径和最小正反例 | 任一 Unknown 不进入 TS-13 | planned |
| TS-02 | 盘点动态与特殊 client Tool | discovery | `tools/src/tool_spec.rs`、ToolSearch、LocalShell、deferred Tool 路径 | ToolSearch、延迟 Tool、LocalShell | 分别追踪模型输入、加载时机、handler 和下一请求 schema 更新；给出 supported/deferred/unavailable | 容器不会假装支持 provider 特殊 item | 保持能力反馈准确且避免猜测 adapter | Complexity: +1 矩阵章节/最小 probe；Reach/Cost: 可能暂时隐藏 TaskSpace 特殊能力，Standard 不变 | 每类有单一结论和解锁单元 | 未证明能力明确隐藏 | planned |
| TS-03 | 盘点 hosted Tool 与 provider flags | discovery | Web/Image ToolSpec、Responses client、DeepSeek/OpenAI provider profile | WebSearch、ImageGeneration、tool choice、parallel flag、strict schema | 核对可控输入、唯一结果、重试边界和各 provider 支持；分别给出 capability 结论 | Hosted 和请求 flags 不再由统一假设驱动 | 防止预检前副作用及 provider 400 回归 | Complexity: +1 provider 矩阵章节/本地 wire probe；Reach/Cost: 无真实 API 费用，未证明能力不进入 TaskSpace | 每个 provider/capability 有 wire fixture 和支持结论 | 真实能力不明时保持 deferred，后续另行申请 probe | planned |
| TS-04 | 证明 Map control 的统一分派 seam | discovery | `taskspace_control.rs`、`ToolCallRuntime`、`session/taskspace_response.rs` | initialize/execute/reopen 与 outer bindings | 做最小本地 seam probe，比较 Router invocation context、Map 原子事务和 bindings 传递；排除 sequence engine 直接提交、参数复制和全局 transient registry | control 能否保持普通 Tool 调用地位在编码前被证实 | 避免把旧 preflight 特例带入新容器 | Complexity: +1 seam probe/决策记录，无生产路由变化；Reach/Cost: 该结论阻塞 Map 接入 | Router lifecycle、单 Map commit、无隐藏 binding source 三项同时成立 | 无可行 seam 时暂停并与用户决策 | planned |
| TS-05 | 盘点旧协议全部生产消费者 | discovery | `sequence*.rs`、`taskspace_control_args*`、turn/provider/benchmark/docs | actions/sibling manifest、旧 reservation 类型、错误码、日志、fixtures | 建立删除清单并逐项审计 `TaskSpaceDeclaredCall`、`ActionMapDeclaredCall`、`TaskSpaceExecute.declared_calls`、`prepare_taskspace_response`、actions parser/mismatch 和 supplemental/developer receipt；区分 Standard 共用逻辑与 TaskSpace 专属逻辑 | 切换后可以完整删除旧路径而不误删 Standard | 降低兼容残留和清理阶段意外发现 | Complexity: +1 消费面清单；Reach/Cost: 无运行时影响 | 每个旧 marker 有 owner、协议中性重命名/删除单元或保留理由 | 清单不完整时不进入 TS-27 | planned |
| TS-06 | 冻结容器输入身份合同 | API contract | 本专题、`tools` JSON fixtures | tool name、item_id、node_id、input | 固化第 3.1 节 canonical 正反 fixture 和 JSON Schema 约束 | 输入身份和普通 Tool 零侵入可机械验收 | 防止实施中增加 bind/current node 等概念 | Complexity: +输入 fixture；Reach/Cost: 后续 wire 变化必须显式审阅 | JSON Schema validator 通过；未知/重名/type mismatch fixture 失败 | 未确认合同则停在 artifact | planned |
| TS-07 | 冻结请求、批次与事务合同 | state contract | 本专题、request/preflight fixtures | open/closed 请求状态、terminal summary、五种合法形状、零执行负例、legal failure 边界 | 先固化 Map open/closed 时 Tool requirement 与最终总结的唯一表达，再固化第 3.2/3.4 节状态前后 fixture；明确 hypothetical preflight、独立 Work 无位置失败传播、prelude 保留和 finish skip | Agent 的合法请求出口与 Runtime 底线完整且无暗门 | 防止 `Auto` 允许零推进、额外总结请求或把合法失败误作原子回滚 | Complexity: +请求状态表/状态 fixture；Reach/Cost: provider wire、缓存和 Map 测试矩阵扩大 | 每个 request state/case 有 provider field、before/after revision/result/reservation/dispatch 期望 | terminal summary 产品合同或 provider 可行性未确认时暂停，不实现 preflight | planned |
| TS-08 | 冻结 multipart 结果合同 | feedback contract | 本专题、protocol fixture | manifest、content ranges、text/image items、唯一 continuation revision | 固化第 3.3 节 output fixture，定义索引相对 manifest 后 payload、空范围、失败/未知和唯一 revision；纯 control payload 不重复 revision | 多 Tool 结果可配对且多模态无损，continuation 只有一个事实 | 避免反馈层再次压缩、重写或双写语义 | Complexity: +结果 fixture；Reach/Cost: control output 与 context/history consumer 必须支持该结构 | ranges 完整无重叠；text/image逐值 round-trip；整个 outer result 只匹配一个 canonical revision | fixture 无法无损 round-trip 或需保留第二 revision 时重新讨论结果外形 | planned |
| TS-09 | 冻结切换前 final-wire | cache baseline | `cache_payload_contract.rs`、snapshots | Standard 与三种 TaskSpace policy 请求对 | 复用现有本地两请求 fixture记录 tools/schema/flags/input hash | 切换前后差异有可信基准 | 防止修改后的快照自证不回归 | Complexity: +基线记录，无生产代码；Reach/Cost: fixture 运行时间增加 | Standard/三 policy repeat 稳定、哈希可复算 | fixture 不稳先修测量 | planned |
| TS-10 | 建立 schema 成本预算 | performance baseline | schema profile、cache report | 当前原生 Tool 总 bytes、重复字段、容器固定开销 | 分开测量原生 schema、旧 control actions 和固定 envelope，形成 TS-13 的可解释预算 | 新容器膨胀可定位到具体结构 | 避免以语义压缩掩盖重复 schema | Complexity: +离线报告；Reach/Cost: 无 token 费用 | 各 section bytes/hash 可复算，预算不依赖真实模型 | 不能分段测量时先修 observer | planned |
| TS-11 | 抽取中性 Tool descriptor | internal | `tools/src/code_mode.rs`、候选 `tools/src/nested_tool.rs` | Function/Freeform/Namespace 转换 | 移出无 Code Mode 语义的名称/schema转换，让 Code Mode 原样复用 | Code Mode 与容器消费同一 ToolSpec 派生物 | 避免第二份普通 Tool 协议 | Complexity: +1 小数据结构、删除重复转换；Reach/Cost: tools crate/Code Mode tests受影响 | Code Mode fixture逐值不变；descriptor hash 对源 spec一致 | 任一 Code Mode wire 变化回退 | planned |
| TS-12 | 提取纯 Map operation schema | internal/API seam | `taskspace_tool.rs`、`taskspace_control_args*`、`taskspace_control_output.rs` | Map operation schema/parser/output 与旧 actions wrapper | 提取共享的纯 Map operation 对象；新纯 output 只保留操作事实、不带 continuation revision；旧生产 parser 临时组合 actions 和旧 output，记录 TS-28 删除条件 | TS-13/19 不再依赖旧 actions 或双 revision 输出 | 解开 schema 建设、反馈唯一事实与原子 cutover 的依赖倒置 | Complexity: +1 暂时旧 wrapper/output adapter，TS-28 必删；Reach/Cost: control parser/output tests受影响，model-visible wire暂不变 | 旧 control fixture逐值不变；纯 Map input/output fixture独立通过且无 canonical revision | 不能保持旧 wire不变则回退，不提前切生产 | planned |
| TS-13 | 生成未接线容器 ToolSpec | API | 候选 `tools/src/taskspace_sequence_tool.rs` | `create_taskspace_tools_tool()` | 从 TS-11 descriptor、TS-12 Map schema和TS-01～03支持矩阵生成 `items[].oneOf`，并按 TS-07 冻结的请求状态生成 provider capability fixture | 候选容器 schema 只有一个事实来源，request requirement 无临时分支 | Agent 合同可在切生产前离线验证 | Complexity: +1 构造器/动态 oneOf；Reach/Cost: 只增加测试编译，暂不进入 prompt | TS-06/07 fixture、source hash、排除原因、TS-10 bytes全部通过 | 手写普通参数、terminal 合同未定或预算不可解释则删除构造器 | planned |
| TS-14 | 解码容器与复原原生调用 | internal | 候选 `taskspace_sequence_decode.rs`、`nested_call.rs` | outer args、composite call id、native ToolCall | 严格解析 items，并为 supported 类型构造 `outer/item` 内部身份和原生 payload | 调用可唯一追踪并进入原 Router | 消除 sibling 二次配对 | Complexity: +1 decoder/metadata struct；Reach/Cost: 未接生产，仅单测增加 | 返回对象逐值断言；invalid args 返回稳定错误，不测试 dispatch | 不加入宽松 parser；失败可独立回退 | planned |
| TS-15 | 实现未接线批次预检 | state | 候选 `taskspace_sequence/preflight.rs` | decoded batch + immutable Map snapshot | 按 TS-07 校验边界、revision、node/readiness、Patch和 hypothetical finish | 非法批次在任何 commit/dispatch 前可判定 | 保持状态机只守机械底线 | Complexity: +1 新 preflight，旧生产 preflight仍暂存至TS-28；Reach/Cost: 状态单测增加 | 每个 fixture 返回精确 plan/error；无 runtime side effect | 规则需要业务语义时停止 | planned |
| TS-16 | 接入统一 control 与 Map prepare | state | TS-04 确认的位置 | control invocation、Map operation、outer bindings/reservations | 按已确认 seam 让统一 item dispatcher 调用原 ToolRouter/handler，由 handler 依赖触发一个 canonical Map 事务；preflight 只产出验证计划和外层 invocation metadata | control 与普通 client Tool 共用执行生命周期且 Map prepare 原子 | 落实 control 普通 Tool 地位，消除 sequence engine 提交旁路 | Complexity: 一个新内核调用 seam，不增持久化；Reach/Cost: Tool runtime/Map tests受影响，生产仍未切换 | dispatcher→Router→handler trace恰好一次；stale/missing/multi-parent结果与TS-07一致；preflight commit=0 | TS-04 未通过不得开始；出现第二 commit源或 control 专属 dispatcher 即回退 | blocked-on-discovery |
| TS-17 | 实现 Map 边界调度 | runtime | 候选 `taskspace_sequence/execution.rs`、现有 Router | prelude/frontier/finish segments | 执行 prelude barrier、同 Ready frontier Work、optional finish barrier；不建 Work DAG | Map 顺序受控，Work 可按现有安全能力并行 | 降低请求放大且不替 Agent 规划 | Complexity: +1 未接线 executor；Reach/Cost: Router并发/测试时间受影响 | 五种合法形状；dependent successor无dispatch；单Patch barrier | 需要 item dependency字段时停止 | planned |
| TS-18 | 结算失败、取消与 reservation | failure | 候选 execution/result settlement、ActionMap runtime | failed/cancelled/not_executed/outcome_unknown | 在 TS-17 上补每种 outcome 的 reservation release和finish skip，不自动重试；同一 frontier 的独立 Work 各自结算，不按数组位置传播失败 | legal failure 保持真实副作用与 open Map，独立 Work 不被伪造依赖 | 防止悬挂节点、重复收费和失败扭曲 | Complexity: +故障分支/fixture；Reach/Cost: cancellation和hosted mock测试增加 | 每个 outcome 的请求次数、revision、reservation、finish count精确断言；前项失败不取消独立后项 | 任一未知被重试、伪装失败或出现列表依赖即回退 | planned |
| TS-19 | 构造结果 manifest 与范围 | feedback | 候选 `taskspace_sequence/result.rs` | metadata item、range table、canonical revision | 只构造 manifest/range 元数据，不接 history；覆盖空/失败/未知范围 | 每项结果有唯一机械索引 | 为无损反馈提供可独立验证的纯函数 | Complexity: +1 result builder；Reach/Cost: 仅单测 | ranges完整无重叠；revision恰好一个 | 纯函数不满足 TS-08 时删除 | planned |
| TS-20 | 拼接无损 multipart outer output | feedback integration | result builder、protocol/context history tests | `FunctionCallOutputPayload::ContentItems` | 将 TS-19 manifest 放首项并原样拼接各业务 `FunctionToolOutput.body`；control 的 continuation revision 只由 manifest 提供；验证 history/normalize不丢 image | outer call 一份反馈保留全部业务内容且只有一个 continuation 事实 | 消除重复反馈同时保护语义 | Complexity: +1 content assembler；Reach/Cost: protocol/history/cache fixtures受影响 | text/image/detail/order逐值 round-trip；revision全局唯一；无额外 developer message | 任一 lossy normalize 或 revision 双写出现则不切生产 | planned |
| TS-21 | 建立新路径生命周期日志 | observability | decode/preflight/execution/result 新模块 | received/rejected/dispatched/settled events | 在未接线内核增加第8节稳定事件和敏感字段排除 | 切换首轮即可诊断新路径 | 避免上线后才补日志或重放语义 | Complexity: +5事件/capture tests；Reach/Cost: item级日志量增加 | event顺序、计数、zero-dispatch和正文缺失断言 | 日志可独立回退，不影响协议 | planned |
| TS-22 | 对候选 schema 运行免费缓存门禁 | cache gate | `scripts/cache-regression/`、candidate fixture | TS-13 schema 与 Standard baseline | stage候选 fixture并运行 `--source index`，记录预期变化，不晋升基线 | schema 成本问题在 cutover 前发现 | 限制生产切换的缓存未知量 | Complexity: +候选报告；Reach/Cost: 无API费用 | Standard不变；TaskSpace差异只来自容器候选 | 意外prompt/projection变化即回退对应单元 | planned |
| TS-23 | 接入 Image hosted adapter | provider API | TS-03确定位置、Responses client | ImageGeneration capability | 仅当 TS-03 已证明产品输入/结果合同后，迁移 MVT4～6 为生产窄 adapter | Image 动作预检后执行一次 | 恢复被证实可控的 hosted 能力 | Complexity: +1 capability adapter；Reach/Cost: provider网络/费用/故障诊断增加 | HTTP=1、history不变、failed/unknown分开 | 未证明则保持隐藏，不阻塞基础容器 | blocked-on-discovery |
| TS-24 | 接入 Web hosted adapter | provider API | TS-03确定位置、Responses client | WebSearch capability | 仅当 TS-03 已证明产品输入/结果合同后实现独立窄 adapter | Web 动作预检后执行一次 | 恢复被证实可控的搜索能力 | Complexity: +1 capability adapter；Reach/Cost: 网络/费用/内容安全面增加 | wire/result/无重试/Map attribution测试 | 未证明则保持隐藏，不阻塞基础容器 | blocked-on-discovery |
| TS-25 | 接入 ToolSearch/延迟 Tool | internal/API | TS-02确定位置 | ToolSearch 与动态加载后的原生 ToolSpec | 仅在发现阶段判为 supported 时实现一个窄 adapter，并证明下一请求只从 canonical registry 更新容器 schema | 动态发现能力不绕过容器且不复制 registry | 保留按需工具发现，同时保持 schema 单一来源 | Complexity: +1 动态 descriptor adapter；Reach/Cost: 请求间 schema hash 可能变化并触发缓存门禁 | 加载前后 schema/source hash、Router trace 和 Standard 0 diff | deferred/unavailable 时隐藏，不阻塞切换 | blocked-on-discovery |
| TS-26 | 接入 LocalShell | internal/API | TS-02确定位置 | LocalShell capability | 仅在输入、权限、sandbox、审批和结果均能通过原 Router 表达时实现独立 adapter | shell 调用继续使用原安全生命周期 | 不因容器丢失本地执行能力，也不复制 shell 协议 | Complexity: +1 LocalShell adapter；Reach/Cost: shell权限与审批回归面增加 | schema/input/approval/sandbox/result 逐层 trace 与 Standard 对照 | 任一生命周期旁路则保持隐藏 | blocked-on-discovery |
| TS-27 | 原子切换生产 TaskSpace 入口 | integration/API | `turn.rs`、registry plan、control schema入口、outer dispatch | model-visible tools/flags、outer call route、纯 control parser | 在一个切换提交中启用 TS-12纯control、只投影TS-13容器、按provider能力设置flags并把outer call路由到TS-14～20；Standard不改 | 生产 TaskSpace 只有一个完整容器协议 | 避免多个半切换提交造成不可运行状态 | Complexity: 一个原子集成提交，无flag/双写；Reach/Cost: TaskSpace prompt/turn/cache/UI事件整体变化 | 固定local mock turn先跑init+work和work+finish；失败提交不得保留 | 整体revert TS-27；不启用旧/new双路由 | planned |
| TS-28 | 删除旧 sibling/manifest 路径 | cleanup | TS-05清单中的源码/tests/scripts/examples | actions wrapper、manifest matching、TaskSpace RejectedNative旧分支 | 删除全部旧TaskSpace消费者和TS-12临时wrapper，保留Standard共用声明逻辑 | 当前代码只剩一个TaskSpace协议 | 降低维护和误接旧逻辑成本 | Complexity: 净删除旧类型/错误码/fixture；Reach/Cost: 历史docs保留，当前测试清单收敛 | `rg`仅历史docs命中；切换smoke仍通过 | 发现漏列consumer时暂停并补TS-05，不加兼容 | planned |
| TS-29 | 执行定向本地回归矩阵 | verification | tools/core/session test targets、WireMock | schema/decode/preflight/Map/Router/feedback/turn | 依次运行第7节命令和Standard/三policy final-wire fixture，记录测试数与命令 | 付费验证前覆盖全部新边界 | 缩小真实运行故障面 | Complexity: +结果记录；Reach/Cost: Rust/fixture CI时间增加，无网络费用 | 所有目标通过；Standard snapshot 0 diff；TaskSpace只有批准差异 | 任一失败回到所属最小单元，不批量补丁 | planned |
| TS-30 | 结算缓存敏感候选 | cache gate | cache gate、candidate snapshots | TS-27/28 final-wire fingerprints | 运行 index免费门禁并生成精确changed set；不手工晋升accepted baseline | 正确变化与缓存接受状态分离 | 防止未知上下文回归 | Complexity: +门禁报告/候选快照；Reach/Cost: 后续可能需要真实缓存预算 | PASS或准确BLOCKED；Standard不在changed set | 意外差异回退对应单元，不绕过gate | planned |
| TS-31 | 执行最小真实产品验证 | product verification | Docker benchmark、Whale run ledger | Standard+三TaskSpace policy同一客观sample | TS-29/30通过后申请4-run专项预算，每臂repeat=1，检查业务/trace/Map/token/cache/time | 证明真实Agent使用生产容器且旧零推进形状消失 | 避免mock成功冒充产品成功 | Complexity: 无代码；Reach/Cost: 4次DeepSeek run及费用/耗时，必须事前授权 | 四臂证据完整；旧sibling/action mismatch=0 | 未获预算不运行；失败不自动repeat | deferred |
| TS-32 | 形成结果并重排R8问题 | docs/governance | 本专题、`01-r8-known-issues.md` | MVT-8/result、I01～I10 | 记录提交/测试/缓存/真实证据，逐项标记resolved/reframed/open | R8恢复唯一问题全集 | 防止继续修已删除协议 | Complexity: 更新文档/依赖；Reach/Cost: 后续编号和归因变化 | 每个问题有证据和唯一状态，Git/ledger一致 | 缺证据保持open | planned |

## 6. Phase 顺序与门槛

### Phase A：合同与能力边界

- Entry: MVT-0～MVT-7 的本地证据可复算。
- Work units: TS-01～TS-10。
- Evidence: 三类 Tool/provider 能力矩阵、control seam、旧消费面、request/terminal/输入/批次/结果合同、final-wire 与成本基线。
- Cross-unit side effects: 只有文档、fixture 和本地测量；无生产行为、无真实 API 成本。
- Next: 所有进入容器的 Tool 类型都有明确合同，control 统一分派 seam 已证明，TS-06～08（尤其 terminal summary）获得确认。

### Phase B：未接生产入口的容器内核

- Entry: Phase A 通过。
- Work units: TS-11～TS-22。
- Evidence: schema 来源唯一、纯 Map seam、decode/preflight/Map/Router/result/log 的组合测试和候选缓存报告。
- Cross-unit side effects: 新内核只在测试入口可达；当前生产继续旧路径。TS-12 临时旧 wrapper 必须由 TS-28 删除，
  不允许出现 runtime 双路由或 feature flag。
- Next: 五种合法形状和全部负例通过，multipart 无损，control 单一分派，MVT-2～6 断言在新对象上复现。

### Phase C：按已证能力补齐特殊 Tool

- Entry: Phase B 完整通过，切换提交范围和缓存影响已审阅。
- Work units: TS-23～TS-26，仅执行 TS-02/03 判为 supported 的具体能力；deferred/unavailable 不阻塞基础容器。
- Evidence: 每个已接能力有独立 schema/wire/Router/result/Map fixture；未接能力有明确隐藏原因。
- Cross-unit side effects: 每个能力增加自己的网络、动态 schema 或 shell 测试成本，不建设通用 adapter framework。
- Next: 当前基础 coding workflow 所需能力全部 supported，或产品接受其在 TaskSpace 中明确不可用。

### Phase D：原子生产切换与旧协议删除

- Entry: Phase B 通过，Phase C 的 supported 项完成或明确 deferred。
- Work units: TS-27、TS-28。
- Evidence: 固定 mock turn 先证明切换可运行，再由清理审计证明旧协议无当前代码残留。
- Cross-unit side effects: TS-27 一次改变 TaskSpace model-visible schema、flags、feedback 和 dispatch；不保留兼容模式。
- Next: TS-27 可独立整体 revert；通过后 TS-28 只做净删除且 smoke 不变。

### Phase E：离线发布门

- Entry: Phase D 通过。
- Work units: TS-29、TS-30。
- Evidence: 定向本地矩阵、Standard 0 diff、TaskSpace changed set 和缓存门禁报告。
- Cross-unit side effects: Rust/fixture CI 时间增加；无真实 provider 成本。
- Next: 无未知 final-wire 差异，Standard 0 diff，TaskSpace 变化完全由容器切换解释。

### Phase F：真实验证与问题重排

- Entry: Phase E 通过且用户批准 TS-31 的 4-run 专项预算。
- Work units: TS-31、TS-32。
- Evidence: ledger、原始 trace、业务 oracle、Map、请求/token/cache/time 对比和最终结果文档。
- Cross-unit side effects: 产生明确 API 费用；问题队列可能重新编号或合并。
- Next: 只有真实结构错误为 0 且业务不回归，才把容器标为 runtime-verified。

## 7. 验收矩阵

| 维度 | 必须成立的证据 |
|---|---|
| 唯一入口 | TaskSpace 主请求 `tools` 恰好只有 `taskspace_tools`；支持时禁顶层并行，所有路径对多个 outer call 零执行拒绝；Standard 不含该 Tool且 flags 不变 |
| 原生零侵入 | 普通 ToolSpec、参数、Router handler、permission/sandbox/hook 测试与 Standard 基线一致 |
| 唯一动作事实 | control 无 `actions[]`；无 sibling manifest、shadow call、双写或 Runtime 猜配 |
| Map 底线 | revision、DAG、Ready、node binding、terminal、Patch 在 dispatch 前校验；非法批次零副作用 |
| 调度边界 | Map prelude/finish 是屏障；Work 依赖只来自 Map；同 Ready frontier 可并行 |
| 反馈保真 | 每项恰好一个状态；multipart 范围无重叠/遗漏；原生 text/image 逐值不改写；最终 revision 恰好一个；未知不伪装失败 |
| Hosted | 只在预检后触发；主 history 不变；非幂等请求不自动重试；未支持能力不暴露 |
| 缓存 | Standard final-wire 0 diff；TaskSpace schema 无平行重复；门禁变化可解释 |
| 行为 | 真实 trace 中旧 `taskspace_action_*mismatch`、单独 control、容器外普通 Tool 均为 0 |
| 清理 | 当前生产代码无旧 actions/sibling 路径；没有兼容 flag/parser/数据迁移 |

### 7.1 离线发布命令

实现阶段允许按工作单元增加更窄的测试，但 TS-29 至少从 `third_party/codex-cli/codex-rs/` 执行并记录以下固定命令：

```bash
cargo test -p codex-tools taskspace_sequence
cargo test -p codex-core taskspace_sequence
cargo test -p codex-core --test all standard_request_pair_preserves_the_complete_prefix
cargo test -p codex-core --test all taskspace_projection_policies_have_independent_request_pairs
```

随后回到仓库根目录、暂存本主题全部变化，再执行：

```bash
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
```

测试筛选名在新增测试时必须采用 `taskspace_sequence` 前缀，避免计划落地后再用临时命令人工挑选。门禁 BLOCKED 时只形成
changed-set 证据和预算申请，不得自动晋升缓存基线或启动真实 Whale Agent run。

## 8. 日志合同

只记录机械身份和状态：

- `taskspace.sequence.received`: outer call id、item count、schema hash、submitted revision；
- `taskspace.sequence.preflight_rejected`: reason code、item ids、canonical revision、`zero_dispatch=true`、
  `state_commit=false`；
- `taskspace.sequence.item_dispatched`: item id、node id、Tool name、execution owner；
- `taskspace.sequence.item_settled`: item id、status、result-ref identity；
- `taskspace.sequence.settled`: completed/failed/not-executed/unknown count、final canonical revision。

日志禁止写入 Tool arguments、命令、文件正文、prompt、result body、节点 goal 和用户文本。日志只用于发现与重建，不作为
Map 或反馈的第二事实源。

## 9. 风险与处理

| Risk | Trigger Signal | Mitigation | Safe Stop / Fallback |
|---|---|---|---|
| 动态 `oneOf` schema 过大 | TS-13 bytes 明显超过 TS-10 中原生 Tool 总量与固定容器开销 | 检查重复 description/schema 和 deferred Tool 集合；不先做语义压缩 | 停在 Phase B，保持当前生产路径并讨论 ToolSearch/按需暴露 |
| 特殊 Tool 没有可控输入合同 | TS-02/03 无法从现有 API/handler 得到输入或唯一结果 | TaskSpace 不暴露该 capability，Standard 保留 | 不造通用 adapter，不双写顶层调用 |
| open Map 仍生成无 Tool 响应 | provider 不支持强制 Tool，或 request state 合同存在纯文本出口 | TS-07 明确状态表；使用 provider 已证能力、L1/L2 协议、唯一 Tool 投影和 Runtime 零执行硬合同，不伪造 `tool_choice` | 若客观 fixture 仍稳定零推进，暂停切换并讨论 provider 适配，不增加关键词判断或固定回复 |
| finish 后最终总结合同缺失 | 需要额外请求、总结进入 control 语义或 Map 已关闭却仍强制 Tool | TS-07 在 schema 编码前冻结唯一表达与缓存影响 | 未确认前 TS-13 不开始，不由 Runtime 代写或补总结 |
| Map control 继续成为特殊旁路 | TS-04/16 需要 sequence engine 直接提交 Map、复制 bindings 或 transient registry | 要求 control 由统一 Tool lifecycle 触发，batch context 只存在于 Runtime 外层且单一传递 | Phase A 停止并与用户重新讨论，不用旧 preflight 特例冒充完成 |
| Provider 忽略/拒绝 Tool flags | DeepSeek thinking/Anthropic 路径拒绝 `tool_choice` 或忽略禁并行 | 按 model capability 发送或省略；Runtime 完整预检和多 outer call 零执行始终生效 | 不为统一 wire 破坏可用 provider；Standard 保持原行为 |
| 容器让 Runtime 重建 Work DAG | preflight/scheduler 出现 item dependency、next/current node | 删除容器依赖字段，只读取 canonical Map Ready frontier | 回退 TS-15/17 并重新审查合同 |
| 反馈再次复制 | outer result 外又出现 control revision/developer receipt | TS-08/12/19/20 结构测试和 final-wire 门禁 | 不进入 Phase D，删除额外 continuation carrier，业务 Tool body 保持原样 |
| Standard 被共享重构污染 | Standard snapshot/tool schema/tool choice 任一变化 | 将模式分支限制在 Tool 投影与 outer dispatch 入口 | 回退共享改动，先恢复 Standard 逐值基线 |
| 切换期间出现双协议 | 旧/new parser 或 feature flag 同时可达 | Phase D 原子切换后立即 TS-28 删除 | 整体回退 TS-27，不保留兼容分支 |

## 10. 外部依据与适用边界

1. [OpenAI Function Calling](https://developers.openai.com/api/docs/guides/function-calling)：Tool 由 JSON Schema 定义，
   多调用通过各自 `call_id` 返回结果；支持并行不意味着调用之间自动形成业务依赖。
2. [OpenAI Agents SDK Tools](https://openai.github.io/openai-agents-python/tools/)：明确区分 hosted Tool 与
   local/runtime Tool；直接绕过 Tool runtime 会跳过 schema、guardrail、timeout、failure 和 tracing。
3. [Model Context Protocol Tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)：Tool 名称、
   `inputSchema`、结果内容和执行错误各有独立协议职责，客户端应校验结果并记录调用。
4. [Claude Tool Definition](https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools)：client Tool 与
   server-side Tool 的执行边界和 schema 能力并不相同，示例和额外描述也会产生固定 token 成本。
5. [DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls) 与
   [DeepSeek Agent Integration](https://api-docs.deepseek.com/quick_start/agent_integrations/oh_my_pi/)：V4 的 strict schema、
   `tool_choice` 和并行控制依 provider 兼容路径而异，不能用单一字段替代 Runtime 预检。

这些资料支持原生 schema 唯一来源、调用身份配对、执行归属隔离和输入/结果校验。TaskSpace 的 Map、合法批次、节点归属
及“Runtime 不替 Agent 决策”仍由本项目产品定义，不从外部 SDK 引入编排语义。

## 11. 计划完成定义

本文当前只是 `drafted` 的工程计划，不表示代码已实施。计划进入执行前需要确认：

1. 第 3 节容器 wire、五种合法形状和单一结果外形没有偏离产品预期；
2. TS-07 已明确 open/closed 请求状态、provider Tool requirement 和 terminal finish 后最终总结的唯一表达；
3. TS-02/03 的特殊 Tool 发现允许“不支持即隐藏”，且 TS-04 control 统一分派 seam 必须在实施前证明；
4. Phase D 采用一次切换并删除旧协议，不保留实验开关；
5. TS-31 的真实运行预算在执行到该阶段时另行申请，当前授权不包含任何 DeepSeek 调用。
