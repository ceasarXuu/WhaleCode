# Problem P-001: Codex Auto Review appears in the model picker
- Status: fixed
- Created: 2026-08-27 08:47
- Updated: 2026-08-27 22:24
- Objective: Ensure `/model` contains only user-selectable conversation models and excludes internal review-only models.
- Symptoms:
  - The user sees `Codex Auto Review` in the `/model` candidate list.
- Expected behavior:
  - Review-only/internal models must not be offered as conversation model choices.
- Actual behavior:
  - `Codex Auto Review` is rendered as a selectable OpenAI Subscription model.
- Impact:
  - Whale v0.0.6 multi-provider model selection is misleading and can route a normal turn to a non-conversation model.
- Reproduction:
  - Sign in with OpenAI Subscription, start Whale v0.0.6, and open `/model`.
- Environment:
  - Linux workspace `whalecode-alpha-48d2219088`, branch `whalecode-alpha`, installed Whale v0.0.6.
- Known facts:
  - The OpenAI ChatGPT model cache contains `codex-auto-review`, described as an automatic approval-review model and marked `visibility: "hide"`.
  - `ModelInfo -> ModelPreset` preserves this as `show_in_picker: false`.
  - The provider-aware popup renders every grouped preset without checking `show_in_picker`; the legacy popup checks it before rendering.
- Ruled out:
  - The provider catalog incorrectly marking `codex-auto-review` as generally visible.
- Fix criteria:
  - The original installed `/model` reproduction no longer shows `Codex Auto Review`; ordinary OpenAI Subscription models remain visible; focused tests identify review-only catalog entries and exclude them from user-selectable presets.
- Current conclusion: Fixed by commit `3140d7b95`, which enforces the upstream `show_in_picker` contract once at the Provider-group `ModelCatalog` boundary. Focused tests and installed TUI validation both pass.
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-001, E-002, E-003, E-006, and E-007
- Close reason:
  - not closed

## Hypothesis H-001: Remote metadata marks the entry as non-user-selectable but the picker loses that signal
- Status: confirmed
- Parent: P-001
- Claim: The remote/cache model entry contains a visibility or purpose discriminator for `codex-auto-review`, but the Whale provider-group projection ignores it and turns the entry into a normal preset.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The entry originates in the OpenAI ChatGPT model cache and the recent multi-provider work projects catalog entries into route groups.
- Falsifiable predictions:
  - If true: The raw entry has a non-default visibility/purpose marker, and the projection does not enforce that marker before constructing model presets.
  - If false: The raw entry is indistinguishable from normal user-selectable conversation models or is already removed before projection.
- Diagnostic evidence plan:
  - Prediction or clause under test: Inspect the complete cached entry and trace every filter between cached models and `/model` presets.
  - Signal: Raw JSON fields plus source-level filter predicates.
  - Capture method: Use `jq` on the isolated workspace cache and inspect model catalog/preset construction code.
  - Event name or marker:
    - model slug `codex-auto-review`
  - Correlation keys:
    - workspace `whalecode-alpha-48d2219088`
  - Differentiates from:
    - H-002
  - Supports if:
    - A discriminator exists and is not checked on the picker projection path.
  - Refutes if:
    - No discriminator exists or the entry is already excluded before projection.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-006
  - E-007
- Conclusion: The hidden visibility signal survives into each `ModelPreset`, but the provider-group rendering loop ignores it.
- Repair design readiness: implemented and validated
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: The upstream catalog itself treats the review model as generally visible
- Status: refuted
- Parent: P-001
- Claim: OpenAI returns `codex-auto-review` with normal list visibility, so the general catalog legitimately retains it and the TUI needs an explicit user-selectability contract distinct from catalog membership.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Internal execution models can be present in a provider catalog even when they are not intended for direct user selection.
- Falsifiable predictions:
  - If true: The raw entry is marked visible like ordinary models, and current upstream preset generation has no semantic field that excludes review-only use.
  - If false: The raw entry is already marked hidden or otherwise non-user-selectable.
- Diagnostic evidence plan:
  - Prediction or clause under test: Compare the review entry with a normal subscription model and inspect upstream model-list filtering semantics.
  - Signal: Field-by-field JSON comparison and source tests around visibility/presets.
  - Capture method: Use `jq` and focused source search/tests.
  - Event name or marker:
    - model slug `codex-auto-review`
  - Correlation keys:
    - workspace `whalecode-alpha-48d2219088`
  - Differentiates from:
    - H-001
  - Supports if:
    - The entry has ordinary list visibility while carrying only review-semantic identity/metadata.
  - Refutes if:
    - A generic non-user-selectable discriminator is available and merely ignored.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-005
- Conclusion: Refuted because the provider metadata explicitly marks the entry hidden.
- Repair design readiness: not applicable to a refuted hypothesis
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Installed workspace cache contains Codex Auto Review
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: config
- Source: `/home/zhangxu/.local/state/whalecode/workspaces/whalecode-alpha-48d2219088/home/models_cache.openai-chatgpt.json`
- Prediction or plan link:
  - Both hypotheses predict that the offending row originates in the OpenAI ChatGPT model catalog/cache rather than being a hard-coded TUI label.
- Matched signal:
  - `display_name` is `Codex Auto Review` at cache line 680.
- Correlation keys:
  - workspace `whalecode-alpha-48d2219088`
- Raw content:
  ```text
  /home/zhangxu/.local/state/whalecode/workspaces/whalecode-alpha-48d2219088/home/models_cache.openai-chatgpt.json:680: "display_name": "Codex Auto Review"
  ```
