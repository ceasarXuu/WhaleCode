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

- Target commit: `ff85e5c32`
- Reviewer launch id: `019f8281-b3ba-7110-bb7a-d81da777b2c2`
- Reviewer nickname: Parfit
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第七个空白上下文；只读审查
- Verdict: `REJECT`

第七轮确认 P1、Q2、Q3、Q5 已关闭，后续计划冲突检查继续为 PASS；P2 仍有 1 个 Blocking、3 个 High：

| # | 级别 | 第七轮发现 | 处置 | 七次修订 |
|---|---|---|---|---|
| U1 | Blocking | promoted 只检查 candidate target 存在，允许旧 target 并存；reverted 不检查 production L4/L5 精确恢复 | Accept | candidate 增加 active production manifest byte snapshot；authority/production target 集合要求规范化后精确相等；promoted 的 L4/L5 单值精确来自 candidate，reverted bytes 精确回 baseline |
| U2 | High | 历史事件只验状态/activation，未对每版重跑 schema、ID、commit、source 和 artifact | Accept | history loop 对每个 manifest 版本执行 schema + 完整 manifest/artifact integrity，再执行状态和 activation |
| U3 | High | manifest 的 role/path/hash 约束不等于 artifact 内容 schema | Accept | 新增版本化 candidate artifact schema，八个 role-specific 条件分支要求各自结构；实际文件逐个 `Test-Json`，空壳反例失败 |
| U4 | High | 只检查叶文件 symlink，父目录 symlink 可逃逸 | Accept | 从 repo root 到 artifact 的每个现存路径分量拒绝 ReparsePoint，并对最终 resolved path 再做 namespace containment |

## 10. 第八轮复审

- Target commit: `b45288ca8`
- Reviewer launch id: `019f8291-e670-7bc0-98e8-136cd9356338`
- Reviewer nickname: Anscombe
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第八个空白上下文；只读审查
- Verdict: `REJECT`

第八轮确认 U2、U4 已关闭，后续计划冲突检查仍为 PASS；U1、U3 及两个执行合同歧义仍有 1 个 Blocking、3 个 High、
1 个 Medium：

| # | 级别 | 第八轮发现 | 处置 | 八次修订 |
|---|---|---|---|---|
| V1 | Blocking | promoted 的期望集合仍来自已修改 authority；终止 candidate 可因 manifest 不再变化而跳过当前 baseline 复验 | Accept | candidate 自有完整 L4/L5 activation targets 并纳入 content id；promoted 双侧精确对 candidate；所有状态持续复验当前 raw-byte baseline |
| V2 | High | role schema 与正例仍允许空 object、任意单元素数组和空 held-out identity | Accept | 八个严格 `$defs` 规定嵌套字段、枚举、基数、sealed identity 和 `additionalProperties:false`；每个角色增加空载荷负例 |
| V3 | High | denial 后才能得到动态 host scope，无法在 commit 前机械预授权且保持现有窄范围授权 | Accept | 可确定 scope 继续 pre-authorize；denial-derived managed-network host 使用唯一 deferred authorization，保留原窄 scope guardian/approval，不猜命令、不申请宽 grant |
| V4 | High | “真实动作必须执行返回”与 `CommittedNotExecuted` 同时存在，指标口径矛盾 | Accept | 统一定义为完成 commit+reservation 的 ordinary dispatch；另报 `carrier_execution_started_rate`，不得把 carrier rate 当执行成功率 |
| V5 | Medium | provider-native `ToolSearch/LocalShell/ImageGeneration/WebSearch` 未明确 carrier disposition | Accept | Wire matrix 穷举全部 ToolSpec/ToolPayload；native built-ins 明确 non-carrier，Namespace 为 container，function/freeform/MCP 走共享 carrier/projection |

## 11. 第九轮复审

- Target commit: `b29e1398f`
- Reviewer launch id: `019f82ad-0297-7d02-a81d-b5eb46454146`
- Reviewer nickname: Heisenberg
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第九个空白上下文；只读审查
- Verdict: `REJECT`

