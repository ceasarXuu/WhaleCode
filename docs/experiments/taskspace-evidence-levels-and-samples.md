# TaskSpace Evidence Levels And Sample Sets

- Status: Ready for use
- Created: 2026-06-18
- Updated: 2026-06-18
- Scope: TaskSpace benchmark and release evidence governance

## 1. Purpose

This document fixes a process failure: the v0.0.5 closeout discussion used a near-100% internal matrix result in a way that could be mistaken for Terminal-Bench E3 accuracy. That is not acceptable for release judgment.

From now on, every TaskSpace experiment must identify both:

1. The evidence level: E1, E2, E3, E4, or E5.
2. The sample set: the exact registered task/scenario names used in the run.

Evidence level is not task difficulty. Difficulty labels like `L1`, `L2`, and `L3` describe scenario shape. Evidence labels like `E1`, `E2`, and `E3` describe how strong a claim the result can support.

## 2. Evidence Levels

| Level | Name | Source | Minimum gates | Allowed claim | Not allowed |
|---|---|---|---|---|---|
| E1 | Mechanism smoke | Internal fixture or focused local scenario | Reproducible prompt, fixture, validator or deterministic manual check | A mechanism path can execute and produce artifacts | Version accuracy, utility, release, or external benchmark claims |
| E2 | Internal engineering regression | Internal constructed scenarios | Paired Standard/TaskSpace run, repeats >= 3, prompt guard clean, provider params complete, aggregate enabled, oracle isolation not failed | Internal scenario utility and regression readiness for the named fixture set | Terminal-Bench, DeepSWE, external benchmark, or product-level accuracy claims |
| E3-candidate | External utility candidate | External benchmark or sanitized historical Whale failure | E3 source metadata and paired artifacts exist, but at least one E3 gate is still pending | Engineering-clean external candidate awaiting audit or proof closure | Final E3 score, release pass, or public product claim |
| E3 | External utility evidence | Terminal-Bench, DeepSWE, or audited historical Whale failure | Repeats >= 5, pinned source, original prompt checksum, validator checksum, official/equivalent validator proof, validator/source isolation proof, paired artifacts, completed audit review | External utility claim for the declared claim scope and sample set | General product claim outside the sample scope |
| E4 | Release calibration evidence | Registered blend of E3 plus required internal regressions | Not implemented in the current runner; must include same-scope comparison to prior release, profile identity, cost gates, score validity, and explicit release decision | Version release readiness for the declared release profile | Any claim before E4 tooling and sample set are registered |
| E5 | Product benchmark board | Frozen long-running benchmark board | Not implemented; must include independent review, recurring schedule, stable sample board, statistical thresholds, cost accounting, and competitor/baseline policy | Product-level benchmark trend or external positioning | Any current v0.0.x release claim |

Current code implements `E1`, `E2`, `E2-candidate`, `E3-candidate`, and `E3`. `E4` and `E5` are governance definitions only until runner support and sample boards are added.

## 3. Candidate Rules

Candidate evidence is intentionally lower than the target level.

| Reported level | Meaning | Can satisfy target |
|---|---|---|
| `E1` | Smoke or degraded evidence | E1 only |
| `E2-candidate` | Internal utility candidate with unresolved E2 gate | No |
| `E2` | Internal utility evidence | E2 |
| `E3-candidate` | External utility candidate with pending E3 gate | No |
| `E3` | Completed external utility evidence | E3 |

If a report says `requested_evidence_target: E3` but `reported_evidence_level: E3-candidate`, the result is not E3.

## 4. Registered Sample Sets

### 4.1 E1: Internal Smoke Set

Sample set id: `taskspace-internal-smoke-v005`

| Sample | Definition | Current manifest target | Purpose |
|---|---|---|---|
| `count-call-stack` | Internal parser/format-sensitive fixture with `scripts/validate.py` oracle | E1 | Tests thin and verification-first behavior around exact output format. |
| `large-output-ref-smoke` | Internal large-output replay fixture with pytest oracle | E1 | Tests output reference creation and large output replay control. |

