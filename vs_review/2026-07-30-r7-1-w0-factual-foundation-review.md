# Subagent VS Review: R7.1 W0 事实基础

- Created: 2026-07-30T03:19:45+08:00
- Updated: 2026-07-30T03:37:36+08:00
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

### Main Agent Response

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
