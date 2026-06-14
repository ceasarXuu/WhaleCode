# TaskSpace E3 Harness Guardrails Implementation Plan

- Created: 2026-06-13
- Updated: 2026-06-14
- Version: 0.3
- Status: Repair-ready plan for hard clean-execution scoring and runtime reduction contract
- Owner / Responsible: WhaleCode core
- Related Systems: TaskSpace E3 benchmark harness, Terminal-Bench adapter, aggregate report, audit manifest, failure taxonomy, E3 proof, score validity gate
- Risk Level: High
- Plan Type: Engineering implementation plan

## 1. Problem Definition

TaskSpace E3 一次完整执行会占用数小时。如果 benchmark harness 或 validator materialization 自身出现工程错误，继续跑完整套 agent / pair / repeat 只会消耗时间，并且最后还可能输出误导性的 diagnostic comparison。

近期暴露的问题是 Terminal-Bench validator 生成时写入了相对 uv cache 路径。执行时 validator 在 pair workspace 内用 `Resolve-Path` 解析该路径，导致 standard 和 taskspace 两边 public validation 都在进入真实测试前失败。这个结果不是模型能力退化，也不是 TaskSpace 0.0.4 行为本身变差，而是 harness materialization failure。当前 harness 已有 E3 proof、metrics extraction、failure taxonomy、pair report 和 aggregate report，但缺少足够早的工程健康检查、sentinel abort 和 run-level circuit breaker，因此异常只能在长时间执行后由报告人工识别。

本方案目标是把这类明显工程异常前移到 preflight / probe / sentinel 阶段发现，并让报告层明确标记 invalid harness run，禁止把 invalid run 写成模型或 TaskSpace 的胜负结论。

## 2. Goals

1. 在进入 agent 执行前发现可静态验证的 harness materialization 问题，例如相对路径、缺失 validator 源文件、缺失 uv cache、proof 目录不可写、remote asset equivalence 未证实。
2. 在真实 public validation 之前增加 validator probe，验证 wrapper 可以解析路径、写入 runtime manifest、读取 uv cache、定位 mount/proof 目录，并能区分 probe failure 与测试失败。
3. 对 E3 repeat 引入 sentinel pair：如果第一组 pair 已经暴露标准侧或双侧同源 infra failure，立即中止该 sample 的剩余 repeat。
4. 对 suite 级多 sample 执行引入 circuit breaker：如果不同 sample 复现同一 harness infra signature，及时中止后续样本。
5. 在 pair report、aggregate report、failure taxonomy 中加入 run validity 语义，invalid harness run 不允许输出 `TaskSpace better/worse/regressed` 一类比较结论。
6. 所有 guardrail 都必须有最小代价测试：静态函数单测、synthetic metrics fixture、generated validator probe fixture、report rendering fixture。完整 E3 只能作为最后验收，不作为发现 guardrail 逻辑错误的主要手段。

## 3. Non-Goals

- 不改 DeepSeek / Whale agent 推理路径。
- 不调整 TaskSpace 0.0.4 的 scoring 逻辑或产品行为。
- 不用 full E3 作为开发迭代测试手段。
- 不把所有 validation failure 都归为 harness failure。进入真实测试后的 assertion failure 仍然是 benchmark outcome。
- 不用静默 fallback 掩盖工程错误。guardrail 发现硬错误时必须生成可定位 artifact 并中止对应 scope。

## 4. Current Code Anchors

| Area | Current File | Current Responsibility | Guardrail Integration Point |
|---|---|---|---|
| Runner | `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` | sample/pair orchestration, workspace creation, agent execution, validation, aggregate | preflight after manifest materialization; probe before `Invoke-RealProcess`; sentinel abort after first pair metrics; invalid run status before aggregate |
| External wrapper | `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1` | materialize external benchmark scenario, then invoke runner | wrapper-level materialization health before runner exists |
| E3 single-task wrapper | `scripts/taskspace-benchmark/run-taskspace-e3-external.ps1` | enforce E3 repeat count for one external task | delegate to suite driver for multi-sample runs |
| E3 finalize | `scripts/taskspace-benchmark/finalize-taskspace-e3-run.ps1` | rebuild reports from existing pair dirs | refuse invalid-harness resume/finalize unless forced |
| Terminal-Bench adapter | `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1` | generate external validator script and proof markers | add probe mode and explicit lifecycle markers |
| uv cache materialization | `scripts/taskspace-benchmark/adapters/terminal-bench-uv-cache.ps1` | materialize cache path and wrapper metadata | expose absolute path contract and regression fixture |
| Metrics extractor | `scripts/taskspace-benchmark/lib/metrics-extractor.ps1` | parse stdout/stderr, docker build result, cleanup result, environment failures | classify pre-test/probe failures and emit infra signature inputs |
| Failure taxonomy | `scripts/taskspace-benchmark/lib/failure-taxonomy.ps1` | map failure signals into diagnostic classes | add harness materialization/probe/circuit breaker classes |
| Pair report | `scripts/taskspace-benchmark/lib/pair-report.ps1` | render per-pair evidence gate and metrics | add harness health and abort evidence section |
| Aggregate report | `scripts/taskspace-benchmark/lib/aggregate-report.ps1` | summarize eligible pairs, valid E3 pairs, failures | add `run_validity`, comparison disablement, invalid reason |
| E3 proof | `scripts/taskspace-benchmark/lib/e3-proof.ps1` | validate proof markers, runtime manifest, Docker inspect, cleanup | reuse proof marker names in probe and post-run verification |
| Existing self-tests | `scripts/taskspace-benchmark/test-terminal-bench-uv-cache-harness.ps1`, `scripts/taskspace-benchmark/test-harness.ps1` | low-cost harness regression checks | extend with guardrail-specific fixtures |

## 5. Target Runtime Contract

### 5.1 New Status Concepts

Add a shared guardrail vocabulary used by runner, metrics, taxonomy and report layers:

| Concept | Values | Meaning |
|---|---|---|
| `run_validity` | `valid`, `invalid_harness`, `ineligible`, `partial_user_aborted` | Whether benchmark output can support comparison |
| `abort_scope` | `none`, `sample`, `suite` | How much work was intentionally stopped |
| `abort_phase` | `preflight`, `probe`, `sentinel_pair`, `suite_circuit_breaker`, `report_gate` | Where the abort decision was made |
| `infra_signature` | stable string plus fields | Deduplicated engineering failure fingerprint |
| `diagnostic_comparison_enabled` | boolean | Whether aggregate may emit better/worse/regressed language |
| `invalid_harness_exit_code` | `3` | Stable process exit code for infrastructure-invalid runs |

`invalid_harness` means the benchmark infrastructure is not trustworthy enough to compare TaskSpace versus baseline. It is not a failed public validation score.

### 5.2 New Artifacts

All artifacts are JSON unless noted otherwise.

| Artifact | Location | Producer | Required Fields |
|---|---|---|---|
| `harness-health.json` | run root | preflight module | `schema_version`, `status`, `findings`, `checked_paths`, `generated_at` |
| `validator-probe-result.json` | per-side validation proof dir | generated validator | `status`, `stage`, `runtime_manifest_path`, `uv_cache_path`, `docker_backend`, `failure_signature` |
| `pair-abort.json` | pair output dir | runner | `abort_scope`, `abort_phase`, `reason`, `infra_signature`, `first_failure_artifact` |
| `suite-health.json` | suite run root | suite driver / runner wrapper | `status`, `signature_counts`, `aborted_samples`, `remaining_samples_skipped` |
| `abort-summary.md` | run root | report layer | human-readable abort reason and reproduction pointers |
| `run-status.json` | run root | runner / wrapper | `phase`, `run_validity`, `exit_code`, `resume_allowed`, `force_rerun_required` |
| `sample-status.json` | sample run root | runner / suite driver | `sample_id`, `phase`, `run_validity`, `abort_scope`, `abort_signature` |

### 5.3 Infra Signature Shape

An infra signature must be deterministic and low-cardinality enough to group repeated harness failures, but detailed enough to locate the source.

```json
{
  "schema_version": 1,
  "category": "harness_materialization_failure",
  "stage": "validator_pretest",
  "stable_code": "path_unresolvable",
  "normalized_message": "Resolve-Path failed for uv-cache path",
  "side": "standard",
  "artifact": "validation-stderr.txt"
}
```

Recommended `stable_code` values:

- `relative_materialized_path`
- `path_unresolvable`
- `validator_source_missing`
- `uv_cache_missing`
- `remote_asset_equivalence_unproven`
- `docker_backend_unavailable`
- `runtime_manifest_missing`
- `validator_probe_failed`
- `no_tests_started_marker`
- `same_infra_signature_both_sides`
- `suite_repeated_infra_signature`

### 5.4 Validator Lifecycle Markers

The sentinel policy is only valid if metrics can prove whether benchmark tests started. The current Terminal-Bench entry script only executes `bash /tests/run-tests.sh`; implementation must inject explicit lifecycle markers before any abort policy consumes them.

Required generated entry-script behavior in `terminal-bench-adapter.ps1`:

```bash
echo validator_lifecycle_stage=entry_started
test "$(pwd)" = "/app"
test -d "$TEST_DIR"
test -f /tests/run-tests.sh
if touch /tests/.whale-write-test 2>/tmp/whale-validator-ro.err; then
  echo validator_mount_readonly=false
  rm -f /tests/.whale-write-test
  exit 81
else
  echo validator_mount_readonly=true
fi
echo validator_lifecycle_stage=tests_started
echo validator_tests_started=true
set +e
bash /tests/run-tests.sh
test_exit=$?
echo validator_lifecycle_stage=tests_completed
echo validator_tests_completed=true
exit $test_exit
```

Required metrics fields in `Get-TaskspaceBenchmarkMetrics`:

| Field | Source | Meaning |
|---|---|---|
| `tests_started_seen` | stdout marker `validator_tests_started=true` | true only when benchmark test command was reached |
| `tests_completed_seen` | stdout marker `validator_tests_completed=true` | true when test command returned and wrapper preserved exit code |
| `validation_lifecycle_stage` | last `validator_lifecycle_stage=*` marker | `entry_started`, `tests_started`, `tests_completed`, or `unknown` |
| `public_validation_reached_tests` | derived | same as `tests_started_seen`; used by report and sentinel |
| `pretest_failure` | derived | validation failed and `tests_started_seen=false` |
| `infra_signature` | derived | stable signature from probe result, lifecycle stage, stderr fallback, Docker result |

Sentinel abort logic must use these fields. It must not infer pre-test failure from exit code alone.

## 6. Abort Policy

### 6.1 Hard Abort Before Agent Execution

Abort the sample before `Invoke-RealProcess` when any of these are true:

| Condition | Detection | Output |
|---|---|---|
| Manifest embeds a path that is relative but later resolved from pair workspace | static preflight over manifest fields and generated command arguments | `run_validity=invalid_harness`, `abort_phase=preflight`, `stable_code=relative_materialized_path` |
| Required validator source or wrapper source is missing | `Test-Path -LiteralPath` from run root and pair workspace | `stable_code=validator_source_missing` |
| uv cache root is missing or cannot be resolved to an absolute path | `Resolve-Path -LiteralPath` plus `IsPathFullyQualified` | `stable_code=uv_cache_missing` or `path_unresolvable` |
| proof dir cannot be created or written | write/delete temp marker in proof root | `stable_code=proof_dir_unwritable` |
| remote asset equivalence preflight is unproven | existing remote asset guard returns non-pass | `run_validity=ineligible`, not `invalid_harness` |

### 6.2 Probe Abort Before Real Validation

Abort the sample after validator probe if generated validator cannot complete pre-test lifecycle:

1. resolve all paths;
2. write `terminal-bench-runtime-manifest.json`;
3. write `validator_probe_started` and `validator_probe_completed` markers;
4. read uv cache metadata;
5. verify Docker backend availability when Docker is required;
6. verify proof dir and mount target variables.

Probe mode must not run benchmark tests. If it needs Docker, the first implementation should support a cheap backend probe only (`docker version` or existing backend check), not a full image build.

### 6.3 Sentinel Pair Abort

For E3 samples with repeat count greater than one, repeat 1 is a sentinel. After both standard and taskspace sides produce metrics:

| Condition | Decision |
|---|---|
| both sides fail before `tests_started` with the same infra signature | abort remaining repeats for this sample |
| standard side fails before `tests_started` with a hard infra signature | abort remaining repeats for this sample |
| taskspace side alone fails before `tests_started` because of harness-generated path/probe/proof failure | abort sample as invalid harness |
| either side reaches `tests_started`, then test assertions fail | do not abort as infra; continue repeat policy |
| public validation timeout happens before `tests_started` | classify as `validator_slow_or_flaky`; abort only if repeated in sentinel on both sides |

### 6.4 Suite Circuit Breaker

The current runner is sample-centric. If P0/E3 orchestration is done by a separate wrapper, the suite-level guard must live in that wrapper or in a new shared driver invoked by the wrapper.

Abort the suite when:

- any global dependency failure appears once: `docker_backend_unavailable`, `uv_cache_missing`, `validator_source_missing`;
- the same hard infra signature appears in two different samples;
- the suite cannot persist `suite-health.json`.

Suite abort must skip remaining samples explicitly and write skipped sample records, rather than leaving the run looking incomplete.

## 7. Implementation Phases

### Phase 0: Contract Lock And Fixture Inventory

Entry criteria:

- Current uv-cache materialization fix is present.
- `scripts/taskspace-benchmark/test-terminal-bench-uv-cache-harness.ps1` and `scripts/taskspace-benchmark/test-harness.ps1` pass on the current worktree.

Implementation tasks:

1. Inventory existing proof markers in `lib/e3-proof.ps1` and generated validator script.
2. Document exact lifecycle markers: `validator_probe_started`, `validator_probe_completed`, `tests_started`, `tests_completed`, `validator_failed_pretest`.
3. Build synthetic fixture inputs for:
   - relative uv cache path failure;
   - missing uv cache;
   - Docker backend unavailable;
   - failure after `tests_started`.
4. Define JSON schema examples for `harness-health.json`, `validator-probe-result.json`, `pair-abort.json`, and `suite-health.json`.

Deliverables:

- This plan updated with final field names.
- Fixture files or inline test fixtures under `scripts/taskspace-benchmark/test-fixtures/guardrails/`.

Validation:

- `git diff --check`
- Existing harness self-tests still pass.

Exit criteria:

- A developer can identify from fixtures which failures must abort and which must not.
- No production runner behavior has changed yet.

### Phase 1: Static Harness Health Preflight

Implementation tasks:

1. Add `scripts/taskspace-benchmark/lib/harness-health.ps1`.
2. Export these functions:
   - `Test-TaskspaceFullyQualifiedPath`
   - `Test-TaskspaceResolvablePathFrom`
   - `Get-TaskspaceHarnessHealth`
   - `Write-TaskspaceHarnessHealth`
   - `New-TaskspaceInfraSignature`
3. In `run-taskspace-external-benchmark.ps1`, normalize `RunRoot` and `scenarioRoot` to absolute paths before invoking the adapter.
4. Wrap adapter invocation in try/catch. If adapter/materialization fails before the inner runner exists, write `external-materialization-health.json`, `run-status.json`, and `abort-summary.md` under the wrapper run root, then exit `3`.
5. In `run-taskspace-benchmark.ps1`, import the new module next to existing lib imports.
6. After manifest/materialization and remote asset preflight, before the pair loop, call `Get-TaskspaceHarnessHealth`.
7. If health status is `fail` with a hard abort finding, write `harness-health.json`, write `run-status.json`, write `abort-summary.md`, set sample phase to `invalid_harness`, and exit `3` before `New-TaskspacePairWorkspace` and `Invoke-RealProcess`.
8. If health status is `ineligible`, preserve the existing ineligible path and include the health artifact.

Acceptance tests:

- Given a relative output root, the current fixed uv cache path is stored and reported as absolute.
- Given a synthetic manifest with a relative path that would be resolved from pair workspace, preflight returns `fail`.
- Given a missing uv cache directory, runner exits before agent execution and writes `harness-health.json`.
- Given adapter materialization throws before `scenario_dir` exists, external wrapper writes `external-materialization-health.json` and exits `3`.
- Given remote asset equivalence unproven, runner marks `ineligible` instead of `invalid_harness`.

Exit criteria:

- No known materialization/path failure can reach agent execution without a health artifact.
- Existing successful PlanOnly or low-cost harness runs remain valid.

### Phase 2: Validator Probe Mode

Implementation tasks:

1. Extend generated `external-validator.ps1` in `terminal-bench-adapter.ps1` with `-ProbeOnly` and optional `-ProbeDocker`.
2. Create `$probeResultPath` before `Get-DockerBackend`, path conversion, or Docker calls. Wrap the whole validator body in a top-level try/catch that always writes `validator-probe-result.json` for probe/pre-test failures.
3. In probe mode, run only validator setup:
   - resolve host paths and Docker paths;
   - create proof dir;
   - write runtime manifest;
   - read uv cache metadata and sha;
   - emit `validator_probe_started` and `validator_probe_completed`;
   - optionally run Docker backend probe.
4. Ensure probe mode exits before benchmark tests and before expensive image build unless `-ProbeDocker` explicitly requests backend verification.
5. Add `validator-probe-result.json` with status and failure signature.
6. Update metrics extractor to parse probe result separately from public validation result.
7. Add stderr fallback parsing for failures that occur before JSON exists:
   - `docker command is required` -> `docker_backend_unavailable`;
   - `Resolve-Path` / `Cannot find path` -> `path_unresolvable`;
   - `run-tests script not found` -> `validator_source_missing`;
   - uv cache path or archive missing -> `uv_cache_missing`.
8. Update runner to invoke probe once per side before agent execution when EvidenceTarget is E3 or when validator manifest declares probe support.

Acceptance tests:

- Probe success writes runtime manifest and probe result without writing `tests_started`.
- Missing uv cache produces `validator_probe_failed` and `stable_code=uv_cache_missing`.
- Docker unavailable during `-ProbeDocker` produces `docker_backend_unavailable`.
- Docker unavailable before Docker result JSON exists is still classified by stderr fallback.
- Probe failure aborts before `Invoke-RealProcess`.

Exit criteria:

- A validator that cannot even initialize is caught without spending agent tokens or running public validation.
- Metrics can distinguish `public validation failed after tests started` from `validator probe failed`.

### Phase 3: Sentinel Pair Abort

Implementation tasks:

1. Add `Get-TaskspaceInfraSignatureFromMetrics` to `harness-health.ps1` or a small `infra-signature.ps1` module.
2. Add runner state:
   - `sentinel_pair_index`;
   - `sentinel_status`;
   - `sample_abort_reason`;
   - `sample_abort_signature`.
3. After repeat 1 validation metrics are collected for both sides, evaluate sentinel abort policy.
4. If aborting, write `pair-abort.json`, set remaining repeats to skipped, and do not call agent execution for remaining repeats.
5. Emit events:
   - `sentinel_pair_started`
   - `sentinel_pair_completed`
   - `sample_aborted_by_guardrail`
   - `pair_skipped_after_guardrail_abort`
6. Ensure resume behavior:
   - write `sample-status.json` whenever a sample enters `invalid_harness`;
   - `-ResumeLatest` and explicit `-RunId` must reject `invalid_harness` runs unless `-ForceRerun` is present;
   - `finalize-taskspace-e3-run.ps1` must refuse to rebuild reports from an invalid-harness run unless called with a new explicit `-AllowInvalidHarnessFinalize` switch for forensic reporting.

