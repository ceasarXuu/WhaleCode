# Problem P-001: R4 evidence gates do not reproduce on a fresh checkout

- Symptom: R4 handoff on the current `whalecode-alpha` checkout cannot reproduce all R4 evidence gates that the closeout documents cite as passing.
- Expected behavior: R4-H should leave docs, machine-readable manifests, scripts, and durable evidence in a state where lightweight R4 gates pass without depending on uncommitted run-cache directories from a previous machine.
- Actual behavior: `test-r4-sample-ledger.ps1` fails because ledger entries point at missing `target/...` run artifacts, and `test-r4-public-10-usage-accounting-gate.ps1` fails because the default public-10 run roots are absent and the generated report has `0/10` rows.
- Impact: R4 progress cannot be handed off cleanly. Engineering fixes may exist, but R4-H evidence consistency is not currently reproducible from the repository state.
- Environment: Linux checkout at `/home/zhangxu/whalecode-alpha`, branch `whalecode-alpha`, HEAD `700cfe1`.
- Fix criteria: R4 lightweight gates must pass from the current checkout using committed docs/scripts or explicitly declared optional external run roots; generated target evidence must state whether it validates a durable snapshot or live run directories.
- Current conclusion: investigating.

# Hypothesis H-001: R4 ledger records ephemeral target run paths as required repository evidence

- Claim: `docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json` uses `target/...` runtime artifacts as `primary_evidence` and `secondary_evidence`; those files are not committed, so the gate fails even though durable CoE or docs evidence exists.
- Prediction: The ledger gate will report missing `target/...` files, while at least some cited CoE/doc evidence paths exist in the repository.
- Diagnostic evidence plan: Run `test-r4-sample-ledger.ps1`, inspect the generated evidence JSON, and compare missing paths against ledger references.
- Status: confirmed.

# Evidence E-001: R4 sample ledger gate fails on missing target evidence paths

- Prediction tested: H-001 predicts missing `target/...` evidence paths.
- Command: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-sample-ledger.ps1`
- Result: failed.
- Key output: `primary_evidence missing for r3-rerun-single-file-fast-fix-positive-control: target/r3-tool-feedback-rerun-single-file-20260630-0343/single-file-fast-fix/20260630-034332-008/pair-001/pair-report.md`
- Generated evidence: `target/r4-sample-ledger/r4-sample-ledger-evidence.json`
- Evidence detail: generated evidence records `sample_count=12`, `failure_count=19`, with failures concentrated in missing `target/...` primary and secondary evidence files.
- Interpretation: The gate behavior matches H-001.

# Evidence E-002: The ledger requires repository-local evidence paths

- Prediction tested: H-001 predicts the validator checks evidence paths as repository-local files.
- Source: `scripts/taskspace-benchmark/test-r4-sample-ledger.ps1`
- Observation: `Test-RepoPath` joins each evidence path to `$repoRoot` and requires `Test-Path -PathType Leaf`.
- Interpretation: Any ledger entry pointing to an uncommitted `target/...` run artifact will fail on a fresh checkout.

# Hypothesis H-002: R4 public-10 usage accounting gate has no durable good-report fixture

- Claim: `test-r4-public-10-usage-accounting-gate.ps1` tries to generate its good report from default Windows run roots before testing accounting invariants. Without those run roots, the accounting gate fails on missing input artifacts instead of exercising the intended positive and negative report checks.
- Prediction: The usage gate will invoke `write-r4-public-10-tool-stress-report.ps1 -RequireComplete`, which fails with `0/10 found` on the current checkout.
- Diagnostic evidence plan: Run the usage accounting gate and inspect the script call path.
- Status: confirmed.

# Evidence E-003: Public-10 usage accounting gate fails before invariant checks

- Prediction tested: H-002 predicts `0/10 found` report generation failure.
- Command: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1`
- Result: failed.
- Key output: `R4 public-10 report incomplete: 0/10 found; missing: vim-terminal-task, heterogeneous-dates, sqlite-db-truncate, git-multibranch, git-workflow-hack, organization-json-generator, sqlite-with-gcov, processing-pipeline, csv-to-parquet, tmux-advanced-workflow`
- Interpretation: The script cannot reach its intended accounting mismatch checks without the external run directories.

