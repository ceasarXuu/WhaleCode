# Problem P-001: TaskSpace DeepSeek official API cache hit rate is unexpectedly low
- Status: root_cause_confirmed
- Created: 2026-06-22 15:24
- Updated: 2026-06-22 15:24
- Objective: Identify why TaskSpace runs on DeepSeek official API show low prompt-cache hit rates, reportedly below 50%, despite the expectation that repeated agent prompts should reach 95%+ cached input.
- Symptoms:
  - TaskSpace mode uses DeepSeek official API.
  - Expected input cache hit rate is very high because most system, tool, and protocol context should be reused.
  - Observed or suspected hit rate is much lower, possibly below 50%, causing cost to rise.
- Expected behavior:
  - Stable shared prompt prefixes should dominate repeated TaskSpace requests.
  - Provider-reported cached input tokens should be high when request prefixes remain identical across adjacent model requests on the same model.
- Actual behavior:
  - Cache hit rate appears materially below expectation during TaskSpace runs.
- Impact:
  - DeepSeek official cache-miss input pricing is much higher than cache-hit input pricing, so low cache hit rate can make TaskSpace benchmark and production costs explode.
- Reproduction:
  - Use existing TaskSpace benchmark artifacts and rollout token events first.
  - If provider balance allows, run a focused low-cost TaskSpace probe and compare per-request cached versus uncached input tokens.
- Environment:
  - Repo: `D:\whalecode-alpha`
  - Branch: `whalecode-alpha`
  - Date: 2026-06-22
  - Provider/model: DeepSeek official API, `deepseek-v4-flash` or `deepseek-v4-pro`
- Known facts:
  - Previous E3 analysis confirmed TaskSpace cost is multiplied by many internal model requests and larger per-request context.
  - DeepSeek official context caching is prefix based according to public API documentation.
- Ruled out:
  - none yet
- Fix criteria:
  - Identify whether the low hit rate is caused by unstable request prefixes, usage-field mapping/reporting, model/provider differences, TaskSpace prompt construction, tool/schema ordering, dynamic context placed too early, or stale artifacts.
  - The conclusion must cite direct artifact, code, or provider documentation evidence.
  - If a code repair is needed, design it only after the causal mechanism is confirmed.
- Current conclusion:
  - Confirmed root cause: TaskSpace inserts dynamic TaskSpace developer context before the large stable skills/tool instruction surface and then rewrites TaskSpace projection developer items between model requests. DeepSeek official API context caching is prefix-based, so the large stable suffix after the changing TaskSpace prefix often cannot be reused. The recent `phase-a-benefit-B-rerun29` artifact reproduces this directly: TaskSpace cached only 18,944 of 136,638 input tokens, or 13.86%, while Standard cached 107,648 of 130,453 input tokens, or 82.52%.

## Hypothesis H-001: Dynamic TaskSpace context appears before reusable prefix content and breaks DeepSeek prefix caching
- Status: confirmed
- Parent: P-001
- Claim: TaskSpace request construction places changing task/map/history/control content early enough in the message prefix that subsequent requests diverge before most large reusable content, preventing DeepSeek from counting the later repeated content as cache hits.
- Rationale:
  - DeepSeek cache reuse is prefix-overlap based; if dynamic content is inserted before stable tool/schema/protocol blocks, a large suffix can repeat but still fail to cache.
- Falsifiable predictions:
  - If true: code building model input will show high-variance TaskSpace developer context or history before large stable sections, or artifacts will show low hit rates on requests whose early history differs.
  - If false: request construction keeps stable system/tools/developer prefixes before dynamic TaskSpace context, so another mechanism must explain low hits.
- Diagnostic evidence plan:
  - Inspect model request construction order, TaskSpace developer context insertion, and any prompt-input debug facility.
  - Compare provider-reported per-request cached/miss token ratios against inferred request sequence when artifacts are available.
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
- Conclusion: Confirmed. TaskSpace's model-visible developer order places mutable TaskSpace state ahead of the large stable skills surface. Subsequent requests repeatedly replace or append TaskSpace projection/recovery developer items, producing different provider payload hashes and very low cache hits on most requests.

