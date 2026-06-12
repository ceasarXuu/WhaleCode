# Problem P-001: TaskSpace P0 benchmark run cannot produce clean E3 evidence
- Status: open
- Created: 2026-06-07 05:18
- Updated: 2026-06-13 01:33
- Objective: explain why the Terminal-Bench P0 run stopped as partial, and define evidence-backed repair criteria before another P0/E3 run is trusted.
- Symptoms:
  - `recover-accuracy-log` exited before agent execution with `Scenario prompt leaks internal TaskSpace concepts: multi-agent`.
  - `query-optimize` produced a partial pair and exited during metrics extraction.
  - Completed samples still did not enter E3 aggregate without audit closure.
- Expected behavior:
  - External benchmark prompts should be rejected only when they leak Whale internal TaskSpace/test-control concepts, not when the benchmark's domain text happens to mention agents.
  - Validator and metrics failures should be classified as harness/environment failures without destroying partial evidence or producing misleading utility data.
  - A P0 run should either complete all configured pairs with audit-ready artifacts or clearly classify each sample as ineligible with a machine-readable reason.
- Actual behavior:
  - Prompt guard hard-fails on generic `multi-agent`.
  - `query-optimize` depends on downloading `oewn.sqlite` from HuggingFace during Docker build; the download failed, then metrics extraction attempted to hash a locked/incomplete SQLite file.
  - Aggregate reports exclude pairs until human audit is written, but the run orchestration did not automatically enter an audit/finalize phase.
- Impact:
  - P0 cannot currently be used as clean E3 utility evidence.
  - TaskSpace/standard comparison is polluted by prompt false positives, external network dependence, and harness extraction failures.
- Reproduction:
  - Run root: `D:\whalecode-alpha\target\benchp0-20260607-014707`.
  - P0 samples: `processing-pipeline`, `multi-source-data-merger`, `recover-accuracy-log`, `query-optimize`.
- Environment:
  - Branch: `whalecode-alpha`.
  - Terminal-Bench source revision: `1a6ffa9674b571da0ed040c470cb40c4d85f9b9b`.
  - Whale binary: `C:\Users\77585\.whale\bin\whale.exe`.
  - Model: `deepseek-v4-flash`, `model_reasoning_effort=max`.
- Known facts:
  - E-001: prompt guard treats `multi-agent` as a hard invalid pattern.
  - E-002: `recover-accuracy-log` task text contains `multi-agent system` as benchmark domain content.
  - E-003: `query-optimize` Docker build failed because `curl` could not connect to `huggingface.co`.
  - E-004: metrics extraction hashes changed files directly with `Get-FileHash`, which fails on locked files.
  - E-005: `processing-pipeline` pair report shows E3 candidate blocked only by missing human audit.
  - E-006: `multi-source-data-merger` pair reports mix business failure, validator timeout, manual-review, and E3 eligibility gates.
  - E-008: test-validity adversarial review found remaining self-deception risks in the first repair plan.
  - E-009: release/observability adversarial review found resumability and classification gaps in the first repair plan.
  - E-017: the 0.0.4 Phase 6 run has the same `Resolve-Path` uv-cache failure in all 30 validation stderr logs.
  - E-018: the 0.0.3 comparable run has no `Resolve-Path` uv-cache failures in its 30 validation stderr logs.
  - E-019: 0.0.4 generated validators embed a relative `uvCacheDir`, while 0.0.3 generated validators embed an absolute path.
  - E-020: the Terminal-Bench adapter writes `$uvCache.root` directly into generated validator scripts without resolving it to an absolute path.
  - E-021: `RunRoot` and Terminal-Bench adapter `OutputRoot` are joined and passed through without a consistent absolute-path boundary.
- Ruled out:
  - The `recover-accuracy-log` failure is not evidence of agent failure; agent execution never started.
  - The `query-optimize` partial result is not clean evidence that TaskSpace failed; validator environment setup failed first.
- Fix criteria:
  - Prompt guard can distinguish internal TaskSpace leakage from external benchmark domain terms.
  - Terminal-Bench samples requiring remote assets are either preflighted and cached locally or marked ineligible before agent execution.
  - Metrics extraction handles locked/incomplete changed files without aborting the whole pair.
  - Run orchestration emits per-sample classifications and can resume audit/finalize deterministically.
  - A rerun of P0 smoke covers at least `recover-accuracy-log` and `query-optimize` through pair-report generation.
