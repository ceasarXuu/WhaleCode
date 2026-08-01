# Subagent VS Review: R6 Phase C 生产纵向切换

- Created: 2026-07-15T17:18:18+08:00
- Updated: 2026-07-15T17:29:08+08:00
- Report schema: adversarial-v1
- Task: 对已经声明完成的 R6 Phase C 生产纵向切换执行独立对抗性审查。
- Report path: `vs_review/2026-07-15-r6-phase-c-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context; reviewer receives only the review packet
- Status: passed

## Round 1: 生产实现完整性审查

### Review Input

#### Objective

验证 R6 Phase C 是否真正把 TaskSpace 生产链路统一到 Rooted DAG 状态机，同时遵守以下产品边界：
Runtime 只执行机械硬约束和忠实存储；Agent 决定目标、拓扑、动作与终结；projection 不进行语义重写、
建议或策略注入；不保留旧模型兼容层。

#### Review Target

已实现的代码、工具合同、持久化/恢复、projection/cache、日志/observer、测试与阶段完成证据。

#### Target Locations

- `docs/v0.0.5/build-R6/00-r6-rooted-dag-state-machine-charter.md`
- `docs/v0.0.5/build-R6/01-r6-phased-implementation-plan.md`
- `docs/v0.0.5/build-R6/05-r6-phase-c-result.md`
- `benchmarks/taskspace/r6/phase-c-result.json`
- `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/event_store.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/projection.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control*.rs`
- `third_party/codex-cli/codex-rs/core/src/client.rs`
- `third_party/codex-cli/codex-rs/core/src/session/`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs`
- `scripts/taskspace-benchmark/test-r6-rooted-dag-contract.ps1`
- `scripts/action-map-*.ps1` 和 `scripts/taskspace-benchmark/lib/`
- commits `f81e96e0b..fa6b6ff18`，仅作为导航，不以 diff 代替直接读生产代码

#### Change Introduction

Phase C 声称把 Phase B 的 Rooted DAG 领域核心接入生产 schema、handler、runtime、Event Store、
snapshot、projection 和 observer；删除旧 Task/Map 双完成权威和零边 schema；修复 bootstrap/反馈循环；
把逐请求动态 projection 替换为每个 context epoch 一份基线加原始 control journal。

#### Risk Focus

- 是否仍有旧状态、旧 schema、root 推断、terminal-list 或旁路 completion 在生产可达路径中。
- 初始化、mutation、transition、finish 是否真的候选校验后原子提交，拒绝是否可能部分写入。
- Root 是否可能提前闭合、Finish 是否可能自动闭合，恢复/压缩后是否产生不同权威。
- epoch 基线与原始 control journal 是否足以让 Agent 看见当前状态，是否在 compaction/resume 后丢失或重复。
- 工具 schema 与 parser/handler 是否一致，失败回执是否忠实、单层、可恢复且没有策略性建议。
- observer、机器结果和 Docker 样本是否可能只验证实现叙事，而未验证真实生产入口。
- Phase C 完成声明是否把 Phase D/E/F/H 的未落地项目误计为已完成。

#### User-Perspective Review Focus

- 将 Agent 视为 `taskspace_control` 的直接用户：schema 是否容易正确初始化、推进和终结 Map。
- 参数错误、revision 冲突和 invariant reject 后，原始机械事实是否足以让 Agent自行恢复。
- 投影是否让 Agent 保持全局图视图，同时不会因重复、过期或缺失状态产生误判。

#### Implementation Completeness Focus

- 对照 Phase C 七个实施项和退出门禁，逐项确认 production path、integration entry、测试与 runtime 证据。
- 特别识别 test-only、schema-only、observer-only、mock-only 或文档声明替代生产接线的情况。
- 区分 Phase C 必须完成项与明确留给 Phase D/E/F/G/H 的工作，不把后续项误报为 Phase C blocker。

#### Target Benefit Focus

