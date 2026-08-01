# R8-I09 Store Hydrate 完整性修复计划

- Created: 2026-07-31
- Issue: R8-I09
- Plan mode: Plan Authoring
- Plan status: completed
- Scope: canonical Map 从 Store 进入 Core Runtime 的恢复边界
- Non-goal: 不修改 Map 产品模型、状态机规则、Tool schema、projection 或 Standard

## 1. 问题与目标

### 当前行为

TaskSpace 已有完整的 canonical Map 校验器：

```rust
rooted_dag::validate(&TaskSpaceMap) -> Vec<Violation>
```

正常 Map transaction 会对 candidate 调用该校验器。但 Store hydrate 当前经过：

```text
SQLite row
  -> decode_map_row
  -> runtime_from_record
  -> restore_store_map
  -> restore_canonical_map
```

其中 Store codec 只检查 schema version、JSON hash、Map ID、revision 和 terminal 列一致性；
`restore_store_map()` 只再次比较 Map ID，随后直接安装 canonical Map，没有调用
`rooted_dag::validate()`。

### 目标行为

任何非空 Store Map 必须在改变 Runtime mode、cache、active map 或 child/fork binding 之前，通过现有
`rooted_dag::validate()`。非法 Map 明确失败、零 Runtime 安装、零新增绑定、零 fallback；合法 active、
closed、reopened 和多父 DAG 原样恢复。

### 非目标

- 不新增第二套 Map validator；
- 不把 Core 图语义复制到 `codex-state`；
- 不自动修图、补边、升级旧 schema 或从 rollout 重建；
- 不为历史 Map 数据增加兼容路径；
- 不运行真实 Whale Agent；I09 是确定性 Store/Runtime 问题。

## 2. 当前证据

| 事实 | 当前源码证据 | 判断 |
|---|---|---|
| 完整 validator 已存在 | `core/src/action_map/rooted_dag/invariants.rs:138` | 不需要设计新校验系统 |
| validator 覆盖图、facts、reservation、reference 和 terminal | `invariants.rs:160-384` | hydrate 应复用同一不变量 |
| 正常 transaction 校验 candidate | `core/src/action_map/rooted_dag/events.rs:159` | 写入前 Runtime 语义边界已存在 |
| Store decode 只检查存储一致性 | `state/src/runtime/taskspace_map_codec.rs:12-36` | 该层职责合理，但不足以证明 Map 合法 |
| hydrate 直接调用 restore | `core/src/session/taskspace_store.rs:98,132-143` | 所有 record 恢复汇入同一入口 |
| restore 只比较 Map ID | `core/src/action_map/runtime/state.rs:193-215` | 根因已定位为现有 validator 漏接入 |
| restore 在校验 Map ID 前修改 mode | `state.rs:199-208` | 失败不是严格零状态变化 |
| child 在 runtime restore 前写绑定 | `core/src/session/taskspace_store.rs:52-80,98` | 非法父 Map 可能留下 child binding |
| `restore_canonical_map` 只有 restore 路径调用 | 当前仓库 `rg "restore_canonical_map\\("` | 可收窄可见性，防止未来绕过 |
| resume/fork/child 合法同图测试已存在 | `core/src/session/taskspace_store_tests.rs:30-138` | 修复后有正向回归基线 |

当前根因置信度：**E1 confirmed**。尚缺非法 Store Map 的确定性失败 fixture 和修复后的 E2 集成证据。

## 3. 目标控制流

```text
load Store record
  -> Store codec 校验 schema/hash/column consistency
  -> Core rooted_dag::validate(canonical Map)
     -> invalid: structured log + return error + no binding/cache/mode mutation
     -> valid: optional child/fork binding
  -> restore canonical Map into a fresh Runtime
  -> install Runtime and Store handle into Session
```

State DB 负责“读到的字节与存储列一致”；Core 负责“这份 TaskSpace Map 在产品语义上合法”。两者是不同边界，
不构成重复 validator。

