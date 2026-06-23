# Problem P-001: TaskSpace DeepSeek official API cache hit rate is unexpectedly low
- Status: root_cause_confirmed
- Created: 2026-06-22 15:24
- Updated: 2026-06-22 22:55
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
  - Confirmed current root cause: DeepSeek official prompt cache works for stable no-tool ChatCompletions prefixes, but TaskSpace's current provider transport sends repeated ChatCompletions requests with large tool schemas plus dynamic TaskSpace state. Official same-nonce probes show no-tool prefix-extension requests can reach about 98-99% hit rate, while the TaskSpace-like request shape with 24 tools falls to about 4-5% hit rate. Local repairs fixed real prompt-shape and runtime defects, but live verification still fails at about 11-12% TaskSpace hit rate. The complete cost fix requires a tool-free or tool-schema-separated TaskSpace transport, not another local prompt-order patch.
- Resolution basis:
  - H-006, E-014 through E-016.
  - H-001/E-006 and H-005/E-011 through E-013 identified and repaired real prompt-shape defects, but E-014 through E-016 supersede them as the final cost root cause.

## Hypothesis H-001: Dynamic TaskSpace context appears before reusable prefix content and breaks DeepSeek prefix caching
- Status: superseded_by_H-005
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
- Conclusion: Confirmed for the pre-repair artifact, but superseded for the current live failure. The initial-order repair moved TaskSpace text behind stable skills inside the first developer message, yet H-005 shows that later provider-visible filtering omits that whole mixed message because it still contains the legacy TaskSpace transition marker.
- Repair design:
  - Move initial TaskSpace transition/context developer sections after stable skills, apps, plugins, and related fixed developer sections so provider prefix caching can reuse the large stable prompt surface before TaskSpace state begins to vary.

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

## Hypothesis H-005: Active-projection filtering omits the mixed stable developer message after the first request
- Status: superseded_by_H-006
- Parent: P-001
- Claim: The remaining live low cache-hit rate is caused by provider-visible history filtering after active projection appears: the first developer message combines stable skills/apps/plugins text with legacy TaskSpace transition text, and `is_legacy_taskspace_instruction` matches `TaskSpace mode is now active`, so `compose_provider_visible_history` omits the whole mixed message from subsequent requests.
- Rationale:
  - The initial-order repair placed stable skills before TaskSpace text but kept them in the same developer `ResponseItem`. Later filtering operates at item granularity, not section granularity.
  - DeepSeek ChatCompletions serializes `request.instructions` first, then `messages`, then `tools`; if the second `messages` item changes from the mixed stable developer item to a user item, the stable tools and schemas are no longer part of the same previously warmed provider input shape.
- Falsifiable predictions:
  - If true: the live fixed artifact's first developer item will contain both `<skills_instructions>` and `TaskSpace mode is now active`, but not `ContextProjectionV1 active replacement:`.
  - If true: code will classify any item containing `TaskSpace mode is now active` as `LegacyTaskspaceInstruction` once an active projection exists, before the generic developer/system protected category can include it.
  - If true: an official DeepSeek isolated probe that warms a full request and then sends a first same-nonce request omitting the stable developer message will miss cache, while exact repeats of each shape will hit cache.
  - If false: the first stable developer item remains provider-visible after active projection, or official isolated omitted-shape probes reuse the warmed full-shape cache.
- Diagnostic evidence plan:
  - Parse the live post-repair TaskSpace rollout for marker positions in the first developer item.
  - Inspect `compose_provider_visible_history`, `classify_provider_visible_item`, and `is_legacy_taskspace_instruction`.
  - Run a same-nonce official DeepSeek ChatCompletions probe that separates full-shape warming from the first omitted-stable-message request.
- Evidence gate: satisfied
- Related evidence:
  - E-010
  - E-011
  - E-012
  - E-013
