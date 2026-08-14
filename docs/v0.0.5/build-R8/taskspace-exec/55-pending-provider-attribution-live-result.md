# Provider 待归属队列真实验收结果

- Date: 2026-08-14
- Subject: `5e99cb63fe33e1154b7959e082c166c723fdf42c`
- Scope: `provider-web-search-probe × map-request × repeat=3`
- Model: `deepseek-v4-flash`
- Result: **归属机制 3/3 通过；端到端业务 2/3 通过；PA-07 部分通过**

## 1. 结果总表

| Repeat | Business | Requests | Input | Cached | Uncached | Output | Request 2+ cache | Elapsed | Provider attributions | Pending at end | Map |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | PASS | 11 | 328,639 | 289,280 | 39,359 | 7,494 | 90.33% | 94.619 s | 2 | 0 | closed |
| 2 | PASS | 11 | 250,250 | 228,096 | 22,154 | 5,224 | 90.71% | 68.529 s | 1 | 0 | closed |
| 3 | FAIL | 12 | 310,981 | 278,400 | 32,581 | 7,208 | 89.11% | 92.765 s | 3 | 0 | open |
| **Total** | **2/3** | **34** | **889,870** | **795,776** | **94,094** | **19,926** | **90.05% mean** | **255.913 s** | **6** | **0** | **2/3 closed** |
| **Mean** | - | **11.33** | **296,623** | **265,259** | **31,365** | **6,642** | **90.05%** | **85.304 s** | **2.0** | **0** | - |

账本估算费用为 `USD 0.0209806128`。34 个 Provider 请求均有 usage 和 Provider boundary 证据，没有预算越界或自动重试。

## 2. 归属机制结论

三轮均完成同一条新链路：

1. 原生 `web_search` 先由 Provider 执行，Runtime 按原生 response 边界持久化逻辑 Action；
2. 下一请求向 Agent 暴露稳定 `action_id/tool/outcome`；
3. Agent 在 `assign_pending_actions` 中选择一个 Work node；
4. Runtime 在同一 SQLite CAS 事务中把 Action 写入 Node 并从 pending 表删除。

三轮共归属 6 个 Provider Action，包括成功和失败 outcome。没有同响应双写、提前登记、未知 ID、遗漏 ID、错绑、默认 Root
或未归属结束。第 3 轮结束后的 SQLite 事实为 `pending_count=0`；三个 `web_search` Action 分别进入 Agent 声明的
`search`、`verify` Work node。因此待归属队列解决了原双写机制的结构性脆弱问题。

## 3. 第三轮为什么失败

第三轮在第 12 个请求后写入了 `provider_fact.json`，但尚未运行本地校验和闭合 Map，故业务验收失败。请求放大来自：

- 首次 `initialize_and_work` 缺少必填 `tools[]`，零副作用拒绝一次；
- 一次提前选择 Waiting 后继节点，零副作用拒绝一次；
- 官方页面 `open_page`、容器 DNS、`web_fetch` 等读取路径连续失败，Agent 多次换路径验证内容；
- 最后一个 Provider Action 已成功归属，队列没有残留，失败与 pending 机制无关。

当前闭集没有直接表达“初始化 Map，同时在同一响应发起 Provider 原生 Tool”的形状。前两轮通过在
`initialize_and_work` 中执行无业务价值的 `pwd` 绕开该缺口；第三轮首次没有添加占位 client Tool，因此被 schema 拒绝。
是否新增该合法初始化形状涉及产品动作模型，需要用户确认，不能由 Runtime 自动插入占位动作。

## 4. 验收后发现并修复的工程问题

真实 trace 证明 TaskSpace base instructions 仍残留“Provider 调用与归属必须同响应双写”的旧文字，与新 Tool
schema 冲突。提交 `ce69cbb13` 已删除旧规则，改为 Runtime 后续暴露 pending 后再由 Agent 归属，并新增防回归断言；免费
final-wire 门禁通过。该提示词修复发生在本轮 Subject 之后，尚无新的真实运行证据。

第三轮一个 9 KB `taskspace_exec` 结果按 Standard 渐进暴露规则变为 `OutputReferenceV1`。旧性能观察器把引用文本误报为
非法 Exec 结果。提交 `4584ad05d` 已按 `artifact_ref` 读取既有冷存储原文；自测通过，第三轮重新生成报告后恢复为可比较。

## 5. 状态与证据

- PA-00～PA-06：verified；
- PA-07 Provider 归属稳定性：verified（3/3）；
- PA-07 端到端三轮闭合：未达成（2/3）；
- 缓存基线：不晋升；当前变更继续保持 candidate/release blocked；
- 后续停点：确认 Provider-first 初始化的合法序列后，才申请最小真实复验预算。

证据：

- Result: `benchmarks/cache-regression/results/WAR-20260814-225157-CACHE-REGRESSION-3F802FA5.json`
- Ledger: `benchmarks/whale-agent-run-ledger.json`
- Evidence: `benchmarks/cache-regression/evidence/WAR-20260814-225157-CACHE-REGRESSION-3F802FA5/`
- Proposal: `benchmarks/cache-regression/proposals/CBP-852C65F588281F07.json`
- Authorization: `benchmarks/cache-regression/authorizations/CBA-20260814-R8-PENDING-PROVIDER-ATTRIBUTION-852C65F588281F07.json`
