# Problem P-001: exec 反馈把 shell 状态误表述为任务级退出状态
- Status: open
- Created: 2026-07-10 18:47
- Updated: 2026-07-10 21:10
- Objective: standard 与 TaskSpace 共用的 exec 反馈忠实暴露 transport、shell、termination 和 output/ref 机械事实，不解析正文、不替 Agent 判断任务成功或自动重试。
- Symptoms:
  - `conda install pytest -y 2>&1 | tail -20` 的上游 conda 失败，但模型反馈显示 `Exit code: 0`。
  - 同一反馈正文包含 `CondaToSPermissionError`，标题与正文形成表面冲突。
- Expected behavior:
  - 反馈明确区分 shell command/list 的退出状态与任务级成功判断。
  - timeout、cancel、signal、spawn failure 不折叠成普通 shell exit。
  - 管道阶段状态仅在无语义扰动且可靠可观测时暴露，否则明确 unavailable。
- Actual behavior:
  - `ExecToolCallOutput` 只有 `exit_code` 和 `timed_out`；freeform formatter 输出笼统的 `Exit code`。
  - event 和 function result 直接用 `exit_code == 0` 分类 Completed/Success。
- Impact:
  - Agent 需要从正文反推状态作用域，增加反馈歧义和无效环境探测风险。
  - TaskSpace NodeEvent/provider history 会忠实保留一个表达不充分的上游结果。
- Reproduction:
  - 检查 R5 G2 rollout 中 `call_00_fNkFDqtpkdrsMqDjVhEq7904`。
  - 本机执行 `false | true` 和 `yes | head`，对比 Bash 默认与 pipefail。
- Environment:
  - Linux/bash，branch `whalecode-alpha`，commit `4bc05bb`，R5 E5。
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
- Ruled out:
  - 不是 projection 丢失：后续 Agent 使用了前轮发现的 pytest 路径，相关输出均低于 raw-output 裁剪阈值。
  - 不能用全局 pipefail 直接修复：有意由 `head` 截断的上游 SIGPIPE 会变成 141。
- Fix criteria:
  - model-visible feedback 使用 `Shell exit code`，不再暗示任务级结论。
  - transport completion、timeout、cancel/signal、spawn failure 有独立机械表达和稳定日志。
  - structured/freeform、standard/TaskSpace 使用同一 outcome renderer。
  - stderr warning + shell 0 保持 0；不得扫描正文重写状态。
  - pipeline stage status 不可靠时明确 unavailable；不得伪造。
  - focused tests、core test/check 与 E5 样本通过。
- Current conclusion: H-001/H-003 修复已通过 focused 和 workspace 回归：exec 结果按机械 outcome 分类，非正常退出不再发布合成 shell code，user decline 与 non-user rejection 由类型区分。待 E5 真实样本通过后关闭 P-001。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - E-001
  - E-002
  - E-003
- Close reason:
  - not closed

## Hypothesis H-001: 单一 exit_code 字段丢失了状态作用域
- Status: confirmed
- Parent: P-001
- Claim: exec substrate 返回的是 shell process/list 状态，但 formatter 和事件字段把它笼统命名为 exit code，并缺少 transport/termination 维度，导致模型无法仅凭结构区分 shell 事实与任务结论。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Bash 默认管道状态来自最后命令；当前协议只有一个 scalar exit code。
- Falsifiable predictions:
  - If true: `ExecToolCallOutput` 无状态作用域和 termination enum，formatter 输出 `Exit code`，event 直接按 0/非0分类。
  - If false: provider-visible result 应已经区分 shell/transport/pipeline facts。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查 protocol、formatter、ToolEmitter 和真实 rollout。
  - Signal: 字段、标题、状态分类与模型可见原文。
  - Capture method: 静态代码对账和 rollout call/output 对账。
  - Event name or marker:
    - `ExecCommandEndEvent`
  - Correlation keys:
    - `call_id`
  - Differentiates from:
    - projection omission 或大输出裁剪。
  - Supports if:
    - 只有 scalar exit code 且 provider 标题无 shell scope。
  - Refutes if:
    - 已存在结构化 transport/shell/termination 事实。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - promote after repair
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 实现共享 outcome renderer 与事件字段。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 全局 pipefail 可以无负作用修复反馈
- Status: refuted
- Parent: P-001
- Claim: 对所有 Agent shell command 默认启用 pipefail，可以忠实表达失败且不改变合法命令语义。
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - pipefail 能让 `false | tail` 返回上游非零，但可能把 SIGPIPE 提升为失败。
- Falsifiable predictions:
  - If true: `yes | head` 在 pipefail 下仍返回成功或存在无歧义通用豁免。
  - If false: pipefail 会返回上游 SIGPIPE 141，改变默认 shell 结果。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对比 Bash 默认和 pipefail 的 `false | true`、`yes | head`。
  - Signal: shell return code 和 `PIPESTATUS`。
  - Capture method: 本机 bash probe。
  - Event name or marker:
    - none
  - Correlation keys:
    - probe command
  - Differentiates from:
    - 单纯 formatter 命名问题。
  - Supports if:
    - 两类 pipeline 均可无副作用提升上游非零。
  - Refutes if:
    - 有意截断被改成非零。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: refuted；全局 pipefail 会改变合法 shell 语义。