- Current conclusion: P0 did not complete because the benchmark harness is not robust enough for this harder external sample set. The original 2026-06-07 failures were primarily harness/environment classification failures, with separate TaskSpace behavior issues visible inside the completed artifacts. The 2026-06-12/13 0.0.4 Phase 6 comparable run is additionally polluted by a confirmed validator materialization regression: generated validators can embed a relative uv-cache path, causing public validation to fail before tests run for both standard and TaskSpace sides.
- Repair-plan conclusion: the fix must be stronger than making failed samples pass. It must add provenance, equivalence proof, taints, resumable run state, and audit boundaries so future failures are still cleanly classifiable.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
- Resolution basis:
  - repair implemented; full P0 rerun not yet executed
  - fix-validation evidence: E-010, E-011, E-012, E-013, E-014
  - closure-hardening evidence: E-015, E-016
- Close reason:
  - not closed

## Hypothesis H-001: prompt guard over-classifies external benchmark domain terms
- Status: confirmed
- Parent: P-001
- Claim: `recover-accuracy-log` was rejected because `multi-agent` is a hard invalid prompt pattern, even though the term appears as task-domain content rather than a TaskSpace-control instruction.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - The benchmark task is explicitly about recovering logs for a generator/judge multi-agent evaluation system. This is normal task content.
- Falsifiable predictions:
  - If true: the guard code contains `multi-agent` in hard patterns and the task prompt contains `multi-agent system`.
  - If false: the failure would come from another internal term or a generated wrapper prompt.
- Diagnostic evidence plan:
  - Prediction or clause under test: hard-pattern and task-text correlation.
  - Signal: code pattern plus task.yaml text plus run stderr.
  - Capture method: inspect `prompt-guard.ps1`, task.yaml, and sample error log.
  - Event name or marker:
    - `Scenario prompt leaks internal TaskSpace concepts`
  - Correlation keys:
    - sample `recover-accuracy-log`
  - Differentiates from:
    - real prompt pollution by the harness.
  - Supports if:
    - `multi-agent` appears in the hard pattern list and benchmark domain text.
  - Refutes if:
    - wrapper prompt contains explicit TaskSpace/node/subagent instructions.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed.
- Repair design readiness: ready
- Next step: design contextual prompt guard categories and external benchmark allowlist metadata.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: `query-optimize` requires remote asset materialization during validation
- Status: confirmed
- Parent: P-001
- Claim: `query-optimize` is not a clean benchmark sample in the current environment because its Dockerfile downloads `oewn.sqlite` from HuggingFace during validator build.
- Layer: environment
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - E3 runner must isolate agent quality from network availability and external artifact drift.
- Falsifiable predictions:
  - If true: validator stderr contains a HuggingFace `curl` failure and Docker build failure before pair metrics are written.
  - If false: validator succeeds and metrics fail for an unrelated reason.
- Diagnostic evidence plan:
  - Prediction or clause under test: remote asset dependency caused validator failure.
  - Signal: validator stderr and Dockerfile command.
  - Capture method: inspect `validation.stderr.log`.
  - Event name or marker:
    - `curl: (28) Failed to connect to huggingface.co`
  - Correlation keys:
    - sample `query-optimize`, pair `pair-001`
  - Differentiates from:
    - agent-generated invalid SQL.
  - Supports if:
    - Docker build fails before tests can run because the asset cannot be downloaded.
  - Refutes if:
    - tests run and fail on solution correctness.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed.
- Repair design readiness: ready
- Next step: add preflight/cache/materialization rules for remote benchmark assets.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: metrics extraction treats locked files as fatal
- Status: confirmed
- Parent: P-001
- Claim: pair finalization can abort when `Get-FileHash` hits a locked or incomplete changed file, instead of recording an inventory warning and preserving available evidence.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-002
- Rationale:
  - Metrics extraction is diagnostic bookkeeping; it should not erase the pair's agent/validator evidence.
- Falsifiable predictions:
  - If true: `metrics-extractor.ps1` hashes every changed file directly, and the sample stderr shows `Get-FileHash` failed on `oewn.sqlite` because another process held it.
  - If false: metrics were never reached or failed from schema validation.
- Diagnostic evidence plan:
  - Prediction or clause under test: locked changed file aborts metrics extraction.
  - Signal: code location plus stderr.
  - Capture method: inspect `metrics-extractor.ps1` and sample stderr.
  - Event name or marker:
    - `Get-FileHash`
  - Correlation keys:
    - file `oewn.sqlite`
  - Differentiates from:
    - validator correctness failure.
  - Supports if:
    - the error references `Get-FileHash` and file-in-use.
  - Refutes if:
    - metrics extraction handles the error and still writes metrics.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-005
  - E-006
