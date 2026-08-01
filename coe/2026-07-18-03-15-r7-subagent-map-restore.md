# Problem P-001: R7 Phase B 子代理 fork 重建 Map 失败
- Status: open
- Created: 2026-07-18 03:15
- Updated: 2026-07-18 03:31
- Objective: 恢复 TaskSpace 节点绑定子代理的正常 fork，同时保持 projection policy 在真正的 TaskSpace resume/fork 中不可变。
- Symptoms:
  - 两个既有 TaskSpace 子代理测试在首个 spawn 时失败，错误为 current binding 与 main lease 不一致。
- Expected behavior:
  - 节点绑定的子代理按既有线性化边界创建，父 Map 保持由主代理管理；真正的 TaskSpace session resume/fork 恢复原 projection policy。
- Actual behavior:
  - 子代理 session 初始化进入 Map checkpoint/delta 重建并因父主代理 lease 状态与子代理 binding 不一致而失败。
- Impact:
  - 完整 TaskSpace 回归仍有两个子代理 spawn 用例失败；Phase A 基线复现证明它不是 Phase B 生产回归。
- Reproduction:
  - `cargo test -p codex-core action_map_completion_watcher_advances_next_spawn_to_next_node -- --nocapture`
- Environment:
  - Linux，分支 `whalecode-alpha`，R7 Phase B 未提交工作树。
- Known facts:
  - projection 定向测试 18/18 通过。
  - 失败可在单测试隔离环境中稳定复现，不是完整测试并行噪声。
  - 报错来自 `map_checkpoint_delta_chain` 的 domain invariant。
  - Phase A 基线提交 `c4f3c3c57` 上同一测试以相同错误失败。
  - Phase B renderer/composer 变更不写 canonical binding 或 lease。
- Ruled out:
  - 仅由完整测试并行执行或共享 `/tmp` 引起。
  - R7 Phase B 新增 projection policy 恢复导致该失败。
  - R7 Phase B renderer/composer 修改了父 Map。
- Fix criteria:
  - 两个原始失败测试隔离通过。
  - TaskSpace resume/fork policy 恢复测试和 projection/replay 回归继续通过。
  - 日志能够机械区分线性化子代理 fork 与真正 TaskSpace session 恢复。
- Current conclusion: 该失败是 Phase A 基线已存在的子代理 fork/replay 缺陷，不是 Phase B 回归；当前最可能窗口是 spawn assignment 已持久化、child attach 尚未发生时截取父 rollout，但该机制留待独立修复验证。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 线性化子代理错误继承 projection policy 并进入 TaskSpace 恢复
- Status: refuted
- Parent: P-001
- Claim: `InitialHistory::Forked` 无条件恢复父 session metadata 中的 projection policy，使本应线性化的节点子代理被识别为 TaskSpace session，并重建不属于它的父 Map。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Phase B 新增了 Forked history 的 policy 恢复；失败恰好发生在既有节点子代理 spawn 初始化阶段。
- Falsifiable predictions:
  - If true: 失败 fork history 含父 policy，但该 spawn 同时选择了 linearize/subagent 边界；移除该错误继承后不会进入 Map restore。
  - If false: 失败 child 没有继承 policy，或在没有 policy 时仍以相同路径重建并失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: 跟踪 child session 的 `InitialHistory`、`linearize_taskspace_for_subagent`、effective map mode 和 restored policy。
  - Signal: 代码分支与定向测试中的 session 初始化参数。
  - Capture method: 代码路径检查；必要时加入只记录模式、policy 和 history kind 的结构化诊断日志。
  - Event name or marker:
    - `taskspace.subagent_session_restore_decision`
  - Correlation keys:
    - parent thread id
    - child thread id
  - Differentiates from:
    - H-002
  - Supports if:
    - 线性化 child 同时携带父 policy，并因此选择 TaskSpace reconstruction。
  - Refutes if:
    - child 没有 policy 或 policy 不参与 reconstruction 分支。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 若新增则保留为不含语义内容的 session lifecycle 日志。
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-002
- Conclusion: Phase A 不存在 R7 policy 字段但同样失败，policy 继承不是该症状的必要条件。
- Repair design readiness: not applicable
- Next step: closed as a refuted R7 regression direction.
- Blocker:
  - none
