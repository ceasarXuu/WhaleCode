# Problem P-001: R7 局部修复在五层整体约束之间反复制造回归
- Status: open
- Created: 2026-07-24 07:51
- Updated: 2026-07-24 08:26
- Objective: 把五层职责和五层改造后已修复问题冻结成一份可执行的整体非回归合同，并在同时满足
  行为、语义、边界、原生 Tool、缓存和成本约束的前提下修复当前固定 Tool schema 放大。
- Symptoms:
  - 两值轻量 binding 把 Tool section 降到 29,200 bytes，但无法结构化保证初始化与首个真实动作连续。
  - 初始化 ordinary Tool carrier 关闭独立初始化回归后，Tool section 又升到 55,578 bytes。
  - 当前实现相对两值轻量 binding 增加 90.3%，只比历史完整 carrier 低 8.5%。
- Expected behavior:
  - 任一修复方案在实施前必须通过完整约束矩阵，而不是只优化当前失败指标。
  - 初始化与首个真实动作保持一个 Agent 声明的连续动作，不恢复独立 control 初始化。
  - 初始化的一次性能力不得以当前冗长形态在整个会话中对所有普通 Tool 永久重复暴露。
  - Runtime 不推断任务语义、不选择动作、不自动建图或修正 Agent 参数。
  - Standard、普通 Tool 原生参数、反馈保真、三种 projection 共享基建和缓存可观测性不得回归。
- Actual behavior:
  - 现有计划分别为连续动作和 Tool schema 成本设置验收门，但缺少一个在每次变更时同时执行全部门禁的
    统一合同。
  - 当前普通 Tool decorator 无条件把完整初始化对象联合复制到所有可见普通 Tool，并在每次 TaskSpace
    provider request 中保持。
- Impact:
  - 简单修复在行为层通过、成本层失败；后续若直接动态切 schema，又可能重新引入缓存形状回归。
  - 缺少整体门禁使同一设计在 A/B 指标之间往返，增加实现、测试和真实模型运行成本。
- Reproduction:
  - 对比 Tool section：Standard 21,669 bytes；两值轻量 binding 29,200；初始化混合联合 51,934；
    当前判别对象联合 55,578；历史完整 carrier 60,743。
  - 当前正式矩阵：commit `b6bf532bf8b6d92d076b30d842e54c4f565fcfee`，
    run `20260724-065244-664`。
- Environment:
  - Linux；branch `whalecode-alpha`；DeepSeek `deepseek-v4-flash`；R7 五层架构。
- Known facts:
  - 初始化对象包含 Root、初始 Work、Finish、additional Work 和 edges。
  - `project_taskspace_binding_tool` 与 ToolSearch loadable projection 对普通 Tool 无条件应用同一 binding schema。
  - 当前工具列表有 13 项，其中中央 control 不装饰，初始化联合约复制到 12 个普通 Tool。
  - 三种 projection policy 共享同一 Tool 集合；Standard 不装饰。
  - 当前初始化行为修复已达到最终 carrier commit 18/18、直接 control 初始化 0。
- Ruled out:
  - 观测器误算：wire trace 的 Tool section bytes/hash 在每个 TaskSpace request 中稳定一致。
  - 请求数单独造成 schema 放大：固定 Tool section 在首请求和每个后续请求都已直接测量。
- Fix criteria:
  - 建立一份机器可读整体约束合同，逐项绑定 owner、禁止项、静态测试、运行时日志和 benchmark 指标。
  - 候选方案必须同时通过五层职责、连续动作、无 Runtime 语义决策、反馈保真、原生 Tool、Standard
    隔离、三策略共享、单 Patch、缓存和成本门。
  - 简单和复杂样本各执行同期 Standard + 三策略，不删除 Agent 错误样本。
  - 初始化最终 carrier commit、直接 control 初始化、首请求采用、Tool bytes/hash、cache shape transition、
    request/input/cached/uncached/wall 和 Map 闭合同时报告。
  - 不保留旧 schema/parser/fallback/feature flag。
- Current conclusion: 已确认的直接机制是完整初始化 schema 被永久复制到所有普通 Tool。整体约束合同已冻结
  动态 lifecycle schema、Runtime 自动骨架、恢复 sibling 和恢复通用 carrier 均不合法。剩余候选只能在
  immutable capability epoch 内压缩 Agent 显式声明的初始化 wire，且必须同时通过全部非回归门。
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

