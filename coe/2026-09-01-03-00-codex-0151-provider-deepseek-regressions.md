# Problem P-001: Codex 0.151 Provider / DeepSeek 重放回归
- Status: fixed
- Created: 2026-09-01 03:00
- Updated: 2026-09-01 04:10
- Objective: 在不改变既定 Provider/DeepSeek 产品语义的前提下，恢复 0.151 上的凭据、模型目录、路由、history projection、Responses 与 cache 合同。
- Symptoms:
  - `codex-model-provider-info` 单元测试因 `RedactedString` 类型迁移未适配而无法编译。
  - 代码盘点显示当前 `models.json` 未检出 DeepSeek 三模型，但现有测试仍要求 bundled catalog 包含它们。
  - TUI 初始 bootstrap 使用无 provider groups 的 `ModelCatalog::new`，需验证是否导致初始 Provider picker 丢失分组。
- Expected behavior:
  - Provider 测试适配 0.151 的敏感字符串类型；bundled catalog 保留 DeepSeek Flash/Pro/Vision，Flash 默认；初始与登录后 Provider catalog 语义一致。
- Actual behavior:
  - 五项迁移遗漏均已由编译、测试或官方 0.151 源码对照确认；修复后正在执行完整 W4 回归。
- Impact:
  - W4 尚不能标记 verified；模型目录或初始分组若确认丢失会直接影响既定多 Provider 产品入口。
- Reproduction:
  - `cargo test --manifest-path third_party/codex-cli/codex-rs/Cargo.toml -p codex-model-provider-info --lib --no-fail-fast`
- Environment:
  - Linux；分支 `whalecode-codex`；vendor `rust-v0.151.0`；W3 commit `6c2203c5c0`。
- Known facts:
  - W3 CLI build/check 与通用回归通过；这些不会编译所有 Provider crate 的 test-only 代码，也不验证 bundled model data。
- Ruled out:
  - CLI substrate 编译失败不是当前问题。
- Fix criteria:
  - Provider/model/login 定向矩阵编译并通过；DeepSeek 三模型与 Flash 默认有数据级测试；初始 Provider groups 有协议/TUI 回归；免费 cache gate 给出可接受结论。
- Current conclusion: H-001 至 H-007 均已确认并修复；W4 定向矩阵与免费 cache gate 已形成可比较的 0.151 final-wire 候选，真实基线晋升保留到 W6。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Resolution basis:
  - Provider、模型目录、认证恢复、路由/history/compaction、DeepSeek Responses、TUI bootstrap 与 final-wire 免费矩阵通过。
  - cache gate 接受候选指纹 `a14e29c02a1c36c51815072a0c948137fab58e8d60da8a58b1b4fd246b739abb`，并继续阻断发布基线晋升。
- Close reason:
  - 0.151 接口与测试迁移缺口已闭合；剩余真实 cache revalidation 是发布资格工作，不是本故障未修复。

## Hypothesis H-001: Provider test overlay 未适配 0.151 RedactedString
- Status: confirmed
- Parent: P-001
- Claim: 上游把 provider bearer token 从 `String` 改为 `RedactedString`，但 Whale 既有测试仍调用 `String::as_str`，造成 test-only 编译失败。
- Layer: compatibility
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 编译器期望 `fn(&RedactedString)`，实际传入 `String::as_str`。
- Falsifiable predictions:
  - If true: 生产字段类型为 `RedactedString`，只需使用其安全暴露接口或等价断言即可恢复编译。
  - If false: 错误来自字段类型推断或生产 API 破坏。
- Diagnostic evidence plan:
  - Prediction or clause under test: test helper 的类型假设是否落后于 0.151 生产字段。
  - Signal: 字段定义、可用安全比较 API、编译错误。
  - Capture method: 只读源码比对与定向测试。
  - Event name or marker:
    - `E0631`
  - Correlation keys:
    - `bearer_token`
  - Differentiates from:
    - Provider 运行时构造失败。
  - Supports if:
    - 仅测试映射函数签名不匹配。
  - Refutes if:
    - 生产调用也无法构造 provider。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: test overlay 的类型迁移遗漏。
- Repair design readiness: ready
- Next step: 使用 `RedactedString` 现有非 Debug 暴露接口完成等价断言，不降低敏感信息保护。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 机械切换遗漏 DeepSeek bundled model data
- Status: confirmed
- Parent: P-001
- Claim: 0.151 官方 `models.json` 替换了 Whale catalog，未重放 DeepSeek Flash/Pro/Vision 条目，导致默认模型和 picker 数据源为空。
- Layer: data
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 当前文件搜索不到 `deepseek-v4`，而 W4 既有测试明确读取 bundled catalog。
- Falsifiable predictions:
  - If true: `codex-models-manager` bundled DeepSeek tests 失败，HEAD^ 文件含三条完整模型定义。
  - If false: DeepSeek 条目由其他生成或合并数据源注入，测试仍通过。
