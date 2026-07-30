# Subagent VS Review: R7.1 W0 事实基础

- Created: 2026-07-30T03:19:45+08:00
- Updated: 2026-07-30T11:53:00+08:00
- Report schema: adversarial-v1
- Task: 审查 R7.1 W0 的实现、测试、真实样本证据与问题关闭是否成立
- Report path: `vs_review/2026-07-30-r7-1-w0-factual-foundation-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: W0 完整性与证据有效性审查

### Review Input

#### Objective

独立尝试证伪 W0 已完成的结论。W0 声明关闭：

- `R71-GI-007`：每个 provider request 的唯一一级失败分类、secondary tags、sibling copy 和
  receipt/revision/cache 归因；
- `R71-GI-005`：节点状态拒绝向 Agent 忠实透传 Runtime 已知的机械事实，不注入动作建议。

#### Review Target

代码实现、生产接线、测试有效性、真实运行日志、COE 证据和关闭文档。

#### Target Locations

- `third_party/codex-cli/codex-rs/core/src/action_map/response.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/`
- `third_party/codex-cli/codex-rs/core/src/action_map/runtime/transactions.rs`
- `third_party/codex-cli/codex-rs/core/src/session/taskspace_response.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
- `third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs`
- `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1`
- `scripts/taskspace-benchmark/lib/r7-request-observability.ps1`
- `scripts/taskspace-benchmark/lib/ordinary-tool-outcome.ps1`
- `scripts/taskspace-benchmark/lib/native-cadence.ps1`
- `scripts/taskspace-benchmark/lib/performance-observation.ps1`
- `scripts/taskspace-benchmark/report-r7-five-layer-matrix.ps1`
- `scripts/taskspace-benchmark/test-*.ps1`
- `docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md`
- `docs/v0.0.5/build-R7/48-r7.1-w0-factual-foundation-result.md`
- `coe/2026-07-29-19-18-r7-a2-c-rejection-taxonomy.md`
- `target/r7-w0-live-credentialed/subscription-billing-repair/20260730-025642-404`
- Commit range `1d086fd7e^..a9264f5ff`

#### Change Introduction

W0 新增 request 级失败 taxonomy 和 provider wire receipt identity，扩展节点状态 violation 的结构化事实，
统一 ordinary Tool 失败解析，并据此将 GI-005、GI-007 从开放问题移入关闭记录。

#### Risk Focus

- 状态事实是否在所有生产失败路径中保留，还是只有部分 transaction/sequence 路径保留；
- `node_state_invalid` 是否错误替代其他 violation，或泄漏语义判断/动作建议；
- request 一级分类是否真的互斥且完整，优先级是否掩盖更关键的失败；
- sibling copy 去重是否会把多个真实独立失败误当复制；
- receipt-before、wire role、revision 和 cache 是否存在 off-by-one、错误 carrier 或缺失 terminal 事件；
- observer 是否仍可在解析失败、缺失边界、未知 schema 或旧字段下生成误导性完整报告；
- 单次 live sample 是否足够满足关闭标准，测试是否只验证实现自身叙事；
- metrics、performance report 和 request taxonomy 是否仍有平行口径。

#### User-Perspective Review Focus

- Agent 实际看到的状态拒绝是否完整、直接、可理解且无 Runtime 建议；
- 失败信息是否因 sibling 复制、嵌套字符串、角色转换或上下文构造而扭曲；
- 报告读者能否明确区分业务成功、普通 Tool 失败、协议失败、状态失败和 Map terminal。

#### Implementation Completeness Focus

- 从 rooted DAG 产生 violation，到 runtime/session/sequence，再到 model-visible Tool output 的完整生产路径；
- 从 rollout/provider wire 原始事件，到 request reconstruction、matrix report 和性能报告的完整生产路径；
- Waiting、Completed、Blocked、Ready/InFlight、multi-parent、重复 reservation、原子零执行等测试是否覆盖真实入口；
- live artifacts 是否由本次实现二进制产生，是否存在事后手工改写、mock 或只在测试中生效的接线。

#### Target Benefit Focus

- W0 声明的收益是“事实可可信归因”和“状态反馈无丢失/扭曲”，不是降低 request、token 或 wall time；
- 检查 baseline、关闭标准、测量方法、对比证据和可能回归；
- 不把 receipt-before 的低缓存、sequence failure 或 Agent 第二次误选节点误写成 W0 已解决收益。

#### Assumptions To Attack

- `token_count` 与 provider request 一一对应；
- wire trace terminal cache facts 总是完整且顺序稳定；
- 同 request 内最高优先级 failure class 足以表示唯一根因；
- 相同 failure signature 的 sibling 结果必然只是复制；
- model-visible failure JSON 总在首行且可完整解析；
- `actual_state` 和 `unsatisfied_predecessor_ids` 在并发或多前驱状态下保持一致；
- live request 12、13、14 足以证明没有因原因缺失发生同形重试；
- historical metrics 不会污染新报告或关闭结论。

#### Adversarial Lenses

- requirements
- state
- failure
- data
- implementation-completeness
- target-benefit
- testing
- observability
- maintenance

#### Verification Status

- Rust rooted DAG、sequence、runtime transaction 和 model-visible 聚焦测试已通过；
- provider wire receipt identity 聚焦测试已通过；
- PowerShell trace、request observer、performance、metrics、cost、contract、matrix 和 harness 门禁已通过；
- locked Whale build 已通过；
- `subscription-billing-repair` Standard/TaskSpace 各运行一次并通过外部验证；
- 尚未执行 W0 的独立对抗性审查；
- 单次 live run 只作为诊断验收，不作为 projection policy 晋升证据。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Try to falsify the closure of GI-005 and GI-007.
- Report only correctness, completeness, evidence, observability, maintenance, or user-comprehension issues with concrete counterexamples.
- Distinguish blocking findings from non-blocking benefit warnings.

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
| complex | 20 minutes | 10 minutes once when alive | 2 | accepted blocking finding must be fixed and reviewed by a fresh replacement |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | W0 已声明阶段完成并关闭两个问题，首要风险是生产接线、真实证据或关闭标准只完成一部分 | 状态反馈全链路、observer 全链路、测试与 live evidence |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | `multi_agent_v1.spawn_agent` | `019faf52-85fa-7260-897c-55634799451c` (`Heisenberg`) | spawn tool result in parent session | `fork_context=false` | Round 1 Review Input | main-agent history、reasoning、drafts、conclusions、full diff dump | yes |

### Reviewer Timeout Records

| Reviewer | Attempt | Wait | Result | Action |
|---|---:|---:|---|---|
| implementation-completeness-adversary | 1 | 20 minutes maximum | completed within initial wait | completed |

### Reviewer Outputs

#### Summary

Reviewer verdict: **W0 的关闭结论不成立，应重新打开 R71-GI-005 和 R71-GI-007**。

结构化状态事实已经进入 Function/Custom Tool 输出，保留样本也确实包含 Waiting/Completed
拒绝。但 ToolSearch sibling 会扭曲该结构，observer 遗漏受支持的非 Function Tool
形态，畸形失败仍能作为 ordinary Tool 完成对账，sibling copy 缺乏因果身份，保留样本也没有证明最终
实现候选。

#### Blocking Findings

