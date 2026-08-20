# Problem P-001: Standard final-wire 基线包含每次运行变化的临时身份
- Status: fixed
- Created: 2026-08-21 05:55
- Updated: 2026-08-21 06:02
- Objective: 让缓存 final-wire 免费门禁只比较 Provider 语义，不因 session/turn/message 临时身份产生永久漂移。
- Symptoms:
  - 相同 HEAD 连续两次 gate 的 Standard candidate hash 不同，首个语义差异仍稳定指向 0.147 instructions。
- Expected behavior:
  - 同一代码与 fixture 的 normalized final-wire snapshot 可重复，临时身份保持相等/不同关系但不携带随机值。
- Actual behavior:
  - `client_metadata`、嵌套 `x-codex-turn-metadata` 和 message `id` 原样进入 snapshot。
- Impact:
  - 晋升任一 candidate 后下一次 gate 仍会失败，无法形成稳定 accepted baseline。
- Reproduction:
  - 比较 U20 与 U21 gate report 的 `standard_two_request_final_wire` candidate。
- Environment:
  - Linux；Codex 0.147 rebase；HEAD `c239e2c69`。
- Known facts:
  - 两次 instructions 字段完全相同。
  - 两次 candidate 仅发现 session/thread/turn/window/message/installation/timestamp 身份变化。
- Ruled out:
  - 0.147 instructions 自身不稳定：两次字段 SHA256 均为 `38e0b9de817f645c4bec37c0d4a3e58baecccb040f5718dc069a72c7385a0bed`。
- Fix criteria:
  - 仅在测试 fixture normalization 层稳定临时身份，不忽略或改写产品请求字段。
  - 保留重复 message id 的相等关系和不同 message id 的区分。
  - 同一 HEAD 连续两次 final-wire candidate 完全一致。
- Current conclusion: H-001 confirmed；fixture-only stabilizer 已补齐并证明连续 candidate 完全一致。
- Related hypotheses:
  - H-001
- Resolution basis:
  - H-001 confirmed by E-001，repaired and validated by E-002/E-003。
- Close reason:
  - 连续两次 snapshot candidate SHA256 均为 `76cfe1d0edd58e0016faeb93b270ad02ac6e35b52ea6eb7fd1f6e37f7156c985`。

## Hypothesis H-001: fixture stabilizer 漏掉 0.147 临时身份字段
- Status: confirmed
- Parent: P-001
- Claim: snapshot 抖动来自测试规范化缺口，而非 Provider wire 语义变化。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 两次 candidate 的文本差异只包含可枚举的运行期身份。
- Falsifiable predictions:
  - If true: 稳定这些字段后重复 gate candidate 相等。
  - If false: 仍有非临时字段随运行变化。
- Diagnostic evidence plan:
  - Prediction or clause under test: 连续 candidate 的精确 JSON 差异。
  - Signal: field-level diff 与 instructions 字段 hash。
  - Capture method: jq canonical extraction、sha256sum、diff。
  - Event name or marker:
    - `standard_two_request_final_wire`
  - Correlation keys:
    - U20/U21 gate report
  - Differentiates from:
    - none
  - Supports if:
    - instructions 相同而所有 candidate 差异均是临时身份。
  - Refutes if:
    - instructions/tools/input 语义内容变化。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: 需要补齐 fixture-only identity stabilizer。
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - Fixed in cache fixture normalization and validated by E-002/E-003.

## Evidence E-001: 相同 HEAD 连续 gate 仅临时身份不同
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `2026-08-21-u20-taskspace-map-export-fix.json` 与 `2026-08-21-u21-final-baseline-qualification.json`
- Prediction or plan link:
  - P-001 重现标准。
- Matched signal:
  - instructions canonical SHA256 完全相同；diff 仅含 client metadata、嵌套 metadata 与 message id。
- Correlation keys:
  - instructions SHA256 `38e0b9de817f645c4bec37c0d4a3e58baecccb040f5718dc069a72c7385a0bed`
- Raw content:
  ```text
  instructions_equal=0 (cmp success)
  changed: session_id/thread_id/turn_id/window_id/message id/installation_id/timestamp
  ```
- Interpretation: 这是免费门禁 fixture 的确定性缺口，不是产品 wire 回归。
- Time: 2026-08-21 05:55

## Evidence E-002: identity-preserving fixture 单元测试通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: codex-core nextest run `eea33fe4-efbc-451c-aa57-b3578a74b6cd`
- Prediction or plan link:
  - P-001 保留身份关系的规范化标准。
- Matched signal:
  - 两组不同原始身份规范化为相同 JSON；重复 message id 映射到同一 placeholder，不同 message id 保持区分；嵌套 metadata 的语义字段保留。
- Correlation keys:
  - `fixture_stabilization_normalizes_ephemeral_wire_ids_without_collapsing_identity`
- Raw content:
  ```text
  3 tests run: 3 passed, 0 skipped
  ```
- Interpretation: 修复只作用于 fixture snapshot，不删除产品字段或放宽 exact wire 比较。
- Time: 2026-08-21 05:58

## Evidence E-003: 连续真实 snapshot candidate 完全一致
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: 同一 staged implementation 连续两次 `standard_request_pair_preserves_the_complete_prefix`
- Prediction or plan link:
  - P-001 重复性标准。
- Matched signal:
  - 两次 `.snap.new` 文件 SHA256 完全相同；测试只因尚未接受的 0.147 语义基线差异按预期失败。
- Correlation keys:
  - candidate SHA256 `76cfe1d0edd58e0016faeb93b270ad02ac6e35b52ea6eb7fd1f6e37f7156c985`
- Raw content:
  ```text
  run A: 76cfe1d0edd58e0016faeb93b270ad02ac6e35b52ea6eb7fd1f6e37f7156c985
  run B: 76cfe1d0edd58e0016faeb93b270ad02ac6e35b52ea6eb7fd1f6e37f7156c985
  ```
- Interpretation: 临时身份抖动已消除，下一步可以安全资格运行并晋升稳定基线。
- Time: 2026-08-21 06:02
