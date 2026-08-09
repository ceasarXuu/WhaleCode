# Problem P-001: 0.146 candidate qualification reports four no-go failures
- Status: fixed
- Created: 2026-08-10 03:16
- Updated: 2026-08-10 04:05
- Objective: Determine which recorded failures belong to the qualification harness/environment versus the immutable upstream candidate, then make only evidence-backed runner corrections.
- Symptoms:
  - The five-command pristine qualification reports 1 passed and 4 failed.
- Expected behavior:
  - Qualification uses the candidate's supported entrypoints in an isolated environment and attributes every failure to the correct layer.
- Actual behavior:
  - CLI fails on `--locked`, core observes an ambient proxy, app-server cannot locate `codex-code-mode-host`, and TUI has a version snapshot mismatch.
- Impact:
  - U2 and production vendor cutover remain blocked because the 0.146 no-go may contain false harness/environment blockers.
- Reproduction:
  - Initial run: `python3 scripts/codex-upstream/qualify_candidate.py --run` at commit `b444ee8095917dbbbc222ecba019a42dcd8b9179`; corrected-run logs are under `docs/v0.0.5/codex-upstream-sync/evidence/rust-v0.146.0/`.
- Environment:
  - Linux 7.0.0-28-generic x86_64; Rust/Cargo 1.95.0; cargo-nextest 0.9.138; branch `whalecode-codex`; candidate `e363b08c9175ac1cbe5893615dd2cb9ddf95043b`.
- Known facts:
  - The candidate identity and production-vendor isolation were previously verified.
  - Existing logs directly show the four distinct failure signals captured in E-001 through E-004.
  - The tag-local source build and just test entrypoints do not require `--locked`; the just test recipe sets `NEXTEST_PROFILE=local` and `--no-fail-fast`.
  - Controlled focused probes independently confirmed three harness/environment causes and one immutable upstream snapshot failure.
- Ruled out:
  - Whale overlay conflict as the direct cause: all four failures were captured in a pristine `git archive` candidate.
- Fix criteria:
  - Each failure is reproduced with a predeclared diagnostic signal; confirmed harness/environment causes have regression tests and runner corrections; immutable upstream failures remain explicit; qualification is rerun; production vendor remains unchanged; model request count remains zero.
- Current conclusion: The CLI, original core proxy, and original app-server missing-helper failures are qualification-runner defects; the TUI release snapshot failure is immutable upstream evidence. The corrected full run also exposes broader local sandbox/fixture failures, so the candidate remains no-go and U2 must not start.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - E-005 through E-009 satisfy all four diagnostic gates.
  - E-010 proves the corrected runner, manifest, evidence paths, zero model requests, and unchanged production vendor.
- Close reason:
  - Root-cause attribution and runner correction are complete; candidate acceptance is intentionally rejected by U1's stop condition.

## Hypothesis H-001: CLI failure is caused by an invalid locked-entrypoint assumption
- Status: confirmed
- Parent: P-001
- Claim: `cargo check ... --locked` is not a valid pristine qualification entrypoint for the tagged 0.146 tree because its committed lockfile is not accepted by Cargo 1.95 without regeneration.
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - The existing failure occurs before compilation and explicitly says the lockfile needs an update.
- Falsifiable predictions:
  - If true: `cargo metadata --locked` or the current CLI command fails without changing the tree, while a candidate-declared non-locked/offline build entrypoint succeeds or advances past lock resolution.
  - If false: the candidate's own documented/CI entrypoint requires `--locked` and succeeds unchanged with the same toolchain and clean environment.
- Diagnostic evidence plan:
  - Prediction or clause under test: Compare the runner command with tag-local README/justfile/CI and execute the smallest metadata/check probes in a disposable tree.
  - Signal: documented command flags, exit codes, and `git status`/lockfile diff.
  - Capture method: inspect the immutable Git object, then run locked and supported alternatives in one temporary export.
  - Event name or marker:
    - `u1.cli.lock-entrypoint`
  - Correlation keys:
    - candidate commit `e363b08c`
  - Differentiates from:
    - dependency/network failure after successful lock resolution
  - Supports if:
    - locked resolution fails before build and the candidate's own entrypoint omits `--locked` or requires lock regeneration.
  - Refutes if:
    - official tag-local entrypoint requires locked mode and succeeds unchanged.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-005
  - E-006
