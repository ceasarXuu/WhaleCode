# I07 逻辑模式选择修复结果

- 日期：2026-08-20
- 状态：离线实现完成，待真实运行闭环

## 问题

Benchmark 只有 `RunSide=left/right` 物理侧过滤。由于奇偶 repeat 会交换 Standard 与 TaskSpace 所在侧，
`RunSide=right` 不能表达“只运行 TaskSpace”，曾额外启动 6 次 Standard，突破已登记的 sample 与 request 预算。

## 修复

- 新增独立的 `RunLogicalMode=both|standard|taskspace`；物理侧与逻辑模式必须同时匹配才执行。
- cache regression contract 不再把逻辑模式错误翻译为 left/right，而是直接传递逻辑模式。
- `PlanOnly` 在启动 Agent 前输出完整执行展开和 `selected_execution_count`，作为预算预检事实。
- pair manifest、事件和状态同时记录物理侧与逻辑模式，避免后处理再次混淆。

## 离线证据

- `test-logical-run-selection.ps1` 通过，覆盖奇偶 repeat 的左右交换及组合过滤。
- cache run contract 10/10 单元测试通过。
- `release-dispatch-repair × repeat=3 × RunLogicalMode=taskspace × PlanOnly` 展开 6 个物理候选，
  仅选择 3 个 TaskSpace，Standard 选择数为 0。

## 边界

该修复只控制 runner 启动哪些逻辑模式，不改变 Agent、Map、Tool、projection、Provider 请求或评分语义。
真实闭环必须确认执行数仍为 3 且没有 Standard 被启动，之后才能关闭 I07 的该子问题。