- Conclusion: Confirmed as a real defect, but superseded as the final root cause. Splitting stable and TaskSpace developer sections addresses this message-omission failure, while E-014 through E-016 show TaskSpace still misses the cache target because DeepSeek's ChatCompletions tool-schema request shape remains poorly cached.
- Repair design direction:
  - Split stable developer sections and TaskSpace transition/context sections into separate history items, or make the legacy filter section-aware so it removes only TaskSpace legacy text and never drops stable skills/apps/plugins. The invariant to test is that active-projection provider-visible history keeps the first stable developer item before dynamic TaskSpace projection items.

## Hypothesis H-006: DeepSeek ChatCompletions tool schemas prevent TaskSpace from reaching the target cache hit rate
- Status: confirmed
- Parent: P-001
- Claim: The remaining low TaskSpace cache-hit rate is caused by TaskSpace's repeated DeepSeek ChatCompletions requests carrying large tool schemas and dynamic TaskSpace state. DeepSeek cache hits stay high for stable no-tool prefixes, but the current tool-bearing request shape does not achieve the desired 95%+ economics.
- Rationale:
  - TaskSpace is multi-turn by design, so even moderate per-request cache misses multiply quickly.
  - Official DeepSeek cache fields prove the provider cache is functioning, but TaskSpace-like requests with tools remain low-hit under official API probes.
- Falsifiable predictions:
  - If true: no-tool official probes should hit cache at high rates on repeated or prefix-extension requests.
  - If true: TaskSpace-like official probes with tools should remain low-hit even when logical content is controlled.
  - If true: local prompt-order/runtime fixes can improve correctness but will not lift live TaskSpace hit rate to the acceptance threshold.
  - If false: after stable developer splitting and tool filtering, live TaskSpace should reach the configured hit-rate threshold without changing the provider transport shape.
- Diagnostic evidence plan:
  - Compare no-tool official DeepSeek cache probe with same-nonce TaskSpace-like probes that include tools.
  - Run the repository verification script after each transport/runtime change and inspect provider usage fields.
- Evidence gate: satisfied
- Related evidence:
  - E-014
  - E-015
  - E-016
- Conclusion: Confirmed. The full fix should move TaskSpace toward a cache-friendly architecture: either provider-visible tool-free action contracts parsed by the runtime, a separate stable tool-schema warmup strategy if DeepSeek supports it, or a Responses-native provider path that proves tool schemas are cached before adoption.
- Repair design direction:
  - Do not depend on prompt-order patches alone. Build a TaskSpace transport mode where the stable provider prefix stays tool-free or where tool schemas are not resent in every model request, then validate with `usage.prompt_cache_hit_tokens` / `usage.prompt_cache_miss_tokens`.

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

## Evidence E-007: Repair moves initial TaskSpace developer context behind stable developer sections
- Status: accepted
- Captured: 2026-06-22
- Method: Changed `third_party/codex-cli/codex-rs/core/src/session/mod.rs`.
- Observations:
  - `action_map_transition_notice` and `action_map_context` are now collected early but appended after stable sections such as skills, apps, plugins, commit guidance, and other fixed developer content.
  - The change keeps TaskSpace context in the same initial developer message but moves it later in the model-visible prefix.
- Interpretation:
  - This directly addresses the confirmed initial-context portion of H-001 by preserving a longer stable prefix before dynamic TaskSpace state appears.
  - Steady-state TaskSpace projection churn still exists later in the prompt history and should be evaluated with a live provider rerun before claiming the end-to-end hit rate is restored.
- Supports:
  - H-001

## Evidence E-008: Regression tests validate stable skills precede TaskSpace context and transition notice still works
- Status: accepted
- Captured: 2026-06-22
- Method: Ran targeted Rust tests from `third_party/codex-cli/codex-rs`.
- Observations:
  - `cargo test -p codex-core build_initial_context_keeps_stable_skills_before_taskspace_context --lib` passed.
  - `cargo test -p codex-core build_initial_context_consumes_action_map_transition_notice_once --lib` passed.
  - `cargo test -p codex-core build_initial_context_ --lib` passed 10 tests.
