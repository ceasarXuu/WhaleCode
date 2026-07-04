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

## 5.24 2026-07-04 validation closeout output-contract coverage gap

editable blocker wording 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703j-editable-wording/runs/terminal_bench__organization-json-generator/20260704-010752-603
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 8
```

关键观察：

| Signal | 结论 |
|---|---|
| TaskSpace 创建 `generate_json.py` | 已越过 inspect / editable blocker，进入生成实现 |
| validation 只运行 `python generate_json.py`，stdout 为 `organization.json generated successfully.` | 工具成功只证明 generator 执行成功 |
| runtime 随后触发 `TaskSpaceForcedValidationCloseoutV1 trigger=validation_success_after_tool_drain` | closeout 将 execution success 升级成 validation success |
| final answer 声称 artifact followed `schema.json` | 反馈层把未验证的契约当成已满足 |
| public validator 报 `KeyError: 'members'`、`KeyError: 'averageDepartmentBudget'` | 输出 contract 实际未满足；生成 JSON 使用 `member_ids` 和 snake_case statistics |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-closeout-output-contract-coverage-gap` | validation node 的 successful tool result 只要 exit 0 就可被 forced closeout 接受；`python generate_json.py` 被解释成“输出契约通过” | validation gate 对声明 output contract artifacts 增加覆盖检查；generator-only command 被拒绝，要求同一次 `run_test` 执行变更脚本并验证 output/schema，或运行真实项目 validator；forced closeout 备份路径会重开引用该结果的 success criteria 并将 generator-only result 标记 invalid | `validation_node_blocks_generator_only_command_for_schema_output_contract`; `force_finish_validation_rejects_generator_only_output_contract_success`; `action_contract_prompt_structures_output_contract_coverage_failure`; `validation_output_contract_coverage_recovery_preserves_next_action` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks_generator_only_command_for_schema_output_contract --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation_rejects_generator_only_output_contract_success --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt_structures_output_contract_coverage_failure --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_output_contract_coverage_recovery_preserves_next_action --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  PASS：77 tests
```

状态：该 validation/feedback class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证 TaskSpace 是否会使用
`python generate_json.py && python -m jsonschema -i organization.json schema.json` 或真实 public-equivalent validator，而不再
generator-only closeout。

## 5.25 2026-07-04 schema fact-source weak validation gap

output-contract coverage 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703k-output-contract/runs/terminal_bench__organization-json-generator/20260704-013819-201
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 9
```

关键观察：

| Signal | 结论 |
|---|---|
| `python process.py` 被 `validation_test_missing_output_contract_coverage` 拒绝 | H-014 的 generator-only guard 已在真实 run 中生效 |
| session 插入 `TaskSpaceValidationNeedsTestRecoveryV1` | 失败语义已传给模型，不是 feedback 丢失 |
| 模型改跑 `python process.py && python -c 'import json; data=json.load(open("organization.json")); print("Valid")'` | 模型响应了 feedback，但 validation 语义仍过弱 |
| runtime 接受该命令，final answer 声称 “validated successfully against the schema” | coverage predicate 把 JSON parse 当成 schema validation |
| public validator 仍报 `KeyError: 'members'`、`KeyError: 'averageDepartmentBudget'` | 输出 contract 未满足；`memberIds` 和缺失统计字段仍未被本地 validation 捕获 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-output-contract-schema-fact-source-gap` | output contract 只声明 `organization.json`，`schema.json` 作为 fact source / success criterion 出现时，coverage gate 只要求命令提到 output artifact 并具有任意 validation marker；`json.load` / 普通 `python -c` 可通过 | schema/validator artifact 从 output contracts、success criteria、fact sources 一并进入 `schema_targets`；有 schema/validator target 时必须看到 schema/validator validation 语义，如 `jsonschema`、`validate`、`pytest`、`run-tests`，不能只靠 JSON parse | `validation_node_requires_schema_fact_source_for_output_contract_check`; `validation_node_blocks`; `force_finish_validation`; `validation_` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  PASS：78 tests

cargo fmt --all --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
  PASS
```

状态：该 validation/feedback class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证 TaskSpace 是否会实际运行
`python -m jsonschema -i organization.json schema.json`、public-equivalent validator 或等价 schema assertions。

## 5.26 2026-07-04 validation recovery next-action projection dilution

schema fact-source guard 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703l-schema-factsource/runs/terminal_bench__organization-json-generator/20260704-014928-473
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 13
right_open_leaf_nodes: 1
```

关键观察：

| Signal | 结论 |
|---|---|
| 弱 validation 被反复拒绝为 `validation_test_missing_output_contract_coverage` | H-015 的 schema fact-source guard 已在真实 run 中生效 |
| `TaskSpaceGateRecoveryV1.next_valid_actions` 包含 `python process.py && python -m jsonschema -i organization.json schema.json` | gate recovery 已生成精确 schema validation 命令 |
| `TaskSpaceValidationNeedsTestRecoveryV1` 明确要求 obey `next_valid_actions`、use the named command exactly | recovery developer feedback 本身没有丢失 |
| 随后的 `ContextProjectionV1 active replacement` 只暴露 `run validator/test command` | active projection 重新推导了泛化 action，稀释了精确 recovery |
| smoke node 最终 `provider_node_request_hard_limit_exceeded request_count=14/20 node_request_count=6/5` | 模型继续弱重试直到节点预算 hard stop |
| public validator 报 `/app/organization.json does not exist` | TaskSpace 未执行 schema-validating command，也未生成最终输出 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-recovery-next-action-projection-dilution` | validation gate 和 recovery developer message 已包含精确 `jsonschema` 命令，但 active/shadow projection 在同一 validation node 只给 `run validator/test command`，导致模型继续弱重试 | runtime 记录最新 gate recovery `next_valid_actions`，active/shadow projection 在 smoke/regression node 优先原样输出这些动作，并追加“不要替换为更弱 validation”的约束；当前节点记录新 main tool result、清理 blocked repeats 时同步清理该 recovery 状态 | `validation_node_requires_schema_fact_source_for_output_contract_check`; `validation_node_blocks`; `force_finish_validation`; `validation_` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked
  PASS：active projection 保留 exact `python process.py && python -m jsonschema -i organization.json schema.json`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  PASS：78 tests
```

状态：该 feedback/projection class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证模型是否执行
`python process.py && python -m jsonschema -i organization.json schema.json`，若仍 wrong，再按新 trace 建立下一层 tools case。

## 5.27 2026-07-04 validation rework target artifact read gap

recovery projection 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703m-recovery-projection/runs/terminal_bench__organization-json-generator/20260704-020629-368
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 8
public_validation_exit_code: 1
```

关键观察：

| Signal | 结论 |
|---|---|
| `item_26` 提交 `python generate_org.py && python -m jsonschema -i organization.json schema.json` | H-016 的 exact recovery projection 已在真实 run 中生效 |
| `item_27` 执行该命令并返回 `members` / `averageDepartmentBudget` 等 schema 错误 | 工具失败语义正确传回，不再是 generator-only 或弱 JSON parse |
| `item_29` 插入 `TaskSpaceImplementNeedsEditRecoveryV1` | validation failure 正确路由到 implement rework |
| `item_33` 尝试读取 `schema.json`，`item_35` 以 `node_policy_violation:implement_solution:read_file:implementation_needs_edit` 拒绝 | action contract 仍正确阻止泛读 |
| `item_40` block：`Cannot apply correct patch without reading generate_org.py` | rework 节点缺少命名目标工件读取权限，无法看到当前实现内容后 patch |
| `item_52` 把问题归因为真实实现缺陷而非 infra | failure 语义已保真，但下一步能力边界没有传入 session contract |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-target-artifact-read-gap` | schema validation failure 没有 traceback/file path，runtime 只知道 validation dependency；`implementation_needs_edit` 状态把 `read_file` 全拦，模型不能读取 `generate_org.py` 修复当前代码 | runtime 从 blocked validation dependency 的 changed artifacts 提取 rework targets；provider snapshot/projection 明确列出 `current_node_validation_rework_artifacts`；session action contract 只允许读取这些命名 target artifact，继续拒绝 `schema.json` 等泛读 | `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; `taskspace_action_contract_allows_named_validation_rework_artifact_read` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  PASS：schema failure 无 traceback 时从 blocked validation dependency changed artifacts 推导 `generate_org.py`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_allows_named_validation_rework_artifact_read --locked
  PASS：`implementation_needs_edit` 下允许读取 `generate_org.py`，仍拒绝 `schema.json`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked
  PASS：11 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core implementation_needs_edit --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  PASS：80 tests

cargo fmt --all --check
  PASS：仅有已知 stable rustfmt config warning

CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
  PASS
```

状态：该 feedback/action-contract class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证 TaskSpace 是否读取或直接 patch
`generate_org.py`，修正 `members` / camelCase statistics 等 schema contract。

## 5.28 2026-07-04 validation jsonschema module missing rework misroute

target artifact read 修复后的真实 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703n-rework-target-read/runs/terminal_bench__organization-json-generator/20260704-022632-418
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 9
right_open_leaf_nodes: 1
public_validation_exit_code: 1
```

关键观察：

| Signal | 结论 |
|---|---|
| TaskSpace 读齐 `schema.json`、`departments.csv`、`employees.csv`、`projects.csv` | fact-source coverage / inspect projection 仍有效 |
| TaskSpace 创建 `organization.json` 并进入 validation | 前序 output-contract gate 已推动到真实验证阶段 |
| validation 命令为 `python3 -c "import json, jsonschema; ... jsonschema.validate(...)"` | 命令语义是 schema validation，而不是 generator-only 或 weak JSON parse |
| validation 失败为 `ModuleNotFoundError: No module named 'jsonschema'` | 失败发生在 validator dependency loading，schema 还没有执行 |
| runtime 插入 `TaskSpaceImplementNeedsEditRecoveryV1` | 旧分类把 validator dependency failure 当成 implementation rework evidence |
| rework 成功 `read_file organization.json` | 5.27 的 named target artifact read guard 在真实 run 中生效 |
| 模型反复 `finish_node` 并引用 missing jsonschema，最终 `provider_node_request_hard_limit_exceeded` | rework 方向错误，导致控制环耗尽 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-jsonschema-module-missing-rework-misroute` | `ModuleNotFoundError: No module named 'jsonschema'` 被归入 noninfra failed validation，自动 block validation 并进入 implement rework；模型没有新的代码 patch 依据，只能反复 finish | runtime 将 jsonschema module missing 从 noninfra rework 分类中排除；validation projection 根据 output contract/schema requirements 给出 `python -m jsonschema -i organization.json schema.json`；缺 validator dependency 不再直接路由成 implementation rework | `validation_missing_jsonschema_dependency_stays_on_validation_with_cli_recovery`; `validation_`; `local_infra`; `force_finish_validation` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_missing_jsonschema_dependency_stays_on_validation_with_cli_recovery --locked
  PASS：missing jsonschema 留在 validation node，projection 输出 `python -m jsonschema -i organization.json schema.json`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  PASS：81 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core local_infra --locked
  PASS：11 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
  PASS
```

状态：该 validation/feedback class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证 TaskSpace 是否改用
`python -m jsonschema -i organization.json schema.json`，并在真实 schema 错误后继续修正 output contract。

## 5.29 2026-07-04 implementation rework repeat-read budget drain

`validation-jsonschema-module-missing-rework-misroute` 修复后，keyed rerun 已越过 validator dependency blocker：

```text
RunDir: target/r4-org-json-real-keyed-20260703o-jsonschema-recovery/runs/terminal_bench__organization-json-generator/20260704-024204-931
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
```

关键观察：

| Signal | 结论 |
|---|---|
| `python csv_processor.py && python -m jsonschema -i organization.json schema.json` 执行 | H-018 修复生效，schema validation 已使用可用 CLI 路径 |
| schema 输出缺 `members`、`averageDepartmentBudget`、`totalEmployees`、`skillDistribution` 等真实字段错误 | failure 语义是可编辑 implementation defect |
| runtime 插入 `TaskSpaceImplementNeedsEditRecoveryV1`，提示不要 rediscover、应 patch target artifact | recovery 方向正确 |
| `csv_processor.py` 被反复 `read_file` 6 次，命令形态为 `sed -n '1,240p' -- csv_processor.py` | model-visible 动作进入重复读取 |
| right rollout 中 `taskspace-action-contract-*-read_file` 的 `main_tool_result` 为 `actionClass=read`、`toolSuccess=true`、`artifactRefs=[]` | Unix action-contract read 丢失 artifact identity |
| 未出现 `validation_rework_duplicate_artifact_read`，最终 `provider_node_request_hard_limit_exceeded` | 既有 duplicate rework gate 因 artifact refs 为空而失明 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `implementation-rework-repeat-read-budget-drain` | 命名 rework target read 被允许后，Unix `sed` 读文件结果不带 artifact ref；runtime 无法识别同目标重复读取，模型反复读 `csv_processor.py` 直到 node budget hard stop | `read_command_artifact_ref` 支持 `sed -n ... -- path`；successful read result 记录 target artifact；第二次同目标 rework read 在无 edit 前触发 `validation_rework_duplicate_artifact_read`，要求 `apply_patch` 或 blocked | `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  PASS：`sed -n '1,240p' -- generate_org.py` 记录 artifact ref；第二次同 target read 被 duplicate rework gate 拒绝

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  PASS：11 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_needs_edit --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core inspect_data_artifact_read_counts_as_working_evidence --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  PASS：81 tests

cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
git diff --check
  PASS
```

状态：该 feedback/attribution class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证 TaskSpace 是否在读取目标 artifact 后执行 patch，并继续通过 schema/public validator。

## 5.30 2026-07-04 validation blocker manual rework origin loss

`implementation-rework-repeat-read-budget-drain` 修复后，post-commit keyed rerun 证明 sed attribution 已生效，
但暴露出下一层 control/feedback 问题：

```text
RunDir: target/r4-org-json-real-keyed-20260703q-postcommit-attestation/runs/terminal_bench__organization-json-generator/20260704-030017-880
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
```

关键观察：

| Signal | 结论 |
|---|---|
| `process.py` 第一次 read 记录 `artifactRef: process.py` | 5.29 sed attribution 修复生效 |
| 第二、三次 `read_file process.py` 被 `validation_rework_duplicate_artifact_read` 阻止 | duplicate rework gate 已能收到正确 artifact identity |
| 模型随后 patch `process.py`，但只修了 line 1，仍留下 line 2 起的顶层缩进 | implementation 仍未完全修好，这是模型 patch 质量问题 |
| validation recovery 中模型先 `create_node(implement_solution)`，后 `blocked node-5` | 出现手动 rework path，而不是 runtime 自动 rework path |
| action map 最终边为 `node-4 -> node-6`，`node-6.origin_node_id` 缺失 | 新 rework node 没有继承 blocked validation node `node-5` |
| 绑定 `node-6` 后的 `apply_patch` 被拒绝，反馈要求先 `state_commit result-14` | lifecycle gate 因 origin 丢失无法识别 active validation rework input |
| 最终 `provider_request_hard_limit_exceeded request_count=20/20` | 可机械修正的状态归因问题被转化成继续采样预算消耗 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-blocker-manual-rework-origin-loss` | 手动创建的 rework node 默认依赖最近 completed implementation，未记录 blocked validation origin；随后 blocked result 保持 unreviewed，patch 被 lifecycle review gate 拦截 | detached `implement_solution` 若从 active validation node 创建，会记录该 validation node 为 `origin_node_id` 并加入依赖；当 origin validation blocked 时，只刷新对应 pending rework node 为 Ready；active rework edit 可使用该 blocker input | `manual_validation_rework_created_before_block_keeps_origin` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core manual_validation_rework_created_before_block_keeps_origin --locked
  PASS：manual rework 记录 validation origin，origin blocked 后 Ready，patch 不再被 unreviewed blocker gate 拦截

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  PASS：12 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  PASS：82 tests

cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 feedback/DAG-origin class 已 focused fixed；R4-G utility 仍需再次 keyed rerun 验证模型能在 duplicate-read
阻断后完成完整 patch、重跑 schema/public validation，并暴露下一层未解决 tools 问题。

## 5.31 2026-07-04 validation stale failure block without current test

`validation-blocker-manual-rework-origin-loss` 修复后的 keyed rerun 证明 origin/lifecycle 问题已经越过：

```text
RunDir: target/r4-org-json-real-keyed-20260703r-manual-rework-origin/runs/terminal_bench__organization-json-generator/20260704-032001-321
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 12
right_open_leaf_nodes: 0
public_validation_exit_code: 1
```

关键观察：

| Signal | 结论 |
|---|---|
| `generate_org.py` 第一次 read 使用 `sed -n '1,240p' -- generate_org.py` 且记录目标 artifact | sed attribution 修复仍生效 |
| 第二次 `read_file generate_org.py` 被 `validation_rework_duplicate_artifact_read` 阻止 | duplicate rework gate 仍生效 |
| 模型只 patch 了 line 1，剩余顶层行仍有前导空格 | implementation 仍需要继续修复 |
| 新 smoke node 没有记录当前 `Build`/`Test` result，却用旧 `IndentationError` 文案执行 `block_node` | 旧失败语义被跨节点复用为当前 validation 结果 |
| graph `open_leaf_nodes=0`，public validation 仍 exit 1 | runtime 接受了没有当前验证证据的 validation block，导致错误闭合 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-stale-failure-block-without-current-test` | rework 后的新 smoke/regression node 可以不运行当前 validation，直接复用上一轮失败文本 block；模型停止继续 patch，public validator 仍失败 | `block_main_node` 对 smoke/regression node 增加当前验证证据要求：声称 validation/test failure 前必须有同节点 `Build`/`Test` tool result；local validator infrastructure blocker 仍走 infra/retry 路径 | `block_validation_node_rejects_stale_failure_without_current_test` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node_rejects_stale_failure_without_current_test --locked
  PASS：fresh validation node 不能用旧 `IndentationError` blocker 代替当前 test/build result

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node --locked
  PASS：3 tests；有同节点 failed validator result 的 validation block 仍允许

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  PASS：83 tests；manual local validator infrastructure blocker 仍通过

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  PASS：12 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
git diff --check
  PASS
