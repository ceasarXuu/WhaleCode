# R4 Phase 收益证据账本

> 本文只记录已经由当前工程产物证明的 phase benefit。没有证据的 phase 不能标记完成。

## 5.1 当前状态

```text
Updated: 2026-06-30
Code state at evidence capture: this R4-D repair changeset
Status:
  R4-A pass: tool path coverage manifest and gate are executable.
  R4-B pass: known sample evidence ledger and gate are executable.
  R4-C pass for direct tool success/error map-preview parity.
  R4-D pass for count-call-stack internal tool feedback path: before wrong/timeout, after solved.
  R4-D remains open for validation-closeout-tool-drain, discovered by large-output-ref-smoke.
  R4-E partial pass for large raw output / rollout bloat: timeout removed and rollout bounded.
  R4-E remains open for full pair-safe projection semantics.
  R4-F/R4-G/R4-H remain open until non-direct tools, public-10, and closeout gates finish.
```

## 5.2 PhaseBenefitEvidenceV1

| Phase | Claimed Engineering Benefit | Baseline Artifact | After Artifact | Measurement Method | Metric | Baseline Value | After Value | Pass Threshold | Pass/Fail | Evidence Paths |
|---|---|---|---|---|---|---|---|---|---|---|
| R4-A | tool path 覆盖从人工 Markdown 变成机器可读 manifest 和 gate，新增或遗漏 path 可被门禁发现 | `docs/v0.0.5/build-R4/01-static-tool-chain-map.md` | `docs/v0.0.5/build-R4/r4-tool-path-coverage.json` + validator output | `test-r4-tool-path-coverage.ps1` 校验 schema、source anchors、owner phase、required semantics | `unknown/unowned/missing-anchor` | 无可执行检查 | `path_count=9`, `failure_count=0` | `failure_count=0` | pass | `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-B | 历史 sample 现场从 scattered target/CoE 变成机器可读账本，known-bad 类型和 owner phase 可验证 | `docs/v0.0.5/build-R4/02-field-evidence-and-sample-ledger.md` | `docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json` + validator output | `test-r4-sample-ledger.ps1` 校验 sample id、failure class、owner phase、evidence path、required classes | `sample_count/missing-evidence` | 无可执行检查 | `sample_count=7`, `failure_count=0` | `sample_count>=6`, `failure_count=0` | pass | `target/r4-sample-ledger/r4-sample-ledger-evidence.json` |
| R4-C | direct tool error 的 TaskSpace map preview 不再走独立摘要，而是从 standard failure response 的 model-visible item 派生 | `parallel.rs` success/error map preview 来源分叉；manifest `direct-tool-error-map-preview=needs-fix` | `failure_response_for_error` + `response_input_model_visible_preview`；manifest `direct-tool-error-map-preview=canonical` | focused Rust unit test + R4 coverage validator | `failure_response_preview` | error path 独立 `action_map_tool_error_preview` | focused tests pass；coverage path canonical | focused tests pass；coverage validator pass | pass | `cargo test -p codex-core failure_response_preview --lib`; `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-D | action-contract internal failed tool outputs、validation gate failure、unreviewed-result blocker、dependency read evidence 都能以可执行语义进入下一轮，而不是被压成无法行动的 raw stderr 或 generic recovery | `count-call-stack` 历史 run：TaskSpace wrong/no patch，后续多轮 timeout；apply_patch 失败、validator 覆盖失败、unreviewed result、dependency read evidence 丢失分别导致卡住 | `third_party/codex-cli/codex-rs/core/src/session/turn.rs`、`action_map/runtime.rs`、`tools/parallel.rs` 修复；真实 rerun solved | focused Rust tests + paired public sample rerun | `outcome_taskspace`, `tool_call_count`, `wall_time_ratio`, `changed_paths`, `public_validation_exit_code` | wrong/no patch 或 900s timeout；`changed_paths` 空；validator/projection 链路卡死 | solved；`changed_paths=src/call_stack_counter.py`；`public_validation_exit_code=0`；tool calls 11 vs standard 20；wall ratio 1.12 | known feedback-loss sample 不再 wrong/no_patch；validation exit 0；无 evidence gate failure | pass for this P0 path | `target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136/pair-001/pair-report.md` |
| R4-E | large raw tool output 不再把 provider payload / rollout 撑爆，output-ref 事件可追踪；但本 phase 的 pair-safe projection 总体尚未完全关闭 | `large-output-ref-smoke` timeout；TaskSpace rollout `490,846,386` bytes | `large-output-ref-smoke` rerun 无 900s timeout；TaskSpace rollout `360,600` bytes；`output_ref.created`；exact payload scan `large_raw_output_tokens=0`、`replacement_confirmed=true` | large-output rerun + rollout size + exact payload scan + output-ref events | `timeout`, `rollout_size_bytes`, `large_raw_output_tokens`, `output_ref.created` | timeout；rollout `490,846,386` bytes；失败日志膨胀 | no timeout；rollout `360,600` bytes；`large_raw_output_tokens=0`；`output_ref.created` | no timeout；rollout bounded；large raw not provider-visible；ref event present | partial pass: large-output/log-bloat 子项 pass；correctness/validation closeout fail | `target/r4-e-large-output-ref-20260630/large-output-ref-smoke/20260630-211225-432/pair-001/pair-report.md`; `target/r4-e-large-output-ref-20260630/large-output-ref-smoke/20260630-211225-432/pair-001/right/artifacts/exact-payload-scan-events.jsonl`; `target/r4-e-large-output-ref-20260630/large-output-ref-smoke/20260630-211225-432/pair-001/right/artifacts/output-ref-events.jsonl` |
| R4-F | CodeMode/multi-agent/MCP 等 non-direct tools 有明确 inclusion/exclusion 和 provider-visible feedback 证明 | R4-A manifest 标记 non-direct paths `needs-fix` | pending | coverage fixtures + exclusion proof | `classified_path_count`, `missing_feedback_count` | blind spots | pending | all non-direct paths classified or intentionally excluded with tests | open | pending |
| R4-G | known-bad 和 10 个公开 benchmark 样本证明收益真实，而不是只靠局部单测 | R3/R4 scattered run evidence；public sample plan was Markdown-only | `r4-public-10-tool-stress-plan.json` + plan gate；paired run pending | public-10 plan gate + paired standard/taskspace 10 public samples + per-sample tool analysis | `tool_feedback_loss_count`, `wall/token/tool ratio`, `cache_hit`, `public_sample_count` | pending | plan gate pending after script run；paired run pending | public-10 plan gate pass；final report 10 rows；feedback loss 0；cache hit >= 0.95 or explained | open | `docs/v0.0.5/build-R4/r4-public-10-tool-stress-plan.json`; `scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1` |
| R4-H | 工程层收口可审计、可复现 | scattered phase evidence | pending | closeout doc + committed artifacts | `open_phase_count`, `unexplained_failure_count` | pending | pending | no phase marked completed without benefit evidence | open | pending |

