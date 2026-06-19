# Problem P-001: TaskSpace E3 run fails with invalid tool-call history
- Status: open
- Created: 2026-06-20 02:00
- Updated: 2026-06-20 02:24
- Objective: Identify the root cause of the TaskSpace-only `terminal-bench_E3-P0_3_1` failure before any repair design.
- Symptoms:
  - `processing-pipeline` TaskSpace-only execution exits before editing files or running validation.
  - The JSONL ends with an OpenAI API error that an assistant `tool_calls` message was not followed by all required tool messages.
- Expected behavior:
  - TaskSpace should preserve a model-valid message history after parallel command executions and continue to implementation or validation.
- Actual behavior:
  - The run fails on the next model request after several parallel `command_execution` items complete.
- Impact:
  - Blocks validating the v0.0.5 TaskSpace gate fix on `terminal-bench_E3-P0_3_1`.
- Reproduction:
  - Use current debug `whale.exe` built from commit `85b029fe5`.
  - Run `processing-pipeline` TaskSpace-only from the materialized `terminal_bench__processing-pipeline` fixture with `exec --json --taskspace -c model_reasoning_effort=max -m deepseek-v4-flash -C W:\app --full-auto --output-last-message ... -`.
- Environment:
  - Windows PowerShell, repository `D:\whalecode-alpha`, branch `whalecode-alpha`, commit `85b029fe5`.
  - Artifacts under `target\terminal-bench_E3-P0_3_1-v005-20260620-taskspace-only\processing-pipeline\run-002\right\artifacts`.
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
- Ruled out:
  - H-002: the PowerShell/manual launcher did not construct the provider message history; the invalid request is created after in-process model/tool/history handling.
- Fix criteria:
  - The original `processing-pipeline` TaskSpace-only reproduction no longer fails with invalid `tool_calls` history.
  - A targeted test or diagnostic replay proves that parallel command executions serialize into model messages with every assistant tool call immediately followed by its matching tool result.
- Current conclusion: H-003 is confirmed. The v0.0.5 provider-visible TaskSpace compact-history filter omits a blocked `shell_command` tool output because it contains TaskSpace recovery text, while keeping the matching assistant `shell_command` tool call. The ChatCompletions adapter then serializes an assistant `tool_calls` message without the required following tool message, causing the provider rejection.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-003, E-002, E-003, E-004, E-005
- Close reason:
  - not closed

## Hypothesis H-001: TaskSpace history projection emits incomplete parallel tool results
- Status: closed
- Parent: P-001
- Claim: The TaskSpace conversation/history projection path constructs a model request that includes an assistant message with multiple tool calls but omits or separates at least one required tool result before the next non-tool message.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The failure occurs immediately after one model turn launches several parallel `command_execution` items and receives their completions.
- Falsifiable predictions:
  - If true: The failing request payload or reconstruction path will show a mismatch between tool call IDs emitted by one assistant message and the following tool messages.
  - If false: The failing request payload is valid, and the API error must be caused by provider-side interpretation or a harness-specific wrapper issue.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare emitted JSONL item order and the code path that serializes completed command executions into the next model request.
  - Signal: artifact item sequence plus source code locations for message/history conversion.
  - Capture method: inspect `whale-exec.jsonl`, rollout/model request artifacts if available, and the Rust adapters that build OpenAI chat messages or responses input.
  - Event name or marker:
    - item.started
    - item.completed
    - turn.failed
  - Correlation keys:
    - item_id
    - command_execution
    - timestamp
  - Differentiates from:
    - H-002
  - Supports if:
    - A code path can emit a second assistant/tool-call batch or agent message before all prior parallel command results are represented as tool outputs in the next request.
  - Refutes if:
    - The request payload is provably valid and all tool call IDs are contiguous with matching tool outputs.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-001
- Conclusion: superseded by H-003. The observed failure is not that one of the seven parallel read outputs vanished at execution time; all seven are recorded. The provider-visible projection later drops a different blocked tool output.
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: inspect request reconstruction and compare with source serialization code.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: The manual TaskSpace-only harness caused an invocation-specific malformed history
- Status: refuted
- Parent: P-001
- Claim: The failing history is an artifact of the ad hoc TaskSpace-only launcher, not the TaskSpace runtime itself.
- Layer: diagnostic
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The TaskSpace-only run was manually launched to avoid the paired Standard runner. Its stderr contains a PowerShell `NativeCommandError` wrapper even though JSONL was produced.
- Falsifiable predictions:
  - If true: The malformed history depends on the manual pipeline/PowerShell wrapper, and the official paired harness or a direct non-pipeline invocation would not produce the same API error.
  - If false: The same history-validity failure is explained by core request serialization independent of the wrapper.
