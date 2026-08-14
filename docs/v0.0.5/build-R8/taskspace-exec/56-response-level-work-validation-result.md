# 完整响应工作存在性校验结果

- Date: 2026-08-15
- Scope: PA-08
- Result: **Provider-first 初始化结构缺口已离线修复；真实行为与成本待复验**

## 1. 回归根因

PA-04 从 `tools[]` 删除旧 Hosted 双写项后，`initialize_and_work` 等工作型序列仍把非空 client `tools[]` 当作局部必要条件。
Provider 原生调用已经是本响应中的实际工作，却无法满足该字段；Agent 只能添加 `pwd` 等无业务价值的 client Tool 占位。
该问题由提交 `a3305e6ab` 的协议迁移暴露，不是待归属队列必须接受的产品限制。

## 2. 实施边界

1. `initialize_and_work`、`work`、`update_and_work`、`reopen_update_and_work` 的 client `tools[]` 改为可选且可为空。
2. 既有 `TaskSpaceExecResponseScope` 在响应完成时把“当前响应是否存在原生 Provider Tool action”交给统一 preflight。
3. preflight 只做机械 OR：当前响应有 Provider work，或 Exec 内有 client work，任一成立即满足工作型序列；两者都没有则在
   Map/client 副作用前拒绝。
4. Provider Action 仍在响应完成后进入 pending Store，下一请求由 Agent 选择节点；本次改动不恢复同响应双写、不自动绑定、
   不默认 Root，也不根据 Tool outcome 改变节点状态。

## 3. 离线证据

| Suite | Result |
|---|---:|
| `cargo test -p codex-core taskspace_exec --lib --locked` | 77 passed |
| `cargo test -p codex-state taskspace --lib --locked` | 19 passed |
| `cargo test -p codex-core --test all cache_final_wire --locked` | 2 passed |

新增覆盖包括：

- Provider-only `initialize_and_work` 可通过 schema、decode 和 preflight，并初始化 Map；
- 相同序列在 Provider/client 均为空时返回 `ResponseWorkMissing`；
- ResponseScope 的 Provider fact 确实进入 Exec claim；
- 生产 handler 链无需占位 client Tool 即可接受 Provider-first 初始化，且不会调用 client handler；
- 原有 client work、pending 精确集合、单 Patch、DAG、Router 和持久化链全部继续通过。

final-wire 首次运行还发现目标快照仍停留在 PA-04 之前的同响应双写合同。只更新
`taskspace_production_tool_wire` 这一张目标快照后，Standard/TaskSpace 两项均通过；目录内另外两张无关 `.snap.new` 未接受。

完整 `codex-core --lib` 共 1889 项，其中 1873 passed、3 ignored、13 failed。失败集中于 Guardian 缺少
`DEEPSEEK_API_KEY` 和旧 projection fixture 未配置持久化 State DB，不经过本次修改路径；为避免真实 API 调用，本轮没有加载
Key 迎合测试。聚焦生产链、State 和 final-wire 均已通过。

## 4. 剩余门禁

本次修改改变 TaskSpace Tool schema 和协议描述，属于缓存敏感面。提交前需执行缓存源代码门禁；真实 Provider
`provider-web-search-probe` 复验必须按全局预算规则另行登记和批准。未执行真实复验前，不宣称请求数、稳定性或成本收益。
