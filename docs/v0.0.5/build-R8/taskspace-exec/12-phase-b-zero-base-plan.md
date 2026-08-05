# Phase B 零基线重建计划

- Created: 2026-08-06
- Status: Active / Phase B0 in progress / ZB-06C verified / ZB-07 next
- Supersedes: [`02-engineering-plan.md`](02-engineering-plan.md) 中 TX-06B 之后的兼容迁移顺序
- Completed foundation: TX-06A (`54fc781fc`)
- Paid Whale Agent run: 本阶段删除与离线建设不需要

## 1. 决策

TaskSpace Exec 不再从当前 sibling 协议渐进迁移。先删除旧协议对 schema、parser、handler、sequence、context、response
和 feedback 的影响，再从 Standard 原生链与 canonical Action Map 原语建设新协议。

不做：

1. 旧 `taskspace_control.actions[]` 到新 calls 的 adapter；
2. 旧、新两套 Agent-visible schema 或 parser；
3. 为保持旧 TaskSpace 可运行而增加 feature branch、fallback 或补字段逻辑；
4. 把旧 sibling tests 改名后继续作为新合同；
5. 从旧 handler 参数反推新的 Map Tool 合同。

旧 TaskSpace rollout、专用对话事件和未发布数据不提供迁移、读取或 fallback；零基线只接受 Standard
原生历史格式与独立 canonical Action Map Store。

## 2. 零基线边界

### 2.1 可复用白名单

| Foundation | Reuse Boundary |
|---|---|
| Standard ToolSpec / Tool registry plan | Tool 事实、静态 schema、能力排序；TaskSpace 不修改普通 Tool |
| Standard ToolRouter / ToolCall | 原生权限、sandbox、hook、handler 和结果执行 |
| Provider response lifecycle | 原始 Function Call、Hosted output item、call ID 和完成边界 |
| Canonical Action Map | DAG、revision、状态转换、Store、restore/replay；不复用旧 Tool wire |
| Projection 三模式 | 只保留 Map 如何进入 context 的既有产品差异，不参与动作协议 |
| TX-06A neutral capability projection | Function/Freeform/Namespace 的单一机械 ToolSpec 投影 |
| Cache gate / request usage evidence | Standard 0-diff、静态 Tool declaration 和成本验证 |

### 2.2 必须删除的旧影响

| Surface | Old Influence To Remove |
|---|---|
| Tool declaration | `taskspace_control` 旧 schema 及 `actions[]` sibling manifest |
| Parser / handler glue | 旧 `TaskSpaceControlArgs`、wire parser、actions 校验和 sibling 专用输出 |
| Shared sequence | control-first、manifest/sibling 配对、TaskSpace barrier 与 prepared sibling executor |
| Runtime context | sibling call 的 node metadata、terminal carrier、额外 pairing receipt |
| Provider response | named control/tool-choice gate、隐藏 Tool 后置拒绝、terminal follow-up 控制 |
| Feedback | developer factual duplicate、supplemental carrier、状态与 Tool 失败重复包装 |
| Conversation backend | 专用 `TaskSpaceEventStore`、单 call owner、outer-control parent 和区别于 Standard 的历史替换路径 |
| Phase A prototype | `source:string` exec、Agent 回显 version/capability/item ID 的 decoder/preflight |
| Tests / prompts | 以旧合法序列、旧字段或旧反馈为正确答案的 active fixture 与说明 |

历史文档和 benchmark evidence 可保留，但不能被编译、注册、加载或作为新测试 fixture 输入。

## 3. 工作单元

