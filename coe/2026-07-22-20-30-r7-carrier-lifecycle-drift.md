# Problem P-001: R7 action carrier 生命周期与实际工作错位
- Status: fixed
- Created: 2026-07-22 20:30
- Updated: 2026-07-22 21:55
- Objective: 解释 FLA-3.5 后 Agent 偶发过早终结以及更频繁的节点推进滞后，区分上下文丢失、Tool 能力缺位、Runtime 顺序错误与 Agent/Tool 合同结构问题。
- Symptoms:
  - `single-file-fast-fix` 原始运行在 `fix` 节点过早调用最终 `complete_then_end`，被 Runtime 拒绝后纠正。
  - 同构建重复三次未再次出现过早最终终结，但两次在 `explore` 下完成 Patch 后错误尝试直接进入 `verify`。
- Expected behavior:
  - 当探索结束、修复开始时，修复动作自身携带 `complete_then_continue(explore -> fix)`；验证动作携带 `complete_then_continue(fix -> verify)`；最后从 `verify` 终结。
- Actual behavior:
  - 四次运行只有一次完全按预期推进；三次 Patch 未携带 transition，导致 Agent 的实际工作阶段领先 canonical Map 一个节点。
- Impact:
  - 产生无效 transition、重读 Map、重复 pytest、占位命令和偶发过早终结；三次重复运行分别使用 13、6、11 个 provider request。
- Reproduction:
  - 使用提交 `53a55f7f6` 构建并 attestation，通过 Docker、`deepseek-v4-flash`、`map-request` 运行 `single-file-fast-fix` 三次 TaskSpace right-only。
- Environment:
  - Linux；分支 `whalecode-alpha`；提交 `53a55f7f6`；TaskSpace contract manifest `1.0.6`；core protocol `taskspace-core-v2.2`。
- Known facts:
  - 三次重复全部业务成功，最终 Map 均为 5 节点、4 条边、0 个开放叶节点。
  - 失败轮 1 在 Patch 前刚执行 `read_map`，仍然遗漏 transition。
  - 成功轮使用同一 `apply_patch` Tool schema 正确携带 transition。
  - 失败轮 reasoning 明确识别“探索完成、进入修复”，但 Tool call 未携带对应字段。
- Ruled out:
  - 不是 Map projection 或控制反馈丢失。
  - 不是 `apply_patch` 缺少 transition 能力。
  - 不是 Runtime 在 Tool 之后才执行 carried transition。
- Fix criteria:
  - Tool 合同让每个普通动作显式声明“继续当前 binding”或“先转换到 successor”，且不要求 Runtime 推断动作语义；真实重复样本不再出现实际工作领先 Map 的静默漂移。
- Current conclusion: 已以必填 `taskspace_action` 判别联合消除遗漏歧义；`continue_current` 只机械核对 revision 与 binding，生命周期动作继续由 Agent 显式选择。装饰边界已从共享 registry 移到 TaskSpace provider 投影，Standard 保持原始 schema；未 dispatch 的拒绝只返回一份事实。三次 TaskSpace 重复均在 Patch 和测试动作边界同步推进，未再出现实际工作领先 Map 的静默漂移。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
- Resolution basis:
  - H-003、H-004、H-005。
  - E-009、E-010、E-012、E-013。
- Close reason:
  - 根因修复已通过定向测试和三组 Docker 配对回归验证。

# Problem P-002: verify 运行时 Agent 误选只关闭 Ready Finish 的动作
- Status: fixed
- Created: 2026-07-22 21:30
- Updated: 2026-07-23 02:20
- Objective: 解释生命周期已同步时，Agent 为何仍在最后一个 Work 节点 Running 状态下选择只适用于 Finish Ready 的 `finish_end`。
- Symptoms:
  - 三次修复后 TaskSpace 运行中两次先调用 `finish_end`，收到 `finish_not_ready` 后改用 `complete_then_end`。
- Expected behavior:
  - 最后一个 Work 节点仍 Running 时直接调用 `complete_then_end(current_node_id=verify)`。
- Actual behavior:
  - 旧合同观察合计 4/5 先误选 `finish_end`；首次修复改名并补 L2 决策规则后，3/3 仍先选择
    `close_ready_finish`，没有一轮首选正确终态动作。
  - 第二次修复后 5 次 TaskSpace 运行未再选择 Ready-Finish 专用动作，但 1 次在仍有 `verify` Pending 时
    过早选择 `complete_active_work_then_end`，另 1 次绕过终态 Tool 直接输出 final。
- Impact:
  - 产生 1 次或 2 次额外 provider request；不破坏 Map，Runtime 拒绝事实准确。
- Known facts:
  - 五次 Patch、测试和 revision 均全程同步，H-003 不再复现。
  - `finish_end` 与 `complete_then_end` 的 schema 描述分别准确陈述适用前置状态。
- Ruled out:
  - 不是 carrier 字段遗漏造成的 Map 滞后。
  - 不是 Runtime 接受了非法终态。
- Fix criteria:
  - 在不引入 Runtime 语义判断、动态 schema 或缓存破坏的前提下，使 Agent 稳定选择与当前机械状态匹配的终态动作。
- Current conclusion: 根因是 Agent 可见的终态 Tool 合同反复暴露了两个互斥前态分支；模型会先按业务完成意图
  选择 Ready-Finish，再生成与该分支内部自洽但不真实的状态断言。现已将公共合同收敛为一个无前态分支的
  `finish_map`：Agent 只选择终态入口节点并提供总结，状态机依据该节点的规范角色和状态执行确定性事务及硬校验。
  三次 Docker 配对复验中，Agent 均首次选择 `verify`，规范结果均为 `terminal_node_role=work`，没有终态拒绝、
  重试或解析错误。
- Related hypotheses:
  - H-006
  - H-007
  - H-008
  - H-009
  - H-010
  - H-011
- Resolution basis:
  - H-006、H-007、H-008、H-009、H-010、H-011。
  - E-011、E-014、E-015、E-016。
  - E-017、E-018、E-019、E-020。
- Close reason:
  - 分支无关的单一终态合同已通过定向测试、子 Agent 终态路径测试和三组 Docker 配对回归验证。

## Hypothesis H-004: carrier schema 按 Collab 开关装饰导致 Standard 污染
- Status: confirmed
- Parent: P-001
- Claim: 生产 Tool registry 以 `collab_tools` 作为 carrier 装饰条件；该开关在 Standard 也启用。可选 carrier
  时代只产生隐性 schema 成本，改为必填后则使 Standard 普通 Tool 也被要求提交 TaskSpace action，而 Session 的
  canonical Map 仍处于 Standard 模式，因而初始化永远无法取得 TaskSpace event source。
- Layer: regression-root-cause
- Factor relation: single
- Depends on:
  - H-003 repair
- Rationale:
  - 失败 trace 使用 Standard base、无 TaskSpace protocol/Map event，却暴露必填 `taskspace_action`。
- Falsifiable predictions:
  - If true: 失败物理 right 轮的 logical mode 为 standard，provider identity 为 Standard，但 Tool schema 要求
    `taskspace_action`；所有 history 以普通 response item 持久化，`initialize_map` 无 canonical event source。
  - If false: 失败轮实际为 TaskSpace，或 Standard Tool schema 不含 carrier。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比对 logical mode、base identity、Map event storage 与 Tool call contract。
  - Signal: `logical-mode-map.json`、`provider-wire-trace.jsonl`、`rollout.jsonl` 和 registry 装饰条件。
  - Capture method: 关联 repeat 2 的 physical right 运行并静态检查 `tool_registry_plan.rs`。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - repeat 2
    - session `019f89ee-478e-7113-b1fb-c53076252ddc`
  - Differentiates from:
    - TaskSpace 初始化 source 记录竞态
    - Agent 初始化参数错误
  - Supports if:
    - Standard profile 与 required TaskSpace carrier 同时出现。
  - Refutes if:
    - 失败轮为 TaskSpace 或 schema 未被装饰。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 保留 provider wire identity 与 logical-mode map。
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-007
- Conclusion: repeat 2 明确为 Standard，却获得 TaskSpace carrier schema；代码中的装饰条件是
  `config.collab_tools`。这完整解释了为何 history 不进入 Map event store、合法初始化仍缺 source，并排除了
  TaskSpace 状态机自身损坏。