## 5.3 R4-D count-call-stack 真实样本证据链

| Run | Result | 发现的问题 | 工程处理 |
|---|---|---|---|
| `target/r4-d-count-call-stack-validator-gate-20260630/count-call-stack/20260630-174500-917` | TaskSpace wrong/no patch | failed edit 只触发 generic needs-edit recovery，且 recovery 次数耗尽后硬停 | 删除硬上限停机；新增 `TaskSpaceEditFailureRecoveryV1`，保留最近失败 edit tool feedback |
| `target/r4-d-count-call-stack-edit-failure-gate-20260630b/count-call-stack/20260630-181843-421` | TaskSpace 改对文件但 900s timeout | validation node 继续 list/read，不执行 validator | validation node action policy 收敛为 `run_test/taskspace_control/blocked`，新增 validation-needs-test recovery |
| `target/r4-d-count-call-stack-validation-policy-20260630/count-call-stack/20260630-185814-888` | TaskSpace 改对文件但 900s timeout | 发现 `scripts/validate.py` 后仍反复跑 pytest；local-validator gate 反馈不够结构化 | 新增 local validator coverage gate 和 `TaskSpaceToolFeedbackV1` structured feedback |
| `target/r4-d-count-call-stack-validator-feedback-20260630/count-call-stack/20260630-193832-942` | TaskSpace 无 patch | `result_validities`/unreviewed-result blocker 反馈不够可行动 | unreviewed-result blocker 进入 recent tool feedback，并白名单为 actionable gate feedback |
| `target/r4-d-count-call-stack-unreviewed-feedback-20260630/count-call-stack/20260630-201304-504` | TaskSpace 无 patch | inspect 节点读到源文件后，implement 投影只剩 preview，丢失可编辑证据 | implement projection 新增 `dependency_read_evidence`，保留上游 inspect read 的路径和有界内容 |
| `target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136` | TaskSpace solved | 修复后真实收益成立 | 标准 solved 138205ms/20 tools；TaskSpace solved 154525ms/11 tools；wall ratio 1.12；tool ratio 0.55；changed `src/call_stack_counter.py`；validation exit 0 |

## 5.4 已执行命令

```text
cargo fmt --all --check
  PASS with existing rustfmt nightly-config warnings

cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS

cargo test -j1 -p codex-core edit_failure_recovery_preserves_failed_tool_feedback --lib
  PASS

cargo test -j1 -p codex-core implement_failed_edit_summary_keeps_latest_tool_feedback --lib
  PASS

cargo test -j1 -p codex-core validation_needs_test_recovery_blocks_discovery_loop --lib
  PASS

cargo test -j1 -p codex-core taskspace_action_contract_node_policy_matrix_blocks_cross_node_actions --lib
  PASS

cargo test -j1 -p codex-core taskspace_tool_result_preview_keeps_shell_command_context --lib
  PASS

cargo test -j1 -p codex-core validation_gate_requires_discovered_local_validator_over_pytest_only --lib
  PASS

cargo test -j1 -p codex-core action_contract_prompt_structures_local_validator_coverage_failure --lib
  PASS

cargo test -j1 -p codex-core action_contract_prompt_structures_unreviewed_result_blocker --lib
  PASS

cargo test -j1 -p codex-core projection_prioritizes_discovered_local_validator_on_validation_node --lib
  PASS

cargo test -j1 -p codex-core implement_projection_includes_dependency_read_evidence --lib
  PASS

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1
  PASS: R4 tool path coverage gate passed: 9 paths

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-sample-ledger.ps1
  PASS: R4 sample ledger gate passed: 7 samples

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 -Scenario count-call-stack -Repeats 1 ...
  PASS: outcome_standard=solved; outcome_taskspace=solved; failure_taxonomy=none

cargo test -j1 -p codex-core rollout_persistence_referenceizes_large_tool_outputs --lib
  PASS

cargo test -j1 -p codex-core active_context_replacement_omits_large_raw_tool_output --lib
  PASS

cargo test -j1 -p codex-core active_context_replacement_keeps_output_reference_payloads --lib
  PASS

cargo build -j1 --profile dev-small -p codex-cli --bin whale
  PASS

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1
  PASS: R4 public-10 tool-stress gate passed: 10 planned samples

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath target/r4-public-10-tool-stress/fixture-report.json
  PASS: report-mode gate accepts a 10-row report containing all required standard/taskspace/tool fields

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1
  PASS: R4 tool path coverage gate passed: 10 paths

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-sample-ledger.ps1
  PASS: R4 sample ledger gate passed: 9 samples

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/run-taskspace-benchmark.ps1 -Scenario large-output-ref-smoke -Repeats 1 ...
  PARTIAL PASS for R4-E large-output/log-bloat: no timeout; rollout 360,600 bytes vs previous 490,846,386 bytes; exact payload scan large_raw_output_tokens=0.
  FAIL for task correctness: outcome_taskspace=wrong; public_validation_exit_code=1; discovered validation-closeout-tool-drain issue.
```

