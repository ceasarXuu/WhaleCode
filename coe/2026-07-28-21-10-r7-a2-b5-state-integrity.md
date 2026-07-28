# Problem P-001: R7.1 A2-B5 仍存在破坏 canonical Map 完整性的生产旁路
- Status: fixed
- Created: 2026-07-28 21:10
- Updated: 2026-07-28 22:37
- Objective: 删除旧生命周期旁路，并保证所有 action identity 错误在 canonical Store commit 前被拒绝
- Symptoms:
  - `task-reborn` / `RestartActionMap` 可把已有 canonical Map 覆盖为 `null`
  - 空或重复 sibling `call_id` 可在 reservation 已提交后失败
  - projection 将不可绑定动作的 Root 暴露在 `active_frontier`
- Expected behavior:
  - Map 只允许 absent -> active、active -> closed、closed -> active 三种生命周期转换
  - 完整 response 的身份、数量、名称和顺序必须在 Store mutation 前完成机械校验
  - projection 只暴露 Agent 可执行的 Work 前沿
- Actual behavior:
  - CLI、TUI 和 App Server 仍可触发旧 reborn/restart 清空路径
  - preflight 不校验 sibling `call_id` 非空和 response-wide 唯一
  - Root 的派生 Ready 状态被 projection 无差别加入动作前沿
- Impact:
  - 可丢失 Map completion、result 和 terminal history
  - 可遗留无法正常闭合的 InFlight reservation
  - Agent 接收不可执行的前沿语义
- Reproduction:
  - 对已有 Map 触发 `RestartActionMap`，观察 Store canonical JSON 变为 `null`
  - 构造两个相同 `call_id` 的 sibling Tool calls，观察 preflight 通过并在 prepare 后失败
  - 构造 active Map projection，观察 Root 出现在 `active_frontier`
- Environment:
  - Linux / branch `whalecode-alpha` / implementation commit `302ea5db3`
- Known facts:
  - fresh 对抗性 reviewer 已给出 CLI/TUI/App Server、core runtime 和 Store 的完整可达代码链
  - 当前 B5 contract/hash、sequence 和 action-map 回归均通过，说明门禁缺少上述反例
- Ruled out:
  - 旧 control/result/oracle fixture 已由 authority 标记为 `obsolete_non_promotable`，不是本问题的生产入口
- Fix criteria:
  - 删除所有 reborn/restart 生产入口、协议和 schema，且残留门通过
  - initialize/execute/reopen 的空或重复 `call_id` 在 Store commit 前拒绝，revision 和 reservation 不变
  - prepared calls 按 response index 保序配对，不再用裸 `call_id` HashMap 折叠
  - projection `active_frontier` 不包含 Root
  - 更新 benchmark marker、oracle、日志 reason code、真实 Store 测试和全量 B5 门
  - fresh closure reviewer 确认 blocking findings 关闭
- Current conclusion: 两条 P0 生产旁路和 projection 缺口均已修复；定向、StateDB、全量核心和 B5 门已通过，第二名 fresh closure reviewer 给出 0 blocking / 0 non-blocking 和 closure PASS
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - preflight 在任一 Store mutation 前拒绝空白和重复 call identity
  - Store prepared calls 按 response index 保序，不再用 call_id HashMap 二次配对
  - CLI、TUI、App Server、protocol 和 runtime 的 reborn/restart 清空路径已删除
  - projection active_frontier 只包含可执行 Work
  - 当前 benchmark observer 和默认 harness 已切换到 reservation/canonical projection 合同
  - 第二名不继承主会话上下文的只读 reviewer 独立确认固定修复范围闭合
- Close reason:
  - fixed；`d87d1af35` 关闭全部已接受 finding，fresh closure review 通过

## Hypothesis H-001: 旧 reborn/restart 入口绕过唯一 Map 生命周期
- Status: confirmed
- Parent: P-001
- Claim: 活跃 CLI、TUI 和 App Server 入口调用 `clear_active_map`，Store CAS 将 canonical Map 覆盖为 `null`
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - reviewer 追踪到公开参数、协议 Op、Session mutation、runtime delete 和 Store nullable serialization
- Falsifiable predictions:
  - If true: 生产入口和协议可达，调用后 canonical bytes、revision、terminal history 发生倒退
  - If false: 入口只存在于失效 fixture，或 Store 拒绝从有效 Map 写入 `None`
