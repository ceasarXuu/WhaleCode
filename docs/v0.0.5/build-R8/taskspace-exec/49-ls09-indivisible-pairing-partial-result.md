# LS-09 不可拆分配对协议阶段结果

- Date: 2026-08-14
- Subject: `ba8198acd56730efe9a1ba53abd7a7c0410931fc`
- Planned matrix: `provider-web-search-probe × map-request × repeat=5`
- Executed: 3/5；第 3 轮超过单轮 500,000 input 观察阈值后停止
- Model: `deepseek-v4-flash`
- Result: **文字强化没有使双写稳定，前三轮 0/3 业务闭环**

## 1. 单变量变更

本轮只把 Hosted Tool 与 Exec 归属描述为一个“不可拆分动作”：二者必须同一响应出现，禁止只写一边或跨响应补写。
没有修改 Runtime 对账、Map、状态机、合法序列或 Provider 结果聚合。实现提交为 `ba8198acd`，TaskSpace Exec 74 项和
base instructions profile 测试通过，缓存敏感面门禁正确识别 Tool description 变化并阻断到真实复验。

## 2. 已执行三轮

| 轮次 | Record | Requests | Input | Cached | Uncached | Output | Req 2+ cache | 耗时 | 费用 | 业务结果 |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | `03135A5B` | 2 | 26,119 | 12,672 | 13,447 | 1,914 | 95.08% | 35.728 s | USD 0.0024539816 | FAIL |
| 2 | `38D79769` | 12 | 318,146 | 289,280 | 28,866 | 7,023 | 90.59% | 86.957 s | USD 0.0068176640 | FAIL |
| 3 | `12F30F0E` | 12 | 947,691 | 877,824 | 69,867 | 21,012 | 92.54% | 224.943 s | USD 0.0181226472 | FAIL；input 阈值触发 |
| **总计** | 3 runs | **26** | **1,291,956** | **1,179,776** | **112,180** | **29,949** | **92.09% weighted** | **347.628 s** | **USD 0.0273942928** | **0/3** |
| **均值** |  | **8.67** | **430,652** | **393,258.67** | **37,393.33** | **9,983** |  | **115.876 s** | **USD 0.0091314309** |  |
| **中位数** |  | **12** | **318,146** | **289,280** | **28,866** | **7,023** |  | **86.957 s** | **USD 0.0068176640** |  |

全量 Token 加权缓存命中率为 91.32%。所有 Provider usage 均完整；停止原因不是费用硬上限，而是预登记的单轮 input
观察阈值。第 4 轮尚未认领账本即被终止，第 4/5 与第 5/5 不计为运行。

## 3. Trace 结论

1. 第一轮首请求漏掉 `initialize_and_work.tools`；第二请求同时生成一个顶层 `web_search` function call 和一个带归属的
   `taskspace_exec`。Provider response 中没有形成原生 `web_search_call`，该轮在 2 requests 后中断。这说明“不可拆分”措辞
   可能诱发模型把公共归属名误当成可直接调用的 Function Tool。
2. 第二轮在包含真实 `web_search_call` 的响应里同时生成了 `taskspace_exec`，但 Exec `tools` 没有归属条目，出现 2 次
   同响应漏登拒绝。文件与公开校验完成，但 12 requests 用尽前未闭合 Map。
3. 第三轮既出现 Hosted 与归属同响应成功，也出现漏登和无 Hosted 时提前登记。Trace 记录 10 次漏登反馈、3 次提前登记
   反馈；模型随后反复搜索并用满 12 requests，单轮 input 达 947,691。

以上计数只判断每个 Provider response scope 是否同时存在原生 ToolSpec `web_search` 和一个 Exec 归属。`search`、`open_page`、
`find_in_page` 仅用于核对原始 Provider 轨迹，不形成独立 TaskSpace Tool、独立绑定或独立双写。

## 4. 判定

- **修复未通过**：0/3 业务闭环，且出现明确的协议误读；当前提交不能据此晋升缓存基线或关闭 I03。
- **根因收敛**：更强文字仍不能为两个独立顶层 response item 建立结构性共现约束；它还可能使公共归属名被误当成顶层
  Function Tool。问题不在 Web Search 内部 action，也没有证据支持 Runtime 自动绑定。
- **成本异常可解释**：第三轮在漏登/补登拒绝后反复搜索，单个请求最高 376,103 input；累计历史使后续请求输入快速放大。
- **下一步停点**：剩余两轮需要提高单轮 input 观察阈值并重新授权；在此之前不继续付费运行，也不叠加第二项代码改动。

## 5. 证据

- Results: `benchmarks/cache-regression/results/WAR-20260814-015028-CACHE-REGRESSION-03135A5B.json`、
  `WAR-20260814-015115-CACHE-REGRESSION-38D79769.json`、`WAR-20260814-015254-CACHE-REGRESSION-12F30F0E.json`
- Evidence roots: `benchmarks/cache-regression/evidence/WAR-20260814-015028-CACHE-REGRESSION-03135A5B/`、
  `WAR-20260814-015115-CACHE-REGRESSION-38D79769/`、`WAR-20260814-015254-CACHE-REGRESSION-12F30F0E/`
- Local full traces: `target/r8-ls09/indivisible-pairing/run/`
