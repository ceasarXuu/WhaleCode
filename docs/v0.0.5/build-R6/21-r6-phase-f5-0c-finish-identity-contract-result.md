# R6 Phase F5.0c Finish Identity 生产合同切换结果

- Date: 2026-07-17
- Status: Complete
- Scope: F5.0c only
- Branch: `whalecode-alpha`
- Production commits: `ab6e2957a`、`2117de6ec`
- Diagnostic commit: `4a30d9d8b`
- Next phase: F5.1 Pending

## 1. 结论

F5.0c 已完成。model-visible 初始化合同已从 `finish: { node_id }` 无兼容分支地切换为
`finish_identity: { id }`；canonical domain 内部的 `finish_node_id` 保持不变。生产 schema、严格 typed parser、
mapping、terminal、event/replay、observer 和 benchmark fixture 已统一。

第一次只切字段的 complex rollout 暴露了 H-013：Agent 能正确生成新 identity，却没有被工具合同告知该 ID 必须
成为 `edges` 的唯一汇点。Runtime 按现有图不变量忠实拒绝，没有补边或解释 Agent 意图。修复只补全同一工具
schema 的机械拓扑说明；没有改 Runtime、projection、系统提示词或 reasoning 路径。

最终 Docker simple/complex 各运行一次，均在第一次初始化提交，业务与外部验证 2/2 通过，Root/Finish 2/2
显式闭合。F5.0c 的验收项为 8/8，完成度 100%。本阶段不评价 F5.1 的动态工具面收益。

## 2. 实施边界

| 层 | 变更 | 明确未做 |
|---|---|---|
| Tool schema | `finish_identity.id`；声明其为 `edges` 唯一汇点，所有节点必须可达 | 不注入任务策略或 next action |
| Typed parser | 严格接受新字段；旧外层 `finish` 和旧内层 `node_id` 均拒绝 | 不保留 alias、fallback 或迁移分支 |
| Mapping/domain | 新 wire identity 映射到既有 canonical `finish_node_id` | 不改变 Rooted DAG 领域模型 |
| Feedback/log | 继续使用 `serde_path_to_error` 和 `taskspace.control_arguments_rejected` | 不纠正参数，不替 Agent 重试 |
| Projection/prompt | 无变更 | 不增加拓扑提示副本 |

## 3. 第一次 Live 暴露的问题

| Sample | 表现 | 根因证据 | 处理 |
|---|---|---|---|
| simple | 第一次初始化成功 | identity 和 Finish 入边均正确 | 无额外处理 |
| complex | 前四次 identity 正确但 Finish 无入边；第五次补边后提交 | F4 旧合同 10/10 初始化都含 Finish 入边，新字段失去旧名称隐含的拓扑关联 | 只补全工具 schema 的机械拓扑合同 |

前四次拒绝中的主要 violation 为 `non_root_zero_indegree: finish`、`node_unreachable_from_root: finish` 和
`finish_unreachable_from_node`。第二次还出现 `verify -> root` cycle。反馈没有丢失、扭曲或由 Runtime 增加
主观解释；问题发生在工具能力语义暴露不完整。

## 4. 确定性验证