- Repair design readiness: implemented and verified
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed and verified by E-010

## Hypothesis H-005: action 拒绝反馈被 carrier 重复包装
- Status: confirmed
- Parent: P-001
- Claim: action 拒绝路径先把失败 JSON 作为普通 Tool failure body，再把同一个 JSON 放进
  `TaskSpaceCarrierResultV2.action_result` 前置，造成一次失败在 Agent 上下文中出现两遍。
- Layer: feedback-regression
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - trace 中每个拒绝 output 均由相同 JSON 的 outer 与 raw body 两份组成。
- Falsifiable predictions:
  - If true: `tool_dispatched=false` 时输出同时包含 outer action_result 与同值 body。
  - If false: 两份内容分别表达 action 和真实工具的独立结果。
- Diagnostic evidence plan:
  - Prediction or clause under test: 未 dispatch 时是否存在真实工具结果。
  - Signal: rejection output 和 `ToolCallRuntime::invalid_call_response`/`wrap_carrier_response` 调用顺序。
  - Capture method: 对照 trace 与执行代码。
  - Event name or marker:
    - `taskspace.carrier_action_rejected`
  - Correlation keys:
    - rejected call_id
  - Differentiates from:
    - 合法的 transition 事实加普通 Tool 输出。
  - Supports if:
    - 未 dispatch 却出现两份同一失败。
  - Refutes if:
    - 第二份来自已执行普通 Tool。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 保留单一结构化拒绝和 rejection log。
- Evidence gate: satisfied
- Related evidence:
  - E-008
- Conclusion: 代码和 trace 均证明是同一 action failure 的重复表达，不是两个独立事实。
- Repair design readiness: implemented and verified
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed and verified by E-012

## Hypothesis H-006: 两个终态 action 的选择合同仍存在显著性歧义
- Status: confirmed
- Parent: P-002
- Claim: L4 在所有状态下同时暴露两个前置状态互斥、但名称语义重叠的终态 action；L2 又没有给出从当前机械状态到两者的明确选择规则。Agent 因而容易把业务语义上的“任务完成”映射为名字更像通用结束动作、参数也更少的 `finish_end`，即使最后一个 Work 仍为 Running。
- Layer: tool-contract-selection
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 错误发生在完整同步的 verify 节点，且 2/3 都是同一方向的误选；Runtime 返回准确状态失败后 Agent 能立即纠正。
- Falsifiable predictions:
  - If true: 更多同构运行会在状态完整、无漂移时继续偏向 `finish_end`，且错误集中在最终 Work Running 到终态的边界。
  - If false: 重复运行不再出现该偏向，或错误由缺失/扭曲的当前状态事实解释。
- Diagnostic evidence plan:
  - Prediction or clause under test: 区分随机 Agent 失误与 Tool action 可辨识性问题。
  - Signal: 终态请求前可见状态、reasoning、完整 Tool schema、所选 action 和拒绝后的纠正路径。
  - Capture method: 先静态对比 L2/L4 终态文本及 provider-visible 顺序，再运行足够重复的复杂样本；不先改代码。
  - Event name or marker:
    - `taskspace.control_rejected`
  - Correlation keys:
    - session id
    - canonical revision
    - current node id/status
  - Differentiates from:
    - carrier 生命周期漂移
    - Map projection 状态丢失
  - Supports if:
    - 完整状态下持续出现同向误选，并在准确拒绝后立刻纠正。
  - Refutes if:
    - 误选与状态不可见或 Map 漂移严格相关，或扩展重复中不形成偏向。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 保留 provider trace 和 control rejection 日志。
- Evidence gate: satisfied
- Related evidence:
  - E-011
  - E-014
- Conclusion: 五次相同终态中四次在完整 transition 反馈后误选 `finish_end`，一次正确选择 `complete_then_end`。错误前 reasoning 均把“测试通过”直接表述为“关闭任务”，说明 Agent 依据业务完成语义选择 action；拒绝后又能准确识别 Work 仍 Running 并纠正，排除了状态事实被扭曲或 Runtime 状态错误。静态 schema 始终同时暴露两个互斥动作，`finish_end` 缺少 `current_node_id` 且名称像通用结束动作；正确动作虽然排在它之前，但 L2 没有终态决策规则。结构根因是终态 Tool 合同的可辨识性不足，而不是 Agent 不知道测试结果或 Map 链路丢失。
- Repair design readiness: first repair attempted; insufficient and superseded by H-007
- Next step: 使用 E-015 验证的更深层 discriminator 假设继续收敛静态 Tool 合同。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-007: branch 描述无法抵消顶层 action 名的意图匹配
- Status: confirmed
- Parent: P-002
- Claim: 即使 L2 与 branch description 已准确给出状态决策，`close_ready_finish` 仍在顶层 discriminator 中把
  “close”与“finish”作为最显著词暴露；Agent 在测试通过后按业务意图匹配该名称，而没有先核对 Ready/Running。
  只改说明文字不能稳定修复，互斥状态必须进入 action 名和必填参数结构。
- Layer: tool-contract-discriminator
- Factor relation: single
- Depends on:
  - H-006 first repair
- Rationale:
  - v2.4 三次运行都完整暴露新 L2 和新 schema，三次仍首先生成 `close_ready_finish`。
- Falsifiable predictions:
  - If true: 误选前 reasoning 只表达业务完成/关闭意图，不表达 Finish 已 Ready；准确拒绝或任意解析失败后，Agent
    能从已有上下文恢复 Running 状态并改用 `complete_then_end`。
  - If false: 新协议未进入上下文、branch 描述未进入实际 schema，或误选前状态反馈缺失。
- Diagnostic evidence plan:
  - Prediction or clause under test: 区分说明未送达与 discriminator 显著性仍错误。
  - Signal: provider-visible L2、验证 Tool feedback、终态 reasoning/call、首次失败内容与下一次纠正。
  - Capture method: 读取提交 `a0fd9e80a` 的三次 Docker 配对 rollout。
  - Event name or marker:
    - `taskspace.control_rejected`
    - `TASKSPACE_INVALID_ARGUMENT`
  - Correlation keys:
    - repeat 1-3
    - canonical revision 4
    - current node `verify`
  - Differentiates from:
    - L2 未注入
    - Tool feedback丢失
    - Runtime 状态错误
  - Supports if:
    - 新文本完整可见仍 3/3 同向误选，且失败后无需新 Map 信息即可纠正。
  - Refutes if:
    - 新文本或 Running 反馈缺失，或误选不形成同向偏差。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 保留 action 计数、参数解析失败和终态 operation 日志。
- Evidence gate: satisfied
- Related evidence:
  - E-015
- Conclusion: 三次都在 revision 4、`verify` Running 且测试成功反馈完整时先表达“close the task”，随后选择
  `close_ready_finish`。repeat 1/2 收到 `finish_not_ready` 后立即纠正；repeat 3 的首次调用仅因 action 值缺引号
  而解析失败，错误反馈没有提供 Finish 状态，但 Agent 下一轮仍自行指出 verify Running 并正确调用
  `complete_then_end`。因此状态事实一直存在，失败的区分点是顶层 action 名和参数结构仍允许按关闭意图走最短路径。