## Hypothesis H-001: 无条件 ordinary Tool 装饰是当前固定成本放大的直接根因
- Status: confirmed
- Parent: P-001
- Claim: 完整 `initialize_map` schema 被 `taskspace_binding_schema()` 复制到约 12 个普通 Tool，并在
  Map 已初始化后的每次请求继续暴露，造成相对两值 binding 的 26,378 bytes/request 回归。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 当前 decorator 不接收 Map 生命周期状态，固定返回包含初始化分支的对象联合。
- Falsifiable predictions:
  - If true: wire trace 中所有 TaskSpace request 的 tools hash/bytes 相同，且移除初始化分支的历史
    两值版本明显更小。
  - If false: 初始化后 wire Tool section 应不再包含初始化字段，或放大应主要来自其他 section。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查最终 wire Tool section 与 decorator 代码是否逐请求保留初始化联合。
  - Signal: Tool bytes、estimated tokens、hash 与 schema 构造代码。
  - Capture method: 对比历史和当前 provider wire trace，并读取 `taskspace_binding.rs`。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - subject commit
    - request index
    - tools hash
  - Differentiates from:
    - 请求路径波动或 observer 估算错误。
  - Supports if:
    - 当前每个 TaskSpace request 都是 55,578 bytes，代码无条件复用完整初始化 schema。
  - Refutes if:
    - 初始化后 schema 已机械收缩，或其他 section 才是主要固定增量。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留现有 wire section 与 tools hash 观测。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: 验证 H-005 固定 schema 候选。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 一次不可逆的机械 schema 生命周期切换可同时保住连续动作和长期成本
- Status: refuted
- Parent: P-001
- Claim: 空 Map 时保持初始化 ordinary Tool carrier，初始化提交后所有普通 Tool 机械切换为不含初始化
  分支的判别对象 binding，可避免永久复制；切换只依据 canonical Map 是否存在，不包含语义决策。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - H-001
- Rationale:
  - 初始化能力只在空 Map 可用；成功初始化后状态机本就永久拒绝再次初始化。
- Falsifiable predictions:
  - If true: 每个 TaskSpace run 只有一个 tools hash transition，之后稳定；初始化连续动作与 Standard
    隔离保持，复杂任务的累计 Tool input 明显下降。
  - If false: tool visibility 无法与同一 request 的 canonical 空 Map 状态一致，或一次 cache miss 的成本
    抵消后续节省并造成行为回归。
- Diagnostic evidence plan:
  - Prediction or clause under test: 用纯 schema probe 和缓存成本模型先验证形状、切换次数与盈亏边界。
  - Signal: bootstrap/active tools bytes、tools hash transition、request-2+ cache、初始化 carrier trace。
  - Capture method: 在不改生产行为前构造 schema 投影 probe；若通过再进入单变量实现。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - map existence
    - canonical revision
    - request index
  - Differentiates from:
    - H-003 紧凑 bootstrap 和 H-004 恢复跨 Tool sibling。
  - Supports if:
    - 生命周期切换纯机械、唯一、可观测，且简单/复杂成本门均优于当前基线。
  - Refutes if:
    - 需要 Runtime 选择业务 Tool/节点，或产生多次 shape 抖动，或不能通过缓存/行为门。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 若实施，保留 lifecycle tools hash transition 事件与计数。
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: refuted；“只切一次”仍然是由 Map 是否存在触发 Tool schema 变化。Map 状态不是 capability
  变化，不能创建新的 epoch；该方案违反静态合同并直接破坏缓存身份。
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - violates immutable capability epoch

## Hypothesis H-003: 紧凑机械 bootstrap 可在固定 schema 下满足全部约束
- Status: refuted
- Parent: P-001
- Claim: 将初始化输入收敛为语义最小字段，并由 Runtime 仅机械创建固定 Root/Work/Finish 骨架，可保持
  tools hash 静态并降低复制成本。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - H-001
- Rationale:
  - 早期 R5 原则允许 Runtime 做语义无关机械初始化，但 R6 rooted DAG 要求 Agent 显式维护图。