Acceptance tests:

- Synthetic metrics where both sides fail with `Resolve-Path` before `tests_started` aborts after repeat 1 and skips repeats 2-N.
- Synthetic metrics where standard fails before `tests_started` with `uv_cache_missing` aborts after repeat 1.
- Synthetic metrics where taskspace fails after `tests_started` with assertion failure does not trigger infra abort.
- Resume from invalid harness run refuses continuation without `-ForceRerun`.
- Finalize from invalid harness run refuses normal aggregate rebuild and points to `sample-status.json`.

Exit criteria:

- Repeated E3 work stops after the first pair for deterministic harness failures.
- Abort artifacts are sufficient to reproduce the guardrail decision without re-running E3.

### Phase 4: Canonical Suite Driver And Circuit Breaker

Implementation tasks:

1. Add `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` as the canonical multi-sample entrypoint.
2. Parameters:
   - `-Benchmark deepswe|terminal-bench`;
   - `-TaskListPath` pointing to JSON or JSONL records with `task_dir`, `sample_id`, and optional `source_version`;
   - `-SourceVersion` default applied when a record omits `source_version`;
   - `-RunRoot`, `-Repeats`, `-WhaleBin`, `-Model`, `-TimeoutSeconds`, `-ValidationTimeoutSeconds`, `-SandboxMode`, `-ConfigOverride`, `-AuditReviewRoot`, `-PlanOnly`;
   - `-ContinueAfterInvalidHarness` only for explicit forensic runs.
3. Child invocation contract:
   - create `$suiteRoot = Join-Path $RunRoot "suite-$timestamp"`;
   - for each record, invoke `run-taskspace-external-benchmark.ps1` with `-RunRoot (Join-Path $suiteRoot "samples\$sampleId")`;
   - require each child to write `sample-status.json`;
   - ingest child `harness-health.json`, `external-materialization-health.json`, `pair-abort.json`, and aggregate validity fields.
4. Add suite state persistence:
   - `suite-health.json`;
   - `signature_counts`;
   - `sample_statuses`;
   - `suite_abort_reason`.
5. Skipped sample record format in `skipped-samples.jsonl`:

```json
{"sample_id":"bench-006","task_dir":"...","run_validity":"invalid_harness","abort_scope":"suite","abort_phase":"suite_circuit_breaker","abort_signature":"harness_materialization_failure/path_unresolvable","skipped_reason":"suite_repeated_infra_signature"}
```

6. Abort remaining samples if a global dependency failure occurs once or the same hard infra signature appears in two samples.
7. Exit behavior:
   - exit `0` when all samples complete and no failed benchmark gate exists;
   - exit `1` when valid samples completed but benchmark gates failed;
   - exit `2` for ineligible input;
   - exit `3` for invalid harness or suite circuit breaker;
   - exit `4` for malformed `TaskListPath` or missing required suite inputs.

Acceptance tests:

- Two synthetic sample summaries with the same `relative_materialized_path` signature trigger suite abort.
- A single `docker_backend_unavailable` sample triggers suite abort.
- A sample with normal assertion failure after `tests_started` does not increment hard infra signature count.
- Suite report lists skipped samples with `abort_scope=suite`.
- `run-taskspace-e3-external.ps1` remains valid for one task, but multi-sample E3 documentation points to `run-taskspace-e3-suite.ps1`.

Exit criteria:

- Full E3 runs cannot continue for hours after a repeated harness-level failure signature has already been observed.
- Operators can see exactly which sample caused the circuit breaker and which samples were skipped.

### Phase 5: Report And Aggregate Semantics

Implementation tasks:

1. Extend aggregate output with:
   - `run_validity`;
   - `diagnostic_comparison_enabled`;
   - `invalid_run_reason`;
   - `abort_scope`;
   - `abort_phase`;
   - `abort_signature`;
   - `first_failure_artifact`.
2. Extend pair report with a `Harness Health / Abort` section.
3. Extend failure taxonomy classes:
   - `harness_materialization_failure`;
   - `validator_probe_failure`;
   - `validator_pretest_failure`;
   - `suite_circuit_breaker`;
   - `invalid_harness_run`.
4. In aggregate markdown, block better/worse/regressed wording when `diagnostic_comparison_enabled=false`.
5. Preserve raw validation exit codes, but label them as infrastructure-invalid when no test lifecycle marker proves real test execution started.

Acceptance tests:

- Invalid harness aggregate says comparison is disabled and points to abort artifacts.
- Invalid harness aggregate does not say TaskSpace regressed, improved, better, worse, or tied.
- Valid assertion failures after `tests_started` still render normal utility/diagnostic comparison.
- Failure taxonomy summary includes new infra classes.

Exit criteria:

- Humans cannot misread an infrastructure-invalid run as a model or TaskSpace quality result from the top-level report.
- Raw artifacts still preserve enough detail for debugging.

### Phase 6: Minimal Validation And Release Gate

Implementation tasks:

1. Run low-cost self-tests first:
   - `.\scripts\taskspace-benchmark\test-terminal-bench-uv-cache-harness.ps1`
   - `.\scripts\taskspace-benchmark\test-harness.ps1`
   - `.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1`
2. Run generated validator probe fixture.
3. Run one minimal real E3 smoke only after self-tests pass:
   - one sample;
   - one sentinel pair;
   - no full repeat suite;
   - capture `harness-health.json`, `validator-probe-result.json`, aggregate report.
4. Only after all above pass, schedule full E3 re-run.

Acceptance tests:

- The historical uv-cache relative path failure is classified as `invalid_harness` before agent execution or at sentinel latest.
- `test-terminal-bench-uv-cache-harness.ps1` keeps its current relative `OutputRoot` fixture and adds probe/error cases rather than replacing the fixture with a slower smoke.
- The fixed current path materialization passes static preflight.
- Generated validator probe produces runtime manifest and proof markers.
- One-pair E3 smoke either reaches `tests_started` or aborts with a classified guardrail artifact.
- Full E3 cannot start unless low-cost guardrail tests pass.

Exit criteria:

- The project has a repeatable release gate that catches harness engineering failures cheaply.
- Full E3 is reserved for measuring behavior, not discovering basic infrastructure defects.

## 8. Developer Checklist

Implementation should proceed in this order:

1. Preserve and extend `test-terminal-bench-uv-cache-harness.ps1` as the named historical regression anchor.
2. Add fixtures and schema examples.
3. Add `lib/harness-health.ps1` with pure functions and unit-style tests.
4. Add wrapper-level materialization health to `run-taskspace-external-benchmark.ps1`.
5. Wire preflight into runner without changing validation/reporting.
6. Inject validator lifecycle markers and add metrics lifecycle fields.
7. Add validator probe mode, top-level probe result try/catch, and stderr fallback parser.
8. Add sentinel abort state and pair abort artifacts.
9. Add `run-taskspace-e3-suite.ps1` and suite circuit breaker.
10. Update failure taxonomy.
11. Update pair and aggregate reports.
12. Update resume/force-rerun/finalize behavior.
13. Run low-cost tests.
14. Run one-pair smoke.
15. Run adversarial review before full E3.

## 9. Expected File Changes

Likely new files:

- `scripts/taskspace-benchmark/lib/harness-health.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`
- `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1`
- `scripts/taskspace-benchmark/test-fixtures/guardrails/*.json`

Likely changed files:

- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1`
- `scripts/taskspace-benchmark/run-taskspace-e3-external.ps1`
- `scripts/taskspace-benchmark/finalize-taskspace-e3-run.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- `scripts/taskspace-benchmark/lib/failure-taxonomy.ps1`
- `scripts/taskspace-benchmark/lib/pair-report.ps1`
- `scripts/taskspace-benchmark/lib/aggregate-report.ps1`
- `scripts/taskspace-benchmark/lib/e3-proof.ps1`
- `scripts/taskspace-benchmark/test-terminal-bench-uv-cache-harness.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`

If any single Whale-owned code file approaches 500 lines after the change, split by responsibility rather than extending the file indefinitely. Vendored upstream files are not part of this change.

## 10. Reporting Examples

Invalid harness aggregate header:

```text
Run validity: invalid_harness
Diagnostic comparison: disabled
Reason: validator pre-test path resolution failed before benchmark tests started
Abort scope: sample
First failure artifact: artifacts/pair-001/standard/validation-stderr.txt
Infra signature: harness_materialization_failure/path_unresolvable
```

Valid benchmark failure header:

```text
Run validity: valid
Diagnostic comparison: enabled
Reason: validation reached tests_started marker; assertion failures are benchmark outcomes
```

## 11. Risks And Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Guardrail false-positive aborts real benchmark failures | Could hide legitimate TaskSpace regressions | Require lifecycle marker distinction: pre-test/probe failures can abort; failures after `tests_started` are benchmark outcomes |
| Probe becomes expensive | Reintroduces wasted time | Default probe avoids image build and full tests; Docker probe is explicit |
| Suite driver is fragmented | Circuit breaker not consistently applied | Identify actual P0/E3 entrypoint in Phase 4; centralize suite state writes |
| Resume semantics accidentally reuse invalid harness artifacts | Future runs inherit bad state | Refuse `-ResumeLatest` on `invalid_harness` unless `-ForceRerun` |
| Report layer still emits comparison language | Users misread invalid run as TaskSpace regression | Make `diagnostic_comparison_enabled=false` a hard rendering gate with fixture tests |

## 12. Open Questions To Resolve Before Implementation

1. Should Docker backend probe run for every sample, or only once per suite and then be cached in `suite-health.json`?
2. Should JSON schemas live under `scripts/taskspace-benchmark/schemas/` or only as test fixtures? Prefer schemas if these artifacts become consumed by external tooling.
3. Should `-AllowInvalidHarnessFinalize` be hidden/forensic-only, or documented next to normal finalize usage?

