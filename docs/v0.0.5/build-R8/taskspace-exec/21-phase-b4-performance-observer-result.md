# Phase B4 性能观察消费结果

- Date: 2026-08-09
- Unit: OB-02B
- Status: verified offline
- Scope: benchmark artifact consumer；未运行 Whale Agent 或 Provider 请求

## 1. 原问题

通用性能报告的请求计数已经读取 I07 `request-facts.json`，但动作路径仍把 TaskSpace 解释为 R7 的
`taskspace_control + sibling manifest`。R8 只有一个 outer `taskspace_exec`，真实 client/map/hosted 动作位于其内部；旧
消费者会把 R8 误报为孤立 Tool、缺失 control 或零普通动作。

## 2. 当前事实源

| 事实 | 唯一来源 | 用途 |
|---|---|---|
| request/logical/attempt、完成状态、usage | I07 canonical request facts | 请求、Token、缓存和可比性 |
| outer Exec、内部 calls、Hosted bindings | canonical rollout | Agent 实际声明的动作与节点归属 |
| client/Hosted outcome | 唯一 `taskspace_exec_result` | 实际执行结果 |
| request/response/outer/Map 关联 | OB-01 tracing | 跨层身份对账，不重复统计成本 |
| Map/node/parents | canonical Map observability | Map 形状与状态 |

## 3. 实施

- 新增独立 R8 解析模块；按协议检测选择 R8 或历史解析，不把两套字段混算。
- R8 普通 Tool 数使用 Exec 内 client actions + Hosted bindings，不再把唯一 outer Exec 当作全部工作量。
- R8 有效性不读取旧 control、manifest、sibling 或 cadence 字段；身份缺失、错绑或 trace request 不在 canonical facts
  中时 fail closed 为 `incomparable`。
- benchmark TaskSpace 容器启用 `error,codex_core::taskspace_exec=info`，复用现有 stderr artifact 捕获 OB-01 事件；未新增
  tracing 数据库或第二成本事实源。
- 通用报告新增 TaskSpace Exec 表；旧 sequence 表仅用于历史 artifact。

## 4. 验证

- `test-taskspace-exec-observation.ps1`：逐 outer/action/node/request ID fixture PASS，未知 request fail closed；
- `test-performance-observation.ps1`：历史与通用报告回归 PASS；
- `check-request-fact-consumers.py`：唯一事实消费者登记 PASS；
- request facts Python tests：17 passed；
- Docker-only call graph gate：PASS；
- 所有手写生产文件不超过 500 行；未执行真实 Whale Agent run。
