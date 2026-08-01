# Problem P-001: 缓存 smoke 无法通过隔离 provider boundary 启动 Whale
- Status: open
- Created: 2026-08-01 13:11
- Updated: 2026-08-01 13:11
- Objective: 让获批缓存 smoke 在不绕过内置 DeepSeek provider 合同的前提下，经隔离 provider boundary 发出并记录请求。
- Symptoms:
  - Standard 首臂在 Agent 启动前退出，map-request 因失败即停未运行。
- Expected behavior:
  - benchmark 保持 `deepseek` 内置 provider 的认证、Responses wire 和模型语义，仅把 base URL 路由到隔离边界。
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
  - Codex 上游对内置 OpenAI 使用独立顶层 `openai_base_url`，而不是覆盖 `model_providers.openai`。
- Ruled out:
  - API Key、模型响应、TaskSpace 行为和缓存命中率不是本次失败原因，因为没有 provider 请求发生。
- Fix criteria:
  - 配置加载测试证明内置 DeepSeek 可通过专用受控 base URL 配置路由，同时保留 `model_providers.deepseek` 覆盖拒绝。
  - provider boundary 离线合同测试证明 benchmark 只注入该专用配置且仍选择 `model_provider="deepseek"`。
  - 获得新预算后，真实 smoke 的 Standard 与 map-request 均能到达边界并形成完整 usage 证据。
- Current conclusion: benchmark 使用了与内置 provider 保护合同冲突的注入路径；根因已由源码、启动错误和零请求边界证据确认。
- Related hypotheses:
  - H-001
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

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
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 provider boundary 与 argv artifact 作为永久观测。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 设计与上游 `openai_base_url` 同构的内置 DeepSeek 专用 base URL 配置，并保持保留 ID 拒绝测试。
- Blocker:
  - 该配置属于产品配置面变更，实施前需与用户确认路线。
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