- Conclusion: confirmed.
- Repair design readiness: ready
- Next step: make changed-file inventory best-effort with explicit hash status.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: E3 runner lacks a complete post-run audit/finalize phase
- Status: confirmed
- Parent: P-001
- Claim: samples can produce E3-candidate evidence but remain excluded because audit review generation/finalization is not part of the P0 run loop.
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - The E3 rules require human review or an explicit audit artifact before inclusion.
- Falsifiable predictions:
  - If true: pair reports show `e3_human_review_not_completed` with otherwise clean candidate evidence.
  - If false: pair reports are excluded for business success or proof failures only.
- Diagnostic evidence plan:
  - Prediction or clause under test: missing audit blocks candidate aggregation.
  - Signal: pair report E3 gate.
  - Capture method: inspect `processing-pipeline` pair reports.
  - Event name or marker:
    - `e3_human_review_not_completed`
  - Correlation keys:
    - sample `processing-pipeline`
  - Differentiates from:
    - agent failure.
  - Supports if:
    - reports show no evidence failures but E3 human review missing.
  - Refutes if:
    - reports show business or proof failures.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-007
- Conclusion: confirmed.
- Repair design readiness: ready
- Next step: add an E3 run-state machine with explicit `execute -> classify -> audit -> finalize` phases.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-005: the first repair plan was underspecified for clean E3 reruns
- Status: confirmed
- Parent: P-001
- Claim: the initial repair direction was right but insufficient because it did not fully constrain prompt-source provenance, remote asset equivalence, metrics taints, resumable status, Docker failure categories, and audit completion.
- Layer: design-validation
- Factor relation: part_of
- Depends on:
  - H-001
  - H-002
  - H-003
  - H-004
- Rationale:
  - E3 evidence can become self-deceptive if the harness merely labels more cases as warnings or resumes runs without a verifiable state machine.
- Falsifiable predictions:
  - If true: independent reviewers will identify benchmark-validity and observability gaps that could still pollute E3 conclusions.
  - If false: reviewers should find the plan already sufficient for clean P0 reruns.
- Diagnostic evidence plan:
  - Prediction or clause under test: pre-implementation repair design completeness.
  - Signal: adversarial review outputs.
  - Capture method: two fresh read-only internal review agents with no inherited main-agent context.
  - Event name or marker:
    - `blocking findings`
  - Correlation keys:
    - review report `2026-06-07-taskspace-p0-benchmark-harness-repair-review.md`
  - Differentiates from:
    - implementation bugs introduced after the plan.
  - Supports if:
    - reviewers identify missing controls that could affect E3 validity.
  - Refutes if:
    - reviewers find only minor style or optional improvements.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-008
  - E-009
- Conclusion: confirmed.
- Repair design readiness: ready after plan amendments.
- Next step: implement the strengthened harness contracts before another P0/E3 run.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: prompt guard hard pattern includes multi-agent
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `scripts\taskspace-benchmark\lib\prompt-guard.ps1`
- Prediction or plan link:
  - H-001 hard-pattern and task-text correlation.
- Matched signal:
  - `(?i)\bmulti-agent\b`
- Correlation keys:
  - prompt guard hard pattern
- Raw content:
  ```text
  $hardPatterns = @(
      ...
      "(?i)\bmulti-agent\b",
      ...
  )
  invalid_prompt = $hardHits.Count -gt 0
  ```
- Interpretation: generic domain wording is currently treated as an unrecoverable internal prompt leak.
- Time: 2026-06-07 05:18

## Evidence E-002: recover-accuracy-log task text uses multi-agent as domain content
- Related hypotheses:
  - H-001
- Direction: supports
- Type: observation
- Source: `C:\Users\77585\AppData\Local\Temp\whale-real-external-benchmarks\terminal-bench\original-tasks\recover-accuracy-log\task.yaml`
- Prediction or plan link:
  - H-001 hard-pattern and task-text correlation.
- Matched signal:
  - `evaluate a multi-agent system`
- Correlation keys:
  - sample `recover-accuracy-log`
- Raw content:
  ```text
  Kevin, an ML researcher, wants to evaluate a multi-agent system.
  The multi-agent system contains a generator and a judge...
  ```
- Interpretation: the sample is about multi-agent evaluation logs; the term is not an instruction to Whale to use TaskSpace/subagents.
- Time: 2026-06-07 05:18

