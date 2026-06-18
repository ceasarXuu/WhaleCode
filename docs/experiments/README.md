# WhaleCode Experiment System

- Status: Ready for use
- Created: 2026-06-18
- Owner: WhaleCode engineering
- Scope: TaskSpace, E3, benchmark, smoke, release-gate, and version comparison evidence

This folder is the canonical entry point for experiment definitions. Historical files under `docs/testing/` remain evidence records, but they are not the source of truth for level definitions or sample-set names.

## Canonical Files

| File | Purpose |
|---|---|
| [taskspace-evidence-levels-and-samples.md](./taskspace-evidence-levels-and-samples.md) | Defines E1-E5, current sample sets, allowed claims, and reporting rules. |

## Non-Negotiable Rules

1. Do not call a run "E3" unless its reported evidence level is `E3`, not `E3-candidate`, `E2`, `E2-candidate`, or `E1`.
2. Every result summary must name the sample set, sample names, repeat count, runner command family, run root, score validity, and audit status.
3. Internal fixture matrices can support engineering readiness claims only. They cannot support external benchmark accuracy claims.
4. Version comparisons are valid only when the evidence level and sample set are the same, or the report explicitly says the comparison is not same-scope.
5. Candidate evidence is not release evidence. `E3-candidate` means the run is waiting on required E3 gates, usually human audit or proof closure.

## Required Result Header

Every future experiment result document should start with this block:

```text
experiment_level: E1 | E2 | E3 | E3-candidate | E4 | E5
sample_set_id: <registered id from docs/experiments>
sample_names: <comma-separated names>
repeats_per_sample: <number>
runner_family: internal-matrix | terminal-bench | deepswe | historical-whale | release-calibration | product-benchmark
runner_entrypoint: <script or command>
run_root: <absolute path>
score_valid: true | false | not_applicable
human_audit_status: not_required | pending | completed | failed
allowed_claim: <one sentence>
```