- Repair design readiness: ready
- Next step: 让两个顶层 action 名直接表达“完成 active Work 后结束”与“没有 active Work 时关闭”，并要求后者
  显式提交 `active_work_status=none`、`finish_status=ready`；Runtime 仍只做机械校验。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-008: 普通终态 action 未表达当前 Work 必须是唯一剩余 Work
- Status: confirmed
- Parent: P-002
- Claim: `complete_active_work_then_end` 虽消除了 Ready-Finish 分支歧义，但任何 Running Work 都符合名称中的
  `active_work`。当 Agent 已经在 `fix` 节点执行了测试、同时自己创建的 `verify` 节点仍 Pending 时，业务意图
  已完成会使它过早选择该 action。完整前置状态必须表达“当前 Work 是最后一个未完成 Work”。
- Layer: tool-contract-discriminator
- Factor relation: single
- Depends on:
  - H-007 second repair
- Rationale:
  - 第二次修复后的扩展重复中，Agent 在 revision 3、`fix` Running、`verify` Pending、Finish Pending 时调用
    `complete_active_work_then_end`，收到 `finish_not_ready` 后还尝试破坏依赖边，读取 Map 后才恢复正确路径。
- Falsifiable predictions:
  - If true: action 名与参数只要求 active current node，不要求其他 incomplete Work 为零；错误调用前上下文已经
    包含 `fix -> verify -> finish`，拒绝后的 `read_map` 只重复这一事实。
  - If false: `verify` 不存在/已完成，或调用时状态事实未进入上下文。
- Diagnostic evidence plan:
  - Prediction or clause under test: 区分状态事实缺失与普通终态分支前置状态表达不完整。
  - Signal: 初始化图、revision 3 projection、首次终态调用、拒绝和纠正路径。
  - Capture method: 读取提交 `1111c3f07` 的 TaskSpace-only 扩展重复 rollout。
  - Event name or marker:
    - `taskspace.complete_terminal_rejected`
    - `taskspace.control_rejected`
  - Correlation keys:
    - pair-003 taskspace
    - revision 3
    - current node `fix`
  - Differentiates from:
    - Map 节点或边丢失
    - Ready-Finish 专用动作歧义
    - Runtime 接受非法终态
  - Supports if:
    - 完整图已可见仍过早选择，且 Runtime 以 `finish_not_ready` 忠实拒绝。
  - Refutes if:
    - `verify` 事实不可见或 canonical Map 已无其他 incomplete Work。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 保留终态 action 计数、状态失败分类与 Map read trace。
- Evidence gate: satisfied
- Related evidence:
  - E-016
- Conclusion: 初始化调用明确创建 `explore -> fix -> verify -> finish`。Agent 在 `fix` 下执行测试成功后调用
  `complete_active_work_then_end`，其参数结构无需声明 `verify` 是否仍 Pending；Runtime 准确拒绝且 revision
  不变。随后 `read_map` 显示的只是原有 `verify` Pending 事实，证明不是反馈丢失。根因是普通终态 action 合同只
  表达 active Work，没有表达 last remaining Work。
- Repair design readiness: ready
- Next step: 将普通终态 discriminator 改为 `complete_last_running_work_then_end`，并要求调用方提交
  `other_incomplete_work_status=none` 与 `finish_status=pending`；canonical 状态仍由原事务校验。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-009: 两个并列终态入口允许先选业务动作再补写虚假状态
- Status: confirmed
- Parent: P-002
- Claim: 即使两个终态 action 都要求调用方声明互斥状态，顶层 `anyOf` 仍先向模型暴露两个竞争的业务动作。
  `close_finish_with_no_active_work` 参数更少，且固定枚举允许模型在没有核对 Map 时直接补写 `none/ready`；因此
  状态声明没有成为选择条件，只成为错误动作选定后的配套参数。
- Layer: tool-contract-shape
- Factor relation: single
- Depends on:
  - H-007
  - H-008
- Rationale:
  - 第三轮已将普通动作改为 `complete_last_running_work_then_end`，并要求显式提交“其他未完成 Work 为零”和
    “Finish Pending”。如果问题只是不知道最后 Work 状态，Agent 在 `verify` Running 时应直接选择该动作。
- Falsifiable predictions:
  - If true: 多次运行会在完整可见 `verify` Running 状态下稳定先选参数更短的 Ready-Finish 分支，并同时填入
    与真实状态矛盾的固定枚举；收到 canonical state 拒绝后会立即改用正确动作。
  - If false: 首次选择会随机分布，或错误调用前缺少 active owner/revision，或拒绝后仍无法选择正确动作。
- Diagnostic evidence plan:
  - Prediction or clause under test: 区分状态事实缺失与并列 action 入口的选择竞争。
  - Signal: 每次终态调用的 owner、arguments、canonical revision、拒绝 violation 和下一次终态调用。
  - Capture method: 读取第三轮三次 paired Docker rollout 的 `taskspace_control` function call/output 事件。
  - Event name or marker:
    - `taskspace.complete_terminal_rejected`
    - `taskspace.complete_terminal_committed`
  - Correlation keys:
    - pair-001/right
    - pair-002/left
    - pair-003/right
    - revision 4
    - owner `verify`
  - Differentiates from:
    - projection 或反馈丢失
    - Runtime 接受非法状态
    - 普通终态动作仍缺少 last-Work 前态
  - Supports if:
    - 三次都由 `verify` owner 先提交虚假 `none/ready`，均收到 `finish_not_ready`，随后均正确完成 `verify` 并终结。
  - Refutes if:
    - owner/revision 不完整，或首次调用已使用正确动作，或错误分支被非法提交。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 将终态候选、状态失败和 terminal-state 分支计数保留为长期 benchmark 观测。
- Evidence gate: satisfied
- Related evidence:
  - E-017
- Conclusion: 第三轮三次运行都在 owner=`verify`、revision=4 时先调用
  `close_finish_with_no_active_work(active_work_status=none, finish_status=ready)`，均被 canonical state 以
  `finish_not_ready` 拒绝；下一次终态调用均为正确的 `complete_last_running_work_then_end` 并提交成功。
  这是稳定的合同选择问题，不是上下文或状态机问题。两个状态互斥的终态操作应收敛为一个 Agent 可见动作，
  由等形的显式 `terminal_state` 判别前态；Runtime 只机械分派并继续校验真实状态。
- Repair design readiness: ready
- Next step: 用单一 `finish_map` 替换两个 Agent 可见 action；要求 `terminal_state` 的两个分支具有相同字段形状，
  并删除旧 action 解析入口，不保留兼容分支。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-010: 统一 action 时过度压缩状态字段重新放大了过早终结
- Status: confirmed
- Parent: P-002
- Claim: 单一 `finish_map` 消除了两个顶层 action 的竞争，但第一版只保留 `terminal_state` 和
  `terminal_node_id`，把 H-008 已证明必要的“其他未完成 Work”声明压缩进一个枚举词。Agent 可以在未枚举实际
  incomplete Work 的情况下直接声称 `last_running_work`，从而在业务结果已满足但 Map 仍有 Pending 后继时过早终结。
- Layer: tool-contract-state-snapshot
- Factor relation: single
- Depends on:
  - H-008
  - H-009 first repair
- Rationale:
  - 第一版统一合同的三次复验不再选择 Ready-Finish 分支，但其中一次在 `fix` Running、`verify` Pending 时调用
    `finish_map(last_running_work, terminal_node_id=fix)`。
