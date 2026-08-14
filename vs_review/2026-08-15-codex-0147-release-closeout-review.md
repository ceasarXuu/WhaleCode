# Subagent VS Review: Codex 0.147 主线融合发布收口

- Created: 2026-08-15T00:00:00+08:00
- Updated: 2026-08-15T00:14:26+08:00
- Report schema: adversarial-v2
- Task: 对已声明完成的 Codex 0.147 主线融合计划做独立、缺陷优先的对抗性审查
- Report path: `vs_review/2026-08-15-codex-0147-release-closeout-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context; reviewer only receives the neutral review packet
- Status: blocked
- Control outcome: user-decision-required
- Automatic round budget: 2
- Completed rounds: 1
- Last known-good checkpoint: `3d97fed892314bb125437399ea654a8cf95931ef`

## Review Control Contract

### Frozen Objective

验证“Codex 0.147 主线融合计划 Phase A–E / U1–U17 已完成”的声明是否由真实生产路径、集成入口、测试和同步证据支持，并识别会使该完成声明失真的高影响缺陷。

### Acceptance Criteria

- U1–U17 的完成声明与代码、集成入口、生成物和 Git 证据一致。
- DeepSeek Responses、Flash 默认/Pro 可见、TaskSpace state/extension/RPC/final-wire 的生产路径已经落地，不是 scaffold、schema-only 或 mock-only。
- 测试适配没有掩盖 Whale 默认 provider/home/binary 产生的真实回归。
- U17 对失败、环境限制和延期项的归因有直接证据，未把阻塞当前产品目标的问题错误延期。
- `plan.md`、专题 README、`UPSTREAM.md`、overlay/replay ledger 和收口报告相互一致。

### Explicit Non-goals

- 本轮不修复发现，不启用 OpenAI hosted、Bedrock、Windows 或已延期 TUI 能力。
- 不进行真实 Whale Agent 或付费模型请求。
- 不重构同步框架，不创建新产品模块、API、依赖或持久化格式。
- 不访问或修改其他分支、worktree 或同级项目。

### Frozen Target Locations

- `docs/v0.0.5/codex-upstream-sync/plan.md`
- `docs/v0.0.5/codex-upstream-sync/README.md`
- `docs/v0.0.5/codex-upstream-sync/decisions.md`
- `docs/migration/codex-sync/2026-08-14-u17-release-closeout.md`
- `third_party/codex-cli/UPSTREAM.md`
- `docs/v0.0.5/codex-upstream-sync/overlay-inventory.json`
- `docs/v0.0.5/codex-upstream-sync/overlay-replay-ledger.json`
- `third_party/codex-cli/codex-rs/` 中与 Whale identity/home、DeepSeek、TaskSpace、测试适配和 release verification 直接相关的路径
- 提交区间：0.147 vendor cutover 至 `3d97fed892314bb125437399ea654a8cf95931ef`

### Allowed Change Categories

- 只允许写入本审查报告。
- 允许只读源码、文档、Git 历史和运行不产生外部状态的验证命令。

### Approval-required Changes

- 任何生产代码、测试、计划或产品文档修复
- 新 top-level module、外部依赖、public API、持久化/schema、跨模块抽象
- 当前 frozen target 之外的修改

### Authoritative Sources

| Authority | Source | What It Controls |
|---|---|---|
| E0 | 用户要求持续完成 Codex 主线融合、保留 DeepSeek/TaskSpace，并明确请求本次对抗性审查 | 目标、范围和审查授权 |
| E1 | `AGENTS.md`、`plan.md`、`decisions.md`、各 U 单元报告 | 产品合同、工程边界、完成标准和延期边界 |
| E2 | 当前生产代码、测试命令结果、Git commit/tree、生成物和失败日志 | 实际落地与运行行为 |
| E3 | 计划引用的 Codex 0.147 与 DeepSeek Responses 官方资料 | 外部版本/API 事实 |
| E4 | reviewer 与 main-agent 推理 | 仅作为待验证假设 |

### Baseline And Rollback

- Baseline revision: `3d97fed892314bb125437399ea654a8cf95931ef`
- Rollback checkpoint: 同一提交；本轮除审查报告外不修改目标 artifact
- Expected benefit: 以一轮 fresh independent review 检出自证闭环、错误完成声明和高影响遗漏
- Acceptable side effects: 新增一份 tracked review report；只读验证产生本地编译缓存可接受
- Automatic round budget: 2

## Round 1: 完成性与证据真实性审查

### Round Control

- Round type: initial
- Round number: 1
- Completed automatic rounds before launch: 0
- User approval for this round: `做对抗性审查`
- Closure finding IDs: n/a
- Permitted closure relation: n/a
- Target scope delta allowed: none

### Review Input

#### Objective

独立尝试证伪 Codex 0.147 融合 Phase A–E / U1–U17 已完成的声明。

#### Acceptance Criteria

- 对照计划逐项检查生产代码、集成入口、测试和文档证据。
- 识别 mock-only、test-only、schema-only、未接入生产路径或被错误延期的项目。
- 检查测试夹具适配是否降低了真实回归检测能力。
- 检查 U17 非全绿项归因是否足以支持 `verified-with-explicit-deferrals`。

#### Explicit Non-goals

- 不修改任何文件。
- 不要求实现 OpenAI hosted、Bedrock、Windows 或已批准延期能力。
- 不把风格偏好、泛化最佳实践或未来扩展性升级为 blocker。

#### Review Target

已实现的 0.147 vendor migration、DeepSeek/TaskSpace overlay、测试策略、发布收口结论及其同步元数据。

#### Target Locations

- `docs/v0.0.5/codex-upstream-sync/{plan.md,decisions.md,README.md}`
- `docs/migration/codex-sync/2026-08-14-u17-release-closeout.md`
- `third_party/codex-cli/UPSTREAM.md`
- `docs/v0.0.5/codex-upstream-sync/{overlay-inventory.json,overlay-replay-ledger.json}`
- `third_party/codex-cli/codex-rs/{core,model-provider-info,models-manager,state,ext/taskspace,app-server,app-server-protocol,tui,cli,exec,mcp-server}/`
- 相关提交与测试夹具变更

#### Baseline And Rollback Checkpoint

- Baseline: `3d97fed892314bb125437399ea654a8cf95931ef`
- Rollback checkpoint: `3d97fed892314bb125437399ea654a8cf95931ef`

#### Change Introduction

仓库将 vendor substrate 切换到 Codex CLI `rust-v0.147.0`，随后按 U5–U16 恢复 DeepSeek 与 TaskSpace，U17 适配测试夹具、完成 Linux 免费验证并将计划标记完成。完整 workspace 测试保留若干已归因失败和平台延期。

#### Risk Focus

- 完成声明是否超出实际验证证据。
- DeepSeek/TaskSpace 是否只在 mock/extension/schema 层存在而未进入生产路径。
- 将上游 OpenAI 模型假设归为产品差异时是否遗漏 Whale 自身通用能力回归。
- `WHALE_HOME`、provider 和 binary 测试适配是否掩盖实际配置优先级、resume 或 auth 问题。
- overlay/replay/provenance 是否与 HEAD 一致并可重复。

#### User-Perspective Review Focus

- “100% 完成”与“仍有延期/未全绿”是否容易误导开发者。
- 新开发者能否从唯一计划和 U17 报告理解已验证范围、恢复步骤与不能宣称的能力。

#### Implementation Completeness Focus

- U5–U10 DeepSeek provider/catalog/native Responses/accounting/compaction/final-wire。
- U11–U16 migration bridge/canonical kernel/store/CAS/extension lifecycle/RPC/events/TUI/viewer/final-wire。
- production registry、app-server、CLI/TUI 实际入口及生成 schema。
- 测试是否只证明本地 mock 而没有证明生产序列化/状态持久化路径。

#### Target Benefit Focus

- 声称的收益是可维护的 0.147 substrate、较薄 overlay、可追溯同步和 0 请求免费回归。
- 检查 180-path overlay 是否与“薄 overlay”措辞冲突；检查是否存在基线、比较方式和回归检测。

#### Evidence Sources And Gaps

- E1: 唯一计划、决策、U5–U17 报告和项目约束。
- E2: HEAD 源码、Git 历史、测试定义、生成物和可复现的只读命令。
- E3: 仅在外部 API/版本事实确有需要时使用计划中引用的一手资料。
- Known evidence gap: 没有 Windows、完整 TUI、真实 DeepSeek/cache live run 证据；报告声称这些已明确延期。

#### Assumptions To Attack

- 独立 mock test 足以代表最终生产请求路径。
- 所有非全绿失败都与当前 Whale 产品目标无关。
- TaskSpace extension seam 已覆盖旧 TaskSpace 所有当前必需语义。
- 生成 inventory 的 index/HEAD 边界与最终提交一致。
- 开发版本 lockfile `0.0.0` 与运行版本 `0.147.0` 不会造成发布证据歧义。

#### Adversarial Lenses

- implementation-completeness
- testing
- release
- state/data
- failure
- maintenance
- comprehension
- target-benefit

#### Verification Status

- 已记录 fmt、workspace all-target check、schema、CLI smoke、cache 7/7、exec 73/73、MCP 4/4、Skills 87/87。
- app-server 822 passed / 35 failed / 1 ignored；core lib 2148/30 failed；core integration 1095/40 failed/8 ignored。
- TUI/Windows/live cache 明确未验证。

#### Reviewer Instructions

- Fresh internal subagent session; no inherited main-agent context.
- Read target files directly; do not modify files.
- Cite evidence paths and line numbers when possible.
- Classify blocking and scope-expanding claims as E0-E4.
- For every blocker include broken assumption, trigger, impact, proof needed and scope effect.
- Treat benefit gaps as non-blocking unless they independently prove correctness, data, security, reliability or operational failure.

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
| complex | 20 minutes | one bounded 10-minute extension if alive | 2 | unavailable review cannot pass |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | 目标明确宣称整个计划已完成，最高风险是 mock/test/schema-only 被计为 production completion 或延期归因掩盖真实缺口 | plan-to-code completeness、production wiring、test evidence、release claim |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | internal `spawn_agent` | `/root/codex_0147_completion_review` | collaboration spawn event and final mailbox result | `fork_turns=none` | 本报告 Round 1 Review Input + neutral launch instruction | main-agent history、reasoning、drafts、conclusions、完整对话 | yes |

### Reviewer Timeout Records

| Reviewer | Initial Wait | Extension Used | Result | Elapsed |
|---|---:|---:|---|---:|
| implementation-completeness-adversary | 20 minutes | no | completed | about 9 minutes |

### Reviewer Outputs

#### IC-B01 — 合法的旧版空 TaskSpace map 无法读取（blocker）

- Broken assumption: U11 migration 能打开旧数据库并保留全部既有 TaskSpace 状态。
- Trigger: 旧版 TaskSpace 已激活、具有 map identity，但 `canonical_map=None`。旧 store 会把这个 `Option` 序列化为 `canonical_json="null"`、`map_revision=0`。
- Failure and impact: 当前 decoder 强制把 `canonical_json` 反序列化为非可选 `TaskSpaceMap`。迁移后的合法旧记录会在 restore/RPC 读取时失败，线程也无法正常重新初始化。
- Evidence:
  - E1: `plan.md:232` 要求旧数据库可打开且保留数据，`plan.md:234` 要求跨线程 rehydrate。
  - E2: 旧提交 `17424eac8` 的 `core/src/action_map/runtime/state.rs:71-97` 支持只有 identity、没有 canonical map 的状态；`core/src/session/taskspace_store.rs:209-227` 会持久化该状态；`state/src/runtime/taskspace_maps.rs:33-75` 序列化 `Option`，因此可写入 JSON `null`。
  - E2: 当前 `third_party/codex-cli/codex-rs/state/src/runtime/taskspace_maps.rs:214` 使用 `serde_json::from_str::<TaskSpaceMap>`；U11 migration test 只使用当前非空 `new_map`，未覆盖真实旧状态。
- Proof needed for closure: 用旧版实际可产生的 `canonical_json="null"` fixture 做 migration + restore/RPC 回归，证明数据可读且语义保持。
- Scope effect: U11/U13/U14 的兼容性修复与测试；不要求新 schema。

#### IC-B02 — 普通 `thread/fork` 静默丢失 TaskSpace（blocker）

- Broken assumption: TaskSpace extension lifecycle 已覆盖计划要求的 resume/fork/replay 语义。
- Trigger: 通过普通 app-server/TUI `thread/fork` 创建线程，而不是 `SessionSource::SubAgent` 子线程。
- Failure and impact: 当前 extension 只从 `SessionSource::SubAgent.parent_thread_id()` 继承，并把关系固定为 `Child`。普通 fork 的来源在线程 initial/fork history 中，但 `ThreadStartInput` 不携带该 lineage；生产代码中没有构造 `TaskSpaceMapRelation::Fork`。新线程因此没有 TaskSpace binding，会静默退回 Standard，丢失 world state 与工具语义。
- Evidence:
  - E1: 专题 README 的验收标准明确要求 TaskSpace `resume/fork/replay/terminal` 测试通过。
  - E2: `core/src/thread_manager.rs:1148` 为 fork 计算 `source_thread_id`；`core/src/extensions/thread_lifecycle.rs:17` 的 `ThreadStartInput` 不含 fork source/history。
  - E2: `ext/taskspace/src/extension.rs:42` 只查询 `session_source.parent_thread_id()`；`protocol/src/protocol.rs:2956` 只对 `SubAgent` 返回 parent；`ext/taskspace/src/runtime.rs:175,186` 绑定关系为 `Child`。
  - E2: 全仓生产构造只发现 `Owner`/`Child`；`Fork` 枚举存在但无生产构造路径。
- Proof needed for closure: 从真实 `thread/fork` RPC/manager 路径启动新线程，验证其继承父线程 map、持久化 `Fork` lineage，并在 reload 后保持。
- Scope effect: lifecycle/fork seam 和集成测试；不要求 schema 或 DeepSeek 改动。

#### IC-B03 — U17 非全绿失败没有逐项可审计证据（blocker）

- Broken assumption: app-server、core lib、core integration 的全部剩余失败都已经证明属于批准延期或上游环境差异。
- Trigger: 依据 U17 报告把当前融合标记为 `verified-with-explicit-deferrals`，但需要复核任一失败的具体归因。
- Failure and impact: 报告只有 aggregate 数量和非穷举的“主要是”描述，没有 exact HEAD 原始日志、完整失败清单、命令/环境及逐测试 E0/E1 映射。35 + 30 + 40 个失败中可能隐藏 Whale/DeepSeek/TaskSpace 回归，现有证据不能支持发布收口结论。
- Evidence:
  - E1: U17 报告第 16 行声明所有失败已归因；第 65/72 行只给 aggregate 和“主要是”的描述。
  - E2: 仓库没有对应 U17 HEAD 的 tracked raw test logs 或逐失败 manifest；现有 `rust-v0.147.0/evidence` 早于 U17 overlay 完成。
- Proof needed for closure: 在 exact HEAD 保存命令、环境、原始输出和完整失败 manifest；每个失败映射到已批准延期/已知上游约束，未映射失败须单独复跑和调查。
- Scope effect: 首先只补发布证据；仅当发现真实回归时才扩大到生产修复。

#### IC-M01 — README 同时呈现两套互相冲突的“当前状态”（non-blocking major）

- 顶部按 U17 表述当前 overlay 为 180 paths，后文旧分析却仍称当前 HEAD 为 `c539cbe...`、当前 overlay 为 11 paths。
- 影响: 新开发者无法可靠辨认当前树、历史基线和 rollback 依据。
- Required action: 把旧段落明确标成历史快照，或统一刷新到 U17 HEAD。
- Evidence class: E2。

#### IC-M02 — U16 final-wire test 绕过生产 composition（non-blocking major）

- 测试使用 `ToggleTaskSpaceStore` 和手工 `ExtensionRegistryBuilder`，只能证明 extension 能生成 body，不能证明 app-server RPC、真实 SQLite、thread lineage 与 production registry 的组合路径。
- IC-B02 正好证明该测试没有捕获实际 integration gap。
- Required action: 增加 process-level app-server + SQLite + typed RPC 的 Standard → TaskSpace final-wire 测试，并包含 fork/reload。
- Evidence class: E2。

#### Reviewer completeness and benefit assessment

- U1–U10 的 vendor/DeepSeek 生产落地总体可见。
- U11 对旧状态空间覆盖不完整；U13 store 已进生产但不兼容合法 `null` row；U14 registry、child/resume 有接入但普通 fork 缺失；U15 RPC 已接线但缺少 process-level TaskSpace RPC 证明；U16 TUI/viewer 已接线但 final-wire 测试为注入式 composition；U17 inventory 基本可追踪，但失败归因证据不足。
- vendor tree、ledger 和 0-request 免费回归的可追踪性收益已实现；overlay 从 730 降到 180 paths，但“薄 overlay”没有 production-only threshold，且 README 的 11-path 说法已陈旧。
- extension 降低 host intrusion 的收益只部分成立，因为 fork lineage seam 尚未闭合；发布可靠性结论被 IC-B03 阻断。
- 用户侧复现检查: 默认 DeepSeek/Flash 与 Standard gate 可见；普通 fork 和旧空 map 升级存在失败路径；README 当前状态易产生误解。

### Main Agent Response

主 Agent 已独立检查 reviewer 引用的当前源码、旧提交 `17424eac8`、计划验收标准、U17 报告和 U16 测试结构；没有仅凭 reviewer 结论接受问题。

| Finding | Triage | Severity | Evidence | Action |
|---|---|---|---|---|
| IC-B01 | accept | blocker | E1 + E2；旧代码确实可写入 JSON `null`，当前 reader 确实要求非可选 map，现有 migration fixture 为非空 map | 本轮只审查，生产/测试修复需用户授权 |
| IC-B02 | accept | blocker | E1 + E2；普通 fork source 未进入 `ThreadStartInput`，生产路径没有 `Fork` relation 构造 | 本轮只审查，lifecycle 与集成测试修复需用户授权 |
| IC-B03 | accept | blocker | E1 + E2；U17 的失败证据只有 aggregate，无法逐项审计“全部已归因” | 本轮只审查，补证据/必要复跑需用户授权 |
| IC-M01 | accept | major | E2；同一 README 的两个“当前”HEAD/overlay 数值冲突 | 与修复批次一起治理文档，需用户授权 |
| IC-M02 | accept | major | E2；测试手工注入 store/registry，未覆盖生产 composition，且未捕获 IC-B02 | 增加 process-level 测试，需用户授权 |

Reviewer 提出的必需修复、缺失测试和日志要求均已映射到上述 finding，没有制造重复问题项。无 finding 被 reject 或 defer；这里“未执行 action”是修改权限冻结，不是问题延期。

### Review Governor

#### Governance inputs

- Completed automatic rounds: 1 / 2。
- Unresolved blockers before Round 1: 0；after triage: 3。
- New blocker classes: legacy data compatibility、fork lifecycle integration、release evidence completeness。
- Scope expansion proposed: yes。关闭 findings 需要修改生产代码、测试和/或计划文档，超出本次只读审查允许范围。
- Authority status: E1/E2 足以确认问题存在，但 E0 只授权“做对抗性审查”，没有授权修复。
- Closure round eligibility: no。尚未修复 accepted blockers，立即开始 closure review 只会重复结论。

#### Governor decision

- Decision: `user-decision-required`。
- Reason: 当前不能宣称 Phase A–E / U1–U17 全部完成；至少 IC-B01、IC-B02、IC-B03 必须关闭。自动修复会违反 frozen change authority，故停止在重要决策点。
- No second reviewer launched: 本轮已确认三个不同 blocker class；在修复前追加 reviewer 不会形成 closure evidence。

### Convergence Reflection

- Converged on: vendor/DeepSeek 主干与大部分 TaskSpace extension 已真实落地；核心争议不是整体 scaffold，而是两个边界状态遗漏和发布证据不足。
- Failed to converge on: “计划已经全部完成”的结论。当前证据明确反驳该结论。
- Next smallest sufficient batch: 修复旧空 map 兼容；补齐普通 fork lineage；形成逐失败证据清单；同步修正 README，并用 production-composition 测试覆盖上述路径。
- Round budget: 保留第 2 轮用于修复后的定向 closure review，不在未修复状态浪费。

### User Decision Required

推荐授权一个单独的收口修复批次：

1. 关闭 IC-B01 与 IC-B02 的生产正确性问题及对应回归测试。
2. 关闭 IC-B03，生成 exact HEAD 的逐失败 manifest；只调查/修复未能映射到既有延期的失败。
3. 同步关闭 IC-M01，并用 IC-M02 要求的 production-composition 测试防止再次出现 lifecycle 假绿。
4. 修复后使用剩余一次自动 round 做定向 closure review。

较窄选项是先只修 IC-B01/IC-B02，但 U17 与整体计划状态必须保持 blocked；接受风险选项则必须撤回 Phase D/E/U17 的完成声明，不能继续标记 100% 完成。

### Closure Status

- Blocking findings found: 3
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Automatic round budget respected: yes
- Scope drift detected: no；修复所需 scope expansion 已被 governor 拦截
- Control outcome: user-decision-required
- Blocked reason: 3 个 accepted blockers 尚未修复，且修复不在本轮授权范围
- Allowed to proceed: no

## Final Conclusion

Round 1 已完成，但发布收口审查未通过。当前不能宣称 Codex 0.147 主线融合计划全部完成：旧版空 TaskSpace map 升级和普通 `thread/fork` 存在生产正确性缺口，U17 的非全绿归因也缺少逐失败审计证据。需要用户批准修复批次后再执行唯一一次定向 closure round。
