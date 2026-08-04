# TaskSpace Tool 序列容器正式工程计划

- Created: 2026-08-02
- Updated: 2026-08-04
- Status: Active plan / product contract realigned / implementation not started
- Scope: 将 Tool 序列容器接入生产 TaskSpace，并兼容 provider 原生 hosted Tool 输出
- Risk depth: Full，涉及 model-visible Tool API、provider 响应解析、Map 事务、反馈和缓存
- Prerequisite: [`00-product-definition.md`](00-product-definition.md)
- Historical evidence: [`01-execution-ownership-mvp-feasibility-plan.md`](01-execution-ownership-mvp-feasibility-plan.md)
- Excludes: 旧 wire/旧数据兼容、Map 压缩、projection 三模式重构、Agent 业务任务拆分

## 0. 本次重写结论

2026-08-04 的两轮讨论纠正了两个底层假设：

1. Provider-hosted Tool 并不是一个必须由 Runtime 顶层 dispatch 的 client call。Provider 在响应生成期间完成动作，
   Runtime 收到 `web_search_call`、`image_generation_call` 等原生输出事实。
2. Tool 执行状态与节点生命周期正交。Tool 成败、完成时机和节点 Ready/InFlight/Completed/Blocked 之间没有自动推导。

因此继续采用容器，但重新定义其职责：

- 它是 TaskSpace 唯一的 **Agent 行动与节点归属账本**；
- 它承载待执行的 client/map 调用，也引用本响应已经完成的 hosted 输出；
- 它只约束 Map 操作位于行动的前置/后置边界，不建立第二份 Work DAG；
- 它不把 provider 输出伪装成待执行调用，也不让 Tool outcome 驱动节点状态。

此前专用 hosted proxy adapter 的 MVT 只保留为技术可行性证据，不再作为默认生产实现。

## 1. 目标请求与响应路径

### 1.1 Model request

```text
TaskSpace request.tools
  - taskspace_tools              # client/map 行动容器
  - provider-native hosted tools # provider 支持的 web/image 等能力
```

TaskSpace 不再单独暴露顶层 client-managed Tool 或顶层 `taskspace_control`。Standard 完整保持原生 Tool 集合、调用方式、
flags 和返回路径。

### 1.2 Provider response

同一个响应可以包含：

```text
provider output item H1: web_search_call(completed, result=...)
provider output item H2: image_generation_call(completed, result=...)
function call C1: taskspace_tools(...)
```

其中 H1/H2 已由 provider 执行；C1 只负责：

- 用稳定身份引用 H1/H2 并声明其 `node_id`；
- 声明需要 Runtime 执行的 client-managed Tool；
- 声明 `taskspace_control` Map 操作。

### 1.3 Runtime processing

```text
等待 provider response 完整结束
  -> 收集 provider-native hosted output facts
  -> 解码唯一 taskspace_tools 容器
  -> 机械核对 hosted refs
  -> 预检尚未发生的 client/map items
  -> map prelude 经统一 ToolRouter 提交
  -> client items 经现有 ToolRouter 执行
  -> map epilogue 经统一 ToolRouter 提交
  -> 分别结算 hosted/client/map 事实
  -> 返回无损结果与唯一 canonical revision
```

Hosted 输出发生在容器预检之前，不能回滚。容器非法时，Runtime 拒绝其 Map/client 部分和 hosted 绑定，但仍保留并反馈
provider 已经产生的原始输出事实。

## 2. 单一数据合同

### 2.1 容器项类别

正式容器暂定名 `taskspace_tools`。每项必须属于以下三类之一：

| 类别 | Agent 声明 | 权威业务事实 | Runtime 动作 |
|---|---|---|---|
| `client_call` | `item_id`、`node_id`、原生 Tool 名和 input | 现有 ToolSpec 与 ToolRouter 结果 | 预检后执行一次 |
| `provider_result_ref` | `item_id`、`node_id`、本响应 hosted 输出引用 | provider 原生输出 item | 核对、绑定、记录，不执行 |
| `map_call` | `item_id`、`taskspace_control` 原生 input | canonical Map transaction | 经统一 ToolRouter 执行 |