1. **GI-007 taxonomy 遗漏受支持的非 Function request 形态。**
   - Broken assumption: 所有生产 Tool request 都进入一级分类。
   - Trigger: `custom_tool_call`、`tool_search_call` 或 `local_shell_call` 在 Standard/TaskSpace
     中失败。
   - Evidence:
     `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1:185-260` 只解析
     `function_call/function_call_output`；生产事件 codec 在
     `third_party/codex-cli/codex-rs/core/src/action_map/event_codec.rs:107-138`
     支持更多类型。
   - Impact: 失败 Tool 可被遗漏，request 仍被归为 `none`，总数仍可形成表面正确的对账。
   - Proof needed: 两种模式覆盖所有受支持执行型 call/output 载体的生产 fixture。

2. **GI-007 对畸形失败和不完整 receipt fail-open。**
   - Broken assumption: `classification_reconciled=true` 表示所有失败和因果事实都已成功解析。
   - Trigger: failure JSON 畸形，或 receipt 有 role 但 hash/revision/delta 缺失、`complete=false`。
   - Evidence:
     `r7-five-layer-trace-analysis.ps1:68-123` 静默吞掉 JSON 解析错误并降级为
     `ordinary_tool/tool_failed_unclassified`；`r7-request-observability.ps1:132-165`
     只复制 receipt 字段，没有完整性门禁。
   - Impact: 损坏或因果不完整的证据仍可被标记为已知、已对账、可比较。
   - Proof needed: schema/parse health/receipt identity 门禁将结果降为 `partial` 或 `blocked`。

3. **GI-007 sibling-copy 统计把独立失败合并。**
   - Broken assumption: 相同 class/code/violation signature 可以证明 sibling-copy 身份。
   - Trigger: 两个独立执行的 call 都返回 `shell_exit_1`，或两个节点独立产生相同 state violation。
   - Evidence:
     `r7-request-observability.ps1:23-49` 的 signature 不包含 call identity、执行状态或因果来源；
     reviewer 和主线程反例都得到 `2 failed calls -> 1 sibling copy`。
   - Impact: 多个真实独立失败被错误从唯一失败计数中删除。
   - Proof needed: 显式 derivative-copy 因果身份或等价的 zero-dispatch provenance。

4. **GI-005 状态事实没有在 ToolSearch sibling 输出中忠实保留。**
   - Broken assumption: 每个 model-visible sibling 都直接保留结构化状态反馈，且不存在嵌套 JSON。
   - Trigger: 状态拒绝与 ToolSearch sibling 同时出现。
   - Evidence:
     `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs:349-393` 生成
     `status=completed` 的空 ToolSearch pairing，并将原始失败 JSON 放入
     `ToolSearchFailureV1.error.message` 字符串；
     `third_party/codex-cli/codex-rs/core/src/tools/sequence.rs:460-470` 将同一状态失败复制到每个 call。
   - Impact: sibling 表面完成，直接 `error.violations[]` 丢失，嵌套 JSON 回归。
   - Proof needed: TaskSpace state rejection + ToolSearch 的端到端测试，断言失败语义和 typed facts。

5. **保留 live run 不能证明最终关闭候选。**
   - Broken assumption: 当前 retained run 由最终脚本和可验证二进制生成。
   - Trigger: build attestation 无效，观测 artifact 早于最后 metrics 实现提交生成。
   - Evidence:
     `whale-binary-preflight-health.json:20-22` 为
     `binary_sha_mismatch,codex_source_commit_mismatch`；
     `run-status.json` 的 `final_aggregate_ready=false` 且 provenance 字段为空；
     `performance-observation.json` 与 `w0-request-observability.json` 均早于
     `4a136ca32` 生成。
   - Impact: source-to-binary provenance 和最终共享 metrics 行为没有被 retained evidence 证明。
   - Proof needed: 修复后由最终 committed candidate 生成、attestation 通过的 live run。

#### Non-blocking Risks

1. 单次运行不能证明“不再因同一原因重复同形请求”的收益具有稳定性；需要 repeat 或显式 retry-cause
   telemetry。
2. rollout、wire、receipt 和 cache 仍依赖 request 位置对齐；等长但重排/重复时会错配，需要跨源逻辑
   request identity。
3. 关闭文档的恢复路径叙述错误：request 14 实际是 `read_map`，request 15 才执行 corrective
   Patch，且出现 ordinary `apply_patch` failure。

#### User-Perspective Checks

| Check | Result |
|---|---|
| Function-call Waiting/Completed rejection 直接返回事实且不注入建议 | pass |
| ToolSearch sibling 返回同样清晰的状态事实 | fail |
| 报告读者能相信每个 request 只有一个正确一级分类 | fail |
| receipt/revision/cache 归因完整且因果相连 | partial |
| Agent 无需中间 read 即完成纠正 | fail |
| 证据畸形或不完整时报告 fail-closed | fail |

#### Implementation Completeness

| Area | Status |
|---|---|
| GI-005 typed DAG state facts | complete |
| GI-005 Function/Custom response path | complete |
| GI-005 ToolSearch sibling path | incomplete, blocking |
| GI-005 state matrix tests | partial |
| GI-007 Function-call taxonomy | partial |
| GI-007 complete request taxonomy | incomplete, blocking |
| GI-007 sibling-copy correlation | incomplete, blocking |
| GI-007 receipt/revision/cache capture | partial |
| GI-007 fail-closed behavior | incomplete, blocking |
| Final live-evidence provenance | incomplete, blocking |

#### Target Benefit Checks

- Function-call 路径已经实现直接事实反馈。
- “所有受支持 request 均可可信归因”尚未实现。
- live sample 的 9 个 receipt correlation 只能证明该样本有数据，不能证明通用完整性。
- 单次运行不能证明消除了 cause-based retry。
- W0 不要求性能收益，本审查没有将 request/token/cache 波动当作阻断项。

#### Required Fixes

- 扩展 trace extraction/taxonomy 到所有受支持的执行型 call/output 载体。
- 使用显式 derivative-copy provenance 取代 signature 去重。
- ToolSearch sibling 保留结构化状态事实和失败语义。
- parse/schema/receipt/cadence/provenance 缺陷必须让报告降级并失去比较资格。
- rollout/wire/receipt/revision/cache 使用稳定的逻辑 request identity。
- 生成 final-commit-attested live run，并纠正文档中的 request 14 叙述。

#### Missing Tests

- TaskSpace state rejection + ToolSearch sibling。
- Standard/TaskSpace 的 Custom、ToolSearch、LocalShell taxonomy。
- malformed failure JSON 导致报告拒绝。
- 独立同码失败与真正 sibling copy 的区分。
- receipt 缺 hash/revision/delta、`complete=false`、旧 schema 和多 receipt。
- cadence 解析错误令顶层 observation 失去资格。
- 跨源重排、retry、重复 terminal、缺 output。
- retained artifact 保存 request identity、source hash 和 implementation commit。

#### Missing Logs / Observability

- 每个 call 的 parse status 和 schema version。
- derivative copy 的 causal call ID/copy group/zero-dispatch reason。
- 跨源 durable logical request ID。
- receipt identity 完整性计数。
- structured 与 nested sibling failure carrier 形态。
- artifact generator version、implementation commit、source hashes 和 final aggregate readiness。

## Round 2: W0 修复闭环复审

### Review Input

#### Objective

独立尝试证伪 R71-GI-005 与 R71-GI-007 已达到关闭条件的主张，并逐项复测 Round 1 的 6 个
blocking findings。

#### Review Target

- W0 修复提交 `e9d705a23558d3f777179ad8696351866e79081a`
- 当前文档提交 `e7c600f25`
- 全局约束 C-01 至 C-21
- W0 代码、测试、COE、唯一问题清单和 final-commit retained matrix

