# Subagent VS Review: Codex 0.151 主线追赶

- Created: 2026-09-01T07:45:00+08:00
- Updated: 2026-09-02T09:42:00+08:00
- Report schema: adversarial-v2
- Task: 对本轮 Codex 0.151 主线追赶的实现完整性、兼容性、验证证据和发布收口状态进行对抗性审查。
- Report path: `vs_review/2026-09-01-codex-0151-catchup-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context；reviewer 只接收本报告的中性导航包。
- Status: completed-blocked
- Control outcome: user-decision-required
- Automatic round budget: 2
- Completed rounds: 1
- Last known-good checkpoint: `a3ac0770df153dea2a18ff7e3cc5df245c76f45a`（0.151 主题开始前的 project main）

## Review Control Contract

### Frozen Objective

验证当前 `whalecode-codex` 分支是否真实、最小充分地完成 Codex CLI `rust-v0.151.0` 追赶，同时保留已确认的 DeepSeek、多 Provider、TaskSpace 和 workspace 隔离合同；识别阻止 W6 收口或合入 main 的高影响缺口。

### Acceptance Criteria

- vendor provenance 精确固定到 `rust-v0.151.0` / `78c290807ce710180111df227df3b7a4fe845452`，同步 metadata 可复验。
- 上游 substrate 的目标修复已进入生产代码路径，而不是只更新版本、协议、fixture 或文档。
- DeepSeek、多 Provider 与 TaskSpace 的现有产品合同未因机械 cut-over 丢失或被上游默认覆盖。
- 本地测试结论区分真实产品回归、已批准延期、平台限定失败和 harness 失败，不把部分验证表述为完成。
- W6 的缓存 gate、生成工件、release report、clean/push 等退出条件有完整证据；未满足时计划必须保持未完成。
- 变更遵守最小充分设计，不为追赶引入第二状态权威、额外公共产品入口或与目标无关的架构治理。

### Explicit Non-goals

- 不追 Codex 0.152 alpha 或未发布 main。
- 不重新设计 TaskSpace、PrimitiveModule、Provider 架构或用户产品逻辑。
- 不要求 Windows、W9/TaskSpace 已批准延期项在本轮强行修复。
- 不自动修复 reviewer 发现；Round 1 只产出证据化 finding 与 triage。
- 不启动新的真实模型运行，不复用已消费的零重试授权。

### Frozen Target Locations

- `third_party/codex-cli/`
- `scripts/codex-upstream/`
- `scripts/cache-regression/`
- `scripts/taskspace-benchmark/`
- `docs/releases/v0.0.7/codex-upstream-sync/`
- `docs/migration/codex-sync/`
- `benchmarks/cache-regression/`
- `benchmarks/whale-agent-run-ledger.json`
- `coe/2026-09-01-06-00-codex-0151-release-closeout-regressions.md`
- `coe/2026-09-01-07-27-cache-run-binary-attestation.md`

### Allowed Change Categories

- 本轮 Round 1 只允许新增/更新本 review report。
- reviewer 只读，不修改文件。
- 后续仅在 reviewer blocker 被主 Agent 接受、E0-E3 证据充分且 governor 判定 `start-closure-round` 时，才允许最小实现、测试或文档修复。

### Approval-required Changes

- 新 top-level module、外部依赖、公共 API、持久化 schema、跨模块抽象。
- 改变 DeepSeek 默认、Provider 路由、TaskSpace 状态权威或用户可见入口。
- 新真实模型预算或复用已经 claim 的授权。
- 超出冻结 target locations 的改动。

### Authoritative Sources

| Authority | Source | What It Controls |
|---|---|---|
| E0 | 用户要求本项目只处理 `whalecode-codex`、追赶 0.151、工程问题自主处理、产品逻辑需确认、当前请求执行对抗性审查 | 目标、边界、产品审批与 review 授权 |
| E1 | `AGENTS.md` | DeepSeek/TaskSpace 产品合同、workspace/cache/预算门禁、最小充分设计、分支边界 |
| E1 | `docs/releases/v0.0.7/codex-upstream-sync/plan.md` | 唯一 0.151 工程计划、W1-W6 退出条件和延期边界 |
| E1 | `prd/2026-08-23-v0.0.6-multi-provider.md` | 多 Provider 产品权威 |
| E1 | `third_party/codex-cli/UPSTREAM.md` 与 sync metadata | vendor provenance、overlay 和验证声明 |
| E2 | Git history `a3ac0770df..dbb48baeea`、本地测试、cache gate/report/result/ledger、COE 文件 | 实际落地和运行行为 |
| E3 | Codex `rust-v0.150.0`、`0.150.1`、`0.151.0` 官方 release/tag/commit | 上游目标边界 |
| E4 | reviewer 与主 Agent 判断 | 只作为待验证假设 |

### Baseline And Rollback

- Baseline revision: `a3ac0770df153dea2a18ff7e3cc5df245c76f45a`
- Review subject: `dbb48baeea`，另有 `stash@{0}` 保存尚未提交的 W6 closeout 文档、metadata、生成脚本和两个 final-wire snapshots。
- Rollback checkpoint: `a3ac0770df153dea2a18ff7e3cc5df245c76f45a`；局部修复可按原子提交逐个 revert。
- Expected benefit: 进入 0.151 的安全、权限、ToolRouter/MCP、PTY/unified-exec、compaction 修复，同时保持 Whale 产品合同。
- Acceptable side effects: 上游 vendor 大规模机械变化、测试 fixture/schema 更新、已明确记录的平台/TaskSpace 延期；不接受未披露产品退化或伪完成。
- Automatic round budget: 2

## Round 1: 实现完整性与发布资格反证

### Round Control

- Round type: initial
- Round number: 1
- Completed automatic rounds before launch: 0
- User approval for this round: 用户于 2026-09-01 明确要求“对本轮codex追赶工作做对抗性审查”。
- Closure finding IDs: n/a
- Permitted closure relation: n/a
- Target scope delta allowed: none

### Review Input

#### Objective

反证当前分支是否真实完成 Codex 0.151 追赶并满足 W6 发布收口条件；优先发现会让“可合入 main / 可宣布追赶完成”不诚实的缺口。

#### Acceptance Criteria

- 对照唯一计划 W1-W6 检查每项是否有 production path、integration entry、test/runtime evidence。
- 检查 0.151 cut-over 是否误删 Whale overlay；特别关注本轮已发现并恢复的 `debug provider` 与隐藏 TaskSpace benchmark switch 是否暗示其他遗漏。
- 检查 DeepSeek Responses/final-wire、Provider route、TaskSpace relational state/fork-resume、extension lifecycle 的生产闭环。
- 检查完整回归中 12 fail + 1 timeout 的延期分类是否有 E0/E1 权威，是否存在被错误延期的 0.151 回归。
- 检查 cache gate 的 candidate transition、失败的真实 run、账本 `unavailable` 和未晋升 baseline 是否明确阻止 W6 完成。
- 检查计划、UPSTREAM、migration report、generated overlay 与实际 HEAD/stash 是否一致。

#### Explicit Non-goals

- 不要求 reviewer 设计或实现修复。
- 不把 0.152、Windows、已批准 W9/TaskSpace 分支工作扩入当前 closure。
- 不用通用最佳实践替换仓库既有产品权威。

#### Review Target

代码实现、测试策略、缓存验证、release/migration 证据和计划完成状态。

#### Target Locations

- `docs/releases/v0.0.7/codex-upstream-sync/plan.md`
- `third_party/codex-cli/UPSTREAM.md`
- `docs/migration/codex-sync/`
- `scripts/codex-upstream/` 与生成 metadata
- `third_party/codex-cli/codex-rs/` 中 Provider、client、session、TaskSpace、extension、exec/CLI 相关代码和测试
- `scripts/cache-regression/`、`benchmarks/cache-regression/`、账本
- 两个 COE case
- `git log/diff a3ac0770df..dbb48baeea` 与 `git stash show -p stash@{0}`

#### Baseline And Rollback Checkpoint

- Baseline: `a3ac0770df153dea2a18ff7e3cc5df245c76f45a`
- Subject: `dbb48baeea`
- Rollback checkpoint: `a3ac0770df153dea2a18ff7e3cc5df245c76f45a`

#### Change Introduction

该主题固定 0.151 upstream tree，生成 replay 合同，机械切换 vendor，然后分批恢复 generic、Provider/DeepSeek、TaskSpace/extension overlay，并执行定向/完整回归。最近在真实缓存资格运行前后发现两个 cut-over 遗漏，已分别恢复隐藏 provider attestation 命令和仅供 benchmark 的隐藏 TaskSpace exec switch。W6 closeout 文件和 final-wire snapshots 仍在 stash，cache baseline 尚未晋升。

#### Risk Focus

- replay/classification 声称全覆盖，但生产 overlay 仍可能遗漏。
- 测试 fixture 修复可能掩盖真实行为退化。
- 失败分类可能把当前目标内 blocker 错标为 TaskSpace/Windows 延期。
- cache runner 的失败和 usage unavailable 可能使预算、baseline 或发布声明不可审计。
- W6 文档在 stash 中可能与 HEAD、计划状态或 UPSTREAM 声明矛盾。
- 为恢复测试入口引入的隐藏 CLI seam 可能侵入产品或缺少维护合同。

#### User-Perspective Review Focus

- 不新增或恢复未批准的用户可见 OpenAI/Codex 产品入口。
- DeepSeek Flash 默认、Pro/Responses、三访问路由及用户错误反馈不退化。
- TaskSpace 与 Codex 原生 task 概念不会形成两个用户状态权威。

#### Implementation Completeness Focus

- W1-W6 每个 work unit 的 production path、entry、测试和运行证据。
- 上游安全/效率收益是否确实保留在最终 vendor tree。
- DeepSeek/TaskSpace overlay 是否只停留在 compile seam、fixture 或 hidden debug path。
- generated metadata、schema、snapshots、UPSTREAM、migration report 是否已落地并可复现。
- 已知未落地项是否仍被计划正确标为 pending/deferred。

#### Target Benefit Focus

- 安全/稳定/效率收益的 baseline 是 0.149；目标是保留 0.151 对应修复且 Whale 回归不扩大。
- 测量方法是固定 commit/tree 可达性、定向测试、隔离完整回归、cache final-wire/live qualification。
- 重点识别只有 commit 可达性、没有 Whale 生产路径验证的弱证据；收益缺口本身为非阻断 warning，除非同时证明正确性或发布风险。

#### Evidence Sources And Gaps

- E0-E3 source: 上述 Authoritative Sources。
- E4 hypothesis: “0.151 replay 仍可能遗漏其他 Whale-only seam”“12 fail + 1 timeout 的分类可能不完整”。
- Known evidence gap: live cache 双臂验证未成功，accepted baseline 未晋升；W6 stash 尚未提交；Windows/W9 已批准延期。

#### Assumptions To Attack

- 883 条 replay decision 足以防止 overlay 漏失。
- 通过定向测试即可证明 Provider/TaskSpace 生产闭环。
- 12 fail + 1 timeout 全部属于已批准延期，不影响 0.151 主线追赶。
- 两个已发现的 hidden seam 遗漏是孤立事件。
- cache usage unavailable 的失败不会污染或弱化发布证据。
- 文档中的 `verified` 与实际 HEAD、stash、ledger 一致。

#### Adversarial Lenses

- requirements
- implementation-completeness
- state
- failure
- data
- maintenance
- testing
- observability
- target-benefit

#### Verification Status

- 计划记录 W1-W5 verified；W6 当前计划文件仍是 not-started/pending。
- 最新完整 core：3969 run / 3956 pass / 12 fail / 1 timeout / 9 skipped；其中 5 个 Guardian failure 已修复并定向 5/5 通过，剩余项被归类为 TaskSpace/W9/Cyber 生命周期延期。
- sync metadata validator、56 个脚本测试、fmt、current overlay 生成/check 曾通过，但对应最终 closeout 工件仍在 stash。
- cache index gate 对候选 transition 通过；clean HEAD live-baseline gate 识别 Standard metadata 与 TaskSpace tool schema 两个变化。
- 真实 cache run `WAR-20260901-073444-CACHE-REGRESSION-3BF5A4B3` 在 Provider/Agent pair 前因 harness 失败，账本状态 failed、usage/cost unavailable、accepted baseline 未晋升。
- 最新本地 hidden CLI 修复已有定向测试和 workspace install/doctor/help probe，尚无新的真实双臂授权。

#### Reviewer Instructions

- Fresh internal subagent session；不得继承主 Agent 对话上下文。
- 只读检查仓库和 Git；不得修改文件。
- 优先输出高影响、可反证的 finding，不做风格清单。
- 每个 blocker/major finding 必须写 broken assumption、failure scenario、trigger、impact、proof needed、E0-E4 authority、证据路径/行号和 closure relation。
- 明确输出 user-perspective、implementation completeness、target benefit、required fixes、missing tests、missing observability。
- 不能把 reviewer 推理本身当作 E2；需要用仓库代码、测试、日志或计划证据支撑 blocker。

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
| complex | 20 minutes | one bounded 10-minute extension only if alive | 2 | review unavailable 时不得 pass |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | 当前最高风险是计划 W1-W6 的 production landing、overlay 完整性与发布证据是否被局部测试或 fixture 伪完成 | implementation completeness、test validity、release evidence |

### Reviewer Launch Records

| Round | Reviewer | Agent / trace | Context | Packet | Excluded context | Mode |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `implementation-completeness-adversary` | `/root/codex_0151_review`；来源：`collaboration.spawn_agent` 返回的 canonical task name | fresh session，`fork_turns=none` | 本文 `Round 1 Review Input` 的冻结合同与精确审查包 | 主 Agent 历史、推理、草稿、预设结论及完整 diff 均未注入 | 只读；禁止编辑、提交、切分支、真实付费运行 |

### Reviewer Timeout Records

| Round | Reviewer | Outcome | Wait record |
| --- | --- | --- | --- |
| 1 | `implementation-completeness-adversary` | completed within initial window | 4 次有界 mailbox wait；未使用 10 分钟 extension，未重试 |

### Reviewer Outputs

#### Verdict

`fail`。W1–W5 有实质实现与测试证据，但 W6 明确未完成；当前分支不能表述为 0.151 追赶完成或具备合入资格。

#### Blocking Findings

**B1 — W6 发布资格未完成（E0/E1）**

- `plan.md` 仍把 W6 标为 `not-started`，Phase E gate 为 `pending`；退出条件要求 provenance/生成物一致、隔离矩阵及 cache gate 达标、commit/push 和 clean tree。
- reviewer 与主 Agent 均复现 `python3 scripts/codex-upstream/validate_sync_metadata.py` exit 1：`replay overlay tree` 和 `overlay inventory` 相对 Git index 过期。
- 真实 cache result `WAR-20260901-073444-CACHE-REGRESSION-3BF5A4B3` 为 `partial`；Standard 在有效 pair 前失败，map-request 未执行；账本为 `failed`，usage 为 `unavailable`，accepted live baseline 未晋升。
- W6 report、current-overlay generator/inventory、COE、metadata 更新和 final-wire snapshots 只存在于 `stash@{0}`，HEAD 不具备可复现 closeout。
- 最小关闭条件：取得新的替代预算；完成 Standard + TaskSpace/map-request 最小双臂真实资格；完整结算 usage/cost；成功后才晋升 baseline；协调并提交 W6 stash；metadata/generator check 通过；W6 `verified`；工作树 clean 且 HEAD 与 remote 一致。

**B2 — 剩余 7 failed + 1 timeout 未逐项绑定既有延期权威（E1）**

- stashed COE 只按 Cyber 继承、queue-only metadata、websocket turn state、executor skill request count 四簇概括，未持久化最终 3969-test 原始日志、精确 failing test 名、签名、0.151 pristine/baseline 对照和逐项用户延期映射。
- 涉及测试位于 `cyber_access_program.rs`、`pending_input.rs`、`turn_state.rs`、`skills_extension.rs` 等真实 lifecycle 路径；仅以概念相似性归入 TaskSpace/W9 不足以排除 Whale overlay 回归。
- 最小关闭条件：持久化最终隔离日志，并为每个 failure/timeout 建立“测试名—签名—官方 0.151/基线结果—生产路径—延期权威”表；无法映射的项必须修复或取得明确风险接受。

#### Non-blocking Findings

- **N1（E1）**：883-entry replay ledger 是迁移控制证据，但不能单独证明所有 Whale-only seam 被保留；`debug provider` 和 hidden TaskSpace exec switch 在 cut-over 中丢失后才由 `e77a41ebb5`、`4fc2e56375` 恢复。后续同步应为此类 hidden operational seam 增加 baseline-preservation assertion。
- **N2（E1/E2）**：目前支持“0.151 修复可达且兼容”的正确性收益，但没有 0.149 vs 0.151 运行基准，不能量化宣称效率提升。
- **N3（E1）**：stash 中新增的 current-overlay generator 没有独立单测；本轮可用 validator/check 覆盖，后续可补一个小型确定性测试。

#### Frozen-scope Completeness

| Unit | Reviewer assessment |
| --- | --- |
| W1 | complete-with-recorded-candidate-risks |
| W2 | complete-as-cut-over-contract；56/56 脚本测试独立通过 |
| W3 | complete |
| W4 | implementation-complete；release cache qualification 留在 W6 |
| W5 | core implementation complete within frozen UI deferral |
| W6 | incomplete / blocking |

#### Independent Checks

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/codex-upstream/tests -p 'test_*.py'`：56/56 passed（reviewer E0）。
- `python3 scripts/codex-upstream/validate_sync_metadata.py`：2 个 stale metadata 错误（reviewer 与主 Agent E0）。
- `HEAD == origin/whalecode-codex == dbb48baeea`；除本 review report 外无未提交 HEAD 改动（E1）。
- 检查了 pristine qualification logs、W1–W5 COE、cache result/gate/ledger、Provider/DeepSeek/TaskSpace production code、baseline hidden seams 和 `stash@{0}`（E1）。
- reviewer 未启动 Rust 全量 suite 或真实模型运行。