## Evidence E-003: recover sample failed before agent execution
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `D:\whalecode-alpha\target\benchp0-20260607-014707\sample-recover-accuracy-log.err.log`
- Prediction or plan link:
  - H-001 hard-pattern and task-text correlation.
- Matched signal:
  - `Scenario prompt leaks internal TaskSpace concepts: multi-agent`
- Correlation keys:
  - run root `benchp0-20260607-014707`
- Raw content:
  ```text
  Scenario prompt leaks internal TaskSpace concepts: multi-agent
  ```
- Interpretation: the sample was rejected by harness prompt validation, not by agent behavior.
- Time: 2026-06-07 05:18

## Evidence E-004: query-optimize validator failed on HuggingFace download
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: `D:\whalecode-alpha\target\benchp0-20260607-014707\runs\terminal_bench__query-optimize\20260607-042646-233\pair-001\left\artifacts\validation.stderr.log`
- Prediction or plan link:
  - H-002 remote asset dependency caused validator failure.
- Matched signal:
  - `curl: (28) Failed to connect to huggingface.co`
- Correlation keys:
  - sample `query-optimize`, pair `pair-001`
- Raw content:
  ```text
  curl: (28) Failed to connect to huggingface.co port 443 after 134052 ms: Couldn't connect to server
  Dockerfile:46
  RUN curl -L -o /app/oewn.sqlite "https://huggingface.co/datasets/.../oewn.sqlite"
  ```
- Interpretation: the validator did not reach a stable test phase; it failed while materializing a remote DB asset.
- Time: 2026-06-07 05:18

## Evidence E-005: metrics extraction hashes changed files directly
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `scripts\taskspace-benchmark\lib\metrics-extractor.ps1`
- Prediction or plan link:
  - H-003 locked changed file aborts metrics extraction.
- Matched signal:
  - `Get-FileHash`
- Correlation keys:
  - changed file inventory
- Raw content:
  ```text
  $sha = (Get-FileHash -Algorithm SHA256 -LiteralPath $absolute).Hash.ToLowerInvariant()
  ```
- Interpretation: a hash read failure currently has no local recovery path.
- Time: 2026-06-07 05:18

## Evidence E-006: query metrics failed on locked SQLite file
- Related hypotheses:
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: `D:\whalecode-alpha\target\benchp0-20260607-014707\sample-query-optimize.err.log`
- Prediction or plan link:
  - H-003 locked changed file aborts metrics extraction.
- Matched signal:
  - `Get-FileHash` and `oewn.sqlite`
- Correlation keys:
  - file `oewn.sqlite`
- Raw content:
  ```text
  Get-FileHash : cannot read file ...\oewn.sqlite:
  file is being used by another process.
  ```
- Interpretation: metrics failed while reading an artifact that should not be allowed to abort the benchmark pair.
- Time: 2026-06-07 05:18

## Evidence E-007: E3 candidates were excluded solely because audit was missing
- Related hypotheses:
  - H-004
- Direction: supports
- Type: observation
- Source: `D:\whalecode-alpha\target\benchp0-20260607-014707\runs\terminal_bench__processing-pipeline\20260607-014712-519\pair-001\pair-report.md`
- Prediction or plan link:
  - H-004 missing audit blocks candidate aggregation.
- Matched signal:
  - `e3_human_review_not_completed`
- Correlation keys:
  - sample `processing-pipeline`, pair `pair-001`
- Raw content:
  ```text
  reported_evidence_level: E3-candidate
  Evidence Gate Failures: none
  E3 Gate Failures: e3_human_review_not_completed
  ```
- Interpretation: the runner produced candidate evidence but did not complete the audit stage required for inclusion.
- Time: 2026-06-07 05:18

## Evidence E-008: test-validity adversarial review found remaining self-deception risks
- Related hypotheses:
  - H-005
- Direction: supports
- Type: adversarial-review
- Source: `vs_review\2026-06-07-taskspace-p0-benchmark-harness-repair-review.md`
- Prediction or plan link:
  - H-005 pre-implementation repair design completeness.
- Matched signal:
  - prompt allowlist provenance, remote asset equivalence, critical metrics taints, explicit audit completion.
- Correlation keys:
  - reviewer `test-validity-adversary`, session `019e9ec7-239e-7682-a1a5-544b9c040dce`
- Raw content:
  ```text
  Prompt guard allowlists lacked provenance constraints.
  Remote asset caching could break Terminal-Bench equivalence.
  Metrics best-effort handling cannot treat critical fixture/hash failures as harmless warnings.
  Audit automation was too ambiguous.
  ```