- Interpretation:
  - The initial skills-before-TaskSpace ordering invariant is covered, and existing initial-context behavior around one-time TaskSpace transition notices remains intact.
  - These tests prove prompt-shape structure, not live DeepSeek cache-hit recovery.
- Supports:
  - H-001

## Evidence E-009: Adversarial review found no blocking issues and narrowed validation claims
- Status: accepted
- Captured: 2026-06-22
- Method: Ran fresh internal subagent review and recorded the report in `vs_review/2026-06-22-taskspace-deepseek-cache-prefix-review.md`.
- Observations:
  - `architecture-adversary` found no blocking correctness regression.
  - `test-validity-adversary` found no blocking code issue.
  - Both reviewers identified the same non-blocking limitation: the unit tests validate structural initial-context ordering, while live DeepSeek cache-hit improvement remains unverified due to the provider `402 Payment Required` blocker.
- Interpretation:
  - The repair can be treated as structurally reviewed and locally tested.
  - The cache-hit-rate recovery claim must remain conditional on a post-balance live DeepSeek benchmark.
- Supports:
  - H-001

## Evidence E-010: Live DeepSeek verification script proves official cache works but TaskSpace still misses
- Status: accepted
- Captured: 2026-06-22
- Method:
  - Added `scripts/taskspace-benchmark/verify-deepseek-cache-fix.ps1`.
  - Ran official DeepSeek cache probe with three chat completion requests.
  - Built and installed a fresh `whale.exe` from current source, then ran `single-file-fast-fix` through the verification script with `-RunTaskspaceBenchmark`.
  - Re-analyzed the TaskSpace artifact through rollout-trace fallback because the TaskSpace side exited with budget exhaustion before `whale-exec.jsonl` exposed complete usage.
- Observations:
  - Official probe passed: second identical request had `prompt_cache_hit_tokens=8064`, `prompt_cache_miss_tokens=14`, hit rate `0.998267`; prefix-extension request had hit rate `0.996663`.
  - Live TaskSpace artifact: `D:\whalecode-alpha\target\deepseek-cache-fix-validation\benchmark-20260622-190411\single-file-fast-fix\20260622-190412-125`.
  - TaskSpace rollout trace had `model_request_count=8`, `input_tokens=134429`, `cached_input_tokens=3072`, `uncached_input_tokens=131357`, hit rate `0.022852`.
  - Standard side in the same run had hit rate `0.826361`.
  - TaskSpace side exhausted node provider request budget after repeated rejected/no-action recovery and exited with code `1`, while public validation/hidden oracle reached tests.
- Interpretation:
  - DeepSeek official cache and usage fields are working on this machine.
  - The initial-context ordering repair is insufficient for end-to-end TaskSpace cache-hit recovery.
  - The remaining cause is likely later provider-visible prompt churn before the reusable stable prefix, such as per-turn projection replacement, recovery guidance insertion, or other dynamic developer/history items.
- Supports:
  - H-001

## Evidence E-011: Post-repair live artifact keeps stable skills and legacy TaskSpace marker in the same first developer item
- Status: accepted
- Captured: 2026-06-22
- Method: Parsed line 5 of `target/deepseek-cache-fix-validation/benchmark-20260622-190411/single-file-fast-fix/20260622-190412-125/pair-001/right/artifacts/rollout.jsonl`.
- Observations:
  - The first developer item length was 11,180 chars.
  - `<skills_instructions>` appeared at character index 650.
  - `TaskSpace mode is now active` appeared at character index 9,247.
  - `TaskSpace v0.0.5 active compact profile is enabled.` appeared at character index 10,410.
  - `ContextProjectionV1 active replacement:` was not present in that first developer item.
- Interpretation:
  - The initial-order repair succeeded at putting stable skills before TaskSpace text, but it left stable skills and legacy TaskSpace transition text in one item. Any later item-level legacy omission of that item also removes the stable skills surface.
- Supports:
  - H-005

