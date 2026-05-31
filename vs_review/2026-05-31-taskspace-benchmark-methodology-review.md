# Subagent VS Review: TaskSpace Benchmark 与 BaseMap 拆解方法论

- Created: 2026-05-31T00:00:00+08:00
- Updated: 2026-05-31T00:00:00+08:00
- Task: 落地两份设计文档：更贴近复杂状况的 TaskSpace 分层 E2E/benchmark，以及 BaseMap 拆解方法论如何在主 agent 生成/更新 map 时注入。
- Report path: `vs_review/2026-05-31-taskspace-benchmark-methodology-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: passed

## Round 1: Design Document Review

### Review Input

#### Objective

审查两份新增设计文档是否足够可落地，是否符合 TaskSpace 产品定位，是否避免过度设计，并能支撑后续工程实施和真实 E2E 压力测试。

#### Review Target

文档设计、测试策略、agent workflow、runtime 约束边界。

#### Target Locations

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`
- `docs/plans/2026-05-31-basemap-decomposition-methodology.md`

#### Change Introduction

新增两份文档：

- TaskSpace 分层 E2E 与 Benchmark 设计：按 L1/L2/L3/L4 复杂度和工程可行性/效用两类指标组织测试体系。
- BaseMap 拆解方法论注入设计：定义主 agent 在 map 创建、更新、reborn 时如何按事实面、风险面、产物面拆解任务，并通过轻量 runtime gate 防止线性退化。

#### Risk Focus

- Benchmark 是否仍然偏人工构造，无法证明真实复杂任务收益。
- L1/L2/L3/L4 指标是否过硬或过软，是否会诱导 agent 为指标而造图。
- BaseMap 方法论是否过度约束主 agent，导致简单任务变慢或复杂任务伪结构化。
- runtime 与主 agent 的责任边界是否清晰，是否让 runtime 做了语义判断。
- 是否遗漏了重复阅读、subagent 结果利用、边依赖健康、standard 对照、可观测性的关键验收点。
- 是否存在与现有 TaskSpace 文档冲突的概念或命名。

#### Verification Status

- 文档已新增，尚未进入工程实施。
- 未运行代码测试；本轮目标是设计审查。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Give blocking findings only for issues that would materially mislead implementation or make the design fail its product goal.

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| TaskSpace Design Adversary | 需要从产品哲学、benchmark 有效性、主 agent 调度约束三个角度攻击文档 | 测试有效性、过度设计、runtime/agent 边界 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| TaskSpace Design Adversary | multi_agent_v1.spawn_agent | `019e7cec-863e-7dc0-9b6c-93b4f64014af` | spawn_agent + subagent_notification | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### TaskSpace Design Adversary

##### Summary

只读审查完成。两份文档方向总体正确：它们明确区分 TaskSpace 的机制健康与业务成功，覆盖了 L1 防拖累、L2/L3 图健康、L4 长程会话，并且 BaseMap 文档主动压制重复阅读、伪并行、runtime 语义越权。

但有两个会误导后续实施的 blocking：一是 benchmark 没有把“自建场景机制验证”和“真实复杂任务收益证明”硬隔离；二是 BaseMap 文档在 runtime 边界上同时说“不做语义判断”，又要求 runtime 校验“合理/必要事实面”，容易把语义判断塞进 runtime。

##### Blocking Findings

- Benchmark 缺少“收益声明门槛”，会让第一阶段自建场景被误用为真实复杂任务收益证明。文档虽承认当前还不能证明复杂真实任务稳定净收益，但没有硬性规定：未跑 paired standard、真实历史 corpus/外部 benchmark、重复运行统计前，不得声称 TaskSpace 有净收益。
- Runtime/主 agent 责任边界存在冲突，可能导致 runtime 做语义判断。BaseMap 文档说 runtime 不做语义评判，只做轻量结构检查，但又要求 runtime 校验“新 node 必须有合理上游依赖”“implementation 不能绕开必要事实面”，这两个判断不是纯结构条件。