```

状态：该 feedback/validation-evidence class 已 focused fixed；下一次 keyed rerun 应验证模型在 rework patch 后会重新运行
schema/public validation，而不是用上一轮失败文本关闭新的 validation node。

## 5.32 2026-07-04 validation rework duplicate-read projection loop

`validation-stale-failure-block-without-current-test` 修复后的 keyed rerun 已越过旧失败复用，并到达真实 schema rework：

```text
RunDir: target/r4-org-json-real-keyed-20260703s-stale-validation-guard/runs/terminal_bench__organization-json-generator/20260704-033716-688
reported_evidence_level: E1
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 16
right_open_leaf_nodes: 1
public_validation_exit_code: 1
```

关键观察：

| Signal | 结论 |
|---|---|
| `python process.py && python -m jsonschema -i organization.json schema.json` 执行 | 已到真实 output/schema validation |
| schema 报 `members` required、`averageDepartmentBudget` / `totalEmployees` / `averageYearsOfService` 等字段缺失 | validation failure 是可编辑 implementation defect |
| rework node `node-4` 第一次成功读取 `process.py`，`result-11` 带 artifact ref | sed attribution 和 target read allowance 正常 |
| 后续 5 次 `read_file process.py` 全部被 `validation_rework_duplicate_artifact_read` 拦截 | duplicate gate 正常工作 |
| projection 仍显示 `read_file validation rework target artifact process.py only if current contents are not visible`，`critical_artifact_evidence` 为 none | provider-visible contract 没有把 result-11 作为当前内容展示 |
| 最终 `provider_node_request_hard_limit_exceeded node_request_count=6/5`，node-4 open | 正确底层反馈被 projection 冲淡，形成重复 read loop |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-duplicate-read-projection-loop` | rework target 已读且 duplicate gate 已拦截重复 read，但 compact projection 仍广告 read/search，并未把 target read result 展示为 critical evidence | target read 后 projection 显示 `use existing validation rework target read result ...`，移除该 target 的 read_file next action，把 target read excerpt 放入 `critical_artifact_evidence`，allowed actions 收窄为 edit/control | `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  PASS：target 未读前仍允许一次命名 read；target 读完后 projection 使用 result、要求 apply_patch，且 critical evidence 包含目标内容 excerpt

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  PASS：12 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  PASS：83 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
git diff --check
  PASS
```

状态：该 feedback/projection class 已 focused fixed；下一次 keyed rerun 应验证模型在第一次读取 `process.py` 后直接
patch `members` 和 statistics camelCase 字段，再重跑 schema/public validation。

## 5.33 2026-07-04 validation stale failure action-contract feedback gap

`validation-rework-duplicate-read-projection-loop` 修复后的 keyed rerun 证明 target-read projection loop 已越过：

```text
RunDir: target/r4-org-json-real-keyed-20260703t-rework-target-projection/runs/terminal_bench__organization-json-generator/20260704-035138-996
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 10
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| rework node 第一次读取 `process.py` 后执行 `apply_patch` | H-022 的 projection 修复生效 |
| patch 只修掉顶部 import 行的缩进，剩余顶层代码仍可能保留前导空格 | 实现仍需要继续验证和修复 |
| 新 `smoke_test` node-5 没有任何同节点 `Build` / `Test` result | 不能把旧失败文案当成当前 validation 证据 |
| 多次 `blocked` 和 `taskspace_control finish_node status/result_validity failed` 继续引用旧 `IndentationError` | action-contract 入口的失败反馈没有把下一步强制收敛到当前 `run_test` |
| 最终 `provider_node_request_hard_limit_exceeded node_request_count=6/5`，node-5 仍 running | runtime guard 存在，但 feedback 分类过泛导致模型重复 closeout 尝试 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-stale-failure-action-contract-feedback-gap` | `block_node`/`finish_node failed` 入口可携带旧 validation failure 文本；runtime 拒绝后 feedback 落到 generic failure，模型继续重试 block/finish，耗尽 validation node 预算 | failed validation `finish_node` aliases 先归一到 `block_node`；`finish_main_node` 也拒绝无当前 test/build result 的 failed-validation closeout；action-contract recent feedback 输出 `validation_stale_failure_without_current_test` / `validation_finish_missing_current_test_result`，下一步只允许当前 validation node 执行 `run_test` | `finish_validation_node_rejects_stale_failure_without_current_test`; `action_contract_failed_validation_finish_normalizes_to_block_node`; `action_contract_feedback_requires_current_test_after_stale_validation_block`; `action_contract_feedback_requires_current_test_after_validation_finish_without_result` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core finish_validation_node_rejects_stale_failure_without_current_test --locked
  PASS：validation node 不能用旧失败摘要 finish 成 failed validation

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_failed_validation_finish_normalizes_to_block_node --locked
  PASS：`finish_node status=failed reason=...` 归一到 `block_node`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_current_test_after_stale_validation_block --locked
  PASS：stale validation block reject 输出 `validation_stale_failure_without_current_test`，下一步是 `run_test`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_current_test_after_validation_finish_without_result --locked
  PASS：validation finish 缺当前 test/build result 时输出 `validation_finish_missing_current_test_result`，下一步是 `run_test`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core block_validation_node --locked
  PASS：3 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  PASS：12 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  PASS：87 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
git diff --check
  PASS
```

状态：该 feedback/action-contract class 已 focused fixed；下一次 keyed rerun 应验证模型在 node-5 上重新运行 schema/public validation，而不是继续用旧 `IndentationError` 文案 closeout。

## 5.34 2026-07-04 validation rework duplicate-read action-contract feedback gap

`validation-stale-failure-action-contract-feedback-gap` 修复后的 keyed rerun 证明当前 validation node 已经会重新运行测试，
但 rework 节点仍因重复读取反馈不够强而没有进入 patch：

```text
RunDir: target/r4-org-json-real-keyed-20260703u-action-contract-feedback/runs/terminal_bench__organization-json-generator/20260704-041121-187
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 12
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
rollout_trace_model_request_count: 19
```

关键观察：

| Signal | 结论 |
|---|---|
| `node-3` 执行 `python process.py && python -m jsonschema -i organization.json schema.json` | H-023 生效，旧失败 closeout loop 已越过 |
| schema 失败集中在 `statistics`：`averageDepartmentBudget`、`totalEmployees`、`skillDistribution`、`departmentSizes`、`projectStatusDistribution`、`averageYearsOfService` 缺失 | 这是可编辑 implementation defect，不是 validator infrastructure |
| runtime 将 `node-3` blocked 并创建 `node-4` rework | phase routing 正确 |
| `node-4` 第一次成功读取 `process.py`，之后继续读 `process.py` / `schema.json` | 模型没有把已有证据转成 patch |
| runtime 拦截为 `validation_rework_duplicate_artifact_read` 或 `implementation_needs_edit`，但最终仍 hard stop | feedback 层没有把拒绝语义压成强 `apply_patch` action contract |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-duplicate-read-action-contract-feedback-gap` | rework target 内容和 failed validation 都已可见，但 duplicate read / implementation_needs_edit 的 action-contract feedback 仍接近泛化工具失败，模型继续读文件耗尽节点预算 | duplicate read 输出 `failure_kind: validation_rework_duplicate_artifact_read`、`target_artifact`、`previous_read_result` 和强 `apply_patch` next action；generic implementation-needs-edit 输出独立 `failure_kind: implementation_needs_edit`；jsonschema required-property 失败被压缩为 `missing_required_properties`；runtime recovery 在 target 已读后不再广告 read_file | `action_contract_feedback_requires_patch_after_rework_duplicate_read`; `action_contract_feedback_requires_patch_after_implementation_needs_edit`; `validation_failure_excerpt_extracts_required_property_list`; `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_rework_duplicate_read -- --nocapture
  PASS：重复读取已读 rework artifact 时，feedback 明确要求 apply_patch 并禁止继续 read/search/schema rediscovery

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_implementation_needs_edit -- --nocapture
  PASS：普通 implementation_needs_edit 拒绝不再落到 generic tool failure

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_failure_excerpt_extracts_required_property_list -- --nocapture
  PASS：jsonschema required-property 输出结构化为 missing_required_properties

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback -- --nocapture
  PASS：target 已读后 runtime recovery 使用 existing result + apply_patch，不再把 read_file 放回 next_valid_actions

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework -- --nocapture
  PASS：12 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_needs_edit -- --nocapture
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ -- --nocapture
  PASS：88 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
git diff --check
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/write-whale-binary-attestation.ps1 -WhaleBin third_party/codex-cli/codex-rs/target/debug/whale -BuildCommand "CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked"
  PASS
```

状态：该 feedback/action-contract class 已 focused fixed；下一次 keyed rerun 应验证模型在 `node-4` 读取 `process.py` 后直接 patch statistics camelCase / distribution 字段，并重新进入 schema/public validation。

## 5.35 2026-07-04 failed edit context refresh after validation rework

`validation-rework-duplicate-read-action-contract-feedback-gap` 修复后的 keyed rerun 已越过重复读 loop：模型读完
rework target 后进入 `apply_patch`。但这暴露出新的 edit recovery 问题。

先记录一次 harness 经验：`target/r4-org-json-real-keyed-20260703v-rework-feedback` 被 preflight 判为
`invalid_harness`，原因是 `whale` 二进制早于 commit `c9a351f`，attestation hash 不匹配。以后 commit 影响
`third_party/codex-cli` 后，必须先 rebuild `whale` 并重写 attestation，再跑 keyed benchmark。

有效 rerun：

```text
RunDir: target/r4-org-json-real-keyed-20260703w-rework-feedback/runs/terminal_bench__organization-json-generator/20260704-042850-855
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 10
right_open_leaf_nodes: 0
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
rollout_trace_model_request_count: 18
```

关键观察：

| Signal | 结论 |
|---|---|
| `node-4` 读 `generate_org.py` 后尝试 `apply_patch` | H-024 生效，重复 read loop 已越过 |
| validation failure 是 `IndentationError: unexpected indent` | 可编辑 implementation defect |
| 第一次 patch 被 action contract 拒绝为 `apply_patch_unanchored_update:generate_org.py` | edit feedback 开始工作 |
| 下一次 patch 进入工具但失败：`Failed to find expected lines` | patch context 不匹配，需要恢复目标上下文或改用正确 hunk |
| 模型随后 blocked：read result 被截断、缺 full file content，无法构造 patch | 防重复读规则和 edit-failure recovery 共同压住了“失败 edit 后同目标刷新上下文” |
| runtime 接受 blocker，node-4 blocked | 可编辑 validation failure 被错误关闭 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `failed-edit-context-refresh-blocked-by-duplicate-read-guard` | rework target 已读后禁止重复读；即使后续 apply_patch 失败，模型也不能刷新同一 target 上下文，最后把“source truncated / need full content”当作 blocker | duplicate-read guard 只拦截无进展重复读；失败 edit 后允许同一 validation rework target 一次 context refresh；unrelated read/search 仍 blocked；truncated-source blocker 会被拒绝；edit-failure feedback 明确允许同目标 read_file 或 corrected apply_patch | `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; `apply_patch_expected_lines_feedback_allows_target_context_refresh` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback -- --nocapture
  PASS：失败 edit 前同目标重复读仍 blocked；失败 edit 后同目标 refresh read 允许；truncated-source blocker 被拒绝

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core apply_patch_expected_lines_feedback_allows_target_context_refresh -- --nocapture
  PASS：`Failed to find expected lines` feedback 允许同目标 read_file 刷新上下文

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework -- --nocapture
  PASS：12 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib apply_patch_ -- --nocapture
  PASS：33 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ -- --nocapture
  PASS：88 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
git diff --check
  PASS
```

非门禁观察：`cargo test ... -p codex-core apply_patch_ -- --nocapture` 会跑到 integration/shell-serialization
用例，这些用例当前期望 legacy `Exit code: 0 ... Output:` 文本，但 harness 实际输出结构化 JSON。
本次变更使用 `--lib apply_patch_` 作为相关回归集。

状态：该 feedback/runtime class 已 focused fixed；下一次 keyed rerun 应验证模型在 failed patch 后刷新 `generate_org.py`
上下文或直接修正 patch，而不是把可编辑 `IndentationError` 关闭为 blocker。

## 5.36 2026-07-04 validation schema repair contract projection gap

`failed-edit-context-refresh-blocked-by-duplicate-read-guard` 修复后的 keyed rerun 没有再撞 failed-edit
refresh；它暴露的是更细的 schema repair feedback 缺口：

```text
RunDir: target/r4-org-json-real-keyed-20260703x-failed-edit-refresh/runs/terminal_bench__organization-json-generator/20260704-044849-474
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 13
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| `node-3` 运行 `python process_csv_to_json.py && python -m jsonschema -i organization.json schema.json` | 当前 validation 节点已执行真实 schema 验证 |
| jsonschema 失败包含 `members` 以及 statistics required 字段 | 失败是可编辑 implementation defect，不是 validator infra |
| `node-4` 先试图读 `schema.json`，被 `implementation_needs_edit` 拒绝 | rework 节点已经应该进入 edit，而不是 schema rediscovery |
| `node-4` 成功读 `process_csv_to_json.py` 后仍重复读 schema/target | projection 只说 target 已读和 apply_patch，不够明确地保留 schema repair contract |
| 最终 `provider_node_request_hard_limit_exceeded node_request_count=6/5` | feedback 层仍未把“缺哪些 schema 字段、改哪个 target”压成足够稳定的工具反馈 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-schema-repair-contract-not-projected` | raw validator/schema facts 存在，但 compact projection、gate recovery、recent tool feedback 没有稳定携带 `missing_required_properties` + schema required sibling group；模型继续读 schema/target | runtime 从失败输出和已读 `schema.json` 提取 `validation_schema_repair_contract`，写入 `critical_artifact_evidence`、`next_valid_actions`、duplicate-read recovery、generic `implementation_needs_edit` recovery；session recent feedback 顶层输出 `repair_contract`，并要求 exactly satisfy 后再 validation | `validation_rework_projects_schema_repair_contract_from_schema_read`; `action_contract_feedback_requires_patch_after_rework_duplicate_read`; `action_contract_feedback_requires_patch_after_implementation_needs_edit` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_projects_schema_repair_contract_from_schema_read --locked
  PASS：schema failure 只暴露 members/averageDepartmentBudget 时，projection 仍从已读 schema.json 带出完整 statistics required group

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_rework_duplicate_read --locked
  PASS：duplicate target read feedback 顶层包含 repair_contract，并要求 apply_patch exactly satisfy

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_implementation_needs_edit --locked
  PASS：generic implementation_needs_edit feedback 不再丢 schema repair contract

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  PASS：13 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_needs_edit --locked
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  PASS：89 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 feedback-layer class 已 focused fixed；下一次 keyed rerun 应验证模型在 `node-4` 直接 patch
`process_csv_to_json.py`，把 `member_ids` 输出修成 schema 需要的 `members`，并补齐 statistics 的 camelCase
required 字段后重新进入 schema/public validation。

## 5.37 2026-07-04 validator path target pollution and native patch grammar feedback

`validation-schema-repair-contract-not-projected` 修复后的 keyed rerun 已经越过 schema contract 缺失：
模型看到 repair contract 后进入 `process.py` 编辑，但暴露出新的 tools 链路问题。

```text
RunDir: target/r4-org-json-real-keyed-20260703y-schema-contract/runs/terminal_bench__organization-json-generator/20260704-051107-000
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 13
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| validation rework feedback 已携带 `validation_schema_repair_contract` | H-026 生效，schema repair 语义没有继续缺失 |
| `node-4` 读 `process.py` 后尝试 `apply_patch` | provider 已进入编辑路径 |
| projection 中出现 `/home/zhangxu/miniconda3/lib/python3.12/site-packages/jsonschema/__main__.py:4` | validator runtime 路径污染了 rework target 列表 |
| patch 同时包含 `*** Update File: process.py`、`--- a/process.py`、`+++ b/process.py`、`@@ ... @@` | provider 混用了 native apply_patch grammar 和 unified/placeholder hunk |
| edit tool 返回 `apply_patch verification failed: Failed to find expected lines` | 错误到达底层工具后才表现为泛化 expected-lines failure |
| 最终 `provider_node_request_hard_limit_exceeded node_request_count=6/5` | feedback 没有把 patch 语法问题压成下一步唯一动作 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validator-path-target-pollution-and-native-patch-grammar-feedback-gap` | validation failure 中的 jsonschema/Python runtime 路径可能被当作 rework artifact；native apply_patch 内混入 `---/+++`、`@@ -...` 或 `@@ ... @@` 时会落到底层 edit tool，失败语义退化成 expected-lines | runtime 过滤外部 validator/runtime 路径，只保留项目 artifact；action contract 在工具执行前拒绝 mixed native/unified 和 placeholder hunk，返回 `apply_patch_mixed_native_unified` / `apply_patch_native_hunk_header`；recovery 要求 exactly one corrected native patch 或 Delete/Add full file，且不计入 no-action retry | `validation_rework_projects_schema_repair_contract_from_schema_read`; `taskspace_action_contract_rejects_mixed_native_unified_patch`; `taskspace_action_contract_rejects_native_placeholder_hunk_patch`; `apply_patch_native_hunk_recovery_does_not_count_as_no_action_retry` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework_projects_schema_repair_contract_from_schema_read --locked
  PASS：jsonschema runtime path 不再进入 schema repair contract / rework target

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_rejects_mixed_native_unified_patch --locked
  PASS：`*** Update File` 混用 `---/+++` 或 range hunk 时，在 action contract 层拒绝

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_rejects_native_placeholder_hunk_patch --locked
  PASS：`@@ ... @@` placeholder hunk 在 normalizer 前拒绝，避免被改写成看似合法 hunk

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_rejects_native_unified_update_hunk_headers --locked
  PASS：native `*** Update File` 内的 `@@ -old,+new @@` range hunk 不再自动改写，而是反馈为混合语法错误

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_rejects_unified_hunk_header_from_add_file --locked
  PASS：native `*** Add File` 内的 unified range hunk 不再静默删除，而是反馈为混合语法错误

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core apply_patch_native_hunk_recovery_does_not_count_as_no_action_retry --locked
  PASS：`TaskSpaceApplyPatchNativeHunkRecoveryV1` 属于 edit recovery，不消耗 generic no-action retry

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked
  PASS：13 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core implementation_needs_edit --locked
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib apply_patch_ --locked
  PASS：32 tests；旧的 mixed native/unified auto-normalization 预期已改为拒绝语义

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  PASS：89 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

执行经验：本地 Rust 测试如果不带 `CODEX_SKIP_VENDORED_BWRAP=1`，会触发 vendored bubblewrap 构建；当前环境缺
`libcap.pc`，因此会在 `codex-linux-sandbox` build script 失败。R4 本地验证沿用
`CODEX_SKIP_VENDORED_BWRAP=1`，除非先安装 libcap development metadata。

