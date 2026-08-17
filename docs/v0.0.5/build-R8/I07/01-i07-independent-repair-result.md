# R8-I07 独立修复结果

- Date: 2026-08-05
- Scope: `I07-W0`～`I07-W8`、`I07-W10`
- Status: 独立修复、对抗性闭环与生产验收完成；I07 closed
- API usage: 离线修复为 0；生产验收关联 `WAR-20260818-013746-R8-I05-I07-ACCEPT-R3`

## 1. 产品结果

性能观察现在会分别表达逻辑采样、本地 Provider attempt、实际越过监督边界的请求、完成响应和有归属的 usage。
无请求身份的状态快照不再生成请求或成本；本地失败未越过边界时不再被写成上游请求不一致；证据缺失、冲突或过期时
明确输出 unavailable/incomparable，而不是按事件条数补一个数字。

成本、性能、缓存、五层 trace、boundary verifier 和 freshness 使用同一份 `request-facts.json`。缓存 shape 仍按本地
attempt 观察，但 token/cache 分母只使用完成且有 measured usage 的记录；严格缓存发布门禁仍要求全部 attempt 完成，未被
通用事实合同放宽。

## 2. 已完成能力

| 单元 | 结果 | 主要提交 |
|---|---|---|
| W0～W1 | 固化 8/15、10/11 反例并建立 consumer inventory 门禁 | `6ad058e10`、`5e816d571` |
| W2～W4 | 建立 canonical facts，修复双计和 attempt/boundary 阶段误判 | `4ad74fd98`、`afdfb68ba`、`3498d93b1` |
| W5 | 性能与五层报告统一 logical/attempt/boundary/completed 计数 | `09de2bac5`、`fe559f904`、`44a9cf468` |
| W6 | 缓存和 section-cost 迁移到 completed measured usage | `ed1d82cf2` |
| W7 | 封存 source SHA、analyzer 组合哈希并接通 freshness | `4870f44c0` |
| W8 | 增加 payload-free observer diagnostics | `825f6fdd1`、`63b0336d3` |

当前规范化 artifact 为 `whalecode-request-facts-v1`，analyzer 为 `i07-review-fixes-v3`，诊断合同为
`whalecode-request-facts-diagnostics-v1`，run evidence manifest 升级为 v2。

首轮对抗性审查发现的 6 个阻断缺口已由 `9dc661aa0` 修复：failed terminal 不再继承 rollout usage；boundary
要求完整 start/stop 生命周期；相同 payload retry 只让逐 attempt 关联不可比较；cache/performance/freshness 不再用
shape 或 completion 冒充 boundary request；canonical consumer 也进入 inventory 门禁。完整审查轨迹见
[`vs_review/2026-08-05-i07-independent-repair-review.md`](../../../../vs_review/2026-08-05-i07-independent-repair-review.md)。

后续 closure review 继续发现并修复 aggregate 空值传播、布尔计数、mode-map 权威性、V4 摘要合同和报告作用域问题。
最终提交链为 `9e7536425`、`d5b768a22`、`b2c05ad5e`、`53d08313f`、`15896eead`、`4675f1a66`、
`8acd79b76`。最终空白复审只重放最后四个反例，结论为 `NO BLOCKING FINDINGS`。

## 3. 验证结果

| 验证 | 结果 |
|---|---|
| I07 历史反例 | 8 completed/usage + 7 snapshots；11 attempts + 10 boundary + 1 local-only failure |
| request facts + inventory + boundary proxy | 23 tests passed；consumer gate passed |
| cache regression | 220 tests passed |
| cost / harness / performance | 全部 self-test passed |
| provenance / freshness | source 变更、analyzer 变更和旧 facts 均 fail closed |
| 24-run 离线报表矩阵 | `R7 request observability report passed.` |
| 缓存敏感面静态门禁 | PASS；观测政策变化仍保持发布阻断，未以静态结果替代真实缓存回归 |
| 最终 closure review | 4 类最新反例全部 fail closed；无 blocking finding |

所有测试只使用本地 fixture，没有启动 Whale Agent，没有产生 DeepSeek 请求，也没有修改 Provider payload、Agent prompt、
Tool schema、Map 状态机或工具执行顺序。

### 3.1 可复现命令

```bash
python3 -m unittest scripts/taskspace-benchmark/test_request_facts.py scripts/taskspace-benchmark/test_request_fact_consumers.py scripts/taskspace-benchmark/docker/test_provider_boundary_proxy.py
python3 -m unittest discover -s scripts/cache-regression -p 'test_*.py'
python3 scripts/taskspace-benchmark/check-request-fact-consumers.py
pwsh -NoProfile -File scripts/taskspace-benchmark/test-i07-characterization.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-cost-instrumentation.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-harness.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-provider-boundary.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-performance-observation.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-r7-five-layer-trace-analysis.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-r7-request-facts-provenance.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-r7-five-layer-evidence-freshness.ps1
pwsh -NoProfile -File scripts/taskspace-benchmark/test-r7-request-observability-report.ps1
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
```

