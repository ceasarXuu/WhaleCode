# R8-I07 默认 Metrics 拒绝漏报修复结果

- Date: 2026-08-20
- Commit: `476d60802`
- Runtime behavior change: 无
- API usage: 0；仅回放已有真实运行证据
- Status: completed

## 1. 产品缺口

TaskSpace Exec 的拒绝已经忠实写入 rollout，专用性能观察器也能识别，但默认 benchmark 的
`metrics.json` 仍只复制旧 `taskspace_control` 失败字段。因此真实发生的 waiting 与状态转换拒绝会在默认结果中显示为
零，污染 I03/I04 的判断。

## 2. 根因与修复

- 默认 metrics 现在直接复用唯一的 `Get-TaskspaceExecObservation`，不复制第二套日志解析规则。
- 新增独立的 `taskspace_exec_rejected_*` 字段；旧 `control_*` 字段继续只表达旧协议，不混写语义。
- 已有明确证据的 `TransitionInvalid` 归入 `state`，同时计入 preflight 聚合；未知拒绝仍单独暴露。
- Standard 侧明确标记 `not_applicable`，不会把“不适用”伪装成 TaskSpace 零失败测量。

## 3. 真实 Trace 离线回放

证据根：`WAR-20260820-000226-R8-MULTILINE-SELF-HEAL-R3/H011`

| Run | Rollout 真实拒绝 | Metrics 总数 | State | Preflight | Unknown |
|---|---:|---:|---:|---:|---:|
| 1 | 0 | 0 | 0 | 0 | 0 |
| 2 | 1 × waiting | 1 | 1 | 1 | 0 |
| 3 | 1 × waiting + 1 × `TransitionInvalid` | 2 | 2 | 2 | 0 |

三轮 observation availability 均为 `measured`，无 findings。原始的 `0/0/0` 漏报已经消失。

## 4. 验证

- `test-taskspace-exec-observation.ps1`: PASS
- `test-performance-observation.ps1`: PASS
- `test-cost-instrumentation.ps1`: PASS
- PowerShell parser: PASS
- 最新三轮真实 trace 离线回放：PASS

完整 `test-metrics-extractor-harness.ps1` 在进入本次新增路径前，被已有 fixture 缺少
`request_shape_classifier` 阻断；本次改用专用测试直接覆盖默认 metrics 接线，不把该无关失败计为通过。

