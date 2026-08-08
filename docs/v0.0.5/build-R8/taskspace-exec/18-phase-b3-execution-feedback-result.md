# Phase B3 MS-03、EX-06～EX-08 执行与反馈结果

- 日期：2026-08-07；2026-08-09 更新
- 状态：blocked；首轮 fresh adversarial review 重新打开 cancellation/shutdown/组合验收缺口
- 核心提交：`1347606e0`、`60fd7e0a8`、`03acb2db6`、`e5925b45d`、`702f885a0`
- 真实 Whale Agent run：0

## 1. 产品结果

TaskSpace 已接入唯一生产执行链：

1. 整个 `taskspace_exec` 先完成结构、Map、节点、Tool 参数和 Hosted 归属预检；失败时不写 Map，也不执行 client Tool。
2. 通过预检后，候选 Map 和每个 client Action 的 `pending` 归属先写入 canonical Store，再立即复用 Standard
   `ToolRouter` 并行或串行执行原生 Tool。
3. 每个 Tool 完成后立即独立结算为 `succeeded`、`failed` 或 `cancelled`，不等待同批慢 Tool。Tool outcome 不改变
   Node 生命周期；节点推进仍由 Agent 显式 Map 操作决定。
4. 若 Exec future 在 Tool 完成前中断，已落账 Action 保持可恢复读取的 `pending`；Runtime 不自动重试 Tool，也不把
   未发生的结果补写为成功或失败。
5. Provider-hosted 结果使用 Responses wire 的真实 `output_index`、Provider ID、Tool 类型和终态 outcome，与 Agent
   声明的 `node_ids[]` 逐项核对。一个 Hosted Action 可归属多个 Work Node；漏绑、错绑、重复、无 Exec、多个 Exec
   或缺少 wire index 均在 client/Map 副作用前拒绝。
6. Agent 只收到一个 outer Function Tool output。它包含稳定的 Map identity、dispatch 时 revision、显式读取结果、每个 client Tool
   的原生 `ResponseInputItem` 和 Hosted 机械归属，不复制 Hosted 原始结果，不注入 developer carrier，也不再解释
   Tool 语义。
7. TaskSpace 请求顶层只暴露 `taskspace_exec` 和 Provider-hosted Tool；普通 client Tool 只能经 Exec 内部进入原生
   Router。Standard 继续使用原来的顶层 Tool registry 和 `handle_tool_call` 返回合同。

## 2. 机械边界

- Agent 决定 calls、参数、顺序和节点归属；Runtime 只检查合同与 Map 硬规则。
- client Tool schema 从原生 `ToolSpec` 派生，普通 Tool 参数、handler、alias、MCP、Tool Search 和并行策略未被修改。
- Runtime 只记录 Action identity、Tool 名、outcome 和所属 Node，不把 Tool 参数、过程或原始结果复制进 Map。
- 取消事实来自单次原生 Tool 调度实际走过的取消分支，不再用整批 cancellation token 的最终状态推断，避免把已成功
  Tool 错记为 `cancelled`。
- response-local reconciler 只管理当前 Provider response，不建立 Event Store、未绑定池、默认 Root 归属或跨响应重放。

## 3. 日志

新增稳定的结构化事件：

| 事件 | 作用 |
|---|---|
| `taskspace.exec.response_finalized` | 记录 response 是否完整、Exec 数和 Hosted 事实数 |
| `taskspace.exec.hosted_fact_observed` | 记录真实 index、Provider ID、Tool 和 outcome |
| `taskspace.exec.preflight_accepted` | 记录 request revision 与各类动作数量 |
| `taskspace.exec.candidate_persisted` | 记录副作用前已固化的 revision 与 Action 数量 |
| `taskspace.action_settlement_queued` | 记录 Tool 返回后已同步投递的窄化终态事实 |
| `taskspace.action_settlement_committed` | 记录 outcome-only 事务在 latest Map head 的提交 |
| `taskspace.action_settlement_failed` | 记录会持续阻断后续 TaskSpace 请求的永久结算故障 |
| `taskspace.action_settlement_barrier_completed` | 记录请求前 FIFO 屏障结果 |
| `taskspace.action_settlement_recovery_completed` | 记录从既有 rollout 对 Pending Action 的机械对账数量 |
| `taskspace.action_settlement_store_busy` | 记录 SQLite writer busy 的有界指数退避轮次 |
| `taskspace.exec.completed` | 记录唯一 outer 反馈的结果数量和成功标志 |
| `taskspace.exec.rejected` / `taskspace.exec.fatal` | 区分合同拒绝与事实层故障 |

日志不记录 Tool 参数、Tool 输出或 Map content。

## 4. 验收证据

