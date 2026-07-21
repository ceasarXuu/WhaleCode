# R7 连续动作回归修复计划对抗性审查

- Review date: 2026-07-21
- Target commit: `e62714c1d`
- Reviewer launch id: `019f81fd-637e-70a0-972e-2bc9031b8a17`
- Reviewer nickname: Einstein
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，空白上下文；只读审查
- Initial verdict: `REJECT`
- Review scope: FLA-3.5 计划、R7/五层后续计划、机器 authority 与相关 Tool 生产入口

## 1. 初审发现与处置

| # | 级别 | 发现 | 处置 | 计划修订 |
|---|---|---|---|---|
| 1 | Blocking | 通用对象 decorator 未覆盖 Responses/custom freeform Patch 与 code mode | Accept | CA-0 冻结 WireApi/ToolSpec/source 矩阵；TaskSpace freeform Patch 使用同名 function wire 投影，仍复用原 handler；任一生产入口失败即阻塞 |
| 2 | Blocking | approval/hook/sandbox 深处执行，简单前置 Map commit 会产生错误时序 | Accept | 冻结共享 `prepare -> commit + reserve -> execute -> post`，补 denial、取消和新/旧 lease 状态表及生产入口 |
| 3 | Blocking | 单 call typed feedback 与旧“双 call 两结果”合同冲突 | Accept | 定义 `TaskSpaceCarrierOutcome { transition_fact, Opaque<ToolCallOutput> }`；FLA-3.5 拥有 transport，FLA-5 只做 conformance；按子载体而非整帧做保真 hash |
| 4 | Blocking | CA-5 错用 FLA-8 评估合同、提前解封 held-out，旧 combined 指标分母失效 | Accept | CA-2 新建专用 `continuous-action-evaluation-v1.json`；使用独立 carrier-validation 样本和 `transition_carrier_rate`；FLA-8 held-out 保持 sealed |
| 5 | High | “真实动作”无机器资格定义，可能出现名称 allowlist 和语义越界 | Accept | registry 增加唯一 `TaskSpaceCarrierCapability`；只描述执行载体能力，不判断动作价值或 no-op |
| 6 | High | code mode nested calls 绕过 barrier、lease attribution 与 one-Patch gate | Accept | 顶层 code-mode call 作为 carrier；nested calls 继承新 lease；复用 cell/turn barrier，Patch 计数 turn-wide |
| 7 | High | 会话静态 schema 与动态 MCP/capability 变化冲突 | Accept | 稳定单位改为 immutable capability epoch；只在 request 间创建新 epoch，Map 状态不得触发；所有发现路径走同一 decorator |
| 8 | High | FLA-3.5/5/7 与 R7 Phase E/G 所有权和验收重复 | Accept | 新增单一阶段 DAG；FLA-5 只做 conformance；FLA-7 是 Phase E 唯一实现；Phase G 是 FLA-8 的共享四臂子矩阵 |
| 9 | High | 回滚未覆盖 CA-2 candidate authority/artifact，删除时点冲突 | Accept | CA-2 使用 candidate namespace，不切 active authority；CA-4 强制 rollback drill；CA-6 单 promotion commit 或完整 revert |
| 10 | Medium | authority 状态枚举与规格冲突、缺少机器 schema/source identity | Accept | 当前非法状态收敛到五值枚举；CA-0 要求 authority JSON Schema、production commit/source/wire hashes |

## 2. 初审确认未发现的问题

- 原 FLA-6 “移除 `required_next_call`”实验已删除，无残留计划冲突。
- 三种 projection policy 继续共享 canonical state、Tool 和 result，只改变 emission。
- R5、D.2、D.4 与 FLA-2/3 历史结果未被改写。
- reviewer 对 governing document 与登记 artifact 执行 SHA-256 对账，初审时一致。

## 3. 修订后待复审门禁

1. freeform/function/code-mode/MCP 的 carrier 形态均有明确 probe 与失败停止条件。
2. prepare、approval、commit、reservation、execute、hook、取消的时序不存在副作用窗口。
3. typed outcome 在 text/image/MCP/truncated/error 上保留 opaque Tool 子载体。
4. FLA-3.5 使用独立评估合同，不读取 FLA-8 held-out。
5. FLA-3.5、FLA-4 至 FLA-8、R7 Phase E-H 只有一条实施与决策路径。
6. candidate、promotion、rejection 和 rollback 不产生双生产合同。

## 4. 复审

- Target commit: `2d5ee3ac5`
- Reviewer launch id: `019f8210-dfc2-7333-b7a7-8456f0c5d4d2`
- Reviewer nickname: Harvey
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，新的空白上下文；只读审查
- Verdict: `REJECT`

复审确认 G（capability epoch）与 I（candidate/rollback 单轨）已关闭，Patch function 投影和 FLA-5 transport
ownership 方向成立；仍发现 3 个 Blocking、3 个 High、1 个 Medium：