- Close reason:
  - Phase A baseline reproduction refuted the causal claim

## Hypothesis H-002: R7 renderer 或 provider composer 改坏了 canonical Map 状态
- Status: refuted
- Parent: P-001
- Claim: provider projection 重构在生成或过滤 projection 时修改了 canonical Map 的 binding/lease，导致子代理看到真实损坏的父 Map。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 报错内容是 canonical domain invariant，不能仅凭变更邻近性认定为 policy restore。
- Falsifiable predictions:
  - If true: 失败前父 Map 本身已出现 binding/lease 不一致，且只执行 projection render/composer 即可改变 hash/state。
  - If false: renderer/composer 为只读，父 Map 在 spawn 前满足 invariant，错误只在 child reconstruction 输入转换后出现。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查 renderer 调用的可变性及 spawn 前后 canonical hash、binding、lease。
  - Signal: 函数签名、快照状态和定向测试断言。
  - Capture method: 代码路径检查与已有 deterministic projection/runtime 测试。
  - Event name or marker:
    - `taskspace.projection_rendered`
  - Correlation keys:
    - map id
    - revision
  - Differentiates from:
    - H-001
  - Supports if:
    - projection 路径可修改 canonical state，或 spawn 前快照已经不合法。
  - Refutes if:
    - projection 路径只读且同一父快照在 child 转换前合法。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: 同一失败先于 R7 renderer/composer 存在，且当前路径只读 canonical Map。
- Repair design readiness: not applicable
- Next step: closed as a refuted R7 regression direction.
- Blocker:
  - none
- Close reason:
  - baseline reproduction and code-path inspection refuted the causal claim

## Hypothesis H-003: spawn 在 child attach 前持久化了不可独立重放的 assignment 中间态
- Status: unverified
- Parent: P-001
- Claim: `prepare_spawn_assignment` 先把 work node 从主路径绑定改为 SubAgent lease 并持久化 snapshot，随后 child 使用此时的父 rollout 做 FullHistory fork；该中间态缺少后续 child attach，无法作为独立 session replay 输入。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - MultiAgentV2 默认 `fork_turns=all`，而 spawn 顺序是 prepare assignment、持久化、fork parent rollout、创建 child、最后 attach child。
- Falsifiable predictions:
  - If true: child 读取的父 rollout 末尾停在 LeaseCreated/assignment snapshot，移到 attach 后取 fork snapshot 或定义原子 assignment envelope 后可恢复。
  - If false: child fork rollout 已包含 attach，或 assignment snapshot 单独满足 canonical replay invariant。
- Diagnostic evidence plan:
  - Prediction or clause under test: 捕获失败 child 的 fork rollout 尾部事件顺序和 replay snapshot binding/lease。
  - Signal: GraphRevisionCommitted、LeaseCreated、snapshot delta、LeaseAttached 的 revision 与顺序。
  - Capture method: 独立修复阶段增加确定性 fork-boundary fixture，不在 R7 Phase B 中扩大状态机改动。
  - Event name or marker:
    - `graph_revision_committed`
    - `map_runtime_snapshot_delta`
    - `lease_attached`
  - Correlation keys:
    - map id
    - lease id
    - parent/child thread id
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - replay 输入截止于 assignment delta 且该 snapshot 直接触发 current binding/main lease invariant。
  - Refutes if:
    - replay 输入已含 attach 或 assignment snapshot 自身合法。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 将复现转为永久回归测试；日志只保留机械生命周期字段。
- Evidence gate: pending
- Related evidence:
  - E-004
- Conclusion: unverified; code顺序支持但尚缺失败 rollout 的直接状态证据。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 在独立状态机修复范围中构造 fork 边界 fixture。
- Blocker:
  - R7 Phase B 明确不修改子代理状态机和 hard gate，当前不扩散阶段范围。
- Close reason:
  - not closed

