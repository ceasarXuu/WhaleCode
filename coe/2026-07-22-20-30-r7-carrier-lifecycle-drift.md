# Problem P-001: R7 action carrier 生命周期与实际工作错位
- Status: open
- Created: 2026-07-22 20:30
- Updated: 2026-07-22 20:30
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
- Current conclusion: 直接触发是 Agent 在已理解节点边界时漏填 `taskspace_transition`；结构性根因是普通 Tool 将该字段设计为可选，字段缺失会被静默解释为“继续当前 binding”，没有要求 Agent 在“继续当前节点”和“进入后继节点”之间作显式选择。Runtime 因而无法区分有意继续与无意遗漏，只能在后续非法跳转或终结时机械拒绝。过早终结是该漂移累积到尾部后的偶发表现。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - E-001 至 E-005。
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
- Repair design readiness: ready; implementation requires user confirmation
- Next step: 设计语义无关的显式 action-binding 联合合同，避免 Runtime 推断 Patch/pytest 属于哪个节点。
- Blocker:
  - none
- Close reason:
  - not closed

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
