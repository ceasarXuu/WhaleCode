# Problem P-001: 缓存 runner 在 provider-route 预检阶段使用过期 Whale binary
- Status: fixed
- Created: 2026-08-02 16:55
- Updated: 2026-08-02 17:17
- Objective: 让获批缓存回归在任何 provider 请求前使用与当前 subject HEAD 能力一致的 Whale binary。
- Symptoms:
  - `run_cache_hit_regression.py` 在 authorization claim 前报错：`provider_route_cli_failed: standard alias debug provider exited 2`。
- Expected behavior:
  - provider-route 预检使用支持当前 `whale debug provider` 合同且与当前源码一致的 binary；不一致时由 binary-health 明确拒绝。
- Actual behavior:
  - runner 默认选择的 `~/.whale/bin/whale` 不包含 `debug provider`，预检以通用 CLI 失败退出。
- Impact:
  - R8 Tool Sequence Phase A 的 MVT-0 真实缓存基线复验尚未启动；authorization 未认领，provider 请求为 0。
- Reproduction:
  - 使用 proposal `CBP-9187676F51FA999C` 和 authorization `CBA-20260802-SEQUENCE-9187676F51FA999C` 启动专用 runner。
- Environment:
  - Linux；分支 `whalecode-alpha`；subject HEAD `2390597f7`；默认 binary `~/.whale/bin/whale`。
- Known facts:
  - 当前源码包含 `whale debug provider`；默认安装 binary 与本地 target binary 均不包含该子命令。
  - 默认安装 binary 的 attestation 绑定旧 HEAD `a65ba90e3`，而命令是在后续提交 `d97aa819f` 引入。
  - provider-route 预检位于全局账本 claim 和 benchmark 内 binary-health 之前。
- Ruled out:
  - 不是 DeepSeek 配置解析失败；CLI 在进入配置解析前就拒绝了不存在的子命令。
  - 不是付费 provider 失败；authorization 尚未 claim，Whale Agent 和 provider 请求均未启动。
- Fix criteria:
  - 当前 HEAD 构建并安装的 binary 暴露 `debug provider`，attestation 绑定当前源码；同一未认领 authorization 的预检通过。
  - 后续单独修复 runner 前置校验缺口，使过期 binary 在 provider-route 前以稳定错误拒绝。
- Current conclusion: 直接阻塞由过期 binary 引起；当前 binary 已恢复，runner 也在 provider-route 前复用唯一 binary-health 合同，旧 binary 会以稳定 attestation 错误被拒绝。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-001、H-002；E-001 至 E-006
- Close reason:
  - 当前安装 binary 与 attestation 通过离线探针，前置顺序由测试锁定。

## Hypothesis H-001: runner 选择了缺少当前 CLI 能力的旧 binary
- Status: confirmed
- Parent: P-001
- Claim: 默认 Whale binary 构建于 `debug provider` 引入前，因此确定性地产生当前退出码 2。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 错误为 Clap 的 unknown subcommand，而不是配置或网络错误。
- Falsifiable predictions:
  - If true: 默认 binary 直接执行 `debug provider` 时由 Clap 拒绝，其 attestation 早于引入提交；当前源码能进入该命令的配置校验。
  - If false: 默认 binary 应能进入 `debug provider` 并产生 resolved-provider JSON 或配置错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较默认 binary 的 CLI surface、attestation 和当前源码命令定义。
  - Signal: 直接执行 `debug provider` 的退出语义、attestation HEAD、源码 Git 引入提交。
  - Capture method: 本地只读命令与 Git 历史查询。
  - Event name or marker:
    - `provider_route_cli_failed`
  - Correlation keys:
    - `WAR-20260802-164939-CACHE-REGRESSION-BF08BDAB`
  - Differentiates from:
    - provider alias 配置错误或网络错误。
  - Supports if:
    - 旧 binary 不含命令且早于源码引入提交。
  - Refutes if:
    - binary 含命令并已进入配置解析。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: closed
- Blocker:
  - none
- Close reason:
  - E-005、E-006 满足修复验收。

## Hypothesis H-002: provider-route 位于 binary-health 之前导致错误未被准确前置拒绝
- Status: confirmed
- Parent: P-001
- Claim: cache runner 的调用顺序先运行 provider-route，再由下层 benchmark 检查 binary attestation，导致旧 binary 以缺命令而非身份不匹配失败。
- Layer: sub-cause
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - 已有 binary-health 能识别当前源码身份，但错误没有来自该层。
- Falsifiable predictions:
  - If true: `run_cache_hit_regression.py` 在 `claim_entry` 和 benchmark command 前直接调用 provider-route，binary-health 只存在于后续 PowerShell benchmark。
  - If false: runner 应在 provider-route 前已经验证当前 binary attestation。
- Diagnostic evidence plan:
  - Prediction or clause under test: 静态追踪 runner 到 provider-route、claim 和 benchmark binary-health 的调用顺序。
  - Signal: 函数调用代码位置。
  - Capture method: 读取生产 runner 与 benchmark 脚本。
  - Event name or marker:
    - `provider route preflight failed before authorization claim`
  - Correlation keys:
    - `WAR-20260802-164939-CACHE-REGRESSION-BF08BDAB`
  - Differentiates from:
    - attestation 校验本身错误。
  - Supports if:
    - provider-route 明确先于 claim，binary-health 仅在 benchmark 内。
  - Refutes if:
    - 存在更早的 current-HEAD binary-health 调用。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 后续将能力/身份预检提升到 provider-route 前并增加回归测试。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: closed
