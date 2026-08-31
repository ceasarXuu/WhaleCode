# Problem P-001: 缓存真实回归在 Provider 前被二进制证明门禁拒绝
- Status: open
- Created: 2026-09-01 07:27
- Updated: 2026-09-01 07:40
- Objective: 让 Codex 0.151 缓存回归使用与当前提交一致的 workspace `whale` 二进制，并在零 Provider 请求的前提下先恢复 attestation preflight。
- Symptoms:
  - 专用 runner 返回 `Whale binary preflight failed before provider route: whale_binary_attestation_invalid`。
- Expected behavior:
  - workspace 二进制证明与当前 HEAD、Codex source commit 和二进制哈希一致，允许进入 Provider route。
- Actual behavior:
  - workspace 二进制存在，但证明仍绑定 2026-08-21 的 Codex 0.149 提交。
- Impact:
  - Codex 0.151 最小双臂真实缓存回归尚未发出 Provider 请求。
- Reproduction:
  - 在 HEAD `6014c8309f6076bb4c74c912235dfdecabd1ce21` 运行已授权的 `run_cache_hit_regression.py`。
- Environment:
  - Linux；branch `whalecode-codex`；workspace `whalecode-codex-2de2f3853e`。
- Known facts:
  - E-001 显示 preflight 在 Provider route 前失败，binary mtime 为 2026-08-21。
  - E-002 显示安装证明明确绑定旧提交 `130a562c...` 与 `whale 0.149.0`。
  - E-003 显示 0.151 cut-over 删除了 `debug provider`，E-004 已验证恢复后的命令测试通过。
  - E-005 显示首个已 claim 的 sample 在容器/Provider 启动前因隐藏 TaskSpace 开关缺失而停止，attempted_pairs=0。
  - E-006 已验证隐藏开关可解析但不出现在普通 `exec --help` 中。
- Ruled out:
  - H-002：不是当前 HEAD 解析或校验器误读，两个独立证据中的当前 HEAD 与旧证明字段均一致可解释失败。
- Fix criteria:
  - 重新安装后 attestation 绑定当前 HEAD/Codex source commit；同一 preflight 不再报告 `whale_binary_attestation_invalid`；Provider 请求数在修复验证前保持 0。
- Current conclusion: H-001、H-003、H-004 的本地 harness 回归均已修复并通过定向验证；真实缓存双臂验证仍需新的、不可复用的预算授权。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: workspace 二进制未随 Codex 0.151 源码刷新
- Status: confirmed
- Parent: P-001
- Claim: workspace `whale` 和 build attestation 仍来自 2026-08-21 的 Codex 0.149 提交，因此严格 preflight 正确拒绝当前 0.151 HEAD。
- Layer: environment
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 报错同时包含 `codex_source_commit_mismatch` 与 `git_build_identity_mismatch`。
- Falsifiable predictions:
  - If true: binary health 和 build attestation 都应显示旧提交、旧版本或早于当前 Codex source commit 的构建时间。
  - If false: attestation 应绑定当前 HEAD，且哈希/提交均一致，失败来自其他校验分支。
- Diagnostic evidence plan:
  - Prediction or clause under test: 旧 workspace 安装证明与当前 HEAD 不一致。
  - Signal: runner health JSON 与安装目录中的 build attestation。
  - Capture method: 只读检查两份 JSON。
  - Event name or marker:
    - whale_binary_attestation_invalid
  - Correlation keys:
    - WAR-20260901-072657-CACHE-REGRESSION-3ABEE540
  - Differentiates from:
    - H-002
  - Supports if:
    - 两份证据均显示证明提交为 `130a562c...`，当前提交为 `6014c830...`。
  - Refutes if:
    - 证明已经绑定当前提交且哈希匹配。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 按既有 `install-whale-local.sh --scope workspace` 合同重建并安装，然后只运行零成本 attestation preflight 验证。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: runner 错误解析了当前提交或误判了有效证明
- Status: refuted
- Parent: P-001
- Claim: 二进制实际是当前源码构建，但 runner 的提交或 attestation 比较逻辑产生误报。
- Layer: diagnostic
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 严格校验器本身也可能发生回归，需要用原始证明区别。
- Falsifiable predictions:
  - If true: 原始 build attestation 应显示当前提交/当前版本，或 health artifact 的 current HEAD 与 Git 不一致。
  - If false: 原始证明应明确显示旧提交和旧版本，health artifact 的 current HEAD 与 Git 一致。
