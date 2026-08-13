# LS-09 Hosted 合同修复复验结果

- Date: 2026-08-13
- Subject: `3c464653f3c03dcdb0c916b625a74881fc20061e`
- Run: `WAR-20260813-061928-CACHE-REGRESSION-04087B3B`
- Matrix: `provider-web-search-probe × map-request × repeat=1`
- Result: **未通过，不重试**

## 1. 结论

Run C 后补充的 Hosted action 文本合同已经进入真实 Provider wire，但没有解决目标行为。Agent 仍在前两次
`taskspace_exec` 中分别使用了 client action 的 `node_id` 和 `input` 形状，随后又把 Exec 内的 Hosted action 当成
Provider Tool 的执行请求。它只有在先执行顶层 `web_search`、再在同一响应内登记 Hosted output 时才通过预检。

因此，Run C 的“合同文字缺失”只是表层缺口。更深的已证实问题是：当前统一 `tools[]` 把两种不同操作放在同一个
`Tool action` 槽位中：

1. client action 是带原生输入、等待 Runtime 执行的请求；
2. Hosted action 是不带原生输入、对同响应已执行 output 的事后归属声明。

两者外观相近、生命周期相反。Agent 按 client action 的显著模式理解 Hosted action，并非 Runtime 丢失或扭曲了 Tool
结果。继续堆叠同义警告不能证明可以收敛该冲突，当前修复不得标记为成功。

## 2. 请求路径

| Provider 请求 | Agent 主要动作 | Runtime / Tool 结果 |
|---:|---|---|
| 1 | `initialize_and_work` 内把 `web_search` 写成 `node_id` | Schema 拒绝：Hosted action 必须使用 `node_ids`；Map 未初始化 |
| 2 | 增加 `node_ids`，同时携带查询 `input` | Schema 拒绝：Hosted action 不接受原生输入 |
| 3 | 使用正确 Hosted 形状，但同响应没有 Hosted output | 对账拒绝：声明 1 项、实际 0 项 |
| 4 | 顶层执行 3 项 `web_search`，同响应提交 `update_and_work` | Map 尚未初始化，完整预检拒绝 |
| 5 | 顶层执行 `web_search`，同响应 `initialize_and_work` 登记 1 项并调用 `web_fetch` | Map 初始化与 Hosted 对账成功；`web_fetch` 获取失败 |
| 6 | Provider 返回失败的 `open_page`，Exec 只声明 client `web_fetch` | 对账拒绝：实际 Hosted output 1 项、声明 0 项 |
| 7 | 没有新 Hosted output，却补写上一轮 Hosted 声明 | 对账拒绝：实际 0 项、声明 1 项 |
| 8 | 新顶层 `web_search`，同响应登记 Hosted output 并调用 `web_fetch` | 对账成功；`web_fetch` 获取失败 |
| 9～12 | 依次尝试 `curl`、Python `urllib`、`web_fetch + ls`、读取本地说明 | 本地网络或 Tool 失败后继续排查；第 12 次触及请求上限，未写业务文件 |

Runtime 的 schema、Map 初始化和 Hosted 同响应核对均在副作用前按既定硬规则执行。失败反馈没有把“已执行”改写成
“可重试”，也没有替 Agent 选择节点或行动。

## 3. 结果、成本与缓存

| 指标 | 实际值 |
|---|---:|
| 业务结果 | 失败；`provider_fact.json` 未生成 |
| Provider requests | 12 |
| Input tokens | 367,309 |
| Cached input | 318,976 |
| Uncached input | 48,333 |
| Output tokens | 13,674 |
| 全量缓存命中率 | 86.84% |
| Request 2+ 缓存命中率 | 88.67% |
| Agent wall time | 115.05 s |
| 日历耗时 | 140.00 s |
| 估算费用 | USD 0.0114884728 |

12 次 Provider 请求的 TaskSpace capability identity 均为
`3710c4d67c5ee996fa3965d9609766224d3d40b4c024e9e57d75173993ec9ed2`；新 Hosted 合同确实进入了最终声明，
因此不能把失败归因于构建未生效。没有零缓存命中或 `tool_choice` 切换；本轮失败与请求放大不适合作为缓存基线。

## 4. Map 与执行状态

- Map 共 5 个节点、4 条边：`root -> search -> write -> validate -> finish`，无孤立节点。
- 截止时 `root/search=in_flight`，其余节点 `waiting`，Map 保持 active。
- Agent 未产生 Patch，公开验证因缺少 `provider_fact.json` 失败；隐藏 oracle 返回 0 不代表业务完成。
- 观测到 12 次 outer `taskspace_exec`、23 个嵌套动作、10 个失败动作，其中 7 个 Provider action 只有 2 个完成对账。

## 5. 根因边界与停点

已坐实：

1. 最终 Provider-visible schema 已包含补充合同；不是协议没有送达。
2. Agent 仍稳定套用 client action 的结构和执行心智；不是一次随机字段拼写错误。
3. 当前同一个 `tools[]` action union 同时表达执行前请求和执行后凭据，语义角色不统一。

后续用户已明确产品边界：`web_search` 是一个不可拆分的逻辑 Tool，`search/open_page` 等均为 Provider 内部过程，Agent
和 Runtime 都不得把它们拆成多个 TaskSpace action。逐 output item 模型因此被废弃，不再作为候选。修复按逻辑 capability
一次声明、一次绑定、一次结果推进；本文件仍保留为旧模型失败证据。I03 保持 `verifying`，LS-09 保持未通过，等待修复复验。

## 6. 证据

- 账本：`benchmarks/whale-agent-run-ledger.json`
- 结算：`benchmarks/cache-regression/results/WAR-20260813-061928-CACHE-REGRESSION-04087B3B.json`
- Provider 证据：`benchmarks/cache-regression/evidence/WAR-20260813-061928-CACHE-REGRESSION-04087B3B/`
- 本地运行目录：`target/r8-ls09/run-d/provider-web-search-probe/WAR-20260813-061928-CACHE-REGRESSION-04087B3B-CACHE-001/`

另有一次启动器在创建 ledger/sample 前因二进制 attestation 仍指向上一提交而退出，证据目录为
`WAR-20260813-061842-CACHE-REGRESSION-4A529F01`。该尝试产生 0 sample、0 Provider request、0 token，不计入复验。