- Falsifiable predictions:
  - If true: 固定 schema 达到成本门，Agent 仍能在首动作表达足够目标并随后维护依赖图。
  - If false: 机械骨架替 Agent 决定了任务结构，导致 Map 退化、额外 mutate 请求或违反 R6 图合同。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对照 R5 机械初始化原则与 R6 Agent-owned rooted DAG 的最终权威合同。
  - Signal: authority 文档、初始 Map 形态、后续 mutate/request 路径。
  - Capture method: 文档所有权审计与静态 schema 尺寸 probe。
  - Event name or marker:
    - none
  - Correlation keys:
    - contract version
  - Differentiates from:
    - H-002 生命周期 schema。
  - Supports if:
    - 权威合同允许且 probe 同时通过 Map 质量与请求成本门。
  - Refutes if:
    - 固定骨架构成 Runtime 语义决策或只是把 schema 成本换成额外 map-maintenance request。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: refuted；本假设把固定 Root/Work/Finish 骨架交给 Runtime 创建，已经替 Agent 决定初始任务结构。
  机械实现不等于语义无关，违反 Agent-owned rooted DAG。固定 schema 下的 wire 压缩仍可继续，但必须完整保留
  Agent 对节点身份、目标、Finish 和边的显式声明，见 H-005。
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - violates Agent semantic ownership

## Hypothesis H-004: 恢复跨 Tool sibling 或通用 carrier 能降低成本且不回归
- Status: refuted
- Parent: P-001
- Claim: 恢复独立初始化 control + sibling，或完整 ordinary Tool carrier，可以在不破坏连续动作和原生 Tool
  的情况下解决当前成本。
- Layer: fix-validation
- Factor relation: alternative
- Depends on:
  - H-001
- Rationale:
  - 两条路径都已有历史实现和真实失败证据，可直接按整体约束复审。
- Falsifiable predictions:
  - If true: 历史证据应显示稳定首次采用、Patch 保真、低 schema 与无额外 request。
  - If false: sibling 继续允许 schema-valid standalone，或通用 carrier 继续复制大 schema、破坏 Patch/MCP。
- Diagnostic evidence plan:
  - Prediction or clause under test: 用历史 repeat、provider probe 和 schema 成本复审两条旧路径。
  - Signal: standalone transition、patch JSON fidelity、Tool bytes、request 数。
  - Capture method: 读取 D.2-D.4、FLA-3.5、FLA-9 证据。
  - Event name or marker:
    - none
  - Correlation keys:
    - historical subject commit
  - Differentiates from:
    - H-002 与 H-003。
  - Supports if:
    - 至少一条旧路径此前被误判，能通过全部现行门禁。
  - Refutes if:
    - 历史直接证据已命中当前禁止项。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: refuted；独立 sibling 允许 schema-valid、sequence-invalid 的 standalone 初始化并增加请求；
  通用 carrier 把完整 lifecycle schema 复制到普通 Tool，并破坏原生 Patch 形态。两者都直接命中已关闭回归。
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - historical evidence violates the integrated regression gates

## Hypothesis H-005: 固定的紧凑 Agent-authored 初始化 wire 可降低复制成本
- Status: unverified
- Parent: P-001
- Claim: 在不改变 capability epoch、不减少 Agent 声明的节点身份/目标/Finish/边、不改变同 call 初始化的前提下，
  将初始化对象改为更紧凑且仍由 JSON Schema 强约束的规范形态，可以降低每个普通 Tool 的复制成本。
- Layer: interaction
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - 当前成本来自同一结构在每个普通 Tool 内重复，而不是 Agent-owned graph 本身无价值。字段布局和重复描述属于
    L4 wire 机械表达，可在不改变 L5 语义的情况下优化。
- Falsifiable predictions:
  - If true: 空 Map 和已初始化 Map 的 tools hash 完全相同；初始化对象仍能无损转换为同一内部
    `TaskSpaceControlArgs::InitializeMap`；Tool section 明显低于 55,578 bytes。
  - If false: schema 只能通过字符串化、弱类型、Runtime 自动补图或额外 provider request 才能变小，或真实模型
    产生初始化/Map 质量回归。
- Diagnostic evidence plan:
  - Prediction or clause under test: 先构造固定 schema 尺寸与 parser fixture，不修改生产执行路径。
  - Signal: schema bytes/hash、字段完备性、valid/invalid parser fixture、initialization carrier outcome。
  - Capture method: schema probe、Rust 定向测试、再进入 Docker simple/complex 四臂。
  - Event name or marker:
    - `taskspace.provider_tool_schema_profile`
    - `taskspace.initialization_carrier_outcome`
  - Correlation keys:
    - capability_set_hash
    - tools_hash
    - request_id
  - Differentiates from:
    - H-002 动态 schema 和 H-003 Runtime 自动骨架。
  - Supports if:
    - 通过整体合同 G-01 至 G-12，且当前固定 Tool bytes 明确下降。
  - Refutes if:
    - 任一已关闭回归重新出现，或成本只转移为额外 request/input。
  - Instrumentation status: existing-observability-sufficient
  - Instrumentation lifecycle:
    - 保留 schema profile、carrier outcome 和 provider wire trace。
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-002
  - E-004