- Interpretation: the first repair plan could still allow misleading E3 conclusions if these boundaries were not strengthened.
- Time: 2026-06-07 05:52

## Evidence E-009: release/observability adversarial review found resumability and classification gaps
- Related hypotheses:
  - H-005
- Direction: supports
- Type: adversarial-review
- Source: `vs_review\2026-06-07-taskspace-p0-benchmark-harness-repair-review.md`
- Prediction or plan link:
  - H-005 pre-implementation repair design completeness.
- Matched signal:
  - status schema, atomic writes, events log, stale locks, cache trust boundary, Docker error taxonomy, PowerShell scan boundary.
- Correlation keys:
  - reviewer `release-ops-and-observability-adversary`, session `019e9ec7-7a72-77e2-b7fc-b7fea6da91da`
- Raw content:
  ```text
  run-status.json and sample-status.json were underspecified for recovery.
  Remote cache trust boundary was weak.
  Docker failures were still too coarse.
  Windows/PowerShell artifact scanning was not hardened.
  ```
- Interpretation: the harness needs operational state and structured error evidence before another P0 run can be trusted.
- Time: 2026-06-07 05:52

## Evidence E-010: harness self-test passed after prompt, metrics, run-state, and adapter changes
- Related hypotheses:
  - H-001
  - H-003
  - H-005
- Direction: supports
- Type: fix-validation
- Source: command `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-harness.ps1`
- Prediction or plan link:
  - Prompt provenance, locked-file metrics handling, run-state events, and Terminal-Bench remote asset metadata should be covered by automated harness assertions.
- Matched signal:
  - `TaskSpace benchmark harness self-test: PASS`
- Correlation keys:
  - run root `D:\whalecode-alpha\target\paired-bench-selftest\single-file-fast-fix\20260607-054052-421`
- Raw content:
  ```text
  TaskSpace benchmark harness self-test: PASS
  ```
- Interpretation: the focused harness regression confirms the repaired code paths at unit/script level.
- Time: 2026-06-07 05:55

## Evidence E-011: remote asset preflight stops before Whale execution
- Related hypotheses:
  - H-002
  - H-005
- Direction: supports
- Type: fix-validation
- Source: command `run-taskspace-benchmark.ps1 -ScenarioPath <remote asset fixture> -WhaleBin Z:\missing\whale.exe`
- Prediction or plan link:
  - A Terminal-Bench sample with unproven remote assets should be classified before agent execution and before checking the Whale binary.
- Matched signal:
  - exit code `2`, `phase=ineligible`, `ineligible_reason=environment_remote_asset_unavailable`
- Correlation keys:
  - run root `D:\whalecode-alpha\target\remote-preflight-smoke\terminal_bench__remote\20260607-054138-404`
- Raw content:
  ```text
  SampleStatus: ...\sample-status.json
  RemoteAssetPreflight: ...\preflight.remote-assets.json
  "phase": "ineligible"
  "ineligible_reason": "environment_remote_asset_unavailable"
  "environment_failure_reason": "remote_asset_equivalence_unproven"
  ```
- Interpretation: `query-optimize`-style remote asset failures can now be excluded as environment-ineligible before agent work starts.
- Time: 2026-06-07 05:55

## Evidence E-012: oracle runner, uv cache, and E3 proof harnesses still pass
- Related hypotheses:
  - H-002
  - H-004
  - H-005
- Direction: supports
- Type: fix-validation
- Source: commands `test-oracle-runner-harness.ps1`, `test-terminal-bench-uv-cache-harness.ps1`, `test-e3-proof-harness.ps1`
- Prediction or plan link:
  - Existing oracle, cache, and E3 proof checks should remain compatible with the repair.
- Matched signal:
  - all three scripts reported `PASS`
- Correlation keys:
  - `oracle-runner-selftest`
  - `terminal-bench-uv-cache-selftest`
  - `e3-proof-selftest`
- Raw content:
  ```text
  TaskSpace oracle-runner self-test: PASS
  Terminal-Bench uv cache self-test: PASS
  TaskSpace E3 proof harness self-test: PASS
  ```
- Interpretation: the repair did not break adjacent proof/cache/oracle harnesses.
- Time: 2026-06-07 05:55

## Evidence E-013: normal PlanOnly smoke still succeeds
- Related hypotheses:
  - H-005