Allowed conclusion: these scenarios can show whether specific mechanisms run. They cannot establish TaskSpace utility or release readiness.

### 4.2 E2: v0.0.5 Internal Regression Matrix

Sample set id: `taskspace-internal-regression-v005`

| Sample | Definition | Scenario level | Current manifest target | Purpose |
|---|---|---:|---|---|
| `single-file-fast-fix` | Internal single-file Python tax calculation fix with pytest oracle | L1 | E2 | Ensures TaskSpace does not damage trivial deterministic fixes. |
| `multi-file-order-pipeline` | Internal multi-file order processing repair with README/test conflict handling | L2 | E2 | Exercises multi-file reasoning and local regression validation. |
| `subscription-billing-repair` | Internal subscription billing repair with wider code surface | L3 | E2 | Exercises broader edit scope and cost warning behavior. |

The v0.0.5 closeout matrix also included the E1 smoke samples above, so the combined run was a mixed internal E1/E2 engineering matrix, not Terminal-Bench E3.

### 4.3 E3 Candidate: Terminal-Bench Original Four

Sample set id: `terminal-bench-original-4`

| Sample | Definition | Historical use |
|---|---|---|
| `hello-world` | Terminal-Bench introductory file/task validation sample | Pre-version full benchmark and E3 harness proof work. |
| `heterogeneous-dates` | Terminal-Bench data/date normalization task | Pre-version external utility exploration. |
| `jsonl-aggregator` | Terminal-Bench JSONL aggregation task | Exposed TaskSpace node growth and cost amplification. |
| `log-summary` | Terminal-Bench log summarization task | Exposed mixed TaskSpace utility and cost behavior. |

This set can support E3 only when every included pair reports `E3`. Earlier runs with excluded or candidate pairs must be described as diagnostic or E3-candidate evidence.

### 4.4 E3 Candidate: Terminal-Bench P0 Comparable Scope

Sample set id: `terminal-bench-p0-comparable`

| Sample | Definition | Status |
|---|---|---|
| `processing-pipeline` | Terminal-Bench processing pipeline repair task | Active comparable P0 sample. |
| `multi-source-data-merger` | Terminal-Bench multi-source merge and conflict report task | Active comparable P0 sample. |
| `recover-accuracy-log` | Terminal-Bench recovery/accuracy log task | Active comparable P0 sample. |
| `query-optimize` | Terminal-Bench query optimization task with remote asset requirements | Excluded until remote asset equivalence is proven; use fail-closed status, not pass/fail accuracy. |

This was the main v0.0.3/v0.0.4 comparable P0 scope. Results from this set cannot be compared directly with v0.0.5 internal fixture results.

### 4.5 E3 Candidate: v0.0.4 Clean 15-Run Comparable Scope

Sample set id: `terminal-bench-v004-clean-15`

| Sample | Definition | Repeat policy | Historical result |
|---|---|---:|---|
| `analyze-access-logs` | Terminal-Bench access-log analysis task | 5 | v0.0.4 clean run: Standard 4/5, TaskSpace 5/5. |
| `log-summary` | Terminal-Bench log summary task | 5 | v0.0.4 clean run: Standard 3/5, TaskSpace 3/5. |
| `count-call-stack` | Terminal-Bench call-stack counting/format task | 5 | v0.0.4 clean run: Standard 0/5, TaskSpace 0/5. |

This is the required same-scope baseline if v0.0.5 needs to prove correctness did not regress against v0.0.4. Until audit reviews are completed and every pair reports `E3`, call it E3-candidate or clean public-validator evidence, not final E3.

### 4.6 E3 Harness Proof: Audited Terminal-Bench Hello World

Sample set id: `terminal-bench-hello-world-audited-proof`

| Sample | Definition | Use |
|---|---|---|
| `hello-world` | Single Terminal-Bench sample with completed audit review and no E3 gate failures in prior proof run | Proves harness closure on a simple external sample; does not prove broad TaskSpace utility. |