- Diagnostic evidence plan:
  - Prediction or clause under test: determine whether stderr wrapper behavior can affect the internal model request payload.
  - Signal: code path for stdin prompt ingestion and exec JSONL; artifact evidence showing the error occurred inside the model API response after completed tool calls.
  - Capture method: inspect exec code and existing official harness artifacts for the same error pattern without running another real agent.
  - Event name or marker:
    - NativeCommandError
    - turn.failed
  - Correlation keys:
    - process exit code
    - artifact path
  - Differentiates from:
    - H-001
  - Supports if:
    - The wrapper can intercept or reorder command outputs or tool results before they reach request serialization.
  - Refutes if:
    - The wrapper only captures process stderr and the malformed history originates after successful in-process tool execution.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-001
- Conclusion: refuted by E-002 through E-005. The launcher only supplies stdin, cwd, and output capture; the invalid message history is produced inside Whale's provider follow-up path after normal ResponseItem recording and TaskSpace filtering.
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: inspect execution boundary and historical official runner artifacts for the same error.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: TaskSpace-only processing-pipeline fails after parallel reads with OpenAI tool-call pairing error
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `target\terminal-bench_E3-P0_3_1-v005-20260620-taskspace-only\processing-pipeline\run-002\right\artifacts\whale-exec.jsonl`
- Prediction or plan link:
  - H-001 diagnostic evidence plan: artifact item sequence around completed command executions and `turn.failed`.
  - H-002 diagnostic evidence plan: distinguish in-process API failure from wrapper-only failure.
- Matched signal:
  - JSONL lines include `item.started` for command executions `item_1` through `item_7`, matching `item.completed` events, then an API error.
- Correlation keys:
  - `item_1` through `item_7`
  - `turn.failed`
- Raw content:
  ```text
  {"type":"error","message":"{\"error\":{\"message\":\"An assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id'. (insufficient tool messages following tool_calls message)\",\"type\":\"invalid_request_error\",\"param\":null,\"code\":\"invalid_request_error\"}}"}
  {"type":"turn.failed","error":{"message":"{\"error\":{\"message\":\"An assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id'. (insufficient tool messages following tool_calls message)\",\"type\":\"invalid_request_error\",\"param\":null,\"code\":\"invalid_request_error\"}}"}}
  ```
- Interpretation: The failure is a model request validity error after tool execution events, not the earlier broad-inspect gate loop. It supports H-001 as a candidate and leaves H-002 open until request construction and wrapper boundaries are inspected.
- Time: 2026-06-20 02:00

## Hypothesis H-003: v0.0.5 provider-visible filtering removes a required tool output after a blocked TaskSpace tool call
- Status: confirmed
- Parent: P-001
- Claim: When active TaskSpace compact projection is present, `prepare_provider_visible_prompt_items` filters ResponseItems one by one. A blocked non-TaskSpace tool output that contains TaskSpace recovery text is classified as `LegacyTaskspaceToolOutput` and omitted, but its matching assistant `shell_command` function call is still included. The ChatCompletions request therefore violates the assistant tool-call/tool-output pairing contract.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The provider error exactly states that an assistant message with `tool_calls` lacks the following tool messages.
  - The failing rollout contains a blocked `shell_command` output with `TaskSpaceGateRecoveryV1` immediately before logical provider request 4.
- Falsifiable predictions:
  - If true: the rollout should show a normal assistant `shell_command` call, a matching `function_call_output` containing TaskSpace recovery text, an active compact projection, then provider request failure.
  - If true: the v0.0.5 provider-visible history filter should omit any `FunctionCallOutput` containing `TaskSpace` or `taskspace_control` while preserving non-`taskspace_control` function calls.
  - If false: the blocked tool output would be preserved, or the matching function call would also be removed, leaving a valid provider-visible sequence.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare rollout item sequence around logical provider request 4 with the provider-visible filtering code and ChatCompletions adapter.
  - Signal: `response_item` lines for the call/output/projection, provider request status lines, and source lines for classification and message conversion.
  - Capture method: inspect `C:\Users\77585\.whale\sessions\2026\06\20\rollout-2026-06-20T01-57-47-019ee108-2418-7f72-8d60-231fd0079943.jsonl` and relevant Rust source.
  - Event name or marker:
    - `function_call`
    - `function_call_output`
    - `TaskSpaceGateRecoveryV1`
    - `ContextProjectionV1 active replacement`
    - `provider_request_budget status=failed`
  - Correlation keys:
    - `call_00_eZMFCYUWQLmauPYwSqA63164`
    - `provider-request:019ee108-24d4-7fe0-af5a-495b5c7dfc6a:logical-4:attempt-1`
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - The blocked output is present in raw history and matches the filter's omission predicate, while its function call does not match an omission predicate.
  - Refutes if:
    - The output does not contain TaskSpace markers, active projection is absent, or code preserves call/output pairs atomically.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied by E-002, E-003, E-004, E-005
