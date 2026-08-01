# Problem P-001: TaskSpace 初始化配对采纳造成稳定请求放大
- Status: closed
- Created: 2026-07-24 05:03
- Updated: 2026-07-24 07:12
- Objective: 解释并消除独立 `initialize_map` 与普通 Tool 的跨 Tool sibling 合同，使初始化和
  第一个真实动作由同一个 Agent 声明的普通 Tool 调用承载；不得通过 Runtime 语义推断、自动补动作
  或替 Agent 决策解决。
- Symptoms:
  - `single-file-fast-fix` repeat-3 的三个 TaskSpace run 都至少一次单独提交
    `initialize_map`。
  - 第 3 个 run 连续三次收到相同机械拒绝，第四次才生成合法 sibling Tool。
  - TaskSpace 总请求为 Standard 的 1.30x，总 input 为 1.67x，未缓存 input 为 4.23x。
- Expected behavior:
  - Agent 的第一个真实普通 Tool 在 `taskspace_binding` 中携带完整 `initialize_map` 对象；
  - Runtime 先提交 Agent 明确声明的图，再执行同一个普通 Tool；
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
  - 简单和复杂样本中初始化与第一个真实动作由同一个 ordinary Tool carrier 表达；
  - provider-visible `taskspace_control` 不再提供独立初始化形状；
  - 不新增 Runtime 语义决策、自动 sibling 或后置惩罚式修复；
  - Standard Tool schema 与行为不受影响；
  - paired repeats 中记录首请求采用、机械拒绝、恢复请求与固定 schema 成本，不用均值波动替代结构验收。
- Current conclusion: 根因是跨 Tool sibling 合同无法由任一单独 Tool schema 表达。修复把初始化
  收敛为第一个普通 Tool 的结构化 binding 分支，并把 `initialize_map` 从中央 control schema 删除。
  首版 `string | object` binding 又形成短分支偏置，18/18 次都先选择 `active`；最终统一为
  `action` 判别对象后，正式 repeat-3 中 18/18 个 TaskSpace run 都通过 ordinary Tool carrier
  完成初始化，独立 control 初始化为 0。16/18 首请求选择初始化、15/18 首请求提交成功；其余运行
  收到机械拒绝后在第二请求恢复。Runtime 没有生成、补全或修改 Agent 图和动作。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - E-005
  - E-006
  - E-007
- Close reason:
  - 独立初始化的跨 Tool 结构缺口已删除；18/18 个正式 TaskSpace run 最终由普通 Tool carrier
    初始化，直接 `taskspace_control initialize_map` 为 0。

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
- Status: closed
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
- Repair design readiness: implemented
- Next step: 单独跟踪普通 Tool schema 固定成本，不恢复跨 Tool sibling。
- Blocker:
  - none
- Close reason:
  - 初始化 lifecycle 与首个真实动作已由同一个 ordinary Tool carrier 结构化表达。

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
- Status: closed
- Parent: P-001
- Claim: Provider 已经生成 ordinary sibling，但 SSE parser、Router visibility 或 sequence
  manifest 构造在 preflight 前将其丢弃。
- Layer: root-cause
- Factor relation: alternative
- Depends on:
  - H-001
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: refuted-for-current-path
- Repair design readiness: diagnostic-only
- Next step: none
- Blocker:
  - none
- Close reason:
  - 修复不依赖 sibling；同一普通 Tool carrier 在正式运行中被 Router 完整解析、提交并执行，
    原缺口消失后不再存在可丢失的初始化 sibling。

## Hypothesis H-005: 轻量 binding 把连续动作退化为不可结构化的跨 Tool 合同
- Status: closed
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
- Repair design readiness: implemented
- Next step: 后续 `bind_node` 与 `complete_then_continue` 仍保留已验证的 control + ordinary Tool
  连续动作合同；本案只修复初始化。
- Blocker:
  - none
- Close reason:
  - 初始化改为单 ordinary Tool carrier；旧独立 control 初始化分支和兼容 parser 已删除。

## Hypothesis H-006: 混合标量和对象 binding 诱导 Agent 选择错误短分支
- Status: closed
- Parent: P-001
- Claim: `active | after_boundary` 字符串与完整 `initialize_map` 对象组成的异形联合，使模型在空 Map
  时稳定选择更短的 `active`，即使 L2 与 bootstrap projection 已明确说明需要初始化。
- Layer: root-cause
- Factor relation: contributing-factor
- Depends on:
  - H-002
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-007
- Conclusion: confirmed-and-fixed
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - 三个分支统一为固定 `action` 判别对象；不动态切 schema、不切 `tool_choice`。

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