| 检查 | 结果 |
|---|---|
| TaskSpace Exec：schema、envelope、preflight、dispatch、Hosted、handler、response scope | 50 passed |
| 快慢 Tool、部分失败、取消、中断遗留 pending、零副作用拒绝 | PASS |
| Hosted 0/1/N、多节点、漏绑/错绑/重复、失败终态、真实 output index | PASS |
| 生产 Router 仅暴露 Exec + Hosted，普通 client 顶层绕过失败 | PASS |
| `codex-api` unit/integration/SSE | 134 passed |
| `codex-state` unit/bin/doc | 132 passed |
| `codex-rollout` | 46 passed |
| `codex-core --lib`（虚拟测试 key，不触发真实 Provider） | 1830 passed / 3 ignored |
| `cargo check --workspace --all-targets` | PASS |
| TaskSpace zero-base gate | PASS |
| cache regression gate，source=index | PASS，fingerprint `fba4afc9...e1677` |
| `git diff --check` | PASS |

全量 core 初次运行未向测试进程提供 key，Guardian/模型刷新测试按预期失败；改用虚拟测试 key 后全部通过，没有调用真实
Provider。另清理了一条已删除 `finish_map/exact_summary` 旧协议的残留日志测试，并同步早期
`code_mode_exec_function` 配置 schema fixture。

## 5. 第二轮审查后的并发闭合

第二轮独立 closure 发现旧实现最多只重放 8 次 Action outcome；持续竞争耗尽后，已执行 Tool 仍可能在 Map 中保持
`pending`。提交 `03acb2db6` 删除了该 CAS retry 路径：事实结算取得 SQLite latest-head 写事务后才读取 Map，在同一
事务中一次应用和提交 outcome。Tool 不重跑，当前生产调用也不驱动 Node 生命周期。

确定性测试证明：两个并发事实提交都被保留；Session 本地缓存落后时结算闭包只执行一次，同时保留其他 Session 的 Map
内容。TaskSpace Exec 56、State 131、Router 8、API 134、两条 Standard 原生反馈测试、workspace check、zero-base 和
cache gate 全通过。但第三轮用户授权 closure 进一步确认：State 的 5 秒 busy timeout 到期后 `BEGIN IMMEDIATE` 仍可
失败；Tool 完成后的 outer cancellation 也可丢弃正在等待的结算 future，两者都没有 durable 补偿入口。此外，公开
latest-head API 可提交任意 canonical Map，outcome-only 边界只靠调用方约定。上述绿色测试未覆盖这些路径。

## 6. 历史重开结论

以下是 2026-08-09 第三轮 closure 后、`702f885a0` 修复前的历史结论：EX-06～EX-08 的既有局部验收不受本轮推翻；MS-03 当时保持 blocked，B3 不得进入 B4。下一控制点是设计 durable、可恢复且
outcome-only 的 Action 事实结算方案；不得用提高 timeout 或另一组有限 retry 代替根因闭合。真实 Provider shape 与产品
对比仍属于 Phase B5，必须另行申请预算。

## 7. MS-03 收敛修复合同

重新核对 Standard、TaskSpace Exec、rollout 与 Map Store 的实际数据链后，否决新增独立持久化消息队列：

1. Standard client Tool 的 `FunctionCallOutput`、Provider-hosted `ResponseItem` 和 TaskSpace Exec 的唯一 outer
   `FunctionCallOutput` 已由 `ContextManager + rollout` 保存；大输出继续复用 Standard `output-ref`，不得把完整 Tool
   结果复制进 Map 或另一套队列。
2. Map 在 Tool 执行前已经持久化 `Pending Action`。Tool 完成后只缺
   `map_id/action_id/node_ids/tool_name/outcome/mutation_id` 这一窄化终态事实，不缺第二份执行历史。
3. 独立持久化队列无法与文件修改、进程、MCP 等外部 Tool 副作用组成同一事务，仍存在“Tool 已执行、队列尚未写入”
   的崩溃窗口；因此它增加事实副本和 ack/recovery 状态，却不提供声称的原子保证。

MS-03 改为以下唯一生产链：

```text
preflight -> persist Pending -> native Tool dispatch
          -> native result enters outer feedback / Standard rollout
          -> Session-owned settlement executor
          -> outcome-only latest-head Store transaction
          -> pre-provider settlement barrier
```

硬边界：

- 内部 Tool future 产出结果后，必须在同一次 poll 内同步投递结算命令；outer turn 取消不得撤回已经投递的命令。
- 结算执行器是 Session 的机械基础设施，不接收 Tool 正文，不解释语义，不修改 Node goal/content/state，不重试 Tool。
- Store API 只允许核对指定 `node_ids/action_id/tool_name` 并执行 `Pending -> succeeded|failed|cancelled`；相同终态幂等，
  不同终态、错节点、错 Tool 或不存在的 Action 是永久冲突。
- SQLite writer busy 属于可恢复存储竞争，执行器持续退避而不是在固定次数或固定 5 秒后丢弃事实。
- 下一次 TaskSpace Provider 请求构造 projection 前必须等待此前已投递事实结算；永久冲突明确阻断请求。
- 恢复对账只处理既有 rollout 中带稳定 TaskSpace Exec 结果标识、且当前 Map 仍为 Pending 的 Action；它不重建 Map、
  不扫描推断普通 Tool、不覆盖 terminal outcome，也不生成新的 Agent 决策。
- 如果 rollout 没有终态结果，Action 忠实保持 Pending/未知；Runtime 不推测、不默认失败、不自动重跑。

