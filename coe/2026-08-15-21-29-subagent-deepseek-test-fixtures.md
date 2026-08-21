# Problem P-001: 子 Agent 回归测试仍使用 GPT/Luna 夹具
- Status: fixed
- Created: 2026-08-15 21:29
- Updated: 2026-08-15 21:52
- Objective: 在不修改生产模型选择语义的前提下，把 16 项子 Agent 回归迁移到 DeepSeek Flash/Pro 合同并恢复通过。
- Symptoms:
  - core lib 2 项与 core integration 14 项子 Agent 测试失败，错误包含 unknown GPT model 或等待 GPT/Luna mock request 超时。
- Expected behavior:
  - 子 Agent 模型优先级、role override、reasoning、service tier、summary、fork context 测试使用 Whale 当前可用的 DeepSeek 模型并验证原始语义。
- Actual behavior:
  - 测试常量和局部 role fixture 仍显式使用 `gpt-5.4`、`gpt-5.4-mini`、`gpt-5.6-sol`、`gpt-5.6-terra`、`gpt-5.6-luna`。
- Impact:
  - Whale 的正式 core 测试矩阵产生 16 个红灯，无法直接证明 Multi-Agent DeepSeek 路径保持上游行为。
- Reproduction:
  - 运行 U17 failure manifest 中 `CL-GPT-SUBAGENT` 与 `CI-GPT-SUBAGENT` 列出的精确测试。
- Environment:
  - Linux；branch `whalecode-codex`；起始提交 `96da8ab53c1d1dd7a4d3322a22290d122412756d`。
- Known facts:
  - 生产默认模型为 `deepseek-v4-flash`；未指定子模型时 child config 继承 parent effective model/provider。
  - U17 日志中 16 项失败都被归入硬编码 GPT/Luna 子 Agent 夹具。
- Ruled out:
  - none
- Fix criteria:
  - 16 个原失败测试全部通过；相关 core lib/integration 子 Agent 测试无新增失败；生产代码无变化；fmt 与 cache gate 通过。
- Current conclusion: 16 项 GPT/Luna 子 Agent 夹具已迁移为 DeepSeek Flash/Pro 合同，精确测试和邻近测试全部通过。
- Related hypotheses:
  - H-001
- Resolution basis:
  - E-004：core lib 精确 2/2、integration 子 Agent 模块 25/25；完整 integration 失败数由 37 降为 23，恰好移除原 14 项。
- Close reason:
  - 测试夹具与 Whale DeepSeek 公共模型合同一致，生产代码未修改。

## Hypothesis H-001: 失败由测试模型夹具未迁移引起
- Status: confirmed
- Parent: P-001
- Claim: 16 项失败的控制流和断言仍然有效，唯一阻断是测试显式选择 Whale 公共目录不存在的 GPT/Luna 型号或等待这些型号的 mock 请求。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 生产 spawn config 从 parent effective model/provider 构建；失败日志明确报告可用模型仅为 DeepSeek Flash/Pro。
- Falsifiable predictions:
  - If true: 每项失败都发生在 GPT 型号校验或等待 GPT/Luna 请求处；替换为能力匹配的 DeepSeek 模型后原断言可以继续执行。
  - If false: 至少一项在改用 DeepSeek 后仍因生产 inheritance、fork、summary、reasoning 或 service-tier 行为错误失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: 逐项失败点只依赖硬编码模型身份，且 DeepSeek 元数据覆盖测试需要的能力差异。
  - Signal: U17 原始日志、测试源码常量/断言、`models.json` DeepSeek capability metadata。
  - Capture method: 对照 16 个失败名、源码引用和模型元数据；先不修改代码。
  - Event name or marker:
    - none
  - Correlation keys:
    - exact test name
  - Differentiates from:
    - 生产子 Agent 模型继承或配置优先级本身损坏
  - Supports if:
    - 16 项均由 GPT/Luna identity 前提触发，DeepSeek Flash/Pro 可表达原测试的 supported/unsupported capability 对照。
  - Refutes if:
    - 出现与模型 identity 无关的 fork/context/summary 行为失败，或 DeepSeek 元数据无法表达测试所需对照。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
- Conclusion: 16 项失败均保留了 GPT/Luna identity 前提；DeepSeek Flash/Pro 可覆盖继承、优先级、fork 与 reasoning 校验，service-tier/summary 的能力差异应由测试私有 catalog 构造，不能篡改生产元数据。
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 生产子 Agent 继承父模型和 provider
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `core/src/tools/handlers/multi_agents_common.rs:185-224`
- Prediction or plan link:
  - H-001：生产 inheritance 本身不依赖 GPT 默认值