`taskspace_control` 在结构、Router 和 handler 地位上仍是普通 Tool。类别差异只来自它提供 Map 能力，不允许建立 control
专用 dispatcher。

### 2.2 候选 schema

以下是合同示例，不是最终字段冻结：

```json
{
  "items": [
    {
      "kind": "map_call",
      "item_id": "map-1",
      "tool": "taskspace_control",
      "input": {
        "action": "execute",
        "expected_revision": 12,
        "mutations": [{"action": "complete_node", "node_id": "inspect"}]
      }
    },
    {
      "kind": "provider_result_ref",
      "item_id": "hosted-1",
      "node_id": "research",
      "output_ref": {"response_item_id": "ws_123"}
    },
    {
      "kind": "client_call",
      "item_id": "work-1",
      "node_id": "implement",
      "tool": "exec_command",
      "input": {"cmd": "cargo test -p codex-core taskspace_sequence"}
    }
  ]
}
```

冻结要求：

1. `item_id` 在一个容器内唯一；内部身份由 `outer_call_id/item_id` 形成。
2. `node_id` 对 client 和 provider ref 必填，对 map_call 禁止；Runtime 不默认、不继承、不推断。
3. client `tool/input` 从现有 ToolSpec 机械派生，Function、Freeform、Namespace/MCP 不手写第二份协议。
4. provider ref 只携带最小稳定身份和节点归属，不复制 Tool 名、参数、状态或结果；这些字段以 provider 输出为准。
5. `taskspace_tools` 不得成为自己的 item，容器只允许一层。
6. `taskspace_control` 删除 `actions[]`，只保留 Map 读取、图变更、节点生命周期和显式终态。
7. provider ref 的最终身份字段必须先由本地 response fixture 证明稳定；不能退回按名称、位置或内容猜配。

### 2.3 合法行动形状

容器只保证 Map 操作位于边界，不为普通 Work 建立全序：

| 形状 | 产品用途 | 机械规则 |
|---|---|---|
| `[map_read]` | 读取 Map 或引用内容 | 单项容器；不改变节点状态 |
| `[map_prelude, actions+]` | initialize、reopen、图变更或完成旧节点后继续工作 | prelude 先提交，随后处理 action 集合 |
| `[actions+]` | Map 无变化时继续多个节点工作 | action 列表不表达业务依赖 |
| `[actions+, map_epilogue]` | Agent 显式变更节点或最终 finish | epilogue 是 Map 边界，不以 Tool 成功为前提 |
| `[map_prelude, actions+, map_epilogue]` | 有限任务的一轮完整推进 | 分别验证两个 Map 边界与 action 归属 |

`actions` 可以混合 client calls 与 provider result refs。B、C 是否有依赖，只由 Map 中 Agent 声明的边表达；容器不得
用数组位置、Ready frontier、Tool 成败或隐藏 scheduler 推导 `B -> C`。

以下结构拒绝尚未发生的 client/map 动作：

- 初始化、reopen 或非终态 complete 后没有任何实际 action；
- Map 操作位于 action 集合中间；
- client/provider ref 缺失 `node_id`，或 map_call 伪造 `node_id`；
- provider ref 不存在、重复引用或跨响应引用；
- 超过一个 `apply_patch`；
- 未知/重复/空 `item_id`，未知 Tool，递归容器；
- 顶层出现 client-managed Tool、顶层 `taskspace_control` 或多个容器。

节点是否 Ready、某次节点状态变化是否合法，仍由 canonical Map 对 Agent 声明进行机械验证；该验证不得读取 Tool outcome
或把 Tool 是否结算当作节点生命周期前置条件。

## 3. 状态与事务边界

### 3.1 两套正交事实

```text
Tool execution fact:
  declared | running | succeeded | failed | cancelled | skipped | outcome_unknown

Node lifecycle fact:
  Ready | InFlight | Completed | Blocked
```

硬规则：