- Repair design readiness: closed
- Next step: 不采用全局 pipefail。
- Blocker:
  - none
- Close reason:
  - refuted

## Evidence E-001: 真实 pytest 样本显示 shell 0 与上游失败并存
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `/home/zhangxu/.whale/sessions/2026/07/10/rollout-2026-07-10T07-45-52-019f4946-031c-72b3-a929-ea86d00cc1f0.jsonl`
- Prediction or plan link:
  - H-001 If true
- Matched signal:
  - `call_00_fNkFDqtpkdrsMqDjVhEq7904` 输出 shell exit 0，正文为 conda 网络/权限失败。
- Correlation keys:
  - `call_00_fNkFDqtpkdrsMqDjVhEq7904`
- Raw content:
  ```text
  command: conda install pytest -y 2>&1 | tail -20
  Exit code: 0
  CondaToSPermissionError: Unable to read/write path (...)
  ```
- Interpretation: runtime 忠实返回 Bash 默认 pipeline 状态，但当前反馈没有表达其 shell/last-pipeline scope。
- Time: 2026-07-10 18:47

## Evidence E-002: 协议和 formatter 只有 scalar exit code
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `protocol/src/exec_output.rs`、`core/src/tools/mod.rs`、`core/src/tools/events.rs`
- Prediction or plan link:
  - H-001 diagnostic evidence plan
- Matched signal:
  - `ExecToolCallOutput { exit_code, ..., timed_out }`；freeform 输出 `Exit code`；event 用 `exit_code == 0` 选择 Completed/Failed。
- Correlation keys:
  - `ExecToolCallOutput`
  - `ExecCommandEndEvent`
- Raw content:
  ```text
  sections.push(format!("Exit code: {}", exec_output.exit_code));
  status: if output.exit_code == 0 { Completed } else { Failed }
  ```
- Interpretation: feedback schema 缺少状态作用域和独立 termination facts。
- Time: 2026-07-10 18:47

## Evidence E-003: pipefail 会把有意 SIGPIPE 提升为失败
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: probe
- Source: 本机 Bash probe
- Prediction or plan link:
  - H-002 If false
- Matched signal:
  - 默认 `yes | head` shell rc=0、PIPESTATUS=141 0；pipefail rc=141。
- Correlation keys:
  - `yes | head`
- Raw content:
  ```text
  false | true       -> rc=0   PIPESTATUS=1 0
  yes | head         -> rc=0   PIPESTATUS=141 0
  pipefail yes|head  -> rc=141 PIPESTATUS=141 0
  ```
- Interpretation: 全局 pipefail 会改变 shell 行为，不能作为反馈层通用修复。
- Time: 2026-07-10 18:47

## Hypothesis H-003: 拒绝来源合并使事件状态扭曲为用户拒绝
- Status: confirmed
- Parent: P-001
- Claim: `ToolError::Rejected` 同时表示用户拒绝与参数、策略、运行时拒绝，`ToolEmitter` 把它们全部转成 `ExecCommandStatus::Declined`，使事件层丢失真实拒绝来源。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 错误类型没有保留拒绝主体，事件 formatter 无法忠实转发。
- Falsifiable predictions:
  - If true: invalid args、policy deny、guardian timeout 和 user deny 会进入同一 variant，并产生相同 `Declined` 状态。
  - If false: 错误契约或事件已区分 user decline 和 non-user rejection。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对账 `ToolError` 所有构造点与 `ToolEmitter::finish` 分支。
  - Signal: variant 类型、调用来源和最终 `ExecCommandStatus`。
  - Capture method: 静态路径审计与 focused event test。
  - Event name or marker:
    - `ExecCommandEndEvent`
  - Correlation keys:
    - `call_id`
  - Differentiates from:
    - 正文裁剪或 projection 改写。
  - Supports if:
    - non-user rejection 与 user decline 共用 variant 并都输出 `Declined`。
  - Refutes if:
    - 事件状态已保留拒绝主体。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - retain typed outcome logs
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 拆分 user-declined 与 non-user rejected，禁止从 message 文本反推来源。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-004: 一个 Rejected variant 覆盖多种拒绝主体
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `core/src/tools/sandboxing.rs`、`core/src/tools/orchestrator.rs`、`core/src/tools/runtimes/unified_exec.rs`、`core/src/tools/events.rs`
- Prediction or plan link:
  - H-003 If true