## 13. Final Acceptance Criteria

The guardrail implementation is complete when all criteria below are true:

1. A path-materialization bug equivalent to the recent uv-cache failure is caught before agent execution or no later than sentinel pair 1.
2. Missing validator source, missing uv cache, and Docker backend unavailability produce classified artifacts and do not run remaining E3 repeats.
3. A failure after `tests_started` remains a normal benchmark result and does not trigger harness abort.
4. Aggregate report for invalid harness run disables diagnostic comparison and contains no better/worse/regressed wording.
5. Process exit code `3` is used consistently for invalid harness at wrapper, runner, sentinel, suite, and finalize guard points.
6. `-ResumeLatest` and explicit `-RunId` cannot silently continue an invalid harness run.
7. Low-cost self-tests cover preflight, wrapper materialization health, lifecycle marker parsing, probe, sentinel abort, suite circuit breaker summary, resume/finalize guard, and report rendering.
8. One-pair E3 smoke demonstrates either valid test lifecycle progression or a correctly classified guardrail abort.
9. Full E3 is only run after the guardrail tests and smoke pass.

## 14. 2026-06-14 Hard Clean-Execution Repair Addendum

This addendum supersedes any earlier wording that treats Docker failures, validator failures, proof failures, incomplete audit, or report-generation gaps as normal comparable benchmark outcomes. For E3 scoring, only three agent outcomes are score-bearing:

- `solved`
- `wrong`
- `agent_exec_timeout`

Every other unexpected condition is `engineering_unclean`. If any pair in a scoring E3 run is `engineering_unclean`, the whole scoring run is invalid and the report must not emit TaskSpace better, Standard better, pass-rate delta, regression, or improvement conclusions.

### 14.1 Current Implementation State

The repository already has part of the guardrail substrate. The repair should extend it instead of replacing it.

| File | Current useful behavior | Repair gap |
|---|---|---|
| `scripts/taskspace-benchmark/lib/harness-health.ps1` | Defines invalid harness exit code `3`, disk checks, Docker storage checks, fully qualified path checks, validator probe parsing, text signatures, hard infra signature checks, sentinel abort decisions, and harness health artifacts. | It can detect many infra signatures, but the runner and report layers do not yet convert every non-agent condition into a hard score-invalid contract. |
| `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` | Provides a suite-level entrypoint and child run orchestration. | It must stop scheduling later samples as soon as a scoring child emits `score_valid=false` or an engineering-unclean abort artifact. |
| `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` | Orchestrates sample and pair execution, writes pair artifacts, aggregate reports, run status, and E3 audit-required states. | It currently allows a run to reach `completed` or `audit_required` even when score validity is unknown or false. It needs an explicit scoring mode gate. |
| `scripts/taskspace-benchmark/lib/aggregate-report.ps1` | Emits `run_validity`, invalid-run fields, and disables some diagnostic comparison fields when the run is invalid. | It does not yet expose the hard scoring contract fields `score_valid`, `score_invalid_reason`, `engineering_unclean_count`, `agent_exec_timeout_count`, and `clean_comparable_pair_count`. |
| `scripts/taskspace-benchmark/lib/audit-manifest.ps1` | Records side metrics, inclusion flags, exclusion reason, failure taxonomy, utility direction, and audit status. | It lacks per-pair hard outcome fields such as `outcome_standard`, `outcome_taskspace`, `engineering_unclean`, `engineering_unclean_reasons`, and `run_score_valid`. |
| `scripts/taskspace-benchmark/lib/failure-taxonomy.ps1` | Maps many observed failures into diagnostic classes and utility directions. | Legacy classes such as `validator_slow_or_flaky` and `taskspace_overhead_timeout` can still look like comparable diagnostic outcomes. They need to feed a separate hard `engineering_unclean` classifier. |
| `scripts/taskspace-benchmark/lib/suite-status.ps1` | Distinguishes completed child process state from failed child process state. | It must also distinguish process completion from score validity. A child can finish cleanly as a process and still invalidate E3 scoring. |

### 14.2 Root Cause From The Previous E3 Run

The previous E3 run was invalid because the execution pipeline measured and aggregated tasks after the benchmark infrastructure had already become unclean.

Observed invalidating signals:

- `e3_human_review_not_completed` on all 15 pairs.
- `manual_review_required` on 10 of 15 pairs.
- `audit_unclean` on all 15 pairs.
- `public_validation_timeout` on 8 of 15 pairs.
- `e3_external_validator_fidelity_unproven` on 8 of 15 pairs.
- `e3_external_validator_not_e3_eligible` on 8 of 15 pairs.
- `docker_run_failure` on 7 of 15 pairs.
- `docker_build_environment_failure` on 1 of 15 pairs.
- `docker_cleanup_container_inspect_failure` on 1 of 15 pairs.
- `e3_exec_timeout` on 2 of 15 pairs, but those pairs were mixed with Docker failures and therefore were not clean agent timeouts.

The engineering bug is not that these signals were invisible. The bug is that no single score-validity gate owned the rule: "score-bearing outcomes are solved, wrong, and clean agent execution timeout only." As a result, diagnostic artifacts were allowed to reach final comparison language.

### 14.3 Concrete Data Contract

Add the following fields to every aggregate JSON and markdown report produced by E3 scoring mode:

```json
{
  "score_valid": false,
  "score_invalid_reason": "engineering_unclean",
  "score_fields_enabled": false,
  "engineering_unclean_count": 15,
  "engineering_unclean_reasons": {
    "e3_human_review_not_completed": 15,
    "public_validation_timeout": 8,
    "docker_run_failure": 7
  },
  "agent_exec_timeout_count": 0,
  "clean_comparable_pair_count": 0,
  "score_bearing_outcomes": ["solved", "wrong", "agent_exec_timeout"]
}
```

Required report behavior:

- If `score_valid=false`, set `taskspace_better`, `standard_better`, `pass_rate_delta`, `diagnostic_pass_rate_delta`, and any "regressed" wording to `null`, `n/a`, or an explicit disabled value.
- If `score_valid=false`, the first markdown section must say the score is invalid before listing task counts.
- If the only non-success condition is clean `agent_exec_timeout`, keep `score_valid=true` and count it as an agent outcome.
- If `agent_exec_timeout` is mixed with Docker, validator, proof, audit, disk, path, or report failure on the same pair, classify that pair as `engineering_unclean`, not as clean agent timeout.
- The aggregate report must preserve diagnostic details, but diagnostic detail is not a score.

Add the following fields to each pair-level audit manifest row:

```json
{
  "outcome_standard": "solved",
  "outcome_taskspace": "engineering_unclean",
  "engineering_unclean": true,
  "engineering_unclean_reasons": [
    "public_validation_timeout",
    "docker_run_failure"
  ],
  "agent_exec_timeout_clean": false,
  "run_score_valid": false,
  "score_exclusion_reason": "engineering_unclean"
}
```

### 14.4 File-Level Repair Plan

Implement the fix in this order.

1. `scripts/taskspace-benchmark/lib/failure-taxonomy.ps1`

   Add a hard outcome classifier that is separate from the existing diagnostic taxonomy:

   ```powershell
   function Get-TaskspaceEngineeringUncleanReasons {
       param(
           [Parameter(Mandatory=$true)] [hashtable] $Metrics,
           [hashtable] $Proof = @{},
           [hashtable] $Audit = @{},
           [hashtable] $HarnessHealth = @{}
       )
       # Return stable reason codes only. Do not return prose.
   }

   function Get-TaskspaceAgentOutcome {
       param(
           [Parameter(Mandatory=$true)] [hashtable] $Metrics,
           [string[]] $EngineeringUncleanReasons = @()
       )
       # solved | wrong | agent_exec_timeout | engineering_unclean
   }
   ```

   Mapping rules:

   | Signal | Outcome |
   |---|---|
   | Public validation succeeded and hidden oracle succeeded, with no infra reason | `solved` |
   | Public validation reached tests and failed assertion, with no infra reason | `wrong` |
   | Agent execution timed out, validation/proof/Docker/audit are otherwise clean | `agent_exec_timeout` |
   | `public_validation_exit_code=124` | `engineering_unclean` reason `public_validation_timeout` |
   | Any Docker build/run/cleanup/inspect failure before trustworthy test result | `engineering_unclean` with the Docker reason code |
   | Validator proof missing, validator fidelity unproven, or E3 eligibility false | `engineering_unclean` |
   | Audit required but no completed audit decision in scoring mode | `engineering_unclean` reason `e3_human_review_not_completed` |
   | Missing report, unparsable metrics, path not absolute, disk below threshold | `engineering_unclean` |

   Keep the old taxonomy functions for diagnostics, but make `Get-TaskspaceUtilityDirection` consume the hard outcome first. If hard outcome is `engineering_unclean`, utility direction must be `invalid_run` or `score_disabled`, never `taskspace_better`, `standard_better`, or `inconclusive`.

2. `scripts/taskspace-benchmark/lib/audit-manifest.ps1`

   Extend the manifest row object after side metrics are loaded:

   - Compute `engineering_unclean_reasons_standard`.
   - Compute `engineering_unclean_reasons_taskspace`.
   - Compute `outcome_standard`.
   - Compute `outcome_taskspace`.
   - Set pair-level `engineering_unclean` if either side has hard unclean reasons, or if cross-side proof/audit state is unclean.
   - Set `run_score_valid=false` for that pair when `engineering_unclean=true`.

   The manifest writer must keep the old fields for backwards compatibility, but the new hard fields are authoritative for scoring.

