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

# Hypothesis H-009: provider budget overrun is caused by missing pre-dispatch hard gate

- Claim: The provider request budget state machine records `over_profile_hint`, but `try_run_sampling_request` does not call `gate_provider_request_pre_dispatch` before opening the provider stream. As a result, request budget events are advisory telemetry rather than a terminal control decision, and the agent can keep sampling after `request_count >= max_requests`.
- Prediction: A real keyed rerun after H-008 fixes will show fewer feedback-layer loops but still continue provider sampling after the active budget is exhausted. Source inspection will show no session-level pre-dispatch hard stop before `stream_with_provider_request_budget`.
- Diagnostic evidence plan: Inspect the keyed rerun trace for request counts after budget exhaustion and inspect `session/turn.rs` for the provider request path. Fix validation requires focused tests that make node and rollout budget exhaustion return `allowed=false`, preserve one explicit `budget_recovery` grace request, and produce a terminal `TaskSpaceProviderBudgetHardStopV1` feedback item.
- Status: confirmed.

# Evidence E-024: keyed rerun confirms advisory budget overrun after feedback fixes

- Prediction tested: H-009 predicts provider sampling continues after active budget exhaustion even when earlier feedback fixes reduce specific loops.
- Real rerun: `target/r4-org-json-real-keyed-20260703d/runs/terminal_bench__organization-json-generator/20260703-235033-117`
- Trace: `pair-001/right/artifacts/whale-exec.jsonl`
- Matched run signals:
  ```text
  request_count=19->20 max=20 state=compact_checkpoint_required->over_profile_hint
  request_count=20->21 ... state=over_profile_hint->over_profile_hint
  request_count=26->27 ... state=over_profile_hint->over_profile_hint
  ```
- Additional signal:
  - TaskSpace read `schema.json`, `departments.csv`, and `employees.csv`.
  - The earlier premature duplicate-read forced transition did not recur.
  - `projects.csv` had not been read before the budget runaway segment.
- Interpretation:
  - H-008 focused fixes improved the feedback layer but did not close the control loop.
  - The remaining failure is a missing hard gate before provider dispatch, not a malformed provider warning string.

# Evidence E-025: provider budget now hard-stops before dispatch

- Prediction tested: H-009 requires provider budget exhaustion to block before the provider stream starts, while preserving one explicit budget recovery grace request.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_budget --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
  git diff --check
  ```
- Result: passed.
- Matched test signals:
  - `taskspace_active_budget_node_request_gate_blocks_before_rollout_budget`
  - `taskspace_active_budget_rollout_request_gate_blocks_pre_dispatch`
  - `taskspace_active_budget_allows_one_budget_recovery_grace_request`
  - `provider_budget_hard_stop_item_is_terminal_recovery_guidance`
- Interpretation:
  - `provider-budget-advisory-runaway` is closed by focused engineering evidence.
  - R4 utility acceptance is still pending a real keyed `organization-json-generator` rerun with the new binary.

# Hypothesis H-010: fixed per-node provider budget can hard-stop inspect before required fact-source coverage

- Claim: After H-009, provider budget hard stop correctly prevents runaway, but a fixed per-node hard limit can stop a data-heavy inspect node before the state machine's minimum fact-source coverage is possible. In this case the active task declared four required fact-source artifacts, but the deep profile per-node hard limit was five requests.
- Prediction: A real rerun with H-009 will no longer timeout, but TaskSpace can stop at a `provider_node_request_hard_limit_exceeded` before reading all declared fact sources. Source inspection will show `provider_request_budget_snapshot` returns the fixed profile `max_model_requests_per_node` without adapting to declared fact-source count.
- Diagnostic evidence plan: Inspect the rerun pair report and right trace for early `TaskSpaceProviderBudgetHardStopV1`; inspect snapshot construction; fix validation requires a focused test where inspect fact-source count expands the effective node request limit, plus provider budget regression tests.
- Status: confirmed.

# Evidence E-026: hard stop removed timeout but exposed premature inspect budget stop

- Prediction tested: H-010 predicts H-009 converts runaway into a bounded stop, but the fixed node limit can be too low for declared fact-source coverage.
- Real rerun: `target/r4-org-json-real-keyed-20260703e-hardgate/runs/terminal_bench__organization-json-generator/20260704-000713-854`
- Matched run signals:
  ```text
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: wrong
  right / taskspace exec_timed_out: False
  right / taskspace wall_time_ms: 27571
  right / taskspace tool_call_count: 5
  TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=5/20 node_request_count=5/5
  ```
- Additional trace signal:
  - TaskSpace read `schema.json` and `departments.csv`, repeated `departments.csv`, then stopped before `employees.csv` and `projects.csv`.
- Interpretation:
  - H-009 fixed the unbounded retry/timeout class.
  - The next blocker is a control-layer budget/fact-source mismatch: hard stop is now real, but its per-node limit is lower than the evidence floor required by the task state.

# Evidence E-027: inspect provider node limit now adapts to declared fact sources

- Prediction tested: H-010 requires data-heavy inspect nodes to receive a higher effective per-node request hard limit before provider dispatch.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_budget --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
  ```
- Result: passed.
- Matched test signals:
  - `taskspace_active_budget_expands_inspect_node_limit_for_fact_sources`
  - `provider_budget_limit_reached_detects_rollout_or_node_limit`
- Interpretation:
  - The hard-stop policy remains in place.
  - For inspect nodes, the effective node request hard limit now respects declared fact-source artifacts so the runtime does not stop before the minimum evidence floor is reachable.
  - Real utility validation still requires another keyed rerun.

# Hypothesis H-011: implementation rework recovery loses the joint validation-failure plus inspected-data contract

- Claim: After adaptive inspect budget lets TaskSpace read all declared fact-source artifacts, the next failure is a feedback-layer rework problem. The validation failure is routed to an implement_solution rework node, but the recovery summary does not reliably combine the latest validation failure with transitive inspect evidence from the original implementation chain. The model can therefore patch a systemic generated-file error line by line, then rewrite using invented fields such as `salary` instead of observed CSV headers.
- Prediction: A keyed rerun after H-010 will read `employees.csv`, `departments.csv`, `projects.csv`, and schema before implementation, then fail during implement rework with visible validation errors such as `IndentationError` and `KeyError`. Source inspection will show implement recovery guidance lacks a failure-priority contract for whole-file indentation errors and missing fields, and working evidence summary needs dependency-chain coverage rather than only direct dependency fallback.
- Diagnostic evidence plan: Inspect the adaptive-budget keyed rerun trace and pair report; inspect `current_main_working_evidence_summary` and `build_taskspace_implement_needs_edit_recovery_item`; fix validation requires focused tests that combine transitive inspect CSV evidence with a blocked validation failure and verify recovery text prioritizes validation failures, whole-file indentation repair, and observed field names.
- Status: confirmed.

# Evidence E-028: adaptive-budget rerun exposes implementation rework feedback loss

- Prediction tested: H-011 predicts the next blocker appears after inspect succeeds, not before it.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703f-adaptive-budget/runs/terminal_bench__organization-json-generator/20260704-001749-411
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 16
  ```
- Matched trace signals:
  - TaskSpace read `data/employees.csv`, `data/departments.csv`, `data/projects.csv`, and schema before implementation; H-010 no longer stopped inspect early.
  - First generated `generate_organization.py` had top-level leading spaces and failed with `IndentationError: unexpected indent`.
  - Recovery patched only the first line; the next validation failed with another `IndentationError` on the following line.
  - A later patch attempt failed, then the replacement script referenced unobserved fields including `salary`.
  - Final validation failed with `KeyError: 'salary'`, followed by `TaskSpaceProviderBudgetHardStopV1 reason=provider_request_hard_limit_exceeded request_count=20/20`.
- Interpretation: The failure signal was not absent from the trace, but the implement rework feedback contract did not preserve the joint instruction "fix this validation failure using the already inspected source-data schema" strongly enough for the next action. This is a new R4-D feedback-layer problem type, not the earlier inspect coverage or provider-budget problem.

# Evidence E-029: implementation rework recovery now joins failure evidence with transitive inspect data

- Prediction tested: H-011 requires recovery feedback to preserve both latest validation failure and upstream data/schema evidence.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_summary_merges_transitive_inspect_evidence_and_failure --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core implement_recovery_prioritizes_validation_failure_and_inspected_fields --lib
  ```
- Result: passed.
- Matched test signals:
  - `current_main_working_evidence_summary()` now includes `validation_rework`, `KeyError`, `salary`, `data/employees.csv`, and `employee_id,name,department_id,title` in the same recovery summary.
  - `TaskSpaceImplementNeedsEditRecoveryV1` now tells the model to treat validation failure as the primary target, fix top-level Python `IndentationError` at whole-file/block scope, and use only observed schema/CSV/JSON field names for `KeyError` repairs.
- Interpretation: The new `implementation-rework-feedback-evidence-join` class is focused-fixed. Real utility validation still requires another keyed rerun to see whether `organization-json-generator` moves past implement rework or exposes the next failure class.

# Hypothesis H-012: inspect next-action projection can advertise finish before declared fact-source coverage

- Claim: After H-011, the next `organization-json-generator` blocker is a provider-visible projection bug. The lower-level duplicate/manual/forced finish guards can detect missing declared fact-source artifacts, but `projection_next_valid_actions` does not receive `TaskState` and therefore cannot apply the same fact-source coverage check. Once an inspect node has any main tool result, projection can still advertise `finish_node -> implement_solution` even when `projects.csv` is unread.
- Prediction: A keyed rerun after H-011 will show TaskSpace missing at least one declared fact source while the final projection still lists an implement transition as a valid next action. Source inspection will show `projection_next_valid_actions(map, current_node_id)` lacks task/fact-source context. Fix validation requires a focused test where `schema.json`, `departments.csv`, and `employees.csv` are read, `projects.csv` is missing, and projection omits `finish_node`.
- Diagnostic evidence plan: Inspect the H-011 rerun pair report, rollout projection, and `whale-exec.jsonl`; inspect `append_context_projection_with_header` and `projection_next_valid_actions`; validate with focused projection and adjacent fact-source guard tests.
- Status: confirmed.

# Evidence E-030: keyed rerun shows projection advertised finish while `projects.csv` was unread

- Prediction tested: H-012 predicts the failure survives after implementation rework feedback is joined and appears as a projection next-action inconsistency.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703g-rework-evidence/runs/terminal_bench__organization-json-generator/20260704-003459-046
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  ```
- Matched trace signals:
  - TaskSpace read `schema.json`, `departments.csv`, and `employees.csv`.
  - TaskSpace did not read `projects.csv`.
  - The final projection listed verified input evidence for the first three artifacts only.
  - The same projection still listed `taskspace_control(action=finish_node, ... next_node_kind="implement_solution" ...)` under `next_valid_actions`.
  - The turn ended with `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=11/20 node_request_count=11/10`.
- Interpretation: The missing signal was not a raw tool failure. The feedback-layer problem is that projection emitted an invalid-looking valid action because it lacked the same task-level fact-source guard that lower-level runtime paths already use.

# Evidence E-031: inspect projection now blocks finish until declared fact sources are read

- Prediction tested: H-012 requires projection to use TaskState fact-source coverage before advertising inspect finish.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_blocks_inspect_finish_until_declared_fact_sources_read --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_prioritizes_inspect_to_implement_after_evidence --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources_block_manual_and_forced_finish_until_read --lib
  ```
- Result: passed.
- Matched test signals:
  - When `projects.csv` is declared but unread, `projection_next_valid_actions` now mentions `projects.csv`, includes `do not finish inspect_code_context`, and omits `finish_node` / `next_node_kind="implement_solution"`.
  - Existing normal projection behavior still permits inspect-to-implement when no declared fact-source artifact is missing.
  - Existing duplicate/manual/forced finish fact-source guards remain passing.
- Interpretation: The new `inspect-projection-finish-before-fact-source-coverage` class is focused-fixed. Real utility validation still requires another keyed rerun to see whether TaskSpace now reads `projects.csv` and moves into implementation, or exposes the next long-flow blocker.

# Hypothesis H-013: implementation rework can misblock editable validation failures as closed infrastructure

- Claim: After H-012, `organization-json-generator` moves past inspect fact-source coverage and into implementation/validation, but a deterministic editable Python failure can still be accepted as a terminal blocker. Specifically, an implement rework node with dependency validation evidence for `IndentationError` can call `block_node` with a reason like "need to inspect file state"; runtime accepts it, leaves no active rework path, and the final response says validation is closed as blocked by local infrastructure even though the failure is implementation code syntax.
- Prediction: A keyed rerun after H-012 will read all fact sources, create `generate_organization.py`, run validation, see `IndentationError`, then accept `block_node` instead of forcing another implementation edit. Source inspection will show `block_main_node` has guards for validator procedure/missing source/internal policy blockers, but not for editable validation failures presented as a blocker. Fix validation requires focused runtime and action-contract prompt tests for rejecting this blocker with a structured `editable_validation_failure_blocker_rejected` feedback kind.
- Diagnostic evidence plan: Inspect the H-012 rerun pair report, validation logs, final action, and `whale-exec.jsonl`; inspect `block_main_node` and recent tool feedback classification; validate with focused runtime/session tests plus adjacent rework-blocker regressions.
- Status: confirmed.

# Evidence E-032: keyed rerun progresses past projection guard and exposes editable failure misblock

- Prediction tested: H-013 predicts the previous projection/fact-source blocker is gone and the next failure is a rework closeout error.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703h-projection-factsource/runs/terminal_bench__organization-json-generator/20260704-004643-993
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 10
  ```
- Matched trace signals:
  - TaskSpace read `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv`.
  - TaskSpace created `generate_organization.py` and ran `python generate_organization.py`.
  - Validation failed first with `IndentationError` on line 2, then after a one-line patch failed again on line 3.
  - The next implement rework node emitted `taskspace_control(action=block_node)` with rationale "Need to inspect file state to fix remaining indentation errors."
  - The final action was `blocked` with reason "closed validation state prevents further editing or testing" and `infra-evidence-unresolved-indentation`.
  - Public validation failed because `/app/organization.json` did not exist.
- Interpretation: H-012 fixed the projection fact-source issue in the real run. The remaining failure is a control/feedback misclassification: source-code `IndentationError` is repairable implementation evidence, not local infrastructure evidence and not a terminal blocker.

# Evidence E-033: editable validation failure blockers are now rejected and structured

- Prediction tested: H-013 requires `block_node` to be rejected when the current implement rework node has dependency validation evidence for an editable code failure and no successful edit.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_editable_validation_failure_blocker_before_edit --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_editable_validation_failure_blocker_rejection --lib
  ```
- Result: passed.
- Matched test signals:
  - Runtime now rejects `block_node` with `cannot be blocked for editable validation failure` and tells the model to `apply_patch` the failed validation artifact.
  - Session recent-feedback classifies the rejection as `failure_kind: editable_validation_failure_blocker_rejected`, not generic `tool_execution_failed`.
  - The recovery text explicitly says top-level Python `IndentationError` / `SyntaxError` should be fixed across the whole affected file or block in one edit.
