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

## 5. 当前结论

只看工程代码和非 agent gate：

```text
v0.0.5 engineering implementation: CODE-COMPLETE CANDIDATE
```

但从版本产品目标看：

```text
v0.0.5 release closeout: NOT PROVEN
```

下一步如果要进入真实样本验证，必须先生成与当前 commit 绑定的 code-complete / non-agent gates / user approval marker，然后由 E3 start gate 明确给出 `full_e3_allowed=true`。