- Diagnostic evidence plan:
  - Prediction or clause under test: `RestartActionMap` 可达并允许 canonical Map -> null
  - Signal: 定向生产入口测试与残留扫描
  - Capture method: 在现有 Store/session 测试中保存 Map 后调用入口，断言当前实现会清空；扫描所有 Op/CLI/schema
  - Event name or marker:
    - request_reborn
  - Correlation keys:
    - map_id
    - store_revision
  - Differentiates from:
    - 仅历史 benchmark marker
  - Supports if:
    - 测试观察到 canonical Map 变为 null 或生产入口仍可编译到 dispatch
  - Refutes if:
    - 入口不可达且 Store 始终保留原 canonical bytes
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 删除旧事件；保留 lifecycle transition 日志
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: 活跃生产路径已由代码链直接证明
- Repair design readiness: ready
- Next step: none；入口、Op、handler、schema 和清空实现已删除，残留门已激活
- Blocker:
  - none
- Close reason:
  - fixed by `d87d1af35`

## Hypothesis H-002: sibling call identity 校验晚于 canonical Store commit
- Status: confirmed
- Parent: P-001
- Claim: preflight 未校验空/重复 `call_id`，prepare 先提交 reservation，后续 owner binding 或 HashMap 配对才失败
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - response preflight、Store prepare、owner binding 和 sequence pairing 的调用顺序已明确
- Falsifiable predictions:
  - If true: 空/重复 `call_id` 能通过 preflight；重复 ID 在 HashMap 中折叠；失败发生于 commit 后
  - If false: 任一 Store mutation 前已有非空和唯一校验，或 post-commit 失败会原子释放全部 reservation
- Diagnostic evidence plan:
  - Prediction or clause under test: invalid call identity 通过 preflight
  - Signal: preflight 失败测试和 Store revision/reservation 测试
  - Capture method: 分别构造 initialize/execute/reopen 的空、重复 ID response，先锁定当前失败，再验证修复后零 commit
  - Event name or marker:
    - taskspace_response_preflight_rejected
  - Correlation keys:
    - response_id
    - call_id
    - map_revision
  - Differentiates from:
    - 普通 Tool 业务执行失败
  - Supports if:
    - 当前 preflight 返回计划而不是 `empty_call_id` / `duplicate_call_id`
  - Refutes if:
    - preflight 已拒绝且 Store revision 不变
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留稳定 reason code 和 revision 日志
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: 生产调用顺序已证明身份校验发生在 commit 后
- Repair design readiness: ready
- Next step: none；身份校验已前移，prepared calls 已改为 index 保序配对
- Blocker:
  - none
- Close reason:
  - fixed by `d87d1af35`

## Hypothesis H-003: projection 将 Root 的派生状态误当成可执行动作前沿
- Status: confirmed
- Parent: P-001
- Claim: Root 在 active Map 中派生为 Ready，而 projection 无节点类型过滤地加入 `active_frontier`
- Layer: sub-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - Root 可用于表达任务整体仍未闭合，但不能接受普通 Tool action
- Falsifiable predictions:
  - If true: active projection 的 `active_frontier` 包含 Root ID
  - If false: projection 只收集 Work，或 Root 不派生为 Ready
- Diagnostic evidence plan:
  - Prediction or clause under test: projection frontier 包含 Root
  - Signal: projection unit test
  - Capture method: 初始化 canonical Map 并断言 Root 不应出现在 `active_frontier`
  - Event name or marker:
    - none
  - Correlation keys:
    - map_id
    - node_id
  - Differentiates from:
    - Root 的合法全局状态展示
  - Supports if:
    - 当前 test 观察到 Root 位于动作前沿
  - Refutes if:
    - Root 只存在于全局 Map 详情而不进入动作前沿
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: 代码条件和 reviewer 独立测试均支持
- Repair design readiness: ready
- Next step: none；projection frontier 已只包含 Work，忠实性测试已通过
- Blocker:
  - none
- Close reason:
  - fixed by `d87d1af35`

## Evidence E-001: fresh reviewer 证明 reborn/restart 的生产可达链
- Related hypotheses:
  - H-001