- Falsifiable predictions:
  - If true: 错误调用前完整图已包含 Pending 后继，且 action 只要求抽象状态枚举，没有要求提交实际 incomplete
    Work 列表或 Finish 身份/状态快照。
  - If false: 错误调用时 `verify` 已完成或不可见，或 schema 仍要求具体列出全部 incomplete Work。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查统一 action 是否因删除具体状态字段而重现 H-008。
  - Signal: revision 3 Map、首次 `finish_map` arguments、canonical rejection、后续 read/continue/finish 路径。
  - Capture method: 读取 v4 paired Docker rollout 和当前 provider-visible schema。
  - Event name or marker:
    - `taskspace.complete_terminal_rejected`
    - `taskspace.complete_handoff_committed`
  - Correlation keys:
    - pair-001/right
    - revision 3
    - owner `fix`
  - Differentiates from:
    - Ready-Finish 分支误选
    - projection/反馈丢失
    - Runtime 非法提交
  - Supports if:
    - 初始化图和 read_map 都显示 `verify` Pending，错误调用仍声明 `last_running_work`，且 Runtime 零提交拒绝。
  - Refutes if:
    - canonical Map 中没有其他 incomplete Work。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 保留 terminal-state 分支、状态失败和 exact incomplete Work mismatch 观测。
- Evidence gate: satisfied
- Related evidence:
  - E-018
- Conclusion: pair-001 在 revision 3 的 `fix` 节点已运行测试，但初始化图仍有 `verify` Pending；Agent 调用
  `finish_map(last_running_work)` 后收到 `finish_not_ready`，read_map 只是重现既有图，随后正确 handoff 到
  `verify` 并终结。H-009 的单入口修复有效，回归点是第一版统一合同没有要求 Agent 提交具体 incomplete Work
  集合。应在同一 `finish_map` 中恢复为等形终态快照，而不是重新拆分 action。
- Repair design readiness: ready
- Next step: `finish_map` 继续保持唯一入口，新增必填 `incomplete_work_node_ids`、`finish_node_id` 和
  `finish_status`；parser 校验它们与 Agent 选择的 `terminal_state` 自洽，状态机校验节点身份和 canonical state。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-011: Agent 自报快照不能消除同一 action 内的前态分支竞争
- Status: fixed
- Parent: P-002
- Claim: `finish_map` 虽然只有一个 action 名，但 `terminal_state` enum 仍要求 Agent 在两个互斥前态之间做一次
  工具分支选择。`incomplete_work_node_ids` 与 `finish_status` 只约束所选分支内部自洽，不能证明它们来自 canonical
  Map；模型仍可先选择业务意图更接近“直接结束”的 Ready-Finish，再生成与该选择配套的空列表和 Ready 状态。
- Layer: tool-contract-prestate-branch
- Factor relation: single
- Depends on:
  - H-009
  - H-010
- Rationale:
  - v5 已要求两个分支使用完全相同字段并提交具体快照；若快照字段能成为真实选择依据，`verify` Running 时不应
    稳定生成 `no_active_work_ready_finish + [] + ready`。
- Falsifiable predictions:
  - If true: 多次运行仍会生成内部自洽但与 canonical Map 矛盾的 Ready-Finish 快照，并在准确拒绝后改为
    `last_running_work`；增加更多自报字段不会根治。
  - If false: v5 首次终态调用稳定匹配 canonical Map，或错误来自字段/反馈缺失。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查等形精确快照是否真正消除了前态分支竞争。
  - Signal: 首次 `finish_map` arguments、owner/revision、canonical rejection 和后续纠正调用。
  - Capture method: 读取 v5 三次 paired Docker rollout 与性能报告。
  - Event name or marker:
    - `taskspace.control_rejected`
    - `taskspace.complete_terminal_committed`
  - Correlation keys:
    - repeats 1-3
    - owner `verify`
    - revision 4
  - Differentiates from:
    - schema/L2 未进入上下文
    - Map feedback 丢失
    - Runtime 接受非法状态
  - Supports if:
    - 至少两次在 `verify` Running 时提交 `no_active_work_ready_finish + [] + ready`，均零提交拒绝并纠正。
  - Refutes if:
    - 首次调用均为真实 `last_running_work`，或 canonical state 本来就是 Ready Finish。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 保留终态候选、分支、state failure、参数和 canonical revision 观测。
- Evidence gate: satisfied
- Related evidence:
  - E-019
  - E-020
- Conclusion: v5 三次都在 `verify` Running 时先提交了内部自洽但不真实的 Ready-Finish 快照；其中一次还在
  `fix` Running、`verify` Pending 时过早提交 last-work 快照。自报字段没有把状态断言变成证据，只增加了错误
  分支的填写成本。根因仍是 Agent 可见合同要求选择底层前态事务。公共合同应只有一个 `finish_map` 语义，由
  Agent 明确发起并提供节点身份和原样 summary；Runtime 在同一状态机 transition 内机械校验并执行当前合法终态
  frontier，不向 Agent 暴露内部事务分支。
- Repair design readiness: implemented and verified
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed and verified by E-020

## Hypothesis H-001: 失败由上下文或 Map 反馈丢失导致
- Status: refuted
- Parent: P-001
- Claim: Agent 因看不到当前 binding、边或 protocol，才在 Patch 时遗漏 transition。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - TaskSpace 历史上存在反馈层语义丢失问题，必须优先排查。
- Falsifiable predictions:
  - If true: 失败请求前缺少 L2、Map 信息或最新 revision，或 `read_map` 结果未进入后续上下文。
  - If false: 失败请求可见完整 L2/Map，且 reasoning 能准确说出当前边界和下一阶段。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败 Patch 前的 provider-visible L2、Map read 与 reasoning 是否完整。
  - Signal: `rollout.jsonl` 中 developer message、`read_map` output、reasoning 和紧随其后的 Tool call。
  - Capture method: 按 rollout 顺序读取三次重复 trace，并将 Map read、reasoning 和 Patch call 关联。
  - Event name or marker:
    - `TaskSpaceMapProjectionR7V1`
  - Correlation keys:
    - run id
    - call_id
  - Differentiates from:
    - H-003
  - Supports if:
    - Patch 前缺少关键状态或协议。
  - Refutes if:
    - Patch 前状态完整且 reasoning 正确识别边界。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: 失败轮 1 在 Patch 前读取 revision 2 的完整 Map，并明确推理“complete exploration and move to fixing”；上下文信息存在且语义正确，因此反馈丢失不是本次根因。
- Repair design readiness: not applicable
- Next step: 保持反馈忠实性回归，但不以增强 Runtime 语义反馈修复此问题。
- Blocker:
  - none
- Close reason:
  - refuted by direct trace

## Hypothesis H-002: carrier 能力或 Runtime 执行顺序错误
- Status: refuted
- Parent: P-001
- Claim: `apply_patch` 实际不能携带 transition，或 Runtime 在 Patch 之后才提交 transition，导致工作落在旧节点。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - FLA-3.5 刚完成 action carrier 生产路径，必须验证实现与文档是否一致。
- Falsifiable predictions:
  - If true: provider schema 不暴露 carrier，成功轮不能在 Patch 上提交 transition，或代码先 dispatch Tool 再 commit transition。
  - If false: 同一 schema 的成功轮能够正确携带，代码和 schema 都明确 transition 先于 Tool。
