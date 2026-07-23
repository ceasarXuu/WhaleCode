# Problem P-001: R7 轻量 Tool 绑定未形成全生产路径硬合同
- Status: open
- Created: 2026-07-24 02:40
- Updated: 2026-07-24 04:42
- Objective: 修复对抗性审查确认的六类生产路径缺口，使 TaskSpace 连续动作、反馈忠实性、Tool 覆盖和 Standard 隔离满足声明的硬合同。
- Symptoms:
  - 未完成 provider response 的 Tool 前缀可能在 mailbox 抢占后执行。
  - 机械无效 control 可能在已有普通 Tool 执行后才失败。
  - ToolSearch 错误被编码为成功空结果。
  - ToolSearch 返回的延迟 Tool 未携带 Runtime 要求的 binding。
  - 部分 ToolSpec 不能参与 binding/preflight。
  - Standard Tool 的同名业务参数会被 TaskSpace Router 逻辑移除。
- Expected behavior:
  - 只有完整 provider response 才能进入原子 preflight 和执行。
  - 所有机械可判定的 response 合同错误在任何 Tool 执行前被拒绝。
  - Tool 失败事实忠实进入上下文并阻止依赖后续动作。
  - 所有 TaskSpace 可见普通 Tool 都能表达 binding；不能表达的 Tool 明确不暴露。
  - Standard 参数和执行语义不经过 TaskSpace 提取或校验。
- Actual behavior:
  - 六个缺口均可由当前生产代码路径直接成立。
- Impact:
  - R7 TaskSpace 连续动作正确性、反馈层语义、动态 Tool 能力和 Standard 隔离。
- Reproduction:
  - 见 H-001 至 H-006 的预测和 E-001 至 E-006 的代码路径证据。
- Environment:
  - Linux；branch `whalecode-alpha`；reviewed commit `a105dfdee`；review report commit `da8114ed7`。
- Known facts:
  - 常规 Function 轻量 binding、中央 lifecycle schema 和 ActionMap gate 已落地。
  - 对抗性审查使用 fresh internal subagent `019f9033-93b3-7230-8417-17edf8279de7`。
  - 既有 9、93、2 项 Rust 测试通过，但未覆盖六个反例。
- Ruled out:
  - schema 成本方向错误：80.7% 降幅算术成立，缺陷位于生产覆盖和失败语义。
  - ActionMap 自动选节点缺失：当前缺陷不要求 Runtime 做语义决策。
- Fix criteria:
  - H-001 至 H-006 均有修复验证测试。
  - provider response 未完成时没有 Tool 前缀执行。
  - control 机械错误整份 response 零执行。
  - ToolSearch 错误对 Agent 可见且使后续 control 跳过。
  - 延迟 Tool 的 schema 与 Runtime binding 要求一致。
  - TaskSpace 可见 ToolSpec 全覆盖；不支持 native 形态明确隐藏并记录。
  - Standard 同名业务参数原样传递；TaskSpace collision 返回确定性错误而非 panic。
  - Rust、五层合同、observer、构建和 Docker 自然样本通过。
  - fresh closure reviewer 不再发现未闭合 blocking finding。
- Current conclusion: Round 2 closure review 发现三条仍可绕过响应级合同的生产路径：
  build failure 从清单消失、ToolSearch 补充事实插入未完成配对之间、隐藏 native event 仍进入
  added/done 副作用处理。提交 `5897cb8ba` 已用统一 provider declaration 序列修复这些路径，
  并补齐 SSE 零副作用、deferred search -> invoke、Standard 实际 dispatch 与 added/done
  native 拒绝测试。Round 3 又发现 client ToolSearch 缺少 `call_id` 时的 Router 枚举遗漏；
  该 shape 现已进入 `build_failed_unpaired` 并由双输入 SSE 证明整响应零 dispatch。定向 Rust
  已通过；accepted blocker 修复后的 fresh Round 4 closure review 尚待执行，因此 P-001
  保持 open。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Repair checkpoint R-001: 六个 blocking finding 的工程修复

- Time: 2026-07-24 03:28
- Status: implemented-awaiting-runtime-closure
- BF-1:
  - Tool sequence 只在 `response.completed` 分支取得执行所有权；
  - pending Tool 非空时 mailbox 抢占延迟；
  - 未完成响应前缀集成测试证明 side effect 为零。