#### Target Locations

- `third_party/codex-cli/codex-rs/core/src/action_map/response.rs`
- `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/`
- `third_party/codex-cli/codex-rs/core/src/tools/failure_provenance.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
- `third_party/codex-cli/codex-rs/core/src/tools/sequence.rs`
- `third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs`
- `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- `third_party/codex-cli/codex-rs/protocol/src/protocol.rs`
- `scripts/taskspace-benchmark/lib/r7-call-evidence.ps1`
- `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1`
- `scripts/taskspace-benchmark/lib/r7-request-observability.ps1`
- `scripts/taskspace-benchmark/lib/r7-artifact-provenance.ps1`
- `scripts/taskspace-benchmark/report-r7-five-layer-matrix.ps1`
- `scripts/taskspace-benchmark/run-r7-five-layer-matrix.ps1`
- `docs/v0.0.5/build-R7/38-r7-five-layer-integrated-change-constraints.md`
- `docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md`
- `docs/v0.0.5/build-R7/48-r7.1-w0-factual-foundation-result.md`
- `coe/2026-07-29-19-18-r7-a2-c-rejection-taxonomy.md`
- `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/e9d705a23558d3f777179ad8696351866e79081a/20260730-045032-584`

#### Change Introduction

Round 2 候选扩展 call/output 观测模型并使证据 fail-closed；增加 explicit zero-dispatch copy provenance
和 provider request identity；重构 ToolSearch failure carrier；拆分 canonical/evaluated state scope；
正式矩阵增加 binary/run artifact provenance 门。

#### Risk Focus

- 生产接线是否完整，还是测试只验证自定义 fixture；
- malformed、unknown、orphan、duplicate、missing、retry 和重排是否仍可 fail-open；
- request/logical request/attempt identity 是否真实因果关联；
- provenance 是否可接受 stale、dirty、伪造、部分或事后生成工件；
- ToolSearch、Custom、LocalShell、MCP 是否保留准确失败语义；
- 新增反馈是否违反 C-01 至 C-21，尤其 Runtime 越界、普通 Tool 侵入和语义建议；
- retained matrix 是否真的满足 GI-005/GI-007 close criteria。

#### User-Perspective Review Focus

Agent-visible feedback 必须是直接、机械、无歧义、无丢失且无动作建议的事实；报告读者必须能区分业务成功、
Tool execution、协议/状态失败、sibling copy、receipt/cache 和 evidence invalidity。

#### Implementation Completeness Focus

逐段追踪 DAG violation 到 model-visible result，以及 provider response 到 rollout/wire、observer、
eligibility 和报告的完整生产链；检查 fixture 是否与生产 ResponseItem 和事件形态一致。

#### Target Benefit Focus

W0 只声明可信事实归因和无扭曲状态反馈，不声明请求、Token 或缓存收益。审查应验证这些收益边界，没有把
GI-001/003/004 的现存失败误写为 W0 收益。

#### Assumptions To Attack

- 非 Function Tool 形态均被完整覆盖；
- token count 只对应完成的 provider response；
- retry/terminal 重排不会破坏 identity；
- attestation/provenance 不可由不一致工件伪造；
- null canonical state 对 rejected transaction 中新节点仍无歧义；
- `classification_reconciled=true` 必然意味着证据完整。

#### Adversarial Lenses

- requirements
- state
- failure
- data
- implementation-completeness
- target-benefit
- testing
- observability
- maintenance

#### Verification Status

- `cargo test -p codex-core --lib`：1926 passed、0 failed、3 ignored；
- `cargo check --workspace`：passed；
- 相关 PowerShell trace/report/cost/performance/harness tests：passed；
- retained matrix：24/24 business success、taxonomy reconciled、artifact provenance valid。

#### Reviewer Instructions

- fresh internal subagent；
- `fork_context=false`，不继承主线程历史；
- 只读，直接读取目标文件与原始 artifact；
- 不修改文件；
- 尝试证伪而不是确认；
- blocking finding 必须给出反例、触发条件、影响、所需证明及路径行号。

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 25 minutes | 10 minutes once when alive | 2 | accepted blocking finding 必须修复并由新的 fresh reviewer 复审 |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | Round 1 blocking closure 修复涉及状态反馈、Tool carrier、provider identity 和正式证据链 | 生产接线、证据 fail-closed、全局边界回归 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | `multi_agent_v1.spawn_agent` | `019fafaf-810e-72f1-bea0-ced3007b425e` (`Boyle`) | spawn tool result in parent session | false | Round 2 Review Input | 主线程历史、reasoning、草稿、结论、完整 diff dump | yes |

## Round 1 Main Agent Response

#### Blocking Finding Triage

1. `accept`：生产事件类型与 observer 支持集合确实不一致；归入重新打开的 GI-007。
2. `accept`：主线程反例确认畸形 `TaskSpaceResponseCommitFailureV1` 被归为
   `ordinary_tool/tool_failed_unclassified`，且 `classification_reconciled=true`；归入 GI-007。
3. `accept`：主线程反例确认两个独立 `shell_exit_1` 被统计为一份 sibling copy；归入 GI-007。
4. `accept`：ToolSearch provider pairing 的 `completed` 可能是载体合同，但 supplemental failure
   确实把 typed state payload 重新嵌套为 message 字符串；归入 GI-005。
5. `accept`：attestation 和最终实现证据不合格；重新验收必须由修复后的最终 commit 生成。
   Reviewer 所述“13/21 requests 与文档 2/3 冲突”不成立：文档同样记录 13/21，2/3 是失败普通
   Tool call 数。本裁决接受 provenance 阻断，不接受该附带表述。

#### Non-blocking Risk Triage

1. `accept`：稳定收益需要 repeat；作为重新关闭 GI-005 的 live 行为证据要求。
2. `accept`：位置相关不是可靠因果 identity；作为 GI-007 的工程验收项。
3. `accept`：保留 trace 明确显示 request 14 是 `read_map`，request 15 才是 corrective execute；
   W0 历史结果文档需要纠正。

#### Required Fix Triage

- `accept`：扩展所有受支持执行型 call/output 载体的观测支持；tracked by GI-007。
- `accept`：引入明确的 derivative-copy provenance；tracked by GI-007。
- `accept`：修复 ToolSearch sibling 的状态失败 carrier；tracked by GI-005。
- `accept`：所有 evidence health 缺陷统一 fail-closed；tracked by GI-007。
- `accept`：建立跨源逻辑 request identity；tracked by GI-007。
- `accept`：最终 committed candidate 重新实跑并纠正文档；tracked by GI-005/GI-007。

#### Missing Test Triage

- `accept`：增加 state rejection + ToolSearch sibling 测试。
- `accept`：增加 Custom/ToolSearch/LocalShell 双模式 taxonomy 测试。
- `accept`：增加 malformed failure JSON fail-closed 测试。
- `accept`：增加 independent same-code 与 derivative copy 对照测试。
- `accept`：增加 receipt identity 完整性矩阵。
- `accept`：增加 cadence parse failure 顶层失效测试。
- `accept`：增加跨源重排、retry、重复 terminal、缺 output 测试。
- `accept`：增加 retained artifact provenance 测试。

#### Missing Observability Triage

- `accept`：记录 per-call parse/schema health。
- `accept`：记录 derivative-copy causal identity 和 zero-dispatch provenance。
- `accept`：记录 durable logical request identity。
- `accept`：输出 receipt identity completeness counters。
- `accept`：输出 sibling failure carrier shape。
- `accept`：输出 artifact generator/source/finalization provenance。

