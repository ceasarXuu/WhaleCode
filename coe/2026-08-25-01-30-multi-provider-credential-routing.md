# Problem P-001: Multi-provider credentials are persisted but misreported or not consumed
- Status: fixed
- Created: 2026-08-25 01:30
- Updated: 2026-08-25 01:44
- Objective: Ensure DeepSeek API and OpenAI ChatGPT subscription credentials are reported and consumed through their selected provider routes in Whale v0.0.6.
- Symptoms:
  - A DeepSeek API key saved from the provider UI is reported configured, but DeepSeek model refresh says `DEEPSEEK_API_KEY` is missing.
  - A workspace with no stored ChatGPT tokens is reported by `whale login status` as logged in using ChatGPT.
  - OpenAI ChatGPT model refresh returns HTTP 401 after the attempted subscription registration.
- Expected behavior:
  - Route-bound model discovery reads the credential slot persisted for that route.
  - ChatGPT is configured only when usable ChatGPT token material exists.
- Actual behavior:
  - Provider credential status reports DeepSeek configured and ChatGPT unconfigured, while model discovery and the legacy login status disagree with those facts.
- Impact:
  - Whale v0.0.6 multi-provider onboarding and model selection cannot reliably use or report the registered provider credentials.
- Reproduction:
  - Start the workspace-installed Whale v0.0.6 without `DEEPSEEK_API_KEY`, register a DeepSeek key in the provider UI, then refresh models.
  - Run `whale login status` against the resulting workspace auth file containing only the DeepSeek slot.
- Environment:
  - Ubuntu 24.04, Whale 0.0.6, branch `whalecode-alpha`, commit `2279c04f1`, workspace id `whalecode-alpha-48d2219088`.
- Known facts:
  - The app-server credential status API reports OpenAI ChatGPT false, OpenAI API false, and DeepSeek API true.
  - A fresh app-server process still emits the missing `DEEPSEEK_API_KEY` model-refresh error.
  - `auth.json` contains a non-empty DeepSeek slot and no ChatGPT tokens.
- Ruled out:
  - A stale TUI process is not sufficient to explain the DeepSeek failure because a fresh app-server reproduces it.
- Fix criteria:
  - A focused test reproduces each original failure before repair and passes after repair.
  - A fresh installed process reports the three credential slots consistently and no longer emits the DeepSeek missing-env error for the stored key.
  - ChatGPT status is not positive without ChatGPT token material, while native ChatGPT login behavior remains intact.
- Current conclusion: Fixed. Provider-only DeepSeek credentials no longer activate native ChatGPT auth, while route-bound DeepSeek discovery remains available.
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-002 confirmed by E-003 through E-006 and fixed by E-007 and E-008.
- Close reason:
  - not closed

## Hypothesis H-001: Route-bound DeepSeek model discovery loses the persisted credential
- Status: refuted
- Parent: P-001
- Claim: The aggregated model-catalog refresh constructs or invokes the DeepSeek endpoint without preserving route-bound `AuthManager::auth_for_route`, causing fallback to the native provider `env_key` path even though the stored DeepSeek slot exists.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - Runtime status and storage confirm the key, while the model endpoint emits the exact missing-environment error used by the legacy provider path.
- Falsifiable predictions:
  - If true: a focused catalog-refresh test with only a stored DeepSeek key and no environment key will fail before repair, and the failure path will show an unbound or unauthenticated DeepSeek endpoint.
  - If false: the endpoint will resolve a route-bound `CodexAuth` from storage and attach it, moving the failure beyond credential resolution.
- Diagnostic evidence plan:
  - Prediction or clause under test: Stored-only DeepSeek credentials are lost specifically between route catalog assembly and endpoint authentication.
  - Signal: Focused test/probe result plus construction and call-chain evidence.
  - Capture method: Trace the aggregated manager construction and add the smallest focused regression test using file-backed auth with the environment key absent.
  - Event name or marker:
    - `model/list`
  - Correlation keys:
    - provider route `deepseek/apiKey`
  - Differentiates from:
    - stale process state, failed persistence, invalid key, or remote DeepSeek rejection
  - Supports if:
    - status reads the stored slot while the same refresh path reaches provider env lookup or has no route auth
  - Refutes if:
    - refresh attaches the stored key and receives a remote/provider response
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: refuted; route-bound auth resolves the stored key, and the observed missing-env error originates from the legacy route-less manager activated by H-002.
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: Closed in favor of H-002.
- Blocker:
  - none
- Close reason:
  - Superseded by the H-002 interaction mechanism.

## Hypothesis H-002: DeepSeek-only auth files fall through to Codex's legacy ChatGPT default
- Status: confirmed
- Parent: P-001
- Claim: `AuthDotJson::resolved_mode` defaults any record without another active auth marker to ChatGPT, so adding a provider-only DeepSeek slot makes legacy login status and ChatGPT catalog requests believe ChatGPT auth exists even though `tokens` is absent.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - The persisted record has only the DeepSeek provider slot, the route status correctly reports ChatGPT false, but native login status reports ChatGPT true.
- Falsifiable predictions:
  - If true: a focused test with only `deepseek_api_key` will resolve legacy mode as ChatGPT and construct ChatGPT auth without tokens before repair.
  - If false: another auth source or persisted token store will explain the positive status.
- Diagnostic evidence plan:
  - Prediction or clause under test: The legacy fallback in `resolved_mode` is sufficient to create the false positive from a DeepSeek-only file.
  - Signal: Focused unit/CLI test and file-open trace proving the exact workspace `auth.json` is the source.
  - Capture method: Reproduce with isolated file storage and compare with Codex-native token-bearing and truly empty auth records.
  - Event name or marker:
    - `login status`
  - Correlation keys:
    - workspace auth path
  - Differentiates from:
    - keyring state, environment credentials, or official Codex-home fallback
  - Supports if:
    - isolated DeepSeek-only storage returns ChatGPT while provider status says ChatGPT false
  - Refutes if:
    - removing external/keyring sources removes the false positive or token material is discovered
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
  - E-005
  - E-006
  - E-007
  - E-008
- Conclusion: confirmed; Whale's new provider-only field violates the native Codex invariant that a stored auth record with no other active marker is a legacy ChatGPT record.
- Repair design readiness: ready
- Next step: Gate native active-auth loading on native auth material while leaving route storage lookup unchanged.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Persisted credential status is route-correct
- Related hypotheses:
  - H-001
- Direction: supports
- Type: probe
- Source: workspace-installed app-server `account/providerCredentials/read`
- Prediction or plan link:
  - H-001 stored credential exists independently of the environment
- Matched signal:
  - DeepSeek API configured true; both OpenAI routes false
- Correlation keys:
  - workspace id `whalecode-alpha-48d2219088`
- Raw content:
  ```text
  openai/chatgpt=false
  openai/apiKey=false
  deepseek/apiKey=true
  ```
- Interpretation: Persistence succeeded and the provider-status route reader can see the DeepSeek slot.
- Time: 2026-08-25 01:24

## Evidence E-002: Fresh process still loses DeepSeek credentials during model refresh
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: fresh workspace-installed `whale app-server --listen stdio://`
- Prediction or plan link:
  - H-001 failure is in the refresh path rather than stale TUI state
- Matched signal:
  - `model/list` emitted the missing-environment error before initialize completed
- Correlation keys:
  - Whale 0.0.6
- Raw content:
  ```text
  failed to refresh available models: Missing environment variable: `DEEPSEEK_API_KEY`.
  ```
- Interpretation: Restarting does not repair the stored-key lookup, ruling out process staleness as the complete cause.
- Time: 2026-08-25 01:24

## Evidence E-003: Login status reads the workspace file and reports ChatGPT
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: `strace -f -e trace=openat whale -c 'cli_auth_credentials_store="file"' login status`
- Prediction or plan link:
  - H-002 exact workspace file is sufficient for the false positive
- Matched signal:
  - Opened workspace `auth.json`; printed `Logged in using ChatGPT`
- Correlation keys:
  - workspace auth path
- Raw content:
  ```text
  openat(.../home/auth.json, O_RDONLY|O_CLOEXEC) = 10
  Logged in using ChatGPT
  ```