| ID | Objective | Location / Target | Concrete Action | Resulting Behavior / Benefit | Side Effects | Verification / Stop | Status |
|---|---|---|---|---|---|---|---|
| ZB-01 | 冻结零基线边界 | R8 global constraints、TaskSpace Exec docs | 写明白名单、删除面和禁用过渡方案 | 后续实现不再被旧 schema/迁移思路牵引 | Complexity: 文档权威切换；Reach: 后续全部单元 | 文档交叉引用一致；旧计划明确 superseded | verified |
| ZB-02 | 删除 Phase A active prototype | `core/src/tools/taskspace_exec/` | 删除 source decoder、旧 plan/preflight/reconcile/catalog 接入；保留 TX-06A 中立投影 | 新实现没有伪 carrier、旧字段或候选 parser 可误用 | Complexity: 净删除；Reach: 仅未注册原型和 tests | `rg` 无 active `source/taskspace.plan/capability_id/item_id`；core build | verified |
| ZB-03G | 切换清零期缓存门禁 | cache final-wire fixtures、cache surface contract | 删除依赖旧 TaskSpace wire 的主动夹具；清零期只比较 Standard 两请求 final wire | 旧协议删除不会因夹具崩溃变成 `uncomparable`，同时 Standard 缓存回归仍阻断 | Complexity: 控制面净删除；Reach: 新 TaskSpace 发布保持阻断 | policy-only commit；Standard final-wire 与 cache gate PASS | verified |
| ZB-03A | 断开旧 control Tool 暴露 | `codex-tools taskspace_tool*`、registry plan、handler kind | 删除旧 declaration、ToolSpec 插入和 Router handler 注册 | Agent 和 Provider 不再看到旧 Map/sibling wire | Complexity: 净删除 Tool 声明；Reach: TaskSpace 暂不可运行，Standard Tool 不变 | registry/spec tests、Standard Tool snapshot；不得保留 adapter | verified |
| ZB-04A | 恢复 Standard 流式 Tool 调度 | `stream_events_utils.rs`、`session/turn.rs` | Tool item 到达时直接构造原生执行 future；恢复 `FuturesOrdered` 收集与统一落账；删除 response-completed 批处理入口和 provider declaration 中间态 | 普通 Tool 重新沿 Standard 原生 ToolCallRuntime 执行，不再等待旧 TaskSpace response sequence | Complexity: 一个原子执行链切换；Reach: response stream 与 Tool result 入历史 | core build；Standard Tool stream tests；不得同时保留 declaration 与 future 双轨 | verified |
| ZB-04B | 删除 sequence 执行抽象及附属状态 | `sequence*`、`provider_tool_declaration.rs`、`parallel.rs`、`context.rs` | 删除 manifest/preflight/prepared-sibling、sequence-only runtime 方法、node metadata context 与 active tests | 共享工具层不再携带旧 TaskSpace 序列、归属或 barrier 概念 | Complexity: 大量净删除；Reach: Tool runtime 与 tests | `rg` 无 active sequence/provider declaration/sibling 调用；parallel/router tests | verified |
| ZB-04C | 证明 Standard 执行基线 | Standard Tool/response/cache fixtures | 验证并行能力仍由 Tool 原生 parallel-safety 决定，串行 Tool 仍由原生锁保证；增加旧执行层 forbidden audit | 删除旧层没有改变 Standard Tool 行为或 provider 前缀 | Complexity: 低成本门禁；Reach: Phase B 后续全部单元 | Standard response/tool tests、cache gate PASS、forbidden set 为零 | planned |
| ZB-03B | 删除旧 control parser/handler | core control handler/args/output files | 删除已无调用的旧 wire parser、actions 校验、handler 和 sibling 输出 | 新 Map Tool 必须从 canonical Map operation 重新设计 | Complexity: 净删除实现；Reach: canonical Action Map 不变 | `rg` 无旧 control 类型或 actions wire；core build | verified |
| ZB-05 | 删除旧 Provider response 控制 | `session/turn.rs`、provider declaration/context helpers | 删除 named-control gate、terminal carrier、后置 follow-up/reject 和重复事实注入 | 新 response envelope 从原生完成事件零基础建立 | Complexity: 净删除分支；Reach: turn lifecycle 与 snapshots | Standard response tests、cache gate；不得新增临时 fallback | verified |
| ZB-06A | 删除旧协议说明与 active fixtures | TaskSpace base instructions/skill、旧 active tests | 移除要求旧 control/sibling/actions 的加载内容和测试 | Agent 不再接收已失效协议，测试不再奖励旧行为 | Complexity: 净删除内容；Reach: TaskSpace 暂无工作协议 | prompt/context snapshots；历史 docs 不计 active residual | verified |
| ZB-06B | 从 Map 删除工具执行状态 | rooted DAG、protocol、snapshot、Viewer | 删除 reservation/tool name/call index/release；只保留 Agent 声明的 `action_id -> node_id` 事实，结果引用不驱动节点生命周期 | Map 不再替 Tool 执行管理节点；Agent 可在动作尚无结果时完成节点 | Complexity: canonical schema 直接升级且不迁移；Reach: Map Store 新数据、API 和 Viewer | replay/invariant/schema/Viewer tests；旧字段静态为零 | verified |
| ZB-06C | 恢复 Standard 原生对话历史 | `TaskSpaceEventStore`、session state、rollout reconstruction | 删除 TaskSpace 专用历史替换、单 call owner、outer-control parent 和专用 compaction checkpoint；所有模式复用 Standard `ContextManager` | 新 Exec 从相同自然上下文基线建设，旧绑定模型不再预设 Hosted/client 归属 | Complexity: 删除第二历史后端；Reach: session/resume/compaction | Standard history/resume/compaction tests；canonical SQLite Map Store 保持不变 | verified |
| ZB-07 | 证明零基线 | 全仓 active source/test/config | 建立 forbidden-symbol audit 和 Standard regression | 新建设从可证明的干净基线开始 | Complexity: 一个静态审计；Reach: CI 增加低成本检查 | forbidden set 为零、Standard exact wire、cache gate PASS | planned |
| NX-01 | 建立纯 Map operation contract | canonical Action Map 邻接模块 + 新 ToolSpec | 从 Map transaction 原语定义 initialize/execute/reopen/read/finish，不含 Tool manifest | Map Tool 只管理 Map，节点工作归属只在 Exec 外层 | Complexity: 一个新合同；Reach: Map fixtures | schema/parser/transaction fixtures；出现 sibling 字段立即停止 | planned |
| NX-02 | 建立结构化 TaskSpace Exec schema | 新 `taskspace_exec` 模块 | 从 TX-06A 与 NX-01 生成 calls/hosted_bindings Tool-specific variants | Agent 只声明动作、原生输入和节点归属 | Complexity: 一个静态 Function schema；Reach: declaration size | 1/N client、hosted-only、empty reject、确定性字节 | planned |
| NX-03 | 建立 request-local identity 与唯一暴露 | Standard request Tool projection 的 TaskSpace 分支 | 从同一快照生成 Runtime identity、Exec declaration 和 nested catalog | TaskSpace 顶层只有 Exec + Hosted；Standard 0-diff | Complexity: 一个模式投影；Reach: provider payload/cache | payload snapshot、Router 一一对应、cache gate | planned |
| NX-04 | 验证 DeepSeek 最终 schema | disposable shape probe + ledger | 获批后运行 1 sample × 1 arm × repeat 1，最多 2 requests | 在 response/Map 大建设前验证最终 Function schema | Complexity: 零生产逻辑；Reach: 付费 API | 首个结构失败即停；启动前另行申请预算 | planned |

