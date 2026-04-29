# Problem P-001: First turn starts late after user input
- Status: fixed
- Created: 2026-04-30 01:52
- Updated: 2026-04-30 02:48
- Objective: Determine and fix why the first natural-language user input can appear idle before Whale starts work.
- Symptoms:
  - User sent "检查项目架构设计" once and saw no visible work until sending another message.
- Expected behavior:
  - After the first input is accepted, the TUI should promptly show that the agent has started or is preparing the turn.
- Actual behavior:
  - The first input entered core, but the visible `task_started` event appeared about 10.6 seconds later.
- Impact:
  - Whale CLI/TUI first-turn responsiveness and user trust in whether a prompt was accepted.
- Reproduction:
  - Start Whale in `D:\side-deepseek`, set model to `deepseek-v4-pro high`, send "检查项目架构设计".
- Environment:
  - Windows PowerShell, Whale session `C:\Users\77585\.whale\sessions\2026\04\30\rollout-2026-04-30T01-36-17-019dda50-2186-7e80-8a2e-5be3fa3d5016.jsonl`, repo `D:\whalecode-alpha`, commit `4a964d6`.
- Known facts:
  - First submission entered `submission_dispatch` at 17:37:01.950.
  - `session_task.turn` and rollout `task_started` appeared at about 17:37:12.576-17:37:12.616.
  - The gap occurs before `RegularTask` emits `TurnStarted`.
  - `whale debug prompt-input` is fast when `web_search` is disabled or when all provider API keys are present in env.
  - The slow path appears when provider env vars are missing and manifest resolution falls back to local secret-store reads.
  - The patched installed binary reduces the same prompt-input path from 25.370s to 2.078s by removing per-provider secret decrypts.
  - User feedback confirmed 2s is still unacceptable for the first prompt path.
  - Removing secret-store reads from manifest construction brought repeated prompt-input runs to about 143-147ms.
- Ruled out:
  - none
- Fix criteria:
  - Add evidence that identifies the slow pre-`task_started` step.
  - Apply a targeted fix or diagnostic infrastructure tied to that evidence.
  - Validate that the first accepted input surfaces visible progress promptly, or that logs can prove the remaining blocker.
- Current conclusion: Fixed. Provider-specific manifest construction no longer reads the secret store; it exposes configured providers and env-discovered hints, while actual web tool execution remains responsible for auth validation.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-001, H-002, H-003, E-008
- Close reason:
  - not closed

## Hypothesis H-001: Pre-turn context construction delays task start
- Status: confirmed
- Parent: P-001
- Claim: The first input is blocked between `submission_dispatch` and `RegularTask::TurnStarted` by an awaited operation in `Session::new_turn_with_sub_id` or its callees.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - Existing logs show `submission_dispatch` for the first user input long before `session_task.turn` starts, and `RegularTask` is where visible `TurnStarted` is emitted.
- Falsifiable predictions:
  - If true: fine-grained timing around `new_turn_with_sub_id`, `new_turn_from_configuration`, and `make_turn_context` will show one or more slow awaited stages before `spawn_task`.
  - If false: timing will show those stages are fast and the delay lies after `spawn_task` or in TUI rendering.
- Verification plan:
  - Inspect code boundaries and add structured timing logs around each pre-`task_started` stage.
- Related evidence:
  - E-001
  - E-002
  - E-004
- Conclusion: confirmed by the timing contrast between web-search-enabled and web-search-disabled prompt-input runs.
- Next step: fix the web manifest secret lookup path and validate startup timing.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Web tool manifest provider path is the slow new stage
- Status: confirmed
- Parent: P-001
- Claim: Whale's routed `web_search`/`web_fetch` provider manifest resolution is on the blocking path and may add noticeable first-turn latency.
- Layer: sub-cause
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - The first visible log after the 10.6 second gap is `web tool manifest availability resolved`, and this provider-specific path is Whale-specific relative to the official Codex CLI path.
- Falsifiable predictions:
  - If true: timing logs inside manifest resolution will account for a large part of the gap.
  - If false: manifest timing will be small, and another pre-turn stage will account for the gap.
- Verification plan:
  - Add timing logs around manifest resolution and compare them with adjacent stages.
- Related evidence:
  - E-001
  - E-003
  - E-004
  - E-005
- Conclusion: confirmed by disabling `web_search` and by bypassing secret-store lookup with provider env vars.
- Next step: reduce manifest resolution to one secret-store read at most and cache by config/env/secrets metadata.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: First submission precedes visible task start by about 10.6 seconds
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: log
- Source: `C:\Users\77585\.whale\logs\whale-tui.log` and rollout jsonl
- Raw content:
  ```text
  17:37:01.950 submission_dispatch starts for sub id 019dda50-cdfe-7680-a9dc-77cbf5799fad
  17:37:12.574 codex_core::web_tools: web tool manifest availability resolved provider_count=1 providers=Tavily fetch_enabled=true
  17:37:12.576 session_task.turn starts
  17:37:12.616 rollout task_started
  ```
- Interpretation: The user input entered core, but no visible task event was emitted until after pre-turn startup completed.
- Time: 2026-04-30 01:52

## Evidence E-002: TurnStarted is emitted inside RegularTask after turn context is built
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/tasks/regular.rs`
- Raw content:
  ```text
  RegularTask::run emits EventMsg::TurnStarted near the beginning of the task body.
  The handler calls new_turn_with_sub_id before spawn_task creates RegularTask.
  ```
- Interpretation: Any slow await before `spawn_task` can produce a visible no-response gap even though the prompt was accepted.
- Time: 2026-04-30 01:52

## Evidence E-003: Web manifest resolution decrypts/checks configured provider secrets during turn context creation
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/web_tools/manifest.rs` and `third_party/codex-cli/codex-rs/secrets/src/local.rs`
- Raw content:
  ```text
  resolve_web_tool_manifest_availability checks search provider env/secrets for Github, Exa, Tavily, Brave, StackExchange, and Jina.
  Local secret reads load and decrypt the local secrets file.
  ```