- Adjacent regression commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_validator_procedure_blocker_before_edit --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_missing_current_artifact_visibility_blocker --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core implement_recovery_prioritizes_validation_failure_and_inspected_fields --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_validator_procedure_blocker_rejection --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_missing_source_blocker_rejection --lib
  ```
- Adjacent regression result: passed.
- Interpretation: The new `implementation-editable-validation-failure-misblocked` class is focused-fixed. Real utility validation still requires another keyed rerun to see whether TaskSpace repairs the full indentation issue and creates `organization.json`.

# Evidence E-034: second rerun exposes the same editable misblock with read-restriction wording

- Prediction tested: H-013 predicts the root class persists if the guard does not recognize the provider's blocker wording.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703i-editable-blocker/runs/terminal_bench__organization-json-generator/20260704-005922-113
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 8
  ```
- Matched trace signals:
  - TaskSpace created `processor.py` and validation failed with fallback command output containing `IndentationError: unexpected indent`.
  - The model repeatedly attempted `finish_node`; runtime rejected at least one finish with "cannot be completed without a recorded successful edit action."
  - The final `block_node` reason was: `Test failed with IndentationError; cannot read files to diagnose because read actions are not allowed in current narrowed state`.
  - Runtime accepted that blocker as `TaskSpace node blocked: node-4 result result-11`, because the editable-failure blocker detector did not yet match `cannot read` / `read actions are not allowed` / `current narrowed state`.
  - Public validation still failed because `/app/organization.json` did not exist.
- Repair update:
  - `blocker_claims_editable_validation_failure_as_blocker` now treats `cannot read`, `read actions are not allowed`, `read restriction`, `insufficient information`, and `current narrowed state` as blocker claims when paired with an editable validation failure.
  - The focused runtime test now uses the exact observed blocker wording.
- Focused command:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_editable_validation_failure_blocker_before_edit --lib
  ```
- Result: passed.
- Interpretation: H-013 remains the active failure class; this evidence tightens the detector to the real provider wording observed in `20260704-005922-113`. Another keyed rerun is still required for real utility validation.

# Hypothesis H-014: validation closeout can treat generator execution as output-contract validation

- Claim: After editable-failure blocker wording is fixed, `organization-json-generator` can create an output file and run its generator with `exit_code=0`, but the validation closeout path can still mark the node complete even when the command only proves script execution and does not check the declared output contract (`organization.json` shape, `schema.json`, public tests, or equivalent assertions).
- Prediction: A keyed rerun after H-013 will show TaskSpace running a command like `python generate_json.py`, receiving `organization.json generated successfully`, and then emitting forced validation closeout/final answer while public validation still fails on schema/field contract keys such as `project.members` and `statistics.averageDepartmentBudget`.
- Diagnostic evidence plan: Inspect public validator stdout, generated `organization.json`, `schema.json`, and `whale-exec.jsonl`; add pre-run validation gate plus forced-closeout backup gate; validate with focused runtime tests and action-contract feedback tests.
- Status: confirmed.

# Evidence E-035: generator-only validation closeout false positive

- Prediction tested: H-014 predicts the next real failure is a validation success false positive rather than an editable blocker.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703j-editable-wording/runs/terminal_bench__organization-json-generator/20260704-010752-603
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: wrong
  right_exec_timed_out: False
  right_tool_call_count: 8
  ```
- Matched trace signals:
  - TaskSpace created `generate_json.py`.
  - The only validation command was `python generate_json.py`.
  - The command exited 0 and printed `organization.json generated successfully.`
  - Runtime emitted `TaskSpaceForcedValidationCloseoutV1 trigger=validation_success_after_tool_drain`.
  - Final answer claimed the artifact followed `schema.json`.
  - Public validator failed with `KeyError: 'members'` and `KeyError: 'averageDepartmentBudget'`.
  - Generated output used `member_ids` and snake_case statistics (`total_employees`, `average_years_of_service`) while schema/tests require `members`, `averageDepartmentBudget`, `totalEmployees`, etc.
- Interpretation: The tool result was not lost. Its semantics were incomplete: `exit_code=0` meant “generator executed”, not “declared output contract validated”. The feedback/control layer upgraded an execution success into validation success.

# Evidence E-036: output-contract coverage gate blocks generator-only validation

- Prediction tested: H-014 requires generator-only `run_test` to be rejected before execution when declared output contract artifacts exist, and forced closeout to downgrade any already-recorded generator-only validation result.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks_generator_only_command_for_schema_output_contract --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation_rejects_generator_only_output_contract_success --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt_structures_output_contract_coverage_failure --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_output_contract_coverage_recovery_preserves_next_action --locked
  ```
- Result: passed.
- Adjacent regression command:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  ```
- Adjacent regression result: passed, 77 tests.
- Matched test signals:
  - `python generate_json.py` is blocked with `validation_test_missing_output_contract_coverage` when the task declares `organization.json` / `schema.json`.
  - The next valid action preserves a combined command such as `python generate_json.py && python -m jsonschema -i organization.json schema.json`.
  - A forced closeout attempt after a generator-only successful result reopens success criteria citing that result and marks the result invalid before rejecting closeout.
  - Direct output artifacts remain allowed when the changed artifact itself is the output and the validation body contains a concrete output value.
  - Existing local validator, changed-artifact coverage, validation infra, and rework routing regressions continue to pass.
- Interpretation: The new `validation-closeout-output-contract-coverage-gap` class is focused-fixed. Real utility validation still requires another keyed rerun to verify that TaskSpace uses a contract-checking validation command and no longer finalizes after generator-only success.

# Hypothesis H-015: output-contract coverage can accept weak JSON parse instead of schema validation

- Claim: After H-014, TaskSpace correctly rejects generator-only validation and feeds the failure back to the model, but the output-contract coverage predicate can still be too weak when the schema artifact is recorded as a fact source or success criterion rather than as an output contract. In that case a command like `python process.py && python -c 'json.load(open("organization.json"))'` mentions the output artifact and has parse semantics, so runtime accepts it even though it does not validate `schema.json`, public tests, or equivalent field assertions.
- Prediction: The next keyed rerun will show `validation_test_missing_output_contract_coverage` feedback for `python process.py`, then the model will switch to a weak JSON parse command, final answer will claim schema validation, and public validation will still fail on schema/test keys. Source inspection will show `task_output_contract_validation_requirements` only derives schema targets from `output_contracts`, not from `fact_sources` / `success_criteria`.
- Diagnostic evidence plan: Inspect the H-014 rerun trace for `TaskSpaceValidationNeedsTestRecoveryV1`, the accepted validation command, public validator errors, and generated JSON keys; add a focused runtime test where `organization.json` is an output contract while `schema.json` is only a fact source; tighten command semantics so schema/validator targets require schema/validator validation semantics.
- Status: confirmed.

# Evidence E-037: keyed rerun blocks generator-only validation but accepts weak JSON parse

- Prediction tested: H-015 predicts the old generator-only closeout is gone and the next failure is weak coverage semantics.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703k-output-contract/runs/terminal_bench__organization-json-generator/20260704-013819-201
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: wrong
  right_exec_timed_out: False
  right_tool_call_count: 9
  ```
- Matched trace signals:
  - Runtime rejected `run_test` command `python process.py` with `validation_test_missing_output_contract_coverage`.
  - Session inserted `TaskSpaceValidationNeedsTestRecoveryV1`, proving the failure semantic reached the model.
  - The model then ran `python process.py && python -c 'import json; data=json.load(open("organization.json")); print("Valid")'`.
  - That command exited 0 and only proved JSON parse/read success.
  - The final answer claimed the processor was validated successfully against the schema.
  - Public validator failed with `KeyError: 'members'` and `KeyError: 'averageDepartmentBudget'`.
  - Generated output still used `memberIds` and omitted `averageDepartmentBudget`, `skillDistribution`, `departmentSizes`, and `projectStatusDistribution`.
- Interpretation: This is not raw tool failure and not feedback loss. It is feedback semantic under-specification: the model obeyed the recovery signal, but runtime accepted a weaker command because `schema.json` lived in fact sources rather than output contracts.

# Evidence E-038: schema fact-source output-contract gate rejects weak JSON parse

- Prediction tested: H-015 requires schema/validator artifacts from fact sources and success criteria to participate in output-contract validation requirements.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  cargo fmt --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
  ```
- Result: passed; `validation_` now covers 78 tests.
- Matched test signals:
  - When `organization.json` is the output contract and `schema.json` is only a fact source, `python process.py && python -c "json.load(...); print('Valid')"` is rejected with `validation_test_missing_output_contract_coverage`.
  - The next valid action preserves `python process.py && python -m jsonschema -i organization.json schema.json`.
  - Schema-aware validation with `python -m jsonschema` is allowed.
  - Direct output artifact edits can still be validated by concrete output assertions such as `assert isinstance(data, dict)`.
- Interpretation: The new `validation-output-contract-schema-fact-source-gap` class is focused-fixed. Real utility validation still requires another keyed rerun to determine whether TaskSpace now performs schema/public-test validation or exposes the next R4 tool/control issue.

# Hypothesis H-016: validation recovery next action is diluted by active projection

- Claim: After H-015, runtime can correctly reject weak validation and generate a precise recovery action, but the active context projection can recompute generic validation-node actions (`run validator/test command`) instead of carrying the latest `TaskSpaceGateRecoveryV1.next_valid_actions`. This creates a feedback-layer semantic loss between gate recovery and the next provider-visible TaskSpace surface.
- Prediction: A keyed rerun after H-015 will show `TaskSpaceGateRecoveryV1` and `TaskSpaceValidationNeedsTestRecoveryV1` containing an exact command such as `python process.py && python -m jsonschema -i organization.json schema.json`, followed by `ContextProjectionV1 active replacement` that only advertises `run validator/test command`. The model will keep submitting weaker validation commands until the smoke node hits provider-node hard stop.
- Diagnostic evidence plan: Inspect rollout ordering around the blocked `run_test`, recovery developer message, following active projection, and provider budget hard stop; persist latest gate recovery next actions in runtime state and feed them into projection for smoke/regression nodes.
- Status: confirmed.

# Evidence E-039: exact recovery exists but active projection weakens it

- Prediction tested: H-016 predicts the feedback is generated correctly but diluted by the projection layer.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703l-schema-factsource/runs/terminal_bench__organization-json-generator/20260704-014928-473
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: wrong
  right_exec_timed_out: False
  right_tool_call_count: 13
  right_open_leaf_nodes: 1
  ```
- Matched trace signals:
  - Runtime rejected repeated weak validation attempts with `validation_test_missing_output_contract_coverage`.
  - `TaskSpaceGateRecoveryV1.next_valid_actions` contained `run_test with command \`python process.py && python -m jsonschema -i organization.json schema.json\``.
  - `TaskSpaceValidationNeedsTestRecoveryV1` told the model to obey `next_valid_actions` and use the named command exactly.
  - The immediately following `ContextProjectionV1 active replacement` exposed only `run validator/test command` plus state-commit guidance for the same `smoke_test` node.
  - Provider hard-stopped at `provider_node_request_hard_limit_exceeded request_count=14/20 node_request_count=6/5`.
  - Public validation failed because `/app/organization.json does not exist`; TaskSpace never reached the required schema-validating command.
- Interpretation: This is feedback semantic loss, not raw tool failure. The exact recovery action was present in the gate payload and recovery developer message, but the projection surface recomputed a weaker action list and competed with the more precise feedback.

# Evidence E-040: projection preserves latest validation gate recovery next action

- Prediction tested: H-016 requires the latest gate recovery actions to be durable enough to survive active/shadow projection.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  ```
- Result: passed; `validation_` covers 78 tests.
- Matched test signals:
  - After weak JSON validation is rejected, active projection contains `python process.py && python -m jsonschema -i organization.json schema.json`.
  - Active projection adds `do not substitute weaker validation; use the exact recovered command unless it cannot run`.
  - Active projection no longer emits the generic `run validator/test command` line while latest gate recovery exists for the validation node.
  - Existing generator-only, schema fact-source, forced closeout, and validation-node guard regressions continue to pass.
- Interpretation: The new `validation-recovery-next-action-projection-dilution` class is focused-fixed. Real utility validation still requires another keyed rerun to verify the model now follows the schema-validating command and exposes the next unresolved R4 tools issue, if any.

# Hypothesis H-017: validation rework lacks permission to read its changed artifact

- Claim: After H-016, TaskSpace can finally execute the exact schema-validation command and route schema failures into implementation rework, but `implementation_needs_edit` still narrows the session action contract to edit-only behavior. If the schema failure has no traceback/file path, runtime does not expose the dependency changed artifact as the readable rework target, so the model cannot inspect the current implementation file before patching.
- Prediction: A keyed rerun after H-016 will show the exact `python generate_org.py && python -m jsonschema -i organization.json schema.json` command executing, schema errors such as missing `members` / `averageDepartmentBudget`, a rework node, then a `read_file` rejection under `node_policy_violation:implement_solution:read_file:implementation_needs_edit` before the model blocks because it cannot read `generate_org.py`.
- Diagnostic evidence plan: Inspect the H-016 rerun trace for the exact schema-validation command, schema error output, rework recovery event, rejected read action, and final blocker text; add focused tests that derive rework targets from validation dependency changed artifacts and allow only that named artifact read through the session action contract.
- Status: confirmed.

# Evidence E-041: schema failure routes to rework but target read is blocked

- Prediction tested: H-017 predicts that H-016 enables exact schema validation, and the next blocker moves to target-artifact visibility inside implementation rework.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703m-recovery-projection/runs/terminal_bench__organization-json-generator/20260704-020629-368
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 8
  ```
- Matched trace signals:
  - `item_26` submitted `run_test` with `python generate_org.py && python -m jsonschema -i organization.json schema.json`.
  - `item_27` executed that exact command and failed with real schema errors: project objects had `member_ids` while `members` was required, and statistics omitted required camelCase keys including `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - `item_29` inserted `TaskSpaceImplementNeedsEditRecoveryV1`.
  - `item_33` tried to read `schema.json`; `item_35` rejected it with `node_policy_violation:implement_solution:read_file:implementation_needs_edit`.
  - `item_40` blocked with `Cannot apply correct patch without reading generate_org.py to see current project processing code`.
  - `item_52` correctly identified the schema failure as a real implementation defect rather than infrastructure.
- Interpretation: The failure semantic is no longer missing and no longer diluted. The remaining feedback/control gap is that validation rework does not carry a precise target-artifact read allowance from the dependency changed artifacts into the session action contract.

# Evidence E-042: validation rework exposes and gates named target artifact reads

- Prediction tested: H-017 requires runtime to derive the rework target from the blocked validation dependency and session action-contract enforcement to permit only that named target read.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_allows_named_validation_rework_artifact_read --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core implementation_needs_edit --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
  ```
- Result: passed.
- Matched test signals:
  - A schema failure without traceback now derives `generate_org.py` from the blocked validation dependency changed artifacts.
  - Implement rework projection lists the action `read_file validation rework target artifact generate_org.py only if current contents are not visible`.
  - The provider budget snapshot exposes `current_node_validation_rework_artifacts`.
  - Session action contract allows `read_file` for `generate_org.py` under `implementation_needs_edit` but still rejects broad reads such as `schema.json`.
  - `validation_rework` passed 11 tests and `validation_` passed 80 tests.
- Interpretation: The new `validation-rework-target-artifact-read-gap` class is focused-fixed. Real utility validation still requires another keyed rerun to verify TaskSpace reads or patches `generate_org.py`, fixes schema fields, and either passes public validation or exposes the next R4 tools issue.

# Hypothesis H-018: missing jsonschema dependency is misrouted as implementation rework

