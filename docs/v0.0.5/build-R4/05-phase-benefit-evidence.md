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
  R4-E/R4-F/R4-G/R4-H remain open until projection/output-ref, non-direct tools, public-10, and closeout gates finish.
```

## 5.2 PhaseBenefitEvidenceV1

| Phase | Claimed Engineering Benefit | Baseline Artifact | After Artifact | Measurement Method | Metric | Baseline Value | After Value | Pass Threshold | Pass/Fail | Evidence Paths |
|---|---|---|---|---|---|---|---|---|---|---|
| R4-A | tool path 覆盖从人工 Markdown 变成机器可读 manifest 和 gate，新增或遗漏 path 可被门禁发现 | `docs/v0.0.5/build-R4/01-static-tool-chain-map.md` | `docs/v0.0.5/build-R4/r4-tool-path-coverage.json` + validator output | `test-r4-tool-path-coverage.ps1` 校验 schema、source anchors、owner phase、required semantics | `unknown/unowned/missing-anchor` | 无可执行检查 | `path_count=9`, `failure_count=0` | `failure_count=0` | pass | `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-B | 历史 sample 现场从 scattered target/CoE 变成机器可读账本，known-bad 类型和 owner phase 可验证 | `docs/v0.0.5/build-R4/02-field-evidence-and-sample-ledger.md` | `docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json` + validator output | `test-r4-sample-ledger.ps1` 校验 sample id、failure class、owner phase、evidence path、required classes | `sample_count/missing-evidence` | 无可执行检查 | `sample_count=7`, `failure_count=0` | `sample_count>=6`, `failure_count=0` | pass | `target/r4-sample-ledger/r4-sample-ledger-evidence.json` |
| R4-C | direct tool error 的 TaskSpace map preview 不再走独立摘要，而是从 standard failure response 的 model-visible item 派生 | `parallel.rs` success/error map preview 来源分叉；manifest `direct-tool-error-map-preview=needs-fix` | `failure_response_for_error` + `response_input_model_visible_preview`；manifest `direct-tool-error-map-preview=canonical` | focused Rust unit test + R4 coverage validator | `failure_response_preview` | error path 独立 `action_map_tool_error_preview` | focused tests pass；coverage path canonical | focused tests pass；coverage validator pass | pass | `cargo test -p codex-core failure_response_preview --lib`; `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-D | action-contract internal failed tool outputs、validation gate failure、unreviewed-result blocker、dependency read evidence 都能以可执行语义进入下一轮，而不是被压成无法行动的 raw stderr 或 generic recovery | `count-call-stack` 历史 run：TaskSpace wrong/no patch，后续多轮 timeout；apply_patch 失败、validator 覆盖失败、unreviewed result、dependency read evidence 丢失分别导致卡住 | `third_party/codex-cli/codex-rs/core/src/session/turn.rs`、`action_map/runtime.rs`、`tools/parallel.rs` 修复；真实 rerun solved | focused Rust tests + paired public sample rerun | `outcome_taskspace`, `tool_call_count`, `wall_time_ratio`, `changed_paths`, `public_validation_exit_code` | wrong/no patch 或 900s timeout；`changed_paths` 空；validator/projection 链路卡死 | solved；`changed_paths=src/call_stack_counter.py`；`public_validation_exit_code=0`；tool calls 11 vs standard 20；wall ratio 1.12 | known feedback-loss sample 不再 wrong/no_patch；validation exit 0；无 evidence gate failure | pass for this P0 path | `target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136/pair-001/pair-report.md` |
| R4-E | projection/output-ref 不丢语义且不制造日志膨胀 | `large-output-ref-smoke` timeout / rollout bloat | pending | large-output rerun + projection audit | `timeout`, `rollout_size_bytes`, `projection_reason` | timeout / large rollout | pending | no timeout；failure details retrievable；rollout bounded | open | pending |
| R4-F | CodeMode/multi-agent/MCP 等 non-direct tools 有明确 inclusion/exclusion 和 provider-visible feedback 证明 | R4-A manifest 标记 non-direct paths `needs-fix` | pending | coverage fixtures + exclusion proof | `classified_path_count`, `missing_feedback_count` | blind spots | pending | all non-direct paths classified or intentionally excluded with tests | open | pending |
| R4-G | known-bad 和 10 个公开 benchmark 样本证明收益真实，而不是只靠局部单测 | R3/R4 scattered run evidence | pending | paired standard/taskspace 10 public samples + per-sample tool analysis | `tool_feedback_loss_count`, `wall/token/tool ratio`, `cache_hit` | pending | pending | public-10 report complete；feedback loss 0；cache hit >= 0.95 or explained | open | `docs/v0.0.5/build-R4/04-benefit-gates-and-public-sample-acceptance.md` |
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
```

## 5.5 不能据此关闭的内容

这些证据关闭 R4-A、R4-B、R4-C direct tool preview parity，以及 R4-D 的 `count-call-stack` P0 internal tool feedback 链路；不能关闭：

1. R4-E projection/output-ref/log bloat 的大输出真实收益证明。
2. R4-F CodeMode、multi-agent、MCP 等 non-direct tool runtime coverage。
3. R4-G known-bad 全量回归和公开 10 样本综合验收。
4. R4-H 工程层完整收口。
