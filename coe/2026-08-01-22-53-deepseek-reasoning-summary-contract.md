# Problem P-001: DeepSeek reasoning summary capability is modeled incorrectly
- Status: fixed
- Created: 2026-08-01 22:53
- Updated: 2026-08-01 23:47
- Objective: Preserve DeepSeek Responses reasoning effort and raw reasoning streaming while omitting unsupported reasoning summaries and reporting the effective state accurately in `/status`.
- Symptoms:
  - `/status` renders `deepseek-v4-flash (reasoning high, summaries auto)` although DeepSeek does not generate Responses API reasoning summaries.
  - The bundled Flash model advertises `supports_reasoning_summaries: true`.
- Expected behavior:
  - Requests send the supported `reasoning.effort` value without `reasoning.summary`.
  - Raw `response.reasoning_text.delta` events continue to stream.
  - `/status` shows the effective effort and does not claim a summary mode for Flash.
- Actual behavior:
  - The model catalog claims summary support, and the TUI substitutes `auto` when no summary override exists.
  - The shared request builder gates the entire reasoning object on summary support, so changing the catalog flag alone would also remove effort.
- Impact:
  - User-visible status is inaccurate, and a naive catalog-only repair would regress DeepSeek thinking effort.
- Reproduction:
  - Resolve the bundled default model and render `/status` for `deepseek-v4-flash` with High effort and no summary override; observe `summaries auto` instead of no summary.
- Environment:
  - Linux, branch `whalecode-codex`, HEAD `e4e3265e2`, DeepSeek Responses API public contract dated 2026-08-01.
- Known facts:
  - DeepSeek Responses supports `reasoning.effort`; `reasoning.summary` may be sent but does not generate a summary.
  - DeepSeek emits raw reasoning through `response.reasoning_text.delta` and `.done`.
  - The local SSE parser already maps `response.reasoning_text.delta` to `ReasoningContentDelta`.
- Ruled out:
  - Missing raw reasoning SSE parser support.
- Fix criteria:
  - A focused request-body test proves effort is serialized and summary is absent for Flash.
  - A focused SSE test proves raw reasoning text remains mapped.
  - A focused TUI test proves `/status` shows effort without a summaries label for Flash.
  - Existing summary-capable model behavior remains covered; related TUI tests pass, and the full TUI baseline contains only the separately tracked W9 failure.
- Current conclusion: H-001 is confirmed and repaired by making reasoning effort independent from summary capability across request construction and status rendering.
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - E-005 proves the provider request preserves `reasoning.effort=high` while omitting `reasoning.summary` and `include` for a model without summary support.
  - E-006 proves raw DeepSeek reasoning events remain parsed independently.
  - E-007 proves Flash status omits the unsupported summary label while summary-capable status coverage remains intact.
  - E-008 proves all focused regressions pass and the full TUI baseline is reduced to the unrelated W9 ActionMap assertion.
  - E-009 proves the mandatory staged cache gate recognizes the exact final-wire change as a comparable candidate transition.
- Close reason:
  - fixed by decoupling effort from summary support and aligning catalog/UI capability reporting with the official DeepSeek Responses contract

## Hypothesis H-001: Summary support incorrectly owns the entire reasoning object
- Status: confirmed
- Parent: P-001
- Claim: `supports_reasoning_summaries` is incorrectly used as the gate for both `reasoning.effort` and `reasoning.summary`, so the model cannot express DeepSeek's supported-effort/unsupported-summary contract.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Request construction and status rendering both use OpenAI-oriented summary defaults after the provider moved to Responses.
- Falsifiable predictions:
  - If true: the request builder returns no reasoning object when summary support is false, even when effort is present; the TUI reports `auto` solely because the provider uses Responses.
  - If false: effort is serialized independently of summary support and TUI display is based on resolved model capability.