- 正确性收益：消除双重状态、零边 Map、reject loop 和反馈扭曲。
- 可靠性收益：唯一 source/sink、原子提交、显式 terminal、旧 session 明确 fatal。
- 缓存收益：修复前简单样本 request 2+ cache 0.32%，修复后 simple/branch-join 为
  91.28%/92.47%，精确扫描 78/78；检查方法和归因是否成立。
- 成本风险：simple 的 R6 request/input 仍为 Standard 的 2.17x/2.65x；当前只允许记录为待重基线，
  不得把单次 branch-join 的正收益泛化为总体收益。

#### Assumptions To Attack

- `taskspace_control` 是不可绕过的唯一 Map 变更入口。
- Event Store linearization 是 provider-visible 上下文的唯一历史 owner。
- 一份 epoch snapshot 加原始 control call/output 可无损表达当前 Map。
- legacy schema rejection 没有兼容或 silent fallback。
- live harness 使用的 attested binary 与当前候选生产代码一致。
- graph observer 的节点/边/状态来自真实 snapshot，不是从日志猜测出的第二套模型。

#### Adversarial Lenses

- implementation-completeness
- state
- failure
- data
- concurrency
- maintenance
- testing
- observability
- usability
- comprehension
- target-benefit

#### Verification Status

- focused Rust tests、schema/control/sequence/reconstruction tests 和 locked Whale build 已通过。
- simple 与 branch-join 的 Standard/R6 Docker pair 各 1 次通过外部验证。
- 两个 R6 run 的 exact payload scan 共 78/78，active projection 恒为 1。
- 完整 workspace/release suite、Phase D 动态 fork/join、Phase E crash/resume/fork 矩阵、Phase F 上下文
  ownership 收敛和 Phase G 三次成本重基线尚未执行。

#### Reviewer Instructions

- Fresh internal subagent session; use role `implementation-completeness-adversary`.
- No inherited main-agent context. Read target files directly. Do not modify files.
- Try to falsify the completion claim; do not confirm it by default.
- Cite repository-relative evidence paths and line numbers whenever possible.
- Separate Phase C blockers from valid later-phase work and non-blocking benefit warnings.
- Return exactly these sections: Summary; Blocking Findings; Non-blocking Risks; User-Perspective Checks;
  Implementation Completeness Checks; Target Benefit Checks; Required Fixes; Missing Tests;
  Missing Logs / Observability; Evidence.
- For every actionable finding include broken assumption, trigger/failure scenario, impact, proof needed, affected plan
  item and production path. Use `none` explicitly when a section has no finding.

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User-recommended agent command: n/a
- User-recommended agent verification: n/a
- User approval requested: n/a
- User-approved CLI command: n/a
- User decision: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 minutes | one bounded 10-minute extension | 2 | accepted blocker requires a fresh closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | Phase C 已声明完成，最高风险是生产接线不完整或证据只覆盖测试/observer | 生产入口、状态权威、持久化、工具反馈、测试与完成证据 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | `multi_agent_v1.spawn_agent`, fresh `explorer`, `gpt-5.5` low | `019f6513-a135-7f13-9f93-8510c1c30cc2` (`Hegel`) | spawn result in main session tool transcript | `fork_context=false` | Round 1 Review Input; spawn envelope points reviewer to that exact section | main-agent history, reasoning, drafts, conclusions, full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1-implementation-completeness | implementation-completeness-adversary | 1 | `019f6513-a135-7f13-9f93-8510c1c30cc2` | 1m27s | completed | fresh reviewer returned all required sections | completed |

### Reviewer Outputs

#### R1-implementation-completeness

##### Summary

Reviewer 判断 Rooted DAG 已接入 `taskspace_control` schema、parser、handler、runtime transaction、
snapshot validation、projection 和显式 Finish；未发现旧 `create_node`、terminal-list、零边 production
schema 或独立 `TaskStatus/MapStatus` 完成权威。Reviewer 报告一个 blocker：认为
`MapRuntimeMode::Standard` 使 TaskSpace 可绕过，违反 Phase C “接线后不得有运行时模式开关”。

