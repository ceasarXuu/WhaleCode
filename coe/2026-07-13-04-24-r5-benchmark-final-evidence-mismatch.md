# Problem P-001: benchmark 完成态证据自相矛盾
- Status: fixed
- Created: 2026-07-13 04:24
- Updated: 2026-07-13 04:32
- Objective: 让 benchmark lifecycle 指标忠实识别 rollout 中已经发生的 Agent 完成事件。
- Symptoms:
  - 同一 TaskSpace artifact 的 rollout 记录 `final_candidate` 和 `task_complete`，但 `metrics.json` 与 pair report 标记 `agent_incomplete`。
- Expected behavior:
  - 同一证据源中存在有效 `task_complete` 且无 interruption 时，完成状态应为 complete，并保留准确来源。
- Actual behavior:
  - CLI exit=0、外部验证通过、rollout 有 `task_complete`，指标仍为 incomplete/none。
- Impact:
  - R5-J7.5 工具收益样本被错误排除，correctness 与工程洁净度门禁产生假阴性。
- Reproduction:
  - 读取 `target/r5-j7-5-contract-billing/subscription-billing-repair/20260713-041900-801/pair-001/right/artifacts` 中的 rollout、metrics 与 pair report。
- Environment:
  - Linux Docker benchmark，branch `whalecode-alpha`，commit `30bb1c0`，DeepSeek `deepseek-v4-flash`。
- Known facts:
  - rollout 最后记录 `provider_response_actionability=final_candidate` 和 `task_complete`。
  - `metrics.json` 同时记录 `agent_completion_status=incomplete`、`agent_final_observed=false`、`agent_completion_source=none`。
  - exec_exit_code=0、sampling_interrupted=false、public/hidden validation 均通过。
- Ruled out:
  - none
- Fix criteria:
  - 原始复现 artifact 重算后为 complete；focused fixture 覆盖 task_complete、final candidate、interruption 优先级；benchmark 回归通过。
- Current conclusion: H-001 已确认：完成态提取器不消费 rollout 的 `task_complete` 和 actionability trace；H-002 已由文件时序与调用顺序反驳。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-001 confirmed by E-002 and repaired; E-004/E-005 verify the fixture and original artifact.
- Close reason:
  - rollout lifecycle events are now consumed directly without inference from external validation.

## Hypothesis H-001: lifecycle 提取器漏读 rollout task_complete
- Status: confirmed
- Parent: P-001
- Claim: metrics 完成态提取只识别旧来源或特定 TaskSpace terminal carrier，没有消费 rollout 顶层 `event_msg.task_complete`。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 最终 artifact 已包含完成事件，但生成指标没有识别。
- Falsifiable predictions:
  - If true: 完成态解析代码不扫描该事件，或 focused fixture 用现有 artifact 重算仍得到 incomplete。
  - If false: 解析器明确扫描该事件且用同一最终 rollout 重算得到 complete。
- Diagnostic evidence plan:
  - Prediction or clause under test: 完成态解析路径是否消费 `event_msg.task_complete`。
  - Signal: parser 分支、输入源和原 artifact 重算结果。
  - Capture method: 静态追踪 `agent_final_observed` 生产路径，并直接重跑指标生成器。
  - Event name or marker:
    - task_complete
  - Correlation keys:
    - turn_id 019f57fc-8e02-78f2-801d-9fefa9249331
  - Differentiates from:
    - H-002 artifact 采集时序
  - Supports if:
    - parser 未消费事件，或最终 rollout 重算仍错误。
  - Refutes if:
    - parser 消费事件且最终 rollout 重算正确。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-004
  - E-005
- Conclusion: `Get-TaskspaceAgentCompletionEvidence` 只从 whale-exec 取 terminal message/actionability，并只从 rollout 识别 `message.phase=final_answer`；它没有识别真实存在的 `event_msg.task_complete` 或 `provider_response_actionability` trace。
- Repair design readiness: ready；用户已通过“执行 J7.5”授权修复阻断性缺口
- Next step: 在同一提取器中机械消费 rollout 完成事件与 actionability tag，增加 focused fixture 后重算原 artifact。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: metrics 在最终事件写入前生成
- Status: refuted
- Parent: P-001
- Claim: runner 在 Agent 进程/rollout 完成前生成 metrics，随后 artifact rollout 继续写入 task_complete，形成时序不一致。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 最终 rollout 与 metrics 内容不一致也可能来自采集先后顺序，而不是 parser 漏分支。
- Falsifiable predictions:
  - If true: runner 调用图或文件时间显示 metrics 生成早于 rollout 最终落盘，使用最终 rollout 重算会变为 complete。
  - If false: metrics 在 Agent 完成并复制最终 rollout 后生成，且重算仍错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: lifecycle 指标生成与 Agent/rollout finalize 的顺序。
  - Signal: runner 调用图、文件 mtime、重算前后差异。
  - Capture method: 检查 runner 脚本和 artifact 时间戳，并用最终 artifact 重跑提取器。
  - Event name or marker:
    - task_complete
  - Correlation keys:
    - pair-001/right
  - Differentiates from:
    - H-001 parser 漏读
  - Supports if:
    - metrics 先于 rollout finalize，且重算修正状态。
  - Refutes if:
    - 顺序正确且重算仍错误。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: metrics 的 mtime 晚于最终 rollout 约 3.19 秒，调用顺序也在 Agent 执行完成后生成 metrics；不是早采集。
