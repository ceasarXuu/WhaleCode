# Problem P-001: 普通 thread/fork 丢失 TaskSpace lineage
- Status: fixed
- Created: 2026-08-15 00:16
- Updated: 2026-08-15 00:33
- Objective: 普通 app-server/TUI fork 应通过 production lifecycle 继承父线程 TaskSpace map，并记录 `Fork` relation。
- Symptoms:
  - 普通 fork 新线程没有 TaskSpace binding，运行时静默保持 Standard。
- Expected behavior:
  - fork 子线程继承 source thread 的 map，relation 为 `Fork`，reload 后仍可恢复。
- Actual behavior:
  - TaskSpace extension 只识别 `SessionSource::SubAgent` parent，并固定写入 `Child` relation。
- Impact:
  - fork 后 TaskSpace world state 与工具消失，计划要求的 fork 语义未实现。
- Reproduction:
  - source thread 已绑定 TaskSpace map；通过普通 `thread/fork` 创建新线程；读取新线程 TaskSpace snapshot/binding。
- Environment:
  - Linux，branch `whalecode-codex`，baseline `3761b8e1a`。
- Known facts:
  - `SessionConfiguration` 已包含 host-resolved `forked_from_thread_id`。
  - lifecycle input 当前没有该字段。
- Ruled out:
  - fork manager 未解析来源：`thread_manager` 已计算并传递 `forked_from_thread_id` 到 session configuration。
- Fix criteria:
  - lifecycle input 接收 trusted fork source。
  - SubAgent 仍使用 `Child`，普通 fork 使用 `Fork`。
  - 真实 state store 的 fork binding 可 reload。
  - 现有 extension contributors 与测试完成机械适配。
- Current conclusion: trusted fork source 已进入 lifecycle；extension 与真实 app-server/SQLite RPC 均验证 `Fork` binding。
- Related hypotheses:
  - H-001
- Resolution basis:
  - H-001；E-001、E-002、E-003、E-004
- Close reason:
  - 普通 thread/fork 的生产 inheritance 已由 process-level test 证明

## Hypothesis H-001: fork lineage 在 extension API 边界被截断
- Status: confirmed
- Parent: P-001
- Claim: Core 已解析普通 fork source，但 `ThreadStartInput` 未携带它，TaskSpace extension 因此只能处理 SubAgent parent 并写 `Child`。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - reviewer 从关系构造反查，主 Agent 从 thread manager 正向追踪，均定位到同一 API seam。
- Falsifiable predictions:
  - If true: `SessionConfiguration.forked_from_thread_id` 在 lifecycle 调用前存在，但 `ThreadStartInput` 无对应字段，生产中无 `TaskSpaceMapRelation::Fork` 构造。
  - If false: extension 能从 trusted host input 取得普通 fork source，且会绑定 `Fork`。
- Diagnostic evidence plan:
  - Prediction or clause under test: fork source 在 session→extension 边界消失。
  - Signal: manager/session 字段赋值、lifecycle struct 与 TaskSpace relation 构造。
  - Capture method: 正向调用链和反向 relation usage 的只读源码检查；随后用 extension + real state 回归测试复现。
  - Event name or marker:
    - none
  - Correlation keys:
    - source_thread_id and forked_thread_id
  - Differentiates from:
    - fork manager 没有解析 source，或 state store 不支持 Fork relation
  - Supports if:
    - session config 有 source，但 lifecycle input 无字段且 relation 只构造 Child。
  - Refutes if:
    - lifecycle 已携带普通 fork source并生产构造 Fork。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: closed
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Core 已保留普通 fork source
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `core/src/thread_manager.rs` and `core/src/session/session.rs`
- Prediction or plan link:
  - H-001 的 manager→session 预测
- Matched signal:
  - manager 设置 `request.forked_from_thread_id`，Session 将其写入 `SessionConfiguration`
- Correlation keys:
  - forked_from_thread_id
- Raw content:
  ```text
  request.forked_from_thread_id = source_thread_id;
  session_configuration.forked_from_thread_id = forked_from_id;
  ```
- Interpretation: 根因不在 fork source discovery，而在后续 extension seam。
- Time: 2026-08-15 00:16

## Evidence E-003: extension lifecycle 区分 Child 与 Fork
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `just test -p codex-taskspace-extension regular_fork_inherits_with_fork_relation`
- Prediction or plan link:
  - P-001 的 relation fix criterion
- Matched signal:
  - regular fork 绑定 relation=Fork、parent_thread_id=source
- Correlation keys:
  - source and forked thread ids
- Raw content:
  ```text
  test runtime_extension_tests::regular_fork_inherits_with_fork_relation ... ok
  ```
- Interpretation: extension seam 已能表达普通 fork，且不复用 Child relation。
- Time: 2026-08-15 00:33

## Evidence E-004: production app-server fork 持久化 TaskSpace binding
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `just test -p codex-app-server --test all thread_fork_inherits_taskspace_through_production_extensions`
- Prediction or plan link:
  - P-001 的 production composition 与 reload fix criteria
- Matched signal:
  - typed thread/fork RPC 经 production registry 后，真实 SQLite 返回同 map、Fork relation、source parent
- Correlation keys:
  - source_thread_id and forked_thread_id
- Raw content:
  ```text
  test suite::v2::thread_fork::thread_fork_inherits_taskspace_through_production_extensions ... ok
  ```
- Interpretation: 修复不是 mock-only；app-server、Core lifecycle、TaskSpace extension 与 SQLite composition 已闭合。
- Time: 2026-08-15 00:33

## Evidence E-002: lifecycle 丢失 source 且 TaskSpace 只构造 Child
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `ext/extension-api/src/contributors/thread_lifecycle.rs`, `core/src/session/session.rs`, `ext/taskspace/src/{extension.rs,runtime.rs}`
- Prediction or plan link:
  - H-001 的 session→extension 预测
- Matched signal:
  - `ThreadStartInput` 无 fork field；TaskSpace 只调用 `session_source.parent_thread_id()` 并绑定 `Child`
- Correlation keys:
  - thread_id and parent_thread_id
- Raw content:
  ```text
  if let Some(parent_thread_id) = input.session_source.parent_thread_id()
  relation: TaskSpaceMapRelation::Child
  ```
- Interpretation: 普通 fork source 无法到达 TaskSpace，精确解释静默丢失。
- Time: 2026-08-15 00:16
