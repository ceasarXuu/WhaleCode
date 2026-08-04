# Problem P-001: Code Mode 嵌套工具结果未进入 Agent 可见输出
- Status: fixed
- Created: 2026-08-05 06:30
- Updated: 2026-08-05 06:39
- Objective: 让 Agent 明确知道嵌套工具返回值默认只存在于当前 JavaScript 中，并正确选择需要送回上下文的结果。
- Symptoms:
  - Agent 三次 `await tools.exec_command(...)` 后没有调用 `text(...)`，对应 `exec` 对 Agent 返回空输出。
  - Agent 因看不到目录和测试结果而重复执行发现与验证动作。
- Expected behavior:
  - `exec` 工具合同明确说明 `await` 与 Agent 可见输出是两个步骤，并给出最短正确示例。
  - Runtime 继续忠实执行 Agent 的输出选择，不自动转发、筛选或改写嵌套结果。
- Actual behavior:
  - 当前描述分别说明嵌套工具会返回值、`text(...)` 会追加输出，但没有明确说明单独 `await` 不会把结果送回 Agent。
- Impact:
  - Code Mode Agent 容易把成功执行误认为没有结果，增加重复请求、token、耗时和预算耗尽风险。
- Reproduction:
  - 在 Function 形态 `exec` 中执行 `await tools.exec_command({ cmd: "pwd" });`，调用成功但 `exec` 没有 Agent 可见文本输出。
- Environment:
  - branch `whalecode-alpha`，commit `58eede42e4ba2b079a8c4f509dea1edc41d85b0a`，`deepseek-v4-flash`，`code_mode_only=true`，`code_mode_exec_function=true`。
- Known facts:
  - WAR-20260805-061947-R8-FUNCTION-EXEC-CONTRACT-FIX-001 的 trace 连续复现三次无 `text(...)` 的空输出。
  - 同一运行后续采用 `const r = await ...; text(r);` 后立即获得目录和 pytest 结果。
  - Runtime 的嵌套工具回调把结果 resolve 回 JavaScript；`text_callback` 才发送 `ContentItem::InputText`。
- Ruled out:
  - 嵌套工具未执行或返回值在 Runtime 内部丢失。
- Fix criteria:
  - 工具描述明确声明单独 `await` 不产生 Agent 可见输出，并提供 `const result = await ...; text(result);` 示例。
  - 回归测试锁定 Function 与 Freeform 两种 exec 描述中的该合同。
  - 本地测试和构建通过；一次获批真实样本不再因遗漏 `text(...)` 重复读取或重复测试。
- Current conclusion: 工具合同已补全；获批真实样本中 6/6 个有效 exec 都显式追加嵌套结果，空输出重复动作消失，Agent 完整结束。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - E-003
  - E-004
- Close reason: 工具合同、本地回归、真实 trace、公开验证和隐藏 oracle 均满足 fix criteria。

## Hypothesis H-001: 输出合同缺少 await 与 Agent 可见结果的关系
- Status: confirmed
- Parent: P-001
- Claim: 描述没有明确告知 Agent，嵌套工具返回值只留在 JavaScript 内，必须调用输出 helper 才能进入 `exec` 结果。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 当前描述中的“nested tools return”与“text appends”彼此分离，未表达遗漏输出 helper 的直接后果。
- Falsifiable predictions:
  - If true: 无 `text(...)` 时嵌套调用成功但 Agent 收到空输出；加入 `text(...)` 后同类结果可见。
  - If false: 无论是否调用 `text(...)`，Agent 都应收到相同的嵌套结果。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对比同一 trace 内有无 `text(...)` 的 `exec_command` 调用结果。
  - Signal: rollout 中的 exec source、嵌套调用结果和 Agent 可见输出。
  - Capture method: 读取已结算的单样本 trace 与代码回调路径。
  - Event name or marker:
    - `nested_result_visibility_contract_implicit`
  - Correlation keys:
    - `WAR-20260805-061947-R8-FUNCTION-EXEC-CONTRACT-FIX-001`
  - Differentiates from:
    - H-002 Runtime 丢失嵌套结果
  - Supports if:
    - 无 `text(...)` 的调用为空，而同一运行中显式 `text(...)` 能返回真实结果。
  - Refutes if:
    - 显式 `text(...)` 仍无法返回结果，或 Runtime 已自动输出所有嵌套结果。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: trace 行为和 Runtime 实现同时支持该机制，根因确认。
- Repair design readiness: ready; user explicitly authorized repair and one rerun
- Next step: 补全工具合同并验证。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Runtime 丢失了嵌套工具返回值
- Status: refuted
- Parent: P-001
- Claim: 嵌套工具结果在执行或回传过程中被 Runtime 丢失，所以 Agent 无法读取。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - Agent 可见空输出也可能由执行链路丢值导致，需要与合同缺失区分。
- Falsifiable predictions:
  - If true: 即使脚本调用 `text(result)`，真实结果仍不可见。
  - If false: `text(result)` 能稳定输出同一嵌套工具的返回值。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查显式输出 helper 是否能送回目录和 pytest 内容。
  - Signal: 同一 rollout 后续 `text(...)` 调用的 Agent 可见结果。
  - Capture method: 读取已结算实验 action path 和 Runtime callback 实现。
  - Event name or marker:
    - `ContentItem::InputText`
  - Correlation keys:
    - `WAR-20260805-061947-R8-FUNCTION-EXEC-CONTRACT-FIX-001`
  - Differentiates from:
    - H-001 合同缺失
  - Supports if:
    - 显式输出仍为空。
  - Refutes if:
    - 显式输出后结果完整可见。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: 显式 `text(...)` 后目录与测试结果均可见，Runtime 丢值假设被排除。
