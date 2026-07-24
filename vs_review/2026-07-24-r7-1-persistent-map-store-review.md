# Subagent VS Review: R7.1 canonical Map Store

- Created: 2026-07-24T23:55:00+08:00
- Updated: 2026-07-25T00:45:00+08:00
- Report schema: adversarial-v1
- Task: 完成 R7.1-A0，并专项审查旧恢复残留、错误引用、双事实源和生产接线缺口。
- Report path: `vs_review/2026-07-24-r7-1-persistent-map-store-review.md`
- Review mode: fresh internal subagents
- Source session policy: 不继承主 Agent 上下文，只接收中立导航包。
- Status: open

## Round 1: A0 实施完整性与残留专项审查

### Review Input

#### Objective

证明独立持久化 Map Store 已成为 TaskSpace canonical Map 的唯一事实源；Session/Runtime 只保留 handle
和可丢弃 cache；rollout 不再恢复 Map；Standard 不受影响。

#### Review Target

`190371b23..5f9a3f034` 的设计、State repository、Core 生产接线、旧 replay 删除、测试与验证证据。

#### Target Locations

- `docs/v0.0.5/build-R7/40-r7.1-milestone-baseline.md`
- `docs/v0.0.5/build-R7/41-r7.1-persistent-map-store-design.md`
- `coe/2026-07-24-21-21-r7-canonical-map-persistence-ownership.md`
- `third_party/codex-cli/codex-rs/state/migrations/0030_taskspace_maps.sql`
- `third_party/codex-cli/codex-rs/state/src/runtime/taskspace_maps.rs`
- `third_party/codex-cli/codex-rs/state/src/runtime/taskspace_map_codec.rs`
- `third_party/codex-cli/codex-rs/core/src/session/taskspace_store.rs`
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
- `third_party/codex-cli/codex-rs/core/src/session/rollout_reconstruction.rs`
- `third_party/codex-cli/codex-rs/core/src/session/taskspace_terminal.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/cli/src/main.rs`

#### Change Introduction

候选复用 `codex-state` SQLite，增加 Map aggregate、thread binding 和 commit audit；Core 在 Runtime
candidate 通过 Store CAS 后安装 cache；删除 rollout snapshot/delta reducer、旧协议和 CLI replay 命令。

#### Risk Focus

- 第二事实源、旁路 mutation、错误旧引用和 Store 缺失 fallback。
- CAS、幂等、事务回滚、binding、resume/fork/child、projection 和 terminal 的生产完整性。
- Standard 模式侵入、测试专用路径掩盖生产缺口、日志字段残缺。

#### User-Perspective Review Focus

- Store 冲突、缺失和完整性错误是否给 Agent 返回忠实事实，不注入下一步决策。
- resume/fork/child 是否保持同一 Map 身份，且失败可理解、不可静默恢复。

#### Implementation Completeness Focus

- 逐项核对 A0.1-A0.7，区分生产接线与 schema/test scaffold。
- 检查所有 Map mutation、rollout reconstruction、CLI/protocol/Cargo 删除及实际入口。

#### Target Benefit Focus

- 可靠性目标：唯一事实源、崩溃/重启后持久性、并发不分叉。
- 非目标：本阶段不证明 R-21 handoff 已修复，也不比较三种 projection 的成本收益。

#### Assumptions To Attack

- Runtime cache 不会被 Store 外路径修改。
- 幂等重放和并发冲突不会安装错误 candidate。
- `cfg(test)` 分支不代表生产 fallback。
- 旧 snapshot/delta 术语仅存在于历史证据或负向断言。

#### Adversarial Lenses

- implementation-completeness
- state
- concurrency
- failure
- data
- maintenance
- testing
- observability

#### Verification Status

- `codex-state --lib`：122/122。
- TaskSpace State：7/7。
- Store hydration：2/2。
- terminal contract：2/2。
- CLI check 与格式检查通过。
- 旧恢复生产符号扫描为零。
- Core 全量仍有 R-21、环境相关 Guardian、model-refresh 和一次注入 key 后的栈溢出，未作为绿色门。