1. Tool success 不自动 complete 节点。
2. Tool failure/unknown 不自动 block、reopen 或 close 节点。
3. completed/blocked 节点可以仍有关联 Tool 在结算；结果到达后仍记录在原绑定下。
4. Agent 可以在同一容器中显式变更节点状态并声明其他动作；Runtime 分别校验，不因状态组合做语义判断。
5. Root 只在 Agent 显式最终 finish 时闭合；用户后续反馈允许 Agent 显式 reopen 同一 Map。

### 3.2 可保证的原子边界

- Provider-hosted 输出：已发生，不属于 Runtime 事务，不能回滚。
- 容器结构与 hosted binding：可在本地完整核对；非法 binding 不写入 Map，但保留 provider 原始事实。
- Map call：每个 call 使用 canonical transaction；不允许外层 sequence runtime 直接提交第二次。
- Client call：只在容器机械预检通过后 dispatch；开始后按真实副作用和结果结算，不伪装回滚。
- Tool outcome 与 Map lifecycle：独立提交和记录，双方都不成为另一方的隐式触发器。

### 3.3 结果合同

结果必须分别保留三类事实：

1. provider 原生 hosted output，不摘要、不转写；
2. client Tool 原生 text/image/error content，不裁剪或再解释；
3. Map call 的接受/拒绝事实与唯一最终 `canonical_revision`。

容器结果可使用一个机械 manifest 索引内容，但不得复制业务结果。每项状态只描述该项本身：

- `provider_result_ref`：`bound` / `unbound` / `invalid_ref`，不重写 provider Tool 状态；
- `client_call`：真实 Tool outcome；
- `map_call`：真实 Map transaction outcome。

`failed`、`outcome_unknown`、`not_executed` 和协议拒绝必须区分。反馈只报告事实，不建议 Agent 下一步，也不暗示节点状态。

### 3.4 请求生命周期

1. Map open 且任务仍需推进时，Agent 通过容器提交 client/map 动作，并可使用 provider 原生 hosted capability。
2. Agent 显式提交最终 finish 后，Runtime 先结算同一响应中的 hosted 事实、client/map 结果和最终 Map revision。
3. 随后恰好发起一次不投影任何 Tool 的 provider 请求，由 Agent 基于刚收到的事实生成最终自然语言总结。
4. 若用户在总结后反馈任务未完成，下一请求重新投影容器与 provider hosted capability，由 Agent 显式 reopen 同一个 Map
   并继续工作。Runtime 不自动 reopen，也不新增“返工”等生命周期语义。

最终总结不进入容器、`taskspace_control` 或 Runtime 固定模板。Tool outcome 不决定 finish 是否可声明，finish 也不改写
任何 Tool outcome。

## 4. 代码责任与复用边界

| 位置 | 目标责任 | 约束 |
|---|---|---|
| `tools/src/tool_spec.rs` | 原生 client Tool schema 唯一事实源 | 不增加 TaskSpace 字段 |
| `tools/src/code_mode.rs` / shared descriptor | Function/Freeform/Namespace 展开 | 抽取中性派生物，不能复制协议 |
| `tools/src/taskspace_tool.rs` | 纯 Map Tool schema | 删除 `actions[]`，不描述其他 Tool |
| `tools/src/taskspace_sequence_tool.rs`（候选） | 生成容器 ToolSpec | 只含薄 envelope、归属和原生派生 schema |
| `core/src/tools/nested_call.rs` | client item 还原为原生 ToolCall | 复用 MVT-1；未知类型明确失败 |
| `core/src/tools/sequence_preflight.rs` | 容器、引用、Map 边界、Patch 硬校验 | 不读取 Tool outcome，不实现 Work DAG |
| `core/src/tools/sequence.rs` | 分段、Router dispatch、结果聚合 | 不提交 Map、不推断节点、不重试 hosted |
| `core/src/session/taskspace_response.rs` | Map transaction 支持 | 由 control handler 唯一调用 |
| `core/src/session/turn.rs` | 模式化 Tool 投影和响应收集 | Standard 原样；TaskSpace=容器+hosted descriptors |
| `core/src/tools/provider_tool_declaration.rs` | provider output 事实识别 | hosted 输出进入 reconciler，不再 `RejectedNative` |
| 新的窄 reconciler（候选） | hosted output identity 核对与绑定 | 不执行 hosted Tool，不解析语义 |

