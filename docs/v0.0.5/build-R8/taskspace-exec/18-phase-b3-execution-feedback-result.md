# Phase B3 MS-03、EX-06～EX-08 执行与反馈结果

- 日期：2026-08-07
- 状态：离线验证完成
- 核心提交：`1347606e0`
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
6. Agent 只收到一个 outer Function Tool output。它包含最新 Map identity/revision、显式读取结果、每个 client Tool
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
| `taskspace.exec.action_settled` | 逐 Action 记录机械终态 |
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

## 5. 阶段结论

MS-03、EX-06、EX-07、EX-08 均达到离线验收，Phase B3 完成。下一阶段是 Phase B4：建设可逐动作对账的完整观测、
更新缓存/性能工具并执行 Docker 离线门禁。真实 Provider shape 与产品对比仍属于 Phase B5，必须另行申请预算。