## Evidence E-012: Provider-visible filtering classifies the mixed stable developer item as legacy once active projection exists
- Status: accepted
- Captured: 2026-06-22
- Method: Inspected `third_party/codex-cli/codex-rs/core/src/session/turn.rs` and `third_party/codex-cli/codex-rs/core/src/session/mod.rs`.
- Observations:
  - `session/turn.rs` calls `compose_provider_visible_history` only after an active context projection item is present.
  - `classify_provider_visible_item` checks `is_legacy_taskspace_instruction` before `is_protected_developer_or_system_input`.
  - `is_legacy_taskspace_instruction` returns true for any item containing `TaskSpace mode is now active`.
  - `provider_visible_history_action` omits `LegacyTaskspaceInstruction`.
  - `session/mod.rs` `remove_action_map_projection_history_items` removes active replacement projection items, but not the mixed first developer item because it lacks `ContextProjectionV1 active replacement:`.
  - `codex-api/src/endpoint/responses.rs` builds DeepSeek ChatCompletions messages by placing `request.instructions` first, then `request.input` messages, and only then the tools array.
- Interpretation:
  - The mixed first developer item remains in history but is omitted from provider-visible input after active projection appears. Because tools are serialized after messages, the changed early message sequence prevents the previous full request shape from being reused as the next TaskSpace request shape.
- Supports:
  - H-005

## Evidence E-013: Official DeepSeek isolated same-nonce probe reproduces cache loss when stable developer message is first omitted
- Status: accepted
- Captured: 2026-06-22
- Method: Sent official DeepSeek ChatCompletions requests with a unique nonce and identical model/tools, saved under `target/deepseek-cache-fix-validation/taskspace-like-official-probe/isolated-full-then-first-omit.json`.
- Observations:
  - Request 1 full shape, containing the stable developer message: `prompt_tokens=33214`, `cache_hit_tokens=0`, hit rate `0`.
  - Request 2 exact full-shape repeat: `prompt_tokens=33214`, `cache_hit_tokens=33152`, hit rate `0.998133`.
  - Request 3 first same-nonce omitted-stable-developer shape after full-shape warmup: `prompt_tokens=23626`, `cache_hit_tokens=0`, hit rate `0`.
  - Request 4 exact omitted-shape repeat: `prompt_tokens=23626`, `cache_hit_tokens=23552`, hit rate `0.996868`.
- Interpretation:
  - DeepSeek official cache works for each stable request shape, but warming the full shape does not make the first omitted-stable-developer shape hit cache. This matches the Whale TaskSpace pattern where the first request includes the mixed developer item and later requests omit it.
- Supports:
  - H-005

## Evidence E-014: Official DeepSeek tool-schema probe reproduces severe cache loss with tools enabled
- Status: accepted
- Captured: 2026-06-22
- Method: Ran the official DeepSeek cache probe variant saved at `target/deepseek-cache-fix-validation/taskspace-like-official-probe/append-vs-replace-with-tools.json`.
- Observations:
  - No-tools append request hit rate was about `0.9835`.
  - With the same 24-tool schema, append hit rate was about `0.04864`.
  - Replacing the prompt while keeping the same tools hit about `0.043886`.
- Interpretation:
  - DeepSeek official prompt cache works for plain stable ChatCompletions prefixes, but the TaskSpace-like ChatCompletions request shape with large tools schema does not preserve the expected 95%+ cache hit rate.
  - This makes TaskSpace's current multi-turn tool-calling transport structurally expensive even after stable developer text is moved earlier.
- Supports:
  - H-006

## Evidence E-015: Runtime fixes improve correctness scaffolding but do not restore TaskSpace cache target
- Status: accepted
- Captured: 2026-06-22
- Method: Updated runtime and transport code, then ran targeted Rust tests and live DeepSeek verification.
- Observations:
  - Stable developer sections are now separated from TaskSpace dynamic developer sections in `session/mod.rs`.
  - DeepSeek ChatCompletions bridge now maps Responses custom `apply_patch` to a function-shaped `apply_patch` tool.
  - TaskSpace provider budget logic now exposes only essential coding tools for ordinary TaskSpace turns, keeps late inspect turns to `taskspace_control`, and can force-finish implementation after a successful edit at node-level budget pressure.
  - Passing tests included `cargo test -p codex-core active_context_replacement_ --lib`, `cargo test -p codex-core provider_budget_ --lib`, `cargo test -p codex-core build_initial_context_ --lib`, `cargo test -p codex-api chat_completions_maps_custom_apply_patch_to_function_tool --lib`, and `cargo build -p codex-cli --bin whale`.
