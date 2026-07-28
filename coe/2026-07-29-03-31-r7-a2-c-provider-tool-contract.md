# Problem P-001: A2-C 中 Agent 可见 Tool 名与 Runtime 校验名不一致
- Status: open
- Created: 2026-07-29 03:31
- Updated: 2026-07-29 04:11
- Objective: 证明并消除 provider-visible Tool 名称与 TaskSpace response preflight 名称空间不一致造成的稳定拒绝
- Symptoms:
  - 18/18 TaskSpace run 的首次 `initialize_and_execute` 都因 `exec_command` 与 `shell_command` 不一致被拒绝
  - Agent 随后根据失败反馈改填内部名称并重试，稳定放大 request、token 和耗时
  - 后续 trace 还出现 `patch` / `apply_patch` 等相同类型的名称错位
- Expected behavior:
  - Agent 在 `actions[].tool` 中声明自己实际可见、可调用的 Tool 名称
  - Runtime 只按同一公开名称空间机械核对 sibling Tool 的数量、名称和顺序
  - 合法的首次初始化与真实 Tool 能在同一个 provider response 中提交并执行
- Actual behavior:
  - Agent 调用 provider 暴露的 `exec_command`，Runtime preflight 却将 sibling 记为 `shell_command`
  - 同一个 action 在 Agent 可见合同和 Runtime 校验合同中没有稳定的单一名称
- Impact:
  - R-10 无法关闭，A2-C 所有 TaskSpace arm 都产生确定性的恢复请求
  - 成本比较被协议缺陷显著污染，不能用于候选晋升
  - 失败容易被误判为 Agent 没有遵循 TaskSpace 协议
- Reproduction:
  - 使用 A2-C 冻结 binary 运行 simple/complex、三种 TaskSpace policy、每臂 repeat 3
  - 检查首次 `initialize_and_execute` 的 `ToolSequencePreflightResultV2`
- Environment:
  - Linux / Docker 29.6.2 / branch `whalecode-alpha`
  - source commit `abe2b872b6708e666293d0018ecd3654bf5a65cc`
  - binary SHA-256 `0264141cdd758129f4843b51630329d66f3cc3ef9dafb6d10215d1b7e8b5c93e`
  - model `deepseek-v4-flash`
- Known facts:
  - 24/24 业务验证成功，18/18 TaskSpace Map 最终成功执行 `finish_map`
  - 18/18 TaskSpace run 均出现 `taskspace_action_tool_mismatch`
  - 失败输出明确写出 `actions[0].tool is exec_command, sibling Tool is shell_command`
- Ruled out:
  - 单一 projection policy 缺陷；三种 policy 和两个样本全部复现
  - Agent 完全无法使用 Map；所有 TaskSpace run 最终均闭合 Map
- Fix criteria:
  - provider schema、`actions[].tool` 合同和 response preflight 使用同一公开 Tool identity
  - simple/complex、三种 TaskSpace policy 的首次初始化不再出现名称错位
  - 普通 Tool schema 和执行器保持原生，不引入 TaskSpace 内部参数
  - 定向单元测试、sequence 回归和四臂 live trace 均通过
- Current conclusion: 根因已确认。provider 公开名在 Router 进入 sequence preflight 前被归一化成内部 dispatch 名，preflight 错用内部名校验 Agent 的公开声明
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - 根因证据门已满足，产品修复尚未实施
- Close reason:
  - not closed

## Hypothesis H-001: preflight 使用内部 handler 名核对 Agent 的 provider-visible 名
- Status: confirmed
- Parent: P-001
- Claim: provider 向 Agent 暴露 `exec_command` / `apply_patch`，而 sequence preflight 从内部 Tool payload 或 handler 读取 `shell_command` / `patch`，两个名称空间未经统一便直接比较
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 错误文本两侧恰好分别是公开调用名和 Codex 内部 Tool 类型名
- Falsifiable predictions:
  - If true: provider schema 注册路径和 sequence sibling 解析路径会为同一 Tool 产生不同名字，且无公开 identity canonicalization
  - If false: 两条路径已共享同一 identity，错误来自 Agent 填写了 schema 明确禁止的别名
