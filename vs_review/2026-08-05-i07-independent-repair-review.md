# Subagent VS Review: R8-I07 独立修复

- Created: 2026-08-05T09:00:00+08:00
- Updated: 2026-08-05T20:39:25+08:00
- Report schema: adversarial-v1
- Task: 审查 I07-W0～W8、W10 是否真正修复请求/usage/边界观测问题，且没有形成新平行口径或虚假证据闭环
- Report path: `vs_review/2026-08-05-i07-independent-repair-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: closed

## Round 1: 独立修复完整性与事实可信性

### Review Input

#### Objective

验证 WhaleCode R8-I07 当前协议下的独立修复是否让 logical request、local attempt、boundary request、completed
response 和 usage 通过唯一 canonical facts 忠实、可复算地进入成本、缓存、性能、freshness 和 provenance 消费面。

#### Review Target

代码实现、测试策略、日志/观测合同、证据新鲜度与完成声明。

#### Target Locations

- `docs/v0.0.5/build-R8/I07/00-i07-observability-trust-repair-plan.md`
- `docs/v0.0.5/build-R8/I07/01-i07-independent-repair-result.md`
- `scripts/taskspace-benchmark/request_facts.py`
- `scripts/taskspace-benchmark/request_fact_*.py`
- `scripts/taskspace-benchmark/lib/request-facts.ps1`
- `scripts/taskspace-benchmark/lib/cost-instrumentation.ps1`
- `scripts/taskspace-benchmark/lib/provider-section-cost.ps1`
- `scripts/taskspace-benchmark/lib/performance-observation.ps1`
- `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1`
- `scripts/taskspace-benchmark/lib/r7-five-layer-evidence-freshness.ps1`
- `scripts/taskspace-benchmark/lib/r7-artifact-provenance.ps1`
- `scripts/taskspace-benchmark/lib/r7-request-facts-provenance.ps1`
- `scripts/taskspace-benchmark/docker/verify_provider_boundary.py`
- `scripts/cache-regression/cache_usage_contract.py`
- `scripts/taskspace-benchmark/request-fact-consumers.json`
- 相关 `test-*`、`test_*.py` 与 I07 fixtures

#### Change Introduction

实现新增 canonical `request-facts.json`，按身份区分状态快照、local attempt、boundary、completion 和 measured usage；
将性能、成本、缓存和 freshness 消费者迁移到该事实；封存来源 SHA 与 analyzer 组合哈希；增加 payload-free diagnostics。

#### Risk Focus

- 是否仍有生产消费者直接把 `payload_captured`、no-ID usage 或 terminal 行数当请求数。
- canonical parser 在 duplicate、retry、partial identity、missing terminal、boundary 缺失/冲突时是否错误给出 measured。
- cache strict contract 是否被通用 facts 无意放松，或 shape/usage 分母再次混同。
- provenance 是否能被篡改 source、遗漏 analyzer 文件、旧 schema、移动路径或重生成产物绕过。
- W0-W8 完成声明是否只由测试 fixture 支撑，生产入口是否真正接通。
- diagnostics 是否泄露 prompt、命令、Tool 正文或形成新的平行事实源。

#### User-Perspective Review Focus

- 报告使用者能否明确区分 attempt、boundary、completed、usage 和 unavailable/incomparable。
- 旧证据或缺证据是否会给出看似精确但不可复算的数字。
- 结果文档是否清楚标明未完成 W9/W11 和真实缓存回归状态。

#### Implementation Completeness Focus

- 逐项核对 W0～W8 的 production reader、integration entry、test evidence、runtime/log evidence。
- 检查是否存在 protocol/schema/test-only landing，或旧 fallback 仍在生产路径被调用。
- 检查 consumer inventory 是否会漏报别名、动态读取、其他语言或未登记目录。

#### Target Benefit Focus

- claimed reliability：8/15 与 10/11 能否逐 ID 复算且负例仍 fail closed。
- claimed consistency：同一运行在性能、缓存、成本、freshness 中的分母是否同源。
- claimed operational diagnosis：数字变化是否能由 diagnostics 和 provenance 定位。
- 已知未测：真实 Whale Agent/DeepSeek 缓存回归未执行，不得误判为已证明收益。

#### Assumptions To Attack

- request ID 在 rollout 与 wire 中总是完整且一致。
- 所有 terminal 状态枚举已经覆盖。
- boundary digest 足以唯一关联，且 boundary 文件缺失不会被误当作 0。
- request facts source path 一定位于 artifact_dir 且不会经过 symlink/相对路径变化。
- analyzer 组合哈希覆盖全部影响分类与汇总的代码。
- PowerShell JSON round-trip 不会改变语义或掩盖类型问题。

#### Adversarial Lenses

- implementation-completeness
- state
- input
- failure
- data
- maintenance
- testing
- observability
- target-benefit

#### Verification Status

- request/inventory 11 tests passed。
- cache regression 219 tests passed。
- cost、Harness、performance、provenance、freshness 与 I07 characterization passed。
- clean-commit 24-run offline report passed。
- 未运行真实 Whale Agent；TaskSpace Exec W9/W11 未实施。

#### Reviewer Instructions

- Fresh internal subagent session; no inherited main-agent context.
- Read target files directly; inspect repository state and relevant call paths.
- Do not modify files.
- Try to falsify completion and correctness claims rather than confirm them.
- Cite evidence paths and line numbers where possible.
- Return the exact report sections: summary, blocking findings, non-blocking risks, user-perspective checks,
  implementation completeness, target benefits, required fixes, missing tests, missing logs, evidence.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 20 minutes | one bounded 10-minute extension if alive | 2 | accepted blocking finding requires fresh closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | I07 宣称多个阶段已完成，最高风险是生产入口漏迁移或测试/协议层完成被误算为完整落地 | production integration、fallback、evidence closure |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | `multi_agent_v1.spawn_agent` | `019fd16e-3a32-7492-8e10-146b585c086a` (Lovelace) | spawn tool result in parent session | `fork_context=false` | Round 1 Review Input | main-agent history、reasoning、drafts、conclusions、full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| `R1-implementation-completeness` | implementation-completeness-adversary | 1 | `019fd16e-3a32-7492-8e10-146b585c086a` | 约 9 分钟 | completed | 正常返回完整结构化结果 | completed |

### Reviewer Outputs

#### R1-implementation-completeness

##### Summary

Reviewer 判定 W0-W8/W10 完成声明暂不成立。两个历史 fixture 已通过，但发现 6 个生产路径缺口：boundary 证据丢失
可能被写成 measured zero，cache/performance 仍可能从 incomparable facts 产生精确指标。

##### Blocking Findings

- **B1：usage 可被归属给 failed response。** rollout 带身份 usage 与同 ID wire `response_failed` 同时存在时，
  normalizer 仍选择 rollout usage，输出 `usage=measured` 且无 finding。
  - Broken assumption: 所有 normalized usage 都属于 completed response。
  - Trigger: 跨来源不一致、trace 损坏或 lifecycle 顺序异常。
  - Impact: failed request 进入 token/cache/cost。
  - Proof needed: failed terminal + rollout usage 必须产生稳定矛盾 finding、排除 usage 并使 usage incomparable。
- **B2：cache consumers 忽略 canonical incomparability。** section-cost reader 可跳过 malformed raw 行并聚合剩余
  shape/usage，performance 又消费该 summary。
  - Broken assumption: canonical availability 约束所有派生 cache 指标。
  - Trigger: JSON、identity、usage conflict。
  - Impact: incomparable 运行仍产生精确 cache rate 和 token denominator。
  - Proof needed: conflict fixture 使 cache rates 为 null，side comparison-ineligible。
- **B3：缺失 boundary log 会成为 measured zero。** shutdown 在 supervisor events 不存在时创建空文件，而 parser 只按
  文件存在判断 boundary available。
  - Broken assumption: 空 boundary 文件证明零请求。
  - Trigger: proxy crash、mount/copy failure、event-log loss。
  - Impact: API/预算请求数被低报为 0。
  - Proof needed: 缺失/不完整生命周期日志必须 unavailable/incomparable；健康零请求必须可区分。
- **B4：performance/freshness 把 completion 重新命名为 boundary request。** boundary unavailable/incomparable 时，
  两者回退 completed count。
  - Broken assumption: completed response 可替代实际 boundary request。
  - Trigger: supervisor 证据缺失或 reconciliation conflict。
  - Impact: accepted-but-failed 请求被漏计，`provider_requests` 语义漂移。
  - Proof needed: boundary 不可用时 `provider_requests=null`，completion 单列且 comparison/freshness fail closed。
- **B5：consumer inventory 没有覆盖 canonical consumers。** gate 只搜索四类 raw literal，读取 `request-facts.json`、
  import builder 或消费 summary 字段的代码可绕过。
  - Broken assumption: inventory PASS 证明所有 production readers 都已分类。
  - Trigger: 新增或已有 canonical artifact consumer。
  - Impact: semantic drift 不触发 W1 gate。
  - Proof needed: 覆盖 import、artifact 文件名、PowerShell generator 和 derived count 字段。
- **B6：相同 payload retry 无法对账。** 两个 physical attempts body digest 相同时，当前算法把 digest ambiguity 扩散到
  attempt、completion、usage 全部 incomparable。
  - Broken assumption: payload digest 唯一标识 physical attempt。
  - Trigger: transport/auth retry 发送相同 body。
  - Impact: 有效 completion/usage 被无谓失效。
  - Proof needed: 重复 body retry fixture；或把 boundary count 与 digest correlation availability 分离。

##### Non-blocking Risks

- **N1** analyzer hash 漏掉 `build-request-facts.py` 和 `lib/request-facts.ps1`。
- **N2** 生产生成器未传 `ExpectedModel`，freshness 未独立证明 boundary model。
- **N3** diagnostics 只有 source 名称，部分 reconciliation findings 缺 source line/location。
- **N4** 结果文档记录测试数量和 commits，但缺命令输出、fixture hash 或 durable evidence path。

##### User-Perspective Checks

| Check | Result |
|---|---|
| 区分 attempt/boundary/completion/usage | partial：字段存在，但 `provider_requests` 仍可表示 completion 或 shape count |
| 缺失证据输出 unavailable | fail：缺 boundary log 可变成 measured zero |
| 冲突证据抑制精确指标 | fail：cache consumer 继续聚合 |
| 报告暴露 count source | fail：表格未稳定展示 source discriminator |
| W9/W11 边界 | pass |
| 真实 cache regression 状态 | pass，明确未运行 |
| 诊断恢复路径 | partial：有 reason counts，source location 不完整 |

##### Implementation Completeness Checks

| Plan Item | Status | Finding Link |
|---|---|---|
| W0 | landed, narrow | none |
| W1 | partial | B5 |
| W2 | partial | B1、B6 |
| W3 | partial | B1 |
| W4 | partial | B3、B6 |
| W5 | partial | B4 |
| W6 | partial | B2 |
| W7 | partial | B4、N1、N2 |
| W8 | partial | N3 |
| W10 | not supportable before fixes | B1～B6 |

##### Target Benefit Checks

| Claimed Benefit | Result | Status | Finding Link |
|---|---|---|---|
| 8/15 fixture 修正 | achieved on narrow fixture | weak-evidence | B1 |
| 10/11 fixture 分层 | achieved on narrow fixture | weak-evidence | B3、B6 |
| negative cases fail closed | regressed for identified counterexamples | regressed | B1～B4 |
| cross-consumer consistency | multiple `provider_requests` meanings remain | regressed | B2、B4、B5 |
| operational diagnosis | aggregate codes exist, source localization partial | weak-evidence | N3 |
| real cache benefit | not run and correctly disclosed | unmeasured | none |

##### Required Fixes

- 修复 B1～B6，并封存 N1/N2 的 producer/model identity。

##### Missing Tests

- failed terminal + identity rollout usage；缺失日志 vs 健康零请求；相同 payload retries；cache conflict；
  boundary unavailable 的 performance/freshness；canonical consumer discovery；builder/wrapper/model provenance；invalid usage。

##### Missing Logs / Observability

- boundary start/stop/flush 与 copied count/bytes；派生 summary 的 availability/finding digest；reconciliation source line；
  count semantic/source；producer/entrypoint hash 与 expected model。

##### Evidence

- `scripts/taskspace-benchmark/request_facts.py:264-338,399-430`
- `scripts/taskspace-benchmark/lib/provider-section-cost.ps1:329-470`
- `scripts/taskspace-benchmark/lib/provider-boundary.ps1:88-118`
- `scripts/taskspace-benchmark/lib/performance-observation.ps1:44-67`
- `scripts/taskspace-benchmark/lib/r7-five-layer-evidence-freshness.ps1:271-306`
- `scripts/taskspace-benchmark/check-request-fact-consumers.py:13-48`
- `scripts/taskspace-benchmark/lib/r7-request-facts-provenance.ps1:1-30`

### Main Agent Response

| Reviewer | Finding | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|
| implementation-completeness | B1 failed terminal usage | blocking | accept | 本地主源码复核确认 `_normalized_rows` 无条件回退 rollout usage | `9dc661aa0` 排除 usage 并新增矛盾 finding | Round 2 closure closed |
| implementation-completeness | B2 cache ignores availability | blocking | accept | cache loop 直接读取 rows，summary 未以 canonical availability gate rates | `9dc661aa0`、`9e7536425` 让 side/fallback/aggregate fail closed | Round 3 closure review |
| implementation-completeness | B3 missing boundary becomes zero | blocking | accept | `provider-boundary.ps1` 明确创建空文件；parser 只看 path exists | `9dc661aa0` 增加完整 lifecycle；后续修复 aggregate/freshness null 传播 | Round 3 closure review |
| implementation-completeness | B4 completion fallback | blocking | accept | performance/freshness 两处源码存在明确 fallback | `9dc661aa0` 删除 completion fallback；后续修复 freshness aggregate | Round 3 closure review |
| implementation-completeness | B5 inventory misses canonical readers | blocking | accept | discovery pattern 只覆盖 raw literals | `9dc661aa0` 扩展 canonical consumer；后续补 usage alias 检测 | Round 3 closure review |
| implementation-completeness | B6 duplicate digest coupling | blocking | accept | 当前将 digest ambiguity 放入 wire errors；选择分离 boundary count 与 correlation，不侵入 Provider payload/header | `9dc661aa0` 分离 count/correlation；verifier 继续区分 correlation incomparable | Round 3 closure review |
| implementation-completeness | N1 producer files not sealed | non-blocking | accept | analyzer 调用链包含 builder/wrapper | `9dc661aa0` 纳入 builder/wrapper/reconciliation 组合哈希 | full regression passed |
| implementation-completeness | N2 expected model not sealed | non-blocking | accept | 由 boundary lifecycle sentinel 提供并封存 expected model，不新增外部平行配置 | `9dc661aa0` 封存 supervisor lifecycle model | architecture decision retained |
| implementation-completeness | N3 source location incomplete | non-blocking | accept | 原始 event 已有 line index，可机械保留 | 增加 rollout/wire/boundary/reconciliation line locations | focused diagnostics test |
| implementation-completeness | N4 durable result evidence weak | non-blocking | accept | 可记录命令、fixture SHA、review report 和 commit | result doc 已记录命令、fixture SHA 与 review path | final evidence update |

## Round 2: Blocking Closure Review

### Reviewer Launch Record

| Reviewer | Internal Mechanism | Session / Job ID | Context Forked | Input | Read-only |
|---|---|---|---|---|---|
| closure-adversary | `multi_agent_v1.spawn_agent` | `019fd185-443e-7151-b3ba-457ed1ace67b` (Newton) | `fork_context=false` | 最新 HEAD、B1～B6 closure packet | yes |

### Reviewer Output

- Summary: B1、B5、B6 已关闭；B2/B3 在 aggregate cache 仍会把空或 malformed side 写成 measured zero；B3/B4
  在 freshness 仍会把 unavailable side 省略后输出 0 或 partial exact total。
- Blocking findings: 2，均接受。
- Non-blocking risks: alias consumer 检测、expected model 独立来源、剩余 line location、durable command output。
- Evidence: reviewer 直接检查 clean HEAD `9e7536425`；未执行真实 Whale Agent/API run。

### Main Agent Response

| Finding | Severity | Decision | Action |
|---|---|---|---|
| aggregate empty/malformed -> measured zero | blocking | accept | aggregate 先发现预期 TaskSpace artifact scope；missing/invalid side 使 request/attempt/cache 为 null，并输出 stable finding |
| freshness unavailable -> zero/partial total | blocking | accept | 为每个 mode/run/aggregate 传播 count availability；healthy zero 由 side coverage 与 count 值分开判断 |
| alias consumer bypass | non-blocking | accept | 增加 canonical usage alias pattern 与 inventory role validation |
| independent ExpectedModel | non-blocking | reject | supervisor lifecycle 的 `allowed_model` 来自独立 boundary 配置并已封存；不再新增平行 expected-model 配置源 |
| remaining line locations | non-blocking | accept | 补齐可机械获得的 rollout/wire/reconciliation 行号 |
| durable command output | non-blocking | accept | 最终 closure 中记录命令结果、提交与证据路径 |

## Round 3: Latest-HEAD Closure Review

### Reviewer Launch Record

| Reviewer | Internal Mechanism | Session / Job ID | Context Forked | Input | Read-only |
|---|---|---|---|---|---|
| final-closure-adversary | `multi_agent_v1.spawn_agent` | `019fd191-e0ea-7b81-9b98-e47f71bc3764` (Bernoulli) | `fork_context=false` | clean HEAD `d5b768a22`、7 项 blocking invariant | yes |

### Reviewer Output

Reviewer 在本地临时 fixture 中复现 4 个 blocking：

1. aggregate 依赖可损坏的 `metrics.json`/right-path 推断 TaskSpace side，可能漏掉 left-side TaskSpace 后输出 partial exact total；
2. Python 将 JSON boolean 视为 integer，boundary `request_count:false` 或 `count:true` 可成为 measured count；
3. freshness 遇到 missing/invalid `logical-mode-map.json` 时仍保留初始 measured flag，输出 0/partial total；
4. lexical consumer gate 未发现 `request_fact_availability.py`、`request_fact_summary.py`、`request_fact_validation.py`。

Reviewer 确认 invariant 1、4、6 已通过；未运行 Whale Agent/API。完整 evidence 基于 clean HEAD
`d5b768a2209b5109fbf5169964c7031fc099c50c`、23 项前的 focused tests、219 cache tests 和静态 cache gate。

### Main Agent Response

| Finding | Severity | Decision | Action |
|---|---|---|---|
| alternating-side scope omission | blocking | accept | aggregate 只按每个 pair 的 `logical-mode-map.json` 选择 TaskSpace artifact；map 缺失/非法即整体不可比较 |
| boolean/schema boundary count | blocking | accept | boundary lifecycle/claim 强校验 schema v1 与 non-boolean integer；增加 boolean/schema 负例 |
| invalid mode map exact totals | blocking | accept | mode map 缺失/非法或 mode coverage 缺口同时使 Standard/TaskSpace totals 为 null |
| canonical-derived modules omitted | blocking | accept | 解析 `request_facts.py` 本地 import closure，并强制所有依赖进入 inventory |
| duplicate verifier wire count looks measured | non-blocking | accept | correlation incomparable 时 `wire_request_count=null` |
| `_put_once` location missing | non-blocking | accept | conflict finding 携带当前 source line，并将 location 与 semantic value 分离以保持幂等 |

## Round 4: Fail-Closed Scope Review

### Reviewer Launch Record

| Reviewer | Session / Job ID | Context Forked | Read-only |
|---|---|---|---|
| scope-contract-adversary (Copernicus) | `019fd1a3-cbe7-7953-a34a-d9c228a1fbc4` | `fork_context=false` | yes |

### Reviewer Output And Response

Reviewer 复现 4 个阻断项：无 wire 来源时 fallback 仍可能给出缓存结论；V4 aggregate 接受部分合同；performance/freshness
未完全以 mode map 为权威；durable cache validator 接受布尔计数。全部接受，并由 `b2c05ad5e`、`53d08313f`、
`15896eead` 修复。相关负例进入本地测试，精确事实在来源或合同不完整时统一为 null。

## Round 5: Evidence Contract Review

### Reviewer Launch Record

| Reviewer | Session / Job ID | Context Forked | Read-only |
|---|---|---|---|
| evidence-contract-adversary (Aristotle) | `019fd1b4-17ed-7331-8754-67a05b222390` | `fork_context=false` | yes |

### Reviewer Output And Response

Reviewer 在 clean HEAD `15896eead` 复现 5 个阻断项：rollout-only usage 可借 payload trace 通过；V4 source/availability/
计数合同不完整；suite cost 未强制 mode map；一个非法 pair 后仍保留其他 pair 的 partial aggregate；durable validator
未校验 wire request ordinal。全部接受，由 `4675f1a66` 修复并补负例。

## Round 6: Strict Identity Review

### Reviewer Launch Record

| Reviewer | Session / Job ID | Context Forked | Read-only |
|---|---|---|---|
| strict-identity-adversary (Mill) | `019fd1d1-214c-7532-aad8-806f95325484` | `fork_context=false` | yes |

### Reviewer Output And Response

Reviewer 在 clean HEAD `4675f1a66` 复现 4 个阻断项：V4 base/section 内部矛盾仍可通过；mode map 重复 JSON key
被 PowerShell 静默覆盖；缺失 `repeat` 仍形成 measured comparison；非法 mode map 与 `side_selection_skipped` 组合会
吞掉作用域错误并泄露精确值。全部接受，由 `8acd79b76` 修复。修复引入共享 strict mode-map reader，并将完整 V4
恒等式验证拆为独立合同模块。

## Round 7: Bounded Final Closure

### Reviewer Launch Record

| Reviewer | Session / Job ID | Context Forked | Input Scope | Read-only |
|---|---|---|---|---|
| bounded-final-adversary (Socrates) | `019fd1eb-fd67-7d92-b642-f527a38ab4e1` | `fork_context=false` | 仅 Round 6 四个反例与四条消费路径 | yes |

### Reviewer Output

`NO BLOCKING FINDINGS`。Reviewer 确认：

1. V4 base identity、section arrays/totals/means/medians 矛盾时 aggregate exact facts 为 null；
2. 重复 JSON key 同时阻断 cache aggregate 和 cost gate；
3. 缺失、布尔、小数或负数 `repeat` 均使 performance comparison unavailable；
4. invalid mode map 与 skipped 组合不会吞 warning，aggregates、ratios、row 和 Markdown 精确值均被抑制；
5. strict reader 已用于 cost、cache、performance、freshness 四条路径。

本轮未运行 Whale Agent 或 DeepSeek API。

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Rejected findings backed by evidence: yes
- Deferred findings documented: yes；I07-W9/W11 仍按专题计划等待 TaskSpace Exec 与授权生产验收
- Implementation completeness gaps resolved or accepted by user: yes（当前协议独立修复范围）
- Target benefit warnings recorded: yes；真实缓存收益仍未测
- Blocked reason: none for I07-W0～W8/W10
- Allowed to proceed: yes

## Final Conclusion

I07 当前协议下的独立修复通过对抗性闭环。该结论只关闭 W0～W8/W10 的确定性观测基础，不关闭全局 I07；W9/W11
继续等待 TaskSpace Exec 身份接入和用户授权的最小生产验收。