验收必须覆盖超过既有 5 秒 busy timeout 的 writer、Tool 完成后的 outer cancellation、跨 Session latest-head 写入、
恢复对账和 outcome-only 负例。

## 8. MS-03 修复与验收结果

提交 `e5925b45d` 先删除通用整图 mutation Store API，增加只允许核对
`map_id/action_id/node_ids/tool_name/outcome` 的 outcome-only 事务。提交 `702f885a0` 将结算生命周期从 outer Tool future
移到 Session 自有 FIFO 执行器：内部 Tool 返回后在同一次 poll 内同步投递窄化事实，完整结果仍只进入唯一 outer feedback
和 Standard rollout。下一次 TaskSpace Provider 请求在 projection 构造前执行 Pending-only rollout 对账并等待 FIFO
屏障；Standard 模式在此前直接返回，不进入该链路。

执行器对 SQLite writer busy 持续有界指数退避，不再沿用固定 5 秒失败边界；相同终态重放幂等，异终态、错节点、错 Tool
或不存在 Action 会成为持续可见的硬错误并阻断后续请求。恢复只使用带稳定
`kind=taskspace_exec_result`、`outer_call_id`、`map_id` 和 `action_id` 的既有反馈；大结果复用 Standard content-addressed
`output-ref` 并校验 SHA，不重建 Map、不重跑 Tool、不复制完整结果。

| 闭合证据 | 结果 |
|---|---|
| `codex-state --lib`，含同终态幂等、异终态/错归属拒绝、SQLite writer busy 超过 5 秒 | 133 passed |
| `codex-core taskspace_exec --lib` | 56 passed |
| Session 结算取消、永久错误屏障、Pending-only 恢复与错归属拒绝 | 4 passed |
| Standard output-reference 与完整恢复读取/SHA 校验 | 11 passed |
| `cargo check --workspace --all-targets` | PASS |
| TaskSpace zero-base gate | PASS |
| cache regression gate，source=index | PASS，fingerprint `61da3cc3...6712b9c` |
| 真实 Whale Agent / Provider 请求 | 0 |

以下是 `702f885a0` 实施后的初始结论，已被第 9 节 fresh adversarial review 推翻：当时认为 MS-03 的已知三项缺口已经
闭合、Phase B3 可进入 Phase B4。该结论不声称任意外部 Tool 副作用
与 Map 事务具备进程级原子性；在 Tool 结果尚未进入 rollout 的极端进程崩溃窗口，Map 会忠实保留 Pending，而不是由
Runtime 猜测终态或自动重试 Tool。

## 9. Fresh adversarial review 重新打开

2026-08-09 的 fresh、`fork_context=false`、只读 reviewer 发现三个 blocker，主流程按生产代码逐项复核后全部接受：

1. 原生 Tool 在 `AbortOnDropHandle` 子任务中完成，但 settlement 只在父 TaskSpace future poll 到 JoinHandle 结果后 enqueue。
   子任务完成到父 future 下一次 poll 之间被 abort 时，Tool 已执行，结果、queue fact 和 rollout feedback 都不存在。
2. graceful Session shutdown 在 abort tasks 后没有等待 TaskSpace producer 或 FIFO barrier，就继续关闭 persistence 并发送
   `ShutdownComplete`；worker 只持有 `Weak<Session>`，Session drop 后可放弃剩余命令。
3. 现有 handler 使用 test-only in-memory Map，持久化测试直接注入 fact/feedback；没有一条测试穿过 persisted handler
   dispatch、SQLite settlement、rollout/output-ref、recovery/barrier 和 provider transport blocking 的组合生产链。

审查还确认三项相邻非 blocker：SQLx 返回 extended SQLite result code 而 classifier 只匹配 `5/6`；recovery 未交叉核对
outer/action identity 或拒绝冲突历史；并发 Store read 可把旧 revision 安装回本地 cache。完整证据、逐项 triage 和治理停点见
[`2026-08-09-r8-ms03-settlement-review.md`](../../../../vs_review/2026-08-09-r8-ms03-settlement-review.md)。

当前结论：MS-03 与 Phase B3 重新 blocked。下一步必须共同设计 TaskSpace-owned execution producer 与 graceful shutdown
drain 顺序，先以确定性交错测试复现，再修复并补组合生产链验收；不得用后置 retry、持久化队列或自动 Tool 重试替代。

## 10. 审查后工程硬化

提交 `4be93ba31` 已关闭不依赖产品决策的相邻工程缺口：SQLite extended result code 按 primary code 识别
`BUSY/LOCKED`；恢复在任何 enqueue 前整批核对 outer/call-index/action identity 并拒绝冲突终态；queued 日志只在
channel send 成功后记录，worker failure 补齐完整机械身份。State 定向 4、Session settlement 6、TaskSpace Exec 56、
workspace check、zero-base 和 cache gate 均通过，真实 Whale Agent/Provider 请求为 0。

该提交没有调整 Tool producer、Session shutdown 或本地 cache 安装语义，因此 B01、B02、B03 与 cache revision 单调性
仍保持 open；MS-03 和 Phase B3 状态不变。