#### Main-Agent Supplemental Blocking Finding

6. **`actual_state` 混淆拒绝事务中的临时候选状态与 canonical 状态。**
   - `transactions.rs:219-228` 先应用 `complete_node` 再应用 reservation；
   - live request 13 在同一拒绝事务内先完成 `explore`，再尝试把 action 绑定到 `explore`；
   - rejection 返回 `state_commit=false`、`canonical_revision=17`，同时返回
     `actual_state=completed`；
   - Agent 在下一轮明确把它理解为 canonical Map 中 “explore already completed”，随后额外
     `read_map`；read_map 显示 revision 17 的 `explore` 实际仍为 `ready`。
   - Triage: `accept`，blocking，归入 GI-005。
   - Required product contract: 同时忠实区分 canonical pre-transaction state 与
     evaluated-at-violation state，不注入建议、不解释任务语义。

#### Main-Agent Counterexample Validation

```json
{
  "independent_failed_calls": 2,
  "reported_sibling_copies": 1,
  "malformed_failure_class": "ordinary_tool",
  "malformed_failure_code": "tool_failed_unclassified",
  "malformed_classification_reconciled": true
}
```

### Closure Status

- Blocking findings found: 6
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - pending after fixes
- Blocking re-review launch records:
  - pending after fixes
- Rejected findings backed by evidence: reviewer finding 5 的附带数值冲突表述已用文档与 artifact
  字段语义驳回
- Deferred findings documented: none
- Implementation completeness gaps resolved or accepted by user: no
- Target benefit warnings recorded: yes
- Blocked reason: GI-005/GI-007 closure criteria not met; accepted blocking findings await fixes and
  fresh closure review
- Allowed to proceed: no

## Final Conclusion

W0 不能维持 `Completed`。R71-GI-005 与 R71-GI-007 必须重新打开；在实现、测试、最终 commit
实跑和 fresh blocking closure review 全部通过前，不得进入 W1。

### Reviewer Output

Reviewer verdict：**R71-GI-005 与 R71-GI-007 均未达到关闭条件，W0 blocking closure 应拒绝。**

#### Blocking Findings

1. **Retained live trace 仍出现 candidate/canonical 误读。**
   - `single-file-fast-fix/r-2/a1` 中，拒绝事务返回 canonical `verify=ready`、evaluated
     `verify=completed`；
   - Agent 随后表述 “Either way, it's already done”，并产生额外普通 Tool 请求、
     `taskspace_control_required` 拒绝和重复测试；
   - 这直接反驳 W0 结果文档与 COE 中“未复现”的结论。
2. **Provider request universe 排除了失败 attempt，且 WebSocket 未接线。**
   - observer 先过滤 `response_completed` 再检查完整性；
   - HTTP retry 每轮生成新的 logical ID，`attempt_seq` 恒为 1；
   - WebSocket 路径没有进入 wire request identity；
   - retained matrix 仅覆盖 278 个成功的 ChatCompletions terminal，未覆盖 retry、cancel、
     stream failure、shape-only 或 WebSocket。
3. **ToolSearch zero-dispatch 仍被报告为成功，MCP fixture 不是生产形态。**
   - ToolSearch skipped pairing 使用 `status=completed`，observer 未消费
     `TaskSpaceToolSkippedV1` supplemental；
   - MCP fixture 使用生产 `ResponseItem` 不存在的 `mcp_tool_call`，真实 MCP 是 namespaced
     `FunctionCall` / `FunctionCallOutput`。
4. **Duplicate output 与任意 Tool 文本可覆盖或伪造 failure provenance。**
   - 同一 call 的后续 output 采用 last-write-wins；
   - 普通 Tool 返回形似 TaskSpace failure schema 的文本时，可伪造 failure class 和 sibling
     copy provenance。
5. **Artifact provenance 与提交态合同门不足。**
   - provenance 记录但不拒绝 `final_aggregate_ready=false`；
   - raw rollout/wire 没有内容哈希锚定；
   - attestation 未绑定 clean source tree 与可执行探针；
   - `taskspace_contract_manifest_v1.json` 中 `sequence.rs` 哈希已漂移，
     `test-r7-five-layer-contracts.ps1` 失败。

#### Non-blocking Risks

- 被拒绝事务中新建的节点只返回 `canonical_state_before_transaction=null`，没有机械区分
  “canonical 中不存在”与“状态缺失”。
- observer 把所有 `token_count` 当作 provider response boundary；无 provider identity 的普通
  token/rate-limit 更新会阻断合法 trace。

#### Boundary Check

Reviewer 未发现 Runtime 自动选 node、自动修正 mutation、读取普通 Tool 业务参数或其他
C-01 至 C-21 越界。阻断项集中在忠实反馈、观测全集和证据可信度。

### Main Agent Triage

- 5 个 blocking findings：全部 `accept`。
- 2 个 non-blocking risks：全部 `accept`，随对应 blocking fix 一并处理。
- Round 2 结论：W0 保持 blocked，不得进入 W1。
- 修复依赖顺序：
  1. canonical / rejected-candidate 事实结构；
  2. provider dispatch / logical request / attempt / terminal identity；
  3. Tool carrier 与可信 supplemental evidence；
  4. build/raw artifact/final aggregate provenance；
  5. final committed repeat-live 与 fresh closure review。

## Round 3: W0 修复闭环完整性复审

### Review Input

- Objective: 对当前 W0 候选重新证伪 GI-005/GI-007 的关闭条件，并逐项复查 Round 1/2 blocker
  与 C-01 至 C-21。
- Reviewed commit: `d64b74191`
- Review mode: fresh internal subagent，`fork_context=false`，只读。
- Verification supplied:
  - 24-run current-commit matrix；
  - Rust 全量、构建、PowerShell gates；
  - Round 1/2 修复链和 artifact provenance。
- Required output: `PASS/BLOCKED`、可复现 blocker、全局约束矩阵、旧 blocker closure matrix，
  不得修改文件。

### Reviewer Launch Record

| Reviewer | Session / Job ID | Context Forked | Read-only | Result |
|---|---|---:|---:|---|
| implementation-completeness-adversary (`Banach`) | `019fb018-6196-7522-a1e6-f8b9eca3c468` | false | yes | blocked |

### Reviewer Output

Reviewer 接受此前 candidate/canonical、provider identity、artifact provenance、ordinary Tool
可信边界等修复，也明确裁定 4 次 `map-request` 保守 `read_map` 不是反馈丢失或扭曲。但发现三个新的
blocking closure gap：

1. **生产 `ToolSearchFailureV3` 与 observer provenance 合同漂移。**
   - producer 没有 `failure_provenance`，observer 却强制要求；
   - fixture 注入了生产不存在的 `tool_sequence_skip` provenance；
   - 合法 ToolSearch 执行失败因此会被报告误判为证据损坏。
2. **缺少 state rejection + ToolSearch 的精确组合测试。**
   - state rejection、ToolSearch 和 complete/reserve canonical/candidate 反例分别有测试；
   - 没有一条测试穿过真实状态机、sequence preflight 和 ToolSearch sibling 的完整路径。
3. **TaskSpace MCP 双模式生产 fixture 不完整。**
   - Standard 已使用 namespaced Function MCP；
   - TaskSpace 缺同形 fixture，metrics 测试仍使用生产不存在的 `mcp_tool_call`。

Round 3 的 C-01、C-14、C-15 因上述合同漂移失败，其余 C-02 至 C-21 未发现 Runtime 越界、普通
Tool 参数侵入、Map 权威分叉或生命周期回归。

