# R8-I07 Final-Wire Observer 增量修复结果

- Date: 2026-08-18
- Scope: 四臂运行新发现的 projection 与 TaskSpace Exec reject 分类漏报
- Runtime behavior change: 无
- API usage: 0；仅回放既有 `WAR-20260818-055427-R8-FOUR-ARM-R3` 原始证据
- Status: completed

## 1. 根因

1. `context-projection-summary.json` 在 Provider final-wire section facts 生成前完成计算，只能读取旧
   `whale-exec`/runtime trace。最终请求已经携带 projection 时，旧汇总仍可能报告
   `projection_unavailable`。
2. `taskspace_exec` 专用观察器只记录拒绝总数；性能报告中的子类型字段仍来自旧
   `taskspace_control` 结果 schema，因此当前 Exec 的合同、状态和语法拒绝全部显示为零。

## 2. 修复

- projection 汇总优先消费同一轮 Provider final-wire 已解析出的 `active_projection_identity`，旧 trace
  只在 final-wire facts 不存在时作为兼容输入。
- 汇总区分 `measured`、`measured_absent`、`projection_unavailable` 和 `source_missing`；没有注入
  projection 的请求不再被误报为证据不可用。
- `taskspace_exec` 观察器按稳定输出合同对拒绝进行互斥主分类：`syntax`、`contract`、`state`、
  `preflight_other`、`unknown`，并保留包含 state 的 preflight 聚合。
- 性能观察结果和 Markdown 报告直接消费 Exec 专用字段；旧 `taskspace_control` 字段继续只表达旧协议。

## 3. 既有真实证据回放

| 模式 | Provider requests | Bootstrap requests | Active requests | Measured absent | 结果 |
|---|---:|---:|---:|---:|---|
| map-always | 30 | 4 | 26 | 0 | measured |
| map-append | 32 | 3 | 29 | 0 | measured |
| map-request | 31 | 0 | 0 | 31 | measured_absent |

三次 `taskspace_exec rejected` 被逐 outer `call_id` 计数：合同错误 1、waiting/state 错误 1、JSON
语法错误 1，未知分类为 0。该结果与原始 rollout 逐项一致。

## 4. 验证

- `test-taskspace-exec-observation.ps1`: PASS
- `test-cost-instrumentation.ps1`: PASS
- `test-performance-observation.ps1`: PASS
- `test-harness.ps1`: PASS
- `test-whale-agent-run-ledger.ps1`: PASS（92 entries）
- 四臂既有真实 trace 离线回放：PASS

本次只修 observer 事实消费与分类，不修改 Provider payload、Agent context、Tool、Map、Runtime
状态机或执行结果，因此不需要新增真实运行验证。