- BF-2:
  - 整响应 preflight 复用 canonical control parser；
  - malformed control、control binding 和不支持 payload 在任何 dispatch 前拒绝；
  - 错误事实包含 `zero_dispatch=true`、`state_commit=false`、revision、call ids 和 sequence hash。
- BF-3:
  - `ToolCallExecution.succeeded` 与 provider 配对输出分离；
  - ToolSearch 仍使用协议兼容的 completed pairing，同时追加含原错误的
    `ToolSearchFailureV1` 事实；
  - 失败会阻止后续 segment。
- BF-4:
  - ToolSearch 返回的 `LoadableToolSpec` 与初始 prompt 共用 typed binding projection；
  - Standard 搜索结果保持原样。
- BF-5:
  - Function、Namespace、ToolSearch、patch 和 code mode 使用同一投影；
  - LocalShell、native WebSearch、ImageGeneration 和未知 Freeform 在 TaskSpace 明确隐藏并记录；
  - 伪造 Custom/LocalShell payload 仍由整响应 preflight 零执行拒绝。
- BF-6:
  - Standard Router 不提取同名业务字段并原样转发；
  - TaskSpace 保留字段 collision 返回 typed error，不 panic、不覆盖。
- 验证：
  - `codex-tools taskspace`: 12/12；
  - `codex-core taskspace`: 99/99；
  - terminal contract: 2/2；
  - mailbox 完整响应所有权集成测试: 1/1；
  - 五层合同、四项 observer 自测、`cargo check`、locked CLI build: 通过。
- Remaining:
  - Docker 自然样本；
  - fresh closure reviewer；
  - 根据 closure review 更新审查报告并关闭或继续修复 P-001。

## Hypothesis H-001: mailbox 抢占绕过完整 response preflight
- Status: confirmed
- Parent: P-001
- Claim: 当已有 pending Tool call 时，Reasoning/Commentary item 上的 mailbox 抢占会以成功 outcome 退出，并在未收到 `response.completed` 时执行该前缀。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - pending calls 在流式 item done 时累积；完成事件和抢占出口分离。
- Falsifiable predictions:
  - If true: 抢占出口允许 `pending_tool_calls` 非空，循环后 fallback 执行这些 calls。
  - If false: 所有非 Completed 成功出口都保证 pending calls 为空或不执行。
- Diagnostic evidence plan:
  - Prediction or clause under test: 抢占条件与循环后执行条件能否同时成立。
  - Signal: `pending_tool_calls.push`、mailbox `break Ok` 和 fallback executor 的控制流。
  - Capture method: 独立审查加本地逐行代码路径核验；补流式回归测试。
  - Event name or marker:
    - `taskspace.response_tool_execution_deferred_until_completed`
  - Correlation keys:
    - turn id
    - provider response id
  - Differentiates from:
    - provider stream 异常关闭；该路径返回 Err，不执行。
  - Supports if:
    - `break Ok` 后 fallback 在 pending 非空时执行。
  - Refutes if:
    - fallback 只在 Completed 后可达。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留完成边界和延迟抢占事实。
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: pending calls 非空时延迟 mailbox 抢占，并删除非 Completed fallback 执行。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 未完成 response 前缀存在成功执行路径
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `core/src/session/turn.rs:2872-2889,2998-3030,3255-3261`；VS review BF-1
- Prediction or plan link:
  - H-001 抢占和 fallback 可同时成立。
- Matched signal:
  - call 在 2874 累积；2886 `break Ok`；3255 对成功 outcome 的 pending calls 执行。
- Correlation keys:
  - reviewer `019f9033-93b3-7230-8417-17edf8279de7`
- Raw content:
  ```text
  if preempt_for_mailbox_mail && mailbox.has_pending() { break Ok(...) }
  if outcome.is_ok() && !pending_tool_calls.is_empty() { execute_response_tool_sequence(...) }
  ```
- Interpretation: 完整 response 所有权不是 executor 的前置条件。
- Time: 2026-07-24 02:35