##### Non-blocking Risks

- L2/L3 的 edge 指标仍可能被 agent 通过“造边”满足；缺少 dependency reason 或 consumed result evidence。
- L1 成本目标偏软，没有定义阈值。
- BaseMap 的“每次创建、更新、reborn map 都必须看到方法论”如果注入体积不受控，会拖累 L1/L2。

##### Required Fixes

- 给 benchmark 增加证据等级：`mechanism smoke`、`constructed regression`、`paired utility`、`real-world utility`。明确只有 paired standard/taskspace + hidden oracle + 历史真实失败/外部 benchmark + 多次运行统计，才允许说“收益”。
- 把 BaseMap runtime 校验改成纯结构表述；“合理/必要事实面”只能作为 agent prompt/review smell，不能作为 runtime hard gate。
- 为关键边增加可观测字段或报告字段：`edge_reason`、`from_result_id`、`consumed_by_node_id`、`dependency_kind`。

##### Missing Tests / Benchmark Gaps

- 缺少 paired standard/taskspace 的统一评分 oracle：关键文件覆盖、禁止修改文件、产品真相选择、隐藏测试、漏项分类。
- 缺少 L1 明确成本阈值。
- 缺少重复运行策略。
- L4 压缩场景缺少实际 harness 设计。

##### Missing Logs / Observability

- 需要记录 `NodeResultConsumed` 或等价事件，否则无法证明主 agent 实施前真的使用了 completed node result。
- 需要导出 per-node `source_refs`、source overlap、重复阅读原因。
- 需要 edge 依赖健康日志：边创建时间、创建者、依赖理由、是否被下游引用。

##### Evidence

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：当前文档承认机制可行不等于真实收益，但缺少证据等级门槛。
- `docs/plans/2026-05-31-basemap-decomposition-methodology.md`：runtime 边界处存在“合理/必要事实面”这类语义判断表述。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| TaskSpace Design Adversary | Benchmark 缺少收益声明门槛 | blocking | accept | 自建场景通过确实只能证明机制和构造回归，不能证明真实产品收益 | 在 benchmark 文档新增 E0/E1/E2/E3 证据等级，规定 E0/E1 不得宣称净收益；只有 E2/E3 才能分别声明构造任务收益和真实复杂任务收益证据 | Round 2 re-review |
| TaskSpace Design Adversary | Runtime/主 agent 责任边界存在语义判断冲突 | blocking | accept | “合理上游依赖”“必要事实面”会把语义判断推给 runtime | 将 runtime 校验改为纯结构条件：edge 引用有效、无环、kind/status/lease、implementation 入边、validation 入边；明确 runtime 不判断事实面是否必要 | Round 2 re-review |
| TaskSpace Design Adversary | edge 指标可能被造边满足 | non-blocking | accept | 只有 edge_count 不足以证明下游使用上游结果 | 新增 key edge reason、NodeResultConsumed、dependency_kind、source refs/overlap 报告要求 | Round 2 re-review |
| TaskSpace Design Adversary | L1 成本目标偏软 | non-blocking | accept | “不明显劣化”无法进入自动化判断 | 新增 L1 初始阈值：wall_time/tool_call/token ratio <= 1.5，超过标记成本劣化 | Round 2 re-review |
| TaskSpace Design Adversary | 方法论注入体积不受控 | non-blocking | accept | 低复杂度任务不应被长 prompt 拖累 | 新增注入裁剪规则：L1 轻量规则，L2/L3 完整方法论，L4 额外上下文边界规则 | Round 2 re-review |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 2 pending
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no

## Round 2: Blocking Closure Review

### Review Input

#### Objective

验证 Round 1 已接受的 blocking findings 是否已经在文档中闭合，并检查修正是否引入新的 blocking 风险。

#### Review Target

文档设计、测试策略、agent workflow、runtime 约束边界。

