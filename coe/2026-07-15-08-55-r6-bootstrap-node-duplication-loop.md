# Problem P-001: R6 初始化节点重复导致无状态请求循环
- Status: fixed
- Created: 2026-07-15 08:55
- Updated: 2026-07-15 09:50
- Objective: 消除 `initialize_map` 工具合同诱发的节点重复声明，并确保参数错误以原始 typed 结果进入 Agent 上下文。
- Symptoms:
  - `single-file-fast-fix` 的 R6 侧连续 73 次调用 `taskspace_control.initialize_map`，均返回 `initialize_map nodes requires unique node_id values`。
  - Agent 更换节点 ID 和图规模后仍重复同一参数形态；Map 始终没有提交。
- Expected behavior:
  - schema 结构应明确区分初始 Work 与其余 Work，使一次合法声明能够初始化 rooted graph。
  - 参数错误应由 `taskspace_control` 以单层 `TaskSpaceControlResultR6V1` 忠实返回。
- Actual behavior:
  - `current_work_node` 与 `work_nodes` 是两个同形字段，后者的字段级描述只有 `Work nodes.`；Agent 持续把当前 Work 同时放进列表。
  - sequence preflight 在 handler 前解析参数，把 typed TaskSpace 错误字符串化到 `ToolSequencePreflightResultV1.error.message`。
- Impact:
  - R6 Phase C 简单样本无法进入业务执行，产生至少 77 次 provider request 和显著 token/时间放大。
- Reproduction:
  - 运行 `target/r6-phase-c-repair2/simple/single-file-fast-fix/20260715-084751-378` 的 right-only Docker 样本。
- Environment:
  - Linux、Docker hard boundary、`deepseek-v4-flash`、branch `whalecode-alpha`、candidate `d12918e9d`。
- Known facts:
  - 73 次 `taskspace_control` 调用全部是 `initialize_map`；2 次普通命令均被 `no_task_path` 拒绝。
  - 首次及后续调用都令 `current_work_node.node_id` 同时出现在 `work_nodes[]`。
  - 唯一一次 `close_agent(root)` 发生在约 70 次相同初始化失败之后，随后 Agent 又回到 `initialize_map`。
  - production Map 从未成功初始化，没有业务文件读取、修改或测试执行。
- Ruled out:
  - `close_agent(root)` 不是该轮循环的起点或持续根因。
- Fix criteria:
  - 模型可见 schema 不再暴露含义重叠的初始 Work/全部 Work 字段。
  - 无效 TaskSpace 参数绕过 manifest 语义解析并由原工具 handler 返回单层 typed JSON。
  - 定向测试覆盖重复形态、单层反馈和 patch 数量硬约束不回归。
  - 原始简单 Docker 样本完成外部验证且不再发生初始化循环。
- Current conclusion: 重叠集合 schema 是初始化重复的根因，preflight 二次包装是反馈放大因素；互斥字段合同与 handler-owned typed feedback 已通过真实 Docker 复验。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - simple 的 Map 首次初始化成功，随后完成 5 节点/4 边 Root -> Finish 路径和外部验证。
  - branch-join 独立样本同样首次建立 4 节点/3 边 Map，没有重复节点或无状态请求循环。
- Close reason:
  - schema、反馈所有权、定向测试和原始 live 样本四项门禁全部满足。

## Hypothesis H-001: schema 把初始 Work 与 Work 列表表达成重叠集合
- Status: confirmed
- Parent: P-001
- Claim: `current_work_node` 与描述含混的 `work_nodes` 诱导 Agent 采用“列表包含当前节点”的常见表达，而解析器实际要求两者互斥，导致每次初始化在协议层失败。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - trace 中 Agent 持续更换 ID，但始终把同一 Work 同时放入对象和数组，表现为结构性误读而非某个 ID 冲突。
- Falsifiable predictions:
  - If true: 所有失败调用都具有当前 Work 重复出现在列表的相同结构，且 parser 会合并这些字段后执行全局 ID 唯一校验。
  - If false: 至少存在不含重复 Work 的调用仍得到相同唯一性错误，或 parser 没有合并两个字段。
