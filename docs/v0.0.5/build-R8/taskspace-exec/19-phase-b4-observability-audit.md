# Phase B4 观测身份链审计

- Date: 2026-08-09
- Unit: OB-01A
- Status: verified
- Scope: 只读审计；未运行 Whale Agent 或 Provider 请求

## 1. 审计结论

B3 已经覆盖请求开始、响应冻结、预检、候选 Map 持久化、Action 结算和 Exec 完成等关键阶段。OB-01 不需要建立新的
日志框架、事件数据库或执行事实副本。当前缺口是同一响应的机械身份没有完整传递到这些既有事件，导致日志不能稳定连接
I07 的 Provider request facts、outer Exec、Hosted fact 和最终 Map revision。

成本与缓存继续以 I07 canonical request facts 为唯一统计事实；TaskSpace tracing 只提供动作路径和失败定位，不重新累计
Provider 请求或 Token。

## 2. 当前身份矩阵

| 阶段 | 当前事件或事实 | 已有身份 | 缺口 | 权威来源 |
|---|---|---|---|---|
| Provider dispatch/terminal | provider wire + token event | request/logical/attempt | 无 | I07 canonical request facts |
| TaskSpace request | `taskspace.exec.request_started` | map/revision | Provider request、response、outer call 尚不可得 | request Map snapshot |
| Hosted item 到达 | `taskspace.exec.hosted_fact_observed` | output index、Provider ID、Tool、outcome | request/response/outer call；尚未完成 Agent 归属 | Provider output item |
| response 完成 | `taskspace.exec.response_finalized` | item 数量、接受结果 | Provider request、response ID、outer call、map/revision | `ResponseEvent::Completed` + response scope |
| Exec preflight | `taskspace.exec.preflight_accepted/rejected` | outer call、map/revision 或文本原因 | rejection 缺稳定阶段码；部分事件缺 Provider response identity | outer Function Call + canonical preflight |
| candidate commit | `taskspace.exec.candidate_persisted` | outer call、map、candidate revision、Action 数量 | Provider response identity；Hosted 逐 Action 归属日志 | canonical Store commit |
| client settlement | queued/committed/failed | outer call、Action、node、Tool、outcome、Map revision | 可经 outer call 连接 Provider response，无需复制 request ID | settlement fact + canonical Store |
| outer completion | `taskspace.exec.completed` | outer call、map、结果数量、success | Provider response identity、最终 dispatch revision | outer result builder |

## 3. 最小修复边界

1. 在 `ResponseEvent::Completed` 已经取得 Provider request identity 和 response ID 后，将其写入当前 response-local scope。
2. scope 冻结事件携带 request/logical/attempt、response ID、outer call、map 和 request revision。
3. preflight、candidate、Hosted attribution 和 completed 事件复用 claim 中同一身份，不生成新 ID。
4. Hosted 的 canonical 观测点放在 candidate 持久化成功后，记录 Agent 声明的 node 集合；早期无归属
   `hosted_fact_observed` 不作为 active canonical 事件继续保留。
5. rejection/fatal 增加稳定的机械阶段码，但原始错误文本保持忠实，不增加修复建议或语义解释。
6. settlement 继续通过 outer call 关联，不把 Provider request identity 复制进 FIFO、SQLite 或 Map。

## 4. 明确不做

- 不新增 TaskSpace trace 数据库、JSONL 事实源或持久化队列；
- 不修改 Standard Tool、Provider wire 或 I07 request accounting；
- 不把 request/response identity 写入 Agent-visible Tool 参数、Map 或 outer result；
- 不记录 Tool 参数、Tool 输出、用户文本或其他敏感 body；
- 不通过日志推导节点状态、业务正确性或 Agent 意图。

## 5. OB-01B 验收

- 同一 fixture 可从 Provider request identity 连接到 response、outer call、Hosted attribution、client settlement 和 Map revision；
- rejection 至少包含稳定阶段码，且失败前不存在不应发生的 Tool/Map 副作用；
- 日志字段只来自 Runtime 已持有事实，不出现内容摘要、语义分类或新事实源；
- TaskSpace Exec、response scope、settlement 定向测试通过，Standard 路径无修改。