状态：该 tools 链路 class 已在 unit/regression/build 层通过；下一次 keyed rerun 应验证模型在 schema repair contract
存在时，用合法 native patch 修正 `process.py`，而不是把 patch 语法错误交给底层 edit tool 或把
jsonschema runtime 路径当作可读 artifact。

## 5.38 2026-07-04 active node empty response final-candidate misclassification

`validator-path-target-pollution-and-native-patch-grammar-feedback-gap` 修复后的 keyed rerun 没有进入
patch 语法层，而是在 TaskSpace 启动后更早退出：

```text
RunDir: target/r4-org-json-real-keyed-20260703z-native-patch-feedback/runs/terminal_bench__organization-json-generator/20260704-053313-020
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 0
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| 第一条响应是合法 `taskspace_control(action=start_task)` | `node-1` 已创建为 `inspect_code_context` |
| 第二次 provider request 只有 reasoning output：`output_tokens=41`, `reasoning_output_tokens=41` | provider 没有给 assistant action / tool / final text |
| runtime 记录 `response_actionability:final_candidate` | active node 上的空响应被误判为可结束 |
| `saw_actionable_output:false`, `assistant_message_present:false`, `recovery_action:none` | no-action recovery 没有触发 |
| TaskSpace 侧 `tool_call_count=0`, `open_leaf_nodes=1` | 控制环提前终止，未进入 inspect/tool 执行 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `active-node-empty-response-final-candidate-misclassification` | 当 TaskSpace 仍有 active node 时，provider 空响应因为 `needs_follow_up=false` 被归为 `final_candidate`，不会插入 recovery，turn 直接结束 | session 层新增 active-node 空响应判定：有 active node、无 actionable output、无 assistant text、无 terminal action 时强制进入 `empty_follow_up`，外层插入 `TaskSpaceNoActionRecoveryV1` 并继续采样；无 active node 的空响应仍可保持 final candidate | `provider_response_actionability_treats_empty_active_node_response_as_recovery`; `provider_response_actionability_allows_empty_response_without_active_node_final_candidate`; `provider_response_actionability_` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core provider_response_actionability_treats_empty_active_node_response_as_recovery --locked
  PASS：active `inspect_code_context` node 上的空响应归入 `empty_follow_up`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core provider_response_actionability_allows_empty_response_without_active_node_final_candidate --locked
  PASS：没有 active node 时，空响应仍可归为 `final_candidate`

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core provider_response_actionability_ --locked
  PASS：8 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked
  PASS：13 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib apply_patch_ --locked
  PASS：32 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
  PASS：89 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 control/feedback class 已在 unit/regression/build 层通过；还需要 keyed rerun，验证 start_task
后的空响应会被 recovery 拉回工具执行路径，然后继续检验 H-027 的 patch 语法修复是否生效。

## 5.39 2026-07-04 success criteria output artifact validation target gap

`active-node-empty-response-final-candidate-misclassification` 修复后的 keyed rerun 已经越过 0-tool 早停：
TaskSpace 创建任务、读取文件、生成 `process.py`、运行 validation，并给出 final answer。但它暴露了新的反馈层问题：
validation gate 接受了弱 JSON parse，把“JSON 能打开”误当成“输出满足 schema/public contract”。

```text
RunDir: target/r4-org-json-real-keyed-20260703aa-empty-response-recovery/runs/terminal_bench__organization-json-generator/20260704-054010-809
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_tool_call_count: 8
right_open_leaf_nodes: 0
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| `tool_call_count=8`, `open_leaf_nodes=0` | H-028 的 active-node 空响应早停已越过 |
| accepted validation command 是 `python process.py && python -c "...json.load(open('organization.json'))..."` | validation 只证明 generator 执行和 JSON 可解析 |
| 输出仅显示 top-level keys：`metadata`, `organization`, `statistics` | 没有证明 schema required fields |
| forced closeout 使用 `validation_success_after_tool_drain` | runtime 把弱 validation success 提升为完成 |
| public validator 报 `KeyError: 'members'`、`KeyError: 'averageDepartmentBudget'` | schema/public contract 仍未满足 |
| code path 中 `requirements.targets=[]`, `schema_targets=["schema.json"]` | `organization.json` 只在 success criteria 中出现，未被提升为 output validation target |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `success-criteria-output-artifact-validation-target-gap` | output contract 可以是泛化描述，如 `Transform CSV data into JSON`；生成目标 `organization.json` 只出现在 success criteria 中时，runtime 只提取 `schema.json`，没有 output target，因此 `json.load` 弱验证被接受 | validation requirements 同时读取 `problem_ledger.success_criteria` 和 legacy cognitive success criteria；success criteria 中的非 schema `.json` 生成物进入 output targets，schema/validator artifact 仍进入 schema_targets；弱 JSON parse 触发 `validation_test_missing_output_contract_coverage` 并给出 exact `python process.py && python -m jsonschema -i organization.json schema.json` recovery | `validation_node_derives_output_target_from_success_criteria_for_schema_check`; `validation_node_`; `validation_` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_node_derives_output_target_from_success_criteria_for_schema_check --locked
  PASS：generic output contract + success criteria 中的 organization.json/schema.json 会生成 targets/schema_targets，并拒绝弱 JSON parse

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_node_ --locked
  PASS：21 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  PASS：90 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 feedback-layer class 已在 validation runtime/build 层 focused fixed；还需要提交二进制 attestation，
再重跑 keyed `organization-json-generator`。下一轮期望不再接受弱 JSON parse，必须运行 schema/public contract 等价验证；
如果仍 wrong，再按新 trace 继续收录下一层 tools 链路问题。

## 5.40 2026-07-04 validation rework duplicate-read recovery dilution

`success-criteria-output-artifact-validation-target-gap` 修复并提交 attestation 后，新的 keyed rerun 证明 H-029 已被越过：
弱 JSON validation 被拒绝，TaskSpace 执行了真实 schema validation。但该 run 暴露了下一层 feedback priority 问题：
runtime/projection 已经说“不要再读，必须 patch”，session recovery 又把该语义泛化成普通 implement-needs-edit，导致模型重复读文件直到 node hard stop。

```text
RunDir: target/r4-org-json-real-keyed-20260703ab-schema-success-target/runs/terminal_bench__organization-json-generator/20260704-055155-897
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| `python generate_org.py && python -m json.tool organization.json > /dev/null` 被 blocked | H-029 生效，弱 JSON parse 不再被当作 output/schema validation |
| 随后执行 `python generate_org.py && python -m jsonschema -i organization.json schema.json` | recovery exact command 生效 |
| schema 报 `members`、`averageDepartmentBudget`、`totalEmployees`、`skillDistribution` 等字段缺失 | failure 语义是可编辑 implementation defect |
| `node-4` 首次读取 `generate_org.py` 为 `result-11` | rework target read 能力存在 |
| active projection 显示 `use existing validation rework target read result result-11`、`apply_patch validation rework target artifact(s): generate_org.py`、`read/search is no longer a valid next action` | runtime/projection 语义正确 |
| provider 后续重复 `read_file generate_org.py` 并最终读 `schema.json` | 模型没有执行 patch-only next action |
| 每次重复读被 action contract 拒绝为 `validation_rework_duplicate_artifact_read` | 底层 gate 正确 |
| session 插入的是泛化 `TaskSpaceImplementNeedsEditRecoveryV1` advisory attempts 3-7 | 专用 failure_kind 被 recovery 层稀释 |
| 结束于 `provider_node_request_hard_limit_exceeded node_request_count=6/5` | control loop 用 node budget 兜底，太晚 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-duplicate-read-recovery-dilution` | action-contract feedback 已包含 `validation_rework_duplicate_artifact_read`、target、previous result、repair contract，但 session follow-up 把它降成 generic `TaskSpaceImplementNeedsEditRecoveryV1`；provider 可连续重复读直到 node hard stop | session 新增 `TaskSpaceValidationReworkDuplicateReadRecoveryV1`，保留 `failure_kind`、`target_artifact`、`previous_read_result`、`repair_contract`、`TaskSpaceGateRecoveryV1`，并要求 exactly one `apply_patch` 或 exact `block_node`；implementation recovery selection 优先该专用 marker | `validation_rework_duplicate_read_recovery_preserves_patch_only_contract`; `implementation_recovery_prioritizes_duplicate_rework_read_feedback`; `validation_rework_duplicate_read`; `validation_` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_duplicate_read_recovery_preserves_patch_only_contract --locked
  PASS：dedicated recovery 保留 target/process result/repair_contract/GateRecovery，并禁止继续 read/search

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework_duplicate_read --locked
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_prioritizes_duplicate_rework_read_feedback --locked
  PASS：duplicate rework read feedback 优先于 generic implement-needs-edit

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_feedback_requires_patch_after_rework_duplicate_read --locked
  PASS：recent tool feedback 仍保留 target_artifact / previous_read_result / repair_contract

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_needs_edit --locked
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --locked
  PASS：14 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_ --locked
  PASS：91 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 feedback priority class 已在 session feedback/build 层 focused fixed；还需要 commit/push、binary attestation 和 keyed rerun。
下一轮期望 provider 在 duplicate-read rejection 后收到专用 patch-only recovery，直接 `apply_patch generate_org.py`，
或者更早给出 bounded block，而不是继续重复读到 node hard stop。

## 5.41 2026-07-04 validation rework immediate recovery bypass

`validation-rework-duplicate-read-recovery-dilution` 的第一轮 focused fix 添加了专用 selector，但新的 keyed rerun 证明
真实链路还有一个分支没有接入 selector：`response_actionability.needs_recovery()` 的即时 implementation recovery
仍然直接插入 generic `TaskSpaceImplementNeedsEditRecoveryV1`。

```text
RunDir: target/r4-org-json-real-keyed-20260703ac-duplicate-read-recovery/runs/terminal_bench__organization-json-generator/20260704-060538-444
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| weak JSON validation 再次被拒绝，随后执行 schema validation | H-029 仍生效 |
| `node-4` 首次读取 `generate.py` 后进入 rework | rework target read 能力存在 |
| 后续重复 `read_file generate.py` 被 gate 文本拦截：target 已读且尚无成功 edit | runtime/gate 语义存在 |
| recovery warning 仍连续插入 `TaskSpaceImplementNeedsEditRecoveryV1` attempts 5-9 | 专用 recovery selector 没覆盖即时 actionability 分支 |
| 结束于 `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5` | control loop 仍靠预算硬停收尾 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-duplicate-read-immediate-recovery-bypass` | 专用 duplicate-read recovery selector 只接在 post-drain fallback，response-completed 的即时 recovery 分支仍直接生成 generic implement-needs-edit guidance；即使 gate 已说明“用已有 result patch”，下一轮也会继续被 generic 语义稀释 | 即时 implementation recovery 分支统一调用 `build_taskspace_implementation_recovery_item`，并传入 failed-edit summary；selector 能从没有稳定 reason 字段的自然语言 blocked-read 文本识别 duplicate rework read；warning 明确输出 `TaskSpaceValidationReworkDuplicateReadRecoveryV1` | `implementation_recovery_selects_duplicate_rework_from_gate_text_without_reason`; `duplicate` filtered suite |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate
  PASS：21 tests；新增测试覆盖真实 blocked-read 文本不含稳定 reason 字段时仍生成专用 duplicate-read recovery

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked implementation_needs_edit
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework
  PASS：14 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_
  PASS：91 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

环境记录：

```text
cargo test -p codex-core --lib --locked duplicate
  FAIL：当前 host 缺少 libcap.pc，vendored bubblewrap build 无法完成

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate
  PASS
```

状态：该 feedback routing class 已在 session feedback 层 focused fixed；本地回归、fmt、diff check 和 `whale`
构建已通过；还需要 commit/push、binary attestation 和 keyed rerun。下一轮期望 live trace 中出现 `TaskSpaceValidationReworkDuplicateReadRecoveryV1`，
随后 provider 只能 patch `generate.py` 或给出 bounded block，不能再用 generic recovery 连续重复读。

## 5.42 2026-07-04 apply_patch native hunk recovery dilution

`validation-rework-duplicate-read-immediate-recovery-bypass` 修复后，新的 keyed rerun 证明 H-031 已越过：
重复读取 validation rework target 时，live trace 已插入 `TaskSpaceValidationReworkDuplicateReadRecoveryV1`。
下一层 blocker 转移到 apply_patch grammar：action contract 能识别 `apply_patch_mixed_native_unified`，但 advisory
warning 和后续 duplicate-read recovery 没有持续保留 native apply_patch grammar 约束。

```text
RunDir: target/r4-org-json-real-keyed-20260703ad-immediate-recovery/runs/terminal_bench__organization-json-generator/20260704-061808-358
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 11
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| line 83 插入 `TaskSpaceValidationReworkDuplicateReadRecoveryV1` | H-031 live trace crossed，duplicate-read 不再被 generic recovery 吃掉 |
| `apply_patch_mixed_native_unified:generate_org.py` 连续出现 | action contract 已识别 patch grammar 错误 |
| mixed patch rejection 后 warning 仍写成 `TaskSpaceImplementNeedsEditRecoveryV1` | advisory observability 分类缺少 NativeHunk 分支 |
| duplicate-read recovery 后模型再次输出 `--- a/...`、`+++ b/...`、`@@ -1,110 +1,110 @@` | patch grammar 约束没有在后续 recovery 中持续保留 |
| 结束于 `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5` | control loop 仍靠 hard stop 收尾 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `apply-patch-native-hunk-recovery-dilution` | `apply_patch_mixed_native_unified:<target>` 已被 action contract 拒绝，但 advisory warning 标成 generic implement-needs-edit；后续 duplicate-read recovery 只要求 patch，没有重申 native grammar，模型继续用 unified diff/range hunk | advisory warning helper 覆盖 `TaskSpaceApplyPatchNativeHunkRecoveryV1` / unanchored / format / missing-target 等 patch recovery；duplicate-read recovery 增加 native apply_patch grammar bullet，禁止 `--- a/...`、`+++ b/...`、range hunk 和 placeholder hunk | `apply_patch_mixed_native_unified_recovery_uses_native_hunk_warning`; `validation_rework_duplicate_read_recovery_preserves_patch_only_contract`; `apply_patch_` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked native_hunk
  PASS：3 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework_duplicate
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked apply_patch_
  PASS：33 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework
  PASS：14 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_
  PASS：91 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 feedback preservation / observability class 已 focused fixed；本地回归、fmt、diff check 和 `whale`
构建已通过；还需要 commit/push、binary attestation 和 keyed rerun。下一轮期望 mixed native/unified
patch rejection 后 live warning 为 `TaskSpaceApplyPatchNativeHunkRecoveryV1`，并且后续 duplicate-read recovery
仍保留 native patch grammar，避免再次输出 unified diff hunk。

## 5.43 2026-07-04 apply_patch dash-native header feedback gap

`apply-patch-native-hunk-recovery-dilution` 修复后的 keyed rerun 越过了 duplicate-read 专用 recovery，
但暴露出新的 apply_patch 语法变体：provider 没有输出标准 unified diff，也没有输出合法 native
`*** Update File`，而是输出 `--- Update File: generate_organization.py` 加 placeholder/range hunk。
这类 payload 原本会进入 apply_patch 工具失败，随后只得到 generic edit-failure recovery，语义再次变弱。

```text
RunDir: target/r4-org-json-real-keyed-20260703ae-patch-grammar-recovery/runs/terminal_bench__organization-json-generator/20260704-063554-000
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| lines 62/69/76 插入 `TaskSpaceValidationReworkDuplicateReadRecoveryV1` | H-031/H-032 的 duplicate-read recovery 仍然生效 |
| line 80 输出 `--- Update File: generate_organization.py` 和 `@@ -... +@@ ... @@` | 新问题不是旧的 `--- a/...` unified diff，而是假 native header |
| line 82 仍标记 actionable | action contract 没有在 dispatch 前拒绝该 patch |
| lines 83/92 插入 `TaskSpaceEditFailureRecoveryV1` | 失败语义降级成 generic edit failure，没有进入 NativeHunk recovery |
| lines 87-89 再次执行 `read_file generate_organization.py` | generic edit failure 允许了同目标 context refresh，重新打开预算消耗路径 |
| line 93 `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5` | control loop 仍靠 hard stop 收尾 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `apply-patch-dash-native-header-feedback-gap` | `--- Update File:` 不是合法 native operation，也不是完整 unified diff；runtime 漏检后进入 apply_patch 工具失败，并退化为 `TaskSpaceEditFailureRecoveryV1` | action contract 将 `--- Update File:` / `--- Add File:` / `--- Delete File:` 归类为 native header/hunk 语法错误，dispatch 前拒绝为 `apply_patch_native_hunk_header:<target>`；NativeHunk recovery 文案显式禁止 `--- Update File:` | `taskspace_action_contract_rejects_dash_native_update_header_patch`; `dash_native` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked dash_native
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked native_hunk
  PASS：3 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked apply_patch_
  PASS：33 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework
  PASS：14 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_
  PASS：91 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 action-contract feedback class 已 focused fixed，并通过本地回归、fmt、diff check 和 `whale` build；
还需要 commit/push、binary attestation 后再次 keyed rerun。下一轮期望 line 80 这类 payload 在工具执行前转成
`TaskSpaceApplyPatchNativeHunkRecoveryV1`，不再进入 generic edit-failure + read refresh 路径。

## 5.44 2026-07-04 duplicate-read after patch grammar feedback loss

`apply-patch-dash-native-header-feedback-gap` 修复并 attestation 后的 keyed rerun 证明 H-033 已越过：本轮没有
`--- Update File:` 漏检；当 provider 输出 `*** Update File` 内夹 `--- a/...` / `+++ b/...` / range hunk 时，
action contract 在工具执行前拒绝为 `apply_patch_mixed_native_unified:generate_org.py`，并插入
`TaskSpaceApplyPatchNativeHunkRecoveryV1`。新的 blocker 是后续 duplicate-read recovery 覆盖了最近 patch grammar
failure。