- Direction: supports
- Type: fix-validation
- Source: command `run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -PlanOnly`
- Prediction or plan link:
  - Samples without remote asset blockers should still pass prompt guard and plan-only setup.
- Matched signal:
  - `PromptInvalid: False`, `PromptManualReview: False`
- Correlation keys:
  - run root `D:\whalecode-alpha\target\planonly-smoke\single-file-fast-fix\20260607-054247-588`
- Raw content:
  ```text
  RunDir: ...\target\planonly-smoke\single-file-fast-fix\20260607-054247-588
  PromptInvalid: False
  PromptManualReview: False
  ```
- Interpretation: the preflight/run-state additions do not block ordinary non-remote benchmark setup.
- Time: 2026-06-07 05:55

## Evidence E-014: real one-pair benchmark smoke writes complete run state
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Direction: supports
- Type: fix-validation
- Source: command `run-taskspace-benchmark.ps1 -Scenario single-file-fast-fix -Repeats 1 -AllowNonE2Result`
- Prediction or plan link:
  - A real agent-execution pair should complete with run/sample status, event timeline, metrics warnings/taints fields, and pair report.
- Matched signal:
  - script exit code `0`, run phase `completed`, completed pairs `1`, pair report `E2-candidate`
- Correlation keys:
  - run root `D:\whalecode-alpha\target\repair-real-smoke\single-file-fast-fix\20260607-054431-059`
- Raw content:
  ```text
  RunSummary: ...\run-summary.md
  PairReport: ...\pair-001\pair-report.md
  "phase": "completed"
  "attempted_pairs": 1
  "completed_pairs": 1
  reported_evidence_level: E2-candidate
  metrics_warnings:
  metrics_taints:
  ```
- Interpretation: the repaired state and metrics plumbing works through an actual Whale execution path. The evidence level is intentionally not E2 because this smoke used one repeat.
- Time: 2026-06-07 05:55

## Evidence E-015: closure hardening tests passed after reviewer blocking findings
- Related hypotheses:
  - H-005
- Direction: supports
- Type: fix-validation
- Source: commands `test-harness.ps1`, `test-e3-proof-harness.ps1`, remote preflight smoke, real one-pair smoke after closure hardening.
- Prediction or plan link:
  - Accepted blocking review findings should be reflected in tests for prompt control rejection, remote asset cache/injection, run-state metadata, Docker result parsing, and adapter file-size split.
- Matched signal:
  - harness and E3 proof self-tests passed; remote preflight exited through expected ineligible path; real one-pair smoke completed.
- Correlation keys:
  - `target\paired-bench-selftest\single-file-fast-fix\20260607-060148-038`
  - `target\e3-proof-selftest\20260607-060147-638`
  - `target\remote-preflight-smoke-3\terminal_bench__remote\20260607-060227-713`
  - `target\repair-real-smoke-3\single-file-fast-fix\20260607-060227-749`
- Raw content:
  ```text
  TaskSpace benchmark harness self-test: PASS
  TaskSpace E3 proof harness self-test: PASS
  RemoteAssetPreflight: ...\preflight.remote-assets.json
  PairReport: ...\pair-001\pair-report.md
  ```
- Interpretation: closure-hardening changes are validated at script level and through one real Whale pair. Full P0 is still not rerun in this step.
- Time: 2026-06-07 06:05

## Evidence E-016: resume smoke reuses existing run and skips completed pair
- Related hypotheses:
  - H-004
  - H-005
- Direction: supports
- Type: fix-validation
- Source: command sequence `run-taskspace-benchmark.ps1 ...` followed by `run-taskspace-benchmark.ps1 ... -ResumeLatest`
- Prediction or plan link:
  - A resumed run should consume existing status/artifacts and skip an already completed pair instead of rerunning it.
- Matched signal:
  - same `RunDir` printed twice; `events.jsonl` contains `resume_requested` and `pair_skipped_completed`.
- Correlation keys:
  - run root `D:\whalecode-alpha\target\resume-real-smoke\single-file-fast-fix\20260607-061536-791`
- Raw content:
  ```text
  RunDir: ...\20260607-061536-791
  RunDir: ...\20260607-061536-791
  event=resume_requested
  event=pair_skipped_completed
  ```
- Interpretation: the runner now has a real resume entrypoint for completed pair reuse. This is not a full phase-by-phase resume matrix, but it closes the previous "metadata only" blocker for completed run recovery.
- Time: 2026-06-07 06:20

