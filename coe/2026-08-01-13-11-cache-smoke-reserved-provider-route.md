# Problem P-001: 缓存 smoke 无法通过隔离 provider boundary 启动 Whale
- Status: validating
- Created: 2026-08-01 13:11
- Updated: 2026-08-01 18:18
- Objective: 让获批缓存 smoke 在不绕过内置 DeepSeek provider 合同的前提下，经隔离 provider boundary 发出并记录请求。
- Symptoms:
  - Standard 首臂在 Agent 启动前退出，map-request 因失败即停未运行。
- Expected behavior:
  - benchmark 复用现有自定义 provider 机制表达与内置 DeepSeek 等价的传输描述，只把本地 ID 和 base URL 改为显式可审计的隔离路由。
- Actual behavior:
  - benchmark 通过 `model_providers.deepseek.base_url` 覆盖保留内置 provider，配置加载器拒绝启动。
- Impact:
  - R8 accepted cache baseline 无法执行；已授权样本未产生模型请求或缓存数据。
- Reproduction:
  - 使用 `ProviderRequestHardLimit > 0` 启动 `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`。
- Environment:
  - Linux Docker benchmark；branch `whalecode-alpha`；subject commit `0490facf13bbef0cc3f75909bccdc9f8271b63be`。
- Known facts:
  - benchmark 固定追加 `model_providers.deepseek.base_url="http://provider-proxy:8080"`。
  - `deepseek` 是 Whale 保留内置 provider ID，用户定义表禁止覆盖。
  - provider boundary 与 wire trace 对账均为 0 请求。
  - 当前 custom provider 字段足以复现 DeepSeek 名称、认证变量、Responses wire、URL/query/header、认证、重试、超时和 WebSocket 能力。
  - 离线 `debug prompt-input` 探针中，内置 DeepSeek 与 transport alias 的 16,336 字节模型可见上下文逐字节相同。
- Ruled out:
  - API Key、模型响应、TaskSpace 行为和缓存命中率不是本次失败原因，因为没有 provider 请求发生。
- Fix criteria:
  - 生产配置解析测试证明 transport alias 可加载，并与内置 DeepSeek 除 `provider_id` 和 `base_url` 外字段完全相同。
  - provider boundary 离线合同测试证明 benchmark 不再覆盖保留 ID，并记录 logical provider 与 transport provider ID。
  - 全局 final-wire provider ID 保护保持不变；测试别名不得成为产品默认配置或静默例外。
  - 获得新预算后，真实 smoke 的 Standard 与 map-request 均能到达边界并形成完整 usage 证据。
- Current conclusion: 根因是 provider boundary 与生产配置加载之间缺少组合测试，导致 harness 把自定义 provider 语法误用于内置 ID；无需新增产品配置能力。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-001 confirmed the invalid reserved-ID override.
  - H-002 and E-004/E-005 validate the custom transport alias through production config parsing, prompt equivalence, container routing, and final-wire contracts.
- Close reason:
  - exact production-entry preflight、alias normal final-wire、route evidence binding 与失败路径清理通过 Round 8 fresh closure。问题继续标记 `validating` 仅因为真实 provider/cache 验证需要另行授权，不代表仍有已知工程缺陷。

## Hypothesis H-001: 保留 provider 覆盖导致启动前失败
- Status: confirmed
- Parent: P-001
- Claim: `ProviderRequestHardLimit` 分支注入 `model_providers.deepseek.base_url`，触发保留内置 ID 校验，因此 Whale 在任何 provider dispatch 前退出。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 启动错误直接命名 `model_providers.deepseek`，且注入点与硬上限分支一一对应。
- Falsifiable predictions:
  - If true: argv 中存在该覆盖；stderr 出现保留 ID 错误；边界请求数为 0。
  - If false: Whale 应越过配置加载，或边界至少观察到一次请求。
