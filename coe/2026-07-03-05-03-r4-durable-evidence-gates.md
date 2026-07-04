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

# Evidence E-089: H-040 rerun crosses provider budget hard stop and exposes schema failure semantic truncation

- Prediction tested: H-040 should prevent a successful edit at the provider/node budget edge from ending as `TaskSpaceProviderBudgetHardStopV1`.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703am-post-edit-transition/runs/terminal_bench__organization-json-generator/20260704-082204-387
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 12
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: c7d5ba971c03b595bca73bf6a3a111d4a75b0834
  build_attestation_status: pass
  ```
- Matched crossed signals:
  - `whale-exec.jsonl` contains `TaskSpaceValidationReworkDuplicateReadHardStopV1` and does not contain `TaskSpaceProviderBudgetHardStopV1`, so H-039/H-040's provider-budget-drain blocker is no longer the active failure in this run.
  - The exact validation command ran: `python generate_org.py && python -m jsonschema -i organization.json schema.json`.
  - The full command output includes all jsonschema missing-required-property lines: `members`, `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
- New blocker signal:
  - `rollout.jsonl` result `result-9` stores only a telemetry preview in the ActionMap body. It is truncated at `average_years_of_servic` before the later statistics required-property lines can be parsed.
  - The derived blocker `result-10` and subsequent rework repair contract only contain `missing_required_properties=members`, so the provider-visible patch contract loses the statistics fields even though the raw tool output had them.
- Interpretation: The next failure class is feedback-layer semantic truncation before ActionMap storage. The validation tool did fail with complete, actionable schema semantics; TaskSpace stored a bounded preview that preserved only the first repeated `members` field and dropped the remaining required properties used by the repair contract.

# Hypothesis H-041: validation schema required-property semantics are truncated before ActionMap storage

- Claim: For ordinary shell-like validation tools, `tools/parallel.rs` records only `tool_output_model_visible_preview(...)` into `record_action_map_main_tool_result`. For exec output this preview is already bounded by telemetry limits, so downstream ActionMap repair-contract parsing reads truncated text and cannot recover required-property failures that appear after the preview cutoff.
- Prediction: Code inspection should show the preview is built from the model-visible response item, while `ExecCommandToolOutput` still has raw output before truncation. A focused test should fail before repair when a long jsonschema output contains later required properties beyond the preview cutoff, and should pass after repair by preserving a `missing_required_properties:` summary at the top of the ActionMap preview.
- Diagnostic evidence plan: Add a semantic-summary extraction step at the tool-result preview boundary using complete raw output where available; add tests for long jsonschema output and for non-schema output; run tool preview, validation rework, validation, fmt, diff check, and `whale` build regressions.
- Status: confirmed.

# Evidence E-090: schema required-property summary is preserved before telemetry preview truncation

- Prediction tested: H-041 predicts that extracting a semantic summary from complete raw output before telemetry preview truncation will let ActionMap repair contracts recover required properties that are absent from the truncated raw preview.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/tools/context.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/context_tests.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_preview_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework_projects_schema_repair_contract_from_schema_read
  ```
- Adjacent regression/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `taskspace_preview_` is 2/2, `validation_rework_projects_schema_repair_contract_from_schema_read` is 1/1, `validation_rework` is 17/17, `validation_` is 94/94, and `apply_patch_` is 35/35. `cargo fmt --check` passed with existing stable rustfmt `imports_granularity` warnings; `git diff --check` and the `whale` build passed.
- Matched test signals:
  - `taskspace_preview_preserves_required_properties_from_untruncated_exec_output` builds a long exec output where later jsonschema required-property lines fall beyond the telemetry preview cutoff; the ActionMap preview starts with `TaskSpaceToolSemanticSummaryV1` and includes all required fields.
  - `taskspace_preview_does_not_add_schema_summary_for_plain_exec_output` keeps ordinary long exec output free of `missing_required_properties`.
  - `validation_rework_projects_schema_repair_contract_from_schema_read` now simulates a truncated raw validation preview plus complete semantic summary, and still produces a repair contract containing `members`, the statistics camelCase fields, and the schema required sibling group.
- Interpretation: The `validation-schema-required-property-summary-truncated-before-action-map` class is focused-fixed at tool-result preview and ActionMap repair-contract levels. Commit/push, binary attestation, and another keyed rerun are still required to prove live `organization-json-generator` rework feedback now carries the full schema repair contract.

# Evidence E-091: c8fe197 rerun shows shell_command truncates before ToolOutput preview

- Prediction tested: E-090 predicted the `ToolOutput` preview-layer summary would appear in the live `organization-json-generator` ActionMap result.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703an-schema-summary/runs/terminal_bench__organization-json-generator/20260704-083751-467
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: c8fe197171b6f236f11641bedc2548ef28ef64a9
  build_attestation_status: pass
  ```
- Result: partially refuted E-090's live-path assumption.
- Matched signals:
  - `whale-exec.jsonl` line 36 contains full raw jsonschema output with all six statistics required properties: `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - `rollout.jsonl` `result-8` body has no `TaskSpaceToolSemanticSummaryV1`; it stores a truncated `TaskSpaceToolInvocationV1` preview ending before `projectStatusDistribution` and `averageYearsOfService`.
  - `result-9` blocker and the final duplicate-read hard stop carry only `missing_required_properties=averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes`.
- Interpretation: The live `shell_command` path converts `ExecToolCallOutput` into a `FunctionToolOutput` error string inside the exec formatter before `tool_output_model_visible_preview` runs. The repair must move to `format_exec_output_str_with_ref` / exec formatter, not just the later ToolOutput preview boundary.

# Hypothesis H-042: shell command exec formatter truncates schema failures before FunctionToolOutput is built

- Claim: `ShellHandler::run_exec_like` calls `ToolEmitter::finish`, which formats `ExecToolCallOutput` into a model-visible string through `format_exec_output_for_model_*`. For nonzero exit codes this string is returned as `FunctionCallError::RespondToModel`, and the ActionMap preview records that already-truncated error response. Therefore the semantic summary must be attached in `format_exec_output_str_with_ref` before `formatted_truncate_text`, not only in `ToolOutput::taskspace_semantic_summary`.
- Prediction: A focused formatter test should reproduce a long `ExecToolCallOutput` where the last required-property lines are past the truncation cutoff and should pass only if `format_exec_output_str_with_ref` prepends a complete `TaskSpaceToolSemanticSummaryV1`. Existing ToolOutput preview and validation rework tests should keep passing.
- Diagnostic evidence plan: Move the shared semantic-summary helper to `tools/mod.rs`, call it from `format_exec_output_str_with_ref` and the freeform formatter, reuse it from `context.rs`, then run `schema_summary`, `taskspace_preview_`, `validation_rework`, `validation_`, fmt/diff/build, and another keyed rerun.
- Status: confirmed.

# Evidence E-092: exec formatter now preserves schema required-property summary before truncation

- Prediction tested: H-042 predicts that the exec formatter, not only the ToolOutput preview layer, must prepend the semantic summary.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/tools/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/context.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/context_tests.rs`
- Focused commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked schema_summary
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_preview_
  ```
- Adjacent regression commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  ```
- Static/build commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `schema_summary` is 2/2, `taskspace_preview_` is 2/2, `validation_rework` is 17/17, `validation_` is 94/94, `apply_patch_` is 35/35, fmt/diff checks pass, and the `whale` binary build passes.
- Matched test signals:
  - `exec_output_formatter_preserves_schema_summary_before_truncation` constructs a long `ExecToolCallOutput` where `projectStatusDistribution` and `averageYearsOfService` appear after the truncation cutoff; `format_exec_output_str_with_ref(..., TruncationPolicy::Bytes(512), ...)` still starts with `TaskSpaceToolSemanticSummaryV1` and carries all six required fields.
  - `taskspace_preview_` still preserves ToolOutput-level semantics and avoids adding schema summaries to plain output.
  - `validation_rework_projects_schema_repair_contract_from_schema_read` still parses the semantic summary into the repair contract.
- Interpretation: H-042 is focused-fixed at the formatter/build level. Commit/push, attestation, and another live rerun are still required before claiming the live `shell_command` path is fixed.

# Evidence E-093: ed3252a live rerun validates schema summary and exposes failed-edit projection dilution

- Prediction tested: H-042 predicted that moving the schema summary to the exec formatter would make the live `shell_command` path carry complete schema required-property semantics into ActionMap/projection.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ao-exec-summary/runs/terminal_bench__organization-json-generator/20260704-085030-109
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: ed3252a9db7d09b5e9e76e31fe7e56c59e464d13
  build_attestation_status: pass
  ```
- Result: supported H-042 for the live shell path, but revealed a new downstream feedback/projection failure.
- Matched schema-summary signals:
  - `whale-exec.stderr.log` starts the failed jsonschema output with `TaskSpaceToolSemanticSummaryV1` and `missing_required_properties: members, averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService`.
  - `rollout.jsonl` active projection includes `validation_rework_schema_repair` with the same full repair contract and `target_artifacts=build_organization.py, build_organization.py:68`.
- New blocker signals:
  - `apply_patch` failed with `Failed to find expected lines in .../build_organization.py` because the submitted hunk referenced a non-existent `return { ... }` block.
  - `TaskSpaceEditFailureRecoveryV1` preserved the failed edit feedback, and runtime rejected repeated `finish_node` attempts with `cannot be completed without a recorded successful edit action`.
  - The active projection still exposed conditional/future guidance as `taskspace_control(action=finish_node) ... after successful edit`, and failed edit feedback was only available through recovery text / hidden refs rather than as `critical_artifact_evidence`.
  - The run ended on `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded` after the model continued claiming the edit had already succeeded.
- Interpretation: The schema semantic truncation class is live-fixed. The active blocker is now `failed-edit-projection-recovery-dilution`: the failed edit result is present but not strong enough in provider-visible projection semantics, and `next_valid_actions` contains a conditional future finish action that the model treats as immediately valid.

# Hypothesis H-043: failed edit feedback is not promoted into provider-visible projection constraints

- Claim: In validation rework after a failed `apply_patch`, `TaskSpaceEditFailureRecoveryV1` carries the failure text, but `ContextProjectionV1` does not promote the failed edit into `critical_artifact_evidence`; `next_valid_actions` also advertises `finish_node` as a conditional future action before a successful edit exists. This dilutes the state-machine guard: runtime rejects premature `finish_node`, but each rejection consumes another provider turn until budget hard stop.
- Prediction: A focused projection regression should show that after a failed edit on a validation rework node, projection contains `failed_edit_feedback signal=latest_failed_edit`, `next_valid_actions` contains the failed edit / corrected apply_patch instruction, and no `taskspace_control(action=finish_node)` next action is exposed until a successful edit result exists.
- Diagnostic evidence plan: Update `projection_critical_artifact_evidence`, `projection_next_valid_actions`, and the compact allowed-actions text; run `validation_rework`, `validation_`, `apply_patch_`, schema-summary/tool-preview tests, fmt/diff/build, then commit/attest/rerun.
- Status: confirmed.

# Evidence E-094: failed edit projection now makes repair immediate and hides premature finish

- Prediction tested: H-043 predicts the failed edit must become provider-visible projection evidence and that `finish_node` must not be listed as a current next action before a successful edit.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked schema_summary
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_preview_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework` is 17/17, `validation_` is 94/94, `apply_patch_` is 35/35, `schema_summary` is 2/2, `taskspace_preview_` is 2/2, fmt/diff checks pass, and the `whale` binary build passes.
- Matched test signals:
  - `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback` now asserts that after a validation rework target read, projection contains `do not taskspace_control(action=finish_node)` and no immediate `taskspace_control(action=finish_node)` next action.
  - After a failed edit, the same regression asserts `failed_edit_feedback` names the failed patch result and `projection_critical_artifact_evidence` includes `signal=latest_failed_edit`.
  - Allowed-action text now says `finish_node` is blocked until successful edit while preserving the one allowed same-target refresh read after a failed edit.
- Interpretation: H-043 is focused-fixed at projection/feedback/build level. Commit/push, attestation, and live rerun are still required before claiming this downstream blocker is cleared.

# Evidence E-095: 14e6aa2 live rerun crosses failed-edit projection and exposes unanchored patch feedback loss

- Prediction tested: H-043 predicted that failed edit feedback should become projection-critical and premature `finish_node` should be blocked before a successful edit.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ap-failed-edit-projection/runs/terminal_bench__organization-json-generator/20260704-090416-015
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: 14e6aa21c291e95f4e89b745ad4743025ca9a44c
  build_attestation_status: pass
  ```
- Result: supported the H-043 projection repair enough to move past the previous blocker, but exposed a related feedback-preservation gap.
- Matched crossed signals:
  - `whale-exec.stderr.log` still starts failed jsonschema output with `TaskSpaceToolSemanticSummaryV1` and `missing_required_properties: members, averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService`.
  - The run reached target inspection and attempted a patch instead of staying in the earlier failed-edit finish loop.
  - Runtime rejected premature `finish_node` with `cannot be completed without a recorded successful edit action`, preserving the state-machine bottom line.
- New blocker signals:
  - The emitted patch was malformed native grammar:
    ```text
    *** Begin Patch
    *** Update File: generate.py
    import csv
    ...
    *** End Patch
    ```
    It had no `@@` hunk and no `-old` / `+new` lines.
  - Action contract rejected before tool dispatch with `TaskSpaceActionV1 rejected: apply_patch_unanchored_update:generate.py. Return exactly one valid taskspace-action-v1 JSON object.`
  - The model then repeated `read_file generate.py`; duplicate-read recovery and hard stop fired, but the hard-stop excerpt preserved the generic repair contract more strongly than the specific `apply_patch_unanchored_update` rejection.
- Interpretation: The next active blocker is `validation-rework-duplicate-read-after-unanchored-patch-feedback-loss`. It is a feedback-layer semantic loss after a valid action-contract rejection, not a tool executor failure and not a state-machine permission breach.

# Hypothesis H-044: duplicate-read recovery drops unanchored patch rejection semantics

- Claim: `build_taskspace_validation_rework_duplicate_read_recovery_item` preserves failed edit/patch grammar feedback for some patch failures, but the recovery text and advisory classifier only treat `apply_patch_mixed_native_unified` and `apply_patch_native_hunk_header` as patch grammar failures. `apply_patch_unanchored_update` remains present only as a lower-priority failed-edit string and can be truncated out of the final hard-stop excerpt by previous blocked feedback and repair-contract text.
- Prediction: A focused duplicate-read recovery test with `TaskSpaceActionV1 rejected: apply_patch_unanchored_update:generate.py` should show `Most recent failed edit feedback to preserve` before `Previous blocked feedback`, classify the recovery as `TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1`, instruct the model to correct patch grammar now, and explicitly say `read_file/context refresh is not a valid recovery`.
- Diagnostic evidence plan: Move failed edit feedback above previous blocked feedback in duplicate-read recovery, include `apply_patch_unanchored_update` in patch grammar preservation and advisory classification, then run focused duplicate-read recovery tests plus `validation_rework`, `validation_`, `apply_patch_`, schema summary/tool preview, fmt/diff/build.
- Status: confirmed.

# Evidence E-096: unanchored patch grammar feedback now survives duplicate-read recovery

- Prediction tested: H-044 predicts that duplicate-read recovery must carry `apply_patch_unanchored_update` as first-class failed patch grammar feedback.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework_duplicate_read
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked schema_summary
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_preview_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework_duplicate_read` is 6/6, `validation_rework` is 18/18, `validation_` is 95/95, `apply_patch_` is 35/35, `schema_summary` is 2/2, `taskspace_preview_` is 2/2, fmt/diff checks pass, and the `whale` binary build passes.
- Matched test signals:
  - `validation_rework_duplicate_read_recovery_preserves_unanchored_patch_feedback` asserts the recovery contains `Most recent failed edit feedback to preserve`, `apply_patch_unanchored_update:generate.py`, `correct that patch grammar now`, and `read_file/context refresh is not a valid recovery`.
  - The same test asserts failed edit feedback appears before `Previous blocked feedback`, so the bounded hard-stop excerpt keeps the concrete patch grammar rejection.
  - Advisory warning classification now emits `TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1` for unanchored patch updates, matching mixed/native patch grammar failures.
- Interpretation: H-044 is focused-fixed at session recovery/feedback level. Commit/push, attestation, and another live rerun are required to verify whether the real `organization-json-generator` case now moves past this blocker.

# Evidence E-097: f9ab63f live rerun crosses unanchored patch feedback and exposes read completeness ambiguity