##### Blocking Findings

- **R1-B1：`MapRuntimeMode::Standard` 使 TaskSpace production 可绕过。**
  - Broken assumption: `taskspace_control` 在所有 production session 中都不可绕过，Phase C 已删除任何运行时模式选择。
  - Failure scenario: session 保持或切换到 `standard`，provider tool visibility 隐藏 `taskspace_control`，无需 Rooted DAG Map。
  - Trigger condition: 客户端执行 `SetMapRuntimeMode { Standard }`，或 session 默认处于 Standard。
  - Impact: reviewer 认为“生产 TaskSpace 已切到单一 Rooted DAG”只在 `Experiment` active 时成立。
  - Proof needed: 删除 Standard/Experiment 选择，或证明并冻结“Standard 是非 TaskSpace 产品模式，而非旧 TaskSpace 实现”。
  - Plan / production path: Phase C 纵向切换；`protocol/src/protocol.rs`、`core/src/session/handlers.rs`、`core/src/session/turn.rs`。

##### Non-blocking Risks

- **R1-N1：收益样本不足以支持稳定性能收益。**
  - Broken assumption: simple/branch-join 各一次足以泛化缓存、成本和耗时收益。
  - Failure scenario: 重复运行因 Agent 轨迹和缓存冷热不同而反转单次结果。
  - Trigger condition: 三次轮换正式矩阵。
  - Impact: 不能把单次 smoke 当作稳定收益结论。
  - Proof needed: Phase G 三次轮换和逐 request section 分解。
- **R1-N2：restore/fork 深度尚未形成完整故障矩阵。**
  - Broken assumption: Phase C smoke 已证明 crash/replay/fork 全部可靠。
  - Failure scenario: 中断或 fork 后恢复出不一致状态。
  - Trigger condition: crash injection、resume、fork 和 corruption 矩阵。
  - Impact: 长会话恢复可靠性仍未完全证明。
  - Proof needed: Phase E/F 计划中的完整验证。

##### User-Perspective Checks

- Usability: active TaskSpace 的 bootstrap schema 明确暴露 `root`、`initial_work_node`、`finish`、`edges` 和 `continuation`；通过。
- Ease of use: active schema 暴露 `mutate_graph`、`transition_node`、`finish_end`、`expand_nodes` 和 `read_output_ref`；通过。
- Ease of understanding: parser failure 和 rooted rejection 带 `state_commit`、`current_revision` 与 invariant violations；通过。
- Reviewer 将 Standard 中看不到 `taskspace_control` 归入 R1-B1。

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| C1 snapshot/event | Root/Finish/revision/role/status 一等字段 | `protocol/src/protocol.rs` | `ActionMapSnapshotMap` | protocol/schema tests | live snapshot | none | landed | none |
| C2 complete init schema | 无旧 create/可空依赖 | `tools/src/taskspace_tool.rs` | `taskspace_control` bootstrap | tool schema tests | live initialize | none | landed | none |
| C3 mutate graph | expected revision + atomic transaction | `rooted_dag/transactions.rs` | handler `MutateGraph` | transaction tests | control events | none | landed | none |
| C4 lifecycle/finish | Root-open、Work transition、显式 terminal | `rooted_dag/events.rs`, `transactions.rs` | runtime handler | transition/finish tests | live `finish_end` | none | landed | none |
| C5 persistence/projection | canonical Map 派生 snapshot/projection | `event_store.rs`, `projection.rs` | session/provider context | reconstruction/projection tests | 78/78 scan | none | landed | none |
| C6 delete old authority | 无独立 mutable Task/Map completion | `action_map/map.rs`, `rooted_dag/model.rs` | production runtime | forbidden-symbol tests | live rooted maps | none | landed | none |
| C7 legacy fatal | 旧 schema 明确拒绝 | `action_map/runtime.rs` | snapshot restore | legacy rejection tests | `legacy_schema_unsupported` | none | landed | none |
| Product mode boundary | Standard 与 TaskSpace 的关系清晰 | protocol/session/turn | `SetMapRuntimeMode` | mode/visibility tests | logical-mode artifacts | none | reviewer disputed | R1-B1 |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| 单一 Rooted DAG 正确性 | R5 零边/双状态 | Phase C invariants | unit + Docker | simple/branch live maps | achieved | none observed | proven | none |
| 原子/显式终结可靠性 | 旧外置 terminal | candidate zero partial commit | transaction/replay tests | focused tests + live finish | achieved for Phase C | crash matrix later | weak-evidence | R1-N2 |
| 缓存前缀恢复 | simple 0.32%, 0/9 | 恢复 stable prefix | wire LCP/cache usage | 91.28%/92.47%, 78/78 | achieved in smoke | repeat variance unknown | weak-evidence | R1-N1 |
| 成本改善 | Standard/R5 baseline | 未在 Phase C 设总体改善目标 | single-pair metrics | simple regressed, branch improved | neutral/mixed | simple 2.17x requests | deferred | R1-N1 |