- Diagnostic evidence plan:
  - Prediction or clause under test: bundled catalog 的实际解析结果是否包含三模型。
  - Signal: models-manager 定向测试与当前/HEAD^ 数据差分。
  - Capture method: 本地测试和只读 diff。
  - Event name or marker:
    - `bundled_deepseek_models_are_visible_with_flash_default`
  - Correlation keys:
    - `deepseek-v4-flash`
  - Differentiates from:
    - picker 过滤逻辑错误。
  - Supports if:
    - 数据条目缺失且 bundled test 失败。
  - Refutes if:
    - 解析结果仍含三模型。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: `models.json` 缺失三条 DeepSeek 定义，解析结果为空；恢复数据后 55/55 通过。
- Repair design readiness: ready
- Next step: 纳入 W4 完整回归与 cache gate。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: TUI bootstrap 未携带 Provider model groups
- Status: confirmed
- Parent: P-001
- Claim: 0.151 bootstrap 只返回 flat available models，TUI 用 `ModelCatalog::new` 丢弃 Provider groups；只有登录后刷新才用 `with_provider_groups`，因此首次启动的 Provider picker 不完整。
- Layer: integration
- Factor relation: part_of
- Depends on:
  - H-002
- Rationale:
  - `AppServerBootstrap` 没有 groups 字段，startup 明确构造空 groups catalog。
- Falsifiable predictions:
  - If true: app-server 已能生成 provider catalog，但 bootstrap 未取/传，TUI 初始 slash/provider 测试缺失或失败。
  - If false: 首次 Provider picker 有独立按需刷新，或 flat models 是设计合同。
- Diagnostic evidence plan:
  - Prediction or clause under test: 初始启动到 Provider picker 是否存在未执行的 catalog refresh。
  - Signal: bootstrap RPC 序列、TUI catalog 初始化与 provider slash tests。
  - Capture method: 源码调用链和定向测试。
  - Event name or marker:
    - `ModelCatalog::new`
  - Correlation keys:
    - `provider_groups`
  - Differentiates from:
    - bundled model data缺失。
  - Supports if:
    - 无启动 refresh 且空 groups 到达 picker。
  - Refutes if:
    - picker 打开前必定刷新 groups。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: `model/list` 已返回 groups，登录刷新路径也消费 groups，只有初始 bootstrap 丢弃该字段。
- Repair design readiness: ready
- Next step: 验证 TUI provider 测试目标。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: RedactedString 测试编译错误
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: `/tmp/codex-0151-w4-provider-info.log`
- Prediction or plan link:
  - H-001：测试仍假设 bearer token 是 String。
- Matched signal:
  - `Option<&RedactedString>::map` 收到 `String::as_str`，编译器报告 E0631。
- Correlation keys:
  - `model_provider_info_tests.rs:17`
- Raw content:
  ```text
  expected function signature fn(&RedactedString) -> _
  found signature fn(&String) -> _
  ```
- Interpretation: test-only overlay 需要按 0.151 敏感类型迁移，不能把生产类型改回裸 String。
- Time: 2026-09-01 03:00

## Hypothesis H-004: Bedrock access keys 未完整进入 0.151 auth 生命周期判定
- Status: confirmed
- Parent: P-001
- Claim: `BedrockAccessKeys` 已加入 auth 类型，但遗漏 `has_native_auth_material`、workspace exemption 与 refresh equality，导致加载或 reload 语义错误。
- Layer: compatibility
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 精确测试先后在初次加载、reload changed 与 revision 断言失败；逐项补齐后通过。
- Falsifiable predictions:
  - If true: 官方同类 API-key auth 分支均有对应处理，补齐后 round-trip 测试通过。
  - If false: 失败会继续发生在存储后端或 logout 删除路径。
- Diagnostic evidence plan:
  - Prediction or clause under test: access keys 是否在 native material、限制验证与 refresh equality 中完整分类。
  - Signal: `access_keys_auth_round_trips_and_logs_out` 的阶段性失败位置。
  - Capture method: 单测与官方 0.151 源码对照。
  - Event name or marker: `BedrockAccessKeys`
  - Correlation keys: `auth_revision`
  - Differentiates from: 存储删除失败。
  - Supports if: 补齐分类后同一测试全绿。
  - Refutes if: 存储内容或删除仍错误。
  - Instrumentation status: removed
  - Instrumentation lifecycle: 临时 redacted variant 输出已删除。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: auth variant replay 不完整。
- Repair design readiness: ready
- Next step: login 全量回归。
- Blocker: none
- Close reason: not closed