## Evidence E-001: 子代理失败可隔离稳定复现
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `cargo test -p codex-core action_map_completion_watcher_advances_next_spawn_to_next_node -- --nocapture`
- Prediction or plan link:
  - P-001 原始症状和 H-001/H-002 的共同前提。
- Matched signal:
  - 首个子代理 spawn 在 session 初始化的 Map checkpoint/delta replay 阶段失败。
- Correlation keys:
  - test `action_map_completion_watcher_advances_next_spawn_to_next_node`
- Raw content:
  ```text
  first spawn should succeed: RespondToModel("collab spawn failed: Fatal error: Failed to initialize session: map_checkpoint_delta_chain: domain_invariant: TaskSpace current binding and main lease are inconsistent.")
  ```
- Interpretation: 这是稳定的 TaskSpace 子代理初始化回归，但单凭该错误尚不能区分错误 policy 继承与父 Map 本身损坏。
- Time: 2026-07-18 03:15

## Evidence E-002: Phase A 基线提交存在相同失败
- Related hypotheses:
  - H-001
  - H-002
- Direction: refutes
- Type: experiment
- Source: detached worktree `c4f3c3c57`; `cargo test -p codex-core action_map_completion_watcher_advances_next_spawn_to_next_node -- --nocapture`
- Prediction or plan link:
  - H-001/H-002 要求失败由 Phase B 新增 policy 或 projection 路径引入。
- Matched signal:
  - 不含 Phase B 代码的 Phase A 基线以完全相同的 `map_checkpoint_delta_chain` invariant 失败。
- Correlation keys:
  - commit `c4f3c3c57`
  - test `action_map_completion_watcher_advances_next_spawn_to_next_node`
- Raw content:
  ```text
  first spawn should succeed: RespondToModel("collab spawn failed: Fatal error: Failed to initialize session: map_checkpoint_delta_chain: domain_invariant: TaskSpace current binding and main lease are inconsistent.")
  ```
- Interpretation: 该症状不是 Phase B 回归，直接证伪 H-001 和 H-002 的 R7 因果前提。
- Time: 2026-07-18 03:31

## Evidence E-003: Phase B projection 路径只读 canonical Map
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: code-location
- Source: `core/src/session/mod.rs::prepare_provider_visible_prompt_items`、`core/src/action_map/runtime.rs::build_developer_context`
- Prediction or plan link:
  - H-002 预测 renderer/composer 修改 binding 或 lease。
- Matched signal:
  - composer 过滤 provider items 后调用只读 snapshot renderer；唯一可变动作是取出 pending projection trace events，不改 Map graph、binding 或 lease。
- Correlation keys:
  - none
- Raw content:
  ```text
  build_developer_context -> append_context_projection_active(&map, ...)
  compose_provider_visible_prompt_items -> items.push(projection_item)
  ```
- Interpretation: Phase B projection 构建无法产生 observed invariant，H-002 被代码路径和基线复现共同证伪。
- Time: 2026-07-18 03:31

## Evidence E-004: spawn 生命周期在 child fork 前提交 assignment
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `core/src/tools/handlers/multi_agents_v2/spawn.rs`、`core/src/action_map/runtime.rs::prepare_spawn_assignment`、`core/src/agent/control.rs::spawn_forked_thread`
- Prediction or plan link:
  - H-003 关于 assignment、fork、attach 顺序的预测。
- Matched signal:
  - handler 先 `prepare_action_map_spawn_assignment` 并持久化事件，随后默认 FullHistory fork 读取父 rollout，只有 child 创建成功后才执行 `attach_action_map_assignment`。
- Correlation keys:
  - lease id
  - parent/child thread id
- Raw content:
  ```text
  prepare_action_map_spawn_assignment
  spawn_agent_with_metadata -> spawn_forked_thread -> get_rollout_history(parent)
  attach_action_map_assignment (only after child success)
  ```
- Interpretation: 代码顺序支持存在不可独立重放的中间边界，但尚未捕获失败 rollout 的具体 snapshot，所以 H-003 不升级为 confirmed。
- Time: 2026-07-18 03:31