# Evidence E-004: Public-10 report writer defaults to non-repository Windows run roots

- Prediction tested: H-002 predicts default roots outside the repository.
- Source: `scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1`
- Observation: when `-RunRoots` is omitted, the script scans `C:\WhaleRunCache\r4-public10-20260701\actual` and `C:\WhaleRunCache\r4-public10-20260702\actual`.
- Interpretation: The public-10 report generation path is valid on the original Windows run host, but not as a fresh-checkout reproducibility gate.

# Hypothesis H-003: Durable docs/CoE evidence plus a public-10 snapshot can restore fresh-checkout R4-H gates

- Claim: R4-H reproducibility can be restored without fabricating raw run artifacts by changing required ledger evidence to repository-local durable docs/CoE paths, preserving original run paths as archived references, and using a committed public-10 report snapshot for accounting-gate shape and negative mutation checks.
- Prediction: After the repair, `test-r4-sample-ledger.ps1`, `test-r4-public-10-tool-stress-plan.ps1 -ReportPath docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json`, and `test-r4-public-10-usage-accounting-gate.ps1` pass on the current checkout.
- Diagnostic evidence plan: Implement the durable evidence repair, then run the three gates above and record their outputs.
- Status: confirmed.

# Evidence E-005: R4 sample ledger gate passes after durable evidence repair

- Prediction tested: H-003 predicts the sample ledger gate passes after replacing required missing target evidence with durable docs/CoE evidence.
- Repair artifacts: `docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json`
- Command: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-sample-ledger.ps1`
- Result: passed.
- Key output: `R4 sample ledger gate passed: 12 samples`
- Interpretation: The fresh checkout now has repository-local evidence for every ledger row while retaining historical run paths under archived fields.

# Evidence E-006: Public-10 snapshot passes the public sample report gate

- Prediction tested: H-003 predicts the durable snapshot satisfies public-10 report-shape validation.
- Repair artifacts: `docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json`
- Command: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json`
- Result: passed.
- Key output: `R4 public-10 tool-stress gate passed: 10 planned samples`
- Interpretation: Public sample membership and required report fields are now verifiable from committed artifacts.

# Evidence E-007: Usage accounting gate passes without external run roots

- Prediction tested: H-003 predicts the usage accounting gate can exercise good-report and bad-report checks from the durable snapshot.
- Repair artifacts: `scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1`
- Command: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1`
- Result: passed.
- Key output: `PASS: R4 public-10 usage accounting gate rejects ambiguous token usage`
- Interpretation: The gate now validates accounting semantics on fresh checkout instead of failing before the intended invariant checks.

# Hypothesis H-004: public-10 report drops timeout-side rollout token usage

- Claim: A timed-out side can still have `rollout_trace_*_tokens` flushed through `token_count` events, but `write-r4-public-10-tool-stress-report.ps1` only used top-level `input_tokens` and `output_tokens` for token ratios and usage status.
- Prediction: A synthetic pair with missing top-level token summary and present rollout trace token counts will be reported as token usage unavailable before the repair, even though partial usage is recoverable.
- Diagnostic evidence plan: Build a synthetic public-10 pair for `heterogeneous-dates` with only rollout trace token fields, run the report writer, and require `token_ratio_availability=recovered_from_rollout_trace`.
- Status: confirmed.

# Evidence E-008: rollout token fallback is now covered by the usage accounting gate

- Prediction tested: H-004 predicts a synthetic timeout pair can recover token ratio from rollout trace token counts.
- Repair artifacts:
  - `scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1`
  - `scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1`
  - `scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1`
- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1`
- Result: passed.
- Key output:
  ```text
  R4 public-10 report written: target/r4-public-10-usage-accounting-gate/rollout-token-fallback-report.json
  complete_run_count=1 missing_run_count=9
  PASS: R4 public-10 usage accounting gate rejects ambiguous token usage
  ```
