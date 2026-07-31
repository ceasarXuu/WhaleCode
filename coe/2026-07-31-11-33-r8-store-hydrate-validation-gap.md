# Problem P-001: Store hydrate 未执行 canonical Map 完整校验
- Status: open
- Created: 2026-07-31 11:33
- Updated: 2026-07-31 11:33
- Objective: 确保任何持久化 canonical Map 在进入 Runtime 或建立 child/fork binding 前通过现有 rooted-DAG validator，失败时不产生状态副作用。
- Symptoms:
  - `restore_store_map()` 只比较 Store Map ID 与 canonical Map ID，随后直接安装 Map。
  - child/fork 首次绑定可能在父 Map 进入 Runtime 校验前写入 Store。
- Expected behavior:
  - Store codec 先验证存储字节、hash、schema、revision 和列一致性。
  - Core restore 边界随后调用唯一 `rooted_dag::validate()` 验证完整 Map 产品不变量。
  - 任何失败都不修改 Runtime mode/cache/active map，也不新增 child/fork binding。
- Actual behavior:
  - Core restore 路径没有调用现有完整 validator。
  - `restore_store_map()` 在 Map ID 检查前先将 Runtime mode 改为 Experiment。
  - child/fork 分支先写 binding，再调用 `runtime_from_record()`。
- Impact:
  - 非法 Map 可能进入 resume、fork、child 或重启后的 Runtime。
  - Map ID mismatch 已能返回错误，但仍会改变 Runtime mode。
  - 非法父 Map 可能留下未成功 hydrate 的 child/fork binding。
- Reproduction:
  - W1 将向 `restore_store_map()` 传入 cycle、不可达节点、fact conflict 和 Map ID mismatch Map，并比较失败前后 Runtime。
  - W5 将通过 State DB 保存存储一致但图不合法的 Map，再执行 resume/fork/child hydrate。
- Environment:
  - Linux，分支 `whalecode-alpha`，基线提交 `ff3c528c3`。
- Known facts:
  - 完整 validator 位于 `core/src/action_map/rooted_dag/invariants.rs:138`。
  - 正常 transaction 在 `rooted_dag/events.rs:159` 校验 candidate。
  - Store codec 在 `taskspace_map_codec.rs:12-36` 只做存储一致性检查。
  - hydrate 通过 `runtime_from_record -> restore_store_map -> restore_canonical_map` 安装 Map。
- Ruled out:
  - 项目缺少 Map 合法性检查方法。
  - 需要在 `codex-state` 复制一套 rooted-DAG validator。
- Fix criteria:
  - 非空 Store Map 安装前调用现有 `rooted_dag::validate()`。
  - 所有 violation 确定性拒绝，合法多父、active、closed/reopened Map 正常恢复。
  - 失败不修改 Runtime mode/cache/active map。
  - 非法父 Map 不新增 child/fork binding。
  - Store 缺失或非法时不从 rollout 或 Session snapshot 重建。
  - 失败日志包含稳定事件、Map/revision 身份和原始错误，不包含用户业务内容。
- Current conclusion: 源码路径支持“现有 validator 漏接入 restore 边界”的根因候选；等待 W1/W5 失败测试满足诊断证据门。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: restore_store_map 漏调用现有 rooted-DAG validator
- Status: unverified
- Parent: P-001
- Claim: 存储一致但图语义非法的 canonical Map 会被 `restore_store_map()` 接受并安装，因为该路径只比较 Map ID。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - `rooted_dag::validate()` 已覆盖完整不变量，但 restore 路径没有调用点。
- Falsifiable predictions:
  - If true: cycle、不可达节点或 fact conflict Map 在 ID 匹配时返回成功并成为 active canonical Map。
  - If false: 当前未被搜索发现的入口会在安装前拒绝这些 Map，或 `ActionMapInstance::from_graph` 自身执行等价校验。