NX-04 通过后再重新设计 response-local envelope、Map admission、Router dispatch、Hosted persistence、一次反馈和生产验收；
旧计划中的 TX-07～TX-19 不自动继承，必须基于零基线代码重新盘点。

## 4. 阶段门禁

### Phase B0：Zero-Base Reset

- Entry: 用户明确要求不保留过渡方案，从 Standard 零基础建设。
- Units: ZB-01～ZB-07。
- Exit: active code/config/prompt/test 不再包含旧 sibling/control 协议；Standard 构建、Tool wire、sequence、response 与缓存门禁成立。
- Stop: 删除触及 canonical Action Map 数据事实、Store、projection 三模式或 Standard 原生 Tool 行为时立即停下重新划界。
- Clarification: 上述 Store 指 SQLite canonical Action Map Store；ZB-06C 删除的是旧方案的对话 Event Store，不是 Map 事实源。

### Phase B1：Clean Contract And Provider Gate

- Entry: B0 forbidden-symbol audit 和 Standard 回归通过。
- Units: NX-01～NX-04。
- Exit: 纯 Map operation、结构化 Exec、唯一能力暴露均有离线证据，且获批 Provider shape probe 支持继续。
- Stop: 需要恢复旧字段、复制普通 Tool schema、增加兼容 adapter、修改普通 Tool 参数或由 Runtime 推断 Agent 动作。

