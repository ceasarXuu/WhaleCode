# Problem P-001: 缓存 smoke 丢失显式 RunId 对应的运行证据
- Status: fixed
- Created: 2026-08-01 13:11
- Updated: 2026-08-01 13:17
- Objective: 让外部授权记录、benchmark 目录、容器标签和证据结算始终使用同一个显式 RunId。
- Symptoms:
  - benchmark 已写出完整失败 artifact，但外层 runner 报告 RunId 解析到 0 个目录，账本最初无法结算请求数。
- Expected behavior:
  - 新运行传入 `-RunId X` 时，实际目录叶子必须为 `X`；已存在时按 resume/force 规则处理。
- Actual behavior:
  - 脚本先计算显式目录，发现目录不存在后又调用时间戳目录创建函数，覆盖了显式目录。
- Impact:
  - 授权身份与 artifact 身份断裂；即使边界证据完整，自动账本仍降级为 unavailable。
- Reproduction:
  - 对一个不存在的 RunId 调用 benchmark；比较参数 RunId 与输出目录叶子。
- Environment:
  - Linux PowerShell benchmark；subject commit `0490facf13bbef0cc3f75909bccdc9f8271b63be`。
- Known facts:
  - 请求 RunId 为 `WAR-20260801-130559-CACHE-REGRESSION-DDFF3293-CACHE-001`。
  - 实际目录叶子为 `20260801-130559-657`。
  - `find_run_dir_by_id` 只接受 `*/<RunId>` 精确目录，因此返回 0。
- Ruled out:
  - artifact 没有生成；实际目录中存在 run status、pair report、stderr 和 provider boundary evidence。
- Fix criteria:
  - 新显式 RunId 创建精确同名目录，并保留 timestamp 默认路径。
  - 非法 RunId、已存在活动目录、resume 和 force 语义均有测试。
  - cache runner 能从显式目录读取边界证据并结算精确请求数。
- Current conclusion: 显式 RunId 现在是唯一目录叶子，非法或重复 ID 被明确拒绝；默认时间戳、E3 guardrails 和 cache regression 回归全部通过。
- Related hypotheses:
  - H-001
- Resolution basis:
  - PowerShell harness 与 E3 guardrails 通过；cache regression Python 196/196 通过。
- Close reason:
  - 根因修复已由本地目录合同和完整相关回归验证。

## Hypothesis H-001: 非 resume 分支覆盖显式 RunId
- Status: confirmed
- Parent: P-001
- Claim: 当显式 RunId 对应目录尚不存在时，`$resuming=false`，随后 `New-TaskspaceBenchmarkRun` 把 `$runDir` 改为时间戳目录。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 控制流对“新显式 ID”和“未指定 ID”没有分支，实际目录正是创建器生成的时间戳格式。
- Falsifiable predictions:
  - If true: 参数 ID 与目录叶子不同，目录叶子匹配时间戳，精确 finder 返回 0。
  - If false: 实际目录叶子应等于参数 ID，finder 应解析到 1 个目录。
