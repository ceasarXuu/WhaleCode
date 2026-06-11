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