- Prediction tested: H-044 predicted that `apply_patch_unanchored_update` rejection semantics would survive duplicate-read recovery/hard-stop.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703aq-unanchored-recovery/runs/terminal_bench__organization-json-generator/20260704-091714-857
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: f9ab63f733ccc488c470211a56375d6a068c944e
  build_attestation_status: pass
  ```
- Result: supported H-044 by crossing the unanchored patch grammar blocker; the trace contains no `apply_patch_unanchored_update` recurrence.
- New blocker signals:
  - Initial validation ran the correct contract command: `python generate_organization.py && python -m jsonschema -i organization.json schema.json`.
  - The validation failed with a precise editable traceback:
    ```text
    NameError: name 'projects_by_dept' is not defined
    ```
  - Validation rework read `generate_organization.py` once and received the relevant code, including `compute_project_budget` and the later `projects_by_dept` construction.
  - The provider then repeated `read_file generate_organization.py` twice with rationales like `Need full file to fix undefined projects_by_dept`, and runtime stopped at `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
- Interpretation: The active blocker moved to `validation-rework-read-file-completeness-ambiguity`: the successful read content existed, but the read feedback did not explicitly mark whether the bounded `sed -n '1,240p'` output reached EOF or was truncated. The model treated the visible result as possibly incomplete and kept asking for the full file.

# Hypothesis H-045: successful read_file results lack completeness semantics in validation rework feedback

- Claim: `taskspace_read_file_command` emits a bounded first-240-line command without a structured completion summary. Projection and duplicate-read recovery can say `result-10` is visible, but cannot distinguish a complete small file from a truncated large file. In traceback-driven validation rework, this ambiguity lets the model justify repeated reads with `Need full file` even when the previous result contains the whole file.
- Prediction: A focused test should show that read_file commands append `TaskSpaceReadFileSummaryV1` with `lines_read`, `eof_reached`, and `max_lines`; ActionMap should preserve this summary in working evidence excerpts, projection `critical_artifact_evidence`, `next_valid_actions`, and duplicate-read gate feedback. `eof_reached=true` should say no additional lines are hidden; `eof_reached=false` should remain a bounded read and must not be mislabeled complete.
- Diagnostic evidence plan: Keep the existing `sed -n '1,240p'` prefix for artifact parsing, append a bounded `awk`/PowerShell summary, update `sed_read_command_artifact_ref` to ignore the summary suffix, parse the summary from multiline or preview-shaped result bodies, and run focused read/projection tests plus `validation_rework`, `validation_`, `apply_patch_`, fmt/diff/build.
- Status: confirmed.

# Evidence E-098: read_file completeness summary is preserved through projection and duplicate-read gates

- Prediction tested: H-045 predicts that read_file completeness must be explicit and must survive ActionMap projection/recovery.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked action_contract_read_file_uses_host_platform_command
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked sed_read_command_artifact_ref_ignores_read_summary_suffix
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked working_evidence_excerpt_preserves_bounded_read_summary
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. The four focused tests are 1/1 each, `validation_rework` is 18/18, `validation_` is 95/95, `apply_patch_` is 35/35, fmt/diff checks pass, and the `whale` binary build passes.
- Matched test signals:
  - `action_contract_read_file_uses_host_platform_command` keeps the Unix command starting with `sed -n 1,240p` and appends `TaskSpaceReadFileSummaryV1`; Windows keeps `Get-Content -TotalCount 240` and appends the same summary.
  - `sed_read_command_artifact_ref_ignores_read_summary_suffix` proves `sed ... && awk ...` still resolves the original artifact path.
  - `working_evidence_excerpt_preserves_bounded_read_summary` proves `eof_reached=false` is preserved as bounded context.
  - `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback` now asserts projection includes `read_context: complete_read`, `eof_reached=true`, and duplicate-read gate feedback says no additional file lines are hidden.
- Interpretation: H-045 is focused-fixed at read feedback/projection/recovery level. Commit/push, attestation, and another live rerun are required to verify whether `organization-json-generator` now moves past the repeated `Need full file` loop.

# Evidence E-099: 1a9eb0c live rerun exposes Unix awk double-dash portability failure

- Prediction tested: H-045 predicted that live `read_file` results would carry `TaskSpaceReadFileSummaryV1`.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ar-read-completeness/runs/terminal_bench__organization-json-generator/20260704-093439-320
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: wrong
  right_exec_timed_out: False
  right_tool_call_count: 16
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: 1a9eb0ceb509b0b505fb3ba78f0dd9ddc933d2e8
  build_attestation_status: pass
  ```
- Result: did not validate H-045 live behavior because the new summary command failed before producing `TaskSpaceReadFileSummaryV1`.
- New blocker signals:
  - Each `read_file` command used `sed -n '1,240p' -- <path> && awk ... -- <path>`.
  - In the benchmark container, `awk` failed with:
    ```text
    awk: cannot open "--" (No such file or directory)
    ```
  - The `sed` prefix printed file contents, but the combined command exited 2, so TaskSpace treated the read as failed and the model entered inspect recovery/budget drain.
- Interpretation: The summary design is correct at the feedback contract level, but the Unix command used a non-portable `awk --` separator. This is a capability-layer command construction bug introduced by H-045's repair.

# Hypothesis H-046: read_file summary command must avoid awk double-dash for POSIX portability

- Claim: Some benchmark environments use an `awk` implementation that does not accept `--` as an option terminator. Since `sed` already safely uses `--` for the actual content read and ActionMap artifact parsing keys off the `sed` prefix, the summary `awk` call should pass the path directly as an operand instead of `-- <path>`.
- Prediction: A direct shell smoke should produce file contents plus `TaskSpaceReadFileSummaryV1` with exit 0. Focused command generation and artifact parser tests should still pass, and validation/apply_patch/build regressions should remain green.
- Diagnostic evidence plan: Remove `--` only from the appended `awk` summary command, update the parser regression fixture, run a direct shell smoke, focused read command/parser tests, `validation_rework`, `validation_`, `apply_patch_`, fmt/diff/build.
- Status: confirmed.

# Evidence E-100: read_file summary command now works without awk double-dash

- Prediction tested: H-046 predicts that removing `--` from `awk` preserves summary output and avoids the benchmark container failure.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Direct smoke:
  ```text
  printf 'a\nb\n' > target/read-summary-smoke.txt
  sed -n '1,240p' -- target/read-summary-smoke.txt && awk '...' target/read-summary-smoke.txt
  ```
- Direct smoke result:
  ```text
  a
  b

  TaskSpaceReadFileSummaryV1: path=target/read-summary-smoke.txt lines_read=2 eof_reached=true max_lines=240
  ```
- Focused and regression commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked action_contract_read_file_uses_host_platform_command
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked sed_read_command_artifact_ref_ignores_read_summary_suffix
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. Focused tests are 1/1 each, `validation_rework` is 18/18, `validation_` is 95/95, `apply_patch_` is 35/35, fmt/diff checks pass, and the `whale` binary build passes.
- Interpretation: H-046 is focused-fixed. Commit/push, attestation, and another live rerun are required to validate H-045's intended read completeness behavior in the real benchmark path.

# Evidence E-101: 184defa live rerun crosses read summary portability and exposes pytest runner dependency misroute

- Prediction tested: H-046 predicted that the live `read_file` summary command would avoid the `awk --` portability failure and produce `TaskSpaceReadFileSummaryV1` with exit 0.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703as-read-summary-portable/runs/terminal_bench__organization-json-generator/20260704-094153-058
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 8
  right_open_leaf_nodes: 1
  public_validation_exit_code: 1
  hidden_oracle_exit_code: 0
  preflight_git_head: 184defa960f3bef835de79efee7af06e9c7eb462
  build_attestation_status: pass
  ```
- Result: supported H-046 and the live half of H-045 for initial fact-source reads, but exposed a new feedback-layer misclassification.
- Matched crossed signals:
  - `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv` reads all ended with `TaskSpaceReadFileSummaryV1` and `exit_code=0`.
  - The earlier `awk: cannot open "--"` signature did not recur.
- New blocker signals:
  - The model created `process.py`, then attempted a smoke command that would have generated `organization.json`, but TaskSpace routed validation to `python -m pytest`.
  - The host command failed with:
    ```text
    /home/zhangxu/miniconda3/bin/python: No module named pytest
    ```
  - Runtime recorded `result-9` as a validator failure, then auto-created `node-4` with kind `implement_solution` and context `Fix the implementation artifact(s)`.
  - The model repeatedly called `taskspace_control(action=finish_node)` on `node-4`; runtime kept injecting `TaskSpaceImplementNeedsEditRecoveryV1` because `node-4` had no successful edit, and the turn ended at `TaskSpaceProviderBudgetHardStopV1`.
- Interpretation: This is semantic distortion, not semantic loss. The raw failure was visible, but runtime translated a validator runner dependency failure into an implementation repair task. The problem type is `validation-pytest-runner-dependency-misroute`.

# Hypothesis H-047: missing pytest runner dependency is misclassified as implementation failure

- Claim: When a validation node runs `python -m pytest` and the Python environment itself lacks pytest, `text_mentions_local_validator_infra_failure` does not classify the result as local validator infrastructure. The generic failed-validation path then creates an `implement_solution` rework node, which has no concrete code edit to apply and enters an implement-needs-edit recovery loop.
- Prediction: Classifying `No module named pytest` under a pytest runner command as local validator infrastructure, and allowing a platform-compatible validation rerun, should mark the failed result invalid, block the original validation node, and bind a new `smoke_test` rerun node instead of an `implement_solution` rework node.
- Diagnostic evidence plan: Add a focused runtime test using the AS failure shape (`command: python -m pytest`, `/home/.../python: No module named pytest`) after an accepted `process.py` edit. The test must assert `ResultValidity::Invalid`, original validation node blocked, new current node kind `smoke_test`, and no `current_main_implement_progress_needs_edit`.
- Status: confirmed.

# Evidence E-102: missing pytest runner dependency now routes to validation rerun, not implement rework

- Prediction tested: H-047 predicts that pytest runner dependency failures must stay in the validation/infra path.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core missing_pytest_runner_dependency_routes_to_validation_rerun_not_implementation --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core local_infra_tool_result_auto_blocks_validation_node --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core state_commit_accepts_failed_validation_result_after_runtime_rework_transition --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_missing_jsonschema_dependency_stays_on_validation_with_cli_recovery --lib
  ```
- Result: passed. The new focused test is 1/1 and the three adjacent regression tests are 1/1 each.
- Matched test signals:
  - `node_result_is_local_validator_infra_failure(result)` is true for `command: python -m pytest` plus `No module named pytest`.
  - The failed validation result is marked invalid.
  - The original validation node is blocked.
  - The newly bound rerun node is `smoke_test` and its context names local validator infrastructure plus `process.py`.
  - `current_main_implement_progress_needs_edit()` is false, so the AS `TaskSpaceImplementNeedsEditRecoveryV1` loop is not reproduced.
- Interpretation: H-047 is focused-fixed at runtime classification level. Commit/push, full R4 regression/build checks, attestation, and another live rerun are still required before claiming the live `organization-json-generator` path crossed this case.

# Evidence E-103: pytest runner dependency repair passes R4-adjacent regression and build gates

- Prediction tested: H-047 repair should not regress adjacent validation, validation rework, patch recovery, or provider budget behavior.
- Commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed.
- Matched counts:
  - `local_infra`: 11/11.
  - `validation_`: 96/96, including `missing_pytest_runner_dependency_routes_to_validation_rerun_not_implementation`.
  - `validation_rework`: 18/18.
  - `apply_patch_`: 35/35.
  - `provider_budget`: 23/23.
  - `taskspace_active_budget`: 11/11.
  - fmt check, whitespace check, and `whale` build pass. The fmt command still prints the known stable rustfmt warning for `imports_granularity`.
- Interpretation: The H-047 repair is regression-clean at focused/build level. The remaining evidence gate is commit/push, build attestation, and a live keyed `organization-json-generator` rerun.

# Evidence E-104: 878248b live rerun exposes fact-source path artifact-ref loss before pytest routing

- Prediction tested: H-047 focused repair should let the live run proceed past pytest runner dependency misrouting if it reaches that validation phase.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703at-pytest-infra/runs/terminal_bench__organization-json-generator/20260704-095934-702
  PairReport: pair-001/pair-report.md
  preflight_git_head: 878248bb9d7fd4232189788dc7ad3fe8e345820f
  build_attestation_status: pass
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: wrong
  right_tool_call_count: 6
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Result: the run did not reach the pytest dependency case. It stopped earlier in `inspect_code_context`.
- Matched signals:
  - The model's `start_task` payload declared four fact-source paths: `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv`.
  - Runtime observability retained four fact sources, but each had only a generated id, natural-language description, and `artifactRef=user-request`; none retained the declared path.
  - The node read `schema.json`, retried the same read three times, then read `departments.csv`.
  - Duplicate read recovery correctly blocked the repeated `schema.json` command, but the inspect node still hit `TaskSpaceProviderBudgetHardStopV1` at `node_request_count=6/5`.
- Interpretation: The adaptive inspect budget rule was not wrong, but its input was missing. Because path artifacts were lost at normalization/parse time, `task_required_fact_source_artifact_refs()` found zero required artifacts and `effective_provider_node_request_limit()` stayed at the base limit of 5.

# Hypothesis H-048: inline fact-source path is discarded before runtime state

- Claim: `taskspace_control` accepts fact-source objects with inline `path`, but `TaskSpaceFactSourceArgs` and normalization do not carry that field into `evidence_refs[].artifact_ref`. Serde ignores the unknown field, so TaskState loses the artifact identity required by inspect coverage guards and adaptive provider node budgets.
- Prediction: Normalizing inline `path`, `artifact_ref`, `artifact_path`, and `source_path` fields, plus their array aliases, into `evidence_refs[].artifact_ref` should preserve required fact-source artifacts for `start_task`, `state_commit`, and `record_fact_source`.
- Diagnostic evidence plan: Add parser tests for `start_task.initial_fact_sources[].path` and `state_commit.fact_sources[].path`, then rerun `taskspace_control`, inspect missing fact-source, provider budget, and adaptive inspect budget tests.
- Status: confirmed.

# Evidence E-105: fact-source path normalization preserves artifact refs for parser and budget gates

- Prediction tested: H-048 predicts that inline path fields should become canonical evidence artifact refs before runtime records fact sources.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs`
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core fact_source_path_normalizes_to_artifact_ref --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget_expands_inspect_node_limit_for_fact_sources --lib
  ```
- Adjacent validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_control --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_request_budget --lib
  ```
- Result: passed.
- Matched test signals:
  - `start_task_fact_source_path_normalizes_to_artifact_ref` confirms `initial_fact_sources[].path` becomes `evidence_refs[].artifact_ref`.
  - `state_commit_fact_source_path_normalizes_to_artifact_ref` confirms mid-task fact-source updates preserve inline path artifacts.
  - Existing adaptive inspect budget test still expands a four-artifact inspect node limit from 5 to 10.
- Interpretation: H-048 is focused-fixed at the parser/contract boundary. Full R4-adjacent regression/build evidence is recorded in E-106; the remaining gate is attestation and a live keyed rerun to confirm the AT `node_request_count=6/5` stopper is crossed.

# Evidence E-106: fact-source path normalization passes R4-adjacent regression and build gates

- Prediction tested: H-048 repair should not regress TaskSpace control aliases, validation feedback, validation rework, patch recovery, provider budget, or active budget behavior.
- Commands:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_control --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_request_budget --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed.
- Matched counts:
  - `taskspace_control`: 35/35.
  - `inspect_missing_fact_sources`: 1/1.
  - `provider_request_budget`: 10/10.
  - `taskspace_active_budget`: 11/11.
  - `validation_`: 96/96.
  - `local_infra`: 11/11.
  - `validation_rework`: 18/18.
  - `apply_patch_`: 35/35.
  - `provider_budget`: 23/23.
  - fmt check, whitespace check, and `whale` build pass. The fmt command still prints the known stable rustfmt warning for `imports_granularity`.
- Interpretation: H-048 is regression-clean at focused/build level. The remaining evidence gate is build attestation and a live keyed rerun.

# Evidence E-107: c9e49ec live rerun crosses inspect fact-source budget and exposes rework target preview truncation

- Prediction tested: H-048 repair plus existing artifact extraction should let the live run move beyond the AT `node_request_count=6/5` inspect stopper.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703au-fact-source-path/runs/terminal_bench__organization-json-generator/20260704-101310-657
  PairReport: pair-001/pair-report.md
  preflight current_git_head: c9e49ec0df3f00acc37d64fb5ebace0834ae6e4d
  build_attestation_status: pass
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 13
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Result: the previous inspect budget stopper is crossed.
- Matched crossed signals:
  - The model used string fact sources, including `Expected files: schema.json, departments.csv, employees.csv, projects.csv`; this did not directly exercise object-form `path`, but it did preserve artifact extraction from fact-source descriptions.
  - Provider budget trace for `node-1` reported `max_model_requests_per_node:10`, not the prior base 5.
  - `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv` were all read successfully with `TaskSpaceReadFileSummaryV1`.
  - After all fact sources were read, a duplicate `schema.json` read triggered `TaskSpaceForcedInspectTransitionV1` into implementation rather than hard-stopping at `6/5`.
