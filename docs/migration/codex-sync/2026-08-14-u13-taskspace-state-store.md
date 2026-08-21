# U13：TaskSpace 唯一 state store、CAS 与 replay

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 真实模型请求：0
- 真实用户数据库读写：0

## 1. 实施边界

U13 只把 U12 canonical kernel 接到现有 `StateRuntime` 的 state SQLite pool。没有创建 TaskSpace 专用数据库，没有接入 session、provider、tool router、AgentGraphStore 或 WorldState，也没有恢复旧 `core/action_map` runtime。

新 migration `0047` 使用 `CREATE ... IF NOT EXISTS`：fresh/current 0.147 数据库创建 canonical map、thread binding 和 commit 表；经 U11 修复的旧 Whale 数据库原地复用已有三张同 schema 表，不复制数据或形成第二权威。

## 2. 实现结果

- `codex-state` 直接消费 U12 的 `TaskSpaceMap`、canonicalize 与 invariant validation；
- 写入前 canonicalize 并校验 schema、map identity、revision、terminal 和 SHA-256；读取时再次 fail-closed 校验；
- `expected_store_revision=0` 原子创建 Map 与 owner binding，后续 CAS 同时要求 store revision 命中、owner 不变且 domain map revision 单调递增；
- `BEGIN IMMEDIATE` 保证并发写入只有一个 winner；
- commit ID + request hash 提供幂等 replay，同 ID 不同输入明确拒绝；
- thread binding 只能绑定到同一 Map，Owner 关系只能属于 map owner，禁止静默改绑或冒充 owner；
- 不保留旧“空 `null` Map”激活状态，新 Map 必须是 U12 合法 canonical map。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| fresh migration/store | passed；`0047` 创建同一 state DB 内三张表 |
| canonical event replay → store → load | passed；Map 逐字段一致 |
| owner/child thread binding | passed；owner immutable，Owner relation reserved |
| commit idempotency / key reuse | replay passed；不同输入 fail-closed |
| concurrent CAS | 1 Applied + 1 Conflict |
| legacy Whale U11→U13 | passed；旧 canonical JSON 原地保留并由新 store 解码 |
| U13 focused tests | 7 passed（含 3 migration bridge tests） |
| `cargo test --offline -p codex-state --lib` | 177 passed |
| `cargo clippy --offline -p codex-state --lib -- -D warnings` | passed |
| sync replay / metadata | 43 tests passed；inventory/replay/metadata checks passed；当前 overlay 56 路径 |
| cache regression index gate | passed；指纹 `4efdd365d0c017c538e9bf58462956ba225a20d67c52944578d1764142df353f`；免费 final-wire 通过，最近一次 live 回归仍为失败且未晋升 |
| 手写生产代码 | 441 行；小于 500 行门禁，单文件 389 行 |
| 外部请求 | 0 |

## 4. 结论与下一步

U13 已建立唯一 canonical TaskSpace 持久化权威，并让 fresh/current 与已知旧 Whale 数据库走同一 `StateRuntime` store。下一步 U14 只能通过 Codex 0.147 extension contributors 注入 store 能力并接 lifecycle/WorldState；不得让 AgentGraphStore 保存 Map，也不得恢复旧 core/session/provider 专用分支。