### Main Agent Triage

三个 blocking finding 全部 `accept`，四个 non-blocking test gap 也全部纳入同一修复：

- canonical node absent 直接序列化测试；
- provider response cross-request affected calls 反例；
- affected call subset 与 duplicate 反例；
- duplicate supplemental 继续 fail-closed。

### Repairs

| Finding | 修复 | 确定性证据 |
|---|---|---|
| ToolSearch provenance 漂移 | 新增 `tool_execution_failure_provenance(call_id)`；producer、observer、fixture 统一要求 `scope=tool_execution`、self cause、精确单 call set、`zero_dispatch=false` | Rust producer JSON 断言；Standard/TaskSpace 正例；错误 scope、缺失/残缺/重复/cross-request 反例 |
| state rejection + ToolSearch 缺口 | 使用真实 Map 初始化、release、complete+reserve 冲突和 ToolSearch sibling 走 production response path | canonical=`ready`、candidate=`completed`、ToolSearch execution=`failed`、完整 affected calls、零 dispatch、零 state commit |
| TaskSpace MCP fixture 缺口 | TaskSpace observer 与 metrics fixture 改为 namespaced `function_call/function_call_output` | 两个 PowerShell harness 通过 |
| canonical absent | 直接断言 `node_present=false`、canonical state=`null`、candidate state 保留 | Rust 单测通过 |

修复还发现旧 `sync-r7-five-layer-contract-manifest.ps1` 会把当前合同降级到 FLA-1。该脚本已收敛为只生成
当前 A2-C 五层合同，删除旧 phase/兼容分支，并由 14 项合同与 inventory gate 验证。

### Replacement Review Readiness

- Fix commit: `1196b4e99ca507d5cb3bcb619343053463cf752c`
- Rust: 1931 passed、0 failed、3 ignored
- Build/check: passed
- PowerShell gates: 14/14 passed
- Current-commit matrix: 24/24，artifact provenance=`valid`，final aggregate=`finalized`
- Replacement requirement: 因 Round 3 存在接受的 blocker，必须由另一名 `fork_context=false`
  reviewer 重新审查，Banach 不得作为自己的修复关闭 reviewer。

## Round 4: W0 replacement closure review

### Review Input

- Objective: 对 Round 3 修复候选和 current-commit retained matrix 执行 replacement closure review，
  逐项复查 GI-005、GI-007、Round 1/2/3 blocker 与 C-01 至 C-21。
- Reviewed commit: `3b0831208`
- Review mode: fresh internal subagent，`fork_context=false`，只读。
- Reviewer: implementation-completeness-adversary (`Ampere`)
- Session / Job ID: `019fb049-4437-7331-be0e-104759654f9c`
- Result: blocked

### Reviewer Output

Reviewer 接受了 Round 1/2 的主要生产链修复，但发现四个新的 blocking closure gap：

1. **supplemental failure 对畸形、未知和残缺结构仍可能 fail-open。**
   - observer 只对已成功解析且 schema 已知的 payload 做严格校验；
   - malformed JSON、未知保留 schema family 或缺少关键字段的可信 developer carrier 可能被跳过。
2. **state rejection + ToolSearch 组合测试没有穿过异步生产入口。**
   - 现有测试覆盖内部 sequence helper，但没有证明 `execute_response_tool_sequence` 在 dispatch 前执行相同
     preflight 和零提交路径。
3. **状态拒绝的文档统计与 retained artifact 不一致。**
   - 文档声称 7 个 `node_state_invalid`、4 个后续 `read_map`；
   - 被审工件实际有 13 个状态拒绝，按 arm 为 7/2/4，只有 3 个下一请求 `read_map`。
4. **ToolSearch failure origin 仍由错误文本中的 scope 推断。**
   - execution failure helper 从嵌套错误 payload 读取 scope；
   - provider-response rejection 与真实 ToolSearch execution failure 没有由 typed control flow 结构性分离。

Reviewer 另将 C-10 单 Patch 和 C-13 成本标记为失败。主线程不把它们作为 W0 新 blocker：两项始终分别由
R71-GI-006、R71-GI-008 公开追踪，W0 从未声明关闭；但 current matrix 必须继续如实报告，不得用该边界
掩盖现象。

### Main Agent Triage

- 四个 W0 blocking findings：全部 `accept`。
- C-10/C-13：作为已知下游开放问题保留，不并入 GI-005/GI-007，不作为 W0 修复范围漂移的理由。
- 结论：Round 4 不通过；完成修复后必须由 Ampere 之外的 fresh reviewer 重新审查。

### Repairs

| Finding | 修复 | 确定性证据 |
|---|---|---|
| supplemental fail-open | 对保留 schema family 的 malformed/unknown/incomplete/duplicate payload、非布尔 `success`、错误 role/scope/call set 全部 fail-closed | `test-r7-supplemental-failure-evidence.ps1` 正反例 |
| 异步生产入口缺口 | 新增真实 Map 初始化、release、complete+reserve 和 ToolSearch sibling 的 `execute_response_tool_sequence` 回归；将 ToolSearch dispatch 指向必失败 handler，证明 preflight 已零执行拦截 | `production_entry_rejects_complete_then_tool_search_before_dispatch` |
| 文档手工错计 | 报告自动输出被拒请求数、违规事实数、canonical/candidate 状态对和下一请求动作；同一请求多个 violation 不重复计为多个请求 | request observer fixture 与 current matrix `trace-analysis.json` |
| ToolSearch origin 推断 | provider-response rejection 只生成原生 pairing；仅真实 ToolSearch dispatch error 生成 `tool_execution` supplemental，不读取错误文本决定来源 | Rust 正例与伪造 `scope=provider_response` 错误文本反例 |
| 合同身份漂移 | 同步生产 manifest 内容 SHA；全量合同身份测试重新通过 | `production_manifest_matches_its_identity` |

### Replacement Review Readiness

- Fix commits: `2c6d65ddb`、`92cccd7e0`、`50c2b77d1`
- Rust: 1933 passed、0 failed、3 ignored
- Build/check: passed
- PowerShell gates: 15/15 passed
- Current-commit matrix:
  `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/50c2b77d199cfa41615f03e97f8dc07e72cd4c74/20260730-083136-712`
- Matrix: 24/24，artifact provenance=`valid`，final aggregate=`finalized`
- State rejection facts: 11 requests / 11 violations / 2 next-request `read_map`
- Replacement requirement: 必须由另一名 `fork_context=false` reviewer 复审，Ampere 不得关闭自己的
  blocker。

## Round 5: W0 strict-evidence replacement review

### Review Input

- Objective: 对 Round 4 修复、current-commit retained matrix、GI-005/GI-007 关闭条件和 C-01 至
  C-21 执行完整 replacement review。
- Reviewed HEAD: `ddab5f12c`
- Production code through: `50c2b77d1`
- Review mode: fresh internal subagent，`fork_context=false`，只读，`gpt-5.6-sol/xhigh`。
- Reviewer: implementation-completeness-adversary (`Franklin`)
- Session / Job ID: `019fb07d-cd31-7f92-9aa3-db1901e1a5f1`
- Timeout record: 10 分钟检查点未完成；在 20 分钟 high-risk 初始窗口内完成。
- Result: blocked

### Reviewer Output

Reviewer 将 GI-007 与 W0 判定为 `BLOCKED`；GI-005 的 Rust 生产实现未被证伪，但因 GI-007 是验收
前置，不能单独关闭。两个 blocking finding 均由独立内存反例复现：