- Diagnostic evidence plan:
  - Prediction or clause under test: 对照 provider Tool registry、`actions[].tool` schema 和 sequence preflight 的名称来源
  - Signal: 生产代码调用链、schema golden 和定向 preflight test
  - Capture method: 只读追踪名称生成代码，并构造同一公开 Tool 的 response fixture
  - Event name or marker:
    - `taskspace_response_preflight_rejected`
  - Correlation keys:
    - response_id
    - call_id
    - tool_name
  - Differentiates from:
    - H-002：schema 已明确要求内部名称但 Agent 忽略
  - Supports if:
    - 同一 Tool 的 provider `name` 与 preflight sibling `tool` 稳定不同
  - Refutes if:
    - schema 和 preflight 都使用同一公开名称
  - Instrumentation status: existing-observability-sufficient
  - Instrumentation lifecycle:
    - 保留当前事实型 reason code
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: `exec_command` 在 provider registry 中公开，Router 将其归一化为 `shell_command`，preflight 随后直接比较内部 `ToolCall.tool_name`
- Repair design readiness: ready
- Next step: 为 `ToolCall` 保留独立的 provider-visible identity，并让 manifest/preflight/Map reservation 使用该 identity；内部 dispatch name 仅供 Router
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: `actions[].tool` 合同已经明确要求内部名称，但 Agent 未遵循
- Status: refuted
- Parent: P-001
- Claim: TaskSpace Tool schema 或 L2 已明确列出 `shell_command` 等内部名称，Agent 仍错误填入公开名称
- Layer: interaction
- Factor relation: alternative
- Depends on:
  - none
- Rationale:
  - 若合同确实完整，问题可能是模型遵循度而不是 wire 身份错位
- Falsifiable predictions:
  - If true: Agent 可见 schema 中存在可枚举的内部名称及其与 sibling 的对应规则
  - If false: schema 只要求任意字符串，或要求填写 Agent 实际调用的公开 Tool 名
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查 provider 实际发送的 `taskspace_control` schema 与 L2 文本
  - Signal: wire schema、manifest hash、Tool description
  - Capture method: 读取 frozen binary 对应源码和 provider wire section identity
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - tools_hash
    - capability_set_hash
  - Differentiates from:
    - H-001
  - Supports if:
    - 合同逐项提供内部名称且不存在歧义
  - Refutes if:
    - Agent 只能看到公开 Tool 名，内部名称只从拒绝反馈中首次出现
  - Instrumentation status: existing-observability-sufficient
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: L2 和 Tool schema 都要求“matching ordinary sibling Tool 的 exact name”，Agent 只能看到并调用 `exec_command`；内部 `shell_command` 从未作为可调用 Tool 暴露
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - 公开合同没有要求内部名称

## Evidence E-001: 三种 policy 的全部 TaskSpace run 复现名称错位
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `target/r7-five-layer-eval-data/abe2b872b6708e666293d0018ecd3654bf5a65cc/20260729-031439-725`
- Prediction or plan link:
  - P-001 的跨 policy 稳定复现条件
- Matched signal:
  - 18 个 TaskSpace rollout 均含 `taskspace_action_tool_mismatch`
- Correlation keys:
  - source commit `abe2b872b6708e666293d0018ecd3654bf5a65cc`
  - execution root `20260729-031439-725`
- Raw content:
  ```text
rollouts=24
mismatch_runs=18
TaskSpace actions[0].tool is `exec_command`, sibling Tool is `shell_command`
  ```
- Interpretation: 问题不依赖 projection policy 或样本复杂度；该证据证明症状稳定，但尚不能单独裁定名称的权威 owner
- Time: 2026-07-29 03:31

## Evidence E-002: provider registry、Router 和 preflight 使用了两个名称空间
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports H-001 / refutes H-002
- Type: code-location
- Source:
  - `third_party/codex-cli/codex-rs/tools/src/local_tool.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/router.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs`
  - `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- Prediction or plan link:
  - H-001/H-002 的名称 owner 判别
- Matched signal:
  - provider spec 注册 `exec_command`
  - `normalize_native_function_alias` 将其改写为 `shell_command`
  - `match_actions` 使用改写后的 `call.tool_name.display()`
  - Tool schema 要求 Agent 填写 matching sibling 的 exact name
- Correlation keys:
  - public Tool `exec_command`
  - dispatch Tool `shell_command`
- Raw content:
  ```text
provider-visible: exec_command
router dispatch identity: shell_command
preflight actual: call.tool_name.display()
manifest contract: Exact name of the matching ordinary sibling Tool call.
  ```
- Interpretation: Agent 按公开合同填写 `exec_command` 是正确行为；拒绝由 Runtime 跨名称空间比较造成
- Time: 2026-07-29 04:11
