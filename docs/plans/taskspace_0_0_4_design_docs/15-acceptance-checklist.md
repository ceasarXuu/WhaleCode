# 15. 0.0.4 Acceptance Checklist

## 1. Pre-merge checklist

```text
[ ] schema version fields added
[ ] 0.0.3 trace remains readable as legacy
[ ] ProblemStateLedger created on start_task
[ ] start_task requires objective or forces immediate ledger completion
[ ] success criteria exists before ordinary work
[ ] record_decision supports dependency refs
[ ] invalid result cannot be referenced by decision
[ ] final synthesis blocks on open blocking questions
[ ] validate node requires validator/test evidence
[ ] graph-health.json emitted
[ ] audit.yaml emitted
[ ] failure taxonomy emitted
```

## 2. E3 focused rerun checklist

```text
[ ] recover-accuracy-log 5 pairs completed
[ ] processing-pipeline 5 pairs completed
[ ] multi-source-data-merger 2 diagnostic pairs completed or explicitly skipped
[ ] query-optimize preflight fail-closed preserved
[ ] cleanup artifacts ok
[ ] remote asset status recorded
[ ] aggregate includes valid_utility_pairs or mechanical exclusion reasons
```

## 3. Behavior checklist

```text
[ ] low complexity task gets thin mode recommendation
[ ] subagent spawn has plan
[ ] subagent result has validity/adoption status
[ ] every failed pair has failure class
[ ] graph health warnings explain known 0.0.3 failure patterns
[ ] result adoption summary visible in viewer
```

## 4. Release note checklist

```text
[ ] release note states 0.0.4 is observability/contract/audit version
[ ] release note does not claim utility win unless clean aggregate supports it
[ ] known limitations documented
[ ] 0.0.5 candidates documented
```

## 5. Hard no-go conditions

```text
[ ] Docker cleanup regression
[ ] remote asset fail-open regression
[ ] valid_utility_pairs=0 with no mechanical explanation
[ ] final synthesis possible with invalid result dependency
[ ] TaskSpace run can finish without success criteria
```

## 6. E3 runtime calibration checklist

```text
[ ] one-pair timing smoke emits pair-timing.json
[ ] one-pair timing smoke emits sample-timing.json
[ ] one-pair timing smoke emits runtime-bottleneck.md
[ ] 3-sample serial calibration emits suite-timing.json
[ ] 3-sample serial calibration emits runtime-calibration-report.md
[ ] sample-parallel smoke emits parallelism.json
[ ] serial-vs-parallel-equivalence.json comparable=true
[ ] serial-vs-parallel-equivalence.json parallel_smoke_score_drift=false
[ ] calibration-gate.json status=pass before full E3
[ ] calibration-gate.json full_e3_allowed=true before full E3
[ ] calibration-gate.json speed_claim_allowed=true before speedup claim
```

## 7. E3 speed no-go conditions

```text
[ ] missing pair-timing.json
[ ] missing sample-timing.json
[ ] missing suite-timing.json
[ ] missing runtime-bottleneck.md
[ ] missing runtime-calibration-report.md
[ ] missing calibration-gate.json
[ ] calibration-gate.json status=fail
[ ] parallel_smoke_score_drift=true
[ ] engineering_unclean_slow present in timing summary
[ ] runtime bottleneck cannot identify top span
```
