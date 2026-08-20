# Hosted 原生调用合同纠正结果

- Date: 2026-08-14
- Subject: `b77663e438bf54d60e729f6f9eb8495537426fcf`
- Model: `deepseek-v4-flash`
- Scope: `provider-web-search-probe × map-request × repeat=1`
- Result: **明确误导已修复；业务通过；首次双写仍漏登，I03 继续 verifying**

## 1. 修复内容

上一阶段错误地要求 Agent “emit native top-level Provider Tool item”，并在首轮
`initialize_and_work` 示例中预填 `web_search + already_executed`。但真实 final wire 中
`web_search` 是原生 `{"type":"web_search"}` ToolSpec，不是 Function Tool，也不存在可由 Agent 构造的
`FunctionCall(name="web_search", queries=...)` schema。

本次删除首轮 Hosted 预填示例和所有“主动 emit 原生 item”的文字，并在协议与 Hosted variant 中明确：

1. `web_search` 保持 Provider 原生 ToolSpec，不能被构造或仿造成 Function Call；
2. `already_executed` 不调用、不请求、不触发 Tool，只记录当前响应中已有原生结果的 Work-node 归属；
3. 没有当前原生结果时不得提前登记，也不得跨响应补登。

Runtime、同响应对账、Map、序列和 Provider Web Search schema 均未改动。TaskSpace Exec 聚焦测试 75/75 通过；
Standard final wire 不变，缓存敏感差异仅为 `taskspace_exec` description。

## 2. 真实运行

| Runs | Requests | Input | Cached | Uncached | Output | Agent wall | 估算费用 | 业务 |
|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | 7 | 191,823 | 156,800 | 35,023 | 6,247 | 56.803 s | USD 0.00709142 | PASS |

- request 2+ cache hit: 84.31%；usage 覆盖 100%；无重试；
- `provider_fact.json` 生成，公开验证和隐藏 oracle 均通过；
- Map 为 5 节点、4 条边、无开放叶节点，最终显式闭合；
- 6 次 `taskspace_exec` 中一次按硬合同拒绝，其余执行成功，普通 client Tool 没有顶层逃逸。

## 3. 请求路径

第一响应包含两个原生 `web_search_call` 内部 item：一次 `search` 成功、一次 `open_page` 失败；它们属于同一个
原生 `web_search` ToolSpec，只形成一个逻辑 Hosted 事实。Agent 同响应生成了 `taskspace_exec`，但其中只有
`web_fetch + exec_command`，没有 `web_search + already_executed`。Runtime 返回 Hosted 漏登事实并保持 Map/client
零副作用。

第二响应再次产生一个原生 `web_search_call(search)`，并在同一响应的 `taskspace_exec.tools[]` 中正确写入：

```json
{"tool":"web_search","execution":"already_executed","node_ids":["search"]}
```

该响应完成 Map 初始化、一个失败的 `web_fetch`、一个成功的 `exec_command`，并返回一条成功 Hosted 归属。后续请求
依次读取本地要求、Patch 文件、运行校验、完成节点并显式结束 Map。

全程没有再出现上一阶段错误的顶层 `FunctionCall(name="web_search")`。原生结果始终是
`web_search_call`，TaskSpace 登记始终只是归属记录。

## 4. 判定

本次单变量修复消除了已坐实的错误诱因，并在真实运行中消除了对应错误调用形态；这不是随机业务成功替代协议验收。
但第一响应仍发生一次“原生结果已出现、Exec 漏写归属”，说明现有 Agent 可见合同尚不能稳定保证首次同响应双写。
第二响应在事实反馈后能够正确完成双写，证明协议可以被执行，不证明首次遵循已经稳定。

因此：

- 明确的 Function Call 误导修复通过；
- I03 保持 `verifying`；
- 当前缓存候选不晋升 accepted baseline；
- 不新增 Runtime 自动绑定、跨响应 pending、默认 Root 或 Web Search 内部 action 映射。

## 5. 证据

- Result: `benchmarks/cache-regression/results/WAR-20260814-205255-CACHE-REGRESSION-B97F06E2.json`
- Evidence: `benchmarks/cache-regression/evidence/WAR-20260814-205255-CACHE-REGRESSION-B97F06E2/`
- Local trace: `target/r8-hosted-contract-fix/run/`
- Proposal: `benchmarks/cache-regression/proposals/CBP-8C7CDEEF5CAEC7B8.json`
- Authorization: `benchmarks/cache-regression/authorizations/CBA-20260814-R8-HOSTED-CONTRACT-FIX-8C7CDEEF5CAEC7B8.json`