- Conclusion: The release tag's generated lockfile retains local workspace crate versions at `0.0.0`; Cargo 1.95 must normalize them to `0.146.0`. The runner's extra `--locked` constraint fails before compilation, while the tag-supported offline check succeeds.
- Repair design readiness: ready; replace `--locked` with `--offline` so dependency resolution remains network-independent while local package versions can normalize in the disposable tree
- Next step: validate the corrected runner command end to end
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Core failure is ambient proxy contamination
- Status: confirmed
- Parent: P-001
- Claim: The core test fails because the qualification runner inherits host proxy variables that the test intentionally expects to be absent.
- Layer: environment
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - The assertion observes `http://127.0.0.1:7890` instead of `not-set`.
- Falsifiable predictions:
  - If true: the focused test fails with the inherited proxy and passes when all case variants of HTTP/HTTPS/ALL proxy variables are removed.
  - If false: the focused test still fails in the scrubbed environment or obtains the proxy from candidate-owned config.
- Diagnostic evidence plan:
  - Prediction or clause under test: Run the same focused test in a disposable candidate with inherited and scrubbed proxy environments.
  - Signal: focused test exit code and observed assertion value.
  - Capture method: two controlled nextest invocations sharing the same candidate tree/target.
  - Event name or marker:
    - `u1.core.proxy-isolation`
  - Correlation keys:
    - test `user_shell_commands_do_not_inherit_managed_network_proxy`
  - Differentiates from:
    - candidate code regression in managed proxy stripping
  - Supports if:
    - inherited run fails with the ambient value and scrubbed run passes.
  - Refutes if:
    - scrubbed run fails with the same value or another candidate-owned value.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-007
- Conclusion: The unchanged focused test passes after removing all case variants of HTTP/HTTPS/ALL/NO proxy variables; the original value came from the host environment inherited by the runner.
- Repair design readiness: ready; construct a qualification-only environment that scrubs proxy variables and preserves unrelated variables
- Next step: validate the complete core package in the corrected runner environment
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: App-server failure is a package-scoped nextest dependency gap
- Status: confirmed
- Parent: P-001
- Claim: `cargo nextest run -p codex-app-server` does not build the sibling `codex-code-mode-host` binary required by one integration test, while the candidate's supported workspace/prebuild flow supplies it.
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - The test cannot locate either Cargo binary environment variable or `target/debug/codex-code-mode-host`.
- Falsifiable predictions:
  - If true: tag-local CI/just configuration includes a workspace/prebuild step, and prebuilding the binary makes the unchanged focused app-server test pass.
  - If false: the binary is built by the package-scoped command or the test still fails after the binary is available.
- Diagnostic evidence plan:
  - Prediction or clause under test: Inspect tag-local test orchestration, then compare focused test behavior before and after the candidate-declared dependency build.
  - Signal: binary existence, command exit code, and focused test result.
  - Capture method: disposable candidate tree with no source changes.
  - Event name or marker:
    - `u1.app-server.code-mode-host`
  - Correlation keys:
    - test `app_server_shares_flag_selected_code_mode_host_across_threads`
  - Differentiates from:
    - app-server runtime defect after host launch
  - Supports if:
    - the prebuild creates the binary and the focused test passes unchanged.
  - Refutes if:
    - the binary is already supplied or the failure remains after prebuild.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-005
  - E-008
- Conclusion: The package manifest does not declare the sibling binary as a dev dependency, while the test resolves it at runtime. Explicitly building the existing binary makes the unchanged focused test pass.
- Repair design readiness: ready; expose the helper build as a separate, audited qualification command before app-server tests
- Next step: validate the complete app-server package after the explicit helper build
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: TUI failure is an immutable upstream release snapshot defect
- Status: confirmed
- Parent: P-001
- Claim: The 0.146 tag committed a snapshot that expects development version `0.0.0` while the release build correctly renders `0.146.0`; runner environment changes cannot make both facts consistent.
- Layer: regression-window
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - The existing diff changes only the current version in the update prompt.
- Falsifiable predictions:
  - If true: tag-local package version is `0.146.0`, committed snapshot contains `0.0.0`, and the focused test fails in a scrubbed disposable tree without other changes.
  - If false: version derives from an ambient runner variable or the committed snapshot already expects `0.146.0`.
- Diagnostic evidence plan:
  - Prediction or clause under test: Inspect version source and snapshot at the immutable tag, then run the focused test in a scrubbed environment.
  - Signal: version source, snapshot content, and focused test diff.
  - Capture method: Git object inspection plus one disposable-tree test.
  - Event name or marker:
    - `u1.tui.release-version-snapshot`
  - Correlation keys:
    - snapshot `pnpm_update_available_history_cell_snapshot`
  - Differentiates from:
    - environment-dependent version injection by the qualification runner
  - Supports if:
    - immutable source/snapshot mismatch and focused reproduction agree exactly.
  - Refutes if:
    - scrubbed test passes or version comes from runner environment.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-009