## Evidence E-005: 初始化改为普通 Tool 单调用载体
- Related hypotheses:
  - H-002
  - H-005
- Direction: supports
- Type: implementation-and-contract-test
- Source:
  - commit `227987322fe125f84a48de5ca947877be27a2ddf`
  - `third_party/codex-cli/codex-rs/tools/src/taskspace_binding.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/taskspace_initialization.rs`
- Prediction or plan link:
  - 删除无法结构化表达的初始化 sibling，保持 Runtime 只执行 Agent 明确声明的图和普通动作。
- Matched signal:
  - 中央 `taskspace_control` 不再暴露 `initialize_map`；
  - 第一个普通 Tool 可携带初始化对象；
  - 初始化提交成功后同一调用继续走原普通 Tool Router；
  - 初始化失败时普通 Tool 不执行，返回 `TaskSpaceInitializationCarrierResultV1`；
  - Standard Tool schema 与同名业务参数转发保持不变。
- Correlation keys:
  - codex-tools TaskSpace tests `12/12`
  - codex-core TaskSpace tests `104/104`
  - terminal integration `2/2`
  - ToolSearch integration `1/1`
- Raw content:
  ```text
  ordinary Tool(taskspace_binding.initialize_map) -> exact graph commit -> same ordinary Tool dispatch
  ```
- Interpretation: 初始化连续动作现在由一个 provider-visible Tool 调用表达，不再依赖 sibling。
- Time: 2026-07-24 06:00

## Evidence E-006: 混合标量/对象联合导致 18/18 首轮误选 active
- Related hypotheses:
  - H-006
- Direction: supports
- Type: docker-repeat-3
- Source:
  - commit `227987322fe125f84a48de5ca947877be27a2ddf`
  - matrix run `20260724-062008-047`
- Prediction or plan link:
  - 验证单调用载体是否自然被 Agent 首请求采用。
- Matched signal:
  - 24/24 运行有效且业务成功；
  - 18/18 个 TaskSpace run 首请求都选择字符串 `active`；
  - 18/18 收到 `no_task_path` 后在第二请求改用初始化对象；
  - 独立中央 control 初始化已经为 0。
- Correlation keys:
  - Docker image `sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`
  - samples `single-file-fast-fix`, `subscription-billing-repair`
  - repeats per arm/sample `3`
- Raw content:
  ```text
  first request: taskspace_binding="active"
  second request: taskspace_binding={initialize_map object}
  ```
- Interpretation: 执行链修复正确，但异形联合给模型保留了稳定的错误短分支；这不是反馈丢失。
- Time: 2026-07-24 06:32

## Evidence E-007: 判别对象联合 repeat-3
- Related hypotheses:
  - H-002
  - H-005
  - H-006
- Direction: supports
- Type: docker-repeat-3-and-trace-analysis
- Source:
  - commit `b6bf532bf8b6d92d076b30d842e54c4f565fcfee`
  - matrix run `20260724-065244-664`
  - `report.md`, `summary.csv`, `aggregate.csv`, `trace-analysis.json`
- Prediction or plan link:
  - 三个 binding 分支统一为固定 `action` 判别对象后，检查初始化载体、首轮采用和直接 control。
- Matched signal:
  - 24/24 运行有效且业务成功；
  - 18/18 个 TaskSpace run 最终提交且仅保留一张由普通 Tool carrier 初始化的 Map；
  - 初始化 carrier 共 20 次尝试、18 次提交、2 次机械拒绝；
  - 首请求选择初始化为 16/18，首请求提交成功为 15/18；
  - 直接 `taskspace_control initialize_map` 为 0；
  - `no_task_path` 从前一版 18 次降为 1 次；
  - 余下三次恢复分别是一次 `active`、一次 binding 声明非法、一次初始化图不合法，均在第二请求恢复。
- Correlation keys:
  - Docker image `sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`
  - whale SHA `4b14533fce352665eec1d33a6631479bf73b49dfac6fff29b4fffde7304ac14b`
  - samples `single-file-fast-fix`, `subscription-billing-repair`
  - repeats per arm/sample `3`
- Raw content:
  ```text
  initialization carrier committed: 18/20
  first request initialize attempt/commit: 16/15
  direct control initialize: 0
  no_task_path: 1
  ```
- Interpretation: 独立初始化回归已关闭。首请求并非绝对 18/18，但剩余失败是 Agent 提交的机械非法
  参数，Runtime 忠实拒绝并允许下一请求纠正；不应为追求 18/18 引入动态 schema、`tool_choice`
  切换或 Runtime 自动建图。
- Time: 2026-07-24 07:12
