# R4 Phase 收益证据账本

> 本文只记录已经由当前工程产物证明的 phase benefit。没有证据的 phase 不能标记完成。

## 5.1 当前状态

```text
Updated: 2026-07-03
Code state at evidence capture: R4 tool-chain convergence changes through public-10 closeout
Status:
  R4-A pass: tool path coverage manifest and gate are executable; canonical paths now require coverage_test.
  R4-B pass: known sample evidence ledger and gate are executable.
  R4-C pass for direct tool success/error map-preview parity.
  R4-D pass for internal feedback, validation closeout drain, and closed-validation contract focused gates.
  R4-E pass for large raw output ref and pair-safe provider projection focused gates.
  R4-F pass for CodeMode, multi-agent, and MCP non-direct tool path classification gates.
  R4-G closed for benchmark/report engineering gate; utility evidence is negative and blocks E3.
  R4-H closed with engineering closeout and E3 no-go decision; 2026-07-03 keyed rerun confirms provider credentials are no longer the blocker, but TaskSpace utility convergence remains blocked.
```

## 5.2 PhaseBenefitEvidenceV1

| Phase | Claimed Engineering Benefit | Baseline Artifact | After Artifact | Measurement Method | Metric | Baseline Value | After Value | Pass Threshold | Pass/Fail | Evidence Paths |
|---|---|---|---|---|---|---|---|---|---|---|
| R4-A | tool path 覆盖从人工 Markdown 变成机器可读 manifest 和 gate，新增或遗漏 path 可被门禁发现 | `docs/v0.0.5/build-R4/01-static-tool-chain-map.md` | `docs/v0.0.5/build-R4/r4-tool-path-coverage.json` + validator output | `test-r4-tool-path-coverage.ps1` 校验 schema、source anchors、owner phase、required semantics | `unknown/unowned/missing-anchor` | 无可执行检查 | `path_count=10`, `failure_count=0` | `failure_count=0` | pass | `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-B | 历史 sample 现场从 scattered target/CoE 变成机器可读账本，known-bad 类型和 owner phase 可验证 | `docs/v0.0.5/build-R4/02-field-evidence-and-sample-ledger.md` | `docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json` + validator output | `test-r4-sample-ledger.ps1` 校验 sample id、failure class、owner phase、evidence path、required classes | `sample_count/missing-evidence` | 无可执行检查 | `sample_count=12`, `failure_count=0` | `sample_count>=6`, `failure_count=0` | pass | `target/r4-sample-ledger/r4-sample-ledger-evidence.json` |
| R4-C | direct tool error 的 TaskSpace map preview 不再走独立摘要，而是从 standard failure response 的 model-visible item 派生 | `parallel.rs` success/error map preview 来源分叉；manifest `direct-tool-error-map-preview=needs-fix` | `failure_response_for_error` + `response_input_model_visible_preview`；manifest `direct-tool-error-map-preview=canonical` | focused Rust unit test + R4 coverage validator | `failure_response_preview` | error path 独立 `action_map_tool_error_preview` | focused tests pass；coverage path canonical | focused tests pass；coverage validator pass | pass | `cargo test -p codex-core failure_response_preview --lib`; `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-D | action-contract internal failed tool outputs、validation gate failure、unreviewed-result blocker、dependency read evidence、validation closeout drain 都能以可执行语义进入下一轮或形成明确 closed validation 状态 | `count-call-stack` 历史 run：TaskSpace wrong/no patch，后续多轮 timeout；`large-output-ref-smoke` 曾把诊断工具成功误判为验证成功；processing-pipeline 曾在 infra failure 后继续 open leaf | `turn.rs`、`action_map/runtime.rs`、`tools/parallel.rs` 修复；真实 rerun solved 或收敛到 infra-blocked closed validation；manifest 对应 P0/P1 path canonical | focused Rust tests + paired public sample rerun + processing-pipeline v11 | `outcome_taskspace`, `changed_paths`, `public_validation_exit_code`, `open_leaf`, `invalid`, `closed_contract` | wrong/no patch、validation false-positive、900s timeout 或 open leaf | count-call-stack solved；large-output solved；processing-pipeline v11 `exec_timed_out=false`, `open_leaf=0`, `invalid=1`, `TaskSpaceActionContractClosedValidationV1=1` | known feedback-loss sample 不再 wrong/no_patch；诊断工具成功不再伪装 validation pass；closed validation 不再继续开新节点 | pass | `target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136/pair-001/pair-report.md`; `target/r4-d-missing-source-blocker-20260701/large-output-ref-smoke/20260701-130225-851/pair-001/pair-report.md`; `C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v11\runs\terminal_bench__processing-pipeline\20260701-214838-298` |
| R4-E | large raw tool output 不再把 provider payload / rollout 撑爆；tool call/result pair 在 active context replacement 中成组 omit/ref/keep，不产生 orphan tool history | `large-output-ref-smoke` timeout；TaskSpace rollout `490,846,386` bytes；active context replacement 曾出现 pair-safe 证明缺失 | `large-output-ref-smoke` rerun 无 900s timeout且 rollout bounded；pair-safe projection focused tests pass；manifest `large-raw-tool-output-ref` 与 `provider-visible-history-projection` 均 canonical | large-output rerun + rollout size + exact payload scan + output-ref events + focused Rust pair-safe tests | `timeout`, `rollout_size_bytes`, `large_raw_output_tokens`, `output_ref.created`, `paired_omit_tests` | timeout；rollout `490,846,386` bytes；失败日志膨胀；pair-safe 未证明 | no timeout；rollout `360,600` bytes；`large_raw_output_tokens=0`；`output_ref.created`；`active_context_replacement_omits_paired` pass | no timeout；rollout bounded；large raw not provider-visible；ref event present；tool call/result 不单边残留 | pass for projection/output-ref engineering gates | `target/r4-e-large-output-ref-20260630/large-output-ref-smoke/20260630-211225-432/pair-001/pair-report.md`; `target/r4-e-large-output-ref-20260630/large-output-ref-smoke/20260630-211225-432/pair-001/right/artifacts/exact-payload-scan-events.jsonl`; `cargo test -j1 -p codex-core active_context_replacement_omits_paired --lib` |
| R4-F | CodeMode/multi-agent/MCP 等 non-direct tools 有明确 inclusion/exclusion 和 provider-visible/code-runtime feedback 证明 | R4-A manifest 标记 non-direct paths `needs-fix`，multi-agent wrapper 缺少独立语义保真测试 | manifest 中 `codemode-nested-tool-result`、`multi-agent-tool-output-wrapper`、`mcp-tool-output-response-item` 均 canonical；新增 multi-agent wrapper tests | coverage fixtures + exclusion proof + R4 coverage validator | `classified_path_count`, `missing_feedback_count`, `coverage_test_presence` | blind spots；3 条 non-direct path needs-fix | `path_count=10`, `canonical_count=10`, `needs_fix_count=0`; CodeMode/MCP/multi-agent focused tests pass | all non-direct paths classified or intentionally excluded with tests | pass | `cargo test -j1 -p codex-core dispatch_lifecycle_trace_records_direct_and_code_mode_requesters --lib`; `cargo test -j1 -p codex-core mcp_tool_output_response_item --lib`; `cargo test -j1 -p codex-core multi_agent_tool_output --lib`; `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-G | known-bad 和 10 个公开 benchmark 样本证明收益真实，而不是只靠局部单测 | `processing-pipeline` v3/v4：单 inspect 节点重复读取，`changed_paths=[]`，metrics `tool_call_count=0` 误导；公开 10 样本缺最终报告 | public-10 final report 10/10 rows；large-rollout 工具调用可从 observability 回退统计；feedback loss 0/0；cache hit 可统计行约 `0.9810-0.9882`；同时暴露 TaskSpace utility no-go | public-10 plan gate + paired standard/taskspace 10 public samples + per-sample tool analysis | `tool_feedback_loss_count`, `wall/token/tool ratio`, `cache_hit`, `public_sample_count`, `rollout_tool_call_count`, `observability_tool_call_count` | no final public-10 report；TaskSpace utility 未验证 | `complete_run_count=10`, `missing_run_count=0`; TaskSpace solved 3/10；standard solved but TaskSpace timeout/wrong 的样本存在 | report gate pass；负收益明确记录，不进入 E3 | closed/evidence-negative | `target/r4-public-10-tool-stress/r4-public-10-tool-stress-report.json`; `docs/v0.0.5/build-R4/05-phase-benefit-evidence.md#510-2026-07-02-r4-g-public-10-final-run` |
| R4-H | 工程层收口可审计、可复现 | scattered phase evidence；无 E3 decision | closeout doc + committed artifacts + E3 no-go | closeout report + gate commands + git commits | `open_phase_count`, `unexplained_failure_count`, `e3_readiness` | open phase；pending closeout | R4 engineering deliverables 8/8；TaskSpace utility 3/10；E3 readiness 0% | no phase marked completed without benefit evidence；E3 decision explicit | pass | `docs/v0.0.5/build-R4/06-r4-engineering-closeout.md` |

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
| second patch | 增加 `provider_request_progress_snapshot_for_node` fallback；收敛判断不再依赖 active budget snapshot，也不安装 active budget | 同一测试显式设置 `state.active_budget=None` 后通过；`cargo fmt --all --check`、`observed_edit_backfill...`、`cargo build` 通过 | v7 真实样本证明 inspect 可转入 implement/test：`forced_inspect_transition`、`node_count=3`、`changed_paths=[generate_report.sh]`；新阻塞转为 validator infra `E_ACCESSDENIED` |

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
3. 第二版修复已通过单测/构建和 v7 真实样本证明；R4-G 仍不能关闭，因为新阻塞转为 validator infra recovery、metrics 新字段 fresh-run 写出证明，以及公开 10 样本综合验收。

