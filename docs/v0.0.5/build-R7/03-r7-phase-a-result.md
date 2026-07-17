# R7 Phase A 合同、Inventory 与冻结基线结果

- Created: 2026-07-18
- Updated: 2026-07-18
- Status: Complete / Phase B Ready
- Production Behavior Change: None
- R6 Frozen Commit: `e29810158`
- R6 Runtime Commit: `2117de6e`
- Machine Result: `benchmarks/taskspace/r7/phase-a-baseline-result.json`

## 1. 阶段结论

Phase A 已完成。R7 的三种 projection 策略已经被收敛为同一 TaskSpace 架构上的一个 emission
策略点，而不是三套 Runtime 或第四个 R6 兼容模式。现存 R6 projection/epoch 耦合全部有明确归属与
删除/替换阶段；Standard/R6 两组 Docker 冻结基线有效；现有 provider wire 观测链可直接复用于
Phase B。

本阶段只新增合同、inventory、验证脚本和结果文档，没有修改生产代码或运行行为。

## 2. 合同与 Inventory

| Gate | 结果 | 证据 |
|---|---:|---|
| 三种 policy 严格枚举 | PASS | `map-always`、`map-append`、`map-request` |
| Standard/R6 不进入 policy enum | PASS | 机器合同显式排除 |
| canonical Map/renderer/composer/tool/event store 单实例 | PASS | `shared_architecture` 合同 |
| request/revision/read/compaction/resume trigger | PASS | 三策略 trigger matrix |
| session 创建时冻结、resume/fork 恢复 | PASS | session lifecycle 合同 |
| R6 compatibility/migration/fallback | NONE | compatibility policy 为 `none` |
| R6 marker 文件处置覆盖 | 32/32 | ownership inventory + `rg` gate |
| unknown ownership | 0 | inventory machine test |

Phase B 最早需要适配 provider wire、client scanner 和 benchmark cost observer，因为 `map-always`
纵向切换必须在上线时就能验证 freshness、section cost 和 projection identity；Phase F 负责把同一套
观测扩展到 append/request 与正式四臂报告，不能把 always 的唯一 projection 断言误用到所有策略。

## 3. 冻结执行合同

```text
Model: deepseek-v4-flash
Reasoning effort: max
Execution: Docker hard boundary
Repeats: 1 per arm
Arms: Standard + frozen R6
Binary SHA256: 07043e7d79823b1f04a5d8d36f6d67b5f9c75d576a4e448d53abc738af6563c1
Aggregate utility: disabled
```

两组样本均为诊断基线，不用于判断三种 R7 策略的收益或默认值。正式效用结论仍需 Phase G
Standard + 三策略四臂、每 arm 至少 3 次。

## 4. Simple 基线

Scenario：`single-file-fast-fix`

| 指标 | Standard | Frozen R6 | R6 / Standard |
|---|---:|---:|---:|
| 结果 | solved | solved | 同为通过 |
| provider request | 7 | 19 | 2.714x |
| tool call（总/普通/control） | 10 / 10 / 0 | 18 / 9 / 9 | - |
| failed tool call | 1 | 1 | - |
| wall time | 16,759 ms | 46,554 ms | 2.778x |
| input token | 49,493 | 231,221 | 4.672x |
| cached input | 47,104 | 207,744 | 4.410x |
| uncached input | 2,389 | 23,477 | 9.827x |
| output token | 1,444 | 4,499 | 3.116x |
| request 2+ cache hit | 94.80% | 89.74% | -5.06 pp |
| message prefix preserved | 100.00% | 88.89% | -11.11 pp |
| context projection（count/token） | 0 / 0 | 1 / 356 | - |
| provider-wire projection token 总量 | 0 | 6,429 | - |
| Map nodes / edges / open leaves | 0 / 0 / 0 | 5 / 4 / 0 | - |

R6 在该单次简单样本上是明确的高成本路径，但它是单次 Agent 行为样本，不能外推为稳定分布。
两臂记录的 1 次 failed tool 分别来自修复前红测和 TaskSpace 初始化前 hard gate；最终 public/hidden
validator、terminal closure 和工程清洁性均通过。

## 5. Complex 基线

Scenario：`subscription-billing-repair`

| 指标 | Standard | Frozen R6 | R6 / Standard |
|---|---:|---:|---:|
| 结果 | solved | solved | 同为通过 |
| provider request | 10 | 16 | 1.600x |
| tool call（总/普通/control） | 17 / 17 / 0 | 22 / 15 / 7 | - |
| failed tool call | 1 | 1 | - |
| wall time | 61,680 ms | 58,927 ms | 0.955x |
| input token | 107,089 | 209,772 | 1.959x |
| cached input | 102,144 | 184,704 | 1.808x |
| uncached input | 4,945 | 25,068 | 5.069x |
| output token | 6,753 | 5,767 | 0.854x |
| request 2+ cache hit | 95.29% | 87.87% | -7.43 pp |
| message prefix preserved | 100.00% | 86.67% | -13.33 pp |
| context projection（count/token） | 0 / 0 | 1 / 279 | - |
| provider-wire projection token 总量 | 0 | 4,469 | - |
| Map nodes / edges / open leaves | 0 / 0 / 0 | 4 / 3 / 0 | - |

该样本中 R6 虽多 60% request、约 1.96 倍 input，但 wall time 低 4.5%，说明单次请求数和 token
不能直接替代任务结果与耗时分布。R6 Map 为合法 Root-to-Finish 路径，无环、无 open leaf。

## 6. Wire 离线重算

新增 `verify-r7-phase-a-baseline.ps1`，直接以每个 arm 的原始 `provider-wire-trace.jsonl` 调用现有
`New-TaskspaceProviderCacheTraceArtifacts`，并与运行时保存的完整 summary 做结构精确比较。

| Scenario | Standard | Frozen R6 |
|---|---:|---:|
| simple cache/section/identity | EXACT MATCH | EXACT MATCH |
| complex cache/section/identity | EXACT MATCH | EXACT MATCH |

这证明 Phase B 不需要另建一套 benchmark parser。需要改变的是 observer 的策略判定规则，不是
section/cache/identity 的原始提取链。

## 7. 验证命令

```powershell
pwsh -NoLogo -NoProfile -File scripts/taskspace-benchmark/test-r7-projection-policy-contract.ps1
pwsh -NoLogo -NoProfile -File scripts/taskspace-benchmark/verify-r7-phase-a-baseline.ps1
```

结果：

```text
R7 projection policy contract passed.
Inventory items: 32
R6 epoch marker files covered: 32
R7 Phase A frozen baseline passed.
Scenarios: 2
Offline wire summaries recomputed: 4
```

## 8. Phase B 准入边界

Phase B 可以开始，但只允许完成一个 `map-always` 纵向切换：

1. 建立共享 policy/trigger/cursor/emission 纯合同；
2. renderer 去除 R6 epoch 产品语义；
3. composer 每个 active request 只放一份最新 projection；
4. 删除 epoch cache、anchor、scope、marker filter 和旧日志；
5. 同步接入 always freshness 与成本观测；
6. 不修改 Rooted DAG、ordinary tools、hard gate、hook 或 Agent 决策语义。

Phase B 完成后必须暂停，执行 Standard/R6/R7-always 的 simple、complex 单次横向验证，再决定是否
进入 Phase C。