## Hypothesis H-005: 外部认证首次失败时的恢复实现重放不完整
- Status: confirmed
- Parent: P-001
- Claim: recovery 门禁允许 cached auth 为 None，但 refresh 实现仍在 None 时提前返回，未执行 provider command。
- Layer: integration
- Factor relation: part_of
- Depends on: none
- Rationale:
  - `has_next()` 为真、`next()` 返回成功，但 cache 仍为 None；与官方 0.151 实现精确不同。
- Falsifiable predictions:
  - If true: 将 external refresh 判定置于 cached-auth 分支之前即可恢复 token。
  - If false: provider script 本身仍失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: 空 cache 是否短路 `refresh_external_auth`。
  - Signal: recovery 后 `auth_cached()`。
  - Capture method: 精确单测与 upstream tag 对照。
  - Event name or marker: `refresh_token_from_authority_impl`
  - Correlation keys: `provider-token`
  - Differentiates from: recovery 门禁错误。
  - Supports if: 官方控制流恢复后测试通过。
  - Refutes if: command 没有产出 token。
  - Instrumentation status: none
  - Instrumentation lifecycle: none
- Evidence gate: satisfied
- Related evidence:
  - E-005
- Conclusion: 官方 0.151 refresh 控制流漏重放。
- Repair design readiness: ready
- Next step: login 全量回归。
- Blocker: none
- Close reason: not closed

## Hypothesis H-006: TUI test fixture 未适配既有 0.151/Whale 字段
- Status: confirmed
- Parent: P-001
- Claim: TUI 生产代码已带 `taskspace_viewer` 与 provider `route`，但 test-only initializer/call sites 仍使用旧签名。
- Layer: test-compatibility
- Factor relation: independent
- Depends on: none
- Rationale:
  - lib test 编译器精确报告 16 个缺字段/缺参数错误，均位于测试文件。
- Falsifiable predictions:
  - If true: fixture 传 `None` 或忽略非断言字段后可编译，生产代码无需改动。
  - If false: 随后会暴露生产类型不一致。
- Diagnostic evidence plan:
  - Prediction or clause under test: 错误是否全部局限 test-only call sites。
  - Signal: E0063/E0061/E0027 的路径。
  - Capture method: `cargo test -p codex-tui --lib provider`。
  - Event name or marker: `missing field route`
  - Correlation keys: `popups_and_settings.rs`
  - Differentiates from: bootstrap groups 生产回归。
  - Supports if: 所有错误文件均为 test support/tests。
  - Refutes if: 生产目标也不能编译。
  - Instrumentation status: none
  - Instrumentation lifecycle: none
- Evidence gate: satisfied
- Related evidence:
  - E-006
- Conclusion: test-only 机械迁移遗漏。
- Repair design readiness: ready
- Next step: 最小 fixture 适配并重跑 TUI provider tests。
- Blocker: none
- Close reason: not closed

## Evidence E-004: Bedrock access keys 分阶段断言定位
- Related hypotheses: [H-004]
- Direction: supports
- Type: test
- Source: `codex-login` 精确测试
- Prediction or plan link: H-004
- Matched signal: 初始加载为 None；补 native material 后 reload 不变但 revision 增加；补 refresh equality 后通过。
- Correlation keys: `access_keys_auth_round_trips_and_logs_out`
- Raw content: `1 passed; 0 failed`
- Interpretation: 三个遗漏共同构成同一 variant 生命周期不完整。
- Time: 2026-09-01 03:35

## Evidence E-005: 官方 0.151 external refresh 控制流对照
- Related hypotheses: [H-005]
- Direction: supports
- Type: source-and-test
- Source: tag `upstream-rust-v0.151.0` 与本地精确测试
- Prediction or plan link: H-005
- Matched signal: 官方先判断 external auth，再对 optional cached auth 分支；恢复后测试通过。
- Correlation keys: `refresh_token_from_authority_impl`
- Raw content: `unauthorized_recovery_retries_provider_command_after_initial_failure ... ok`
- Interpretation: 首次 provider command 失败后的 bounded retry 已恢复。
- Time: 2026-09-01 03:40

## Evidence E-006: TUI test-only 0.151 接口编译错误
- Related hypotheses: [H-006]
- Direction: supports
- Type: compile-test
- Source: `cargo test -p codex-tui --lib provider`
- Prediction or plan link: H-006
- Matched signal: 3 个 `taskspace_viewer`、12 个 popup `route`、1 个 ThreadSettings `route` 缺失。
- Correlation keys: `E0063`, `E0061`, `E0027`
- Raw content: `could not compile codex-tui (lib test) due to 16 previous errors`
- Interpretation: 均为 fixture/call-site 迁移，不需要改变产品逻辑。
- Time: 2026-09-01 03:50

## Evidence E-002: 当前 bundled data 未检出 DeepSeek slug
- Related hypotheses:
  - H-002