#### Target Locations

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`
- `docs/plans/2026-05-31-basemap-decomposition-methodology.md`
- `vs_review/2026-05-31-taskspace-benchmark-methodology-review.md`

#### Change Introduction

根据 Round 1 审查：

- Benchmark 文档新增 E0/E1/E2/E3 证据等级和收益声明门槛。
- Benchmark 文档新增 L1 成本阈值、paired 对照重复运行策略、统一 oracle、key edge reason、NodeResultConsumed、source refs/overlap、L4 harness 草案。
- BaseMap 文档新增方法论注入裁剪规则。
- BaseMap 文档将 runtime 校验改成结构条件，明确 runtime 不判断必要事实面。
- BaseMap 文档新增 EdgeReason 与 NodeResultConsumed 观测概念。

#### Risk Focus

- Round 1 两个 blocking 是否闭合。
- 新增证据等级是否清楚阻止“构造场景通过即宣称真实收益”。
- runtime 边界是否已经避免语义判断。
- 新增 edge/result observability 是否足够支撑后续实施。
- 是否出现新的过度设计或不可落地要求。

#### Verification Status

- 文档已修改。
- 未做代码实现；本轮仍是设计闭合审查。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- 只把仍会误导实施或让设计目标失败的问题列为 blocking。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| TaskSpace Closure Reviewer | Accepted blocking fixes require fresh closure review | Blocking closure、runtime/agent boundary、benchmark evidence gate |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| TaskSpace Closure Reviewer | multi_agent_v1.spawn_agent | pending | pending | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### TaskSpace Closure Reviewer

##### Summary

Round 1 的两个 blocking findings 已经闭合。Benchmark 文档现在明确区分 E0/E1 机制证据与 E2/E3 收益证据，且禁止 E0/E1 宣称真实复杂任务净收益。BaseMap 文档也把 runtime hard gate 收敛到结构条件，语义拆解仍归主 agent/review/E2E smell 处理。

##### Blocking Findings

none

##### Non-blocking Risks

- BaseMap 里仍有少量“巨大 inspect node”“宽任务”这类表述，如果实现时没有工具调用数、持续时长、source overlap 等量化依据，可能让 runtime gate 变得主观。但当前文档后续已把主要 hard gate 写成 edge/kind/status/lease/无环/入边检查，不构成 blocking。
- `vs_review` 文档的 Round 2 输出和 closure status 仍是 pending；这是流程记录未落地的问题，不是两份设计文档本身的 blocking。

##### Required Fixes

none for blocking closure.

##### Evidence

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：新增 E0/E1/E2/E3 证据等级，并明确 E0/E1 不能宣称净收益，E2 才支持构造任务收益，E3 才支持真实复杂任务收益证据。
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：E2 起要求 standard/taskspace paired 对照、统一 oracle、重复运行和成本/漏项统计。
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：报告要求包含证据等级、key edge reason、NodeResultConsumed 等价证据、source refs/overlap，并禁止只用 edge_count 解释健康依赖。
- `docs/plans/2026-05-31-basemap-decomposition-methodology.md`：runtime 校验列为 node 存在、current main node、edge 引用有效、无环、入边、subagent 绑定 ready/running inspect node 等结构条件，并明确 runtime 不判断必要事实面。
- `docs/plans/2026-05-31-basemap-decomposition-methodology.md`：runtime 应做结构约束和 observability，不做关键词路由、语义完成判断、node 质量打分或 task 意图判断。
- `docs/plans/2026-05-31-basemap-decomposition-methodology.md`：EdgeReason 与 NodeResultConsumed 的观测概念足够支撑后续实施。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| TaskSpace Closure Reviewer | Round 1 两个 blocking 已闭合 | blocking closure | accept | Reviewer 明确 blocking findings 为 none | 保持已修正文档 | n/a |
| TaskSpace Closure Reviewer | “巨大 inspect node/宽任务”需要量化依据，避免 gate 主观化 | non-blocking | accept | 该风险会影响后续实现，但不影响设计闭合 | 在 BaseMap 文档新增退化信号处理等级、数据来源、初始阈值，并明确 smell 不做 hard gate | n/a |
| TaskSpace Closure Reviewer | Round 2 报告 pending | non-blocking | accept | 审查报告流程记录需要补齐 | 已补齐 Round 2 reviewer output、main response、closure status | n/a |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: n/a
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2
- Blocking re-review launch records:
  - TaskSpace Closure Reviewer: `019e7cf1-132b-7572-a7b1-eedead180789`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes

## Final Conclusion

The two design documents may proceed. Round 1 blocking findings were accepted and fixed; Round 2 fresh closure review found no blocking issues. Remaining risks are implementation-level clarifications around threshold tuning and smell-vs-gate classification, now documented in the BaseMap methodology.

## Round 3: Benchmark Engineering Detail Review

### Review Input

#### Objective

审查新增的 E2E benchmark 工程实施细节是否足够具体、可落地，并能控制变量保证 standard/taskspace 对照有效。

#### Review Target

测试工程设计、paired run 协议、变量控制、oracle、指标抽取、报告、聚合、flake 重跑规则。

#### Target Locations

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`