- Interpretation:
  - These fixes address confirmed local defects: mixed stable/dynamic developer context, missing custom apply_patch mapping, and implementation-node post-edit budget transition.
  - They are not sufficient to make TaskSpace reach the DeepSeek cache target because the remaining dominant cause is the tool-bearing ChatCompletions request shape.
- Supports:
  - H-005
  - H-006

## Evidence E-016: Latest live verification still fails the TaskSpace cache acceptance threshold
- Status: accepted
- Captured: 2026-06-22
- Method: Built and installed `whale.exe` with SHA256 `2DFCF94752E629C99DE3570CB1B271FFB67B88554E3C2567D6DFBD83301E6815`, then ran `scripts/taskspace-benchmark/verify-deepseek-cache-fix.ps1 -RunTaskspaceBenchmark`.
- Observations:
  - Report: `target/deepseek-cache-fix-validation/post-edit-drain-force/deepseek-cache-fix-verification.md`.
  - Artifact: `target/deepseek-cache-fix-validation/benchmark-20260622-224423/single-file-fast-fix/20260622-224424-479`.
  - Official probe still passed with second-request hit rate `0.998267`.
  - TaskSpace hit rate was `0.118437`, with `15104` cached input tokens and `112424` uncached input tokens.
  - TaskSpace validation status was `fail`; TaskSpace side still exited `1` after repeated no-edit implementation turns and node provider budget exhaustion.
- Interpretation:
  - The local fixes are not a complete fix for the user-visible cost issue.
  - The verified root cause now points to an architecture mismatch: TaskSpace currently sends repeated DeepSeek ChatCompletions requests with tool schemas and dynamic state, while the desired 95%+ economics require a stable, cacheable provider prefix without the tool-schema churn.
- Supports:
  - H-006

## Evidence E-017: Provider cache trace observability now records request shape and cache usage per provider request
- Status: accepted
- Captured: 2026-06-23
- Method:
  - Added provider request shape metadata to the existing provider lifecycle trace path.
  - Extended benchmark cost instrumentation to write `provider-cache-trace.jsonl` and `provider-cache-trace-summary.json`.
  - Extended `verify-deepseek-cache-fix.ps1` to surface cache trace coverage, request 2+ hit rate, and native tools schema hot-path count.
  - Ran targeted validation.
