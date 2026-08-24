# Problem P-001: Multi-provider credentials are persisted but misreported or not consumed
- Status: open
- Created: 2026-08-25 01:30
- Updated: 2026-08-25 01:43
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
- Current conclusion: H-002 is repaired in focused and subsystem tests. Runtime fix validation against the installed workspace remains pending.
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - not satisfied
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