## Hypothesis H-002: control 机械错误只在分段执行期间发现
- Status: confirmed
- Parent: P-001
- Claim: control action 解析失败被 manifest 折叠为 `None`，control 携带 binding 也不由 preflight 拒绝，因此早于 control 的 segment 可先执行。
- Layer: sub-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - manifest 与 canonical control parser 没有共享有效性结果。
- Falsifiable predictions:
  - If true: ordinary(active) + malformed control 能通过 response preflight。
  - If false: preflight 在任何执行前返回稳定机械错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: invalid control 是否被纯 preflight 接受。
  - Signal: validator 返回值与 `executed_tool_call_count`。
  - Capture method: 失败优先单元测试和 executor spy。
  - Event name or marker:
    - `tool.response_preflight_rejected`
  - Correlation keys:
    - sequence hash
  - Differentiates from:
    - handler 的 DAG 状态机拒绝；本假设只覆盖参数/保留字段机械错误。
  - Supports if:
    - validator 返回 Ok，错误延迟到 handler。
  - Refutes if:
    - validator 返回零执行错误。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 preflight reason code。
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: preflight 复用 canonical control parser，并拒绝 control binding。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-002: manifest 丢失 control parse failure
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `core/src/tools/sequence_manifest.rs:54-61`、`sequence_preflight.rs:106-167`、`taskspace_binding.rs:16-24`
- Prediction or plan link:
  - H-002 invalid control 延迟发现。
- Matched signal:
  - JSON/action 失败变成 `None`；preflight 未检查 control binding；dispatch validator 才拒绝。
- Correlation keys:
  - reviewer BF-2
- Raw content:
  ```text
  serde_json::from_str(...).ok().and_then(...)
  ```
- Interpretation: 机械错误不能保证整份 response 零执行。
- Time: 2026-07-24 02:35

## Hypothesis H-003: ToolSearch 错误在输出代数中被压成成功
- Status: confirmed
- Parent: P-001
- Claim: ToolSearch handler 错误被转换成 `status=completed` 空结果且丢弃错误文本，sequence 因此继续执行后续 control。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - ToolSearch 特殊输出没有使用 Function 输出的 success 字段。
- Falsifiable predictions:
  - If true: 空 query 错误的 sequence success 为 true，模型可见内容不含原错误。
  - If false: 输出携带失败事实且后续 segment skipped。
- Diagnostic evidence plan:
  - Prediction or clause under test: ToolSearch Err 到 sequence 的状态和文本。
  - Signal: ToolCallExecution success、输出类型、后续 control dispatch。
  - Capture method: handler/executor 回归测试。
  - Event name or marker:
    - `tool_search.failed`
  - Correlation keys:
    - call id
  - Differentiates from:
    - 合法零搜索结果；合法空结果仍应成功。
  - Supports if:
    - Err 与合法空结果生成同一输出。
  - Refutes if:
    - Err 有独立失败状态和原始错误事实。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留错误类别和 call id。
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: ToolCallExecution 独立携带真实 success，并为特殊输出追加原始错误事实。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-003: ToolSearch Err 与成功空结果不可区分
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `core/src/tools/parallel.rs:276-284`、`sequence.rs:285-291`、`handlers/tool_search.rs:66-82`
- Prediction or plan link:
  - H-003 错误被压成成功。
- Matched signal:
  - 空 query 返回 Err；failure response 丢弃 message；sequence 把 completed 判成功。
- Correlation keys:
  - reviewer BF-3
- Raw content:
  ```text
  ToolPayload::ToolSearch => status: "completed", tools: []
  ```
- Interpretation: 这是反馈丢失和状态扭曲，不是 Agent 理解问题。
- Time: 2026-07-24 02:35

## Hypothesis H-004: ToolSearch 延迟 Tool 跳过可见性投影
- Status: confirmed
- Parent: P-001
- Claim: ToolSearch 返回 raw `LoadableToolSpec`，TaskSpace Agent 看不到 binding 字段，但 Runtime 对随后 Function/MCP 调用强制要求 binding。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 初始 prompt visibility 和 ToolSearch output 使用两条 schema 构造路径。
- Falsifiable predictions:
  - If true: TaskSpace search output 的 parameters 无 binding，随后调用缺字段被拒绝。
  - If false: search output 与初始可见 Tool 使用同一投影。
