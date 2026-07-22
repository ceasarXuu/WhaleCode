# Problem P-001: R7 action carrier 生命周期与实际工作错位
- Status: fixed
- Created: 2026-07-22 20:30
- Updated: 2026-07-22 21:30
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

# Problem P-002: verify 运行时 Agent 偶发误选 finish_end
- Status: open
- Created: 2026-07-22 21:30
- Updated: 2026-07-22 21:30
- Objective: 解释生命周期已同步时，Agent 为何仍在最后一个 Work 节点 Running 状态下选择只适用于 Finish Ready 的 `finish_end`。
- Symptoms:
  - 三次修复后 TaskSpace 运行中两次先调用 `finish_end`，收到 `finish_not_ready` 后改用 `complete_then_end`。
- Expected behavior:
  - 最后一个 Work 节点仍 Running 时直接调用 `complete_then_end(current_node_id=verify)`。
- Actual behavior:
  - 当前观察合计 4/5 先误选 `finish_end`；1/5 直接选择正确动作。
- Impact:
  - 产生 1 次或 2 次额外 provider request；不破坏 Map，Runtime 拒绝事实准确。
- Known facts:
  - 三次 Patch、测试和 revision 均全程同步，H-003 不再复现。
  - `finish_end` 与 `complete_then_end` 的 schema 描述分别准确陈述适用前置状态。
- Ruled out:
  - 不是 carrier 字段遗漏造成的 Map 滞后。
  - 不是 Runtime 接受了非法终态。
- Fix criteria:
  - 在不引入 Runtime 语义判断、动态 schema 或缓存破坏的前提下，使 Agent 稳定选择与当前机械状态匹配的终态动作。
- Current conclusion: 这是与 P-001 独立的终态 action 选择问题；现有证据只证明它仍然存在，尚不足以确认是命名显著性、L2/L4 重复、action 集合组织还是其他机制导致。
- Related hypotheses:
  - H-006
- Resolution basis:
  - E-011。
- Close reason:
  - not closed

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
- Status: open
- Parent: P-002
- Claim: 当测试成功后，`finish_end` 的名称或位置比 `complete_then_end` 更容易被模型选择，即使当前 Work 仍为 Running；现有静态描述不足以稳定映射当前机械状态到正确 action。
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
- Evidence gate: not satisfied
- Related evidence:
  - E-011
- Conclusion: 尚未确认。
- Repair design readiness: not ready
- Next step: 用户确认是否将该独立问题进入诊断阶段后，再做静态合同审计和重复实验。
- Blocker:
  - none
- Close reason:
  - not closed

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
