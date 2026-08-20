# initialize_map 候选 2 十轮复验

- Date: 2026-08-16
- Subject implementation: `92fcf5594`
- Candidate: `initialize_map` 使用就地 object schema，保留完整首次请求示例
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Effective repeats: 10 个独立 `repeat=1`
- Retry: 0
- Ledger: `WAR-20260816-192321-INIT-MAP-C2-REPEAT10`

## 1. 验证范围

本轮不再改代码，只扩大 C2 的目标模型样本。主要观察量是首个原始 Provider Function Call 中
`initialize_map` 的实际 JSON 类型；同时保留首次请求是否带 work、业务正确性、响应合同逃逸、请求成本和缓存证据，避免只看目标字段而遗漏整体回归。

10 轮拆成两批并行执行，每轮独立 run root、最多 12 个 Provider 请求。没有重试失败观测，也没有用补跑替换异常样本。

观测资格为 9 个 complete TaskSpace row、1 个 interrupted TaskSpace row；10 个 row 均有完整 request-facts usage。
每个 run 的 Standard left side 都按 right-only 设计标记为 skipped，不是零成本测量，也不参与本报告比较。

## 2. 逐轮结果

| Run | 首次 map | 首次带 work | Agent | External | Requests | Input | Cached | Uncached | Output | Wall |
|---:|---|---|---|---|---:|---:|---:|---:|---:|---:|
| 1 | object | 是 | complete | passed | 7 | 101,310 | 94,208 | 7,102 | 2,138 | 17.683s |
| 2 | object | 否，下一请求修正 | complete | passed | 7 | 98,797 | 92,032 | 6,765 | 1,950 | 17.743s |
| 3 | object | 是 | interrupted | failed | 2 | 25,314 | 24,320 | 994 | 482 | 4.162s |
| 4 | object | 否，下一请求修正 | complete | passed | 12 | 187,318 | 176,512 | 10,806 | 3,443 | 30.927s |
| 5 | object | 是 | complete | passed | 7 | 103,057 | 95,744 | 7,313 | 1,870 | 16.557s |
| 6 | object | 是 | complete | passed | 7 | 103,496 | 97,536 | 5,960 | 2,196 | 18.355s |
| 7 | object | 是 | complete | passed | 7 | 102,146 | 95,488 | 6,658 | 2,001 | 18.443s |
| 8 | object | 否，下一请求修正 | complete | passed | 8 | 118,754 | 109,824 | 8,930 | 2,845 | 23.031s |
| 9 | object | 否，下一请求修正 | complete | passed | 9 | 132,638 | 123,648 | 8,990 | 2,327 | 21.625s |
| 10 | object | 是 | complete | passed | 7 | 100,277 | 93,312 | 6,965 | 1,567 | 15.226s |
| **Total** | **object 10 / string 0** | **6/10** | **9/10** | **9/10** | **73** | **1,073,107** | **1,002,624** | **70,483** | **20,819** | **183.752s** |
| **Mean** | - | - | - | - | **7.3** | **107,310.7** | **100,262.4** | **7,048.3** | **2,081.9** | **18.375s** |
| **Median** | - | - | - | - | **7** | **102,601.5** | **95,616** | **7,033.5** | **2,069.5** | **18.049s** |

## 3. 类型与序列结论

1. 首个 `initialize_map` 为 object 10/10；本批没有出现 JSON string，也没有触发类型反馈。
2. 只有 6/10 首次请求同时包含 client work。另 4 次虽然 Map 对象完整，但 `tools` 缺失；Runtime 以
   `initialize_and_work requires work` 零副作用拒绝，Agent 下一请求 4/4 补齐同一 Map 与 work。
3. 因而 C2 的字段类型表现稳定，但“首次完整合法序列”并未达到 10/10。不能把 object 10/10 扩大解释为整个 Exec 协议已经稳定。
4. 连同 C2 既有五轮，当前 C2 的首发类型累计为 object 15/15、string 0/15。既有未改 schema 的 C1 也曾得到 object 5/5；
   与原基线 4/5 相比，样本仍不足以把内联 schema 的因果效果与模型随机波动区分开。