- New blocker signals:
  - Validation failed on schema-required fields: project `members` plus statistics `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - Runtime routed to validation rework and allowed one `process.py` target read (`result-12`).
  - Duplicate-read recovery included `result-12 artifacts=process.py`, but only as a single-line compact preview; the patch-relevant lower half of the file was truncated to `...`.
  - The model repeated `read_file process.py`, hit `TaskSpaceValidationReworkDuplicateReadRecoveryV1`, repeated again, then hit `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
- Interpretation: The new problem type is `validation-rework-target-read-preview-truncation`. The feedback says the target contents are already visible, but the recovery evidence is not self-sufficient for patching because it carries only a compact single-line preview.

# Hypothesis H-049: duplicate-read recovery needs multiline target excerpt, not only result id / compact preview

- Claim: When a validation rework node has already read the target artifact, duplicate-read recovery depends on `current_main_working_evidence_summary()`. That summary flattens working evidence through `single_line_preview(..., 1200)`, so patch-relevant target context can be truncated even though recovery says the current contents are already visible.
- Prediction: Adding a dedicated `validation_rework_target_read` evidence summary with a bounded multiline excerpt should make duplicate-read recovery self-sufficient for patch generation and reduce repeated target reads.
- Diagnostic evidence plan: Add a runtime regression where the patch-relevant marker is beyond the old 1200-char compact preview but below the new bounded target excerpt. The test must assert `current_main_working_evidence_summary()` includes `validation_rework_target_read`, `late_schema_repair_marker`, and the schema-required field names.
- Status: confirmed.

# Evidence E-108: validation rework target read now appears as bounded multiline evidence

- Prediction tested: H-049 predicts that current rework target contents must be available as a multiline evidence block, not only a result id or compact preview.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_projects_schema_repair_contract_from_schema_read --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_duplicate --lib
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed.
- Matched test signals:
  - `current_main_working_evidence_summary()` includes `validation_rework_target_read`.
  - The evidence summary retains `late_schema_repair_marker` placed beyond the old 1200-character compact preview.
  - The retained excerpt includes schema-required repair names such as `averageDepartmentBudget`.
- Interpretation: H-049 is focused-fixed with R4-adjacent regression/build coverage. The remaining gate is attestation and another keyed rerun to see whether the rework node now patches instead of repeating target reads.

# Evidence E-109: a625122 live rerun exposes non-forcing inspect duplicate-read recovery

- Prediction tested: H-049 repair should let the live run move past validation rework duplicate target reads if it reaches that phase again.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703av-rework-target-evidence/runs/terminal_bench__organization-json-generator/20260704-102918-033
  PairReport: pair-001/pair-report.md
  preflight current_git_head: a6251227d5a6c5204bcc8609fa499b1ba1a4c734
  build_attestation_status: pass
  outcome_standard: solved
  outcome_taskspace: wrong
  right_exec_timed_out: False
  right_tool_call_count: 10
  right_open_leaf_nodes: 1
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Result: the run did not reach the validation rework phase; H-049 was not live-exercised by this sample.
- New blocker signals:
  - `start_task.initial_fact_sources` correctly listed `departments.csv`, `employees.csv`, `projects.csv`, and `schema.json`.
  - Projection after reading `schema.json` correctly listed `read_file declared fact-source artifact departments.csv/employees.csv/projects.csv next`.
  - `TaskSpaceDuplicateReadSearchInspectRecoveryV1` correctly said the duplicate `schema.json` read was not new evidence and listed missing fact-source artifacts.
  - Despite that, the model repeated `read_file schema.json` seven times. The duplicate tool gate blocked those actions, but the provider loop kept consuming node requests.
  - The model eventually read `departments.csv` at logical request 11, then hit `TaskSpaceProviderBudgetHardStopV1` with `node_request_count=11/10`; `employees.csv` and `projects.csv` remained unread.
- Interpretation: The new problem type is `inspect-duplicate-read-recovery-nonforcing-budget-drain`. This is not artifact path loss and not missing next-action text; it is a control/feedback hardness gap where repeated blocked read/search actions can continue consuming provider budget even though the state machine already knows the bounded declared fact-source reads required next.

# Hypothesis H-050: repeated inspect duplicate-read recovery must execute bounded missing fact-source reads

- Claim: The existing recovery path only auto-bootstraps after repeated duplicate diagnostic actions. When the repeated blocked action is a duplicate read/search and declared fact sources are still missing, the runtime leaves recovery as advisory text. The model can ignore the advisory and drain provider/node budget without new evidence.
- Prediction: When repeated blocked read/search evidence is detected on an `inspect_code_context` node and runtime can name missing required fact-source artifacts, automatically running a bounded read of those declared artifacts will update inspect evidence coverage and prevent another duplicate-read budget drain at the same point.
- Diagnostic evidence plan: Add a focused regression proving bootstrap output with `===== employees.csv` / `===== projects.csv` sections is recorded as read evidence and clears `current_main_inspect_missing_required_fact_source_artifacts()`. Add a turn-layer regression proving the bootstrap command emits bounded `TaskSpaceReadFileSummaryV1` reads for declared artifacts.
- Status: confirmed.

# Evidence E-110: repeated duplicate inspect read now bootstraps missing declared fact sources

- Prediction tested: H-050 predicts bounded fact-source bootstrap results must be recorded as inspect read evidence, not just appended to chat history.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core missing_fact_source_bootstrap_command_reads_bounded_declared_artifacts --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources_shrink_after_bootstrap_read_sections --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_budget --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed.
- Matched test signals:
  - `taskspace_missing_fact_source_bootstrap_command()` includes bounded read commands and `TaskSpaceReadFileSummaryV1`.
  - A bootstrap shell result containing `===== employees.csv` and `===== projects.csv` clears the runtime missing fact-source list.
  - Existing duplicate read/search and provider budget regressions still pass.
- Interpretation: H-050 is focused-fixed with R4-adjacent regression/build coverage. The remaining gate is attestation and another keyed rerun to verify the live sample no longer drains inspect budget on repeated `schema.json`.

# Evidence E-111: cd00f0c live rerun crosses inspect drain and exposes implementation finish-without-edit drain

- Prediction tested: H-050 repair should let the live sample move beyond repeated `schema.json` inspect duplicate-read budget drain.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703aw-missing-fact-bootstrap/runs/terminal_bench__organization-json-generator/20260704-104628-266
  PairReport: pair-001/pair-report.md
  preflight current_git_head: cd00f0c2a87ef93f9536ce35d843b7be31cd90cf
  build_attestation_status: pass
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 7
  right_open_leaf_nodes: 1
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Result: supported the inspect-side H-050 live gate, but exposed a new implementation rework blocker.
- Matched signals:
  - TaskSpace read all declared fact-source artifacts directly: `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv`.
  - The correct schema validation command ran: `python generate_organization.py && python -m jsonschema -i organization.json schema.json`.
  - The validation failure was real implementation/schema mismatch: `statistics` used snake_case keys and missed required camelCase keys such as `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - The rework projection for `node-4` exposed `validation_schema_repair_contract`, named `generate_organization.py`, and said `do not taskspace_control(action=finish_node) until this node records a successful edit result`.
  - `whale-exec.jsonl` then showed repeated `taskspace_control finish_node` messages claiming the patch had succeeded, while no post-validation `apply_patch` action occurred.
  - The session inserted `TaskSpaceImplementNeedsEditRecoveryV1` attempts 2 through 8, then stopped only at `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5`.
- Interpretation: The new problem type is `implementation-rework-finish-without-edit-budget-drain`. The schema failure and next action were not missing. The semantics were present but not terminally enforced; the model converted its own rationale ("patch applied successfully") into a false fact, and the runtime kept issuing advisory recovery until the provider-node budget hard stop.

# Hypothesis H-051: implementation needs-edit recovery must hard-stop repeated finish attempts per node

- Claim: `TaskSpaceImplementNeedsEditRecoveryV1` currently remains advisory on an `implement_solution` node. If the provider repeatedly emits `finish_node` or equivalent progress claims without any successful edit result, the turn loop can keep sampling until the provider/node budget hard stop instead of producing bounded terminal feedback.
- Prediction: Source inspection will show that validation rework duplicate-read recovery already has a hard-stop path, while plain implementation needs-edit recovery only increments a counter and continues. A focused test can prove a per-node third plain needs-edit recovery produces a terminal `TaskSpaceImplementationNeedsEditHardStopV1` item that is not reclassified as another recovery item.
- Diagnostic evidence plan: Add a session-layer hard-stop item for repeated plain implementation needs-edit recovery, count attempts per current node so an earlier legitimate implement recovery does not poison later rework nodes, and validate with focused recovery tests plus R4-adjacent validation/rework/patch/budget regressions and a `whale` build.
- Status: confirmed.

# Evidence E-112: repeated implementation finish-without-edit now hard-stops before provider budget drain

- Prediction tested: H-051 predicts repeated plain implementation needs-edit recovery should terminate before another advisory sampling loop drains node budget.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - Added `TaskSpaceImplementationNeedsEditHardStopV1`.
  - Counts plain implementation needs-edit recovery per current node id.
  - On the third plain needs-edit recovery for the same implementation node, records bounded terminal recovery and stops provider sampling for the turn.
  - Leaves validation rework duplicate-read, failed edit, and patch grammar recovery paths intact.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit_hard_stop_triggers_on_third_plain_recovery --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework` 18/18, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `validation_` 96/96, `apply_patch_` 35/35, `taskspace_control` 35/35, and `local_infra` 11/11 all passed.
- Interpretation: H-051 is focused-fixed with regression/build coverage. The remaining live gate is attestation plus another keyed `organization-json-generator` rerun to see whether TaskSpace now patches `generate_organization.py` or exposes the next R4 tools-chain blocker.

# Hypothesis H-052: action-contract recent tool outputs must be scoped after latest active TaskSpace context

- Claim: In cache-optimized action-contract transport, `TaskSpaceActionContractRecentToolOutputsV1` currently selects tool outputs after the latest user message. A single long TaskSpace turn can cross multiple nodes under the same user message, so a successful edit from an earlier `implement_solution` node can remain provider-visible after a later validation rework node is active. This stale success can override the current node's `implementation_needs_edit` state and make the model reasonably choose `finish_node`.
- Prediction: In the AX2 live rerun, the current node will be `node-4`, `node-4` will have no successful edit, but model reasoning will cite a prior `apply_patch` success (`A generate_org_json.py`) and the progress hint "A file edit already succeeded" before trying to finish `node-4`.
- Diagnostic evidence plan: Inspect AX2 `rollout.jsonl` and `whale-exec.jsonl` for the current node, recent-output hint, assistant rationale, and runtime rejection. Then add a session-layer regression where an older active projection is followed by an apply_patch success, then a newer active projection; the prepared action-contract prompt must not include the stale success output.
- Status: confirmed.

# Evidence E-113: AX2 rerun shows stale prior-node edit success pollutes current implementation rework prompt

- Prediction tested: H-052 predicts the remaining live failure after E-112 is semantic distortion from stale recent-output scope, not missing gate semantics.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260703ax2-implement-needs-edit-hard-stop/runs/terminal_bench__organization-json-generator/20260704-110832-426
  PairReport: pair-001/pair-report.md
  preflight current_git_head: 212a1c27e64e737a35f8afd845209b0c49e3024b
  build_attestation_status: pass
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 12
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Matched signals:
  - The run crossed the previous inspect and validation routing blockers, then reached `node-4` as a validation rework `implement_solution` node.
  - `node-4` read `generate_org_json.py`, but no successful edit was recorded on `node-4`.
  - Before the final `finish_node`, model reasoning said the current node was `node-4` while also citing a recent `apply_patch` success: `Success. Updated the following files: A generate_org_json.py`.
  - The same reasoning followed the progress hint `A file edit already succeeded. Do not repeat apply_patch, read_file, or search. Next action must be taskspace_control with action=finish_node`.
  - Runtime correctly rejected that action with `TaskSpace implement_solution node node-4 cannot be completed without a recorded successful edit action`.
  - The new `TaskSpaceImplementationNeedsEditHardStopV1` from E-112 then stopped the turn before provider/node budget drain.
- Interpretation: The exact answer to the user question is: the current failure is semantic distortion, not pure missing semantics. The runtime gate correctly knows `node-4` has no edit, but the provider-visible recent-output aggregator leaks an older node's edit success into the current node, making the next-action hint wrong for the active node. The repair should scope recent tool outputs after the latest active TaskSpace context/projection, not merely after the latest user input.

# Evidence E-114: action-contract recent outputs are now scoped to the latest active context

- Prediction tested: H-052 predicts stale prior-node tool outputs should be excluded when a newer active TaskSpace context/projection appears after them.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - `prepare_taskspace_action_contract_prompt_items_for_node()` now tracks the index of the latest active TaskSpace context.
  - `TaskSpaceActionContractRecentToolOutputsV1` uses `max(latest_user_index, latest_active_context_index)` as the lower bound for included tool outputs.
  - Tool outputs after the latest active context remain visible, preserving normal same-node feedback behavior.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_recent_outputs_are_scoped_after_latest_active_context --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt_includes_recent_post_user_tool_output_summaries --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `action_contract_prompt` 28/28, `implementation_needs_edit` 3/3, `validation_rework` 18/18, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `validation_` 96/96, `apply_patch_` 35/35, `duplicate_read_search` 2/2, `inspect_missing_fact_sources` 2/2, `taskspace_control` 35/35, and `local_infra` 11/11 all passed. The fmt check still prints the known stable rustfmt `imports_granularity` warning.
- Interpretation: H-052 is focused-fixed with regression/build coverage. The remaining live gate is a new keyed `organization-json-generator` rerun to verify that `node-4` no longer receives stale prior-node edit-success guidance and either patches the schema mismatch or exposes the next R4 tools-chain blocker.

# Evidence E-115: 25e3fcb live rerun removes stale edit success and exposes current rework target read loss

- Prediction tested: H-052 predicts `node-4` should no longer receive stale prior-node edit-success guidance after scoping recent outputs to latest active context.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704ay-stale-recent-output-scope/runs/terminal_bench__organization-json-generator/20260704-112438-158
  PairReport: pair-001/pair-report.md
  preflight current_git_head: 25e3fcb8fca885146a939ca8a0868c2ea1877609
  build_attestation_status: pass
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 15
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Matched H-052 signals:
  - `whale-exec.jsonl` no longer contains `A file edit already succeeded`.
  - It no longer contains `TaskSpaceImplementationNeedsEditHardStopV1`.
  - The run reached validation rework `node-4`, read `generate_org.py`, and did not claim an edit succeeded on that node.
- New blocker signals:
  - After `node-4` read `generate_org.py`, the next model request did not receive the full read output as a recent tool output because the new latest active context was after the read output.
  - The active projection did contain `validation_rework_target_read result=result-11 artifact=generate_org.py`, but only as compact projected evidence.
  - Model reasoning said the result was only a summary and requested `read_file generate_org.py` again.
  - Runtime correctly blocked the duplicate read with `validation_rework_duplicate_artifact_read` and finally emitted `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
- Interpretation: H-052 is live-supported, but the first repair was too broad at the feedback-retention boundary. The next root-cause candidate is that current-node rework target read outputs must be retained across active context replacement when the latest projection explicitly references that target read.

# Hypothesis H-053: current validation rework target read output must survive active-context replacement

- Claim: Scoping all recent tool outputs after the latest active TaskSpace context prevents stale cross-node edit-success leakage, but it also drops the full read output for the current validation rework target when an active projection is recorded immediately after the read. The projection may name `validation_rework_target_read result=* artifact=*`, yet still provide only compact evidence, so the model asks to read the same target again.
- Prediction: A focused prompt-composition regression with `node-2 apply_patch success`, `node-4 generate_org.py read output`, and a latest `node-4` active projection naming `validation_rework_target_read result=result-11 artifact=generate_org.py` should keep the `generate_org.py` read output but exclude the stale `node-2` apply_patch success.
- Diagnostic evidence plan: Collect all tool outputs, then include only normal action-contract candidates after the latest active context plus whitelisted validation rework target read outputs referenced by the latest active context. Validate that stale edit-success progress hints stay absent.
- Status: confirmed.

# Evidence E-116: current rework target read outputs are retained while stale edit success remains excluded

- Prediction tested: H-053 predicts the prompt composer must preserve the current validation rework target read output across active context replacement, without reintroducing stale prior-node apply_patch success.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - Prompt composition now collects all tool outputs initially.
  - Normal recent tool output summaries still require the existing action-contract candidate filter and latest active-context floor.
  - A narrow exception includes tool outputs whose text contains `TaskSpaceReadFileSummaryV1: path=<artifact>` when the latest active context explicitly names `validation_rework_target_read ... artifact=<artifact>`.
  - Stale apply_patch success from an older active context is still excluded and cannot generate `A file edit already succeeded`.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_keeps_current_rework_target_read_across_latest_context --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_recent_outputs_are_scoped_after_latest_active_context --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `action_contract_prompt` 28/28, `validation_rework` 18/18, `validation_` 96/96, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `apply_patch_` 35/35, `taskspace_control` 35/35, and `local_infra` 11/11 all passed. The fmt command still prints the known stable rustfmt warning for `imports_granularity`.
- Interpretation: H-053 is focused-fixed with regression/build coverage. The remaining live gate is another keyed rerun to see whether TaskSpace now patches `generate_org.py` instead of repeatedly reading it.

# Evidence E-117: 9d0be48 live rerun clears target-read loss and exposes pytest command normalization / runner misroute

- Prediction tested: H-053 predicts the live sample should stop repeating the current validation rework target read after full target read output is retained.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704az-rework-target-read-retained/runs/terminal_bench__organization-json-generator/20260704-113802-241
  PairReport: pair-001/pair-report.md
  preflight current_git_head: 9d0be484b6638d9dd66b07f2435b63d8d4170aa4
  build_attestation_status: pass
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 13
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Matched H-053 signals:
  - The live trace did not recur into `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
  - The run advanced past the prior repeated target-read failure and reached a later validation path.