- Diagnostic evidence plan:
  - Prediction or clause under test: summary support false removes the complete reasoning object, and Responses provider status inserts `auto` without consulting model capability.
  - Signal: code-path condition plus exact Flash rendering reproduction.
  - Capture method: inspect `build_reasoning` and `StatusHistoryCell::new`; run a temporary exact Flash status assertion.
  - Event name or marker:
    - response.reasoning_text.delta
  - Correlation keys:
    - model=deepseek-v4-flash
  - Differentiates from:
    - H-002
  - Supports if:
    - both code paths are gated as predicted and the Flash probe renders `summaries auto`.
  - Refutes if:
    - effective model summary defaults are already passed through both paths.
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - temporary probe removed after diagnosis; convert to permanent regression test during repair
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: Confirmed by official contract, code-path inspection, and exact Flash reproduction.
- Repair design readiness: ready; user explicitly authorized implementation
- Next step: monitor the official DeepSeek Responses contract and restore summary support only if the provider begins generating summaries.
- Blocker:
  - none
- Close reason:
  - repaired and validated by E-005, E-007, and E-009

## Hypothesis H-002: DeepSeek raw reasoning cannot be preserved by the Responses SSE parser
- Status: refuted
- Parent: P-001
- Claim: Removing reasoning summaries would also remove all visible DeepSeek reasoning because the local Responses parser only understands summary events.
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - Codex upstream historically displays summary events, while DeepSeek emits raw reasoning text events.
- Falsifiable predictions:
  - If true: no parser branch maps `response.reasoning_text.delta` into an internal reasoning-content event.
  - If false: a dedicated mapping exists independently of reasoning-summary events.
- Diagnostic evidence plan:
  - Prediction or clause under test: inspect the Responses SSE event dispatch for raw reasoning event support.
  - Signal: exact event-name mapping.
  - Capture method: source inspection and existing parser tests.
  - Event name or marker:
    - response.reasoning_text.delta
  - Correlation keys:
    - none
  - Differentiates from:
    - H-001
  - Supports if:
    - only `response.reasoning_summary_text.delta` is mapped.
  - Refutes if:
    - `response.reasoning_text.delta` maps to `ReasoningContentDelta`.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-004
- Conclusion: Refuted; raw reasoning has an independent parser branch.
- Repair design readiness: no repair needed for this hypothesis
- Next step: retain and regression-test the existing mapping.
- Blocker:
  - none
- Close reason:
  - refuted by E-004

## Evidence E-001: DeepSeek Responses compatibility contract
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: external-review
- Source: https://api-docs.deepseek.com/zh-cn/guides/responses_api/
- Prediction or plan link:
  - H-001 contract clause and H-002 event-name clause
- Matched signal:
  - effort supported; summary accepted but not generated; raw reasoning text events supported
- Correlation keys:
  - model=deepseek-v4-flash
- Raw content:
  ```text
  reasoning: 部分支持。effort 支持；summary 可传入但不生成摘要
  response.reasoning_text.delta / response.reasoning_text.done: 思维链文本增量 / 完整思维链文本
  ```
- Interpretation: DeepSeek needs a reasoning object with effort but no summary, and raw reasoning is a separate response capability.
- Time: 2026-08-01 22:53

## Evidence E-002: Request builder couples effort to summary support
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/client.rs`, `build_reasoning`
- Prediction or plan link:
  - H-001 prediction that summary support false removes the entire reasoning object
- Matched signal:
  - `if model_info.supports_reasoning_summaries { Some(Reasoning { effort, summary }) } else { None }`
- Correlation keys:
  - none
- Raw content:
  ```text
  The whole Reasoning object is conditional on supports_reasoning_summaries.
  ```
- Interpretation: A catalog-only flag change would incorrectly remove supported effort.
- Time: 2026-08-01 22:53

## Evidence E-003: Exact Flash status reproduction
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: temporary `diagnostic_flash_status_uses_catalog_reasoning_defaults` TUI test
- Prediction or plan link:
  - H-001 prediction that Responses status substitutes `auto`
- Matched signal:
  - `deepseek-v4-flash (reasoning high, summaries auto)`
- Correlation keys:
  - model=deepseek-v4-flash
- Raw content:
  ```text
  expected: deepseek-v4-flash (reasoning high, summaries off)
  actual:   deepseek-v4-flash (reasoning high, summaries auto)
  ```
- Interpretation: The user-visible symptom reproduces on the real Whale default model path.
- Time: 2026-08-01 22:53

## Evidence E-004: Raw reasoning SSE is independently parsed
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/codex-api/src/sse/responses.rs`
- Prediction or plan link:
  - H-002 prediction that no raw reasoning mapping exists