## 4. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|
| I09-W1 | 固化非法恢复反例 | test | `core/src/action_map/runtime/state.rs` | `restore_store_map` tests | 增加 cycle、不可达节点、fact conflict 和 Map ID mismatch fixture，并断言失败前后 Runtime 完全相等 | 当前缺口先以失败测试复现 | 防止实现只覆盖一个图错误或掩盖原子性问题 | 定向运行 runtime state tests；修复前预期失败、修复后全部通过 | 只增加测试，可单独回退 | completed |
| I09-W2 | 接入唯一 Map validator | runtime | `core/src/action_map/runtime/state.rs` | `restore_store_map()` | 对 `Some(map)` 先校验 expected Map ID，再调用现有 `rooted_dag::validate()`；只有零 violation 后才修改 mode 和安装 Map | 非法 canonical Map 无法进入 Runtime | 保证所有上层 revision、节点状态和反馈建立在合法 Map 上 | W1 全部通过；合法 Map 测试保持通过 | 整体回退该函数改动，不保留开关或双路径 | completed |
| I09-W3 | 封闭未校验安装入口 | internal API | `core/src/action_map/runtime/state.rs` | `restore_canonical_map()` | 将其可见性收窄为仅 `restore_store_map()` 内部可调用，或内联安装逻辑 | 未来调用者不能绕过 restore 校验 | 降低同类缺口再次出现的维护风险 | `rg` 证明无外部调用；core 编译和测试通过 | 若编译揭示合法调用者，暂停并重新盘点，不增加旁路 | completed |
| I09-W4 | 防止非法父 Map 留下绑定 | Store integration | `core/src/session/taskspace_store.rs` | `hydrate_action_map_store()` parent binding branch | 在 `bind_thread_to_taskspace_map()` 前验证父 record 可恢复；验证失败直接返回，不写 child/fork binding | 非法父 Map hydrate 失败时 Store 绑定保持不变 | 避免一次失败污染后续 child/fork 恢复关系 | 新增非法父 Map + child/fork 测试，失败后查询 binding 必须为空 | 单独回退分支顺序；不得通过失败后补删实现 | completed |
| I09-W5 | 验证真实 Store 恢复边界 | integration test | `core/src/session/taskspace_store_tests.rs` | hydrate resume/fork/child cases | 通过 State DB 写入存储一致但图不合法的 canonical JSON，断言 resume/fork/child 全部拒绝；补合法多父、closed/reopen 正向 fixture | Store codec 可通过但产品 Map 非法的真实场景被 Core 拦截 | 证明修复覆盖生产 hydrate，而不只是直接函数测试 | 定向运行 session TaskSpace Store tests；合法和非法矩阵全部通过 | 测试数据仅临时 SQLite 目录，失败不修改生产数据 | completed |
| I09-W6 | 建设失败日志 | observability | `core/src/session/taskspace_store.rs` | hydrate rejection log | 在 hydrate 拒绝处记录固定 event、reason code、map/store revision 和 relation；详细 violation 仅保留在返回错误中，不写入日志 | 非法恢复可由机械身份和稳定原因码定位 | 后续诊断无需重放用户会话，且不泄露 Map 业务内容 | tracing capture test 断言事件与字段、日志不含 goal | 删除新增日志不影响拒绝行为 | completed |
| I09-W7 | 排除第二恢复权威 | cleanup audit | `core/src/session`、`core/src/action_map` | rollout/session restore call sites | 搜索并核对所有 canonical Map 构造与恢复入口；发现 rollout fallback 则作为独立删除提交，不做兼容 | Store 缺失或非法时不会从聊天历史重建 Map | 保持 Map Store 唯一事实源，避免修复被旁路抵消 | `rg` 清单、resume/fork/child tests、Store-missing tests | 若发现独立产品入口，停止并向用户报告影响后再改 | completed |
| I09-W8 | 整体回归与结果记录 | verification/docs | core/state test targets、`build-R8/I09` | I09 acceptance | 运行定向测试、相关 core/state 回归、格式和编译检查；记录变更、日志和约束检查 | I09 有可复算关闭证据 | 后续 I01 revision 调查可以信任 hydrate 后的 canonical Map | 所有验收项通过，Standard path 无 diff，不运行 Whale Agent | 任一底层约束回归则整体回退 I09 行为提交 | completed |