3. `scripts/taskspace-benchmark/lib/aggregate-report.ps1`

   Add a score-validity reducer before any better/worse counts are calculated:

   ```powershell
   $scoreRows = $AuditRows | Where-Object { $_.sample_kind -eq 'e3' }
   $engineeringUncleanRows = $scoreRows | Where-Object { $_.engineering_unclean -eq $true }
   $cleanRows = $scoreRows | Where-Object { $_.engineering_unclean -ne $true }

   $scoreValid = ($engineeringUncleanRows.Count -eq 0)
   ```

   Required reducer outputs:

   - `score_valid`
   - `score_invalid_reason`
   - `score_fields_enabled`
   - `engineering_unclean_count`
   - `engineering_unclean_reasons`
   - `agent_exec_timeout_count`
   - `clean_comparable_pair_count`
   - `score_bearing_outcomes`

   If `$scoreValid -eq $false`, do not calculate public score deltas from all rows. Instead:

   - Preserve raw counts under a `diagnostics` object.
   - Set comparison fields to disabled values.
   - Render the markdown summary as "Score validity: invalid".

4. `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`

   Introduce a scoring mode switch without breaking diagnostic runs:

   ```powershell
   [switch] $ScoringMode,
   [switch] $RequireScoreValidity
   ```

   Execution rules:

   - `-ScoringMode` implies `-RequireScoreValidity`.
   - In scoring mode, an `audit_required` sample is not score-complete unless a completed audit decision exists.
   - After each pair, read the pair audit manifest or pair metrics and run the hard classifier.
   - If a pair is `engineering_unclean`, write `pair-abort.json`, set sample status to `invalid_harness`, set process exit code `3`, and stop scheduling later repeats for the sample.
   - If the pair is clean `agent_exec_timeout`, continue aggregation and count it as an agent outcome.
   - If `-ScoringMode` is absent, allow diagnostic completion but still set `score_valid=false` when hard unclean reasons exist.

5. `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1`

   The suite driver must treat child process completion and score validity as separate gates.

   Required loop behavior:

   ```powershell
   foreach ($sample in $Samples) {
       $child = Invoke-E3Sample -ScoringMode
       $status = Read-TaskspaceSampleStatus $child.RunRoot
       $aggregate = Read-TaskspaceAggregate $child.RunRoot

       if ($status.run_validity -eq 'invalid_harness' -or $aggregate.score_valid -eq $false) {
           Write-TaskspaceSuiteHealth -Status 'invalid_harness' -AbortScope 'suite'
           exit 3
       }
   }
   ```

   The driver must not run sample 2 or sample 3 after sample 1 proves the scoring environment is unclean.

6. `scripts/taskspace-benchmark/lib/suite-status.ps1`

   Add score-validity fields to suite summaries:

   - `completed_child_processes`
   - `score_valid_child_runs`
   - `score_invalid_child_runs`
   - `first_score_invalid_run`
   - `suite_score_valid`

   A child with phase `completed` but `score_valid=false` must not be counted as a successful scoring child.

7. `scripts/taskspace-benchmark/finalize-taskspace-e3-run.ps1`

   Finalize must refuse to generate scoring language when existing artifacts contain hard unclean reasons.

   Required behavior:

   - Default: fail with exit code `3` and write `run_validity=invalid_harness`.
   - `-DiagnosticOnly` or existing forensic override: allow report regeneration, but preserve `score_valid=false`.
   - Never convert an invalid scoring run into a valid run during finalize.

### 14.5 Low-Cost Test Fixtures

Create or extend a focused test script:

- Preferred new script: `scripts/taskspace-benchmark/test-e3-score-validity.ps1`
- Acceptable alternative: extend `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` if the new tests stay readable.

Use synthetic JSON fixtures under:

```text
scripts/taskspace-benchmark/test-fixtures/e3-score-validity/
```

Required fixture cases:

| Fixture | Standard side | TaskSpace side | Expected pair outcome | Expected score validity |
|---|---|---|---|---|
| `clean-solved.json` | validation pass | validation pass | `solved` or `both_success` | `true` |
| `clean-wrong.json` | validation fail after tests started | validation fail after tests started | `wrong` | `true` |
| `clean-agent-timeout.json` | clean timeout or normal finish | `exec_timed_out=true`, no infra reason | `agent_exec_timeout` | `true` |
| `validator-timeout.json` | `public_validation_exit_code=124` | any | `engineering_unclean` | `false` |
| `docker-run-failure.json` | `docker_run_failure=true` | any | `engineering_unclean` | `false` |
| `proof-false.json` | validator fidelity false | any | `engineering_unclean` | `false` |
| `audit-missing.json` | manual review required | manual review not completed | `engineering_unclean` in scoring mode | `false` |
| `timeout-plus-docker.json` | timeout plus Docker failure | any | `engineering_unclean`, not clean timeout | `false` |
| `previous-e3-rerun-summary.json` | previous run counts | previous run counts | all scoring disabled | `false` |

Minimum assertions:

```powershell
Assert-Equal $result.score_valid $false
Assert-Equal $result.score_fields_enabled $false
Assert-Contains $result.engineering_unclean_reasons.Keys 'public_validation_timeout'
Assert-Null $result.taskspace_better
Assert-Null $result.standard_better
Assert-Null $result.pass_rate_delta
```

### 14.6 Smoke And Regression Commands

Run the tests in this order. Do not start full E3 until every command below passes.

```powershell
.\scripts\taskspace-benchmark\test-e3-score-validity.ps1
.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1
.\scripts\taskspace-benchmark\test-e3-proof-harness.ps1
.\scripts\taskspace-benchmark\test-harness.ps1
```

After the cheap tests pass, run exactly one scoring smoke pair:

```powershell
.\scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1 `
  -Version '0.0.4' `
  -SampleLimit 1 `
  -RepeatCount 1 `
  -ScoringMode
```

Expected smoke result:

- If clean, aggregate JSON has `score_valid=true` and exactly one clean comparable pair or clean agent timeout.
- If unclean, process exits `3`, `suite-health.json` names the first engineering reason, and no remaining samples are scheduled.
- Any other exit code or missing `score_valid` field is a harness bug and blocks full E3.

Only after the one-pair smoke passes may full E3 be scheduled:

```powershell
.\scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1 `
  -Version '0.0.4' `
  -SampleLimit 3 `
  -RepeatCount 5 `
  -ScoringMode
```

### 14.7 Full E3 Start Gate

Before running a multi-hour E3 suite, the operator must verify these artifacts from the cheap gate:

Canonical command:

```powershell
.\scripts\taskspace-benchmark\invoke-taskspace-e3-start-gate.ps1 `
  -ScenarioPath <materialized-scenario-path> `
  -RunRoot <planned-e3-run-root> `
  -OutputDir <gate-output-dir> `
  -RunSelfTests
```

For external suites where scenarios are materialized later by the adapter, pass `-TaskListPath <task-list>` and still use `-RunSelfTests`. In that mode the path-contract gate is explicitly `skipped` with reason `no_scenario_manifest`; it is not a pass. Full E3 may proceed only when either a materialized scenario path-contract gate has passed or the adapter/materialization preflight is expected to enforce the path contract before agent execution.

The implementation is intentionally strict:

- omitted `-RunSelfTests` is a failed gate unless `-AllowSkippedSelfTests` is supplied for a diagnostic-only smoke;
- omitted scenario/path-contract evidence is a failed gate unless `-AllowSkippedPathContract` is supplied and the operator relies on later adapter/materialization preflight;
- `TaskListPath` must exist, parse, contain at least one task, include `task_dir`, and provide `source_version` either per row or through the command default;
- Docker storage checks must produce an explicit pass; an empty/unverified Docker storage result is not a pass.

| Gate | Required value |
|---|---|
| Disk preflight | `harness-health.json.status = pass` and no checked volume below configured free-space threshold |
| Docker storage | Docker data root and target run root have enough free space |
| Path contract | Generated validator paths are fully qualified, not relative to pair workspace |
| Cheap start-gate artifact | `e3-start-gate.json.status = pass` and `run_validity = valid` |
| Cheap self-tests | `cheap_self_tests.status = pass` when `-RunSelfTests` is used |
| Proof harness | `test-e3-proof-harness.ps1` passes |
| Score validity fixtures | all fixture cases pass |
| One-pair smoke | has explicit `score_valid=true` or classified exit code `3` |
| Report language | invalid run report contains no better/worse/regressed conclusion |

The gate writes `e3-start-gate.json` and `e3-start-gate.md`. If any hard gate fails, the process exits `3` and the JSON uses `run_validity=invalid_harness`. A failed start gate is not an E3 result and must not be summarized as TaskSpace better/worse/regressed.

If any gate fails, do not run full E3. Fix the gate first and rerun the cheap suite.

### 14.8 Logging Requirements

Add explicit structured events. These are required because the last failure was visible only after expensive work had already completed.

| Event | Emitted by | Required fields |
|---|---|---|
| `score_validity_evaluated` | aggregate reducer | `run_id`, `sample_id`, `score_valid`, `engineering_unclean_count`, `agent_exec_timeout_count` |
| `engineering_unclean_detected` | runner after pair | `run_id`, `pair_id`, `side`, `reasons`, `first_failure_artifact` |
| `scoring_run_aborted` | runner and suite driver | `run_id`, `abort_scope`, `abort_phase`, `exit_code`, `reason` |
| `audit_score_blocked` | audit manifest writer | `run_id`, `pair_id`, `audit_status`, `missing_decision_count` |
| `suite_score_invalidated` | suite driver | `suite_run_id`, `child_run_id`, `sample_id`, `reason`, `remaining_samples_skipped` |

The markdown report should include the first failure artifact path. The JSON logs should include stable reason codes so future tooling can aggregate failures across runs.

### 14.9 Acceptance Criteria For This Repair

This repair is complete only when all items below are true:

1. The previous E3 failure pattern is reproducible by a cheap fixture and produces `score_valid=false`.
2. Public validation timeout is classified as `engineering_unclean`, not a model wrong answer.
3. Docker build/run/cleanup/inspect failure is classified as `engineering_unclean`, not an agent outcome.
4. Missing audit decision in scoring mode invalidates the score.
5. Clean agent execution timeout remains score-bearing.
6. Timeout mixed with Docker, validator, proof, audit, disk, path, or report failure is not counted as clean timeout.
7. Suite process stops after the first scoring-invalid child run.
8. Aggregate JSON includes all score-validity fields.
9. Invalid markdown reports contain no better/worse/regressed language.
10. Full E3 cannot be started from the documented workflow before cheap fixture tests and one-pair smoke pass.

### 14.10 Non-Negotiable Engineering Constraint

A completed process is not the same thing as a valid benchmark score. E3 execution is considered successful only when:

- all planned scoring pairs either produce `solved`, `wrong`, or clean `agent_exec_timeout`;
- no engineering-unclean reason appears in any scoring pair;
- audit requirements are completed before score reporting;
- aggregate report sets `score_valid=true`.

If these conditions are not met, the run may still be useful for diagnostics, but its score is invalid and must be reported as such.

## 15. 2026-06-14 Runtime Reduction And Parallel Execution Addendum

This addendum extends the hard clean-execution plan with a runtime-reduction track. Runtime optimization must not weaken scoring validity. The execution order is:

1. instrument phase timing;
2. stop invalid scoring runs early;
3. reduce validator/Docker overhead;
4. introduce bounded parallelism;
5. tune TaskSpace-side agent cost only under explicit benchmark profiles.

### 15.1 Baseline From Rerun3

Use `C:\w\e3v004-rerun3-20260614-010208` as the current timing baseline until a newer clean timing run exists.

| Metric | Value | Interpretation |
|---|---:|---|
| suite elapsed | ~333.72 min | `2026-06-14T01:02:08` to `2026-06-14T06:35:51` |
| pair loop elapsed | 332.45 min | almost all time is inside pair execution |
| average pair elapsed | 22.16 min | 15 pair intervals from `events.jsonl` |
| agent execution total | 190.52 min | sum of Standard and TaskSpace `wall_time_ms` |
| non-agent overhead | 141.92 min | validation, Docker, oracle, report, proof, audit |
| overhead share | 42.7% | too high for repeated engineering-unclean runs |
| Standard agent time | 44.68 min | 23.5% of agent time |
| TaskSpace agent time | 145.83 min | 76.5% of agent time |
| public validation timeout pairs | 8/15 | each timeout can spend up to 420 seconds per side |
| Docker failure pairs | 9/15 | Docker noise invalidates scoring and consumes time |
| agent timeout pairs | 2/15 | only score-bearing if otherwise clean |

Current code serialization facts:

- `run-taskspace-e3-suite.ps1` loops samples at line 70 and invokes each child synchronously with `& powershell @args` at line 137.
- `run-taskspace-benchmark.ps1` loops repeats at line 178.
- `run-taskspace-benchmark.ps1` runs pair sides sequentially for agent execution at line 317.
- `run-taskspace-benchmark.ps1` runs pair sides sequentially for public validation and hidden oracle at lines 368-374.

### 15.2 Runtime Goals

| Goal | Target | Measurement |
|---|---|---|
| Invalid scoring run fast-fail | stop in `<30-40 min` when first pair proves engineering-unclean | suite exit code `3`, `suite-health.json`, first-failure artifact |
| Preflight-invalid run fast-fail | stop in `<5 min` before agent execution | `harness-health.json`, `run-status.json` |
| One-pair scoring smoke | finish or classify in `<35 min` | `suite-timing.json` |
| Clean full E3 conservative speedup | reduce wall time by `>=30%` without score-validity regressions | compare against serial timing baseline |
| Clean full E3 target speedup | reach `2-3x` after resource governor validation | calibrated run with bounded parallelism |
| Timing observability | every phase has start/end/duration fields | `pair-timing.json`, `sample-timing.json`, `suite-timing.json` |

Do not use wall-time speedup alone as acceptance. A faster run with weaker score validity, missing proof, or hidden artifact collisions is a failed optimization.

### 15.3 Phase R0: Phase Timing Instrumentation

#### Objective

Make runtime explainable before changing execution behavior.

#### Entry Criteria

- Section 14 score-validity fields are either implemented or scheduled in the same branch.
- Existing `events.jsonl` writing works for sample and pair events.
- Rerun3 timing baseline is documented in this plan and in COE.

#### Implementation Tasks

1. Add a small timing helper, preferably `scripts/taskspace-benchmark/lib/timing.ps1`.
2. Add `Start-TaskspaceTimingSpan` and `Stop-TaskspaceTimingSpan` helpers that emit:
   - `span_id`
   - `parent_span_id`
   - `run_id`
   - `sample_id`
   - `pair_id`
   - `side`
   - `logical_mode`
   - `phase`
   - `started_at`
   - `finished_at`
   - `duration_ms`
   - `exit_code`
   - `timed_out`
   - `engineering_unclean_reasons`
   - `concurrency_slot`
3. Emit phase spans for:
   - suite startup and shutdown
   - sample materialization
   - sample preflight
   - pair preflight
   - validator probe per side
   - pair workspace materialization
   - side agent execution
   - oracle isolation probe
   - public validation per side
   - hidden oracle per side
   - metrics extraction
   - E3 proof generation
   - audit manifest generation
   - pair report rendering
   - aggregate report rendering
4. Write artifacts:
   - `pair-timing.json` in each pair directory
   - `sample-timing.json` in each sample run root
   - `suite-timing.json` in the suite root
5. Add timing summary to markdown reports:
   - total elapsed
   - agent elapsed
   - validation elapsed
   - Docker elapsed
   - proof/report/audit elapsed
   - queue wait elapsed if parallelism is enabled later

#### Deliverables

- `lib/timing.ps1`
- timing fields in pair/sample/suite JSON
- timing summary in aggregate markdown
- fixture tests for span aggregation

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| timing helper unit behavior | synthetic start/stop fixture | duration is non-negative and phase names are stable |
| serial run timing | one-pair smoke | sum of child spans is within 5% of pair elapsed |
| report rendering | fixture aggregate | markdown shows agent vs overhead split |
| missing stop protection | fixture with interrupted span | reports span as `incomplete`, not zero |

#### Exit Criteria

- A one-pair smoke produces `pair-timing.json`, `sample-timing.json`, and `suite-timing.json`.
- Reports can explain whether time was spent in agent, validation, Docker, oracle, report, or queue wait.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| instrumentation changes behavior | benchmark comparability risk | new failures after timing-only patch | keep helpers write-only and side-effect-free | disable timing with `-DisableTimingArtifacts` |
| timing files are incomplete after abort | misleading reports | missing `finished_at` | mark span incomplete and include abort reason | rely on existing `events.jsonl` |

### 15.4 Phase R1: Invalid Run Fast-Fail

#### Objective

Convert the most certain speedup into a hard runtime behavior: engineering-unclean scoring runs stop early.

#### Design Approach

This is the speed payoff from Section 14. Do not run full repeat suites after the first scoring-invalid evidence appears.

Abort rules in scoring mode:

| Condition | Stop Scope | Expected Time |
|---|---|---:|
| disk/path/materialization preflight failure | suite or sample | `<5 min` |
| validator probe failure before agent execution | sample or suite | `<5 min` |
| first pair has public validation timeout | suite | first pair duration, target `<30-40 min` |
| first pair has Docker build/run/cleanup/inspect failure | suite | first pair duration, target `<30-40 min` |
| audit is required but no completed decision in scoring mode | before score report | no better/worse output |
| clean agent execution timeout only | continue | score-bearing outcome |

#### Implementation Tasks

1. Wire `score_valid=false` from pair-level hard classifier into runner control flow.
2. Add `-ScoringMode` to suite and sample runners as described in Section 14.
3. After each pair, if `engineering_unclean=true`:
   - write `pair-abort.json`;
   - write `sample-status.json` with `run_validity=invalid_harness`;
   - write `suite-health.json` with `status=invalid_harness`;
   - skip remaining repeats and samples;
   - exit `3`.
4. Add `remaining_pairs_skipped` and `remaining_samples_skipped` to suite health.
5. Add `expected_time_saved_minutes` to `suite-health.json` using the current serial baseline when available.

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| previous invalid pattern | synthetic rerun3 fixture | exits `3`, does not schedule sample 2 |
| public validation timeout | one-pair fixture | aborts after pair 1 |
| Docker failure | one-pair fixture | aborts after pair 1 |
| clean agent timeout | fixture with no infra reasons | does not abort as invalid |
| report language | invalid aggregate fixture | no better/worse/regressed wording |

#### Exit Criteria

- An engineering-unclean scoring run cannot reach full 15-pair completion.
- The invalid-rerun path drops from the rerun3 baseline of ~333 minutes to first-pair classification time.

### 15.5 Phase R1.5: Runtime Bottleneck Attribution

#### Objective

Explain why a 15-pair E3 run takes hours before changing concurrency or agent budgets. The first implementation should produce phase-level timing evidence from the artifacts already written by the runner, then use that evidence to decide which optimization is allowed in scoring mode.

#### Timing Model

Every pair must be decomposed into these spans:

| Span | Source Artifact | Owner | Optimization Boundary |
|---|---|---|---|
| `standard_agent_duration_ms` | standard side run metadata / process timing | agent side | comparable scoring behavior; timeout is valid result |
| `taskspace_agent_duration_ms` | TaskSpace side run metadata / process timing | agent side | profile changes require versioned score profile |
| `validator_probe_duration_ms` | `validator-probe-result.json` | harness | must not run tests |
| `public_validation_duration_ms` | validation process timing | harness / oracle | score-bearing; cannot skip unless engineering-unclean already proven |
| `hidden_oracle_duration_ms` | oracle process timing | harness / oracle | no hidden rerun when public validation already made run invalid |
| `docker_build_duration_ms` | `docker-build-result.json` | harness / Docker | cache only with immutable digest-pinned Dockerfile proof |
| `docker_run_duration_ms` | `docker-run-result.json` or metrics stderr/stdout markers | harness / Docker | no score semantic changes |
| `docker_cleanup_duration_ms` | `validation-cleanup-result.json` | harness / Docker | bounded cleanup; failure invalidates scoring run |
| `worker_queue_wait_ms` | future `parallelism.json` | scheduler | resource governor controlled |
| `resource_wait_ms` | future `resource-governor.json` | scheduler | must not hide disk/API/Docker pressure |

