# Problem P-001: 缓存 smoke 无法通过隔离 provider boundary 启动 Whale
- Status: fixed
- Created: 2026-08-01 13:11
- Updated: 2026-08-01 14:12
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
  - 当前 custom provider 字段足以复现 DeepSeek 名称、认证变量、Responses wire、重试、超时和 WebSocket 能力。
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
  - 工程缺陷已离线闭合；新的真实 smoke 只用于缓存效果复验，不再用于证明该配置加载根因。

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