- Repair design readiness: not applicable
- Next step: closed
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 同一最终 artifact 的完成态冲突
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `target/r5-j7-5-contract-billing/subscription-billing-repair/20260713-041900-801/pair-001/right/artifacts`
- Prediction or plan link:
  - P-001 症状复现；尚不区分 H-001 与 H-002。
- Matched signal:
  - rollout `task_complete` 与 metrics `agent_completion_status=incomplete` 同时存在。
- Correlation keys:
  - turn_id 019f57fc-8e02-78f2-801d-9fefa9249331
- Raw content:
  ```text
  rollout: provider_response_actionability=final_candidate
  rollout: event_msg.type=task_complete
  metrics: agent_completion_status=incomplete
  metrics: agent_final_observed=false
  metrics: agent_completion_source=none
  exec_exit_code=0; sampling_interrupted=false; external_validation_status=passed
  ```
- Interpretation: 证明 lifecycle 观测链存在假阴性，但尚不能区分 parser 漏读与采集时序。
- Time: 2026-07-13 04:24

## Evidence E-002: 完成态提取器没有 rollout lifecycle 分支
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `scripts/taskspace-benchmark/lib/metrics-extractor.ps1::Get-TaskspaceAgentCompletionEvidence`
- Prediction or plan link:
  - H-001：完成态解析路径是否消费 `event_msg.task_complete`。
- Matched signal:
  - 函数没有 `task_complete` 分支；rollout 循环只识别 canonical `message.phase=final_answer`，actionability 只从 whale-exec error 文本提取。
- Correlation keys:
  - function Get-TaskspaceAgentCompletionEvidence
- Raw content:
  ```text
  $taskspaceFinalCandidateObserved = true only when responseItem.type=message,
  role=assistant and phase=final_answer.
  No task_complete branch exists.
  ```
- Interpretation: 直接证明最终 rollout 中的机械完成事件被提取器遗漏，解释原 artifact 的假阴性。
- Time: 2026-07-13 04:29

## Evidence E-004: completion extractor focused regression passes
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `test-r4-metrics-extractor-large-rollout.ps1` and `test-metrics-extractor-harness.ps1`
- Prediction or plan link:
  - P-001 fix criteria：task_complete、final candidate 与现有 completion fixture 均通过。
- Matched signal:
  - 两个 focused harness 均 PASS；新增断言验证 source=`task_complete_event`、actionability=`final_candidate`。
- Correlation keys:
  - completion/taskspace-task-complete-rollout.jsonl
- Raw content:
  ```text
  PASS: R4 metrics extractor large rollout gate passed
  TaskSpace metrics extractor harness self-test: PASS
  ```
- Interpretation: 修复没有破坏旧 completion 路径，并覆盖新正式 lifecycle 事件。
- Time: 2026-07-13 04:32

## Evidence E-005: 原始失败 artifact 重算为 complete
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `Get-TaskspaceAgentCompletionEvidence` 对 J7.5 billing 原 artifact 的直接重算
- Prediction or plan link:
  - P-001 fix criteria：原始复现 artifact 重算后为 complete。
- Matched signal:
  - agent_final_observed=true，source=task_complete_event，actionability=final_candidate。
- Correlation keys:
  - turn_id 019f57fc-8e02-78f2-801d-9fefa9249331
- Raw content:
  ```json
  {"agent_final_observed":true,"agent_completion_source":"task_complete_event","last_agent_message_source":"agent_message","agent_message_count":5,"last_provider_response_actionability":"final_candidate"}
  ```
- Interpretation: 原始假阴性在同一输入证据上已消失，修复直接命中根因。
- Time: 2026-07-13 04:32

## Evidence E-003: metrics 在 rollout 最终落盘之后生成
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: diagnostic-log
- Source: `stat` on J7.5 billing artifacts and runner metrics write path
- Prediction or plan link:
  - H-002：metrics 是否早于 rollout finalize。
- Matched signal:
  - rollout mtime `04:20:41.215`，metrics mtime `04:20:44.405`；metrics 晚约 3.19 秒。
- Correlation keys:
  - pair-001/right
- Raw content:
  ```text
  04:20:41.215  rollout.jsonl
  04:20:44.405  metrics.json
  ```
- Interpretation: 排除“metrics 先生成、rollout 后补 task_complete”的时序解释。
- Time: 2026-07-13 04:29