不新增第二个 Tool registry、Map scheduler、工作流引擎、provider client、持久化表、配置开关或旧协议兼容层。

## 5. 已验证证据与失效假设

| 证据 | 处理 |
|---|---|
| MVT-1 原 Router 复用，`228c68ff8` | 保留，client item 基础 |
| TS-04 control Router seam，`148406cde` | 保留，map_call 基础 |
| MVT-7 Standard 隔离 | 保留，持续门禁 |
| MVT-4～6 hosted proxy adapter | 保留为 adapter 可行性证据，不进入默认生产路径 |
| “hosted 必须预检后执行” | 撤回；原生 provider 输出已发生 |
| “失败 Tool 阻止 finish/保持节点 Ready” | 撤回；Tool 与节点状态正交 |
| “所有 Work 必须在同一 Ready frontier 才能 dispatch” | 撤回为 Tool gate；Map 只验证 Agent 声明的图和生命周期变更 |
| “completed/blocked 节点不能拥有未结算动作” | 撤回；延迟 Tool 事实仍可归属原节点 |

## 6. 工作单元

每个单元只处理一个明确边界并单独提交。未经用户预算批准，不运行真实 Whale Agent。

| ID | 目标 | 变更位置 | 验证 | 状态 |
|---|---|---|---|---|
| TS-01 | 盘点 client Tool 类型与原生 descriptor 来源 | ToolSpec、Code Mode、MCP/Namespace、ToolSearch、LocalShell | 每类 source/input/router/result 路径 | completed（见 05） |
| TS-02 | 冻结 provider-native hosted output 类型与稳定身份 | protocol models、stream events、response fixtures | Web/Image output item identity 与同响应共存 fixture | completed（见 05，`7c75c03ab`） |
| TS-03 | 盘点 provider 请求能力 | provider profiles、tool flags | 容器+hosted descriptors 的本地 wire fixture | completed（见 05） |
| TS-04 | 证明 control 统一 Router seam | control/Router/Map transaction | 已有本地测试 | verified (`148406cde`) |
| TS-05 | 盘点旧 actions/sibling/RejectedNative 消费面 | core/tools/session/tests/docs | 删除清单与 Standard 共用边界 | planned |
| TS-06 | 冻结三类容器 item schema | JSON schema fixtures | 正反 schema fixture；无递归/复制 | planned |
| TS-07 | 冻结五种容器形状与 Map 边界 | preflight fixtures | map 中置、空推进、双 Patch 等负例 | planned |
| TS-08 | 冻结 Tool/节点状态正交合同 | state fixtures | outcome×node lifecycle 交叉矩阵 | planned |
| TS-09 | 冻结无损结果与唯一 revision 合同 | protocol fixtures | text/image/error round trip | planned |
| TS-10 | 抽取共享原生 Tool descriptor | tools crate | Code Mode/Standard wire 逐值不变 | planned |
| TS-11 | 生成未接线容器 ToolSpec | tools crate | schema hash、大小、无 self-reference | planned |
| TS-12 | 实现纯 decoder 与 hosted ref reconciler | core/tools | 存在/重复/跨响应/未绑定 fixture | planned |
| TS-13 | 实现无副作用 preflight | core/tools | 非法 client/map 零 dispatch；hosted 原事实保留 | planned |
| TS-14 | 接入 client item 原 Router | nested_call/Router | Function/Freeform/MCP 定向测试 | planned |
| TS-15 | 接入 map_call 原 Router | control handler/Map transaction | 单 Map commit、无外层旁路 | planned |
| TS-16 | 实现三类事实独立结算 | result/reconciliation | 不发生 Tool→node 自动转换 | planned |
| TS-17 | 建立生命周期日志 | sequence/reconciler | 事件计数、身份、正文不入日志 | planned |
| TS-18 | 建立候选请求与缓存门禁 | final-wire/cache gate | Standard 0 diff；TaskSpace changed set 可解释 | planned |
| TS-19 | 原子切换 TaskSpace Tool 投影 | turn/provider declaration | TaskSpace=容器+hosted；Standard 原样 | planned |
| TS-20 | 删除旧 sibling/manifest/RejectedNative 路径 | TS-05 清单 | 当前源码仅历史文档命中 | planned |
| TS-21 | 执行完整本地回归与对抗性审查 | tools/core/session | 定向矩阵、残留审计、缓存门禁 | planned |
| TS-22 | 最小真实产品验证并重排 R8 问题 | Docker benchmark/ledger/docs | 另行申请预算；trace/Map/token/cache/time | deferred |