##### Required Fixes

- **R1-A1：**删除 Standard/Experiment 选择，或文档和代码明确 Standard 是非 TaskSpace 产品模式且 TaskSpace 不存在 legacy/rooted 选择。关联 R1-B1。

##### Missing Tests

- **R1-T1：**从真实 session/request 入口证明 Phase C 后 TaskSpace 不会在 `taskspace_control` 隐藏时继续运行。关联 R1-B1。
- **R1-T2：**若目标是所有 production session 强制 TaskSpace，则证明 `SetMapRuntimeMode { Standard }` 不可用或被拒绝。关联 R1-B1。
- **R1-T3：**证明 active R6 TaskSpace 的 provider-visible tools 不会回退为 Standard visibility。关联 R1-B1。

##### Missing Logs / Observability

- **R1-L1：**增加 session 运行于 Standard 而非 TaskSpace 的结构化日志/指标，使所谓 bypass 可见。关联 R1-B1。

##### Evidence

- `docs/v0.0.5/build-R6/01-r6-phased-implementation-plan.md:190-210` - Phase C 纵向切换和“运行时模式开关”文字。
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs:778-789` - `SetMapRuntimeMode` 与 Standard/Experiment。
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs:1011-1028` - provider tool visibility。
- `third_party/codex-cli/codex-rs/core/src/session/mod.rs:3806-3808` - active TaskSpace mode 判定。
- `third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs:390-430` - bootstrap/active R6 control schema。
- `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args.rs:10-90` - R6 action parser。
- `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/transactions.rs:120-230` - mutation/finish transaction。
- `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/events.rs:241-260` - terminal 同时闭合 Finish/Root。
- `third_party/codex-cli/codex-rs/core/src/action_map/projection.rs:91-140` - epoch projection。
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime.rs:1203-1215` - legacy schema fatal。

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | R1-B1 | 把产品级 Standard/TaskSpace 选择当成 R5/R6 实现切换 | blocking | reject | 宪章明确“TaskSpace 是 Standard 自然上下文的图化重组”；计划固定 Standard/R5/R6 为不同对照臂。静态扫描无 legacy/rooted feature flag，active TaskSpace 只暴露 R6 schema | 澄清计划：禁止的是 R5 legacy/R6 rooted 双实现开关；更新 protocol 注释说明 Standard 是非 TaskSpace 产品模式 | 无 accepted blocker，不触发 closure round |
| implementation-completeness-adversary | R1-N1 | 单次样本被误泛化为稳定收益 | non-blocking | accept | Phase C 结果已明确每臂 1 次、无统计显著性，simple 成本回退也已披露 | 保持 Phase G 三次轮换和逐 request section 重基线 | Phase G |
| implementation-completeness-adversary | R1-N2 | Phase C smoke 被误当成 crash/resume/fork 完整证明 | non-blocking | defer | Phase C 只要求纵向生产切换；计划明确把 crash/replay/fork matrix 放在 Phase E，context ownership 放在 F | 不提前扩张 Phase C；报告保留风险 | Phase E/F |
| implementation-completeness-adversary | R1-A1 | 要求删除 Standard 或明确产品边界 | blocking duplicate | reject | 删除 Standard 会破坏已冻结产品定义和横向基线；边界已有宪章/计划证据 | 接受其中的文字清晰度建议，更新计划和 protocol 注释，不改产品行为 | none |
| implementation-completeness-adversary | R1-T1 | 假设隐藏 control 后仍是 TaskSpace session | blocking duplicate | reject | `taskspace_mode_active` 决定 TaskSpace ownership；Standard 是退出 TaskSpace，不是 TaskSpace fallback。Docker `logical-mode-map.json` 明确区分左右臂 | 无错误前提测试 | none |
| implementation-completeness-adversary | R1-T2 | 假设所有 production session 都必须强制 TaskSpace | blocking duplicate | reject | 产品要求保留 Standard；计划 1.6.1 和总验收均要求 Standard/R5/R6 横向臂 | 不增加拒绝 Standard 的测试 | none |
| implementation-completeness-adversary | R1-T3 | active TaskSpace 可能静默回退 Standard tool visibility | major test | reject | `session/turn.rs:1499-1555` 已测试 TaskSpace 只保留 control、Standard 隐藏 control、bootstrap 强制 control、active 同时暴露 ordinary+control | 现有测试已覆盖建议场景 | none |
| implementation-completeness-adversary | R1-L1 | Standard/TaskSpace 路由不可观察 | observability | reject | `MapRuntimeEvent::ModeChanged`、`TaskContextOwnershipChanged` 已结构化记录；benchmark 输出 `logical-mode-map.json` 和 pair report logical mode | 不增加重复日志 | none |

### Response Validation

| Validation | Result | Evidence |
|---|---|---|
| Provider tool visibility focused tests | PASS, 12/12 | `cargo test -p codex-core active_context_replacement_tests` |
| R6 machine contract | PASS | `test-r6-rooted-dag-contract.ps1`: 14 fixtures, 31 ownership items, Phase A-C results |
| Protocol lint/fix | PASS | `just fix -p codex-protocol`; only pre-existing `large_enum_variant` warnings |
| Rust formatting | PASS | `just fmt` |
| Report/diff whitespace | PASS | `git diff --check` before final commit |

按 vendor `AGENTS.md` 约束，`just fmt` 后不重复运行测试。格式化没有改变注释之外的 Rust 代码。

### Closure Status

- Blocking findings found: yes, one reviewer finding
- Accepted blocking findings fixed: n/a; blocker rejected as product-boundary category error
- Blocking re-review completed: n/a
- Blocking re-review passed: n/a
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - n/a
- Rejected findings backed by evidence: yes
- Deferred findings documented: yes, R1-N2 in Phase E/F
- Implementation completeness gaps resolved or accepted by user: yes; no Phase C production gap remains
- Target benefit warnings recorded: yes, R1-N1 retained for Phase G
- Blocked reason: n/a
- Allowed to proceed: yes

## Final Conclusion

Round 1 独立 reviewer 完成。唯一 blocker 把 Standard 产品模式误判为 legacy TaskSpace bypass；该判断
与 R6 宪章、固定对照臂、生产 schema 和现有 mode/visibility tests 冲突，已基于证据 reject。计划和
protocol 注释已补充边界说明。未发现 Phase C 生产接线、旧权威删除、反馈、projection 或显式终结的
未闭合阻断项。Phase C 可继续进入 Phase D；性能稳定性与 crash/resume/fork 深度按原计划进入 G 和 E/F。