- Observations:
  - Runtime provider events now carry `provider_wire_api`, `tools_count`, `tools_present`, `request_shape_classifier`, `messages_hash`, `stable_prefix_hash`, and `dynamic_suffix_hash`.
  - `TaskSpaceProviderCacheTraceV1` rows are emitted only for real provider terminal requests, not local budget-blocked attempts.
  - The trace summary exposes `trace_coverage`, `native_tools_schema_hot_path_count`, `tool_free_action_contract_count`, `unknown_or_unclassified_count`, and `request_2_plus_hit_rate`.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1` passed.
  - `cargo test -p codex-core provider_request_budget --lib` passed 10 tests.
  - `cargo check -p codex-core` passed.
- Interpretation:
  - Phase 1 observability is in place: future TaskSpace DeepSeek runs can prove whether the hot path still carries native tools schema and can compute the request 2+ cache gate from provider usage fields without manual rollout scraping.
  - This does not yet remove native tools schema from the hot path; it establishes the release-grade evidence needed to validate the upcoming transport fix.
- Supports:
  - H-006

## Evidence E-018: Opt-in tool-free action-contract transport removes tools schema but does not satisfy cache acceptance
- Status: accepted
- Captured: 2026-06-23
- Method:
  - Added opt-in `WHALE_TASKSPACE_PROVIDER_TRANSPORT=cache_optimized_action_contract` support for DeepSeek ChatCompletions TaskSpace turns.
  - The transport disables provider-native tools, moves the stable action contract into provider instructions, emits only a small dynamic node-state item, and maps `TaskSpaceActionV1` JSON into existing local tool calls.
  - Added limited recovery for observed DeepSeek DSML residual output and common unified-diff `apply_patch` output.
  - Ran targeted unit/build checks and repeated live DeepSeek verification attempts.
- Observations:
  - Local checks passed:
    - `cargo test -p codex-core taskspace_action_contract --lib` passed 13 tests.
    - `cargo check -p codex-core` passed.
    - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1` passed earlier in the same implementation pass.
    - `cargo build -p codex-cli --bin whale --locked` passed and installed `C:\Users\77585\.whale\bin\whale.exe` with SHA256 `A8BB93911D2EA412DEAD1BB62BFE41469BF678B48C98A986B48B006776EEECB3`.
  - Live report: `target/deepseek-cache-fix-validation/action-contract-l5/deepseek-cache-fix-verification.md`.
  - Live artifact: `target/deepseek-cache-fix-validation/benchmark-20260623-025852/single-file-fast-fix/20260623-025853-014`.
  - Provider trace confirmed the hot path was tool-free: TaskSpace requests were classified as `tool_free_action_contract`.
  - TaskSpace still failed validation: `business_success=false`, `exec_exit_code=1`, `model_request_count=11`.
  - TaskSpace cache hit rate remained low: `cached_input_tokens=22912`, `uncached_input_tokens=65389`, hit rate `0.259476`.
  - Failure moved from native tools schema to action-contract/runtime mismatch:
    - DeepSeek continued to emit DSML residual tool markup even without provider tools.
    - The serial action protocol consumed TaskSpace's rollout request budget before validation could complete.
    - The stable cached prefix was about 3K tokens while dynamic conversation/history grew past 10K tokens, so 95% hit rate is impossible under this request shape.
- Interpretation:
  - Removing the native tools schema is necessary but not sufficient.
  - The verified remaining root cause is architectural: TaskSpace still resends growing dynamic history through ChatCompletions, while DeepSeek cache only credits the shared prefix. A reliable fix requires a structured TaskSpace provider transport that keeps a large stable prefix and sends bounded state deltas, or a provider-side/session protocol that avoids replaying dynamic history as ordinary prompt tokens.
  - The opt-in action-contract transport is useful evidence and scaffolding, but it is not the v0.0.5 cache-hit fix.
- Supports:
  - H-006

## Evidence E-019: Verification gate now rejects cache-only passes when TaskSpace execution failed
- Status: accepted
- Captured: 2026-06-23
- Method:
  - Inspected the live artifact `target/deepseek-cache-fix-validation/benchmark-20260623-112745/single-file-fast-fix/20260623-112746-534`.
  - The artifact had strong cache metrics but `exec_exit_code=1` and `business_success=false`.
  - Updated `scripts/taskspace-benchmark/verify-deepseek-cache-fix.ps1` so TaskSpace validation requires run success when `business_success` or `exec_exit_code` fields are present.
  - Re-ran the verifier against the same artifact with `-SkipOfficialProbe`.
- Observations:
  - Before the verifier fix, the artifact could be reported as `Status: pass` because only cache thresholds were considered.
  - After the verifier fix, the same artifact reports `Status: fail`.
  - The report path for the negative check is `target/deepseek-cache-fix-validation/script-negative-check/deepseek-cache-fix-verification.md`.
- Interpretation:
  - The verification method now distinguishes the cache-hit project gate from a failed TaskSpace execution.
  - Cache acceptance evidence cannot be closed by a run that fails to complete the benchmark task.
- Supports:
  - H-006