#### Change Introduction

在 benchmark 文档中补充工程实施细节：

- 统一 harness 目录结构。
- scenario manifest 契约。
- run artifact 目录。
- paired standard/taskspace run 协议。
- 变量控制和污染禁止项。
- public validation、hidden oracle、TaskSpace structural oracle。
- metrics.json 字段。
- pair report 和 aggregate report。
- flake 与重跑规则。
- 外部 benchmark 薄适配原则。

#### Risk Focus

- paired 对照是否真的只改变 mode，不改变其他变量。
- scenario manifest 是否足够支撑 L1/L2/L3/L4 和 E2/E3。
- oracle 分层是否会泄漏内部结构或污染用户 prompt。
- metrics/report 是否能支撑失败归因。
- flake 重跑规则是否避免刷成功样本。
- 是否引入过度复杂、难以第一阶段落地的 harness。

#### Verification Status

- 文档已修改。
- 未实施代码；本轮审查工程方案可落地性。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target file directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- 只把仍会导致对照无效、收益判断失真或工程无法落地的问题列为 blocking。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Benchmark Harness Reviewer | 新增内容主要是测试工程和变量控制，需要独立攻击 paired 对照有效性 | paired validity、oracle、artifact、flake policy |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Benchmark Harness Reviewer | multi_agent_v1.spawn_agent | pending | pending | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Benchmark Harness Reviewer

##### Summary

文档整体方向是可落地的，尤其是 paired run、变量控制、oracle 分层、artifact、pair report、flake 规则都有明确骨架。主要问题集中在三处：paired 对照的“只改变 mode”还没有被工程化成可验证的不变量；hidden oracle 的隔离规则还停留在声明层；Scenario Manifest 目前只覆盖单轮场景，尚不足以承载 L4 和 E3 外部 benchmark。

##### Blocking Findings

- Paired 对照仍可能改变 agent 可见变量，导致“only mode differs”不成立。路径名 `standard/repo` 与 `taskspace/repo` 可能通过 cwd、日志、命令输出被 agent 看见；模型配置也缺少 temperature/top_p/max output/reasoning budget/tool set/system prompt 版本等逐项固定或 delta 声明。
- Hidden oracle 隔离没有工程边界，可能污染 prompt 或被 agent 读取。scenario 结构包含 `oracle/`，manifest 指向 `oracle/hidden_oracle.py`，旧报告格式还会产出 `hidden_oracle.py`。
- Scenario Manifest 不能完整支撑 L4 和 E3。当前 manifest 是单 prompt、单 fixture、单 validation、单 oracle；L4 需要 turns、压缩、resume assertions；E3 外部 benchmark 需要 original sample id、validator/container/resource/network policy、privacy scrub、human review metadata。

##### Non-blocking Risks