- Interpretation: The TUI label is supplied by provider model metadata; this observation does not yet establish whether generic visibility metadata was ignored.
- Time: 2026-08-27 08:48

## Evidence E-002: Provider metadata explicitly marks the review model hidden
- Related hypotheses:
  - H-001
- Direction: supports
- Type: config
- Source: `jq` query against `/home/zhangxu/.local/state/whalecode/workspaces/whalecode-alpha-48d2219088/home/models_cache.openai-chatgpt.json`
- Prediction or plan link:
  - H-001 predicts a non-user-selectable marker; H-002 predicts ordinary visibility.
- Matched signal:
  - The model is review-specific and has `visibility: "hide"`.
- Correlation keys:
  - model slug `codex-auto-review`
- Raw content:
  ```text
  "slug": "codex-auto-review"
  "display_name": "Codex Auto Review"
  "description": "Automatic approval review model for Codex."
  "visibility": "hide"
  ```
- Interpretation: The catalog contract is correct; this model is intentionally present for internal resolution but explicitly excluded from pickers.
- Time: 2026-08-27 08:49

## Evidence E-003: Visibility is preserved but ignored only by grouped popup rendering
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `protocol/src/openai_models.rs:841`, `tui/src/chatwidget/model_popups.rs:127`, and `tui/src/chatwidget/model_popups.rs:246`
- Prediction or plan link:
  - H-001 predicts that the hidden signal exists on the preset and is bypassed by the provider-group projection.
- Matched signal:
  - Conversion sets `show_in_picker` from visibility; the legacy popup filters on it; the provider-aware loop does not.
- Correlation keys:
  - model slug `codex-auto-review`
- Raw content:
  ```text
  show_in_picker: info.visibility == ModelVisibility::List
  for preset in group.models {
  .filter(|preset| preset.show_in_picker)
  ```
- Interpretation: This is a renderer regression, not an upstream model-list or authentication problem.
- Time: 2026-08-27 08:50

## Evidence E-004: Regression was introduced with routed TUI model selection
- Related hypotheses:
  - H-001
- Direction: supports
- Type: regression-window
- Source: `git blame` and `git log` for `tui/src/chatwidget/model_popups.rs`
- Prediction or plan link:
  - H-001 predicts the grouped renderer is the first path that bypasses the legacy visibility filter.
- Matched signal:
  - Commit `787919394` added `open_provider_models_popup` and its unfiltered `for preset in group.models` loop.
- Correlation keys:
  - commit `787919394`
- Raw content:
  ```text
  787919394 feat(provider): add routed tui model selection
  787919394 ... for preset in group.models {
  ```
- Interpretation: The bug belongs to the multi-provider TUI grouping change and has a narrow repair boundary.
- Time: 2026-08-27 08:51

## Evidence E-005: Hidden visibility contradicts generally visible catalog hypothesis
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: config
- Source: `jq` query against `/home/zhangxu/.local/state/whalecode/workspaces/whalecode-alpha-48d2219088/home/models_cache.openai-chatgpt.json`
- Prediction or plan link:
  - H-002 requires `codex-auto-review` to have ordinary list visibility.
- Matched signal:
  - The actual visibility is `hide`.
- Correlation keys:
  - model slug `codex-auto-review`
- Raw content:
  ```text
  "visibility": "hide"
  ```
- Interpretation: H-002's required prediction is false.
- Time: 2026-08-27 08:52

## Evidence E-006: Focused tests enforce upstream visibility at the shared group boundary
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `cargo test -p codex-tui provider_and_model_pickers_preserve_route_groups -- --nocapture`
- Prediction or plan link:
  - The fix criteria require a hidden model fixture to be absent from both the grouped catalog and rendered picker while ordinary provider groups remain functional.
- Matched signal:
  - The test inserts `codex-auto-review` first with `show_in_picker: false`, verifies every retained grouped preset is visible, and verifies the rendered popup omits `Codex Auto Review`; 1 test passed.
- Correlation keys:
  - commit `3140d7b95`
- Raw content:
  ```text
  test chatwidget::tests::slash_commands::provider_and_model_pickers_preserve_route_groups ... ok
  test result: ok. 1 passed; 0 failed
  ```
- Interpretation: The repair enforces the general visibility contract rather than relying on a model-name blacklist or display order.
- Time: 2026-08-27 22:17

## Evidence E-007: Installed TUI excludes the real hidden review model
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: R10 installed TUI run `WAR-20260827-222257-HIDDEN-MODEL-VISIBILITY-R10` and `logs_2.sqlite`
- Prediction or plan link:
  - The original reproduction must no longer show `Codex Auto Review`, while the OpenAI Subscription group and ordinary models remain visible.
- Matched signal:
  - `/model` showed `OpenAI Subscription` followed by `GPT-5.6-Sol (default)`; searching `Codex Auto Review` returned `no matches`; process logs contained zero inference markers.
- Correlation keys:
  - process `pid:311884:11b85a39-a347-4745-9207-9575445729d8`
  - thread `01a0439b-287b-7950-8754-b607ecfe2b6c`
  - binary SHA256 `179a6ce8a0d5fe3a5b54b9192098727d2ed3461358bb93fa703ea51f6d6280c9`
- Raw content:
  ```text
  OpenAI Subscription
  GPT-5.6-Sol (default)
  Codex Auto Review
  no matches
  inference_markers: 0
  ```
- Interpretation: The installed product no longer reproduces the reported symptom and preserves ordinary subscription model visibility.
- Time: 2026-08-27 22:24