- New blocker signals:
  - The assistant emitted `run_test` with command `python -m pytest test_organization.py -v`.
  - Action-contract normalization executed `/bin/bash -lc 'pytest tests/test_organization.py -v'`, dropping the `python -m` runner prefix while adding the `tests/` path.
  - The shell returned `/bin/bash: line 1: pytest: command not found`.
  - Runtime did not classify the bare pytest runner failure as local validator infrastructure. It inserted `TaskSpaceImplementNeedsEditRecoveryV1`, then the rework path read `generate_organization.py`, tried to read missing `tests/test_organization.py`, and finally hit `TaskSpaceImplementationNeedsEditHardStopV1`.
- Interpretation: H-053 is live-supported, and the next problem type is `validation-pytest-command-normalization-and-runner-misroute`. This is a combined ability-layer and feedback-layer issue: command normalization must preserve the selected pytest runner, and bare `pytest: command not found` must route like the already-fixed `python -m pytest` missing-module runner dependency instead of implementation rework.

# Hypothesis H-054: pytest command normalization must preserve runner prefix and bare pytest missing runner must remain validation infra

- Claim: `normalize_taskspace_action_contract_test_command()` currently recognizes `python -m pytest` and `cd src && python -m pytest`, but reconstructs every normalized test-file command as bare `pytest ...`. When bare pytest is unavailable, `text_mentions_missing_pytest_runner_dependency()` does not recognize shell `pytest: command not found`, so the failure is treated as a validator/test failure that can create implementation rework.
- Prediction:
  1. A focused action-contract test with command `python -m pytest test_tax_calc.py -v` should currently normalize to bare `pytest tests/test_tax_calc.py -v`; after repair it must normalize to `python -m pytest tests/test_tax_calc.py -v`.
  2. A runtime test with body `command: pytest tests/test_organization.py -v` and output `/bin/bash: line 1: pytest: command not found` should currently create implementation rework or avoid local-infra invalidation; after repair it must mark the result invalid/local validator infra and create a smoke-test rerun whose origin is the failed validation node.
- Diagnostic evidence plan: Repair the command normalizer to rebuild the command with its matched runner prefix, then extend missing-pytest runner dependency detection to shell command-not-found variants. Validate with focused tests plus `validation_`, `local_infra`, `validation_rework`, `action_contract_prompt`, provider/budget regressions, format, whitespace, and `whale` build.
- Status: confirmed.

# Evidence E-118: pytest command normalization and bare runner infra routing are focused-fixed

- Prediction tested: H-054 predicts pytest test-file path normalization should not change the runner prefix, and shell `pytest: command not found` should route as local validator infrastructure rather than implementation rework.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Repair behavior:
  - `normalize_taskspace_action_contract_test_command()` now rebuilds normalized pytest commands with the matched prefix (`pytest`, `python -m pytest`, or `cd src && python -m pytest`) while still resolving missing bare test-file paths to `tests/<file>`.
  - Added a focused action-contract test proving `python -m pytest test_tax_calc.py -v` becomes `python -m pytest tests/test_tax_calc.py -v`.
  - `text_mentions_missing_pytest_runner_dependency()` now recognizes shell runner-missing forms such as `pytest: command not found`, `pytest: not found`, `command not found: pytest`, and Windows `pytest is not recognized`, gated by an explicit pytest runner command mention.
  - Added a runtime regression proving `command: pytest tests/test_organization.py -v` plus `/bin/bash: line 1: pytest: command not found` marks the result invalid/local validator infra, blocks the failed validation node, creates a smoke-test rerun with the validation node as origin, and does not set implementation needs-edit.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_action_contract_run_test_preserves_python_m_pytest_prefix --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core bare_pytest_command_not_found_routes_to_validation_rerun_not_implementation --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_action_contract_run_test --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core pytest_runner_dependency --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `taskspace_action_contract_run_test` 5/5, `local_infra` 11/11, `action_contract_prompt` 28/28, `validation_rework` 18/18, `validation_` 97/97, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `implementation_needs_edit` 3/3, `apply_patch_` 35/35, `duplicate_read_search` 2/2, `missing_fact_source_bootstrap` 1/1, `inspect_missing_fact_sources` 2/2, and `taskspace_control` 35/35 all passed. The fmt check still prints the known stable rustfmt `imports_granularity` warning.
- Interpretation: H-054 is focused-fixed with regression/build coverage. The remaining live gate is another keyed `organization-json-generator` rerun to verify the sample moves beyond pytest runner infra and either reaches a schema-correct `organization.json` or exposes the next R4 tools-chain blocker.

# Evidence E-119: 9f08638 live rerun exposes duplicate list_files data bootstrap gap before pytest

- Prediction tested: H-054 should be live-gated by another keyed `organization-json-generator` rerun on an attested `9f08638` binary.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704ba-pytest-runner-feedback/runs/terminal_bench__organization-json-generator/20260704-115228-006
  PairReport: pair-001/pair-report.md
  preflight current_git_head: 9f086386e3a8baeba5f1387bb179b4f1306e1895
  build_attestation_status: pass
  outcome_standard: solved
  outcome_taskspace: wrong
  right_exec_timed_out: False
  right_tool_call_count: 6
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  right_maps: 1
  right_nodes: 1
  right_open_leaf_nodes: 1
  ```
- Result: inconclusive for H-054 live utility because the run failed earlier than pytest validation. It did not reproduce the pytest runner blocker.
- New blocker signals:
  - First inspect action `list_files "."` executed as `rg --files .` and successfully listed `employees.csv`, `departments.csv`, `projects.csv`, `schema.json`, `task.yaml`, and related artifacts.
  - The model repeated `list_files "."` five more times.
  - Runtime blocked each repeat as `inspect_duplicate_successful_read_or_search`, which is correct.
  - The recovery remained advisory/action-nonforcing: it did not execute bounded reads of the data artifacts named in the file-list result, and existing missing fact-source bootstrap did not trigger because these artifacts were only present in success criteria/list output, not structured `initial_fact_sources`.
  - The inspect node hit `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded node_request_count=6/5`.
- Interpretation: This is a new feedback/control gap named `inspect-duplicate-list-files-data-bootstrap-gap`. The runtime is correct not to finish inspect from file-list evidence alone, but the session fallback must turn repeated duplicate list/search/read loops into bounded source/test/data artifact evidence rather than merely telling the model not to repeat.

# Hypothesis H-055: repeated duplicate inspect read/search should trigger bounded data artifact bootstrap when force-finish is unsafe

- Claim: The session-level no-action recovery path only runs `TaskSpaceRepeatedBlockedInspectBootstrapV1` for repeated blocked diagnostic commands, not repeated duplicate read/search gates. The static bootstrap command also only reads `*.py`, `*.md`, and `*.txt`, so data-processing samples with `schema.json` and `*.csv` can remain stuck after a successful `rg --files` result.
- Prediction:
  1. Source inspection will show that when `taskspace_message_has_repeated_blocked_action()` is true and the reason is `inspect_duplicate_successful_read_or_search`, the code attempts force-finish and then falls back to another recovery item without running bootstrap.
  2. A focused session test should prove repeated duplicate read/search recovery with unchanged progress and unsafe force-finish executes a bootstrap command instead of issuing only advisory recovery.
  3. The bootstrap command should include bounded `*.json`, `*.csv`, `*.yaml`, and `*.yml` reads in addition to existing source/test text globs.
- Diagnostic evidence plan: Extend the repeated blocked inspect bootstrap command to bounded source/test/data artifact globs, and run it when repeated duplicate read/search cannot safely force-finish. Validate with focused bootstrap tests plus duplicate/read-search, missing fact-source, taskspace control, budget, validation, format, whitespace, and `whale` build.
- Status: confirmed.

# Evidence E-120: repeated duplicate inspect read/search now triggers bounded source/test/data bootstrap

- Prediction tested: H-055 predicts repeated duplicate read/search should not continue advisory recovery until node budget hard-stop when inspect cannot safely finish from existing evidence.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - `TaskSpaceRepeatedBlockedInspectBootstrapV1` now reads bounded source/test/data artifacts by globbing `*.py`, `*.md`, `*.txt`, `*.json`, `*.csv`, `*.yaml`, and `*.yml`.
  - The command remains bounded: Unix uses `head -n 12` and `sed -n '1,120p'`; Windows uses `Select-Object -First 12` and `Get-Content -TotalCount 120`.
  - Added `taskspace_repeated_duplicate_read_search_should_bootstrap()` so the session fallback can distinguish repeated duplicate read/search from duplicate diagnostics.
  - When repeated `inspect_duplicate_successful_read_or_search` cannot force-finish inspect, the session now executes the bootstrap command instead of emitting only another advisory recovery item.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core repeated_duplicate_read_search_triggers_inspect_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core repeated_blocked --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `duplicate_read_search` 3/3, `missing_fact_source_bootstrap` 1/1, `inspect_missing_fact_sources` 2/2, `taskspace_control` 35/35, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `action_contract_prompt` 28/28, `validation_rework` 18/18, `validation_` 97/97, `local_infra` 11/11, `apply_patch_` 35/35, and `implementation_needs_edit` 3/3 all passed. The fmt check still prints the known stable rustfmt `imports_granularity` warning.
- Interpretation: H-055 is focused-fixed with regression/build coverage. The remaining live gate is commit/push, attestation, and another keyed `organization-json-generator` rerun to verify the repeated `list_files` budget drain is gone.

# Evidence E-121: 807a3cf live rerun clears early list_files drain and exposes post-target-read patch-only feedback gap

- Prediction tested: H-055 should let the live `organization-json-generator` sample move beyond repeated `list_files` inspect budget drain after an attested `807a3cf` binary.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bb-data-bootstrap/runs/terminal_bench__organization-json-generator/20260704-120401-124
  PairReport: pair-001/pair-report.md
  preflight current_git_head: 807a3cf5802e59688bb911fe69216e314f4e33ff
  build_attestation_status: pass
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  right_maps: 1
  right_nodes: 4
  right_open_leaf_nodes: 1
  ```
- Result: partially supports H-055 live utility. The exact BA repeated `list_files` drain did not recur, but the run did not directly execute `TaskSpaceRepeatedBlockedInspectBootstrapV1` because the model declared and read `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv` without repeating `list_files`.
- New blocker signals:
  - The validation node eventually executed `python generate_org.py && python -m jsonschema -i organization.json schema.json`.
  - The validator produced real implementation/schema failures: every project emitted `member_ids` instead of required `members`, and `statistics` used snake_case fields while missing `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - The rework node read `generate_org.py` successfully; `TaskSpaceReadFileSummaryV1` reported `lines_read=89 eof_reached=true max_lines=240`.
  - A duplicate `read_file generate_org.py` then produced `TaskSpaceValidationReworkDuplicateReadRecoveryV1`, correctly saying target contents were already visible and the next action must be `apply_patch` or `block_node`.
  - The next model action was `read_file schema.json`. Action contract rejected it as generic `node_policy_violation:implement_solution:read_file:implementation_needs_edit`, and session emitted generic `TaskSpaceImplementationNeedsEditHardStopV1 reason=repeated_finish_without_successful_edit`.
- Diagnostic source evidence:
  - `rollout.jsonl` line 432 showed the provider prompt already contained `validation_schema_repair_contract`, `validation_rework_target_read result=result-12 artifact=generate_org.py`, schema/source CSV evidence, `TaskSpaceGateRecoveryV1 reason=validation_rework_duplicate_artifact_read`, and the explicit instruction: do not call `read_file`, `list_files`, `search`, broad discovery, schema inspection, or validation before a successful edit.
  - `rollout.jsonl` line 457 showed the active projection narrowed the current node contract to `edit, control(block_node only; finish_node blocked until successful edit; read/search of already visible validation rework targets will be blocked)`.
- Interpretation: This is semantic distortion, not semantic loss. The needed repair facts and patch-only contract were present, but the session/action-contract feedback collapsed the later off-contract schema read into generic `implementation_needs_edit` instead of preserving the specific `validation_rework_patch_only_after_target_read` state. The new problem type is `validation-rework-patch-only-after-target-read-feedback`.

# Hypothesis H-056: post-target-read validation rework needs a patch-only recovery/hard-stop semantic

- Claim: After validation rework has both a schema repair contract and a visible target-file read, any subsequent read/search/schema inspection before a successful edit violates a stricter patch-only state. The session currently treats the resulting action-contract rejection as ordinary `implementation_needs_edit`, so logs and recovery semantics no longer explain that the runtime has crossed from "read target once" to "patch or block only".
- Prediction:
  1. A focused session test with generic `implementation_needs_edit` rejection plus evidence containing `validation_rework_target_read result=... artifact=generate_org.py` should build `TaskSpaceValidationReworkPatchOnlyRecoveryV1` with `failure_kind: validation_rework_patch_only_after_target_read`, not generic `TaskSpaceImplementNeedsEditRecoveryV1`.
  2. The first patch-only recovery should give one more chance to emit `apply_patch` or `block_node`.
  3. A second patch-only recovery on the same turn should produce `TaskSpaceValidationReworkPatchOnlyHardStopV1 reason=repeated_non_edit_after_validation_rework_target_read`, not the generic implementation needs-edit hard stop.
- Diagnostic evidence plan: Add session focused tests for the recovery selector and hard-stop trigger, then run R4-adjacent regression groups for validation rework, implementation needs-edit, action-contract prompt, provider/budget control, formatting, whitespace, and the `whale` build. A live rerun must verify whether the sample now either patches the schema mismatch after the new patch-only recovery or exposes the next R4 tools-chain problem.
- Status: confirmed.

# Evidence E-122: validation rework patch-only feedback is focused-fixed with R4 regression coverage

- Prediction tested: H-056 predicts the session should preserve the post-target-read patch-only semantic instead of collapsing it into plain implementation needs-edit.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - Added `TaskSpaceValidationReworkPatchOnlyRecoveryV1` with `failure_kind: validation_rework_patch_only_after_target_read`.
  - `build_taskspace_implementation_recovery_item()` now detects evidence containing `validation_rework_target_read` and emits the patch-only recovery instead of generic `TaskSpaceImplementNeedsEditRecoveryV1`.
  - The first patch-only recovery gives one more provider chance to emit `apply_patch` or `block_node`.
  - Added `TaskSpaceValidationReworkPatchOnlyHardStopV1 reason=repeated_non_edit_after_validation_rework_target_read` for a second same-turn post-target-read non-edit/discovery violation.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_recovery_selects_patch_only_after_target_read_evidence --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_patch_only_hard_stops_after_one_recovery --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_recovery_does_not_enter_patch_only_before_target_read --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_recovery --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_duplicate_read --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `implementation_recovery` 5/5, `implementation_needs_edit` 3/3, `validation_rework_duplicate_read` 6/6, `validation_rework` 19/19, `action_contract_prompt` 28/28, `validation_` 98/98, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `duplicate_read_search` 3/3, `missing_fact_source_bootstrap` 1/1, `inspect_missing_fact_sources` 2/2, `local_infra` 11/11, `apply_patch_` 35/35, and `taskspace_control` 35/35 all passed. The fmt check still prints the known stable rustfmt `imports_granularity` warning. `git diff --check` and the `whale` build passed.
- Interpretation: H-056 is focused-fixed with R4-adjacent regression/build coverage. The remaining live gate is commit/push, binary attestation, and another keyed `organization-json-generator` rerun to verify whether the model patches after the new patch-only recovery or exposes the next tools-chain blocker.

# Evidence E-123: f5ba9ed live rerun exposes accepted blocker despite visible validation rework evidence

- Prediction tested: H-056 should preserve the patch-only semantic in a live keyed `organization-json-generator` rerun and either force an edit or expose the next tools-chain blocker.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bc-patch-only-feedback/runs/terminal_bench__organization-json-generator/20260704-122254-562
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 6
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Matched H-056 live signals:
  - `whale-exec.jsonl` recorded `TaskSpaceValidationReworkPatchOnlyRecoveryV1`, so the specialized post-target-read recovery was emitted.
  - `whale-exec.jsonl` recorded `TaskSpaceReadFileSummaryV1: path=process.py lines_read=83 eof_reached=true max_lines=240`.
  - The provider prompt contained `validation_rework_target_read result=result-7 artifact=process.py`, schema evidence from `result-1 artifacts=schema.json`, and next-valid-action text requiring `apply_patch process.py`.
- New blocker signal:
  - The model emitted `taskspace_control block_node` with reason `Need to view full current process.py content and schema.json to apply correct fix for smoke test failure`.
  - Its rationale claimed the current projection excerpt was incomplete and that it could not construct an accurate patch without seeing remaining code and schema definition.
  - Runtime accepted the blocker, leaving `provider_request_context_missing_reason:current_main_node_missing`, then the final message became `blocked_by_taskspace_action_contract: local infrastructure evidence prevents execution...`.
- Interpretation: This is a feedback-layer semantic recognition gap, not a model-context loss. The required source/schema evidence and patch-only contract were present, but `blocker_claims_missing_inspected_source_evidence()` did not classify `need to view full current ...` / `remaining code` / `request read access` as a missing-visibility blocker, so `block_main_node()` accepted a blocker that contradicted already-visible evidence.

# Hypothesis H-057: validation rework must reject missing-visibility block_node wording after target read

- Claim: `blocker_claims_missing_inspected_source_evidence()` recognizes missing-source blocker wording such as `need to read`, `need to inspect`, `not visible`, and `cannot construct`, but misses equivalent live wording such as `need to view full current <target>`, `without seeing remaining code`, and `request read access`. In validation rework nodes with dependency repair evidence and no successful edit, that leak lets the model close the node instead of applying the required patch.
- Prediction:
  1. A focused runtime test in the post-target-read validation rework state should currently accept the live blocker reason `Need to view full current generate_org.py content and schema.json...`; after repair it must return an error containing `missing source visibility` and `apply_patch`.
  2. Extending the missing-visibility classifier with these equivalent phrases should not change the legitimate failed-edit refresh path, where the same target may be read once after a failed edit made context stale.
  3. `validation_rework`, `taskspace_control`, and R4-adjacent suites should continue to pass.
- Diagnostic evidence plan: Add the live wording to the existing validation rework target-read test before the failed-edit branch, extend the blocker classifier phrase set, then run focused validation rework and taskspace-control regressions plus adjacent action-contract, provider/budget, apply-patch, formatting, whitespace, and `whale` build checks.
- Status: confirmed.

# Evidence E-124: visible-evidence blocker guard is focused-fixed with R4 regression coverage

- Prediction tested: H-057 predicts the live post-target-read blocker wording must be rejected as missing source visibility, while existing validation rework and failed-edit refresh behavior remains intact.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Repair behavior:
  - `blocker_claims_missing_inspected_source_evidence()` now treats `need to view`, `view full`, `full current`, `without seeing`, `remaining code`, and `request read access` as equivalent missing-visibility blocker claims.
  - `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback` now includes the live-style blocker reason `Need to view full current generate_org.py content and schema.json...` after a complete target read and before any successful edit.
  - The focused regression asserts the blocker is rejected with `missing source visibility` and `apply_patch`.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework` 19/19, `taskspace_control` 35/35, `implementation_needs_edit` 3/3, `action_contract_prompt` 28/28, `validation_` 98/98, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `apply_patch_` 35/35, `local_infra` 11/11, `duplicate_read_search` 3/3, `missing_fact_source_bootstrap` 1/1, and `inspect_missing_fact_sources` 2/2 all passed. The fmt check still prints the known stable rustfmt `imports_granularity` warning. `git diff --check` and the `whale` build passed.