### 5.8.5 2026-07-01 processing-pipeline v7 真实收益和新阻塞

v6 暴露出一个门禁/安装链路问题：`scripts/install-whale-local.ps1` 默认优先安装
`debug\whale.exe`，而当前构建命令产物是 `dev-small\whale.exe`。因此 v6 虽然
刷新了 attestation，但实际仍运行旧二进制，不能作为 second patch 真实验证。

修复：

1. `install-whale-local.ps1` 在未显式传入 `-BinaryPath` 时改为从候选 `whale.exe`
   中选择最新产物。
2. 候选列表加入 `dev-small\whale.exe`。
3. 安装后自动调用 `write-whale-binary-attestation.ps1` 刷新 binary attestation。

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-whale-local.ps1 -InstallDir D:\whalecode-alpha\target\install-whale-local-selftest-2
Source: D:\BuildCache\whalecode\cargo-target\dev-small\whale.exe
Hash: 29A68B8C57B425DFBFA326B23C9877D0E45B0BE51119A9D5CD82740181A9CB06
WhaleBinaryAttestation: D:\whalecode-alpha\target\install-whale-local-selftest-2\whale.exe.build-attestation.json
```

随后用新二进制跑 `processing-pipeline-v7`：

```text
RunDir: C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v7\runs\terminal_bench__processing-pipeline\20260701-163507-220
forced_inspect_transition: count=18 in rollout scan
trigger: inspect_progress_convergence
request_count:13
max_requests:20
source_node_kind: inspect_code_context
next_node_kind: implement_solution
bound_next_node_id: node-2
```

真实收益：

| 指标 | v6 旧 binary / 无效验证 | v7 新 binary / 有效验证 |
| --- | --- | --- |
| TaskSpace graph | `node_count=1`, `edge_count=0`, `changed_paths=[]` | `node_count=3`, `edge_count=2`, `changed_paths=[generate_report.sh]` |
| inspect 收敛 | 无 `forced_inspect_transition` | `forced_inspect_transition` 出现，`node-1` 转入 `node-2` |
| 实现动作 | 无 patch | `apply_patch` 成功修改 `generate_report.sh` |
| 后续节点 | 无 implementation/test | `taskspace_control finish_node` 后进入 `node-3(smoke_test)` |

这证明 second patch 的核心收益已经成立：TaskSpace 不再卡死在单个 inspect
节点的重复读取里，而是能按工具反馈推进到实现节点并产生真实代码修改。

新阻塞：

```text
taskspace-action-contract-16-run_test
Tool call failed before producing a result.
local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED
```

该阻塞是本机 validator/shell 基础设施问题，不是当前 inspect 收敛修复的失败；
但它仍会导致本次 paired run 在 `TimeoutSeconds=180` 下未完成 blocked/final 收口，
所以 R4-G 仍不能关闭。

同时发现 metrics 统计缺口：v7 修复前生成的 `metrics.json` 仍显示
`tool_call_count=0`，但 rollout 复算结果为：

```text
Get-TaskspaceRolloutToolStats(v7 rollout)
Completed: 15
Failed: 1
Control: 2
Availability: measured
```

因此已修复 `metrics-extractor.ps1`：当 `whale-exec.jsonl` 缺少 action-contract
工具项时，从可扫描的 `rollout.jsonl` 统计普通工具调用、失败工具调用和
`taskspace_control` 调用，并新增 `rollout_tool_call_count`、
`rollout_failed_tool_call_count`、`rollout_control_tool_call_count` 字段。

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-r4-metrics-extractor-large-rollout.ps1
PASS: R4 metrics extractor large rollout gate passed
```

当前结论：

1. `processing-pipeline` 的 inspect no-progress 工程缺陷已从真实样本层面收敛到下一层问题。
2. R4-G 仍打开，原因转移为 validator infra recovery 收口、metrics 新字段在新 run 中写出证明、
   以及公开 10 样本综合验收未完成。
3. v7 是 R4-G 的阶段性正收益证据，不是 R4-G closeout。

### 5.8.6 2026-07-01 validator infra closeout 修复

v7 暴露的新阻塞不是解题错误，也不是 inspect 收敛修复失败，而是 validation 工具返回了确定性的本地基础设施错误后，TaskSpace 仍要求模型再发一轮 `state_commit` 或 `blocked` 来把该事实写回 map：

```text
taskspace-action-contract-16-run_test
Tool call failed before producing a result.
local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED
```

根因：
1. 工具反馈已经通过标准 `MainToolCall` 结果进入 TaskSpace map。
2. runtime 已有 `current_main_validation_node_local_infra_failure_summary`、`validation_node_local_infra_unvalidated_artifact_result`、`block_main_node` 等结构化能力。
3. 但自动阻塞路径只挂在 `state_commit_for_main` 后面；也就是必须等模型再请求一次 `taskspace_control(action=state_commit)`，才能把确定性的 infra 失败转成 validation node 的 blocked/closure 证据。
4. 在 `TimeoutSeconds=180` 的真实样本里，v7 到达 recovery prompt 时已经接近外层超时，导致没来得及闭环。

修复：
1. 在 `record_main_tool_result_with_class` 记录失败的 `Build`/`Test` 工具结果后，立即复用现有 local-infra 判定器检查当前 validation node。
2. 如果命中本地验证基础设施失败，则把该工具结果标记为 `ResultValidity::Invalid`，原因是它不能证明或证伪 changed artifacts。
3. 随后复用 `block_main_node` 关闭 validation node；有上游 changed artifact 时沿用现有逻辑创建 implement rework node。
4. 这不是新的旁路工具反馈机制；模型可见反馈仍来自标准工具结果，TaskSpace 只是把确定性的 map 状态提交从模型下一轮搬到 runtime。

已验证：
```text
cargo test -j1 -p codex-core local_infra_tool_result_auto_blocks_validation_node --lib
PASS

cargo test -j1 -p codex-core action_contract_run_test_local_infra_result_auto_blocks_validation --lib
PASS

cargo test -j1 -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib
PASS

cargo test -j1 -p codex-core local_infra --lib
PASS: 6 passed

cargo fmt --all --check
PASS
```

当前收益边界：
1. 已证明工程机制层面消除了“local validator infra failure 必须再等模型 state_commit 才能关闭 validation node”的额外请求依赖。
2. 该修复应降低 v7 现场那类 validation infra blocker 的超时概率，并让 map 状态更早形成 `invalid result + blocked validation node`。
3. 仍需重新构建/安装二进制并复跑 `processing-pipeline`，验证真实样本是否从 `E_ACCESSDENIED` 后的 recovery 超时变成可观测的 blocked/rework 收口。

### 5.8.7 2026-07-01 inspect 引用脚本补读与 local infra 分类收口

v8 复跑证明上一节的 local infra closeout 不是唯一阻塞。`processing-pipeline`
在 inspect 阶段已经读到 `run_pipeline.sh` 引用了 `./generate_report.sh`，但没有强制
读取该被引用脚本，模型继续反复读取已读文件，最终：

```text
processing-pipeline-v8
exec_timed_out=true
changed_paths=[]
node_count=1
open_leaf_nodes=1
```

修复：

1. runtime 在 `inspect_code_context` 节点发现已读脚本引用了未读脚本时，阻止
   list/search/re-read/apply_patch/run_test 等非目标读取动作。
2. action-contract prompt 增加 `TaskSpaceActionContractInspectMissingScriptsV1`，
   明确给出下一步必须读取的脚本目标。

验证：

```text
cargo test -j1 -p codex-core inspect_unread_referenced_script_gate_requires_missing_read --lib
PASS

cargo test -j1 -p codex-core taskspace_action_contract_inspect_missing_scripts_narrows_to_read_file --lib
PASS
```