#### User Outcome / Scope Assessment

当前实现已形成可信的 0.151 substrate，并保留 Whale identity、DeepSeek Flash 默认、三路 Provider、DeepSeek 本地压缩及 relational TaskSpace 路径；但还不能承诺 release-qualified cache 行为和完整解释的 core regression 边界。未发现阻断性的过度设计或范围漂移；vendor 大差分属于整树 cut-over，current-overlay inventory 是 provenance 修正，不是第二运行时权威。

### Main Agent Response

| Finding | Triage | Evidence / rationale | Required action |
| --- | --- | --- | --- |
| B1 | **accept-blocking / static-fixed** | `b631eb7e67` 落地 final-wire；`f68c09c4c9` 落地 883-path inventory、validator、release report 与 gate evidence；免费 final-wire gate 通过，但 live gate 仍以 accepted `4a15…` != current `e39d…` 阻断 | 保持 W6 未完成；不得复用已消费授权。仍需新预算完成双臂运行和 baseline 晋升 |
| B2 | **accept-blocking / fixed-pending-re-review** | `f68c09c4c9` 持久化精确 8-test JUnit、全量 JUnit、pristine 对照和逐项“签名—生产路径—延期权威”manifest；定向结果严格为 7 failed + 1 timeout | 等待与 B1 一起进入 focused closure review；不在本轮修改已批准延期的 TaskSpace 产品路径 |
| N1 | **accept-nonblocking** | 两个 hidden seam 的恢复提交证明 replay metadata 不是行为完备性证明 | 作为下一次 upstream cut-over 的测试改进项记录，不扩大本轮 W6 |
| N2 | **accept-nonblocking** | 本轮有 commit reachability 和兼容性证据，无 A/B 性能 benchmark | 发布措辞限于可达性、兼容性和对应修复，不量化效率提升 |
| N3 | **accept-nonblocking** | generator 尚在 stash，未见 dedicated test | validator/check 足够本轮收口；后续小型单测可延期 |