| 验证 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo test -p codex-tools taskspace --lib` | 4 passed |
| `cargo test -p codex-core taskspace_control --lib` | 26 passed |
| `cargo test -p codex-core sequence --lib` | 20 passed |
| `cargo test -p codex-core event_store --lib` | 16 passed |
| `cargo test -p codex-core --test all taskspace_terminal_contract` | 2 passed |
| `scripts/taskspace-benchmark/test-r6-rooted-dag-contract.ps1` | Pass |
| `scripts/taskspace-benchmark/test-performance-observation.ps1` | Pass |
| `scripts/taskspace-benchmark/test-r6-f5-bootstrap-contract.ps1` | Pass |
| `scripts/taskspace-benchmark/test-r6-f5-finish-identity.ps1` | Pass |
| `cargo build -p codex-cli --bin whale --locked` | Pass |
| `git diff --check` | Pass |

最终构建 attestation 状态为 pass，commit 为 `2117de6ecac76640d89e7d1b4f9a626fbd6ad25b`，binary SHA-256 为
`07043e7d79823b1f04a5d8d36f6d67b5f9c75d576a4e448d53abc738af6563c1`。

## 5. 最终 Docker 样本

| Metric | simple | complex |
|---|---:|---:|
| Result / external | solved / passed | solved / passed |
| 首次 initialize 提交 | 1/1 | 1/1 |
| 合法 `finish_identity` | 1/1 | 1/1 |
| legacy `finish` | 0 | 0 |
| 首次 Finish 入边 | 1/1 | 1/1 |
| Root 到 Finish 全节点可达 | Pass | Pass |
| continuation 执行 | 1/1 | 1/1 |
| Root / Finish 闭合 | Pass / Pass | Pass / Pass |
| Requests | 13 | 16 |
| Runtime tools | 12 | 21 |
| Nested actions | 3 | 5 |
| Wall | 27.87s | 65.89s |
| Input tokens | 118,282 | 229,372 |
| Cached / uncached | 98,304 / 19,978 | 200,576 / 28,796 |
| Full cache hit | 83.11% | 87.45% |
| Request 2+ hit | 84.26% | 88.12% |
| Map nodes / edges / open | 5 / 4 / 0 | 5 / 4 / 0 |
| Semantic retention / replacement | 100% / 0% | 100% / 0% |

Artifacts：

- simple: `target/r6-phase-f5-0c-final/single-file-fast-fix/20260717-160531-941`
- complex: `target/r6-phase-f5-0c-final/subscription-billing-repair/20260717-160531-953`

## 6. 验收门

| 验收项 | 证据 | 状态 |
|---|---|---|
| schema/parser 正负 fixture | 新字段通过，旧字段严格拒绝 | Complete |
| terminal/replay/malformed feedback | deterministic suites 全通过 | Complete |
| 首次初始化错误为 0/2 | final Docker trace | Complete |
| identity 正确且无兼容字段 | 2/2 新字段，0/2 旧字段 | Complete |
| Rooted DAG 拓扑正确 | 2/2 首次 Finish 入边，所有节点可达 | Complete |
| continuation 实际执行 | 2/2 | Complete |
| Root/Finish 显式闭合 | 2/2 | Complete |
| 业务与外部验证 | 2/2 | Complete |

## 7. Schema 成本

| 合同 | Full-lifecycle bytes | 相对旧 D |
|---|---:|---:|
| 旧 D `finish: { node_id }` | 9,427 | baseline |
| field-only E `finish_identity: { id }` | 9,436 | +9 |
| topology-complete E | 9,527 | +100 |

拓扑合同相对 field-only E 增加 91 B，约为每 request 23 个输入 token 的量级估算，并非 provider 实测 token。
这部分是完整机械合同的固定成本；F5.1 将独立验证 hard-state 对齐工具面能否消除更大的 schema 固定成本。

## 8. 非阻断观察

1. complex 初始化 continuation 的 `send_message` 使用了错误参数形态。普通工具失败被忠实返回，Map 仍保持已提交
   状态，Agent 后续自行纠正并完成任务；这不是 F5.0c control feedback 缺陷。
2. simple 出现一次无 current binding 的 ordinary action hard-state reject；complex 出现一次尝试 bind Finish 后被拒绝。
   两者均不影响 F5.0c identity 验收，留给 F5.1 动态工具面继续观察。
3. 两次最初 harness 预检因 shell 进程没有导出 `DEEPSEEK_API_KEY` 而退出，属于 `invalid_harness`，未发 provider
   request，已从结果排除。Docker benchmark runner 读取进程环境，不会自动 source `.env.local`；本地运行前必须
   使用 `set -a; source .env.local; set +a`，且不得打印 secret。

## 9. 阶段结论

F5.0c 关闭，H-012 的生产修复和 H-013 的拓扑合同修复均已验证。按计划在此暂停；F5.1 尚未开始，Phase G
继续 blocked。