- Matched signal:
  - `build_agent_shared_config` 将 `turn.model_info.slug` 和 `turn.provider.info()` 写入 child config。
- Correlation keys:
  - none
- Raw content:
  ```text
  config.model = Some(turn.model_info.slug.clone());
  config.model_provider = turn.provider.info().clone();
  ```
- Interpretation: 未显式覆盖时，DeepSeek parent 会生成 DeepSeek child；测试里的 GPT 常量不是生产默认值。
- Time: 2026-08-15 21:29

## Evidence E-003: DeepSeek 生产元数据与 16 项失败所需能力边界
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-result
- Source: `models-manager/models.json`、U17 `failure-manifest.md`、16 项测试源码
- Prediction or plan link:
  - H-001：模型 identity 是失败前提，但迁移必须保持各测试原始能力对照
- Matched signal:
  - `deepseek-v4-flash` 与 `deepseek-v4-pro` 均支持 `standard/high/max` reasoning，均不支持生产 service tier，且默认不发送 reasoning summary 参数；14 项 integration 测试可用两者表达模型优先级，summary 测试已有私有 catalog mutation；2 项 service-tier 测试需要同样的私有 catalog mutation。
- Correlation keys:
  - U17 `CL-GPT-SUBAGENT` 2 项、`CI-GPT-SUBAGENT` 14 项
- Raw content:
  ```text
  deepseek-v4-flash: reasoning=[standard, high, max], service_tiers=[], supports_reasoning_summary_parameter=false
  deepseek-v4-pro:   reasoning=[standard, high, max], service_tiers=[], supports_reasoning_summary_parameter=false
  ```
- Interpretation: 失败根因成立，但不能做盲目字符串替换；summary 与 reasoning 差异可用测试私有元数据表达，service-tier 用例应直接断言 Flash/Pro 的真实“不支持并清理”合同，避免伪造生产能力。
- Time: 2026-08-15 21:43

## Evidence E-004: DeepSeek 子 Agent 回归与完整矩阵验证
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test-result
- Source: 本机 `just test` 输出
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - core lib 的 2 项 service-tier 精确测试 2/2 通过；`subagent_notifications` 模块 25/25 通过；完整 core integration 为 1100 passed、23 failed、8 skipped，相比 U17 的 37 failed 精确减少 14 项；隔离 `TMPDIR=/dev/shm`、清除代理并串行运行的 core lib 为 2157 passed、21 failed，相比 U17 扣除已知代理污染后的 23 个有效失败减少 2 项。
- Correlation keys:
  - `spawn_agent_service_tier_`
  - `subagent_notifications`
- Raw content:
  ```text
  core lib exact: 2 tests run: 2 passed
  core integration subagent module: 25 tests run: 25 passed
  core integration full: 1123 run; 1100 passed; 23 failed; 8 skipped
  core lib isolated full: 2178 run; 2157 passed; 21 failed
  ```
- Interpretation: 原 16 项失败已全部消除且未把延期的 Guardian、remote catalog/plugin、hosted image 等产品面误报为通过；完整 lib 首轮额外失败来自 `/tmp/.git`、`/tmp/.codex` 和宿主代理污染，隔离复跑后消失。
- Time: 2026-08-15 21:52

## Evidence E-002: 上游子 Agent 测试仍声明 GPT/Luna 常量
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `core/tests/suite/subagent_notifications.rs:65-71`、`core/src/tools/handlers/multi_agents_tests.rs`
- Prediction or plan link:
  - H-001：失败测试显式选择不存在于 Whale 公共目录的模型
- Matched signal:
  - 常量与局部 fixture 使用 `gpt-5.4`、`gpt-5.6-terra`、`gpt-5.6-sol`、`gpt-5.6-luna`。
- Correlation keys:
  - U17 `CL-GPT-SUBAGENT`、`CI-GPT-SUBAGENT`
- Raw content:
  ```text
  const REQUESTED_MODEL: &str = "gpt-5.4";
  const V2_DEFAULT_MODEL: &str = "gpt-5.6-terra";
  const V2_REQUESTED_MODEL: &str = "gpt-5.6-sol";
  const ROLE_MODEL: &str = "gpt-5.4";
  ```
- Interpretation: 至少公共 integration fixture 尚未迁到 DeepSeek；仍需逐项确认能力断言能否等价迁移。
- Time: 2026-08-15 21:29
