# Provider 待归属队列离线实施结果

- Date: 2026-08-14
- Scope: PA-00～PA-06
- Result: 离线实现与生产链测试通过；PA-07 真实结果见 [`55-pending-provider-attribution-live-result.md`](55-pending-provider-attribution-live-result.md)

## 实施结果

1. Provider 原生 Action 在响应完成后独立持久化，不再依赖同响应 `taskspace_exec` 双写。
2. 同一 Provider response 内同一原生 ToolSpec 的内部 search/open 等 item 机械聚合为一个逻辑 Action；不同 response
   或不同 ToolSpec 不合并。
3. 下一请求尾部只暴露稳定的 `action_id/tool/outcome`，不复制 Tool input/output。
4. `taskspace_exec.tools[]` 只包含 client Tool。Provider 归属使用
   `assign_pending_actions[{action_id,node_ids}]`，支持与初始化、更新、工作、结束组合，也支持有证据的纯归属与
   初始化后归属序列。
5. 除 `read_map` 外，Runtime 在副作用前要求完整覆盖请求开始时可见的 pending 集合；未知、重复、遗漏和非 Work
   节点全部拒绝。Agent 独立选择一个或多个 owner，Runtime 不自动绑定或默认 Root。
6. Provider Action 可归属 Ready、InFlight 或 Completed Work node；归属本身不改变 Node state。
7. Node action 写入和 pending 删除进入同一个 SQLite Map CAS 事务。请求处理中刚产生的新 Provider Action 不会污染本请求
   已冻结的 pending 集合，而是保留到下一请求。
8. 请求可见 pending 非空时，未调用 Exec 的最终响应被硬门拒绝；`read_map` 可暂时保留 pending，但后续请求仍受同一门禁。

## 离线证据

| Suite | Result |
|---|---:|
| `cargo test -p codex-core taskspace_exec` | 74 passed |
| `cargo test -p codex-state taskspace` | 19 passed |
| `cargo test -p codex-core cache_final_wire` | 2 passed |

覆盖持久化/重启、幂等采集、内部 item 聚合、动态事实暴露、精确集合、重复/错误 ID、多节点、Completed node、只读例外、
Map/出队原子性、失败回滚和生产 Router 归属链。历史同响应双写文档仅保留为失败证据，不再构成活动合同。