- Diagnostic evidence plan:
  - Prediction or clause under test: 同一 Loadable Tool 在 Standard/TaskSpace 的输出差异。
  - Signal: serialized search output parameters 和后续调用结果。
  - Capture method: schema fixture 加实际 router/handler 测试。
  - Event name or marker:
    - `taskspace.tool_search_schema_projected`
  - Correlation keys:
    - search call id
    - returned tool name
  - Differentiates from:
    - Agent 漏填已可见字段。
  - Supports if:
    - search output 原始 schema 不含 binding。
  - Refutes if:
    - TaskSpace output 明确要求 binding。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留投影数量和 collision 数量。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 对 LoadableToolSpec 复用同一 typed decorator。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-004: 延迟 Tool 输出使用 raw schema
- Related hypotheses:
  - H-004
- Direction: supports
- Type: code-location
- Source: `core/src/tools/tool_search_entry.rs:55-74`、`context.rs:338-351`、`context_tests.rs:384`
- Prediction or plan link:
  - H-004 search output 跳过投影。
- Matched signal:
  - LoadableToolSpec 直接序列化；现有测试明确断言无 binding。
- Correlation keys:
  - reviewer BF-4
- Raw content:
  ```text
  tools.iter().map(serde_json::to_value)
  ```
- Interpretation: provider-visible能力合同和 Runtime 合同矛盾。
- Time: 2026-07-24 02:35

## Hypothesis H-005: 非 Function ToolSpec 绕过统一序列合同
- Status: confirmed
- Parent: P-001
- Claim: LocalShell、native WebSearch、ImageGeneration 和未知 Freeform Tool 不可添加 binding，仍可在 TaskSpace 可见或产生 provider-native事件。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - ToolSpec enum 包含无 parameters 的 native variants，decorator fallback 原样返回。
- Falsifiable predictions:
  - If true: exhaustive visibility 测试发现 TaskSpace 可见但无 binding 的普通 Tool。
  - If false: 每个可见普通 Tool 都可进入同一 preflight，或被明确排除。
- Diagnostic evidence plan:
  - Prediction or clause under test: ToolSpec 变体覆盖表。
  - Signal: variant -> TaskSpace projection outcome。
  - Capture method: exhaustive unit test和生产 visibility test。
  - Event name or marker:
    - `taskspace.provider_tool_hidden_unsequenced`
  - Correlation keys:
    - tool name
    - tool kind
  - Differentiates from:
    - Function provider-specific WebSearch；它可以正常装饰。
  - Supports if:
    - native variant 原样留在 TaskSpace。
  - Refutes if:
    - 可投影 Tool 被投影，不可投影 Tool 确定性隐藏并记录。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留隐藏原因日志。
- Evidence gate: satisfied
- Related evidence:
  - E-005
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 采用混合策略：Function 等价形态投影；无法进入 client preflight 的 native 形态在 TaskSpace 明确隐藏。
- Blocker:
  - none；用户“修复”授权接受前述推荐方向。
- Close reason:
  - not closed

## Evidence E-005: decorator fallback 保留不可绑定 ToolSpec
- Related hypotheses:
  - H-005
- Direction: supports
- Type: code-location
- Source: `tools/src/tool_spec.rs:22-58`、`tools/src/taskspace_binding.rs:12-41`、`core/src/tools/router.rs:245-271`
- Prediction or plan link:
  - H-005 非 Function Tool 绕过。
- Matched signal:
  - fallback 返回原 spec；LocalShell 创建 call 时 binding 固定 None。
- Correlation keys:
  - reviewer BF-5
- Raw content:
  ```text
  other => other
  ```
- Interpretation: “所有普通 Tool 不可绕过”声明不成立。
- Time: 2026-07-24 02:35

## Hypothesis H-006: binding 提取未按 TaskSpace 模式隔离
- Status: confirmed
- Parent: P-001
- Claim: Router 在 Standard 和 TaskSpace 中都移除同名字段，且 TaskSpace schema collision 使用 panic。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - provider visibility 是 mode-scoped，但参数提取不是。
- Falsifiable predictions:
  - If true: Standard 外部 Tool 的业务字段被抽走；TaskSpace prompt build 遇 collision panic。
  - If false: Standard 原样转发；TaskSpace 返回 typed configuration error。