- Matched signal:
  - Synthetic `heterogeneous-dates` row reports `token_ratio_availability=recovered_from_rollout_trace`.
  - `taskspace_token_ratio=3`.
  - standard and TaskSpace usage statuses are `recovered_from_rollout_trace`.
- Interpretation:
  - Timeout rows no longer have to collapse to fully unavailable cost evidence when rollout `token_count` events survived the process timeout.
  - This is a report/accounting repair, not TaskSpace utility parity evidence.

# Hypothesis H-005: R4 acceptance state needs a single machine-readable readiness gate

- Claim: R4 handoff previously required reading several docs and running multiple independent scripts to know whether the build was complete, blocked, or ready for a real utility rerun.
- Prediction: A single readiness script can aggregate lightweight R4 gates, write a JSON status artifact, and fail with a stable blocker when the current checkout lacks `DEEPSEEK_API_KEY`.
- Diagnostic evidence plan: Add an R4 readiness script, run it on the current checkout, and require engineering gates to pass while status is `blocked` with `provider_credential_missing`.
- Status: confirmed.

# Evidence E-009: R4 acceptance readiness gate reports the current blocked state

- Prediction tested: H-005 predicts the current checkout has passing engineering gates but cannot close R4 without provider credentials.
- Repair artifact: `scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1`
- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1`
- Result: expected blocked exit code `3`.
- Report: `target/r4-acceptance-readiness/r4-acceptance-readiness.json`
- Matched signal:
  ```text
  status=blocked
  engineering_gates_status=pass
  provider_credential_status=missing
  e3_readiness=not_ready_until_real_utility_evidence_passes
  gate_count=5
  failed_gate_count=0
  blocker=provider_credential_missing
  ```
- Interpretation:
  - R4-H engineering readiness is now machine-readable from one command.
  - The readiness gate intentionally refuses to mark completion until a real DeepSeek utility rerun is possible and passes the required evidence checks.

# Hypothesis H-006: changed-file inventory has a file-disappearance race

- Claim: During real benchmark post-processing, `Add-TaskspaceChangedPath` checks whether a changed path exists, then calls `Get-Item` outside the retry/catch block. If the file disappears between discovery and metadata capture, metrics extraction throws and aborts the whole paired run after model execution.
- Prediction: The keyed `organization-json-generator` run will fail in `metrics-extractor.ps1` at the initial `Get-Item` for a changed path that no longer exists; wrapping metadata capture in the existing retry/missing-file logic will let post-processing continue and record `hash_status=missing`.
- Diagnostic evidence plan: Use the keyed run stack trace and code location to confirm the uncaught `Get-Item` path, then add a focused harness assertion that disappeared changed paths are represented as missing instead of throwing.
- Status: confirmed.

# Evidence E-010: keyed organization-json-generator run aborts in metrics extractor post-processing

- Prediction tested: H-006 predicts post-processing can abort after a changed file disappears before metadata capture.
- Command: `run-taskspace-external-benchmark.ps1 ... organization-json-generator ... deepseek-v4-flash`
- Run root: `target/r4-org-json-real-keyed-20260703/runs/terminal_bench__organization-json-generator/20260703-154156-481`
- Result: process exited `1` during metrics extraction.
- Matched signal:
  ```text
  Get-Item: scripts/taskspace-benchmark/lib/metrics-extractor.ps1:173
  Could not find item
  .../pair-001/left/app/.python-version.
  ```
- Interpretation:
  - The DeepSeek key successfully moved the run past provider preflight into real paired execution.
  - The current blocker is a harness post-processing crash, not a completed utility result.

# Evidence E-011: metrics extractor performs uncaught metadata read after Test-Path

- Prediction tested: H-006 predicts the initial metadata read is outside the catch/retry block.
- Source: `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- Matched signal:
  ```text
  if (Test-Path -LiteralPath $absolute -PathType Leaf) {
      $fileInfo = Get-Item -LiteralPath $absolute
      $size = [int64]$fileInfo.Length
      for ($attempt = 0; $attempt -lt 3; $attempt++) {
          try {
              ...
  ```