- Interpretation: H-057 is focused-fixed. The remaining live gate is commit/push, binary attestation, and another keyed `organization-json-generator` rerun to see whether TaskSpace now patches after the patch-only recovery or exposes the next unresolved tools-chain case.

# Evidence E-125: 7bf1485 live rerun clears visible-evidence blocker and exposes read-summary template shadowing

- Prediction tested: H-057 should prevent the live post-target-read `block_node` from closing the rework node.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bd-visible-evidence-blocker/runs/terminal_bench__organization-json-generator/20260704-123716-651
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 12
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Matched H-057 live signals:
  - The previous `Need to view full current process.py...` blocker did not recur.
  - TaskSpace progressed through all declared source data reads, generated `generate_org.py`, ran schema validation, entered rework, and preserved `TaskSpaceValidationReworkPatchOnlyRecoveryV1`.
- New blocker signals:
  - The first rework target read result contained the actual final summary `TaskSpaceReadFileSummaryV1: path=generate_org.py lines_read=108 eof_reached=true max_lines=240`.
  - The provider prompt nevertheless rendered `read_context: read_summary; TaskSpaceReadFileSummaryV1: path=%s lines_read=%d eof_reached=%s max_lines=240...`, because the shell command field also contains an awk `printf` template with the same marker string.
  - The model then reasoned that the current read summary was truncated and repeated `read_file generate_org.py` twice, ending at `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
- Interpretation: H-057 is live-cleared, and the next problem type is `read-summary-command-template-shadowing`. The runtime had complete file evidence, but read-summary extraction selected the command-template marker before the raw-output summary, changing `complete_read/eof_reached=true` into ambiguous `read_summary/path=%s`. This is a feedback-layer semantic distortion that directly explains the repeated target read.

# Hypothesis H-058: read summary extraction must prefer raw-output summary over command templates

- Claim: `read_file_summary_line()` scans a tool-result body from top to bottom. Action-contract `read_file` shell commands include an awk format string containing `TaskSpaceReadFileSummaryV1: path=%s lines_read=%d eof_reached=%s...`, so the parser can capture that template instead of the actual appended raw-output summary. That prevents `validation_rework_read_context_status()` from reporting `complete_read`, and the model keeps requesting a full file despite `eof_reached=true` being available later in the same result body.
- Prediction:
  1. A focused unit body containing both the command template marker and a final actual summary should currently classify as `read_summary` or include `path=%s`; after repair it must select the final actual summary and report `complete_read`.
  2. Existing validation rework target-read tests should still show `complete_read`, `eof_reached=true`, no repeated target read next action, and the failed-edit refresh path unchanged.
  3. R4-adjacent validation/session/build regressions should continue to pass.
- Diagnostic evidence plan: Change summary extraction to prefer the last marker line in the body, add a focused parser regression for command-template shadowing, then rerun validation rework, validation/session/control, provider/budget, formatting, whitespace, and `whale` build checks. A later live rerun must verify the prompt no longer shows `path=%s` in `read_context`.
- Status: confirmed.

# Evidence E-126: read-summary command-template shadowing is focused-fixed

- Prediction tested: H-058 predicts the read summary parser should prefer the raw-output summary appended at the end of the tool body over the command field's awk format template.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Repair behavior:
  - `read_file_summary_line()` now scans body lines from the end toward the beginning, so the actual appended summary wins over earlier command-template marker strings.
  - Added `read_file_summary_prefers_actual_output_over_command_template`, which includes both `path=%s lines_read=%d eof_reached=%s` in the command field and `path=generate_org.py lines_read=108 eof_reached=true` in raw output.
  - The focused regression asserts `read_file_summary_eof_reached()` returns `Some(true)` and `validation_rework_read_context_status()` starts with `complete_read`, with no `path=%s` leakage.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core read_file_summary_prefers_actual_output_over_command_template --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `validation_rework` 19/19, `taskspace_control` 35/35, `implementation_needs_edit` 3/3, `action_contract_prompt` 28/28, `validation_` 98/98, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `apply_patch_` 35/35, `local_infra` 11/11, `duplicate_read_search` 3/3, `missing_fact_source_bootstrap` 1/1, and `inspect_missing_fact_sources` 2/2 all passed. The fmt check still prints the known stable rustfmt `imports_granularity` warning. `git diff --check` and the `whale` build passed.
- Interpretation: H-058 is focused-fixed. The remaining live gate is commit/push, binary attestation, and another keyed `organization-json-generator` rerun to verify the provider prompt shows `complete_read` instead of `path=%s` and whether the sample advances to an edit.

# Evidence E-127: fd5c705 live rerun shows read-summary parser fix was necessary but insufficient

- Prediction tested: H-058 should make the live provider prompt use the actual raw-output read summary instead of the awk command template.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704be-read-summary-template/runs/terminal_bench__organization-json-generator/20260704-124752-951
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Matched H-058 focused signal:
  - The parser regression still passes when both command-template marker and actual raw-output summary are present in the same body.
- New blocker signals:
  - `whale-exec.jsonl` recorded the actual final summary `TaskSpaceReadFileSummaryV1: path=process_csv.py lines_read=92 eof_reached=true max_lines=240`.
  - The active projection still rendered `read_context: read_summary; TaskSpaceReadFileSummaryV1: path=%s lines_read=%d eof_reached=%s max_lines=240...`.
  - The stored ActionMap result body for the rework target read contained the shell command field and a truncated `preview:` ending at `[... telemetry preview truncated ...]`, but not the actual final summary line.
  - The model repeated `read_file process_csv.py`, received duplicate-read recovery, and ultimately hit `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
- Interpretation: H-058 fixed the parser behavior for complete stored bodies, but live ActionMap persistence was already losing the real summary before the parser ran. The new problem type is `read-summary-preview-tail-loss`: a feedback-layer semantic field is missing, and the remaining command template becomes the only visible marker.

# Hypothesis H-059: read_file preview truncation must preserve the structured read summary sentinel

- Claim: `tool_output_model_visible_preview()` applies telemetry head truncation before `taskspace_tool_result_preview_with_invocation()` stores a TaskSpace tool result in ActionMap. For action-contract `read_file`, the shell command itself contains an awk `TaskSpaceReadFileSummaryV1: path=%s...` template near the top, while the actual `eof_reached=true/false` summary appears at the raw output tail. When telemetry truncation drops the tail, ActionMap records an ambiguous template-only result, so validation rework cannot prove the target file was completely read.
- Prediction:
  1. A focused exec-output preview test with a large body and a final `TaskSpaceReadFileSummaryV1: ... eof_reached=true` should preserve that exact summary after the telemetry truncation notice.
  2. The action-contract recent tool output summarizer should preserve the same summary after its 2400-character truncation.
  3. Runtime summary parsing should prefer any parseable `eof_reached=true/false` summary over later command-template markers.
  4. Existing schema semantic summary preservation, validation rework target-read tests, and R4-adjacent suites should remain green.
- Diagnostic evidence plan: Add a shared preview tail-sentinel helper that extracts only parseable read summaries, apply it to model-visible tool preview and recent feedback truncation, harden runtime parsing, then run focused preview/recent/parser tests plus R4-adjacent validation/session/build gates.
- Status: confirmed.

# Evidence E-128: read-summary preview tail loss is focused-fixed

- Prediction tested: H-059 predicts telemetry/recent-output truncation must preserve the real read summary sentinel and ignore non-parseable command templates.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/tools/mod.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/context.rs`
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Repair behavior:
  - Added `append_taskspace_tool_tail_sentinels()`, which extracts the last `TaskSpaceReadFileSummaryV1` line containing `eof_reached=true` or `eof_reached=false` and appends it after truncated previews as `TaskSpaceToolTailSentinelV1`.
  - `tool_output_model_visible_preview()` and `response_input_model_visible_preview()` now preserve that sentinel before ActionMap records the result.
  - `taskspace_action_contract_recent_tool_outputs_item()` now preserves the same sentinel after recent-output truncation.
  - `read_file_summary_line()` now prefers parseable read summaries over non-parseable command templates.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_preview_preserves_read_file_summary_after_telemetry_truncation --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_recent_output_preserves_truncated_read_summary --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core read_file_summary_prefers_parseable_output_over_later_command_template --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core read_file_summary_prefers_actual_output_over_command_template --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_preview_preserves_required_properties_from_untruncated_exec_output --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_preview_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core read_file_summary_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `taskspace_preview_` 3/3, `read_file_summary_` 3/3, `validation_rework` 19/19, `taskspace_control` 35/35, `implementation_needs_edit` 3/3, `action_contract_prompt` 28/28, `validation_` 98/98, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `apply_patch_` 35/35, `local_infra` 11/11, `duplicate_read_search` 3/3, `missing_fact_source_bootstrap` 1/1, and `inspect_missing_fact_sources` 2/2 all passed. The fmt check still prints the known stable rustfmt `imports_granularity` warning. `git diff --check` and the `whale` build passed.
- Interpretation: H-059 is focused-fixed with R4-adjacent regression/build coverage. The remaining gate is commit/push, attestation, and keyed rerun to verify live prompts now retain `complete_read/eof_reached=true` and advance beyond duplicate re-read.

# Evidence E-129: e70aeed live rerun clears read-summary tail loss but exposes non-forcing complete-read recovery

- Prediction tested: H-059 should make live prompts retain the real target-read summary after preview truncation and prevent `path=%s` template shadowing.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bf-preview-tail-sentinel/runs/terminal_bench__organization-json-generator/20260704-130600-674
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  included_in_utility_aggregate: False
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 13
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  ```
- Matched H-059 live signals:
  - Provider-visible developer context contained `validation_rework_target_read result=result-11 artifact=process.py`.
  - The same context contained `read_context: complete_read; TaskSpaceReadFileSummaryV1: path=process.py lines_read=97 eof_reached=true max_lines=240`.
  - The schema read ActionMap result also retained `TaskSpaceToolTailSentinelV1` after telemetry preview truncation.
- New blocker signals:
  - After `TaskSpaceValidationReworkPatchOnlyRecoveryV1` required exactly one `apply_patch` or precise `block_node`, the model still emitted `read_file process.py`.
  - `TaskSpaceValidationReworkDuplicateReadRecoveryV1` correctly reported `Result result-11 is a complete read_file context (TaskSpaceReadFileSummaryV1 eof_reached=true; no additional file lines are hidden)` and repeated the repair contract.
  - The next model action repeated `read_file process.py` with rationale `Need full content of process.py to construct an accurate patch for adding missing schema properties; the projected excerpt is insufficient.`
  - The turn ended at `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
- Interpretation: H-059 is live-cleared. The next problem type is `validation-rework-complete-read-duplicate-read-nonforcing`: complete-read feedback is now semantically correct and visible, but session recovery still gives the model another advisory round before hard-stop, so an already-invalid duplicate read can consume one more provider request and delay the required edit/block action.

# Hypothesis H-060: complete-read duplicate validation-rework reads must hard-stop immediately

- Claim: In validation rework, once the target artifact has a complete read summary (`TaskSpaceReadFileSummaryV1 eof_reached=true`) and no successful edit has been recorded, a repeated `read_file` for the same artifact has no valid information gain. Treating the first duplicate complete read as advisory recovery lets the model reinterpret the visible `excerpt` label as partial evidence and retry, even though runtime already knows no additional file lines are hidden.
- Prediction:
  1. A focused session test with duplicate-read recovery text containing `complete read_file context` / `eof_reached=true` should hard-stop immediately at previous duplicate recovery count `0`.
  2. The existing generic duplicate-read behavior without complete-read evidence should still allow one advisory recovery before hard-stop.
  3. Patch-only recovery text should explicitly state that `complete_read` / `eof_reached=true` means no hidden lines remain, so the model must not treat the displayed target-read excerpt as partial evidence.
  4. Validation rework, action contract, taskspace control, provider/budget, apply-patch, formatting, whitespace, and `whale` build regressions should continue to pass.
- Diagnostic evidence plan: Add complete-read detection to session duplicate-read hard-stop classification, add focused tests for immediate hard-stop and patch-only wording, then rerun R4-adjacent regressions and a keyed `organization-json-generator` rerun.
- Status: confirmed.

# Evidence E-130: complete-read duplicate recovery is focused-fixed with R4 regression coverage

- Prediction tested: H-060 predicts complete-read duplicate validation-rework reads should hard-stop immediately, while generic duplicate-read recovery and failed-edit refresh behavior remain intact.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - `taskspace_validation_rework_duplicate_read_should_hard_stop()` now treats duplicate-read recovery containing `complete read_file context` or `read_context: complete_read` as terminal even when duplicate recovery count is still `0`.
  - `TaskSpaceValidationReworkPatchOnlyRecoveryV1` now explicitly says `complete_read` / `eof_reached=true` means no additional file lines are hidden and the displayed target-read excerpt must not be treated as partial evidence.
  - Added `validation_rework_duplicate_read_complete_context_hard_stops_immediately`.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_duplicate_read_complete_context_hard_stops_immediately --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_duplicate_read --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. Pre-format regression tests passed through `validation_rework_duplicate_read` 7/7, `validation_rework` 20/20, `taskspace_control` 35/35, `implementation_needs_edit` 3/3, `action_contract_prompt` 28/28, `validation_` 99/99, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `apply_patch_` 35/35, `local_infra` 11/11, `duplicate_read_search` 3/3, `missing_fact_source_bootstrap` 1/1, and `inspect_missing_fact_sources` 2/2. The first full command stopped only because `cargo fmt --check` required a one-line rustfmt reflow. After running `cargo fmt`, the focused complete-context test, `cargo fmt --check`, `git diff --check`, and `whale` build all passed. The fmt command still prints the known stable rustfmt `imports_granularity` warning.
- Interpretation: H-060 is focused-fixed with R4-adjacent regression/build coverage. The remaining live gate is commit/push, binary attestation, and another keyed `organization-json-generator` rerun to verify the turn now stops immediately on complete-read duplicate requests and to expose the next unresolved tools-chain blocker.

# Evidence E-131: 6b0cb51 live rerun clears duplicate advisory but exposes complete-read content truncation

- Prediction tested: H-060 should make a complete-read duplicate validation-rework read stop immediately instead of giving the model another advisory round.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bg-complete-read-hardstop/runs/terminal_bench__organization-json-generator/20260704-132354-931
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  included_in_utility_aggregate: False
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: 6b0cb516c419a0fb0be609f934bc8665349002fb
  ```
