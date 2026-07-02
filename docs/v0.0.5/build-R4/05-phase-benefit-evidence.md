# R4 Phase 收益证据账本

> 本文只记录已经由当前工程产物证明的 phase benefit。没有证据的 phase 不能标记完成。

## 5.1 当前状态

```text
Updated: 2026-07-02
Code state at evidence capture: R4 tool-chain convergence changes through public-10 closeout
Status:
  R4-A pass: tool path coverage manifest and gate are executable; canonical paths now require coverage_test.
  R4-B pass: known sample evidence ledger and gate are executable.
  R4-C pass for direct tool success/error map-preview parity.
  R4-D pass for internal feedback, validation closeout drain, and closed-validation contract focused gates.
  R4-E pass for large raw output ref and pair-safe provider projection focused gates.
  R4-F pass for CodeMode, multi-agent, and MCP non-direct tool path classification gates.
  R4-G closed for benchmark/report engineering gate; utility evidence is negative and blocks E3.
  R4-H closed with engineering closeout and E3 no-go decision.
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