v9 随后证明该问题被推进：TaskSpace 读到了 `generate_report.sh`，产生
`forced_inspect_transition`，并进入实现节点修改 `generate_report.sh`。但 v9 暴露出
新的分类问题：`Bash/Service/CreateInstance/E_ACCESSDENIED` 被当成“可通过实现 rework
解决”的 local infra 问题，导致 validation blocked 后又创建 implement rework。

根因：

1. `InvalidEndOfLine` 确实是可由 agent 改命令分隔符解决的 host-shell syntax failure。
2. `E_ACCESSDENIED` 是执行器/服务不可用，不是代码失败，也不是可通过 patch 代码恢复。
3. 旧逻辑把两者都归入 local validator infra，并在存在 changed artifact 时统一路由到 rework。

修复：

1. 新增 local infra recoverability 分类：只有 `InvalidEndOfLine`/statement separator
   这类命令语法错误允许进入平台兼容 rework。
2. `E_ACCESSDENIED` 这类不可恢复 executor/service failure 只把 validation result 标记
   为 `invalid`，并关闭 validation node，不创建 implement rework。
3. prompt recovery 同步区分 recoverable command failure 与 unrecoverable infra failure。
4. validation 已 blocked 且没有 active node 时增加
   `TaskSpaceActionContractClosedValidationV1`，要求直接 final/blocked，禁止重新
   `start_task/create_node`。

验证：

```text
cargo test -j1 -p codex-core access_denied_local_infra_blocks_validation_without_rework_after_changed_artifact --lib
PASS

cargo test -j1 -p codex-core local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework --lib
PASS

cargo test -j1 -p codex-core taskspace_action_contract_closed_validation_forbids_new_nodes --lib
PASS

cargo test -j1 -p codex-core local_infra --lib
PASS: 8 passed

cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

真实样本收益：

| 指标 | v8 未读引用脚本循环 | v9 误入 infra rework | v11 分类与闭环修复后 |
| --- | --- | --- | --- |
| timeout | `true` | `true` | `false` |
| changed_paths | 空 | `generate_report.sh` | `generate_report.sh` 为实际 diff |
| node_count | 1 | 4 | 3 |
| open_leaf_nodes | 1 | 1 | 0 |
| validation infra | 未到达 | `E_ACCESSDENIED` 后进入 rework | `E_ACCESSDENIED` 后 validation blocked |
| closed contract | 无 | 无 | `TaskSpaceActionContractClosedValidationV1=1` |

v11 真实运行：

```text
RunDir: C:\WhaleRunCache\r4-public10-20260701\actual\processing-pipeline-v11\runs\terminal_bench__processing-pipeline\20260701-214838-298
exec_timed_out=false
wall_time_ms=174916
node_count=3
edge_count=2
result_count=15
blocked_node_ratio=0.3333
open_leaf=0
invalid=1
rollout_tool_call_count=12
rollout_failed_tool_call_count=1
TaskSpaceActionContractClosedValidationV1=1
```

注意：pair 级别仍因外部 scoring harness 的
`e3_external_validator_fidelity_unproven` / `e3_external_validator_not_e3_eligible`
返回 PairAbort；这不是 TaskSpace 内部图闭环失败。v11 的内部证据证明 R4 当前这条
tool-feedback/control 闭环已经从真实超时收敛为可解释的 infra-blocked 完成态。

## 5.9 2026-07-01 R4-F non-direct tool path closeout

本轮把 R4-A coverage manifest 从“登记路径”升级为“登记路径 + 可运行证据”：

1. `test-r4-tool-path-coverage.ps1` 现在要求所有 `canonical` path 必须声明 `coverage_test`。
2. `r4-tool-path-coverage.json` 中 10 条路径全部为 `canonical`，`needs_fix_count=0`。
3. R4-F 的三条 non-direct path 不再只停留在静态推理：
   - CodeMode nested tool：证明 direct call 有 model-visible call id，CodeMode call 不伪装为 direct provider result，但保留 code cell parent attribution 和 raw result payload。
   - MCP tool：证明 response item 保留 wall time、structured content、content items，且大 structured content 会被截断；CodeMode 结果保持 raw `CallToolResult`。
   - multi-agent tool：新增 wrapper 单测，证明 provider-visible function output 和 CodeMode structured result 都保留 `agent_id/status/message`，并保留 `success` metadata。

已执行命令：

```text
cargo test -j1 -p codex-core multi_agent_tool_output --lib
PASS: 2 passed

cargo test -j1 -p codex-core dispatch_lifecycle_trace_records_direct_and_code_mode_requesters --lib
PASS: 1 passed

cargo test -j1 -p codex-core mcp_tool_output_response_item --lib
PASS: 3 passed

cargo test -j1 -p codex-core mcp_tool_output_code_mode_result_stays_raw_call_tool_result --lib
PASS: 1 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1
PASS: R4 tool path coverage gate passed: 10 paths
```

收益边界：

1. R4-F 可以关闭：non-direct tool path 都有明确 inclusion/exclusion 语义和 focused coverage。
2. R4-D/E 的 manifest 级缺口已经关闭，但真实收益仍以 5.3、5.7、5.8 的样本证据为准。
3. R4-G 仍不能关闭：还缺 10 个公开 benchmark 的实际 paired run/report。
4. R4-H 仍不能关闭：最终 closeout 依赖 R4-G 的 10 样本综合验收结果。

## 5.10 2026-07-02 R4-G public-10 final run

本轮完成 R4-G 公开 10 样本综合验收。最终计划文件：

```text
docs/v0.0.5/build-R4/r4-public-10-tool-stress-plan.json
```

最终报告：

```text
target/r4-public-10-tool-stress/r4-public-10-tool-stress-report.json
```

已执行门禁：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1 -RequireComplete
PASS: complete_run_count=10 missing_run_count=0

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath target/r4-public-10-tool-stress/r4-public-10-tool-stress-report.json
PASS: R4 public-10 tool-stress gate passed: 10 planned samples
```

样本调整：

1. `sanitize-git-repo` 被剔除：`setup.sh` 真实执行未 pinned 的
   `git clone https://github.com/jeffreywpli/test-secret-removal.git`，无法给出本地等价证明。
2. 替换为 `organization-json-generator`：来自同一 `terminal-bench-core 0.1.1`
   public registry subset，预检只包含 JSON schema metadata URL 和 uv validator cache。

本轮同时修复两类 external asset scanner 误判：

1. `https://localhost:8443/...` validator 本地服务端点被误判为外部网络依赖；
   修复后标记为 `local_service_endpoint`，并去除 URL 尾随标点。
2. shell heredoc 内写入 fixture 的 URL 被误判为 runtime network；修复后标记为
   `fixture_literal_endpoint`，仍记录审计但不要求远程资产证明。

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1 -RunRoot C:\WhaleRunCache\r4-public10-20260702\adapter-selftest-local-url
PASS

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1 -RunRoot C:\WhaleRunCache\r4-public10-20260702\adapter-selftest-heredoc-url
PASS
```

10 样本结果摘要：

| sample | standard | taskspace | wall x | tool x | token x | cache hit | feedback loss |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| `vim-terminal-task` | solved | solved | 1.33 | 0.60 | 3.73 | 0.9876 | 0/0 |
| `heterogeneous-dates` | solved | solved | 4.67 | 1.43 | 11.08 | 0.9856 | 0/0 |
| `sqlite-db-truncate` | solved | wrong | 1.06 | 0.40 | 3.59 | 0.9864 | 0/0 |
| `git-multibranch` | timeout | timeout | 1.00 | n/a | n/a | n/a | 0/0 |
| `git-workflow-hack` | engineering_unclean | timeout | 2.13 | 0.20 | n/a | 0.9882 | 0/0 |
| `organization-json-generator` | solved | timeout | 9.80 | 1.42 | n/a | n/a | 0/0 |
| `sqlite-with-gcov` | wrong | wrong | 0.38 | 0.15 | 0.52 | 0.9825 | 0/0 |
| `processing-pipeline` | timeout | wrong | 0.97 | n/a | n/a | 0.9810 | 0/0 |
| `csv-to-parquet` | solved | solved | 2.21 | 0.80 | 5.40 | 0.9864 | 0/0 |
| `tmux-advanced-workflow` | engineering_unclean | engineering_unclean | 1.97 | 0.41 | n/a | 0.9863 | 0/0 |

说明：`token x=n/a` 表示至少一侧 agent 被 timeout 杀掉后 provider usage 未完整落盘，
不能作为成本收益判断，只作为 timeout/收敛问题证据。`tool x=n/a` 表示 standard
侧工具调用数为 0，比例不可比；TaskSpace 绝对工具调用数仍在报告中保留。

本轮对 large-rollout 工具调用统计做了证据修正：当 rollout 因超过扫描阈值被跳过时，
报告会从 TaskSpace observability 的 `main_tool_call` 结果回退计数。修正后
`organization-json-generator` 不再误报 TaskSpace `tool_call_count=0`，而是
`standard_tool_calls=12`、`taskspace_tool_calls=17`、`tool x=1.42`，
且 `tool_call_analysis_summary` 标记 `taskspace_tool_source=observability_results`。
对应门禁：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-metrics-extractor-large-rollout.ps1 -RunRoot target/r4-metrics-extractor-large-rollout-selftest
PASS: R4 metrics extractor large rollout gate passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1 -RequireComplete
complete_run_count=10 missing_run_count=0

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath target/r4-public-10-tool-stress/r4-public-10-tool-stress-report.json
PASS: R4 public-10 tool-stress gate passed: 10 planned samples
```