第九轮确认 V3、V4 已关闭；V1、V2、V5 仍 open，阶段冲突检查因 promotion metadata 可越权而为 FAIL，共 1 个
Blocking、3 个 High：

| # | 级别 | 第九轮发现 | 处置 | 九次修订 |
|---|---|---|---|---|
| W1 | Blocking | promotion 只比较 path/hash/phase，丢弃 role/layer/status；FLA-3.5 active 状态在 authority/manifest schema 中不可表达 | Accept | candidate 保存 role-keyed 完整 authority/production record；只激活 L4/result，保留 projection/lifecycle 原状态；schema 增加 FLA-3.5 activation 与 blocking repair active 状态 |
| W2 | High | history 只枚举 manifest commit，漏掉中间 authority/production drift；candidate commit 无 ancestry；旧 terminal candidate 会永久阻塞后续合法 promotion | Accept | 重放 manifest/authority/production commit 并集；要求 candidate commit 是事件祖先；terminal candidate 通过 `superseded_by` 显式退出 authority claim，且与后继首次 `promotion_pending` 同 commit 绑定 |
| W3 | High | 八类 schema 仍允许字段齐全但语义空洞的载荷 | Accept | schema 升级为 action-conditional transition、完整 outcome facts、具名 oracle、精确 rollback target 与完整评估预注册；增加 well-formed hollow 负例 |
| W4 | High | capability matrix 没有 ToolPayload 维度，closure hash 未绑定实际生成产物与 Rust enum inventory | Accept | entry closure 成为第九个候选 artifact；矩阵覆盖 WireApi/ToolSpec/ToolPayload/source/route；测试从 Rust enum 源生成 inventory 并与 schema/closure 精确对账 |

## 12. 第十轮复审

- Target commit: `6d4a5ff4e`
- Reviewer launch id: `019f82d2-761d-7441-b8e2-44f1f4295adb`
- Reviewer nickname: Boyle
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第十个空白上下文；只读审查
- Verdict: `REJECT`

第十轮确认连续动作与单 pipeline 的方向未被否定，V4 的 dispatch/execution 口径已关闭；机器晋升与证据合同仍有
2 个 Blocking、4 个 High、1 个 Medium：

| # | 级别 | 第十轮发现 | 处置 | 十次修订 |
|---|---|---|---|---|
| X1 | Blocking | promotion 只校验 L4/L5 和 repair status，允许 L1、phase gate 或 repair metadata 越权漂移 | Accept | 从 candidate 冻结 baseline 派生完整 expected authority/production，只允许显式 delta，规范化后比较全对象；加入 L1 与 repair metadata exploit |
| X2 | Blocking | lifecycle、metric、rollback、evaluation 仍可用语义空壳通过 | Accept | 生命周期改精确事实表；完整指标公式/单位/阈值改 const；sample identity/order 唯一；FLA-8 保存 sealed ids/mount 断言；rollback 与 candidate snapshots 交叉绑定 |
| X3 | High | 历史 artifact 从当前 worktree/candidate commit 读取，无法发现中间污染后恢复 | Accept | history union 扩到 candidate namespace；每个事件 commit 重读九类 artifact、authority、production 并做 schema/hash/tree-mode/candidate-commit 双重校验 |
| X4 | High | `superseded_by` 首次写入后仍可改写 | Accept | 首次写入绑定后继首个 `promotion_pending` commit，之后逐事件要求 id 不变且不得清除 |
| X5 | High | entry closure 只有枚举名与 contains，缺源码派生的精确闭包 | Accept | 增加五类源码 hash、canonical generation digest、双 wire、registration/invocation 拆分、route/reason 枚举和 closure/matrix 精确集合校验 |
| X6 | High | deferred managed-network terminal outcome 丢失 scope/denial/decision 等事实 | Accept | 四种 variant 全量保留 kind、scope、denial hash、decision、grant id；Denied 另保留 factual error |
| X7 | Medium | baseline sync 与主 validator 没有真实 FLA-3.5 操作入口 | Accept | CA-0 明确新增 candidate 专用 generator/transition command，并登记 `-Phase FLA-3.5` 独立门禁；baseline generator 不扩成伪晋升器 |