- Claim: After H-017, TaskSpace can read validation rework targets, but a validation command that fails before schema execution because `python3` lacks the `jsonschema` module is still classified as a non-infrastructure validation failure. Runtime then routes to implementation rework, where the model can read the output artifact but repeatedly tries `finish_node` because it believes the validator dependency, not the implementation, failed.
- Prediction: A keyed rerun after H-017 will show TaskSpace executing a schema validation command with `python3 -c "import json, jsonschema; ..."`, receiving `ModuleNotFoundError: No module named 'jsonschema'`, entering `TaskSpaceImplementNeedsEditRecoveryV1`, reading the named target artifact, then repeatedly trying `finish_node` until provider-node hard stop.
- Diagnostic evidence plan: Inspect the H-017 rerun trace and pair report; inspect runtime classification around `validation_node_failed_noninfra_result` and local infra predicates; add a focused test proving `ModuleNotFoundError: jsonschema` stays on the validation node and projection emits `python -m jsonschema -i organization.json schema.json`.
- Status: confirmed.

# Evidence E-043: jsonschema module missing routes into rework and loops

- Prediction tested: H-018 predicts the targeted rework read is now fixed, and the next blocker is validator dependency failure misclassification.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703n-rework-target-read/runs/terminal_bench__organization-json-generator/20260704-022632-418
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 9
  right_open_leaf_nodes: 1
  ```
- Matched trace signals:
  - TaskSpace read `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv`, then created `organization.json`.
  - `item_26` submitted `run_test` command `python3 -c "import json, jsonschema; schema=json.load(open('schema.json')); data=json.load(open('organization.json')); jsonschema.validate(data, schema); print('Validation passed')"`.
  - `item_27` failed before schema execution with `ModuleNotFoundError: No module named 'jsonschema'`.
  - `item_29` routed to `TaskSpaceImplementNeedsEditRecoveryV1`.
  - `item_33` successfully read `organization.json`, proving H-017's named-target read allowance works.
  - `item_41`, `item_48`, `item_55`, `item_62`, and `item_69` repeatedly tried `finish_node` while citing missing `jsonschema` as environment validation failure.
  - `item_73` ended with `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=15/20 node_request_count=6/5`.
- Interpretation: The new failure is not missing target-read capability. It is validation feedback classification: dependency failure happened before the schema check, so it should stay on validation with an alternate validator command instead of being treated as implementation evidence.

# Evidence E-044: missing jsonschema dependency stays on validation with CLI recovery

- Prediction tested: H-018 requires `ModuleNotFoundError: jsonschema` to be excluded from non-infra implementation rework and projected as an actionable validation retry.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused and adjacent commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_missing_jsonschema_dependency_stays_on_validation_with_cli_recovery --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core local_infra --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
  ```
- Result: passed; `validation_` now covers 81 tests, and `local_infra` remains 11 tests.
- Matched test signals:
  - A failed validation result with `ModuleNotFoundError: No module named 'jsonschema'` no longer triggers `validation_node_failed_noninfra_result`.
  - The current node remains `smoke_test` instead of auto-routing to implementation rework.
  - Projection exposes `python -m jsonschema -i organization.json schema.json` as the next validation action.
  - Existing local-infrastructure blockers such as E_ACCESSDENIED, uv cache access denial, and bwrap bootstrap failure still pass their focused coverage.
- Interpretation: The new `validation-jsonschema-module-missing-rework-misroute` class is focused-fixed. Real utility validation still requires another keyed rerun to verify TaskSpace uses the CLI validator, reaches real schema errors, and either fixes the output contract or exposes the next R4 tools issue.

# Hypothesis H-019: action-contract sed reads lose validation rework artifact identity

- Claim: After H-018, TaskSpace reaches real schema errors and enters implementation rework, but the Unix action-contract `read_file` transport maps `read_file(path)` to `sed -n '1,240p' -- path`. Runtime records those successful reads with `actionClass=read` but `artifactRefs=[]` because `read_command_artifact_ref` only recognizes PowerShell `Get-Content`, `cat`, and `type`. The existing `validation_rework_duplicate_artifact_read` gate therefore cannot tell the target artifact was already read, so repeated `read_file csv_processor.py` calls drain the provider-node budget instead of forcing `apply_patch` or `blocked`.
- Prediction: A keyed rerun after H-018 will show schema validation executing with `python -m jsonschema`, rework reading `csv_processor.py` repeatedly on node-4, each `main_tool_result` carrying `artifactRefs: []`, repeated `TaskSpaceImplementNeedsEditRecoveryV1`, and final `TaskSpaceProviderBudgetHardStopV1`.
- Diagnostic evidence plan: Inspect the H-018 rerun pair report and right-side trace for repeated `read_file csv_processor.py`, trace artifact refs, and node hard stop; inspect `read_command_artifact_ref` and read-result reservation code; add a focused test proving Unix `sed -n '1,240p' -- generate_org.py` records the artifact ref and that a second read of the same validation rework target is blocked before edit.
- Status: confirmed.

# Evidence E-045: repeated rework read loses artifact refs and bypasses duplicate gate

- Prediction tested: H-019 predicts that the next blocker after jsonschema recovery is artifact identity loss on Unix action-contract reads.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703o-jsonschema-recovery/runs/terminal_bench__organization-json-generator/20260704-024204-931
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  ```
- Matched trace signals:
  - `item_26` ran `python csv_processor.py && python -m jsonschema -i organization.json schema.json`.
  - `item_27` reached real schema errors: project objects used `member_ids` where `members` is required, and statistics omitted required camelCase fields such as `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - Runtime inserted `TaskSpaceImplementNeedsEditRecoveryV1` with instructions not to rediscover and to patch the artifact.
  - `item_33`, `item_41`, `item_49`, `item_57`, `item_65`, and `item_73` repeatedly emitted `read_file` for `csv_processor.py`; each executed as `sed -n '1,240p' -- csv_processor.py`.
  - Right rollout trace recorded `main_tool_result` entries such as `taskspace-action-contract-15-read_file` with `actionClass:"read"`, `toolSuccess:true`, and `artifactRefs:[]`.
  - No `validation_rework_duplicate_artifact_read` or `node_policy_violation` event appeared before `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=15/20 node_request_count=6/5`.
- Code-path evidence:
  - `read_command_artifact_ref` recognized PowerShell `Get-Content`, `cat`, and `type`, but did not recognize the Unix action-contract read command shape `sed -n '1,240p' -- path`.
  - `record_main_tool_result_with_class` uses reserved artifact refs for successful `ActionClass::Read`; when reservation refs are empty, `result_artifact_refs` is empty and `implement_node_duplicate_validation_rework_artifact_read` cannot match the previous read.
- Interpretation: This is a feedback-layer identity loss. The validation failure semantic reached the model, but the read result lost the artifact key needed by the runtime control gate.

# Evidence E-046: sed read artifact attribution blocks repeated validation rework reads

- Prediction tested: H-019 requires Unix action-contract read commands to reserve the target artifact and let the existing duplicate rework gate fire on the second read.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused and adjacent commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_needs_edit --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core inspect_data_artifact_read_counts_as_working_evidence --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  git diff --check
  ```
- Result: passed. `validation_rework` remains 11 tests, `validation_` remains 81 tests, and the CLI build completed after the sed attribution repair.
- Matched test signals:
  - `sed -n '1,240p' -- generate_org.py` now records `result_artifact_refs(read_result) == ["generate_org.py"]`.
  - The first named validation rework target read remains allowed before edit.
  - A second read of the same target before a successful edit is rejected with `validation_rework_duplicate_artifact_read`.
  - The recovery message contains the target artifact and `apply_patch`, preserving the existing edit-or-block convergence path.
- Interpretation: The new `implementation-rework-repeat-read-budget-drain` class is focused-fixed. Real utility validation still requires another keyed rerun to verify the model now patches `csv_processor.py` after the first target read instead of draining node budget.

# Hypothesis H-020: manually created validation rework nodes lose blocked-validation origin

- Claim: After H-019, the duplicate rework read gate correctly blocks repeated target reads, but the model can manually create a follow-up `implement_solution` node from a validation recovery context before it records the validation node as blocked. `create_node` defaults the new node dependency to the latest completed implementation node and leaves `origin_node_id` empty. When the validation node is later blocked, the new rework node is not recognized as a legitimate dependency of that blocked validation result, so the lifecycle review gate rejects the next patch with `result still unreviewed` and the run exhausts provider budget.
- Prediction: A keyed rerun after H-019 will show duplicate read feedback firing, a manual `create_node` for a rework implementation node, a blocked validation result left unreviewed, a later `bind_node` for the new rework node, then an `apply_patch` rejection requiring `state_commit` for the blocker result instead of allowing the patch as active validation rework.
- Diagnostic evidence plan: Inspect the H-019 rerun action map for node statuses, edges, result validity, blocked actions, and provider actionability; add a focused runtime test where a validation node creates a detached implementation rework node before blocking, then verify that blocking the validation node marks the rework node ready and allows edit without requiring a separate state_commit for the blocker input.
- Status: confirmed.

# Evidence E-047: blocked validation result is visible but manual rework node is not attached to it

- Prediction tested: H-020 predicts that the previous duplicate-read fix works, and the next failure is origin/dependency loss around a manually created rework node.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703q-postcommit-attestation/runs/terminal_bench__organization-json-generator/20260704-030017-880
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  ```
- Matched trace signals:
  - `item_41` successfully read `process.py` through `sed -n '1,240p' -- process.py`; `result-11` recorded `artifactRef: process.py`, proving H-019's sed read attribution repair was active.
  - `item_48` and `item_55` repeated `read_file process.py`; runtime blocked both with `validation_rework_duplicate_artifact_read`.
  - `item_62` then patched `process.py`, and `node-4` completed with accepted handoff `result-13`.
  - `item_82` manually created `node-6` as `implement_solution` from the validation recovery context; the final graph recorded edge `node-4 -> node-6`, not `node-5 -> node-6`, and `node-6.origin_node_id` was absent.
  - `item_89` blocked `node-5` with `result-14`; `result-14` stayed `unreviewed`.
  - `item_95` bound `node-6`, but `item_102` `apply_patch` was rejected with feedback requiring `taskspace_control(action=state_commit)` for unreviewed `result-14`.
  - The turn then hard-stopped with `TaskSpaceProviderBudgetHardStopV1 reason=provider_request_hard_limit_exceeded request_count=20/20`.
- Interpretation: This is not another artifact attribution failure. The blocked result and its failure text were visible, but the manually created rework node had lost the validation origin needed by `unreviewed_result_is_active_rework_input_blocker`.

# Evidence E-048: manual validation rework keeps origin and can edit using blocker input

- Prediction tested: H-020 requires detached implementation nodes created from an active validation node to keep that validation node as origin, become ready when the origin validation node is blocked, and pass the lifecycle review gate for edits.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused and adjacent commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core manual_validation_rework_created_before_block_keeps_origin --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework` now covers 12 tests and `validation_` now covers 82 tests.
- Matched test signals:
  - A detached `implement_solution` node created while a validation node is active records `origin_node_id` as that validation node and adds an edge from the validation node.
  - When the validation node is blocked, only matching pending validation rework nodes are refreshed to `Ready`; generic blocked nodes still do not unlock downstream work.
  - After binding the manual rework node, `apply_patch` is allowed even though the validation blocker result is still unreviewed, because it is now recognized as active rework input evidence.
- Interpretation: The new `validation-blocker-manual-rework-origin-loss` class is focused-fixed. Real utility validation still requires another keyed rerun to verify the model now patches the rework artifact instead of exhausting budget on state-commit recovery.

# Hypothesis H-021: validation nodes can reuse stale failure blockers without current validation result

- Claim: After H-020, TaskSpace can patch from validation rework, but a new validation node may be blocked with an older validation failure summary before that node records any current test/build result. This lets the model convert stale, still-editable failure text into a terminal validation block and reach graph closeout without rerunning validation after the latest edit.
- Prediction: A keyed rerun after H-020 will show a rework patch, then a new smoke/regression validation node with no same-node test/build result; `block_node` will cite the previous `IndentationError` instead of running the required validator command, leaving the public validator failing while the action map has no open leaf.
- Diagnostic evidence plan: Inspect the post-H-020 keyed rerun pair report and right rollout trace for current validation node result context, blocked reason, test/build result absence, and public validation status; add a focused runtime test proving a smoke/regression node rejects failed-validation blockers until it records a current `Build` or `Test` result, while existing local validator infrastructure blockers still route correctly.
- Status: confirmed.

# Evidence E-049: keyed rerun blocks validation with stale IndentationError before rerunning test

- Prediction tested: H-021 predicts that the next failure after manual rework origin repair is not an origin/lifecycle gate problem but stale validation failure reuse on a fresh validation node.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703r-manual-rework-origin/runs/terminal_bench__organization-json-generator/20260704-032001-321
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 12
  right_open_leaf_nodes: 0
  public_validation_exit_code: 1
  ```
- Matched trace signals:
  - TaskSpace created `generate_org.py` with leading spaces on all top-level lines.
  - Weak generator-only smoke validation was rejected by `validation_test_missing_output_contract_coverage`, then the exact command `python generate_org.py && python -m jsonschema -i organization.json schema.json` failed with `IndentationError: unexpected indent`.
  - Validation rework read `generate_org.py` through `sed -n '1,240p' -- generate_org.py`; the read recorded the target artifact and the next repeated read was blocked by `validation_rework_duplicate_artifact_read`, proving H-019 remained fixed.
  - The model patched only line 1, leaving the remaining top-level leading spaces in the file.
  - The next smoke node (`node-5`) was blocked with the old `IndentationError` text without recording any same-node `Build` or `Test` result.
  - Final readiness rejected an attempted final answer, but the graph had no open leaf and public validation still failed because `organization.json` was not generated.
- Interpretation: The failure semantic was not fully lost; stale failure text was present. The missing control invariant was that a new validation node cannot claim failed validation until that node has actually run a validation tool, except for explicit external/local infrastructure blockers.

# Evidence E-050: validation block guard requires current validation tool evidence

