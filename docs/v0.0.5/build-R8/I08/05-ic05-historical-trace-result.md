# IC-05 历史真实 Trace 成本复算

- Status: complete
- Date: 2026-08-17
- Paid Whale Agent runs: 0
- Evidence boundary: 历史真实 Provider usage 与 v1 wire section；新增 history/tool 子分类不可追溯，明确记为 unavailable

## 1. 结论

历史证据支持两个同时成立的结构因素：

1. TaskSpace 的平均请求数更多；同一旧提交的成功简单样本中为 `8.0` 对 `6.0`，放大 `1.333x`。
2. TaskSpace 的平均每请求 input 也更大；为 `14,964.6` 对 `12,465.7`，放大 `1.200x`。

两者相乘得到总 input 均值 `1.601x`。因此不能再把高 input 单独归因于请求次数，也不能单独归因于 Tool schema；
当前可解释模型是“更多请求重复携带更大的每请求上下文”。

## 2. 同提交历史对照

证据来自 `WAR-20260816-005101-INIT-MAP-CANDIDATE1`，subject commit `326e1430c`。五轮业务与隐藏 oracle 均通过，
实际为 TaskSpace 3 轮、Standard 2 轮。它不是当前提交的晋升基线，只用于定位历史成本结构。

| Mode | Runs | Requests total / mean / median | Input total / mean / median | Cached input | Output | Wire bytes total |
|---|---:|---:|---:|---:|---:|---:|
| Standard | 2 | 12 / 6.0 / 6 | 149,588 / 74,794 / 74,794 | 146,560 | 2,892 | 608,450 |
| TaskSpace | 3 | 24 / 8.0 / 7 | 359,151 / 119,717 / 103,318 | 332,544 | 7,628 | 1,447,208 |

逐轮数据：

| Run | Mode | Requests | Input | Cached | Output | Wire bytes |
|---|---|---:|---:|---:|---:|---:|
| 1 | TaskSpace | 7 | 103,318 | 96,512 | 2,204 | 415,110 |
| 2 | Standard | 6 | 74,985 | 73,472 | 1,537 | 303,990 |
| 3 | TaskSpace | 10 | 152,515 | 141,952 | 3,150 | 616,623 |
| 4 | Standard | 6 | 74,603 | 73,088 | 1,355 | 304,460 |
| 5 | TaskSpace | 7 | 103,318 | 94,080 | 2,274 | 415,475 |

TaskSpace 的 10-request 单轮会拉高均值；即使使用中位数，TaskSpace 仍为 7 requests / 103,318 input，Standard 为
6 requests / 74,794 input。小样本只能证明结构存在，不能给稳定频率。

### 2.1 平均每请求结构

| Section | Standard bytes/request | TaskSpace bytes/request | Delta |
|---|---:|---:|---:|
| system messages | 5,049.0 | 5,049.0 | 0.0 |
| natural history | 5,616.3 | 10,245.6 | +4,629.3 |
| tools | 18,623.0 | 25,001.0 | +6,378.0 |
| other payload | 21,395.8 | 19,984.8 | -1,411.1 |
| tool choice | 20.0 | 20.0 | 0.0 |
| **Wire total** | **50,704.2** | **60,300.3** | **+9,596.2 / +18.93%** |

该 v1 trace 把 Responses `instructions` 放在 `other_payload`，也无法继续拆分 natural history。IC-01～IC-03 已修正新 trace，
但不能从旧 hash 反推细分内容。

### 2.2 放大关系

```text
request amplification          = 8.0 / 6.0       = 1.333x
per-request input amplification = 14,964.6 / 12,465.7 = 1.200x
total input amplification       = 119,717 / 74,794 = 1.601x
```

