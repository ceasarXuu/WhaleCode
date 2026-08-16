# Provider-hosted Runtime 机械归纳结果

- Date: 2026-08-15
- Scope: Provider-hosted Tool 的 TaskSpace 记录边界
- Status: 生产代码已实现，离线验证通过；最小真实缓存回归因独立的 Exec 参数类型错误未通过业务验收

## 1. 产品决策

当前阶段不再要求 Agent 为 Provider-hosted Tool 双写或延迟补写节点归属。Provider Tool 可以暂时脱离 Agent 管理的工作节点；
Runtime 只保留最小调用账目：

1. 原生 Provider Tool 实际发生后才创建专用节点，不提前创建空节点；
2. `node_id` 与 `goal` 逐字使用原生 `ToolSpec::name()`，例如 `web_search`、`image_generation`；
3. 节点是 Root 的直接子节点，状态固定为 Completed，并连接 Finish；
4. 同一响应内同一种原生 Tool 聚合为一个 Action，后续响应追加到同名节点；
5. Action 只保存稳定 identity、原生 Tool 名和机械 outcome，不复制 input/output；
6. 无 Map 或同名 Agent 节点冲突时允许 escape，只记录诊断，不建队列、不阻止 Agent。

## 2. 删除范围

- Agent-visible 双写、`already_executed` 和 Provider 归属结果；
- `assign_pending_actions`、纯归属序列和相关 parser/preflight/handler；
- pending Provider Action SQLite migration、模型、Store API、CAS 消费和重启恢复；
- 下一请求 context 暴露、base instructions 提示和错误反馈；
- benchmark 中对旧归属字段的活动解析。

历史实验报告保留原始事实，但 PD5/PD8 已由 PD10 取代，不能继续指导实现。

## 3. 实现边界

Provider ResponseItem 仍由 response-local scope 从原生 ToolSpec 分类。本报告完成时，Provider work 还参与响应级
`response_work+` 判断；该历史规则已由
[`65-client-work-structural-restoration.md`](65-client-work-structural-restoration.md) 废止。当前工作型序列必须包含非空 client
`tools[]`，Provider Root 归纳保持不变且不参与 Exec 合法性判断。

Provider 聚合在本响应的 client/Map 工作 drain 后执行，因此同响应初始化的 Map 可以立即接收记录。聚合节点不表示 Agent
认为该业务节点完成，也不影响其他 Work node 的状态。

## 4. 离线证据

| 验证 | 结果 |
|---|---|
| TaskSpace Exec focused suite | 69 passed |
| `codex-state` TaskSpace suite | 16 passed |
| 持久化 Map 首次创建 `web_search` 聚合节点 | passed |
| 后续响应追加到同名节点且不重复 Finish parent | passed |
| 已闭合 Map 追加 `image_generation` 后仍保持闭合且 DAG 合法 | passed |
| 同名 Agent 节点冲突不被 Runtime 改写 | passed |
| 旧 active Rust/SQL/Prompt Provider 归属符号 | 仅保留负向防回归断言 |

缓存敏感面包含 TaskSpace base instructions 与 Tool declaration。最终提交前必须通过项目缓存门禁；若门禁要求真实回归，
需另行申请预算。

## 5. 已知取舍

- Provider work 暂时不能表达 Agent 选择的业务节点归属，也不会参与这些节点的完成判断；
- 无 Map 或名称冲突时 Provider 调用可能完全不进入 Map；诊断日志可观察这一事实；
- 专用节点是 Runtime 机械账目，不是新的语义编排能力；未来恢复 Provider 管理必须重新做产品设计，不能复活旧双写或 pending。

## 6. 最小真实回归

获批预算 `CBP-1CA2CC90FD60E80B` 在提交 `682164844` 上执行
`single-file-fast-fix × (standard, map-request) × repeat=1`，零重试。结果不能晋升缓存基线：

| Arm | 业务 | Requests | Input | Cached | Uncached | Output | request 2+ cache |
|---|---|---:|---:|---:|---:|---:|---:|
| Standard | passed | 6 | 74,711 | 67,072 | 7,639 | 1,326 | 97.30% |
| map-request | failed | 10 | 151,194 | 125,056 | 26,138 | 9,340 | 87.18% |

两臂共 16 个 Provider 请求，225,905 input、192,128 cached input、33,777 uncached input、10,666 output，
已知费用估算为 USD 0.0082532184。map-request 的 10 次 `taskspace_exec` 均在 Map 或 client Tool 副作用前被拒绝：
首请求把 schema 要求为 object 的 `initialize_map` 编码成了 JSON string，后续请求在错误上下文中重复或改用不存在的
wrapper。Map 始终未创建，Provider-hosted Tool 也未发生，因此该运行没有触达本专题新增的 Root 归纳路径。

当前可确认缓存结构没有回归；不能据此宣称 Provider 归纳在线通过，也不能把 Exec 参数错误归因为本次 Provider 归纳实现。
该错误继续归入 I03 的 Function Tool 参数稳定性，不在本专题内扩张修复范围。
