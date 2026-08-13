# LS-09 逻辑 Hosted Tool 修复复验结果

- Date: 2026-08-13
- Subject: `a73d73dc8ab6db49255e517307fbcf6cbe81efb6`
- Run: `WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE`
- Matrix: `provider-web-search-probe × map-request × repeat=1`
- Result: **逻辑聚合通过；端到端未通过；不重试**

## 1. 结论

本轮修复解决了用户指定的拆分错误：TaskSpace 不再把 `search`、`open_page`、`find_in_page` 等 Provider 内部步骤建模为多个
动作。真实响应中的 `search succeeded + open_page failed` 被 Runtime 识别为一个 `web_search` capability；另一响应中的
`find_in_page failed` 与 Agent 的一次 `web_search` 声明成功核对，并只返回一个逻辑 Hosted result。outer result 不再暴露
Provider ID、output index 或内部 action subtype。

但样本仍未完成。当前合同只接受“同一 Provider 响应内已经发生的 Hosted Tool + 同一响应内的 Exec 归属声明”。Agent 在
包含真实搜索结果的两次响应里都漏写了 `web_search` 归属；Runtime 当轮准确拒绝。Agent 下一请求再补声明时，当前响应已经
没有新的 Hosted 事实，Runtime 又准确拒绝。Agent 因而把响应级事实误解为“工具注册状态反复变化”，产生多轮无效纠正。

这不是内部 output 聚合回归，也不是 Runtime 丢失搜索结果。它暴露了一个更底层的协议断层：同响应声明一旦遗漏，现有合同
没有合法恢复路径。继续增加同义提示不能改变该时序事实；是否引入“待归属 Hosted 事实”的跨响应恢复边界，需要单独产品决策。

## 2. 请求路径

| Provider 请求 | Agent 主要动作 | Runtime / Tool 结果 |
|---:|---|---|
| 1 | 把 `web_search` 写成带 `node_id/input` 的 client action | Schema 拒绝；Map 未初始化 |
| 2 | 改用 `node_ids`，仍携带查询 `input` | Schema 拒绝未知字段；Map 未初始化 |
| 3 | 只声明 `web_search + node_ids`，但本响应未执行搜索 | `actual=[] / declared=[web_search]`，零副作用拒绝 |
| 4 | 用 `exec_command(ls)` 初始化 Map | 成功，建立 5 节点、4 边 |
| 5 | 顶层 `search` 成功、`open_page` 失败，随后 Exec 只声明 `web_fetch` | Runtime 将两个内部步骤聚合为 `actual=[web_search]`；因漏声明归属而整批拒绝 |
| 6 | 下一响应补声明 `web_search` | 当前响应无新 Hosted 事实，`actual=[]`，拒绝 |
| 7 | `curl` 抓取 | Client 正常执行，环境无 `curl` |
| 8 | Python `urllib` 抓取 | Client 正常执行，容器 DNS 受限 |
| 9 | 顶层 `find_in_page` 失败，并在同响应声明一次 `web_search` | 对账成功；只返回 1 个逻辑 Hosted result，outcome=`failed` |
| 10 | 顶层 `search` 成功，随后 Patch 未声明 `web_search` | `actual=[web_search]`，整批拒绝，Patch 未执行 |
| 11 | 下一响应补声明 `web_search` 并重放 Patch | `actual=[]`，整批拒绝，Patch 未执行 |
| 12 | 仅重放 Patch | 成功写入 `provider_fact.json`；请求上限在本地校验和 Map 闭合前耗尽 |

## 3. 验收项

| 验收项 | 结果 | 证据 |
|---|---|---|
| Provider 内部步骤不拆成 TaskSpace actions | PASS | Request 5 的 `search + open_page` 只形成 `actual=[web_search]` |
| 单次逻辑 Hosted 声明可进入 outer result | PASS | Request 9 只返回一个 `hosted_results[0]` |
| 内部 ID/index/subtype 不进入 Map 或 outer result | PASS | Request 9 的 result 只有 `tool_index/action_id/tool/outcome/node_ids` |
| Agent 能稳定在同响应声明 Hosted 归属 | FAIL | Requests 5、10 均漏声明，下一请求补声明又失去当前响应事实 |
| 业务文件、校验与 Map 完整闭环 | FAIL | 文件在 Request 12 写入，但未执行校验，Map 保持 active |

## 4. 成本与缓存

| 指标 | 实际值 |
|---|---:|
| Provider requests | 12 |
| Input tokens | 301,975 |
| Cached input | 264,064 |
| Uncached input | 37,911 |
| Output tokens | 10,772 |
| 全量缓存命中率 | 87.45% |
| Request 2+ 缓存命中率 | 89.71% |
| Agent wall time | 98.20 s |
| 日历耗时 | 129.17 s |
| 估算费用 | USD 0.0090630792 |

本轮 Tool capability identity 在 12 个请求中保持为
`18d7af7230501496c3a4011605f80ff00d8fb6e0cd32d73cc959174fb6665cf7`，`tool_choice` 没有切换；失败不是能力 schema
在请求间变化。缓存结果比上一失败轮改善，但业务失败路径不能晋升为正式缓存基线。

## 5. Map 与停点

- Map 为 `root -> search_docs -> write_fact -> validate -> finish`，共 5 节点、4 边，无孤立节点。
- 截止时 `search_docs=completed`、`write_fact=in_flight`、`validate/finish=waiting`、Root 保持 open。
- `provider_fact.json` 已写入，但公开校验未执行，隐藏 oracle 未通过，因此不得记为业务成功。
- I03 保持 `verifying`。逻辑 Hosted 聚合子问题已通过，剩余阻塞是同响应归属遗漏后的不可恢复性。
- 在明确“严格同响应且不提供恢复”与“保留待归属事实并由 Agent 后续显式认领”的产品边界前，不继续改 Runtime，也不申请新真实预算。

## 6. 证据

- 账本：`benchmarks/whale-agent-run-ledger.json`
- 结算：`benchmarks/cache-regression/results/WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE.json`
- Provider 证据：`benchmarks/cache-regression/evidence/WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE/`
- 本地运行目录：`target/r8-ls09/run-e/provider-web-search-probe/WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE-CACHE-001/`

另有一次启动器使用相对 `run-root`，在创建 ledger/sample 前因路径归一化失败。其 preflight 明确为
`operation=config_resolution_only`、`network_mode=none`，没有 Provider 请求或 token；诊断证据保留在
`benchmarks/cache-regression/evidence/WAR-20260813-085427-CACHE-REGRESSION-8686E8BA/`，不计入真实样本。