- Conclusion: The tag declares workspace version `0.146.0`, but its committed snapshot still expects `0.0.0`; the exact mismatch persists in a scrubbed environment.
- Repair design readiness: not applicable in U1; changing the candidate or accepting the snapshot would weaken immutable qualification
- Next step: preserve the failure in the full rerun and keep the 0.146 candidate no-go
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Existing CLI locked-resolution failure
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: Git object `b444ee8095917dbbbc222ecba019a42dcd8b9179:docs/v0.0.5/codex-upstream-sync/evidence/rust-v0.146.0/02-cli-check.log`
- Prediction or plan link:
  - H-001 prediction that failure occurs before compilation because the lockfile needs an update
- Matched signal:
  - Cargo exit 101 with explicit locked-file update error
- Correlation keys:
  - candidate commit `e363b08c`
- Raw content:
  ```text
  error: cannot update the lock file <candidate>/codex-rs/Cargo.lock because --locked was passed to prevent this
  ```
- Interpretation: Supports a lock-resolution mismatch but does not yet prove which command the candidate officially supports.
- Time: 2026-08-10 03:16

## Evidence E-002: Existing core proxy assertion
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: Git object `b444ee8095917dbbbc222ecba019a42dcd8b9179:docs/v0.0.5/codex-upstream-sync/evidence/rust-v0.146.0/03-core-tests.log`
- Prediction or plan link:
  - H-002 prediction that the failing child shell observes the inherited host proxy
- Matched signal:
  - observed proxy is `http://127.0.0.1:7890`; expected `not-set`
- Correlation keys:
  - test `user_shell_commands_do_not_inherit_managed_network_proxy`
- Raw content:
  ```text
  Diff < left / right > :
  <http://127.0.0.1:7890
  >not-set
  ```
- Interpretation: Directly identifies an ambient value but still needs the paired scrubbed-environment result.
- Time: 2026-08-10 03:16

## Evidence E-003: Existing app-server missing binary failure
- Related hypotheses:
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: `docs/v0.0.5/codex-upstream-sync/evidence/rust-v0.146.0/04-app-server-tests.log`
- Prediction or plan link:
  - H-003 prediction that package-scoped nextest does not supply the sibling binary
- Matched signal:
  - neither Cargo binary env var nor target fallback exists
- Correlation keys:
  - test `app_server_shares_flag_selected_code_mode_host_across_threads`
- Raw content:
  ```text
  Error: could not locate binary "codex-code-mode-host"; tried env vars ["CARGO_BIN_EXE_codex-code-mode-host", "CARGO_BIN_EXE_codex_code_mode_host"]
  ```
- Interpretation: Confirms the immediate missing-artifact mechanism, not yet the correct official orchestration.
- Time: 2026-08-10 03:16

## Evidence E-004: Existing TUI version snapshot mismatch
- Related hypotheses:
  - H-004
- Direction: supports
- Type: diagnostic-log
- Source: `docs/v0.0.5/codex-upstream-sync/evidence/rust-v0.146.0/05-tui-tests.log`
- Prediction or plan link:
  - H-004 prediction that the committed expected version and release-rendered version differ
- Matched signal:
  - snapshot expects `0.0.0`; actual output renders `0.146.0`
- Correlation keys:
  - snapshot `pnpm_update_available_history_cell_snapshot`
- Raw content:
  ```text
  -│ ✨ Update available! 0.0.0 -> 9.9.9
  +│ ✨ Update available! 0.146.0 -> 9.9.9
  ```
- Interpretation: Strongly supports an upstream snapshot mismatch; focused clean reproduction is still required.
- Time: 2026-08-10 03:16

## Evidence E-005: Tag-local build and test orchestration
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: source-inspection
- Source: immutable candidate files `docs/install.md`, `justfile`, `.github/workflows/rust-ci-full-nextest-platform.yml`, `codex-rs/app-server/Cargo.toml`, and `codex-rs/app-server/BUILD.bazel`
- Prediction or plan link:
  - H-001 and H-003 predictions about candidate-supported entrypoints and runtime helper orchestration
- Matched signal:
  - source build uses `cargo build`; just test uses `NEXTEST_PROFILE=local cargo nextest run --no-fail-fast`; app-server runtime depends on `codex-code-mode-host`, but its Cargo package does not declare that sibling binary as a dev dependency
- Correlation keys:
  - candidate commit `e363b08c`
- Raw content:
  ```text
  cargo build
  RUST_MIN_STACK=8388608 NEXTEST_PROFILE=local cargo nextest run --no-fail-fast "$@"
  //codex-rs/code-mode-host:codex-code-mode-host
  ```
- Interpretation: The runner added an unsupported locked constraint and omitted a runtime helper required by its package-scoped app-server invocation.
- Time: 2026-08-10 03:45