- Diagnostic evidence plan:
  - Prediction or clause under test: 同一 collision schema/arguments 在两种模式的行为。
  - Signal: handler 接收参数和 prompt build 结果。
  - Capture method: Router forwarding test 和 projection collision test。
  - Event name or marker:
    - `taskspace.tool_schema_collision`
  - Correlation keys:
    - tool name
  - Differentiates from:
    - Agent 非法注入隐藏字段；本假设使用 Tool 自己公开声明的业务字段。
  - Supports if:
    - Standard 字段缺失或 TaskSpace panic。
  - Refutes if:
    - Standard 完整且 TaskSpace确定性失败。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 typed collision 日志。
- Evidence gate: satisfied
- Related evidence:
  - E-006
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: mode-aware extraction和 fallible typed projection。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-006: Standard 提取与 TaskSpace panic 共用同一 collision
- Related hypotheses:
  - H-006
- Direction: supports
- Type: code-location
- Source: `core/src/tools/router.rs:311-355`、`tools/src/taskspace_binding.rs:49-55`
- Prediction or plan link:
  - H-006 mode 隔离。
- Matched signal:
  - build_tool_call 无条件提取；decorator 用 `assert!`。
- Correlation keys:
  - reviewer BF-6
- Raw content:
  ```text
  const FIELD: &str = codex_tools::TASKSPACE_BINDING_FIELD;
  assert!(!properties.contains_key(FIELD))
  ```
- Interpretation: Standard 语义被 TaskSpace 保留字段侵入，TaskSpace collision 不可恢复。
- Time: 2026-07-24 02:35

## Hypothesis H-007: build failure 未进入完整 provider response 清单
- Status: closed
- Parent: P-001
- Claim: `build_tool_call` 返回 model-visible 参数错误时，调用只被即时写回，未进入
  response preflight；同一 response 中更早的合法调用仍会执行。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-002
  - H-003
- Rationale:
  - stream item 构建与 response sequence 使用两个不同的失败所有权。
- Falsifiable predictions:
  - If true: 合法 side-effect 前缀 + malformed 后缀在 `response.completed` 后会产生前缀文件。
  - If false: build-failed item 会 poison 整份 response，所有 client calls 零 dispatch。
- Diagnostic evidence plan:
  - Prediction or clause under test: build failure 是否从声明序列消失。
  - Signal: 前缀文件、ProviderToolResponsePreflightV1、executed tool count。
  - Capture method: 真实 SSE 回归测试。
  - Event name or marker:
    - `tool.call_build_failed_queued`
    - `tool.response_provider_declaration_rejected`
  - Correlation keys:
    - call id
    - provider response
  - Differentiates from:
    - stream 未完成；本测试包含确定的 `response.completed`。
  - Supports if:
    - malformed 后缀有反馈但前缀文件存在。
  - Refutes if:
    - 前缀文件不存在，所有 call identity 都有配对失败输出。
  - Instrumentation status: permanent
  - Instrumentation lifecycle:
    - 保留 build failure 和 response rejection 两个机械事实。
- Evidence gate: satisfied
- Related evidence:
  - E-007
  - E-008
- Conclusion: confirmed-fixed
- Repair design readiness: implemented
- Next step: fresh closure review。
- Blocker:
  - none
- Close reason:
  - 提交 `5897cb8ba` 的真实 SSE 回归证明整响应零 dispatch。

## Evidence E-007: Round 2 复现 build failure 清单断层
- Related hypotheses:
  - H-007
- Direction: supports
- Type: independent-review
- Source: fresh reviewer `019f9075-0b18-7593-8e96-b0c1ce457865`
- Prediction or plan link:
  - H-007 build failure 从声明序列消失。
- Matched signal:
  - `handle_output_item_done` 对 `RespondToModel` 立即写回且返回 `tool_call=None`；
    Completed 只执行此前成功构建的 calls。
- Correlation keys:
  - reviewer Round 2 B1
- Raw content:
  ```text
  valid side-effect prefix + malformed build suffix -> prefix remains executable
  ```
- Interpretation: response preflight 的输入不是 provider 实际声明的完整调用集合。
- Time: 2026-07-24 03:55

## Evidence E-008: 统一 declaration 序列关闭 build failure 旁路
- Related hypotheses:
  - H-007
- Direction: refutes
- Type: fix-validation
- Source: `session::tests::malformed_suffix_rejects_the_complete_provider_tool_response`
- Prediction or plan link:
  - H-007 修复后整响应零 dispatch。
