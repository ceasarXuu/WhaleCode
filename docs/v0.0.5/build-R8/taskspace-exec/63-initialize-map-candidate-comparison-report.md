# initialize_map 四候选单变量对比报告

- Date: 2026-08-16
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- TaskSpace arm: `map-request`
- Retry: 0
- Original four-candidate budget: no more than 5 CNY
- Original four-candidate ledger cost: `0.2562812 CNY`
- Separately approved C2 repeat=10 ceiling / actual: `2.5 / 0.13217348 CNY`

## 1. 候选定义

| Candidate | 唯一变量 | 目标 |
|---|---|---|
| C1 | 类型错误反馈增加 expected/actual | 错误发生后让 Agent 准确理解 object/string 差异 |
| C2 | `initialize_map` 从 `$ref` 改为同合同内联 object | 减少嵌套 schema 表达边界 |
| C3 | 删除首次 `initialize_and_work` 完整示例 | 检验示例是否诱发 object 二次序列化 |
| C4 | 仅设置 `taskspace_exec.strict=true` | 检验 Provider strict 能否直接保证参数类型 |

## 2. TaskSpace 在线结果

| Version | Repeats | 首发 object/string | 首次合法序列 | Agent complete | External passed | Requests | Input | Cached | Uncached | Output | Request 2+ cache | Wall | 估算 CNY |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 既有基线 | 5 | 4 / 1 | 未专项统计 | 3/5 | 5/5 | 43 | 666,392 | 619,776 | 46,616 | 16,448 | 92.41% | 234.550s | 0.09190752 |
| C1 反馈增强 | 5 | 5 / 0 | 未专项统计 | 4/5 | 4/5 | 34 | 502,795 | 467,584 | 35,211 | 10,646 | 92.18% | 92.015s | 0.06585468 |
| C2 schema 内联 | 5 | 5 / 0 | 5/5 | 5/5 | 5/5 | 40 | 592,745 | 546,176 | 46,569 | 11,898 | 92.90% | 102.802s | 0.08128852 |
| C2 扩大复验 | 10 | 10 / 0 | 6/10 | 9/10 | 9/10 | 73 | 1,073,107 | 1,002,624 | 70,483 | 20,819 | 92.71% | 183.752s | 0.13217348 |
| C3 删除示例 | 5 | 5 / 0 | 2/5 | 5/5 | 5/5 | 39 | 572,942 | 512,640 | 60,302 | 13,420 | 92.95% | 109.474s | 0.09739480 |
| C4 strict | 0 | N/A | N/A | N/A | N/A | 0 | 0 | 0 | 0 | 0 | N/A | 0 | 0 |

成本按同一冻结公式归一化：cached input `0.02 CNY/M`、uncached input `1 CNY/M`、output `2 CNY/M`。C1 表中只统计五个 TaskSpace 观测；该候选账本还包含 runner 意外产生的 2 个 Standard 观测，因此实际预算结算高于表中 TaskSpace-only 成本。

## 3. 判断

| Candidate | 效果判断 | 是否保留 |
|---|---|---|
| C1 | 反馈语义正确且无观察到的缓存回归；但 5 轮没有触发 string，恢复收益未直接验证 | 保留为独立低风险反馈候选，不宣称解决首发错误 |
| C2 | 类型累计 15/15 object；最新十轮只有 6/10 首次携带 work、9/10 业务通过；Tool section 减少 63 bytes；C1 同样 0/5 string，因果仍不足 | 当前代码保留此候选，状态仍是未晋升候选 |
| C3 | 类型 0/5 string，但首次合法初始化降至 2/5；删除示例稳定引入其他结构错误 | 拒绝并回退 |
| C4 | 聚焦 Catalog 至少有 7 个 strict 不兼容点；机械改必填会改变产品语义 | 静态拒绝，未调用 Provider |

当前证据最重要的结论不是“已找到首发根因”，而是：

1. Runtime 类型反馈确有忠实性改进空间，但它只影响错误后的恢复。
2. 内联 schema 是最小、无观察回归的结构候选，但 5 次样本不足以证明它降低随机错误率。
3. 完整首次示例不是纯冗余，它在当前模型上明显帮助 Agent 生成“初始化并工作”的完整序列。
4. strict 不是可直接开启的开关；当前协议的可选语义与 DeepSeek strict 合同不兼容。

C2 扩大复验把累计类型样本推进到 object 15/15，但同时观察到 4/10 首次缺 work 和 1/10 顶层 client Tool 逃逸。
因此它强化了“内联表达值得保留”的证据，没有把 H-003 升级为已证实因果修复，也没有证明完整 Exec 行为已经稳定。

因此 H-003 继续保持 unverified，不把 `1/5 -> 0/5` 当作因果坐实。后续若继续验证 C2，应使用更大且预先批准的同版本样本，并把“类型正确率”和“完整合法序列率”同时作为门禁。

## 4. 预算结算

| Ledger scope | CNY |
|---|---:|
| C1 mixed batch + supplements | 0.07759788 |
| C2 | 0.08128852 |
| C3 | 0.09739480 |
| C4 | 0 |
| 原四候选总计 | 0.25628120 |
| 原四候选批准上限 | 5.00000000 |
| 原四候选使用率 | 5.13% |
| C2 后续 repeat=10（另行批准） | 0.13217348 |
| C2 repeat=10 批准上限 | 2.50000000 |
| 两批实际合计 | 0.38845468 |

没有自动重试。C4 因确定性离线不兼容停止，没有为已知无效合同消耗预算。

## 5. 证据索引

- Baseline: [`58-initialize-map-type-repeat5-result.md`](58-initialize-map-type-repeat5-result.md)
- C1: [`59-initialize-map-candidate1-feedback-result.md`](59-initialize-map-candidate1-feedback-result.md)
- C2: [`60-initialize-map-candidate2-inline-schema-result.md`](60-initialize-map-candidate2-inline-schema-result.md)
- C3: [`61-initialize-map-candidate3-no-first-turn-example-result.md`](61-initialize-map-candidate3-no-first-turn-example-result.md)
- C4: [`62-initialize-map-candidate4-strict-feasibility-result.md`](62-initialize-map-candidate4-strict-feasibility-result.md)
- C2 repeat=10: [`64-initialize-map-candidate2-repeat10-result.md`](64-initialize-map-candidate2-repeat10-result.md)
- COE: `coe/2026-08-15-23-15-initialize-map-json-string.md`
- Ledger: `benchmarks/whale-agent-run-ledger.json`