| # | 级别 | 复审发现 | 处置 | 二次修订 |
|---|---|---|---|---|
| R1 | Blocking | sandbox 首次执行后才知道 denial，升级审批仍可能落在 Map commit 后；上传类参数改写有副作用 | Accept | 冻结 commit 前一次性 escalation/network 预授权；commit 后禁止再次审批；上传/materialization 归 execute；PreparedToolCall 固定 grant/args/cancel 字段 |
| R2 | Blocking | code-mode 顶层 `exec` 本身是 freeform，仍无法结构携带 transition | Accept | TaskSpace-only 同名 function 投影 `{source, taskspace_transition}`；同一 handler 同时接收 Standard Custom 与 TaskSpace Function；source byte exact gate |
| R3 | Blocking | reject/cancel 时没有 ToolCallOutput，原 typed outcome 强制字段不成立；AfterToolUse 可替换 output | Accept | outcome 改为 `RejectedBeforeCommit` / `CommittedNotExecuted` / `Executed` 和类型；PostToolUse failure 独立，不能丢弃原 output |
| R4 | High | 旧名称/source/命令 classifier 仍可能控制 attribution/reservation | Accept | CA-3 明确删除 gate/lease 上的旧 classifier；CA-4 静态审计 capability metadata 是唯一机械输入 |
| R5 | High | Phase E/F/G 虽声明别名，仍保留独立实施项和 gate；lifecycle authority 仍写 FLA-5/7 | Accept | 删除 E/F/G 独立执行/gate；实现分别归 FLA-6/7/8；lifecycle activation 唯一归 FLA-7；新增 machine ownership lint |
| R6 | High | 完整 CA-5 合同仍可能在 probe 后冻结；可执行规格摘要保留旧 combined 指标 | Accept | 完整样本/重复/seed/阈值/hash 全部移至 CA-0；CA-1/5 不挂载 FLA-8 held-out；candidate 旧指标 lint 失败；规格摘要改 carrier 指标 |
| R7 | Medium | contract/implementation/runtime 状态仍共用不同含义的 `status`，且无 schema | Accept | 立即拆为三个字段，新增 authority 与 production-manifest JSON Schema，更新 generator 和合同测试 |

## 5. 第三轮复审

- Target commit: `b739ea085`
- Reviewer launch id: `019f8228-393f-76d2-a80e-b05f216f10e0`
- Reviewer nickname: Kant
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第三个空白上下文；只读审查
- Verdict: `REJECT`

第三轮确认 A、D、E、F、G 已关闭，Patch/code-mode 投影、评估隔离、classifier 删除和 capability epoch 已形成
可执行门禁；继续发现 3 个 Blocking、2 个 High：

| # | 级别 | 第三轮发现 | 处置 | 三次修订 |
|---|---|---|---|---|
| T1 | Blocking | execution 已开始后可能返回 FunctionCallError 或上传失败，没有 ToolCallOutput；原三分支无法表达 | Accept | 冻结 execution-start；Executed 内扩为 Returned/Failed/CancelledAfterStart，PostToolUse 扩为 NotRun/Succeeded/Failed；原 outcome 不可被 hook 覆盖 |
| T2 | Blocking | 当前 MCP mapper 无法同时保留 content、structuredContent、isError、_meta | Accept | 新增 `McpToolOutputV1`，冻结安全处理、完整 retained store、截断、wire frame 顺序和逐阶段 hash；CA-1 强制 round-trip |
| T3 | Blocking | `25` 将 Phase F 引用写成 FLA-7/8，`01/33` 写 FLA-6/7 | Accept | 统一为 Phase E/F/G 只读引用 FLA-6/7/8，并由 ownership JSON 唯一约束 |
| T4 | High | candidate_status 只有枚举，没有 candidate entity；candidate manifest 与 active const 冲突 | Accept | authority/schema 增加 candidates 与状态迁移；manifest schema 增加 active/candidate 条件模式；合同测试加入合法/非法 candidate fixtures；CA-0 实现 generator |
| T5 | High | CA-3 漏列 orchestrator、hook、MCP upload 等真实时序 owner | Accept | 补齐 orchestrator/hook events/parser/mcp_tool_call/mcp_openai_file/protocol models；增加 commit 后无 approval、prepare 无上传断言 |

## 6. 最终复审

- Target commit: `b0eafbba8`
- Reviewer launch id: `019f823c-14e1-7683-9b5e-63eefea8a892`
- Reviewer nickname: Gauss
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第四个空白上下文；只读审查
- Verdict: `REJECT`

第四轮确认 T3 已关闭，连续动作结构方向、Runtime 语义边界、CA-0/2/5/6 顺序和 held-out 隔离均未被否定；
仍发现 3 个 Blocking、1 个 High、1 个 Medium：

