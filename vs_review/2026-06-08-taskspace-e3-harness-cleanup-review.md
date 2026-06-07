# TaskSpace E3 Harness Cleanup Review

## Round 1: Pre-Implementation Review

### Review Input

Objective: fix TaskSpace E3 engineering blockers before running another benchmark. The user wants a plan, adversarial review, blocker fixes, then execution.

Review target:
- `docs/testing/2026-06-08-taskspace-0.0.3-engineering-fix-plan.md`
- `scripts/taskspace-benchmark/lib/oracle-runner.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- related tests under `scripts/taskspace-benchmark/test-*.ps1`

Known symptom from TaskSpace 0.0.2:
- Some Terminal-Bench public validator Docker containers stayed `Up` after pair reports and aggregate were already written.
- Parent benchmark driver stayed blocked until containers were manually stopped.
- Observed containers included `whale-tbench-a36416c2c3c3eb8e`, `whale-tbench-edae1b09396d3e14`, `whale-tbench-2ff3f1da5204be26`.

Current code facts to inspect:
- `Invoke-TaskspaceValidationCommand` catches `Invoke-RealProcess` timeout and returns `124`.
- Terminal-Bench adapter generates validator PowerShell with `docker run --label whale.taskspace.terminal_bench=true` and an internal `finally` that runs `docker rm -f` and `docker rmi -f`.
- `Invoke-RealProcess` kills the started process tree on timeout, but previous run indicates Docker/WSL descendants may remain.

Proposed direction:
- Add parent-side, run-specific validation cleanup driven by validator runtime manifest under the validation proof dir.
- Never cleanup by broad label alone.
- Preserve fail-closed semantics: timeout remains exit code `124`.
- Add cleanup artifact such as `validation-cleanup-result.json`.
- Add tests proving timeout cleanup is attempted and timeout is not hidden.

Reviewer instructions:
- Fresh internal subagent session; do not inherit main-agent context.
- Read the target files directly.
- Do not modify files.
- Report blocking findings, non-blocking risks, missing tests, and evidence paths/lines when possible.

### Reviewer Launch Records

- Reviewer A: Windows/Docker timeout cleanup reviewer
  - mechanism: `multi_agent_v1.spawn_agent`
  - agent_id: `019ea3b5-64f2-7441-a7f9-80efc2276e9d`
  - fork_context: `false`
  - input: read-only review packet in this report's Review Input section, focused on PowerShell -> WSL -> Docker cleanup precision.
  - excluded context: main conversation history, drafts, and implementation diff.
- Reviewer B: E3 evidence integrity reviewer
  - mechanism: `multi_agent_v1.spawn_agent`
  - agent_id: `019ea3b5-9385-7521-807d-00503f3be0f7`
  - fork_context: `false`
  - input: read-only review packet in this report's Review Input section, focused on metrics/evidence/audit contamination risk.
  - excluded context: main conversation history, drafts, and implementation diff.

### Reviewer Outputs

#### Reviewer A

Summary: parent-side cleanup is necessary, but the original plan was not strict enough.

Blocking findings:
- Accept: cleanup by exact container name alone is insufficient. The container name was based on repo path hash and did not prove the container belongs to this proof/run. Cleanup must bind and verify labels such as `repo_hash`, `proof_nonce`, and proof/artifact identity before `docker rm -f`.
- Accept: timeout and cleanup failures must be structurally classified, not only returned as exit code `124`.
- Accept: cleanup must be no-throw and must not replace the original validation exit code.
- Accept: WSL backend cleanup must record and reuse the same distro.
- Accept: tests must cover fake WSL/Docker style command capture and exact identity cleanup, plus real Docker smoke later.

Non-blocking risks:
- Accept: image cleanup can interfere with parallel runs if tags are shared. Use run-unique tags.
- Accept: runtime manifest should include `repo_hash`.
- Accept: E3 proof should expose parent cleanup artifact, not only child-script cleanup markers.

#### Reviewer B

Summary: cleanup alone is not enough; validation timeout and cleanup results must enter the complete evidence chain.

Blocking findings:
- Accept: public validation timeout can otherwise be mistaken for a standard/taskspace success delta. It must become `public_validation_timeout` environment failure.
- Accept: `finalize-taskspace-e3-run.ps1` was dropping `metrics_taints` and `validator_environment_failures`, which could make an existing run look cleaner after finalization.
- Accept: `validation-cleanup-result.json` must be part of metrics/proof/audit artifacts.

Non-blocking risks:
- Accept: version registry must state 0.0.3 is a harness controllability fix, not a TaskSpace utility result.
- Accept: missing runtime manifest must be fail-closed and recorded as `cleanup_not_attempted_manifest_missing`.

### Main-Agent Responses

- Accepted Reviewer A / container identity binding:
  - `terminal-bench-adapter.ps1` now generates run-unique Docker image/container names using `repoHash` plus `proofNoncePrefix`.
  - Docker labels now include `whale.taskspace.repo_hash`, `whale.taskspace.proof_nonce`, and `whale.taskspace.proof_dir_hash`.
  - `oracle-runner.ps1` reads runtime manifest, runs `docker inspect`, verifies labels, and only then calls `docker rm -f`.
- Accepted Reviewer A / no-throw cleanup:
  - cleanup helpers return structured result objects and write `validation-cleanup-result.json`.
  - timeout still returns `124`; cleanup writes artifacts and stderr markers.
- Accepted Reviewer A / WSL identity:
  - runtime manifest and cleanup result include `wsl_distro`; parent cleanup reuses the manifest distro.
- Accepted Reviewer A / run-unique image tags:
  - generated image tag is now `whale-taskspace-terminal-bench:<repoHash>-<proofNoncePrefix>`.
- Accepted Reviewer B / timeout classification:
  - `Get-TaskspaceDockerValidationResult` emits `public_validation_timeout` when validation exit code is `124`.
  - `test-harness.ps1` asserts `public_validation_timeout` prevents E3 aggregate inclusion.
- Accepted Reviewer B / finalize preservation:
  - `finalize-taskspace-e3-run.ps1` now re-aggregates `metrics_taints` and `validator_environment_failures` and passes them into `Get-TaskspaceEvidenceGate`.
- Accepted Reviewer B / audit and proof artifacts:
  - `audit-report.ps1` requires Terminal-Bench side artifacts for `terminal-bench-runtime-manifest.json` and `validation-cleanup-result.json`.
  - `e3-proof.ps1` records parent cleanup result path, under-artifact status, classification, and identity match status.

Validation run after fixes:
- `.\scripts\taskspace-benchmark\test-oracle-runner-harness.ps1`: PASS
- `.\scripts\taskspace-benchmark\test-harness.ps1`: PASS
- `.\scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1`: PASS

### Closure Status

- Round 2 closure review completed by `019ea3bf-7c26-7bf2-a0b1-48534a18eb5d`.
- Accepted remaining blocking finding: cleanup invocation still needed a strict no-throw wrapper.
  - Action: `Invoke-TaskspaceValidationCommand` now calls `Invoke-TaskspaceValidationCleanupNoThrow`, which catches cleanup exceptions and preserves the original validation exit code.
  - Regression: `test-oracle-runner-harness.ps1` now covers invalid cleanup manifest while preserving timeout exit `124`.
- Accepted remaining blocking finding: finalize preservation needed an executable regression.
  - Action: `test-harness.ps1` now constructs a minimal run directory and invokes `finalize-taskspace-e3-run.ps1`, asserting `public_validation_timeout`, metrics taint, and E3 aggregate exclusion survive finalization.
  - Action: `finalize-taskspace-e3-run.ps1` also imports `report-summary.ps1` and `aggregate-report.ps1`, which are required by its own summary output path.
- Accepted residual proof risk:
  - Action: `e3-proof.ps1` now requires cleanup result to live under the artifact root with classification `ok` for runtime proof.
  - Regression: `test-e3-proof-harness.ps1` fixture now includes `validation-cleanup-result.json`.

Validation run after closure fixes:
- `.\scripts\taskspace-benchmark\test-oracle-runner-harness.ps1`: PASS
- `.\scripts\taskspace-benchmark\test-harness.ps1`: PASS
- `.\scripts\taskspace-benchmark\test-e3-proof-harness.ps1`: PASS
- `.\scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1`: PASS

Round 3 closure review pending before real E3 execution.

Round 3 closure review completed by `019ea3c8-0d71-7200-b4af-55672d8fb6c1`.
- Accepted remaining blocking finding: cleanup failure path needed executable regression proving validation timeout still returns `124`.
  - Action: `test-oracle-runner-harness.ps1` now uses a fake Docker shim where `docker rm -f` returns exit code `7`; the test asserts `Invoke-TaskspaceValidationCommand` still returns `124` and writes `docker_cleanup_container_failure`.
- Accepted remaining blocking finding: finalize regression needed to assert cleanup-specific environment failure is preserved.
  - Action: `test-harness.ps1` now asserts finalized `pair-report.md` still includes `docker_cleanup_container_failure`.

Validation run after final closure fixes:
- `.\scripts\taskspace-benchmark\test-oracle-runner-harness.ps1`: PASS
- `.\scripts\taskspace-benchmark\test-harness.ps1`: PASS
- `.\scripts\taskspace-benchmark\test-e3-proof-harness.ps1`: PASS
- `git diff --check`: PASS

Closure decision: blocking review items are closed; proceed to real Terminal-Bench smoke before any larger E3 run.

Post-closure execution:
- First real `hello-world` smoke exposed a generated validator parser bug: duplicate `proof_nonce` key in the runtime manifest hash literal. Fixed in `terminal-bench-adapter.ps1` and covered by a PowerShell parser check in `test-terminal-bench-adapter-harness.ps1`.
- Real smoke `D:\whalecode-alpha\target\bench003smoke-20260608-043849` proved timeout cleanup worked and left no Docker label residuals, but cleanup marker extraction was incomplete.
- Marker writer was fixed to emit `validation_cleanup_result_path` on a standalone stderr line; metrics regex was made robust.
- Final real smoke `D:\whalecode-alpha\target\bench003smoke2-20260608-044924` proved metrics/proof can read cleanup artifacts and no Docker label residuals remain.