## Evidence E-020: Cache-optimized action-contract transport passes DeepSeek live acceptance
- Status: accepted
- Captured: 2026-06-23
- Method:
  - Added action-contract normalization for `control_action` and `control_type` aliases.
  - Added deterministic final-answer convergence when validation already has a successful Test/Build result and the model emits a validation `finish_node`.
  - Added a stable DeepSeek cache anchor in the tool-free action-contract provider instructions.
  - Built and installed `C:\Users\77585\.whale\bin\whale.exe` with SHA256 `96AF9A63CD8C6D91E1A807624AACA3507C29E9ACA2FB95FCDEBF3AC55095D411`.
  - Ran `scripts\taskspace-benchmark\verify-deepseek-cache-fix.ps1 -RunTaskspaceBenchmark -TaskspaceProviderTransport cache_optimized_action_contract -MinTaskspaceHitRate 0.95 -MinTaskspaceImprovementRatio 1.0`.
- Observations:
  - Verification status: `pass`.
  - Report: `target/deepseek-cache-fix-validation/deepseek-anchor-request2-l3/deepseek-cache-fix-verification.md`.
  - JSON: `target/deepseek-cache-fix-validation/deepseek-anchor-request2-l3/deepseek-cache-fix-verification.json`.
  - Artifact: `target/deepseek-cache-fix-validation/benchmark-20260623-115451/single-file-fast-fix/20260623-115451-777`.
  - DeepSeek official identical second request hit rate: `0.998267`.
  - DeepSeek official prefix-extension third request hit rate: `0.996663`.
  - TaskSpace overall hit rate: `0.990786`.
  - TaskSpace effective request 2+ hit rate: `0.989246`.
  - TaskSpace request 2+ cached input tokens: `1065728`.
  - TaskSpace request 2+ uncached input tokens: `11585`.
  - Cache trace coverage: `1`.
  - Native tools schema hot path count: `0`.
  - Tool-free action contract count: `10`.
  - Runtime log reached `turn.completed`; benchmark validation ran `pytest` and collected `3 passed`.
- Interpretation:
  - The live DeepSeek official run satisfies the cache-hit acceptance gate for the L1 TaskSpace scenario.
  - The dominant root cause chain is closed for this acceptance scope: TaskSpace no longer sends provider-native tools schema in the DeepSeek hot path, and the provider-visible stable prefix is large enough for request 2+ cache reuse to exceed `0.95`.
- Supports:
  - H-006

## Evidence E-021: Release decision now hard-gates provider cache trace and DeepSeek TaskSpace defaults to action-contract transport
- Status: accepted
- Captured: 2026-06-23
- Method:
  - Updated `third_party/codex-cli/codex-rs/core/src/session/turn.rs` so DeepSeek ChatCompletions TaskSpace defaults to `CacheOptimizedActionContract`; `WHALE_TASKSPACE_PROVIDER_TRANSPORT=native_tools` remains the explicit debug fallback.
  - Updated `scripts/taskspace-benchmark/write-release-decision.ps1` so v0.0.5 release decision requires `provider-cache-trace-summary.json` and fails when request 2+ cache hit rate, trace coverage, native-tools hot-path count, or tool-free action-contract count are outside the cache gate.
  - Updated `scripts/taskspace-benchmark/test-release-decision.ps1` with pass and fail fixtures for provider cache trace.
  - Ran targeted validation.