```text
RunDir: target/r4-org-json-real-keyed-20260703af-dash-native-header/runs/terminal_bench__organization-json-generator/20260704-064858-360
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 13
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| preflight git head `fa9a5d2b...` 且 attestation pass | H-033 修复进入实测二进制 |
| lines 54/61/68 插入 `TaskSpaceValidationReworkDuplicateReadRecoveryV1` | duplicate-read 专用 recovery 仍生效 |
| line 74 `apply_patch_mixed_native_unified:generate_org.py` | mixed native/unified patch 被 action contract 前置拒绝 |
| line 75 `TaskSpaceApplyPatchNativeHunkRecoveryV1` | H-032/H-033 live crossed，专用 NativeHunk recovery 已送达 |
| line 79 再次 `read_file generate_org.py` | provider 在 NativeHunk recovery 后仍尝试读文件 |
| line 82 普通 `TaskSpaceValidationReworkDuplicateReadRecoveryV1` | duplicate-read recovery 覆盖了最近 patch grammar failure summary |
| line 83 `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5` | 最后一次 recovery 没机会再次送达模型 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-duplicate-read-after-patch-grammar-feedback-loss` | NativeHunk recovery 后的重复读被普通 duplicate-read recovery 接管，recent failed edit summary 中的 `apply_patch_mixed_native_unified:<target>` 不再出现在 provider-visible recovery 中 | duplicate-read recovery 现在接收最近 failed edit summary；当 summary 含 `apply_patch_mixed_native_unified` / `apply_patch_native_hunk_header` 时，明确必须 corrected native apply_patch，`read_file/context refresh` 不是有效恢复；warning 输出 `TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1` | `validation_rework_duplicate_read_recovery_preserves_failed_patch_grammar`; `implementation_recovery_preserves_failed_patch_grammar_on_duplicate_read` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked failed_patch_grammar
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_rework
  PASS：15 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked native_hunk
  PASS：3 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked apply_patch_
  PASS：33 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core --lib --locked validation_
  PASS：92 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 session recovery composition class 已 focused fixed，并通过本地回归、fmt、diff check 和 `whale` build；
还需要 commit/push、binary attestation 和下一轮 keyed rerun。下一轮期望 line 82 这种场景输出
`TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1`，并在正文保留最近
`apply_patch_mixed_native_unified:<target>` failure，避免 patch grammar 语义被 duplicate-read 语义覆盖。

## 5.45 2026-07-04 mixed native/unified patch auto-normalization

`validation-rework-duplicate-read-after-patch-grammar-feedback-loss` 修复并 attestation 后的 keyed rerun 证明
H-034 已越过：trace 中没有出现普通 duplicate-read recovery 覆盖 patch grammar 的问题，`TaskSpaceApplyPatchNativeHunkRecoveryV1`
连续插入并且语义明确。新的 blocker 是能力层边界过硬：provider 连续输出可机械转换的 mixed native/unified patch，
但 action contract 在 normalizer 前拒绝，导致 rework node 预算耗尽。

```text
RunDir: target/r4-org-json-real-keyed-20260703ag-duplicate-read-after-grammar/runs/terminal_bench__organization-json-generator/20260704-070452-184
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 10
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| line 50 `IndentationError: unexpected indent` | validation failure 是可编辑实现错误 |
| lines 66/73/80/87/94 连续输出 `*** Update File: csv2json.py` + `--- a/...` / `+++ b/...` + concrete range hunk | 这些 payload 有明确 target、明确 old/new file header、明确 range hunk，不是 placeholder |
| lines 68/75/82/89/96 `apply_patch_mixed_native_unified:csv2json.py` | action contract 在 normalizer 之前拒绝 |
| lines 69/76/83/90/97 `TaskSpaceApplyPatchNativeHunkRecoveryV1` | 反馈语义已经正确送达，不是本轮主要缺陷 |
| line 98 `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5` | provider 未能按反馈自我修正，control loop 仍靠 hard stop 截断 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `apply-patch-mixed-native-unified-auto-normalization-gap` | 安全 mixed native/unified patch 在 `normalize_taskspace_unified_diff_patch` / `normalize_taskspace_apply_patch` 前被拒绝为 `apply_patch_mixed_native_unified:<target>`，即使它只需要 strip unified file headers 并把 `@@ -... +... @@` 归一为 native `@@` | action contract 前置只拒绝 malformed `--- Update File:` 和 placeholder `@@ ... @@`；安全 mixed patch 先进入 normalizer，规范化后再执行 mixed/native/unanchored/missing-target 检查 | `taskspace_action_contract_normalizes_live_wrapped_mixed_native_unified_patch`; `taskspace_action_contract_normalizes_live_unwrapped_mixed_native_unified_patch`; `mixed_native` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked mixed_native
  PASS：4 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked native_unified_update
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked unified_hunk_header_from_add
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked native_hunk
  PASS：3 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked dash_native
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：33 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：15 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：92 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

操作经验：从 repo root 运行 Rust 命令必须带
`--manifest-path third_party/codex-cli/codex-rs/Cargo.toml`；本仓库根目录不是 Cargo workspace root，裸
`cargo test -p codex-core ...` 会失败。

状态：该 action-contract normalization class 已 focused fixed，并通过本地回归、fmt、diff check 和 `whale` build。
后续 keyed rerun `20260704-071947-777` 已证明该类 live crossed：trace 中不再出现连续
`apply_patch_mixed_native_unified:csv2json.py` / `TaskSpaceApplyPatchNativeHunkRecoveryV1` loop。新的 blocker 记录在 5.46。

## 5.46 2026-07-04 non-diff Update File payload feedback gap

mixed native/unified auto-normalization 修复、commit/push、binary attestation 后的 keyed rerun 证明 H-035 已越过：
本轮没有 `apply_patch_mixed_native_unified` 拒绝，也没有 NativeHunk recovery loop。TaskSpace 进入真实 schema validation，
然后暴露新的 apply_patch payload coverage gap：provider 把一段 `python3 -c` JSON transformation command 塞进
`*** Update File: organization.json` section，runtime 没有在 dispatch 前识别它不是 native diff 内容，最终退化成 generic
`TaskSpaceEditFailureRecoveryV1` 并再次打开 read drain path。

```text
RunDir: target/r4-org-json-real-keyed-20260703ah-mixed-normalization/runs/terminal_bench__organization-json-generator/20260704-071947-777
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: 5a29b811a077776ae2d31ee6741aa8c775a89ee5
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| trace 中无 `apply_patch_mixed_native_unified` / `TaskSpaceApplyPatchNativeHunkRecoveryV1` loop | H-035 实测越过 |
| line 41 执行 `python -m jsonschema -i organization.json schema.json` | validation exact command 已运行，不是弱验证绕过 |
| line 43 报缺 `members`、`averageDepartmentBudget`、`totalEmployees`、`skillDistribution` 等 schema required properties | rework failure 是业务 schema mismatch |
| lines 50/55 duplicate `read_file organization.json` 被 `TaskSpaceValidationReworkDuplicateReadRecoveryV1` 拦截 | duplicate-read feedback 仍生效 |
| line 73 `apply_patch` payload 为 `*** Update File: organization.json` 后跟 `python3 -c` 脚本，没有 `@@` / `-old` / `+new` | 新 payload 不是 native diff，而是 command text 被误放进 patch |
| lines 76/83/92 只插入 generic `TaskSpaceEditFailureRecoveryV1` | action contract 没有在工具前保留具体 patch grammar 语义 |
| line 93 `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5` | generic recovery 后仍进入预算耗尽路径 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `apply-patch-non-diff-update-payload-feedback-gap` | `*** Update File` section 只要不是 add-only `+...` 形态，就可能穿过 unanchored detector；没有任何 `+`/`-` diff change 行的 command/text payload 会进入 apply_patch 工具，并被降级成 generic edit failure | Update File section 若有内容但没有任何 native diff change 行，dispatch 前拒绝为 `apply_patch_unanchored_update:<target>`；recovery 明确不能把 shell/Python/JSON transformation command 放入 apply_patch payload；deletion-only update 仍合法 | `taskspace_action_contract_rejects_non_diff_update_payload`; `taskspace_action_contract_allows_delete_only_update_patch` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked non_diff_update_payload
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked unanchored_update
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked delete_only_update
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：33 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：15 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：92 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 action-contract feedback class 已 focused fixed，并通过本地回归、fmt、diff check 和 `whale` build；
还需要 commit/push、binary attestation 和下一轮 keyed rerun。下一轮期望 line 73 这类 payload 在工具执行前被拒绝为
`apply_patch_unanchored_update:organization.json`，并插入 `TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1`，不再退化成 generic
`TaskSpaceEditFailureRecoveryV1`。

## 5.47 2026-07-04 Python Add File common-indent normalization

non-diff Update File payload 修复、commit/push、binary attestation 后的 keyed rerun 没有复现 H-036 的
`python3 -c` inside patch 形态；新的 blocker 是 Python Add File 共同前导空格。provider 这次生成了脚本而不是直接生成
JSON，但在 native `*** Add File` 中每个 Python 内容行都写成 `+ import...` / `+ def...`，工具按字面创建文件后
`python generate_org_json.py` 在 line 1 报 `IndentationError: unexpected indent`。

```text
RunDir: target/r4-org-json-real-keyed-20260703ai-non-diff-patch/runs/terminal_bench__organization-json-generator/20260704-073032-022
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 15
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: 711b69e577f8b21033abe1b1ebeeecf0b9982160
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| line 29 `*** Add File: generate_org_json.py` 中所有内容行都是 `+ import` / `+ def` / `+     ...` | provider 常见地把 patch marker 后的空格当成排版，而工具按字面写入源文件 |
| line 43 `IndentationError: unexpected indent` at line 1 | 统一多一格前导空格让 Python 文件无效 |
| line 50 首次 `read_file generate_org_json.py` 输出确认整文件都有共同一格前导空格 | 不是单行局部缩进问题，是 Add File 共同缩进问题 |
| lines 58/65/72/79 `TaskSpaceValidationReworkDuplicateReadRecoveryV1` | duplicate-read feedback 生效，但模型在 thin/budget recovery 中仍反复读 |
| line 80 `TaskSpaceProviderBudgetHardStopV1 node_request_count=6/5` | 继续依赖 rework 纠偏会消耗节点预算 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `python-add-file-common-indent-normalization-gap` | Python Add File 中每个 `+` 内容行统一多一个空格时，工具忠实写入，生成语法无效文件并把问题推给后续 validation rework | `normalize_taskspace_apply_patch` 对 `*** Add File: *.py` / `*.pyw` 做窄范围共同缩进规范化：仅当所有非空新增内容行都统一多一格时，去掉一层；非 Python 文件和混合缩进内容不动 | `taskspace_apply_patch_strips_common_python_add_file_indent`; `taskspace_apply_patch_preserves_non_python_add_file_indent` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked common_python_add_file_indent
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked non_python_add_file_indent
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：15 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：92 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 apply_patch capability normalization class 已 focused fixed，并通过本地回归、fmt、diff check 和 `whale` build；
还需要 commit/push、binary attestation 和下一轮 keyed rerun。下一轮期望 line 29 这类 Python Add File 直接生成无共同前导空格的
脚本，不再在第一轮 validation 出现 line 1 `IndentationError`。

## 5.48 2026-07-04 anchored placeholder hunk normalization

Python Add File common-indent 修复后的 keyed rerun 证明 H-037 已越过：`generate_json.py` 没有 line 1
`IndentationError`，脚本成功运行并生成 `organization.json`。新的 blocker 是最后的 targeted patch 使用
`@@ ... @@` placeholder hunk。该 patch 已带真实上下文行和新增代码，但 action contract 在 hard stop 前将其拒绝为
`apply_patch_native_hunk_header:generate_json.py`。

```text
RunDir: target/r4-org-json-real-keyed-20260703aj-python-add-indent/runs/terminal_bench__organization-json-generator/20260704-073958-389
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: a4abc1e199de8f1ab12ba7e9c18fe8552c14dbdf
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| line 57 `organization.json generated successfully.` 后进入 `jsonschema` 错误 | H-037 越过，脚本已能执行 |
| schema errors 指向 `skills` string、`member_ids`、statistics camelCase fields | 进入真实业务 rework，不再是 Python 语法问题 |
| line 66 read `generate_json.py`，内容无统一前导空格 | Python Add File normalizer live 生效 |
| line 101 patch 使用 `*** Update File` + `--- a/...` / `+++ b/...` + `@@ ... @@`，且有 `def build_organization...` anchor | payload 可机械转换为 native `@@` |
| line 103 `apply_patch_native_hunk_header:generate_json.py`，line 104 NativeHunk recovery，line 105 hard stop | 过严拒绝发生在预算恢复末尾 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `apply-patch-anchored-placeholder-hunk-normalization-gap` | 所有 `@@ ... @@` placeholder hunk 都被硬拒绝，即使后面有真实上下文/变更行，导致可执行 patch 在 hard stop 前被丢弃 | anchored placeholder hunk 规范化为 native `@@`；随后仍经过 unanchored/context/missing-target 检查；malformed `--- Update File:` 仍拒绝 | `taskspace_action_contract_normalizes_native_placeholder_hunk_patch`; `taskspace_action_contract_normalizes_live_mixed_placeholder_hunk_patch` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked placeholder_hunk
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked mixed_placeholder
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked native_hunk
  PASS：3 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：15 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：92 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 action-contract normalization class 已 focused fixed，并通过本地回归、fmt、diff check 和 `whale` build；
还需要 commit/push、binary attestation 和下一轮 keyed rerun。下一轮期望 line 101 这类 anchored placeholder patch 直接规范化并进入
`apply_patch` 工具，不再在 hard stop 前被 `apply_patch_native_hunk_header` 丢弃。

## 5.49 2026-07-04 validation rework duplicate-read advisory hard stop

anchored placeholder hunk 修复、commit/push、binary attestation 后的 keyed rerun 证明 H-038 已越过：right-side trace 中没有
`apply_patch_native_hunk_header` 或 `TaskSpaceApplyPatchNativeHunkRecoveryV1`。新的 blocker 回到 feedback/control
层：validation rework 已经拥有 `generate_org.py` 的 target read result，projection 也明确写出
`read/search is no longer a valid next action`，但模型继续重复 `read_file generate_org.py`，runtime 继续插入 advisory
recovery，直到 provider node budget hard stop。

```text
RunDir: target/r4-org-json-real-keyed-20260703ak-placeholder-hunk/runs/terminal_bench__organization-json-generator/20260704-075115-109
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 16
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: 75923e09bb069d5a5a57c264c56f0d0be7ae99e5
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| right trace 无 `apply_patch_native_hunk_header` / NativeHunk recovery | H-038 live 越过 |
| line 50 执行 `python generate_org.py && python -m jsonschema -i organization.json schema.json` | 已进入真实 schema validation |
| schema errors 指向 `members`、statistics camelCase fields | rework 有明确可编辑目标 |
| active projection 显示 `use existing validation rework target read result result-11` 和 `apply_patch validation rework target artifact(s): generate_org.py` | feedback 语义没有缺失 |
| lines 69/76/83/90/97 连续 `TaskSpaceValidationReworkDuplicateReadRecoveryV1`，line 98 budget hard stop | 正确语义被 advisory loop 稀释 |

结论：该 case 不是“工具失败语义缺失”，而是“失败语义出现降级/扭曲”。Action contract 和 projection 已经把下一步限定为
`apply_patch` 或具体 `block_node`，但外层 provider loop 仍把重复违反同一 gate 当成可继续采样的 advisory recovery。
因此本轮修复不让 runtime 生成代码补丁，也不把节点伪装成外部 blocker；只在同一 validation rework duplicate-read
recovery 第二次出现、或 GateRecovery 已携带 `repeated_blocked_action` 时，插入稳定终止 marker：
`TaskSpaceValidationReworkDuplicateReadHardStopV1`，停止本 turn 的 provider sampling，保留 bounded evidence。

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-duplicate-read-advisory-loop` | 已读 rework target 后，重复 `read_file` 被正确拒绝并带 patch-only recovery，但同一非法动作仍可反复触发 advisory recovery 直到 provider/node budget hard stop | 第一条 duplicate-read recovery 仍保留纠错机会；第二条同类 recovery 或带 `repeated_blocked_action` 的 gate 直接升级为 `TaskSpaceValidationReworkDuplicateReadHardStopV1`，不再继续采样烧预算 | `validation_rework_duplicate_read_hard_stops_after_one_recovery`; `validation_rework_duplicate_read_repeated_gate_hard_stops_immediately` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework_duplicate_read
  PASS：5 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：17 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：94 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 runtime recovery-loop class 已 focused fixed；还需要 commit/push、binary attestation 和下一轮 keyed rerun。下一轮期望不再出现
`TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded` 的重复 read_file 预算耗尽形态；若模型仍不 patch，
应以 `TaskSpaceValidationReworkDuplicateReadHardStopV1` 明确暴露，而不是继续 advisory loop。

## 5.50 2026-07-04 post-edit forced validation transition

duplicate-read advisory hard stop 修复、commit/push、binary attestation 后的 keyed rerun 证明 H-039 不再是当前 blocker：
模型没有第二次重复读取同一 validation rework target，而是在第一条 duplicate-read recovery 后尝试 `apply_patch`。
新的断点出现在成功 edit 之后：action map 已经记录 `generate_organization.py` 的成功修改，但 runtime 没有把
implement rework 节点收束到 validation，下一次 provider dispatch 前才被 node budget hard stop 截断。

