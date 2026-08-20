# initialize_map 候选 1：类型反馈实验

- Date: 2026-08-16
- Subject implementation: `326e1430c`
- Candidate: 仅把 schema 类型拒绝从笼统的“类型错误”改为 `expected object, got string`
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Effective repeats: 5
- Retry: 0

## 1. 变量边界

本候选不改变 Tool schema、合法序列、Map 状态规则或错误值处置。Runtime 仍然在任何副作用前拒绝类型错误，不解析、不修复、不接受 string；唯一变化是把 schema 已知的期望类型和收到的实际类型忠实写入反馈。

两条聚焦测试证明 validator 与完整 `taskspace_exec` handler 均返回：

```text
$.initialize_map: expected JSON type object, got string
No Map or Tool actions were executed
```

## 2. 五轮 TaskSpace 结果

| Run | 首次 `initialize_map` | 类型反馈触发 | Agent | External | Requests | Input | Cached | Uncached | Output | Wall |
|---:|---|---|---|---|---:|---:|---:|---:|---:|---:|
| 1 | object | 否 | complete | passed | 7 | 103,318 | 96,512 | 6,806 | 2,204 | 18.573s |
| 2 | object | 否 | complete | passed | 10 | 152,515 | 141,952 | 10,563 | 3,150 | 27.181s |
| 3 | object | 否 | complete | passed | 7 | 103,318 | 94,080 | 9,238 | 2,274 | 19.956s |
| 4 | object | 否 | complete | passed | 8 | 118,244 | 110,720 | 7,524 | 2,512 | 21.531s |
| 5 | object | 否 | interrupted | failed | 2 | 25,400 | 24,320 | 1,080 | 506 | 4.774s |
| Total | object 5 / string 0 | 0 | complete 4/5 | passed 4/5 | 34 | 502,795 | 467,584 | 35,211 | 10,646 | 92.015s |

- Request 2+ 加权缓存命中率：`92.18%`。
- 估算费用：`0.07759788 CNY`，包含同批 runner 意外产生的两轮 Standard 对照；仅 TaskSpace 五轮费用低于该值。

Run 5 成功执行 `initialize_and_work` 后，Agent 在下一响应生成了未暴露的顶层 `exec_command`，没有继续使用 `taskspace_exec`，进程随即中断。该运行没有触发类型拒绝，不能归因于候选 1。

## 3. 与既有基线对比

| 版本 | TaskSpace repeats | 首发 object | 首发 string | string 后恢复 | External passed | Request 2+ cache |
|---|---:|---:|---:|---:|---:|---:|
| 既有基线 | 5 | 4 | 1 | 1/1，下一请求 | 5/5 | 92.41% |
| 候选 1 | 5 | 5 | 0 | 未触发，无法评价 | 4/5 | 92.18% |

两组样本太小，且候选 1 不影响首发 schema 生成，因此不能把 `1/5 -> 0/5` 解释为首发错误率改善。候选 1 的直接收益仅在 string 已发生后才可观察，本轮没有获得该触发条件。

## 4. 结论

1. 候选 1 的工程语义正确：反馈更完整，错误值仍被零副作用拒绝。
2. 五轮未观察到反馈引起的正确性、缓存或请求回归。
3. 本轮没有直接验证恢复率收益，不能仅凭未触发 string 宣称候选有效。
4. 候选 1 可以作为低风险反馈修复保留候选，但不能解释或根治首发二次序列化。

## 5. 证据

- Mixed batch: `target/r8-initialize-map-candidates/candidate1-feedback/single-file-fast-fix/20260816-183025-671`
- Supplement 1: `target/r8-initialize-map-candidates/candidate1-feedback-supplement-1/single-file-fast-fix/20260816-183604-674`
- Supplement 2: `target/r8-initialize-map-candidates/candidate1-feedback-supplement-2/single-file-fast-fix/20260816-183655-623`
- Ledger: `WAR-20260816-005101-INIT-MAP-CANDIDATE1`、`WAR-20260816-183437-INIT-MAP-CANDIDATE1-SUPPLEMENT`

同批 `repeat=5` runner 因左右臂交替实际产生 TaskSpace 3 轮、Standard 2 轮；两个缺失的 TaskSpace 观测随后以两个独立 `repeat=1` 补足，没有重跑前三个 TaskSpace 观测。