## Hypothesis H-006: 0.0.4 Phase 6 validators embed a relative uv-cache path
- Status: confirmed
- Parent: P-001
- Claim: the 0.0.4 Phase 6 comparable run's all-standard/all-TaskSpace public-validation failures were caused by generated Terminal-Bench validators embedding a relative `uvCacheDir`, which `Resolve-Path` evaluates from each pair execution directory and fails before the Docker validator runs real tests.
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - Standard mode was not expected to regress to 0/15 without a harness/environment change. The raw validation logs show a common pre-test path-resolution failure on both logical modes, not solution-specific assertion failures.
- Falsifiable predictions:
  - If true: every 0.0.4 validation stderr should show the same `Resolve-Path` failure for `target\bench005-20260612-phase6\materialized-scenarios\_adapter-generated\uv-cache`, while the uv-cache exists at the absolute run-root path.
  - If true: the 0.0.3 comparable validators should embed absolute uv-cache paths and their validation stderr logs should not contain this failure.
  - If false: public validation should reach Docker build/test output and failures should vary by solution or sample rather than failing at the same path-resolution line.
- Diagnostic evidence plan:
  - Prediction or clause under test: common pre-test relative uv-cache path-resolution failure.
  - Signal: validation stderr group counts, generated validator path literals, and adapter generator source.
  - Capture method: inspect 0.0.4 and 0.0.3 validation stderr logs, generated `external-validator.ps1`, and `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`.
  - Event name or marker:
    - `Resolve-Path : Cannot find path`
  - Correlation keys:
    - run root `D:\whalecode-alpha\target\bench005-20260612-phase6`
    - run root `D:\whalecode-alpha\target\bench004-20260608-202551`
  - Differentiates from:
    - agent answer quality regression
    - Terminal-Bench task-level assertion failure
    - Docker availability failure
  - Supports if:
    - 0.0.4 logs all fail on relative uv-cache resolution and 0.0.3 logs do not.
  - Refutes if:
    - failures occur after Docker/test execution or the generated 0.0.4 validator uses an absolute uv-cache path.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-017
  - E-018
  - E-019
  - E-020
  - E-021
- Conclusion: confirmed. The 0.0.4 Phase 6 diagnostic pass-rate comparison is invalid as an agent-quality comparison; the run is polluted by validator materialization failure shared by both standard and TaskSpace sides.
- Repair design readiness: ready
- Next step: repair the adapter so generated validators always embed absolute uv-cache paths, add a regression test that invokes generated validation from a pair working directory, then rerun the comparable Phase 6 scope.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-017: 0.0.4 validation stderr logs all fail on uv-cache Resolve-Path
- Related hypotheses:
  - H-006
- Direction: supports
- Type: diagnostic-log
- Source: command grouping `target\bench005-20260612-phase6\runs\**\validation.stderr.log` by `uv-cache` and `Resolve-Path`.
- Prediction or plan link:
  - H-006 prediction: every 0.0.4 validation stderr should show the same pre-test relative uv-cache path-resolution failure.
- Matched signal:
  - all 30 validation stderr logs matched the uv-cache `Resolve-Path` failure.
- Correlation keys:
  - run root `D:\whalecode-alpha\target\bench005-20260612-phase6`
- Raw content:
  ```text
  Name Count
  ---- -----
  True    30

  Resolve-Path : Cannot find path 'target\bench005-20260612-phase6\materialized-scenarios\_adapter-generated\uv-cache' because it does not exist.
  ```
- Interpretation: the 0.0.4 run did not produce valid public-validation pass/fail evidence for agent quality; both logical modes failed on a shared validator setup error.
- Time: 2026-06-13 01:33

## Evidence E-018: 0.0.3 validation stderr logs do not show the uv-cache Resolve-Path failure
- Related hypotheses:
  - H-006
- Direction: supports
- Type: diagnostic-log
- Source: command grouping `target\bench004-20260608-202551\runs\**\validation.stderr.log` by `uv-cache` and `Resolve-Path`.
- Prediction or plan link:
  - H-006 prediction: the 0.0.3 comparable run should not contain the 0.0.4 relative uv-cache failure.
- Matched signal:
  - all 30 comparable 0.0.3 validation stderr logs were negative for the uv-cache `Resolve-Path` pattern.
- Correlation keys:
  - run root `D:\whalecode-alpha\target\bench004-20260608-202551`
- Raw content:
  ```text
  Name  Count
  ----  -----
  False    30
  ```
- Interpretation: the all-failed 0.0.4 validation pattern is a new run/materialization defect, not a stable property of the benchmark or standard mode.
- Time: 2026-06-13 01:33