- Conclusion: unverified
- Repair design readiness: ready for static schema probe
- Next step: 比较不含 Runtime 语义补全的固定初始化 wire 候选。
- Blocker:
  - 候选 schema 尚未通过 G-01 至 G-12。
- Close reason:
  - not closed

## Evidence E-001: 当前 wire Tool section 固定为 55,578 bytes
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: matrix run `20260724-065244-664` 的 260 个 TaskSpace provider wire request
- Prediction or plan link:
  - H-001 对逐 request 固定 schema 的预测。
- Matched signal:
  - 三种 TaskSpace arm 的每个 request 都为 13 tools、55,578 bytes、13,895 estimated tokens、
    同一 tools hash。
- Correlation keys:
  - subject commit `b6bf532bf8b6d92d076b30d842e54c4f565fcfee`
  - tools hash `2125a1b8fbf6efa02b70fe9b28d4f14298cd905873388b72a3427704f9b23da5`
- Raw content:
  ```text
  TaskSpace tools: count=13 bytes=55578 estimated_tokens=13895
  Standard tools: count=13 bytes=21669 estimated_tokens=5418
  ```
- Interpretation: 放大来自 provider 最终 wire schema，不是 observer 推算或自然历史。
- Time: 2026-07-24 07:51

## Evidence E-002: 初始化联合由普通 Tool decorator 无条件复制
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/tools/src/taskspace_binding.rs`
- Prediction or plan link:
  - H-001 对无生命周期输入的 decorator 预测。
- Matched signal:
  - `project_taskspace_binding_tool` 和 loadable Tool 路径最终都调用不接收 Map 状态的
    `taskspace_binding_schema()`；该 schema 始终包含 `initialize_map_schema()`。
- Correlation keys:
  - current branch `whalecode-alpha`
- Raw content:
  ```text
  properties.insert(TASKSPACE_BINDING_FIELD.into(), taskspace_binding_schema());
  JsonSchema::object_any_of(vec![initialize_map_schema(), active, after_boundary], ...)
  ```
- Interpretation: 一次性初始化能力被设计成会话全程固定的普通 Tool schema 成员。
- Time: 2026-07-24 07:51

## Evidence E-003: 历史方案已暴露相互独立的回归
- Related hypotheses:
  - H-002
  - H-003
  - H-004
- Direction: neutral
- Type: observation
- Source: R7 D.2-D.5、FLA-3.5、FLA-8、FLA-9 文档与 COE
- Prediction or plan link:
  - 候选淘汰前先冻结历史事实。
- Matched signal:
  - sibling 路径存在稳定 standalone control；通用 carrier 存在 schema 放大和 nested Patch fidelity
    问题；两值 binding 成本低但初始化结构缺位；当前初始化 carrier 行为正确但固定成本回归。
- Correlation keys:
  - historical R7 phases
- Raw content:
  ```text
  sibling: schema-valid, sequence-invalid
  full carrier: 60,743 bytes/request
  two-value binding: 29,200 bytes/request
  current initialization carrier: 55,578 bytes/request
  ```
- Interpretation: 不能直接回退任何一个旧实现；必须先用统一合同复核。
- Time: 2026-07-24 07:51

## Evidence E-004: 整体合同冻结静态 epoch 与历史非回归约束
- Related hypotheses:
  - H-002
  - H-003
  - H-004
  - H-005
- Direction: differentiates
- Type: contract
- Source:
  - `docs/v0.0.5/build-R7/23-r7-taskspace-five-layer-architecture-design.md`
  - `docs/v0.0.5/build-R7/38-r7-five-layer-integrated-change-constraints.md`
- Prediction or plan link:
  - 候选必须同时满足静态 Tool schema、Agent 语义所有权、连续动作和历史非回归门。
- Matched signal:
  - C-06 明确 Map revision、空/非空和 lifecycle state 不能改变同一 capability epoch 的 Tool schema。
  - C-02/C-08 明确 Runtime 不能创建任务拓扑或选择节点。
  - R-08 至 R-18 冻结 sibling、optional binding、nested Patch、Standard 污染、full carrier 和独立初始化的修复。
- Correlation keys:
  - integrated contract v1
- Raw content:
  ```text
  Map state cannot create a capability epoch.
  Runtime cannot author task topology.
  A candidate must pass every closed-regression gate.
  ```
- Interpretation: H-002/H-003/H-004 无需进入生产 probe 即可由权威合同证伪；H-005 是当前唯一未被原则预先
  淘汰的候选方向。
- Time: 2026-07-24 08:26