#### Reviewer Instructions

- 使用全新内部 subagent，不继承主 Agent 上下文。
- 直接读取目标文件，只读，不修改文件。
- 以反例和可复现证据为准，提供路径与行号。
- 重点发现代码残留、错误引用、双事实源和只落 scaffold 的计划项。

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User-recommended agent command: n/a
- User-recommended agent verification: n/a
- User approval requested: n/a
- User-approved CLI command: n/a
- User decision: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 分钟 | 最多一次 10 分钟 | 2 | 审查不可用时不得通过 |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | 当前最高风险是阶段被错误计为完成或旧路径仍留在生产 | 生产接线、双事实源、残留、测试有效性 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | `multi_agent_v1.spawn_agent`，`gpt-5.5` low | `019f94b8-b5a6-7b11-9b7b-b2587ae5c2ec` | spawn tool result | `fork_context=false` | Round 1 Review Input 和中立代码导航包 | 主 Agent 历史、reasoning、草稿、结论与完整 diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1 | implementation-completeness-adversary | 1 | `019f94b8-b5a6-7b11-9b7b-b2587ae5c2ec` | 约 12 分钟 | completed | reviewer 正常完成 | completed |

### Reviewer Outputs

#### R1

##### Summary

Store repository、CAS 和主 mutation 已落地，但 child 的生产接线仍存在三个阻塞缺口。

##### Blocking Findings

- TaskSpace resume/fork/child 在 Store 或 binding 缺失时仍可能返回 `Ok(None)`。
  - Broken assumption: TaskSpace 启动必定有 canonical binding。
  - Failure scenario: child 退回空 Session cache 后继续执行。
  - Trigger condition: child 启动时 State DB 或 parent binding 缺失。
  - Impact: 第二事实源和 silent fallback。
  - Proof needed: 无 Store/无 parent binding 的失败测试。
- child spawn 后 attach 失败被吞掉。
  - Broken assumption: spawn 成功意味着 child 已与 lease 原子绑定。
  - Failure scenario: orphan thread、registry slot 和开放 spawn edge 残留。
  - Trigger condition: `attach_child_action_map_binding` 返回错误。
  - Impact: handoff 状态分叉。
  - Proof needed: attach 失败后的 thread/slot/edge/Shutdown 断言。
- nested spawn 硬门直接读取 Session cache。
  - Broken assumption: cache 始终等于 Store。
  - Failure scenario:外部 revision 推进后按旧 binding 决策。
  - Trigger condition: Store 已更新而 Session cache 未刷新。
  - Impact: 错误拒绝或错误放行 nested spawn。
  - Proof needed: nested spawn 走 Store-backed read。

##### Non-blocking Risks

- 活跃 observer 脚本仍引用旧 `debug taskspace-replay`。

##### Implementation Completeness Checks

| Plan Item | Status | Finding |
|---|---|---|
| repository/CAS | landed | none |
| resume/fork/child | partial | R1-B1 |
| child attach | partial | R1-B2 |
| nested read | partial | R1-B3 |
| observer | partial | R1-N1 |

##### Required Fixes

- R1-B1：TaskSpace session 必须 fail closed，Standard child 不继承 binding。
- R1-B2：attach 失败必须原子回收 child。
- R1-B3：nested spawn 使用 Store-backed read。

##### Missing Tests

- child 无 Store、parent 无 binding、Standard child 隔离。
- attach 失败后的完整回收。

##### Missing Logs / Observability