阶段冲突检查本轮为 `FAIL`，直接原因是 promotion 可夹带与 FLA-3.5 无关的 authority 变更；X1 将该问题升级为
完整快照的 exact-delta 门禁。所有 finding 均接受，不改变连续动作产品合同，不进入生产实现。

## 13. 第十一轮复审

- Target commit: `d47fb73c3`
- Reviewer launch id: `019f82ea-9820-7162-aa50-e4fda58eb1c7`
- Reviewer nickname: Popper
- Model: `gpt-5.6-sol`, reasoning `xhigh`
- Context: `fork_context=false`，第十一个空白上下文；只读审查
- Verdict: `REJECT`

第十一轮确认连续动作、Runtime 语义中立和单 Tool pipeline 的产品方向未被否定，但发现 4 个 Blocking、5 个 High、
2 个 Medium；phase-conflict 为 `FAIL`：

| # | 级别 | 第十一轮发现 | 处置 | 十一次修订 |
|---|---|---|---|---|
| Y1 | Blocking | `-Phase FLA-3.5` 在零 candidate/零实现时误报通过 | Accept | 分离 `FLA-3.5-Scaffold` 与唯一 completion gate；当前 completion 必须失败，`All` 只验证 active baseline |
| Y2 | Blocking | candidate 可选择已经漂移的 authority/production 作为 baseline | Accept | CA-0 在 candidate 外创建不可变 baseline，独立 commit 固定父 commit/hash/mode；candidate 只能引用它，历史从该 commit 重放 |
| Y3 | Blocking | CA-6 要同时改 FLA-8/文档，却被 exact-delta 拒绝 | Accept | FLA-8 与文档完全移出 CA-6；只预注册并改变 repair、L4/L5、authority status、manifest version/pointer/hash |
| Y4 | Blocking | v1 artifact 是 descriptor，不是 executable evidence | Accept | 明确 v1 仅为 plan scaffold；CA-0 必须生成 executable v2，八类真实 schema/golden/fixtures/diff-derived rollback/evaluation 才可建 candidate |
| Y5 | High | 历史事件使用当前 schema/linter 重新解释 | Accept | candidate pin v2 schemas/generator/transition/verifier blobs；从 baseline 枚举其变更，历史使用 pinned verifier；manifest mode 也校验 |
| Y6 | High | supersession 可晚于后继第一次 pending | Accept | 计算后继最早 `promotion_pending` commit，要求首次 backlink 与其同 commit，之后 id 永久不变 |
| Y7 | High | entry closure 仍是自证 generated | Accept | CA-0 使用 Rust AST/compiled registry-plan generator 覆盖 provider/profile/alias/MCP/code-mode/mapper；completion 重新生成并 exact diff |
| Y8 | High | denial 后自动重跑 handler 可能重复首次副作用 | Accept | 增加 attempt/partial-effects/narrow grant facts；批准不自动 replay，失败与 grant 忠实返回，由 Agent 决定是否重试 |
| Y9 | High | FLA-3.5 提前占有 FLA-4/5/7 | Accept | L4/L5-result 在 3.5 仅 repair-active，4/5 分别升 active；carrier oracle 不含 lifecycle/recovery；FLA-7/8 独占原范围 |
| Y10 | Medium | execution-start、request/cache/Standard hash 观测不闭合 | Accept | 增加 carrier conservation=100%、paired amplification 公式、epoch cache 分组和运行时重算 Standard hashes |
| Y11 | Medium | PowerShell JSON 接受重复 key | Accept | 增加 strict I-JSON duplicate-key rejection，并纳入 candidate/history/artifact 读取 |

所有发现均接受。Y8 不削弱连续动作合同：第一次 ordinary Tool 已与 transition 同 call dispatch；执行失败后 Runtime
不替 Agent 决定重试。Y9 只修正阶段所有权和状态语义，不撤回 carrier transport。

## 14. 第十二轮复审

第十一轮仍含 Blocking/High finding。十一次修订提交后必须由第十二个空白 reviewer 复核 Y1-Y11、v1/v2 门禁分离、
不可变 baseline、exact-delta 可实现性和 FLA-3.5/4/5/7/8 所有权。