- Diagnostic evidence plan:
  - Prediction or clause under test: trace 参数形态与 parser 节点集合构造方式一致。
  - Signal: rollout function calls、schema JSON 和 `validate_initialize_map` 代码路径。
  - Capture method: 统计全部 control 调用并检查首尾参数；读取 schema/parser 定义。
  - Event name or marker:
    - `task_context_event_recorded`
  - Correlation keys:
    - rollout sequence、call_id
  - Differentiates from:
    - H-002
  - Supports if:
    - 失败调用均重复当前 Work，且 parser 明确将两处节点放入同一唯一性集合。
  - Refutes if:
    - 调用没有重复或错误来自 graph Runtime transition。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: 模型可见字段结构和 parser 实际集合语义不一致，稳定诱发重复节点声明。
- Repair design readiness: satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - implemented and validated by E-004/E-005

## Hypothesis H-002: sequence preflight 扭曲了 TaskSpace 参数错误
- Status: confirmed
- Parent: P-001
- Claim: manifest 为统计 patch 提前执行 TaskSpace 参数解析，并把已有 typed JSON 当字符串写入另一 typed JSON，降低反馈的结构清晰度。
- Layer: interaction
- Factor relation: all_of
- Depends on:
  - H-001
- Rationale:
  - provider 可见输出的顶层 schema 是 `ToolSequencePreflightResultV1`，内层 `error.message` 才包含转义后的 `TaskSpaceControlResultR6V1`。
- Falsifiable predictions:
  - If true: `ToolSequenceManifest::from_calls` 将 `FunctionCallError` 格式化为字符串，`ToolSequencePreflightFailure::outputs` 再把它放入 `error.message`。
  - If false: 无效参数直接到达 TaskSpace handler，或输出只含一个 typed 结果。
- Diagnostic evidence plan:
  - Prediction or clause under test: 参数解析在 handler 前发生且丢失原错误类型。
  - Signal: `sequence_manifest.rs`、`sequence_preflight.rs` 和 rollout output。
  - Capture method: 对齐代码调用链与首个失败 call_id 的输出。
  - Event name or marker:
    - `tool.request_manifest_rejected`
  - Correlation keys:
    - call_id `call_00_w01taHfIItRMZSNoeYsM7449`
  - Differentiates from:
    - H-001
  - Supports if:
    - preflight 确实把 `RespondToModel` 内容转成普通 message。
  - Refutes if:
    - handler 自身生成双层结果。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: 消息条目未丢失，但 typed 错误被 preflight 二次包装和转义，属于反馈语义扭曲。
- Repair design readiness: satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - implemented and validated by E-004/E-005

## Hypothesis H-003: close_agent(root) 是循环的独立根因
- Status: refuted
- Parent: P-001
- Claim: multi-agent `close_agent(root)` 错误导致 Runtime 无法推进并触发持续重试。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - stderr 最后出现 `root is not a spawned agent`，初看可能是新的生命周期循环。
- Falsifiable predictions:
  - If true: `close_agent(root)` 应在循环开始前出现，后续请求持续调用 close 或依赖其结果。
  - If false: 大量初始化拒绝先发生，close 只出现一次，之后仍回到原初始化错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: close 调用在完整工具时序中的位置和频率。
  - Signal: rollout 全部 function call/output 顺序。
  - Capture method: 按 sequence 提取工具名和结果。
  - Event name or marker:
    - `task_context_event_recorded`
  - Correlation keys:
    - rollout sequence 143-146
  - Differentiates from:
    - H-001
  - Supports if:
    - close 是首个失败或被持续重复。
  - Refutes if:
    - close 发生在大量初始化失败之后且仅一次。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: close 是 Agent 在长期 bootstrap 失败后的单次逃逸动作，不是循环根因。
- Repair design readiness: not applicable
- Next step: 不修改 multi-agent runtime。
- Blocker:
  - none
- Close reason:
  - root-cause claim refuted

## Evidence E-001: 完整 trace 从未成功初始化 Map
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: reproduction
- Source: `target/r6-phase-c-repair2/simple/single-file-fast-fix/20260715-084751-378/pair-001/right/artifacts/home/.whale/sessions/2026/07/15/rollout-2026-07-15T00-47-53-019f633e-93f0-7f92-8dbc-eb905a1fb4d7.jsonl`
- Prediction or plan link:
  - H-001/H-003 工具调用频率与时序
- Matched signal:
  - 73 `taskspace_control`、2 `exec_command`、1 `list_agents`、1 `close_agent`