## Evidence E-019: generated validators changed from absolute to relative uvCacheDir
- Related hypotheses:
  - H-006
- Direction: supports
- Type: code-location
- Source: generated validators under `target\bench005-20260612-phase6\materialized-scenarios` and `target\bench004-20260608-202551\materialized-scenarios`.
- Prediction or plan link:
  - H-006 prediction: 0.0.4 generated validators should embed a relative uv-cache path while 0.0.3 uses an absolute path.
- Matched signal:
  - 0.0.4 uses `target\bench005...`; 0.0.3 uses `D:\whalecode-alpha\target\bench004...`.
- Correlation keys:
  - line `external-validator.ps1:108`
- Raw content:
  ```text
  0.0.4:
  $uvCacheDir = 'target\bench005-20260612-phase6\materialized-scenarios\_adapter-generated\uv-cache'

  0.0.3:
  $uvCacheDir = 'D:\whalecode-alpha\target\bench004-20260608-202551\materialized-scenarios\_adapter-generated\uv-cache'
  ```
- Interpretation: the literal path embedded in generated validator scripts is sufficient to explain why the 0.0.4 script fails when launched from a pair directory.
- Time: 2026-06-13 01:33

## Evidence E-020: adapter writes the cache root literal without absolutizing it
- Related hypotheses:
  - H-006
- Direction: supports
- Type: code-location
- Source: `scripts\taskspace-benchmark\adapters\terminal-bench-adapter.ps1` and `terminal-bench-uv-cache.ps1`.
- Prediction or plan link:
  - H-006 prediction: generator source should allow a relative `OutputRoot` to become a relative `uvCacheDir` literal in generated validators.
- Matched signal:
  - `New-TerminalBenchUvCache` builds `$cache` from `$OutputRoot`, and `terminal-bench-adapter.ps1` writes `$uvCache.root` directly into `$uvCacheLiteral`.
- Correlation keys:
  - `terminal-bench-uv-cache.ps1:3`
  - `terminal-bench-adapter.ps1:126`
  - `terminal-bench-adapter.ps1:130`
  - `terminal-bench-adapter.ps1:296`
- Raw content:
  ```text
  $cache = Join-Path $OutputRoot "_adapter-generated\uv-cache"
  $uvCache = New-TerminalBenchUvCache $OutputRoot
  $uvCacheLiteral = "'" + $uvCache.root.Replace("'", "''") + "'"
  "`$uvCacheDir = $uvCacheLiteral"
  ```
- Interpretation: if Phase 6 passes a relative run/materialization root into the adapter, the generated validator records a relative cache path. That relative path is then resolved from the later pair execution directory and fails before public validation tests run.
- Time: 2026-06-13 01:33

## Evidence E-021: runner and adapter do not enforce an absolute path boundary
- Related hypotheses:
  - H-006
- Direction: supports
- Type: code-location
- Source: `scripts\taskspace-benchmark\run-taskspace-benchmark.ps1`, `lib\workspace.ps1`, and `adapters\terminal-bench-adapter.ps1`.
- Prediction or plan link:
  - H-006 prediction: a relative run root can propagate into generated validator materialization paths.
- Matched signal:
  - the runner only defaults `RunRoot` when empty, then passes it through `Join-Path`; the workspace helper creates run dirs via `Join-Path $RunRoot`; the adapter accepts `OutputRoot` and uses it for `_adapter-generated` and uv-cache creation without resolving it.
- Correlation keys:
  - `run-taskspace-benchmark.ps1:47`
  - `run-taskspace-benchmark.ps1:73`
  - `workspace.ps1:7`
  - `terminal-bench-adapter.ps1:125-130`
- Raw content:
  ```text
  if (-not $RunRoot) { $RunRoot = Get-NeutralTaskspaceBenchmarkRunRoot $repoRoot }
  $runDir = Join-Path (Join-Path $RunRoot $manifest.Id) $RunId
  New-Dir (Join-Path $RunRoot "$ScenarioId\$stamp")
  $generatedDir = New-TaskspaceExternalDir (Join-Path $OutputRoot "_adapter-generated")
  $uvCache = New-TerminalBenchUvCache $OutputRoot
  ```
- Interpretation: the durable fix should establish an absolute path boundary either immediately after `RunRoot` input normalization, inside the adapter for `OutputRoot`, or both. Fixing only one generated script would not address the root cause.
- Time: 2026-06-13 01:33