```text
RunDir: target/r4-org-json-real-keyed-20260703al-duplicate-read-hard-stop/runs/terminal_bench__organization-json-generator/20260704-080713-106
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: 23d20b5cd547ffdaed19725a38f70919af8cf672
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| trace 中没有第二条同类 duplicate-read recovery | H-039 没有被触发；模型已经推进到 patch 尝试 |
| `trace-208 kind=main_tool_result nodeId=node-4 resultId=result-14 callId=taskspace-action-contract-15-apply_patch actionClass=edit toolSuccess=true artifactRefs=["generate_organization.py"]` | 成功 edit 语义已经进入 action map |
| 同一 provider request 记录 `node_request_count=5` / `max_model_requests_per_node=5` / `runtime_budget_state=thin_downgraded` | edit 发生在 implement node 请求边界 |
| 后续没有 `TaskSpaceForcedImplementTransitionV1` / `forced_implement_transition` | post-tool-drain 收束没有执行 |
| `whale-exec.jsonl` 以 `TaskSpaceProviderBudgetHardStopV1 ... node_request_count=5/5` 结束 | 下一轮 pre-dispatch hard stop 兜底截断，validation 没机会运行 |

代码证据显示 `provider_request_budget_snapshot_pressure_active_for_node` 固定返回 `false`，导致
`force_finish_implement_for_provider_budget` 即使看到成功 edit，也无法在 node/profile hard-limit 边界触发。

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `post-edit-forced-validation-transition-gap` | implement node 已记录成功 edit，且已到 node request hard-limit 边界；runtime 没有强制 finish 到 validation，下一次 provider request 被 `TaskSpaceProviderBudgetHardStopV1` 截断，留下 open leaf | snapshot budget-pressure predicate 使用真实 hard-limit 判断：`request_count >= max_requests` 或 `node_request_count >= max_model_requests_per_node`；成功 edit + 压力边界会触发 `forced_implement_transition` 到 smoke validation；低于边界仍不抢跑 | `provider_budget_node_limit_force_finishes_implementation_into_smoke_test_after_edit`; `provider_budget_below_node_limit_does_not_force_finish_implementation_after_edit` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked provider_budget
  PASS：23 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_active_budget
  PASS：11 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：17 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：94 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked duplicate_rework
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 control/feedback class 已 focused fixed；还需要 commit/push、binary attestation 和下一轮 keyed rerun。下一轮期望
`result-14` 这类 successful edit 后出现 `TaskSpaceForcedImplementTransitionV1 trigger=implement_observed_edit_after_tool_drain`
或等价 forced transition trace，随后进入 schema validation，而不是以 implement node hard stop 结束。

## 5.51 2026-07-04 schema required-property semantic summary before ActionMap truncation

post-edit forced validation transition 修复、commit/push、binary attestation 后的 keyed rerun 证明 H-039/H-040 不再是当前 blocker：
本轮 trace 没有 `TaskSpaceProviderBudgetHardStopV1`，重复 validation rework read 最终以
`TaskSpaceValidationReworkDuplicateReadHardStopV1` 明确暴露。但真实 schema validation 失败后，repair contract 只包含
`members`，未包含 statistics camelCase required fields。

```text
RunDir: target/r4-org-json-real-keyed-20260703am-post-edit-transition/runs/terminal_bench__organization-json-generator/20260704-082204-387
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 12
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: c7d5ba971c03b595bca73bf6a3a111d4a75b0834
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| `whale-exec.jsonl` line 43 的 command output 包含 `members`、`averageDepartmentBudget`、`totalEmployees`、`skillDistribution`、`departmentSizes`、`projectStatusDistribution`、`averageYearsOfService` | 工具原始失败语义完整 |
| `rollout.jsonl` `result-9` body 截断在 `average_years_of_servic` | ActionMap 存储的是 telemetry preview，不是完整 raw output |
| `result-10` blocker 只含 `missing_required_properties: members` | blocker summary 已经继承截断后的残缺语义 |
| 后续 repair contract 为 `missing_required_properties=members | target_artifacts=generate_org.py...` | rework patch contract 丢失 statistics required fields |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-schema-required-property-summary-truncated-before-action-map` | validator raw output 有完整 required-property 失败；进入 ActionMap 的 `MainToolCall` preview 被 telemetry 截断，repair contract 只能解析出 `members` | `ToolOutput` 在完整 raw output 阶段抽取 `TaskSpaceToolSemanticSummaryV1`，前置 `missing_required_properties:` 摘要；bounded raw preview 仍可截断，非 schema 输出不添加摘要 | `taskspace_preview_preserves_required_properties_from_untruncated_exec_output`; `taskspace_preview_does_not_add_schema_summary_for_plain_exec_output`; `validation_rework_projects_schema_repair_contract_from_schema_read` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_preview_
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework_projects_schema_repair_contract_from_schema_read
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：17 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：94 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
git diff --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 feedback-layer semantic-preservation class 已 focused fixed；本地 focused/regression、fmt、diff check 和 `whale`
build 已通过。还需要 commit/push、binary attestation 和下一轮 keyed rerun。
下一轮期望 validation rework recovery / duplicate-read hard stop 中的 `repair_contract` 至少包含
`members` 与 statistics camelCase required fields，而不是只剩 `members`。

## 5.52 2026-07-04 schema summary must be attached at exec formatter boundary

`schema required-property semantic summary` 修复、commit/push、binary attestation 后的 keyed rerun 证明第一段修复点仍太晚：
`organization-json-generator` 的 `shell_command` 非零退出路径不是把 `ExecCommandToolOutput` 直接交给
`tool_output_model_visible_preview`，而是在 `ToolEmitter::finish` 中先通过 exec formatter 生成
`FunctionCallError::RespondToModel(content)`。因此 `ToolOutput` 层 summary 没有进入 live ActionMap。

```text
RunDir: target/r4-org-json-real-keyed-20260703an-schema-summary/runs/terminal_bench__organization-json-generator/20260704-083751-467
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 11
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: c8fe197171b6f236f11641bedc2548ef28ef64a9
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| `whale-exec.jsonl` line 36 raw command output 包含六个 statistics required fields | validation 工具原始失败语义完整 |
| `rollout.jsonl` `result-8` 没有 `TaskSpaceToolSemanticSummaryV1` | live shell path 绕过了 `ExecCommandToolOutput::taskspace_semantic_summary` |
| `result-8` body 在 `projectStatusDistribution` / `averageYearsOfService` 前截断 | ActionMap 仍只看到 formatter 后的 bounded output |
| final duplicate-read hard stop 的 repair contract 只有 `averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes` | H-041 只修了一半，后两项仍在 formatter 截断中丢失 |

本轮新增并 focused 修复的问题类型细化：

| Case | Before | After | Evidence |
|---|---|---|---|
| `schema-required-property-summary-after-exec-formatter-truncation` | shell_command error path 在 `ExecToolCallOutput -> FunctionCallError::RespondToModel` 阶段先截断；ToolOutput preview 层已无法恢复完整 schema failure semantics | semantic summary helper 上移到 `tools/mod.rs`；`format_exec_output_str_with_ref` 在 `formatted_truncate_text` 之前抽取并前置 `TaskSpaceToolSemanticSummaryV1`；`context.rs` 复用同一 helper | `exec_output_formatter_preserves_schema_summary_before_truncation`; `taskspace_preview_`; `validation_rework`; `validation_` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked schema_summary
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_preview_
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：17 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：94 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 formatter-level feedback semantic class 已 focused fixed，并通过 fmt/diff/build；还需要 commit/push、binary attestation
和下一轮 keyed rerun。下一轮期望 `result-8` 或等价 failed validation result body 明确包含
`TaskSpaceToolSemanticSummaryV1`，且 downstream `repair_contract` 包含全部 six statistics required fields。

## 5.53 2026-07-04 failed edit feedback must be projection-critical

`ed3252a` 的 keyed rerun 证明上一节 formatter-level schema summary 已进入 live path：失败 validation 输出和 active
projection 都包含完整的 `members + six statistics required fields` repair contract。该 run 没有再卡在 schema
语义截断，而是暴露下一层 feedback/projection 问题。

```text
RunDir: target/r4-org-json-real-keyed-20260703ao-exec-summary/runs/terminal_bench__organization-json-generator/20260704-085030-109
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 11
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: ed3252a9db7d09b5e9e76e31fe7e56c59e464d13
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| `whale-exec.stderr.log` 中 failed jsonschema output 以 `TaskSpaceToolSemanticSummaryV1` 开头 | exec formatter summary 已在真实 shell path 生效 |
| projection 中 `validation_rework_schema_repair` 包含 `members, averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService` | repair contract 完整传给 TaskSpace |
| 后续 `apply_patch` 失败为 `Failed to find expected lines`，hunk 引用了不存在的 `return { ... }` block | 新 blocker 转为 patch feedback/recovery |
| runtime 两次拒绝 `finish_node`：`cannot be completed without a recorded successful edit action` | 状态机底线正确 |
| active projection 仍在 no-successful-edit 状态暴露 `taskspace_control(action=finish_node) ... after successful edit` | provider-visible `next_valid_actions` 混入条件性未来动作，模型把它当成立即合法 |
| failed edit 只在 recovery 文本 / hidden refs 中出现，未进入 `critical_artifact_evidence` | failed edit 反馈可见性不够硬，导致 repeated finish/budget drain |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `failed-edit-projection-recovery-dilution` | failed `apply_patch` 结果存在，但 active projection 没有把它列为 critical evidence；`next_valid_actions` 同时展示 corrected patch 和条件性 future finish，模型继续声明 edit succeeded 并 finish | `projection_critical_artifact_evidence` 增加 `failed_edit_feedback signal=latest_failed_edit`；validation rework 在无成功 edit 前不再暴露 immediate `finish_node` next action；allowed-actions 文本明确 `finish_node` blocked until successful edit，仍保留必要的同目标 refresh read | `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; `validation_rework`; `validation_`; `apply_patch_` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：17 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：94 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked schema_summary
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_preview_
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 projection/feedback class 已 focused fixed，并通过 diff/build；还需要 commit/push、binary attestation
和下一轮 keyed rerun。下一轮期望 failed edit 后 projection 明确显示 latest failed edit，且不会再把
`finish_node` 作为当前 next action 暴露给模型。

## 5.54 2026-07-04 unanchored patch feedback must survive duplicate-read recovery

`14e6aa2` 的 keyed rerun 证明上一节 failed-edit projection 修复已经把流程推进到下一层：schema repair contract
仍然完整，premature `finish_node` 被 runtime 拒绝，模型也开始读取目标文件并尝试 patch。新的失败不在工具执行层，而在
action-contract rejection 到 duplicate-read recovery/hard-stop 的反馈保真路径。

```text
RunDir: target/r4-org-json-real-keyed-20260703ap-failed-edit-projection/runs/terminal_bench__organization-json-generator/20260704-090416-015
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 11
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: 14e6aa21c291e95f4e89b745ad4743025ca9a44c
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| failed validation stderr 仍以 `TaskSpaceToolSemanticSummaryV1` 开头，并包含 `members + six statistics required fields` | schema semantic summary 没有回退 |
| model 在 validation failure 后读取 `generate.py` 并尝试 patch | 已越过上一轮 failed-edit projection dilution 主 blocker |
| action contract 拒绝 `apply_patch_unanchored_update:generate.py` | 工具/合约层正确识别 malformed native patch，未让坏 patch 进入 executor |
| 后续重复 `read_file generate.py` 触发 duplicate-read recovery/hard-stop | 状态机继续维护 patch-only gate |
| hard-stop excerpt 未把 `apply_patch_unanchored_update` 拒绝语义放在足够高优先级 | 反馈层在 bounded recovery 摘要里丢失了最该纠正的 patch grammar failure |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-duplicate-read-after-unanchored-patch-feedback-loss` | `apply_patch_unanchored_update` 被 action contract 正确拒绝，但 duplicate-read recovery 只把 mixed/native hunk 类失败当成 patch grammar recovery；unanchored rejection 容易被 previous blocked feedback / repair contract 挤出 hard-stop excerpt | duplicate-read recovery 将 `Most recent failed edit feedback to preserve` 提前到 previous blocked feedback 之前；patch grammar preservation/advisory 覆盖 `apply_patch_unanchored_update`；recovery 明确要求立即修正 patch grammar，且 `read_file/context refresh is not a valid recovery` | `validation_rework_duplicate_read_recovery_preserves_unanchored_patch_feedback`; `validation_rework_duplicate_read`; `validation_rework`; `validation_`; `apply_patch_`; CoE E-095/H-044/E-096 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework_duplicate_read
  PASS：6 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：18 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：95 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked schema_summary
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked taskspace_preview_
  PASS：2 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 feedback-preservation class 已 focused fixed，并通过 diff/build；还需要 commit/push、binary attestation
和下一轮 keyed rerun。下一轮期望 unanchored patch rejection 后，duplicate-read recovery/hard-stop 明确展示
`apply_patch_unanchored_update` 并要求修正 patch grammar，而不是让模型继续 context refresh。

## 5.55 2026-07-04 read_file completeness must be explicit in validation rework

`f9ab63f` 的 keyed rerun 没有复现 `apply_patch_unanchored_update`，说明上一节的 unanchored patch feedback case
已被越过。新的失败点发生在 traceback-driven validation rework：工具返回了可编辑失败和目标文件内容，但 read 成功结果没有告诉模型
这是不是完整文件，模型连续以 “Need full file” 为理由重复读取同一文件。

```text
RunDir: target/r4-org-json-real-keyed-20260703aq-unanchored-recovery/runs/terminal_bench__organization-json-generator/20260704-091714-857
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 11
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: f9ab63f733ccc488c470211a56375d6a068c944e
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| trace 中没有 `apply_patch_unanchored_update` | H-044 的 unanchored patch recovery blocker 已越过 |
| validation 命令为 `python generate_organization.py && python -m jsonschema -i organization.json schema.json` | validation coverage 仍保持正确 |
| failed validation 为 `NameError: name 'projects_by_dept' is not defined` | 新失败是可编辑实现错误，不是 schema summary 缺失 |
| rework 第一次 `read_file generate_organization.py` 已返回 `compute_project_budget` 和 `projects_by_dept` 相关代码 | 目标文件内容已经进入反馈链路 |
| 后续 provider 两次重复 `read_file generate_organization.py`，rationale 是 `Need full file` | 成功 read 的完整性语义不明确，模型把 `sed -n 1,240p` 当成可能截断 |
| runtime 最后触发 `TaskSpaceValidationReworkDuplicateReadHardStopV1` | 状态机仍能阻断重复读，但没有帮助模型从“需要完整文件”转向 patch |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-read-file-completeness-ambiguity` | `read_file` 输出是 bounded first-window 内容，没有结构化说明是否到达 EOF；projection 只能说已有 `result-10`，不能区分完整小文件与截断大文件 | `read_file` 保持 `sed -n 1,240p` 前缀并追加 `TaskSpaceReadFileSummaryV1: lines_read/eof_reached/max_lines`；ActionMap 解析该 summary，并在 working evidence、critical artifact evidence、next actions、duplicate-read gate 中前置；`eof_reached=true` 明确 no additional lines hidden，`false` 保持 bounded_read | `action_contract_read_file_uses_host_platform_command`; `sed_read_command_artifact_ref_ignores_read_summary_suffix`; `working_evidence_excerpt_preserves_bounded_read_summary`; `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; CoE E-097/H-045/E-098 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked action_contract_read_file_uses_host_platform_command
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked sed_read_command_artifact_ref_ignores_read_summary_suffix
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked working_evidence_excerpt_preserves_bounded_read_summary
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：18 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：95 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 read feedback completeness class 已 focused fixed，并通过 diff/build；还需要 commit/push、binary attestation
和下一轮 keyed rerun。下一轮期望 `generate_organization.py` 首次 read 后 projection 显示 `complete_read` /
`eof_reached=true`，重复 full-file read 被明确反馈为无隐藏行，应推动模型转向 `apply_patch`。

## 5.56 2026-07-04 read_file summary command must be awk-portable

`1a9eb0c` 的 keyed rerun 没有验证到上一节的 `read_file` completeness 语义，因为新 summary 命令自身在 benchmark
容器里失败了。`sed` 前半段已经输出文件内容，但追加的 `awk ... -- <path>` 在该环境中把 `--` 当作文件名处理：

```text
RunDir: target/r4-org-json-real-keyed-20260703ar-read-completeness/runs/terminal_bench__organization-json-generator/20260704-093439-320
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 16
right_open_leaf_nodes: 1
public_validation_exit_code: 1
hidden_oracle_exit_code: 0
preflight_git_head: 1a9eb0ceb509b0b505fb3ba78f0dd9ddc933d2e8
build_attestation_status: pass
```

关键观察：

| Signal | 结论 |
|---|---|
| `read_file` command 形如 `sed -n '1,240p' -- schema.json && awk ... -- schema.json` | summary 命令已进入 live path |
| stderr/output 出现 `awk: cannot open "--" (No such file or directory)` | benchmark 环境 awk 不接受 `--` option terminator |
| `sed` 已打印文件正文，但整体 exit 2 | 成功 read 被误记录为失败 read |
| TaskSpace 后续进入 inspect recovery / node budget drain | H-045 无法在该 run 中验证 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `read-file-summary-awk-double-dash-portability` | Unix read summary 使用 `awk ... -- <path>`，在部分 awk 实现中 `--` 被当作输入文件，导致 every read_file exit 2 | 保留 `sed -- <path>` 作为实际读取和 artifact 解析前缀；仅 summary `awk` 改为 `awk <script> <path>`；parser regression 仍证明 `sed ... && awk ...` 能解析原始 artifact | direct shell smoke; `action_contract_read_file_uses_host_platform_command`; `sed_read_command_artifact_ref_ignores_read_summary_suffix`; CoE E-099/H-046/E-100 |

验证：

```text
printf 'a\nb\n' > target/read-summary-smoke.txt
sed -n '1,240p' -- target/read-summary-smoke.txt && awk '...' target/read-summary-smoke.txt
  PASS：输出 TaskSpaceReadFileSummaryV1 ... eof_reached=true

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked action_contract_read_file_uses_host_platform_command
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked sed_read_command_artifact_ref_ignores_read_summary_suffix
  PASS：1 test

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_rework
  PASS：18 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked validation_
  PASS：95 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core --lib --locked apply_patch_
  PASS：35 tests

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

状态：该 portability class 已 focused fixed，并通过 diff/build；还需要 commit/push、binary attestation 和下一轮 keyed
rerun。下一轮重新验证 5.55 的目标：`read_file` 成功、summary 出现、`eof_reached` 进入 projection/recovery。

## 5.57 2026-07-04 generic fact-source success-criteria artifact gap

`d11321c` 的 keyed rerun 证明 schema validation path 已可达，但 TaskSpace 仍未达到 utility success。新的 blocker
是 inspect coverage：success criteria 已经点名 `departments.csv`、`employees.csv`、`projects.csv`，但 fact source
只记录了泛化目录描述，runtime 未把这些具体输入工件纳入 required read gate。

```text
RunDir: target/r4-org-json-real-keyed-20260704bl-schema-rename-hints/runs/terminal_bench__organization-json-generator/20260704-144300-806
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
```

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `generic-fact-source-success-criteria-artifact-gap` | `initial_fact_sources` 泛化为 repository root，inspect 只读 `schema.json` 就进入实现；模型猜测 CSV 字段，后续 validation 出现大面积 missing required properties | `task_required_fact_source_artifact_refs()` 从 success criteria 补齐 concrete input artifacts；排除 output target / generated JSON，避免误要求读取 `organization.json` | `inspect_requires_success_criteria_artifacts_when_fact_source_is_generic_directory`; `CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core inspect_ -- --nocapture` 62/62 PASS; CoE H-067/E-141/E-142 |

结论：这是 R4-D feedback/coverage focused 修复，不是 R4-G utility acceptance。下一轮 keyed rerun 应首先验证模型是否在实现前读取
三个 CSV；如果仍 wrong，继续按新 trace 收录下一层 tools/control case。

## 5.58 2026-07-04 complete-read duplicate hard-stop timing