- Prediction tested: H-021 requires smoke/regression `block_node` to reject failed-validation blockers when the current node has no current test/build result, while preserving existing manual local-infrastructure validation blocks.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused and adjacent commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node_rejects_stale_failure_without_current_test --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  git diff --check
  ```
- Result: passed. `validation_` now covers 83 tests and `validation_rework` remains 12 tests.
- Matched test signals:
  - A fresh smoke node with no current validation tool result rejects a blocker that cites `IndentationError` and asks to rerun the required validation command first.
  - A smoke node with a same-node failed validator result can still be blocked.
  - Manual local validator infrastructure blockers such as `Local validator infrastructure failed: PowerShell InvalidEndOfLine. Cannot execute test commands.` still route to validation retry/rework instead of being misclassified as stale failure reuse.
- Interpretation: The new `validation-stale-failure-block-without-current-test` class is focused-fixed. Real utility validation still requires another keyed rerun to verify the model reruns validation after rework, exposes the remaining indentation failure, and continues patching instead of closing the graph.

# Hypothesis H-022: validation rework duplicate-read feedback is diluted by projection after target read

- Claim: After H-021, TaskSpace reruns real schema validation and routes schema failures into implementation rework, but once the model reads the target artifact, the compact projection still advertises `read_file validation rework target artifact ... only if current contents are not visible` and `allowed action classes: read, search, edit, control`. Because the target read result is not surfaced as current critical evidence, the model repeatedly requests the same read, the duplicate-read gate blocks each attempt, and provider node budget is exhausted without an edit.
- Prediction: A keyed rerun after H-021 will show the stale validation block no longer occurring, schema validation reaching real `members`/camelCase statistics failures, one successful target read on the rework node, multiple `validation_rework_duplicate_artifact_read` blocked actions, no successful edit on that rework node, and `provider_node_request_hard_limit_exceeded` with an open leaf.
- Diagnostic evidence plan: Inspect the post-H-021 keyed rerun pair report, right rollout, projection text, and action-map observability; add a focused runtime projection test proving that after a validation rework target read, projection lists the existing result as critical evidence, removes read-file next actions for that target, and narrows allowed actions to edit/control until a successful edit.
- Status: confirmed.

# Evidence E-051: duplicate rework read gate works but projection keeps inviting reads

- Prediction tested: H-022 predicts that the next blocker after stale validation guard is not stale validation closeout; it is projection/recovery conflict after the target artifact is already visible.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703s-stale-validation-guard/runs/terminal_bench__organization-json-generator/20260704-033716-688
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 16
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  ```
- Matched trace signals:
  - TaskSpace read all declared fact sources, generated `process.py`, and ran `python process.py && python -m jsonschema -i organization.json schema.json`.
  - The validation failure reached real schema semantics: project objects used `member_ids` where `members` is required, and statistics used snake_case fields instead of required camelCase fields such as `averageDepartmentBudget`, `totalEmployees`, and `averageYearsOfService`.
  - Runtime routed to validation rework node `node-4`; `item_49` read `process.py` successfully and `result-11` recorded artifact ref `process.py`.
  - `item_56`, `item_63`, `item_70`, `item_77`, and `item_84` repeated `read_file process.py`; each was blocked with `validation_rework_duplicate_artifact_read`.
  - The action-map observability for `node-4` showed five blocked read actions, one successful read result, no successful edit result, and node status still `running`.
  - The final projection still showed `next_valid_actions` containing `read_file validation rework target artifact process.py only if current contents are not visible`, plus noisy line/external refs such as `process.py:63` and the jsonschema CLI file path, while `critical_artifact_evidence` was `none`.
  - The turn ended with `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=17/20 node_request_count=6/5`.
- Interpretation: This is not the old sed attribution issue; artifact identity is present and the duplicate-read gate fires. The unresolved defect is provider-visible feedback conflict: the compact projection does not make the target read result visible as the current contents and continues to advertise read/search as valid after the read.

# Evidence E-052: projection uses existing rework target read and stops advertising reads

- Prediction tested: H-022 requires projection to expose the already-read target result and make `apply_patch` the next action after a validation rework target read.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused and adjacent commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  git diff --check
  ```
- Result: passed. `validation_rework` remains 12 tests and `validation_` remains 83 tests.
- Matched test signals:
  - Before the target read, projection still advertises one named `read_file validation rework target artifact generate_org.py` action.
  - After the target read, projection advertises `use existing validation rework target read result ... for generate_org.py`, no longer advertises `read_file` for that target, and keeps `apply_patch generate_org.py` as the next action.
  - `critical_artifact_evidence` includes the target read excerpt with `signal=validation_rework_target_read`.
  - The current node contract narrows to `edit, control` with an explicit warning that read/search of visible rework targets will be blocked until a successful edit.
- Interpretation: The new `validation-rework-duplicate-read-projection-loop` class is focused-fixed. Real utility validation still requires another keyed rerun to verify the model patches `process.py` after the first target read and then reruns schema/public validation.

# Hypothesis H-023: action-contract validation failure closeout feedback loses the current-test requirement

- Claim: After H-022, TaskSpace can patch after the target read, but if the patch is incomplete the next validation node may still reuse older failure text through action-contract `blocked` or `taskspace_control(action=finish_node, status=failed|reason=...)`. The runtime rejects or fails these actions, yet the session feedback classifies them as generic tool failures instead of a validation-node current-test requirement, so the model repeats block/finish attempts until provider node budget is exhausted.
- Prediction: The post-H-022 keyed rerun will show the new validation node has no same-node `Build`/`Test` result, repeated `blocked` / failed `finish_node` attempts citing an older `IndentationError`, `node-5` still running with no results, and a provider-node hard stop. Source inspection will show `block_main_node` has the current-test guard, while action-contract recent feedback lacks a `validation_stale_failure_without_current_test` class and `finish_node` failure aliases are not consistently normalized to `block_node`.
- Diagnostic evidence plan: Inspect the post-H-022 rerun trace and action-map observability for `node-5` result context and repeated closeout attempts; add focused runtime and session tests proving failed validation closeout without a current test/build result is rejected and fed back as a required `run_test`, not a generic retry.
- Status: confirmed.

# Evidence E-053: action-contract closeout retries exhaust validation node budget without current test evidence

- Prediction tested: H-023 predicts a feedback gap around rejected validation closeout actions, not another target-read projection failure.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703t-rework-target-projection/runs/terminal_bench__organization-json-generator/20260704-035138-996
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_taskspace: engineering_unclean
  outcome_standard: solved
  right_exec_timed_out: False
  right_tool_call_count: 10
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - The H-022 projection fix worked enough for the model to patch `process.py` after the target read; it stopped the duplicate-read loop.
  - The patch only removed leading spaces from the first import block, leaving the file still malformed.
  - `node-5` was a fresh `smoke_test` node with status `running`, no results, and no blocked actions in `action-map-observability.json`.
  - `item_61`, `item_89`, and `item_96` attempted `taskspace_control(action=finish_node, status/result_validity/reason failed...)`; `item_68`, `item_75`, and `item_82` attempted action-contract `blocked`; all cited the older `IndentationError` instead of running a same-node validator.
  - The run ended with `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=19/20 node_request_count=6/5 state=compact_checkpoint_required node_kind=smoke_test`.
- Code-path evidence:
  - `block_main_node` already rejected stale failed-validation blockers without a current test/build result, but session recent-feedback did not classify that rejection as a validation `run_test` requirement.
  - `finish_node` normalization only converted some failed validation shapes; `status=failed`, `result_validity=failed`, and failure text in `reason/result` could flow to generic finish failure handling.
- Interpretation: The stale failure semantic was present, but feedback semantics were incomplete. The model was not told that the only legal recovery was to run validation on the current node before any block/finish/rework transition.

# Evidence E-054: action-contract stale validation closeout feedback now forces current run_test

- Prediction tested: H-023 requires both `block_node` and failed `finish_node` action-contract paths to preserve the current-test requirement.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core finish_validation_node_rejects_stale_failure_without_current_test --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_failed_validation_finish_normalizes_to_block_node --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_current_test_after_stale_validation_block --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_current_test_after_validation_finish_without_result --locked
  ```
- Adjacent regression and build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  git diff --check
  ```
- Result: passed.
- Matched test signals:
  - A validation node cannot be finished as failed validation with stale `IndentationError` text before it records a same-node test/build result.
  - Action-contract `finish_node` with `status=failed` and failure text normalizes to `block_node`, so the same runtime evidence guard handles it.
  - Rejected stale validation closeout is summarized as `failure_kind: validation_stale_failure_without_current_test`.
  - Generic validation `finish_node` without a current test/build result is summarized as `failure_kind: validation_finish_missing_current_test_result`.
  - Recent action-contract feedback now says the next action must be `run_test` and forbids `finish_node`, `block_node`, rework creation, reads, lists, or searches until the current validation result exists.
- Regression signals:
  - `block_validation_node` remains 3/3 passing.
  - `validation_rework` remains 12/12 passing.
  - `validation_` now covers 87 passing tests including the new finish/action-contract feedback paths.
  - `cargo fmt --check`, `codex-cli --bin whale` build, and `git diff --check` all pass.
- Interpretation: The new `validation-stale-failure-action-contract-feedback-gap` class is focused-fixed at the unit level. Real utility validation still requires another keyed rerun to prove the model reruns validation on `node-5` instead of retrying stale failure closeout.

# Hypothesis H-024: validation rework duplicate-read feedback is not action-forcing enough

- Claim: After H-023, TaskSpace correctly forces a fresh validation run and routes schema failures into implementation rework, but the action-contract feedback for rework duplicate reads and generic `implementation_needs_edit` rejects still degrades to broad tool failure/recovery language. The raw state is correct: the target artifact read result and schema failure are present. The missing piece is an explicit provider-visible contract saying the next action must be `apply_patch` against the already-read artifact, with structured validation fields from jsonschema output.
- Prediction: The post-H-023 keyed rerun will show a real `run_test` on the validation node, a blocked schema validation result, a rework node that reads `process.py` once, repeated `validation_rework_duplicate_artifact_read` / `implementation_needs_edit` rejects, no successful edit on the rework node, and provider-node hard stop. Unit repair should add a distinct `validation_rework_duplicate_artifact_read` feedback class, a generic `implementation_needs_edit` feedback class, structured missing-required-property extraction, and no read-file next action in runtime recovery after target contents are visible.
- Diagnostic evidence plan: Inspect the post-H-023 keyed rerun trace and observability; add focused session tests for duplicate rework read and generic implementation-needs-edit feedback, plus runtime tests for jsonschema required-property extraction and post-target-read recovery next actions.
- Status: confirmed.

# Evidence E-055: rework reaches real schema validation but repeats reads instead of patching

- Prediction tested: H-024 predicts the stale validation closeout loop is gone, but implementation rework still fails because duplicate-read feedback is not converted into an action-forcing patch contract.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703u-action-contract-feedback/runs/terminal_bench__organization-json-generator/20260704-041121-187
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 12
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  rollout_trace_model_request_count: 19
  ```
- Matched trace signals:
  - `node-3` ran `python process.py && python -m jsonschema -i organization.json schema.json`; this confirms H-023 worked for the stale closeout class.
  - The validator failed on real schema fields: `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService` were required but missing from `statistics`.
  - Runtime blocked `node-3` and created `node-4` as `implement_solution` rework with blocker text that named the failed validation result.
  - `node-4` successfully read `process.py` once as `result-10`.
  - After that, the model repeated `read_file process.py` and `read_file schema.json`; runtime rejected these with `validation_rework_duplicate_artifact_read` or `implementation_needs_edit`.
  - `node-4` ended `running` with one read result, three blocked duplicate-read actions in observability, no successful edit result, and `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=18/20 node_request_count=6/5`.
- Interpretation: Runtime state was not out of bounds; the issue was feedback-layer actionability. The model had enough evidence to patch `process.py`, but the tool feedback did not preserve a distinct “already-read validation rework artifact; patch now” semantic and did not structure jsonschema required-property failures enough for a compact repair.

# Evidence E-056: duplicate rework read and implementation-needs-edit feedback now force apply_patch

- Prediction tested: H-024 requires rejected duplicate reads and generic implement-needs-edit rejects to be summarized as `apply_patch`-forcing feedback, and jsonschema required-property failures to become compact repair evidence.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_rework_duplicate_read -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_implementation_needs_edit -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_failure_excerpt_extracts_required_property_list -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback -- --nocapture
  ```
- Adjacent regression commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_needs_edit -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  git diff --check
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/write-whale-binary-attestation.ps1 -WhaleBin third_party/codex-cli/codex-rs/target/debug/whale -BuildCommand "CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked"
  ```
- Result: passed.
- Matched test signals:
  - Duplicate rework reads now produce `failure_kind: validation_rework_duplicate_artifact_read`, `target_artifact`, `previous_read_result`, and `next_valid_action: emit exactly one apply_patch`.
  - Generic implementation-needs-edit rejects now produce `failure_kind: implementation_needs_edit` instead of generic tool failure.
  - Recent tool feedback emits progress hints that forbid `read_file`, `list_files`, `search`, schema rediscovery, and implementation-node validation until a successful edit is recorded.
  - `validation_failure_body_excerpt` extracts jsonschema required-property failures into `missing_required_properties: averageDepartmentBudget, totalEmployees, skillDistribution, ...`.
  - Runtime recovery after a visible rework target read now lists the existing read result and `apply_patch`, and no longer re-advertises `read_file validation rework target artifact ...`.
  - `validation_` now covers 88 passing tests including the new required-property summary and feedback paths.
  - `cargo fmt --check`, `codex-cli --bin whale` build, `git diff --check`, and whale binary attestation all pass.
- Interpretation: The new `validation-rework-duplicate-read-action-contract-feedback-gap` class is focused-fixed at the unit level. Real utility validation still requires another keyed rerun to verify the provider patches `process.py` instead of repeating rework reads.

# Hypothesis H-025: failed edit context recovery is blocked by duplicate-read guard

- Claim: After H-024, the provider no longer loops on rework reads and does attempt `apply_patch`, but a failed patch can make the current target context stale or insufficient. The duplicate-read guard still forbids reading the same validation rework target before a successful edit, while edit-failure recovery also says not to read. This turns an editable `IndentationError` into a false blocker: the model claims the source excerpt is truncated and cannot construct a patch.
- Prediction: The post-H-024 keyed rerun will show no duplicate-read loop, at least one `apply_patch` attempt on the rework target, a failed patch such as `apply_patch_unanchored_update` or `Failed to find expected lines`, then a `blocked` result saying the read result was truncated / full file content is needed. Repair should allow a same-target context refresh after a failed edit, reject truncated-source blockers for editable validation failures, and keep unrelated read/search blocked.
- Diagnostic evidence plan: Inspect the post-H-024 keyed rerun trace and observability; add runtime tests proving duplicate reads are blocked before failed edit but same-target refresh is allowed after failed edit, and session tests proving apply_patch expected-lines feedback allows a target context refresh.
- Status: confirmed.

# Evidence E-057: H-024 rerun reaches patch attempt but blocks on truncated source after failed edit

- Harness hygiene note:
  - First attempt under `target/r4-org-json-real-keyed-20260703v-rework-feedback` aborted at whale binary preflight because the binary was built before commit `c9a351f`; preflight correctly reported `whale_binary_stale_for_codex_source` and invalid attestation `codex_source_commit_mismatch`.
  - Lesson recorded: after committing Codex-source changes, rebuild `whale` and rewrite attestation before a keyed benchmark rerun.
- Real rerun after rebuild + attestation:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703w-rework-feedback/runs/terminal_bench__organization-json-generator/20260704-042850-855
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 10
  right_open_leaf_nodes: 0
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  rollout_trace_model_request_count: 18
  ```
- Matched trace signals:
  - H-024 was crossed: `node-4` read `generate_org.py` once and then attempted `apply_patch`; there was no rework duplicate-read loop.
  - The validation failure was a repairable top-level `IndentationError` from leading spaces in `generate_org.py`.
  - First rework patch was rejected by action contract as `apply_patch_unanchored_update:generate_org.py`.
  - The next patch reached the edit tool but failed with `apply_patch verification failed: Failed to find expected lines`.
  - The model then blocked `node-4` with: existing read result was truncated at line 56, missing critical trailing content; full file content is needed to patch.
  - Runtime accepted that blocker and closed `node-4` as blocked even though dependency validation evidence identified an editable implementation failure and no successful edit existed.
- Interpretation: This is a new R4 feedback/capability-layer issue, not a regression in H-024. The system needs to distinguish no-progress duplicate reads from failed-edit context refresh reads.

# Evidence E-058: failed edit can refresh same target context while unrelated reads remain blocked

- Prediction tested: H-025 requires the duplicate-read guard to reset only after a failed edit on the same rework path, and it requires edit-failure feedback to expose that action.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core apply_patch_expected_lines_feedback_allows_target_context_refresh -- --nocapture
  ```