- Direction: supports
- Type: external-review
- Source: fresh reviewer `019fa8c9-a775-7620-854e-31a0d025aa4f`
- Prediction or plan link:
  - H-001 生产入口和 Store 覆盖预测
- Matched signal:
  - CLI/TUI/App Server -> RestartActionMap -> request_reborn -> clear_active_map -> canonical_json null
- Correlation keys:
  - commit 302ea5db3
- Raw content:
  ```text
--task-reborn、thread/actionMap/restart 和 RestartActionMap 均为活跃生产入口；
clear_active_map 删除 runtime Map，Store 随后序列化 canonical_map=None。
  ```
- Interpretation: 排除“仅历史字符串残留”，确认完整生产旁路
- Time: 2026-07-28 21:05

## Evidence E-002: fresh reviewer 证明 call identity 的 post-commit 失败顺序
- Related hypotheses:
  - H-002
- Direction: supports
- Type: external-review
- Source: fresh reviewer `019fa8c9-a775-7620-854e-31a0d025aa4f`
- Prediction or plan link:
  - H-002 preflight 缺口和调用顺序预测
- Matched signal:
  - preflight 未检查 ID；prepare 提交；HashMap 折叠重复 ID；owner binding 拒绝空 ID
- Correlation keys:
  - commit 302ea5db3
- Raw content:
  ```text
Store mutation/reservation 先提交；重复 ID 随后在 HashMap 配对中 Fatal，空 ID 随后在 owner binding 中失败。
  ```
- Interpretation: 说明问题是机械身份校验时机错误，不是 Agent 语义错误
- Time: 2026-07-28 21:05

## Evidence E-003: fresh reviewer 观察到 Root 位于 active_frontier
- Related hypotheses:
  - H-003
- Direction: supports
- Type: external-review
- Source: fresh reviewer `019fa8c9-a775-7620-854e-31a0d025aa4f`
- Prediction or plan link:
  - H-003 projection frontier 预测
- Matched signal:
  - Root Ready 状态被 projection 无差别加入 active_frontier
- Correlation keys:
  - commit 302ea5db3
- Raw content:
  ```text
Root 在非终态派生为 Ready；projection 将全部 Ready/InFlight 节点加入 active_frontier。
  ```
- Interpretation: 全局状态本身可以保留，但动作前沿语义不忠实
- Time: 2026-07-28 21:05

## Evidence E-004: 三种 prepared action 均未在 preflight 拒绝无效 call identity
- Related hypotheses:
  - H-002
- Direction: supports
- Type: test
- Source: `cargo test -p codex-core --lib taskspace_preflight_rejects --locked`
- Prediction or plan link:
  - H-002 preflight 缺口预测
- Matched signal:
  - 3 个新增测试全部失败，返回 `TaskSpaceExecute` 而不是 protocol failure
- Correlation keys:
  - commit 302ea5db3
- Raw content:
  ```text
taskspace_preflight_rejects_empty_sibling_call_id_for_every_prepared_action ... FAILED
taskspace_preflight_rejects_duplicate_call_id_for_every_prepared_action ... FAILED
taskspace_preflight_rejects_control_and_sibling_with_same_call_id ... FAILED
  ```
- Interpretation: initialize、execute、reopen 的无效身份都能越过 Store 前的唯一 preflight
- Time: 2026-07-28 21:15

## Evidence E-005: 实际 projection 把 Root 加入动作前沿
- Related hypotheses:
  - H-003
- Direction: supports
- Type: test
- Source: `cargo test -p codex-core --lib active_frontier_contains_executable_work_but_not_root --locked`
- Prediction or plan link:
  - H-003 projection frontier 预测
- Matched signal:
  - actual `["inspect", "root"]`，expected `["inspect"]`
- Correlation keys:
  - map_id projection-map
- Raw content:
  ```text
left: ["inspect", "root"]
right: ["inspect"]
  ```
- Interpretation: Root 的全局 Ready 状态被错误投影为 Agent 可执行前沿
- Time: 2026-07-28 21:15

## Evidence E-006: call identity 在 Store 前被稳定拒绝
- Related hypotheses:
  - H-002