## 5.5 不能据此关闭的内容

这些证据关闭 R4-A、R4-B、R4-C direct tool preview parity，以及 R4-D 的 `count-call-stack` P0 internal tool feedback 链路；不能关闭：

1. R4-E pair-safe projection 总体证明；大输出/日志膨胀子项已有真实收益证明，但不能关闭整个 R4-E。
2. R4-F CodeMode、multi-agent、MCP 等 non-direct tool runtime coverage。
3. R4-G known-bad 全量回归和公开 10 样本综合验收；公开 10 样本计划已有门禁，但还没有最终 paired run 报告。
4. R4-H 工程层完整收口。
5. 新增 `validation-closeout-tool-drain`：工具执行成功不能被误判成公开验证成功。
## 5.6 2026-07-01 R4-D latest large-output-ref-smoke evidence

Latest focused reruns show R4-D is still open. The current fixes improved several
engineering subproblems but did not close the sample:

| Run | Main observation | Evidence |
| --- | --- | --- |
| `target/r4-d-duplicate-diagnostic-transition-fix2-20260701/.../20260701-013045-328` | duplicate diagnostic gate recovery triggered and forced inspect transition, but it transitioned too early using only `rg --files`; TaskSpace still timed out and did not patch `src/large_output_demo.py`. | `TaskSpaceForcedInspectTransitionRecoveryV1=1`; outcome_taskspace=`agent_exec_timeout`; rollout `764,690,287` bytes. |
| `target/r4-d-duplicate-diagnostic-read-evidence-20260701/.../20260701-021208-100` | duplicate-diagnostic recovery was visible and rollout dropped versus the prior rerun, but `rg --files` was still treated as enough working evidence; TaskSpace still timed out. | `TaskSpaceDuplicateDiagnosticInspectRecoveryV1=5`; outcome_taskspace=`agent_exec_timeout`; rollout `515,799,514` bytes. |
| `target/r4-d-path-list-not-working-evidence-20260701/.../20260701-030240-333` | `rg --files` no longer forced transition; graph stayed on inspect. Remaining blocker is repeated blocked `run_test` despite model-visible recovery saying the diagnostic already completed. | nodes=`1`; outcome_taskspace=`agent_exec_timeout`; rollout `992,140,475` bytes. |

Validated engineering fixes from this slice:

```text
cargo test -j1 -p codex-core forced_inspect_transition_accepts_duplicate_diagnostic_gate_recovery --lib
  PASS

cargo test -j1 -p codex-core duplicate_diagnostic_recovery_keeps_inspect_on_source_evidence --lib
  PASS

cargo test -j1 -p codex-core thin_route_allows_narrow_file_discovery_in_inspect --lib
  PASS

cargo fmt --all --check
  PASS
```

Current unresolved root cause:

TaskSpace action-contract mode can keep accepting provider turns that repeat the
same blocked action (`run_test python scripts/emit_large_log.py`) even after the
runtime has returned clear model-visible `TaskSpaceGateRecoveryV1` feedback and a
dedicated duplicate-diagnostic recovery message. The next R4-D fix should be a
systemic repeated-blocked-action convergence rule in the action-contract control
loop, not another prompt-only reminder. The rule must preserve Standard-mode
tool-result visibility semantics while preventing unbounded repeats of an
identical blocked action in the same node.

## 5.7 2026-07-01 R4-D/E large-output-ref-smoke 收敛证据

