# Subagent VS Review: R7.1 W0 事实基础

- Created: 2026-07-30T03:19:45+08:00
- Updated: 2026-07-30T03:19:45+08:00
- Report schema: adversarial-v1
- Task: 审查 R7.1 W0 的实现、测试、真实样本证据与问题关闭是否成立
- Report path: `vs_review/2026-07-30-r7-1-w0-factual-foundation-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

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

待内部子代理完成后记录。

### Reviewer Outputs

待内部子代理完成后记录。

### Main Agent Response

待 reviewer findings 返回后逐项记录 `accept`、`reject` 或 `defer`。

### Closure Status

- Blocking findings found: unknown
- Accepted blocking findings fixed: n/a
- Blocking re-review completed: n/a
- Blocking re-review passed: n/a
- Blocking re-review round links:
  - n/a
- Blocking re-review launch records:
  - n/a
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Implementation completeness gaps resolved or accepted by user: unknown
- Target benefit warnings recorded: unknown
- Blocked reason: n/a
- Allowed to proceed: no

## Final Conclusion

审查进行中。