## 4. 唯一业务失败

Run 3 首次 `initialize_and_work` 合法并执行成功。第二个 Provider 响应却生成了顶层
`FunctionCall(name=exec_command)`，而真实 TaskSpace 顶层只暴露 `taskspace_exec` 与 Provider Tool。响应合同在任何第二步副作用前拒绝，运行中断，业务文件未修改。

这与 `initialize_map` 的类型和内联 schema 无关，并非本轮首次出现：C1 五轮中的唯一失败也是成功初始化后发生同类顶层
`exec_command` 逃逸。该重复证据继续归入 I03 的通用动作组织稳定性，不为它新增问题编号，也不把它计为 C2 类型失败。

除该响应级逃逸外，10 轮共有 8 次 `taskspace_exec` 零副作用拒绝：4 次首次缺 work、1 次提前选择 waiting `verify`、
3 次无效状态转换。Run 4 因两次状态转换错误放大到 12 个请求，但仍完成业务、验证、Map 和最终答复。

## 5. Map

- 8 轮建立 5 个节点、4 条边；2 轮建立 4 个节点、3 条边，全部是从 Root 到明确 Finish 的单链 DAG。
- 9 个成功运行的 Map 均 `complete=true`，Root 与 Finish 已完成，open leaf 为 0。
- Run 3 的 Map 已持久化 5 个节点、4 条边，但因响应级逃逸停在 `complete=false`：Root 与 inspect 为 InFlight，后续节点 Waiting。
- 没有孤立节点、边顺序违规或 Map Store 错误。

## 6. 成本与缓存

- 全批加权缓存命中率：`93.43%`。
- Request 2+：`881,024 / 950,256 = 92.71%`。
- 相邻请求严格完整前缀：`0 / 63`；差异都首先出现在自然追加的 message，不能把该指标解释成 Provider 缓存未命中。
- `tool_choice` transition：0；cache-shape transition：0；zero-cache request：0；same-shape zero：0。
- 估算费用：`0.13217348 CNY`，占本次 `2.5 CNY` 硬上限的 `5.29%`。

缓存和 Tool shape 在 10 轮中保持稳定：首请求 `tools_hash` 均为
`848829796c7fb90ba7b0f48d0c21784459cb0c5d1c8e7f23c597f4a96ca825bf`，`tool_choice=auto`，没有观察到 C2 引入缓存回归。

## 7. 判断

1. **保留 C2 候选**：累计 15 次没有复现 string，且当前批缓存、成本和业务路径没有显示由 schema 内联导致的负向变化。
2. **H-003 仍不关闭**：本轮强化了相关性证据，但没有同版本并发对照；C1 的 0/5 仍阻止把 `$ref` 宣称为已坐实根因。
3. **I03 继续 verifying**：首次无 work 4/10 和顶层 client Tool 逃逸 1/10 表明整体序列生成仍有独立问题。
4. 本轮只验证 `single-file-fast-fix × map-request`，不形成 Standard 成本对比，也不晋升缓存正式基线。

## 8. 证据

- Run roots: `target/r8-initialize-map-candidates/candidate2-repeat10-{1..10}`
- 每个实际 run root 均包含 `performance-observation.{json,md}` 与 `performance-observation-events.jsonl`
- Capability identity: `a95be2ff3edf5911780794843ddee89f4348358e206f69227f675d0cc041ef11`
- Binary attestation: `third_party/codex-cli/codex-rs/target/debug/whale.build-attestation.json`

## 9. 后续处置

本轮 4/10 的首次 Map-only 空推进证明：退出 Provider Agent 归属协议后，遗留的响应级 Provider/client OR 合同不再成立。
后续已按最新产品决策恢复工作型序列的非空 client `tools[]` 结构前置条件，详见
[`65-client-work-structural-restoration.md`](65-client-work-structural-restoration.md)。本报告的原始统计保持不变。
