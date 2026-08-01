# Problem P-001: 缓存 smoke 丢失显式 RunId 对应的运行证据
- Status: open
- Created: 2026-08-01 13:11
- Updated: 2026-08-01 13:11
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
- Current conclusion: 新运行分支无条件调用时间戳创建器，覆盖显式 RunId；根因已由控制流和实际目录差异确认。
- Related hypotheses:
  - H-001
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

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
- Next step: 把“选择新目录名”和“初始化目录”拆开，显式 RunId 只使用经过校验的同名目录。
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
