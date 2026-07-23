# Problem P-001: TaskSpace 初始化配对采纳造成稳定请求放大
- Status: open
- Created: 2026-07-24 05:03
- Updated: 2026-07-24 05:42
- Objective: 解释并消除 Agent 在明确知道 `initialize_map + ordinary Tool` 配对合同后，仍单独
  生成 `initialize_map` 所造成的请求、未缓存 token 和耗时放大；不得通过 Runtime 语义推断、
  自动补动作或替 Agent 决策解决。
- Symptoms:
  - `single-file-fast-fix` repeat-3 的三个 TaskSpace run 都至少一次单独提交
    `initialize_map`。
  - 第 3 个 run 连续三次收到相同机械拒绝，第四次才生成合法 sibling Tool。
  - TaskSpace 总请求为 Standard 的 1.30x，总 input 为 1.67x，未缓存 input 为 4.23x。
- Expected behavior:
  - Agent 首次响应直接生成 `initialize_map`，并紧邻生成携带
    `taskspace_binding=after_boundary` 的首个普通 Tool。
  - 非法响应仍由 Runtime 整响应零执行拒绝，不自动修复 Agent 动作。
- Actual behavior:
  - Runtime 正确拒绝非法序列，但 Agent 对跨 Tool sibling 合同的首次采纳不稳定。
- Impact:
  - 简单任务中产生 1 至 3 个纯协议恢复请求；
  - 增加累计自然历史、control feedback、未缓存输入和耗时；
  - 第 3 个 run 的 TaskSpace input 达到对应 Standard 的 2.30x。
- Reproduction:
  - Docker scenario `single-file-fast-fix`，Standard/map-request paired repeat-3；
  - run `20260724-045719-715`；
  - 三个 pair 均为 E2、valid、both solved。
- Known facts:
  - L2 明确要求同一 response 中 `initialize_map` 后立即生成普通 Tool。
  - Tool sequence feedback 完整给出实际序列、期望序列、零执行和未提交状态。
  - 第 3 个 run 的 reasoning 两次明确表述要生成两个 Tool，随后仍只生成 control。
  - 第 3 个 run 后续通过相同 ChatCompletions/SSE/Router 路径成功生成并执行
    `taskspace_control + exec_command`，还成功保留四个同响应普通 Tool call。
  - 合法配对最终均可生成，Map 最终全部闭合。
- Ruled out:
  - 反馈缺失或扭曲：三个 pair 的 rejection 都携带完整机械事实。
  - Runtime 部分执行：所有协议拒绝的执行数和 state commit 均为零。
  - Map 初始化状态损坏：合法配对后初始化均成功。
- Fix criteria:
  - 简单和复杂样本中首次初始化配对稳定生成；
  - 不新增 Runtime 语义决策、自动 sibling 或后置惩罚式修复；
  - Standard Tool schema 与行为不受影响；
  - paired repeats 中协议恢复请求和未缓存 token 明显下降。
- Current conclusion: 当前缺口位于 Agent 可生成的 Tool 动作形状与跨 Tool sibling 合同之间。
  L2 和独立 Tool schema 能说明要求，Runtime 能保证底线，却不能让 provider schema
  结构化表达“本次 control call 必须与另一个普通 Tool 同时存在”。轻量 binding 将原本可由
  ordinary Tool carrier 单调用表达的 lifecycle + action，重新拆成两个独立顶层调用，因此形成
  schema-valid、sequence-invalid 的合法生成空间。repeat-3 证明这不是单次偶发。现有 artifact
  未保存原始 provider SSE delta，因而不能以严格取证标准完全排除 adapter 在 preflight 前丢弃
  sibling；但同一 run 的合法双调用和四调用均被完整保留，现有代码也按 provider index 累积全部
  Tool call，证据高置信指向 provider 只生成了 standalone control。是否调整 Tool 形态属于后续
  产品/架构决策，本案暂不实施修复。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 初始化失败反馈被丢失或扭曲
- Status: closed
- Parent: P-001
- Claim: Agent 重复单独初始化，是因为看不到必须携带 sibling ordinary Tool 的准确失败事实。
- Layer: root-cause
- Factor relation: alternative
- Depends on:
  - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: refuted
- Repair design readiness: not-applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - rollout 证明反馈完整，Agent reasoning 也准确复述了配对要求。

