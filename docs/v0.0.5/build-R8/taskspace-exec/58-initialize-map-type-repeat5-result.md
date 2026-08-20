# initialize_map 参数类型五轮复验

- Date: 2026-08-15
- Subject: `5e5f01e3fcf6c641b6f42d7043f0e33d90fd52ab`
- Binary: `38f000033c490dff0c5128c7fa1d37491dc60c74f514b6bed0207b530dc8d1bd`
- Matrix: `single-file-fast-fix × map-request × repeat=5`
- Model: `deepseek-v4-flash`
- Retry: 0
- Status: 五个观测全部完成；I03 保持 verifying

## 1. 验证目标

上一轮最小缓存回归中，Agent 连续十次把 schema 要求为 object 的 `initialize_map` 写成 JSON string，Map 始终没有初始化。
本轮只复验同一版本、同一模型、同一 sample 和同一 arm，统计首个 `taskspace_exec` 的实际参数形态及错误后的恢复行为。

原 repeat=5 runner 在第 2 轮业务生命周期失败后错误停止，尽管预算合同没有选择
`after_any_business_failure`。剩余第 3 至 5 个观测在原批准总数内拆成三个独立 repeat=1；没有重跑前两轮，也没有增加 sample。

## 2. 结果

| Run | 首次 `initialize_map` | 恢复 | Agent 生命周期 | 外部正确性 | Requests | Input | Cached | Uncached | Output | Request 2+ cache | Time | 费用 USD |
|---:|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | object | 不需要 | complete | public + hidden passed | 8 | 125,472 | 116,352 | 9,120 | 3,250 | 92.07% | 45.538s | 0.0025126 |
| 2 | JSON string | 下一请求改为 object | interrupted | public + hidden passed | 10 | 157,103 | 144,896 | 12,207 | 3,839 | 91.67% | 51.547s | 0.0031896 |
| 3 | object | 不需要 | complete | public + hidden passed | 8 | 120,085 | 112,896 | 7,189 | 2,607 | 93.46% | 42.481s | 0.0020525 |
| 4 | object | 不需要 | interrupted | public + hidden passed | 10 | 158,511 | 146,944 | 11,567 | 3,751 | 92.19% | 51.447s | 0.0030811 |
| 5 | object | 不需要 | complete | public + hidden passed | 7 | 105,221 | 98,688 | 6,533 | 3,001 | 93.12% | 43.537s | 0.0020312 |
| Total | object 4 / string 1 | string 1/1 recovered | complete 3/5 | passed 5/5 | 43 | 666,392 | 619,776 | 46,616 | 16,448 | 92.41% weighted | 234.550s | 0.0128671 |
| Mean | - | - | - | - | 8.6 | 133,278.4 | 123,955.2 | 9,323.2 | 3,289.6 | - | 46.910s | 0.0025734 |
| Median | - | - | - | - | 8 | 125,472 | 116,352 | 9,120 | 3,250 | - | 45.538s | 0.0025126 |

两个 interrupted 运行都已经修改正确文件、通过公开测试和隐藏 oracle，并把 Map 完整闭合为 Root completed、零 open leaf。
它们在第 10 个请求执行完 `update_and_finish` 后触达请求硬上限，缺少下一次 Provider 请求生成最终自然语言回复，因此测量层没有把
Agent 生命周期记为 complete。这不是代码修复失败或 Map 未闭合。

## 3. 参数类型事实

本轮错误只发生在 Run 2 的首个原始 Provider Function Call：顶层 `taskspace_exec.arguments` 可以正常解码，`type` 和
`tools[]` 也保持结构化对象，但 `initialize_map` 单独多序列化了一层：

```text
initialize_map: "{\"root\": ..., \"work_nodes\": ..., \"finish\": ...}"
```

Runtime 在任何 Map 或 client Tool 副作用前忠实拒绝：

```text
invalid top-level contract: $.initialize_map: value has the wrong JSON type
```

下一请求 Agent 直接改为 object，初始化和首个 client work 同批成功。Runtime 没有把 object 改成 string，也没有静默把 string
解析成 object；错误形态已经存在于 Provider 返回的原始 Function Call 中。

五轮首请求的 Tool schema hash、cache shape、Base Instructions hash、system section hash、`tool_choice=auto`、Tool 数量和
payload bytes 全部相同。自然历史包含逐运行事实，因此 hash 不同，但字节长度相同。当前证据排除了“某一轮切换了 schema、提示词版本或
tool_choice”这一结构性解释；它支持目标模型在相同 Function Tool 合同下存在随机的嵌套字段二次序列化错误。

## 4. 结论边界

1. 当前批次首发正确率为 4/5，首发 JSON string 频率为 1/5；样本量不足以外推长期概率。
2. 错误反馈在本轮唯一错误中支持一次请求内恢复，但上一轮曾连续十次重复，因此不能宣称恢复已经稳定。
3. 25,001-byte Tool section 和较深嵌套是待检验风险因素，不是本轮坐实根因；4/5 正确也证明 schema 并非确定性不可生成。
4. 不应由 Runtime 猜测并重解释任意 string。当前正确底线仍是类型校验、零副作用拒绝和忠实错误反馈。
5. I03 保持 verifying。后续若优化 schema，必须单变量验证首发类型正确率、普通序列能力和缓存成本，不能只迎合本 sample。

## 5. 同轮其他观察

- Run 1、4、5 各出现一次对 waiting `verify` 节点的提前调用，均被硬规则零副作用拒绝，Agent 随后完成父节点再继续；归入 I04。
- Run 4 另有两次后续 `taskspace_exec` JSON 语法错误，与 `initialize_map` 类型无关；Agent 自行修正。
- Run 4 把有依赖的 Patch 与 pytest 放在同一无序 Tool batch，测试可能先于 Patch 执行；Agent 从结果识别并重新验证。该行为属于 I03
  的序列组织质量，不应由 Runtime 为普通 Tool 人为推断依赖顺序。
- 缓存结构稳定，五轮 request 2+ 加权命中率为 92.41%；本轮没有 Standard arm，不形成相对性能结论，也不晋升缓存基线。

## 6. 证据

- Gate: `benchmarks/cache-regression/gate-reports/2026-08-15-init-map-type-repeat5-head.json`
- Initial batch: `WAR-20260815-220536-CACHE-REGRESSION-343CA6AE`
- Remaining observations: `WAR-20260815-230742-CACHE-REGRESSION-9E24A5D2`,
  `WAR-20260815-230836-CACHE-REGRESSION-4999CA46`, `WAR-20260815-230939-CACHE-REGRESSION-0D15CD38`
- Ledger: `benchmarks/whale-agent-run-ledger.json`
- Pre-start evidence `WAR-20260815-220446-CACHE-REGRESSION-2753CF0B` 只完成 Provider route 和 binary health
  预检；runner 因显式相对 `--run-root` 无法执行 `relative_to(repo)` 而在登记 planned 账本及任何 Provider 请求前退出，
  不计入五个 sample 或费用。