## 7. Phase 与停点

### Phase A：事实与合同

- 单元：TS-01～TS-09。
- 收益：先确定 provider 输出身份、容器三类 item、状态正交和反馈边界，不在错误假设上写生产代码。
- 停点：provider 输出身份已经通过；若 TS-06 仍无法在不复制原生 Tool、按内容猜配或要求 Agent 回显 provider ID 的前提下
  表达节点归属，暂停并与用户讨论。

### Phase B：未接线内核

- 单元：TS-10～TS-17。
- 收益：容器 schema、decoder、reconciler、preflight、Router 和结果可独立测试，生产仍走旧路径。
- 停点：若必须修改普通 Tool schema、引入第二 Router/Map scheduler，或读取 Tool outcome 才能运行，判定设计偏离并停止。

### Phase C：请求与原子切换

- 单元：TS-18～TS-20。
- 收益：TaskSpace 一次切换到“容器 + 原生 hosted descriptors”，随后删除旧协议，不保留双路径或兼容开关。
- 停点：缓存门禁先报告精确 changed set；若需要真实缓存回归，按全局预算规则另行申请。

### Phase D：验证与问题重排

- 单元：TS-21～TS-22。
- 收益：本地回归和对抗性审查先关闭工程残留，再用最小真实运行验证 Agent 行动路径，最后重排 R8 问题全集。
- 停点：真实 Whale Agent run 必须单独说明 arm/sample/repeat、请求/token/费用上限并取得批准。

## 8. 验收矩阵

| 维度 | 必须成立 |
|---|---|
| 请求入口 | TaskSpace 只暴露一个 client/map 容器和 provider 原生 hosted descriptors；Standard 0 diff |
| 原生零侵入 | 普通 ToolSpec、参数、handler、权限、sandbox、hook 和结果不含 TaskSpace 概念 |
| 唯一动作事实 | 无 actions manifest、sibling 配对、shadow call、current node、单独 bind 或 Runtime 猜配 |
| Hosted | Provider 输出是执行事实；容器只引用并绑定；未绑定事实不丢失、不回滚、不重执行 |
| Map | Agent 声明节点/边/生命周期；Runtime 只做 canonical 合法性检查 |
| 状态正交 | Tool outcome 不自动改变节点；节点状态也不伪造 Tool outcome |
| 顺序 | Map 操作只在边界；普通 Work 依赖只来自 Map，不来自容器数组 |
| 失败边界 | 非法 client/map 零执行；已发生 hosted 输出如实保留；未知不伪装失败 |
| 反馈 | 原生结果无损；hosted/client/map 分别结算；唯一 canonical revision；不注入建议 |
| 缓存 | Standard final-wire 0 diff；TaskSpace 变化只来自批准的容器/hosted 投影 |
| 清理 | 当前生产代码无旧 actions/sibling/RejectedNative TaskSpace 路径和兼容分支 |

## 9. 日志合同

只记录机械身份和状态，不写 Tool 参数、命令、正文、prompt、节点 goal 或用户文本：