- Interpretation: Keyring and a different Codex home are not required for the false-positive status.
- Time: 2026-08-25 01:23

## Evidence E-004: Workspace auth shape contains no ChatGPT token material
- Related hypotheses:
  - H-002
- Direction: supports
- Type: config
- Source: redacted structural query of workspace `auth.json`
- Prediction or plan link:
  - H-002 DeepSeek-only records fall through to ChatGPT mode
- Matched signal:
  - Non-empty `DEEPSEEK_API_KEY`; null OpenAI API key; no `tokens` or `auth_mode` field
- Correlation keys:
  - auth file timestamp 2026-08-25 01:18:01
- Raw content:
  ```text
  OPENAI_API_KEY=null
  DEEPSEEK_API_KEY=<present>
  tokens=<absent>
  auth_mode=<absent>
  ```
- Interpretation: The reported ChatGPT state is not backed by persisted ChatGPT tokens.
- Time: 2026-08-25 01:18

## Evidence E-005: Isolated DeepSeek-only auth reproduces false ChatGPT state
- Related hypotheses:
  - H-002
- Direction: supports
- Type: test
- Source: `codex-login auth::manager::tests::deepseek_only_credentials_do_not_create_legacy_chatgpt_auth`
- Prediction or plan link:
  - H-002 isolated DeepSeek-only storage returns ChatGPT before repair
- Matched signal:
  - Assertion that legacy auth must be empty failed twice at the same point
- Correlation keys:
  - nextest run `0cc7988f-3451-417e-b5ba-531ac34b4d2e`
- Raw content:
  ```text
  a provider-only DeepSeek credential must not become legacy ChatGPT auth
  1 test run: 0 passed, 1 failed
  ```
- Interpretation: The file shape alone is sufficient; no keyring, environment auth, remote request, or TUI state is needed.
- Time: 2026-08-25 01:34

## Evidence E-006: Native Codex fallback and Whale route lookup explain all symptoms
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: code-location
- Source: official Codex `login/src/auth/manager.rs`; local `AuthDotJson::resolved_mode`, `AuthManager::auth_for_route`, `ProviderModelsCatalog::list_model_groups`, and provider auth resolution
- Prediction or plan link:
  - H-001 and H-002 call-chain differentiation
- Matched signal:
  - Official Codex defaults an otherwise unclassified stored record to ChatGPT; Whale route lookup separately reads `deepseek_api_key` successfully; catalog availability trusts cached ChatGPT auth before checking stored tokens.
- Correlation keys:
  - route `deepseek/apiKey`; route `openai/chatgpt`
- Raw content:
  ```text
  Native resolved_mode fallback: AuthMode::Chatgpt
  Whale deepseek route: stored.deepseek_api_key -> CodexAuth::ApiKey
  Whale ChatGPT route: cached ChatGPT auth -> available
  ```
- Interpretation: H-001's lost-route-key mechanism is false. H-002 creates the bogus cached ChatGPT state, which marks the subscription route available, skips native login, triggers 401, and makes the legacy DeepSeek manager attempt an env-backed refresh.
- Time: 2026-08-25 01:36

## Evidence E-007: Minimal native-auth gate passes focused and subsystem regressions
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: local nextest runs after adding `AuthDotJson::has_native_auth_material`
- Prediction or plan link:
  - H-002 repair must make DeepSeek-only legacy auth empty without breaking native Codex auth or route storage
- Matched signal:
  - Diagnostic test changed from repeatable failure to pass; all codex-login tests, CLI status test, and app-server credential isolation test passed.
- Correlation keys:
  - nextest runs `2077839d-9be5-46b7-b1e5-f10d64365db6`, `48c68467-768d-4394-b832-c936f1bc28fe`, `ea93b766-6c0e-4f13-9d2e-a9f10f095376`, `c50dc882-34ac-42fd-b498-e8962e747cf5`
- Raw content:
  ```text
  codex-login: 198 passed, 0 failed
  CLI DeepSeek-only login status: passed
  app-server provider credential isolation: passed
  cargo fmt --check: passed
  git diff --check: passed
  ```
- Interpretation: The repair is narrow at the native-auth loading boundary and preserves native ChatGPT, OpenAI API, DeepSeek route, storage, refresh, and logout behaviors covered by the subsystem suite.
- Time: 2026-08-25 01:43

## Evidence E-008: Installed workspace reproducer no longer shows either provider symptom
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: attested workspace Whale 0.0.6 at commit `70bb91272`; `login status`, app-server initialize, `account/providerCredentials/read`, and `model/list`
- Prediction or plan link:
  - P-001 original runtime symptoms must disappear with the same stored credentials
- Matched signal:
  - Legacy status is not logged in; DeepSeek remains configured and available; both OpenAI routes are missing credentials; model list succeeds without DeepSeek missing-env or ChatGPT catalog 401 errors.
- Correlation keys:
  - workspace id `whalecode-alpha-48d2219088`
  - installed binary SHA-256 `a01eebc2660c50f829cbf29fc8786ca99d3af0a43ff85fca97cd2237dd090263`
- Raw content:
  ```text
  whale login status: Not logged in
  openai/chatgpt: configured=false, availability=missingCredentials
  openai/apiKey: configured=false, availability=missingCredentials
  deepseek/apiKey: configured=true, availability=available
  model/list: success
  ```
- Interpretation: The original credential misclassification and its downstream model-refresh failures are absent in a fresh installed process. The remaining featured-plugin 401 is a separate unauthenticated plugin-prewarm request.
- Time: 2026-08-25 01:44

# Problem P-002: OpenAI subscription login succeeds but the pending provider switch fails
- Status: fixed
- Created: 2026-08-25 06:00
- Updated: 2026-08-25 06:27
- Objective: Complete a TUI switch from DeepSeek to an authenticated OpenAI subscription without a false DeepSeek environment error or a rejected thread-settings update.
- Symptoms:
  - OpenAI subscription tokens are persisted and native login status reports ChatGPT.
  - The post-login model refresh logs a missing `DEEPSEEK_API_KEY` environment variable even though the DeepSeek provider slot is stored.
  - The TUI reports `thread/settings/update failed` immediately after login.
- Expected behavior:
  - Built-in provider runtimes resolve their selected credential slot.
  - A provider switch submits a target route, target model, and collaboration mode that agree.
- Actual behavior:
  - The compatibility model manager and initial session provider remain route-less.
  - The TUI sends the target OpenAI route/model together with the previous DeepSeek collaboration-mode model.
- Impact:
  - The browser login itself succeeds, but the user cannot complete the OpenAI subscription switch.
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Current conclusion: Fixed. The target route/model/collaboration mode now transition atomically, and every built-in startup client retains its selected credential route.
- Resolution basis:
  - H-003 through H-005 confirmed by E-009 through E-012 and fixed by E-013.
- Close reason:
  - not closed

## Hypothesis H-003: TUI carries the previous provider model into the route transition
- Status: confirmed
- Parent: P-002
- Claim: `sync_active_thread_provider_model_setting` sends the current collaboration mode unchanged, so core validation prefers its DeepSeek model over the selected OpenAI model and rejects the route.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Falsifiable predictions:
  - The same OpenAI route update succeeds when its collaboration-mode model is OpenAI and fails when that field remains DeepSeek.
- Diagnostic evidence plan:
  - Signal: paired JSON-RPC `thread/settings/update` probes against the same authenticated empty thread.
  - Differentiates from: invalid ChatGPT tokens, unavailable OpenAI model, or failed credential persistence.
- Evidence gate: satisfied
- Related evidence:
  - E-009
  - E-010
- Conclusion: confirmed; the contradictory TUI payload exactly reproduces the user's post-login error.
- Repair design readiness: ready
- Next step: Resolve a route default when needed and derive the submitted collaboration mode from the target model/effort.

## Hypothesis H-004: Built-in default managers bypass route credential storage
- Status: confirmed
- Parent: P-002
- Claim: `build_models_manager` and the initial session provider use route-less constructors, so DeepSeek falls back to its environment-only auth path even when its provider slot is stored.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Falsifiable predictions:
  - Fresh app-server initialization and thread start emit the missing-environment error before any model turn, while route credential status remains configured.
- Diagnostic evidence plan:
  - Signal: fresh-process logs plus constructor call-chain inspection.
  - Differentiates from: stale TUI process, invalid DeepSeek key, or failed remote authentication.
- Evidence gate: satisfied
- Related evidence:
  - E-011
- Conclusion: confirmed; both compatibility discovery and initial session startup are constructed without the built-in route.
- Repair design readiness: ready
- Next step: Use route-bound managers/providers for built-in initial routes while preserving custom-provider construction.

## Hypothesis H-005: ModelClient reconstructs and loses the session provider route
- Status: confirmed
- Parent: P-002
- Claim: Even after session configuration resolves a route-bound provider, `ModelClient::new` reconstructs it from provider metadata and the auth manager without carrying the route, restoring legacy environment lookup during auth prewarm.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-004
- Falsifiable predictions:
  - Route-bound session construction still emits the DeepSeek missing-environment warning until the route reaches the `ModelClient` provider constructor.
- Diagnostic evidence plan:
  - Signal: installed startup log plus a focused `ModelClient::prewarm_auth` test using a stored DeepSeek credential with environment auth disabled.
  - Differentiates from: model catalog refresh, TUI settings payload, or invalid stored credentials.
- Evidence gate: satisfied
- Related evidence:
  - E-012
- Conclusion: confirmed; the route was present in `SessionConfiguration` but discarded at the final model-client construction boundary.
- Repair design readiness: ready
- Next step: Preserve the upstream-compatible constructor and add a crate-private route-aware constructor used only by the session path.

## Evidence E-009: OpenAI credentials and native login are valid
- Related hypotheses:
  - H-003
- Direction: supports
- Type: runtime-state
- Source: redacted workspace `auth.json` shape and `whale login status`
- Matched signal:
  - `auth_mode=chatgpt`; access token, refresh token, and account ID present; status is `Logged in using ChatGPT`.
- Interpretation: Browser callback and credential persistence completed successfully.
- Time: 2026-08-25 05:57

## Evidence E-010: Only the contradictory TUI settings payload fails
- Related hypotheses:
  - H-003
- Direction: supports
- Type: reproduction
- Source: fresh installed app-server using the same workspace credentials and an empty DeepSeek thread
- Matched signal:
  - `openai/chatgpt + gpt-5.4` succeeds.
  - The same route/model plus `collaborationMode.model=deepseek-v4-flash` returns `a model available on that route`.
- Interpretation: The error is a stale cross-provider model snapshot, not an OpenAI authentication failure.
- Time: 2026-08-25 06:03

## Evidence E-011: Fresh startup repeats environment-only DeepSeek lookup
- Related hypotheses:
  - H-004
- Direction: supports
- Type: reproduction
- Source: two fresh installed app-server processes
- Matched signal:
  - Initialization and thread start emit `Missing environment variable: DEEPSEEK_API_KEY` while the stored DeepSeek route remains configured.
- Interpretation: A route-less built-in manager/provider is still active outside the grouped catalog path.
- Time: 2026-08-25 06:03

## Evidence E-012: Route-aware ModelClient prewarm resolves stored provider credentials
- Related hypotheses:
  - H-005
- Direction: supports
- Type: fix-validation
- Source: focused core unit test and constructor call-chain inspection
- Matched signal:
  - `route_bound_model_client_prewarms_with_stored_deepseek_credentials` passes with environment auth disabled.
  - `cold_resume_rebinds_last_successful_provider_runtime` and the TUI provider-route tests remain green.
- Interpretation: Carrying the selected route through the model-client boundary removes the last environment-only fallback without changing the public upstream constructor or unrelated callers.
- Time: 2026-08-25 06:20

## Evidence E-013: Installed OpenAI subscription switch completes without provider-auth errors
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Direction: supports
- Type: fix-validation
- Source: fresh workspace-installed app-server at commit `798a9d960`; initialize, `model/list`, `thread/start`, and `thread/settings/update`
- Matched signal:
  - OpenAI Subscription is available and returns the native ChatGPT model catalog.
  - OpenAI API is visibly `missingCredentials`; DeepSeek API is available.
  - A DeepSeek thread starts without `Missing environment variable: DEEPSEEK_API_KEY`.
  - Updating the same thread to `openai/chatgpt + gpt-5.6-sol` succeeds, and the emitted collaboration mode also contains `gpt-5.6-sol`.
- Correlation keys:
  - workspace id `whalecode-alpha-48d2219088`
  - installed binary SHA-256 `635adb29b4f63dd507c525c10c6cc012672b64ff7e54c9cb26fb111026a39116`
- Interpretation: The original post-login failure is absent across catalog, startup auth prewarm, and next-turn provider transition paths. No model turn was sent.
- Time: 2026-08-25 06:27

## Evidence E-014: User-operated TUI registers the OpenAI subscription route successfully
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Direction: supports
- Type: user-feedback
- Source: workspace runtime logs from the user-operated Whale process started at 2026-08-25 07:37:46 CST
- Matched signal:
  - OpenAI model discovery returns HTTP 200 with `auth_mode="Chatgpt"` and an attached authorization header.
  - Request 5 submits `openai/chatgpt + gpt-5.6-sol`, with the collaboration-mode model also set to `gpt-5.6-sol`.
  - Core accepts the submission and emits `thread/settings/updated` for thread `01a03623-3757-7f32-ad30-278df460f75d`.
  - No provider, authentication, model, or settings error occurs in the new process through the observed switch.
- Interpretation: The user's UI operation successfully registered the OpenAI subscription route for the next turn. No post-switch turn exists yet, so actual inference routing remains unobserved rather than failed.
- Time: 2026-08-25 07:39

# Problem P-003: OpenAI model footer shows an unresolved or incorrect reasoning default
- Status: diagnosed
- Created: 2026-08-25 07:45
- Updated: 2026-08-25 07:52
- Objective: Make the active OpenAI model footer show the effective reasoning effort supplied by authoritative model metadata.
- Symptoms:
  - After switching to OpenAI Subscription, the composer footer shows `gpt-5.6-sol default`.
  - Earlier runtime catalog evidence reported `low` as the model default, while official OpenAI documentation specifies `medium`.
- Expected behavior:
  - The catalog should preserve the authoritative default reasoning effort.
  - The footer should display the effective value rather than the unresolved sentinel `default`.
- Actual behavior:
  - The visible footer does not expose the effective reasoning effort, and available evidence suggests the local model metadata may also disagree with OpenAI.
- Impact:
  - Users cannot tell which reasoning effort the next OpenAI turn will use.
- Related hypotheses:
  - H-006
  - H-007
- Current conclusion: The footer exposes an unresolved sentinel even though the runtime resolves the next request to the bundled Codex product default, currently `low` for GPT-5.6 Sol.

## Hypothesis H-006: Whale's fallback catalog duplicates and misstates OpenAI model metadata
- Status: confirmed
- Parent: P-003
- Claim: When the ChatGPT models endpoint returns no models, Whale falls back to a locally maintained catalog whose `gpt-5.6-sol` default reasoning effort is `low` instead of the official `medium`.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Falsifiable predictions:
  - Repository or runtime cache data contains an independently defined `gpt-5.6-sol` entry with default effort `low`.
- Diagnostic evidence plan:
  - Compare the official model page, runtime models response/cache, and repository catalog source.
- Evidence gate: satisfied
- Related evidence:
  - E-015
  - E-016
- Conclusion: Confirmed with qualification. Whale carries the Codex upstream bundled catalog rather than a new multi-provider catalog; its Codex product default is `low`, which intentionally overrides the API omission default of `medium`.
- Repair design readiness: ready

## Hypothesis H-007: Composer footer renders the unresolved Default sentinel
- Status: confirmed
- Parent: P-003
- Claim: Provider switching leaves session effort unset, and the footer formats that state as `default` without resolving it through the selected model's metadata.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Falsifiable predictions:
  - The accepted settings update contains `effort=None`, while footer rendering maps that value directly to `default` instead of the catalog default.
- Diagnostic evidence plan:
  - Trace the accepted runtime settings and the TUI footer formatting/model metadata lookup.
- Evidence gate: satisfied
- Related evidence:
  - E-014
  - E-017
  - E-018
- Conclusion: Confirmed. Provider selection submits no effort, the footer renders that as `default`, and request construction later resolves the same state to the catalog's `low`.
- Repair design readiness: ready

## Evidence E-015: Official GPT-5.6 Sol default reasoning effort is medium
- Related hypotheses:
  - H-006
- Direction: supports
- Type: external-authority
- Source: official OpenAI GPT-5.6 Sol model documentation, fetched 2026-08-25
- Matched signal:
  - The model page lists `none`, `low`, `medium (default)`, `high`, `xhigh`, and `max`.
- Interpretation: Any Whale metadata that declares `low` as the model default is inconsistent with current official OpenAI documentation.
- Time: 2026-08-25 07:45

## Evidence E-016: The low default comes from the vendored Codex bundled catalog
- Related hypotheses:
  - H-006
- Direction: supports
- Type: provenance
- Source: `models-manager/models.json`, manager fallback logic, vendor provenance, and git history
- Matched signal:
  - `gpt-5.6-sol.default_reasoning_level` is `low` in the bundled catalog imported by the Codex 0.147 substrate sync.
  - The current vendor contract identifies the tree as an OpenAI Codex upstream substrate, now based on rust-v0.149.0.
  - The authenticated ChatGPT models response cached zero models, so `apply_remote_models` retained the bundled catalog.
- Interpretation: This is a locally versioned upstream Codex product catalog, not a second catalog introduced for multi-provider. Its `low` value is an explicit Codex client override of the API's omission default.
- Time: 2026-08-25 07:51

## Evidence E-017: Provider switching leaves effort unresolved and the footer prints the sentinel
- Related hypotheses:
  - H-007
- Direction: supports
- Type: runtime-and-code-path
- Source: user-operated settings log and TUI provider/footer code
- Matched signal:
  - The accepted switch contains `effort=None` and collaboration `reasoning_effort=None`.
  - `effective_reasoning_effort` therefore returns `None`.
  - `status_line_reasoning_effort_label` maps `None` directly to the literal `default`.
- Interpretation: The footer reports storage state rather than the effective reasoning effort.
- Time: 2026-08-25 07:51

## Evidence E-018: Request construction resolves the same state to low
- Related hypotheses:
  - H-007
- Direction: supports
- Type: code-path
- Source: core request construction and bundled GPT-5.6 Sol metadata
- Matched signal:
  - `ModelClient::build_reasoning` uses the explicit effort or falls back to `model_info.default_reasoning_level`.
  - For bundled GPT-5.6 Sol metadata that fallback is `low`.
- Interpretation: The visible `default` label and actual next-request effort disagree; actual runtime behavior is `low`.
- Time: 2026-08-25 07:52

# Problem P-004: OpenAI subscription turns are rejected as an obsolete Codex client
- Status: fixed
- Created: 2026-08-25 08:02
- Updated: 2026-08-26 21:44
- Objective: Complete a real OpenAI subscription turn from the installed Whale v0.0.6 client.
- Symptoms:
  - A `gpt-5.6-sol low` turn containing `hi` receives HTTP 400 saying the model requires a newer Codex version.
  - The same installed process warns that `codex-code-mode-host` is missing.
- Expected behavior:
  - Whale keeps its own v0.0.6 product identity while speaking the OpenAI Codex protocol at the version of its vendored upstream substrate.
  - Workspace installation includes the enabled Code Mode host.
- Actual behavior:
  - OpenAI model discovery and User-Agent were separated from Whale's product version, but the OpenAI provider's inference-only `version` HTTP header still advertises `0.0.6`.
  - The installer copies optional helpers but omits `codex-code-mode-host` entirely.
- Impact:
  - OpenAI subscription inference is unusable for current models, remote model discovery falls back to bundled data, and Code Mode fails closed.
- Related hypotheses:
  - H-008
  - H-009
  - H-010
  - H-011
  - H-012
  - H-013
  - H-014
- Current conclusion: Fixed. Whale retains product version `0.0.6` while all OpenAI protocol compatibility surfaces use stable Codex `0.149.1`; native originator inheritance is restored. The installed binary completed a schema-correct `openai/chatgpt + gpt-5.6-sol low` turn with exactly one provider request and no error.
- Resolution basis:
  - H-014 confirmed by E-030 through E-032, repaired by E-033, and live-validated by E-034.
- Close reason:
  - The original installed-client reproduction now completes successfully within the approved one-request budget.

## Hypothesis H-008: Whale product version is incorrectly used as the OpenAI Codex client version
- Status: confirmed
- Parent: P-004
- Claim: The workspace version bump to `0.0.6` changed both the models `client_version` and request `User-Agent` below the minimum accepted for `gpt-5.6-sol`, despite Whale containing the Codex 0.149.0 substrate.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Falsifiable predictions:
  - The failed turn advertises a version derived from `CARGO_PKG_VERSION`, while local upstream provenance and model metadata require a materially newer Codex compatibility version.
- Diagnostic evidence plan:
  - Trace the models-query and default-header constructors; compare their version source with the vendored upstream provenance and model minimum.
- Evidence gate: satisfied
- Related evidence:
  - E-019
  - E-020
- Conclusion: Confirmed. Both protocol surfaces derive from Whale's workspace package version instead of the vendored Codex compatibility version.
- Repair design readiness: ready

## Hypothesis H-009: Workspace installation omits the enabled Code Mode host
- Status: confirmed
- Parent: P-004
- Claim: The build contains a `codex-code-mode-host` package, but `install-whale-local.sh` never copies that executable into the workspace binary directory.
- Layer: contributing
- Factor relation: part_of
- Depends on:
  - none
- Falsifiable predictions:
  - The host exists as a workspace binary target but is absent from the installer's helper manifest and installed directory.
- Diagnostic evidence plan:
  - Compare Cargo workspace members, installer helper list, and the installed binary directory.
- Evidence gate: satisfied
- Related evidence:
  - E-019
  - E-021
- Conclusion: Confirmed. The installer helper list omits the host, so an enabled feature can only fail closed after launch.
- Repair design readiness: ready

## Evidence E-019: User's real OpenAI turn reproduces both failures
- Related hypotheses:
  - H-008
  - H-009
- Direction: supports
- Type: reproduction
- Source: user-operated installed Whale v0.0.6 TUI and runtime trace for thread `01a03623-3757-7f32-ad30-278df460f75d`
- Matched signal:
  - The `hi` turn uses `openai/chatgpt`, `gpt-5.6-sol`, and `low`, then receives HTTP 400 requiring a newer Codex client.
  - Startup cannot find `codex-code-mode-host` in the workspace binary directory.
- Interpretation: Provider selection succeeds, but the first actual inference fails at OpenAI's client-version gate before a model response.
- Time: 2026-08-25 07:40

## Evidence E-020: OpenAI compatibility surfaces derive from the Whale package version
- Related hypotheses:
  - H-008
- Direction: supports
- Type: code-and-provenance
- Source: `login/src/auth/default_client.rs`, `models-manager/src/lib.rs`, `models-manager/models.json`, and `third_party/codex-cli/UPSTREAM.md`
- Matched signal:
  - `User-Agent` and models `client_version` both use `CARGO_PKG_VERSION`, currently `0.0.6`.
  - The selected model declares minimum client version `0.144.0`.
  - The local Codex substrate is `rust-v0.149.0`.
- Interpretation: Whale's product version and its OpenAI Codex protocol compatibility are distinct version domains and must not share one value.
- Time: 2026-08-25 08:08

## Evidence E-021: Code Mode binary exists in Cargo but not in the install manifest
- Related hypotheses:
  - H-009
- Direction: supports
- Type: packaging-inspection
- Source: Cargo workspace, `code-mode-host/Cargo.toml`, and `scripts/install-whale-local.sh`
- Matched signal:
  - `codex-code-mode-host` is a workspace binary and upstream qualification builds it.
  - The local installer helper array does not include it and silently skips absent helpers.
- Interpretation: The warning is a deterministic packaging defect rather than a runtime provider failure.
- Time: 2026-08-25 08:08

## Hypothesis H-010: Embedded app-server initialization reintroduces `0.0.6` in the request identity suffix
- Status: refuted
- Parent: P-004
- Claim: TUI and exec initialize their embedded app-server client with Whale's product version, causing `initialize_processor` to append `client-name; 0.0.6` to the otherwise corrected Codex User-Agent; OpenAI's inference version gate still observes that stale version.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-008
- Falsifiable predictions:
  - After the first repair, initialize reports a base version of `0.149.0` but retains a `0.0.6` client suffix, model discovery succeeds, and inference still receives the obsolete-client 400.
- Diagnostic evidence plan:
  - Capture initialize identity and a single real turn after the first repair; trace construction of the suffix back to TUI/exec `client_version`.
- Evidence gate: satisfied
- Related evidence:
  - E-022
- Conclusion: Refuted as the remaining inference root cause. TUI/exec did expose the wrong product version, but a second E2E with both base and suffix set to `0.149.0` received the same 400.
- Repair design readiness: not applicable

## Evidence E-022: First post-repair E2E isolates the remaining client suffix
- Related hypotheses:
  - H-008
  - H-010
- Direction: supports
- Type: live-fix-validation
- Source: installed binary at commit `19bc4ebc0`, ledger record `WAR-20260825-075400-OPENAI-SUBSCRIPTION-HI-E2E`
- Matched signal:
  - Initialize returned `Codex Desktop/0.149.0 ... (whale-e2e; 0.0.6)`.
  - Authenticated models discovery returned the full OpenAI Subscription catalog and cached it under client version `0.149.0`.
  - Route switching to `openai/chatgpt + gpt-5.6-sol low` succeeded.
  - The only `hi` request still received the obsolete-client HTTP 400 and was not retried.
  - No missing Code Mode host warning occurred.
- Interpretation: Base User-Agent, models discovery, route switching, and host packaging are repaired. This run suggested the suffix as the next differentiator, but E-023 later refuted it as the complete cause.
- Time: 2026-08-25 07:56

## Hypothesis H-011: OpenAI's live inference gate requires the newly published stable patch `0.149.1`
- Status: refuted
- Parent: P-004
- Claim: OpenAI began requiring the current stable Codex patch `0.149.1` for `gpt-5.6-sol`; advertising the older `0.149.0` substrate version is rejected even though model discovery succeeds.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-008
- Falsifiable predictions:
  - Current official distribution metadata identifies `0.149.1` as latest, an aligned `0.149.0` client still fails, and the patch diff contains no required Responses wire change that Whale lacks.
- Diagnostic evidence plan:
  - Compare official npm/version metadata and the official `rust-v0.149.0...rust-v0.149.1` source diff; run one final Whale E2E advertising `0.149.1`.
- Evidence gate: satisfied
- Related evidence:
  - E-023
  - E-024
- Conclusion: Refuted as the complete remaining cause. A live request advertising `0.149.1` in both User-Agent version positions still received the same 400.
- Repair design readiness: not applicable

## Evidence E-023: Aligning both User-Agent version positions to `0.149.0` still fails
- Related hypotheses:
  - H-010
  - H-011
- Direction: refutes H-010; supports H-011
- Type: live-differential
- Source: installed binary at commit `8a869dc6f`, ledger record `WAR-20260825-080230-OPENAI-SUBSCRIPTION-HI-E2E-R2`
- Matched signal:
  - Initialize returned `Codex Desktop/0.149.0 ... (codex-tui; 0.149.0)`.
  - Route switching to `openai/chatgpt + gpt-5.6-sol low` succeeded.
  - The single `hi` request received the same obsolete-client HTTP 400 and was not retried.
- Interpretation: The stale `0.0.6` suffix is not sufficient to explain the gate; `0.149.0` itself is below the live accepted stable patch.
- Time: 2026-08-25 08:03

## Evidence E-024: Official stable Codex advanced to `0.149.1` without Responses wire changes
- Related hypotheses:
  - H-011
- Direction: supports
- Type: external-authority-and-source-diff
- Source: official `@openai/codex` npm metadata and GitHub comparison `rust-v0.149.0...rust-v0.149.1`, fetched 2026-08-25
- Matched signal:
  - npm reports `0.149.1` as latest, published about three hours earlier.
  - An official issue reports successful use of `gpt-5.6-sol` on Codex `0.149.1`.
  - The five-commit patch diff changes exec thread classification, image compaction, and memory metadata, but no Responses request endpoint or client-version transport.
- Interpretation: Advertising `0.149.1` is a patch-level compatibility declaration for the existing substrate, not an unsupported protocol claim or a reason to import unrelated upstream changes.
- Time: 2026-08-25 08:07

## Hypothesis H-012: Whale inherits Codex Desktop's private originator and is classified as the wrong client product
- Status: refuted
- Parent: P-004
- Claim: When Whale is launched from a Codex Desktop terminal, the private `CODEX_INTERNAL_ORIGINATOR_OVERRIDE=Codex Desktop` environment variable wins over Whale's app-server client identity, so OpenAI sees a Desktop client rather than the compatible CLI substrate.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-008
- Falsifiable predictions:
  - The failed `0.149.1` process environment contains the Desktop override and its User-Agent begins with `Codex Desktop/`.
  - Official Codex `0.149.1` source produces `codex_cli_rs/0.149.1` without that private parent override.
  - Removing only the inherited Desktop value before runtime initialization restores Whale's own `codex-tui`/`codex_exec` originator selection without changing custom explicit overrides.
- Diagnostic evidence plan:
  - Inspect the failed process environment, compare the official `rust-v0.149.1` default-client source, and exercise app-server initialization under the inherited variable without making a provider request.
- Evidence gate: satisfied
- Related evidence:
  - E-025
  - E-026
  - E-027
- Conclusion: Refuted as the inference root cause. Official standalone Codex `0.149.1` preserves the same inherited `Codex Desktop` originator and completes the target model successfully, while Whale previously failed with that originator. Clearing the parent originator is therefore not justified by this failure and diverges from native behavior.
- Repair design readiness: not applicable

## Evidence E-025: Current-version E2E exposes the inherited Desktop identity
- Related hypotheses:
  - H-011
  - H-012
- Direction: refutes H-011; supports H-012
- Type: live-differential-and-source-comparison
- Source: installed binary at commit `a0da5ade2`, ledger record `WAR-20260825-080909-OPENAI-SUBSCRIPTION-HI-E2E-R3`, and official `rust-v0.149.1` `login/src/auth/default_client.rs`
- Matched signal:
  - Initialize returned `Codex Desktop/0.149.1 ... (codex-tui; 0.149.1)` and the process environment contained `CODEX_INTERNAL_ORIGINATOR_OVERRIDE=Codex Desktop`.
  - Route switching to `openai/chatgpt + gpt-5.6-sol low` succeeded, but the only `hi` request received the same obsolete-client HTTP 400.
  - Official standalone Codex `0.149.1` constructs its default identity as `codex_cli_rs/0.149.1`; the private environment override takes precedence over app-server `clientInfo` in the shared upstream helper.
  - No missing Code Mode host warning occurred.
- Interpretation: The live request disproves version-only repair and identifies a parent-process environment leak that changes the client product classification. The smallest safe repair is to clear exactly the `Codex Desktop` inherited value at Whale process entry, before threads start; arbitrary custom overrides remain intact.
- Time: 2026-08-25 08:12

## Evidence E-026: Repaired Whale restores its own app-server identity without a provider request
- Related hypotheses:
  - H-012
- Direction: supports
- Type: local-fix-validation
- Source: rebuilt `target/debug/whale` app-server initialized under `CODEX_INTERNAL_ORIGINATOR_OVERRIDE=Codex Desktop`
- Matched signal:
  - Before the repair, initialize returned `Codex Desktop/0.149.1 ... (codex-tui; 0.149.1)` under the inherited environment.
  - After the repair, the same command and initialize payload returned `codex-tui/0.149.1 ... (codex-tui; 0.149.1)`.
  - The targeted guard test confirms that only the exact parent value `Codex Desktop` is selected for cleanup; `codex_cli_rs` and arbitrary custom values are preserved.
- Interpretation: The repair removes the identified identity leak without broad environment sanitization. A final live OpenAI turn is still required to close P-004.
- Time: 2026-08-25 08:15

## Evidence E-027: First approved final validation is invalid because the driver omitted `route`
- Related hypotheses:
  - H-012
- Direction: inconclusive
- Type: invalid-harness
- Source: installed binary at commit `572bd051e`, ledger record `WAR-20260825-090337-OPENAI-SUBSCRIPTION-HI-E2E-R4`, and `ThreadSettingsUpdateParams` protocol schema
- Matched signal:
  - Initialize returned the repaired identity `codex-tui/0.149.1 ... (codex-tui; 0.149.1)`.
  - The driver sent obsolete unknown fields `modelProvider` and `authMode`; the protocol requires `route: { modelProviderId, accessMethod }`.
  - Unknown fields were ignored, the thread remained on DeepSeek, and DeepSeek rejected `gpt-5.6-sol` against its model allowlist.
  - The one request was stopped without retry and produced no token usage or Code Mode host warning.
- Interpretation: This run validates installed identity and packaging only. It neither supports nor refutes successful OpenAI inference or provider switching because no OpenAI route was committed. The next validation must use the schema-authoritative `route` object and observe `thread/settings/updated` before starting the turn.
- Time: 2026-08-25 09:07

## Hypothesis H-013: `gpt-5.6-sol` subscription access is gated on the current `0.150.0` prerelease line
- Status: refuted
- Parent: P-004
- Claim: OpenAI's ChatGPT subscription inference gate has advanced beyond stable Codex `0.149.1` to the currently published `0.150.0-alpha.8` client line, while the public model catalog is visible to older clients.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-008
- Falsifiable predictions:
  - A schema-correct OpenAI ChatGPT route with exact stable `0.149.1` identity still receives the newer-client 400.
  - Official distribution metadata exposes a newer `0.150.0` prerelease for every supported platform.
  - Comparing official `0.149.1` and `0.150.0-alpha.8` shows no new mandatory root Responses authentication or request-header contract absent from Whale.
- Diagnostic evidence plan:
  - Validate the committed route notification before one live request, inspect official current release tags, and compare the two official source archives on login/default-client and core request construction.
- Evidence gate: satisfied
- Related evidence:
  - E-028
  - E-029
- Conclusion: Refuted. The model catalog returned for `0.149.1` is identical to the prerelease catalog, and official Codex `0.149.1` completes `gpt-5.6-sol` with the current subscription. Whale's failure is caused by a remaining `version: 0.0.6` inference header, not a prerelease requirement.
- Repair design readiness: not applicable

## Evidence E-028: Schema-correct OpenAI E2E rejects stable `0.149.1`
- Related hypotheses:
  - H-011
  - H-012
  - H-013
- Direction: refutes version/originator-only sufficiency; supports H-013
- Type: live-fix-validation
- Source: installed binary at code commit `572bd051e`, ledger record `WAR-20260825-092520-OPENAI-SUBSCRIPTION-HI-E2E-R5`
- Matched signal:
  - Initialize returned `codex-tui/0.149.1 ... (codex-tui; 0.149.1)`.
  - `thread/settings/updated` authoritatively committed `openai/chatgpt`, `modelProvider=openai`, `gpt-5.6-sol`, and `low` before inference.
  - The sole `hi` request reached the OpenAI subscription route and received HTTP 400 requiring a newer Codex client.
  - No retry, token usage, or missing Code Mode host warning occurred.
- Interpretation: Provider switching is working and the inherited Desktop identity is gone. The remaining rejection is specifically the live minimum-version gate above stable `0.149.1`.
- Time: 2026-08-25 09:27

## Evidence E-029: Official `0.150.0-alpha.8` advances identity without a new root Responses contract
- Related hypotheses:
  - H-013
- Direction: supports
- Type: official-release-and-source-diff
- Source: current official `@openai/codex` distribution tags and official source archives `rust-v0.149.1` / `rust-v0.150.0-alpha.8`, inspected 2026-08-25
- Matched signal:
  - Stable remains `0.149.1`; the current cross-platform alpha tag is `0.150.0-alpha.8`.
  - Default-client identity construction is unchanged apart from the package version.
  - Core changes do not add a mandatory root Responses authentication header or alter the root HTTP request endpoint; relevant changes concern parent-thread metadata, realtime sideband helpers, and internal item handling.
- Interpretation: Advancing Whale's single OpenAI compatibility identity to the official alpha is smaller and more accurate than importing unrelated alpha changes or changing Whale's `0.0.6` product version.
- Time: 2026-08-25 09:31

## Hypothesis H-014: The OpenAI provider still sends Whale `0.0.6` in the inference `version` header
- Status: confirmed
- Parent: P-004
- Claim: The compatibility repair covered User-Agent and models `client_version` but missed `ModelProviderInfo::create_openai_provider`, which independently builds a `version` HTTP header from Whale's `CARGO_PKG_VERSION`; inference therefore still identifies as Codex `0.0.6` and receives the generic upgrade error.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-008
- Falsifiable predictions:
  - Old client identities continue to authenticate while `/models` filters their compatible catalog.
  - Official Codex `0.149.1` succeeds with the same subscription and target model.
  - Whale source shows a remaining inference header derived from package `0.0.6`, while official `0.149.1` derives the same header from `0.149.1`.
- Diagnostic evidence plan:
  - Differentially probe `/models` with the same stored ChatGPT credential and several client versions without inference.
  - Execute one official `0.149.1` target-model turn and compare the provider header constructor against Whale.
- Evidence gate: satisfied
- Related evidence:
  - E-030
  - E-031
  - E-032
- Conclusion: Confirmed. Authentication, catalog compatibility, and native `0.149.1` inference all work. Whale's OpenAI provider alone retains `version: 0.0.6`, exactly isolating the stale inference identity omitted by the earlier repair.
- Repair design readiness: ready

## Evidence E-030: Old clients authenticate and receive version-filtered model catalogs
- Related hypotheses:
  - H-013
  - H-014
- Direction: refutes H-013; supports H-014
- Type: authenticated-non-inference-differential
- Source: direct ChatGPT `/models` probes using the same stored subscription credential, captured under `/tmp/whale-model-catalog-probe.2AgIpn`
- Matched signal:
  - `client_version=0.0.6` returns HTTP 200 with zero compatible models rather than an authentication failure.
  - `0.149.0`, `0.149.1`, and `0.150.0-alpha.9` each return the same nine-model catalog, the same ETag, and `gpt-5.6-sol`.
- Interpretation: Login validity and model compatibility are separate contracts. The prerelease catalog provides no capability unavailable to stable `0.149.1`.
- Time: 2026-08-26 04:45

## Evidence E-031: Official Codex `0.149.1` completes the target subscription turn
- Related hypotheses:
  - H-012
  - H-013
  - H-014
- Direction: refutes H-012 and H-013; supports H-014
- Type: native-runtime-differential
- Source: official `@openai/codex@0.149.1` executed with the current ChatGPT subscription and `gpt-5.6-sol`
- Matched signal:
  - The official client completed `hi` successfully and reported normal token usage.
  - Its inherited request identity remained `originator: Codex Desktop` and `Codex Desktop/0.149.1`.
- Interpretation: Neither stable `0.149.1` nor the inherited Desktop originator prevents subscription inference. Whale differs elsewhere in its provider request construction.
- Time: 2026-08-26 04:57

## Evidence E-032: Whale's OpenAI inference header remains bound to its product package version
- Related hypotheses:
  - H-008
  - H-014
- Direction: supports
- Type: source-and-provenance
- Source: Whale and official `rust-v0.149.1` `model-provider-info/src/lib.rs`, plus Whale `login/src/auth/default_client.rs` and `models-manager/src/lib.rs`
- Matched signal:
  - `create_openai_provider` inserts `version = env!("CARGO_PKG_VERSION")`; in Whale that value is `0.0.6`, while in official Codex it is `0.149.1`.
  - Whale's User-Agent and models query now use a separate compatibility constant, so this inference-only header escaped the earlier fix.
  - The header exists unchanged in official source, establishing that it is part of the native OpenAI request contract rather than Whale-specific metadata.
