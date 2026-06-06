# Problem P-001: TaskSpace P0 benchmark run cannot produce clean E3 evidence
- Status: open
- Created: 2026-06-07 05:18
- Updated: 2026-06-07 05:52
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
- Ruled out:
  - The `recover-accuracy-log` failure is not evidence of agent failure; agent execution never started.
  - The `query-optimize` partial result is not clean evidence that TaskSpace failed; validator environment setup failed first.
- Fix criteria:
  - Prompt guard can distinguish internal TaskSpace leakage from external benchmark domain terms.
  - Terminal-Bench samples requiring remote assets are either preflighted and cached locally or marked ineligible before agent execution.
  - Metrics extraction handles locked/incomplete changed files without aborting the whole pair.
  - Run orchestration emits per-sample classifications and can resume audit/finalize deterministically.
  - A rerun of P0 smoke covers at least `recover-accuracy-log` and `query-optimize` through pair-report generation.
- Current conclusion: P0 did not complete because the benchmark harness is not robust enough for this harder external sample set; the failures are primarily harness/environment classification failures, with separate TaskSpace behavior issues visible inside the completed artifacts.
- Repair-plan conclusion: the fix must be stronger than making failed samples pass. It must add provenance, equivalence proof, taints, resumable run state, and audit boundaries so future failures are still cleanly classifiable.
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - not satisfied
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