- Interpretation: The provider-specific manifest path is a plausible latency contributor, but timing is needed because earlier session-init manifest logs were quick.
- Time: 2026-04-30 01:52

## Evidence E-004: Web-search and env-var timing isolates the slow path
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: experiment
- Source: PowerShell `Measure-Command` with installed `whale.exe`
- Raw content:
  ```text
  whale -C D:\side-deepseek debug prompt-input 'hi' *> $null
  TotalSeconds: 25.370

  whale -C D:\side-deepseek -c 'web_search="disabled"' debug prompt-input 'hi' *> $null
  TotalSeconds: 0.163

  whale -C D:\side-deepseek -c 'tools.web_search.enabled=false' debug prompt-input 'hi' *> $null
  TotalSeconds: 0.151

  With all six provider env vars set to non-empty placeholders:
  TotalSeconds: 0.163

  With only TAVILY_API_KEY set:
  TotalSeconds: 17.736
  ```
- Interpretation: The delay is caused by web-search manifest provider availability resolution falling back to secret-store checks for providers whose env vars are missing.
- Time: 2026-04-30 02:05

## Evidence E-005: Manifest checks six providers with one secret-store get per missing env
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/web_tools/manifest.rs`
- Raw content:
  ```text
  SEARCH_PROVIDER_MANIFEST_ORDER has six providers.
  resolve_search_providers_for_manifest calls has_secret(env_name) for every provider.
  has_configured_secret checks env first, then calls secrets.get(Global, name).
  LocalSecretsBackend::get loads and decrypts local.age on each call.
  ```
- Interpretation: The code shape explains the timing pattern: missing env vars multiply local secret-store decrypt/keyring work before `task_started`.
- Time: 2026-04-30 02:05

## Evidence E-006: Patched binary validation
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: fix-validation
- Source: Cargo tests, local build, installed `C:\Users\77585\.whale\bin\whale.exe`
- Raw content:
  ```text
  cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core web_tools::manifest
  3 passed

  cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core provider_specific_manifest
  2 passed

  cargo build --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-cli --bin whale
  Finished dev profile

  scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\debug\whale.exe -PersistUserPath -BackupLegacyCopies
  Installed Whale: C:\Users\77585\.whale\bin\whale.exe
  SHA256: AE2DD5EA33B2BBC3A4600D2CF7FAEA60F3AFBD4174AA720E5FB5292A29A65D61

  whale -C D:\side-deepseek debug prompt-input 'hi' *> $null
  TotalSeconds: 2.078
  ```
- Interpretation: The previously isolated pre-turn prompt-input path no longer spends 25s in repeated manifest secret-store lookups. TUI-visible behavior uses the same turn-context manifest construction path.
- Time: 2026-04-30 02:12

## Hypothesis H-003: One remaining secret-store list is still too slow for pre-turn startup
- Status: confirmed
- Parent: P-001
- Claim: Even a single local secret-store list during manifest construction leaves first-turn startup at about 2 seconds, which is too slow for user input acknowledgement.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-002
- Rationale:
  - The first patch changed multiple secret-store decrypts to one list call. The measured path is much faster but still above an acceptable interactive startup budget.
- Falsifiable predictions:
  - If true: removing secret-store reads from manifest construction should make the same prompt-input path close to the web-search-disabled baseline.
  - If false: removing secret-store reads will leave prompt-input near 2 seconds, meaning another stage dominates.
- Verification plan:
  - Change manifest resolution to derive provider tool exposure from config and env-only hints, leaving secret validation to actual tool execution.
- Related evidence:
  - E-006
  - E-007
  - E-008
- Conclusion: confirmed and fixed by making manifest resolution secret-store-free.
- Next step: stop.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-007: User rejects remaining 2s startup
- Related hypotheses:
  - H-003
- Direction: supports
- Type: user-feedback
- Source: conversation
- Raw content:
  ```text
  你不觉得2s 仍然长的可怕吗
  ```
- Interpretation: The previous validation is insufficient; the fix criteria should require removing secret-store I/O from the pre-turn path, not merely reducing repeated I/O.
- Time: 2026-04-30 02:20

## Evidence E-008: Secret-store-free manifest validation
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: fix-validation
- Source: Cargo tests, local build, installed `C:\Users\77585\.whale\bin\whale.exe`
- Raw content:
  ```text
  cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core web_tools::manifest
  3 passed

  cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-core provider_specific_manifest
  2 passed

  scripts\install-whale-local.ps1 -BinaryPath D:\BuildCache\whalecode\cargo-target\debug\whale.exe -PersistUserPath -BackupLegacyCopies
  SHA256: 2CD9DBD204440671A782E473BEA842760BE3BBFF590E3597A8FCC91F741331E9

  whale -C D:\side-deepseek debug prompt-input 'hi' *> $null
  First run after install: 0.456s
  Repeated runs: 147ms, 146ms, 144ms, 143ms, 145ms

  whale -C D:\side-deepseek -c 'web_search="disabled"' debug prompt-input 'hi' *> $null
  TotalSeconds: 0.156
  ```
- Interpretation: The remaining 2s secret-store list was removed from pre-turn manifest construction. The web-search-enabled path is now close to the disabled baseline after normal process warmup.
- Time: 2026-04-30 02:48