- Flake 规则方向正确，但 aggregate denominator 还不清楚。
- Metrics 有 boolean/count 字段，但缺少每个指标的 evidence pointer。
- Harness 目录设计不算过度复杂，但第一阶段应明确 MVP 边界。

##### Required Fixes

- 增加 `mode_delta_contract`。
- 将 run workspace 改成 agent 不可区分的中性路径，例如 `left/repo` 与 `right/repo`。
- `manifest.resolved.json` 必须记录并比较完整 provider/model params。
- 定义 hidden oracle filesystem contract。
- 扩展 manifest schema：支持 `turns[]`、`compaction_trigger`、`resume_assertions`、`external_benchmark`、`validator`、`resource_policy`、`human_review_required`。

##### Missing Tests / Harness Gaps

- `variable-control self-test`
- `hidden-oracle-isolation test`
- L4 manifest fixture
- flake lineage fixture
- external benchmark dry-run fixture

##### Evidence

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：paired run、变量控制、manifest、oracle、metrics、aggregation、external adapter、L4 harness 草案。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Benchmark Harness Reviewer | Paired 对照可能改变 agent 可见变量 | blocking | accept | `standard/taskspace` 路径名和不完整模型参数会破坏 only-mode-diff | 改为 `left/right` 中性路径，新增 `logical-mode-map.json`；新增 mode delta contract；要求记录完整 model params、tool policy、cwd policy，非 allowed delta 一律 invalid pair | Round 4 closure |
| Benchmark Harness Reviewer | Hidden oracle 隔离没有工程边界 | blocking | accept | oracle 进入 agent 可见目录会污染 business_success | 改为 `private-oracle` 和 `reviewer-only/private-oracle`；明确 agent cwd 只能是 left/right repo；hidden oracle 不复制进 repo/artifacts，只记录 stdout/stderr/exit/sha256 | Round 4 closure |
| Benchmark Harness Reviewer | Manifest 不支撑 L4/E3 | blocking | accept | 单 prompt manifest 不能表达多 turn、压缩、外部 benchmark 元数据 | 扩展 manifest：`turns[]`、`compaction_trigger`、`resume_assertions`、`external_benchmark`、`validator`、`resource_policy`、`privacy_scrub`、`human_review_required` | Round 4 closure |
| Benchmark Harness Reviewer | aggregate denominator 不清楚 | non-blocking | accept | 重跑/排除规则会影响收益统计 | 新增 utility aggregate 纳入条件、excluded pairs、rerun lineage | Round 4 closure |
| Benchmark Harness Reviewer | metrics 缺 evidence pointer | non-blocking | accept | boolean 指标不可追溯 | 新增 `evidence` 和 `taskspace_evidence` 字段示例 | Round 4 closure |
| Benchmark Harness Reviewer | harness MVP 边界 | non-blocking | accept | 一次性全做可能拖慢落地 | 新增 harness self-test，并保留现有 E1 脚本迁移路径；MVP 仍以 paired run + artifact + report 闭环为中心 | Round 4 closure |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Blocking re-review round links:
  - Round 4 pending
- Blocking re-review launch records:
  - pending
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: no

## Round 4: Benchmark Engineering Closure Review

### Review Input

#### Objective

验证 Round 3 accepted blocking fixes 是否闭合：paired only-mode-diff、hidden oracle isolation、L4/E3 manifest 支撑。

#### Review Target

测试工程设计、paired run 协议、变量控制、oracle、manifest、aggregation。

