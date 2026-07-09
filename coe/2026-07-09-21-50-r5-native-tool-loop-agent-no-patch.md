# Problem P-001: R5-C1 native tool-loop 后 TaskSpace 单样本 agent_no_patch
- Status: fixed
- Created: 2026-07-09 21:50 CST
- Updated: 2026-07-09 22:00 CST
- Objective: 解释并修复 R5-C1 native tool-loop 收敛后 `count-call-stack` TaskSpace 侧出现读完文件但未 patch、转向验证环境排查的失败。
- Symptoms:
  - `target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987/pair-001/pair-report.md` 中 standard solved，TaskSpace wrong。
  - TaskSpace 侧 `failure_taxonomy=agent_no_patch`，`changed_paths` 为空。
  - 工具 alias 已能执行，且本轮没有 TaskSpace blocked/unsupported tool。
- Expected behavior:
  - TaskSpace 在 native tool-loop 下应忠实传递用户目标、工具结果和失败反馈，不额外注入策略语义；Agent 在语义可见时应可完成与 standard 同类的简单 patch。
- Actual behavior:
  - TaskSpace 侧 Agent 多次读取源码/测试/脚本后运行测试和环境探测，没有执行 patch。
- Impact:
  - R5-C1 native tool-loop 和 runtime 边界收敛的 correctness gate 未稳定关闭。
- Reproduction:
  - `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 -Scenario count-call-stack -Repeats 1 -RunSide both -RunRoot target/r5c1-native-tool-loop-clean ...`
- Environment:
  - Repo: `/home/zhangxu/whalecode-alpha`
  - Branch: `whalecode-alpha`
  - Current phase: v0.0.5 build-R5 Phase C1
  - Model/API key source: `.env.local` contains provider key; key value not recorded.
- Known facts:
  - E-001: failing rollout 中 native `exec_command` alias 均执行并返回 ordinary tool output，没有 unsupported/blocked alias 症状。
  - E-002: active projection 在 Agent 识别 bug 前后仍渲染 `hard action-class constraints` 和 `allowed action classes`，且 inspect node 不列出 `edit`。
  - E-003: Agent 明确识别 `CALL_STACK_DEPTH=<integer>` 与 `depth: ...` 的差异后，没有 patch，而是选择先跑 pytest。
  - E-004: 原始用户任务仍进入 provider-visible payload，blank map 不是当前 no-patch 的直接语义丢失根因。
  - E-005: 后续 budget hard stop 发生在 pytest/环境探测之后，是放大器而非第一根因。
  - E-006: 删除 model-visible action-class contract 后，复跑 `count-call-stack` paired sample，standard/taskspace 均 solved，TaskSpace 执行 `apply_patch` 并通过 validator。
- Ruled out:
  - H-001: native alias/transport 丢失或阻断不是本次 no-patch 根因。
  - H-003: budget/cadence 不是 Agent 未开始 patch 的第一根因；它放大了 projection 污染后的错误路径。
  - H-004: mechanical blank map 初始化不是本次 no-patch 的直接根因。
- Fix criteria:
  - Focused unit/regression tests pass.
  - A targeted `count-call-stack` sample shows TaskSpace no longer fails due to runtime semantic gate/unsupported native alias.
  - Any remaining failure is classified with trace evidence, not assumption.
- Current conclusion: fixed by removing model-visible action-class contract from active projection and deleting the unused `NodeContract.allowed_actions` structure. H-002 confirmed; E-006 validates the original sample no longer fails.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - H-002
  - E-002
  - E-003
  - E-006
- Close reason:
  - fixed

## Hypothesis H-001: native tool alias/transport still loses or blocks actions
- Status: refuted
- Parent: P-001
- Claim: TaskSpace failed because native DeepSeek emitted tool aliases whose execution or feedback was still not faithfully handled.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The immediately preceding failure family showed native `exec_command`/`read_file` aliases were not supported or were classified as unknown before the alias fix.
- Falsifiable predictions:
  - If true: the failing rollout should contain rejected/unsupported tool calls, missing tool outputs, or alias calls that do not become node events.
  - If false: alias calls should execute and return visible outputs without TaskSpace blocked/unsupported errors.
- Diagnostic evidence plan:
  - Prediction or clause under test: alias calls should execute and return visible outputs without TaskSpace blocked/unsupported errors.
  - Signal: rollout function calls/outputs, pair report taxonomy, node event traces.
  - Capture method: inspect only bounded trace excerpts from the failing run.
  - Event name or marker:
    - function_call
    - function_call_output
    - TaskSpace blocked/unsupported markers
  - Correlation keys:
    - target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987
  - Differentiates from:
    - H-002
    - H-003
    - H-004
  - Supports if:
    - unsupported or blocked alias feedback appears before the no-patch outcome.
  - Refutes if:
    - all alias calls execute and return ordinary outputs.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: refuted by E-001
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: provider-visible projection or sentinel text polluted Agent behavior
- Status: confirmed
- Parent: P-001
- Claim: TaskSpace injected or amplified non-neutral semantic text, such as sentinel warnings or validation guidance, which diverted Agent from patching into environment/debug activity.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - R5 design principle forbids projection/sentinel from adding strategy guidance. The latest run still produced `validator_failure` sentinel traces, and prior run was marked unclean by `unclassified_shell_action`.
- Falsifiable predictions:
  - If true: provider-visible payload or active projection should contain strategy-like sentinel/validation warning text before or during the divergence.
  - If false: provider-visible text should show only map/node/events/refs/hard status and raw tool outputs.
- Diagnostic evidence plan:
  - Prediction or clause under test: provider-visible payload contains strategy-like sentinel/validation text before or during divergence.
  - Signal: context projection summary, provider request payload excerpts, observability/sentinel trace.
  - Capture method: inspect bounded artifacts and code path that renders active sentinel warnings.
  - Event name or marker:
    - active_sentinel_warning
    - validator_failure
    - active projection
  - Correlation keys:
    - target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987
  - Differentiates from:
    - H-001
    - H-003
    - H-004
  - Supports if:
    - warning/guidance text is present in model-visible context before the agent spends requests on environment probing.
  - Refutes if:
    - sentinel traces are only offline observability and not model-visible, or appear only after the no-patch decision.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-003
  - E-006
- Conclusion: confirmed. The active projection still exposed the removed semantic action-class contract; this is sufficient to explain no-patch without assuming model/tool-loop instability.
- Repair design readiness: ready
- Next step: remove model-visible action-class contract and add regression coverage
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: budget/cadence still cuts off the patch opportunity
- Status: refuted
- Parent: P-001
- Claim: TaskSpace native loop still consumes the rollout request budget before the Agent reaches implementation, producing no-patch as a lifecycle artifact rather than a model decision.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Earlier R5-B/C0 failures were caused by request lifecycle cliff and post-feedback hard stop.
- Falsifiable predictions:
  - If true: budget events should show hard stop or no follow-up opportunity immediately after useful context became available.
  - If false: the Agent should have multiple successful tool results and available requests before budget exhaustion; no-patch is not caused by a runtime hard stop.
- Diagnostic evidence plan:
  - Prediction or clause under test: budget events show hard stop before patch opportunity.
  - Signal: request summary, budget events, last message, rollout trace request count.
  - Capture method: inspect bounded request/budget artifacts.
  - Event name or marker:
    - TaskSpaceProviderBudgetHardStopV1
    - model_request_count
  - Correlation keys:
    - target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987
  - Differentiates from:
    - H-002
    - H-004
  - Supports if:
    - hard stop occurs right after source/test visibility and before any implementation chance.
  - Refutes if:
    - rollout has ordinary opportunities and the Agent chooses validation/environment probes.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-005
- Conclusion: refuted as root cause by E-005. Budget hard stop occurred, but after the Agent had already identified the fix and chosen validation/environment probing instead of editing.
- Repair design readiness: not applicable
- Next step: keep budget as a secondary cost/backstop issue
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: task goal anchoring was diluted by mechanical blank map initialization
- Status: refuted
- Parent: P-001
- Claim: Runtime-created blank map context made the Agent under-anchor on the user task and over-anchor on exploratory bookkeeping, even though ordinary tool results were visible.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - User allowed semantics-free mechanical initialization but required Agent to know the map is blank and needs completion. If the projection overemphasizes blank state, it may dilute the original task target.
- Falsifiable predictions:
  - If true: early provider-visible context should show the user task absent or weaker than blank-map/taskspace bookkeeping.
  - If false: original task prompt remains visible and the blank map is represented as mechanical status only.
- Diagnostic evidence plan:
  - Prediction or clause under test: early provider-visible context shows task absent or diluted by blank-map text.
  - Signal: first provider request payload/context summary and assistant first actions.
  - Capture method: inspect bounded provider payload excerpts and active projection text.
  - Event name or marker:
    - mechanical_blank_map_initialized
    - active projection
  - Correlation keys:
    - target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - user task is absent/truncated while blank-map bookkeeping is prominent.
  - Refutes if:
    - user task is visible and blank-map text is clearly mechanical.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: refuted as direct root cause by E-004.
- Repair design readiness: not applicable
- Next step: keep mechanical blank-map wording minimal in later projection audits
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: native exec_command alias executed and returned outputs
- Related hypotheses:
  - H-001
- Direction: refutes
- Type: observation
- Source: `target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-001 prediction: alias calls should execute and return visible outputs without TaskSpace blocked/unsupported errors.
- Matched signal:
  - `exec_command` calls returned `Exit code` outputs for `ls`, `cat README.md`, `cat pyproject.toml`, file reads, pytest/env probes.
- Correlation keys:
  - `target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987`
- Raw content:
  ```text
  CALL exec_command {"cmd": "cat src/call_stack_counter.py", ...}
  OUTPUT ... Exit code: 0 ... def format_depth() -> str: return f"depth: {count_stack_depth()}"
  pair-report: engineering_unclean=False; failure_taxonomy=agent_no_patch
  ```
- Interpretation: 本次失败不是 alias unsupported/block 导致；工具调用和输出已进入上下文。
- Time: 2026-07-09 21:50 CST

## Evidence E-002: active projection exposed old action-class contract without edit
- Related hypotheses:
  - H-002
- Direction: supports
- Type: observation
- Source: `target/r5c1-native-tool-loop-clean/count-call-stack/20260709-214720-987/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-002 prediction: provider-visible payload contains strategy-like sentinel/validation text before or during divergence.
- Matched signal:
  - active projection text included `hard action-class constraints` and `allowed action classes: read, search, build, test, control`.