- Interpretation:
  - A file that vanishes after `Test-Path` but before `Get-Item` bypasses the intended retry/catch behavior and terminates the script.

# Evidence E-012: missing changed files are now represented instead of aborting metrics extraction

- Prediction tested: H-006 predicts the repaired extractor records vanished changed paths as `hash_status=missing`.
- Repair artifacts:
  - `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
  - `scripts/taskspace-benchmark/test-metrics-extractor-harness.ps1`
- Focused command: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-metrics-extractor-harness.ps1`
- Focused result: passed.
- Real rerun: `target/r4-org-json-real-keyed-20260703b/runs/terminal_bench__organization-json-generator/20260703-155610-406`
- Matched real-run signal:
  ```text
  .python-version[??] hash_status=missing
  PairReport: pair-001/pair-report.md
  ```
- Interpretation:
  - The previous post-processing crash is fixed.
  - A real keyed run now reaches pair report and metrics emission even when a discovered changed file disappears before hashing.
  - This is harness durability evidence, not TaskSpace utility success.

# Evidence E-013: keyed organization-json-generator rerun exposes TaskSpace execution convergence failure

- Prediction tested: After H-006 repair, the keyed rerun should either produce utility evidence or expose the next real blocker.
- Command: `run-taskspace-external-benchmark.ps1 ... organization-json-generator ... deepseek-v4-flash`
- Run root: `target/r4-org-json-real-keyed-20260703b/runs/terminal_bench__organization-json-generator/20260703-155610-406`
- Result: process exited `1`, but `run-status.json` reports a valid completed run with one completed pair.
- Matched signal:
  ```text
  reported_evidence_level: E1
  included_in_utility_aggregate: False
  outcome_standard: wrong
  outcome_taskspace: agent_exec_timeout
  right / taskspace exec_exit_code: 124
  right / taskspace exec_timed_out: True
  right / taskspace tool_call_count: 92
  ```
- Additional diagnostic signal:
  ```text
  TaskSpaceProviderRequestBudgetEventV1 ... request_count=89->90 max=20 state=over_profile_hint
  TaskSpaceNoActionRecoveryV1 ... Recovery attempt 32 ... advisory threshold 3
  bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted
  ```
- Interpretation:
  - `DEEPSEEK_API_KEY` is now correctly wired for the harness; provider preflight passed and the model was invoked.
  - R4 remains not accepted: this run is E1 diagnostic evidence only, not score-valid utility evidence.
  - The next blocker is TaskSpace convergence under sandbox/tool failures: the path exceeded provider request budget hints and timed out instead of producing a bounded blocked-with-evidence result.

# Hypothesis H-007: sandbox/tool runtime bootstrap failures lack task-level terminal feedback semantics

- Claim: When an action-contract ordinary tool fails before execution because the sandbox/tool runtime cannot bootstrap, TaskSpace records raw output on the current node but does not classify the failure as a terminal task-level infrastructure blocker. With no active node after a model-emitted `blocked`, the action contract still permits creating another node, so the agent converts a non-recoverable tool-runtime failure into repeated discovery/retry.
- Prediction: The keyed run will show a specific sandbox bootstrap signature in tool feedback, repeated request/recovery events after the failure, and no runtime classification equivalent to `sandbox_bootstrap_failed` that forbids new ordinary tools after the closed node.
- Diagnostic evidence plan: Inspect the keyed run JSONL and the TaskSpace runtime/session contract code. Confirm whether `bwrap` bootstrap failure is classified separately from ordinary validation failure and whether the no-active-node contract blocks `create_node` after this failure type.
- Status: confirmed.

# Evidence E-014: bwrap bootstrap failure is visible but not terminally classified before repair

- Prediction tested: H-007 predicts raw sandbox bootstrap evidence exists while TaskSpace continues recovery.
- Run root: `target/r4-org-json-real-keyed-20260703b/runs/terminal_bench__organization-json-generator/20260703-155610-406`
- Matched run signals:
  ```text
  bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted
  TaskSpaceProviderRequestBudgetEventV1 ... request_count=89->90 max=20 state=over_profile_hint
  TaskSpaceNoActionRecoveryV1 ... Recovery attempt 32 ... advisory threshold 3
  ```
