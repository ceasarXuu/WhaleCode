# TaskSpace E3 Harness Guardrails Implementation Plan

- Created: 2026-06-13
- Updated: 2026-06-14
- Version: 0.2
- Status: Repair-ready plan for hard clean-execution scoring contract
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

| Gate | Required value |
|---|---|
| Disk preflight | `harness-health.json.status = pass` and no checked volume below configured free-space threshold |
| Docker storage | Docker data root and target run root have enough free space |
| Path contract | Generated validator paths are fully qualified, not relative to pair workspace |
| Proof harness | `test-e3-proof-harness.ps1` passes |
| Score validity fixtures | all fixture cases pass |
| One-pair smoke | has explicit `score_valid=true` or classified exit code `3` |
| Report language | invalid run report contains no better/worse/regressed conclusion |

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
