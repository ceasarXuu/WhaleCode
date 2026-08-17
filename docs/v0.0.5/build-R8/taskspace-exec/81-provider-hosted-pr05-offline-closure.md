# Provider-hosted PR-05 离线收口

- Date: 2026-08-18
- Status: verified offline；production evidence pending
- Runtime change: none
- Paid run: none

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

## 3. 结论与停点

PR-05 的离线收口完成，当前没有证据支持继续修改 Runtime、Tool schema 或提示词。唯一剩余验收是一次真实
Provider-hosted Tool 命中，确认原生 `web_search` 事实能在当前生产链创建 Root 聚合节点，且不提前创建空节点、不重复、
不影响业务和 Map 闭合。该运行需要单独预算批准。