- Matched code signals before repair:
  - `action_map/runtime.rs` only had local validator infrastructure classification for validation `Build`/`Test` results.
  - `session/turn.rs` only had `TaskSpaceActionContractClosedValidationV1` for blocked validation with no active node.
  - The generic no-active-node action contract still allowed `taskspace_control(create_node)`.
- Interpretation:
  - The failure semantic was missing, not merely distorted. The raw error text survived, but there was no durable `tool_runtime_bootstrap_failure` / `sandbox_bootstrap_failed` classification that changed the next allowed action set.
  - This is an R4 feedback-layer P0 path because a non-recoverable ability-layer failure must be communicated as terminal tool unavailability, not as another inspect/search opportunity.

# Evidence E-015: bwrap bootstrap failure now terminates ordinary TaskSpace tool retries

- Prediction tested: H-007 predicts the repair must classify bwrap bootstrap failure as terminal tool-runtime infrastructure evidence and prevent new ordinary tool nodes after the closed node.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/exec.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
  - `docs/v0.0.5/build-R4/r4-tool-path-coverage.json`
- Focused command: `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core bootstrap_failure --lib`
- Focused result: passed.
- Matched test signals:
  - `sandbox_detection_identifies_bwrap_loopback_bootstrap_failure`
  - `bwrap_bootstrap_failure_auto_blocks_validation_as_local_infra`
  - `tool_runtime_bootstrap_failure_blocks_inspect_node`
  - `taskspace_action_contract_tool_runtime_bootstrap_failure_forbids_new_nodes`
- Regression command: `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core local_infra --lib`
- Regression result: passed, `11 passed`.
- R4 manifest command: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1`
- R4 manifest result: passed, `11 paths`.
- Interpretation:
  - The current repair closes the feedback-layer semantic gap for the observed bwrap bootstrap case.
  - It does not claim R4 utility acceptance; a real `organization-json-generator` rerun is still required to prove the long-flow timeout no longer occurs in the live benchmark path.

# Hypothesis H-008: R4 still has related tool-feedback gaps after bootstrap classification

- Claim: The `organization-json-generator` follow-up runs expose a family of R4 tool-chain failures where the raw signal exists, but TaskSpace either routes it to the wrong phase, omits a required next action, or treats partial inspect evidence as sufficient convergence.
- Prediction: Focused tests will reproduce each subcase as a feedback/coverage contract problem rather than as a generic model retry issue.
- Diagnostic evidence plan: Add focused runtime/session tests for host-platform read commands, duplicate successful read/search, input data artifact evidence, changed-artifact validation coverage, missing validation command scripts, and declared fact-source coverage before inspect finish.
- Status: confirmed.

# Evidence E-016: platform-specific read_file recovery commands are now host-correct

- Prediction tested: H-008 predicts a recovery command can carry the wrong platform syntax and lose feedback actionability.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused command: `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core host_platform_command --lib`
- Result: passed.
- Matched test signals:
  - `action_contract_read_file_uses_host_platform_command`
  - `repeated_blocked_inspect_bootstrap_uses_host_platform_command`
- Interpretation: Recovery feedback no longer tells a Unix shell to run PowerShell `Get-Content`, or a Windows shell to run Unix `sed`.

# Evidence E-017: duplicate successful read/search feedback is structured and repeat-gated

- Prediction tested: H-008 predicts repeated successful inspect reads must not be surfaced as a vague retry opportunity.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_node_blocks_repeated_successful_read_command --lib`
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core forced_inspect_transition_accepts_duplicate_read_search_gate_recovery --lib`
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core duplicate_read_search_recovery_pushes_inspect_transition --lib`
- Result: passed.
- Interpretation: The duplicate read/search case is now a named feedback type, `inspect_duplicate_successful_read_or_search`, with repeat state and recovery context instead of an unbounded loop.

# Evidence E-018: schema/data artifacts now count as inspect working evidence