- Blocker:
  - none
- Close reason:
  - E-006 证明 binary-health 先于 provider-route 和 authorization claim。

## Evidence E-001: provider-route 原始 CLI 失败
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `benchmarks/cache-regression/evidence/WAR-20260802-164939-CACHE-REGRESSION-BF08BDAB/provider-route-preflight/resolved-provider-standard.json.stderr.log`
- Prediction or plan link:
  - H-001 的 unknown subcommand 预测。
- Matched signal:
  - `error: unrecognized subcommand 'provider'`
- Correlation keys:
  - `WAR-20260802-164939-CACHE-REGRESSION-BF08BDAB`
- Raw content:
  ```text
  error: unrecognized subcommand 'provider'
  ```
- Interpretation: 失败发生在 CLI 参数解析阶段，尚未进入 provider 配置解析。
- Time: 2026-08-02 16:49

## Evidence E-002: 默认 binary 直接拒绝 provider 子命令
- Related hypotheses:
  - H-001
- Direction: supports
- Type: probe
- Source: `~/.whale/bin/whale debug provider`
- Prediction or plan link:
  - H-001 的 CLI surface 预测。
- Matched signal:
  - Clap 返回 `unrecognized subcommand 'provider'`；当前源码构建的 binary 对同一命令进入 HMAC key 配置校验。
- Correlation keys:
  - binary SHA-256 `ac4c1579277b1018db8fc97586cbb5115afe825a443ecaa881498edcd87ee4db`
- Raw content:
  ```text
  stale binary: error: unrecognized subcommand 'provider'
  current binary: Error: provider descriptor HMAC key file is required
  ```
- Interpretation: `provider` 在当前源码中是隐藏命令，不能用 `debug --help` 判断；直接调用结果证明默认 binary 不具备该命令，而当前 binary 已进入命令实现。
- Time: 2026-08-02 16:53

## Evidence E-003: binary 身份早于命令引入提交
- Related hypotheses:
  - H-001
- Direction: supports
- Type: config
- Source: `~/.whale/bin/whale.build-attestation.json` 与 Git log
- Prediction or plan link:
  - H-001 的版本窗口预测。
- Matched signal:
  - attestation HEAD 为 `a65ba90e3`；`debug provider` 引入提交为 `d97aa819f`。
- Correlation keys:
  - binary SHA-256 `ac4c1579277b1018db8fc97586cbb5115afe825a443ecaa881498edcd87ee4db`
- Raw content:
  ```text
  current_git_head: a65ba90e324ed72fd0f903c14d15d3d5b5e07847
  d97aa819f feat(cli): attest resolved provider behavior
  ```
- Interpretation: 回归窗口由源码与 binary 身份直接闭合。
- Time: 2026-08-02 16:53

## Evidence E-004: runner 校验顺序晚于 provider-route
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `scripts/cache-regression/run_cache_hit_regression.py` 与 `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- Prediction or plan link:
  - H-002 的调用顺序预测。
- Matched signal:
  - Python runner 先调用 `run_provider_route_preflight`，随后才 claim；binary-health 只在下游 benchmark 第 243 行执行。
- Correlation keys:
  - `WAR-20260802-164939-CACHE-REGRESSION-BF08BDAB`
- Raw content:
  ```text
  provider_route = run_provider_route_preflight(...)
  ...
  claim_entry(ledger_path, entry)
  ...
  $binaryHealth = New-TaskspaceWhaleBinaryHealth $WhaleBin $repoRoot
  ```
- Interpretation: 现有身份校验无法为 provider-route 预检提供前置保护。
- Time: 2026-08-02 16:54

## Evidence E-005: 当前安装 binary 通过共享 health 合同
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `scripts/taskspace-benchmark/check-whale-binary-health.ps1`
- Prediction or plan link:
  - H-001 的当前 binary 身份和能力验收。
- Matched signal:
  - `status=pass`、`build_attestation_status=pass`、无 finding。
- Correlation keys:
  - binary SHA-256 `45d37c43cef498e2ed075856ee6f2d631f85c35efedf7f05d04fec9165625aa0`
- Raw content:
  ```text
  status=pass run_validity=valid build_attestation_status=pass findings=[]
  ```
- Interpretation: 当前运行环境满足 provider-route 所需的 binary provenance。
- Time: 2026-08-02 17:12

## Evidence E-006: binary-health 已前置且不会认领授权
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: `scripts/cache-regression/test_cache_binary_health.py`；提交 `0076e720a`
- Prediction or plan link:
  - H-002 的校验顺序验收。
- Matched signal:
  - attestation failure 时 provider-route 未调用，ledger 仍为空。
- Correlation keys:
  - `CacheBinaryHealthRunnerTest.test_blocks_before_provider_route_and_authorization_claim`
- Raw content:
  ```text
  provider_route_mock.assert_not_called(); ledger.entries == []
  ```
- Interpretation: 过期 binary 现在在 provider-route 和 authorization claim 前被准确拒绝。
- Time: 2026-08-02 17:14