- Interpretation: The stale `version` header is the single remaining version-domain leak and directly explains the split between successful catalog discovery and rejected inference.
- Time: 2026-08-26 05:04

## Evidence E-033: Repair unifies all OpenAI compatibility surfaces on stable `0.149.1`
- Related hypotheses:
  - H-014
- Direction: supports
- Type: local-fix-validation
- Source: focused unit tests, cache regression gate, and Whale binary build on 2026-08-26
- Matched signal:
  - `OPENAI_CODEX_COMPATIBILITY_VERSION` now belongs to model-provider metadata and is fixed at stable `0.149.1`.
  - The OpenAI provider `version` header, default User-Agent, models `client_version`, TUI initialization, and exec initialization all consume that single constant.
  - The non-native startup hook that removed inherited `Codex Desktop` originator was deleted.
  - Provider-header, default-client, and models-version focused tests pass; cache regression gate fingerprint `bacc832c563f53d1e90e7651d4bd6a663c0416688c4b0f359f520c05761c5799` passes; `cargo build -p codex-cli --bin whale` succeeds.
  - The broader `codex-exec` integration-test binary remains blocked by the pre-existing unrelated `AuthDotJson` fixture missing `deepseek_api_key` in `exec/tests/suite/apply_patch.rs`.
- Interpretation: The repair compiles and prevents the three version surfaces from drifting. The original live subscription symptom still requires the approved single-sample installed-binary validation before P-004 can be marked fixed.
- Time: 2026-08-26 05:13

## Evidence E-034: Installed Whale completes the OpenAI subscription turn with one request
- Related hypotheses:
  - H-014
- Direction: supports
- Type: live-fix-validation
- Source: installed binary at code commit `9a8ff88f9`, ledger record `WAR-20260826-214015-OPENAI-SUBSCRIPTION-HI-E2E-R6`, provider trace `/tmp/whale-r6-provider-wire-20260826-214015.jsonl`
- Matched signal:
  - Initialize reports native inherited identity `Codex Desktop/0.149.1 ... (codex-tui; 0.149.1)` from the installed Whale `0.0.6` binary.
  - `thread/settings/updated` authoritatively commits `openai/chatgpt`, `modelProvider=openai`, `gpt-5.6-sol`, and `low` before inference.
  - The `hi` turn completes without error and returns `Hi! What can I help you with today?`.
  - Provider hard-limit state records exactly one request; wire trace records one logical websocket request and terminal `response_completed` with 10,319 input and 14 output tokens.
- Interpretation: The original obsolete-client failure is absent under the same installed-client reproduction. Stable `0.149.1` is sufficient once the inference `version` header is aligned, proving the repair closes P-004 without a prerelease identity.
- Time: 2026-08-26 21:44

# Problem P-005: Cross-provider `/model` selection dispatches the new model through the previous provider
- Status: fixed
- Symptom: After selecting `gpt-5.6-sol` from the grouped `/model` picker and sending `hi`, Whale rejects the turn with the DeepSeek model allowlist error: the supported models are `deepseek-v4-pro`, `deepseek-v4-flash`, and `deepseek-v4-flash-vision-exp`, but the request model is `gpt-5.6-sol`.
- Expected behavior: Selecting an OpenAI model from `/model` atomically stages both its OpenAI provider route and model for the next turn.
- Actual behavior: The selected OpenAI model is visible, but the next turn is validated or dispatched as DeepSeek.
- Impact: The product requirement that `/model` directly switches across provider groups is unusable when the selected model belongs to a different provider.
- Known facts:
  - E-034 proves the installed app-server can complete the same OpenAI subscription turn when `route` and `model` are explicitly updated together.
  - No Whale TUI process remained alive when the report was inspected; the installed binary hash matched the previously validated build.
- Fix criteria:
  - A focused TUI test proves cross-provider model selection emits and commits the model's matching route for the next turn.
  - A local non-billable integration path proves the turn configuration cannot pair `deepseek` with `gpt-5.6-sol`.
  - One user-authorized installed-binary live turn succeeds after selecting the OpenAI model through `/model`.
  - One installed-binary black-box run proves `/provider` followed by same-route `/model` selection does not contaminate a restarted session's provider/model defaults.
- Active hypotheses:
  - H-015
  - H-016
  - H-017
- Current conclusion: H-017 is fixed. The exact installed-TUI sequence `/provider` OpenAI Subscription → same-route `/model` selection → exit → restart preserves the DeepSeek startup default, and a subsequent OpenAI Subscription `hi` turn completes through ChatGPT without the DeepSeek model allowlist error.
- Resolution basis: H-017; E-038, E-039, and E-040.

## Hypothesis H-015: The `/model` picker drops the selected model's provider route
- Status: refuted
- Parent: P-005
- Claim: The grouped catalog retains a `ProviderRoute`, but the model-row selection callback emits only model/effort update events; the active thread therefore keeps its previous DeepSeek route and combines it with `gpt-5.6-sol` on the next turn.
- Layer: root-cause
- Factor relation: causes
- Falsifiable predictions:
  - The cross-provider model-row callback does not emit `SelectProviderModel` or another route-bearing update.
  - The next-turn settings path accepts the model-only event while retaining the existing thread route.
  - A focused event test selecting an OpenAI model while the active route is DeepSeek observes model events without an OpenAI route event.
- Diagnostic evidence plan:
  - Trace the callback from grouped `/model` rows through `AppEvent` handling to `thread/settings/update` and record the exact fields carried.
  - Run or add a diagnostic-only focused test that inspects emitted events without contacting a provider.
- Evidence gate: pending
- Related evidence:
  - E-036
- Conclusion: Refuted. The grouped model row carries its route into `OpenReasoningPopup`, and every normal reasoning-selection branch with a route emits `SelectProviderModel`; the route loss occurs later across the session/global persistence boundary.
- Repair design readiness: not applicable

## Hypothesis H-016: The reported turn used stale executable or persisted session state rather than the current picker path
- Status: refuted
- Parent: P-005
- Claim: A previously running binary or stale persisted session state prevented the current atomic route-switch implementation from taking effect.
- Layer: environment-alternative
- Factor relation: alternative_to H-015
- Falsifiable predictions:
  - A live Whale process resolves to a pre-repair executable hash, or recent runtime records show a route already changed to OpenAI before the rejected turn.
  - If neither is present and the source callback drops route, this hypothesis is downgraded or refuted.
- Diagnostic evidence plan:
  - Compare all live Whale executable targets and hashes with the installed validated binary.
  - Query recent redacted workspace logs/session records for the rejected turn's model and provider route.
- Evidence gate: satisfied
- Related evidence:
  - E-035
- Conclusion: Refuted as an executable-staleness cause. No old process remained, the installed hash matched the validated build, and the latest process created the mismatched session directly from current persisted defaults.
- Repair design readiness: not applicable

## Hypothesis H-017: A session-only provider route can persist its model into the global default
- Status: confirmed
- Parent: P-005
- Claim: A cross-provider selection correctly updates the active thread route without changing the global provider, but once that route becomes current, a later model or effort selection on the same route satisfies `current_provider_route() == route` and emits `PersistModelSelection`; this writes `model` and `model_reasoning_effort` to global config without the session-only provider route, so the next new session starts with the default DeepSeek provider and the persisted OpenAI model.
- Layer: root-cause
- Factor relation: supersedes H-015
- Falsifiable predictions:
  - The current picker preserves route through `SelectProviderModel`, refuting H-015's original callback-loss mechanism.
  - `select_provider_model` permits global model persistence based on the mutable current session route rather than the immutable global/default provider.
  - A new session built from the resulting config starts as `deepseek/api_key + gpt-5.6-sol` before any picker interaction.
- Diagnostic evidence plan:
  - Correlate the latest runtime startup and turn records with the redacted config fields and the persistence predicate in `select_provider_model`.
  - Exercise the persistence decision with a focused zero-network test that distinguishes the configured default provider from the active session route.