## Evidence E-006: Offline CLI check normalizes release lock and compiles
- Related hypotheses:
  - H-001
- Direction: supports
- Type: controlled-probe
- Source: disposable candidate `/tmp/whale-u1-candidate-4D8Bft`
- Prediction or plan link:
  - H-001 prediction comparing locked and supported offline checks
- Matched signal:
  - the exact locked command exits 101; `cargo check -p codex-cli --bin codex --offline` exits 0 and changes only local workspace package versions in the disposable `Cargo.lock` from `0.0.0` to `0.146.0`
- Correlation keys:
  - candidate commit `e363b08c`
  - Cargo 1.95.0
- Raw content:
  ```text
  --locked: cannot update the lock file because --locked was passed
  --offline: Finished `dev` profile; local workspace versions 0.0.0 -> 0.146.0
  ```
- Interpretation: Network access is unnecessary; local release-version normalization is necessary, so `--offline` is the narrow supported isolation constraint.
- Time: 2026-08-10 03:51

## Evidence E-007: Scrubbed core proxy test passes
- Related hypotheses:
  - H-002
- Direction: supports
- Type: controlled-probe
- Source: disposable candidate focused nextest run
- Prediction or plan link:
  - H-002 paired-environment prediction
- Matched signal:
  - the unchanged focused test passes 1/1 after removing all case variants of HTTP/HTTPS/ALL/NO proxy variables
- Correlation keys:
  - test `user_shell_commands_do_not_inherit_managed_network_proxy`
- Raw content:
  ```text
  1 test run: 1 passed, 3116 skipped
  ```
- Interpretation: The runner inherited the assertion value from its host environment; no candidate source change is required.
- Time: 2026-08-10 03:55

## Evidence E-008: Explicit code-mode-host build satisfies app-server test
- Related hypotheses:
  - H-003
- Direction: supports
- Type: controlled-probe
- Source: disposable candidate helper build followed by focused nextest run
- Prediction or plan link:
  - H-003 prebuild prediction
- Matched signal:
  - `cargo build --offline -p codex-code-mode-host --bin codex-code-mode-host` succeeds and the unchanged focused app-server test then passes 1/1
- Correlation keys:
  - test `app_server_shares_flag_selected_code_mode_host_across_threads`
- Raw content:
  ```text
  helper build exit 0
  1 test run: 1 passed, 1022 skipped
  ```
- Interpretation: The initial failure was qualification orchestration, not app-server behavior.
- Time: 2026-08-10 03:59

## Evidence E-009: Scrubbed TUI snapshot test preserves exact failure
- Related hypotheses:
  - H-004
- Direction: supports
- Type: controlled-probe
- Source: immutable version/snapshot inspection and disposable candidate focused nextest run
- Prediction or plan link:
  - H-004 clean-reproduction prediction
- Matched signal:
  - workspace version is `0.146.0`, committed snapshot expects `0.0.0`, and the unchanged test fails with the same `0.0.0` versus `0.146.0` diff after proxy scrubbing
- Correlation keys:
  - snapshot `pnpm_update_available_history_cell_snapshot`
- Raw content:
  ```text
  expected: Update available! 0.0.0 -> 9.9.9
  actual:   Update available! 0.146.0 -> 9.9.9
  ```
- Interpretation: This is an immutable upstream release fixture defect. U1 must record rather than repair or suppress it.
- Time: 2026-08-10 04:03

## Evidence E-010: Corrected package-level qualification rerun
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Direction: supports
- Type: fix-validation
- Source: `docs/v0.0.5/codex-upstream-sync/upstream-candidate.json` and normalized `01` through `06` logs
- Prediction or plan link:
  - P-001 fix criteria and U1/V1
- Matched signal:
  - format, offline CLI check, and explicit code-mode-host build pass; core, app-server, and TUI package suites remain failed; the original proxy and missing-helper signatures are removed; the TUI release-version mismatch remains; additional failures include unavailable nested sandbox/network behavior and other upstream fixtures
- Correlation keys:
  - candidate commit `e363b08c`
  - six-command qualification set
- Raw content:
  ```text
  summary: 6 commands; 3 passed; 3 failed
  core: 3108 run; 3013 passed; 94 failed; 1 timed out
  app-server: 1022 run; 1003 passed; 19 failed
  tui: 3247 run; 3219 passed; 27 failed; 1 timed out
  production_vendor_unchanged: true
  model_request_count: 0
  ```
- Interpretation: Runner defects identified by U1 are corrected without candidate edits, but V1's all-entrypoint acceptance threshold is not met. This validates the no-go decision rather than authorizing cutover.
- Time: 2026-08-10 04:20