- `taskspace.sequence.received`: outer id、item count、schema hash、submitted revision；
- `taskspace.hosted.observed`: provider item id、provider item type、provider status；
- `taskspace.hosted.binding`: container item id、node id、provider item id、bound/invalid/unbound；
- `taskspace.sequence.preflight_rejected`: reason code、item ids、zero_client_dispatch、state_commit；
- `taskspace.sequence.client_dispatched`: item id、node id、Tool name；
- `taskspace.sequence.item_settled`: item id、item kind、item-local status、result ref；
- `taskspace.sequence.settled`: 三类计数、final canonical revision。

日志用于发现和重建，不是 Map、provider 输出或 Tool 结果的第二事实源。

## 10. 风险与安全停止

| 风险 | 发现信号 | 处理 |
|---|---|---|
| Hosted 节点归属含糊 | provider item 有稳定 id，但 Agent 的节点声明无法与该事实机械关联 | 停在 TS-06；不按内容猜配，也不要求 Agent 回显 provider ID |
| 容器 schema 膨胀 | TS-11 大量复制原生 description/schema | 检查 descriptor 派生；不做语义压缩掩盖重复 |
| Runtime 重建 Work DAG | 出现 item dependency、success gate、current/next node | 删除该逻辑，回到 Map 唯一依赖事实 |
| Tool outcome 污染节点 | 失败自动 block、成功自动 complete、finish 等待 Tool 成功 | TS-08/16 交叉矩阵阻断切换 |
| Hosted 原始事实被吞 | 无绑定或容器非法时结果消失 | 保留 unbound provider output，拒绝绑定而非拒绝事实 |
| Control 再成旁路 | sequence runtime 直接提交 Map 或复制 binding | 强制复用 TS-04 seam；停止集成 |
| Standard 被共享重构污染 | Tool schema/flags/request/result 任一变化 | 回退共享改动，先恢复 Standard 基线 |
| 新旧协议并存 | feature flag、兼容 parser 或双写同时可达 | 原子切换并删除旧路径；不保留兼容 |

## 11. 本地验证与付费边界

实现阶段至少运行：

```bash
cargo test -p codex-tools taskspace_sequence
cargo test -p codex-core taskspace_sequence
cargo test -p codex-core --test all standard_request_pair_preserves_the_complete_prefix
cargo test -p codex-core --test all taskspace_projection_policies_have_independent_request_pairs
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
```

测试名允许在 TS-01 阶段按真实模块调整，但必须形成固定矩阵。缓存门禁阻断时先报告 changed set 和原因，再申请专用
真实回归预算。本文不授权任何 Whale Agent run。

## 12. 外部依据与适用边界

1. [OpenAI Web Search](https://developers.openai.com/api/docs/guides/tools-web-search)：web search 是 provider 执行的
   hosted Tool，响应包含对应 Tool output item。
2. [OpenAI Image Generation](https://developers.openai.com/api/docs/guides/image-generation)：image generation 由 Responses
   API 执行并返回 `image_generation_call` 结果。
3. [OpenAI Function Calling](https://developers.openai.com/api/docs/guides/function-calling)：client Tool 使用 JSON Schema
   声明并按 call identity 返回执行结果，多调用本身不构成业务依赖。
4. [Model Context Protocol Tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)：Tool 名称、inputSchema、
   结果内容与执行错误有独立协议职责。
5. [DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls)：client function call 由模型生成、客户端执行；不同
   provider 能力不能被统一假设替代。

外部资料只用于确认 provider/client Tool 的执行边界和原生协议。TaskSpace 的 Map、节点归属、容器边界和 Runtime
不替 Agent 决策，仍由本项目产品定义约束。

## 13. 完成定义

本计划完成必须同时满足：

1. 三类容器 item 与 hosted 响应内身份有可复算合同；
2. client/map Tool 只通过容器，hosted capability 保持 provider 原生执行；
3. Tool 状态与节点生命周期交叉矩阵全部通过；
4. Standard wire 逐值不变，TaskSpace changed set 全部可解释；
5. 旧 actions/sibling/RejectedNative 当前生产路径彻底删除；
6. 日志、定向回归和缓存门禁通过；
7. 获批真实验证证明 Agent 可稳定生成容器且业务不回归；
8. R8 已知问题按新基线重新盘点，不能直接沿用旧根因。