- Matched signal:
  - rollout 含 `ProviderToolResponsePreflightV1` 和原始 parse error；
  - 合法前缀目标文件不存在。
- Correlation keys:
  - commit `5897cb8ba`
  - calls `prefix-side-effect`, `malformed-suffix`
- Raw content:
  ```text
  1 passed; prefix file absent; executed_tool_call_count=0
  ```
- Interpretation: build failure 与 Ready call 现在由同一 response-level preflight 所有。
- Time: 2026-07-24 04:25

## Hypothesis H-008: 隐藏 provider-native event 仍进入非 Tool 副作用路径
- Status: closed
- Parent: P-001
- Claim: TaskSpace 虽不暴露 native WebSearch/ImageGeneration schema，但 provider/replay 发出
  added/done event 时，事件仍按非 Tool item 处理，ImageGeneration 可写文件。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-005
- Rationale:
  - schema visibility 与 stream event admission 没有共享机械边界。
- Falsifiable predictions:
  - If true: hidden image event 可在 TaskSpace artifact 目录产生文件。
  - If false: added/done 都在非 Tool handler 前转为无效 declaration，且整 response 零 dispatch。
- Diagnostic evidence plan:
  - Prediction or clause under test: added 与 done 两个入口是否先于 image 落盘被拒绝。
  - Signal: artifact path、declaration rejection fact、去重后的 declared count。
  - Capture method: direct handler 与真实 SSE 两条独立测试。
  - Event name or marker:
    - `taskspace.provider_native_tool_added_rejected`
    - `taskspace.provider_native_tool_rejected`
  - Correlation keys:
    - provider item id
  - Differentiates from:
    - provider 侧已经执行的 server action；本假设只约束 Whale 本地接纳与副作用。
  - Supports if:
    - 本地 artifact 存在或 event 未进入 response failure。
  - Refutes if:
    - artifact 不存在且 response 返回机械失败事实。
  - Instrumentation status: permanent
  - Instrumentation lifecycle:
    - 保留 added/done rejection 事件。
- Evidence gate: satisfied
- Related evidence:
  - E-009
  - E-010
- Conclusion: confirmed-fixed
- Repair design readiness: implemented
- Next step: fresh closure review。
- Blocker:
  - none
- Close reason:
  - direct handler 和 added-only SSE 均证明无本地副作用。

## Evidence E-009: Round 2 复现 native event admission 旁路
- Related hypotheses:
  - H-008
- Direction: supports
- Type: independent-review
- Source: fresh reviewer `019f9075-0b18-7593-8e96-b0c1ce457865`
- Prediction or plan link:
  - H-008 隐藏 Tool 仍可作为 stream event 被接纳。
- Matched signal:
  - Router 返回非 Tool；image 非 Tool handler 保存 result，WebSearch 被记录为外部上下文。
- Correlation keys:
  - reviewer Round 2 B3
- Raw content:
  ```text
  hidden schema != rejected runtime event
  ```
- Interpretation: visibility 不是 runtime admission contract。
- Time: 2026-07-24 03:55

## Evidence E-010: added/done native event 在副作用前拒绝
- Related hypotheses:
  - H-008
- Direction: refutes
- Type: fix-validation
- Source:
  - `session::tests::taskspace_hidden_native_tools_are_rejected_before_local_side_effects`
  - `session::tests::taskspace_rejects_hidden_image_added_event_before_artifact_write`
- Prediction or plan link:
  - H-008 added/done 两个入口统一关闭。
- Matched signal:
  - direct done Web/Image 均成为 `RejectedNative`；
  - added-only SSE 进入 `ProviderToolResponsePreflightV1`；
  - 两条路径 artifact 均不存在。
- Correlation keys:
  - commit `5897cb8ba`
- Raw content:
  ```text
  2 focused tests passed; zero artifact writes
  ```
- Interpretation: TaskSpace 对隐藏 native shape 的 schema 和 runtime 行为已经一致。
- Time: 2026-07-24 04:35

## Evidence E-011: deferred 与 Standard 实际 dispatch 覆盖补齐
- Related hypotheses:
  - H-004
  - H-006
- Direction: refutes
- Type: fix-validation
- Source:
  - `search_tool::taskspace_tool_search_binding_survives_search_and_is_stripped_at_dispatch`
  - `search_tool::standard_dynamic_dispatch_preserves_business_taskspace_binding_field`