## 5. 验收矩阵

| 场景 | 预期结果 |
|---|---|
| 合法 active Map | 原样恢复相同 map_id、revision、nodes、edges 和 facts |
| 合法多父 DAG | 正常恢复，不误判为树结构错误 |
| 合法 closed/reopen history | terminal history 与旧 Work completion 保持不变 |
| cycle/self-loop/孤立节点 | 安装前拒绝并返回对应 violation |
| Root 不可达或无法到达 Finish | 安装前拒绝 |
| completion/block/reservation 冲突 | 安装前拒绝 |
| Map ID mismatch | 安装前拒绝，Runtime mode 和 active map 不变 |
| resume 非法 Map | Session 不安装 Runtime/handle |
| child/fork 非法父 Map | 不新增 binding，不安装 Runtime/handle |
| Store 缺失或损坏 | 明确失败，不从 rollout 重建 |
| Standard session | 不读取、不绑定 TaskSpace Map，行为不变 |

## 6. 日志合同

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason Field | Correlation Field | Consumer |
|---|---|---|---|---|---|---|
| Store row -> Core validation | canonical Map | existing `taskspace.map_store_loaded` | `taskspace.map_store_hydrate_rejected` | `reason_code`, `error` | `map_id`, `store_revision`, `map_revision` | developer / benchmark |
| Parent Map -> child/fork binding | relation binding | existing `taskspace.map_store_thread_bound` | hydrate rejection before bound event | `reason_code` | `map_id`, `actor_thread_id`, `parent_thread_id` | developer / subagent diagnostics |

日志只记录机械身份和 violation code，不记录 node goal、工具结果或用户内容。

## 7. 风险与控制

| Risk | Trigger Signal | Mitigation | Safe Stop / Fallback |
|---|---|---|---|
| validator 错拒合法多父或 terminal Map | 现有合法恢复测试失败 | 先补多父、closed/reopen 正向 fixture，再接入 | 暂停 W2，不放宽 validator 特例 |
| child 绑定顺序调整引入并发窗口 | binding 或 revision 集成测试不稳定 | 复用当前 Store transaction/CAS，不在 Core 做补偿删除 | 暂停 W4，先画清并发时序 |
| error/log 为输出方便而复制 violation 语义 | 两套 violation 格式出现差异 | 直接使用现有 `ViolationCode`，不创建平行分类 | 删除新分类，只保留原始 code |
| 修复扩散到 State DB 复制图规则 | `codex-state` 出现 rooted-DAG 逻辑 | 保持 storage integrity 与 product invariant 分层 | 停止改动并回退复制逻辑 |

## 8. 外部依据

- [SQLite Atomic Commit](https://www.sqlite.org/atomiccommit.html)：事务内变更应表现为全部发生或全部不发生，
  支持 hydrate 失败不留下绑定或 Runtime 状态的原子性目标；
- [Rust `Result`](https://doc.rust-lang.org/std/result/)：使用显式成功/失败返回传播恢复错误，不采用静默 fallback；
- [OWASP Input Validation](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)：
  区分语法与语义校验，并在数据进入可信处理边界前执行 allowlist/invariant validation。

这些资料支持原子失败和边界校验原则；具体不变量仍以 WhaleCode 当前 `rooted_dag::validate()` 为唯一权威。

## 9. 执行顺序

```text
W1 failing fixtures
  -> W2 validator 接入
  -> W3 安装入口封闭
  -> W4 binding 原子性
  -> W5 Store 集成矩阵
  -> W6 日志
  -> W7 第二事实源审计
  -> W8 整体回归与结果文档
```

每个工作单元单独提交并验证。任何单元发现产品模型或重大技术路线与本计划不同，暂停并与用户讨论。