## 5. 执行记录

| Unit | Date | Evidence | Conclusion | Next |
|---|---|---|---|---|
| TX-06A | 2026-08-06 | `54fc781fc`；tools 154 passed / 1 ignored；TaskSpace Exec 33 passed；cache gate PASS | 中立 ToolSpec projection 保留；旧 TaskSpace prototype integration 不保留 | ZB-01 |
| ZB-01 | 2026-08-06 | `1143706a1`；全局约束、README、active plan 交叉引用 | 旧兼容迁移计划 invalidated；零基线计划成为唯一 active Phase B 计划 | ZB-02 |
| ZB-02 | 2026-08-06 | `2960ea03a`；`cargo check -p codex-core --lib` PASS；`codex-tools` 154 passed / 1 ignored | Phase A source-only prototype 无生产依赖并已从 active code 全部删除 | ZB-03A |
| ZB-03G | 2026-08-06 | `4472c2afa`；cache gate policy-only PASS；Standard 两请求 final-wire tests PASS | 清零期缓存门禁已删除旧 TaskSpace wire 夹具，只验证 Standard；发布保持阻断，NX-03 后重建 TaskSpace 合同 | ZB-03A |
| ZB-03A | 2026-08-06 | `cd327d938`；`codex-tools` 145 passed / 1 ignored；core build、ToolSpec 与 provider visibility unit tests PASS | 旧 control declaration、schema、registry handler 和 active schema fixtures 已删除；未增加替代或兼容入口 | ZB-04A |
| ZB-04A | 2026-08-06 | core build PASS；stream events 15 passed；mailbox、malformed arguments、missing client identity Standard tests 各 1 passed | Tool item 恢复为原生 future；turn 恢复 `FuturesOrdered` 落账；response completion 不再调用旧 sequence | ZB-04B |
| ZB-04B / ZB-03B / ZB-05 | 2026-08-06 | `5228efd80`；旧 sequence、control parser/handler、Provider response gate 静态为零；core build 与 Standard stream tests PASS | Tool response 已回到 Standard 原生调度与反馈路径，无兼容 adapter | ZB-06A |
| ZB-06A / ZB-06B | 2026-08-06 | core build；rooted DAG 19 tests；protocol 3 tests；schema fixtures 4 tests；Viewer 3 tests；core-skills 95 tests；skills 1 test | 旧 Prompt/Skill/fixture 已删除；Map 只保存 action-node 归属，不再保存或等待工具 reservation | ZB-06C |
| ZB-06C | 2026-08-06 | 删除约 2200 行旧 Event Store/codec/checkpoint/test；core/protocol build；历史后端目标测试 2 条；Standard rollout reconstruction 19 条；普通 Tool 错误反馈与 Hosted output 各 1 条；schema fixtures 4 条；Standard final-wire 1 条 | 所有模式只使用 Standard `ContextManager` 与 Standard rollout；模式切换不再搬运聊天历史；canonical SQLite Action Map Store 未改动；不兼容旧专用事件 | ZB-07 |
