# 完整响应工作存在性校验结果

- Date: 2026-08-15
- Scope: PA-08
- Result: **响应级机械校验离线正确；真实 Provider 生命周期未形成目标共现，行为验收未通过**

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

## 4. 真实复验

- Subject: `e4e7fc8748c2226d097ab556ad4988dd4f5c2d2b`
- Record: `WAR-20260815-003551-CACHE-REGRESSION-90733C69`
- Scope: `provider-web-search-probe × map-request × repeat=1`
- Result: business failed / Map 未闭合 / 缓存基线不晋升

| Requests | Input | Cached | Uncached | Output | Request 2+ cache | Elapsed | Estimated cost |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 12 | 314,069 | 271,744 | 42,325 | 7,859 | 88.98% | 94.505 s | USD 0.0088869032 |

真实 trace 中前三个 assistant response 都只生成了 `taskspace_exec(initialize_and_work)`，没有同响应原生
`web_search`，因此被新的响应级规则正确拒绝。第四个 response 才单独执行 `web_search`；随后 Agent 仍用无业务价值的
`pwd` 作为 client action 初始化 Map。下一请求又把纯 pending 归属写成 `type: work`，因既无本响应 Provider action、
也无 client action 再被拒绝。四次协议拒绝加上官方页面 `open_page`、容器 DNS 和 `web_fetch` 失败耗尽 12 请求，
`provider_fact.json` 尚未写入。

因此本轮证明的是：

1. 已实现的 OR 检查与其“单个 assistant response”边界一致，没有误放真正空响应；
2. 真实运行没有生成“Provider action 与 Exec 同响应共现”，所以该修复没有消除 Provider-first 占位行为；
3. 不能把离线正例扩大为产品收益，也不能据此晋升 final-wire/cache baseline；
4. 下一步必须先明确“完整请求”是单个 Provider response，还是包含 Hosted continuation 的完整 Agent turn。若选择后者，
   需要重新设计机械事实边界；不得在没有产品确认时把任意 pending 队列存在自动解释为本轮 work。

第一次启动在接触 Provider 和认领账本之前被旧 `~/.whale/bin/whale` 的 attestation 拒绝；随后从当前干净 HEAD 重建并以
`target/r8-pa08/bin/whale` 通过 `pass/valid` 预检。该预检失败没有产生 Provider 请求，也未消费 sample repeat。

## 5. 剩余门禁

本次 Tool schema/final-wire 仍是未接受候选。真实运行已结算但业务失败，不能生成 acceptance 或执行 baseline promotion；
PA-08 保持 `partial`。任何新真实运行都需要新的独立预算，当前授权已由上述单次 sample 完整消费。