`77e8e46` 的 keyed rerun 证明 5.57 的 fact-source gate 已 live-cleared：TaskSpace 在实现前读取了 schema 和三个 CSV。
新的 blocker 是 validation rework duplicate-read recovery 的 hard-stop timing。

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-complete-read-duplicate-hardstop-too-early` | 第一次 duplicate-read rejection 刚产生 `complete read_file context` / `eof_reached=true` / `no additional file lines are hidden` 强反馈，就立即升级 hard-stop，模型没有机会用这条反馈转向 patch | complete-read duplicate-read recovery 先给一次 provider recovery；第二次重复或已有 repeated-blocked-action 证据才 hard-stop | `validation_rework_duplicate_read_complete_context_gets_one_recovery_before_hard_stop`; `validation_rework_duplicate_read` 7/7 PASS; `validation_rework` 20/20 PASS; CoE H-068/E-143/E-144 |

结论：这是 R4-D control/feedback focused 修复，不是 utility acceptance。下一轮 keyed rerun 要验证模型是否在收到 complete-read duplicate
feedback 后发出 `apply_patch`，或暴露下一层 repair-contract/actionability 问题。

## 5.59 2026-07-04 validation rework block rejection wording drift

`9f370dd` 的 keyed rerun 证明 5.58 的 complete-read hard-stop timing 已 live-cleared：第一次完整 target read 后没有再直接
`TaskSpaceValidationReworkDuplicateReadHardStopV1`。新的 blocker 是 block rejection 的反馈分类漂移。

本轮新增的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-block-rejection-wording-drift` | runtime 正确拒绝 `"Need to read schema.json ... before fixing process.py"`，但 session 只识别旧 wording `already recorded implementation source evidence`；新 wording `dependency evidence already identifies the implementation artifact or validation rework target` 没被结构化，导致 provider 没收到 `missing_source_visibility_blocker_rejected` | old/new missing-source block rejection 共用 recognizer；结构化 feedback、progress hint 和 actionability 候选统一识别，并要求下一步 `apply_patch` | keyed rerun `20260704-150817-545`; CoE H-069/E-146; `action_contract_prompt_structures_validation_rework_missing_source_blocker_rejection` |

结论：这是 R4-D feedback classification 修复，不是 utility acceptance。下一轮 keyed rerun 要验证 runtime block rejection 是否被发送为
结构化 tool feedback，并观察模型是否转向 `apply_patch`，或暴露下一层 patch synthesis 问题。

## 5.60 2026-07-04 validation rework patch directive buried after evidence

`431e0ee` 的 keyed rerun 没有复现 5.59 的 block-rejection path，因此 H-069 仍是 focused-fixed、live pending。该轮进入了
schema validation rework，并暴露新的反馈布局问题：事实齐全，但动作指令太靠后。

本轮新增的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-patch-directive-buried-after-evidence` | `TaskSpaceValidationReworkPatchOnlyRecoveryV1` / `DuplicateReadRecoveryV1` 先输出长 repair/evidence，再输出 `Current required behavior`；模型在完整 `process.py` 和 repair contract 已可见时仍重复读 | recovery payload 先输出 patch/block action directive，再输出 previous feedback 和 long evidence；明确 evidence 只用于构造 patch，不是重复 discovery 许可 | keyed rerun `20260704-151923-804`; CoE H-070/E-149; ordering tests |

结论：这是 R4-D recovery actionability/layout 修复，不是 utility acceptance。下一轮 keyed rerun 要验证模型是否在 first complete-read
recovery 后转向 `apply_patch`，或进入更低层的 patch synthesis/validation failure。

## 5.61 2026-07-04 closed action-space noncompliance

`41b1cf6` 的 keyed rerun 证明 5.60 的 recovery ordering 已进入 live path，但没有解决终局问题。模型仍在 closed
repair action space 下输出非法 `read_file`。

本轮新增未解问题类型：

| Case | Observed | Implication |
|---|---|---|
| `validation-rework-closed-action-space-noncompliance` | active projection 明确 `next_valid_actions` 为使用完整 read result、不要 read/search、`apply_patch generate_organization.py`；current node contract 明确 `allowed action classes: edit, control(...)` 且 read/search 会被 blocked；provider 仍输出 `read_file generate_organization.py`，最终 duplicate-read hard-stop | 这已不是 feedback 文字缺失/顺序问题，而是 action-space 闭合后模型仍可选择非法动作；本轮采用 action schema narrowing：已有 visible validation rework target read 后，taskspace-action-v1 `read_file` 在转换为 shell read 前直接拒绝，并路由到 patch-only recovery |

结论：H-070 live-applied but insufficient；H-071 已有 focused control-layer fix。下一步需要 keyed rerun 验证该 schema narrowing
是否让模型进入 `apply_patch`，或暴露更深层的 patch synthesis/repair quality 问题。

## 5.62 2026-07-04 closed action rejection must not downgrade to NoAction

`26d991c` 的 keyed rerun 证明 H-071 schema narrowing 已进入 live path：非法 target re-read 没有再变成普通 shell
`read_file`，而是在 action-contract schema 转换前被拒绝。但新的 blocker 是 feedback routing：正确 rejection 被 session
降级成泛化 NoAction recovery，丢失了 validation rework 的 patch-only 语义。

```text
RunDir: target/r4-org-json-real-keyed-20260704bq-closed-action-narrowing/runs/terminal_bench__organization-json-generator/20260704-154904-391
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 13
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| `TaskSpaceActionV1 rejected: validation_rework_closed_action_space_read_disallowed:read_file` | H-071 的 schema/control 层拒绝 live 生效，非法 read 没有进入 ordinary shell tool |
| 后续插入 `TaskSpaceNoActionRecoveryV1` | rejection 语义在 session recovery 层被降级，未进入 patch-only repair path |
| provider 多次重复 `read_file generate_organization.py` | NoAction recovery 没有传达“已闭合 action space，下一步只能 patch”的语义 |
| 最终 `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded` | 错误 recovery 通道继续烧 provider/node budget |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-closed-action-rejection-noaction-downgrade` | closed target re-read 已被 action-contract 拒绝，但 rejection marker 未被识别为 implementation-needs-edit，recovery 降级为泛化 `TaskSpaceNoActionRecoveryV1` | `validation_rework_closed_action_space_read_disallowed` 归入 implementation-needs-edit、recent-output patch-only hint 和 validation rework patch-only recovery；第一条 closed schema rejection 给一次 patch-only recovery，第二次同类 rejection hard-stop | keyed rerun `20260704-154904-391`; CoE H-072/E-154/E-155; `implementation_recovery_selects_patch_only_after_closed_action_space_read_reject`; `validation_rework` 23/23; `action_contract_prompt` 29/29 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_selects_patch_only_after_closed_action_space_read_reject --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --lib
  PASS: 23/23

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：该 case 不是工具失败没有传给 runtime，也不是 H-071 schema narrowing 失败。底层 rejection 已正确产生；缺陷发生在
session feedback routing，把 repair-actionability rejection 降级成 NoAction。修复不允许继续读，也不扩大重试空间，只保证
closed-action rejection 进入 patch-only recovery 并有 bounded hard-stop。

## 5.63 2026-07-04 post patch-only noncompliance remains a utility blocker

`d61186a` 的 keyed rerun 证明 5.62 已 live-cleared：closed-action rejection 没有再降级到
`TaskSpaceNoActionRecoveryV1`，而是进入 `TaskSpaceValidationReworkPatchOnlyRecoveryV1`。但 utility 仍未成功。

```text
RunDir: target/r4-org-json-real-keyed-20260704br-closed-reject-patch-recovery/runs/terminal_bench__organization-json-generator/20260704-160458-158
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 15
right_wall_time_ms: 281902
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
```

关键观察：

| Signal | 结论 |
|---|---|
| closed-action rejection 后插入 `TaskSpaceValidationReworkPatchOnlyRecoveryV1` | H-072 的 NoAction downgrade 已 live-cleared |
| 第二次 closed-action rejection 后插入 `TaskSpaceValidationReworkPatchOnlyHardStopV1` | loop 已 bounded，不再继续烧到 provider-node hard stop |
| run 已推进到 successful edit、coverage-correct schema validation、完整 required-property failure summary | 前置 inspect/validation feedback 链路继续有效 |
| `process_csv.py` 已完整读取，`eof_reached=true`，repair contract 已存在，但 provider 仍重复 `read_file process_csv.py` | 新 blocker 是 patch-only 反馈后的 repair synthesis/模型服从问题，不是 NoAction routing |

本轮新增未解问题类型：

| Case | Observed | Implication |
|---|---|---|
| `validation-rework-post-patch-only-noncompliance` | validation rework 已有 complete target read、schema repair contract、patch-only recovery 和 closed-action rejection；provider 仍连续请求同一 target read，最终按 bounded hard-stop 退出 | 已先采用通用 repair-synthesis scaffold：patch-only recovery 将 schema rename hints / missing required properties / traceback signals 转成 patch construction steps，并再次约束 native apply_patch grammar；模型/profile escalation 仍保留为下一层候选 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_selects_patch_only_after_target_read_evidence --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_selects_patch_only_after_closed_action_space_read_reject --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

结论：H-072 live-cleared；H-073 focused-fixed，real rerun pending。R4-G utility 仍未过，下一轮 keyed rerun 要验证 scaffold
是否让 live model 输出 `apply_patch`，否则再升级到模型/profile 或更强 patch-plan gate。

## 5.64 2026-07-04 start_task output contract downgrade

`d07644c` 的 keyed rerun 没有验证到 5.63 的 repair-synthesis scaffold，因为路径更早分叉。provider 的顶层 rationale
仍写明要产出 `organization.json` 并符合 `schema.json`，但 `start_task` args 把 `initial_success_criteria` /
`initial_output_contracts` 降级成 inspect-style 文件发现和 schema summary。runtime 接受了该弱 contract，后续把
`python process.py` 这种 generator-only 成功通过 `TaskSpaceForcedValidationCloseoutV1 trigger=validation_success_after_tool_drain`
关掉 validation node，未强制同一次 validation 覆盖 `organization.json` 和 `schema.json`。

```text
RunDir: target/r4-org-json-real-keyed-20260704bs-repair-synthesis-scaffold/runs/terminal_bench__organization-json-generator/20260704-161809-385
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 8
right_wall_time_ms: 106910
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
```

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `start-task-output-contract-downgrade` | 顶层 rationale/用户目标包含 generated output 和 schema obligations，但 provider-supplied `start_task` args 可把 output contract 写窄成 inspect summary；validation closeout 因而接受 generator-only tool success | `taskspace_control.start_task` transport 将 top-level rationale 合并进 objective；runtime 从 objective、success criteria 和 evidence refs 推导 generated JSON output contract 与 schema/validator contract，且不重复已有显式 contract | keyed rerun `20260704-161809-385`; CoE H-074/E-158/E-159; `taskspace_action_contract_preserves_start_task_rationale_as_objective`; `start_task_derives_output_contracts_from_objective_when_model_records_inspect_outputs`; generator-only closeout regressions |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core taskspace_action_contract_preserves_start_task_rationale_as_objective --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core start_task_derives_output_contracts_from_objective_when_model_records_inspect_outputs --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core force_finish_validation_rejects_generator_only_output_contract_success --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_node_derives_output_target_from_success_criteria_for_schema_check --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：该 case 是 capability/contract 层问题，不是普通 tool execution failure。raw 工具成功存在，但成功含义被弱
start-task contract 误解释成任务成功。修复原则是让 runtime 恢复 objective-level output/schema obligations，而不是给
`organization-json-generator` 写专用 validator。

## 5.65 2026-07-04 validation rework read exception conflicts with patch-only closure

`72ffe01` attested keyed rerun 证明 5.64 的 output/schema contract enforcement 已进入 live path：TaskSpace 拒绝了单独
`python generate_org.py`，要求 `python generate_org.py && python -m jsonschema -i organization.json schema.json`。但该轮仍
没有成功，原因回到 validation rework patch-only 分支。

```text
RunDir: target/r4-org-json-real-keyed-20260704bu-start-task-contracts-attested/runs/terminal_bench__organization-json-generator/20260704-163615-799
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 13
right_wall_time_ms: 254407
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
current_git_head: 72ffe01c218a1b5e659a8defe9bac21272f45a2d
whale_binary_sha256: 5895318ca496ff4da368ad1a48f82612b29b8f07561f558402b58d58d322ab46
```

关键观察：

| Signal | 结论 |
|---|---|
| `python generate_org.py` 被 validation coverage gate 拒绝 | H-074 的 output/schema contract enforcement live 生效 |
| `python generate_org.py && python -m jsonschema -i organization.json schema.json` 执行，输出 `missing_required_properties` | validation failure 语义进入工具反馈 |
| `generate_org.py` 完整读取，`eof_reached=true`，随后出现 `TaskSpaceValidationReworkPatchOnlyRecoveryV1` 和 `Patch construction scaffold` | H-073 scaffold live 可见 |
| provider 仍重复 `read_file generate_org.py`，被 `validation_rework_closed_action_space_read_disallowed:read_file` 拒绝，最后 hard-stop | 闭合 action space 仍未收敛到 patch |
| 静态 `TaskSpaceActionContractV1` 同时写着 named validation rework artifact 的 `read_file` 可有效 | 静态 contract 与动态 patch-only recovery 存在语义冲突 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-static-read-exception-conflicts-with-patch-only` | 静态 action contract 允许 `read_file` targeting named validation rework artifact；动态 recovery 在 complete target read 后又说 read 已闭合，只能 patch/block；provider 选择了静态例外路径 | 静态 implement rule 明确：validation rework target read 只在尚未 complete-read 前有效；若 state/projection/recent feedback 出现 `validation_rework_patch_only_after_target_read`、`complete_read/eof_reached=true` 或 `validation_rework_closed_action_space_read_disallowed`，read/list/search/schema inspection 均无效，只能 apply_patch 或 block_node | keyed rerun `20260704-163615-799`; CoE H-075/E-160/E-161; `taskspace_static_contract_closes_complete_validation_rework_reads`; `validation_rework` 24/24 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core taskspace_static_contract_closes_complete_validation_rework_reads --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_selects_patch_only_after_target_read_evidence --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core implementation_recovery_selects_patch_only_after_closed_action_space_read_reject --lib
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --lib
  PASS: 24/24

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：该 fix 不禁止 validation rework 的第一次 target read；它只关闭 complete-read 之后的重复 read 例外。下一轮 keyed
rerun 要验证静态/dynamic contract 不再冲突后，provider 是否转向 `apply_patch`，否则再升级到 native-tool transport /
模型 profile / patch-plan gate。

## 5.66 2026-07-04 failed-edit refresh reopens complete validation rework read

`c8a2d16` attested keyed rerun 证明 5.65 的静态 contract 冲突已经清除到下一层：provider 在完整 target read 和 patch-only
recovery 之后首次进入了 `apply_patch`。但 patch hunk 因上下文不匹配失败后，runtime/projection 又把 same-target refresh read
作为合法下一步暴露出来，即使 `process.py` 已经完整读取且 `eof_reached=true`。

```text
RunDir: target/r4-org-json-real-keyed-20260704bv-static-read-override/runs/terminal_bench__organization-json-generator/20260704-164819-131
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_wall_time_ms: 206321
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
current_git_head: c8a2d1681e44dd1e19edf303bd6e180bffd5630d
whale_binary_sha256: 136008edf014e6ab1bfc86ae6c0188623723b5089de322a65bfe6348f9dd2eef
```

关键观察：

| Signal | 结论 |
|---|---|
| provider 在 patch-only recovery 后调用 `apply_patch` | H-075 live-cleared 到 patch branch |
| `apply_patch verification failed: Failed to find expected lines` | 新 blocker 是 failed-edit recovery，不是 read refusal 本身 |
| validation rework target `process.py` 完整读取，`eof_reached=true` | 没有隐藏行需要 refresh |
| projection 暴露 `only same validation rework target refresh reads are allowed after a failed edit` | failed-edit refresh 例外缺少 complete-read 边界 |
| 后续 repeated `read_file process.py` 被 closed action-space gate 拒绝并进入 budget hard-stop | projection 和 gate 的反馈语义不一致 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-failed-edit-refresh-reopens-complete-read` | failed `apply_patch` 后，same-target refresh-read 例外对完整 read 也生效；projection 说可 refresh，runtime closed gate 又拒绝 read，导致 provider 继续 retry | refresh-read 例外只在 failed edit 之后且先前 target read 不是 `eof_reached=true` 时开放；完整 read 后失败 patch 仍保持 read/search closed，只能修正 `apply_patch` 或 block_node | keyed rerun `20260704-164819-131`; CoE H-076/E-162/E-163; `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; `validation_rework` 24/24 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_map::runtime::tests::validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked
  PASS: 24/24

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --locked
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：该 case 不是 tool execution failure，也不是 failure marker 缺失。失败 patch 的语义已经存在，问题是 failed-edit
refresh policy 把“可能因为截断/陈旧导致 hunk 失败”的例外错误套用到 `complete_read/eof_reached=true`。修复保留 bounded/truncated
read 的 refresh 逃生口，但完整读取后仍强制沿 patch 修正路径前进。

## 5.67 2026-07-04 partial-excerpt blocker accepted after patch grammar failure

`dc2a986` keyed rerun 证明 5.66 的 failed-edit refresh loop 已清除：provider 没有停在失败 patch 后重复 `read_file`，而是进入
`apply_patch` / patch grammar recovery 分支。但该轮仍未通过 public validation，新的 failure mode 是 validation rework node
被一个 missing-source blocker 错误关闭。

```text
RunDir: target/r4-org-json-real-keyed-20260704bw-complete-read-failed-edit-closed/runs/terminal_bench__organization-json-generator/20260704-170158-193
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 12
right_wall_time_ms: 403592
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
current_git_head: dc2a98680400249e896b36f06c91378fc046bd17
whale_binary_sha256: d3c3611d0bc27779110238090be6c87c8b6b6e12f616085b896eae91722238bf
```

关键观察：

