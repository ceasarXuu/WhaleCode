# Problem P-001: TaskSpace 初始化配对采纳造成稳定请求放大
- Status: open
- Created: 2026-07-24 05:03
- Updated: 2026-07-24 05:03
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
  结构化表达“本次 control call 必须与另一个普通 Tool 同时存在”。repeat-3 证明这不是单次偶发；
  是否调整 Tool 形态属于后续产品/架构决策，本案暂不实施修复。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
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