- Direction: supports
- Type: test
- Source: `cargo test -p codex-core --lib taskspace_preflight_rejects --locked`
- Prediction or plan link:
  - H-002 修复标准
- Matched signal:
  - initialize、execute、reopen 的空白、重复和 control/sibling 冲突均返回 preflight rejection
- Correlation keys:
  - reason_code
  - call_id
- Raw content:
  ```text
3 passed; 0 failed
  ```
- Interpretation: 无效 response identity 不再进入 reservation transaction
- Time: 2026-07-28 21:40

## Evidence E-007: 无效身份不改变持久化 Map
- Related hypotheses:
  - H-002
- Direction: supports
- Type: integration-test
- Source: `cargo test -p codex-core --test all taskspace_terminal_contract --locked`
- Prediction or plan link:
  - H-002 Store revision/reservation 不变
- Matched signal:
  - 初始化失败保持 revision 0/canonical null；active execute 失败保持 revision 2；closed reopen 失败保持 revision 3 和 terminal
- Correlation keys:
  - thread_id
  - map_revision
  - terminal
- Raw content:
  ```text
5 passed; 0 failed
  ```
- Interpretation: initialize、execute、reopen 三个生命周期位置均满足零部分提交
- Time: 2026-07-28 22:18

## Evidence E-008: projection 动作前沿排除 Root
- Related hypotheses:
  - H-003
- Direction: supports
- Type: test
- Source: `cargo test -p codex-core --lib active_frontier_contains_executable_work_but_not_root --locked`
- Prediction or plan link:
  - H-003 修复标准
- Matched signal:
  - active_frontier 只包含 `inspect`
- Correlation keys:
  - map_id projection-map
- Raw content:
  ```text
1 passed; 0 failed
  ```
- Interpretation: Root 仍保留在全局 Map，但不再被再解释成可执行动作
- Time: 2026-07-28 21:40

## Evidence E-009: 旧生命周期和禁用测试残留门通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: contract-test
- Source: `pwsh scripts/taskspace-benchmark/test-r7-five-layer-contracts.ps1 -Phase A2-B5`
- Prediction or plan link:
  - H-001 生产入口和残留扫描
- Matched signal:
  - reborn/restart 生产符号为零；multi-agent 旧 binding/lease helper 和 `#[cfg(any())]` 为零
- Correlation keys:
  - authority hash
  - production manifest hash
- Raw content:
  ```text
R7 integrated change constraints: PASS
R7.1 A2-B5 five-layer contract validation passed
  ```
- Interpretation: 旧入口不能再通过 CLI、TUI、App Server 或禁用测试回流
- Time: 2026-07-28 22:25

## Evidence E-010: 完整工程回归通过
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: regression-test
- Source:
  - `cargo test -p codex-core --lib --locked`
  - `scripts/run-action-map-regression.ps1`
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - core 1911 passed / 0 failed / 3 ignored
  - Action Map 10 Rust runs、124 tests、0 failed；脚本 5 passed、1 platform skip
- Correlation keys:
  - report `target/test-reports/action-map-20260728-222839-257/report.md`
- Raw content:
  ```text
test result: ok. 1911 passed; 0 failed; 3 ignored
overall: PASS
  ```
- Interpretation: 修复未破坏 Standard 隔离、App schema、multi-agent tools 或当前 Map 回归面
- Time: 2026-07-28 22:26

## Evidence E-011: fresh closure reviewer 独立确认修复闭合
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: external-review
- Source: fresh reviewer `019fa925-3778-7da0-a8f2-30098be0962b`
- Prediction or plan link:
  - P-001 fresh closure reviewer fix criterion
- Matched signal:
  - blocking 0、non-blocking 0、closure PASS
- Correlation keys:
  - fixed commit `d87d1af35`
  - review `vs_review/2026-07-28-r7-a2-b5-review.md`
- Raw content:
  ```text
旧 restart/reborn 生产路径已删除；call identity 在 Store mutation 前拒绝；
prepared calls 按 response order 配对；active_frontier 只包含 Work。
未发现 fixed scope 内的 blocking 或 non-blocking finding。
  ```
- Interpretation: 首轮接受的六项 finding 和额外残留已由独立 reviewer 关闭
- Time: 2026-07-28 22:37
