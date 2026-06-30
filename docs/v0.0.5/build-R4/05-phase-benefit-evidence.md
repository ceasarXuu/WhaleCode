# R4 Phase 收益证据账本

> 本文只记录已经由当前工程产物证明的 phase benefit。没有证据的 phase 不在本文标记完成。

## 5.1 当前状态

```text
Updated: 2026-06-30
HEAD at evidence capture: 01d96a391
Status: R4-A/R4-B governance gates landed and validated; R4-C direct tool error parity landed and validated; R4-D internal tool feedback structuring landed locally and validated; remaining runtime repair phases remain open.
```

## 5.2 PhaseBenefitEvidenceV1

| Phase | Claimed Engineering Benefit | Baseline Artifact | After Artifact | Measurement Method | Metric | Baseline Value | After Value | Pass Threshold | Pass/Fail | Evidence Paths |
|---|---|---|---|---|---|---|---|---|---|---|
| R4-A | tool path 覆盖从人工 Markdown 变成机器可读 manifest 和 gate，新增/遗漏 path 可被门禁发现 | `docs/v0.0.5/build-R4/01-static-tool-chain-map.md` | `docs/v0.0.5/build-R4/r4-tool-path-coverage.json` + validator output | `test-r4-tool-path-coverage.ps1` 校验 schema、source anchors、owner phase、required semantics | `unknown/unowned/missing-anchor` | 无可执行检查 | `path_count=9`, `failure_count=0` | `failure_count=0` | pass | `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-B | 历史 sample 现场从 scattered target/CoE 变成机器可读账本，known-bad 类型和 owner phase 可验证 | `docs/v0.0.5/build-R4/02-field-evidence-and-sample-ledger.md` | `docs/v0.0.5/build-R4/r4-sample-evidence-ledger.json` + validator output | `test-r4-sample-ledger.ps1` 校验 sample id、failure class、owner phase、evidence path、required classes | `sample_count/missing-evidence` | 无可执行检查 | `sample_count=6`, `failure_count=0` | `sample_count>=6`, `failure_count=0` | pass | `target/r4-sample-ledger/r4-sample-ledger-evidence.json` |
| R4-C partial | direct tool error 的 TaskSpace map preview 不再走独立摘要，而是从 standard failure response 的 model-visible item 派生 | `parallel.rs` success/error map preview 来源分叉；manifest `direct-tool-error-map-preview=needs-fix` | `failure_response_for_error` + `response_input_model_visible_preview`；manifest `direct-tool-error-map-preview=canonical` | focused Rust unit test + R4 coverage validator | `failure_response_preview` | error path 独立 `action_map_tool_error_preview` | `3 passed`; coverage path canonical | focused tests pass；coverage validator pass | pass | `cargo test -p codex-core failure_response_preview --lib`; `target/r4-tool-path-coverage/r4-tool-path-coverage-evidence.json` |
| R4-D partial | action-contract internal failed tool outputs 在下一轮 action-contract prompt 中变成结构化 `TaskSpaceToolFeedbackV1`，不再只是未分类 raw stderr | internal tool failure 只以 raw `FunctionCallOutput`/`CustomToolCallOutput` summary 进入 recent tool outputs | `taskspace_action_contract_tool_feedback_summary` 为 failed `taskspace-action-contract-*` 输出添加 tool_source/action/result/failure_kind/next_valid_action/raw_output | focused Rust unit tests | `action_contract_prompt_structures_*` | no structured marker | 2 focused tests passed | focused tests pass；真实 `count-call-stack` rerun 待执行 | pass for unit gate / pending for sample benefit | `cargo test -p codex-core action_contract_prompt_structures_internal_apply_patch_missing_target_feedback --lib`; `cargo test -p codex-core action_contract_prompt_structures_generic_internal_tool_failure --lib` |

## 5.3 已执行命令

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1
  PASS: R4 tool path coverage gate passed: 9 paths

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-sample-ledger.ps1
  PASS: R4 sample ledger gate passed: 6 samples

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-v005-non-agent-gates-builder.ps1
  PASS: v005 non-agent gates builder selftest passed

cargo test -p codex-core failure_response_preview --lib
  PASS: 3 passed

cargo test -p codex-core action_contract_prompt_structures_internal_apply_patch_missing_target_feedback --lib
  PASS: 1 passed

cargo test -p codex-core action_contract_prompt_structures_generic_internal_tool_failure --lib
  PASS: 1 passed
```

## 5.4 不能据此关闭的内容

这些证据关闭 R4-A/R4-B 的治理和证据账本缺口，关闭 R4-C 的 direct tool error
map preview 分叉子问题，并为 R4-D internal tool feedback 建立了 focused unit proof；不能关闭：

1. R4-C canonical tool feedback contract 的全部路径。
2. R4-D action-contract internal tool parity 的真实样本收益证明。
3. R4-E projection/output-ref/log bloat 修复。
4. R4-F non-direct tool runtime coverage。
5. R4-G known-bad 和公开 10 样本综合验收。
6. R4-H 工程层完整收口。