- Adjacent regression and build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib apply_patch_ -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  git diff --check
  ```
- Result: passed.
- Additional non-gating observation:
  - `cargo test ... -p codex-core apply_patch_ -- --nocapture` reaches integration/shell-serialization tests whose existing expectations look for legacy `Exit code: 0 ... Output:` strings while the current harness emits structured JSON `{"output":...,"metadata":...}`. The lib-scoped `--lib apply_patch_` set passed 33/33 and is the relevant regression set for this code change.
- Matched test signals:
  - Before a failed edit, the same validation rework target duplicate read remains blocked.
  - After a failed edit result, projection advertises `read_file validation rework target artifact ... once to refresh context after failed edit`, and action permission allows that same target read.
  - Runtime rejects blockers claiming truncated/missing full file content for editable validation failures.
  - Session feedback for `Failed to find expected lines` now says the next action may be one `read_file` of the same target only if context is truncated/stale, otherwise corrected `apply_patch`.
- Interpretation: The new `failed-edit-context-refresh-blocked-by-duplicate-read-guard` class is focused-fixed at the unit level. Another keyed rerun is required to verify the provider refreshes context, patches indentation, and continues to schema validation.

# Hypothesis H-026: validation schema repair contract is not projected into rework feedback

- Claim: After H-025, the next keyed rerun can reach real schema validation and create rework, but the compact projection and action-contract rejection feedback only say "patch the already-read target"; they do not preserve a short, structured schema repair contract. The failed validator output and earlier `schema.json` read contain enough facts, but those facts are not carried into `critical_artifact_evidence`, `next_valid_actions`, or recent tool feedback in a form that tells the provider exactly which required fields must be satisfied.
- Prediction: The post-H-025 keyed rerun will show `node-3` failing schema validation on `members` plus statistics fields, `node-4` reading the target script once, then repeated `schema.json` / target rediscovery attempts and `validation_rework_duplicate_artifact_read` or `implementation_needs_edit` rejects. Projection will include target read evidence and apply_patch next actions, but not a durable `validation_schema_repair_contract` containing missing required properties and schema-required sibling groups.
- Diagnostic evidence plan: Inspect the post-H-025 keyed trace, action-map observability, and runtime projection output; add focused tests proving schema required-property groups are extracted from already-read `schema.json`, surfaced in projection/recovery, and preserved in session recent tool feedback.
- Status: confirmed.

# Evidence E-059: post-H-025 rerun repeats schema/target rediscovery despite enough schema facts

- Prediction tested: H-026 predicts a feedback-layer contract gap after validation rework, not an edit-refresh failure.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703x-failed-edit-refresh/runs/terminal_bench__organization-json-generator/20260704-044849-474
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 13
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - `node-3` ran `python process_csv_to_json.py && python -m jsonschema -i organization.json schema.json` and failed on schema-required fields.
  - The validator output showed project objects using `member_ids` while schema required `members`, and statistics required `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - `node-4` first attempted `read_file schema.json`; runtime rejected it as `implementation_needs_edit`.
  - `node-4` then successfully read `process_csv_to_json.py` as `result-11`.
  - After the target read, projection said to use existing target result and `apply_patch`, but only exposed target source excerpt under `critical_artifact_evidence`; it did not expose a compact schema repair contract with the missing required fields and schema sibling group.
  - The provider repeated `read_file schema.json` and `read_file process_csv_to_json.py`, causing `implementation_needs_edit` and `validation_rework_duplicate_artifact_read` rejects until `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded node_request_count=6/5`.
- Interpretation: This confirms a feedback-layer semantic absence. The raw validation failure and schema read existed, but the provider-visible contract did not carry the schema repair facts tightly enough for the next edit.

# Evidence E-060: schema repair contract is now projected and fed back after blocked rework reads

- Prediction tested: H-026 requires the runtime projection and session recent tool feedback to carry a structured repair contract.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_projects_schema_repair_contract_from_schema_read --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_rework_duplicate_read --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_implementation_needs_edit --locked
  ```
- Adjacent regression and build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_needs_edit --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework` is now 13/13, `implementation_needs_edit` remains 2/2, and `validation_` is now 89/89.
- Matched test signals:
  - Runtime extracts `missing_required_properties` both from raw jsonschema lines and from prior compact `missing_required_properties:` summaries.
  - Runtime parses already-read `schema.json` JSON and finds required-property groups that contain observed missing fields.
  - A schema failure that only exposes `members` and `averageDepartmentBudget` still projects the full relevant statistics group including `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - `critical_artifact_evidence` now includes `validation_rework_schema_repair signal=validation_schema_repair_contract`.
  - `next_valid_actions`, duplicate-read gate recovery, generic `implementation_needs_edit` recovery, and session `TaskSpaceActionContractRecentToolOutputsV1` now carry `repair_contract` and tell the provider to satisfy it exactly before validation rerun.
- Interpretation: The new `validation-schema-repair-contract-not-projected` class is focused-fixed at unit/regression level. A new keyed rerun is required to verify that `organization-json-generator` now patches schema fields instead of rereading schema/target.

# Hypothesis H-027: validator path pollution and native patch grammar errors are not treated as first-class tool feedback

- Claim: After H-026, the schema repair contract is visible enough for the provider to attempt an edit, but two adjacent tool-link issues still drain the node budget. First, validation failure text can leak validator/runtime file paths such as `/site-packages/jsonschema/__main__.py:4` into rework target artifacts, diluting next valid actions with a non-project path. Second, malformed `apply_patch` payloads that mix native apply_patch grammar with unified diff or placeholder hunk syntax are passed to the edit tool, where they fail as generic expected-line errors instead of returning a precise action-contract correction before tool execution.
- Prediction: A keyed rerun after H-026 will show `validation_schema_repair_contract` present and the provider attempting to patch `process.py`; projection may list a jsonschema runtime path as a target artifact; patch attempts will include `*** Update File` mixed with `--- a/...`, `+++ b/...`, `@@ -...`, or `@@ ... @@`, followed by failed edit results and provider-node hard stop. Repair should filter external validator paths from validation rework targets and reject mixed/placeholder native patch hunks in the action contract with a recovery item that does not consume no-action retry budget.
- Diagnostic evidence plan: Inspect the post-H-026 keyed run trace and observability; add runtime tests proving jsonschema runtime paths are ignored as rework artifacts; add session tests proving mixed native/unified patch syntax and `@@ ... @@` placeholder hunks are rejected before tool execution and projected as edit recovery guidance.
- Status: confirmed.

# Evidence E-061: H-026 rerun reaches schema-contract edit but fails on target pollution and patch grammar

- Prediction tested: H-027 predicts that the schema contract is no longer missing, and the next failure moves to capability/feedback handling around target extraction and edit syntax.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703y-schema-contract/runs/terminal_bench__organization-json-generator/20260704-051107-000
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 13
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - H-026 was crossed: validation rework feedback carried `validation_schema_repair_contract`, and `node-4` stopped rereading schema before its first edit attempt.
  - `node-4` read the target `process.py`, then attempted `apply_patch`.
  - Projection still included a polluted target such as `/home/zhangxu/miniconda3/lib/python3.12/site-packages/jsonschema/__main__.py:4`, which is validator runtime output, not a project artifact.
  - The first patch mixed native grammar and unified/placeholder hunk syntax: `*** Update File: process.py` plus `--- a/process.py`, `+++ b/process.py`, and `@@ ... @@`.
  - The edit tool failed with `apply_patch verification failed: Failed to find expected lines`.
  - A later patch again used stale or placeholder context and failed expected-line matching; the node then hit `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded node_request_count=6/5`.
- Interpretation: This is not a schema contract absence anymore. The feedback gap is that invalid edit syntax and external validator paths were not made first-class action-contract facts before the lower-level tool failure.

# Evidence E-062: validator paths are filtered and native patch grammar is rejected before tool execution

- Prediction tested: H-027 requires both capability-layer target filtering and feedback-layer patch grammar recovery.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework_projects_schema_repair_contract_from_schema_read --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_rejects_mixed_native_unified_patch --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_rejects_native_placeholder_hunk_patch --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_rejects_native_unified_update_hunk_headers --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_rejects_unified_hunk_header_from_add_file --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core apply_patch_native_hunk_recovery_does_not_count_as_no_action_retry --locked
  ```
- Adjacent regression and build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core implementation_needs_edit --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib apply_patch_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework` remains 13/13, `implementation_needs_edit` remains 2/2, lib-scoped `apply_patch_` is 32/32 after updating the old auto-normalization expectations to reject mixed native/unified syntax, and `validation_` remains 89/89.
- Harness hygiene note:
  - Running these tests without `CODEX_SKIP_VENDORED_BWRAP=1` triggered the vendored bubblewrap build and failed on missing local `libcap.pc`; this was an environment precondition failure, not a code regression. Continue using the established `CODEX_SKIP_VENDORED_BWRAP=1` gate for local R4 Rust verification unless libcap development metadata is installed.
- Matched test signals:
  - `validation_failure_text_artifact_refs` ignores external Python/jsonschema runtime paths while retaining real project artifacts such as `generate_org.py`.
  - Mixed native/unified patch payloads are rejected as `apply_patch_mixed_native_unified:<target>` before invoking the edit tool.
  - Native placeholder hunk payloads such as `@@ ... @@` are rejected as `apply_patch_native_hunk_header:<target>` before normalization can hide the bad syntax.
  - `TaskSpaceApplyPatchNativeHunkRecoveryV1` tells the provider to emit exactly one corrected native `apply_patch` or a full Delete/Add replacement, and it does not consume the generic no-action recovery allowance.
  - Pure unified diffs still normalize to native apply_patch; only mixed native/unified or placeholder native hunks are rejected.
- Interpretation: The new `validator-path-target-pollution-and-native-patch-grammar-feedback-gap` class is fixed at unit/regression/build level. Binary attestation and another keyed rerun are required before claiming the external `organization-json-generator` case advances past this edit-syntax layer.

# Hypothesis H-028: empty provider response on an active TaskSpace node is misclassified as final candidate

- Claim: After H-027, a keyed rerun may fail before reaching the patch-feedback layer if the provider returns a reasoning-only / empty assistant response while a TaskSpace node is still running. The session classifier currently treats `saw_actionable_output=false` and `assistant_message_present=false` as `final_candidate` when `needs_follow_up=false`, even though an active node still requires a tool/control/result. This ends the turn with zero tool calls and leaves the node open.
- Prediction: A rerun will show `taskspace_control start_task` creating `node-1`, then a second model request with only reasoning tokens, no assistant message, no actionable output, `response_actionability:final_candidate`, no recovery item, and `tool_call_count=0` / `open_leaf_nodes=1`. Repair should classify this state as `empty_follow_up`, emit `TaskSpaceNoActionRecoveryV1`, and continue sampling.
- Diagnostic evidence plan: Inspect the H-027 rerun trace and request summary; add focused session tests proving empty responses require recovery only when an active TaskSpace node exists, while no-active-node empty responses can remain final candidates.
- Status: confirmed.

# Evidence E-063: H-027 rerun exits after start_task because empty active-node response is accepted

- Prediction tested: H-028 predicts an early control/feedback failure before any ordinary tool call.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703z-native-patch-feedback/runs/terminal_bench__organization-json-generator/20260704-053313-020
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: wrong
  right_exec_timed_out: False
  right_tool_call_count: 0
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - The first model response emitted a valid `taskspace_control(action=start_task)` action and `node-1` was created as `inspect_code_context`.
  - The following provider request returned only reasoning output: `output_tokens=41`, `reasoning_output_tokens=41`.
  - Runtime recorded `response_actionability:final_candidate`, `saw_actionable_output:false`, `assistant_message_present:false`, `recovery_action:none`, with active `node_kind:inspect_code_context`.
  - No `TaskSpaceNoActionRecoveryV1` was inserted, no ordinary tool ran, and the turn ended with `tool_call_count=0`.
- Interpretation: This rerun did not reach H-027's patch syntax layer. It exposed a lower control-loop issue: final-candidate semantics were too permissive while TaskSpace still had active work.

# Evidence E-064: active-node empty responses now trigger no-action recovery

- Prediction tested: H-028 requires empty/no-action responses to recover only when an active TaskSpace node exists.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core provider_response_actionability_treats_empty_active_node_response_as_recovery --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core provider_response_actionability_allows_empty_response_without_active_node_final_candidate --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core provider_response_actionability_ --locked
  ```
- Adjacent regression and build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib apply_patch_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `provider_response_actionability_` is now 8/8, `validation_rework` remains 13/13, lib-scoped `apply_patch_` remains 32/32, and `validation_` remains 89/89.
- Matched test signals:
  - `node_kind=inspect_code_context`, no actionable output, no assistant message, and no terminal action now becomes `empty_follow_up` and requires recovery.
  - The same empty response without an active node still classifies as `final_candidate`.
  - Existing actionability behavior for final-gate rejection, no-action follow-up, gate recovery, actionable responses, and replayable trace recording remains intact.
- Interpretation: The new `active-node-empty-response-final-candidate-misclassification` class is fixed at unit/regression/build level. Another keyed rerun is required before checking whether H-027's edit-syntax layer is now reached.

# Hypothesis H-029: success-criteria output artifacts are not promoted to validation coverage targets

- Claim: After H-028, a keyed rerun can execute tools and close TaskSpace without timeout, but the validation coverage requirement can still be incomplete when the generated output artifact is named only in problem success criteria rather than in the explicit output contract. In the observed run, the output contract said only "Transform CSV data into JSON"; `organization.json` appeared in the success criterion, and `schema.json` appeared in the success criterion/fact source. Runtime extracted the schema target but did not promote `organization.json` to an output validation target, so a weak `json.load(open("organization.json"))` command was accepted as validation.
- Prediction: A post-H-028 keyed rerun will show `python process.py && python -c "... json.load(open('organization.json')) ..."` accepted, forced validation closeout triggered, and public validation failing with schema/public-test fields such as `members` and `averageDepartmentBudget`. Repair should derive generated JSON output targets from `problem_ledger.success_criteria` while keeping schema/validator artifacts in `schema_targets`, then reject weak JSON parse commands with `validation_test_missing_output_contract_coverage` and an exact `python process.py && python -m jsonschema -i organization.json schema.json` recovery.
- Diagnostic evidence plan: Inspect the post-H-028 keyed pair report and runtime trace; add focused runtime tests that record a generic output contract plus a success criterion naming `organization.json` and `schema.json`, then prove weak JSON parsing is rejected while schema-aware validation is allowed.
- Status: confirmed.

# Evidence E-065: H-028 rerun accepts weak JSON parse because success criteria output target is absent

- Prediction tested: H-029 predicts the failure is a validation coverage target gap, not a tool execution failure or empty-response control failure.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703aa-empty-response-recovery/runs/terminal_bench__organization-json-generator/20260704-054010-809
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: wrong
  right_tool_call_count: 8
  right_open_leaf_nodes: 0
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - H-028 was crossed: TaskSpace no longer stopped with zero tools and an open active node.
  - TaskSpace listed/read files, created `process.py`, ran a validation command, and emitted a final answer.
  - The accepted validation command only proved generator execution plus JSON parsing/top-level keys:
    `python process.py && python -c "import json; data = json.load(open('organization.json')); print('Valid JSON: ', list(data.keys()))"`.
  - Output showed only `['metadata', 'organization', 'statistics']`; it did not prove schema/public-test requirements.
  - Runtime forced validation closeout via `TaskSpaceForcedValidationCloseoutV1 trigger=validation_success_after_tool_drain`.
  - Public validation still failed with `KeyError: 'members'` and `KeyError: 'averageDepartmentBudget'`.
  - Code-path inspection showed `task_output_contract_validation_requirements` read schema/validator artifacts from success criteria and fact sources, but only read output targets from explicit output contracts. With a generic output contract description, `requirements.targets` was empty.
- Interpretation: The semantic signal was missing, not merely distorted. The runtime had schema requirements, but the generated output file named in success criteria was absent from validation target coverage, so weak JSON parse appeared sufficient.

# Evidence E-066: success criteria now contribute generated JSON output targets to validation coverage

- Prediction tested: H-029 requires validation requirements to derive generated output targets from the problem ledger success criteria, reject weak JSON parsing, and preserve schema-aware validation recovery.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_node_derives_output_target_from_success_criteria_for_schema_check --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_node_ --locked
  ```
- Adjacent regression command:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_node_` is 21/21 and `validation_` is 90/90.
- Matched test signals:
  - A generic output contract `"Transform CSV data into JSON"` no longer needs to carry the exact file name for coverage to work.
  - The success criterion `"Output file organization.json is generated and follows schema.json"` contributes `organization.json` to output validation targets and `schema.json` to schema targets.
  - CSV/input artifacts are not promoted to generated output targets.
  - `python process.py && python -c "...json.load(open('organization.json'))..."` is rejected with `validation_test_missing_output_contract_coverage`.
  - The recovery text includes the exact command `python process.py && python -m jsonschema -i organization.json schema.json`.
- Interpretation: The new `success-criteria-output-artifact-validation-target-gap` class is focused-fixed at validation runtime/build level. Binary attestation and a keyed rerun are still required before claiming this external sample advances past weak validation closeout.

# Hypothesis H-030: validation rework duplicate-read feedback is diluted by generic implement recovery

- Claim: After H-029, the runtime correctly rejects weak validation and reaches real schema failure, but a follow-up feedback-layer gap can still drain the implement rework node. When `validation_rework_duplicate_artifact_read` blocks a repeated read of the already-read target artifact, the recent tool feedback and projection say to use the previous result and `apply_patch`; however, the session follow-up path then replaces that specific blocked feedback with generic `TaskSpaceImplementNeedsEditRecoveryV1`. The provider keeps retrying `read_file`, and the turn only stops at provider node hard limit.
- Prediction: A keyed rerun after H-029 will show `TaskSpaceValidationNeedsTestRecoveryV1` blocking weak JSON validation, then exact `python ... jsonschema ...` execution, a validation rework node reading the target once, repeated duplicate reads of that same target, generic `TaskSpaceImplementNeedsEditRecoveryV1` advisory attempts, and finally `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5`. Repair should preserve a dedicated `TaskSpaceValidationReworkDuplicateReadRecoveryV1` marker with target artifact, previous read result, repair contract, and patch-only next action, and it should prioritize that marker over generic implement-needs-edit recovery.
- Diagnostic evidence plan: Inspect the post-H-029 keyed trace, active projection, and session recovery messages; add focused session tests proving duplicate validation rework reads produce a dedicated patch-only recovery and that implementation recovery selection prioritizes it.
- Status: confirmed.

# Evidence E-067: H-029 rerun crosses weak validation but drains budget on duplicate rework reads

- Prediction tested: H-030 predicts H-029 is fixed and the next failure is recovery dilution after validation rework target read.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ab-schema-success-target/runs/terminal_bench__organization-json-generator/20260704-055155-897
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - H-029 was crossed: weak validation `python generate_org.py && python -m json.tool organization.json > /dev/null` was blocked because declared output contract artifacts required schema/validator coverage.
  - The provider then executed the exact schema command: `python generate_org.py && python -m jsonschema -i organization.json schema.json`.
  - The validator returned real schema defects: project objects used `member_ids` while `members` was required; statistics missed `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - `node-4` read the target `generate_org.py` once as `result-11`.
  - Active projection was correct and patch-only: `use existing validation rework target read result result-11`, `apply_patch validation rework target artifact(s): generate_org.py`, and `read/search is no longer a valid next action`.
  - The provider repeated `read_file generate_org.py` four times and then `read_file schema.json`.
  - Each target repeat was blocked as `validation_rework_duplicate_artifact_read`, but session recovery emitted generic `TaskSpaceImplementNeedsEditRecoveryV1` advisory attempts 3 through 7.
  - The turn ended with `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded node_request_count=6/5`.
- Interpretation: The feedback was not absent at the runtime/projection layer; it was diluted in the session recovery layer after action-contract rejection. This is a feedback-layer priority bug.

# Evidence E-068: duplicate validation rework reads now get dedicated patch-only recovery

- Prediction tested: H-030 requires session recovery to preserve duplicate-read semantics instead of falling back to generic implement-needs-edit guidance.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_duplicate_read_recovery_preserves_patch_only_contract --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_duplicate_read --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_prioritizes_duplicate_rework_read_feedback --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_rework_duplicate_read --locked
  ```
- Adjacent regression command:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_needs_edit --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework_duplicate_read` is 2/2, `implementation_needs_edit` is 2/2, `validation_rework` is 14/14, and `validation_` is 91/91.
- Matched test signals:
  - New `TaskSpaceValidationReworkDuplicateReadRecoveryV1` preserves `failure_kind=validation_rework_duplicate_artifact_read`, `target_artifact`, `previous_read_result`, `repair_contract`, and the embedded `TaskSpaceGateRecoveryV1`.
  - The recovery requires exactly one `apply_patch` targeting the already-read artifact or one exact `block_node`; it forbids `read_file`, `list_files`, `search`, schema inspection, and validation before a successful edit.
  - Implementation recovery selection prioritizes duplicate rework read feedback over generic `TaskSpaceImplementNeedsEditRecoveryV1`.
  - Existing action-contract recent feedback for duplicate reads still exposes `target_artifact`, `previous_read_result`, and `repair_contract`.
- Interpretation: The new `validation-rework-duplicate-read-recovery-dilution` class is focused-fixed at session feedback/build level. Binary attestation and another keyed rerun are required before claiming the external sample advances from patch-only recovery into a corrected edit.

# Hypothesis H-031: immediate actionability recovery bypasses duplicate-read recovery selection

- Claim: The first H-030 fix added a dedicated duplicate-read recovery selector, but one live session path still bypassed it. When a blocked tool action is classified by `response_actionability.needs_recovery()` inside the response-completed path, the implementation branch directly calls generic `build_taskspace_implement_needs_edit_recovery_item` instead of the newer selector. This means the gate feedback can correctly say "validation rework artifact already read; patch now", while the next developer recovery message still becomes generic `TaskSpaceImplementNeedsEditRecoveryV1`.
- Prediction: The keyed rerun after E-068 will still cross weak validation and schema validation, then show repeated validation rework target reads blocked by the gate. The actionability preview will contain natural-language duplicate-read feedback, but the following recovery warnings will remain generic `TaskSpaceImplementNeedsEditRecoveryV1` until the provider node hard limit. Repair should route the immediate actionability implementation recovery through `build_taskspace_implementation_recovery_item`, preserve the duplicate-read marker even when the stable reason string is absent, and log a distinct `TaskSpaceValidationReworkDuplicateReadRecoveryV1` warning.
- Diagnostic evidence plan: Inspect the post-E-068 keyed rerun trace, add a focused selector test using the real natural-language blocked-read text without the stable reason field, and patch the immediate response-actionability recovery branch.
- Status: confirmed.

# Evidence E-069: E-068 fix was bypassed by the immediate actionability branch

- Prediction tested: H-031 predicts H-030's dedicated recovery exists but is not used by the live immediate recovery path.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ac-duplicate-read-recovery/runs/terminal_bench__organization-json-generator/20260704-060538-444
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - The run crossed H-029 again: weak validation was rejected and the exact schema validation command was executed.
  - `node-4` entered implementation rework and read `generate.py` once.
  - Repeated reads of `generate.py` were blocked with feedback text: `validation rework node ... already read failure artifact ... no successful edit has been recorded`.
  - Despite that blocked-read semantics, the next recovery warnings were generic `TaskSpaceImplementNeedsEditRecoveryV1` advisory attempts 5, 6, 7, 8, and 9.
  - The turn stopped at `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=17/20 node_request_count=6/5`.
- Interpretation: The duplicate-read semantics were not missing in the gate; they were lost because the response-actionability branch did not use the specialized implementation recovery selector. This is a session feedback routing bug, not a model ability failure.

# Evidence E-070: immediate implementation recovery now uses the duplicate-read selector

- Prediction tested: H-031 requires the live actionability recovery branch and warning text to use the same specialized selector as the post-drain fallback path.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused command:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked implementation_needs_edit
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. The `duplicate` filtered suite ran 21 tests including the new `implementation_recovery_selects_duplicate_rework_from_gate_text_without_reason`; `implementation_needs_edit` is 2/2, `validation_rework` is 14/14, and `validation_` is 91/91. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - The selector recognizes the real blocked-read wording even when `TaskSpaceGateRecoveryV1.reason=validation_rework_duplicate_artifact_read` is absent from the visible text.
  - The generated recovery contains `TaskSpaceValidationReworkDuplicateReadRecoveryV1`, `failure_kind: validation_rework_duplicate_artifact_read`, `target_artifact: generate.py`, `previous_read_result: result-11`, and the repair contract.
  - The recovery does not contain the generic `TaskSpaceImplementNeedsEditRecoveryV1` marker.
  - `taskspace_special_recovery_warning_message` now logs `TaskSpaceValidationReworkDuplicateReadRecoveryV1` distinctly instead of falling through to a generic recovery warning.
- Operational note: a bare `cargo test -p codex-core ...` failed in this host because `codex-linux-sandbox` tried to build vendored bubblewrap and `libcap.pc` is unavailable. The stable local test command for this repository remains `CODEX_SKIP_VENDORED_BWRAP=1 cargo test ...`.
- Interpretation: The new `validation-rework-duplicate-read-immediate-recovery-bypass` class is focused-fixed at session feedback level. A binary rebuild/attestation and another keyed rerun are required to verify the external sample now receives the dedicated patch-only recovery in the live trace.

# Hypothesis H-032: patch grammar recovery is diluted after mixed native/unified rejection

- Claim: After H-031, duplicate validation rework reads receive the dedicated patch-only recovery in the live trace, but a neighboring apply_patch feedback class still drains the implementation rework node. `apply_patch_mixed_native_unified:<target>` is rejected by the action contract, but the advisory warning path labels the recovery as generic `TaskSpaceImplementNeedsEditRecoveryV1`. Then a later duplicate-read recovery says "patch now" without restating native apply_patch grammar, so the provider repeats unified-diff headers inside native `*** Update File` payloads until the node hard limit.
- Prediction: The post-H-031 keyed rerun will show `TaskSpaceValidationReworkDuplicateReadRecoveryV1` after a duplicate read, proving H-031 crossed. The next blocker will be repeated `apply_patch_mixed_native_unified:generate_org.py` rejections followed by generic implement-needs-edit warnings and `TaskSpaceProviderBudgetHardStopV1`. Repair should classify advisory warnings for `TaskSpaceApplyPatchNativeHunkRecoveryV1` distinctly, and duplicate-read recovery should preserve native apply_patch grammar constraints so it does not erase the previous patch grammar rejection.
- Diagnostic evidence plan: Inspect the post-H-031 keyed rerun trace and pair report; add focused tests for mixed-native/unified advisory warning and duplicate-read recovery grammar preservation; run apply_patch and validation rework regression filters.
- Status: confirmed.

# Evidence E-071: H-031 rerun crosses duplicate-read recovery and exposes patch grammar dilution

- Prediction tested: H-032 predicts the dedicated duplicate-read recovery appears in live trace, and the next failure moves to apply_patch grammar recovery.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ad-immediate-recovery/runs/terminal_bench__organization-json-generator/20260704-061808-358
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - H-031 crossed: repeated `read_file generate_org.py` on validation rework was blocked, and the next warning was `TaskSpaceValidationReworkDuplicateReadRecoveryV1`.
  - The provider's first rework patch used mixed native/unified syntax and was rejected as `apply_patch_mixed_native_unified:generate_org.py`.
  - The advisory warning logged generic `TaskSpaceImplementNeedsEditRecoveryV1` after the mixed patch rejection.
  - After the duplicate-read recovery, the provider emitted another mixed native/unified patch, again rejected as `apply_patch_mixed_native_unified:generate_org.py`.
  - The turn ended with `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=17/20 node_request_count=6/5`.
- Interpretation: The duplicate-read feedback fix is live, but patch grammar feedback is still diluted in advisory observability and can be overwritten by later recovery text. This is a feedback-layer routing and preservation bug, not an inability to detect invalid patch syntax.

# Evidence E-072: mixed native/unified patch recovery now preserves native grammar semantics

- Prediction tested: H-032 requires mixed native/unified patch rejection to keep a dedicated advisory label, and duplicate-read recovery to carry native apply_patch grammar constraints.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked native_hunk
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework_duplicate
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked apply_patch_
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `native_hunk` is 3/3, `duplicate_rework` is 2/2, `validation_rework_duplicate` is 2/2, `apply_patch_` is 33/33, `validation_rework` is 14/14, and `validation_` is 91/91. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - `apply_patch_mixed_native_unified:generate_org.py` now maps to `TaskSpaceApplyPatchNativeHunkRecoveryV1` advisory warning instead of `TaskSpaceImplementNeedsEditRecoveryV1`.
  - The native hunk recovery tells the provider not to call `read_file` and to re-emit exactly one native apply_patch.
  - `TaskSpaceValidationReworkDuplicateReadRecoveryV1` now restates native apply_patch grammar and explicitly forbids `--- a/...`, `+++ b/...`, range hunks, and placeholder hunks.
- Interpretation: The `apply-patch-native-hunk-recovery-dilution` class is focused-fixed at session feedback text/observability/build level. It still needs commit/push, binary attestation, and another keyed rerun to prove the external sample advances beyond patch grammar.

# Hypothesis H-033: dash-native apply_patch headers bypass native hunk recovery

- Claim: After H-032, the provider can still emit a malformed native-looking patch that starts with `--- Update File: <path>` rather than `*** Update File: <path>`. This is neither a standard unified diff (`--- a/...` followed by `+++ b/...`) nor a valid native apply_patch operation, so the current action contract misses it and lets the edit tool fail. The following session recovery becomes generic `TaskSpaceEditFailureRecoveryV1`, which can allow a same-target read refresh and drain the rework node budget instead of forcing a corrected native patch.
- Prediction: The post-H-032 keyed rerun will show `TaskSpaceValidationReworkDuplicateReadRecoveryV1`, then an `apply_patch` payload beginning with `--- Update File: generate_organization.py` and a placeholder/range hunk such as `@@ -... +@@ ... @@`. The trace will not show `TaskSpaceApplyPatchNativeHunkRecoveryV1`; instead it will insert `TaskSpaceEditFailureRecoveryV1`, accept a subsequent `read_file generate_organization.py`, and end at `TaskSpaceProviderBudgetHardStopV1`.
- Diagnostic evidence plan: Inspect the post-H-032 keyed rerun trace for the malformed patch payload, the exact recovery marker, and whether a duplicate target read is executed after edit-failure recovery. Add a focused action-contract test using the live `--- Update File` patch shape. Repair should reject this payload before tool execution as `apply_patch_native_hunk_header:<target>` and preserve `TaskSpaceApplyPatchNativeHunkRecoveryV1` guidance.
- Status: confirmed.

# Evidence E-073: H-032 rerun exposes dash-native header feedback gap

- Prediction tested: H-033 predicts the mixed native/unified feedback class is improved, but a different malformed native-looking header bypasses pre-dispatch recovery.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ae-patch-grammar-recovery/runs/terminal_bench__organization-json-generator/20260704-063554-000
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - Lines 61, 68, and 75 block repeated `read_file generate_organization.py` actions, and lines 62, 69, and 76 insert `TaskSpaceValidationReworkDuplicateReadRecoveryV1`, proving the duplicate-read recovery remains live.
  - Line 80 emits `apply_patch` with `--- Update File: generate_organization.py` plus `@@ -... +@@ ... @@`, a malformed native header and placeholder/range hunk.
  - Line 82 still classifies the response as actionable.
  - Lines 83 and 92 insert generic `TaskSpaceEditFailureRecoveryV1`; no `TaskSpaceApplyPatchNativeHunkRecoveryV1` appears.
  - Lines 87-89 execute another `read_file generate_organization.py` after the failed edit, then line 93 ends at `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded node_request_count=6/5`.
- Interpretation: This is a feedback/action-contract coverage gap. The runtime can detect and recover duplicate rework reads, but the malformed `--- Update File:` variant is not classified before tool execution, so the failure semantics are weakened to generic edit failure and the loop regains a read path.

# Evidence E-074: dash-native header is rejected before tool execution

- Prediction tested: H-033 requires the action contract to classify `--- Update File:` as native patch grammar failure instead of dispatching it to the edit tool.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused command:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked dash_native
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked native_hunk
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. The new `taskspace_action_contract_rejects_dash_native_update_header_patch` test rejects the live patch shape as `apply_patch_native_hunk_header:generate_organization.py`. `native_hunk` is 3/3, `apply_patch_` is 33/33, `validation_rework` is 14/14, `validation_` is 91/91, and `duplicate_rework` is 2/2. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - `--- Update File: generate_organization.py` no longer reaches `apply_patch` tool execution.
  - The native hunk recovery text explicitly forbids `--- Update File:` alongside `--- a/...`, `+++ b/...`, range hunks, and placeholder hunks.
- Interpretation: The `apply-patch-dash-native-header-feedback-gap` class is focused-fixed at action-contract classification/build level. Commit/push, binary attestation, and another keyed rerun are still required before claiming live external convergence beyond this malformed-header case.

# Hypothesis H-034: duplicate-read recovery drops recent patch grammar failure after NativeHunk recovery

- Claim: After H-033, action-contract classification and NativeHunk advisory labels are correct, but a later duplicate validation-rework read can overwrite the recent patch grammar failure context. `build_taskspace_implementation_recovery_item` prioritizes `TaskSpaceValidationReworkDuplicateReadRecoveryV1` over `failed_edit_summary`, so if the provider calls `read_file` immediately after `TaskSpaceApplyPatchNativeHunkRecoveryV1`, the next duplicate-read recovery no longer carries `apply_patch_mixed_native_unified:<target>` / `apply_patch_native_hunk_header:<target>` feedback. The node then reaches its hard limit with the most specific edit failure semantics no longer visible in the final recovery.
- Prediction: The post-H-033 keyed rerun will show `apply_patch_mixed_native_unified:generate_org.py` rejected before tool execution and `TaskSpaceApplyPatchNativeHunkRecoveryV1` inserted, proving H-032/H-033 crossed. If this hypothesis is true, the provider then repeats `read_file generate_org.py`, TaskSpace inserts ordinary `TaskSpaceValidationReworkDuplicateReadRecoveryV1`, and the turn ends at `TaskSpaceProviderBudgetHardStopV1` without a recovery warning that preserves the failed patch grammar.
- Diagnostic evidence plan: Inspect the post-H-033 keyed rerun trace around the NativeHunk rejection and subsequent duplicate read. Add focused tests proving duplicate-read recovery preserves a recent failed patch grammar summary and emits a distinct advisory warning when both semantics are present.
- Status: confirmed.

# Evidence E-075: H-033 rerun crosses dash-native gap and exposes duplicate-read/patch-grammar preservation gap

- Prediction tested: H-034 predicts H-033 is fixed live, then the next failure is recovery semantic loss after NativeHunk recovery.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703af-dash-native-header/runs/terminal_bench__organization-json-generator/20260704-064858-360
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 13
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - Preflight used git head `fa9a5d2b9ad9514b4fb2ca31ccadef265e07b5e4` with passing binary attestation.
  - Lines 53, 60, and 67 block duplicate `read_file generate_org.py`; lines 54, 61, and 68 insert `TaskSpaceValidationReworkDuplicateReadRecoveryV1`.
  - Line 72 emits a mixed native/unified patch with `*** Update File: generate_org.py`, `--- a/generate_org.py`, `+++ b/generate_org.py`, and range hunks.
  - Line 74 rejects it as `TaskSpaceActionV1 rejected: apply_patch_mixed_native_unified:generate_org.py`.
  - Line 75 correctly inserts `TaskSpaceApplyPatchNativeHunkRecoveryV1`, so H-032/H-033 crossed.
  - Line 79 repeats `read_file generate_org.py`; line 82 inserts ordinary `TaskSpaceValidationReworkDuplicateReadRecoveryV1`, and line 83 hard-stops with `provider_node_request_hard_limit_exceeded node_request_count=6/5`.
- Interpretation: The apply_patch grammar failure is detected and labeled correctly, but the next duplicate-read recovery does not preserve that failed edit summary. This is a recovery-composition bug between duplicate-read feedback and patch-grammar feedback, not a missing patch parser.

# Evidence E-076: duplicate-read recovery now preserves recent patch grammar failure

- Prediction tested: H-034 requires duplicate-read recovery to keep the recent failed edit summary when the blocked read follows a patch grammar rejection.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked failed_patch_grammar
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework
  ```