- Correlation keys:
  - session `019f633e-93f0-7f92-8dbc-eb905a1fb4d7`
- Raw content:
  ~~~text
  73 taskspace_control
   2 exec_command
   1 list_agents
   1 close_agent
  every taskspace_control action: initialize_map
  ordinary command result: hard_state no_task_path
  ~~~
- Interpretation: Map 从未提交；close 不是循环起点，业务执行也未开始。
- Time: 2026-07-15 08:50

## Evidence E-002: schema 与 parser 对 Work 集合的表达不一致
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `tools/src/taskspace_tool.rs:219-246`; `core/src/tools/handlers/taskspace_control_args.rs:261-294`
- Prediction or plan link:
  - H-001 parser 节点集合构造
- Matched signal:
  - 字段级 schema 写 `Work nodes.`，parser 把 current Work 与该数组合并后校验唯一
- Correlation keys:
  - none
- Raw content:
  ~~~text
  all_nodes.push(root);
  all_nodes.push(current_work_node);
  all_nodes.push(finish);
  all_nodes.extend(work_nodes);
  validate_unique_nodes(&all_nodes, "initialize_map nodes")?;
  ~~~
- Interpretation: 工具结构没有忠实表达 parser 所要求的互斥集合。
- Time: 2026-07-15 08:53

## Evidence E-003: preflight 将 typed 错误降级为字符串
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `core/src/tools/sequence_manifest.rs:21-34`; `core/src/tools/sequence_preflight.rs:19-54`; rollout sequence 5-6
- Prediction or plan link:
  - H-002 handler 前解析及二次包装
- Matched signal:
  - `TaskSpaceControlResultR6V1` 出现在 `ToolSequencePreflightResultV1.error.message` 的转义字符串中
- Correlation keys:
  - call_id `call_00_w01taHfIItRMZSNoeYsM7449`
- Raw content:
  ~~~text
  taskspace_control call `call_00_w01taHfIItRMZSNoeYsM7449` is invalid:
  {\"schema_version\":\"TaskSpaceControlResultR6V1\",...}
  ~~~
- Interpretation: 反馈条目存在，但结构和错误所有权被 preflight 扭曲。
- Time: 2026-07-15 08:54

## Evidence E-004: 新合同与反馈所有权通过定向回归
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: fix-validation
- Source: `cargo test -p codex-tools`; `cargo test -p codex-core taskspace_control`; `cargo test -p codex-core tools::sequence`
- Prediction or plan link:
  - P-001 前三项 fix criteria
- Matched signal:
  - schema 只暴露 `initial_work_node` 和 `additional_work_nodes`；非法参数通过 sequence preflight 并保留给 handler；多 patch 仍被拒绝
- Correlation keys:
  - candidate worktree after `d12918e9d`
- Raw content:
  ~~~text
  codex-tools: 141 passed, 1 ignored
  taskspace_control: 16 passed
  tools::sequence: 13 passed
  just fix -p codex-tools: completed with pre-existing warning
  just fix -p codex-core: completed with pre-existing warnings
  just fmt: completed
  ~~~
- Interpretation: 本地合同和反馈链修复成立；是否消除 provider 循环仍等待原始 Docker 样本验证。
- Time: 2026-07-15 09:04

## Evidence E-005: 互斥初始化合同通过两个 Docker 样本
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports H-001/H-002 repair; refutes H-003
- Type: fix-validation
- Source: `target/r6-phase-c-epoch/simple/single-file-fast-fix/20260715-094309-889`; `target/r6-phase-c-epoch/branch-join/multi-file-order-pipeline/20260715-094519-735`
- Prediction or plan link:
  - P-001 live fix criteria
- Matched signal:
  - 两个 R6 arm 的 `initialize_map` 都提交成功，后续进入 ordinary tools 并最终显式终结
- Correlation keys:
  - simple run `20260715-094309-889`
  - branch-join run `20260715-094519-735`
- Raw content:
  ~~~text
  simple: initialize_map=1, task=completed, external=passed
  branch-join: initialize_map=1, task=completed, external=passed
  duplicate bootstrap rejects: 0
  ~~~
- Interpretation: Agent 不再被 schema 引导为重复声明 initial Work；成功来自合同表达修正，不是 Runtime 自动修复节点集合。
- Time: 2026-07-15 09:50