- Matched H-060 live signals:
  - The provider-visible patch-only recovery contained the new line: `If the validation_rework_target_read evidence says complete_read or eof_reached=true, no additional file lines are hidden`.
  - The model still emitted a duplicate `read_file generate_organization.py`.
  - Runtime produced `TaskSpaceValidationReworkDuplicateReadHardStopV1` with `attempt_count: 1`, proving the first complete-read duplicate stopped the turn instead of issuing another model advisory request.
- New blocker signals:
  - ActionMap `result-10.body` for `generate_organization.py` contained a complete summary: `TaskSpaceReadFileSummaryV1: path=generate_organization.py lines_read=95 eof_reached=true max_lines=240`.
  - The same stored body truncated the file content at `total_departments = len(departments_csv)` and then inserted `[... telemetry preview truncated ...]`.
  - Provider-visible projection showed `read_context: complete_read` but the visible `excerpt:` block ended with `...`.
  - The patch-only recovery also included `TaskSpaceToolTailSentinelV1`, but only the summary tail was restored; the patch-relevant lower half of the source file was not visible.
- Interpretation: H-060 is live-cleared. The next problem type is `complete-read-content-preview-truncation`: the read summary is correct about the tool execution, but provider-visible feedback overclaims usable source visibility because ActionMap persists only a telemetry preview plus summary sentinel, not the full content of a bounded complete read.

# Hypothesis H-061: complete read_file results must preserve bounded full content, not only summary tails

- Claim: `tool_output_model_visible_preview()` stores ActionMap tool results through `bounded_model_visible_text_preview()`, which uses the 2KiB telemetry preview limit. For action-contract `read_file`, the shell command already bounds output to `sed -n '1,240p'`, and `TaskSpaceReadFileSummaryV1 eof_reached=true` proves the whole file was captured. Persisting only the telemetry head plus `TaskSpaceToolTailSentinelV1` leaves `read_context: complete_read` in the prompt while hiding patch-relevant lower lines, so the model has a legitimate reason to ask for full content.
- Prediction:
  1. A focused tool preview test with a complete `read_file` output over the 2KiB telemetry threshold but under a bounded full-read cap should preserve late file lines and should not include `[... telemetry preview truncated ...]`.
  2. The existing summary-tail truncation test should still pass for incomplete reads (`eof_reached=false`) or oversized reads.
  3. Validation rework target-read evidence should then be able to show the complete small source file when `eof_reached=true`, making the `complete_read/no hidden lines` feedback true at the provider-visible level.
  4. R4-adjacent preview, validation rework, action contract, formatting, whitespace, and `whale` build checks should pass.
- Diagnostic evidence plan: Add a bounded complete-read preview preservation path before telemetry truncation, keep non-complete/oversized reads on the truncating path with tail sentinels, add focused tests, then rerun R4-adjacent checks and a keyed `organization-json-generator` rerun.
- Status: confirmed.

# Evidence E-132: complete read_file content preview is focused-fixed

- Prediction tested: H-061 predicts complete read_file output should bypass the 2KiB telemetry preview when it is bounded, while incomplete reads should still truncate and preserve only the summary sentinel.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/tools/context.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/context_tests.rs`
- Repair behavior:
  - `bounded_model_visible_text_preview()` now preserves the full model-visible text for complete read_file output when the result has a parseable `TaskSpaceReadFileSummaryV1` with `eof_reached=true`, is at most 64 KiB, and is at most 320 lines.
  - Incomplete reads (`eof_reached=false`) and oversized reads still use telemetry preview truncation plus `TaskSpaceToolTailSentinelV1`.
  - Added `taskspace_preview_preserves_complete_read_file_content_beyond_telemetry_limit`; adjusted `taskspace_preview_preserves_read_file_summary_after_telemetry_truncation` to cover incomplete reads.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_preview_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core read_file_summary_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. `taskspace_preview_` 4/4, `read_file_summary_` 3/3, `validation_rework` 20/20, `action_contract_prompt` 28/28, `validation_` 99/99, `taskspace_control` 35/35, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `apply_patch_` 35/35, and `local_infra` 11/11 all passed. `cargo fmt --check` still prints the known stable rustfmt `imports_granularity` warning; `git diff --check` and `whale` build passed.
- Interpretation: H-061 is focused-fixed. The remaining gate is commit/push, attestation, and a keyed rerun to verify the patch-only recovery now exposes the complete small rework target content instead of a telemetry-truncated head.

# Evidence E-133: bd93d5d live rerun exposes optional fact-source hardening loop

- Prediction tested: H-061 should let the provider see complete bounded read_file content instead of a telemetry-truncated head.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bh-complete-read-preview/runs/terminal_bench__organization-json-generator/20260704-133520-535
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: wrong
  right_exec_timed_out: False
  right_tool_call_count: 18
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: bd93d5d3b80a6263f96ffeadd81f916d6ed15e92
  ```
- New blocker signals:
  - The initial TaskSpace start request declared `initial_fact_sources`: `schema.json`, `departments.csv`, `employees.csv`, `projects.csv`, and `package.json or any config files (if present)`.
  - `rg --files` listed the workspace inputs and did not include `package.json`.
  - Reads of the required `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv` succeeded.
  - The model attempted `read_file package.json`; the shell returned `sed: can't read package.json: No such file or directory`.
  - The provider-visible projection still advertised `read_file declared fact-source artifact package.json next` and `do not finish inspect_code_context until declared fact-source artifacts are read`.
  - Duplicate inspect-read recovery repeatedly reported `missing_fact_source_artifact:package.json`; the turn ended at `TaskSpaceProviderBudgetHardStopV1` with `node_request_count: 13/12`.
- Interpretation: H-061 is not live-verified because an earlier inspect feedback-layer issue blocked the run. The new problem type is `optional-fact-source-required-loop`: runtime converted a conditional source phrase into a hard inspect gate and did not use absence evidence from file discovery to let the task proceed.

# Hypothesis H-062: optional fact-source descriptions must not become hard inspect gates

- Claim: `task_required_fact_source_artifact_refs()` extracts artifact-like tokens from every fact-source `id` and `description` without preserving optionality. A description such as `package.json or any config files (if present)` therefore becomes a required `package.json` artifact, and `inspect_missing_required_fact_source_artifacts()` cannot clear it because failed reads are not successful read/search evidence.
- Prediction:
  1. A focused inspect test with required schema/CSV fact sources plus optional `package.json or any config files (if present)` should report no missing required fact-source artifacts after the required files are read and `rg --files` shows no `package.json`.
  2. The projection should advertise `finish_node` into `implement_solution`, not `read_file declared fact-source artifact package.json next`.
  3. Duplicate inspect read/search recovery should not contain `missing_fact_source_artifact:package.json` for that optional source.
  4. Existing hard-required fact-source tests for missing `employees.csv` and `projects.csv` should still block finish until those artifacts are read.
- Diagnostic evidence plan: Add optional fact-source detection to the required-artifact extraction path, cover the live conditional phrase with focused tests for missing-artifacts and projection behavior, then run inspect missing-source, duplicate-read/search, TaskSpace control, validation, formatting, whitespace, and `whale` build gates.
- Status: confirmed.

# Hypothesis H-063: generated missing-source bootstrap shell commands must classify as read

- Claim: `run_taskspace_missing_fact_source_bootstrap()` generates a shell command like `printf ...; sed -n ... && awk ...` without a `taskspace-action-contract-*` call id. The shell classifier does not recognize bounded `sed -n` read commands, so preflight/gate classification can conservatively route the generated bootstrap through a non-read path before the session later records the result as `ActionClass::Read`.
- Prediction:
  1. `classify_shell_text()` should classify a bounded `sed -n ... && awk ...` read command as `ActionClass::Read`.
  2. The same command with a `printf` section header prefix, matching missing-source bootstrap, should also classify as `ActionClass::Read`.
  3. Mutating `sed -i`, redirection, formatter, and existing edit/test/build/search classifications should remain unchanged.
- Diagnostic evidence plan: Add focused classifier tests for bounded `sed -n` and `printf; sed -n` bootstrap forms, then extend read classification narrowly enough that existing shell action classification tests still pass.
- Status: confirmed.

# Evidence E-134: optional fact-source and bootstrap read classification are focused-fixed

- Prediction tested: H-062 predicts optional fact-source descriptions must not become hard inspect gates; H-063 predicts generated missing-source bootstrap read commands must classify as read before tool execution.
- Repair artifacts:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
- Repair behavior:
  - `task_required_fact_source_artifact_refs()` now skips fact-source records whose id or description contains conditional/optional markers such as `if present`, `if available`, `when present`, or `optional`.
  - Fact-source-derived output schema/validator requirements use the same optional-source guard, so optional absent schemas/config files do not become validation hard gates.
  - Added `inspect_optional_fact_source_absence_does_not_block_finish`, covering the live phrase `package.json or any config files (if present)`, absence evidence from `rg --files`, successful reads of required schema/CSV files, projection `finish_node`, and duplicate `rg --files` recovery without `missing_fact_source_artifact:package.json`.
  - `classify_shell_text()` now treats bounded `sed -n` read commands as `ActionClass::Read`; existing edit classification still runs first, so `sed -i`, redirection, formatter writes, and other mutating commands remain edit-classified.
  - `shell_action_classifier_identifies_core_taskspace_classes` now covers both `sed -n ... && awk ...` and missing-source bootstrap's `printf ...; sed -n ... && awk ...` shape.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_optional_fact_source_absence_does_not_block_finish --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core shell_action_classifier_identifies_core_taskspace_classes --lib --locked
  ```
- R4-adjacent regression/build validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core inspect_missing_fact_sources --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core missing_fact_source_bootstrap --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core duplicate_read_search --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core output_contract --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_control --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_active_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core taskspace_preview_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core read_file_summary_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. Focused H-062 and H-063 tests passed. Regression suites passed through `inspect_missing_fact_sources` 2/2, `missing_fact_source_bootstrap` 1/1, `duplicate_read_search` 3/3, `output_contract` 8/8, `taskspace_control` 35/35, `action_contract_prompt` 28/28, `validation_` 99/99, `provider_budget` 23/23, `taskspace_active_budget` 11/11, `apply_patch_` 35/35, `local_infra` 11/11, `taskspace_preview_` 4/4, and `read_file_summary_` 3/3. `cargo fmt --check` still prints the known stable rustfmt `imports_granularity` warning but exits 0 after formatting. `git diff --check` and the `whale` build passed.
- Interpretation: H-062 and H-063 are focused-fixed with R4-adjacent regression/build coverage. The remaining live gate is commit/push, attestation, and another keyed `organization-json-generator` rerun to verify the run advances past optional config inspection and any missing-source bootstrap remains read-classified.

# Evidence E-135: a09a966 live rerun clears optional fact-source loop but exposes failed-edit recovery priority inversion

- Prediction tested: H-062/H-063 should let `organization-json-generator` advance beyond optional `package.json` inspect loops and keep missing-source bootstrap read-classified.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bi-optional-fact-source/runs/terminal_bench__organization-json-generator/20260704-135944-845
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 12
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: a09a966cf74729d4625b556b044f9327c8e57e21
  ```
- Matched H-062/H-063 live signals:
  - The task started with required `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv` fact sources only.
  - No `package.json` missing fact-source loop occurred.
  - TaskSpace read all fact sources, forced inspect transition into implementation, ran schema validation, and entered validation rework.
  - The turn completed in about 283s, not a 900s timeout.
- New blocker signals:
  - Validation result `result-9` summarized `missing_required_properties: members, averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService`.
  - Rework node `node-4` read complete target contents from `process.py` as `result-11`.
  - The model emitted an `apply_patch`, but the patch failed with `apply_patch verification failed: Failed to find expected lines ... process.py`.
  - Instead of surfacing `TaskSpaceEditFailureRecoveryV1` / `apply_patch_expected_lines_mismatch`, runtime inserted `TaskSpaceValidationReworkPatchOnlyHardStopV1` and ended the turn. The graph was left with open leaf `node-4`.
- Interpretation: optional fact-source and bootstrap read classification are live-cleared. The new problem type is `validation-rework-failed-edit-recovery-shadowed-by-patch-only-hardstop`: an actual failed edit attempt should reset the model into failed-edit recovery, not count as a repeated non-edit/read violation of the patch-only contract.

# Hypothesis H-064: failed apply_patch feedback must take priority over validation rework patch-only recovery

- Claim: `build_taskspace_implementation_recovery_item()` checks `taskspace_evidence_has_validation_rework_target_read(evidence_summary)` before `failed_edit_summary.is_some()`. Once a validation rework node has target-read evidence, any later needs-edit recovery chooses `TaskSpaceValidationReworkPatchOnlyRecoveryV1` even after a concrete failed `apply_patch`. Because patch-only recovery count was already 1 from the prior read attempt, the next patch-only recovery becomes `TaskSpaceValidationReworkPatchOnlyHardStopV1`, shadowing the more specific apply_patch expected-lines recovery.
- Prediction:
  1. A focused session recovery test with target-read evidence plus failed `apply_patch expected lines` summary should produce `TaskSpaceEditFailureRecoveryV1`, not `TaskSpaceValidationReworkPatchOnlyRecoveryV1`.
  2. The edit failure recovery should preserve `apply_patch_expected_lines_mismatch` guidance, including not repeating the same hunk and optionally one narrow same-target context refresh.
  3. Existing patch-only recovery still applies when target-read evidence exists and there is no failed edit summary.
  4. Apply-patch, validation rework, action-contract prompt, provider budget, formatting, whitespace, and `whale` build checks should pass.
- Diagnostic evidence plan: Reorder implementation recovery selection so duplicate-read recovery remains first, failed edit recovery comes before target-read patch-only recovery, add focused tests for the live priority inversion, then rerun R4-adjacent apply_patch/validation/action-contract/build gates.
- Status: confirmed.

# Evidence E-136: failed edit recovery now outranks validation rework patch-only recovery

- Prediction tested: H-064 predicts a validation rework node with target-read evidence plus concrete failed `apply_patch` feedback should receive `TaskSpaceEditFailureRecoveryV1`, not `TaskSpaceValidationReworkPatchOnlyRecoveryV1` or its hard-stop variant.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - `build_taskspace_implementation_recovery_item()` still lets duplicate-read recovery run first, because repeated reads/searches after target visibility remain a separate no-progress case.
  - Concrete failed edit summaries now run before `taskspace_evidence_has_validation_rework_target_read(evidence_summary)`, so `apply_patch` expected-lines/context mismatch feedback is not hidden by the more general post-target-read patch-only contract.
  - Added `implementation_recovery_prioritizes_failed_edit_over_patch_only_after_target_read`, covering the live shape: validation rework target-read evidence, schema repair requirements, and failed `apply_patch verification failed: Failed to find expected lines ... process.py`.
  - The focused test asserts the provider-visible recovery contains `TaskSpaceEditFailureRecoveryV1`, `Failed to find expected lines`, `do not repeat the same hunk`, and one same-target narrow read option, while excluding `TaskSpaceValidationReworkPatchOnlyRecoveryV1`.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_recovery_prioritizes_failed_edit_over_patch_only_after_target_read --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_recovery_selects_patch_only_after_target_read_evidence --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_patch_only_hard_stops_after_one_recovery --lib --locked
  ```
- R4-adjacent regression validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core apply_patch_ --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core implementation_needs_edit --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. Focused H-064 test passed; original post-target-read patch-only recovery and hard-stop tests still passed. Regression suites passed through `apply_patch_` 35/35, `validation_rework` 20/20, `action_contract_prompt` 28/28, `implementation_needs_edit` 3/3, and `provider_budget` 23/23. `cargo fmt --check` exits 0 with only the known stable rustfmt `imports_granularity` warnings. `git diff --check` and the `whale` build passed.
- Interpretation: H-064 is focused-fixed with R4-adjacent regression/build coverage. The remaining gates are commit/push, attestation, and another keyed `organization-json-generator` rerun to verify the live trace advances past failed-edit recovery priority inversion.

# Evidence E-137: c3ccd49 live rerun clears H-064 path but exposes generic local-infra retry command

- Prediction tested: H-064 should no longer shadow concrete failed-edit feedback with validation rework patch-only hard-stop.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bj-failed-edit-recovery/runs/terminal_bench__organization-json-generator/20260704-141543-156
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: engineering_unclean
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 8
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: c3ccd498b56eff26f8dd77161b24b01d3ca8a609
  ```