- Repair design readiness: not applicable
- Next step: closed
- Blocker:
  - none
- Close reason:
  - refuted by E-001 and E-002

## Evidence E-001: 同一真实运行中显式输出决定结果可见性
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports H-001; refutes H-002
- Type: reproduction
- Source: `benchmarks/taskspace/r8/evidence/WAR-20260805-061947-R8-FUNCTION-EXEC-CONTRACT-FIX-001.json`
- Prediction or plan link:
  - H-001/H-002 对有无 `text(...)` 的对照预测
- Matched signal:
  - 三次无 `text(...)` 返回空输出；后续使用 `text(...)` 后获得目录、`2 failed / 1 passed` 和 `3 passed`。
- Correlation keys:
  - `WAR-20260805-061947-R8-FUNCTION-EXEC-CONTRACT-FIX-001`
- Raw content:
  ```text
  模型三次 await 嵌套工具但未调用 text，exec 对模型返回空输出，直接造成重复发现和重复测试。
  ```
- Interpretation: 嵌套结果存在且可被脚本访问，但是否进入 Agent 上下文取决于显式输出 helper。
- Time: 2026-08-05 06:23

## Evidence E-002: Runtime 将工具返回值与 Agent 可见输出分开处理
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports H-001; refutes H-002
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/code-mode/src/runtime/callbacks.rs`
- Prediction or plan link:
  - H-001/H-002 的 Runtime 数据流预测
- Matched signal:
  - `tool_callback` 创建 Promise 并发送 ToolCall；`text_callback` 单独发送 `RuntimeEvent::ContentItem(InputText)`。
- Correlation keys:
  - commit `58eede42e4ba2b079a8c4f509dea1edc41d85b0a`
- Raw content:
  ```text
  tool_callback -> RuntimeEvent::ToolCall
  text_callback -> RuntimeEvent::ContentItem(FunctionCallOutputContentItem::InputText)
  ```
- Interpretation: Runtime 的职责边界是把结果返回给脚本，并只转发 Agent 显式追加的内容；现有行为符合设计，缺口位于公开合同。
- Time: 2026-08-05 06:30

## Hypothesis H-003: 补全合同可消除空输出导致的重复动作
- Status: confirmed
- Parent: P-001
- Claim: 在 exec 描述中直接说明 await 不自动输出并给出显式追加示例后，Agent 会把所需嵌套结果送回上下文，不再因空输出重复操作。
- Layer: fix-validation
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 修复只补全 Agent 可见合同，不改变 Runtime 执行或结果选择语义。
- Falsifiable predictions:
  - If true: 有效嵌套调用均使用输出 helper，目录发现和 pytest 不因空输出重复。
  - If false: 仍会出现无输出 helper 的嵌套调用，并发生相同重复动作。
- Diagnostic evidence plan:
  - Prediction or clause under test: 单次获批真实样本中所有有效 exec 的 source 和 output。
  - Signal: `text(...)` 使用、空输出数量、重复发现/测试数量和最终生命周期。
  - Capture method: Docker benchmark rollout、provider boundary、公开 validator 和隐藏 oracle。
  - Event name or marker:
    - `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002`
  - Correlation keys:
    - run `20260805-063809-645`
  - Differentiates from:
    - Runtime 自动转发嵌套结果
  - Supports if:
    - 需要反馈的嵌套调用全部显式输出且没有空输出重复。
  - Refutes if:
    - 任何需要反馈的调用仍因未显式输出而重复。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 rollout 与 provider boundary 作为运行证据。
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: 6/6 个有效 exec 显式输出，重复发现和重复 pytest 均为 0，完整生命周期成功。
- Repair design readiness: completed
- Next step: closed
- Blocker:
  - none
- Close reason:
  - confirmed by E-003 and E-004

## Evidence E-003: 本地合同回归和构建通过
- Related hypotheses:
  - H-003
- Direction: supports
- Type: test
- Source: `cargo test -p codex-code-mode --locked`; `cargo test -p codex-tools --lib --locked`; Code Mode core integration；CLI build；cache regression gate
- Prediction or plan link:
  - H-003 的工具合同存在性与回归预测
- Matched signal:
  - 24 个 code-mode 测试通过；149 个 tools 测试通过、1 ignored；core integration 通过；CLI 构建通过；免费 final-wire 门禁通过。
- Correlation keys:
  - commit `49213445b5251ff6b574f8f9c1fb943c6d1c87e6`
- Raw content:
  ```text
  cache regression gate: PASS 88c27e2e1ecc32c492f6652ca6d77426bf7f3a258ddfd23a8945a2c0df35b0f7
  ```
- Interpretation: 两种 exec carrier 均包含结果可见性合同，相关工具和集成回归未受损。
- Time: 2026-08-05 06:34

## Evidence E-004: 真实样本不再遗漏嵌套结果
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `benchmarks/taskspace/r8/evidence/WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002.json`
- Prediction or plan link:
  - H-003 的真实行为预测
- Matched signal:
  - 6/6 个有效 exec 显式调用 `text(...)`；空输出、重复发现、重复 pytest 均为 0；最终答复在第 8 请求完成。
- Correlation keys:
  - `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002`
  - run `20260805-063809-645`
- Raw content:
  ```text
  valid_function_exec_calls_with_explicit_output=6
  valid_function_exec_calls_without_explicit_output=0
  agent_completion_status=complete
  public validation: 3 passed
  hidden oracle passed
  ```
- Interpretation: 原始症状在修复后的获批真实样本中消失，且没有通过 Runtime 自动处理结果来改变职责边界。
- Time: 2026-08-05 06:39