- Diagnostic evidence plan:
  - Prediction or clause under test: provider-visible capability与执行顺序。
  - Signal: 成功 rollout 的 Patch 参数、`taskspace_transition_schema` 描述、`commit_carried_transition` 调用路径。
  - Capture method: 对照成功 trace与 `tools/src/taskspace_tool.rs`、`core/src/tools/taskspace_carrier.rs`。
  - Event name or marker:
    - `taskspace.carrier_transition_committed`
  - Correlation keys:
    - call_id
  - Differentiates from:
    - H-003
  - Supports if:
    - carrier 未暴露或在动作后执行。
  - Refutes if:
    - 成功轮和代码均证明先转换再动作。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: 同一构建的成功轮在 `apply_patch` 上提交 `explore -> fix_impl` 并成功；schema 和执行器均明确在 Tool dispatch 前提交 transition。
- Repair design readiness: not applicable
- Next step: 不修改 transition 的原子执行顺序。
- Blocker:
  - none
- Close reason:
  - refuted by code and reproduction

## Hypothesis H-003: 可选 carrier 把无意遗漏静默解释为继续当前节点
- Status: confirmed
- Parent: P-001
- Claim: 普通 Tool 的 `taskspace_transition` 是可选 sidecar；字段缺失时 Runtime 合法地继续当前 binding，因此 Agent 即使已经决定进入下一阶段，Tool call 漏填也不会在该动作边界暴露，canonical Map 与实际工作由此错位。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - transition 不能在每次 Tool call 都强制发生，但当前 schema 也没有要求 Agent 显式声明“继续当前节点”；遗漏与有意继续使用相同的空值表示。
- Falsifiable predictions:
  - If true: 失败 reasoning 已决定切换，Patch 参数却完全缺少 transition；Runtime 接受 Patch 并保持 revision/binding；下一次 Agent 按实际阶段选择 `verify` 时出现非法跳边。
  - If false: 失败 Patch 携带了 transition 但被解析丢失，或字段缺失会在当前动作上立即产生机械错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: 从边界意图、Tool 参数、Runtime commit 到后续非法跳转的完整因果链。
  - Signal: reasoning 文本、Patch JSON、Patch result、后续 `complete_then_continue` error、carrier schema required fields。
  - Capture method: 逐 request 对照原始运行和三次重复，并静态检查 Tool decorator 的 required 列表。
  - Event name or marker:
    - `TASKSPACE_LIFECYCLE_INVARIANT`
  - Correlation keys:
    - run id
    - call_id
    - canonical revision
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - 漏填 Patch 被接受且 Map 不动，随后出现与实际工作领先一节点一致的非法跳转。
  - Refutes if:
    - transition 实际生成后被链路丢失，或错位在 Patch 前已存在。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 action、binding、carried transition 和 canonical revision 的关联日志。
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-003
  - E-005
- Conclusion: 四次运行中三次 Patch 不携带 transition；这三次实际修复均发生在 `explore` binding 下，随后分别以跳过 `fix`、补记 transition 或过早终结表现出来。可选字段将漏填静默等同于有意继续，是问题能够形成并延后暴露的结构原因。
- Repair design readiness: implemented and verified
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed and verified by E-009

## Evidence E-001: L2 在三次重复运行中均进入 provider-visible history
- Related hypotheses:
  - H-001
- Direction: refutes
- Type: observation
- Source: 三次 `rollout.jsonl` 的首个 developer message
- Prediction or plan link:
  - H-001 provider-visible L2 检查。
- Matched signal:
  - 三次均包含 `taskspace_core_protocol version="taskspace-core-v2.2"`，并明确 successor 首动作携带 `complete_then_continue`。
- Correlation keys:
  - `20260722-202126-190`
  - `20260722-202126-218`
  - `20260722-202126-183`
- Raw content:
  ```text
  When the active Work node is complete and work continues, put complete_then_continue in the successor's first real action Tool.
  ```
- Interpretation: 行为差异不能由 L2 在部分运行中缺失解释。
- Time: 2026-07-22 20:30

## Evidence E-002: 失败轮在识别边界后仍生成无 transition 的 Patch
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: `target/r7-fla35-repeat-check/single-file-fast-fix/20260722-202126-190/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-003 边界意图与 Tool 参数不一致。
- Matched signal:
  - Agent 先读取 revision 2 Map，随后 reasoning 明确探索完成并进入修复，但 Patch JSON 只有 `input`。
- Correlation keys:
  - Patch call `call_00_MXjC1h8AuDxZ5KFkokOK6860`
- Raw content:
  ```text
  Now I understand the bug. Let me complete the exploration and move to fixing the issue.
  apply_patch {"input":"*** Begin Patch ..."}
  ```
- Interpretation: 这不是 Agent 不知道当前状态或边界，而是 Tool call 形成阶段遗漏了可选 carrier。
- Time: 2026-07-22 20:30

## Evidence E-003: 同一 schema 的成功轮正确携带 transition
- Related hypotheses:
  - H-002
  - H-003
- Direction: refutes
- Type: reproduction
- Source: `target/r7-fla35-repeat-check/single-file-fast-fix/20260722-202126-218/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-002 carrier capability检查；H-003 非确定性漏填检查。
- Matched signal:
  - Patch 携带 `complete_then_continue(explore -> fix_impl)`，pytest 携带 `complete_then_continue(fix_impl -> verify)`，6 个 provider request 完成。
- Correlation keys:
  - Patch call `call_00_FjPXk8htoks9DXHzzxuD6219`
- Raw content:
  ```text
  "taskspace_transition":{"action":"complete_then_continue","current_node_id":"explore","expected_revision":2,"next_node_id":"fix_impl"}
  ```
- Interpretation: 能力和链路可用；问题是合同遵循不稳定，而不是 ABI 缺位。
- Time: 2026-07-22 20:30