### 4.7 Future E3: Historical Whale Failure Corpus

Sample set id: `historical-whale-failures`

Definition: sanitized real Whale use cases, session failures, runtime failures, or product regressions. A sample must include original prompt hash, sanitized fixture, validator or audit path, privacy review, and artifact audit review.

Current status: corpus rules exist, but no active sample names are registered here yet. It cannot support release conclusions until concrete sample ids are added to this document.

### 4.8 Future E3: DeepSWE Adapter Scope

Sample set id: `deepswe-adapter-spike`

Definition: DeepSWE long-horizon software engineering task subset through the Whale external benchmark adapter.

Current status: adapter exists as a spike path, but no active sample names are registered here yet. It cannot support release conclusions until concrete sample ids and validator fidelity proof are registered.

## 5. E4 And E5 Registration Policy

E4 and E5 are intentionally empty until the project adds the required tooling and freezes their sample boards.

| Level | Required sample registration before first run |
|---|---|
| E4 | A release calibration set that names every included E3 sample set, every required E2 regression sample, prior-version baseline, profile hash policy, cost gates, and release-decision owner. |
| E5 | A product benchmark board that names external benchmark families, sample selection policy, refresh cadence, statistical thresholds, competitor/baseline policy, and independent review process. |

Until those entries exist in this file, no document may claim E4 or E5 evidence.

## 6. Allowed Version-Comparison Claims

| Comparison | Allowed? | Required wording |
|---|---|---|
| v0.0.5 internal E1/E2 matrix vs v0.0.4 Terminal-Bench E3 candidate | No | "Not same-scope; cannot compare accuracy." |
| v0.0.5 rerun on `terminal-bench-v004-clean-15` vs v0.0.4 clean 15-run | Yes, if same profile and score validity are documented | "Same-scope Terminal-Bench clean 15-run comparison." |
| v0.0.5 E2 matrix vs previous E2 matrix | Yes, if same internal sample set and repeats | "Internal engineering regression comparison." |
| E3-candidate vs E3 | No | "Candidate evidence pending E3 gates." |
| Any E1/E2 result as release readiness | No | "Engineering readiness only." |

## 7. Mandatory Run Record Fields

Every experiment report must include:

```text
experiment_level:
reported_evidence_level:
requested_evidence_target:
sample_set_id:
sample_names:
scenario_levels:
benchmark_family:
runner_entrypoint:
runner_profile_hash:
source_version:
repeats_per_sample:
mode_pairing:
run_root:
score_valid:
engineering_clean:
human_audit_status:
token_summary_status:
allowed_claim:
explicit_non_claims:
```

If any field is missing, the result is `diagnostic-only` until corrected.

## 8. Naming Rules

Use these terms precisely:

| Term | Meaning |
|---|---|
| `internal matrix` | A run over `benchmarks/taskspace/scenarios/*` fixtures. Usually E1/E2. |
| `Terminal-Bench E3` | A run through the external benchmark adapter with Terminal-Bench source metadata and E3 gates. |
| `clean public-validator evidence` | A run with valid engineering harness and public validator results, but without final E3 audit closure. |
| `full E3` | Only a run where included pairs report `E3`, audit is completed, and score validity is true. |
| `same-scope comparison` | Same sample set id, same sample names, same repeat policy, and materially same runner profile. |

## 9. v0.0.5 Correction

The v0.0.5 `24/25` result must be described as:

```text
v0.0.5 achieved 24/25 raw business success on a mixed internal E1/E2 engineering matrix:
taskspace-internal-regression-v005 plus taskspace-internal-smoke-v005.
```

It must not be described as:

```text
v0.0.5 achieved near-100% Terminal-Bench E3 accuracy.
```

The correct next same-scope correctness check for v0.0.5 is to rerun `terminal-bench-v004-clean-15` under the v0.0.5 profile and compare it to the v0.0.4 clean 15-run.
