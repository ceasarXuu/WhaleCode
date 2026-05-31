# Subagent VS Review: TaskSpace Benchmark Harness Implementation Plan

- Created: 2026-05-31T00:00:00+08:00
- Updated: 2026-05-31T00:00:00+08:00
- Task: 制定 TaskSpace paired benchmark harness MVP 的详细工程实施计划。
- Report path: `vs_review/2026-05-31-taskspace-benchmark-harness-plan-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: Implementation Plan Review

### Review Input

#### Objective

审查新增工程计划是否足够可执行，是否符合 TaskSpace benchmark 策略，并能指导下一步实现 paired benchmark harness MVP。

#### Review Target

工程计划、测试 harness 设计、变量控制、prompt guard、hidden oracle 隔离、首批场景范围、实施阶段切片。

#### Target Locations

- `docs/plans/2026-05-31-taskspace-benchmark-harness-implementation-plan.md`
- `docs/plans/2026-05-31-taskspace-benchmark-strategy.md`

#### Change Introduction

新增一份工程实施计划，范围限定为 paired benchmark harness MVP：

- 先做 L1 `single-file-fast-fix` 和 L2 `order-pipeline-growth`。
- 实现统一 harness scaffold、scenario manifest、prompt guard、left/right workspace、hidden oracle isolation、metrics、pair report、aggregate report。
- 明确 Phase 0-4 实施顺序。

#### Risk Focus

- 计划是否仍然过大，无法作为 MVP 落地。
- paired 对照是否保持 only-mode-diff。
- prompt guard 是否符合用户叙事硬约束，避免误杀真实工程词。
- hidden oracle isolation 是否足够具体。
- 首批场景是否足以验证 L1 不拖累和 L2 paired 对照。
- 是否遗漏必要自测或验收标准。

#### Verification Status

- 文档已新增。
- 未开始代码实现。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- 只把会导致实施方向错误、MVP 不可落地或 benchmark 可信度失真的问题列为 blocking。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Harness Implementation Reviewer | 计划将直接进入工程实现，需要攻击 MVP 范围、变量控制和验收标准 | implementation feasibility、paired validity、test reliability |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Harness Implementation Reviewer | multi_agent_v1.spawn_agent | `019e7d35-d7f4-7ca0-aa2f-26e82e53106c` | spawn_agent + subagent_notification | no | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Harness Implementation Reviewer

##### Summary

计划总体方向正确，目录、模块边界、left/right 中性路径、only-mode-diff、prompt guard、pair/aggregate report 都与策略大体一致。但作为下一步实现 paired benchmark harness MVP，还不能直接照做。主要问题是：E2 证据门禁没有落成可执行规则；hidden oracle isolation 在当前 `--dangerously-bypass-approvals-and-sandbox` 命令下过于乐观。

##### Blocking Findings

- Hidden oracle isolation 不足以支撑可信 E2。实施计划只要求 agent cwd 是 `left/repo` 或 `right/repo`，并检查 repo 内不存在 private oracle，但 Whale 命令明确使用 `--dangerously-bypass-approvals-and-sandbox`。cwd 中性不等于文件系统隔离。
- E2 gate 仍可能被 scenario/acceptance 误标。runner 默认 `Repeats 1`，manifest 允许声明 `evidence_target=E2`，MVP 验收只要求两个场景生成 pair report，没有要求 `Repeats >= 3`、provider 参数可观测、或 E2 gate 失败时强制降级/排除。

##### Non-blocking Risks

- MVP 仍偏大。Phase 0/1 是可落地的，但完成条件同时要求两个场景、aggregate、observability、self-test。
- 现有脚本复用路径写得不全，可能重复造轮子。
- `order-pipeline-growth` 标为 L2 可以接受，但策略中是 L2/L3，实施时不应把 L3 阈值强压到 L2 MVP。

##### Required Fixes

- 增加显式 evidence gate：`reported_evidence_level` 由 harness 计算，不能只信 `scenario.json`。
- 增加 hidden oracle isolation contract：记录 `oracle_isolation_level`；没有 hard sandbox 时，必须把结果标为 non-E2，并扫描/记录 private oracle 路径泄漏。
- 把 existing-script reuse 写成实施清单。

##### Missing Tests / Plan Gaps

- `evidence-gate self-test`
- `provider-param observability gap test`
- `oracle path leak test`
- `run-order alternation test`
- `manual_review_required report persistence test`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Harness Implementation Reviewer | Hidden oracle isolation 不足以支撑可信 E2 | blocking | accept | `--dangerously-bypass` 下 cwd 中性不等于 FS 隔离 | 新增 `oracle_isolation_level` contract；hard_sandbox/soft_denylist/failed 分级；soft_denylist 不默认进入 E2；增加路径泄漏扫描范围和目标 | Round 2 closure |
| Harness Implementation Reviewer | E2 gate 可能被误标 | blocking | accept | `Repeats 1` 和 manifest `evidence_target=E2` 不能直接成为 E2 证据 | 新增 Evidence Gate：`reported_evidence_level` 由 harness 计算；Repeats < 3、provider unknown、manual review 未闭环、oracle isolation 非 hard 都不得进入 E2 | Round 2 closure |
| Harness Implementation Reviewer | MVP 范围偏大 | non-blocking | accept | 首个可运行闭环应尽快落地 | 将完成条件拆为 MVP、MVP+1、MVP+2；MVP 只要求 single-file valid pair + self-test，order-pipeline/aggregate/repeats 后移 | Round 2 closure |
| Harness Implementation Reviewer | 复用清单不足 | non-blocking | accept | 避免重复造轮子 | 增加现有脚本复用清单，标注直接复用、wrapper 调用、迁移参考 | Round 2 closure |
| Harness Implementation Reviewer | 缺少自测 | non-blocking | accept | E2 gate 和 isolation 需要自测保护 | 增加 evidence-gate、provider-param、oracle path leak、run-order alternation、manual-review persistence 自测 | Round 2 closure |

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

## Round 2: Closure Review

### Review Input

#### Objective

验证 Round 1 accepted blocking fixes 是否闭合：hidden oracle isolation 可执行、E2 evidence gate 不会误标、MVP 范围已经收敛。

#### Review Target

工程计划、证据等级门禁、oracle isolation、实施阶段切片、自测清单。

#### Target Locations

- `docs/plans/2026-05-31-taskspace-benchmark-harness-implementation-plan.md`
- `vs_review/2026-05-31-taskspace-benchmark-harness-plan-review.md`

#### Change Introduction

根据 Round 1：

- 增加 `reported_evidence_level` 和 E2 最低门禁。
- 增加 `oracle_isolation_level`：hard_sandbox、soft_denylist、failed。
- 明确 `--dangerously-bypass` 下默认不能声明 hard sandbox。
- 增加 private oracle 路径泄漏扫描范围和目标。
- 增加现有脚本复用清单。
- 将完成条件拆成 MVP、MVP+1、MVP+2。
- 增加 evidence-gate/provider-param/oracle path leak/run-order/manual-review 自测。

#### Risk Focus

- E2 是否仍可能被单次 run 或 manifest 声明误标。
- hidden oracle 是否仍可能在无硬隔离下进入 E2。
- MVP 是否仍然过大。
- 是否有新的 blocking 级可执行性问题。

#### Verification Status

- 文档已修改。
- 未做代码实现；本轮审查 closure。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- 只把会导致实施方向错误、MVP 不可落地或 benchmark 可信度失真的问题列为 blocking。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Harness Plan Closure Reviewer | Round 1 accepted blocking fixes require fresh closure review | E2 gate、oracle isolation、MVP scope |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Harness Plan Closure Reviewer | multi_agent_v1.spawn_agent | `019e7d3a-fd8a-7940-8919-302607613beb` | spawn_agent + subagent_notification | no | Round 2 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Harness Plan Closure Reviewer

##### Summary

Round 1 的两项 blocking fix 基本已闭合：计划已经把 E2 证据等级改为 harness 计算，并补了 repeats/provider/oracle/manual-review 门禁；也明确了 `--dangerously-bypass-approvals-and-sandbox` 下不能默认声明 `hard_sandbox`。但仍存在一个会影响 benchmark 可信度的阻塞级歧义：`soft_denylist` 是否可在用户接受风险后进入 E2 utility aggregate。

##### Blocking Findings

- `soft_denylist` 的 E2 规则存在冲突，可能让无硬隔离结果进入 E2 aggregate。计划一处写明 accepted soft isolation 明确不进入 E2，且任一 E2 条件不满足只能是 E1/E2-candidate；Evidence Gate 也禁止 soft_denylist 默认进入 E2。但 oracle contract 又写 `soft_denylist` 不得进入 E2 utility aggregate，除非用户显式接受该风险。这个例外会让实现者把 bypass 模式下的软隔离结果纳入 E2 utility aggregate，削弱 hidden oracle isolation 的可信度。

##### Non-blocking Risks

- MVP scope 已拆分，但 Runner 的 “MVP 支持” 包含生成 `aggregate-report.md`，而验收切片把 aggregate report 放到 MVP+1，Phase 3 又把 aggregate 与 repeats 放在一起。这会影响首个闭环边界。
- Round 2 review report 仍是 pending 状态，尚未记录本轮 reviewer output / closure status。

##### Required Fixes

- 统一 `soft_denylist` 规则：在 `--dangerously-bypass-approvals-and-sandbox` 或任何非 hard sandbox 情况下，`soft_denylist` 最多只能是 E1/E2-candidate，不能进入 E2 utility aggregate。用户显式接受风险只能作为 report 标注或单独 exploratory aggregate，不能提升为 E2。
- 明确首个可运行闭环是否生成 aggregate。如果 aggregate 不属于 MVP，就把 Runner “MVP 支持” 中的 `aggregate-report.md` 移到 MVP+1/Phase 3；如果保留，则说明它只是单 pair summary，不是 utility aggregate/E2 aggregate。

##### Missing Tests / Plan Gaps

- 必要自测已经补齐：evidence gate、provider-param gap、oracle path leak、run-order alternation、manual-review persistence。
- 还缺一个针对上述 blocking 歧义的自测：`soft_denylist + accepted risk` 不得进入 E2 utility aggregate。

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Harness Plan Closure Reviewer | `soft_denylist` accepted-risk 例外会让无硬隔离结果进入 E2 | blocking | accept | E2 可信度依赖 hidden oracle hard isolation；用户接受风险不能改变证据等级 | 删除 accepted-risk 升格例外；明确 `soft_denylist` 只能是 E1/E2-candidate；accepted risk 只能标注或进入 exploratory aggregate；新增 self-test | 已修复 |
| Harness Plan Closure Reviewer | MVP aggregate 边界不一致 | non-blocking | accept | MVP 首个闭环应只证明单 pair 可运行，不应混入 utility aggregate | 将 MVP 输出改为 `run-summary.md`；`aggregate-report.md` 标为 MVP+1 起生成；执行流程写成 aggregate enabled 时才更新 | 已修复 |
| Harness Plan Closure Reviewer | Round 2 report pending | non-blocking | accept | 审查报告必须闭环 | 写入本轮 reviewer output、主 agent response 和 closure status | 已修复 |
| Harness Plan Closure Reviewer | 缺少 `soft_denylist + accepted risk` 自测 | non-blocking | accept | 防止未来实现重新引入升格例外 | 新增 `soft-denylist accepted-risk test` | 已修复 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2 completed in this report
- Blocking re-review launch records:
  - `019e7d3a-fd8a-7940-8919-302607613beb`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes

## Final Conclusion

工程计划已完成二轮对抗审查并闭环。下一步可以进入 TaskSpace paired benchmark harness 的 MVP 实施：先实现 `single-file-fast-fix` 的真实 Whale CLI paired run、prompt guard、oracle isolation 记录、pair report、run summary 和 harness self-tests；再推进 `order-pipeline-growth` 与 utility aggregate。

## Round 3: Final Closure Review

### Review Input

#### Objective

验证 Round 2 blocking 修复是否真正闭合，重点确认 `soft_denylist` 不会因用户 accepted risk 被提升为 E2，并确认 MVP 边界不再混入 utility aggregate。

#### Target Locations

- `docs/plans/2026-05-31-taskspace-benchmark-harness-implementation-plan.md`
- `vs_review/2026-05-31-taskspace-benchmark-harness-plan-review.md`

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| Harness Final Closure Reviewer | Round 2 accepted blocking fix requires fresh closure review | `soft_denylist` E2 exclusion、MVP aggregate boundary |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| Harness Final Closure Reviewer | multi_agent_v1.spawn_agent | `019e7d3d-a934-72f1-a8b6-e5d160153067` | spawn_agent + subagent_notification | no | Round 3 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless needed | yes |

### Reviewer Outputs

#### Harness Final Closure Reviewer

##### Summary

Round 2 blocking appears closed. The implementation plan now prevents `soft_denylist` and accepted soft-isolation risk from becoming E2 utility evidence, keeps MVP to `pair-report.md` + `run-summary.md`, adds the requested self-test, and the review report records Round 2 output plus main-agent response.

##### Blocking Findings

none

##### Non-blocking Risks

- Minor wording risk: implementation plan line 41 listed “hard sandbox, or accepted soft isolation 且明确不进入 E2” under “E2 最低门禁”. This was slightly awkward because accepted soft isolation is explicitly non-E2, but later rules were already clear enough.

##### Required Fixes

none

##### Missing Tests / Plan Gaps

none blocking.

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| Harness Final Closure Reviewer | No blocking findings | blocking | accept | Round 3 found no blocking issue | n/a | n/a |
| Harness Final Closure Reviewer | Line 41 wording could be misread | non-blocking | accept | The E2 gate sentence should not include accepted soft isolation in the same bullet | Changed the bullet to: hard sandbox is required for E2; accepted soft isolation must downgrade to E1/E2-candidate | n/a |

### Closure Status

- Blocking findings found: no
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 3 completed in this report
- Blocking re-review launch records:
  - `019e7d3d-a934-72f1-a8b6-e5d160153067`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Allowed to proceed: yes

## Final Conclusion After Round 3

工程计划已完成三轮对抗审查并闭环。下一步可以进入 TaskSpace paired benchmark harness 的 MVP 实施：先实现 `single-file-fast-fix` 的真实 Whale CLI paired run、prompt guard、oracle isolation 记录、pair report、run summary 和 harness self-tests；再推进 `order-pipeline-growth` 与 utility aggregate。
