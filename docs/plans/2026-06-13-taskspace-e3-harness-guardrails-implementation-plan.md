# TaskSpace E3 Harness Guardrails Implementation Plan

- Created: 2026-06-13
- Updated: 2026-06-14
- Version: 0.5
- Status: Implementation-ready plan draft for hard clean-execution scoring, runtime bottleneck diagnosis, and governed speedup; not approved for speed claims
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
- 不把所有 validation failure 都归为 harness failure。进入真实测试后的 assertion failure 仍然是 benchmark outcome；validator timeout 例外，仍按 engineering-unclean 处理，因为硬执行约束只允许 agent execution timeout 作为意外超时结果。
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
| public validation timeout happens before or after `tests_started` | classify as engineering-unclean timeout; lifecycle stage only determines reason detail |

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
- Synthetic metrics where validation times out after `tests_started` remains engineering-unclean and score-invalid.
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
- Valid assertion failures after `tests_started` still render normal utility/diagnostic comparison; validator timeouts do not.
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
| Guardrail false-positive aborts real benchmark failures | Could hide legitimate TaskSpace regressions | Require lifecycle marker distinction: pre-test/probe failures can abort; assertion failures after `tests_started` are benchmark outcomes; validator timeouts remain engineering-unclean |
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
3. An assertion failure after `tests_started` remains a normal benchmark result and does not trigger harness abort; a validator timeout after `tests_started` remains engineering-unclean and score-invalid.
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
| `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` | Provides a suite-level entrypoint and child run orchestration. | It must stop scheduling later samples as soon as a scoring child emits an engineering-unclean abort artifact. Pure audit-required children must block score publication but not invalidate the harness. |
| `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` | Orchestrates sample and pair execution, writes pair artifacts, aggregate reports, run status, and E3 audit-required states. | It currently allows a run to reach `completed` or `audit_required` without one authoritative score-readiness gate. It needs an explicit scoring mode gate. |
| `scripts/taskspace-benchmark/lib/aggregate-report.ps1` | Emits `run_validity`, invalid-run fields, and disables some diagnostic comparison fields when the run is invalid. | It does not yet expose the hard scoring contract fields `score_ready`, `score_valid`, `score_block_reason`, `engineering_unclean_count`, `audit_required_count`, `agent_exec_timeout_count`, and `clean_comparable_pair_count`. |
| `scripts/taskspace-benchmark/lib/audit-manifest.ps1` | Records side metrics, inclusion flags, exclusion reason, failure taxonomy, utility direction, and audit status. | It lacks per-pair hard outcome fields such as `outcome_standard`, `outcome_taskspace`, `engineering_unclean`, `audit_required`, `engineering_unclean_reasons`, `run_score_ready`, and `run_score_valid`. |
| `scripts/taskspace-benchmark/lib/failure-taxonomy.ps1` | Maps many observed failures into diagnostic classes and utility directions. | Legacy classes such as `validator_slow_or_flaky` and `taskspace_overhead_timeout` can still look like comparable diagnostic outcomes. They need to feed a separate hard `engineering_unclean` classifier. |
| `scripts/taskspace-benchmark/lib/suite-status.ps1` | Distinguishes completed child process state from failed child process state. | It must also distinguish process completion from score validity. A child can finish cleanly as a process and still invalidate E3 scoring. |

### 14.2 Root Cause From The Previous E3 Run

The previous E3 run was invalid because the execution pipeline measured and aggregated tasks after the benchmark infrastructure had already become unclean.

Observed invalidating signals:

- `e3_human_review_not_completed` on all 15 pairs; this blocks score publication but is not by itself a hard engineering failure.
- `manual_review_required` on 10 of 15 pairs.
- `audit_unclean` on all 15 pairs in the legacy taxonomy; the repaired contract must split pure audit-pending from malformed or mismatched audit evidence.
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
  "score_ready": false,
  "score_valid": false,
  "score_block_reason": "engineering_unclean",
  "score_invalid_reason": "engineering_unclean",
  "score_fields_enabled": false,
  "engineering_unclean_count": 15,
  "engineering_unclean_reasons": {
    "public_validation_timeout": 8,
    "docker_run_failure": 7
  },
  "audit_required_count": 15,
  "agent_exec_timeout_count": 0,
  "clean_comparable_pair_count": 0,
  "score_bearing_outcomes": ["solved", "wrong", "agent_exec_timeout"]
}
```

Required report behavior:

- If `score_valid=false`, set `taskspace_better`, `standard_better`, `pass_rate_delta`, `diagnostic_pass_rate_delta`, and any "regressed" wording to `null`, `n/a`, or an explicit disabled value.
- If `score_valid=false`, the first markdown section must say the score is invalid before listing task counts.
- If `engineering_unclean=false` but `audit_required=true`, the first markdown section must say the score is pending audit, not invalid harness. It must still disable better/worse/regressed wording.
- If the only non-success condition is clean `agent_exec_timeout`, keep `score_valid=true` and count it as an agent outcome.
- If `agent_exec_timeout` is mixed with Docker, validator, proof, audit, disk, path, or report failure on the same pair, classify that pair as `engineering_unclean`, not as clean agent timeout.
- The aggregate report must preserve diagnostic details, but diagnostic detail is not a score.

Add the following fields to each pair-level audit manifest row:

```json
{
  "outcome_standard": "solved",
  "outcome_taskspace": "engineering_unclean",
  "engineering_unclean": true,
  "audit_required": false,
  "engineering_unclean_reasons": [
    "public_validation_timeout",
    "docker_run_failure"
  ],
  "agent_exec_timeout_clean": false,
  "run_score_ready": false,
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
   | `public_validation_exit_code=124`, even after `tests_started` | `engineering_unclean` reason `public_validation_timeout`; lifecycle markers only explain where the timeout happened |
   | Any Docker build/run/cleanup/inspect failure before trustworthy test result | `engineering_unclean` with the Docker reason code |
   | Validator proof missing, validator fidelity unproven, or E3 eligibility false | `engineering_unclean` |
   | Audit required but no completed audit decision in scoring mode, with no other hard infra reason | `audit_required`, `score_ready=false`, `score_valid=false`, not `engineering_unclean` |
   | Audit review malformed, hash-mismatched, or contradicts pair artifacts | `engineering_unclean` reason `e3_audit_review_invalid` |
   | Missing report, unparsable metrics, path not absolute, disk below threshold | `engineering_unclean` |

   Keep the old taxonomy functions for diagnostics, but make `Get-TaskspaceUtilityDirection` consume the hard outcome first. If hard outcome is `engineering_unclean`, utility direction must be `invalid_run` or `score_disabled`, never `taskspace_better`, `standard_better`, or `inconclusive`.

2. `scripts/taskspace-benchmark/lib/audit-manifest.ps1`

   Extend the manifest row object after side metrics are loaded:

   - Compute `engineering_unclean_reasons_standard`.
   - Compute `engineering_unclean_reasons_taskspace`.
   - Compute `outcome_standard`.
   - Compute `outcome_taskspace`.
   - Set pair-level `engineering_unclean` if either side has hard unclean reasons, or if cross-side proof/audit state is corrupt.
   - Set pair-level `audit_required` when the only blocker is missing human review.
   - Set `run_score_ready=false` when either `engineering_unclean=true` or `audit_required=true`.
   - Set `run_score_valid=false` when `engineering_unclean=true`, and keep it false until required audit is completed.

   The manifest writer must keep the old fields for backwards compatibility, but the new hard fields are authoritative for scoring.

3. `scripts/taskspace-benchmark/lib/aggregate-report.ps1`

   Add a score-validity reducer before any better/worse counts are calculated:

   ```powershell
   $scoreRows = $AuditRows | Where-Object { $_.sample_kind -eq 'e3' }
    $engineeringUncleanRows = $scoreRows | Where-Object { $_.engineering_unclean -eq $true }
    $auditRequiredRows = $scoreRows | Where-Object { $_.audit_required -eq $true }
    $cleanRows = $scoreRows | Where-Object { $_.engineering_unclean -ne $true -and $_.audit_required -ne $true }

    $scoreReady = ($engineeringUncleanRows.Count -eq 0 -and $auditRequiredRows.Count -eq 0)
    $scoreValid = $scoreReady
   ```

   Required reducer outputs:

   - `score_valid`
   - `score_ready`
   - `score_block_reason`
   - `score_invalid_reason`
   - `score_fields_enabled`
   - `engineering_unclean_count`
   - `engineering_unclean_reasons`
   - `audit_required_count`
   - `agent_exec_timeout_count`
   - `clean_comparable_pair_count`
   - `score_bearing_outcomes`

   If `$scoreReady -eq $false`, do not calculate public score deltas from all rows. Instead:

   - Preserve raw counts under a `diagnostics` object.
   - Set comparison fields to disabled values.
   - Render the markdown summary as "Score status: audit required" when the only blocker is audit.
   - Render the markdown summary as "Score validity: invalid" when any hard engineering unclean row exists.

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
   - If the only blocker is `audit_required`, set sample phase `audit_required`, keep `run_validity=valid`, keep the run resumable/finalizable, continue executable pairs, and exit without score language.
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

        if ($status.run_validity -eq 'invalid_harness' -or $aggregate.score_block_reason -eq 'engineering_unclean') {
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
   - `audit_required_child_runs`
   - `first_score_invalid_run`
   - `suite_score_ready`
   - `suite_score_valid`

   A child with phase `audit_required` and no hard engineering unclean reason is process-complete but score-pending. It must not be counted as a successful scoring child and must not invalidate the harness.

7. `scripts/taskspace-benchmark/finalize-taskspace-e3-run.ps1`

   Finalize must refuse to generate scoring language when existing artifacts contain hard unclean reasons.

   Required behavior:

   - Default for hard engineering unclean: fail with exit code `3` and write `run_validity=invalid_harness`.
   - Pure audit-pending artifacts: write `phase=audit_required`, `run_validity=valid`, `score_ready=false`, and no score language.
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
| `audit-missing.json` | manual review required | manual review not completed | `audit_required`, score pending, not hard infra | `false`, `score_ready=false` |
| `audit-invalid.json` | manual review hash mismatch | any | `engineering_unclean` | `false` |
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
  -OnePairSmokeRoot <one-pair-smoke-root> `
  -RunSelfTests
```

For external suites where scenarios are materialized later by the adapter, pass `-TaskListPath <task-list>` and still use `-RunSelfTests`. In that mode the path-contract gate is explicitly `skipped` with reason `no_scenario_manifest`; it is not a pass. Full E3 may proceed only when either a materialized scenario path-contract gate has passed or the adapter/materialization preflight is expected to enforce the path contract before agent execution.

The implementation is intentionally strict:

- omitted `-RunSelfTests` is a failed gate unless `-AllowSkippedSelfTests` is supplied for a diagnostic-only smoke;
- omitted scenario/path-contract evidence is a failed gate unless `-AllowSkippedPathContract` is supplied and the operator relies on later adapter/materialization preflight;
- omitted `-OnePairSmokeRoot` is a failed gate unless `-AllowSkippedOnePairSmoke` is supplied for a diagnostic-only gate;
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
| Clean full E3 conservative speedup | reduce wall time by the calibrated target without score-validity regressions | compare against serial timing baseline |
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

#### Current Implementation Notes

- `scripts/taskspace-benchmark/lib/timing.ps1` now emits `timing_breakdown`, subtotal percentages, largest span, `bottleneck_classification`, and `bottleneck_reason` for pair/sample/suite timing artifacts.
- Pair timing aggregates Docker build/run/cleanup durations from side metrics when validator artifacts provide them.
- Sample and suite timing aggregate Docker subtotals and bottleneck counts across child artifacts.
- Sample and suite timing expose `timing_quality`, `runtime_optimization_status`, and `runtime_optimization_blockers`; missing or malformed timing artifacts block runtime optimization claims instead of silently producing a speedup basis.
- `test-e3-harness-guardrails.ps1` includes synthetic timing fixtures for `agent_bound`, `validator_bound`, `docker_build_bound`, `cleanup_bound`, and `engineering_unclean_slow`.
- Sample and suite timing now include per-phase median/p95 summaries, repeated Docker cache key detection, and report-rendered top spans in aggregate reports.
- Real one-pair calibration on `single-file-fast-fix` writes `sample-timing.json`, and aggregate reports now render `Timing Summary` from that artifact before final status is written.
- Remaining R1.5 work is to calibrate these fields on an E3/Terminal-Bench one-pair run and use the resulting bottleneck class to choose the next runtime optimization.

### 15.5.1 E3 Runtime Bottleneck Investigation And Speedup Plan

#### Problem Statement

A 15-task E3 run taking several hours is not automatically acceptable. It can be legitimate if the agent spends most time solving within the configured timeout, but it is a harness defect if repeated validator setup, Docker builds, cleanup, score-invalid retries, or report/finalize rerenders dominate wall time. Runtime work must therefore first prove where time is spent, then apply only optimizations that preserve score validity.

#### Current Evidence To Collect

Every E3 run intended for scoring or timing calibration must persist enough data to answer these questions without reading raw console logs:

| Question | Required Artifact Field | Source |
|---|---|---|
| How much wall time was agent execution? | `timing_breakdown.agent_duration_ms` | pair timing |
| How much wall time was public validation? | `public_validation_duration_ms` | side metrics |
| How much validation time was Docker build? | `docker_build_duration_ms` | generated validator / Docker result |
| How much validation time was Docker run? | `docker_run_duration_ms` | generated validator / Docker result |
| How much time was cleanup? | `docker_cleanup_duration_ms` | cleanup result |
| Was the run already score-invalid? | `score_valid`, `engineering_unclean_reasons` | aggregate report |
| Were same Docker images rebuilt repeatedly? | `repeated_docker_cache_keys`, `cache_hit` | sample/suite timing |
| Did finalize rerun expensive work? | `finalize-health.json.validation_rerun_allowed=false` | finalize artifact |
| Did workers wait for resources? | `parallelism.json.resource_wait_ms` | planned resource governor |

Missing timing fields are themselves a blocker. A run with missing phase durations may be usable for functional debugging, but it is not acceptable evidence for runtime optimization decisions.

#### Bottleneck Classification Rules

Use deterministic classification before choosing a fix:

| Class | Trigger | Primary Fix Path |
|---|---|---|
| `agent_bound` | agent execution is the largest span and score is valid | profile/cost controls only after score profile is versioned |
| `validator_bound` | public validation dominates and tests started | validator timeout split, validation parallelism, better phase instrumentation |
| `docker_build_bound` | Docker build is largest or repeated same cache key builds appear | digest-pinned Docker image cache and cache-key locks |
| `docker_run_bound` | Docker run dominates after cache hit | validation parallelism with Docker concurrency governor |
| `cleanup_bound` | cleanup is non-trivial or unbounded | bounded cleanup and cleanup failure classification |
| `engineering_unclean_slow` | score invalid and time still continues after first hard infra failure | scoring fast-fail, suite circuit breaker, finalize no-op |
| `resource_wait_bound` | planned workers spend large time waiting for disk/Docker/model tokens | tune concurrency down or raise host capacity |

#### Engineering Workstream

1. Baseline one-pair timing.
   - Run a single Terminal-Bench E3 sample with serial execution and current scoring flags.
   - Require `pair-timing.json`, `sample-timing.json`, aggregate `Timing Summary`, and `score_valid`.
   - Exit early if `score_valid=false`; do not use invalid timing to justify clean-run speedups.
2. Baseline 3-sample/15-task timing from artifacts only.
   - For each sample, calculate agent, public validation, Docker build, Docker run, cleanup, reporting, and total wall time.
   - Produce a compact table sorted by total wall time and by largest non-agent span.
   - Flag repeated Docker cache keys and cache misses.
3. Fix avoidable serial waste first.
   - Enable Docker image cache only for digest-pinned immutable Dockerfiles.
   - Ensure `finalize` is artifact-only and never reruns validators or hidden oracle.
   - Ensure score-invalid runs stop after the first hard engineering-unclean pair in scoring mode.
4. Add resource-governed parallelism only after serial artifacts are clean.
   - Start with sample-level parallelism because it has the least artifact coupling.
   - Add pair-level parallelism only after deterministic pair roots and aggregate merge order are tested.
   - Add validation-level parallelism only after Docker cache locks and Docker concurrency limits are tested.
5. Recalibrate full E3.
   - Compare serial clean baseline to resource-governed parallel run.
   - Keep the scoring profile identical unless the run is explicitly labeled non-comparable.
   - Require `score_ready=true` and `score_valid=true` before claiming score-bearing speedup.

#### Speedup Targets

| Stage | Expected Impact | Acceptance Standard |
|---|---:|---|
| score-invalid fast-fail | prevents multi-hour invalid runs | first hard engineering-unclean pair exits `3` and no later samples run |
| artifact-only finalize | avoids accidental rerun waste | finalize modifies reports/timing only; validation stdout mtime is unchanged |
| Docker cache | removes repeated immutable image builds | second same-scenario validation records `cache_hit=true` |
| pretest timeout split | avoids full validator timeout before tests start | no-marker validator aborts before full test timeout |
| sample-level parallelism | clean-suite wall-time reduction | meets the calibrated wall-time target with deterministic artifacts |
| calibrated parallel profile | larger clean-suite reduction | target `2-3x` only after score validity and resource governor proof |

#### Validation Gate

Do not claim "E3 is faster" unless all of these are true:

- the compared runs use the same scoring profile and task list;
- both runs are `score_ready=true` and `score_valid=true`;
- serial timing artifacts reconcile to wall time within a documented tolerance, and parallel timing reconciles exclusive wall attribution plus critical path;
- Docker cache behavior is explicit in artifacts;
- parallelism settings are recorded;
- no engineering-unclean condition is hidden inside diagnostics.

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
   - any validator timeout is `engineering_unclean`; lifecycle markers distinguish pretest timeout from post-`tests_started` timeout only for diagnosis.
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
- Runtime cache hit behavior is generated-script covered, metadata/invalidation tested, and real-Docker covered by `scripts/taskspace-benchmark/test-terminal-bench-docker-cache-smoke.ps1`. The smoke builds a digest-pinned Terminal-Bench fixture, runs the generated validator twice with `TASKSPACE_DOCKER_IMAGE_CACHE=1`, and requires the second run to record `cache_hit=true`.
- `finalize-taskspace-e3-run.ps1` now performs artifact-only rerender work, writes `finalize-health.json`, rebuilds `sample-timing.json` before aggregate rendering, and records `validation_rerun_allowed=false` / `hidden_oracle_rerun_allowed=false`.
- `run-taskspace-e3-suite.ps1` now records suite score-validity fields and conservative `expected_time_saved_minutes` in `suite-health.json`; skipped samples also get their own `sample-status.json` with `phase=skipped`.
- Suite-level invalidation now emits `suite_score_invalidated` to `events.jsonl` with the invalid sample, reason, child run path, and remaining skipped sample count.
- `invoke-taskspace-e3-start-gate.ps1` now requires `-OnePairSmokeRoot` unless `-AllowSkippedOnePairSmoke` is explicitly set; valid one-pair smoke evidence is either `aggregate.json.score_valid=true`, a classified `invalid_harness` suite health artifact, or a classified `sample-status.json` with a non-empty `abort_signature`. This matters because a scoring-mode one-pair smoke can correctly abort before aggregate generation once the harness proves an engineering-unclean condition.
- No-op finalize behavior is covered by `test-harness.ps1`: the fixture verifies that validation stdout mtime is unchanged after finalize. Parallel execution remains future work until its own tests pass.

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

### 15.9.1 Runtime Speedup Engineering Plan

The speedup work is feasible, but it must be staged. The target is not simply "make E3 faster"; the target is "make clean E3 faster while invalid E3 stops early." These are separate engineering problems.

The immediate engineering question is why a 15-task E3 run takes hours. A valid answer must come from timing artifacts, not from console impressions. The runner must be able to answer, for every run:

- how much wall time was agent execution versus public validation versus Docker build/run versus cleanup/reporting;
- whether slow time happened before benchmark tests started or inside the benchmark tests;
- whether the same task is slow on Standard, TaskSpace, or both;
- whether repeated runs are rebuilding immutable Docker assets unnecessarily;
- whether the run kept scheduling work after it was already score-invalid;
- whether model/API queueing or local Docker/storage contention is the dominant bottleneck.

Observed one-pair smoke evidence already shows that this is not purely "agent thinking time": a single official Terminal-Bench smoke spent roughly 228s in agent execution and roughly 270s in public validation, with Docker run time dominating the validator side. That evidence is not enough to generalize across all 15 tasks, but it is enough to require validator/Docker timing as first-class data before approving any speedup claim.

#### 15.9.1.1 Runtime Bottleneck Model For 15-Task E3

For planning purposes, model a full v0.0.4 E3 run as:

```text
total_wall_time =
  suite_setup
  + sum(samples) {
      sample_preflight
      + sum(repeats) {
          standard_agent
          + standard_public_validation
          + taskspace_agent
          + taskspace_public_validation
          + pair_reporting
      }
      + sample_aggregate
    }
  + suite_aggregate
```

The model must be refined by actual artifacts before implementation decisions:

| Suspected Time Sink | How To Prove | Engineering Response |
|---|---|---|
| agent timeout or long solving | `*_agent_ms`, exec timeout flag, model request timing | keep as benchmark outcome unless the profile is intentionally changed |
| public validation runs for minutes per side | `*_public_validation_ms`, `*_docker_run_ms`, lifecycle markers | split pretest/test timeout, cache immutable setup, parallelize only with Docker tokens |
| repeated Docker image builds | `*_docker_build_ms`, cache key, image digest, cache hit/miss | deterministic Docker cache with immutable input hash |
| invalid run keeps scheduling later samples | first invalid timestamp vs later pair start times | hard circuit breaker and skipped-work records |
| API/model queue wait | model request timing and retry counters | `MaxModelConcurrency`, queue timing, provider error classification |
| local disk or Docker storage pressure | disk preflight snapshots and Docker system df snapshots | disk reservation token and early abort |
| reporting/finalize reruns validators | validation artifact mtimes and finalize spans | artifact-only finalize |

Acceptance for this model:

- `suite-timing.json` reconciles at least `95%` of wall time into named buckets before any speedup result is accepted.
- If `unattributed_ms / wall_time_ms > 5%`, the report must say `runtime_optimization_status=blocked`.
- The report must identify `largest_phase` and `largest_non_agent_phase`; otherwise speed planning is incomplete.
- A 15-task run without complete timing artifacts cannot be used to claim "faster", "slower", or "same speed".
- In parallel mode, timing reconciliation must separate inclusive worker duration from exclusive wall-clock critical path. Inclusive child spans may exceed wall time and must not be used as wall-time savings evidence.

#### Phase S0: Make Runtime Accounting Complete

Objective: explain every long run from artifacts, without reading console logs.

Implementation details:

1. Add `pair-timing.json` for every pair:
   - `pair_id`, `sample_id`, `scenario_id`;
   - `started_at`, `completed_at`, `elapsed_ms`;
   - `standard_agent_ms`, `taskspace_agent_ms`;
   - `standard_public_validation_ms`, `taskspace_public_validation_ms`;
   - `standard_docker_build_ms`, `taskspace_docker_build_ms`;
   - `standard_docker_run_ms`, `taskspace_docker_run_ms`;
   - `standard_cleanup_ms`, `taskspace_cleanup_ms`;
   - `reporting_ms`, `finalize_ms`;
   - `largest_phase`, `largest_non_agent_phase`.
2. Add `sample-timing.json` in each sample root:
   - pair totals;
   - skipped/cancelled pair counts;
   - first invalid-harness timestamp;
   - time spent after first invalid-harness event.
3. Add `suite-timing.json` in the E3 suite root:
   - wall time;
   - sum by phase;
   - top five slowest pairs;
   - bottleneck class;
   - timing fields missing count.
4. Add a `Write-TaskspaceTimingSpan` helper in `scripts/taskspace-benchmark/lib/timing.ps1` instead of open-coded stopwatch fields.
5. Make timing writes atomic, using the existing JSON writer pattern in `run-state.ps1`.
6. Add a timing extractor path for existing artifacts:
   - parse `metrics.json` for exec/public validation durations when present;
   - fall back to process start/end timestamps only with `timing_quality=partial`;
   - record nullable fields with `missing_reason`, not silent `0`.
7. Add stage timing around these runner functions:
   - manifest/materialization;
   - preflight/probe;
   - Standard `Invoke-RealProcess`;
   - Standard public validation;
   - TaskSpace `Invoke-RealProcess`;
   - TaskSpace public validation;
   - hidden oracle if enabled;
   - pair report;
   - aggregate/finalize.
8. Add `runtime-bottleneck.md` beside aggregate report:
   - top slow pairs;
   - phase share table;
   - slowest side per task;
   - hard invalid time saved estimate;
   - blocked fields if timing is incomplete.
9. Store every timing span with interval fields, not only duration:
   - `span_id`, `parent_span_id`;
   - `worker_id`;
   - `resource_kind`;
   - `started_at`, `completed_at`, `duration_ms`;
   - `wait_ms` and `run_ms` where resource tokens are used.
10. Add parallel timing reducers:
   - `inclusive_duration_ms` for total work performed;
   - `exclusive_wall_ms` for non-overlapped wall attribution;
   - `critical_path_ms` for the longest dependency path;
   - `resource_wait_ms` for time blocked on Docker/model/disk/validation tokens.

Acceptance:

- A synthetic three-pair fixture produces deterministic `suite-timing.json`.
- Missing timing fields make runtime optimization status `blocked`, not `pass`.
- Timing subtotal difference from suite wall time is reported as `unattributed_ms`.
- `unattributed_ms` must be below a documented tolerance before using the run as speed evidence.
- One official one-pair smoke must produce `pair-timing.json`, `sample-timing.json`, and `runtime-bottleneck.md`.
- The timing report must distinguish `agent_duration_ms` from `public_validation_duration_ms`.
- The timing report must classify a validator-dominant pair as `validator_bound`, not `agent_bound`.
- A parallel overlap fixture must prove that inclusive child totals can exceed wall time while `exclusive_wall_ms` still reconciles to suite wall time.

#### Phase S1: Stop Wasting Time On Invalid Runs

Objective: once E3 is score-invalid for a hard engineering reason, stop scheduling new scoring work.

Implementation details:

1. Normalize scoring-mode invalid harness exits to process exit `3`:
   - suite runner;
   - external benchmark runner;
   - sample runner;
   - finalize/report rerun paths.
2. Add a suite circuit breaker:
   - trigger on `run_validity=invalid_harness`;
   - trigger on hard failure classes except allowed `exec_timeout`;
   - stop new pairs/samples immediately;
   - write `cancelled_due_to_score_invalid` for work not started.
3. Preserve already-written evidence:
   - do not delete partial artifacts;
   - do not rebuild aggregate as if the run were score-bearing;
   - write `sample-status.json` and `suite-health.json`.
4. Ensure final reports use "invalid execution" language and never report Standard/TaskSpace score deltas from invalid runs.
5. Treat audit lifecycle separately from engineering unclean:
   - `e3_human_review_not_completed` is `audit_required`, not a hard engineering failure;
   - it must block final score publication;
   - it must not stop the suite before all executable pairs run;
   - real audit corruption such as mismatched hashes or malformed review remains a hard score-validity failure.
6. Add skipped-work accounting:
   - `skipped_sample_count`;
   - `skipped_pair_count`;
   - `cancelled_after_invalid_ms`;
   - `estimated_time_saved_ms` computed from completed-pair median, marked as estimate.
7. Add explicit allowed-unexpected list:
   - agent `exec_timeout` is an allowed benchmark outcome;
   - validator timeout before or after `tests_started` is engineering-unclean; lifecycle proof only determines whether the stable reason is `validator_pretest_timeout` or `public_validation_timeout_after_tests_started`;
   - Docker build/run failure, missing source, path failure, no lifecycle marker, cleanup failure, and storage exhaustion are hard engineering unclean.

Acceptance:

- Injected Docker run failure exits `3`.
- The suite writes `suite-health.json.run_validity=invalid_harness`.
- No later sample starts after the first hard invalid-harness event.
- Final report says the score is invalid and points to abort artifacts.
- A pure `audit_required` run exits with a resumable/audit-required status and does not claim a score.
- A hard engineering failure after pair 1 prevents pair 2 from starting in scoring mode.
- The suite report shows how much work was skipped and why.
- Pretest validator timeout and post-`tests_started` validator timeout are both score-invalid engineering failures, with different reason codes for diagnosis.

#### Phase S2: Remove Avoidable Validator And Docker Overhead

Objective: reduce non-agent time without changing the benchmark task or scoring semantics.

Implementation details:

1. Split validation timeout:
   - `ValidationPretestTimeoutSeconds` covers setup before `tests_started`;
   - `ValidationTestTimeoutSeconds` covers actual tests after marker;
   - no-marker failures abort with `no_tests_started_marker`.
2. Cache Docker images only when inputs are immutable:
   - cache key includes Dockerfile hash, validator script hash, source version, uv cache hash, and benchmark adapter version;
   - digest-pinned base images are cache-eligible;
   - mutable tags require opt-in diagnostic mode or cache disabled.
3. Add Docker cache locks:
   - one writer per cache key;
   - concurrent readers wait for writer completion;
   - failed build invalidates that cache entry.
4. Bound cleanup:
   - cleanup timeout has its own field;
   - cleanup failures classify as engineering-unclean;
   - cleanup cannot hide validation results.
5. Make finalize artifact-only:
   - no validator rerun;
   - no hidden oracle rerun;
   - only aggregate, markdown, and health files are regenerated.
6. Add validation-stage reuse rules:
   - public validation outputs are immutable once `validation_completed=true`;
   - rerun/finalize may read them but cannot overwrite them unless `-ForceRerunValidation` is explicitly passed;
   - every reuse must record source artifact hash and mtime.
7. Add Docker/system preflight timing:
   - Docker daemon availability check;
   - `docker system df` snapshot;
   - free disk snapshot before sample scheduling and before each Docker-heavy phase;
   - fail before scheduling new workers if disk reserve is below threshold.
8. Split validator shell script normalization from validator execution:
   - normalization happens at materialization time;
   - normalized script hash is included in cache key;
   - runtime validator does not mutate source scripts.

Acceptance:

- Second run of the same immutable scenario records `cache_hit=true`.
- A mutable Dockerfile does not use cache in scoring mode.
- A no-marker validator aborts before full validation timeout.
- Finalize does not change validation stdout/stderr mtimes.
- A disk-reserve failure aborts before starting a new Docker build/run.
- Validator script normalization is covered by a BOM/CRLF fixture and does not change test semantics.
- Docker cache hit/miss appears in `pair-timing.json` and `runtime-bottleneck.md`.

#### Phase S3: Add Resource-Governed Parallelism

Objective: reduce clean full-suite wall time after serial mode is already clean.

Implementation details:

1. Keep defaults serial:
   - `MaxParallelSamples=1`;
   - `MaxParallelPairsPerSample=1`;
   - `MaxParallelValidationsPerPair=1`;
   - `MaxParallelSidesPerPair=1`.
2. Implement resource tokens before worker jobs:
   - Docker token;
   - model token;
   - validation token;
   - disk reservation token;
   - Docker cache-key lock.
3. Start with sample-level parallelism:
   - independent sample roots;
   - deterministic merge order by sample ID;
   - no shared current directory.
4. Add pair-level parallelism only after sample parallelism passes:
   - deterministic pair roots;
   - no shared temporary files;
   - pair-local stdout/stderr only.
5. Add validation-level parallelism only after Docker cache locks pass:
   - Standard and TaskSpace validations can overlap only if Docker concurrency permits;
   - both sides must record independent timing and proof markers.
6. Keep side-agent parallelism disabled for scoring comparison unless the report explicitly sets `timing_comparison_valid=false`.
7. Implement parallel execution as a suite-driver concern first:
   - worker jobs call the existing sample runner with isolated run roots;
   - parent process owns scheduling, cancellation, status merge, and final report;
   - child processes never write directly into shared aggregate files.
8. Add resource budget defaults:
   - `MaxParallelSamples=1` for scoring until accepted;
   - first experimental profile uses `MaxParallelSamples=2`;
   - `MaxDockerConcurrency=1` until cache locks pass;
   - `MaxModelConcurrency=1` until provider queue timing is observable.
9. Add deterministic merge:
   - sort samples by task ID;
   - sort repeats numerically;
   - write merge manifest with child run IDs and hashes.
10. Add cancellation protocol:
   - when parent detects hard invalid harness, it stops scheduling new jobs;
   - running jobs are allowed to finish unless they are in pre-agent/probe stage;
   - cancelled jobs write `cancelled_due_to_score_invalid`.

Acceptance:

- `parallelism.json` records configured and observed concurrency.
- Parallel smoke has no duplicate artifact paths.
- Serial and parallel runs have equivalent score readiness, score validity, inclusion/exclusion rows, hard outcome classification, and audit/proof status.
- The first accepted full parallel profile reports observed wall-time reduction; it only passes the speed target when calibration says `>=30%` is plausible for the observed bottleneck class.
- Parent suite report can be rebuilt from child manifests without rerunning children.
- Killing or cancelling one child leaves the suite status explicit, not silently incomplete.
- Parallel mode cannot be enabled unless disk and Docker reserve checks pass.
- Parallel timing report includes inclusive duration, exclusive wall attribution, critical path, and resource wait attribution.

#### Phase S4: Decide Whether Agent-Side Speed Changes Are Allowed

Objective: avoid "speedups" that are actually weaker TaskSpace capability.

Implementation details:

1. Treat model, reasoning effort, timeout, subagent count, and tool-call budget as scoring profile fields.
2. Do not change these in the v0.0.4 comparable score profile.
3. If a faster agent profile is tested, mark it diagnostic-only until promoted to a new benchmark profile.
4. Report capability tradeoffs separately from harness speedups.

Acceptance:

- v0.0.4 comparable run has identical score profile fields before and after speed work.
- Diagnostic speed runs cannot be merged into score comparison tables.
- Any adopted cheaper profile gets a new profile ID and a fresh baseline.

#### Phase S5: Calibrate Full 15-Task Runtime And Decide Speed Target

Objective: answer whether E3 can be made dramatically faster with evidence, and choose the next engineering target.

Implementation details:

1. Run a serial clean calibration only after hard engineering-unclean issues are fixed:
   - same task list;
   - same model/profile;
   - `MaxParallel*=1`;
   - complete timing artifacts required.
2. Compute baseline:
   - total wall time;
   - median/p95 per-pair time;
   - agent share;
   - validator share;
   - Docker build/run share;
   - cleanup/reporting share;
   - invalid/audit/timeout counts.
3. Run one controlled parallel smoke:
   - start with `MaxParallelSamples=2`;
   - keep Docker and model concurrency at `1` unless resource tokens prove safe;
   - compare score validity and artifact hashes against serial run where deterministic.
4. Decide the next target from bottleneck class:
   - if validator/Docker bound, prioritize Docker cache and validation parallelism;
   - if agent bound, only pursue agent profile changes as a new benchmark profile;
   - if storage bound, prioritize cleanup and disk reservation;
   - if API queue bound, prioritize model concurrency controls and queue telemetry.

Acceptance:

- The calibration report states whether `>=30%` speedup is currently plausible and why.
- Any `2-3x` claim must have both a serial baseline and a parallel smoke artifact.
- If the bottleneck is agent time, the plan must say v0.0.4 comparable score cannot be sped up substantially without changing the scoring profile.
- If the bottleneck is validator/Docker time, the plan must name the exact cache/parallelism work item that unlocks the next speed target.
- If calibration says `>=30%` is not plausible for the observed bottleneck, the phase exits with a smaller evidence-backed target instead of failing the plan.

#### Expected Speedup Bound

Use these estimates only after clean timing artifacts exist:

| Optimization | Plausible impact | Why |
|---|---:|---|
| invalid-run fast-fail | hours saved on bad runs | stops after first hard engineering defect instead of consuming all 15 pairs |
| Docker cache | medium to high | avoids repeated image builds for immutable scenarios |
| pretest timeout split | high on broken validators | no-marker/setup failures do not wait for full test timeout |
| sample-level parallelism | `1.5-2x` initial target | independent sample roots have low coupling |
| pair/validation parallelism | additional `1.2-1.5x` | useful after Docker contention is governed |
| agent budget reduction | unknown and non-comparable | may reduce capability, so not a v0.0.4 harness speedup |

Do not promise `2-3x` until a serial clean baseline and a resource-governed parallel smoke both pass. A realistic first milestone is a calibration-backed clean wall-time reduction target, often around `>=30%` only when validator/Docker or sample scheduling is the measured bottleneck.

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
| full clean calibrated run | selected concurrency profile | faster than serial if bottleneck allows | evidence-backed wall-time reduction target, `score_ready=true`, `score_valid=true`, no inclusion/exclusion drift |

### 15.11 Required Commands Before Full Parallel E3

```powershell
.\scripts\taskspace-benchmark\test-e3-score-validity.ps1
.\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1
.\scripts\taskspace-benchmark\test-e3-proof-harness.ps1
.\scripts\taskspace-benchmark\test-harness.ps1
.\scripts\taskspace-benchmark\test-terminal-bench-docker-cache-smoke.ps1
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

Planned first safe parallel smoke after Phase R3 implements the flags:

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
  -MaxParallelSamples 2 `
  -MaxDockerConcurrency 1 `
  -MaxModelConcurrency 1 `
  -SandboxMode full-auto `
  -ConfigOverride 'model_reasoning_effort="max"' `
  -ScoringMode
```

Parallel execution flags such as `-MaxParallelSamples`, `-MaxDockerConcurrency`, and `-MaxModelConcurrency` are planned runner contract, not current suite-runner CLI. The command above is the post-R3 target command shape. Do not add these flags to production E3 commands until Phase R3 implements them and `parallelism.json.observed.max_parallel_samples=2` is proven by smoke artifacts.

Do not run full parallel E3 until the smoke artifacts prove:

- no pair/sample path collisions;
- no Docker cache race;
- no missing timing spans;
- no score readiness, score-validity, inclusion/exclusion, hard-outcome, audit, or proof mismatch between serial and parallel mode;
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
- clean full E3: calibration-backed speedup first, then calibrated `2-3x` only after resource-governed parallelism proves stable and bottlenecks support it.

## 16. Integrated Runtime Bottleneck And Speedup Plan

This section turns the runtime concern into a hard execution plan. It is part of the same E3 guardrail work, not a separate optimization project, because a slow invalid run is itself a guardrail failure.

Section 16 is the canonical runtime implementation sequence for work after version `0.5` of this plan. Earlier runtime notes in section 15 remain useful design background, but they are superseded wherever they conflict with the `P0` to `P5` order, gates, and acceptance criteria below. Engineers implementing runtime work must follow section 16 first and use section 15 only as supporting rationale.

### 16.1 Runtime Problem Statement

A 15-task E3 run taking several hours is not automatically wrong, but it is currently unexplained enough to be operationally unsafe. The runner must stop making operators infer bottlenecks from console output.

The current known timing evidence from the official one-pair smoke is:

| Evidence | Observed Signal | Planning Meaning |
|---|---|---|
| total one-pair wall time | about 575s | one pair can already consume close to 10 minutes |
| agent duration | about 225s | agent time is material but not the whole run |
| public validation duration | about 324s | validator/Docker time can exceed agent time |
| Docker run duration | about 56s in recorded Docker spans | Docker is a real sub-bottleneck but the validation wrapper can spend additional time outside the recorded Docker span |
| TaskSpace side | agent timed out, then validation also timed out | current runner can waste validator time after an agent execution timeout |

This evidence is enough to block blind reruns. It is not enough to claim a full-suite bottleneck or a speedup target. The first deliverable must be complete timing attribution.

### 16.2 Hard Runtime Questions The Plan Must Answer

Before another full 15-task E3 is treated as useful, the harness must answer these questions from artifacts:

1. How much wall time is spent in agent execution, public validation, hidden oracle, Docker build, Docker run, cleanup, reporting, and waiting for resources?
2. How much time is spent after the run is already score-invalid?
3. Does agent execution timeout cause avoidable downstream validator timeout?
4. Are Docker images rebuilt when the immutable input hash is unchanged?
5. Are Standard and TaskSpace slow on the same tasks, or is one profile creating more expensive states?
6. Is the bottleneck local Docker/storage, model/API queueing, benchmark tests, or Whale agent time?
7. Does any proposed parallelism preserve score validity, artifact determinism, and side comparability?

If any question cannot be answered, runtime optimization status is `blocked`.

### 16.3 Phase P0: Timing Evidence Before More Full Runs

#### Objective

Make every long E3 run explainable from machine-readable timing artifacts.

#### Entry Criteria

- Current hard clean-execution taxonomy is present.
- `pair-timing.json` and `sample-timing.json` exist for the latest one-pair smoke or can be generated from existing run artifacts.
- No full 15-task speed claim is being made from partial console evidence.

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| timing artifacts are emitted | run one official one-pair smoke | `pair-timing.json`, `sample-timing.json` | harness |
| timing fields distinguish agent and validator | inspect JSON fields | `agent_duration_ms`, `public_validation_duration_ms` | harness |
| missing timing is explicit | synthetic missing-field fixture | `timing_quality=partial` or `runtime_optimization_status=blocked` | harness |

#### Design Approach

Use additive timing artifacts. Do not infer performance from markdown reports. The timing writer must record intervals, not only durations, so future parallel runs can distinguish inclusive child work from exclusive wall-clock savings.

#### Implementation Tasks

1. Introduce or complete `scripts/taskspace-benchmark/lib/timing.ps1`.
2. Add one shared `Write-TaskspaceTimingSpan` helper with atomic JSON writes.
3. Wrap agent execution, public validation, hidden oracle, Docker build/run, cleanup, pair report, aggregate report, preflight, and probe.
4. Write `pair-timing.json` for each pair and `sample-timing.json` for each sample.
5. Add suite-level `suite-timing.json` for multi-sample wrapper runs.
6. Add `runtime-bottleneck.md` beside aggregate output, generated only from timing JSON.
7. Add required wait and queue spans:
   - `model_queue_wait_ms`;
   - `model_retry_backoff_ms`;
   - `model_request_duration_ms`;
   - `process_launch_wait_ms`;
   - `docker_token_wait_ms`;
   - `validation_token_wait_ms`;
   - `disk_reservation_wait_ms`;
   - `cache_lock_wait_ms`;
   - `resource_wait_ms_total`.
8. Add a reducer that computes:
   - wall time;
   - named phase totals;
   - largest phase;
   - largest non-agent phase;
   - top slow pairs;
   - unattributed time;
   - time after first hard invalid event.
9. Add `bottleneck_classification=unknown` when model/API/resource wait spans are unavailable and any unexplained time exceeds tolerance.

#### Deliverables

- `lib/timing.ps1`
- timing JSON artifacts
- `runtime-bottleneck.md`
- synthetic timing reducer tests

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| synthetic timing fixture | unit-style PowerShell test | deterministic phase totals |
| missing timing fixture | unit-style PowerShell test | status is `blocked`, not success |
| official one-pair smoke | one materialized Terminal-Bench scenario | timing artifacts exist and identify largest phase |
| reconciliation | reducer check | `unattributed_ms / wall_time_ms <= 5%` or blocked with reason |
| model/API wait timing | synthetic timing fixture | queue, retry backoff, request duration, and resource wait fields are present or explicitly unavailable |
| unknown bottleneck guard | synthetic timing fixture | missing wait spans produce `runtime_optimization_status=blocked` |

#### Exit Criteria

- Runtime bottleneck is visible without reading console output.
- A full run without complete timing is not allowed to produce a speed conclusion.
- Unknown model/API/resource wait attribution blocks speed claims.
- The next phase can decide whether time is being wasted after invalidation.

#### Review Plan

Review the timing schema before implementation and review the first official smoke artifacts after implementation. The review must check whether reported totals reconcile to wall time and whether nullable fields have explicit reasons.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| timing fields are silently zero | false bottleneck | zero duration with no skip reason | nullable fields plus `missing_reason` | block speed report |
| interval overlap is double-counted | fake speedup | child totals exceed wall time with no inclusive/exclusive split | store span parent and worker IDs | serial-only timing |
| report code becomes another slow step | extra runtime | reporting duration grows materially | time report generation itself | write JSON only, skip markdown |

#### Gate To Next Phase

Proceed only when the one-pair smoke and synthetic timing fixture pass.

### 16.4 Phase P1: Stop Validator Waste After Agent Timeout

#### Objective

Prevent an allowed agent execution timeout from cascading into avoidable public validation timeout and multi-minute wasted runtime.

#### Entry Criteria

- Phase P0 timing artifacts can show agent timeout and validator time separately.
- Failure taxonomy already treats `agent_exec_timeout` as the only allowed unexpected benchmark timeout.
- Validator timeout remains engineering-unclean.
- A pre-agent validator/proof probe contract exists and can produce a pass/fail artifact before agent execution.

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| exec timeout is detectable before validation | inspect side metrics after agent run | `exec_timed_out=true` | runner |
| validation can be skipped explicitly | fixture or dry run | `public_validation_skipped=true` | runner |
| pre-agent proof passed before skip | inspect proof artifact | `pre_agent_validator_probe_status=passed` and `validator_equivalence_ok=true` | proof |
| skipped validation is not counted as pass | score validity fixture | side outcome is `agent_exec_timeout`, not success | taxonomy |

#### Design Approach

If a side's agent execution times out, the runner should not run expensive public validation on an incomplete workspace by default. It should classify that side as `agent_exec_timeout`, write an explicit validation skip record, and continue only the work needed to preserve harness proof and pair-level accounting.

This skip is score-clean only when the pre-agent validator/proof probe passed before agent execution. If the probe is missing, failed, stale, or hash-mismatched, the side becomes `engineering_unclean` with `pre_agent_validator_probe_missing_or_failed`; the runner must not silently convert it into a clean agent timeout. This does not relax validator timeout policy; it avoids creating validator timeout from a known incomplete agent attempt after the validator environment has already been proven.

#### Implementation Tasks

1. Add side-level validation skip contract:
   - `public_validation_skipped=true`;
   - `public_validation_skip_reason=agent_exec_timeout`;
   - `public_validation_exit_code=null`;
   - `validator_environment_failures=[]`;
   - `business_success=false`;
   - `validation_timing_ms=0`;
   - `validation_skip_recorded_at`;
   - `pre_agent_validator_probe_status=passed`;
   - `pre_agent_validator_probe_hash`;
   - `validator_equivalence_ok=true`.
2. In `run-taskspace-benchmark.ps1`, branch after `Invoke-TaskspaceAgentRun` and before public validation.
3. Ensure hidden oracle is also skipped for the timed-out side unless an explicit diagnostic flag requests it.
4. Preserve pair-level proof:
   - preflight/probe proof still runs before agent execution;
   - skipped validation must not mark validator fidelity unproven only if pre-agent proof already established equivalence;
   - missing or failed pre-agent proof makes the skipped side engineering-unclean;
   - skipped validation must not fabricate runtime proof.
5. Update `Get-TaskspaceAgentOutcome` and aggregate paths so `agent_exec_timeout` is score-bearing as a timeout outcome, while the run remains clean if no other engineering issue exists.
6. Add `-ValidateAfterAgentTimeout` as diagnostic-only if operators need forensic validator output; scoring mode default must be skip.

#### Deliverables

- runner branch for timed-out agent side
- metrics schema fields for validation skip
- taxonomy tests for clean `agent_exec_timeout`
- timing report that counts saved validator time

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| timeout side fixture | synthetic metrics | side outcome is `agent_exec_timeout` |
| validation skip fixture | runner-level dry fixture | no public validation process starts |
| missing pre-agent probe fixture | score validity fixture | skipped validation becomes `engineering_unclean`, not clean timeout |
| failed pre-agent probe fixture | score validity fixture | `pre_agent_validator_probe_missing_or_failed` blocks score validity |
| non-timeout agent failure fixture | runner-level fixture | validation still runs normally |
| score validity fixture | `test-e3-score-validity.ps1` | no `public_validation_timeout` reason appears |
| official one-pair smoke with short agent timeout | real Terminal-Bench scenario | timed-out side skips validator, run is not invalid solely from downstream validator timeout |

#### Exit Criteria

- Agent timeout no longer causes avoidable validator timeout.
- Skipped validation is score-clean only with a passed, current pre-agent validator/proof probe.
- Runtime report shows estimated validator time saved for skipped timed-out sides.
- Any actual validator timeout that occurs when validation is legitimately run still invalidates the execution.

#### Review Plan

Review must focus on whether this incorrectly hides real validator failures. The acceptance evidence must show that validation is skipped only after `exec_timed_out=true`, and that non-timeout agent failures still run validation normally.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| skip hides a validator infrastructure defect | false clean run | validator was never probed before agent | require pre-agent probe/proof before skip can be clean | mark skipped side `engineering_unclean` |
| timeout side becomes incomparable | unclear score | aggregate treats timeout as missing | make `agent_exec_timeout` explicit score-bearing outcome | exclude pair with reason |
| diagnostic needs validator output | less forensic data | operator requests deeper debug | `-ValidateAfterAgentTimeout` diagnostic-only | rerun single pair diagnostic |

#### Gate To Next Phase

Proceed only when a timed-out agent side can finish without creating `public_validation_timeout` and without weakening proof gates.

### 16.5 Phase P2: Fast-Fail Invalid Full Runs

#### Objective

Stop scheduling new E3 work after the first hard engineering-unclean condition makes the score invalid.

#### Entry Criteria

- Phase P0 timing can measure time after invalidation.
- Phase P1 prevents agent timeout from producing avoidable validator timeout.
- Failure taxonomy has a complete hard engineering-unclean list.

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| hard invalid reason is stable | taxonomy tests | stable reason code | taxonomy |
| suite can cancel future work | suite fixture | skipped pair/sample records | suite driver |
| audit pending is not hard invalid | score validity tests | `audit_required`, not `engineering_unclean` | aggregate |

#### Design Approach

Fast-fail applies only to hard engineering-unclean conditions. It must not stop because of `audit_required`, `agent_exec_timeout`, or ordinary agent wrong answers. The cancellation must be explicit: skipped samples and pairs are artifacts, not missing outputs.

#### Implementation Tasks

1. Add suite-level scoring invalid flag and first-invalid timestamp.
2. Stop scheduling new samples/pairs when a hard invalid reason appears.
3. Allow currently running child jobs to finish only if killing them would corrupt artifacts; otherwise cancel before agent execution.
4. Write `suite-health.json`, skipped records, and `abort-summary.md`.
5. Add `estimated_time_saved_ms` using completed clean pair median and mark it as estimate.
6. Ensure aggregate disables all Standard vs TaskSpace score language when score is invalid.

#### Deliverables

- suite circuit breaker
- skipped-work artifacts
- invalid-run aggregate language
- fast-fail tests

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| injected Docker failure | synthetic runner fixture | exits `3`, no later sample starts |
| repeated infra signature | two-sample fixture | suite aborts after second same hard signature |
| audit-only run | fixture | run remains executable and score publication waits for audit |
| official smoke invalid case | one real sample | abort artifacts are complete |

#### Exit Criteria

- Invalid scoring runs no longer consume all 15 tasks.
- Reports make invalidity impossible to confuse with model performance.

#### Review Plan

Review skipped-work records and aggregate output. Confirm that no score/better/worse field is populated when `score_valid=false`.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| fast-fail stops useful diagnostic batch | less debug evidence | operator needs all failures | explicit diagnostic mode disables scoring fast-fail | diagnostic-only rerun |
| cancellation leaves partial state ambiguous | confusing resume | missing status files | write status before cancellation | force rerun sample |
| invalid reason too broad | excessive aborts | agent wrong answer aborts suite | taxonomy fixture for each allowed outcome | serial no-fast-fail mode |

#### Gate To Next Phase

Proceed only after a hard invalid fixture proves that later work is skipped and score output is suppressed.

### 16.6 Phase P3: Remove Avoidable Docker And Validator Overhead

#### Objective

Reduce non-agent runtime while preserving external benchmark semantics.

#### Entry Criteria

- Phase P0 identifies validator/Docker time as a material bottleneck.
- Phase P2 prevents invalid runs from wasting full-suite time.
- Docker disk preflight is present.

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Docker build/run timings exist | timing JSON | build/run fields per side | harness |
| immutable input hash exists | adapter proof | Dockerfile, validator, source, uv cache hashes | adapter |
| disk reserve check exists | preflight artifact | free space and threshold | harness |

#### Design Approach

Only cache immutable validator assets. Scoring mode must prefer correctness over speed. Mutable Dockerfiles, unpinned source, or unknown uv cache state must disable cache and mark the reason.

#### Implementation Tasks

1. Add Docker cache key:
   - Dockerfile hash;
   - validator script hash;
   - normalized run-tests hash;
   - Terminal-Bench source version;
   - uv cache hash;
   - adapter version;
   - resolved base image digest, not only the base tag;
   - full Docker build context hash after `.dockerignore` application;
   - platform/architecture;
   - Docker engine version and buildx/backend mode when it can affect output;
   - remote package lockfiles and package manager metadata used during build;
   - network-disabled or network-allowed mode;
   - environment variables passed into build or run.
2. Add cache manifest with image digest, created time, source hashes, and scoring eligibility.
3. Add per-cache-key build lock.
4. Split validation timeout into pretest and test timeout.
5. Abort no-marker validators on pretest timeout instead of full test timeout.
6. Make finalize artifact-only and forbid validator reruns unless `-ForceRerunValidation` is passed.
7. Record cache hit/miss and reuse reason in timing artifacts.

#### Deliverables

- Docker cache manifest
- cache lock implementation
- pretest/test timeout split
- artifact-only finalize guard
- cache hit/miss timing fields

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| cache hit | run same immutable scenario twice | second run records `cache_hit=true` |
| mutable source | fixture with mutable tag/input | cache disabled with reason |
| base image drift | fixture with changed resolved digest | cache miss or score-ineligible cache |
| build context drift | fixture with changed context file | cache miss |
| remote package drift | fixture with package metadata change | cache miss or scoring cache disabled |
| platform drift | fixture with changed platform/arch | cache miss |
| uv cache drift | fixture with changed uv cache hash | cache miss |
| no-marker validator | fixture | aborts at pretest timeout |
| finalize | mtime check | validation stdout/stderr unchanged |
| disk reserve | low-threshold fixture | abort before Docker build/run |

#### Exit Criteria

- Repeated immutable tasks avoid rebuild work.
- Scoring cache is disabled unless the full immutable build context and resolved base inputs are proven.
- Broken setup fails before full validation timeout.
- Finalize cannot accidentally rerun expensive validators.

#### Review Plan

Review cache key construction and proof artifacts. Reject the phase if any mutable input can hit scoring cache without an explicit opt-in diagnostic mode.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| stale Docker cache | false pass/fail | source hash mismatch | full hash key and digest check | rebuild and mark cache invalid |
| cache locks deadlock | stuck run | lock wait exceeds threshold | lock timeout with owner metadata | disable cache |
| pretest timeout too short | false infra failure | legitimate setup exceeds threshold | calibrate from clean timing | raise pretest timeout with evidence |

#### Gate To Next Phase

Proceed only when immutable cache hit and no-marker timeout fixtures pass.

### 16.7 Phase P4: Resource-Governed Parallelism

#### Objective

Reduce clean full-suite wall time with controlled parallelism after serial correctness and timing are proven.

#### Entry Criteria

- Serial one-pair smoke is clean or cleanly classified.
- Full serial calibration has complete timing.
- Docker/cache/disk preflight passes.
- Score validity and inclusion/exclusion rules are stable.

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| serial baseline exists | full or representative serial run | `suite-timing.json` | benchmark owner |
| resource tokens exist | unit test | Docker/model/disk token acquisition and release | suite driver |
| deterministic merge exists | parallel fixture | stable aggregate ordering and hashes | suite driver |

#### Design Approach

Parallelism belongs in the suite driver. Child runs write isolated artifacts; the parent owns scheduling, cancellation, merge, and final reports. Defaults remain serial until the parallel smoke proves no score drift.

#### Implementation Tasks

1. Add configuration:
   - `MaxParallelSamples`;
   - `MaxParallelPairsPerSample`;
   - `MaxParallelValidationsPerPair`;
   - `MaxDockerConcurrency`;
   - `MaxModelConcurrency`;
   - `DiskReserveGb`.
2. Implement resource token manager.
3. Start with `MaxParallelSamples=2`, `MaxDockerConcurrency=1`, `MaxModelConcurrency=1`.
4. Keep Standard and TaskSpace agent execution serial within a pair for comparable scoring.
5. Write `parallelism.json` with configured and observed concurrency.
6. Merge child artifacts deterministically by sample ID and pair ID.
7. Compare serial and parallel output fields:
   - `score_ready`;
   - `score_valid`;
   - Standard and TaskSpace score values;
   - per-task and per-pair outcomes;
   - pass/fail/timeout counts;
   - better/worse/tie counts;
   - inclusion/exclusion counts;
   - hard outcome counts;
   - audit status;
   - proof status;
   - model ID, reasoning effort, timeout, sandbox, and tool budget;
   - prompt/config hash;
   - profile hash;
   - scenario input hash;
   - validator/proof artifact hashes;
   - pair ordering and deterministic merge manifest.

#### Deliverables

- suite-level worker scheduler
- resource token manager
- `parallelism.json`
- deterministic merge manifest
- serial-vs-parallel comparison check

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| path isolation | two-worker fixture | no duplicate artifact paths |
| Docker token | controlled parallel fixture | observed Docker concurrency does not exceed config |
| deterministic merge | repeat fixture | aggregate order and hashes stable |
| parallel smoke | `MaxParallelSamples=2` | no score validity, outcome, score value, profile/config hash, or inclusion drift |
| serial-vs-parallel score diff | comparison fixture | per-task outcomes, pair rows, score values, profile hashes, prompt/config hashes, artifact hashes, and ordering match expected equivalence rules |
| cancellation | injected invalid while worker pending | pending work is skipped explicitly |

#### Exit Criteria

- First parallel smoke reports observed wall-time reduction and no scoring drift.
- Serial-vs-parallel comparison proves no actual result drift, not only no metadata drift.
- Parallel mode remains opt-in until a full calibrated run passes.

#### Review Plan

Review must compare serial and parallel artifacts field by field. Any missing artifact, changed score validity, or changed inclusion reason blocks rollout.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| parallelism creates path collisions | invalid evidence | duplicate path/hash conflict | isolated child roots | serial mode |
| model queueing erases speedup | no wall-time gain | high `resource_wait_ms` | lower `MaxModelConcurrency` and record queue time | serial model token |
| Docker contention causes failures | engineering unclean | Docker run/build failures under parallelism | keep Docker concurrency 1 first | serial validation |
| disk fills faster | system instability | reserve check fails | disk token and cleanup gate | abort scheduling |

#### Gate To Next Phase

Proceed only when serial-vs-parallel comparison shows no scoring drift and timing proves actual wall-time reduction.

### 16.8 Phase P5: Speed Target Decision

#### Objective

Answer whether E3 can be made substantially faster, with a defensible target.

#### Entry Criteria

- Phase P0-P4 validation has passed or produced explicit blocked reasons.
- A serial baseline and at least one governed parallel smoke exist.
- All timing artifacts reconcile.

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| serial baseline complete | inspect `suite-timing.json` | `timing_quality=complete` | benchmark owner |
| parallel smoke comparable | serial-vs-parallel diff | no scoring drift | benchmark owner |
| bottleneck class stable | timing reducer | named bottleneck with share | benchmark owner |

#### Design Approach

The speed target is selected from evidence, not from aspiration. If the bottleneck is agent solving, the comparable v0.0.4 profile cannot be made dramatically faster without changing the benchmark profile. If the bottleneck is validator/Docker/scheduling, speedup is a harness engineering target.

#### Implementation Tasks

1. Generate `runtime-calibration-report.md`.
2. Classify the dominant bottleneck:
   - `agent_bound`;
   - `validator_bound`;
   - `docker_build_bound`;
   - `docker_run_bound`;
   - `storage_bound`;
   - `model_queue_bound`;
   - `invalid_waste_bound`;
   - `mixed`.
3. Compute plausible speedup from observed shares:
   - fast-fail savings for invalid runs;
   - cache savings for immutable Docker work;
   - sample parallelism savings from observed critical path;
   - validation parallelism savings only if Docker tokens permit.
4. Publish one of three decisions:
   - `speedup_target_approved`;
   - `speedup_target_limited`;
   - `speedup_blocked`.
5. If approved, set the next target:
   - first milestone usually `>=30%` clean wall-time reduction;
   - `2-3x` only after serial baseline plus full governed parallel run supports it.

#### Deliverables

- `runtime-calibration-report.md`
- bottleneck classification
- approved or blocked speed target
- next implementation issue list

#### Testing And Validation

| Validation Item | Method | Passing Standard |
|---|---|---|
| report completeness | artifact review | every major phase has duration and share |
| claim guard | report fixture | no `2-3x` claim without required evidence |
| agent-bound case | synthetic timing fixture | target limited or blocked |
| validator-bound case | synthetic timing fixture | cache/parallel work item named |

#### Exit Criteria

- The project has a defensible answer to "why does 15-task E3 take hours?"
- The project has a defensible answer to "can it be made much faster?"
- Any speed claim names the bottleneck, evidence, target, and constraints.

#### Review Plan

Run adversarial review against the calibration report before using it to justify a full E3 rerun or performance claim.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| speed target is based on invalid run | misleading plan | `score_valid=false` baseline | separate invalid fast-fail from clean speed | rerun clean calibration |
| benchmark profile changes | incomparable result | model/timeout/reasoning fields changed | profile hash in report | new profile baseline |
| optimistic parallel extrapolation | missed target | parallel smoke not representative | require full governed run before strong claim | smaller target |

#### Gate To Next Phase

Do not start a full optimized E3 until this phase has either approved a target with evidence or explicitly blocked speed claims.

### 16.9 Runtime Execution Order

The implementation order is mandatory:

1. Fix hard engineering-unclean causes that make current E3 invalid.
2. Implement P0 complete timing.
3. Implement P1 skip validation after agent timeout.
4. Implement P2 fast-fail invalid full runs.
5. Run one official one-pair smoke and inspect timing.
6. Implement P3 Docker/validator overhead reduction only if timing says it matters.
7. Run serial calibration.
8. Implement P4 resource-governed parallelism.
9. Run parallel smoke.
10. Complete P5 speed target decision.
11. Only then run full optimized E3.

### 16.10 Runtime Plan Acceptance Checklist

- [ ] Every major runtime phase has a named timing field.
- [ ] `agent_exec_timeout` does not create downstream `public_validation_timeout` by default.
- [ ] Validator timeout remains engineering-unclean when validation actually runs.
- [ ] Invalid scoring runs stop scheduling new work.
- [ ] Audit-required runs are not confused with engineering-unclean runs.
- [ ] Docker cache is allowed only with immutable input proof.
- [ ] Parallelism is opt-in, resource-governed, and artifact-isolated.
- [ ] Serial and parallel score-validity fields match before parallel mode is accepted.
- [ ] Any speed claim is backed by serial baseline, parallel smoke, and timing reconciliation.
- [ ] `2-3x` is forbidden as a claim until proven by a full governed run.
