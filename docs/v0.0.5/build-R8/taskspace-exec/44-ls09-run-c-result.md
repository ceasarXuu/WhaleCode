# LS-09 Run C 真实验收结果

- Date: 2026-08-13
- Subject: `b30de916d8519d181d2673205aa743da91801870`
- Run: `WAR-20260813-053410-CACHE-REGRESSION-93CFAC19`
- Matrix: `provider-web-search-probe × map-request × repeat=1`
- Result: **未通过，不重试**

## 1. 验收结论

本轮不能作为 LS-09 通过证据。Agent 最终生成了正确的业务文件，公开验证器和隐藏 oracle 均通过，修复后的
L2/L4 序列在 Map 初始化后也被正确使用；但 Agent 在初始化前连续试探 Provider-hosted Tool 的 Exec 声明方式，
消耗了 7 次失败请求。第 12 次 Provider 请求完成 Patch 后即触及预算上限，未再执行 Agent 自己规划的校验、Map
闭合和最终回复。因此正式结果是 `partial / business_failure`，不能因外部验证器通过而改写为成功。

这轮没有复现 Run B 的 Waiting 后继误选。现有证据支持“L2/L4 分支适用条件修复已进入真实 wire 且未回归”，但单次
未闭环运行不足以证明 I04 已稳定收敛。

## 2. 请求路径

| 阶段 | Provider 请求 | Agent 动作 | Runtime 结果 |
|---|---:|---|---|
| Hosted 合同试探 | 1～3 | 依次尝试 `node_id`、`input`、顶层 `queries` | Schema 正确拒绝未知或缺失字段，零副作用 |
| Hosted 事实对账试探 | 4～7 | 声明没有同响应事实的 `web_search`；遗漏同响应 `open_page`；client `web_fetch` 未绑定节点 | Hosted 数量或 client owner 预检正确拒绝，零副作用 |
| 初始化与检索 | 8 | 同响应执行新的 Provider `web_search`，并以 `initialize_and_work` 声明 Hosted 归属和 client `exec_command` | Map 初始化、Hosted 对账和 client 执行成功 |
| 完成并继续 | 9 | `update_and_work` 完成 `search`，继续 `write_file` | L4 选择正确，无 Waiting 拒绝 |
| 工作 | 10～11 | 两轮 `work` 执行本地检查与准备 | L2 选择正确 |
| 完成并继续 | 12 | `update_and_work` 提交 `apply_patch` | Patch 成功；随后预算截止，未进入校验和 `finish_end` |

Runtime 的硬拒绝语义正确且没有执行被拒绝的动作。问题不在 Runtime 应继续增加推理或纠错，而在 Agent 可见合同没有
完整说明 Provider-hosted action 的操作语义：

1. Hosted action 在 Exec 中是对**同一 Provider 响应内已执行 output item**的声明与节点归属，不携带查询参数。
2. 同响应每个 Hosted output item 都必须逐项声明，包括失败的 `open_page` 等 Provider action 变体。
3. Provider action 变体在对账时归属于公开 capability `web_search`，并按稳定 output 顺序逐项核对。

Schema 已表达 `tool + node_ids` 的结构，却没有表达上述使用方法。Agent 因而通过错误重试自行探明 Runtime 的实际合同。
该缺口继续归入 I03，不新增问题编号。

## 3. 成本与缓存

| 指标 | 实际值 |
|---|---:|
| Provider requests | 12 |
| Input tokens | 334,942 |
| Cached input | 291,584 |
| Uncached input | 43,358 |
| Output tokens | 15,517 |
| 全量缓存命中率 | 87.06% |
| Request 2+ 缓存命中率 | 89.05% |
| Agent wall time | 122.19 s |
| 日历耗时 | 145.27 s |
| 估算费用 | USD 0.0112313152 |

12 次请求的 Tool schema、`tool_choice` 和 wire shape 没有变化，也没有零命中请求；缓存证据不支持 schema 漂移回归。
Request 2+ 为 89.05%，低于 90% 观察线 0.95 个百分点，本轮失败和请求放大使其不能晋升为新缓存基线。

## 4. Map 与执行结果

- Map 为线性 `root -> search -> write_file -> validate -> finish`，共 5 节点、4 条边、无孤立节点。
- 截止时 `search=completed`，`root/write_file=in_flight`，`validate/finish=waiting`；根节点未闭合符合实际中断事实。
- 1 次 Patch 声明和 1 次 Patch 结果均被正确观测，无 Patch 预检拒绝或解析错误。
- `provider_fact.json` 已生成，公开验证器与隐藏 oracle 均返回 0；这证明业务产物正确，不替代 Agent 生命周期闭环。

## 5. 后续门禁

1. 先以 catalog 同源方式补全 Hosted action 的最小操作合同，不改变 Provider 执行、DAG、节点状态或拒绝规则。
2. 增加确定性测试，覆盖同响应多 Hosted output、失败 output、缺项、错序和 client/Hosted 混合序列。
3. 离线测试和缓存敏感面门禁通过后，再单独申请最小真实复验预算；本轮不得自动重试。

## 6. 证据

- 账本：`benchmarks/whale-agent-run-ledger.json`
- 结算：`benchmarks/cache-regression/results/WAR-20260813-053410-CACHE-REGRESSION-93CFAC19.json`
- Provider 证据：`benchmarks/cache-regression/evidence/WAR-20260813-053410-CACHE-REGRESSION-93CFAC19/`
- 本地运行目录：`target/r8-ls09/run-c/provider-web-search-probe/WAR-20260813-053410-CACHE-REGRESSION-93CFAC19-CACHE-001/`

另有一次启动器在创建 ledger/sample 前因相对 `run-root` 路径校验失败，记录为
`WAR-20260813-053343-CACHE-REGRESSION-E54EF25F` 的 preflight 证据；它产生 0 sample、0 Provider request、0 token，
不计入本次真实运行，也不视为重试。