## Evidence E-004: carrier schema 与 Runtime 都定义先转换后动作
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs:187`；`third_party/codex-cli/codex-rs/core/src/tools/taskspace_carrier.rs:19`
- Prediction or plan link:
  - H-002 执行顺序检查。
- Matched signal:
  - schema 描述为 `before executing the carrying Tool`；executor 先调用 `execute_taskspace_transition`，成功后才由后续 sequence dispatch Tool。
- Correlation keys:
  - none
- Raw content:
  ```text
  Atomically complete the active Work node and bind one Agent-selected Ready successor before executing the carrying Tool.
  ```
- Interpretation: Runtime 时序与产品合同一致，不导致旧 binding 下执行。
- Time: 2026-07-22 20:30

## Evidence E-005: 漏填后 Map 保持 explore 并在后续跳转暴露
- Related hypotheses:
  - H-003
- Direction: supports
- Type: experiment
- Source: 原始运行及三次重复的 `rollout.jsonl`、`metrics.json`
- Prediction or plan link:
  - H-003 漏填被静默解释为继续当前 binding。
- Matched signal:
  - 四次中三次 Patch 无 transition；两次随后请求 `explore -> verify` 被 `TASKSPACE_LIFECYCLE_INVARIANT` 拒绝，原始运行在 `fix` 上过早终结后纠正；只有携带 transition 的运行无生命周期失败。
- Correlation keys:
  - `20260722-174521-239`
  - `20260722-202126-190`
  - `20260722-202126-218`
  - `20260722-202126-183`
- Raw content:
  ```text
  observed exact premature terminal: 1/4
  observed Patch without transition and lifecycle lag: 3/4
  clean carrier path: 1/4
  ```
- Interpretation: 过早终结是偶发表现；由 carrier 漏填造成的实际工作/Map 错位是重复出现的共同机制。
- Time: 2026-07-22 20:30

## Evidence E-006: 新合同下两次真实 TaskSpace 运行均在动作边界同步推进
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation-partial
- Source: `target/r7-action-fix/single-file-fast-fix/20260722-210449-752/pair-001` 与 `pair-003`
- Prediction or plan link:
  - H-003 显式 action 合同应消除静默 Patch 漂移。
- Matched signal:
  - 两轮 logical TaskSpace 均 solved；Patch 携带 `complete_then_continue(explore -> fix_impl)`，测试携带
    `complete_then_continue(fix_impl -> verify)`，最终 5 nodes / 4 edges / 0 open leaves。
- Correlation keys:
  - repeat 1
  - repeat 3
- Raw content:
  ```text
  TaskSpace solved: 2/2
  silent Patch/lifecycle drift: 0/2
  ```
- Interpretation: 目标结构在实际 TaskSpace 路径生效，但尚需修复 Standard 污染后重新做平衡重复验证。
- Time: 2026-07-22 21:15

## Evidence E-007: 失败 repeat 2 是被 carrier 污染的 Standard
- Related hypotheses:
  - H-004
- Direction: supports
- Type: diagnostic-trace
- Source: `target/r7-action-fix/single-file-fast-fix/20260722-210449-752/pair-002`
- Prediction or plan link:
  - H-004 模式与 schema 装饰条件不一致。
- Matched signal:
  - `logical-mode-map.json` 指定 right=`standard`；provider base identity profile=`standard`、无 TaskSpace core
    protocol；但 `exec_command` schema 要求 `taskspace_action`，遗漏即返回 `TASKSPACE_ACTION_REQUIRED`。
- Correlation keys:
  - repeat 2
- Raw content:
  ```text
  right logical_mode: standard
  base profile: standard
  TaskSpace event Map: absent
  ordinary Tool rejection: TASKSPACE_ACTION_REQUIRED
  ```
- Interpretation: 合法 `initialize_map` 缺 source 是 Standard 上不存在 Map event store 的必然结果，不是
  TaskSpace 初始化随机失效。
- Time: 2026-07-22 21:15

## Evidence E-008: 未 dispatch 的 action failure 在一次反馈中重复两遍
- Related hypotheses:
  - H-005
- Direction: supports
- Type: diagnostic-trace
- Source: repeat 2 `rollout.jsonl` 与 `core/src/tools/parallel.rs`
- Prediction or plan link:
  - H-005 rejection feedback 组合检查。
- Matched signal:
  - output 先含 `TaskSpaceCarrierResultV2.action_result=<failure>`，换行后再次出现同一 failure；同时
    `tool_dispatched=false`，不存在普通工具事实。
- Correlation keys:
  - `call_00_znQM8v0heu00UqQFP1Yo5675`
- Raw content:
  ```text
  TaskSpaceCarrierResultV2(action_result=TaskSpaceActionValidationResultV1, tool_dispatched=false)
  TaskSpaceActionValidationResultV1
  ```
- Interpretation: 反馈层重复会放大输入并削弱错误显著性，应收敛为一个准确 envelope。
- Time: 2026-07-22 21:15

## Evidence E-009: 显式 action 合同三次消除静默生命周期漂移
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `target/r7-action-scope-fix/single-file-fast-fix/20260722-212237-506` 三次 logical TaskSpace rollout
- Prediction or plan link:
  - H-003 fix criteria：每个动作显式声明 continuation 或 lifecycle action，实际工作不再领先 Map。
- Matched signal:
  - 三次均由首个真实命令携带 `initialize_map`；全部读取携带 `continue_current(explore)`；全部 Patch 携带
    `complete_then_continue(explore -> fix)`；全部测试携带 `complete_then_continue(fix -> verify)`。
- Correlation keys:
  - pair-001 taskspace
  - pair-002 taskspace
  - pair-003 taskspace
- Raw content:
  ```text
  solved: 3/3
  silent Patch/lifecycle drift: 0/3
  final Map: 5 nodes / 4 edges / 0 open leaves (each run)
  ```
- Interpretation: 在原问题同构真实样本中，必填判别联合消除了遗漏与有意继续使用相同空值的结构歧义。
- Time: 2026-07-22 21:30

## Evidence E-010: Standard 与 TaskSpace carrier schema 已按模式隔离
- Related hypotheses:
  - H-004
- Direction: supports
- Type: fix-validation
- Source: 同一三组配对运行的 `provider-wire-trace.jsonl` 和 core 定向测试
- Prediction or plan link:
  - H-004 repair：共享 registry 保留原 schema，只在 TaskSpace provider visibility 投影 carrier。
- Matched signal:
  - Standard 三次 Tool hash 均为 `84911a77...`、12 tools、每请求 21,669 tool bytes；TaskSpace 三次 Tool hash
    均为 `02cddab3...`、13 tools、每请求 60,747 tool bytes。Standard rollout 中 `taskspace_action` 与
    `TASKSPACE_ACTION_` 均为 0；`carrier_schema_is_visible_only_in_taskspace_mode` 通过。
- Correlation keys:
  - pair-001 至 pair-003
- Raw content:
  ```text
  Standard solved: 3/3; taskspace_action occurrences: 0
  TaskSpace solved: 3/3; required taskspace_action active
  ```
- Interpretation: Standard 污染已消除，且两个模式仍共享 registry 与 router，没有建立平行工具实现。
- Time: 2026-07-22 21:30

## Evidence E-011: 生命周期同步后仍存在独立终态 action 误选
- Related hypotheses:
  - H-006
- Direction: neutral
- Type: observation
- Source: 同一三次 TaskSpace rollout 与性能观察报告
- Prediction or plan link:
  - P-002 初始观测，尚未执行区分性实验。
- Matched signal:
  - pair-001 与 pair-003 在 verify Running 时先调用 `finish_end`，被 `finish_not_ready` 拒绝后分别通过
    `read_map + complete_then_end` 和直接 `complete_then_end` 纠正；pair-002 直接调用 `complete_then_end`。
- Correlation keys:
  - pair-001 taskspace
  - pair-002 taskspace
  - pair-003 taskspace
- Raw content:
  ```text
  premature finish_end selection: 2/3
  illegal terminal state commit: 0/3
  ```
- Interpretation: 该现象与已修复的 carrier 漂移可分离；样本量和现有 trace 尚不足以确认其具体根因。
- Time: 2026-07-22 21:30

## Evidence E-012: 未 dispatch 拒绝反馈已收敛为单一事实
- Related hypotheses:
  - H-005
- Direction: supports
- Type: fix-validation
- Source: `core/src/tools/taskspace_carrier_tests.rs` 与 FLA-3.5 executable contract
- Prediction or plan link:
  - H-005 repair：`tool_dispatched=false` 时 envelope 替换内部 failure body。
- Matched signal:
  - 定向测试构造拒绝响应并断言失败文本只出现一次；合同 gate 静态断言拒绝分支调用
    `replace_function_output(response, header)`。
- Correlation keys:
  - unit fixture call id `call`
- Raw content:
  ```text
  assert_eq!(text.matches("stale revision").count(), 1)
  ```
- Interpretation: 未执行普通 Tool 时不再伪造第二份工具事实，反馈语义和长度均已收敛。
- Time: 2026-07-22 21:30

## Evidence E-013: 提交后 manifest 1.0.8 二进制配对冒烟通过
- Related hypotheses:
  - H-003
  - H-004
  - H-006
- Direction: supports
- Type: fix-validation
- Source: `target/r7-action-postcommit-smoke/single-file-fast-fix/20260722-214106-772`
- Prediction or plan link:
  - 验证最终合同哈希和二进制接线未偏离三组行为验证。
- Matched signal:
  - 提交 `3d29e3916` 后重新构建和 attestation；Standard 与 TaskSpace 均 solved，公开和隐藏验证均通过；
    provider trace 识别 manifest `1.0.8` 且哈希
    `c97bb1c7...` 匹配。TaskSpace 初始化、Patch、验证均由正确动作携带，最终 5 nodes / 4 edges / 0 open。
    Standard trace 中无 `taskspace_action` 或 `TASKSPACE_ACTION_*`。终态再次先误选一次 `finish_end` 后纠正。
- Correlation keys:
  - session `019f8a0f-3fcd-7141-9819-77b7f0c5d96b`
- Raw content:
  ```text
  standard: solved, 6 requests, 8 tools, 14.25s
  taskspace: solved, 8 requests, 6 ordinary tools, 3 controls, 22.79s
  build attestation source: 3d29e39168a2362d01ea4c5ac45a33078f4ccd53
  manifest identity: 1.0.8 / matches_current_contract=true
  ```
- Interpretation: 最终生产合同接线保持本次结构修复；P-002 的终态误选可稳定地与 carrier 漂移区分。
- Time: 2026-07-22 21:40

## Evidence E-014: 终态误选发生在状态事实完整但 action 合同可辨识性不足时
- Related hypotheses:
  - H-006
- Direction: supports
- Type: diagnostic-trace-and-contract
- Source: 五次 TaskSpace rollout、L2 core protocol 与 provider-visible `taskspace_control` schema
- Prediction or plan link:
  - 区分状态反馈缺失、action 顺序偏差和终态合同选择歧义。
- Matched signal:
  - 五次验证 Tool 输出都明确提交 `complete_then_continue(... -> verify/run_tests)`、revision 4 和对应
    `node_bound` 事件；4/5 随后 reasoning 表达“tests passed, close task”并调用 `finish_end`。准确拒绝后 Agent
    立即说明最后 Work 仍 Running，并改用 `complete_then_end`。唯一成功首选也处于相同状态。
  - L4 每次同时暴露 `complete_then_end` 和 `finish_end`；前者需要 `current_node_id`，后者参数更少且名称更像
    通用结束动作。正确动作位于 schema 前面，排除“仅因顺序靠后”。L2 只要求显式关闭 Finish，没有写明终态
    状态到 action 的选择规则。
- Correlation keys:
  - formal repeats 1-3
  - final smoke
  - post-commit smoke
- Raw content:
  ```text
  reasoning: All 3 tests passed. Let me close the task properly.
  selected: finish_end(expected_revision=4)
  exact rejection: finish_not_ready
  correction: complete_then_end(current_node_id=run_tests, expected_revision=4)
  ```
- Interpretation: 事实链没有丢失或扭曲；问题是两个状态互斥动作被无差别同时暴露，且名称、参数形状和 L2
  说明没有让 Agent 稳定区分“完成 Running Work 后终结”与“只关闭已经 Ready 的 Finish”。
- Time: 2026-07-22 21:55

## Evidence E-015: v2.4 说明完整可见时仍在三次运行中误选终态动作
- Related hypotheses:
  - H-006
  - H-007
- Direction: supports
- Type: fix-validation-and-diagnostic
- Source: `target/r7-terminal-action-fix/single-file-fast-fix/20260723-000158-444`
- Prediction or plan link:
  - H-006 首次修复应使 Agent 根据 Running/Ready 状态直接选择正确终态动作。
  - H-007 区分说明未送达与顶层 discriminator 仍按业务关闭意图产生错误匹配。
- Matched signal:
  - 三次 TaskSpace rollout 都完整携带 L2 v2.4；验证反馈都已提交
    `complete_then_continue(... -> verify)`，当前 revision 为 4，`verify` 为 node-bound Running Work。
  - 三次首次终态选择均为 `close_ready_finish`，没有一轮直接选择 `complete_then_end`。
  - repeat 1/2 收到准确的 `finish_not_ready` 后改用 `complete_then_end`；repeat 3 的首次调用仅因 action 值
    缺少引号而参数解析失败，反馈未提供任何 Map 状态，但下一轮仍自行指出 `verify` Running 并正确纠正。
- Correlation keys:
  - repeats 1-3
  - canonical revision 4
  - current node `verify`
- Raw content:
  ```text
  reasoning: All 3 tests pass. Let me close the task.
  first selected action: close_ready_finish
  first-choice mismatch: 3/3
  solved: 3/3
  ```
- Interpretation: 首次命名与说明修复没有消除误选。Agent 已拥有并能自行恢复当前状态，因而不是上下文、反馈或
  Map 事实丢失；branch 描述不足以抵消顶层 action 名对“关闭任务”业务意图的吸引。互斥机械状态必须进入
  discriminator 和必填参数结构。
- Time: 2026-07-23 00:15

## Evidence E-016: 第二次修复消除专用动作误选但暴露普通终态动作前置状态不完整
- Related hypotheses:
  - H-007
  - H-008
- Direction: supports
- Type: fix-validation-and-diagnostic
- Source:
  - `target/r7-terminal-action-contract-v2/single-file-fast-fix/20260723-003214-028`
  - `target/r7-terminal-action-contract-v2-repeat/single-file-fast-fix/20260723-003633-112`
- Prediction or plan link:
  - H-007 第二次修复应消除 Ready-Finish 专用动作误选。
  - H-008 检查普通终态动作是否仍缺少“唯一剩余 Work”判别。
- Matched signal:
  - 5 次 TaskSpace 运行中 `close_finish_with_no_active_work` 误选为 0；4 次实际终态调用均最终使用
    `complete_active_work_then_end`，说明 H-007 的专用分支歧义已收敛。
  - 其中 1 次在 `fix` Running、`verify` Pending 时过早调用 `complete_active_work_then_end`，收到
    `finish_not_ready`；Agent 随后错误尝试删除依赖边，读取 Map 后才用 `complete_then_continue` 进入 `verify`。
  - 另 1 次没有调用任何终态 action，直接输出 final，被 `taskspace_terminal_protocol_violation` 终止；该现象与
    action 误选分开记录，不作为 H-008 的因果证据。
- Correlation keys:
  - paired repeats 1-3
  - taskspace-only repeats pair-001/pair-003
  - premature call revision 3 / current node `fix`
- Raw content:
  ```text
  initialized edge: fix -> verify -> finish
  selected: complete_active_work_then_end(current_node_id=fix, expected_revision=3)
  rejection: finish_not_ready
  read_map: verify role=work status=pending
  Ready-Finish specialized misselection: 0/5
  ```
- Interpretation: 第二次修复证明把互斥状态写进专用分支有效，但普通终态分支仍以不完整的 `active_work` 状态
  暴露。必须继续把“唯一剩余 Work”写进 discriminator 和参数形状，而不能依赖 Runtime 拒绝后教学。
- Time: 2026-07-23 00:45

## Evidence E-017: 第三轮三次均先伪造 Ready-Finish 状态再纠正
- Related hypotheses:
  - H-009
- Direction: supports
- Type: fix-validation-and-diagnostic
- Source: `target/r7-terminal-action-contract-v3/single-file-fast-fix/20260723-005132-870`
- Prediction or plan link:
  - H-009 检查两个并列终态入口是否允许模型先按业务意图选择较短分支，再补写与真实 Map 矛盾的固定枚举。
- Matched signal:
  - 三次 TaskSpace 均 solved，且调用前 owner 均为 `verify`、canonical revision 均为 4。
  - 三次首次终态调用均为
    `close_finish_with_no_active_work(active_work_status=none, finish_status=ready)`，均收到
    `TASKSPACE_LIFECYCLE_INVARIANT / finish_not_ready`，revision 保持 4。
  - 三次随后均调用
    `complete_last_running_work_then_end(current_node_id=verify, other_incomplete_work_status=none,
    finish_status=pending)` 并在 revision 5 原子提交 Work、Finish 与 Root 闭合。
  - pair-002 在准确拒绝后额外 `read_map` 一次；另外两次无需重读即可纠正，进一步排除状态事实丢失。
- Correlation keys:
  - pair-001/right call `call_00_aj78oLJ9Ukgqf9hK0uvc5825`
  - pair-002/left call `call_00_mOAaxKYhFGEFO6r86ot54764`
  - pair-003/right call `call_00_8GrulFNStnK7dc5pShcX5686`
- Raw content:
  ```text
  first terminal choice: close_finish_with_no_active_work = 3/3
  submitted state: active_work_status=none, finish_status=ready
  canonical rejection: finish_not_ready = 3/3
  corrected action: complete_last_running_work_then_end(current_node_id=verify) = 3/3
  illegal state commit: 0/3
  ```
- Interpretation: Tool 结果和 Map 状态正确进入上下文，Runtime 也忠实拒绝了非法动作。失败来自 Agent 可见
  Tool schema 同时提供两个竞争终态 action，且固定枚举可以在选定较短分支后机械补齐，未真正迫使模型先判断状态。
- Time: 2026-07-23 01:15

## Evidence E-018: 单一终态 action 消除分支误选但出现一次抽象 last-work 误报
- Related hypotheses:
  - H-009
  - H-010
- Direction: supports
- Type: fix-validation-and-diagnostic
- Source: `target/r7-terminal-action-contract-v4/single-file-fast-fix/20260723-012048-340`
- Prediction or plan link:
  - H-009 验证单一 action 是否消除 Ready-Finish 竞争入口。
  - H-010 检查压缩后的状态字段是否足以阻止有 Pending 后继时声称 last Running Work。
- Matched signal:
  - Standard 与 TaskSpace 均 3/3 solved；TaskSpace 三次合法终态都使用
    `finish_map(terminal_state=last_running_work, terminal_node_id=verify)`。
  - `no_active_work_ready_finish` 调用为 0/3，相比 v3 的错误 Ready-Finish 首选 3/3，H-009 修复方向成立。
  - pair-001 在 revision 3、owner=`fix` 时额外调用一次
    `finish_map(last_running_work, terminal_node_id=fix)`；canonical Map 中 `verify` 仍 Pending，调用被
    `finish_not_ready` 零提交拒绝。Agent read_map 后 handoff 到 `verify`，再正确终结。
  - pair-003 首次 `finish_map` 把 action 值输出为未加引号的裸标识符，parser 以 `argument_failed` 零提交拒绝；
    这是独立 JSON 生成错误，不是 terminal-state 选择或反馈扭曲。
- Correlation keys:
  - pair-001/right call `call_00_XOMcle90R7c8MHHDoq6Z1303`
  - pair-003/right call `call_00_QFZePHqGyD7L7HqgbQST3555`
- Raw content:
  ```text
  Ready-Finish state selections: 0/3
  pair-001 canonical graph: fix(running) -> verify(pending) -> finish(pending)
  pair-001 submitted: terminal_state=last_running_work, terminal_node_id=fix
  pair-001 result: finish_not_ready, state_commit=false
  pair-003 malformed: {"action": finish_map, ...}
  ```
- Interpretation: 单一 action 已修复原始竞争入口，但状态合同不能只用一个抽象 enum 代替具体 Map 快照。
  Runtime 和反馈均按设计工作；下一轮只补齐 Tool 输入的机械事实，不增加 Runtime 语义选择。
- Time: 2026-07-23 01:30

## Evidence E-019: 等形精确快照仍在三次运行中稳定伪造 Ready-Finish 前态
- Related hypotheses:
  - H-010
  - H-011
- Direction: supports
- Type: failed-fix-validation-and-diagnostic
- Source: `target/r7-terminal-action-contract-v5/single-file-fast-fix/20260723-013922-261`
- Prediction or plan link:
  - H-010 检查具体 incomplete Work 列表能否阻止过早终结。
  - H-011 检查自报快照是否仍只是分支选择后的配套断言。
- Matched signal:
  - Standard 与 TaskSpace 均 3/3 solved，但 TaskSpace 三次都至少一次提交
    `terminal_state=no_active_work_ready_finish`、`terminal_node_id=finish`、
    `incomplete_work_node_ids=[]`、`finish_status=ready`。
  - 三次调用的 canonical owner 均为 `verify`、revision 均为 4，实际 Finish 为 Pending；三次均收到
    `finish_not_ready` 且 `state_commit=false`，随后改用
    `last_running_work + [verify] + pending` 成功闭合。
  - pair-002 还在 revision 3、owner=`fix`、`verify` Pending 时先提交
    `last_running_work + [fix] + pending`，同样被零提交拒绝。
  - 没有参数解析失败，说明新字段完整进入生产 schema 且被模型正确序列化；失败是内容不真实，不是链路丢失。
- Correlation keys:
  - pair-001/right terminal sequences 48/57
  - pair-002/left terminal sequences 32/38/47
  - pair-003/right terminal sequences 29/39
- Raw content:
  ```text
  taskspace solved: 3/3
  Ready-Finish selections: 3/3 first terminal at verify
  state failures: 1, 2, 1
  parse errors: 0/3
  aggregate requests: standard 19, taskspace 36
  aggregate wall: standard 44.57s, taskspace 125.59s
  ```
- Interpretation: 精确快照字段只验证 Agent 参数内部自洽，无法迫使它以 canonical Map 为依据选择分支。Runtime
  拒绝和反馈仍然正确；继续增加字段或提示词只会扩大 schema 与请求成本。应删除 Agent 可见前态分支，让
  `finish_map` 成为一个显式、单义的状态机操作。
- Time: 2026-07-23 01:50

## Evidence E-020: 分支无关 finish_map 三次首次选择正确终态入口
- Related hypotheses:
  - H-011
- Direction: supports
- Type: fix-validation
- Source: `target/r7-terminal-action-contract-v6/single-file-fast-fix/20260723-021426-795`
- Prediction or plan link:
  - H-011 验证删除 Agent 可见前态分支和自报快照后，是否还会在 `verify` Running 时误选 Ready Finish。
- Matched signal:
  - Standard 与 TaskSpace 均 3/3 solved；TaskSpace 三次都只调用一次 `finish_map`。
  - 三次参数均为 `expected_revision=4, terminal_node_id=verify`；不存在 `terminal_state`、
    `incomplete_work_node_ids` 或 `finish_status`。
  - 三次规范反馈均为 `status=committed`、`canonical_revision=5`、`terminal_node_role=work`，并在同一图修订中
    完成 `verify`、使 Finish Ready 并提交最终闭合。
  - 终态 state failure、重试和参数解析错误均为 0；没有调用 `terminal_node_id=finish`。
  - pair-001 的一次失败是初始化前 `read_map` 返回 `map_id missing`，发生在终态之前且不属于终态合同。
- Correlation keys:
  - pair-001/right call `call_00_Hq4FRDHoC4UZ7b9f5MCL3286`
  - pair-002/left call `call_00_wbG1Ia9agxikJ50TOAMl1837`
  - pair-003/right call `call_00_30ks72JVREC5ezM94sZZ8287`
- Raw content:
  ```text
  taskspace solved: 3/3
  finish_map calls: 1, 1, 1
  terminal_node_id=verify: 3/3
  canonical terminal_node_role=work: 3/3
  Finish-entry commit: 0/3
  terminal state failures: 0/3
  terminal parse errors: 0/3
  ```
- Interpretation: 删除前态分支后，Agent 不再被要求猜测底层终态事务，也无法先选 Ready-Finish 再补写伪快照。
  Runtime 没有替 Agent 选择节点或解释任务语义，只根据 Agent 明确提交的节点身份执行状态机硬规则。原问题的
  结构根因已消除。
- Time: 2026-07-23 02:20
