# R5 PhaseB 最小 NodeEvent 契约执行记录

> PhaseB 目标：把 TaskSpace 工具反馈从旧 `NodeResult` 主路径收敛为 node-local
> `NodeEvent`，为后续 thin projection 提供忠实事件来源；不做兼容层、不双写。

## 1. 状态

```text
Phase: R5-B
Status: implemented, live sample blocker open
Updated: 2026-07-09
Primary code:
  third_party/codex-cli/codex-rs/core/src/action_map/map.rs
  third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
  third_party/codex-cli/codex-rs/core/src/session/turn.rs
  third_party/codex-cli/codex-rs/protocol/src/protocol.rs
COE:
  coe/2026-07-09-05-20-r5-budget-feedback-grace.md
```

PhaseB 的代码契约已经落地：ordinary main tool feedback 直接进入 `NodeEvent`，
projection/debug snapshot 从 `node_events` 读取 recent feedback、refs 和 artifact refs。
本阶段仍未关闭 live sample gate，因为 `count-call-stack` 暴露了请求预算对 TaskSpace
多节点生命周期的截断。

## 2. 实现内容

| Area | Change | Boundary |
|---|---|---|
| map schema | 增加 `NodeEvent`、`NodeEventRef`、`node_events` map | 事件归档，不表达语义 truth |
| tool feedback | `record_main_tool_result*` 返回 `node-event-*` 并写入 `NodeEvent` | main tool 不再写旧 `NodeResult` |
| snapshot protocol | `ActionMapSnapshotMap.node_events` 和 node `node_event_ids` | viewer/debug 可读 |
| projection read path | recent tool feedback、result refs、artifact refs 改读 `node_events` | 不重新解释工具语义 |
| budget feedback | 修复 pre-limit `budget_recovery` 误消耗 post-budget grace | 只保证反馈交付窗口，不放宽状态机 |
| hard rejection feedback | 状态机硬拒绝在预算临界点可获得一次 feedback follow-up | 拒绝仍然拒绝，规则不变 |

## 3. 测试证据

已通过：

```text
cargo fmt --all
cargo test -p codex-core main_tool_feedback_records_node_event_without_node_result -- --nocapture
cargo test -p codex-core active_projection_preserves -- --nocapture
cargo test -p codex-core taskspace_active_budget_allows_one_budget_recovery_grace_request -- --nocapture
cargo test -p codex-core provider_request_budget_explicit_budget_recovery_phase_remains_advisory -- --nocapture
cargo test -p codex-core provider_request_budget_allows_rebuilt_context_after_recovery_grace_spent -- --nocapture
cargo test -p codex-core post_budget_grace_counter_ignores_pre_limit_budget_recovery_request -- --nocapture
cargo test -p codex-core actionable_feedback_at_rollout_limit_requests_budget_recovery_followup -- --nocapture
cargo test -p codex-core state_machine_rejection_at_rollout_limit_requests_budget_recovery_followup -- --nocapture
cargo check -p codex-core
cargo build -p codex-cli --bin whale
```

`cargo fmt --all` 仍输出既有 stable Rust 警告：
`can't set imports_granularity = Item`。该警告不影响格式化退出码。

## 4. PhaseB 样本对比

本阶段按 R5 规则选择 `count-call-stack` 作为 tool feedback / node lifecycle 样本，
执行 1 次 paired E1 诊断；修复过程中为验证反馈截断又追加两次同样本定向 rerun。
这些结果不计入 utility aggregate。

| Baseline | Run | Standard | TaskSpace / R5 | 结论 |
|---|---|---:|---:|---|
| R4-D 历史 | `target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136` | solved, 138205ms, 20 tools | solved, 154525ms, 11 tools | R4 证明该样本可被 TaskSpace 解决 |
| R5-B 初跑 | `target/r5-phaseB-samples/count-call-stack/20260709-045241-275` | solved, 39962ms, 17 tools | wrong, 58014ms, 5 tools | failed `apply_patch` 已入 `node-event-5`，但反馈被 hard stop 截断 |
| R5-B grace 修复后 | `target/r5-phaseB-samples-after-grace/count-call-stack/20260709-050448-661` | solved, 20214ms, 9 tools | wrong, 152930ms, 5 tools | 状态机拒绝反馈在预算临界点仍可能被截断 |
| R5-B rejection follow-up 后 | `target/r5-phaseB-samples-after-rejection-grace/count-call-stack/20260709-051151-818` | solved, 19735ms, 11 tools | wrong, 37838ms, 6 tools | Agent 成功 `finish_node` 到 implement，但全局请求预算 7/6 后截断，未执行 patch |

第三次 R5 rerun 的关键变化：

```text
before: hard stop 发生在工具失败或状态机拒绝反馈交付前
after: Agent 能看到后续反馈，并完成 inspect -> implement 状态转移
remaining: implement 节点创建后没有预算再进行 patch
```

## 5. 根因收敛

已修复：

1. pre-limit `budget_recovery` 被错误计入 post-budget grace。
2. `TaskSpaceActionV1 rejected` 等硬拒绝反馈在预算临界点没有 follow-up 机会。

仍未关闭：

```text
request budget lifecycle cliff:
  verification_first profile max_rollout_model_requests=6
  count-call-stack 在 R5-B 当前路径需要 inspect -> implement
  Agent 在 inspect 内完成必要读取后，于 request_count=7/6 才创建 implement node
  hard stop 立即阻断 implement patch
```

这不是 PhaseB NodeEvent 契约失败：读取结果、工具结果和状态机反馈都进入了上下文。
它是预算 hard baseline 与多节点 TaskSpace 生命周期的边界问题，建议在 R5-C/E 之间单独收敛：
预算只能保护资源底线，不能把已经完成的状态转移截断成不可执行半成品。

## 6. 操作记录

样本运行使用 `.env.local` 中的 `DEEPSEEK_API_KEY`，命令形态：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 `
  -Scenario count-call-stack `
  -Repeats 1 `
  -RunSide both `
  -Model deepseek-v4-flash `
  -SandboxMode full-auto
```

经验记录：

1. paired benchmark 的 `whale-exec.jsonl` 位于 `pair-001/right/artifacts/whale-exec.jsonl`，不是 `pair-001/right/whale-exec.jsonl`。
2. E1 单次样本返回非零时仍会写出 `pair-report.md`、`run-summary.md` 和 artifacts，应先读报告再判断失败层。
3. R5 反馈问题要先看 `node-event-*`、`provider-request-events.jsonl`、`TaskSpaceProviderBudgetHardStopV1` 的相对顺序。