## Hypothesis H-002: 跨 Tool sibling 合同缺少结构化生成形状
- Status: confirmed
- Parent: P-001
- Claim: 当前合同分布在 L2、control schema、ordinary Tool schema 与 response preflight；
  单个 provider Tool schema 不能直接要求同一响应必须出现另一个 sibling call。
- Layer: root-cause
- Factor relation: primary
- Depends on:
  - H-001
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: confirmed
- Repair design readiness: requires-product-decision
- Next step: 在不扩散 Runtime 职责的前提下，对比保持双 Tool 合同与单调用复合 Tool 形态的收益、
  普通 Tool 覆盖、patch/MCP 兼容和 schema 成本。
- Blocker:
  - 需要用户确认是否进入 Tool 形态调整。
- Close reason:
  - not closed

## Hypothesis H-003: 聚合成本仅由一个异常 run 拉高
- Status: closed
- Parent: P-001
- Claim: 若去掉第 3 个异常 run，TaskSpace 成本基本与 Standard 持平。
- Layer: contributing-factor
- Factor relation: part_of
- Depends on:
  - H-002
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: refuted
- Repair design readiness: not-applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - 前两个 pair 中 TaskSpace input 仍为 Standard 的 1.33x，未缓存 input 仍为 4.23x。

## Hypothesis H-004: ChatCompletions SSE 或 Router 丢弃了已生成的 sibling
- Status: open
- Parent: P-001
- Claim: Provider 已经生成 ordinary sibling，但 SSE parser、Router visibility 或 sequence
  manifest 构造在 preflight 前将其丢弃。
- Layer: root-cause
- Factor relation: alternative
- Depends on:
  - H-001
- Evidence gate: partial
- Related evidence:
  - E-003
- Conclusion: unlikely
- Repair design readiness: diagnostic-only
- Next step: 若进入修复实验，先增加不记录参数正文的 provider Tool delta 形状日志，记录 response
  内 index、name 和最终 call count，再跑定向 sample 闭合最后观测缺口。
- Blocker:
  - 现有 artifact 不保存原始 provider SSE delta。
- Close reason:
  - not closed

## Hypothesis H-005: 轻量 binding 把连续动作退化为不可结构化的跨 Tool 合同
- Status: confirmed
- Parent: P-001
- Claim: 普通 Tool 只携带 `active | after_boundary` 标记，lifecycle 参数独立放在
  `taskspace_control`；两个独立 schema 都无法要求另一个 sibling 必须存在，导致 Agent 可以生成
  单个 schema 合法的 control，再由后置 response preflight 拒绝。
- Layer: root-cause
- Factor relation: primary
- Depends on:
  - H-002
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: confirmed
- Repair design readiness: requires-product-decision
- Next step: 设计阶段对比单调用 carrier、复合 action Tool 与保持双调用合同三条路径；不得使用
  Runtime 自动插入 ordinary Tool，也不得用语义推断替 Agent 选择动作。
- Blocker:
  - 需要确认连续动作应由哪一种 provider-visible Tool 形状承载。
- Close reason:
  - not closed

## Evidence E-001: 三次初始化配对均发生协议恢复
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports-and-refutes
- Type: runtime-trace
- Source:
  - `pair-001`、`pair-002`、`pair-003` TaskSpace rollout
- Prediction or plan link:
  - 检查反馈是否完整，以及 Agent 是否理解后仍生成错误动作形状。
- Matched signal:
  - 三个 pair 的首次 standalone initialization 均返回
    `taskspace_boundary_requires_after_boundary_action`；
  - `actual_sequence` 只有 control，`expected_sequence` 明确要求 ordinary Tool 和
    `after_boundary`；
  - `executed_tool_call_count=0`、`state_commit=false`；
  - 第 3 个 pair 连续失败三次，reasoning 明确说要生成两个 Tool，第四次才真正生成二者。
- Correlation keys:
  - pair-001: 1 protocol rejection
  - pair-002: 1 protocol rejection
  - pair-003: 3 protocol rejections
- Raw content:
  ```text
  a boundary taskspace_control must be immediately followed by an ordinary Tool
  with taskspace_binding after_boundary
  ```
- Interpretation: 反馈层忠实，稳定成本来自动作生成合同未被稳定采纳。
- Time: 2026-07-24 05:03

## Evidence E-002: repeat-3 成本与异常分布
- Related hypotheses:
  - H-003
- Direction: refutes
- Type: performance-observation
- Source:
  - performance observation for run `20260724-045719-715`