- Diagnostic evidence plan:
  - Prediction or clause under test: 原始证明是否与 runner 的 mismatch 判断矛盾。
  - Signal: 原始 build attestation 字段与 health artifact 字段。
  - Capture method: 只读字段比较。
  - Event name or marker:
    - git_build_identity_mismatch
  - Correlation keys:
    - WAR-20260901-072657-CACHE-REGRESSION-3ABEE540
  - Differentiates from:
    - H-001
  - Supports if:
    - 原始证明绑定当前提交但仍被拒绝。
  - Refutes if:
    - 原始证明明确绑定旧提交和 `whale 0.149.0`。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: refuted
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: closed
- Blocker:
  - none
- Close reason:
  - 原始证明与 runner 判断一致。

## Evidence E-001: runner 生成的二进制健康证据
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `benchmarks/cache-regression/evidence/WAR-20260901-072657-CACHE-REGRESSION-3ABEE540/provider-route-preflight/whale-binary-health.json`
- Prediction or plan link:
  - H-001 的旧提交/旧时间预测；H-002 的 current HEAD 一致性预测。
- Matched signal:
  - `codex_source_commit_mismatch,git_build_identity_mismatch`
- Correlation keys:
  - WAR-20260901-072657-CACHE-REGRESSION-3ABEE540
- Raw content:
  ```text
  current_git_head=6014c8309f6076bb4c74c912235dfdecabd1ce21
  whale_binary_last_write_utc=2026-08-21T00:08:08.9863695Z
  stale_for_codex_source=true
  build_attestation_status=invalid
  build_attestation_reason=codex_source_commit_mismatch,git_build_identity_mismatch
  ```
- Interpretation: 当前源码晚于 workspace 二进制，且严格证明比较直接识别出两个提交维度不匹配；失败发生在 Provider route 前。
- Time: 2026-09-01 07:27

## Evidence E-002: workspace 原始 build attestation 绑定旧提交
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: config
- Source: `/home/zhangxu/.local/share/whalecode/workspaces/whalecode-codex-2de2f3853e/bin/whale.build-attestation.json`
- Prediction or plan link:
  - H-001 与 H-002 对原始证明提交和版本的相反预测。
- Matched signal:
  - 旧提交和旧版本均直接存在于证明。
- Correlation keys:
  - workspace whalecode-codex-2de2f3853e
- Raw content:
  ```text
  codex_source_latest_commit=130a562ce54a631966291e8026d23bffdd6b8769
  current_git_head=130a562ce54a631966291e8026d23bffdd6b8769
  executable_probe.output=whale 0.149.0
  generated_at=2026-08-21T00:08:12.978088+00:00
  ```
- Interpretation: 证明并非当前构建，runner 没有误读；既有 workspace 安装未在本轮源码合入后刷新。
- Time: 2026-09-01 07:28

## Hypothesis H-003: 0.151 cut-over 删除了 provider route 证明入口
- Status: confirmed
- Parent: P-001
- Claim: 提交 `6c2203c5c0` 删除了 Whale 自有隐藏命令 `debug provider`，导致离线 provider-route preflight 退出 2。
- Layer: regression-window
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 更新二进制证明后，preflight 的 stderr 明确报告 unknown subcommand。
- Falsifiable predictions:
  - If true: cut-over diff 应删除该 enum arm、dispatch 和实现，恢复后定向测试通过。
  - If false: 命令仍存在，失败来自 provider 配置或网络。
- Diagnostic evidence plan:
  - Prediction or clause under test: cut-over diff 与 CLI help 是否同时证明入口缺失。
  - Signal: Git diff、stderr、定向测试。
  - Capture method: 只读历史检查并运行 `debug_provider` 测试。
  - Event name or marker:
    - provider_route_cli_failed
  - Correlation keys:
    - WAR-20260901-072905-CACHE-REGRESSION-3C0F40DF
  - Differentiates from:
    - provider 配置或网络失败
  - Supports if:
    - diff 明确删除命令且恢复后测试通过。
  - Refutes if:
    - 命令未删除或恢复后仍以相同原因失败。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: confirmed and fixed
- Repair design readiness: ready
- Next step: closed
- Blocker:
  - none
- Close reason:
  - 既有隐藏证明入口已恢复，定向测试 1/1 通过，后续 preflight 通过。

## Hypothesis H-004: benchmark 与已移除的公开 TaskSpace 参数合同不一致
- Status: confirmed
- Parent: P-001
- Claim: 0.151 产品 CLI 正确隐藏 TaskSpace 开关后，开发 benchmark 仍要求它出现在帮助文本中，导致 sample 在 attempted_pairs=0 时退出。
- Layer: interaction
- Factor relation: single
- Depends on:
  - H-003