真实收益结论：

1. R4-G 的 public benchmark 选择、公开 registry 校验、报告生成和报告字段门禁已经跑通；
   `-RequireComplete` 明确证明 10/10 样本均有一行结果。
2. 已知 tool feedback loss 在 10 行报告中为 `0/0`，说明当前 direct tool result
   进入 TaskSpace map/报告链路没有再出现已知的语义丢失计数。
3. DeepSeek provider cache 命中率在可统计 TaskSpace 行中约为 `0.9810-0.9882`，
   证明上下文前缀/cache 维护机制在这些长流程中基本稳定。
4. TaskSpace 解题质量和成本收益没有被证明。相反，公开 10 样本显示明显负证据：
   `organization-json-generator` standard solved 但 TaskSpace 900s timeout；
   `sqlite-db-truncate` standard solved 但 TaskSpace wrong；
   `heterogeneous-dates` 虽同解但 wall/token 分别约 4.67x/11.08x；
   `csv-to-parquet` 虽同解但 wall/token 分别约 2.21x/5.40x。

R4-G 状态可以关闭为“验收机制与证据生成完成，TaskSpace 当前收益不成立且问题已暴露”。
R4-H 关闭前必须把这些负证据转化为后续工程项，而不是声明 TaskSpace 已优于 standard。

## 5.11 2026-07-02 sqlite-db-truncate ready recovery 增量证据

本轮针对 `sqlite-db-truncate` 继续做 R4-D/R4-G 交叉验证，结论不是样本 solved，而是确认并修复了一个新的状态机缺口。

已修复问题：

1. 当 validation node 已 blocked，但同时存在 ready 的 `implement_solution`/validation recovery node 时，action-contract 不应注入 `TaskSpaceActionContractClosedValidationV1`。
2. terminal `blocked` 改写路径也必须复用同一判断，不能把 ready recovery path 伪装成 closed validation path。

代码修复：

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
third_party/codex-cli/codex-rs/core/src/session/turn.rs
```

验证：

```text
cargo test -j1 -p codex-core blocked_validation_with_ready_recovery_node_is_not_closed --lib
PASS

相关回归：
- direct_final_response_rejects_open_contract_without_validation_after_thin_work
- access_denied_bash_validation_command_routes_unvalidated_artifact_to_rework
- access_denied_local_infra_blocks_validation_without_rework_after_changed_artifact
- validation_node_blocks_vacuous_test_after_changed_artifact
- validation_rework_rejects_validator_procedure_blocker_before_edit
- validation_rework_rejects_missing_current_artifact_visibility_blocker
- manual_local_infra_validation_block_routes_unvalidated_changed_artifact_to_rework
- action_contract_prompt_structures_validator_procedure_blocker_rejection
PASS

cargo fmt --all -- --check
PASS

cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

真实复跑：

```text
RunDir:
C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-ready-recovery-fix\runs\terminal_bench__sqlite-db-truncate\20260702-101238-428\pair-001

outcome_standard=solved
outcome_taskspace=engineering_unclean
business_success=false
public_validation_skipped=true
public_validation_skip_reason=agent_exec_timeout
taskspace_wall_time_ms=900037
tool_call_count=24
rollout_trace_model_request_count=28
changed_paths=recover.py, trunc.db.recovered
nodes=6
open_leaf_nodes=1
```

收益判断：

1. ready recovery 被 closed-validation 覆盖的问题已被修复，真实复跑从 9 次工具调用推进到 24 次工具调用，并进入 `node-6` recovery path。
2. 样本仍失败，新根因转移为长流程 convergence/timeout：`python recover.py` 暴露 `PermissionError: [WinError 5]` 后，TaskSpace 在 `node-6` 上继续运行到 900s timeout。
3. 这条证据强化 R4 的 no-go 结论：R4 的 tool-chain observability 和若干状态机局部修复有效，但 TaskSpace utility 仍未收敛，后续必须进入 utility-convergence 专项。

## 5.12 2026-07-02 run_test PowerShell OR 链规范化证据

本轮继续追踪 `sqlite-db-truncate`，新暴露的问题不是预算，也不是解题错误，而是 action-contract `run_test` 对 host shell 的规范化缺口：

```text
sqlite3 trunc.db ".tables" 2>&1 || echo 'sqlite3 not available, trying python'; python -c ...
```

在 Windows PowerShell 5.1 下，顶层 `||` 会直接触发：

```text
FullyQualifiedErrorId : InvalidEndOfLine
```

修复：

```text
third_party/codex-cli/codex-rs/core/src/session/turn.rs
```

核心行为：

```text
cmd1 || cmd2; tail
=>
cmd1; if ($LASTEXITCODE -ne 0) { cmd2 }; tail
```

同时保持引号内 `||` 和 `;` 不被误切。

验证：

```text
cargo test -j1 -p codex-core run_test_normalizes --lib
PASS: 2 tests

cargo test -j1 -p codex-core taskspace_powershell_ --lib
PASS: 2 tests

cargo fmt --all -- --check
PASS

cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

真实复验：

```text
RunDir:
C:\WhaleRunCache\r4-rerun-20260702\sqlite-db-truncate-powershell-or-chain-fix\runs\terminal_bench__sqlite-db-truncate\20260702-163512-422\pair-001

left whale-exec.jsonl len=0
right whale-exec.jsonl len=0
left timeout=900s before first JSON event
right timeout=900s before first JSON event
```

收益判断：

1. H-024 的工程收益已由 action-contract 单测证明：TaskSpace 生成的 `run_test` 顶层 OR 链不会再被原样交给 PowerShell。
2. 这轮真实 pair 复验没有进入工具执行层，不能作为 H-024 真实收益证明。
3. 新现场应单独归类为 provider/model first-event timeout 或 harness 首包超时问题，不能混入 tool-chain 语义修复结论。

## 5.13 2026-07-02 public-10 timeout usage accounting 修复

本轮继续对照 R4-G/R4-H 的成本证据门禁，确认并修复一个报告层缺口：timeout 行中缺失 provider usage 时，public-10 report 曾把 token/cache 缺失默认成 `0`。这会把“未落盘/不可统计”误读成“真实 0 成本”，从而污染 TaskSpace 成本收益判断。

baseline：

```text
target/r4-public-10-tool-stress/r4-public-10-tool-stress-report.json

git-multibranch:
  taskspace_token_ratio=0
  request_2_plus_cache_hit_rate=0
  standard/taskspace metrics token_summary_availability=usage_unavailable

organization-json-generator:
  taskspace_token_ratio=0
  request_2_plus_cache_hit_rate=0
  taskspace metrics token_summary_availability=usage_unavailable
```

修复：

```text
scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1
scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1
scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1
docs/v0.0.5/build-R4/r4-public-10-tool-stress-plan.json
```

新语义：

```text
taskspace_token_ratio = null when unavailable
token_ratio_availability = measured | unavailable
standard_usage_accounting_status = measured | usage_unavailable_after_timeout | usage_source_missing | usage_unavailable
taskspace_usage_accounting_status = measured | usage_unavailable_after_timeout | usage_source_missing | usage_unavailable
request_2_plus_cache_hit_rate = null when unavailable
request_2_plus_cache_hit_rate_availability = measured | derived_from_token_summary | cache_trace_unavailable | source_missing
```

after：

```text
git-multibranch:
  token_ratio_availability=unavailable
  standard_usage_accounting_status=usage_unavailable_after_timeout
  taskspace_usage_accounting_status=usage_unavailable_after_timeout
  request_2_plus_cache_hit_rate_availability=cache_trace_unavailable

organization-json-generator:
  token_ratio_availability=unavailable
  standard_usage_accounting_status=measured
  taskspace_usage_accounting_status=usage_unavailable_after_timeout
  request_2_plus_cache_hit_rate_availability=cache_trace_unavailable
```

已执行门禁：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1 -RequireComplete
PASS: complete_run_count=10 missing_run_count=0

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath target/r4-public-10-tool-stress/r4-public-10-tool-stress-report.json
PASS: R4 public-10 tool-stress gate passed: 10 planned samples

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1
PASS: R4 public-10 usage accounting gate rejects ambiguous token usage
```

收益判断：

1. R4-G report 不再把 timeout 后未 flush 的 provider usage 伪装成 0，成本收益表的语义更准确。
2. 新 gate 能拒绝 `token_ratio_availability=measured` 但 `taskspace_token_ratio` 缺失的报告，防止回归。
3. 该修复不等于 provider timeout usage 已完整 flush；真实 token usage 仍需后续从 provider event writer 或超时回收路径补强。

## 5.14 2026-07-02 public-10 model request amplification accounting

继续审计 R4-G 成本证据时，发现 public-10 report 仍有一个报告层语义缺口：`request-summary.json` 顶层 `model_request_count` 可能来自 token summary 聚合，而 TaskSpace 的真实多轮请求数存在于 `rollout_trace.model_request_count` 或 provider cache trace 中。如果报告只暴露 token ratio，不暴露有效模型请求数，就无法解释“cache hit 很高但 token/时长仍然放大”的根因。

现场证据：

```text
heterogeneous-dates:
  standard request-summary model_request_count=1
  taskspace request-summary top-level model_request_count=1
  taskspace rollout_trace.model_request_count=12
  taskspace provider_cache_trace.provider_request_count=11
  taskspace_token_ratio=11.082
  request_2_plus_cache_hit_rate=0.98556
```

修复：

```text
scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1
scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1
scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1
docs/v0.0.5/build-R4/r4-public-10-tool-stress-plan.json
docs/v0.0.5/build-R4/04-benefit-gates-and-public-sample-acceptance.md
```

新增字段：

```text
standard_model_request_count
taskspace_model_request_count
taskspace_model_request_ratio
standard_model_request_count_source
taskspace_model_request_count_source
model_request_count_availability
```

计数来源优先级：

```text
rollout_trace.model_request_count
provider-cache-trace-summary.provider_request_count
request-summary.model_request_count
metrics.model_request_count
```

更新后的 public-10 关键行：

```text
vim-terminal-task: standard=1 taskspace=6 ratio=6 cache_hit=0.987628 token_ratio=3.734
heterogeneous-dates: standard=1 taskspace=12 ratio=12 cache_hit=0.985560 token_ratio=11.082
sqlite-db-truncate: standard=1 taskspace=9 ratio=9 cache_hit=0.986363 token_ratio=3.5924
git-workflow-hack: standard=1 taskspace=21 ratio=21 cache_hit=0.988228 token_ratio=unavailable
sqlite-with-gcov: standard=1 taskspace=18 ratio=18 cache_hit=0.982483 token_ratio=0.5248
csv-to-parquet: standard=1 taskspace=8 ratio=8 cache_hit=0.986379 token_ratio=5.4041
tmux-advanced-workflow: standard=1 taskspace=28 ratio=28 cache_hit=0.986285 token_ratio=unavailable
```

已执行门禁：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/write-r4-public-10-tool-stress-report.ps1 -RequireComplete
PASS: complete_run_count=10 missing_run_count=0

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath target/r4-public-10-tool-stress/r4-public-10-tool-stress-report.json
PASS: R4 public-10 tool-stress gate passed: 10 planned samples

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1
PASS: R4 public-10 usage accounting gate rejects ambiguous token usage
```

收益判断：

1. R4-G report 现在能证明高 cache hit 不等于低成本；TaskSpace 的主要成本放大来自多轮请求放大。
2. gate 会拒绝 `model_request_count_availability=measured` 但缺少 `taskspace_model_request_ratio` 的报告，防止请求放大被再次隐藏。
3. 这不是 TaskSpace utility 修复；它把剩余 P0 问题更准确地收敛为 long-flow convergence 和请求轮数控制。

## 5.15 2026-07-02 inspect 成功诊断后的低价值读取收敛修复

基于 5.14 暴露的请求放大证据，继续分析 `heterogeneous-dates`。该样本 standard 和 TaskSpace 都 solved，因此适合排除 correctness 干扰，单独观察请求放大。

现场证据：

```text
RunDir:
C:\WhaleRunCache\r4-public10-20260702\actual\heterogeneous-dates-v1\runs\terminal_bench__heterogeneous-dates\20260702-042837-780\pair-001

standard_model_request_count=1
taskspace_model_request_count=12
taskspace_token_ratio=11.082
request_2_plus_cache_hit_rate=0.98556
```

TaskSpace inspect node 的关键结果：

```text
result-3: read task-deps/daily_temp_sf_high.csv success
result-4: run_test success, output 11.428571428571429
result-5: re-read task-deps/daily_temp_sf_high.csv success
result-6: read daily_temp_sf_low.csv at wrong root failed
result-7: forced inspect transition after inspect_no_action_with_evidence
```

根因：

`should_finish_node_after_successful_required_action(...)` 已经覆盖 `implement_solution` 和 validation 节点：成功 edit/test 后，后续低价值动作会被转成 `finish_node`。但 `inspect_code_context` 没有同类语义收敛路径；inspect 只有 duplicate diagnostic、no-action recovery、progress pressure 等后置路径，导致已具备成功诊断和工作证据后仍可继续 read/search。

修复：

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
third_party/codex-cli/codex-rs/core/src/session/mod.rs
third_party/codex-cli/codex-rs/core/src/session/turn.rs
```

行为变化：

```text
inspect_code_context
  if successful diagnostic exists
  and working read/search evidence exists
  and no unread referenced script blocks convergence
  and next action is list_files/search/read_file
=> rewrite to taskspace_control(action=finish_node, next_node_kind=implement_solution)
```

验证：

```text
cargo fmt --all -- --check
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core inspect_successful_diagnostic_and_working_evidence_marks_convergence_ready --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_finish_inspect_to_implementation_action_builds_next_node --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core taskspace_action_contract_finish_node --lib
PASS
```

未完成验证：

```text
CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-core
TIMEOUT after 604s
```

收益判断：

1. 工程收益已由 focused tests 证明：inspect 节点具备成功诊断和工作证据后，后续低价值 read/search 会进入 finish-to-implement，而不是继续消耗请求。
2. 真实样本收益尚未证明，因为新 Whale 二进制未能在本轮构建完成，不能用旧 bin 复验。
3. 该修复仍保持非硬预算原则：它不限制复杂任务继续做新的必要诊断，只收敛成功诊断后的低价值读取。

## 5.16 2026-07-02 final readiness 收尾证据接受修复

在 inspect convergence 修复后，`heterogeneous-dates` 的真实复跑暴露了一个更靠后的收尾问题：TaskSpace 已经完成实现和验证，public validation 也能通过，但 `final_answer` 被 readiness gate 持续拒绝，最终跑到 900s timeout。

失败现场：

```text
RunDir:
C:\WhaleRunCache\r4-inspect-convergence-heterogeneous-20260702-minfree15\runs\terminal_bench__heterogeneous-dates\20260702-180700-127\pair-001

outcome_standard=solved
outcome_taskspace=agent_exec_timeout
failure_taxonomy=engineering_unclean, taskspace_overhead_timeout, audit_unclean
standard_wall_ms=86114
taskspace_wall_ms=900039
public_validation_exit_code_standard=0
public_validation_exit_code_taskspace=0
taskspace_changed_paths=avg_temp.txt, solve.py
```

根因：

`force_finish_validation_after_successful_tool(...)` 只把 validation 节点收尾，没有接受直接依赖的 `implement_solution` edit/lifecycle 证据，也没有接受 forced validation closeout 自己生成的 lifecycle result。final readiness gate 因此正确拒绝 final answer，但图上已经没有可操作的当前节点，形成无效循环。

修复：

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
```

- forced validation closeout 前接受直接依赖实现节点上的成功 edit evidence。
- 同时接受直接依赖实现节点上的 lifecycle result。
- validation 节点 finish 后接受 forced validation closeout lifecycle result。

验证：

```text
cargo fmt --all -- --check
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core forced_validation_closeout_accepts_dependency_edit_for_final_readiness --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core force_finish_validation_after_successful_tool_closes_smoke_node --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core finish_final_synthesis_accepts_open_behavior_after_accepted_fix_and_validation --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

真实复验：

```text
RunDir:
C:\WhaleRunCache\r4-final-readiness-heterogeneous-20260702\runs\terminal_bench__heterogeneous-dates\20260702-185101-849\pair-001

outcome_standard=solved
outcome_taskspace=engineering_unclean
failure_taxonomy=engineering_unclean, audit_unclean
standard_wall_ms=42043
taskspace_wall_ms=229459
standard_exec_timed_out=false
taskspace_exec_timed_out=false
public_validation_exit_code_standard=0
public_validation_exit_code_taskspace=0
taskspace_model_request_count=16
request_2_plus_cache_hit_rate=0.988319
active_context_replacement_confirmed=true
legacy_taskspace_history_present=false
taskspace_control_count=7
```

收益判断：

1. 真实收益成立：同一类现场从 900s timeout 降为 229s 非 timeout 完成，public validation 继续通过。
2. active context replacement 与 taskspace control usage 在该轮均能被报告证实，不再是不可观测状态。
3. 该修复没有让样本达到最终 `solved`，因为后续又暴露 validation path-error classification 污染；因此它只证明 final readiness 收尾收益，不证明 TaskSpace utility parity。

## 5.17 2026-07-02 known input path validation 误分类修复

5.16 的真实复验虽然不再 timeout，并且 public validation 通过，但仍返回 `engineering_unclean`。继续追踪后发现，blocked graph 主要来自 validation 命令在错误工作目录引用 `daily_temp_sf_high.csv`，而 TaskSpace map 已经知道真实输入路径是 `task-deps/daily_temp_sf_high.csv`。

根因：

`validation_node_failed_noninfra_result(...)` 把 validation 阶段的 `FileNotFoundError` 统一视为非 infra validation blocker，没有区分：

- 真实缺少实现产物或未知输入。
- validator 命令使用了已知输入 artifact 的 basename，但没有使用 map 中已经记录的真实路径。

修复：

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
```

- 新增 `validation_failure_is_known_input_path_error(map, result)`。
- 当 stderr 中的缺失 basename 已经能映射到 map 里的已知 artifact path 时，不把它升级为 implementation rework。
- 保留未知文件缺失的原有 rework 行为。

验证：

```text
cargo fmt --all -- --check
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_known_input_path_error_stays_on_validation_node --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_node_failed_test_blocks_repeated_validation --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

真实复验：

```text
RunDir:
C:\WhaleRunCache\r4-validation-path-heterogeneous-20260702\runs\terminal_bench__heterogeneous-dates\20260702-190925-951\pair-001

outcome_standard=wrong
outcome_taskspace=engineering_unclean
failure_taxonomy=engineering_unclean, agent_patch_wrong, audit_unclean
standard_wall_ms=56673
taskspace_wall_ms=404609
public_validation_exit_code_standard=1
public_validation_exit_code_taskspace=1
```

收益判断：

1. 工程收益由 focused regression 证明：已知输入 artifact 的 basename path error 不再被误判为实现失败。
2. 真实样本收益暂不能确认：这一轮 standard 也失败，且 TaskSpace 生成了无效 Python，属于被模型随机错误污染的复验。
3. 下一步需要继续跑非污染样本，才能把该修复升级为真实收益证明。

## 5.18 2026-07-02 validation closeout ledger adoption 修复

继续复跑 `heterogeneous-dates` 后，又暴露出一个更精确的收尾缺口：模型已经产生合法 `final_answer`，`avg_temp.txt` 已写出，public validation 和 hidden oracle 都能通过，但 TaskSpace 仍持续请求模型直到 timeout。原因不是工具失败，也不是解题错误，而是原始 user criteria / output contract 没有被验证证据采纳。

失败现场：

```text
RunDir:
C:\WhaleRunCache\r4-h028-rerun-heterogeneous-20260702\runs\terminal_bench__heterogeneous-dates\20260702-192443-140\pair-001

outcome_standard=solved
outcome_taskspace=agent_exec_timeout
failure_taxonomy=engineering_unclean, taskspace_overhead_timeout, audit_unclean
taskspace_wall_ms=900034
public_validation_exit_code_taskspace=0
hidden_oracle_exit_code_taskspace=0
taskspace_changed_paths=avg_temp.txt
provider_request_count=57
```

关键证据：

```text
criterion-1 status=open evidenceRefs=[]
criterion-2 status=open evidenceRefs=[]
criterion-3 status=open evidenceRefs=[]
output-contract-1 status=open
sc-node-3-validation-pass status=satisfied result-7
```

根因：

`force_finish_validation_after_successful_tool(...)` 已经接受 implementation edit 和 validation result，但只新增了 node-local validation criterion，没有把已被实现和验证共同证明的原始验收项更新为 `satisfied`。因此 final readiness gate 仍认为“用户验收项未完成”。

修复：

```text
third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs
```

- 新增 validation-closeout ledger adoption。
- 在 accepted implementation + accepted validation 同时成立时，自动将可由 validated artifact 证明的 open criteria 更新为 `satisfied`。
- 自动采纳范围只包括：`test`、`validator`、`artifact`、`behavior`、`user_visible_output`。
- 不自动满足 `performance` / `compatibility` 等不能由一次 smoke validation 直接证明的验收项。
- satisfied criteria 同时引用 implementation result 和 validation result，保留 why-chain。

验证：

```text
cargo fmt --all -- --check
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core forced_validation_closeout_satisfies_open_user_criteria_for_final_answer --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core forced_validation_closeout_accepts_dependency_edit_for_final_readiness --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core force_finish_validation_after_successful_tool_closes_smoke_node --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_known_input_path_error_stays_on_validation_node --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo test -j1 -p codex-core validation_node_failed_test_blocks_repeated_validation --lib
PASS

CARGO_TARGET_DIR=D:\BuildCache\whalecode\cargo-target cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

真实复验：

```text
RunDir:
C:\WhaleRunCache\r4-ledger-adoption-heterogeneous-20260702\runs\terminal_bench__heterogeneous-dates\20260702-195745-535\pair-001

outcome_standard=solved
outcome_taskspace=solved
failure_taxonomy=engineering_unclean, audit_unclean
standard_wall_ms=57668
taskspace_wall_ms=105841
standard_exec_timed_out=false
taskspace_exec_timed_out=false
public_validation_exit_code_standard=0
public_validation_exit_code_taskspace=0
hidden_oracle_exit_code_standard=0
hidden_oracle_exit_code_taskspace=0
standard_tool_call_count=13
taskspace_tool_call_count=6
taskspace_tool_call_ratio=0.46
taskspace_wall_time_ratio=1.84
```

观测证据：

```text
accepted results=5
final artifacts=1
cognitive hard gate=True
finalArtifactMissingWhyChainCount=0
nonAcceptedFinalArtifactDependencyCount=0
criterion-1 / criterion-2 / output-contract-1 在 validation closeout 后更新
rollout_trace.model_request_count=9
provider_request_count=8
request_2_plus_hit_rate=0.981959
```

收益判断：

1. 真实收益成立：同一个公开样本从 900s timeout 变成 solved，public validation 和 hidden oracle 都通过。
2. 工具效率收益成立：TaskSpace tool_call_count=6，standard=13，比例 0.46。
3. 时长仍比 standard 慢，taskspace_wall_time_ratio=1.84，但已经低于该报告的 wall-time warning 阈值。
4. pair report 仍有 `engineering_unclean`，原因是 E3 外部 validator fidelity / audit review 未完成，不是 TaskSpace 执行失败。

## 5.16 2026-07-03 timeout usage accounting fallback

继续收口 R4-H evidence durability 时，确认 public-10 报告还有一个 timeout 成本语义缺口：当 `metrics.json` 顶层 `input_tokens` / `output_tokens` 缺失，但 `request-summary.json` 或 metrics 中已经保留 rollout `token_count` 聚合时，报告仍可能把该行归为 usage unavailable。

本轮修复：

- `write-r4-public-10-tool-stress-report.ps1` 新增 token accounting fallback。
- 顶层 provider token summary 可用时仍标为 `measured`。
- 顶层 token summary 不可用但 rollout `token_count` 可用时，填充 input/output token 和 token ratio，并标为 `recovered_from_rollout_trace`。
- missing run 的 token/cache 字段从 `0` 改成 `null`，避免把缺失证据误读为真实 0 成本。
- `test-r4-public-10-tool-stress-plan.ps1` 允许并验证 `recovered_from_rollout_trace` 状态。
- `test-r4-public-10-usage-accounting-gate.ps1` 新增 synthetic timeout pair，证明 writer 会从 rollout trace 恢复 partial usage。

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1
PASS

synthetic heterogeneous-dates:
token_ratio_availability=recovered_from_rollout_trace
taskspace_token_ratio=3
standard_usage_accounting_status=recovered_from_rollout_trace
taskspace_usage_accounting_status=recovered_from_rollout_trace
```

边界：

- 这是报告层和 evidence gate 的修复，不证明 TaskSpace utility parity。
- 如果 provider 进程在写出 `response.completed` 或 rollout `token_count` 前被 kill，exact usage 仍不可得；该情况仍必须显式标为 unavailable，而不是伪装成 0。

## 5.17 2026-07-03 R4 acceptance readiness gate

为避免 R4-H 继续依赖人工串读多个 gate 和文档，本轮新增统一 readiness gate：

```text
scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1
```

该 gate 聚合：

- tool path coverage
- sample ledger
- public-10 snapshot gate
- usage accounting gate
- external wrapper harness
- DeepSeek provider credential preflight state

当前主机复验结果：

```text
status=blocked
engineering_gates_status=pass
provider_credential_status=missing
e3_readiness=not_ready_until_real_utility_evidence_passes
gate_count=5
failed_gate_count=0
blocker=provider_credential_missing
```

输出：

```text
target/r4-acceptance-readiness/r4-acceptance-readiness.json
```

解释：

- R4 工程 readiness 当前可由一个 JSON artifact 证明。
- 该 gate 不会把 R4 判成完成；缺 `DEEPSEEK_API_KEY` 时以 exit code `3` 明确阻断。
- key 配置后，该 gate 应先变为 `ready_for_real_utility_rerun`，再继续真实 public sample 复验。

## 5.18 2026-07-03 keyed organization-json-generator 复验

用户提供 `.env.local` 中的 `DEEPSEEK_API_KEY` 后，R4 readiness gate 从 credential blocked 进入可真实复验状态。key 只通过当前 shell 环境传给子进程，不进入仓库、报告或日志摘要。

### 5.18.1 readiness gate

```text
set -a; . ./.env.local; set +a; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1
R4 acceptance readiness passed for real utility rerun
ReadinessReport: target/r4-acceptance-readiness/r4-acceptance-readiness.json
```

解释：

- lightweight engineering gates pass。
- provider credential preflight pass。
- 这只表示“可以重跑真实 utility”，不表示 R4 已验收。

### 5.18.2 metrics extractor durability repair

首次 keyed `organization-json-generator` 真实执行进入 model run 后，在 post-processing 阶段失败：

```text
Get-Item: scripts/taskspace-benchmark/lib/metrics-extractor.ps1:173
Could not find item .../pair-001/left/app/.python-version.
```

根因：`Add-TaskspaceChangedPath` 在 `Test-Path` 后、进入 retry/catch 前执行 `Get-Item`，文件若在这段窗口消失会直接终止 metrics extraction。

修复：

- `Get-Item` 移入原有 retry/catch 块。
- `PathNotFound` / `ItemNotFound` / `Could not find item` / `Cannot find path` 归一为 `hash_status=missing`。
- `test-metrics-extractor-harness.ps1` 增加 vanished changed path 断言。

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-metrics-extractor-harness.ps1
TaskSpace metrics extractor harness self-test: PASS
```

第二次 keyed run 的真实 metrics 也记录：

```text
.python-version[??] hash_status=missing
```

### 5.18.3 keyed rerun utility result

第二次 keyed rerun 成功写出完整 paired artifacts：

```text
RunDir: target/r4-org-json-real-keyed-20260703b/runs/terminal_bench__organization-json-generator/20260703-155610-406
PairReport: pair-001/pair-report.md
reported_evidence_level: E1
included_in_utility_aggregate: False
```

核心结果：

| Side | Result |
| --- | --- |
| standard | `exec_exit_code=0`，`public_validation_exit_code=1`，`hidden_oracle_exit_code=0`，`business_success=False`，`outcome_standard=wrong` |
| taskspace | `exec_exit_code=124`，`exec_timed_out=True`，`wall_time_ms=900088`，`tool_call_count=92`，`business_success=False`，`outcome_taskspace=agent_exec_timeout` |

TaskSpace diagnostic signals：

```text
TaskSpaceProviderRequestBudgetEventV1 ... request_count=89->90 max=20 state=over_profile_hint
TaskSpaceNoActionRecoveryV1 ... Recovery attempt 32 ... advisory threshold 3
bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted
```

判断：

1. R4 的 provider credential blocker 已解除；`.env.local` 配置方式可用。
2. R4 仍不能验收：该 run 是 E1 诊断证据，不进入 utility aggregate，也不满足 E3。
3. 当前 P0 blocker 是 TaskSpace 在 sandbox/tool failure 后不能收敛到 bounded blocked-with-evidence，导致 request amplification 和 900s timeout。
4. 下一步应优先做 request budget hard gate 或 repeated no-action recovery hard stop，再重跑 `organization-json-generator`。

## 5.19 2026-07-03 tool-runtime bootstrap feedback 修复

5.18 的 `bwrap` 现场被收录为 R4-D P0 path：`tool-runtime-bootstrap-failure`。根因判断是反馈层语义缺失：
raw error 已经进入工具输出，但 TaskSpace 没有把它升级为 `sandbox_bootstrap_failed` 任务级 blocker，
导致 node-level `blocked` 之后仍可继续 `create_node` 并重复 recovery。

修复内容：

| Layer | Change |
|---|---|
| ability detection | `exec.rs` 增加 bwrap loopback/RTM_NEWADDR bootstrap signature 测试，保持 `SandboxType::None` 不误判 |
| map feedback | `ActionMapRuntime` 新增 `tool_runtime_bootstrap_failure` 分类、trace tag、task-level blocker summary |
| validation path | bwrap validation failure 继续按 local validator infra invalidation 处理，但不生成可重试 rework node |
| next-turn contract | 新增 `TaskSpaceActionContractToolRuntimeBootstrapFailureV1`，无 active node 时只允许 `final_answer` 或 `blocked` |
| manifest | `r4-tool-path-coverage.json` 增加第 11 条 canonical path |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core bootstrap_failure --lib
4 passed

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core local_infra --lib
11 passed

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1
R4 tool path coverage gate passed: 11 paths
```

同步轻量 gates：

```text
jq empty docs/v0.0.5/build-R4/r4-tool-path-coverage.json docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-sample-ledger.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-tool-stress-plan.ps1 -ReportPath docs/v0.0.5/build-R4/r4-public-10-tool-stress-report.snapshot.json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-public-10-usage-accounting-gate.ps1
set -a; . ./.env.local; set +a; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-acceptance-readiness.ps1
git diff --check
```

结论：该 case 的 feedback-layer 语义已修复并纳入 R4 coverage；R4 验收仍需真实 keyed rerun 证明
`organization-json-generator` 不再在同类 tool-runtime failure 上 900s timeout。

## 5.20 2026-07-03 R4-D tools feedback 子类型收敛

本轮继续沿 `organization-json-generator` 真实 keyed run 暴露的问题推进。结论是：多个 case 的 raw signal
已经存在，但 feedback 层或 phase gate 没有把它变成正确的下一步语义。因此本节只记录 R4-D 工程收益，不把
R4-G utility parity 标记为通过。

| Case | Before | After | Evidence |
|---|---|---|---|
| host-platform read recovery | recovery payload 可能给出非当前主机命令 | `read_file` recovery 按 host platform 生成 `sed` 或 `Get-Content` | `cargo test -j1 -p codex-core host_platform_command --lib` |
| duplicate read/search | inspect 重复成功 read/search 后可能继续重复或过早推进 | named gate `inspect_duplicate_successful_read_or_search`，带 previous result 和 repeat state | `inspect_node_blocks_repeated_successful_read_command`; `duplicate_read_search_recovery_pushes_inspect_transition` |
| data artifact evidence | `.json`/`.csv` 读入不稳定计为 working evidence | command/body/evidence refs 合并 input data artifacts | `inspect_data_artifact_read_counts_as_working_evidence` |
| validation coverage | 不覆盖 changed artifact 的 validation 被拒绝，但反馈不够可执行 | `validation_test_missing_changed_artifact_coverage` 带 required command / next action | `action_contract_prompt_structures_changed_artifact_coverage_failure` |
| missing validation script | `python process.py` 不存在脚本被当成 implementation failure | `validation_command_missing_script` 留在 validation node | `validation_missing_command_script_stays_on_validation_node` |
| missing fact-source coverage | 重复 `departments.csv` 后强制进入 implement，漏读 `employees.csv`/`projects.csv` | duplicate gate 列出缺失 fact-source artifacts；manual/forced inspect finish 都被 coverage guard 拦截 | `inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish`; `inspect_missing_fact_sources_block_manual_and_forced_finish_until_read` |
| restricted Linux sandbox | bwrap netns/proc/userns 限制可能在业务命令前失败 | sandbox preflight/fallback 区分 recoverable ability 降级和 terminal bootstrap failure | `cargo test -j1 -p codex-linux-sandbox --lib`; sandbox smoke |
| provider budget overrun | active budget 到达上限后仍继续 provider sampling，`over_profile_hint` 只是 telemetry | provider dispatch 前执行 hard gate；插入 `TaskSpaceProviderBudgetHardStopV1` 并结束当前 turn；保留一次 `budget_recovery` grace | `cargo test -j1 -p codex-core provider_budget --lib`; `cargo test -j1 -p codex-core taskspace_active_budget --lib` |
| premature inspect node hard stop | per-node hard limit 低于声明 fact-source evidence floor，导致 inspect 未读全输入就停止 | inspect 节点的 effective per-node limit 按声明 fact-source artifacts 扩展；边界 recovery 请求标记为 `budget_recovery` | `taskspace_active_budget_expands_inspect_node_limit_for_fact_sources`; `provider_budget_limit_reached_detects_rollout_or_node_limit` |