| Signal | 结论 |
|---|---|
| provider 进入 `apply_patch`，随后出现 `apply_patch verification failed` | H-076 清除了 failed-edit refresh-read loop，问题推进到 patch 质量/语法 |
| 第二次 patch 被 `apply_patch_mixed_native_unified:process.py` 拒绝，runtime 插入 `TaskSpaceApplyPatchNativeHunkRecoveryV1` | patch grammar recovery 正常触发 |
| provider 随后 `blocked`：`Insufficient file content visibility... only partial excerpt... full content is needed... ability to read the full file` | missing-source blocker 换了新 wording |
| runtime 接受该 blocker，后续 `provider_context_missing:current_main_node_missing` | repairable validation rework node 被错误关闭 |
| final candidate 变成 CSV/schema missing blocker，但这些文件已经在 `result-2..result-5` 成功读取 | 关闭当前节点后反馈链路退化为错误终止语义 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-partial-excerpt-blocker-wording-drift` | 已完整读取 target 且 patch grammar recovery 后，provider 用 `partial excerpt/full content needed/ability to read full file` 表达 missing-source blocker；recognizer 未覆盖，runtime 接受 blocker 并关闭 node | missing-source blocker recognizer 覆盖 partial-excerpt/full-content wording；complete-read validation rework blocker rejection 明确只能重试 `apply_patch`，不得 refresh read | keyed rerun `20260704-170158-193`; CoE H-077/E-164/E-165; `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; `validation_rework` 24/24 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_map::runtime::tests::validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --locked
  PASS: 24/24

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --locked
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：该 case 和 H-069 同属 missing-source blocker feedback classification，但触发点不同。H-069 是 runtime 新 rejection wording
没有被 session 识别；H-077 是 provider 的 blocker wording 没被 runtime recognizer 拒绝，导致当前节点被关闭。

## 5.68 2026-07-04 repeated malformed patch hunks after complete target read

`42feaee` keyed rerun 证明 5.67 的 partial-excerpt blocker 已清除：run 没有接受 blocker，也没有进入
`provider-context-missing`。但 validation rework 仍未通过，新的 blocker 是反复生成 fragile/malformed `apply_patch`。

```text
RunDir: target/r4-org-json-real-keyed-20260704bx-partial-excerpt-blocker-reject/runs/terminal_bench__organization-json-generator/20260704-171735-273
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 14
right_wall_time_ms: 512227
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
current_git_head: 42feaeea93f88d717ca4f48d838af2152ee6fe94
whale_binary_sha256: 5a699fafb8d844b47c579e895a4cec71bd020bd4c94dbf9d98d134fa4ac7b88f
```

关键观察：

| Signal | 结论 |
|---|---|
| run 未进入 `provider-context-missing`，仍停留在 `node-4` implement_solution | H-077 live-cleared |
| 多次 `apply_patch` 失败为 `Failed to find expected lines` | patch context/hunk strategy 不稳定 |
| 后续 patch 继续混合 native wrapper 与 unified/range hunk syntax | apply_patch grammar recovery actionability 不足 |
| 出现 live malformed wrapper：`*** Update File: process.py` 在 `*** Begin Patch` 前 | normalizer 缺少该 wrapper 变体 |
| 最后 `apply_patch_mixed_native_unified:process.py` 后触发 `TaskSpaceProviderBudgetHardStopV1` | 未能在 node request budget 内收敛到合法 patch |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-repeated-malformed-patch-hunks-after-complete-read` | complete target read 后多次 expected-lines mismatch，recovery 仍允许 fragile Update File hunks；live malformed wrapper 未被正规化，最终 mixed-native-unified rejection + node budget hard-stop | complete target read + expected-lines/context mismatch 时，edit-failure recovery 强制 full rewrite：`*** Delete File` + `*** Add File`；normalizer 支持 `Update File` 在 `Begin Patch` 前的 live wrapper | keyed rerun `20260704-171735-273`; CoE H-078/E-166/E-167; focused tests; `apply_patch --lib` 47/47 |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core complete_validation_rework_expected_lines_failure_forces_full_rewrite --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core taskspace_action_contract_normalizes_misordered_begin_update_mixed_patch --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core apply_patch --lib --locked
  PASS: 47/47

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib --locked
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --lib --locked
  PASS: 25/25

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：该 case 是 tools 能力层和反馈层交叉问题。工具执行失败语义存在，grammar recovery 也存在；缺口是完整 target read
下仍让 provider 走脆弱 hunk patch，而不是升级为整文件 rewrite。非 `--lib` 的 `cargo test -p codex-core apply_patch --locked`
在通过 47 个 lib apply_patch tests 后继续跑 `tests/all.rs` 并触发既有 stack overflow，因此本轮使用 `--lib` 作为 scoped gate。

## 5.69 2026-07-04 patch-only schema synthesis too weak

`0b8e5a1` keyed rerun 证明 5.68 的 repeated malformed patch hard-stop 已消失：本轮没有进入
`Failed to find expected lines` 循环，也没有 `apply_patch_mixed_native_unified` 或 provider-node budget hard-stop。但任务仍未
通过，新的 blocker 是 patch-only recovery 的 schema repair synthesis 不够可执行。

```text
RunDir: target/r4-org-json-real-keyed-20260704by-full-rewrite-after-patch-mismatch/runs/terminal_bench__organization-json-generator/20260704-173608-346
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 11
right_wall_time_ms: 174600
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
current_git_head: 0b8e5a1802f6aa59018715fe3ddf3219b042b289
```

关键观察：

| Signal | 结论 |
|---|---|
| validation semantic summary: `missing_required_properties: members, averageDepartmentBudget, totalEmployees, skillDistribution, departmentSizes, projectStatusDistribution, averageYearsOfService` | validator 已给出可直接修复的字段集合 |
| provider 完整读取 `generate_organization.py`，`TaskSpaceReadFileSummaryV1: lines_read=87 eof_reached=true` | target source 已完整可见 |
| runtime 插入 `TaskSpaceValidationReworkPatchOnlyRecoveryV1` | patch-only recovery 路由正确 |
| provider 随后两次输出 `read_file generate_organization.py`，理由是 `Need full content` / `Read the full content` | 模型仍把 repair synthesis 理解为需要 discovery |
| 两次 read 均被 `validation_rework_closed_action_space_read_disallowed:read_file` 拒绝，最后 `TaskSpaceValidationReworkPatchOnlyHardStopV1` | runtime/control 层正确，缺口在反馈 actionability |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-patch-only-schema-synthesis-too-weak` | patch-only recovery 泛化提示使用 validation failure / repair contract，但没有把 missing fields 和 rename hints 前置成直接 patch plan；provider 继续 closed read | recovery 增加 `Schema repair synthesis from current validation failure`，列出 exact missing properties、rename hints、exact schema spelling，并说明这是 patch-construction requirement，不是再次读取理由 | keyed rerun `20260704-173608-346`; CoE H-079/E-168/E-169; focused tests |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core implementation_recovery_selects_patch_only_after_target_read_evidence --lib --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core implementation_recovery_selects_patch_only_after_closed_action_space_read_reject --lib --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --lib --locked
  PASS: 25/25

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib --locked
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：该 case 不是 H-072 的 recovery routing 降级，也不是 H-075/H-076 的 action-space 冲突。`read_file` 已被正确拒绝；
问题是反馈没有把 validator 的字段级失败语义转译成足够明确的 patch 合成任务。

## 5.70 2026-07-04 missing fact-source bootstrap does not transition

`6ef01cc` keyed rerun 没有到达 5.69 的 validation rework path；它在 inspect 阶段更早失败。H-079 因而仍是 focused-fixed /
live-unverified，新 blocker 是 missing fact-source bootstrap 后没有自动进入 implementation。

```text
RunDir: target/r4-org-json-real-keyed-20260704bz-schema-repair-synthesis/runs/terminal_bench__organization-json-generator/20260704-174618-510
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 7
right_wall_time_ms: 29239
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
current_git_head: 6ef01cc
```

关键观察：

| Signal | 结论 |
|---|---|
| provider 重复 `list_files`，duplicate inspect gate 拦截 | inspect 行为已卡在重复 discovery |
| `TaskSpaceMissingFactSourceBootstrapV1` 读取 `schema.json` | bootstrap capability 生效 |
| `TaskSpaceRepeatedBlockedInspectBootstrapV1` 随后读 bounded json/csv/yaml 内容 | runtime 已经能补充更多证据 |
| 没有 `TaskSpaceForcedInspectTransitionV1`，current node 仍是 `inspect_code_context` | phase bridge 缺失 |
| 最后 `TaskSpaceProviderBudgetHardStopV1 reason=provider_node_request_hard_limit_exceeded node_request_count=5/5`，没有生成 `organization.json` | budget 兜底太晚 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `inspect-missing-fact-source-bootstrap-no-transition` | missing fact-source bootstrap 读完后仍把控制权交回 inspect 模型，模型继续 list/search/read 直到 node budget hard-stop | bootstrap 返回后若 required fact-source coverage 已清空，session 立即触发 `inspect_missing_fact_source_bootstrap_complete` forced transition；runtime 接受该 trigger | keyed rerun `20260704-174618-510`; CoE H-080/E-170/E-171; focused tests |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core inspect_missing_fact_source_bootstrap_complete_forces_transition_after_coverage --lib --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core inspect_missing_fact_sources --lib --locked
  PASS: 2/2

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core forced_inspect_transition_accepts_duplicate_read_search_gate_recovery --lib --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core forced_inspect_transition --lib --locked
  PASS: 5/5

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core inspect_bootstrap --lib --locked
  PASS: 3/3

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_rework --lib --locked
  PASS: 25/25

CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib --locked
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：这不是允许未读完 fact-source 时提前进入 implementation。既有 `inspect_missing_fact_sources_block_manual_and_forced_finish_until_read`
仍通过；修复只处理 bootstrap 已补齐覆盖后的自动过渡。

## 5.71 2026-07-04 bootstrap read classification and hard-stop convergence bridge

`5b9bdc4` keyed rerun 证明 5.70 的修复仍未 live-clear。它暴露了两个相邻缺口：missing fact-source bootstrap
本身生成的 read-only shell command 被能力层误判为 edit；而在模型后续手动读完整事实源之后，runtime 仍在 pre-dispatch hard-stop
前没有强制完成 inspect transition。

```text
RunDir: target/r4-org-json-real-keyed-20260704ca-bootstrap-transition/runs/terminal_bench__organization-json-generator/20260704-175447-182
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 13
right_wall_time_ms: 39757
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
current_git_head: 5b9bdc4
```

关键观察：

| Signal | 结论 |
|---|---|
| `TaskSpaceMissingFactSourceBootstrapV1` 生成 `printf ...; sed -n ... && awk ...` 读取 CSV | runtime 已尝试补 fact-source |
| bootstrap result 被 inspect gate 拦截为 `Requested tool shell_command action class: edit` | 内部 read-only 命令被 shell action classifier 误判 |
| 误判根因是 awk summary 中 `if (lines > 240)` 命中 `>` redirection/edit heuristic | 这是 capability-layer classification 问题 |
| 后续手动 reads 形成 verified evidence：`schema.json`、`departments.csv`、`projects.csv`、`employees.csv` | inspect coverage 最终已经完整 |
| active projection 明确给出 `finish_node -> implement_solution` 为 next valid action，但下一步仍是 `TaskSpaceProviderBudgetHardStopV1 node_request_count=10/10` | feedback/control bridge 在 hard-stop 前缺失 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `inspect-bootstrap-read-classification-and-hard-stop-transition-gap` | missing fact-source bootstrap 的 read-only shell command 被误判为 edit；即使后续 inspect evidence 完整，node hard-stop 前也只提示模型 finish_node，没有 runtime forced transition | Unix bounded read summary 避免生成 `>`，真实 bootstrap 命令分类锁为 `ActionClass::Read`；pre-dispatch hard-stop 前若 inspect progress ready，则以 `inspect_hard_stop_progress_convergence` 强制 transition；runtime 接受该 trigger | keyed rerun `20260704-175447-182`; CoE H-081/E-172/E-173; focused tests |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core missing_fact_source_bootstrap_command_reads_bounded_declared_artifacts --lib --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core shell_action_classifier_identifies_core_taskspace_classes --lib --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core inspect_hard_stop_progress_convergence_forces_transition_after_coverage --lib --locked
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core inspect_missing_fact_sources --lib --locked
  PASS: 2/2

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core forced_inspect_transition --lib --locked
  PASS: 5/5

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core inspect_bootstrap --lib --locked
  PASS: 3/3

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib --locked
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --lib --locked
  PASS: 25/25

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：这不是放宽 inspect read-only gate。bootstrap 命令仍是普通 bounded read，并通过 classifier/read tests 锁定；
hard-stop bridge 也仍依赖 `action_map_current_inspect_progress_ready_for_transition()`，不会绕过 missing fact-source 或 unread script guard。

## 5.72 2026-07-04 validation required-command advisory-to-runtime bridge

`51edaaf` keyed rerun 证明 H-081 已 live-clear：TaskSpace 跨过 inspect、进入 implementation、生成并验证
`organization.json`。新的失败点出现在 validation feedback 层：runtime 已经知道唯一 coverage-correct 的下一条 validation
命令，但只把它作为 `TaskSpaceValidationNeedsTestRecoveryV1` advisory 返回给模型。模型继续尝试不可用 pytest 和 generator-only
命令，最终耗尽 provider request budget。

```text
RunDir: target/r4-org-json-real-keyed-20260704cb-hardstop-bridge/runs/terminal_bench__organization-json-generator/20260704-180719-471
reported_evidence_level: E2-candidate
valid_pair: True
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_tool_call_count: 17
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
final_hard_stop: provider_request_hard_limit_exceeded request_count=20/20 node_kind=smoke_test
```

关键观察：

| Signal | 结论 |
|---|---|
| `TaskSpaceForcedInspectTransitionV1 trigger=inspect_no_action_with_evidence` | H-081 inspect bridge live-clear |
| `TaskSpaceValidationNeedsTestRecoveryV1` 包含 `python generate_organization.py && python -m jsonschema -i organization.json schema.json` | 反馈语义存在，且 exact next action 未丢 |
| provider 先后尝试 `python -m pytest -v`、`pytest`、generator-only 命令 | advisory 未被执行语义接管 |
| runtime 再次拒绝 generator-only，并重发 recovery，随后全局 request hard-stop | 控制层把确定命令交给模型重试，预算被消耗 |

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-output-contract-next-action-advisory-loop` | validation gate 已给 exact `run_test with command ...`，但 `TaskSpaceValidationNeedsTestRecoveryV1` 只是 advisory；模型忽略后继续烧请求预算 | session 在 validation recovery item 插入后、validation closeout 前，识别 changed-artifact/output-contract gate 的 exact command，自动通过 `shell_command` 执行，并以 `ActionClass::Test` 记录；trace 记录 `TaskSpaceValidationRequiredCommandBootstrapV1` | keyed rerun `20260704-180719-471`; CoE H-082/E-174/E-175; `validation_required_command_bridge`; `validation_needs_test`; `validation_output_contract`; `action_contract_prompt`; `validation_rework` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_required_command_bridge --lib --locked
  PASS: 2/2

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_needs_test --lib --locked
  PASS: 1/1

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt_structures_output_contract_coverage_failure --lib --locked
  PASS: 1/1

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_output_contract --lib --locked
  PASS: 1/1

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib --locked
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --lib --locked
  PASS: 25/25

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：这不是放宽 validation gate，也不是让 runtime 猜测任意测试命令。bridge 只接受
`TaskSpaceGateRecoveryV1` 中 changed-artifact/output-contract coverage gate 已明确给出的 exact command；普通 pytest 失败、
local validator coverage gate、无 gate 的失败都不会自动执行。

## 5.73 2026-07-04 validation required-command chained gate bridge

`37ebc22` keyed rerun 证明 validation bridge 已 live-trigger，但也暴露了 staged validation gate 的第二层缺口。bridge
执行了 first-hop changed-artifact command `python transform.py`，该命令随后被 output-contract gate 拒绝，并返回更严格的
combined command `python transform.py && python -m jsonschema -i organization.json schema.json`。旧 bridge 没继续追这条 nested
gate command，而是把 first-hop gate rejection 记录成 failed Test，导致 runtime 从内部 gate rejection 创建 validation rework。

```text
RunDir: target/r4-org-json-real-keyed-20260704cc-validation-bridge/runs/terminal_bench__organization-json-generator/20260704-182700-317
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: engineering_unclean
right_tool_call_count: 10
right_public_validation_exit_code: 1
final_hard_stop: provider_node_request_hard_limit_exceeded request_count=14/20 node_kind=implement_solution node_request_count=6/5
```

本轮新增并 focused 修复的问题类型：

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-required-command-bridge-one-hop-only` | bridge 执行 first-hop `python transform.py` 后，把 output-contract gate rejection 记录为 failed Test，并提前进入 validation rework | bridge 最多追 3 次 changed-artifact/output-contract gate 链；nested gate 使用 `TaskSpaceValidationRequiredCommandBootstrapChainedV1` 记录；只把最终命令结果写入 `ActionClass::Test` | keyed rerun `20260704-182700-317`; CoE H-083/E-176/E-177; `validation_required_command_bridge`; `validation_needs_test`; `action_contract_prompt`; `validation_rework` |

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_required_command_bridge --lib --locked
  PASS: 3/3

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_needs_test --lib --locked
  PASS: 1/1

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core action_contract_prompt --lib --locked
  PASS: 29/29

CODEX_SKIP_VENDORED_BWRAP=1 cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-core validation_rework --lib --locked
  PASS: 25/25

CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --manifest-path third_party/codex-cli/codex-rs/Cargo.toml --all --check
  PASS

git diff --check
  PASS

CODEX_SKIP_VENDORED_BWRAP=1 cargo build --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-cli --bin whale --locked
  PASS
```

边界说明：这仍不是执行任意模型建议命令。链式 bridge 只追 TaskSpace 自己的 changed-artifact/output-contract
coverage gate 产出的 stricter exact command，并有固定 3-hop 上限；同一命令不会自循环。

## 5.74 2026-07-04 patch-only recovery tail action lock

`2ab7a05` keyed rerun 证明 H-083 live-clear：trace 出现 `TaskSpaceValidationRequiredCommandBootstrapChainedV1`，
并执行 combined command。新的失败点进入 validation rework：目标文件已完整读取，patch-only recovery 已正确禁止 read/search，
但长 evidence 尾部的压缩预览让 provider 以 `current projection truncated` 为理由再次读 target。

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-patch-only-tail-truncation-drift` | patch-only recovery 顶部语义正确，但 evidence 尾部没有再次锁定动作；provider 受 truncated preview 影响继续 read_file | recovery 在 evidence 后追加 `Final action lock`，明确 complete/eof target 下 projection truncation 不是重读理由，只能 apply_patch 或 block_node | keyed rerun `20260704-183656-438`; CoE H-084/E-178/E-179; `implementation_recovery_selects_patch_only_after_target_read_evidence`; `validation_rework`; `action_contract_prompt` |

验证：focused test PASS；`validation_rework` 25/25 PASS；`action_contract_prompt` 29/29 PASS；fmt/check/build PASS。

## 5.75 2026-07-04 failed-edit whole-file replacement tail lock