本轮继续跟踪 `large-output-ref-smoke`，确认 R4-D 的 tool feedback/action-contract
链路已经从 timeout/no-patch 收敛为 solved，但 R4-G 性能门禁仍未关闭。

### 5.7.1 新增根因和修复

| 问题 | 根因 | 修复 | 证据 |
| --- | --- | --- | --- |
| duplicate diagnostic recovery 后仍可能重复无效 `run_test` | blocked action 没有按 node/progress/action/command 建状态化 repeat fingerprint | `runtime.rs` 增加 `blocked_action_repeats`，`TaskSpaceGateRecoveryV1` 带 `repeated_blocked_action` 和 `same_action_allowed=false` | `cargo test -j1 -p codex-core forced_inspect_transition_accepts_duplicate_diagnostic_gate_recovery --lib` PASS |
| inspect 中已有 `src/large_output_demo.py` 证据，但 implement 节点接受“源码不可见/no excerpt” blocker | `block_main_node` 只拒绝诊断缺失和内部策略 blocker，没有识别“已有 implementation source evidence 时的假缺证据 blocker” | 新增 `implement_node_has_dependency_implementation_source_evidence` 和 `blocker_claims_missing_inspected_source_evidence`，拒绝 missing source visibility blocker | `cargo test -j1 -p codex-core implement_node_rejects_missing_source_blocker_when_source_evidence_is_available --lib` PASS |
| runtime 拒绝后模型可能只看到 generic failure | action-contract recent tool outputs 没有 missing-source blocker 的结构化分类 | `turn.rs` 新增 `failure_kind: missing_source_visibility_blocker_rejected` 和 `progress_hint`，要求用 failed patch feedback 修正 `apply_patch` | `cargo test -j1 -p codex-core action_contract_prompt_structures --lib` PASS，8 tests |

### 5.7.2 真实样本 before/after

| Run | outcome_standard | outcome_taskspace | changed_paths_taskspace | wall ratio | tool ratio | 结论 |
| --- | --- | --- | --- | --- | --- | --- |
| `target/r4-d-path-list-not-working-evidence-20260701/large-output-ref-smoke/20260701-030240-333` | solved | `agent_exec_timeout` | `.large_output_probe_ran` | timeout | 0 | 仍卡在重复 blocked diagnostic，未改目标文件 |
| `target/r4-d-internal-policy-blocker-20260701/large-output-ref-smoke/20260701-121553-941` | solved | `agent_exec_timeout` | `.large_output_probe_ran` | 18.89 | 0 | apply_patch 失败反馈可见，但 missing-source blocker 被接受，进入 unreviewed blocker loop |
| `target/r4-d-missing-source-blocker-20260701/large-output-ref-smoke/20260701-130225-851` | solved | solved | `.large_output_probe_ran`, `src/large_output_demo.py` | 6.45 | 0.57 | 第二次 `apply_patch` 成功，`pytest` 通过，public/hidden oracle 通过 |

### 5.7.3 关键工具链证据

```text
RunDir: target/r4-d-missing-source-blocker-20260701/large-output-ref-smoke/20260701-130225-851
PairReport: pair-001/pair-report.md
outcome_standard: solved
outcome_taskspace: solved
failure_taxonomy: none
standard_wall_time_ms: 33328
taskspace_wall_time_ms: 214813
taskspace_wall_time_ratio: 6.45
standard_tool_call_count: 7
taskspace_tool_call_count: 4
taskspace_tool_call_ratio: 0.57
public_validation_exit_code: 0
hidden_oracle_exit_code: 0
taskspace_changed_paths: .large_output_probe_ran, src/large_output_demo.py
```

真实 rollout 证明：

```text
taskspace-action-contract-7-apply_patch
  apply_patch verification failed: Failed to find expected lines ...

taskspace-action-contract-8-apply_patch
  Success. Updated the following files:
  M src/large_output_demo.py

taskspace-action-contract-11-run_test
  pytest -> 2 passed
```

provider/cache 证明：