- Adjacent regression commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked native_hunk
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `failed_patch_grammar` is 2/2, `duplicate_rework` is 2/2, `validation_rework` is 15/15, `native_hunk` is 3/3, `apply_patch_` is 33/33, and `validation_` is 92/92. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - Duplicate-read recovery includes `Most recent failed edit feedback to preserve` with `apply_patch_mixed_native_unified:<target>` or `apply_patch_native_hunk_header:<target>`.
  - The recovery explicitly says patch grammar must be corrected now and `read_file/context refresh is not a valid recovery` for that failure.
  - Advisory observability emits `TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1` when duplicate-read and patch-grammar semantics are both present.
- Interpretation: The `validation-rework-duplicate-read-after-patch-grammar-feedback-loss` class is focused-fixed at session recovery composition/build level. Commit/push, binary attestation, and another keyed rerun are still required before claiming live progress beyond the NativeHunk recovery follow-up.

# Hypothesis H-035: mixed native/unified apply_patch rejection should normalize safely convertible patches

- Claim: After H-034, patch grammar feedback is preserved and correctly labeled, but a remaining capability-layer boundary rejects mechanically convertible `apply_patch` payloads before normalization. The provider repeatedly emits `*** Update File: <path>` with adjacent `--- a/<path>` / `+++ b/<path>` headers and range hunks. Those payloads are structurally sufficient to convert into native apply_patch grammar, but the action contract rejects them as `apply_patch_mixed_native_unified:<target>` before the existing normalizer can strip unified file headers and normalize range hunk headers. This drains the rework node even though the tool boundary could safely repair the payload shape.
- Prediction: The post-H-034 keyed rerun will show `TaskSpaceApplyPatchNativeHunkRecoveryV1` repeatedly after `apply_patch_mixed_native_unified:csv2json.py`, proving feedback is not missing. The same trace will show the patch payload contains a valid target path, old/new unified file headers, and concrete range hunks, not placeholder `@@ ... @@` hunks or malformed `--- Update File:` headers. Code inspection will show mixed native/unified detection occurs before `normalize_taskspace_unified_diff_patch` / `normalize_taskspace_apply_patch`.
- Diagnostic evidence plan: Inspect the post-H-034 trace for repeated mixed native/unified rejections and NativeHunk recovery markers; inspect `taskspace_action_to_tool_call` ordering; add focused tests using the live wrapped and unwrapped `csv2json.py` patch shapes. Repair should keep rejecting malformed dash-native operation headers and placeholder hunks, but normalize safe mixed native/unified payloads before final validation.
- Status: confirmed.

# Evidence E-077: H-034 rerun exposes safe mixed patch normalization gap

- Prediction tested: H-035 predicts patch feedback is delivered, but action dispatch rejects a safely convertible mixed patch before normalization.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ag-duplicate-read-after-grammar/runs/terminal_bench__organization-json-generator/20260704-070452-184
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 10
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  ```
- Matched trace signals:
  - Line 50 records the actionable validation failure: `IndentationError: unexpected indent` in `csv2json.py` line 2.
  - Lines 66, 73, 80, 87, and 94 emit `apply_patch` payloads for `csv2json.py` containing `*** Update File`, `--- a/csv2json.py`, `+++ b/csv2json.py`, and concrete range hunks such as `@@ -1,2 +1,2 @@`.
  - Lines 68, 75, 82, 89, and 96 reject those payloads as `TaskSpaceActionV1 rejected: apply_patch_mixed_native_unified:csv2json.py`.
  - Lines 69, 76, 83, 90, and 97 insert `TaskSpaceApplyPatchNativeHunkRecoveryV1`, so the feedback semantics are present and not diluted.
  - Line 98 hard-stops with `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded node_request_count=6/5`.
- Code-path signal:
  - `taskspace_action_to_tool_call` checked `taskspace_apply_patch_mixed_native_unified_targets(&patch)` before calling `normalize_taskspace_unified_diff_patch(&patch).unwrap_or_else(|| normalize_taskspace_apply_patch(&patch))`.
  - `normalize_taskspace_native_hunk_headers` already knows how to strip adjacent `---` / `+++` headers inside native update sections and normalize range hunk lines to native `@@`.
- Interpretation: This is not a feedback-layer loss in this concrete case. The provider received the correct failure and recovery marker repeatedly. The remaining root cause is a capability-layer policy boundary that treats a safely normalizable mixed patch as fatal instead of canonicalizing it before dispatch.

# Evidence E-078: safe mixed native/unified patches normalize before dispatch

- Prediction tested: H-035 requires safely convertible mixed native/unified patches to reach the existing normalizer before mixed-patch rejection, while malformed dash-native operations and placeholder hunks remain rejected before tool execution.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked mixed_native
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked native_unified_update
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked unified_hunk_header_from_add
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked native_hunk
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked dash_native
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `mixed_native` is 4/4, `native_unified_update` is 1/1, `unified_hunk_header_from_add` is 1/1, `native_hunk` is 3/3, `dash_native` is 1/1, `apply_patch_` is 33/33, `validation_rework` is 15/15, `validation_` is 92/92, and `duplicate_rework` is 2/2. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - Live wrapped trace shape `*** Begin Patch` + `*** Update File: csv2json.py` + `--- a/csv2json.py` / `+++ b/csv2json.py` + `@@ -1,2 +1,2 @@` normalizes to one native `*** Update File` section with native `@@`.
  - Live unwrapped trace shape starting at `*** Update File: csv2json.py` also normalizes by adding `*** Begin Patch` / `*** End Patch`, stripping unified file headers, and normalizing range hunk headers.
  - `@@ ... @@` placeholder hunk still rejects as `apply_patch_native_hunk_header:<target>`.
  - `--- Update File: <target>` still rejects as `apply_patch_native_hunk_header:<target>`.
- Operational note: from repository root, stable Rust commands must use `--manifest-path third_party/codex-cli/codex-rs/Cargo.toml`; a bare `cargo test -p codex-core ...` fails because the project root is not a Cargo workspace root.
- Interpretation: The `apply-patch-mixed-native-unified-auto-normalization-gap` class is focused-fixed at action-contract normalization/build level. Commit/push, binary attestation, and another keyed rerun are still required before claiming live external convergence beyond this case.

# Evidence E-079: H-035 rerun crosses mixed patch normalization and exposes non-diff update payload gap

- Prediction tested: H-035 fix should remove the `apply_patch_mixed_native_unified` / `TaskSpaceApplyPatchNativeHunkRecoveryV1` loop from the live trace.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ah-mixed-normalization/runs/terminal_bench__organization-json-generator/20260704-071947-777
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: 5a29b811a077776ae2d31ee6741aa8c775a89ee5
  build_attestation_status: pass
  ```
- Matched trace signals:
  - No `apply_patch_mixed_native_unified` rejection appears in the rerun trace, and no `TaskSpaceApplyPatchNativeHunkRecoveryV1` loop appears.
  - The task advanced into schema validation: line 41 ran `python -m jsonschema -i organization.json schema.json`.
  - Line 43 reported real schema failures including missing project `members` and missing statistics keys such as `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - Lines 50-58 show duplicate reads of already-read `organization.json` are still blocked and receive `TaskSpaceValidationReworkDuplicateReadRecoveryV1`.
- Interpretation: H-035 is live-crossed. The remaining blocker is a new apply_patch payload-class issue, not mixed native/unified grammar.

# Hypothesis H-036: non-diff Update File payloads bypass action-contract grammar feedback

- Claim: In validation rework, the provider can emit `apply_patch` with `*** Update File: <path>` followed by shell/Python command text instead of native diff hunk content. The current action contract only rejects add-only unanchored updates when it sees `+...` lines without context, so command payloads with no `+` / `-` change lines reach the `apply_patch` tool. Tool failure then degrades to generic `TaskSpaceEditFailureRecoveryV1`, which can reopen read attempts and drain the node budget.
- Prediction: The H-035 rerun will show an `apply_patch` payload with `*** Update File: organization.json` followed by `python3 -c` script text and no `@@`, `-old`, or `+new` hunk. The trace will mark that response actionable, then insert generic `TaskSpaceEditFailureRecoveryV1` rather than `TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1`. Repair should reject such payloads before tool execution as `apply_patch_unanchored_update:<target>` and should keep deletion-only native updates valid.
- Diagnostic evidence plan: Inspect the H-035 rerun trace around the failed edit; add focused tests for the live `python3 -c` payload and for deletion-only native update. Expand the unanchored update detector to flag `*** Update File` sections with content but no native diff change lines.
- Status: confirmed.

# Evidence E-080: non-diff Update File payloads now reject before apply_patch tool execution

- Prediction tested: H-036 requires command/text payloads inside `*** Update File` to be caught by the action contract, while valid deletion-only update hunks remain allowed.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked non_diff_update_payload
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked unanchored_update
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked delete_only_update
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `non_diff_update_payload` is 1/1, `unanchored_update` is 2/2, `delete_only_update` is 1/1, `apply_patch_` is 33/33, `validation_rework` is 15/15, `validation_` is 92/92, and `duplicate_rework` is 2/2. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - The live `*** Update File: organization.json` + `python3 -c` payload now rejects as `apply_patch_unanchored_update:<target>` before tool execution.
  - The recovery text now says not to put shell, Python, or JSON transformation commands inside apply_patch payloads.
  - A deletion-only patch with `@@` and `-old` remains a valid `apply_patch` tool call.
- Interpretation: The `apply-patch-non-diff-update-payload-feedback-gap` class is focused-fixed at action-contract/recovery-text/build level. Commit/push, binary attestation, and another keyed rerun are still required before claiming live convergence beyond this case.

# Evidence E-081: post-H-036 rerun exposes Python Add File common-indent capability gap

- Prediction tested: H-036 direct payload class should no longer be the only known blocker after commit/push/attestation; the next rerun should identify the next tools-chain issue if the sample still fails.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ai-non-diff-patch/runs/terminal_bench__organization-json-generator/20260704-073032-022
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 15
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: 711b69e577f8b21033abe1b1ebeeecf0b9982160
  build_attestation_status: pass
  ```
