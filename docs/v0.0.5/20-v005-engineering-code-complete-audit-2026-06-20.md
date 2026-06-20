# v0.0.5 工程代码完成度审计

- 日期：2026-06-20
- 范围：只审计 v0.0.5 目标相关工程实现、非 agent 单测和脚本 gate
- 明确排除：没有运行真实 E3，没有运行真实 agent benchmark sample
- 当前判断：工程代码路径与非 agent gate 已闭合；真实效果仍必须通过后续 targeted diagnostic 和正式 E3 证明

## 1. 本轮补齐内容

### 1.1 provider-visible tool-call/output 配对修复

修复文件：

- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`

修复内容：

- active context replacement 过滤 provider-visible history 时，不再单独移除 tool output 或 tool call。
- 当一个 `function_call_output` 被 active replacement 省略时，匹配的 assistant function call 也会被省略。
- 当一个 `function_call` 被省略时，匹配的 tool output 也会被省略。
- 避免 ChatCompletions 请求出现 orphan tool call / orphan tool output。

提交：

- `296f11ab4 fix: preserve provider tool history pairs`

### 1.2 spawn_agent 测试与 Whale 模型过滤约束对齐

修复文件：

- `third_party/codex-cli/codex-rs/core/tests/suite/subagent_notifications.rs`

修复内容：

- 测试模型从 `gpt-*` 改为 `deepseek-*` 前缀，符合 Whale 当前模型列表只暴露 DeepSeek 系列的过滤规则。
- spawn override 测试注入静态 model catalog，覆盖 inherited / requested / role 三类模型。
- 子线程 snapshot 获取不再依赖 `ThreadManager.list_thread_ids()` 的时序轮询，而是从 `spawn_agent` tool output 的 `agent_id` 读取。
- role locked settings 文案断言改为使用测试常量，避免再次与模型常量漂移。

提交：

- `6c987f062 test: align spawn agent model override coverage`

### 1.3 budget / skill alias 回归修复

修复文件：

- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/tests/suite/client.rs`

修复内容：

- `trace_event_is_emitted_before_barrier_event_when_budget_is_exhausted` 不再依赖固定事件下标或 `trace-{n}` 序号。
- 该测试现在验证真正的契约：预算耗尽对应的 trace event 必须先于 maintenance barrier event。
- skill alias 测试不再把“短路径别名存在”和“预算压力下保留 skill 描述”绑死；当前契约只要求 `r0/.../SKILL.md` alias 生效。

## 2. 非 agent 验证结果

已通过 Rust focused tests：

```text
cargo test -p codex-core active_context_replacement -- --nocapture
cargo test -p codex-core provider_request_budget -- --nocapture
cargo test -p codex-core state_commit -- --nocapture
cargo test -p codex-core projection -- --nocapture
cargo test -p codex-core output_reference -- --nocapture
cargo test -p codex-core spawn_agent -- --nocapture
cargo test -p codex-core budget -- --nocapture
```

已通过 PowerShell fixture gates：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\v005-continuation-harness-selftest-2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-continuation-release-selftest-2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-continuation-start-gate-selftest-2
```

说明：

- `cargo fmt` 通过，但仍输出仓库既有的 stable rustfmt 对 `imports_granularity = Item` 的 warning。
- 并行运行部分 cargo / PowerShell gate 时曾出现锁等待或超时；串行重跑后通过。
- 这些 gate 不调用真实 agent，也不能替代 Terminal-Bench 效果结论。

## 3. v0.0.5 工程模块完成度

| 模块 | 当前工程状态 | 证据 |
|---|---|---|
| provider request lifecycle / budget event | 已实现并通过 focused test | `provider_request_budget`、`budget` |
| active context replacement | 已实现并通过 focused test | `active_context_replacement`、`projection` |
| tool-call/output protocol invariant | 已修复并通过 focused test | `active_context_replacement` |
| output reference | 已实现并通过 focused test | `output_reference` |
| state_commit | 已实现并通过 focused test | `state_commit` |
| state_commit displacement artifact / gate | 已实现并通过 fixture test | `test-cost-instrumentation.ps1`、`test-release-decision.ps1` |
| spawn/node budget | 已实现并通过 focused / fixture test | `spawn_agent`、`budget`、`test-cost-instrumentation.ps1` |
| release decision gate | 已实现并通过 fixture test | `test-release-decision.ps1` |
| E3 start gate | 已实现并通过 fixture test | `test-e3-start-gate.ps1` |
| 实验命名与证据等级制度 | 已建立 | `docs/experiments/taskspace-evidence-levels-and-samples.md` |

## 4. 仍未证明的部分

以下不是“代码还没写完”，而是“效果尚未通过真实样本证明”：

- `terminal-bench_E3-P0_3_5` 尚未在当前代码上正式运行。
- TaskSpace 成本是否达到 v0.0.5 目标，尚未由真实 Terminal-Bench P0 结果证明。
- TaskSpace 正确率是否相对 Standard / v0.0.4 clean 口径未下降，尚未由正式 E3 证明。
- 后续真实 run 必须先由 start gate、code-complete marker、non-agent gate marker 和用户批准共同放行。

## 5. 2026-06-20 当前态复核

本节记录在 commit `bf9471c26` 之后重新执行的当前态非 agent gate。该复核不包含真实 E3，不包含真实 agent benchmark sample。

Rust focused tests：

```text
cargo test -p codex-core active_context_replacement -- --nocapture
cargo test -p codex-core provider_request_budget -- --nocapture
cargo test -p codex-core state_commit -- --nocapture
cargo test -p codex-core output_reference -- --nocapture
cargo test -p codex-core projection -- --nocapture
cargo test -p codex-core budget -- --nocapture
```

结果：全部通过。

PowerShell fixture gates：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-current-release-selftest
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1 -RunRoot target\v005-current-harness-selftest
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-current-start-gate-selftest
```