1. **畸形及类型混淆 supplemental evidence 仍可 fail-open。**
   - `{"padding":0,"schema_version":"ToolSearchFailureV3"` 因 schema 不在首字段而被忽略，request 仍为
     `primary_failure_class=none`、`evidence_health=valid`；
   - `affected_call_ids` 使用字符串而非数组时，PowerShell pipeline 将其提升为单元素数组并接受；
   - 根因分别是 malformed 识别依赖字段位置，以及 shape validator 没有验证 provenance JSON 类型。
2. **状态 violation 汇总按内容三元组合并不同事实。**
   - 同一 request 的两个 violation 使用相同 node/canonical/candidate state，但 subjects、allowed states
     和前驱集合不同；
   - 汇总仍从 2 条压成 1 条；
   - 根因是去重 key 只有 `node_id|canonical_state|candidate_state`，重新引入了 Round 1 已淘汰的
     signature heuristic。

Reviewer 对 Round 1-4 其他 blocker 均判定 `PASS`：异步生产入口、ToolSearch typed origin、
canonical/candidate 状态域、provider attempt/WS 接线、ordinary Tool trust boundary、MCP 生产形态、
artifact provenance 和 manifest identity 未发现回归。

### Global Constraint Result

| Constraint | Result | Review conclusion |
|---|---|---|
| C-03 | fail | malformed supplemental 可静默遗漏，独立 violation 可被合并 |
| C-14 | fail | 上述两项可产生虚假 comparison eligibility 或错误归因 |
| C-10 | fail, known downstream | current matrix 有 3 次 TaskSpace multi-Patch，由 GI-006 追踪 |
| C-13 | fail, known downstream | receipt/cache 成本未达发布门，由 GI-002/GI-008 追踪 |
| C-01/C-02/C-04 至 C-09/C-11/C-12/C-15 至 C-21 | pass | 未发现 Runtime 越界、普通 Tool 侵入、双 Map 权威、current/Open 回归或兼容分叉 |

C-10/C-13 没有被 W0 隐藏或恶化，不作为本轮新 W0 blocker；C-03/C-14 是必须修复的新证伪结果。

### User-Perspective And Benefit Checks

- 11 个 live state rejection 中 10 个后续 reasoning 正确区分 canonical/candidate；
- 1 个 Agent 将 `allowed_states` 误读为状态迁移集合，但反馈仍是纯机械事实，没有建议、自动改绑或自动推进；
  该现象继续作为 GI-003/GI-004 下游理解风险，不要求 Runtime 增加语义纠正；
- 当前矩阵 provenance、11/11/2 状态统计与文档一致；
- W0 只主张事实保真与可归因，不主张性能收益，24/24 业务成功不能作为 W0 收益证明。

### Main Agent Triage

- Finding 1：`accept`，归 GI-007/C-03/C-14。
- Finding 2：`accept`，归 GI-007/C-03/C-14。
- 用户理解风险：`accept` 为下游观察，不扩大 W0 Runtime 语义。
- C-10/C-13：保留在 GI-006 与 GI-002/GI-008，不重复建账。
- 结论：Round 5 不通过；修复后必须由 Franklin 之外的 fresh reviewer 重新审查。

### Repairs

| Finding | 修复 | 确定性证据 |
|---|---|---|
| malformed/type-confused supplemental | 保留 schema family 使用顺序无关识别；严格验证 status/success、object/array/string/boolean、error class、scope、call identity 和 schema-specific 字段 | reordered malformed、scalar/object/boolean/array mutation 负向矩阵 |
| violation signature merge | 删除状态内容 key；仅按 producer 的 `copy_group_id + affected_call_ids + zero_dispatch` 合并 sibling carrier，按原始 ordinal 保存完整 violation | 同状态对不同 subjects/allowed/predecessors 保留 3 条；同 copy group 两个 sibling 只计一次 |

### Replacement Review Readiness

- Fix commit: `e72637d070764c2f2de03a978761a6739780f37b`
- Rust production baseline: 1933 passed、0 failed、3 ignored
- PowerShell gates: 15/15 passed
- Current-commit matrix:
  `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/e72637d070764c2f2de03a978761a6739780f37b/20260730-091713-589`
- Matrix: 24/24 run finalized，23/24 business success，24/24 comparison eligible，
  artifact provenance=`valid`
- TaskSpace requests: 289，classification unreconciled run=0
- State rejection facts: 11 requests / 11 violations / 3 next-request `read_map`
- Replacement requirement: 必须由另一名 `fork_context=false` reviewer 复审，Franklin 不得关闭自己的
  blocker。

## Round 6: W0 evidence-identity replacement review

### Review Input

- Objective: 对 Round 5 修复、current-commit retained matrix、GI-005/GI-007 关闭条件和 C-01 至
  C-21 执行完整 replacement review。
- Reviewed HEAD: `905f8adf3`
- Production code through: `e72637d07`
- Review mode: fresh internal subagent，`fork_context=false`，只读，`gpt-5.6-sol/xhigh`。
- Reviewer: evidence-identity-adversary (`Curie`)
- Session / Job ID: `019fb0a6-6602-7c90-b655-9d4048351510`
- Result: blocked

### Reviewer Output

Reviewer 独立重算了 GI-005 的 11 个状态拒绝：11/11 反馈均保留机械事实，11 个后续 reasoning 均引用了
正确状态事实，3 次 `read_map` 发生在正确理解之后，没有证据表明 carrier 继续扭曲或丢失语义。GI-005
生产路径本身未被证伪，但其关闭仍依赖 GI-007 的可信 observer。

GI-007 与 W0 因四个证据身份缺口继续 `BLOCKED`：

1. **保留 supplemental schema family 仍存在 fail-open 形态。**
   - schema array、root array 和使用 JSON unicode escape 的畸形截断可以绕过保留 family 识别；
   - 这些输入可能被静默当作无 supplemental 的普通请求。
2. **provider/skip/bound provenance 可以伪造。**
   - observer 验证了部分字段存在，但未证明 copy group、cause call、reservation 和 owning request
     是该 call 的精确生产身份。
3. **request 级 token 事实不完整。**
   - wire 已有 output/reasoning/total token，但 request path 只保留 input/cached；
   - 因而报告无法证明输出 token 没有在 join/aggregate 中丢失，也无法做总量守恒。
4. **resolved manifest 只封存文件哈希，没有验证内容身份。**
   - sample、repeat、side、mode、projection、model、capability、prompt/fixture 和 Docker image 可以与
     四臂合同不一致，但旧 provenance 仍可能判定 valid。

### Global Constraint Result

| Constraint | Result | Review conclusion |
|---|---|---|
| C-01/C-03/C-14 | fail | F1-F4 可导致事实静默遗漏、伪造因果身份或错误比较资格 |
| C-13 | fail, known downstream | TaskSpace 成本仍高于 Standard，由 GI-002/GI-008 追踪；F3 属于 W0 观测缺口 |
| C-10 | fail, known downstream | multi-Patch 仍由 GI-006 追踪 |
| C-02/C-04 至 C-09/C-11/C-12/C-15 至 C-21 | pass | 未发现 Runtime 语义决策、普通 Tool 入侵、Map 双权威或生命周期扩张 |

### Main Agent Triage