- Prediction or plan link:
  - 区分结构成本与单个异常 run。
- Matched signal:
  - 三组 pair 总计：Standard 292,440 input，TaskSpace 487,947 input；
  - 三组 pair 总计：Standard 7,384 uncached，TaskSpace 31,243 uncached；
  - 去掉第 3 个 pair：input ratio 仍为 1.33x，uncached ratio 仍为 4.23x；
  - 第 3 个 pair 将 request ratio 从前两组的 1.07x 放大到全量的 1.30x。
- Correlation keys:
  - model `deepseek-v4-flash`
  - projection policy `map-request`
  - valid E2 pairs: 3
- Raw content:
  ```text
  standard: 3/3 solved, 23 requests, 292440 input
  taskspace: 3/3 solved, 30 requests, 487947 input
  ```
- Interpretation: 第 3 个 run 明显放大均值，但固定 schema、历史与至少一次协议恢复构成持续成本。
- Time: 2026-07-24 05:03

## Evidence E-003: sibling 在 sequence preflight 前已经缺失
- Related hypotheses:
  - H-004
  - H-005
- Direction: supports-and-weakly-refutes
- Type: runtime-trace-and-code-path
- Source:
  - `pair-003/right/artifacts/rollout.jsonl`
  - `third_party/codex-cli/codex-rs/codex-api/src/sse/chat_completions.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs`
- Prediction or plan link:
  - 区分 provider generation、SSE parsing 与 Runtime preflight/dispatch 丢失。
- Matched signal:
  - 三次失败的 `tool_call_count` 均为 1，`actual_sequence` 只有 `initialize_map`；
  - preflight 对完整 parsed call slice 计数，随后整响应零执行；
  - ChatCompletions parser 按 provider `index` 在 `BTreeMap` 中累积并在 finish 时逐个输出；
  - 同一 rollout 第四次成功保留 control + ordinary sibling，后续还保留四个普通调用。
- Correlation keys:
  - pair-003/right
  - request 1-4
- Raw content:
  ```text
  failed attempts: tool_call_count=1, executed_tool_call_count=0
  successful attempt: taskspace_control initialize_map -> exec_command after_boundary
  ```
- Interpretation: Runtime preflight、Router 和 dispatch 没有在收到完整双调用后删除 sibling。缺失点在
  preflight 之前；原始 SSE 未落盘使 provider omission 与 parser omission 尚不能严格二分，但现有
  正反样本和 parser 实现高置信支持 provider 仅生成单调用。
- Time: 2026-07-24 05:42

## Evidence E-004: repeat-3 成本由固定开销和恢复回合相乘
- Related hypotheses:
  - H-002
  - H-003
  - H-005
- Direction: supports
- Type: performance-observation
- Source:
  - 三个 pair 的 `provider-wire-trace.jsonl`
  - 三个 TaskSpace rollout
- Prediction or plan link:
  - 解释 1.30x request 如何放大为 1.67x input 和 4.23x uncached input。
- Matched signal:
  - Standard 平均每 request 约 12,715 input，TaskSpace 约 16,265，为 1.28x；
  - 首请求中 TaskSpace Tool section 比 Standard 多约 1,882 estimated tokens，system section
    净多约 386，Map handle/natural history 净多约 133；
  - 三个 TaskSpace run 共出现五次 standalone initialization、一次初始化前 ordinary Tool 和
    一次过早 `finish_map`；
  - TaskSpace request 总数为 Standard 的 1.30x，平均 request input 为 1.28x，两者相乘约为
    总 input 的 1.67x；
  - request-2+ cache hit 为 93.05%，Standard 为 97.26%。错误回合追加新的 call、拒绝结果和
    reasoning，使未缓存尾部持续增长。
- Correlation keys:
  - run `20260724-045719-715`
  - projection policy `map-request`
- Raw content:
  ```text
  request ratio: 30 / 23 = 1.30x
  average input per request ratio: 16265 / 12715 = 1.28x
  total input ratio: 487947 / 292440 = 1.67x
  ```
- Interpretation: 成本不是单一异常 request，也不是缓存实现突然失效。根因是每个 TaskSpace
  request 的固定合同/schema 开销更高，再被协议恢复和 lifecycle 恢复回合放大；缓存仍命中稳定
  前缀，但无法命中每轮新增的错误尾部。
- Time: 2026-07-24 05:42