- Rationale:
  - 抛错发生在 benchmark 的帮助文本检查，早于容器和 Provider boundary 创建。
- Falsifiable predictions:
  - If true: run events 应只有 preflight，attempted_pairs=0；隐藏开关恢复后可解析且普通帮助仍不显示。
  - If false: 应存在 agent/container/provider 请求证据，或普通帮助必须暴露该参数才能工作。
- Diagnostic evidence plan:
  - Prediction or clause under test: sample 是否在容器/Provider 前停止，以及隐藏参数是否能在不公开的条件下工作。
  - Signal: run events/status、CLI parse test、help probes。
  - Capture method: 只读失败 artifact；修复后运行零网络 help probe。
  - Event name or marker:
    - attempted_pairs=0
  - Correlation keys:
    - WAR-20260901-073444-CACHE-REGRESSION-3BF5A4B3
  - Differentiates from:
    - Provider 调用失败或 TaskSpace 产品行为变更
  - Supports if:
    - 无 pair/provider 事件且隐藏参数可解析、普通 help 不显示。
  - Refutes if:
    - 已发生 Provider 请求或必须公开参数。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-005
  - E-006
- Conclusion: confirmed and fixed
- Repair design readiness: ready
- Next step: 等待新的真实运行授权完成最终 fix-validation。
- Blocker:
  - 原 R2 授权已 claim 且零重试，不能复用。
- Close reason:
  - not closed

## Evidence E-003: cut-over 删除 debug provider
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `git show 6c2203c5c0 -- third_party/codex-cli/codex-rs/cli/src/main.rs`
- Prediction or plan link:
  - H-003 的 regression-window 预测。
- Matched signal:
  - `DebugSubcommand::Provider`、dispatch 与 `run_debug_provider_command` 被同一提交删除。
- Correlation keys:
  - 6c2203c5c0
- Raw content:
  ```text
  error: unrecognized subcommand 'provider'
  ```
- Interpretation: preflight 失败由 Whale overlay 在 cut-over 中遗漏导致，不是外部 Provider 故障。
- Time: 2026-09-01 07:30

## Evidence E-004: provider route 入口修复验证
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `cargo test -p codex-cli --test debug_provider`
- Prediction or plan link:
  - H-003 恢复后定向测试通过的预测。
- Matched signal:
  - 1 passed; 0 failed；R2 provider-route preflight status=passed。
- Correlation keys:
  - e77a41ebb5
- Raw content:
  ```text
  test debug_provider_loads_custom_alias_without_exposing_secret ... ok
  provider_route_attestation.status=passed
  ```
- Interpretation: 隐藏 provider 证明命令已恢复且不会泄露 provider secret。
- Time: 2026-09-01 07:35

## Evidence E-005: 已 claim sample 在 Provider 前停止
- Related hypotheses:
  - H-004
- Direction: supports
- Type: reproduction
- Source: `target/cache-hit-regression/WAR-20260901-073444-CACHE-REGRESSION-3BF5A4B3/.../events.jsonl` 与 `run-status.json`
- Prediction or plan link:
  - H-004 的 attempted_pairs=0 预测。
- Matched signal:
  - provider credential preflight 通过后立即因 help 检查抛错；attempted_pairs=0，事件中无 container/provider boundary。
- Correlation keys:
  - WAR-20260901-073444-CACHE-REGRESSION-3BF5A4B3
- Raw content:
  ```text
  Whale exec does not expose --taskspace.
  attempted_pairs=0
  completed_pairs=0
  ```
- Interpretation: 此次授权已 claim，但实际 Agent/Provider 工作未开始；runner 因缺少 accounting artifact 保守结算为 unavailable。
- Time: 2026-09-01 07:35

## Evidence E-006: 隐藏 TaskSpace benchmark 开关修复验证
- Related hypotheses:
  - H-004
- Direction: supports
- Type: fix-validation
- Source: exec CLI unit test、container benchmark runner test、workspace 安装后的 help probe。
- Prediction or plan link:
  - H-004 的“可解析但不公开”预测。
- Matched signal:
  - parse test 1/1 通过；benchmark runner tests passed；`exec --taskspace --help` exit 0，普通 `exec --help` 不含 `--taskspace`。
- Correlation keys:
  - 4fc2e56375
- Raw content:
  ```text
  test cli::tests::parses_hidden_taskspace_exec_flag ... ok
  container benchmark runner tests passed
  workspace doctor status=passed
  ```
- Interpretation: 开发 harness 可恢复原实验控制，同时没有把该开关重新暴露为用户产品选项。
- Time: 2026-09-01 07:40