## Hypothesis H-002: Cache hit rate is being calculated from the wrong aggregate denominator or field mapping
- Status: refuted_for_current_artifact
- Parent: P-001
- Claim: The apparent below-50% hit rate is a reporting or aggregation error, such as mixing total input with uncached-only fields, using direct `turn.completed` deltas instead of per-response usage, or losing provider-specific cache fields.
- Rationale:
  - Existing TaskSpace usage accounting aggregates many internal model requests into a user-turn total; a derived hit-rate can be wrong if fields are not mapped consistently.
- Falsifiable predictions:
  - If true: raw rollout `token_count.last_token_usage` events or provider usage structs will show a high cached-input share, while downstream reports show a lower hit rate from a different field or denominator.
  - If false: raw provider-derived per-response usage also shows low cached-input share.
- Diagnostic evidence plan:
  - Inspect `TokenUsage` mapping from DeepSeek/OpenAI-compatible responses, rollout token events, benchmark reports, and cost diagnostics scripts.
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-004
  - E-005
- Conclusion: Refuted for the current artifact. The low hit rate is visible in raw per-response `token_count.last_token_usage` events, and the DeepSeek ChatCompletions usage mapping directly maps `prompt_tokens_details.cached_tokens` into `cached_input_tokens`.

## Hypothesis H-003: Tool/schema or message serialization is nondeterministic across TaskSpace requests
- Status: possible_secondary
- Parent: P-001
- Claim: Even stable logical content serializes differently across requests, for example due to unordered maps, changing generated tool descriptions, changing timestamps/IDs, or varying tool availability order, reducing prefix equality.
- Rationale:
  - Provider prefix caching requires byte/token-level early-prefix stability, not semantic similarity.
- Falsifiable predictions:
  - If true: repeated request serialization will vary in tool lists, developer messages, or metadata before dynamic history needs to change.
  - If false: serialized stable request sections are deterministic and ordered.
- Diagnostic evidence plan:
  - Inspect request payload construction and any debug dump facilities; search for unordered collection iteration or dynamic fields in system/developer/tool surfaces.
- Evidence gate: partial
- Related evidence:
  - E-004
  - E-006
- Conclusion: Possible secondary contributor. Provider payload hashes differ for each logical request, but the available artifacts prove dynamic TaskSpace prompt placement and replacement are sufficient to explain the low cache rate. No direct unordered serialization bug is required for the current conclusion.

## Hypothesis H-004: Current artifacts already show high DeepSeek cache share, so the reported below-50% observation comes from a different run or projection
- Status: refuted
- Parent: P-001
- Claim: The local evidence set does not reproduce the low cache hit rate; recent clean TaskSpace artifacts may actually show high cached-input share, meaning the user-observed issue belongs to a newer run, different model, or different report calculation.
- Rationale:
  - Previous E3 evidence recorded large cached-input totals, but that may not match newer v0.0.5 or Phase A runs.
- Falsifiable predictions:
  - If true: parsing local artifacts will show cached_input_tokens / input_tokens near or above 95% for TaskSpace.
  - If false: relevant local artifacts show low provider-reported cache share.
- Diagnostic evidence plan:
  - Parse recent TaskSpace benchmark artifacts under `target/` and compare cached-input share by run, pair, and per-response event.
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: Refuted. The local `phase-a-benefit-B-rerun29` sample shows TaskSpace hit rate at 13.86%, well below the user's reported below-50% concern.

## Evidence E-001: Recent nonzero usage artifact reproduces low TaskSpace cache hit rate
- Status: accepted
- Captured: 2026-06-22
- Method: Read `target/phase-a-benefit-B-rerun29/single-file-fast-fix/20260622-073752-777/token-summary.json`.
- Observations:
  - Standard: input 130,453; cached input 107,648; uncached input 22,805; hit rate 82.52%.
  - TaskSpace: input 136,638; cached input 18,944; uncached input 117,694; hit rate 13.86%.
  - Both sides completed successfully, so this is not a failed-run zero-usage artifact.
- Interpretation:
  - The low hit rate is real in this local artifact and is worse than the user's reported below-50% concern.
- Supports:
  - H-001
  - H-004 refuted

