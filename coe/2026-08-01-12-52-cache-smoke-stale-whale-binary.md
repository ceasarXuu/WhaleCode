# Problem P-001: R8 真实缓存 smoke 在 Whale 二进制预检阶段终止
- Status: open
- Created: 2026-08-01 12:52
- Updated: 2026-08-01 12:52
- Objective: 恢复与当前源码身份一致、可由 attestation 证明的本机 Whale 二进制，使获批缓存 smoke 能进入实际 pair 执行。
- Symptoms:
  - 获批批次 `WAR-20260801-125219-CACHE-REGRESSION-34ACB825` 的 Standard 首臂在 8.577 秒后退出 3。
  - harness 报告 `invalid_harness/whale_binary_attestation_invalid`，没有开始任何 pair。
- Expected behavior:
  - `~/.whale/bin/whale` 的 binary hash、Codex source commit 和 Git build identity 与当前干净源码一致，预检通过后才启动容器和 provider boundary。
- Actual behavior:
  - 已安装二进制仍绑定旧 HEAD `4c6f7a7ca` 和旧 Codex source commit `923e8c945`，当前预检要求 Codex source commit `0d3af4b54`。
- Impact:
  - 本次授权在零重试规则下被消费，map-request 未执行，不能建立 accepted baseline。
- Reproduction:
  - 使用提案 `CBP-60338F2F86A4B693` 和授权 `CBA-20260801-60338F2F86A4B693` 启动缓存回归；读取运行目录中的 `whale-binary-preflight-health.json`。
- Environment:
  - Linux，分支 `whalecode-alpha`，subject HEAD `860373cfa`，Whale `0.1.0`。
- Known facts:
  - 预检发生在 attempted pair、容器和 provider boundary 之前。
  - 账本忠实记录 `api_requests=null`、minimum `0`、evidence status `unavailable`，因此不能把费用正式结算为精确 0。
  - 容器、网络和 host secret 清理均为 `verified_absent`。
- Ruled out:
  - 不是 Agent、模型、sample 业务逻辑或 TaskSpace 执行失败，因为实际 pair 数为 0。
- Fix criteria:
  - 从当前干净源码重新构建并安装 Whale；生成 schema v2 attestation；`New-TaskspaceWhaleBinaryHealth` 返回 `status=pass` 和 `run_validity=valid`。
- Current conclusion: 根因已确认是本机 Whale 构建产物落后于当前 Codex 源码身份；先恢复构建证明，不修改 TaskSpace 或缓存门禁语义。
- Related hypotheses:
  - H-001
- Resolution basis:
  - pending fix-validation E-003
- Close reason:
  - not closed

## Hypothesis H-001: 已安装 Whale 的构建证明与当前源码身份不一致
- Status: confirmed
- Parent: P-001
- Claim: harness 预检因 binary attestation 绑定旧 Codex source commit 和旧 Git tree 而拒绝运行。
- Layer: environment
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - abort summary 和 binary health 都给出稳定错误码及具体 identity mismatch。
- Falsifiable predictions:
  - If true: health artifact 同时出现 `codex_source_commit_mismatch`、`git_build_identity_mismatch`，且 attempted pair 为 0。
  - If false: 当前 attestation 应匹配源码，失败应发生在容器、provider 或 Agent 阶段。
- Diagnostic evidence plan:
  - Prediction or clause under test: attestation 身份是否精确匹配当前源码。
  - Signal: binary health 的 attestation status/reason、source commit 和 attempted pair。
  - Capture method: 读取本次持久 health、run-status、sample-status 和 attestation 文件。
  - Event name or marker:
    - whale_binary_attestation_invalid
  - Correlation keys:
    - WAR-20260801-125219-CACHE-REGRESSION-34ACB825
  - Differentiates from:
    - provider 网络失败、模型失败、Agent 业务失败
  - Supports if:
    - attestation 报两项 identity mismatch，且没有 pair/provider artifact。
  - Refutes if:
    - attestation 为 pass，或失败发生在实际 pair 之后。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留现有 binary health 和稳定错误码。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed by persisted preflight identity evidence
- Repair design readiness: ready
- Next step: 从干净 HEAD 重建、安装并签发 attestation，然后执行离线 health 验证。
- Blocker:
  - 新的真实 smoke 必须另行取得授权，不能复用本次零重试授权。
- Close reason:
  - not closed

## Evidence E-001: binary health 精确报告两项身份不匹配
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target/cache-hit-regression/WAR-20260801-125219-CACHE-REGRESSION-34ACB825/single-file-fast-fix/20260801-125219-762/whale-binary-preflight-health.json`
- Prediction or plan link:
  - H-001 attestation identity prediction
- Matched signal:
  - `whale_binary_attestation_invalid`
- Correlation keys:
  - WAR-20260801-125219-CACHE-REGRESSION-34ACB825
- Raw content:
  ```text
  build_attestation_status: invalid
  build_attestation_reason: codex_source_commit_mismatch,git_build_identity_mismatch
  attested codex source: 923e8c94508aa9687c9db8ed1b295cc6374e7912
  required codex source: 0d3af4b54250f159c4d06ea837369bbb7dcab64e
  ```
- Interpretation: 预检失败由可复算的本地构建身份错配解释，不需要推测 Agent 行为。
- Time: 2026-08-01 12:52

## Evidence E-002: 运行在任何 pair 和 provider 证据前终止
- Related hypotheses:
  - H-001
- Direction: supports
- Type: runtime-state
- Source: 本次 `run-status.json`、全局运行账本和结果文件
- Prediction or plan link:
  - H-001 attempted pair prediction
- Matched signal:
  - attempted_pairs=0；actual_sample_runs=1；map-request 位于 unverified_scope。
- Correlation keys:
  - WAR-20260801-125219-CACHE-REGRESSION-34ACB825
- Raw content:
  ```text
  attempted_pairs: 0
  completed_pairs: 0
  provider_boundary_requests_minimum: 0
  provider_boundary_accounting_status: unavailable
  cleanup: verified_absent
  ```
- Interpretation: 没有 Agent 业务结果或缓存性能结果；账本保守保留未知精确请求数，不能伪造 0 费用结论。
- Time: 2026-08-01 12:52

## Evidence E-003: 重建后的 binary health
- Related hypotheses:
  - H-001
- Direction: neutral
- Type: fix-validation
- Source: pending
- Prediction or plan link:
  - P-001 fix criteria
- Matched signal:
  - pending
- Correlation keys:
  - current HEAD
- Raw content:
  ```text
  pending
  ```
- Interpretation: 等待干净源码重建和离线 health probe。
- Time: 2026-08-01 12:52