- Matched H-064 live signals:
  - No `apply_patch verification failed` occurred in the TaskSpace side.
  - No `TaskSpaceValidationReworkPatchOnlyHardStopV1` occurred.
  - The run advanced through inspect and implementation into smoke-test validation.
- New blocker signals:
  - The model created `process.py`, then ran `pytest -v`.
  - The shell returned `/bin/bash: line 1: pytest: command not found`.
  - Runtime correctly treated this as local validator infrastructure and blocked `node-3`, then created `node-4` as a `smoke_test` retry instead of an implementation rework.
  - The `node-4` context knew the changed artifact was `process.py`, but the recovery text still said to run a platform-compatible command "such as `python recover.py`".
  - The provider then hit five stream disconnects during validation recovery and ended at `TaskSpaceProviderBudgetHardStopV1`; the pair is also engineering-unclean because the external Docker backend was unavailable.
- Interpretation: H-064 is not contradicted by the rerun. The newly actionable feedback-layer problem type is `validation-local-infra-retry-command-generic`: local-infra retry context names the changed artifact but loses it when presenting the command, weakening the next action from exact `python process.py` to a generic sample.

# Hypothesis H-065: local-infra validation retry must name exact changed-artifact commands

- Claim: `block_main_node()` creates local-infra validation retry nodes from `validation_node_local_infra_unvalidated_artifact_result()` / `validation_node_local_infra_blocker_unvalidated_artifact_result()`. Those helpers already compute `validation_dependency_changed_artifacts()`, but the retry node context uses the hard-coded example `python recover.py` instead of a command derived from the changed artifact list.
- Prediction:
  1. A failed validation result with changed artifact `merge_users.py` should create a retry node whose context includes `python merge_users.py` and does not include generic `python recover.py`.
  2. A manual local-infra blocker with changed artifact `recover.py` should still include `python recover.py`.
  3. Existing local-infra, validation rework, action-contract prompt, and provider-budget tests should continue to pass.
- Diagnostic evidence plan: Add command-hint generation for changed script artifacts (`.py`, `.js`, `.mjs`, `.sh`) and use it in both local-infra failed-result and blocker summaries; replace the retry node text with "named platform-compatible command(s)" so the exact command from the summary is authoritative.
- Status: confirmed.

# Evidence E-138: local-infra retry context now carries exact changed-artifact command hints

- Prediction tested: H-065 predicts local-infra validation retry nodes should preserve the changed artifact as a concrete runnable command.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Repair behavior:
  - Added `platform_compatible_validation_commands()` and `platform_compatible_validation_command_hint()` to derive command hints from changed script artifacts.
  - `.py` artifacts become `python <artifact>`, `.js` / `.mjs` become `node <artifact>`, and `.sh` becomes `sh <artifact>`.
  - Local-infra failed-result and blocker summaries now append `Suggested platform-compatible command(s): ...` when changed script artifacts are known.
  - Validation retry node context now tells the model to run the named command(s) above, instead of hard-coding `python recover.py`.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib --locked
  ```
- R4-adjacent regression validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core local_infra --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. Focused test passed for both failed-result and manual-blocker paths; `local_infra` 11/11, `validation_rework` 20/20, `action_contract_prompt` 28/28, and `provider_budget` 23/23 passed. `cargo fmt` and `cargo fmt --check` exit 0 with only the known stable rustfmt `imports_granularity` warnings. `git diff --check` and the `whale` build passed.
- Interpretation: H-065 is focused-fixed with R4-adjacent regression/build coverage. Remaining gates are commit/push, attestation, and another keyed rerun.

# Evidence E-139: 4fcf4ab live rerun clears local-infra command genericity but exposes schema rename hint gap

- Prediction tested: H-065 should make validation retry context specific enough to steer away from bare pytest/local-infra loops.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bk-local-infra-command-hint/runs/terminal_bench__organization-json-generator/20260704-143221-395
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 11
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: 4fcf4ab21dd8070f6d2faf4f6c5b99acd6e6416d
  ```
- Matched H-065 live signals:
  - Docker validation environment was available; `tests_started_seen=True` on both sides.
  - TaskSpace did not repeat the prior `python recover.py` generic local-infra retry.
  - The model reached the correct combined validation command: `python csv_to_json.py && python -m jsonschema -i organization.json schema.json`.
  - The validator produced real schema failures rather than local-infra failures.
- New blocker signals:
  - Validation failed with required property gaps: `members`, `averageDepartmentBudget`, `totalEmployees`, `skillDistribution`, `departmentSizes`, `projectStatusDistribution`, and `averageYearsOfService`.
  - The failed output itself contained obvious source/target rename clues: project objects used `member_ids` while schema required `members`; statistics used snake_case keys such as `total_employees` while schema required camelCase.
  - Runtime generated a schema repair contract with missing properties and schema required groups, but not the source-to-required rename hints.
  - The model repeatedly tried to read `schema.json` instead of patching `csv_to_json.py`, then hit `TaskSpaceValidationReworkPatchOnlyHardStopV1`.
- Interpretation: H-065 is live-cleared. The newly actionable feedback-layer problem type is `validation-schema-repair-rename-hint-gap`: validation output contains enough structured evidence to suggest key renames, but the repair contract only lists missing required names.

# Hypothesis H-066: schema repair contracts should include property rename hints from validator output

- Claim: `implement_node_dependency_validation_rework_repair_contract()` already collects missing required properties and schema required groups, but it does not inspect the offending JSON object keys in the validator output. When the output has `member_ids` and the missing property is `members`, or `total_employees` and the missing property is `totalEmployees`, the provider receives weaker repair guidance than the evidence supports.
- Prediction:
  1. A focused schema repair contract test with `{'name': 'Madrid', 'member_ids': [...]}` and missing `members` should include `member_ids->members`.
  2. A focused schema repair contract test with `{'total_employees': 12}` and missing `totalEmployees` should include `total_employees->totalEmployees`.
  3. Existing validation rework, output contract, action-contract prompt, and provider-budget suites should pass.
- Diagnostic evidence plan: Extract quoted object keys from validation failure lines that end in `'<property>' is a required property`; compare normalized key names with missing required properties; append unique `schema_property_rename_hints` to the repair contract.
- Status: confirmed.

# Evidence E-140: schema repair contract now includes validator-derived rename hints

- Prediction tested: H-066 predicts rename hints should be added without weakening existing validation rework gates.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Repair behavior:
  - Added `validation_failure_property_rename_hints()` and helpers to extract quoted object keys from validator output.
  - The repair contract now appends `schema_property_rename_hints=...` when an offending output key normalizes to a missing required property, or clearly contains its singular form.
  - Focused fixture now covers `member_ids->members`, `total_employees->totalEmployees`, and `average_department_budget->averageDepartmentBudget` style evidence.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework_projects_schema_repair_contract_from_schema_read --lib --locked
  ```
- R4-adjacent regression validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core validation_rework --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core output_contract --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core action_contract_prompt --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-core provider_budget --lib --locked
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -j1 -p codex-cli --bin whale --locked
  ```
- Result: passed. Focused test passed after updating the fixture to include the live-style statistics required-property lines. Regression suites passed through `validation_rework` 20/20, `output_contract` 8/8, `action_contract_prompt` 28/28, and `provider_budget` 23/23. `cargo fmt` and `cargo fmt --check` exit 0 with only the known stable rustfmt `imports_granularity` warnings. `git diff --check` and the `whale` build passed.
- Interpretation: H-066 is focused-fixed with R4-adjacent regression/build coverage. Remaining gates are commit/push, attestation, and another keyed rerun.

# Evidence E-141: d11321c live rerun clears rename-hint contract but exposes generic fact-source coverage gap

- Prediction tested: H-066 should make schema repair contracts richer when validation reaches real required-property failures.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bl-schema-rename-hints/runs/terminal_bench__organization-json-generator/20260704-144300-806
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 10
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: d11321c9dcb66441b4611b0a931051c94df87de5
  ```
- Matched H-066-adjacent live signals:
  - Docker validation environment was available; both sides reached `tests_completed`.
  - TaskSpace reached the schema-validating command path and failed on real output-contract properties rather than local infra.
  - The failure remained bounded: no 900s timeout; the turn ended through TaskSpace hard-stop semantics.
- New blocker signals:
  - `start_task` recorded success criteria that explicitly named `departments.csv`, `employees.csv`, and `projects.csv`, but `initial_fact_sources` only said `repository root containing schema.json and CSV files`.
  - The inspect node listed repository files and read `schema.json`, then implementation started without reading any CSV contents.
  - The generated `process.py` guessed nonexistent employee columns such as `email`, `role`, and `salary`, producing schema failures for missing `position`, `skills`, `years_of_service`, `members`, `deadline`, and statistics keys.
  - The model attempted `read_file departments.csv` only after implementation had already succeeded, and runtime correctly rejected that as too late for the implementation node.
- Interpretation: H-066 is not contradicted, but the latest blocker is upstream of repair-contract quality. The new problem type is `generic-fact-source-success-criteria-artifact-gap`: declared fact-source text can be too generic, while success criteria contain concrete input artifacts that must still be enforced during inspect.

# Hypothesis H-067: inspect fact-source coverage must derive concrete input artifacts from success criteria

- Claim: `task_required_fact_source_artifact_refs()` only extracted concrete artifacts from recorded fact sources. When `initial_fact_sources` used a generic directory phrase but success criteria named concrete input files, inspect coverage did not require those named inputs before implementation.
- Prediction:
  1. A focused test matching the live shape should treat `departments.csv`, `employees.csv`, and `projects.csv` in success criteria as required fact-source artifacts even when the fact source says only `repository root containing schema.json and CSV files`.
  2. The same test should not treat generated `organization.json` as an input fact source.
  3. Projection and manual finish should refuse implement transition until the named CSVs are read.
  4. Existing inspect/fact-source regressions should pass.
- Diagnostic evidence plan: Extend required fact-source artifact extraction to scan success criteria and cognitive success criteria, exclude output-contract targets and generated JSON outputs, then add a focused runtime test and rerun inspect/fact-source regression coverage.
- Status: confirmed.

# Evidence E-142: success-criteria input artifacts now participate in inspect fact-source gates

- Prediction tested: H-067 predicts generic directory fact sources no longer allow implementation before user-named input artifacts are read.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- Repair behavior:
  - `task_required_fact_source_artifact_refs()` now adds concrete artifact refs from problem-ledger and cognitive success criteria.
  - The added path excludes declared output-contract targets and generated non-schema JSON outputs, so `organization.json` is not forced as a pre-implementation input read.
  - Existing explicit fact-source records keep their previous behavior, including JSON input support.
  - Added `inspect_requires_success_criteria_artifacts_when_fact_source_is_generic_directory`, covering the live shape and asserting projection/manual finish remain blocked until all three CSVs are read.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core inspect_ -- --nocapture
  ```
- R4-adjacent regression validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core output_contract -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core provider_budget -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. Focused inspect suite passed `62/62`, including the new H-067 test plus adjacent inspect, duplicate-read, missing fact-source, projection, and provider-budget inspect tests. Regression suites passed through `output_contract` 8/8, `validation_rework` 20/20, `action_contract_prompt` 28/28, and `provider_budget` 23/23. `cargo fmt --check` exits 0 with only the known stable rustfmt `imports_granularity` warnings. `git diff --check` and the `whale` build passed.
- Interpretation: H-067 is focused-fixed with R4-adjacent regression/build coverage. Remaining gates are commit/push, attestation, and another keyed rerun to verify the live model now reads CSV source files before implementation.

# Evidence E-143: 77e8e46 live rerun clears success-criteria fact-source gate but stops after first complete-read duplicate recovery

- Prediction tested: H-067 should force concrete CSV fact-source reads before implementation.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bm-criteria-fact-sources/runs/terminal_bench__organization-json-generator/20260704-145742-157
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 16
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: 77e8e46a3a1435ddef0c8b1a8c674488e308d62a
  ```
- Matched H-067 live signals:
  - The provider-visible projection listed `/app/schema.json`, `/app/departments.csv`, `/app/employees.csv`, and `/app/projects.csv` as declared fact-source artifacts.
  - After `/app/...` paths failed, TaskSpace recovered through `rg --files` and the model read `schema.json`, `departments.csv`, `employees.csv`, and `projects.csv` before implementation.
  - The implementation used real CSV fields such as `position`, `skills`, `years_of_service`, `member_ids`, and `deadline` instead of hallucinated `email`, `role`, or `salary`.
- New blocker signals:
  - The schema repair contract reached the provider and included `missing_required_properties=members, averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService` and `schema_property_rename_hints=member_ids->members`.
  - Validation rework read `generate_organization.py` completely as `result-14` with `TaskSpaceReadFileSummaryV1 eof_reached=true`.
  - The model repeated `read_file generate_organization.py` instead of applying a patch.
  - Runtime generated the stronger duplicate-read feedback saying `result-14` is a complete read_file context and no additional file lines are hidden, but immediately converted that first duplicate-read recovery into `TaskSpaceValidationReworkDuplicateReadHardStopV1`; the model never received a chance to act on the stronger duplicate-read feedback.
- Interpretation: H-067 is live-cleared. The new feedback/control problem type is `validation-rework-complete-read-duplicate-hardstop-too-early`: first duplicate-read recovery after complete target visibility should be one recoverable provider turn, with hard-stop reserved for a second duplicate or explicit repeated-blocked-action evidence.

# Hypothesis H-068: complete-read duplicate rework feedback needs one recoverable turn before hard-stop

- Claim: `taskspace_validation_rework_duplicate_read_should_hard_stop()` treated any duplicate-read recovery containing `complete read_file context` / `read_context: complete_read` as an immediate hard-stop even when no previous duplicate-read recovery had been delivered. This prevents the model from responding to the first feedback that explicitly says the target read was complete and no file lines are hidden.
- Prediction:
  1. A focused complete-read duplicate-read test should not hard-stop at previous recovery count `0`.
  2. The same complete-read duplicate-read recovery should hard-stop when previous recovery count is `1`.
  3. Repeated-blocked-action evidence should still hard-stop immediately.
  4. Existing validation rework duplicate-read and validation rework suites should pass.
- Diagnostic evidence plan: Remove `complete_read` from the immediate hard-stop predicate, keep hard-stop for previous recovery count and repeated-blocked-action markers, then update focused tests.
- Status: confirmed.

# Evidence E-144: complete-read duplicate rework feedback now gets one provider recovery turn

- Prediction tested: H-068 predicts complete-read duplicate-read feedback is recoverable once, while true repeats still hard-stop.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - `taskspace_validation_rework_duplicate_read_should_hard_stop()` no longer treats `complete_read` as sufficient for immediate hard-stop.
  - `validation_rework_duplicate_read_complete_context_gets_one_recovery_before_hard_stop` asserts previous count `0` does not hard-stop and previous count `1` does.
  - `validation_rework_duplicate_read_repeated_gate_hard_stops_immediately` preserves immediate hard-stop when the gate already reports repeated blocked action.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework_duplicate_read -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework -- --nocapture
  ```
- Result: passed. `validation_rework_duplicate_read` passed 7/7; `validation_rework` passed 20/20.
- Interpretation: H-068 is focused-fixed. Remaining gates are formatting/build, commit/push, attestation, and another keyed rerun to verify whether the provider patches after receiving the complete-read duplicate feedback.

# Evidence E-145: H-068 formatting and build gates passed before commit

- Prediction tested: H-068 focused repair should not break repository formatting, whitespace checks, or the `whale` dev build.
- Validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `cargo fmt --check` exits 0 with the known stable rustfmt `imports_granularity` warnings; `git diff --check` exits 0; `whale` dev build finished successfully.
- Interpretation: H-068 is ready for commit/push and attestation before live keyed rerun.

# Evidence E-146: 9f370dd live rerun clears complete-read duplicate timing and exposes block rejection wording drift