- Diagnostic evidence plan:
  - Prediction or clause under test: 覆盖在配置加载期被拒绝且请求尚未 dispatch。
  - Signal: Whale argv、stderr、provider boundary evidence、相关配置源码。
  - Capture method: 读取已授权运行的不可变 artifact，并追踪配置合并函数。
  - Event name or marker:
    - `provider-boundary-evidence`
  - Correlation keys:
    - `WAR-20260801-130559-CACHE-REGRESSION-DDFF3293-CACHE-001`
  - Differentiates from:
    - 凭证失败、provider 网络失败、模型拒绝和 Agent 行为失败。
  - Supports if:
    - argv 含保留 ID 覆盖、stderr 精确拒绝、边界和 wire 均为 0。
  - Refutes if:
    - 配置加载成功并到达 provider boundary。
  - Instrumentation status: permanent-observability
  - Instrumentation lifecycle:
    - 保留 provider boundary 与 argv artifact 作为永久观测。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 使用现有 custom provider transport alias，并增加字段等价、配置加载和路由证据测试。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 启动错误命中保留 provider 校验
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target/cache-hit-regression/WAR-20260801-130559-CACHE-REGRESSION-DDFF3293/single-file-fast-fix/20260801-130559-657/pair-001/left/artifacts/whale-exec.stderr.log`
- Prediction or plan link:
  - H-001 启动期拒绝预测。
- Matched signal:
  - `model_providers contains reserved built-in provider IDs: deepseek`
- Correlation keys:
  - `WAR-20260801-130559-CACHE-REGRESSION-DDFF3293-CACHE-001`
- Raw content:
  ```text
  Error loading config.toml: model_providers contains reserved built-in provider IDs: `deepseek`.
  Built-in providers cannot be overridden.
  ```
- Interpretation: 失败发生在配置解析阶段，不是模型或 Agent 返回的错误。
- Time: 2026-08-01 13:06

## Evidence E-002: benchmark 注入精确命中被禁配置
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1:301`
- Prediction or plan link:
  - H-001 argv 覆盖预测。
- Matched signal:
  - `model_providers.deepseek.base_url="http://provider-proxy:8080"`
- Correlation keys:
  - none
- Raw content:
  ```text
  if ($ProviderRequestHardLimit -gt 0) {
      $effectiveConfigOverrides += 'model_providers.deepseek.base_url="http://provider-proxy:8080"'
  }
  ```
- Interpretation: 运行错误与唯一注入点存在直接因果对应。
- Time: 2026-08-01 13:11