#### Target Locations

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`
- `vs_review/2026-05-31-taskspace-benchmark-methodology-review.md`

#### Change Introduction

根据 Round 3 审查：

- 新增 `mode_delta_contract`。
- run workspace 改为 agent 不可区分的 `left/repo` 与 `right/repo`，logical mode 写入 reviewer-only mapping。
- `manifest.resolved.json` 要求记录完整 model params、tool policy、prompt/fixture checksum、cwd policy。
- 新增 hidden oracle filesystem contract，private oracle 不进入 agent repo。
- 扩展 manifest schema 支持 L4 turns/compaction/resume assertions，以及 E3 external benchmark/resource/privacy/human review。
- 新增 utility aggregate 纳入条件、rerun lineage、evidence pointer、harness self-tests。

#### Risk Focus

- paired 对照是否现在能工程化验证 only-mode-diff。
- hidden oracle 是否不会被 agent 读取。
- manifest 是否能覆盖 L4 和 E3。
- 是否还有 blocking 级变量污染或收益统计失真。

#### Verification Status

- 文档已修改。
- 未做代码实现；本轮审查 closure。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- 只把仍会导致对照无效、收益判断失真或工程无法落地的问题列为 blocking。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Benchmark Closure Reviewer | Round 3 accepted blocking fixes require fresh closure review | paired validity、oracle isolation、manifest coverage |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Benchmark Closure Reviewer | multi_agent_v1.spawn_agent | pending | pending | no | Round 4 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Benchmark Closure Reviewer

##### Summary

上一轮 3 个 blocking fixes 已经在 TaskSpace benchmark 文档中闭合。当前没有仍会导致 paired 对照无效、收益判断失真或工程无法落地的 blocking 问题。

##### Blocking Findings

none

##### Non-blocking Risks

- `vs_review` 的 Round 4 记录仍停在 pending 状态，Reviewer Outputs、Main Agent Response、Closure Status 都未落盘更新。这是审查记录完整性问题，不是 benchmark 方法论本身的 blocking。
- hidden oracle 允许在 sandbox 暂时无法硬隔离时退化为 denylist + isolation level 记录；文档已要求 isolation failure 不进入 aggregate，但实现时必须把这个检查做成硬门禁，否则会回归成 oracle 泄漏风险。

##### Required Fixes

none for blocking closure.

##### Evidence

- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：`mode_delta_contract` 已新增，并明确 allowed deltas 只限 `taskspace_flag`、runtime behavior、structural tools、observability export；同时禁止 prompt、fixture、model params、permissions、output budget、agent-readable path label、hidden oracle visibility 差异。
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：run workspace 已改为 `left/repo` 与 `right/repo`，并通过 `logical-mode-map.json` 记录真实模式；`left/right` 是 agent 可见路径中唯一 mode 区分。
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：`manifest.resolved.json` 已要求记录 model params、tool policy、permissions/sandbox、prompt/fixture checksum、cwd policy；任一非 allowed delta 不一致会标记 `invalid_pair`。
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：hidden oracle 已放到 `private-oracle` / `reviewer-only/private-oracle`，agent cwd 只能是 `left/repo` 或 `right/repo`，private oracle 不复制进 agent repo。
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：manifest 已支持 L4 的 `turns[]`、`compaction_trigger`、`resume_assertions`，并说明当前 CLI 不支持强制压缩时必须标为 harness gap。
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：E3 manifest 已补充 `external_benchmark`、`validator`、`resource_policy`、`privacy_scrub`、`human_review_required`。
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`：utility aggregate、rerun lineage、evidence pointers、harness self-tests 已覆盖收益统计防失真。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Benchmark Closure Reviewer | Round 3 三个 blocking 已闭合 | blocking closure | accept | Reviewer 明确 blocking findings 为 none | 保持已修正文档 | n/a |
| Benchmark Closure Reviewer | Round 4 报告 pending | non-blocking | accept | 审查轨迹需要完整落盘 | 已补齐 Round 4 outputs、main response、closure status | n/a |
| Benchmark Closure Reviewer | hidden oracle isolation 降级实现必须硬门禁 | non-blocking | accept | 如果 denylist 降级未被标成 isolation failure，会重新污染收益统计 | 在后续实现约束中按文档执行：oracle isolation failure 不进入 utility aggregate；harness self-test 必须覆盖 hidden-oracle-isolation | implementation follow-up |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: n/a
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 4
- Blocking re-review launch records:
- Benchmark Closure Reviewer: `019e7d11-4376-71b3-ae9d-be801867667e`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes
