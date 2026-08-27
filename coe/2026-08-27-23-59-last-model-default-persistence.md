# Problem P-001: Routed model selection is not the default for new sessions
- Status: resolved
- Created: 2026-08-27 23:59
- Updated: 2026-08-28 00:54
- Objective: Make a successful `/model` selection update the current session and become the exact default route, model, and effort for later new sessions by reusing Codex-native persistence contracts.
- Symptoms:
  - A routed model selection updates the active thread but a new session returns to the previous configured provider/model.
- Expected behavior:
  - The current session uses the selected route/model from the next turn.
  - A new session defaults to the last successful route/model/effort selection.
  - OpenAI Subscription and OpenAI API remain distinguishable when both credential slots exist.
- Actual behavior:
  - Routed selections emit only `SelectProviderModel`, which calls `thread/settings/update`; the native `PersistModelSelection` path is bypassed.
- Impact:
  - Users must repeat model selection in each new session, and a partial persistence repair could recreate provider/model mismatches.
- Known facts:
  - Codex-native selection writes `model` and `model_reasoning_effort` through one config batch.
  - Whale already uses native `model_provider` for provider identity and `ProviderAccessMethod` for the non-secret route dimension.
  - `ProviderRuntimeRegistry::initial_route` and `build_models_manager` currently infer OpenAI access method from active auth only because config does not carry that route dimension.
- Fix criteria:
  - Routed selection keeps the existing active-thread update.
  - The new-session default persists provider/model/effort through the native config update path.
  - OpenAI route selection persists the non-secret access method without mutating or duplicating credential material.
  - Focused tests cover the atomic route/model edit, active-thread no-op selection, config reload, and new-session route restoration; the existing provider catalog/login tests continue to cover the three credential routes and dual-credential visibility.
- Current conclusion: H-001 and H-002 jointly explained the behavior. The repair extends the Codex-native atomic model config batch with the existing non-secret `ProviderAccessMethod`; it does not create another default store or mutate credential state. Current-thread updates and same-process/new-process session defaults now share one successful selection. Installed-binary E2E passed after rebuilding the CLI from the subject source; an earlier negative `/new` result was traced to a pre-change binary copied by the installer, not to the repaired source path.
- Related hypotheses:
  - H-001
  - H-002

## Hypothesis H-001: Routed selection deliberately bypasses Codex-native model persistence
- Status: confirmed
- Parent: P-001
- Claim: The routed `/model` branch emits `SelectProviderModel` instead of the native `UpdateModel`/`UpdateReasoningEffort`/`PersistModelSelection` sequence, so only active thread state changes.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - The implementation was previously changed to satisfy superseded PD7, which prohibited new-session inheritance.
- Falsifiable predictions:
  - If true: routed selections contain no config write and focused tests assert that global model defaults remain unchanged.
  - If false: routed selections already write the native model config and another loader discards it.
- Diagnostic evidence plan:
  - Signal: selection event sequence and config-write assertions.
  - Capture method: inspect `model_selection_actions`, `select_provider_model`, and existing TUI tests.
  - Differentiates from: stale process state or config loader precedence.
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed; the routed branch has no native persistence event, and R8-era tests explicitly require no global-default mutation.
- Repair design readiness: ready
- Next step: Preserve the thread update and restore native config persistence after it succeeds.
- Blocker:
  - none

## Hypothesis H-002: Provider/model persistence alone cannot distinguish both OpenAI access paths
- Status: confirmed
- Parent: P-001
- Claim: `model_provider = "openai"` and the model name do not encode OpenAI Subscription versus OpenAI API; startup derives that distinction from the native active auth mode.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - Both OpenAI groups share provider ID and model slugs while using separate credential slots.
- Falsifiable predictions:
  - If true: config has no provider-access field and startup selects `Chatgpt` only when `AuthManager::auth_cached()` is ChatGPT; otherwise it selects `ApiKey`.
  - If false: an existing config field already serializes `ProviderRoute` or the model slug uniquely identifies the route.
- Diagnostic evidence plan:
  - Signal: config schema and startup route construction.
  - Capture method: inspect `ConfigToml`, `ProviderRuntimeRegistry::initial_route`, `build_models_manager`, and auth storage updates.
  - Differentiates from: provider ID loss or credential deletion.
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: confirmed; exact restoration needs one explicit non-secret access-method config value because provider/model alone are ambiguous.
- Repair design readiness: ready
- Next step: Add `ProviderAccessMethod` to the existing model default config batch and consume it in startup route construction.
- Blocker:
  - none

## Evidence E-001: Native and routed picker branches emit different persistence events
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code inspection
- Source: `tui/src/chatwidget/model_popups.rs`
- Matched signal: The route-less branch emits `PersistModelSelection`; the routed branch emits only `SelectProviderModel`.
- Interpretation: New-session persistence is skipped at the picker boundary for every provider-aware model.
- Time: 2026-08-27 23:59

## Evidence E-002: Routed selection only updates active thread settings
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code inspection
- Source: `tui/src/app/provider_login.rs` and `tui/src/app/thread_settings.rs`
- Matched signal: Successful selection calls `ThreadSettingsUpdateParams { route, model, effort }` and performs no config write.
- Interpretation: Current-session behavior works independently of new-session defaults.
- Time: 2026-08-27 23:59

## Evidence E-003: Native config has provider/model/effort but no OpenAI access-method field
- Related hypotheses:
  - H-002