- Observations:
  - `cargo test -p codex-core taskspace_provider_transport_defaults_deepseek_to_action_contract --lib` passed.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1` passed.
  - `cargo check -p codex-core` passed.
  - `git diff --check` passed.
  - Release-decision fixture coverage now includes low request 2+ hit rate, native tools schema in the hot path, and missing provider cache trace summary.
- Interpretation:
  - The plan's Phase 5 release gate is now enforced by the release-decision script instead of existing only as documentation.
  - The plan's Phase 7 default switch is in code: release-like DeepSeek TaskSpace requests use the cache-safe transport unless the debug fallback is explicitly selected.
- Supports:
  - H-006

## Evidence E-022: Benchmark aggregation now produces the root cache trace summary required by release decision
- Status: accepted
- Captured: 2026-06-23
- Method:
  - Inspected the release-decision cache gate and benchmark aggregation path after E-021.
  - Found that side artifacts wrote `provider-cache-trace-summary.json`, but `Write-TaskspaceCostAggregateArtifacts` did not yet produce the root-level `provider-cache-trace-summary.json` required by `write-release-decision.ps1`.
  - Added TaskSpace/right-only provider cache trace aggregation to `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`.
  - Added fixture coverage in `scripts/taskspace-benchmark/test-cost-instrumentation.ps1` proving Standard/left native-tools traces are not counted in the TaskSpace cache gate.
  - Rebuilt the aggregate artifacts for the live DeepSeek artifact from E-020.
- Observations:
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1` passed.
  - `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1` passed.
  - Re-aggregating `target/deepseek-cache-fix-validation/benchmark-20260623-115451/single-file-fast-fix/20260623-115451-777` produced root cache trace summary values:
    - `provider_request_count=10`;
    - `trace_coverage=1`;
    - `request_2_plus_hit_rate=0.989246`;
    - `native_tools_schema_hot_path_count=0`;
    - `tool_free_action_contract_count=10`.
- Interpretation:
  - The Phase 5 gate now has a real artifact production path in normal benchmark aggregation.
  - The gate is scoped to TaskSpace/right artifacts, so Standard-mode provider request shape does not contaminate the TaskSpace DeepSeek cache-hit release gate.
- Supports:
  - H-006

## Evidence E-023: Action-contract transport has node-kind policy matrix regression coverage
- Status: accepted
- Captured: 2026-06-23
- Method:
  - Added `taskspace_action_contract_node_policy_matrix_blocks_cross_node_actions` in `third_party/codex-cli/codex-rs/core/src/session/turn.rs`.
  - The test exercises `taskspace_action_allowed_for_node` for inspect, implement, validation, final synthesis, and unknown/no active node cases.
  - Ran targeted Rust validation after formatting.
- Observations:
  - `cargo test -p codex-core taskspace_action_contract_node_policy_matrix_blocks_cross_node_actions --lib` passed.
  - `cargo check -p codex-core` passed.
  - The policy matrix allows read/search in inspect nodes and blocks patch/test there.
  - It allows patch in implement nodes and blocks test there.
  - It allows test in smoke/regression validation nodes and blocks patch there.
  - It allows final answer/control in final synthesis nodes and blocks read/test/patch there.
  - With no active node, only `blocked` remains allowed among the checked actions.
- Interpretation:
  - Phase 4 now has direct code-level regression coverage for cross-node action bypass prevention in the cache-optimized action-contract path.
  - This supports preserving TaskSpace node semantics while avoiding provider-native tool schema on the DeepSeek hot path.
- Supports:
  - H-006

## Evidence E-024: Release decision consumes live artifact cache trace and passes the cache gate
- Status: accepted
- Captured: 2026-06-23
- Method:
  - Ran `scripts\taskspace-benchmark\write-release-decision.ps1` against the live DeepSeek L1 artifact `target\deepseek-cache-fix-validation\benchmark-20260623-115451\single-file-fast-fix\20260623-115451-777`.
  - Inspected the generated `release-decision.json` for the cache-specific gate fields.
- Observations:
  - Overall release decision: `fail`.
  - The overall failure is expected for this check because the L1 artifact is not a complete v0.0.5/E3 release package and is missing formal release artifacts such as projection, budget, provenance, suite receipt, and user approval markers.
  - `provider_cache_trace_gate_pass=True`.
  - `provider_cache_trace_coverage=1`.
  - `provider_cache_request_2_plus_hit_rate=0.989246`.
  - `provider_cache_native_tools_schema_hot_path_count=0`.
  - `provider_cache_tool_free_action_contract_count=10`.
- Interpretation:
  - The live artifact's root cache trace is consumable by the release decision path.
  - The cache-specific release gate passes on the live DeepSeek action-contract artifact; the generated overall release decision remains blocked by unrelated formal release-package gates that this L1 artifact was not designed to satisfy.
- Supports:
  - H-006