### Review Governor

- Decision: `user-decision-required`
- Reason: B2 与 B1 静态部分已按最小范围修复；B1 的最终关闭仍需要一笔新的真实模型预算，而冻结合同禁止复用已消费授权或自行启动付费运行。
- Round 2: 未启动。只有 B1/B2 完成最小关闭动作后，才允许一次仅针对这两个 finding 的 focused blocker-closure review；它是自动 round budget 中的最后一轮。
- Scope growth: 无；未要求修复 Windows、0.152、TaskSpace 产品逻辑或新增架构。

### Closure Status

- Blocking findings found: B1, B2
- Accepted blocking findings fixed: B2 fixed-pending-re-review；B1 static-fixed/live-pending
- Blocking re-review completed: n/a
- Blocking re-review passed: n/a
- Blocking re-review round links: n/a
- Blocking re-review launch records: n/a
- Rejected findings backed by evidence: n/a（无 rejected finding）
- Deferred findings documented: N1、N3；N2 为 release-claim constraint
- Implementation completeness gaps resolved or accepted by user: B2 resolved；B1 live qualification unresolved
- Target benefit warnings recorded: yes（N2）
- Automatic round budget respected: yes
- Third-or-later round explicitly user-approved before launch: n/a
- Scope drift detected: no
- Evidence sufficient for scope-expanding actions: no scope expansion proposed
- Convergence reflection required and recorded: n/a
- Control outcome: user-decision-required
- Blocked reason: W6 cache qualification requires replacement budget
- Allowed to proceed: 仅在取得新预算后执行最小双臂真实 cache qualification；不得宣称完成或合入 main

## Final Conclusion

Round 1 已完成，当前仍为 **not merge-ready**。W1–W5 的生产实现总体成立；B2 和 B1 静态部分已由 `b631eb7e67`、`f68c09c4c9` 修复，未引入产品逻辑或范围扩张。唯一剩余 blocker 是 W6 live cache qualification 与 accepted baseline 晋升。

下一步最小顺序：申请一笔新的最小双臂 DeepSeek cache 资格预算关闭 B1；成功后晋升 baseline、更新 W6 状态，并只对 B1/B2 做一次 focused closure review。