`538c116` keyed rerun 显示 H-084 已推动 provider 进入 apply_patch，但 failed edit 后仍回到 fragile hunk 和 refresh-read。

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-failed-edit-fragile-patch-fallback` | complete target read 后，expected-lines/mixed-hunk patch 失败仍导致 read refresh 或继续 ranged hunk | patch-only final action lock 明确要求 whole-file native replacement：`*** Delete File` + `*** Add File` | keyed rerun `20260704-184541-992`; CoE H-085/E-180/E-181; focused test; `validation_rework` 25/25 |

## 5.76 2026-07-04 schema-context blocker wording drift after patch-only recovery

`e0f8d3d` keyed rerun 证明 validation bridge、nested gate bridge、complete target read、patch-only recovery 均已进入真实链路，
但 provider 在 repairable rework node 上返回了新的 blocker：`Need full content of schema.json ... current projection excerpt ...
insufficient`。runtime 接受该 blocker 后关闭 node，随后语义被扭曲为 local infrastructure unavailable；这不是 schema 缺失，
也不是 Python/jsonschema 不可用，因为同轮已经执行 combined command 并得到 `KeyError: 'id'`。

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-schema-context-blocker-after-patch-only` | complete target read + repair contract 已存在时，full `schema.json` / schema context / projection excerpt insufficient blocker 被接受，节点关闭并退化为 `provider_context_missing` | missing-source blocker recognizer 覆盖 schema/output-structure/full-content/projection-excerpt-insufficient wording；complete target read 时拒绝 blocker，并要求使用 existing evidence `apply_patch` | keyed rerun `20260704-190021-739`; CoE H-086/E-182/E-183; `validation_rework_rejects_missing_current_artifact_visibility_blocker`; `validation_rework` 25/25; `action_contract_prompt` 29/29 |

边界说明：这不是禁止所有 blocker。只有在 validation rework 已有 dependency validation evidence 且完整 target read/repair contract
存在、且 blocker 本质是“还要看 schema/投影不够”的情况下，才归类为 missing-source visibility rejection。真正外部不可编辑原因仍可
用 `block_node` 表达。

## 5.77 2026-07-04 repeated duplicate list_files inspect bootstrap gap

`1fde25d` keyed rerun 没有命中 H-086；run 在更早 inspect 阶段耗尽 node budget。首个 `list_files` 成功返回
`schema.json` 与 CSV 文件清单，provider 随后连续重复同一 `list_files`。runtime 每次都正确拒绝 duplicate read/search，
但只给 advisory recovery，没有执行 bounded bootstrap 或 forced transition。

| Case | Before | After | Evidence |
|---|---|---|---|
| `inspect-duplicate-list-files-no-bootstrap-transition` | repeated duplicate `list_files` 只产生 advisory recovery，直到 inspect node hard-stop；path listing 不算 working evidence，generic bootstrap 也未写入 ActionMap | repeated duplicate read/search 触发 `TaskSpaceRepeatedBlockedInspectBootstrapV1`，bootstrap 输出写入 ActionMap `Read`；带 `=====` section 的 schema/csv 内容计为 input-data working evidence；随后用 `inspect_duplicate_read_search_bootstrap_complete` forced transition | keyed rerun `20260704-191110-654`; CoE H-087/E-184/E-185; focused test; `inspect_bootstrap` 3/3; `forced_inspect_transition` 5/5; `inspect_missing_fact_sources` 2/2 |

边界说明：路径列表本身仍不算 implementation evidence；只有 bounded bootstrap 读取到具体 `.json/.csv/...` 文件内容后，才允许
inspect 收敛。若显式 missing fact-source 未读完，既有 guard 仍阻止 forced finish。

## 5.78 2026-07-04 validation rework recovery counter cross-node leak

`f934ceb` keyed rerun 已越过 inspect duplicate list_files bootstrap，进入 validation rework。runtime 执行了 combined
schema validation command，得到真实 schema failure：root output 缺少 `metadata` 和 `organization`。随后 `node-6`
第一次完整读取 `processor.py` 后，runtime 直接发出 `TaskSpaceValidationReworkPatchOnlyHardStopV1 attempt_count=3`，
而不是先给该 rework node 一次 patch-only recovery。trace 证明 `node-4` 之前已消耗两次 patch-only recovery，
旧计数器把这些次数带到了新的 `node-6`。

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-rework-recovery-counter-cross-node-leak` | validation rework duplicate-read/patch-only recovery 计数是 turn-global；新 rework node 可能继承旧节点次数并在首次 target read 后 hard-stop | duplicate-read 与 patch-only recovery 计数按当前 provider snapshot `node_id` 重置；同节点重复违规仍 hard-stop，跨节点不继承 | keyed rerun `20260704-192256-883`; CoE H-088/E-186/E-187; `validation_rework_recovery_count_resets_when_rework_node_changes`; `validation_rework` 26/26; `action_contract_prompt` 29/29 |

边界说明：这不是放松 patch-only hard-stop。runtime 仍会在同一个 validation rework node 内把重复 read/search/discovery
升级为 hard-stop；修复只防止旧 node 的恢复次数污染新 node。

## 5.79 2026-07-04 apply_patch recovery budget drain after H-088

`851bf3c` keyed rerun 证明 H-088 live-clear：新的 validation rework node 不再继承旧 node 的 patch-only recovery
计数，trace 越过上一层并进入多次 `apply_patch`。新的 blocker 是 patch/edit-failure recovery 自身没有 hard-stop escalation：
runtime 已给出 `TaskSpaceEditFailureRecoveryV1` 和 `TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1`，但 repeated malformed/stale-context
patch 继续消耗 provider node budget，最后退化为 `TaskSpaceProviderBudgetHardStopV1`。

| Case | Before | After | Evidence |
|---|---|---|---|
| `apply-patch-recovery-budget-drain-after-validation-rework` | 同一 implement/rework node 内多次 context mismatch、closed read、unanchored update recovery 后，仍继续 provider sampling，直到 node request budget hard-stop | 新增 `TaskSpaceApplyPatchRecoveryHardStopV1`；apply_patch/edit-failure recovery 计数按 node scoped，第四次 repeated recovery 直接 hard-stop 并保留最后 recovery contract | keyed rerun `20260704-193906-178`; CoE H-089/E-188/E-189; `apply_patch_recovery_hard_stops_after_repeated_same_node_failures`; `apply_patch_` 36/36; `validation_rework` 26/26 |

边界说明：这不是让 malformed patch 成功，也不是自动生成代码补丁；它把“同节点持续无法按 patch feedback 修正”的循环从 generic
provider budget hard-stop 改成专用、可审计的 apply_patch recovery hard-stop。

## 5.80 2026-07-04 whole Python Update File replacement normalization

`eebd0e1` keyed rerun live-cleared H-089：最终 marker 是 `TaskSpaceApplyPatchRecoveryHardStopV1`，不是 generic provider
budget hard-stop。新的问题进入 capability layer：provider 多次表达“整文件替换 `generate_org_json.py`”意图，但使用了
`*** Update File: generate_org_json.py` 后直接跟完整 Python 源码正文。runtime 将其拒绝为 `apply_patch_unanchored_update`，
语义正确但能力不足；这类明确的 Python whole-file replacement 可以安全转成 `Delete File + Add File`，而 `python3 -c`
这类命令 payload 仍必须拒绝。

| Case | Before | After | Evidence |
|---|---|---|---|
| `apply-patch-whole-python-update-replacement-normalization-gap` | 完整 Python 源码正文放在 `*** Update File` 下会被拒绝为 unanchored update，最后进入 patch recovery hard-stop | 单一 `.py/.pyw` Update File、无 hunk/diff/change marker、首个非空行像 Python source 时，normalize 为 `*** Delete File` + `*** Add File`；命令 payload 继续拒绝 | keyed rerun `20260704-195220-438`; CoE H-090/E-190/E-191; `taskspace_action_contract_normalizes_whole_python_update_replacement`; `taskspace_action_contract_rejects_non_diff_update_payload`; `apply_patch_` 36/36 |

边界说明：该能力层修复只接受明显源码整文件替换，不把任意文本、shell/Python 命令、JSON transformation command 当成 patch。

## 5.81 2026-07-04 validation schema feedback chain

H-090 后继续 keyed rerun，`organization-json-generator` 未进入 utility success，而是暴露 schema validation rework 反馈链条的下一组问题。
本节只记录 R4-D 工程收益和未闭环项，不把 R4-G 标记为通过。

| Case | Before | After | Evidence |
|---|---|---|---|
| `validation-schema-repair-rename-hint-gap` | validation feedback 只包含 missing required properties，模型需要猜 `member_ids` 等旧 key 应改成什么 | validator summary 提供 `schema_property_rename_hints=member_ids->members, total_employees->totalEmployees, average_years_of_service->averageYearsOfService` | keyed rerun `20260704-201836-345`; CoE H-091/E-192; `a93391e` |
| `validation-rework-target-read-evidence-order` | complete target-read evidence 被长 validation output 淹没 | target-read evidence 前置，repair context 更早进入 working summary | CoE H-092/E-193; `697ec6c`; focused validation/action-contract/apply_patch tests |
| `validation-rework-complete-target-replacement-scaffold` | complete target read 后 patch-only recovery 没有直接 full replacement scaffold，模型仍可重复读 | recovery 提供 `Delete File + Add File` scaffold；真实 run 越过 repeat-read/no-edit | keyed rerun `20260704-205001-147`; CoE H-093/E-194; `7c7c892` |
| `validation-rework-complete-read-content-visibility` | runtime 有 complete/eof 状态，但 provider-visible 内容可能只是 excerpt，full replacement directive 缺少事实基础 | evidence 显式区分 `full_content_visible` / `summary_excerpt_only`；full-visible 时携带更大 target context | keyed rerun `20260704-210512-809`; CoE H-094/E-195; `44938a3` |
| `validation-rework-full-visible-patch-mismatch-recovery` | full-visible target patch mismatch 后仍允许 read refresh 和 fragile `Update File` | focused recovery 已改成 replacement-only，禁止 read/search/validation、`Update File`、placeholder hunk | CoE H-095/E-196; `dde7173`; real keyed rerun pending |

最新验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core implementation_recovery --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core complete_validation_rework --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core apply_patch_ --lib
cargo fmt --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo check -p codex-core
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale
```

收益判断：

1. R4-D feedback layer 的收益是局部成立的：schema rename hint、target-read visibility、patch-only scaffold、failed-edit recovery closure 都已有 focused 或 live-cross evidence。
2. R4-G utility 仍未通过：`20260704-210512-809` 在 `dde7173` 前仍以 public validation exit 1 / `TaskSpaceApplyPatchRecoveryHardStopV1` 收尾。
3. 下一轮必须对 `dde7173` 重新 attestation + keyed rerun；若仍失败，按新 trace 收录下一层 patch synthesis、schema repair quality 或 validation coverage issue type。

## 5.82 2026-07-04 final gate rejection reason preservation

`dde7173` 后的 keyed rerun 已执行：

```text
RunDir: target/r4-org-json-real-keyed-20260704cp-visible-mismatch-replacement/runs/terminal_bench__organization-json-generator/20260704-212411-195
reported_evidence_level: E2-candidate
outcome_standard: solved
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 20
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
right_open_leaf_nodes: 0
```

收益判断：

1. H-095 有进展但未完全 live-clear：旧 `TaskSpaceApplyPatchRecoveryHardStopV1` 不再是结尾，rework 进入了 successful local schema validation；但 live trace 仍有 fragile `Update File` / placeholder hunk。
2. 新的 R4-D feedback 修复是 `final-answer-gate-rejection-reason-loss`：final readiness gate 的具体错误现在会保留给下一轮 provider，而不是被 boolean `.is_err()` 降级成泛化提示。
3. R4-G utility 仍未通过：public validator 因 `project.members` 使用员工姓名而非员工 id 失败；这属于 schema validation 不覆盖 public relationship oracle 的质量差距。

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core final_answer_gate_rejection_followup_preserves_specific_reason --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core final_readiness --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo check -p codex-core
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale
```

已知相关测试债：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core final_response --lib
```

当前失败在既有 `final_response_completes_running_final_synthesis_node`：该 fixture 期望 running final_synthesis 可直接 final response，
但当前 final readiness gate 要求 success criteria/output contract evidence。该失败不是本次 H-096 改动引入，但属于同一 final gate
区域，需要后续单独收敛。

## 5.83 2026-07-04 duplicate empty Update File wrapper normalization

`b5f2ee2` 后 rerun 证明 H-096 的 live 验证被更早的 patch grammar 层挡住：

```text
RunDir: target/r4-org-json-real-keyed-20260704cq-final-gate-reason/runs/terminal_bench__organization-json-generator/20260704-213755-290
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 15
right_public_validation_exit_code: 1
right_open_leaf_nodes: 1
final_marker: TaskSpaceApplyPatchRecoveryHardStopV1
```

收益判断：

1. H-096 focused fix 仍有效但 live pending：本轮没有到 final_answer gate。
2. 新的 focused 修复是 H-097：重复空 `Update File` wrapper 现在会在 native hunk 检查前被折叠。
3. R4-G utility 仍未通过：run 停在 validation rework patch recovery hard-stop，public validation 失败。

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core duplicate_unwrapped_update_wrapper --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core apply_patch_ --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo check -p codex-core
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale
```

## 5.84 2026-07-04 no-action recovery hard-stop semantics

`af95784` 后 rerun 证明 H-097 没有被当前样本命中；新的 blocker 是 no-action recovery 的 terminal semantics：

```text
RunDir: target/r4-org-json-real-keyed-20260704cr-duplicate-wrapper-normalized/runs/terminal_bench__organization-json-generator/20260704-214746-740
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 10
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
right_open_leaf_nodes: 1
final_marker: TaskSpaceProviderBudgetHardStopV1
```

收益判断：

1. 新收录的 R4-D feedback 修复是 `no-action-recovery-budget-drain`：超过 no-action advisory threshold 后不再继续消耗 provider budget，而是记录专用 `TaskSpaceNoActionRecoveryHardStopV1`。
2. 这是“失败语义缺失”而不是“失败语义扭曲”：runtime 已知道是 no-action recovery，但缺少 terminal marker，最终可见结论被 budget hard-stop 覆盖。
3. R4-G utility 仍未通过：本轮 public validation 失败，且 no-action hard-stop 需要 keyed rerun 才能 live-clear。

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core no_action_recovery --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core apply_patch_ --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo check -p codex-core
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale
git diff --check
```

## 5.85 2026-07-04 natural-language slash fact-source extraction

`3b6b269` 后 rerun 没有命中 no-action recovery；新的 blocker 是 inspect fact-source coverage 误把自然语言
`employees/projects` 识别为 required artifact：

```text
RunDir: target/r4-org-json-real-keyed-20260704cs-no-action-hardstop/runs/terminal_bench__organization-json-generator/20260704-215805-102
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: wrong
right_exec_timed_out: False
right_tool_call_count: 19
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
right_open_leaf_nodes: 1
final_marker: TaskSpaceProviderBudgetHardStopV1
```

收益判断：

1. 新收录的 R4-D capability/feedback 修复是 `inspect-natural-language-slash-fact-source-false-positive`：真实 CSV/schema coverage 仍强制，但自然语言 slash 关系词不会阻塞 inspect finish。
2. H-098 的 live 状态仍是 pending：本轮没有插入 `TaskSpaceNoActionRecoveryV1`。
3. R4-G utility 仍未通过：TaskSpace 还没进入 implementation，下一轮必须验证 inspect 能否 transition 到 implementation。

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core natural_language_slash --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core inspect_missing_fact_source --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core inspect_requires_success_criteria_artifacts_when_fact_source_is_generic_directory --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_map::runtime::tests::inspect --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core no_action_recovery --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo check -p codex-core
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale
git diff --check
```

## 5.86 2026-07-04 targetless unified apply_patch fake target attribution

`b330f33` 后 rerun 证明 H-099 已越过 inspect coverage：TaskSpace 不再要求自然语言 `employees/projects`，
并进入 implementation/rework。新的 blocker 是 apply_patch 反馈目标被扭曲：

```text
RunDir: target/r4-org-json-real-keyed-20260704ct-slash-fact-source/runs/terminal_bench__organization-json-generator/20260704-220721-916
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 12
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
final_marker: TaskSpaceApplyPatchRecoveryHardStopV1
```

收益判断：

1. 新收录的 R4-D feedback 修复是 `apply-patch-targetless-unified-header-fake-target`：无目标 `---` / `+++` patch
   不再被归因为 `src/---`。
2. 这是“失败语义扭曲”而不是“失败语义缺失”：runtime 已经拒绝了 malformed patch，但 provider-visible failure
   给了不存在的目标，容易让下一轮围绕伪目标恢复。
3. R4-G utility 仍未通过：public validation 仍失败，下一轮需要验证该 case 是否 live-clear，并继续定位后续 apply_patch
   或 validation rework blocker。

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core targetless_unified_headers --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core bare_file_patch_normalizer_does_not_treat_unified_separator_as_path --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core apply_patch_ --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo check -p codex-core
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale
git diff --check
```

## 5.87 2026-07-04 separator update section apply_patch normalization

`60d5257` 后 rerun live-clear 了 H-100：`src/---` 伪目标不再出现。TaskSpace 进入 schema validation rework，
但最终停在 separator-style patch intent：

```text
RunDir: target/r4-org-json-real-keyed-20260704cu-targetless-header/runs/terminal_bench__organization-json-generator/20260704-222503-663
reported_evidence_level: E1
outcome_standard: wrong
outcome_taskspace: engineering_unclean
right_exec_timed_out: False
right_tool_call_count: 15
right_public_validation_exit_code: 1
right_hidden_oracle_exit_code: 0
final_marker: TaskSpaceApplyPatchRecoveryHardStopV1
```

收益判断：

1. 新收录的 R4-D ability/feedback 修复是 `apply-patch-separator-update-section-normalization-gap`：
   `<old block>` / `---` / `<new block>` 现在能转换成 native apply_patch hunk。
2. 该 case 是能力层和反馈层交界问题：模型已经表达了具体编辑意图，但 runtime 只把它当 malformed intent/recovery 处理。
3. 这不是放宽自然语言输出通道；只容忍 apply_patch JSON 后单个尾随 `"`，其他非严格 suffix 仍拒绝。
4. R4-G utility 仍未通过：需要下一次 keyed rerun 验证该 patch shape 是否实际应用，或者定位后续 validation rework failure。

验证：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core separator_update_sections --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core trailing_quote --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core apply_patch_ --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo fmt --check
CODEX_SKIP_VENDORED_BWRAP=1 cargo check -p codex-core
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale
git diff --check
```