Pair report and aggregate report should show:

- total wall time;
- agent time subtotal;
- validator/Docker subtotal;
- cleanup subtotal;
- idle/queue subtotal when parallelism is enabled;
- top three spans by duration;
- time saved by fast-fail or cache, when applicable.

#### Bottleneck Classification Rules

Use deterministic thresholds so the operator can tell whether the run is slow for valid benchmark reasons or harness reasons:

| Classification | Rule | Action |
|---|---|---|
| `agent_bound` | agent subtotal is `>=70%` of wall time and no engineering-unclean signals exist | optimize only through explicit score profile or accept as benchmark cost |
| `validator_bound` | public/hidden validation subtotal is `>=30%` of wall time | implement timeout split, probe, and no-op rerender first |
| `docker_build_bound` | Docker build subtotal is `>=15%` or repeated same-scenario builds appear | enable digest-pinned Docker image cache after two-run smoke |
| `cleanup_bound` | cleanup subtotal is `>=5%` or cleanup has unbounded tail | move to bounded cleanup and fail invalid in scoring mode |
| `queue_bound` | workers wait `>=10%` of wall time after parallelism is enabled | tune resource governor, not agent profile |
| `engineering_unclean_slow` | any non-timeout infra failure appears after expensive work | fail-fast bug; add preflight/probe/sentinel coverage |

These classes are diagnostics. They must not change scoring results directly; they decide which engineering phase is allowed next.

#### Implementation Tasks

1. Add timing span extraction to `scripts/taskspace-benchmark/lib/metrics-extractor.ps1` using existing process start/end data and generated validator artifacts.
2. Add missing validator-side timestamps in generated Terminal-Bench validators:
   - before Docker image inspect/build;
   - after image build or cache hit;
   - before Docker run;
   - after Docker run;
   - before cleanup dispatch;
   - after bounded cleanup result is recorded.
3. Add `timing_breakdown` to pair metrics:
   - raw durations;
   - subtotal percentages;
   - largest span name;
   - bottleneck classification.
4. Add aggregate timing summary:
   - median and p95 per span;
   - total time by span across all pairs;
   - repeated Docker build count by cache key;
   - `estimated_serial_full_run_minutes`;
   - `estimated_optimized_full_run_minutes`.
5. Add regression fixtures:
   - synthetic agent-bound pair;
   - synthetic validator-bound pair;
   - repeated Docker-build fixture;
   - cleanup-bound fixture;
   - engineering-unclean slow fixture.
6. Add report language rule: if `score_valid=false`, timing report may say "time wasted / avoided" but must not say TaskSpace was better or worse.

#### Acceptance Tests

| Validation Item | Method | Passing Standard |
|---|---|---|
| timing extraction | synthetic metrics fixture | every span is present or explicitly `null` with reason |
| bottleneck classification | fixed-duration fixture matrix | each class maps to the expected rule |
| aggregate summary | 3-pair fixture | median/p95/subtotal values are deterministic |
| repeated Docker build detection | same cache key fixture | report lists duplicate build count |
| invalid slow run language | invalid aggregate fixture | reports engineering waste without score comparison |

#### Exit Criteria

- A 15-pair run can be explained by phase totals without reading raw logs.
- Runtime optimization work is prioritized from measured bottlenecks, not intuition.
- The plan can distinguish "agent legitimately slow" from "harness wasting hours".

### 15.6 Phase R2: Validator And Docker Overhead Reduction

#### Objective

Reduce the 141.92-minute non-agent overhead without weakening validator fidelity.

#### Design Approach

The current run repeatedly pays for validator setup, Docker work, and validation timeout. Optimize only when the validator source and Dockerfile are proven immutable for the pair.

Safe reductions:

1. cache Docker builds by scenario source hash;
2. separate "no tests started" timeout from full test timeout;
3. reuse materialized uv cache safely;
4. make cleanup bounded and observable;
5. avoid rerunning hidden oracle if public validation already produced a hard engineering-unclean reason in scoring mode.

#### Implementation Tasks

1. Add validator timing fields to `metrics.json`:
   - `validator_probe_duration_ms`
   - `public_validation_duration_ms`
   - `hidden_oracle_duration_ms`
   - `docker_build_duration_ms`
   - `docker_run_duration_ms`
   - `docker_cleanup_duration_ms`
   - `tests_started_at`
   - `tests_completed_at`
2. Add Docker image cache metadata:
   - `scenario_id`
   - `source_version`
   - `dockerfile_sha256`
   - `validator_script_sha256`
   - `uv_cache_sha256` or materialization marker
   - `cache_hit`
   - `cache_key`
3. Build or reuse validator image once per immutable scenario hash.
4. If an agent modifies Dockerfile, validator source, or proof-sensitive files, mark `engineering_unclean`; do not use the cache to hide the mutation.
5. Split validation timeout:
   - `ValidationPretestTimeoutSeconds`, default `90-120`;
   - `ValidationTestTimeoutSeconds`, default remains close to current `420` after `tests_started`;
   - a timeout before `tests_started` is `engineering_unclean`.
   - score-bearing public validation, not only `-ProbeOnly`, must use this split.
   - stdout/stderr markers must be durable while the process is running; timeout classification cannot depend on normal process exit.
6. Add bounded cleanup:
    - cleanup must have a timeout;
    - cleanup failure is engineering-unclean in scoring mode;
    - cleanup artifacts include container/image IDs and failure reason.
   - generated validators must not run unbounded Docker cleanup in `finally`; cleanup should be delegated to the bounded runner cleanup path or use an equivalent bounded helper.
7. Add a no-op mode for diagnostic rerenders so `finalize` does not re-run expensive validators.

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| Docker cache hit | two-pair same scenario smoke | second pair records `cache_hit=true` without missing proof |
| Docker cache invalidation | mutate Dockerfile fixture | cache is bypassed or run invalidated |
| pretest timeout | synthetic no-marker validator | aborts before full 420s |
| tests-started timeout | synthetic slow test fixture | classified separately from pretest failure |
| cleanup timeout | synthetic cleanup failure | bounded duration and classified artifact |

#### Current Implementation Notes

- `ValidationPretestTimeoutSeconds` and `ValidationTestTimeoutSeconds` are now runner-facing budgets for score-bearing public validation.
- The validation runner polls stdout/stderr files during execution and records `taskspace_validation_timeout_phase`, `taskspace_tests_started_at`, and `taskspace_tests_completed_at` markers when available.
- Terminal-Bench generated validator cleanup is deferred to the runner cleanup path, which performs identity-checked Docker cleanup with bounded process execution and writes `validation-cleanup-result.json`.
- Terminal-Bench Docker image caching is implemented behind `-EnableDockerImageCache`; it sets `TASKSPACE_DOCKER_IMAGE_CACHE=1` only for score-bearing public validation, records `cache_key/cache_image/fixture_sha256/dockerfile_sha256`, and treats `cache_hit` as a non-failure Docker build phase.
- Cache invalidation is content-hash based: fixture tree or Dockerfile changes produce a different cache key.
- Cache eligibility is intentionally fail-closed: generated validators only use the image cache when every parsed Dockerfile `FROM` reference is digest-pinned as `@sha256:<64 hex>`. Floating tags, `ARG`-based bases, missing `FROM`, or parser-uncertain forms record `cache_eligible=false` and bypass cache even when `-EnableDockerImageCache` is set.
- Runtime cache hit behavior is generated-script covered and metadata/invalidation tested; a real-Docker two-run smoke is still required before enabling cache in full production E3.
- No-op diagnostic rerenders and parallel execution remain future work until their own tests pass.

#### Exit Criteria

- Timing artifacts show validator/Docker overhead by phase.
- Repeated same-scenario validation does not rebuild immutable Docker images unnecessarily.
- No validator optimization changes score-bearing semantics.

### 15.7 Phase R3: Bounded Parallel Execution

#### Objective

Reduce clean full E3 wall time with controlled concurrency while preserving artifact isolation and score validity.

#### Parallelism Levels

| Level | Flag | Default | Risk | Intended Use |
|---|---|---:|---|---|
| L0 serial | existing behavior | enabled | lowest | baseline and debugging |
| L1 side validation parallelism | `-MaxParallelValidationsPerPair` | 1 | moderate Docker contention | reduce validation wall time |
| L2 pair parallelism within sample | `-MaxParallelPairsPerSample` | 1 | shared Docker/cache collisions | clean run speedup after cache locks |
| L3 sample parallelism | `-MaxParallelSamples` | 1 | high disk/Docker/API contention | calibrated full-suite speedup |
| L4 side agent parallelism | `-MaxParallelSidesPerPair` | 1 | can distort wall-time comparison | only when `timing_comparison_valid=false` or explicitly accepted |

Do not enable L4 by default. The benchmark currently reports wall-time ratios; concurrent side execution can change timing comparability because model/API rate limits and host contention may affect sides unevenly.

#### Resource Governor

Add a scheduler module, preferably `scripts/taskspace-benchmark/lib/resource-governor.ps1`, before enabling parallel execution.

Required resources:

| Resource | Config | Guard |
|---|---|---|
| model/API calls | `-MaxModelConcurrency` | prevents runaway cost/rate-limit contention |
| Docker build/run | `-MaxDockerConcurrency` | prevents WSL/Docker saturation |
| validation process | `-MaxValidationConcurrency` | bounds validator timeout pileups |
| disk free space | existing disk checks plus per-worker reservation | prevents D drive / Docker storage exhaustion |
| CPU/memory | optional host probe | emits warning and lowers concurrency if unavailable |
| artifact paths | unique pair/sample/run roots | prevents write collisions |
| Docker image cache | lock by cache key | prevents simultaneous rebuild races |

Every worker must acquire resource tokens before execution and release them in `finally`.

#### Implementation Tasks

1. Add concurrency flags to suite runner:
   - `-MaxParallelSamples`
   - `-MaxParallelPairsPerSample`
   - `-MaxParallelValidationsPerPair`
   - `-MaxDockerConcurrency`
   - `-MaxModelConcurrency`
   - `-DisableParallelTimingComparison`
2. Keep defaults at `1`.
3. Add a worker abstraction for sample execution:
   - job ID;
   - sample ID;
   - run root;
   - stdout/stderr paths;
   - exit code;
   - score-validity result;
   - timing result.
4. Add a worker abstraction for pair execution only after sample-level parallelism is stable.
5. Use unique artifact roots and deterministic pair IDs:
   - `pair-001`;
   - worker temp path under that pair only;
   - no shared current working directory.
6. Merge worker outputs in deterministic sample/pair order before aggregate reporting.
7. If any worker reports `engineering_unclean` in scoring mode:
   - stop scheduling new work;
   - allow already-running workers to finish or terminate by explicit policy;
   - mark their state as `cancelled_due_to_score_invalid` if terminated.
8. Write `parallelism.json`:
   - configured concurrency;
   - actual max concurrency observed;
   - resource wait times;
   - cancelled workers;
   - timing comparability status.

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| default compatibility | serial self-tests | artifacts unchanged except timing fields |
| sample parallel smoke | 3 samples x 1 repeat with `-MaxParallelSamples 2` | no path collisions; deterministic aggregate order |
| pair parallel smoke | 1 sample x 2 repeats with `-MaxParallelPairsPerSample 2` | no pair artifact collisions |
| validation parallel smoke | 1 pair with `-MaxParallelValidationsPerPair 2` | both side metrics written correctly |
| fail-fast with workers | injected engineering-unclean worker | new scheduling stops and suite exits `3` |
| resource governor | low disk fixture | workers do not start when reservation fails |

#### Exit Criteria

- Parallelism is opt-in and bounded.
- Serial mode remains the default and passes all guardrail tests.
- Parallel mode produces deterministic artifacts and equivalent score validity.

### 15.8 Phase R4: TaskSpace-Side Agent Cost Controls

#### Objective

Reduce unnecessary TaskSpace-side runtime without silently changing the v0.0.4 score profile.

#### Design Approach

TaskSpace is the largest agent-side cost in rerun3. However, changing reasoning effort, subagent budgets, or node expansion can change agent capability. Therefore:

- scoring profile must remain explicit and reproducible;
- performance experiments must be labeled as non-comparable unless adopted into a new versioned benchmark profile;
- no silent fallback or hidden prompt shortcut is allowed.

#### Implementation Tasks

1. Add benchmark profile metadata:
   - `score_profile_id`
   - `model`
   - `model_reasoning_effort`
   - `taskspace_budget`
   - `subagent_budget`
   - `max_tool_calls`
   - `max_open_nodes`
2. Report TaskSpace cost drivers:
   - tool calls;
   - model calls;
   - token usage if available;
   - node count;
   - edge count;
   - subagent result count;
   - unreviewed result count;
   - open leaf nodes.
3. Add a diagnostic-only "budget profile" smoke:
   - `model_reasoning_effort=high` or lower;
   - smaller subagent budget;
   - shorter timeout;
   - mark output `diagnostic_only=true`.
4. If a cheaper profile is considered for scoring, require a new versioned profile and a fresh comparison against the old profile.

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| profile metadata | one-pair smoke | aggregate includes profile ID and model settings |
| diagnostic profile | non-scoring smoke | report says `diagnostic_only=true` |
| scoring profile lock | full E3 command | profile settings are recorded and reproducible |
| no hidden shortcuts | prompt/code review | no hardcoded natural-language answer paths |

#### Exit Criteria

- TaskSpace runtime can be analyzed by cost driver.
- Any runtime-saving profile change is explicit and versioned.

### 15.9 Runtime Reduction Rollout Plan

Proceed in this order:

1. Implement timing artifacts in serial mode.
2. Implement runtime bottleneck attribution and aggregate timing summaries.
3. Implement score-validity fast-fail.
4. Validate invalid rerun fixture exits early.
5. Implement validator/Docker duration fields.
6. Add Docker cache and pretest timeout split behind flags.
7. Run one-pair scoring smoke.
8. Add resource governor with defaults still serial.
9. Add sample-level parallelism.
10. Add pair-level parallelism only after sample-level proof.
11. Add validation-level parallelism only after Docker cache locks are proven.
12. Leave side-agent parallelism disabled unless timing comparability is explicitly disabled.

### 15.10 Runtime Acceptance Matrix

| Scenario | Command Shape | Expected Runtime Behavior | Passing Standard |
|---|---|---|---|
| preflight invalid | one sample scoring run | abort before agent | `<5 min`, exit `3` |
| first-pair engineering-unclean | one sample scoring run | abort after pair 1 | `<30-40 min`, no pair 2 |
| invalid rerun3 fixture | synthetic aggregate | no full scheduling | score invalid and no score language |
| timing attribution fixture | synthetic 3-pair aggregate | runtime split by phase | deterministic bottleneck class and subtotal percentages |
| serial one-pair clean smoke | `MaxParallel*=1` | baseline timing | complete or clean timeout with timing artifacts |
| sample parallel smoke | `-MaxParallelSamples 2` | two samples overlap | deterministic artifacts, no score drift |
| pair parallel smoke | `-MaxParallelPairsPerSample 2` | two repeats overlap | no path/cache collision |
| validation parallel smoke | `-MaxParallelValidationsPerPair 2` | both side validations overlap | correct metrics for both sides |
| full clean calibrated run | selected concurrency profile | faster than serial | `>=30%` wall-time reduction and `score_valid=true` |

### 15.11 Required Commands Before Full Parallel E3

```powershell
.\scripts\taskspace-benchmark\test-e3-score-validity.ps1
.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1
.\scripts\taskspace-benchmark\test-e3-proof-harness.ps1
.\scripts\taskspace-benchmark\test-harness.ps1
```

Timing smoke:

```powershell
.\scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1 `
  -Benchmark terminal-bench `
  -TaskListPath <task-list> `
  -SourceVersion <terminal-bench-source-version> `
  -Repeats 1 `
  -RunRoot <run-root> `
  -WhaleBin <whale-bin> `
  -Model deepseek-v4-flash `
  -TimeoutSeconds 900 `
  -ValidationTimeoutSeconds 420 `
  -SandboxMode full-auto `
  -ConfigOverride 'model_reasoning_effort="max"' `
  -ScoringMode
```

First safe parallel smoke:

```powershell
.\scripts\taskspace-benchmark\run-taskspace-e3-suite.ps1 `
  -Benchmark terminal-bench `
  -TaskListPath <task-list> `
  -SourceVersion <terminal-bench-source-version> `
  -Repeats 1 `
  -RunRoot <run-root> `
  -WhaleBin <whale-bin> `
  -Model deepseek-v4-flash `
  -TimeoutSeconds 900 `
  -ValidationTimeoutSeconds 420 `
  -SandboxMode full-auto `
  -ConfigOverride 'model_reasoning_effort="max"' `
  -ScoringMode
```

Parallel execution flags such as `-MaxParallelSamples`, `-MaxDockerConcurrency`, and `-MaxModelConcurrency` are planned runner contract, not current suite-runner CLI. Do not add them to production E3 commands until Phase R3 implements and tests them.

Do not run full parallel E3 until the smoke artifacts prove:

- no pair/sample path collisions;
- no Docker cache race;
- no missing timing spans;
- no score-validity mismatch between serial and parallel mode;
- enough disk and Docker storage remains after cleanup.

### 15.12 Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| parallel workers collide in artifact paths | invalid evidence | duplicate or missing pair artifacts | deterministic path allocation and path-lock tests | disable parallel flags |
| Docker cache hides validator mutation | false clean score | source hash mismatch or modified Dockerfile | source guard and cache key proof | rebuild image and mark score invalid |
| side-agent parallelism distorts wall-time ratio | misleading utility comparison | timing comparison changes under concurrency | keep side-agent parallelism off for scoring | serial side execution |
| model/API rate limits slow or fail workers | flaky runtime | provider errors or long queue wait | `MaxModelConcurrency`, retry policy, queue timing | lower concurrency |
| disk fills faster under concurrency | system instability | disk preflight fails or free space drops below threshold | per-worker disk reservation | abort scheduling new workers |
| fast-fail cancels useful diagnostics | less evidence | operator needs forensic run | `-DiagnosticOnly` or `-DisableScoringFastFail` explicit flag | diagnostic serial run |
| timing attribution is incomplete | wrong optimization priority | spans are `null` without reason or totals do not reconcile | require nullable reason and reconciliation check | keep serial baseline and block parallel rollout |

### 15.13 Runtime Success Definition

Runtime work is successful only when both statements are true:

- invalid scoring runs fail early with precise engineering-unclean artifacts;
- clean scoring runs are faster without losing score validity, proof fidelity, artifact determinism, or report clarity.

The expected practical result is:

- engineering-unclean runs: from multi-hour to `<30-40 min` when the first pair exposes the issue;
- preflight-invalid runs: `<5 min`;
- clean full E3: conservative `>=30%` speedup first, then calibrated `2-3x` only after resource-governed parallelism proves stable.