## Evidence E-002: Per-response rollout token events show low cache hit rate on almost every TaskSpace request
- Status: accepted
- Captured: 2026-06-22
- Method: Parsed `target/phase-a-benefit-B-rerun29/single-file-fast-fix/20260622-073752-777/pair-001/right/artifacts/rollout.jsonl` for `event_msg.payload.type == token_count` and `last_token_usage`.
- Observations:
  - Nine TaskSpace model requests summed to input 136,638 and cached input 18,944.
  - Per-request cached/input ratios: 1.57%, 1.81%, 1.76%, 2.39%, 93.46%, 2.22%, 3.61%, 2.14%, 6.95%.
  - Only request 5 hit a high-cache path; the other eight requests mostly hit only a few hundred cached tokens.
- Interpretation:
  - The low rate is not caused by an aggregate denominator error. It exists at the provider-response event level.
- Supports:
  - H-001
  - H-002 refuted
  - H-004 refuted

## Evidence E-003: DeepSeek official API documents context caching as prefix matching with explicit hit and miss fields
- Status: accepted
- Captured: 2026-06-22
- Method: Checked DeepSeek official Context Caching documentation at `https://api-docs.deepseek.com/guides/kv_cache`.
- Observations:
  - DeepSeek reports cache status with `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens`.
  - The documentation states that the hard disk cache matches the prefix part of the user's input and that cache construction is best-effort.
- Interpretation:
  - Repeated content placed after a changing prefix cannot be assumed to hit, even when the repeated content is large and semantically identical.
- Supports:
  - H-001

## Evidence E-004: TaskSpace provider payload hashes differ on every logical request despite active projection replacement passing
- Status: accepted
- Captured: 2026-06-22
- Method: Read `provider-request-events.jsonl` and `exact-payload-scan-events.jsonl` from the TaskSpace side of `phase-a-benefit-B-rerun29`.
- Observations:
  - Logical requests 1 through 8 had different `provider_payload_sha256` values.
  - Payload sizes changed across the same run: 56,886; 58,239; 63,163; 66,386; 67,383; 69,645; 71,098; then 20,950 bytes.
  - Exact payload scans passed, `active_projection_present` was true, `legacy_taskspace_history_present` was false, and `large_raw_output_tokens` was zero.
- Interpretation:
  - The low cache hit rate is not caused by legacy TaskSpace history leaking or raw output replay. The active compact projection still changes the provider-visible prefix enough to invalidate most cache reuse.
- Supports:
  - H-001
  - H-003 as possible secondary

## Evidence E-005: DeepSeek ChatCompletions usage mapping preserves provider cached token field directly
- Status: accepted
- Captured: 2026-06-22
- Code evidence:
  - `third_party/codex-cli/codex-rs/codex-api/src/sse/chat_completions.rs` maps `ChatUsage.prompt_tokens` to `TokenUsage.input_tokens`.
  - The same file maps `ChatUsage.prompt_tokens_details.cached_tokens` to `TokenUsage.cached_input_tokens`.
  - `third_party/codex-cli/codex-rs/model-provider-info/src/lib.rs` configures the DeepSeek provider with `WireApi::ChatCompletions`.
- Interpretation:
  - The currently inspected low cache hit rate is not explained by reversed or dropped hit/miss fields in local code.
- Supports:
  - H-002 refuted

## Evidence E-006: Code and artifact order place dynamic TaskSpace context before stable skills instructions
- Status: accepted
- Captured: 2026-06-22
- Method: Parsed first TaskSpace developer response item in `rollout.jsonl` and inspected model-visible developer section construction in `third_party/codex-cli/codex-rs/core/src/session/mod.rs` plus per-request projection injection in `third_party/codex-cli/codex-rs/core/src/session/turn.rs`.
- Observations:
  - The first TaskSpace developer item order was: permissions block 620 chars, TaskSpace bootstrap 1,163 chars, TaskSpace compact profile 770 chars, then `<skills_instructions>` 8,597 chars.
  - Later developer items repeatedly contained TaskSpace projection or recovery text of roughly 0.5-2.3 KB.
  - `session/mod.rs` pushes `action_map_runtime.build_developer_context()` before `AvailableSkillsInstructions`.
  - `session/turn.rs` calls `remove_action_map_projection_history_items()` and records a fresh action-map projection before constructing `sampling_request_input`.
- Interpretation:
  - The largest stable developer surface in this artifact, the skills list, is after dynamic TaskSpace context instead of before it. This directly breaks the desired DeepSeek prefix-cache shape.
- Supports:
  - H-001