- child binding/attach 失败需要结构化事实日志。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken |
|---|---|---|---|---|---|
| R1 | TaskSpace child 可无 Store/binding 继续 | blocking | accept | 与唯一事实源合同冲突 | `36395074e` 强制 resume/fork/child binding，Standard 隔离；新增 5 项 hydration 测试 |
| R1 | attach 失败被吞 | blocking | accept | 会遗留 orphan child | `abort_agent_after_spawn_failure` 关闭 edge、Shutdown、remove、release |
| R1 | nested spawn 读 cache | blocking | accept | cache 不是 authority | 新增 `read_canonical_action_map` 并接入 nested spawn |
| R1 | observer 旧 replay | non-blocking | accept | 活跃错误引用 | 进入 Round 2 专项闭合 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - `019f94d6-6deb-7110-8632-95c572520337`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Implementation completeness gaps resolved or accepted by user: yes
- Target benefit warnings recorded: yes
- Blocked reason: n/a
- Allowed to proceed: yes，进入 observer 残留修复

## Round 2: child binding closure 与活跃旧入口复审

### Review Input

- Objective: 复核 Round 1 三个 Rust blocking 是否关闭，并继续搜索 rollout replay 残留。
- Target: `36395074e` 及其 Store hydration、nested read、spawn abort、CLI/protocol/scripts 引用。
- Risk focus: silent fallback、orphan child、stale cache、旧 CLI 调用和测试缺口。
- Explicit exclusions: 主 Agent 历史、reasoning、结论和说服性 diff。
- Reviewer instructions: fresh、read-only、直接读代码、给出路径行号。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts |
|---|---:|---:|---:|
| complex | 20 分钟 | 10 分钟 | 2 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Read-only |
|---|---|---|---|---|---|
| blocking-closure-adversary | `multi_agent_v1.spawn_agent`，`gpt-5.5` low | `019f94d6-6deb-7110-8632-95c572520337` | spawn tool result | `fork_context=false` | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Status | Action |
|---|---|---:|---|---|---|
| R2 | blocking-closure-adversary | 1 | `019f94d6-6deb-7110-8632-95c572520337` | completed | completed |

### Reviewer Outputs

#### R2

##### Summary

Round 1 的三个 Rust blocking 均关闭；仍有一个活跃脚本 blocking。

##### Blocking Findings

- `export-action-map-observability.ps1` 仍 dot-source 旧 replay helper，并调用已删除的
  `whale debug taskspace-replay --rollout`。
  - Broken assumption: 旧恢复入口已全部删除。
  - Failure scenario: observer 在真实运行中直接失败，或诱使重新引入 rollout authority。
  - Trigger condition: benchmark/E2E 导出 TaskSpace observer。
  - Impact: 活跃工具不可用且架构倒退。
  - Proof needed: observer 直接按 `thread_id` 读取 Store，旧命令扫描为零。

##### Non-blocking Risks

- app-server 和个别 debug read 仍可能暴露 cache 视图。
- 缺少真实 observer Store export 集成测试。

##### Required Fixes

- 删除活跃脚本中的旧 replay 调用，不做兼容别名。

##### Missing Tests

- reference gate 覆盖所有活跃脚本调用。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Action Taken |
|---|---|---|---|---|
| R2 | observer 调用已删除 replay CLI | blocking | accept | `f8e3f67fb` 新增 `debug taskspace-map --thread-id`；`776671113` 全链迁移到 `source.mapStore` 并 fail closed |
| R2 | app-server/cache read 风险 | non-blocking | accept | 留给 Round 3 以生产入口证据复核 |
| R2 | 缺少真实 observer 集成测试 | non-blocking | accept | 留给 Round 3 验证 fixture 是否掩盖问题 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 3
- Allowed to proceed: yes

## Round 3: Store observer 与全局 read ownership 复审

### Review Input

- Objective: 验证 canonical Map 由独立 Store 唯一持有，rollout 不再恢复/重建，observer 直接读 Store。
- Review target: HEAD `776671113`，范围 `190371b23..776671113`。
- Target locations: State migration/repository、Core hydration/mutation/read、child attach、CLI export、PowerShell
  observer、benchmark extractor、fixture 和 reference gate。
- Risk focus: 旧 replay 符号、旁路 mutation/read、漏传 `thread_id`、错误 fallback、fake fixture 掩盖生产缺口、
  Standard 侵入和缺日志。