| # | 级别 | 第四轮发现 | 处置 | 四次修订 |
|---|---|---|---|---|
| Q1 | Blocking | `execution_started` 以首个业务副作用定义，无法覆盖只读 Tool 立即失败、handoff 前取消和无 payload StartFailure | Accept | 改为任何 handler 工作前的原子 runtime handoff；StartFailure/CancelledBeforeStart 携带 factual payload；新增边界竞态 oracle |
| Q2 | Blocking | MCP Optional 字段被 null/bool 降维，未知 block、retention/provider mapping failure 没有完整 outcome | Accept | presence-aware 字段、有 source index 的 policy-visible 原始 JSON block、retention/delivery facts；从原始 MCP fixture 做跨阶段 fault round-trip |
| Q3 | Blocking | PreToolUse 会运行任意外部命令，不能被称为无副作用 prepare | Accept | 保持现有 hook 能力，不建 TaskSpace 分支；明确为 pre-commit 外部步骤并冻结 PreHookFact；零副作用保证只覆盖 ordinary Tool handler |
| Q4 | High | candidate 单文件 schema 可接受 ID mismatch、重复 promoted、伪 backlink，且 promoted 无 revert | Accept | candidate 实体移至独立 namespace，避免 active authority/self-hash；新增 pending/reverted 状态和跨文件 linter/反例/active pointer 原子切换 |
| Q5 | Medium | 生产入口清单漏 central router/spec/tool_registry_plan，手写 ownership 不能证明闭包 | Accept | CA-0 从 ToolSpec/ToolPayload/router 生成 entry closure；CA-4 每条路径要求 decorator/parser/epoch/outcome mapper 精确命中一次 |

主审额外确认原 `candidates[] + active_authority_sha256` 会在 candidate 写回 active authority 时产生自引用风险；
四次修订将 candidate record 移出 active authority，active authority 在 CA-6 前保持字节不变。

## 7. 第五轮复审

- Target commit: `224697c3c`
- Reviewer launch id: `019f8255-3bc0-7293-87d6-a4006c2402ea`
- Reviewer nickname: Linnaeus
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第五个空白上下文；只读审查
- Verdict: `REJECT`

第五轮确认 Q2、Q3、Q5 已关闭，后续计划 ownership/gate/decision 冲突检查为 PASS；Q1、Q4 仍 open：

| # | 级别 | 第五轮发现 | 处置 | 五次修订 |
|---|---|---|---|---|
| P1 | Blocking | outcome 缺 commit 原子失败阶段，`CancelledAfterStart` 没有 CancellationFact | Accept | 增加 Commit stage 与 `CommitFailedNoState(TransactionFailureFact)`；所有取消带 generation/观测点/handler-start/partial-effects；latch/fault 每例唯一 variant |
| P2 | High | candidate 可使用伪 commit、任意单 artifact、伪 source authority，并可直接 promoted/reverted | Accept | candidate id=active snapshot + 8 个角色 hash 的规范摘要，另有真实 candidate_commit；source/active authority 双向一致；artifact 限定 namespace/commit/hash；first-parent history 重放；补全反例 |

主审同时将 production pointer 扫描移出“candidate 目录存在”条件：即使目录缺失，孤立 active pointer 也必须失败。

## 8. 第六轮复审

- Target commit: `c2aeb4ae2`
- Reviewer launch id: `019f8269-2144-74f0-b9a2-089e132ff7bf`
- Reviewer nickname: Kuhn
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第六个空白上下文；只读审查
- Verdict: `REJECT`

第六轮确认 P1、Q2、Q3、Q5 已关闭，后续计划冲突检查继续为 PASS；P2 仍有 1 个 Blocking、3 个 High：

| # | 级别 | 第六轮发现 | 处置 | 六次修订 |
|---|---|---|---|---|
| S1 | Blocking | promoted/reverted 只校验 pointer，未与迁移 commit 当时的 authority L4/L5/runtime 加密绑定 | Accept | 完整 first-parent 事件重放；在每个 promoted/reverted 事件 commit 读取 authority/production manifest，校验 candidate L4/L5 hash、active pointer 或 baseline byte hash |
| S2 | High | 只看 HEAD/HEAD^，非法初态可被后续无关提交掩盖 | Accept | 枚举 manifest 全部 first-parent 变更 commit，从初始 evaluation 顺序重放每次状态迁移，并补 worktree 未提交尾事件 |
| S3 | High | 八个角色可复用同一路径/任意 blob，测试 fixture 正在这样做 | Accept | 角色专属文件名/schema/artifact_role，路径必须唯一；fixture 使用不同 role hash；实际文件逐 role 解析验证 |
| S4 | High | 字符串前缀允许 `<id>/../`，未拒绝 symlink/Git tree mode | Accept | `GetFullPath` 后验证仍位于 namespace；拒绝 symlink；candidate commit tree mode 仅允许普通文件；修正边界反例 |

## 9. 第七轮复审

第六轮仍含 Blocking finding。六次修订提交后必须由第七个空白 reviewer 复核 P2 的历史、authority 绑定、角色与
规范路径四项门禁。