- Prediction tested: H-068 should prevent the first complete-read duplicate feedback from being immediately hard-stopped.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bn-complete-read-recovery/runs/terminal_bench__organization-json-generator/20260704-150817-545
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 14
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: 9f370ddfa3f12397ed1d966b321b5e1f0a86c3b2
  whale_binary_sha256: 06a7a8e3bb0b00540066f5ca62687650f4ef7e66eb4d7894ad8545a4c426cbcb
  ```
- Matched H-068 live signal:
  - The run did not stop with `TaskSpaceValidationReworkDuplicateReadHardStopV1` after a first complete target read.
  - The model received a provider turn after validation rework and read the target `process.py` completely as `result-15` (`TaskSpaceReadFileSummaryV1 eof_reached=true`).
- New blocker signals:
  - The provider then emitted action-contract JSON `{"action":"blocked","reason":"Need to read schema.json to understand required output structure and check test expectations before fixing process.py..."}`.
  - Runtime correctly rejected that block in `whale-exec.stderr.log`: `cannot be blocked for missing source visibility because dependency evidence already identifies the implementation artifact or validation rework target`.
  - Session did not structure this newer runtime wording as `missing_source_visibility_blocker_rejected`; it fell back into `TaskSpaceValidationReworkPatchOnlyHardStopV1`.
- Interpretation: H-068 is live-cleared. New issue type is `validation-rework-block-rejection-wording-drift`: the feedback layer recognized the old missing-source block rejection wording but missed the newer validation-rework wording, so a correct runtime rejection was not semantically carried into the next provider prompt.

# Hypothesis H-069: missing-source block rejection wording drift hides repair feedback after validation rework

- Claim: session prompt preparation only recognized missing-source block rejection when the tool output contained `already recorded implementation source evidence`. Runtime now emits the validation-rework-specific wording `dependency evidence already identifies the implementation artifact or validation rework target`. Because that wording was not classified as actionable gate feedback, the provider did not receive the structured `missing_source_visibility_blocker_rejected` feedback and the turn escalated to patch-only hard-stop.
- Prediction:
  1. A focused action-contract prompt test using the live runtime wording should produce `failure_kind: missing_source_visibility_blocker_rejected`.
  2. The prepared prompt should include the progress hint that the previous block_node action was rejected and the next action must be `apply_patch`.
  3. Adjacent `action_contract_prompt` and `validation_rework` tests should pass.
- Diagnostic evidence plan: Add a shared recognizer for both old and new missing-source block rejection wording, use it in actionable output detection, recent-output progress hints, and tool-output summary structuring, then run focused suites.
- Status: confirmed.

# Evidence E-147: validation-rework missing-source block rejection wording now structures as apply_patch feedback

- Prediction tested: H-069 predicts the live runtime wording `dependency evidence already identifies the implementation artifact or validation rework target` is actionable missing-source block rejection feedback.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - Added `taskspace_text_mentions_missing_source_visibility_blocker_rejection()` for both old and new missing-source block rejection wording.
  - Reused it in actionable gate feedback detection, recent-output progress hint selection, and action-contract tool-output summary structuring.
  - Added `action_contract_prompt_structures_validation_rework_missing_source_blocker_rejection` using the exact validation-rework wording from the live rerun.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt_structures_validation_rework_missing_source_blocker_rejection -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework -- --nocapture
  ```
- Result: passed. Focused test passed 1/1; `action_contract_prompt` passed 29/29; `validation_rework` passed 21/21.
- Interpretation: H-069 is focused-fixed. Remaining gates are formatting/build, commit/push, attestation, and another keyed rerun to verify the block rejection reaches the provider as structured feedback before patch-only hard-stop.

# Evidence E-148: H-069 formatting and build gates passed before commit

- Prediction tested: H-069 feedback-classification repair should not break repository formatting, whitespace checks, or the `whale` dev build.
- Validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `cargo fmt` and `cargo fmt --check` exit 0 with the known stable rustfmt `imports_granularity` warnings; `git diff --check` exits 0; `whale` dev build finished successfully.
- Interpretation: H-069 is ready for commit/push and attestation before live keyed rerun.

# Evidence E-149: 431e0ee rerun diverges past block rejection and exposes buried patch directive in long validation rework evidence

- Prediction tested: H-069 should structure the validation-rework missing-source block rejection if that path recurs.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bo-block-rejection-feedback/runs/terminal_bench__organization-json-generator/20260704-151923-804
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 18
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: 431e0eebd1e94425287173393a381851c50485ce
  whale_binary_sha256: 5f56624a255f52abbe450345f01e0f2518fe61a717426fd7d15093ececeed912
  ```
- Result: H-069 path did not recur in this rerun, so H-069 remains focused-fixed but not live-cleared.
- New blocker signals:
  - Runtime reached schema validation and generated a repair contract with `missing_required_properties=members, averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService`, `schema_required_groups`, and `schema_property_rename_hints=member_ids->members`.
  - The model read `process.py` completely as `result-14` (`TaskSpaceReadFileSummaryV1 eof_reached=true`).
  - `TaskSpaceValidationReworkPatchOnlyRecoveryV1` contained the full repair contract, complete target read, schema/CSV evidence, and prohibition on `read_file`, but its `Current required behavior` section appeared after a long evidence block.
  - The provider repeated `read_file process.py`, received `TaskSpaceValidationReworkDuplicateReadRecoveryV1`, then repeated `read_file process.py` again and hit `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
- Interpretation: New issue type is `validation-rework-patch-directive-buried-after-evidence`: the feedback contained the right facts, but the operative patch directive was placed after a long evidence section, weakening actionability for DeepSeek V4 Flash in repair loops.

# Hypothesis H-070: validation rework recovery should put patch directive before long evidence

- Claim: `TaskSpaceValidationReworkPatchOnlyRecoveryV1` and `TaskSpaceValidationReworkDuplicateReadRecoveryV1` placed `Current required behavior` after long repair/evidence excerpts. In live rerun `20260704-151923-804`, the model had all required evidence but still repeated `read_file`, indicating the immediate action directive was too late in the feedback payload.
- Prediction:
  1. Focused tests should assert both patch-only and duplicate-read validation rework recoveries put `Current required behavior` before `Already inspected evidence available to use now`.
  2. Existing `validation_rework_duplicate_read`, `validation_rework`, and `action_contract_prompt` suites should still pass.
  3. The next keyed rerun should either patch after the first complete-read recovery or expose a lower-level patch synthesis failure.
- Diagnostic evidence plan: Reorder validation rework recovery payloads so the action directive appears before previous feedback and long evidence, while preserving repair contract, complete-read evidence, schema/CSV excerpts, and hard-stop bounds.
- Status: confirmed.

# Evidence E-150: validation rework patch directive now precedes long evidence without dropping complete-read audit semantics

- Prediction tested: H-070 predicts validation rework recovery payloads should put the operative patch directive before long evidence, while preserving existing duplicate-read and action-contract behavior.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - `TaskSpaceValidationReworkPatchOnlyRecoveryV1` now emits `Current required behavior` before previous feedback and long evidence.
  - `TaskSpaceValidationReworkDuplicateReadRecoveryV1` now emits `Current required behavior` before previous feedback, gate context, and long evidence.
  - `TaskSpaceValidationReworkDuplicateReadHardStopV1` explicitly preserves `read_context: complete_read` when the recovery text contains complete-read signals, so moving the directive forward does not hide the audit marker behind excerpt truncation.
  - Focused tests assert the patch directive precedes `Already inspected evidence available to use now`.
- Validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework_duplicate_read -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework -- --nocapture
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt -- --nocapture
  ```
- Result: passed. `validation_rework_duplicate_read` passed 7/7; `validation_rework` passed 21/21; `action_contract_prompt` passed 29/29.
- Interpretation: H-070 is focused-fixed. Remaining gates are formatting/build, commit/push, attestation, and another keyed rerun.

# Evidence E-151: H-070 formatting and build gates passed before commit

- Prediction tested: H-070 recovery-layout repair should not break repository formatting, whitespace checks, or the `whale` dev build.
- Validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. `cargo fmt` and `cargo fmt --check` exit 0 with the known stable rustfmt `imports_granularity` warnings; `git diff --check` exits 0; `whale` dev build finished successfully.
- Interpretation: H-070 is ready for commit/push and attestation before live keyed rerun.

# Evidence E-152: 41b1cf6 rerun shows front-loaded patch directive is live but insufficient

- Prediction tested: H-070 should make the validation rework patch directive provider-visible before long evidence.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bp-frontloaded-rework-guidance/runs/terminal_bench__organization-json-generator/20260704-153051-437
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E2-candidate
  outcome_standard: solved
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 17
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: 41b1cf63f051f3d0e674c4fb712735000ffa576c
  whale_binary_sha256: d2c17206bbc8f6958603c7ba20d040d9192ca2a38a8880fc309c5eb69b9ef561
  ```
- Matched H-070 live signals:
  - The `TaskSpaceValidationReworkDuplicateReadHardStopV1` excerpt shows `Current required behavior` before the recovery evidence.
  - The hard-stop explicitly preserves `read_context: complete_read; complete read_file context already visible; no additional file lines are hidden`.
- New blocker signals:
  - The active projection before final duplicate read listed `next_valid_actions`: use existing complete read result `result-13`, do not read/search again before edit, apply_patch target `generate_organization.py`, and read/search no longer valid.
  - The current node contract said `allowed action classes: edit, control(block_node only; finish_node blocked until successful edit; read/search of already visible validation rework targets will be blocked)`.
  - Despite both the front-loaded recovery and the closed action contract, the provider emitted another `read_file generate_organization.py` action and hit `TaskSpaceValidationReworkDuplicateReadHardStopV1`.
- Interpretation: H-070 is live-applied but not sufficient. New unresolved issue type is `validation-rework-closed-action-space-noncompliance`: feedback semantics are no longer missing or buried; the remaining problem is that the model can still emit an invalid action after the action space is closed, and runtime can only reject/hard-stop rather than force a patch-producing action.

# Hypothesis H-071: validation rework needs a stronger action-space transition after closed-action noncompliance

- Claim: Once validation rework has a complete target read, repair contract, and projection contract that says read/search is blocked, another `read_file` is not a feedback wording problem. It indicates the tool-free action loop lacks a stronger transition for closed action space noncompliance, such as model escalation, alternate repair mode, or a deterministic patch-plan gate before any more provider sampling.
- Prediction:
  1. More prompt wording alone is unlikely to reliably fix the case, because the live payload already included the directive, complete-read evidence, next_valid_actions, and allowed-action contract.
  2. A durable fix should change the capability/control layer, not only append another advisory sentence.
  3. Candidate fixes must be designed separately because they affect model routing, action-contract enforcement, and validation rework recovery policy.
- Diagnostic evidence plan: Compare possible capability-layer designs: (a) escalate validation rework closed-action noncompliance to stronger model/profile, (b) require a structured patch-plan artifact before allowing further read-like actions, (c) turn the second duplicate read into an explicit repair-synthesis node rather than another provider retry, or (d) tighten the action schema so invalid read actions in closed repair state are impossible to emit. Need design review before implementation.
- Status: focused-fixed; real keyed rerun pending.

# Evidence E-153: H-071 action-contract narrowing focused fix validates closed rework read rejection

- Prediction tested: H-071 says a durable fix should change the capability/control layer rather than adding more advisory wording.
- Repair implemented:
  - `ActionMapRuntime::current_main_node_has_visible_validation_rework_target_read()` exposes whether the active implement rework node already has a visible validation rework target read and no successful edit.
  - `Session::action_map_current_main_node_has_visible_validation_rework_target_read()` exposes that state to the action-contract turn loop.
  - `taskspace_closed_validation_rework_read_reject_reason()` rejects a taskspace-action-v1 `read_file` for the already visible validation rework target before it can become a shell read tool call.
  - The rejection reason is `validation_rework_closed_action_space_read_disallowed:read_file`, and the normal recovery builder maps it to patch-only recovery when validation rework target evidence is present.
- Verification:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core taskspace_action_contract_rejects_closed_validation_rework_target_read --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_selects_patch_only_after_closed_action_space_read_reject --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_duplicate_read --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. The new focused tests prove first validation target read remains allowed, closed target re-read is rejected at action-contract schema/control level, and closed-action rejection routes to patch-only recovery instead of shell read. Regression suites `validation_rework_duplicate_read` 7/7, `validation_rework` 23/23, and `action_contract_prompt` 29/29 passed.
- Interpretation: H-071 has a focused control-layer fix. It still needs a real keyed `organization-json-generator` rerun to determine whether the model now emits `apply_patch`, or whether the next unresolved tools-chain issue is deeper than action-contract narrowing.

# Evidence E-154: H-071 live rerun proves closed read rejection works but is downgraded to NoAction recovery

- Prediction tested: H-071 action-contract narrowing should reject a closed validation rework target re-read before it becomes an ordinary shell read.
- Real rerun:
  ```text
  RunDir: target/r4-org-json-real-keyed-20260704bq-closed-action-narrowing/runs/terminal_bench__organization-json-generator/20260704-154904-391
  PairReport: pair-001/pair-report.md
  reported_evidence_level: E1
  outcome_standard: wrong
  outcome_taskspace: engineering_unclean
  right_exec_timed_out: False
  right_tool_call_count: 13
  right_public_validation_exit_code: 1
  right_hidden_oracle_exit_code: 0
  current_git_head: 26d991c4a9957c2cc9c6438cb3936758cb383a48
  whale_binary_sha256: 53ff46c380e498245dfd3627ef26f341a979600e33dde70b90e2d7b4ad53d748
  ```
- Matched H-071 live signal:
  - The provider emitted `read_file generate_organization.py` after the target artifact was already visible and the projection required `apply_patch`.
  - Action-contract conversion rejected it as `TaskSpaceActionV1 rejected: validation_rework_closed_action_space_read_disallowed:read_file`.
  - The read did not become an ordinary shell command.
- New blocker signals:
  - The rejection was not classified as implementation-needs-edit feedback in the session recovery path.
  - The next provider-visible recovery became generic `TaskSpaceNoActionRecoveryV1`, not `TaskSpaceValidationReworkPatchOnlyRecoveryV1`.
  - The provider retried the same illegal `read_file generate_organization.py` several more times and eventually hit `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded`.
- Interpretation: H-071's schema narrowing is live-cleared, but a new feedback-layer problem remains: `validation_rework_closed_action_space_read_disallowed` is a repair-actionability rejection and must route to patch-only recovery. Downgrading it to NoAction loses the "must patch now" semantics and makes the agent think retrying the read is still a valid recovery path.

# Hypothesis H-072: closed action-space rejections are semantically downgraded when routed through generic NoAction recovery

- Claim: `taskspace_message_hit_implementation_needs_edit()` did not recognize `validation_rework_closed_action_space_read_disallowed`. As a result, a closed-action rejection produced by the schema/control layer fell through to `TaskSpaceNoActionRecoveryV1`, which is too generic for validation rework repair and does not carry the patch-only next-action contract.
- Prediction:
  1. The live rejection text should be classified as implementation-needs-edit feedback.
  2. The recovery builder should choose `TaskSpaceValidationReworkPatchOnlyRecoveryV1`, not `TaskSpaceNoActionRecoveryV1`.
  3. Recent tool output progress hints should treat the closed-action rejection like validation rework duplicate-read feedback and require `apply_patch`.
  4. The first closed-action rejection should get one patch-only recovery chance; a second closed-action rejection should hard-stop to avoid budget drain.
- Diagnostic evidence plan: Add the closed-action rejection marker to implementation-needs-edit classification and recent-output progress hints, then update focused tests to assert patch-only routing and bounded hard-stop behavior.
- Status: confirmed.

# Evidence E-155: closed action-space rejection now routes to patch-only recovery with bounded hard-stop

- Prediction tested: H-072 predicts `validation_rework_closed_action_space_read_disallowed` should preserve repair semantics instead of falling to NoAction recovery.
- Repair artifact:
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Repair behavior:
  - `taskspace_message_hit_implementation_needs_edit()` now recognizes `validation_rework_closed_action_space_read_disallowed`.
  - Recent action-contract output hints classify the same marker as validation rework closed-action feedback and say the next action must be `apply_patch`.
  - `taskspace_validation_rework_patch_only_should_hard_stop()` gives the first schema-level closed-action rejection one patch-only recovery turn, then hard-stops on the second closed-action rejection.
  - `implementation_recovery_selects_patch_only_after_closed_action_space_read_reject` asserts patch-only recovery is selected, NoAction recovery is not selected, and the hard-stop threshold is bounded.
- Focused validation:
  ```text
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_selects_patch_only_after_closed_action_space_read_reject --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib
  CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  git diff --check
  CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  ```
- Result: passed. Focused closed-action routing test passed; `validation_rework` passed 23/23; `action_contract_prompt` passed 29/29; formatting, whitespace, and `whale` build gates passed.
- Interpretation: H-072 is focused-fixed. A new keyed rerun is required to verify the live model now receives `TaskSpaceValidationReworkPatchOnlyRecoveryV1` after the first closed-action read rejection and either emits `apply_patch` or exposes the next tools-chain blocker.