- Implementation completeness: A0.1-A0.7 的 production path、integration entry、test/log evidence、mock exposure。
- Target benefit: 唯一事实源、重启持久性、并发不分叉；成本不作为本轮 correctness blocking。
- Reviewer instructions: fresh internal session，no inherited context，read-only，尝试证伪并给出路径行号。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | 最终残留和错误引用专项 | Store ownership、observer、mock exposure |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | `multi_agent_v1.spawn_agent`，`gpt-5.5` low | `019f94eb-c908-7c93-ba3e-47d0952af755` | spawn tool result 与 subagent notification | `fork_context=false` | 本 Round Review Input | 主 Agent 历史、reasoning、草稿、结论和完整 diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Action |
|---|---|---:|---|---:|---|---|
| R3 | implementation-completeness-adversary | 1 | `019f94eb-c908-7c93-ba3e-47d0952af755` | 约 4 分钟 | completed | completed |

### Reviewer Outputs

#### R3

##### Summary

Blocking findings: none。Store schema、CAS、hydration、mutation、CLI export 和 fail-closed observer 已落地。

##### Non-blocking Risks

- PowerShell tests 仍从 rollout snapshot 合成 fake Store export，可能掩盖真实 CLI/SQLite/schema 失败。
- App Server `thread_action_map_read` / `thread_taskspace_read` 仍直接读 Session cache。
- `rollout_reconstruction` 命名可能诱导未来重新加入 Map aggregate recovery；当前代码只恢复 history/context events。

##### Implementation Completeness Checks

| Item | Status | Gap |
|---|---|---|
| A0.1 schema | landed | none |
| A0.2 CRUD/CAS/audit | landed | none |
| A0.3 hydration | landed | history-only reconstruction 命名风险 |
| A0.4 Store-first mutation | landed | test-only fallback |
| A0.5 resume/fork/child | landed | App Server freshness |
| A0.6 CLI export | landed | valid missing-binding test缺失 |
| A0.7 observer | partial | fake rollout-derived fixture |

##### Missing Tests

- 真实 CLI + SQLite + PowerShell observer。
- App Server read 在外部 Store revision 推进后刷新。
- Rust production 旧恢复符号门禁。
- valid thread/no binding CLI failure。

##### Missing Logs / Observability

- CLI export 成功/缺 binding 的结构化日志。
- App Server Store revision/freshness 日志。
- binding conflict 的 current/attempted map id 日志。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Action |
|---|---|---|---|---|
| R3 | fake fixture 从 rollout 合成 Store | non-blocking | accept | fixture 改为显式 snapshot；新增真实 SQLite + CLI + PowerShell 集成测试 |
| R3 | App Server 直接读 cache | non-blocking | accept | `CodexThread::action_map_snapshot` 改走 `read_canonical_action_map`，外部 CAS freshness 测试通过 |
| R3 | reconstruction 命名风险 | non-blocking | accept | 模块首部冻结 history/context-only 边界；既有测试断言 reconstruction 前后 Map 不变；Rust reference gate 禁止旧符号 |
| R3 | valid thread/no binding test | non-blocking | accept | CLI 集成测试新增明确失败且不产出 envelope |
| R3 | Rust production reference gate | non-blocking | accept | PowerShell machine gate 扫描 core/protocol/cli/state production source |
| R3 | CLI 与 binding conflict 日志 | non-blocking | accept | 新增 exported/missing-binding/binding-conflict 结构化事件 |
| R3 | App Server freshness metadata | non-blocking | accept | `taskspace.map_store_read` 已记录 map/relation/store/graph revision |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: n/a
- Blocking re-review completed: n/a
- Rejected findings backed by evidence: n/a
- Deferred findings documented: no
- Implementation completeness gaps resolved or accepted by user: yes
- Target benefit warnings recorded: yes
- Blocked reason: n/a
- Allowed to proceed: pending final review

## Final Conclusion

前三轮发现均已响应；等待基于最终提交的 Round 4 空白复审。