- Prediction tested: H-008 predicts data-heavy tasks can lose inspect progress if `.json`/`.csv` reads are not counted as working evidence.
- Repair artifact: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused command: `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_data_artifact_read_counts_as_working_evidence --lib`
- Result: passed.
- Interpretation: `schema.json` and CSV inputs read by a shell command are included in working evidence summaries, so TaskSpace can distinguish real data inspection from path listing.

# Evidence E-019: validation changed-artifact coverage feedback is now actionable

- Prediction tested: H-008 predicts validation gates can block a command correctly but fail to pass the exact required command back to the model.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_node_blocks_vacuous_test_after_changed_artifact --lib`
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_changed_artifact_coverage_failure --lib`
- Result: passed.
- Interpretation: A validation command that does not exercise the changed artifact is now reported as `validation_test_missing_changed_artifact_coverage` with an explicit coverage-correct next action.

# Evidence E-020: wrong validation script names stay on the validation node

- Prediction tested: H-008 predicts a command such as `python process.py` can be a validation command error, not implementation evidence.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_missing_command_script_stays_on_validation_node --lib`
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_missing_validation_script_failure --lib`
- Result: passed.
- Interpretation: `can't open file ... process.py` is classified as `validation_command_missing_script`, keeping recovery on the validation node and pointing back to the existing changed script instead of spawning an implementation rework node.

# Evidence E-021: declared fact-source coverage prevents premature inspect convergence

- Prediction tested: H-008 predicts duplicate read/search recovery can force inspect into implement before all declared task fact-source artifacts are read.
- Real-run signal:
  - Run root: `target/r4-org-json-real-keyed-20260703-validation-command-routing`
  - Trace: right side read `schema.json` and `departments.csv`, repeated `departments.csv`, then `TaskSpaceForcedInspectTransitionV1 trigger=inspect_duplicate_read_search_gate_recovery` created implement before `employees.csv` and `projects.csv` were inspected.
  - Later validation failures included `IndentationError` and `KeyError: 'id'`, consistent with implementation based on incomplete CSV/schema evidence.
- Repair artifact: `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused commands:
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish --lib`
  - `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources_block_manual_and_forced_finish_until_read --lib`
- Result: passed.
- Interpretation: Duplicate read/search feedback now reports missing declared fact-source artifacts such as `employees.csv` and `projects.csv`, omits `finish_node` while they are missing, and blocks both manual and forced inspect finish until coverage exists.

# Evidence E-022: Linux sandbox now has ability-layer fallback for restricted netns/proc environments

- Prediction tested: H-007 and H-008 both require tool runtime failure handling to distinguish non-recoverable bootstrap failure from recoverable environment restrictions.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/linux-sandbox/src/linux_run_main.rs`
  - `third_party/codex-cli/codex-rs/linux-sandbox/src/linux_run_main_tests.rs`
  - `third_party/codex-cli/codex-rs/linux-sandbox/README.md`
- Focused command: `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-linux-sandbox --lib`
- Result: passed.
- Smoke signals:
  - bwrap fallback created `target/linux-sandbox-netns-fallback-smoke.txt`.
  - Network socket smoke under sandbox returned `PermissionError: [Errno 1] Operation not permitted`, confirming seccomp still enforces network restriction after fallback.
- Interpretation: Non-proxy restricted networking can degrade from isolated netns to full bwrap network plus seccomp, and legacy-compatible failures can fall back to Landlock/seccomp instead of surfacing as repeated agent-level tool failures.

# Evidence E-023: provider budget advisory remains unresolved as a hard-stop policy

- Prediction tested: H-008 predicts feedback fixes reduce specific loops but do not by themselves make provider budget hints terminal.
- Current result: focused runtime tests now prevent the observed bootstrap, duplicate-read, validation-command, and fact-source convergence errors; no repair in this slice changes provider request budget advisory into a hard stop.
- Remaining signal: `organization-json-generator` still requires a real keyed rerun after the new fixes. If it still exceeds request budget or repeats no-action recovery, the next hypothesis should target request-budget hard gating or repeated no-action terminal blocking.
- Interpretation: This remains an open R4 utility-convergence risk, not a closed engineering benefit.