本轮新增的关键语义边界：

1. `taskspace runtime` 可以负责工具可用性检测、反馈分类、next_valid_actions 和 phase completion guard。
2. 它不能把“重复成功证据”解释为“inspect 已完成”；完成条件必须包含声明 fact source coverage。
3. 如果缺失 `fact_sources` 中的 artifact，反馈层应继续要求 read/search 这些 artifact，而不是提供 `finish_node`。
4. validation 命令错误要先区分“验证命令写错”和“实现代码失败”，否则会把工具反馈错误路由到 implement rework。
5. provider budget 是 runtime 控制边界，不是普通提示词；到达 hard limit 后必须在 dispatch 前停止，而不是继续让模型“自觉”收敛。
6. hard limit 不能低于 phase completion 的最低证据地板；否则会把 timeout 修成过早 blocked/wrong。

本轮验证命令：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources_block_manual_and_forced_finish_until_read --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_node_blocks_repeated_successful_read_command --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core forced_inspect_transition_accepts_duplicate_read_search_gate_recovery --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_data_artifact_read_counts_as_working_evidence --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core duplicate_read_search_recovery_pushes_inspect_transition --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_successful_diagnostic_and_working_evidence_marks_convergence_ready --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_budget --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
```

仍未关闭：

- `organization-json-generator` utility evidence：需要基于新二进制重跑 keyed sample，确认是否还会 900s timeout 或进入新的 failure class。
- repeated no-action terminal policy：如果 hard budget stop 后仍在新 turn 或恢复路径形成无效循环，需要另建 case，把 repeated no-action recovery 超阈值升级为 bounded blocked-with-evidence。

## 5.21 2026-07-04 implementation rework feedback evidence join

adaptive inspect budget 后的 keyed rerun 继续把 `organization-json-generator` 推进到下一层 failure class：

```text
RunDir: target/r4-org-json-real-keyed-20260703f-adaptive-budget/runs/terminal_bench__organization-json-generator/20260704-001749-411
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 16
```

关键观察：

| Signal | 结论 |
|---|---|
| inspect 已读 `employees.csv`、`departments.csv`、`projects.csv` 和 schema | `provider-node-budget-premature-inspect-stop` 已越过真实 rerun 的 inspect blocker |
| `generate_organization.py` 顶层整体带缩进，先后触发 line 1 / line 2 `IndentationError` | validation failure 可见，但 rework feedback 没有要求系统性修复整文件或整块缩进 |
| replacement 使用未观察字段 `salary`，最终 `KeyError: 'salary'` | recovery 没有把上游 CSV/schema evidence 和 validation failure 合并成同一行动上下文 |
| 最终 `TaskSpaceProviderBudgetHardStopV1 request_count=20/20` | hard stop 正常工作，但 request budget 被低质量 rework 消耗完 |

本轮收录并 focused 修复的新问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `implementation-rework-feedback-evidence-join` | rework node 能看到 validation failure，但 recovery summary 可能只保留直接依赖或单一 fallback，缺少上游 inspect 数据字段；模型逐行修 `IndentationError` 并凭空使用 `salary` | `current_main_working_evidence_summary()` 使用当前节点有界依赖闭包，合并 `validation_rework` 与 inspect data evidence；`TaskSpaceImplementNeedsEditRecoveryV1` 明确 validation failure 优先、Python 顶层缩进按文件/块整体修、`KeyError` 只能用已观察字段 | `validation_rework_summary_merges_transitive_inspect_evidence_and_failure`; `implement_recovery_prioritizes_validation_failure_and_inspected_fields` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_summary_merges_transitive_inspect_evidence_and_failure --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core implement_recovery_prioritizes_validation_failure_and_inspected_fields --lib
  PASS
```

状态：该 feedback-layer class 已 focused fixed；R4-G utility 仍未通过，需要再次 keyed rerun 验证是否越过 implement rework，或暴露下一层问题。

## 5.22 2026-07-04 inspect projection fact-source guard

rework evidence join 修复后的 keyed rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703g-rework-evidence/runs/terminal_bench__organization-json-generator/20260704-003459-046
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 11
```

关键观察：

| Signal | 结论 |
|---|---|
| TaskSpace 读了 `schema.json`、`departments.csv`、`employees.csv`，但没有读 `projects.csv` | fact-source coverage 仍未完成 |
| `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded request_count=11/20 node_request_count=11/10` | hard stop 正常截断，没有退回 900s timeout |
| final projection 的 `verified_input_evidence` 只列出前三个输入 | runtime 已有足够状态判断缺 `projects.csv` |
| 同一 projection 的 `next_valid_actions` 仍包含 `finish_node -> implement_solution` | provider-visible projection 缺少 fact-source coverage guard，向模型暴露了错误的合法下一步 |

本轮新增并 focused 修复的新问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `inspect-projection-finish-before-fact-source-coverage` | 底层 duplicate/manual/forced finish guard 已能识别缺 fact source，但 `projection_next_valid_actions` 不接收 `TaskState`，因此只要 inspect 有任意 tool result 就广告 `finish_node -> implement_solution` | context projection 调用 `projection_next_valid_actions(..., Some(task))`；inspect 节点缺声明 fact-source artifacts 时只输出“读取缺失 artifact”的 next action，不再提示 finish/implement | `projection_blocks_inspect_finish_until_declared_fact_sources_read`; `projection_prioritizes_inspect_to_implement_after_evidence`; `inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish`; `inspect_missing_fact_sources_block_manual_and_forced_finish_until_read` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_blocks_inspect_finish_until_declared_fact_sources_read --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_prioritizes_inspect_to_implement_after_evidence --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources_block_manual_and_forced_finish_until_read --lib
  PASS
```

状态：该 projection-layer feedback class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证 TaskSpace 是否会读取
`projects.csv` 并进入 implement，或暴露下一层 long-flow 问题。

## 5.23 2026-07-04 editable validation failure misblock

projection fact-source guard 修复后的 keyed rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703h-projection-factsource/runs/terminal_bench__organization-json-generator/20260704-004643-993
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 10
nodes: 6
edges: 5
open_leaf_nodes: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| TaskSpace 读了 `schema.json`、`departments.csv`、`employees.csv`、`projects.csv` | 上一轮 projection/fact-source blocker 已越过 |
| TaskSpace 创建 `generate_organization.py` 并执行 `python generate_organization.py` | 已进入 implement/validation 链路 |
| validation 先后报 `IndentationError` line 2 / line 3 | 这是可编辑实现错误，不是 validator infra |
| rework 节点接受 `block_node`，最终 blocked reason 写入 `closed validation state prevents further editing` / `infra-evidence-unresolved-indentation` | control/feedback 层把实现错误错误升级成 terminal blocker |
| public validation 报 `/app/organization.json does not exist` | 目标输出 artifact contract 未满足 |

本轮新增并 focused 修复的新问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `implementation-editable-validation-failure-misblocked` | implement rework 的依赖 validation 已明确指向 `IndentationError` / `SyntaxError` / `KeyError` 等可编辑实现失败，但 `block_node` 可把它说成 infra/closed validation，导致不再继续 patch | `block_main_node` 在 rework node 无 successful edit 且依赖 validation evidence 是可编辑失败时拒绝 block；action-contract recent feedback 输出 `editable_validation_failure_blocker_rejected`，下一步必须 patch 失败 artifact，Python 顶层缩进/语法错误按文件或块整体修 | `validation_rework_rejects_editable_validation_failure_blocker_before_edit`; `action_contract_prompt_structures_editable_validation_failure_blocker_rejection` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_editable_validation_failure_blocker_before_edit --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_editable_validation_failure_blocker_rejection --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_validator_procedure_blocker_before_edit --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_missing_current_artifact_visibility_blocker --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core implement_recovery_prioritizes_validation_failure_and_inspected_fields --lib
  PASS
```

状态：该 control/feedback class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证 TaskSpace 是否会继续 patch
`generate_organization.py`，生成 `organization.json`，并通过 public validation。

补充真实复验：

```text
RunDir: target/r4-org-json-real-keyed-20260703i-editable-blocker/runs/terminal_bench__organization-json-generator/20260704-005922-113
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 8
```

该 run 证明同一问题类型仍未在所有 provider 文案下关闭：模型最终使用
`Test failed with IndentationError; cannot read files to diagnose because read actions are not allowed in current narrowed state`
作为 `block_node` reason，runtime 接受了 blocker。已把 `cannot read`、`read actions are not allowed`、
`read restriction`、`insufficient information`、`current narrowed state` 纳入
`implementation-editable-validation-failure-misblocked` 的 blocker detector，并用真实文案更新 focused test。

新增验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_editable_validation_failure_blocker_before_edit --lib
  PASS
```