- Evidence gate: satisfied
- Related evidence:
  - E-035
  - E-036
  - E-037
- Conclusion: Confirmed. A routed selection is correctly committed to the active thread, but a later same-route selection emits the native global model persistence event. Because PD7 forbids persisting the session-only route globally, the resulting `config.toml` retains DeepSeek as the startup provider while overriding its model with `gpt-5.6-sol`.
- Repair design readiness: ready

## Evidence E-035: The latest process starts in the invalid provider/model pair before user input
- Related hypotheses:
  - H-016
  - H-017
- Direction: refutes H-016; supports H-017
- Type: runtime-state-and-log-correlation
- Source: workspace SQLite logs IDs 1605, 1652, and 1684 from 2026-08-27; installed binary/process inspection; redacted workspace `config.toml`
- Matched signal:
  - Session initialization records `model=gpt-5.6-sol` with `route=deepseek/api_key` before the turn.
  - The only subsequent user action is `hi`; its turn settings carry `route: None` and inherit the invalid session pair.
  - The turn reaches DeepSeek and returns its exact supported-model allowlist error.
  - No stale Whale process remains; the installed executable hash is `4cdb6cb09a61dfd3f5422a5cf1840a190b3727269a70b3e277f786b004e07015`.
  - The redacted config contains `model = "gpt-5.6-sol"` and `model_reasoning_effort = "low"`, but no global provider override; Whale therefore retains its DeepSeek default.
- Interpretation: This failure is deterministic startup state produced by partial global persistence, not a network, credential, OpenAI compatibility, or old-process problem.
- Time: 2026-08-27 05:27

## Evidence E-036: Routed selection preserves thread route but conditionally persists only model and effort globally
- Related hypotheses:
  - H-015
  - H-017
- Direction: refutes H-015; supports H-017
- Type: source-path-causal-trace
- Source: `tui/src/chatwidget/model_popups.rs`, `tui/src/app/provider_login.rs`, `tui/src/app/thread_settings.rs`, and `tui/src/app/config_persistence.rs`
- Matched signal:
  - The grouped model picker carries `ProviderRoute` through reasoning selection and emits `SelectProviderModel`.
  - `sync_active_thread_provider_model_setting` atomically sends route, model, effort, and matching collaboration mode to the active thread.
  - `select_provider_model` additionally emits `PersistModelSelection` whenever the selected route is already the current session route.
  - That persistence event writes only global `model` and `model_reasoning_effort`; it does not and, under PD7, must not persist the session provider route.
- Interpretation: The first route transition is session-correct, but any later selection on that route can leak a route-scoped model into provider-independent global defaults.
- Time: 2026-08-27 06:10

## Evidence E-037: Product and upstream config contracts require avoiding partial global persistence
- Related hypotheses:
  - H-017
- Direction: supports
- Type: product-authority-and-official-config-contract
- Source: PRD PD7/PD10/PD14/PD16; official OpenAI Codex configuration reference; official DeepSeek Responses API reference, inspected 2026-08-27
- Matched signal:
  - PD7 requires provider switches to persist within the current session and forbids modifying new-session global defaults.
  - PD10 scopes the last successful model to each access method in the current session.
  - Official Codex config defines `model` and `model_provider` as distinct global keys, so persisting only `model` is not an atomic provider selection.
  - DeepSeek validates the request model against its own supported identifiers, explaining the deterministic rejection without requiring any fallback.
- Interpretation: The minimal contract-preserving repair is to keep routed model selections entirely in thread/session settings and stop emitting native global model persistence for them.
- Time: 2026-08-27 06:15

## Evidence E-038: Routed selections no longer mutate new-session model defaults
- Related hypotheses:
  - H-017
- Direction: supports repair
- Type: local-fix-validation
- Source: focused TUI app-server tests, existing provider picker/settings tests, cache regression gate, and debug Whale build on 2026-08-27
- Matched signal:
  - `select_provider_model` now performs only the existing atomic thread settings update; the conditional `PersistModelSelection` emission and its mutable-current-route predicate are removed.
  - `routed_model_selection_does_not_persist_new_session_defaults` starts an embedded thread, performs a routed model selection, and proves no global persistence event is emitted.
  - Existing grouped picker route, provider collaboration-model replacement, and route-only settings-update tests pass.
  - Formatting/diff checks pass; cache regression index gate passes with fingerprint `bacc832c563f53d1e90e7651d4bd6a663c0416688c4b0f359f520c05761c5799`.
  - `cargo build -p codex-cli --bin whale` succeeds. Existing unrelated vendor warnings remain unchanged.
- Interpretation: The repair closes the confirmed partial-persistence mechanism without adding a new config layer, fallback, or provider special case. The final gate is one installed TUI cross-provider live turn.
- Time: 2026-08-27 06:35

## Evidence E-039: Installed TUI cross-provider switch completes through OpenAI Subscription
- Related hypotheses:
  - H-017
- Direction: confirms repair
- Type: installed-live-tui-validation
- Source: run ledger `WAR-20260827-054109-OPENAI-SUBSCRIPTION-TUI-SWITCH-R7`; workspace SQLite logs IDs 1788, 1828, 1829, 1865, and 1867; rollout `01a04006-1dff-75e0-884a-ab0eae6c4b21`
- Matched signal:
  - The newly installed binary starts a fresh TUI with the unpolluted `deepseek-v4-flash` default.
  - Selecting the first model under the OpenAI Subscription group commits one `ThreadSettingsOverrides` value containing `openai/chatgpt`, `gpt-5.6-sol`, and `low` before inference.
  - The request trace records `provider=OpenAI`, `auth_mode=Chatgpt`, and `model=gpt-5.6-sol`; no DeepSeek model allowlist error occurs.
  - Exactly one provider request completes and returns `Hi! What would you like to work on in WhaleCode?`.
  - The completed rollout reports 14,415 input tokens, 0 cached input tokens, 17 output tokens, 6,163 ms wall time, and no retry.
- Interpretation: The immediate single-session switch path works, but this evidence alone does not prove the same-route persistence and restart boundary; E-040 supplies that missing equivalent cross-session validation.
- Time: 2026-08-27 05:44

## Evidence E-040: Equivalent cross-session installed TUI reproduction passes
- Related hypotheses:
  - H-017
- Direction: confirms repair and closes P-005
- Type: equivalent-cross-session-black-box-validation
- Source: run ledger `WAR-20260827-060206-OPENAI-CROSS-SESSION-EQUIVALENT-E2E-R8`; workspace SQLite logs IDs 1970, 1990, 2047, 2097, 2117, 2160, 2161, and 2189; Phase B rollout `01a0401a-a88f-7230-a49a-02372ad8ae3c`
- Matched signal:
  - Phase A starts at `deepseek-v4-flash`, switches through `/provider` to `openai/chatgpt`, then uses `/model` to select `gpt-5.6-sol low` while OpenAI is already the current session route—the exact old persistence trigger.
  - Phase A exits with zero inference submissions; both before and after exit, global `config.toml` has no `model`, `model_reasoning_effort`, or `model_provider` fields.
  - Phase B starts a new process. Startup log ID 2047 authoritatively records `model=deepseek-v4-flash` and `route=deepseek/api_key`, proving no cross-session provider/model mismatch.
  - Phase B repeats `/provider` and same-route `/model`; logs 2097 and 2117 atomically record `openai/chatgpt`, `gpt-5.6-sol`, and `low` before inference.
  - The sole provider request records `provider=OpenAI` and `auth_mode=Chatgpt`, completes without retry, and returns `你好！想一起做点什么？`; the DeepSeek model allowlist error is absent.
  - The completed rollout records 14,415 input tokens, including 3,840 cached and 10,575 uncached, 12 output tokens, and 5,488 ms wall time.
- Interpretation: The repair survives the exact history-dependent boundary that the earlier app-server smoke and single-session TUI run skipped. Session-only route/model choices no longer contaminate new-session defaults, and the subsequent OpenAI turn uses the selected provider.
- Time: 2026-08-27 06:06