- Matched trace signals:
  - Line 29 created `generate_org_json.py` with native `*** Add File`, but every non-empty Python content line was emitted as `+ import ...`, `+ def ...`, etc., meaning the created file has one unintended leading space on every line.
  - Line 43 ran the exact validation command `python generate_org_json.py && python -m jsonschema -i organization.json schema.json`, and Python failed at line 1 with `IndentationError: unexpected indent`.
  - Line 50 executed the first allowed `read_file generate_org_json.py`, and the output confirmed the entire file carries a common one-space prefix.
  - Lines 58, 65, 72, 79 repeat `TaskSpaceValidationReworkDuplicateReadRecoveryV1`; the feedback layer is present but the model keeps rereading instead of patching until line 80 hard-stops.
- Interpretation: This is a capability-layer normalization gap at the edit boundary. The patch payload contains a common LLM-native `+ ` spacing mistake for a new Python file; treating it as literal source creates invalid Python and forces a rework loop. A safe narrow normalizer can strip one shared leading space only for Python Add File sections where every non-empty added line has that extra space.

# Hypothesis H-037: Python Add File payloads with common extra leading space should normalize

- Claim: When a native `*** Add File: *.py` section has every non-empty added content line starting with exactly at least one space after `+`, the provider likely used `+ ` as patch-list formatting rather than intended module-level indentation. For Python files, a whole-file top-level one-space indent is syntactically invalid and repeatedly leads to `IndentationError`. The edit boundary should strip one common leading space from every added Python content line, while preserving relative indentation and leaving non-Python files or mixed-indent Python content untouched.
- Prediction: A focused test using the live `generate_org_json.py` shape should normalize `+ import csv` to `+import csv` and `+     print(...)` to `+    print(...)`. A non-Python Add File with leading spaces should be preserved.
- Diagnostic evidence plan: Add normalizer tests for Python Add File common indent and non-Python preservation; route all native apply_patch postprocessing through the new normalizer; run apply_patch and validation rework regression filters.
- Status: confirmed.

# Evidence E-082: Python Add File common leading indent now normalizes narrowly

- Prediction tested: H-037 requires Python Add File common leading indentation to be stripped only when all non-empty added lines share the extra leading space.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked common_python_add_file_indent
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked non_python_add_file_indent
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `common_python_add_file_indent` is 1/1, `non_python_add_file_indent` is 1/1, `apply_patch_` is 35/35, `validation_rework` is 15/15, `validation_` is 92/92, and `duplicate_rework` is 2/2. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - Python `*** Add File: generate_org_json.py` normalizes `+ import csv` to `+import csv`.
  - Relative indentation is preserved by removing only one shared leading space, e.g. `+     print('ok')` becomes `+    print('ok')`.
  - Non-Python `*** Add File: notes.txt` keeps leading spaces unchanged.
- Interpretation: The `python-add-file-common-indent-normalization-gap` class is focused-fixed at apply_patch normalization/build level. Commit/push, binary attestation, and another keyed rerun are required before claiming live convergence beyond this case.

# Evidence E-083: H-037 rerun crosses Python Add File indentation and exposes anchored placeholder hunk rejection

- Prediction tested: H-037 fix should prevent first-line Python `IndentationError` from common Add File indentation.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703aj-python-add-indent/runs/terminal_bench__organization-json-generator/20260704-073958-389
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: a4abc1e199de8f1ab12ba7e9c18fe8552c14dbdf
  build_attestation_status: pass
  ```
- Matched trace signals:
  - No line-1 `IndentationError` appears. Line 57 shows `organization.json generated successfully.` before schema validation failures, proving the generated Python script executed.
  - The remaining schema errors are semantic output mismatches: skills are strings rather than arrays, projects use `member_ids` instead of `members`, and statistics use snake_case / alternate names rather than required camelCase fields.
  - Line 66 reads `generate_json.py` once and shows the file has no common top-level leading indentation, so H-037 crossed.
  - Lines 76, 83, 90, and 97 still show duplicate-read recovery after repeated re-reads.
  - Line 101 finally emits a targeted patch, but the hunk uses mixed native/unified headers plus `@@ ... @@` placeholder with a real anchor context line.
  - Line 103 rejects it as `apply_patch_native_hunk_header:generate_json.py`, line 104 inserts `TaskSpaceApplyPatchNativeHunkRecoveryV1`, and line 105 immediately hard-stops.
- Interpretation: H-037 is live-crossed. The next blocker is not Python file creation but over-strict rejection of an anchored placeholder hunk that is mechanically convertible to native `@@` and has enough context for unanchored detection to remain safe.

# Hypothesis H-038: anchored placeholder hunks should normalize to native hunk headers

- Claim: Native apply_patch can safely accept `@@` hunk headers. If a provider emits `@@ ... @@` in an update section but also includes concrete context or `-old` / `+new` lines, the placeholder marker is mechanically convertible to native `@@`. Current action-contract rejection treats all `@@ ... @@` as fatal, so a potentially valid patch can be rejected at the end of a budget-recovery window.
- Prediction: The live line 101 patch shape should normalize by stripping `--- a/...` / `+++ b/...` and converting `@@ ... @@` to `@@`; a test should confirm the resulting payload contains the real context line and no placeholder marker. Add-only placeholder updates with no anchor should still be caught by `apply_patch_unanchored_update`.
- Diagnostic evidence plan: Update placeholder hunk tests from reject to normalize; add a live mixed placeholder hunk test from the trace; run native_hunk, apply_patch, validation rework, and validation regression filters.
- Status: confirmed.

# Evidence E-084: anchored placeholder hunks now normalize before final action-contract checks

- Prediction tested: H-038 requires placeholder hunk markers with real context to normalize to native `@@`, while malformed dash-native headers remain rejected and unanchored add-only updates remain blocked.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked placeholder_hunk
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked mixed_placeholder
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked native_hunk
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `placeholder_hunk` is 2/2, `mixed_placeholder` is 1/1, `native_hunk` is 3/3, `apply_patch_` is 35/35, `validation_rework` is 15/15, `validation_` is 92/92, and `duplicate_rework` is 2/2. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - `@@ ... @@` with real context normalizes to native `@@`.
  - The live mixed shape `*** Update File` + `--- a/...` / `+++ b/...` + `@@ ... @@` strips unified file headers and preserves the anchor context.
  - `--- Update File:` malformed operation headers are still rejected by the dash-native test.
- Interpretation: The `apply-patch-anchored-placeholder-hunk-normalization-gap` class is focused-fixed at action-contract normalization/build level. Commit/push, binary attestation, and another keyed rerun are required before claiming live convergence beyond this case.

# Evidence E-085: H-038 rerun crosses placeholder hunk rejection and exposes duplicate-read advisory loop

- Prediction tested: H-038 should remove `apply_patch_native_hunk_header` / `TaskSpaceApplyPatchNativeHunkRecoveryV1` as the live blocker after commit/push/build attestation.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ak-placeholder-hunk/runs/terminal_bench__organization-json-generator/20260704-075115-109
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 16
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: 75923e09bb069d5a5a57c264c56f0d0be7ae99e5
  build_attestation_status: pass
  ```
- Matched trace signals:
  - No `apply_patch_native_hunk_header` or `TaskSpaceApplyPatchNativeHunkRecoveryV1` appears in the right-side trace, so H-038 crossed.
  - Line 50 runs the expected schema validation command and reaches real jsonschema errors: projects emit `member_ids` instead of required `members`, and statistics still use snake_case / alternate keys instead of required camelCase fields.
  - Line 59 reads `generate_org.py`; active projection later shows `critical_artifact_evidence` with `result-11` and states `read/search is no longer a valid next action`.
  - Lines 69, 76, 83, 90, and 97 repeatedly insert `TaskSpaceValidationReworkDuplicateReadRecoveryV1`; line 98 hits `TaskSpaceProviderBudgetHardStopV1`.
- Interpretation: The failure semantic is not missing. The projection and recovery both preserve the correct next action: patch the already-read rework artifact or block with concrete external reason. The live gap is that runtime treats repeated violation of this patch-only gate as another advisory recovery, allowing provider sampling to continue until budget hard stop.

# Hypothesis H-039: validation rework duplicate-read recovery needs a hard-stop escalation

- Claim: For validation rework duplicate reads, the first structured recovery is useful, but repeated `validation_rework_duplicate_artifact_read` after the same patch-only contract is no longer a recoverable tool failure. Continuing provider sampling converts a correct action-contract rejection into a budget-drain loop. Runtime should recognize a repeated duplicate-read gate and stop provider sampling with a stable hard-stop marker instead of issuing another advisory recovery request.
- Prediction: A focused unit test should show the first `TaskSpaceValidationReworkDuplicateReadRecoveryV1` is not terminal, but the next recovery for the same class, or any recovery carrying `repeated_blocked_action`, produces `TaskSpaceValidationReworkDuplicateReadHardStopV1`. The hard-stop item must not be classified as ordinary no-action recovery or implement-needs-edit advisory.
- Diagnostic evidence plan: Add a dedicated marker, recovery counter, repeated-gate detection, and tests for both count-based and `repeated_blocked_action`-based escalation. Run validation rework, duplicate rework, validation, apply_patch, fmt/diff, and whale build regressions.
- Status: confirmed.

# Evidence E-086: duplicate-read advisory loop now escalates to a named hard stop

- Prediction tested: H-039 requires repeated validation rework duplicate reads to stop model sampling before provider/node budget exhaustion.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework_duplicate_read
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework_duplicate_read` is 5/5, `validation_rework` is 17/17, `duplicate_rework` is 2/2, `validation_` is 94/94, and `apply_patch_` is 35/35. `cargo fmt --check`, `git diff --check`, and the `whale` build passed.
- Matched test signals:
  - First duplicate-read recovery remains advisory.
  - A second same-class recovery escalates to `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
  - A recovery that already carries `repeated_blocked_action` hard-stops immediately.
  - The hard-stop item is not counted as `TaskSpaceNoActionRecoveryV1` or `TaskSpaceImplementNeedsEditRecoveryV1`.
- Interpretation: The `validation-rework-duplicate-read-advisory-loop` class is focused-fixed at runtime recovery-loop level. This does not auto-generate the missing code patch; it prevents a correct patch-only tool failure from being diluted into repeated model retries and budget hard stop. Commit/push, binary attestation, and another keyed rerun are required before claiming live behavior.

# Evidence E-087: H-039 rerun crosses duplicate-read escalation and exposes post-edit transition gap

- Prediction tested: H-039 should stop repeated validation rework duplicate reads from draining the node. A rerun can still expose a different downstream tools-chain problem once the model emits a patch.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703al-duplicate-read-hard-stop/runs/terminal_bench__organization-json-generator/20260704-080713-106
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: 23d20b5cd547ffdaed19725a38f70919af8cf672
  build_attestation_status: pass
  ```
- Matched trace signals:
  - The trace does not repeat the same duplicate-read recovery twice; after the first duplicate-read recovery, the model attempts `apply_patch`, so H-039's hard stop is not the active blocker in this run.
  - `rollout.jsonl` records `trace-208` as `kind=main_tool_result`, `nodeId=node-4`, `resultId=result-14`, `callId=taskspace-action-contract-15-apply_patch`, `actionClass=edit`, `toolSuccess=true`, and `artifactRefs=["generate_organization.py"]`.
  - The same provider request records `node_request_count=5`, `max_model_requests_per_node=5`, and `runtime_budget_state=thin_downgraded`.
  - The next actionability trace is still `response_actionability:actionable`, `recovery_action:none`, with no `TaskSpaceForcedImplementTransitionV1` / `forced_implement_transition` before the turn stops.
  - `whale-exec.jsonl` then ends with `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=15/20 node_request_count=5/5`.
- Interpretation: The edit result semantics are present in the action map; the missing behavior is the post-edit runtime transition into validation. This is a control/feedback boundary bug after successful tool execution, not an absent or malformed edit feedback case.

# Hypothesis H-040: successful implement edit at node budget boundary is not forced into validation

- Claim: `force_finish_implement_for_provider_budget` is designed to close an implement node and open validation after a successful edit under provider-budget pressure, but the budget-pressure predicate is currently inert. Because `provider_request_budget_snapshot_pressure_active_for_node` returns `false`, the post-tool-drain path sees a successful edit but cannot perform the forced transition. The following provider request reaches the pre-dispatch hard stop, leaving the implement node open.
- Prediction: Code inspection should show the pressure predicate is fixed `false`. A focused runtime test should reproduce a successful edit at `node_request_count == max_model_requests_per_node` and fail to transition before the fix. Repair should make node/profile pressure active at the hard-limit boundary and preserve the existing pre-dispatch hard stop for cases without a successful edit.
- Diagnostic evidence plan: Replace the inert predicate with a real boundary check over node request count and profile request budget, add focused tests for implement forced transition at node hard limit and no transition below pressure, then run provider budget, taskspace active budget, validation rework, and build regressions.
- Status: confirmed.

# Evidence E-088: successful implement edit at node limit now forces validation transition

- Prediction tested: H-040 requires a successful edit at the node request hard-limit boundary to close the implement node and open a validation node before the next provider pre-dispatch hard stop.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked provider_budget
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_active_budget
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `provider_budget` is 23/23, `taskspace_active_budget` is 11/11, `validation_rework` is 17/17, `validation_` is 94/94, `apply_patch_` is 35/35, and `duplicate_rework` is 2/2. `cargo fmt --check` passed with existing stable rustfmt `imports_granularity` warnings; `git diff --check` and the `whale` build passed.
- Matched test signals:
  - `provider_budget_node_limit_force_finishes_implementation_into_smoke_test_after_edit` records an edit, builds a snapshot with `node_request_count == max_model_requests_per_node`, then observes a completed implement node, a running `SmokeTest` node, and a `forced_implement_transition` trace event.
  - `provider_budget_below_node_limit_does_not_force_finish_implementation_after_edit` records the same successful edit below the node limit and leaves the implement node running.
- Interpretation: The `post-edit-forced-validation-transition-gap` class is focused-fixed at runtime control/feedback level. A binary attestation and another keyed rerun are required to prove the external sample now proceeds from the successful `apply_patch` into schema validation instead of ending at `TaskSpaceProviderBudgetHardStopV1`.