- Correlation keys:
  - `projection-verification_first-task-1-map-1`
  - `node-1 kind=inspect_code_context`
- Raw content:
  ```text
  TaskSpace v0.0.5 active thin projection. This surface contains the TaskSpace map skeleton, current-node event excerpts, result references, and hard action-class constraints.
  Current node contract:
  - node: node-1 kind=inspect_code_context
  - allowed action classes: read, search, build, test, control
  ```
- Interpretation: Runtime 已不再阻止 ordinary edit，但 projection 仍把旧 action contract 暴露给 Agent，且 inspect node 不包含 edit；这是 model-visible 语义注入和约束残留。
- Time: 2026-07-09 21:50 CST

## Evidence E-003: Agent recognized the fix then chose validation first
- Related hypotheses:
  - H-002
- Direction: supports
- Type: observation
- Source: `rollout.jsonl`
- Prediction or plan link:
  - H-002 prediction: provider-visible semantic constraints can explain divergence before any tool transport failure.
- Matched signal:
  - Assistant message identified the exact bug before running pytest and still did not emit an edit tool.
- Correlation keys:
  - `provider-request:019f4722-4e3f-7280-ab34-ed80a61d1d15`
- Raw content:
  ```text
  I can see the bug now. The README and tests specify the output format as `CALL_STACK_DEPTH=<integer>`, but the implementation prints `depth: ...` instead. Let me first run the tests to confirm the failure.
  ```
- Interpretation: The key divergence happened when the Agent already knew the patch and still avoided edit, consistent with the projection's visible no-edit contract.
- Time: 2026-07-09 21:50 CST

## Evidence E-004: user task remained visible alongside mechanical blank map
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: observation
- Source: `rollout.jsonl`
- Prediction or plan link:
  - H-004 prediction: early provider-visible context shows task absent or diluted by blank-map text.
- Matched signal:
  - Provider-visible messages included the original user request after environment context and before ordinary tool calls.
- Correlation keys:
  - `provider-request:019f4722-4e3f-7280-ab34-ed80a61d1d15:logical-1`
- Raw content:
  ```text
  The small project has a command-line formatter bug.

  Read the README and tests first, then fix the implementation. The final command must print the call-stack depth in the exact format required by the validator. Run the local validation command before finishing.
  ```
- Interpretation: The original task was not lost. Blank-map wording remains a projection hygiene concern, but it does not explain this no-patch failure.
- Time: 2026-07-09 21:50 CST

## Evidence E-005: budget hard stop occurred after validation/environment probes
- Related hypotheses:
  - H-003
- Direction: refutes
- Type: observation
- Source: `rollout.jsonl`, `request-summary.json`, `provider-request-events.jsonl`
- Prediction or plan link:
  - H-003 prediction: budget events show hard stop before patch opportunity.
- Matched signal:
  - The trace shows source/test reads, explicit bug recognition, pytest failure, pip install attempt, pytest availability probes, then hard stop at `request_count: 7/6`.
- Correlation keys:
  - `provider-request:019f4722-4e3f-7280-ab34-ed80a61d1d15`
- Raw content:
  ```text
  TaskSpaceProviderBudgetHardStopV1:
  reason: provider_request_hard_limit_exceeded
  request_count: 7/6
  node_request_count: 7/2
  ```
- Interpretation: Budget still needs cost/cadence cleanup, but the no-patch root cause was already present before the hard stop.
- Time: 2026-07-09 21:50 CST

## Evidence E-006: fixed sample passes without action-class projection
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: `target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052/pair-001/pair-report.md`
- Prediction or plan link:
  - P-001 fix criteria: targeted `count-call-stack` sample shows TaskSpace no longer fails due to runtime semantic gate/unsupported native alias.
- Matched signal:
  - `outcome_standard=solved`, `outcome_taskspace=solved`, `failure_taxonomy=none`, `engineering_unclean=False`.
  - TaskSpace changed `src/call_stack_counter.py`, ran validator successfully, and rollout contains no `allowed action classes` or `hard action-class constraints`.
- Correlation keys:
  - `target/r5c1-native-tool-loop-no-action-contract/count-call-stack/20260709-215916-052`
- Raw content:
  ```text
  outcome_standard: solved
  outcome_taskspace: solved
  right / taskspace:
  business_success: True
  public_validation_exit_code: 0
  hidden_oracle_exit_code: 0
  changed_paths: src/call_stack_counter.py
  ```
- Interpretation: Removing the projection action-class contract fixes the observed no-patch failure without adding runtime semantic constraints or assuming model/tool-loop instability.
- Time: 2026-07-09 22:00 CST