```text
exact_payload_scan_passed=true for all captured provider requests
stable_prefix_hash=11ca199361e22844c00ce06ab2bd3b9ce9ee66f94f96222989cc9af46e486061
cached_input_tokens observed range: 118400..120320
input_tokens per request around 119302..120516
```

### 5.7.4 当前结论

R4-D 的 `large-output-ref-smoke` correctness blocker 可以关闭：已从
`agent_exec_timeout/no target patch` 收敛为 `both_success`，且能证明失败 patch
反馈被模型看到并纠错。R4-E 的 large-output ref/log-bloat 子项仍保持通过，且本次
exact payload scan 继续通过。

但 R4-G 性能和综合验收仍未关闭：TaskSpace wall time 仍为 standard 的 6.45x，
样本证据等级仍是 `E2-candidate` 且 `repeats_lt_3`、`aggregate_not_enabled`。
下一步必须继续做 10 个公开 tool-stress samples 和性能归因，不能把 R4 标记为完成。

## 5.8 2026-07-01 R4-G processing-pipeline 公开样本现场

本轮开始执行公开 10 样本综合验收。计划门禁已在
`C:\WhaleRunCache\r4-public10-20260701\suite-plan-threshold10\suite-20260701-132121`
通过；D 盘根目录可用空间只有 19.41 GiB，低于默认 20 GiB 保护阈值，因此本次计划门禁使用
`TASKSPACE_MIN_FREE_GIB=10`。WSL/docker 数据盘仍有约 941 GiB 可用，实际限制来自 Windows 根路径保护阈值。

### 5.8.1 remote asset 资格误判修复

`csv-to-parquet` 和 `processing-pipeline` 首次实跑在模型执行前被判为
`not_eligible_remote_asset_unproven`。根因不是样本本身不可跑，而是 Terminal-Bench adapter 把官方隐藏答案文件也纳入 remote asset 扫描，并且没有把任务常用的
`https://astral.sh/uv/install.sh` 映射到已经缓存的 uv installer。

| 修复点 | 证据 |
| --- | --- |
| remote asset scanner 跳过隐藏答案文件 `solution.sh` / `solution.yaml` | `scripts/taskspace-benchmark/adapters/terminal-bench-remote-assets.ps1` |
| uv cache 增加 unversioned installer alias，并让 curl wrapper 覆盖 `astral.sh/uv/install.sh` | `scripts/taskspace-benchmark/adapters/terminal-bench-uv-cache.ps1` |
| adapter 自测增加隐藏答案 remote URL fixture，确认不会污染资格判定 | `scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1` |

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1 -RunRoot C:\WhaleRunCache\r4-public10-20260701\adapter-selftest
Terminal-Bench adapter self-test: PASS
```

### 5.8.2 successful edit artifact backfill 收益

`processing-pipeline` v2 暴露出 action-contract internal `apply_patch` 成功后，TaskSpace map 中对应 implement result 没有 `changed_artifacts`，导致后续 validation action 被反复拦截为
`validation_test_missing_changed_artifact_coverage`。

| Run | outcome_taskspace | changed artifact coverage block | graph/result 现场 | 结论 |
| --- | --- | ---: | --- | --- |
| `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v2\runs\terminal_bench__processing-pipeline\20260701-132747-117` | `agent_exec_timeout` | 94 | nodes=3, edges=388, result_count=13, rollout=408,573,635 bytes | 成功 edit 没有回填 artifacts，validation 被无效阻塞循环拖死 |
| `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v3\runs\terminal_bench__processing-pipeline\20260701-141309-114` | `agent_exec_timeout` | 0 | nodes=1, edges=0, result_count=107, rollout=437,721,918 bytes | coverage block 已消失，说明 artifact backfill 修复有真实收益 |

对应代码修复：

| 文件 | 改动 |
| --- | --- |
| `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs` | 增加 `backfill_successful_implementation_edit_artifacts`，把 turn diff 中的 changed files 回填到最新成功 implement edit result 的 `changed_artifacts` 和 `evidence_refs` |
| `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | `record_taskspace_observed_implement_edit` 不再因为“已有成功 edit”就跳过，只有已有成功 edit artifacts 时才跳过；缺失 artifacts 时先尝试 backfill |
| `third_party/codex-cli/codex-rs/core/src/session/mod.rs` | 增加 session wrapper，保持 turn/runtime 边界清晰 |

