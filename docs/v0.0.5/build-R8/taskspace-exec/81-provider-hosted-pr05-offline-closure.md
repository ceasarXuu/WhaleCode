# Provider-hosted PR-05 离线收口

- Date: 2026-08-18
- Status: verified in production
- Runtime change: none
- Paid run: `WAR-20260818-040158-CACHE-REGRESSION-3E3BBA3B`

## 1. 当前产品边界

Provider-hosted Tool 不再要求 Agent 双写或延迟归属。Runtime 只在原生 Hosted 动作实际完成后，按原生 Tool 名在 Root 下
创建或复用 Completed 聚合节点，并追加机械 Action：

- 不复制 input/output，不改变 Agent 工作节点状态；
- 同一响应同种 Tool 聚合一次，后续响应追加到同名节点；
- 同响应 Map 初始化和 client 工作先 drain，再记录 Provider 动作；
- 无 Map、Map 已切换或同名 Agent 节点冲突时不覆盖、不排队、不阻塞，只记录
  `taskspace.provider_actions_escaped`；
- Provider 动作不参与 `taskspace_exec` 合法序列和 client work 非空判断。

## 2. 离线证据

| 门禁 | 结果 |
|---|---:|
| TaskSpace Exec focused suite | 77 passed |
| Provider DAG 聚合 | 4 passed |
| Session 持久化聚合 | 1 passed |
| `codex-state` TaskSpace | 16 passed |
| TaskSpace observer | passed |
| zero-base gate | passed |
| cache regression gate | passed |

覆盖场景包括同响应多次 Web Search 聚合、失败 outcome、无 Exec 的 Hosted response、可恢复 client 旁路同时保留 Hosted
事实、首次创建、跨响应追加、已闭合 Map、Finish 连接去重和同名 Agent 节点冲突。旧 `hosted_bindings`、
`already_executed`、`assign_pending_actions` 和 pending Store 仅保留在负向防回归测试或历史文档中，没有活动生产消费者。

## 3. 生产证据

获批的 `provider-web-search-probe × map-request × repeat=1` 已在生产 Provider 路径完成：

| 项目 | 结果 |
|---|---:|
| 业务 / 公开验证 / Map 闭合 | passed |
| Provider requests | 10 |
| Input / cached / uncached / output | 267,288 / 239,232 / 28,056 / 5,225 |
| Request 2+ cache hit | 89.03% |
| 运行时长 | 89.613s |
| 冻结价格估算 | CNY 0.04329064 |
| Provider usage 结算 | 10/10，零缺失、零重试、未触及预算 |

持久化 Map 证据：

- Runtime 在真实 Hosted 动作发生后提交 3 次 `record_provider_actions`，没有提前创建节点；
- 四个 Provider 内部 `web_search_call` item 按响应粒度聚合为三条逻辑 `web_search` Action，符合“同响应同原生 Tool
  只记录一次”的产品合同；
- Map 中恰有一个 `web_search` 节点，父节点为 `root`，状态为 `completed`，三条 Action 的 outcome 为
  `failed/succeeded/succeeded`；
- 未创建 `image_generation` 等未使用的空节点；
- `finish.parents` 包含 `web_search`，最终 Map 无开放叶节点。

完整结果位于
`benchmarks/cache-regression/results/WAR-20260818-040158-CACHE-REGRESSION-3E3BBA3B.json`，Map 事实位于该结果指向的
`taskspace-map-store.json`。

## 4. 结论

PR-05 的离线与生产验收均完成。原生 Provider-hosted Tool 能由 Runtime 机械归纳到 Root 专用节点，不要求 Agent 双写，
不进入 `taskspace_exec` 合法序列，也不改变 Agent 工作节点状态。当前没有证据支持继续修改 Runtime、Tool schema 或提示词；
后续只在新增 Hosted Tool 类型或真实 trace 出现 escape/冲突时重新打开该边界。