- Matched signal:
  - empty args、policy forbidden、guardian timeout、user deny 均构造 `ToolError::Rejected(String)`；`ToolEmitter::finish` 将该 variant 统一转成 `ToolEventFailure::Rejected`，最终 exec event 状态为 `Declined`。
- Correlation keys:
  - `ToolError::Rejected`
  - `ToolEventFailure::Rejected`
- Raw content:
  ```text
  enum ToolError { Rejected(String), Codex(CodexErr) }
  ToolEventFailure::Rejected(...) -> ExecCommandStatus::Declined
  ```
- Interpretation: 这是类型契约丢失，不能通过解析 message 补回。
- Time: 2026-07-10 20:05

## Evidence E-005: 共享 exec outcome 契约已贯通到模型历史和 UI
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `protocol/src/exec_output.rs`、`protocol/src/protocol.rs`、`core/src/tools/events.rs`、`core/src/tools/mod.rs`、`app-server-protocol/src/protocol/v2.rs`、`tui/src/exec_cell/model.rs`
- Prediction or plan link:
  - P-001 Fix criteria
  - H-001 If true
  - H-003 Next step
- Matched signal:
  - `ExecOutcome` 区分 exited/timed_out/cancelled/signaled/spawn_failed/rejected/execution_error。
  - `shell_exit_code` 仅在 outcome=exited 时可用；`ExecCommandEndEvent` 改为 `Option<i32>`，timeout/cancel/reject 均为 `None`。
  - structured/freeform 共用 `ExecOutputMetadata`，管道 stage status 无法无扰动采集时明确 `unavailable`。
  - `ToolError::UserDeclined` 与 `ToolError::Rejected` 分离，事件状态分别为 Declined 与 Failed，不解析 message 推断拒绝主体。
  - TaskSpace 删除了根据 tool display text 中的 `Exit code` 自动中断 Agent 已请求 action sequence 的路径。
- Correlation keys:
  - `ExecOutputMetadata`
  - `ExecCommandEndEvent.shell_exit_code`
  - `ToolError::UserDeclined`
- Raw content:
  ```text
  Execution outcome: timed_out
  Shell exit code: unavailable
  Pipeline stage exit codes: unavailable
  Termination signal: unavailable
  ```
- Interpretation: 反馈层只构造可观测事实；不重写正文、不推断任务成功、不替 Agent 中断或重试。
- Time: 2026-07-10 21:10

## Evidence E-006: focused、历史、TaskSpace 和 workspace 回归通过
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: test-result
- Source: 本地 Cargo 回归
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - `cargo test -p codex-protocol`: 200 passed。
  - `cargo test -p codex-core tools::tests --lib`: 8 passed。
  - `cargo test -p codex-core exec::tests --lib`: 64 passed, 2 ignored。
  - user-shell unit/integration: 6 + 7 passed。
  - app-server thread history: 30 passed。
  - TUI command execution replay: 4 passed。
  - timeout focused: 2 passed；rejection approval focused: 1 passed；large-output truncation focused: 4 passed。
  - realistic TaskSpace action-map scenario: 1 passed。
  - `cargo check --workspace --tests`: passed。
- Correlation keys:
  - `tool.exec_outcome_recorded`
  - `request_permissions_denied`
  - `shell-truncated`
- Raw content:
  ```text
  workspace check: Finished dev profile
  focused failures after correction: 0
  ```
- Interpretation: 新契约已覆盖 launcher、renderer、history、event、app-server 和 TUI；真实 Agent 样本仍是 P-001 的最后关闭门禁。
- Time: 2026-07-10 21:10