- Matched signal:
  - `response.reasoning_text.delta` maps to `ResponseEvent::ReasoningContentDelta`
- Correlation keys:
  - event=response.reasoning_text.delta
- Raw content:
  ```text
  response.reasoning_text.delta -> ReasoningContentDelta
  ```
- Interpretation: Summary removal does not require removing raw reasoning streaming.
- Time: 2026-08-01 22:53

## Evidence E-005: Request construction preserves effort without summaries
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: `codex-core client::tests::responses_request_preserves_effort_without_summary_support`
- Prediction or plan link:
  - P-001 request-body fix criterion
- Matched signal:
  - serialized `reasoning.effort` is `high`; `reasoning.summary` is absent; `include` is empty
- Correlation keys:
  - model=deepseek-v4-flash
- Raw content:
  ```text
  1 test run: 1 passed
  ```
- Interpretation: Summary capability no longer owns the whole reasoning object.
- Time: 2026-08-01 23:20

## Evidence E-006: Raw reasoning parser regression passes
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: test
- Source: `codex-api sse::responses::tests::parses_raw_reasoning_text_delta_without_summary_events`
- Prediction or plan link:
  - P-001 SSE fix criterion
- Matched signal:
  - raw reasoning text delta is emitted without any summary event
- Correlation keys:
  - event=response.reasoning_text.delta
- Raw content:
  ```text
  Responses SSE suite: 28 passed
  ```
- Interpretation: Omitting summaries does not regress visible DeepSeek thinking output.
- Time: 2026-08-01 23:20

## Evidence E-007: Capability-aware status regressions pass
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: `codex-tui status::tests`
- Prediction or plan link:
  - P-001 TUI fix criterion
- Matched signal:
  - Flash renders `deepseek-v4-flash (reasoning high)` without `summaries`; summary-capable fixtures still render configured summary detail
- Correlation keys:
  - model=deepseek-v4-flash
- Raw content:
  ```text
  status test group: 28 passed
  ```
- Interpretation: `/status` now reports resolved model capabilities instead of inferring summary support from the wire API.
- Time: 2026-08-01 23:25

## Evidence E-008: Related and full regressions isolate the remaining failure
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: focused Nextest runs and `scripts/codex-upstream/run_tui_baseline.py`
- Prediction or plan link:
  - P-001 regression criteria
- Matched signal:
  - model manager 36/36, Responses SSE 28/28, core client 24/24, TUI status 28/28; full TUI 1887 passed, 1 failed, 5 skipped
- Correlation keys:
  - remaining_failure=app::tests::action_map_commands_are_routed_through_app_server_in_tui
- Raw content:
  ```text
  full TUI: 1888 tests run; 1887 passed; 1 W9 ActionMap failure; 5 skipped
  ```
- Interpretation: The reasoning-summary repair is green; the only full-suite failure is the independently tracked W9 route-mode assertion.
- Time: 2026-08-01 23:31

## Evidence E-009: Cache-sensitive final-wire candidate is comparable
- Related hypotheses:
  - H-001
- Direction: supports
- Type: gate
- Source: `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`
- Prediction or plan link:
  - P-001 cache gate criterion
- Matched signal:
  - mandatory index gate passes with candidate surface `1921a1272cdbc41d848ec8ab1a204fee5f387dad34ae3878722495860acbd934`
- Correlation keys:
  - first_difference=/request_1/include/length
- Raw content:
  ```text
  cache regression gate: PASS ...（已发现可比较的候选变更；发布继续阻断）
  ```
- Interpretation: Removing unsupported encrypted-reasoning includes is an intentional, mechanically comparable provider-wire transition; live baseline acceptance remains a release concern rather than a repair correctness failure.
- Time: 2026-08-01 23:47