- Diagnostic evidence plan:
  - Prediction or clause under test: ID 匹配但 rooted-DAG 非法的 Map 是否被安装。
  - Signal: `restore_store_map()` 返回值与 `canonical_map_for_store()`。
  - Capture method: `runtime/state.rs` 定向单元测试。
  - Event name or marker:
    - `restore_store_map_rejects_invalid_canonical_map`
  - Correlation keys:
    - map id
  - Differentiates from:
    - Store codec 损坏或 schema/hash mismatch
  - Supports if:
    - 当前测试显示 restore 成功且 Runtime 导出非法 Map。
  - Refutes if:
    - 当前代码在安装前返回 rooted-DAG violation。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 测试转为永久回归
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-002
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 执行 W1 failing fixture。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: child/fork hydrate 失败会留下提前写入的 binding
- Status: unverified
- Parent: P-001
- Claim: 当 child/fork 没有现有 binding 时，父 Map 会先绑定到新线程，之后的 Runtime hydrate 失败不会回滚该 binding。
- Layer: sub-cause
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - `hydrate_action_map_store()` 在 `runtime_from_record()` 前调用并提交 `bind_thread_to_taskspace_map()`。
- Falsifiable predictions:
  - If true: 非法父 Map 导致 hydrate 返回错误后，新 child/fork thread 仍可从 Store 查询到 binding。
  - If false: binding 与 restore 位于同一事务，或失败路径会原子回滚 binding。
- Diagnostic evidence plan:
  - Prediction or clause under test: hydrate 失败后的 binding 是否存在。
  - Signal: `load_taskspace_map_for_thread(child_or_fork)`。
  - Capture method: `taskspace_store_tests.rs` 使用临时 SQLite 的集成测试。
  - Event name or marker:
    - `invalid_parent_map_does_not_bind_child`
  - Correlation keys:
    - parent thread id
    - child/fork thread id
    - map id
  - Differentiates from:
    - Runtime cache mutation
  - Supports if:
    - hydrate 失败后 binding 查询仍返回记录。
  - Refutes if:
    - hydrate 失败且 binding 查询为空。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 测试转为永久回归
- Evidence gate: pending
- Related evidence:
  - E-003
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: W5 在 W1 后执行。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 完整 Map validator 已存在并覆盖恢复所需不变量
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/invariants.rs:138-384`
- Prediction or plan link:
  - H-001 关于无需新增 validator 的判断。
- Matched signal:
  - `validate()` 依次检查 identity、edges、degrees、reachability、facts 和 terminal。
- Correlation keys:
  - canonical Map
- Raw content:
  ```text
  pub(crate) fn validate(map: &TaskSpaceMap) -> Vec<Violation>
  ```
- Interpretation: 修复应复用现有 validator，不能复制图语义。
- Time: 2026-07-31 11:33

## Evidence E-002: restore 路径只比较 Map ID 后直接安装
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/action_map/runtime/state.rs:193-215`
- Prediction or plan link:
  - H-001 关于 restore 漏校验调用的判断。
- Matched signal:
  - `Some(map)` 分支只执行 `map.map_id != map_id`，随后调用 `restore_canonical_map`。
- Correlation keys:
  - map id
- Raw content:
  ```text
  if map.map_id != map_id { return Err(...); }
  self.restore_canonical_map(map, Some(owner_session_id));
  ```
- Interpretation: 源码中没有完整 Map validator 调用；仍需失败测试证明 `from_graph` 不会隐式拒绝。
- Time: 2026-07-31 11:33

## Evidence E-003: child/fork binding 提交发生在 Runtime restore 之前
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/session/taskspace_store.rs:52-98`
- Prediction or plan link:
  - H-002 关于失败绑定副作用的判断。
- Matched signal:
  - `bind_thread_to_taskspace_map(...).await?` 先完成，`runtime_from_record(&record)?` 后执行。
- Correlation keys:
  - parent thread id
  - actor thread id
  - map id
- Raw content:
  ```text
  state_db.bind_thread_to_taskspace_map(...).await?;
  ...
  let runtime = runtime_from_record(&record)?;
  ```
- Interpretation: 两步不在同一事务；仍需集成测试证明 restore 失败时 binding 确实保留。
- Time: 2026-07-31 11:33