结果：全部通过。

工作区状态：

```text
git status --short
```

结果：无未提交源码改动。

## 6. 当前结论

2026-06-20 对抗性审查发现 3 个 blocking 工程缺口：

- active budget 只在 hard cap 阻断，compact checkpoint soft state 没有真实 dispatch response。
- `state_commit` 仍不是 active profile 下 direct legacy state mutation 的强制替代路径。
- non-agent gate subgate evidence 的 `git_commit` 没有强制绑定当前 HEAD。

已在当前工作区补齐的修复：

- provider request budget 在 `compact_checkpoint_required` 状态下会阻断普通 provider dispatch，只允许 `budget_recovery` / `final_synthesis` / `final_abort` 收敛请求继续。
- `ActionMapProviderRequestBudgetSnapshot` 携带 runtime-derived `request_phase`，`turn.rs` 使用 snapshot phase，不再硬编码所有请求为 `model_sampling`；`FinalSynthesis` 节点会产生 `final_synthesis` phase。
- `taskspace_control` handler 拒绝 direct legacy state mutation action，要求使用 `state_commit`；`start_task` 的 initial fields 和 `state_commit` 保持可用。
- E3 start gate 和 release decision 均要求 non-agent subgate evidence 的 `git_commit` 等于当前 HEAD；code-complete marker 在 release decision 中也必须等于当前 HEAD。
- start gate 和 release decision 均增加 stale code-complete `git_commit` 负向 fixture。

修复后已通过：

```text
cargo test -p codex-core provider_request_budget -- --nocapture
cargo test -p codex-core provider_request_budget_snapshot_uses_final_synthesis_phase -- --nocapture
cargo test -p codex-core active_profile_rejects_direct_legacy_state_action -- --nocapture
cargo test -p codex-core active_profile_allows_state_commit_action -- --nocapture
cargo test -p codex-core state_commit -- --nocapture
cargo test -p codex-core budget -- --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1 -RunRoot target\v005-review-release-selftest
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 -RunRoot target\v005-review-start-gate-selftest
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1
```

只看工程代码和非 agent gate，并结合 Round 3 对抗性闭环审查结果，修复后状态为：

```text
v0.0.5 engineering implementation: CODE-COMPLETE AFTER ADVERSARIAL CLOSURE REVIEW
```

但从版本产品目标看：

```text
v0.0.5 release closeout: NOT PROVEN
```

下一步如果要进入真实样本验证，必须先生成与当前 commit 绑定的 code-complete / non-agent gates / user approval marker，然后由 E3 start gate 明确给出 `full_e3_allowed=true`。

## 7. 2026-06-20 Round 3 对抗性闭环审查

审查报告：`vs_review/2026-06-20-v005-engineering-completion-review.md`

Round 3 使用新的只读 implementation-adversary 子审查会话，针对 Round 1 和 Round 2 的 accepted blocking findings 重新检查。结论：

- active budget soft response：已关闭。`compact_checkpoint_required` 下普通 provider dispatch 会被阻断，`final_synthesis` 等收敛 phase 通过 runtime snapshot 进入真实 attribution。
- `state_commit` active-profile path：已关闭。direct legacy state mutation action 在 handler 层被拒绝，`state_commit` 与 `start_task` 初始字段路径保留。
- HEAD-bound non-agent gates：已关闭。E3 start gate 与 release decision 要求 non-agent evidence 和 code-complete marker 绑定当前 HEAD，并具备 stale commit 负向 fixture。

剩余风险是非 blocking：尚无完整 streamed-turn integration test 覆盖 final-synthesis provider lifecycle attribution；当前证明来自 focused Rust tests 与 PowerShell fixture gates。