- Prediction or plan link:
  - H-004 搜索后真实调用；H-006 Standard handler 实际入参。
- Matched signal:
  - TaskSpace 搜索结果要求 binding，后续 dynamic invocation 成功且业务 handler 看不到 binding；
  - Standard dynamic handler 收到完整 business-owned `taskspace_binding`。
- Correlation keys:
  - commit `5897cb8ba`
- Raw content:
  ```text
  2 integration tests passed
  ```
- Interpretation: schema、Router 与 handler 的两种 mode 行为均由生产入口证明。
- Time: 2026-07-24 04:38

## Hypothesis H-009: client ToolSearch 缺少 call_id 时被当作非 Tool
- Status: closed
- Parent: P-001
- Claim: `execution=client` 且 `call_id=None` 的 ToolSearch 未进入 Ready、BuildFailed
  或无配对失败声明，导致同 response 合法前缀仍可执行。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-007
- Rationale:
  - Router 只匹配 `call_id=Some` 的 client ToolSearch，剩余 ToolSearch 统一返回 `Ok(None)`。
- Falsifiable predictions:
  - If true: 合法 active Tool 前缀 + missing-call-id ToolSearch 会执行前缀。
  - If false: 有无 provider item id 都会产生 `build_failed_unpaired`，整 response 零 dispatch。
- Diagnostic evidence plan:
  - Prediction or clause under test: client ToolSearch identity 缺失是否仍进入 declaration。
  - Signal: side-effect 文件、descriptor status、原始错误文本。
  - Capture method: TaskSpace 真实 SSE，分别覆盖 `id=Some` 和 `id=None`。
  - Event name or marker:
    - `tool.call_build_failed_queued`
    - `tool.response_provider_declaration_rejected`
  - Correlation keys:
    - provider item id
    - provider response
  - Differentiates from:
    - server-executed ToolSearch；只对 `execution=client` 要求 client pairing identity。
  - Supports if:
    - missing-call-id item 不出现在 declaration descriptor。
  - Refutes if:
    - descriptor 为 `build_failed_unpaired` 且前缀文件不存在。
  - Instrumentation status: permanent
  - Instrumentation lifecycle:
    - 复用 build failure 与 response rejection 事件。
- Evidence gate: satisfied
- Related evidence:
  - E-012
  - E-013
- Conclusion: confirmed-fixed
- Repair design readiness: implemented
- Next step: fresh Round 4 closure review。
- Blocker:
  - none
- Close reason:
  - 双输入 SSE 回归证明无 provider id 时同样零 dispatch。

## Evidence E-012: Round 3 发现 client ToolSearch 模式匹配缺口
- Related hypotheses:
  - H-009
- Direction: supports
- Type: independent-review
- Source: fresh reviewer `019f9094-dbcf-7d31-89d6-f658a67ca95a`
- Prediction or plan link:
  - H-009 missing call id 被统一 fallback 吞掉。
- Matched signal:
  - `router.rs` 仅为 `call_id=Some` 构建 call，其他 shape 返回 `Ok(None)`；
    stream 非 Tool 分支不产生 declaration。
- Correlation keys:
  - Round 3 BF-A
- Raw content:
  ```text
  valid FunctionCall prefix + client ToolSearch call_id=None -> ToolSearch disappears
  ```
- Interpretation: ProviderToolDeclaration 架构正确，但 Router 枚举覆盖不完整。
- Time: 2026-07-24 04:45

## Evidence E-013: missing-call-id ToolSearch 进入无配对失败声明
- Related hypotheses:
  - H-009
- Direction: refutes
- Type: fix-validation
- Source: `session::tests::missing_client_tool_search_call_id_rejects_the_complete_response`
- Prediction or plan link:
  - H-009 修复后 `id=Some` 和 `id=None` 均零 dispatch。
- Matched signal:
  - 两轮真实 SSE 均记录原始错误和 `build_failed_unpaired`；
  - `provider_tool_declaration_invalid` 存在；
  - 两个 side-effect 文件均不存在。
- Correlation keys:
  - provider item `provider-search-id`
  - missing provider item id
- Raw content:
  ```text
  1 test / 2 provider-id variants passed
  ```
- Interpretation: 无法配对的 client call 不再从完整 response declaration 消失。
- Time: 2026-07-24 04:55