- Related evidence:
  - E-002
  - E-003
  - E-004
  - E-005
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: repair provider-visible history composition so it preserves model-valid tool-call/output pairs, either by retaining required outputs or removing paired calls and any adjacent assistant content as an atomic group.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-002: Raw rollout records all seven parallel read outputs before the later failure
- Related hypotheses:
  - H-001
  - H-003
- Direction: refutes H-001 as the direct execution-time loss mechanism; supports narrowing H-003
- Type: trace
- Source: `C:\Users\77585\.whale\sessions\2026\06\20\rollout-2026-06-20T01-57-47-019ee108-2418-7f72-8d60-231fd0079943.jsonl`
- Prediction or plan link:
  - H-001 diagnostic evidence plan: compare tool call IDs and outputs around the parallel reads.
  - H-003 diagnostic evidence plan: separate the earlier parallel read batch from the later blocked tool-call failure.
- Matched signal:
  - Lines 53 through 59 contain seven `shell_command` function calls.
  - Lines 67, 74, 87, 88, 89, 90, and 91 contain matching `function_call_output` records for `call_00_YQgQffAP354qKdYAbgYf7823`, `call_01_9C06RtepFGBwKqRoxeH18711`, `call_02_Jlkwtw7ZvzmaJxnbTPov5698`, `call_03_SJG92jquoaKXG1Y7JcJy9220`, `call_04_8G5l2pagzndzMETqD0xE0690`, `call_05_QmSn23cSnAHyBhoAv01g6098`, and `call_06_JpEGFxr9j6DK6XfkMJpg8617`.
- Correlation keys:
  - `call_00_YQgQffAP354qKdYAbgYf7823`
  - `call_06_JpEGFxr9j6DK6XfkMJpg8617`
- Raw content:
  ```text
  call shell_command call_00_YQgQffAP354qKdYAbgYf7823
  call shell_command call_01_9C06RtepFGBwKqRoxeH18711
  call shell_command call_02_Jlkwtw7ZvzmaJxnbTPov5698
  call shell_command call_03_SJG92jquoaKXG1Y7JcJy9220
  call shell_command call_04_8G5l2pagzndzMETqD0xE0690
  call shell_command call_05_QmSn23cSnAHyBhoAv01g6098
  call shell_command call_06_JpEGFxr9j6DK6XfkMJpg8617
  output call_00_YQgQffAP354qKdYAbgYf7823
  output call_01_9C06RtepFGBwKqRoxeH18711
  output call_02_Jlkwtw7ZvzmaJxnbTPov5698
  output call_03_SJG92jquoaKXG1Y7JcJy9220
  output call_04_8G5l2pagzndzMETqD0xE0690
  output call_05_QmSn23cSnAHyBhoAv01g6098
  output call_06_JpEGFxr9j6DK6XfkMJpg8617
  ```
- Interpretation: The original suspicion that the seven parallel inspect reads lost an output during execution is not the direct cause. The invalid provider request occurs later.
- Time: 2026-06-20 02:18

## Evidence E-003: The failing boundary is a blocked `shell_command` output followed by active compact projection and provider request 4 failure
- Related hypotheses:
  - H-002
  - H-003
- Direction: supports H-003 and refutes H-002
- Type: trace
- Source: `C:\Users\77585\.whale\sessions\2026\06\20\rollout-2026-06-20T01-57-47-019ee108-2418-7f72-8d60-231fd0079943.jsonl`
- Prediction or plan link:
  - H-003 diagnostic evidence plan: verify the call/output/projection/failure sequence.
- Matched signal:
  - Line 103 records `shell_command` call `call_00_eZMFCYUWQLmauPYwSqA63164`.
  - Line 105 records `tool_action_blocked` with `actionClass="unknown"`.
  - Line 110 records matching `function_call_output` containing `TaskSpace blocked this tool call` and `TaskSpaceGateRecoveryV1`.
  - Line 111 records active compact projection text.
  - Lines 112 through 116 record logical provider request 4 from `started` to `failed`.
