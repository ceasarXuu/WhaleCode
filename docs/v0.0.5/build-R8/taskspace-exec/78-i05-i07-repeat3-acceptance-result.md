# I05 / I07 三轮真实验收结果

- Date: 2026-08-18
- Run ledger: `WAR-20260818-013746-R8-I05-I07-ACCEPT-R3`
- Run root: `target/whale-agent-runs/WAR-20260818-013746-R8-I05-I07-ACCEPT-R3/single-file-fast-fix/20260818-014006-935`
- Model: `deepseek-v4-flash`
- Matrix: `single-file-fast-fix × (standard + map-request) × repeat=3`
- Result: 3/3 Pair 双侧通过

## 1. 验收结论

| 验收面 | 结果 |
|---|---|
| 业务正确性 | Standard 3/3、TaskSpace 3/3 均通过公开测试和隐藏 oracle |
| Map | TaskSpace 3/3 均持久化一张 Map，Root、全部工作节点和 Finish 均为 `completed`，图警告为 0 |
| 请求事实 | 41 logical = 41 boundary = 41 completed = 41 usage；local-only、boundary-unattributed、duplicate、retry、finding 均为 0 |
| 失败停止 | 真实命令显式启用 `-StopOnAnySideFailure`，本轮没有失败，未触发停止；失败分支继续由确定性 runner 测试覆盖 |
| I05 正常路径 | 3 次 TaskSpace 均未发生顶层 client Tool 逃逸、Fatal 中断或反馈重复，证明修复后正常路径无回归 |
| I05 恢复分支 | 本轮未自然触发顶层 client Tool 逃逸，不能声称获得新的在线恢复命中证据；同 `call_id`、零执行、可继续反馈由定向测试证明 |
| 缓存 | 第 2 请求起 Standard 命中率 97.80%，TaskSpace 92.35%；无零命中、Tool shape 或 `tool_choice` 切换 |

I07 的生产请求身份、usage、Map 完成判定和最终报告已能从同一组 artifact 逐项复算，关闭 `R8-I07`。I05 的工程修复和
正常生产路径通过，但自然样本没有触发逃逸恢复分支，因此 `R8-I05` 保持 `verifying`，不把“未复现”写成“在线命中”。

## 2. 成本

| 模式 | Runs | Requests | Input | Cached | Uncached | Output | Agent wall |
|---|---:|---:|---:|---:|---:|---:|---:|
| Standard | 3 | 20 | 244,094 | 239,360 | 4,734 | 3,734 | 40.615s |
| map-request | 3 | 21 | 321,120 | 299,136 | 21,984 | 6,289 | 53.956s |
| Total | 6 | 41 | 565,214 | 538,496 | 26,718 | 10,023 | 94.571s |

按运行时北京时间低谷单价估算总费用为 **CNY 0.1121053**；同用量按高峰单价为 CNY 0.2242106，均低于批准的
CNY 0.60 上限。map-request 相比 Standard：请求 `1.05x`、总 input `1.32x`、平均每请求 input `1.25x`、Agent wall
`1.33x`。这是单一简单样本的验收数据，不外推为复杂样本性能结论。

## 3. I07 逐身份闭环

六个 side 的 `request-facts.json` 分别记录 `7/7/6/7/7/7` 个请求，总和 41。每个 side 内：

- `logical_request_count == boundary_request_count == completed_response_count == usage_record_count`；
- `local_only_attempt_count == boundary_unattributed_count == duplicate_event_count == 0`；
- `failed_or_cancelled_attempt_count == retried_logical_request_count == 0`；
- canonical findings 与 observer diagnostics findings 均为 0。

这证明生产报告没有再次从 raw terminal/wire 重建第二套请求或 usage 事实，也没有把状态快照计成请求。

## 4. 边界

- 本轮不改变 Tool、Map、Provider payload 或 Agent 协议；只验收已经提交的修复。
- Standard 第一轮的一次失败 Tool 是修复前主动运行测试得到 `2 failed, 1 passed`，随后修复并通过，不是 Runtime 或 Harness 故障。
- I05 后续不应通过构造自然语言诱导来追求逃逸命中；若真实工作自然出现逃逸，再用同一 `call_id` 证据链验收恢复行为。
- I07 的失败停止行为没有在本轮故意制造付费失败；确定性测试与真实命令参数共同覆盖该合同。