- Direction: neutral
- Type: code-location
- Source: `models-manager/models.json` 当前 index 与 `HEAD^`
- Prediction or plan link:
  - H-002：需由实际解析测试确认数据影响。
- Matched signal:
  - 当前搜索无 `deepseek-v4`；W3 前文件在前 126 行内含 Flash、Pro、Vision 三条。
- Correlation keys:
  - `models.json`
- Raw content:
  ```text
  HEAD^:4 deepseek-v4-flash
  HEAD^:65 deepseek-v4-pro
  HEAD^:126 deepseek-v4-flash-vision-exp
  ```
- Interpretation: 高风险数据差异已存在，但在测试结束前不直接定根因。
- Time: 2026-09-01 03:00

## Evidence E-003: TUI startup 当前创建空 provider groups catalog
- Related hypotheses:
  - H-003
- Direction: neutral
- Type: code-location
- Source: `tui/src/app/startup.rs`、`tui/src/app_server_session.rs`
- Prediction or plan link:
  - H-003：需确认是否有后续按需 refresh 覆盖。
- Matched signal:
  - bootstrap 仅含 `available_models`；startup 调用 `ModelCatalog::new(available_models)`；登录完成路径才使用 `with_provider_groups`。
- Correlation keys:
  - `AppServerBootstrap`
  - `ModelCatalog::new`
- Raw content:
  ```text
  let model_catalog = Arc::new(ModelCatalog::new(available_models.clone()));
  ```
- Interpretation: 这是潜在首次启动集成缺口，尚需调用链证据。
- Time: 2026-09-01 03:00

## Hypothesis H-007: core integration-test 聚合目标仍含 0.151 test-only 迁移遗漏
- Status: confirmed
- Parent: P-001
- Claim: lib 定向测试编译通过后，`core --test all` 仍会被 ToolCall 生命周期、RedactedString 与 FunctionCallOutput 新字段的旧夹具阻断，导致 cache gate 无法生成 final-wire 候选。
- Layer: test-compatibility
- Factor relation: independent
- Depends on: none
- Rationale: 两条 final-wire 命令最初均 exit 101 且无报告；直接运行聚合目标得到 5 个精确编译错误。
- Falsifiable predictions:
  - If true: 仅迁移三个 test-only call site 后，聚合目标可编译并进入 snapshot 比较。
  - If false: 修复后仍会在生产 API 或 runner 初始化阶段失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: 不可比较是否由共享 integration-test 编译失败导致。
  - Signal: `tool_batch_preflight.rs`、`cache_payload_contract.rs`、`client.rs` 的 E0726/E0195/E0308/E0063。
  - Capture method: 直接运行 cache gate 配置中的两个 cargo 命令。
  - Event name or marker: `could not compile codex-core (test "all")`
  - Correlation keys: `ToolCall<'call>`, `RedactedString`, `FunctionCallOutput`
  - Differentiates from: final-wire 真实序列化差异。
  - Supports if: 修复后从 uncomparable 转为 changed/unchanged。
  - Refutes if: 仍无候选报告。
  - Instrumentation status: none
  - Instrumentation lifecycle: none
- Evidence gate: satisfied
- Related evidence: [E-007, E-008]
- Conclusion: test-only 迁移遗漏掩盖了真实 final-wire 差分。
- Repair design readiness: completed
- Next step: none
- Blocker: none
- Close reason: fixed

## Evidence E-007: core 聚合目标编译恢复
- Related hypotheses: [H-007]
- Direction: supports
- Type: compile-test
- Source: `cargo test -q -p codex-core --test all --no-run`
- Prediction or plan link: H-007
- Matched signal: 修复三类夹具后 exit 0；测试二进制成功生成。
- Correlation keys: `b393bca6be`
- Raw content: `3 files changed, 10 insertions(+), 5 deletions(-)`
- Interpretation: 门禁的首轮 uncomparable 是验证基础设施迁移遗漏，而不是 payload 无法构造。
- Time: 2026-09-01 04:05

## Evidence E-008: W4 免费 cache gate 候选可比较
- Related hypotheses: [H-002, H-003, H-007]
- Direction: supports
- Type: deterministic-gate
- Source: `/tmp/w4-cache-gate.json`
- Prediction or plan link: W4 exit
- Matched signal: gate status pass、discovery_state changed、candidate_transition true；两个 final-wire 场景均由 uncomparable 转为 changed，其余六项免费合同通过。
- Correlation keys: `a14e29c02a1c36c51815072a0c948137fab58e8d60da8a58b1b4fd246b739abb`
- Raw content: `cache regression gate: PASS ...（已发现可比较的候选变更；发布继续阻断）`
- Interpretation: W4 可提交；0.151 新增的 context-window metadata/message passthrough 需在 W6 作为候选基线接受范围审查，不能静默覆盖旧基准。
- Time: 2026-09-01 04:10
