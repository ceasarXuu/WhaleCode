# R8-I07 独立修复结果

- Date: 2026-08-05
- Scope: `I07-W0`～`I07-W8`、`I07-W10`
- Status: 独立修复与首轮对抗性修复完成；closure review 进行中；完整 I07 保持 queued
- API usage: 0；全部验证均为本地确定性 fixture

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

当前规范化 artifact 为 `whalecode-request-facts-v1`，analyzer 为 `i07-review-fixes-v2`，诊断合同为
`whalecode-request-facts-diagnostics-v1`，run evidence manifest 升级为 v2。

首轮对抗性审查发现的 6 个阻断缺口已由 `9dc661aa0` 修复：failed terminal 不再继承 rollout usage；boundary
要求完整 start/stop 生命周期；相同 payload retry 只让逐 attempt 关联不可比较；cache/performance/freshness 不再用
shape 或 completion 冒充 boundary request；canonical consumer 也进入 inventory 门禁。完整审查轨迹见
[`vs_review/2026-08-05-i07-independent-repair-review.md`](../../../../vs_review/2026-08-05-i07-independent-repair-review.md)。

## 3. 验证结果

| 验证 | 结果 |
|---|---|
| I07 历史反例 | 8 completed/usage + 7 snapshots；11 attempts + 10 boundary + 1 local-only failure |
| request facts + inventory + boundary proxy | 21 tests passed；consumer gate passed |
| cache regression | 219 tests passed |
| cost / harness / performance | 全部 self-test passed |
| provenance / freshness | source 变更、analyzer 变更和旧 facts 均 fail closed |
| 24-run 离线报表矩阵 | `R7 request observability report passed.` |
| 缓存敏感面静态门禁 | PASS；观测政策变化仍保持发布阻断，未以静态结果替代真实缓存回归 |

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
| `attempt-boundary-events.jsonl` | `7ae6af39b685544ea9c6e27568808821cb2adf540d2b077cdeb67e4f941022a7` |
| `attempt-boundary-wire.jsonl` | `cde1c1591c5bfc38112547b66a3bb1ca09a990c5ce750f70ebeb0f4e3404e16e` |
| `usage-double-count-rollout.jsonl` | `eee6797144db27dd84b74b5ca654d643a99eb3a2787f4885c57547eee454f991` |

## 4. 已知副作用

- 旧 evidence manifest v1 和旧 analyzer 产物会被标记 legacy/incomparable，不做推测兼容；
- 报告增加 logical、attempt、boundary、completed 和 availability 字段，历史错误总量会下降；
- request facts 生成继续使用 Harness 已有的本地 Python 进程；
- 缓存敏感面门禁仍要求后续经用户授权的真实回归才能晋升发布基线。

## 5. 未完成边界

- `I07-W9`：TaskSpace Exec outer call、internal item/node 与 Provider facts 的身份关联；
- `I07-W11`：新协议稳定后另行申请最小真实预算，执行生产 trace 逐 ID 对账；
- 上述工作不得重建第二套 Provider 计数规则，也不得让 observer 反向控制 Agent 或 Tool。

因此本结果关闭的是“当前协议下可独立修复的观测缺陷”，不关闭全局问题 `R8-I07`。