### 3.2 Fixture 身份

| Fixture | SHA-256 |
|---|---|
| `attempt-boundary-events.jsonl` | `41231806f39c862ef8ea4dc4d26e823bfeb8bb859e50125172f1e740c9eed665` |
| `attempt-boundary-wire.jsonl` | `cde1c1591c5bfc38112547b66a3bb1ca09a990c5ce750f70ebeb0f4e3404e16e` |
| `usage-double-count-rollout.jsonl` | `eee6797144db27dd84b74b5ca654d643a99eb3a2787f4885c57547eee454f991` |

## 4. 已知副作用

- 旧 evidence manifest v1 和旧 analyzer 产物会被标记 legacy/incomparable，不做推测兼容；
- 报告增加 logical、attempt、boundary、completed 和 availability 字段，历史错误总量会下降；
- request facts 生成继续使用 Harness 已有的本地 Python 进程；
- 缓存敏感面门禁仍要求后续经用户授权的真实回归才能晋升发布基线。

## 5. 独立修复结束时的未完成边界

- `I07-W9`：TaskSpace Exec outer call、internal item/node 与 Provider facts 的身份关联；
- `I07-W11`：新协议稳定后另行申请最小真实预算，执行生产 trace 逐 ID 对账；
- 上述工作不得重建第二套 Provider 计数规则，也不得让 observer 反向控制 Agent 或 Tool。

截至 2026-08-05，本结果只关闭“当前协议下可独立修复的观测缺陷”，尚未关闭全局问题 `R8-I07`；后续第 6～8 节记录了
剩余接线、生产验收与最终关闭。

## 6. 2026-08-17 当前生产反例修复

提交 `827b660a4` 修复后续 repeat=3 暴露的三个硬缺口：

- Map 完成告警从持久化节点状态推导，不再用 `open_leaf_nodes=0` 猜测所有节点终止；
- request summary 的 usage 行数与 canonical `request-facts` 共用唯一事实；
- runner 通过显式 `-StopOnAnySideFailure` 执行失败停止策略，保留当前 Pair 证据、记录跳过轮次，并按实际完成轮数结算
  `stopped/exit_code=1`，停止产物不能被普通 resume 绕过。

聚焦测试、benchmark harness、E3 harness guardrails 和历史失败 trace 离线重算均通过；没有运行 Whale Agent。I07 继续
保持 `verifying`，只剩获批真实运行对本次接线的最终验收。

## 7. 2026-08-18 Consumer 漂移闭环

提交 `7a4346156` 修复首轮对抗性审查发现的第二套请求事实模型：

- `r7-request-observability.ps1` 不再从 raw terminal 重建请求身份、attempt、完成状态或 usage；
- 生产矩阵把已经封存的 `request-facts.json` 直接传给 observer，避免重复生成或双重解释；
- raw wire 读取只保留其独有的 message shape、LCP、transport 和 final-control identity；
- 相同的重复 wire attempt/terminal 现在由 canonical facts 明确标为不可比较，重复 rollout 状态快照仍保持幂等；
- consumer inventory 增加代码合同，禁止 observer 重新读取 terminal 身份字段。

Python request-fact 测试 22/22、五层 trace、Provider token identity、完整 observability report 和 performance self-test
均通过。该修复没有运行 Whale Agent；I07 仍等待获批真实矩阵验证实际产物结算和失败停止合同。

## 8. 2026-08-18 生产验收与关闭

获批的 `single-file-fast-fix × (standard + map-request) × repeat=3` 共完成 6 次真实运行、41 个 Provider 请求。3/3 Pair
双侧业务、公开测试和隐藏 oracle 全部通过；TaskSpace 3/3 Map 完整闭合且没有图警告。

六个 side 的 canonical facts 合计满足 `41 logical = 41 boundary = 41 completed = 41 usage`，并且 local-only、
boundary-unattributed、duplicate、retry、failed/cancelled 和 finding 均为 0。真实命令显式启用
`-StopOnAnySideFailure`；本轮没有故意制造失败，停止分支由确定性 runner 测试覆盖。请求事实、Map 完成判定、显式停止参数、
最终聚合和账本结算已经形成同一条可复算生产证据链，因此关闭 `R8-I07`。完整结果见
[`../taskspace-exec/78-i05-i07-repeat3-acceptance-result.md`](../taskspace-exec/78-i05-i07-repeat3-acceptance-result.md)。