- F1-F4：全部 `accept`，归 GI-007/C-01/C-03/C-14。
- GI-005 语义路径：接受 reviewer 的独立重算结论，但在 GI-007 关闭前不单独关闭。
- C-10/C-13：继续留在 GI-006 与 GI-002/GI-008，不扩大 W0 产品范围。
- 结论：Round 6 不通过；修复后必须由 Curie 之外的 fresh reviewer 重新审查。

### Repairs

| Finding | 修复 | 确定性证据 |
|---|---|---|
| reserved family fail-open | JSON unicode escape 归一化后识别保留 family；root/schema array、非 object root、畸形截断和严格 schema 字符串类型全部 fail-closed | 扩展 supplemental 正反例矩阵 |
| provenance 可伪造 | provider、ToolSearch、skip、bound 分别校验精确 copy group、call set、cause call、reservation、scope、zero-dispatch 和先后关系；state/status/class code 也进入语义合同 | provider/skip/bound forged provenance 负向测试 |
| request token 丢失 | request path 保留 input/cached/output/reasoning/total 五字段；校验非负数、字段不等式与 `total=input+output`；run aggregate 与 request sum 对账 | `test-r7-provider-token-identity.ps1` 和 347-request live 汇总 |
| resolved manifest 内容未验 | 新增 resolved manifest identity validator；逐 run 校验 sample/repeat/side/mode/policy/model/binary/provider/capability，逐 sample-repeat 校验精确四臂及共享 prompt/fixture/model/image/binary | `test-r7-resolved-manifest-identity.ps1` 与 24-run provenance |

### Replacement Review Readiness

- Fix commit: `2005442d34416820a888dc7395ac8ba1b3812635`
- Rust: 1933 passed、0 failed、3 ignored
- Build/check: passed
- PowerShell gates: 14/14 scripts passed
- Current-commit matrix:
  `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/2005442d34416820a888dc7395ac8ba1b3812635/20260730-101516-783`
- Matrix: 24/24 run finalized，24/24 business success，24/24 comparison eligible，
  artifact provenance=`valid`
- Request token identity: 347 requests；input `7,947,276`、cached `3,519,488`、output
  `135,380`、reasoning `54,288`、total `8,082,656`，request/run aggregate 全部对账
- State rejection facts: 5 requests / 5 violations / 2 next-request `read_map`
- Replacement requirement: 必须由另一名 `fork_context=false` reviewer 复审，Curie 不得关闭自己的
  blocker。

## Round 7: W0 evaluation-authority replacement review

### Review Input

- Objective: 对 Round 6 修复、retained matrix、GI-005/GI-007 关闭条件和 C-01 至 C-21 执行完整
  replacement review。
- Reviewed production commit: `2005442d34416820a888dc7395ac8ba1b3812635`
- Review mode: fresh internal subagent，`fork_context=false`，只读，`gpt-5.6-sol/xhigh`。
- Reviewer: evidence-authority-adversary (`Turing`)
- Result: blocked

### Reviewer Output

Reviewer 接受 Round 1-6 中 ToolSearch carrier、canonical/evaluated state、provider attempt、reserved
family 基础门、request token 字段完整性和 resolved manifest 内容校验的主要修复，但发现四个更深的证据权威
blocker：

1. **保留 supplemental JSON 仍可 fail-open。**
   - root/object 形态之外，递归重复保留属性和相关畸形结构没有被唯一属性门拒绝；
   - malformed family 仍可能被当作没有 supplemental 的请求。
2. **failure provenance 与真实 request/output 顺序没有完全绑定。**
   - reservation/cause 可以跨 request 引用；
   - supplemental 可以在受影响 call 的实际 output 缺失时成立；
   - 后续 ordinary output 可以覆盖 supplemental failure；
   - skip/bound 没有证明 cause output 或 reservation 属于同一个 owning request。
3. **token identity 不是精确整数身份。**
   - string、fraction、double 和超过 IEEE-754 精确范围的值可能通过数值比较；
   - 多层求和即使总数表面相同，也不能证明逐 request token 没有被舍入或替换。
4. **评估身份仍可由同一轮 mutable manifest 自证。**
   - sample、repeat、arm、model、image、sandbox、provider API/transport 和 Tool identity 没有由独立冻结
     authority 声明；
   - resolved manifest 与 matrix manifest 可以同步篡改后继续相互通过。

### Main Agent Triage

- F1-F4：全部 `accept`，归 GI-007/C-01/C-03/C-14。
- GI-005：reviewer 未给出新的 carrier 扭曲反例，但继续依赖 GI-007 的可信 observer，暂不关闭。
- C-10/C-13：current matrix 中的 multi-Patch 和成本问题继续归 GI-006/GI-008，不扩大 W0 范围。
- 结论：Round 7 不通过；修复后必须由 Turing 之外的 fresh reviewer 重新审查。

### Repairs

| Finding | 修复 | 确定性证据 |
|---|---|---|
| supplemental fail-open | 严格识别保留 JSON family，递归拒绝重复属性和畸形 root/schema；字段顺序不影响识别 | 扩展 supplemental malformed/duplicate 负向矩阵 |
| provenance/order 可伪造 | actual output、owning request、copy group、affected calls、cause output、reservation 和 ordinal 全部精确绑定；普通后续 output 不得覆盖已识别 failure | cross-request、missing-output、overwrite、skip/bound forgery 反例 |
| token 非精确身份 | token/count 只接受非负 Int64；拒绝 string/fraction/double/越界，request/run/report 使用精确求和 | provider token 与 performance observation 类型/守恒门 |
| mutable manifest 自证 | 新增冻结 evaluation contract 和 production authority hash chain；逐 run 校验 sample/repeat/arm/model/reasoning/image/sandbox/API/transport，实际 wire tools hash/count 必须稳定且匹配权威 | resolved manifest、matrix harness 和 artifact provenance 篡改反例 |

### Replacement Review Readiness

- Fix commits: `93985d3f5`、`a2f697c6b`、`9dc9add2d`、`a67737478`
- Rust: 1933 passed、0 failed、3 ignored
- Build/check: passed
- PowerShell gates: 14/14 scripts passed
- 第一份矩阵因 observation 将整数 token 转成 double，在 report gate 被拒绝并弃用。
- Retained current-commit matrix:
  `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/a677374782ff4ac15ad3242af19e1a681fec7e08/20260730-114506-875`
- Matrix: 24/24 finalized，24/24 business success，24/24 comparison eligible，
  artifact provenance=`valid`，0 findings。
- Request token identity: 360 requests；input `8,794,679`、cached `3,649,536`、output
  `146,406`、reasoning `64,166`、total `8,941,085`。
- Replacement requirement: 必须由另一名 `fork_context=false` reviewer 复审，Turing 不得关闭自己的
  blocker。

## Round 8: W0 output/token/candidate-authority review

### Review Input

- Objective: 对 Round 7 修复、retained matrix、GI-005/GI-007 和 C-01 至 C-21 执行 fresh
  closure review。
- Reviewed HEAD: `f167616ff`
- Reviewed candidate: `a677374782ff4ac15ad3242af19e1a681fec7e08`
- Primary reviewer: `Halley`，session `019fb12c-...`，fresh、`fork_context=false`、只读；
  platform safety classifier 终止，没有 reviewer output，不能计为通过。
- Replacement reviewer: evidence-authority-adversary (`Jason`)
- Session / Job ID: `019fb136-da88-7031-bbd4-a723998d9aff`
- Review mode: fresh internal subagent，`fork_context=false`，只读，`gpt-5.6-sol/xhigh`。
- Result: blocked

### Reviewer Output