若按 Standard 每请求 input 先计算请求数增量，再计算 TaskSpace 每请求体量增量：总均值差 `44,923` input 中，约
`24,931` 来自多出的 2 个请求，约 `19,992` 来自 8 个请求各自更大。这个 `55.5% / 44.5%` 仅是明确顺序下的算术分解，
不是两个独立因果效应；真实当前比例必须由 IC-06 同版本双臂确认。

## 3. 最近干净 TaskSpace 五轮

证据来自 `WAR-20260817-034839-SELF-HEAL-R5`，subject commit `2c2144e7343e82e0d7cdfc80f8b53d4ee3634124`。
5/5 业务、公开验证、隐藏 oracle 和 Map 闭合通过，零 syntax/protocol/state reject。

| Metric | Total | Mean | Median |
|---|---:|---:|---:|
| Requests | 34 | 6.8 | 7 |
| Input | 528,450 | 105,690.0 | 108,426 |
| Cached input | 491,136 | 98,227.2 | 101,376 |
| Output | 11,356 | 2,271.2 | 2,122 |
| Provider wire bytes | 2,132,989 | 426,597.8 | 438,406 |

Wire 累计面积：

| Section | Bytes area | Share |
|---|---:|---:|
| system messages | 171,666 | 8.05% |
| natural history | 360,100 | 16.88% |
| tools | 907,392 | 42.54% |
| other payload | 693,151 | 32.50% |
| tool choice | 680 | 0.03% |
| **Total** | **2,132,989** | **100.00%** |

旧 observer 中 `other_payload` 主要包含 Base instructions。固定 system/tools/base/wrapper 每请求都会重复计入 input，累计占
`83.12%` wire area；自然历史占 `16.88%`。这不表示固定内容应删除，而是证明额外请求会重复放大一整块固定前缀。

## 4. 异常请求成本证据

`WAR-20260812-235208-CACHE-REGRESSION-273B1476` 的复杂样本共 12 requests、211,107 input，其中存在 1 次 JSON syntax
reject 和 2 次 Waiting frontier reject。三个产生零 Map/Tool 副作用的 Provider 响应位于 request 6、7、9：

- input 合计 `56,997`，占该 run 总 input 的 `27.00%`；
- wire bytes 合计 `224,902`，占该 run wire area 的 `26.83%`。

这些数字是“异常响应承载的实际面积”，不是删除异常后的精确反事实收益：恢复后本来可能仍需下一次正常语义推进。不过它坐实 H6：
本地零副作用拒绝仍会完整消耗一次 Provider input/output，并继续扩大后续历史。

## 5. 假设状态

| Hypothesis | IC-05 状态 | 证据 |
|---|---|---|
| H1 请求次数放大 | supported | 同提交历史对照 8.0 vs 6.0 requests/run |
| H2 Tool wire 固定增量 | supported | 历史 +6,378 B/request；当前 IC-04 +11,965 B 首请求 |
| H3 outer history 重复 | unresolved | v1 trace 无法拆分 Exec call/output 与 nested result |
| H4 Map read/projection 主增量 | not supported as primary, unresolved in detail | map-request 无 active projection；全部 history 仅占干净五轮 16.88%，但内部仍不可拆 |
| H5 协议跨层重复 | unresolved | 需要当前 v2 trace，不从旧 hash 推断文本 |
| H6 reject 放大 | supported | 复杂样本三个零副作用响应承载 56,997 input |
| H7 观察器/usage 错误 | old trace bounded | request-facts 与 wire identity 完整；旧 section 细分明确 unavailable |

## 6. IC-06 必须回答的问题

1. 当前提交中，Standard 与 map-request 是否仍维持约 `1.33x` 请求数和 `1.20x` 每请求 input。
2. v2 history breakdown 中，`taskspace_exec_call/output` 各自累计多少，是否复制 nested Tool 事实。
3. 当前 Tool 增量约 12 KB/request 在真实 Provider token 中对应多少，而不是用 bytes 估算。
4. 两臂均无 reject 时，总差值是否仍成立；出现异常则停止比较，不自动补跑。
