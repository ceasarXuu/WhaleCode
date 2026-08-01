# R5 通用性能观察工具

Status: Implemented

## 1. 目标

把 benchmark 已有 artifact 统一汇总为稳定、可重复生成的性能报告，覆盖：

1. 结果：Agent 完成状态、业务结果、public/hidden validation、变更文件。
2. 动作：provider 请求、普通工具、失败工具、shell、patch、TaskSpace control。
3. 成本：wall time、input/cached/uncached/output tokens。
4. 缓存：全量命中率、request-2+ 命中率、严格前缀保持和 trace coverage。
5. Map：节点、边、结果、open leaf、根任务状态、控制动作、结果生命周期和语义保存指标。

工具只读取 artifact，不执行模型、validator 或 map 动作，不对 Agent 行为做语义重写。

## 2. 命令

```powershell
pwsh -NoProfile -File scripts/taskspace-benchmark/write-performance-observation.ps1 `
  -RunRoot target/<benchmark-run>
```

可选参数：

- `-OutputDirectory <dir>`：指定输出目录，默认写入 run root。
- `-ReportBaseName <name>`：指定文件名前缀，默认 `performance-observation`。

输出：

- `performance-observation.json`：机器可读事实、聚合值和比率。
- `performance-observation.md`：结果、动作、成本、缓存和 map 表格。
- `performance-observation-events.jsonl`：生成事件、缺失 artifact 和机械异常日志。

## 3. 证据口径

| 指标 | 首选来源 | 回退来源 |
|---|---|---|
| logical mode | `logical-mode-map.json` | `metrics.json.logical_mode` |
| provider requests | `provider-cache-trace-summary.json` | final `provider-wire-trace.jsonl` |
| tools/cost/result | `metrics.json` | 无；缺失即 `N/A` |
| shell/patch/control | `rollout.jsonl` | `whale-exec.jsonl` |
| cache/prefix | `provider-cache-trace-summary.json` | 无；缺失即 `N/A` |
| map structure | `graph-health.json`、observability | `metrics.json` 计数 |
| map preservation | `map-management-summary.json` | 无；缺失即 `N/A` |
| control/runtime events | `taskspace-control-usage.json` | 无；缺失即 `N/A` |

报告按 logical mode 聚合，不假设 `left=standard` 或 `right=taskspace`。`case_id` 使用相对路径，允许在 suite root 下区分重名的 `pair-001`。

## 4. 比较资格

只有同时满足以下条件的 side 才进入 Standard/TaskSpace 聚合和比率：

1. side 没有 `side_selection_skipped` taint；
2. `agent_completion_status=complete`；
3. provider request count 可测量；
4. input tokens 大于 0。

skipped 和 incomplete side 仍保留在逐行报告中，但不以零值污染聚合。货币成本在没有冻结 provider 单价 artifact 时必须显示 `N/A`。

## 5. Map 机械观察

当前报告记录但不替 Runtime 做语义决策：

- 多节点无依赖边；
- 节点全部关闭后根任务仍为 active；
- unreviewed result 存在；
- request/node、tools/node 密度；
- retention、salience、semantic replacement、protected miss 和 compaction。

这些字段用于定位 map 粒度、生命周期或语义保存问题。工具不得自动拆节点、补边、接受结果或推断 Agent 意图。

## 6. 验证

```powershell
pwsh -NoProfile -File scripts/taskspace-benchmark/test-performance-observation.ps1
```

fixture 覆盖左右模式轮换、right-only 占位 side、缓存加权、map 节点/边/结果、聚合比率和事件日志。真实验证使用 R5 G1 的 `count-call-stack` 三次配对样本及 `subscription-billing-repair` right-only 样本。

## 7. 后续 Docker-only 输入

后续将按 `13-r5-unified-docker-benchmark-and-logging-plan.md` 把正式 benchmark 收敛到 Docker-only。性能观察工具届时必须增加但不得混算以下字段：

- image digest、container id/role 和资源配置；
- build/pull/create/start/preflight/log collection/cleanup 时间；
- Agent、validator、oracle 各自的容器日志和退出状态；
- container runtime/log coverage；
- stats 观测开销和 host fallback scan。

该事项当前为 planned/deferred，本阶段不修改报告器代码或执行容器样本。