- Diagnostic evidence plan:
  - Prediction or clause under test: 新显式 ID 被时间戳创建器覆盖。
  - Signal: runner 控制流、argv RunId、实际目录和 finder 错误。
  - Capture method: 静态追踪与已授权运行 artifact 对照。
  - Event name or marker:
    - `evidence_error`
  - Correlation keys:
    - `WAR-20260801-130559-CACHE-REGRESSION-DDFF3293-CACHE-001`
  - Differentiates from:
    - artifact 写入失败、路径权限失败和清理误删。
  - Supports if:
    - 新运行调用时间戳创建器且实际存在时间戳目录。
  - Refutes if:
    - 显式 ID 目录实际存在，或 finder 指向错误根目录。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 RunId、实际 run root 和证据路径到结果/账本。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 执行 PowerShell harness 与 cache regression 测试，确认新 ID、默认时间戳和 resume 路径没有回归。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 控制流覆盖显式目录
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:80-88`
- Prediction or plan link:
  - H-001 新显式 ID 被时间戳创建器覆盖预测。
- Matched signal:
  - `$runDir` 先由 `$RunId` 赋值，随后在 `-not $resuming` 分支无条件重新赋值。
- Correlation keys:
  - none
- Raw content:
  ```text
  $runDir = Join-Path (Join-Path $RunRoot $manifest.Id) $RunId
  ...
  if (-not $resuming) {
      $runDir = New-TaskspaceBenchmarkRun $RunRoot $manifest.Id
  }
  ```
- Interpretation: 代码路径充分解释了显式 ID 与实际目录不一致。
- Time: 2026-08-01 13:11

## Evidence E-002: 实际目录与授权 RunId 不同
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `benchmarks/cache-regression/results/WAR-20260801-130559-CACHE-REGRESSION-DDFF3293.json` 与 `target/cache-hit-regression/WAR-20260801-130559-CACHE-REGRESSION-DDFF3293/single-file-fast-fix/20260801-130559-657`
- Prediction or plan link:
  - H-001 目录叶子差异预测。
- Matched signal:
  - requested `...-CACHE-001`；actual `20260801-130559-657`；finder resolved 0 directories。
- Correlation keys:
  - `WAR-20260801-130559-CACHE-REGRESSION-DDFF3293-CACHE-001`
- Raw content:
  ```text
  RuntimeError: benchmark run id WAR-20260801-130559-CACHE-REGRESSION-DDFF3293-CACHE-001 resolved to 0 directories
  actual directory: single-file-fast-fix/20260801-130559-657
  ```
- Interpretation: artifact 存在但身份链断裂，排除了“未写出证据”。
- Time: 2026-08-01 13:06

## Hypothesis H-002: 显式目录合同修复可消除身份断裂
- Status: confirmed
- Parent: P-001
- Claim: 仅在未传 RunId 时生成时间戳；传入 RunId 时校验并创建同名目录，能够让授权、容器标签和证据 finder 使用同一身份。
- Layer: fix-validation
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - finder 已要求精确目录名，修复创建端比增加模糊查找更能保持身份唯一性。
- Falsifiable predictions:
  - If true: 显式 ID 目录叶子完全相同，重复和路径型 ID 被拒绝，默认运行仍生成时间戳。
  - If false: 测试仍观察到目录叶子变化、身份碰撞或默认路径回归。
- Diagnostic evidence plan:
  - Prediction or clause under test: 显式和默认目录合同均保持确定。
  - Signal: PowerShell harness、Python cache contract tests、实际目录断言。
  - Capture method: 运行不调用模型的本地测试。
  - Event name or marker:
    - `benchmark_run_id_invalid`
    - `benchmark_run_id_already_exists`
  - Correlation keys:
    - none
  - Differentiates from:
    - finder fallback 或运行后扫描修补。
  - Supports if:
    - 所有目录合同测试通过。
  - Refutes if:
    - 任一身份、重复或默认命名断言失败。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留明确错误码和测试。
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 真实 cache smoke 获得新预算后，验证外层 runner 自动读取同名目录；该步骤不阻塞本地根因关闭。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-003: 显式 RunId 目录合同实现
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `scripts/taskspace-benchmark/lib/workspace.ps1`、`scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- Prediction or plan link:
  - H-002 显式 ID 保持与非法/重复 ID 拒绝预测。
- Matched signal:
  - 创建器接受可选 RunId，校验单一路径段，拒绝已存在目录；runner 将 RunId 原样传入。
- Correlation keys:
  - none
- Raw content:
  ```text
  New-TaskspaceBenchmarkRun $RunRoot $manifest.Id $RunId
  ```
- Interpretation: 修复位于身份创建源头，没有引入 finder fallback。
- Time: 2026-08-01 13:14

## Evidence E-004: RunId 相关回归全部通过
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: `scripts/taskspace-benchmark/test-harness.ps1`、`scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1`、`scripts/cache-regression/test_*.py`
- Prediction or plan link:
  - H-002 目录身份、默认命名和相关调用链均不回归的预测。
- Matched signal:
  - harness PASS；E3 guardrails PASS；cache regression 196 tests OK。
- Correlation keys:
  - none
- Raw content:
  ```text
  TaskSpace benchmark harness self-test: PASS
  E3 harness guardrails self-test: PASS
  Ran 196 tests in 6.683s
  OK
  ```
- Interpretation: 显式 ID 修复没有破坏默认运行、E3 约束或缓存门禁调用合同。
- Time: 2026-08-01 13:17