验证：

```text
cargo test -j1 -p codex-core observed_edit_backfill_records_changed_artifacts_on_implementation_result --lib
PASS

cargo test -j1 -p codex-core validation_node_blocks_vacuous_test_after_changed_artifact --lib
PASS

cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

### 5.8.3 新暴露未收敛问题

v3 不是完整通过。修掉 coverage block 后，TaskSpace 暴露出新的 read-only inspect loop：

1. graph 只有 1 个 `inspect_code_context` node，edges=0，没有进入 implement/test 节点。
2. result_count=107 且全部 unreviewed，rollout 达到 437,721,918 bytes。
3. rollout 中 provider request counter 最高到 53，说明 `request_count` 不是硬上限；这符合前面“不要用 request 硬砍预算”的设计方向，但也要求 convergence 机制能识别无进展读取循环。
4. 工具序列包含反复 `list_files` / `read_file`，甚至读取了 `task.yaml`；需要继续确认 Terminal-Bench fixture 是否泄漏任务元数据，或只是任务目录内正常可见文件。
5. metrics extractor 对 action-contract internal tools 仍有统计缺口：pair report 里 TaskSpace `tool_call_count=0`，但 graph/rollout 已证明有 107 个 result 和大量工具调用。

因此 R4-G 仍打开。下一步不是再修 validation coverage，而是专项收敛：

1. 修 inspect-node repeated read / no-progress convergence，让 TaskSpace 在已有足够局部证据时推进到 implement，且不依赖粗暴 request 硬上限。
2. 修 large rollout/action-contract metrics 统计，让公开 10 样本表格能可靠展示工具调用次数、时长和 token。
3. 检查 Terminal-Bench fixture 暴露面，确认 `task.yaml`、hidden tests、solution files 不会进入 agent 可见工作区。

### 5.8.4 inspect no-progress 修复进展

针对 5.8.3 的 read-only inspect loop，已做两轮工程修复：

| 版本 | 改动 | 验证 | 真实样本结论 |
| --- | --- | --- | --- |
| first patch | 在 `record_main_tool_result_with_class` 写入 read/search 工具结果后，立即检查既有 `inspect_progress_convergence` 规则 | `inspect_progress_convergence_force_finishes_after_contract_hint` 通过 | v4 仍失败：`node_count=1`, `result_count=94`, `rollout=351,884,108 bytes`，没有 `forced_inspect_transition` |
| second patch | 增加 `provider_request_progress_snapshot_for_node` fallback；收敛判断不再依赖 active budget snapshot，也不安装 active budget | 同一测试显式设置 `state.active_budget=None` 后通过；`cargo fmt --all --check`、`observed_edit_backfill...`、`cargo build` 通过 | 真实收益证明未闭合：v5 外层 10 分钟超时且被停止，没有导出 graph-health |

v4 真实现场：

```text
RunDir: C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v4\runs\terminal_bench__processing-pipeline\20260701-151740-194
graph-health: node_count=1, edge_count=0, result_count=94
rollout_bytes=351,884,108
active-budget-events.jsonl length=0
```

当前结论：

1. `processing-pipeline` 的 validation coverage blocker 已从 94 降为 0，这个收益已证明。
2. inspect no-progress loop 的第一版修复被 v4 证伪，根因是仍依赖 budget snapshot。
3. 第二版修复已通过单测/构建，但还缺真实样本证明；R4-G 不能关闭。