Reviewer 独立重算了 24/24 run、360 request、token 汇总和 192/192 raw seal，与报告一致；GI-005 的
机械反馈事实被判定 `PASS`。GI-007 仍有三个真实 blocker：

1. provider supplemental 只绑定 call set，没有绑定每个 call 的实际 output 内容与先后关系；
2. observation 把字符串 `"0"` token 归一为 null 后，仍可标记 complete/eligible，report 又把缺失值
   默认成零；
3. current dirty worktree 没有进入 binary/provenance 一致性条件，报告脚本、evaluation contract 和
   production authority 的当前字节也没有绑定候选提交 blob。

Reviewer 另建议所有 business-incomplete run 不得进入 finalized report。

### Main Agent Triage

- output binding：`accept`，归 GI-007/C-03/C-14。
- token fail-open：`accept`，归 GI-007/C-03/C-14。
- dirty/current commit bytes：`accept`，归 GI-007/C-01/C-14。
- business-incomplete 不得 finalized：`reject`。C-14 要求保留 Agent 的真实失败；`finalized` 表示证据
  已封存，不表示所有业务运行成功。基础设施/token 无效 run 必须 fail-closed，但合法观察到的 Agent
  incomplete run 必须保留。

### Repairs

| Finding | 修复 | 确定性证据 |
|---|---|---|
| supplemental 未绑定 actual output | call 保存实际 output 原文；provider supplemental 要求全部 affected call 已有一次失败 output 且原文逐字相同；supplemental-before-output 和覆盖均拒绝 | `test-r7-supplemental-failure-evidence.ps1` 与 live serialized carrier |
| token 缺失/类型 fail-open | performance observation 与 request/report 只接受非负精确 Int64；字符串、浮点、负数、溢出、缺失及算术不守恒均 `invalid`，并记录稳定事件 | `test-performance-observation.ps1`、`test-r7-provider-token-identity.ps1` |
| candidate authority 未闭合 | current worktree 必须 clean；evaluation contract、production authority、report script 当前字节必须匹配 manifest candidate commit 的 Git blob | `test-r7-candidate-authority.ps1`、matrix harness、artifact provenance |
| production serialized output 差异 | production output 为 JSON string；observer 机械解码显式 bool `success`，不再回退文本启发式误判；exact-output 合同不放宽 | 第一轮 matrix report fail-closed、E-025、serialized fixture |

### Replacement Review Readiness

- Fix commits: `ba8cf3564`、`7b13b6bc6`、`4869e3f65`、`f2dea4765`、`ebaedcf6c`
- Rust: 1933 passed、0 failed、3 ignored
- Build/check: passed
- PowerShell gates: 15/15 passed
- 第一份 `f2dea4765` matrix 的 24/24 raw run 完成，但 report 发现 production serialized output
  解码缺口并 fail-closed；没有把失败候选作为有效结果。
- Retained current-commit matrix:
  `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/ebaedcf6c6c641a0d4361d68ab9c99fc0c594f22/20260730-125939-756`
- Matrix: 24/24 complete、24/24 business success、24/24 comparison eligible；
  artifact provenance=`valid`、192 raw artifacts、0 findings、final aggregate=`finalized`。
- Request tokens: 385 requests；input `8,991,942`、cached `4,454,912`、uncached
  `4,537,030`、output `140,946`、reasoning `62,246`、total `9,132,888`。
- State rejection facts: 12 requests / 12 violations / 2 next-request `read_map`。
- Replacement requirement: 必须由 Jason 之外的新 `fork_context=false` reviewer 关闭 blocker。

## Round 9: W0 full-constraint closure review

### Review Input

- Objective: 尝试证伪 Round 8 修复、retained current-commit matrix、GI-005/GI-007 关闭条件和
  C-01 至 C-21；不得把业务成功当作证据正确性的替代。
- Review target: production/observer implementation、负向 fixtures、冻结评估 authority、current-commit
  matrix 与 R7.1 全局约束。
- Candidate commit: `ebaedcf6c6c641a0d4361d68ab9c99fc0c594f22`
- Matrix:
  `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/ebaedcf6c6c641a0d4361d68ab9c99fc0c594f22/20260730-125939-756`
- Reviewer mode: fresh internal subagent，`fork_context=false`，只读，`gpt-5.6-sol/xhigh`。
- Reviewer: evidence-authority-adversary (`Mencius`)
- Session / Job ID: `019fb16d-c10a-7082-b81a-7c328f19e9da`
- Mechanism: internal `spawn_agent`，未继承主线程上下文。
- Context excluded: 主线程聊天、隐藏 reasoning、既有结论和说服性摘要。
- Required output: summary、blocking findings、non-blocking risks、C-01 至 C-21 逐项结论、GI-005/GI-007
  关闭意见、对矩阵关键数字和 seal 的独立复算、每项 finding 的路径/行号与可复现证据。
- Timeout policy: high-risk，初始等待 20 分钟，必要时只延长一次。
- Status: blocked

### Reviewer Output

Reviewer 独立复算 retained matrix，确认 24/24 run、385 request、token 守恒、192 raw seal 与报告一致，
且候选生产代码未漂移；同时复现四项 blocker：

1. `TaskSpaceResponseCommitFailureV3` 和直接 `TaskSpaceControlResultV2` 可缺失 violations 或节点状态
   机械事实，observer 仍标记 valid；
2. 非 token 的 provider/action/failure count 仍使用 `double`，缺失时 report 默认成 0；
3. W0 结果和唯一问题清单仍引用 `a677374` 的旧矩阵数字，与 retained `ebaedcf6c` 工件冲突；
4. provider supplemental 在比较前排序 affected call IDs，交换实际调用顺序仍可通过。

判定：C-01/C-03/C-09/C-12/C-13/C-14 FAIL，GI-005/GI-007 均不可关闭。

### Main Agent Triage

- F1-F4：全部 `accept`。它们均属于 producer→observer→report→seal 事实身份缺口，不需要扩大 Runtime
  状态机职责，也不需要修改 Agent 行为。
- detached/signed root、可复现构建和 live Blocked carrier：记录为非阻断证据增强，不纳入本轮产品语义。
- 结论：Round 9 不通过；修复后必须使用 Mencius 之外的新空白 reviewer。

### Repairs

| Finding | 修复 | 确定性证据 |
|---|---|---|
| state rejection 结构残缺 | 按 producer 合同校验 state violation；`node_state_invalid` 必须保留 node、canonical/candidate state、allowed states 与两侧前驱；直接 control failure 从 `error.actual.violations` 读取 | 新增 state failure contract 正反例 |
| count 精度/缺失 | 普通计数只接受非负 Int64；observation 记录稳定 invalid event，report 对必需字段 fail-closed，聚合使用 BigInt 中间和 Int64 上界 | fractional/string/2^53/missing 反例 |
| 权威文档冲突 | 唯一问题清单和 W0 结果先绑定 retained `ebaedcf6c` sealed matrix，并明确它是 Round 9 修复前证据；新矩阵后只切换一个当前引用 | 文档数值与 artifact 独立复算一致 |
| affected call 顺序丢失 | request call IDs 与 supplemental affected IDs 逐位 ordinal 比较，不再排序成集合 | 交换两个 call 的负向 fixture |

## Current Conclusion

W0 当前保持 `validating`。Round 9 四项 blocker 已在 `ea6f27b1b` 修复；必须完成 current-commit 正式
矩阵，并由 Mencius 之外的 fresh replacement reviewer 复审。通过前不得关闭 R71-GI-005/R71-GI-007，
也不得进入 W1。