- Correlation keys:
  - `call_00_eZMFCYUWQLmauPYwSqA63164`
  - `logical-4:attempt-1`
- Raw content:
  ```text
  function_call shell_command call_00_eZMFCYUWQLmauPYwSqA63164
  tool_action_blocked actionClass="unknown" reason="inspect_code_context does not allow unknown"
  function_call_output call_00_eZMFCYUWQLmauPYwSqA63164 output contains "TaskSpace blocked this tool call" and "TaskSpaceGateRecoveryV1"
  developer message contains "TaskSpace v0.0.5 active compact profile is enabled" and "ContextProjectionV1 active replacement"
  provider_request_budget logical-4 status=failed
  ```
- Interpretation: The failure is caused after normal in-process tool blocking and history recording. The manual PowerShell launcher is outside this sequence and cannot explain the malformed provider-visible history.
- Time: 2026-06-20 02:20

## Evidence E-004: v0.0.5 provider-visible history filtering omits TaskSpace-marked tool outputs but keeps ordinary shell function calls
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-path
- Source: `third_party\codex-cli\codex-rs\core\src\session\turn.rs`
- Prediction or plan link:
  - H-003 diagnostic evidence plan: verify the exact omission predicate.
- Matched signal:
  - `compose_provider_visible_history` only activates filtering when an active projection item exists.
  - `is_legacy_taskspace_tool_output` returns true for `FunctionCallOutput` or `CustomToolCallOutput` containing `TaskSpace`, `ActionMap`, or `taskspace_control`.
  - `provider_visible_history_action` omits `LegacyTaskspaceToolOutput`.
  - `is_taskspace_control_call` only omits calls whose name is `taskspace_control`; it does not omit a `shell_command` call whose output was blocked by TaskSpace.
- Correlation keys:
  - `TASKSPACE_ACTIVE_PROFILE_MARKER`
  - `LegacyTaskspaceToolOutput`
  - `call_00_eZMFCYUWQLmauPYwSqA63164`
- Raw content:
  ```text
  fn compose_provider_visible_history(items: Vec<ResponseItem>) -> ProviderVisibleHistoryComposition {
      if !items.iter().any(is_active_context_projection_item) { ... Include ... }
      ...
      let action = provider_visible_history_action(&category);
      if matches!(action, ProviderVisibleHistoryAction::Include) {
          prepared.push(item);
      }
  }

  fn is_legacy_taskspace_tool_output(item: &ResponseItem) -> bool {
      matches!(item, ResponseItem::FunctionCallOutput { .. } | ResponseItem::CustomToolCallOutput { .. })
          && (response_item_text_contains(item, "TaskSpace")
              || response_item_text_contains(item, "ActionMap")
              || response_item_text_contains(item, "taskspace_control"))
  }
  ```
- Interpretation: Line 110's blocked output is guaranteed to be omitted once line 111's active projection is present, while line 103's `shell_command` call remains provider-visible.
- Time: 2026-06-20 02:22

## Evidence E-005: ChatCompletions conversion requires preserved call/output pairing
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-path
- Source: `third_party\codex-cli\codex-rs\codex-api\src\endpoint\chat_completions.rs` and `third_party\codex-cli\codex-rs\codex-api\src\endpoint\responses.rs`
- Prediction or plan link:
  - H-003 diagnostic evidence plan: verify how filtered ResponseItems become provider `messages`.
- Matched signal:
  - `build_chat_completions_body` extends `messages` with `chat_messages_from_response_items(&request.input)`.
  - `chat_messages_from_response_items` accumulates assistant `FunctionCall` items into `tool_calls`.
  - Only a `FunctionCallOutput` flushes and emits the required `role="tool"` message with `tool_call_id`.
  - If v0.0.5 filtering removes the `FunctionCallOutput`, the adapter emits an assistant message with `tool_calls` and no following matching tool message.
- Correlation keys:
  - `chat_messages_from_response_items`
  - `tool_call_id`
- Raw content:
  ```text
  ResponseItem::FunctionCall { name, arguments, call_id, .. } => {
      pending_assistant.push_tool_call(name, arguments, call_id);
  }
  ResponseItem::FunctionCallOutput { call_id, output } => {
      pending_assistant.flush_into(&mut messages);
      messages.push(json!({"role": "tool", "tool_call_id": call_id, ...}));
  }
  ```
- Interpretation: The provider error is the expected downstream effect of E-004. The final malformed object is not created by the manual harness, but by filtering a ResponseItem sequence without preserving tool protocol invariants.
- Time: 2026-06-20 02:24
