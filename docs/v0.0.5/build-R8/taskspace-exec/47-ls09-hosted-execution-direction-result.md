# LS-09 Hosted 执行方向复验结果

- Date: 2026-08-13
- Subject: `72196ecb7d20fe6e1481b32cb4cbb59a5c58ab35`
- Run: `WAR-20260813-220517-CACHE-REGRESSION-ED5FF5CE`
- Matrix: `provider-web-search-probe × map-request × repeat=1`
- Result: **业务通过；方向判别生效；response-scope 漏登仍存在**

## 1. 结论

`execution: "already_executed"` 且禁止 `input` 的结构可被 DeepSeek 正确理解和使用。Agent 在收到第一次准确反馈后，明确
复述该结构只登记 Provider 已执行结果、不调用 Tool；后续在同一 Provider response 中生成真实 `web_search` 和
`taskspace_exec` 归属声明，对账成功并只产生一个逻辑 Hosted result。业务文件、公开验证、隐藏 oracle 和 Map 闭合全部通过。

但该结构没有完全消除错误。首次请求中 Agent 仍在没有真实 Hosted output 时提前声明一次；第一次真实搜索后又漏写归属，
下一请求补写时因 response scope 已变化而被拒绝。随后 Agent 才把真实搜索和归属声明放到同一响应并成功。因此本轮证明：

1. 执行方向歧义是一个真实且可修复的因素；新结构明显缩短了理解路径。
2. 它不是全部根因；同响应组合的生成时机仍不稳定。
3. 不引入 Runtime 自动绑定、默认 Root 或跨响应 pending；先保留当前轻量结构并继续收集同响应漏登证据。

## 2. 关键路径

| 阶段 | Agent 动作 | 结果 |
|---|---|---|
| 首次请求 | 初始化 Map，同时提前声明 `web_search/already_executed` | 准确拒绝：本响应没有真实 Hosted result；零副作用 |
| 第二次请求 | 仅初始化 Map并执行 workspace 检查 | 成功 |
| 首次真实搜索 | Provider 执行 `search + open_page`，Exec 未声明归属 | 准确拒绝：已执行但未归属 |
| 下一请求 | 补写 `already_executed` | 准确拒绝：当前响应没有新 Hosted result |
| 再次搜索 | 同一响应执行 `search` 并声明 `already_executed` | 成功；一个逻辑 `web_search` 绑定 `search` 节点 |
| 交付 | 写文件、运行校验、闭合 Map | 全部通过 |

本轮没有复现把 `input` 填入 Hosted action，也没有把 `search/open_page` 拆成多个 TaskSpace action。Agent 的 reasoning 明确
引用了 `already_executed` 和“does not invoke”反馈，说明新语义已进入其有效上下文。

## 3. 成本

| 指标 | 本轮 | 上一轮逻辑 Hosted 复验 |
|---|---:|---:|
| Provider requests | 12 | 12 |
| Input tokens | 302,780 | 301,975 |
| Cached input | 254,720 | 264,064 |
| Uncached input | 48,060 | 37,911 |
| Output tokens | 7,710 | 10,772 |
| 全量缓存命中率 | 84.13% | 87.45% |
| Request 2+ 命中率 | 86.28% | 89.71% |
| Agent wall time | 77.33 s | 98.20 s |
| 估算费用 | USD 0.009600416 | USD 0.0090630792 |
| 业务结果 | PASS | FAIL |

请求数没有下降，不能宣称成本收益；本轮主要收益是从失败闭环变为完整成功，reasoning/output 与耗时下降。缓存下降和 input
基本持平只是一轮观测，不能归因于该字段。

## 4. Map 与证据

- Map: `root -> search -> write -> finish`，4 节点、3 边，无孤立节点、无 open leaf。
- `provider_fact.json`、公开 validator 和隐藏 oracle 均通过。
- Usage 完整：12/12 Provider requests 均可对账；实际费用低于获批 USD 0.02。
- Result: `benchmarks/cache-regression/results/WAR-20260813-220517-CACHE-REGRESSION-ED5FF5CE.json`
- Evidence: `benchmarks/cache-regression/evidence/WAR-20260813-220517-CACHE-REGRESSION-ED5FF5CE/`
- Local run: `target/r8-ls09/hosted-execution-direction/run/provider-web-search-probe/WAR-20260813-220517-CACHE-REGRESSION-ED5FF5CE-CACHE-001/`