- Direction: supports
- Type: schema inspection
- Source: `config/src/config_toml.rs`
- Matched signal: `ConfigToml` exposes `model`, `model_provider`, and `model_reasoning_effort`; no `ProviderRoute` or provider access method is persisted.
- Interpretation: Adding only `model_provider = "openai"` cannot preserve the selected OpenAI group.
- Time: 2026-08-27 23:59

## Evidence E-004: Startup already treats native active auth mode as OpenAI route authority
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code inspection
- Source: `core/src/provider_runtime.rs`, `core/src/thread_manager.rs`, and `login/src/auth/manager.rs`
- Matched signal: OpenAI startup route is `Chatgpt` only for cached ChatGPT auth and otherwise `ApiKey`; native OpenAI API login writes `auth_mode = ApiKey`, while ChatGPT auth writes `auth_mode = Chatgpt`.
- Interpretation: This fallback preserves backward compatibility for existing configs, but does not prove auth storage is a safe model-default authority.
- Time: 2026-08-27 23:59

## Evidence E-005: Auth-mode mutation cannot represent environment-only OpenAI API selection safely
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code inspection
- Source: `login/src/auth/manager.rs`
- Matched signal: Route credentials accept `OPENAI_API_KEY`, while native active-auth loading gives environment precedence only to `CODEX_API_KEY`; `AuthDotJson` API-key mode requires a stored `openai_api_key`.
- Interpretation: Mutating `auth.json.auth_mode` would couple model defaults to credential storage and can fail for a supported environment-only route. The access method belongs in the existing non-secret model config batch.
- Time: 2026-08-28 00:04

## Evidence E-006: Routed selection persists one complete native config batch
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports resolution
- Type: implementation and focused test
- Source: `tui/src/config_update.rs`, `tui/src/app/provider_login.rs`, and `tui/src/config_update_tests.rs`
- Matched signal: A successful `thread/settings/update` is followed by one config batch containing `model_provider`, `model_provider_access_method`, `model`, and `model_reasoning_effort`; a successful no-op thread update still persists the chosen default.
- Interpretation: Current session and future session defaults are updated from the same selection without a second persistence system.
- Time: 2026-08-28

## Evidence E-007: Same-process new sessions reload the complete selection
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports resolution
- Type: integration-style TUI test
- Source: `tui/src/app/tests.rs`
- Matched signal: After routed selection, the test reloads config through the same method used by `/new` and observes the exact provider, access method, model, and effort.
- Interpretation: The behavior is not limited to restart-time config loading.
- Time: 2026-08-28

## Evidence E-008: Explicit access method is authoritative with backward-compatible fallback
- Related hypotheses:
  - H-002
- Direction: supports resolution
- Type: core configuration and runtime tests
- Source: `core/src/config/config_tests.rs` and `core/src/provider_runtime_tests.rs`
- Matched signal: New configs restore OpenAI Subscription from `model_provider_access_method = "chatgpt"` without active auth inference; old configs with no field retain the existing auth-based fallback.
- Interpretation: Dual OpenAI credentials no longer make the chosen route ambiguous, while existing user configs remain valid.
- Time: 2026-08-28

## Evidence E-009: Focused isolated regression passes
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports resolution
- Type: isolated test execution
- Source: `scripts/codex-upstream/run_isolated_tests.py`
- Matched signal: Two core tests and two TUI tests pass under the repository's isolated Codex test runner.
- Interpretation: The repaired persistence and restoration path is stable outside host credential and temporary-socket state.
- Time: 2026-08-28

## Evidence E-010: Installed Whale uses the selected OpenAI Subscription route
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports resolution
- Type: installed-binary E2E
- Source: `benchmarks/provider-e2e/results/WAR-20260828-004242-MODEL-PERSISTENCE-E2E-R1.json`
- Matched signal: The `hi` turn ran with route `openai/chatgpt`, model `gpt-5.6-sol`, effort `medium`, returned a normal model response, and recorded 14,524 input plus 13 output tokens.
- Interpretation: Current-session routing reaches OpenAI Subscription rather than the previously selected DeepSeek endpoint.
- Time: 2026-08-28 00:46

## Evidence E-011: Config batch, `/new`, and cold start preserve the complete selection
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports resolution
- Type: installed-binary E2E
- Source: `benchmarks/provider-e2e/results/WAR-20260828-004242-MODEL-PERSISTENCE-E2E-R1.json`
- Matched signal: The installed binary emitted `config/batchWrite`, persisted all four route/model keys, then displayed `gpt-5.6-sol medium` in both a `/new` session and a separately launched Whale process.
- Interpretation: The last successful model selection is the default across both same-process and new-process sessions.
- Time: 2026-08-28 00:52

## Evidence E-012: The first installed-binary mismatch was a stale artifact
- Related hypotheses:
  - H-001
- Direction: neutral diagnostic
- Type: build artifact inspection
- Source: installed binary hashes and `logs_2.sqlite`
- Matched signal: The first installed binary had been built before the persistence change and contained no new persistence trace string; its run emitted `thread/settings/update` but no `config/batchWrite`. Rebuilding changed the binary hash and the repeated selection emitted both the config write and persistence confirmation.
- Interpretation: Local installation copies an existing binary and does not itself compile changed source, so E2E must rebuild before reinstalling.
- Time: 2026-08-28 00:51
