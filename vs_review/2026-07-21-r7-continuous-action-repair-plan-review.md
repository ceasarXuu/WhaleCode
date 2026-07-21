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

二次修订完成后必须再次使用新的空白 reviewer。R1-R3 属于 Blocking，不能以本报告作者自审替代最终复审。
