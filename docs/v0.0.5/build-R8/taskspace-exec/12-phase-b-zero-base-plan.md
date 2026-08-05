# Phase B 零基线重建计划

- Created: 2026-08-06
- Status: Active / Phase B0 in progress / ZB-01 and ZB-02 complete
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
| Phase A prototype | `source:string` exec、Agent 回显 version/capability/item ID 的 decoder/preflight |
| Tests / prompts | 以旧合法序列、旧字段或旧反馈为正确答案的 active fixture 与说明 |

历史文档和 benchmark evidence 可保留，但不能被编译、注册、加载或作为新测试 fixture 输入。

## 3. 工作单元

| ID | Objective | Location / Target | Concrete Action | Resulting Behavior / Benefit | Side Effects | Verification / Stop | Status |
|---|---|---|---|---|---|---|---|
| ZB-01 | 冻结零基线边界 | R8 global constraints、TaskSpace Exec docs | 写明白名单、删除面和禁用过渡方案 | 后续实现不再被旧 schema/迁移思路牵引 | Complexity: 文档权威切换；Reach: 后续全部单元 | 文档交叉引用一致；旧计划明确 superseded | verified |
| ZB-02 | 删除 Phase A active prototype | `core/src/tools/taskspace_exec/` | 删除 source decoder、旧 plan/preflight/reconcile/catalog 接入；保留 TX-06A 中立投影 | 新实现没有伪 carrier、旧字段或候选 parser 可误用 | Complexity: 净删除；Reach: 仅未注册原型和 tests | `rg` 无 active `source/taskspace.plan/capability_id/item_id`；core build | verified |
| ZB-03 | 删除旧 control Tool wire | `codex-tools taskspace_tool*`、core control handler/args | 移除旧 declaration、parser、handler kind 和注册 | 新 Map Tool 必须从 canonical Map operation 重新设计 | Complexity: 净删除 Tool；Reach: TaskSpace 暂不可运行，Standard Tool 不变 | registry/spec tests、Standard Tool snapshot；不得保留 adapter | planned |
| ZB-04 | 恢复共享 sequence 为 Standard 基线 | `sequence*`、`parallel.rs`、`registry.rs` | 删除 TaskSpace manifest/preflight/prepared-sibling 分支和 context glue | 普通多 Tool 调用重新只有一条 Standard 执行链 | Complexity: 大量净删除；Reach: shared execution tests | Standard sequence/parallel/router 全绿；TaskSpace 专属 symbols 为零 | planned |
| ZB-05 | 删除旧 Provider response 控制 | `session/turn.rs`、provider declaration/context helpers | 删除 named-control gate、terminal carrier、后置 follow-up/reject 和重复事实注入 | 新 response envelope 从原生完成事件零基础建立 | Complexity: 净删除分支；Reach: turn lifecycle 与 snapshots | Standard response tests、cache gate；不得新增临时 fallback | planned |
| ZB-06 | 删除旧协议说明与 active fixtures | TaskSpace base instructions/skill、旧 active tests | 移除要求旧 control/sibling/actions 的加载内容和测试 | Agent 不再接收已失效协议，测试不再奖励旧行为 | Complexity: 净删除内容；Reach: TaskSpace 暂无工作协议 | prompt/context snapshots；历史 docs 不计 active residual | planned |
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
| ZB-02 | 2026-08-06 | `cargo check -p codex-core --lib` PASS；`codex-tools` 154 passed / 1 ignored | Phase A source-only prototype 无生产依赖并已从 active code 全部删除 | ZB-03 |