## Evidence E-003: 请求未离开 Whale
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target/cache-hit-regression/WAR-20260801-130559-CACHE-REGRESSION-DDFF3293/single-file-fast-fix/20260801-130559-657/pair-001/left/artifacts/provider-boundary-evidence.json`
- Prediction or plan link:
  - H-001 请求尚未 dispatch 预测。
- Matched signal:
  - `status=reconciled, boundary_request_count=0, wire_request_count=0`
- Correlation keys:
  - `WAR-20260801-130559-CACHE-REGRESSION-DDFF3293-CACHE-001`
- Raw content:
  ```text
  {"status":"reconciled","boundary_request_count":0,"wire_request_count":0,"errors":[]}
  ```
- Interpretation: 凭证、网络、模型和缓存路径均未被执行，排除了这些替代根因。
- Time: 2026-08-01 13:06

## Hypothesis H-002: 现有 custom provider 可无损承载隔离路由
- Status: confirmed
- Parent: P-001
- Claim: 以非保留 ID 声明与内置 DeepSeek 字段等价的 provider，并显式传入模型，只会改变本地 provider ID 和代理 URL，不改变模型可见上下文或 provider 请求语义。
- Layer: fix-validation
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 运行 client 接收 `ModelProviderInfo` 而非 provider ID；DeepSeek 专用压缩策略按 provider 名称判断；benchmark 已显式传模型。
- Falsifiable predictions:
  - If true: 配置可加载；alias 与内置 provider 字段除 URL 外相同；模型可见上下文逐字节相同。
  - If false: 配置拒绝、provider 字段漂移，或 prompt input 出现差异。
- Diagnostic evidence plan:
  - Prediction or clause under test: 现有自定义 provider 机制足以表达隔离 DeepSeek 路由。
  - Signal: 配置加载结果、provider 字段比较、`debug prompt-input` 字节和 SHA-256。
  - Capture method: 使用隔离 HOME 执行不发 provider 请求的机械命令，并增加本地生产解析测试。
  - Event name or marker:
    - `provider_routing`
  - Correlation keys:
    - `deepseek-boundary`
  - Differentiates from:
    - 新增顶层产品配置或放开保留 ID 覆盖。
  - Supports if:
    - 配置加载成功，字段比较和 prompt 字节比较均通过。
  - Refutes if:
    - 任一执行语义字段或模型可见输入发生非预期变化。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 把 logical/transport provider 身份保留在 benchmark artifact。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 完成离线回归；真实 provider 复验仍需新预算。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-004: transport alias 配置与 prompt 离线等价
- Related hypotheses:
  - H-002
- Direction: supports
- Type: experiment
- Source: `target/provider-alias-probe`
- Prediction or plan link:
  - H-002 配置加载与模型可见上下文等价预测。
- Matched signal:
  - `login status` 越过配置加载；两份 prompt input 均为 16,336 bytes，SHA-256 同为 `5393ca12accc851bc78e1b97469b8f92edd00c2a8e49094c9f4eddd3ccf5e144`。
- Correlation keys:
  - `deepseek-boundary`
- Raw content:
  ```text
  builtin_sha256=5393ca12accc851bc78e1b97469b8f92edd00c2a8e49094c9f4eddd3ccf5e144
  alias_sha256=5393ca12accc851bc78e1b97469b8f92edd00c2a8e49094c9f4eddd3ccf5e144
  cmp_exit_code=0
  ```
- Interpretation: 不需要新增产品 base URL 配置；现有 custom provider 已能保持模型可见上下文不变。
- Time: 2026-08-01 13:29

## Evidence E-005: 生产解析、容器路由与最终请求合同回归
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source:
  - `third_party/codex-cli/codex-rs/core/tests/suite/cache_provider_boundary_route.rs`
  - `scripts/taskspace-benchmark/test-provider-boundary.ps1`
  - `third_party/codex-cli/codex-rs/core/tests/suite/cache_payload_contract.rs`
- Prediction or plan link:
  - H-002 字段等价与最终请求语义不变预测。
- Matched signal:
  - 生产 `ConfigToml` 成功解析 transport alias，且 alias 与内置 DeepSeek 除 base URL 外的 `ModelProviderInfo` 完全相等。
  - Docker provider boundary 自测通过，未出现 `model_providers.deepseek.*` 覆盖。
  - DeepSeek cache payload 合同 9 项通过；缓存回归 196 项通过。
- Correlation keys:
  - `deepseek-boundary`
- Raw content:
  ```text
  provider_boundary_alias_matches_builtin_deepseek_runtime_fields: PASS
  provider boundary tests passed
  cache_payload_: 9 passed
  cache regression: 196 tests OK
  cache regression gate: PASS (pending validation policy change; release remains blocked)
  ```
- Interpretation: 修复仅改变 benchmark 的本地传输寻址，并保留生产 DeepSeek provider、模型输入和最终请求合同。
- Time: 2026-08-01 14:12

## Evidence E-006: exact CLI、no-dispatch Docker 与证据绑定闭环
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source:
  - `scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1`
  - `target/provider-route-preflight/provider-route-preflight.json`
  - `third_party/codex-cli/codex-rs/cli/tests/debug_provider.rs`
  - `third_party/codex-cli/codex-rs/core/tests/suite/cache_provider_boundary_route.rs`
- Prediction or plan link:
  - H-002 的完整配置加载、provider 语义等价和永久观测要求。
- Matched signal:
  - 真实 Whale CLI 在 Docker `network=none` 下分别解析 Standard 与 map-request；两臂 descriptor SHA 相同。
  - 本证据最初记录的 `dispatch=0` 是脚本常量，已撤销为有效观测信号；由 E-007/E-008 的 `config_resolution_only` 代码路径与 inspect receipt 取代。
  - alias 与内置 DeepSeek 的 normal final-wire JSON body 完全相等。
  - route identity 与预检 artifact SHA 已绑定 arm、result 和 ledger；promotion 会重读原件。
  - 非 DeepSeek 模型、route 漂移、原件篡改和 preflight-before-claim 均有拒绝测试。
- Correlation keys:
  - `deepseek-boundary`
  - `whalecode-provider-route-preflight-v1`
- Raw content:
  ```text
  provider route preflight: passed, network=none
  profiles: standard, taskspace
  cache regression: 202 tests OK
  debug_provider: 1 passed
  cache_provider_boundary_route: 2 passed
  ```
- Interpretation: 第一轮对抗性审查接受的 B1、B2、B3 与 N2 已形成离线工程证据；Round 2 进一步发现 descriptor 与原始 artifact 复核缺口。
- Time: 2026-08-01 16:18

## Evidence E-007: Round 2 descriptor、原始证据和入口边界修复
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source:
  - `third_party/codex-cli/codex-rs/cli/src/main.rs`
  - `scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1`
  - `scripts/cache-regression/cache_provider_route.py`
  - `scripts/cache-regression/accepted_cache_baseline.py`
  - `scripts/taskspace-benchmark/lib/provider-boundary.ps1`
- Prediction or plan link:
  - H-002 的完整 provider 行为等价、原始证据可复核和非 DeepSeek 机械拒绝要求。
- Matched signal:
  - query/header/token 等行为字段已进入非明文 descriptor；一致但越界的 query 参数会被 alias/built-in 等价检查拒绝。
  - promotion 会从当前 Git source 读取四份 resolved-provider 原件，缺失、摘要篡改和跨 record 均有拒绝测试。
  - Standard arm 绑定 Standard profile，TaskSpace arm 绑定 TaskSpace profile；result/ledger 不接受错绑。
  - benchmark 最早参数门禁与 boundary 启动层均拒绝 `gpt-5` 等非 DeepSeek 模型。
  - Python cache regression `207 tests OK`；CLI `1 passed`；normal final-wire `2 passed`；Docker boundary 与 config-only preflight 均通过。
- Correlation keys:
  - `deepseek-boundary`
  - `whalecode-provider-route-preflight-v1`
- Raw content:
  ```text
  cache regression: 207 tests OK
  debug_provider: 1 passed
  cache_provider_boundary_route: 2 passed
  provider boundary tests passed
  ProviderRoutePreflight: passed (config_resolution_only, network=none)
  ```
- Interpretation: Round 2 的四个 P1 和两个证据类 P2 已按根因收敛；仍等待新的 fresh reviewer 复核后关闭工程问题。
- Time: 2026-08-01 16:48

## Evidence E-008: keyed descriptor 与 Docker inspect receipt
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source:
  - `third_party/codex-cli/codex-rs/cli/src/main.rs`
  - `third_party/codex-cli/codex-rs/cli/tests/debug_provider.rs`
  - `scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1`
  - `scripts/cache-regression/cache_provider_route.py`
- Prediction or plan link:
  - H-002 的 provider 行为完整等价与敏感配置不泄露要求。
- Matched signal:
  - 同一 HMAC key 下 token 值变化会改变 descriptor；同一敏感值换 key 后 fingerprint 变化，artifact 不能作为离线猜测校验器。
  - HMAC key 通过临时只读 secret mount 交给四次 CLI 解析，不进入环境变量值、artifact 或 Git；预检结束后源文件已删除。
  - 四份 sanitized inspect receipt 均来自 Docker inspect，记录 `network_mode=none`、workspace 只读和 key 文件挂载；不含 `/home/`、`/tmp/` 或 secret 源路径。
  - 实际 preflight attestation 与 8 份受保护原始/receipt artifact 通过 Python 复核；cache regression `207 tests OK`。
- Correlation keys:
  - `whalecode-provider-route-container-inspect-v1`
  - `whalecode-provider-route-preflight-v1`
- Raw content:
  ```text
  debug_provider: 1 passed
  cache regression: 207 tests OK
  4 sanitized inspect receipts passed; raw host and secret paths absent
  actual preflight attestation + 8 protected source artifacts passed
  ```
- Interpretation: Round 3 的 descriptor 完整性、离线猜测和 inspect 真实性问题已形成离线修复证据；等待 fresh Round 4 closure review。
- Time: 2026-08-01 17:13

## Evidence E-009: failure-atomic 临时证据所有权
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source:
  - `scripts/cache-regression/cache_provider_route.py`
  - `scripts/cache-regression/test_cache_provider_route.py`
  - `scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1`
  - `scripts/taskspace-benchmark/lib/container-runtime.ps1`
- Prediction or plan link:
  - keyed fingerprint 的 key 不得在成功、失败或外层 timeout 后与 artifact 同时遗留；raw inspect 不得进入持久化证据目录。
- Matched signal:
  - Python 父进程拥有唯一临时目录和 0600 HMAC key，并通过 `TemporaryDirectory` 覆盖正常、非零和 timeout 清理。
  - container runtime 可把 raw inspect 定向到该临时目录；PowerShell 在检查 CLI 退出码前先生成 sanitized receipt，并在 `finally` 删除 raw inspect。
  - timeout 与非零单元故障注入通过；真实 Docker 使用 `/bin/false` 的非零路径中 raw inspect 和 secret 宿主路径遗留计数均为 0。
- Correlation keys:
  - `provider_route_cli_failed`
  - `provider route preflight timed out before authorization claim`
- Raw content:
  ```text
  test_cache_provider_route: 9 tests OK
  expected_failure=provider_route_cli_failed
  raw_inspect_count=0
  secret_path_leak_count=0
  ```
- Interpretation: Round 4 发现的 failure-atomic P1 已按资源所有权根因修复；等待 fresh Round 5 closure review。
- Time: 2026-08-01 17:38

## Evidence E-010: 进程组、授权指纹与只读 receipt 闭环
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source:
  - `scripts/cache-regression/cache_process_control.py`
  - `scripts/cache-regression/cache_execution_identity.py`
  - `scripts/cache-regression/cache_surface.py`
  - `scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1`
- Prediction or plan link:
  - 父进程退出不能代替整个进程组退出证明；能改变 route 证明的脚本必须受授权指纹保护；receipt 必须证明 secret mount 只读。
- Matched signal:
  - POSIX timeout 保存 PGID；SIGTERM 后仍存活的组会收到 SIGKILL，只有确认组不存在才设置 `descendants_guaranteed_terminated=true`。
  - 真实忽略 SIGTERM 的后代故障注入返回 `killed`，未继续持有 key；preflight 拒绝未确认的进程树清理。
  - `invoke-provider-route-preflight.ps1` 已进入 execution identity 和 cache control-plane，修改后旧授权拒绝。
  - 失败 attestation 不再保存原始异常；receipt 新增强制 `descriptor_key_read_only=true`。
- Correlation keys:
  - `posix_process_group`
  - `descriptor_key_read_only`
- Raw content:
  ```text
  process control: 20 tests OK
  provider route: 11 tests OK
  success: receipts=4 leaks=0
  nonzero: receipts=1 leaks=0
  ```
- Interpretation: Round 5 的两个 P1 和两个 P2 已按可复现失败场景修复；等待 fresh Round 6 closure review。
- Time: 2026-08-01 17:55

## Evidence E-011: lifecycle 脱敏与 secret 源挂载唯一性
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source:
  - `scripts/taskspace-benchmark/lib/container-runtime.ps1`
  - `scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1`
  - `scripts/cache-regression/cache_provider_route.py`
- Matched signal:
  - container create/cleanup 失败的持久化 lifecycle 只记录稳定 reason code 与机械摘要；宿主路径故障注入通过。
  - raw inspect 同时按预期 destination 和解析后的 DescriptorKeyPath source 计数；两者都必须恰为 1，且 mount 必须只读。
  - 最新真实 Docker config-only preflight 生成 4 份 receipt，全部满足 source unique/read-only 且不含 `/home/`、`/tmp/`。
- Raw content:
  ```text
  container runtime tests passed
  provider route: 11 tests OK
  passed receipts=4 artifact=target/provider-route-round6-1785578747
  ```
- Interpretation: Round 6 的两个非阻断证据缺口已修复，等待窄范围 fresh Round 7 closure review。
- Time: 2026-08-01 18:06

## Evidence E-012: secret mount 同一实体关联证明
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source:
  - `scripts/taskspace-benchmark/lib/container-contract.ps1`
  - `scripts/taskspace-benchmark/test-provider-boundary.ps1`
  - `scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1`
- Matched signal:
  - mount helper 要求同一个 mount 同时满足 host source、container destination，并在该对象上验证 `RW=false`。
  - correct source/wrong destination 与 wrong source/correct destination 的双 mount 拼接 fixture 无法再伪造证明。
  - receipt 新增强制 `descriptor_key_mount_identity_confirmed=true`；Python promotion 重读验证。
- Raw content:
  ```text
  provider boundary tests passed
  provider route: 11 tests OK
  passed receipts=4 artifact=target/provider-route-round7-1785579110
  ```
- Interpretation: Round 7 split-mount P1 已按 mount 实体关联根因修复，等待 mandatory fresh Round 8 review。
- Time: 2026-08-01 18:12

## Evidence E-013: mandatory fresh Round 8 closure
- Related hypotheses:
  - H-002
- Direction: supports
- Type: adversarial-closure
- Source:
  - `vs_review/2026-08-01-r8-provider-boundary-alias-review.md`
- Matched signal:
  - fresh reviewer 在窄范围内未发现 P0/P1/P2。
  - 63 种 mount 组合的错误通过数为 0；伪造 identity 并重算 receipt/attestation SHA 后，promotion 仍拒绝。
- Raw content:
  ```text
  PASS
  mount combinations enumerated: 63
  false accepts: 0
  ```
- Interpretation: provider route 工程问题完成 fresh closure；真实缓存收益仍是受预算门禁保护的外部验证项。
- Time: 2026-08-01 18:18
