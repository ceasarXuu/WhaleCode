# WhaleCode Codex Upstream Substrate 审计与融合准则

---

## 一、结论

2026-04-27 更新：本文档不再表示“只参考 Codex 后重新实现”。新的主线见
`docs/plans/2026-04-27-codex-cli-upstream-substrate-migration-plan.md`。
WhaleCode 不应该在成熟 coding agent 基础设施上从 0 重新设计；后续核心
底座默认采用 **Codex CLI whole-repo upstream substrate** 策略：

1. 先整仓导入并 pin Codex CLI upstream，让实现细节留在仓库里。
2. 用 bridge/overlay 改造 Codex，而不是手写缩水版 permission、patch、exec、
   session、context、MCP/skills。
3. Codex 未覆盖或产品语义不足时，再看 Claude Code 语义、OpenCode、Pi。
4. WhaleCode 自己只拥有差异层：DeepSeek V4、Multi-Agent First、
   Create/Debug、Viewer、Swarm、Primitive Modules。

这不是一次性深 fork。Codex 是持续可同步的 upstream substrate；
WhaleCode 通过 `whalecode-codex-bridge` 和 overlay 保持产品差异。没有完成
Codex import/inventory/bridge 决策，不允许在权限、工具、补丁、会话、上下文、
MCP、日志、命令执行这些成熟方向继续从 0 自创方案。

---

## 二、上游与补充来源优先级

| 优先级 | 来源 | 目标位置 | 使用方式 |
|--------|------|----------|----------|
| P0 | Codex CLI | `third_party/codex-cli` | whole-repo upstream substrate；permission、sandbox、unified exec、apply patch、context compaction、session trace、MCP/skills 默认从这里融合 |
| P1 | Claude Code 语义 | `tmp/whalecode-refs/cc-from-scratch`、`tmp/whalecode-refs/claw-code` | 子 agent、plan mode、permission modes、skill 语义、用户体验 parity；不作为安全实现标准 |
| P2 | OpenCode | `tmp/whalecode-refs/opencode` | permission request UX、read-before-write、diff metadata、LSP diagnostics、DB-backed session service、pubsub |
| P3 | Pi | `tmp/whalecode-refs/pi` | JSONL session tree、event bus、web UI/runtime presentation、session branching、context stats |

Claude Code 官方实现不可直接审计时，公开复刻项目只能作为行为语义参考，不能替代生产安全设计。`claw-code` 当前根目录未发现标准 LICENSE，license 明确前禁止复制实现代码。

---

## 三、Substrate Adoption Gate

任何核心底座设计进入实现前，都必须在设计文档或 ADR 中补齐以下字段：

| 字段 | 要求 |
|------|------|
| `codex_upstream_source` | 列出 Codex upstream commit 和具体路径 |
| `adoption_mode` | `adopt_as_is` / `adapt_in_bridge` / `keep_disabled` / `reject_for_whale` / `needs_whale_redesign` |
| `borrowed_behavior` | 明确 WhaleCode 通过 Codex substrate 采用哪些行为、约束、测试语义 |
| `whalecode_delta` | 明确因为 DeepSeek / Multi-Agent / Create-Debug / Viewer 需要新增或改造什么 |
| `rejected_behavior` | 明确不照搬什么，以及原因 |
| `license_boundary` | 明确上游许可证、归属、NOTICE 和本地 patch 状态 |
| `sync_impact` | 说明未来 Codex 升级时该模块如何重放 patch 或调整 bridge |
| `acceptance_tests` | 把采用的成熟行为转成测试或 fixture，避免只写进文档 |

审计输出最小模板：

```yaml
codex_upstream_source:
  codex:
    commit: "<codex-sha>"
    paths:
      - third_party/codex-cli/codex-rs/core/src/exec_policy.rs
adoption_mode: adapt_in_bridge
borrowed_behavior:
  - deny rules outrank allow rules
  - approval policy decides whether a prompt may be shown
whalecode_delta:
  - add WorkflowPhase and AgentRole to permission decisions
rejected_behavior:
  - do not expose a blanket shell escape path in HYPOTHESIZE
license_boundary:
  - Apache-2.0 upstream substrate with attribution
sync_impact:
  - bridge owns Whale WorkflowPhase and AgentRole mapping
acceptance_tests:
  - permission deny precedence
  - ask disallowed when approval policy is never
```

---

## 四、八个成熟问题的补全方向

### 4.1 Permission / Sandbox / Command Policy

第一参考：

- `tmp/whalecode-refs/codex-cli/codex-rs/protocol/src/permissions.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/exec_policy.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/tools/handlers/unified_exec.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/linux-sandbox/src/lib.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/linux-sandbox/src/landlock.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/exec-server/src/fs_sandbox.rs`

补充参考：

- `tmp/whalecode-refs/opencode/internal/permission/permission.go`
- `tmp/whalecode-refs/cc-from-scratch/src/tools.ts`

采用：

- 文件系统和网络权限分开建模。
- deny 优先于 read/write allow。
- 特殊路径用结构化枚举表达，例如 root、cwd、project roots、tmpdir。
- read deny pattern 解析失败时 fail closed。
- approval policy 与 sandbox policy 分层，不能把“允许询问用户”当作“允许执行”。
- 命令风险规则独立于 shell 执行器，先判定再执行。
- 对 shell、python/node/ruby/perl、sudo、git、env、osascript 等动态命令前缀保持保守。

WhaleCode 改造：

- `PermissionDecision` 必须加入 `WorkflowPhase`、`AgentRole`、`WorkUnitId`、`PatchOwnership`。
- Debug `HYPOTHESIZE` 阶段即使用户配置宽松，也只能开启只读 deterministic 工具。
- Multi-agent 下 session grant 不能默认扩散给所有 agent，必须带 `agent_scope`。
- Viewer 必须能审查 permission escalation，并可阻止高风险 phase transition。

不照搬：

- 不把 Codex 的 OpenAI/Codex 专属 approval UX 作为产品边界。
- 不把 Claude-like 复刻项目的 `bypassPermissions` 作为生产默认能力。

验收测试：

- deny 优先级。
- approval policy 不允许询问时必须直接 reject。
- HYPOTHESIZE 阶段 shell write command 被拒绝。
- session grant 只对指定 agent / task 生效。

### 4.2 Patch / File Edit / Workspace Safety

第一参考：

- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/apply_patch.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/apply-patch/src/parser.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/apply-patch/src/invocation.rs`

补充参考：

- `tmp/whalecode-refs/opencode/internal/llm/tools/write.go`
- `tmp/whalecode-refs/opencode/internal/llm/tools/edit.go`

采用：

- patch 解析、验证、应用分层。
- 先判断 patch 是否只触及可写路径，再决定是否自动允许。
- read-before-write 和 mtime/version 检查。
- diff metadata 必须作为 permission request 和 session event 的一部分。
- edit 操作必须能区分 create、replace、delete。
- 写后运行可配置 diagnostics，例如 LSP 或测试命令。

WhaleCode 改造：

- 多 agent 不直接写共享工作区。并行 Implementer 产出 `PatchArtifact`，Supervisor 单线程应用。
- `PatchArtifact` 必须带 `baseCommit`、`touchedFiles`、`ownership`、`testsRun`、`riskSummary`。
- Patch League 可以比较多个候选，但最终 apply gate 只有一个。

不照搬：

- 不把任意 shell heredoc patch 当作安全 apply。
- 不在冲突时让 LLM 猜测合并用户改动。

验收测试：

- 未读文件直接写入失败。
- mtime 变化后 edit 失败。
- patch 越权路径失败。
- ownership 重叠的两个 patch 不能同时应用。

### 4.3 Session / Event Log / Replay

第一参考：

- `tmp/whalecode-refs/codex-cli/codex-rs/rollout-trace/src/raw_event.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/state/`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/thread_manager.rs`

补充参考：

- `tmp/whalecode-refs/pi/packages/coding-agent/docs/session.md`
- `tmp/whalecode-refs/pi/packages/coding-agent/src/core/agent-session.ts`
- `tmp/whalecode-refs/opencode/internal/session/session.go`

采用：

- append-only、schema-versioned、monotonic seq 的事件日志。
- 事件必须带 session/thread/turn/tool/agent 关联 ID。
- raw event 与 reducer 分离，replay 不能依赖 UI 状态。
- JSONL first，SQLite later 只做索引和查询加速。
- session parent/branch 关系必须可表达。

WhaleCode 改造：

- event schema 增加 `WorkUnit`、`Cohort`、`ConsensusReport`、`PatchArtifact`、`ViewerConcern`。
- Web Viewer 只能消费已经写入 SessionStore 的事件，不能展示无法 replay 的瞬时状态。
- 所有导出都必须经过 redaction。

不照搬：

- Phase 1 不直接做 DB-first session。
- 不让 Web UI 成为状态权威。

验收测试：

- JSONL replay 得到相同 DAG 状态。
- event seq 单调。
- compaction、permission、patch apply 都能 replay。
- parent session branch 可追踪。

### 4.4 Context / Compaction / Repo Knowledge

第一参考：

- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/compact.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/context_manager/history.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/context/fragment.rs`

补充参考：

- `tmp/whalecode-refs/pi/packages/coding-agent/src/core/agent-session.ts`
- `tmp/whalecode-refs/cc-from-scratch/src/agent.ts`

采用：

- compaction 是独立模型轮次，不嵌在普通回答中。
- history replacement 必须保留用户意图、关键 tool result 和压缩摘要。
- context fragment 使用 typed marker，而不是散落字符串拼接。
- context overflow 时允许 emergency remove-oldest，并记录 warning。
- compaction trace 和 token stats 必须可观测。

WhaleCode 改造：

- 1M context 不等于无限堆历史。Supervisor 负责构造 Shared Task Pack，agent 只拿任务所需切片。
- 多 agent 下每个 agent 有独立 context budget；共享前缀优先稳定，以提高 DeepSeek cache hit。
- RepoMap、EvidenceIndex、PatchArtifact 用引用注入，不把大对象反复复制进 prompt。

不照搬：

- 不把 Claude-like 复刻项目的简单 snip/microcompact 作为唯一机制。
- 不让 compaction 丢失 artifact ID 和 evidence 引用。

验收测试：

- compaction 后 artifact reference 仍可解析。
- 低上下文任务不触发压缩。
- token overflow 有确定性降级路径。
- 多 agent 不共享可变 message history。

### 4.5 Tool Runtime / Parallel Execution

第一参考：

- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/tools/handlers/unified_exec.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/tools/context.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/tools/handlers/mod.rs`

补充参考：

- `tmp/whalecode-refs/pi/packages/coding-agent/src/core/event-bus.ts`
- `tmp/whalecode-refs/opencode/internal/pubsub/broker.go`

采用：

- tool handler 不直接绕过 permission/context/sandbox。
- tool 参数必须 schema validate。
- 输出必须可截断、可结构化记录、可 replay。
- read-only tools 可并行，write tools 受 lock/ownership 限制。
- long-running exec 必须有 session id、stdin channel、yield timeout、max output。

WhaleCode 改造：

- `ToolRuntime` 的并发权由 `ConcurrencyGovernor`、`PermissionEngine`、`WorkflowPhase` 同时约束。
- Multi-agent write lock 不以 agent 数量为准，而以 workspace ownership 为准。
- DeepSeek parallel tool calls 只能进入 ToolRuntime 调度队列，不能直接并发写盘。

不照搬：

- 不让模型直接选择任意 shell。
- 不把 tool result 原文无限写进 history。

验收测试：

- read tools 可并行。
- write tools 同文件互斥。
- long exec 可继续写 stdin 并可取消。
- tool output 超限时有 head/tail/truncation metadata。

### 4.6 MCP / Skills / Tool Exposure

第一参考：

- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/mcp_tool_exposure.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/codex-mcp/src/mcp_tool_names.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/skills.rs`

补充参考：

- `tmp/whalecode-refs/cc-from-scratch/src/subagent.ts`
- `tmp/whalecode-refs/cc-from-scratch/src/tools.ts`

采用：

- MCP tool 名称必须 sanitize、长度限制、冲突处理。
- 工具太多时支持 deferred exposure，避免 prompt 被工具 schema 淹没。
- connector/app 类工具可以有独立曝光策略。
- skill dependency 中的 secret 只进 session memory，不写明文配置。

WhaleCode 改造：

- MCP/Skills 暴露前必须经过 `PermissionEngine` 和 `WorkflowPhase` 过滤。
- Evolution Agent 只能提交 skill proposal，不能直接自改生产 skill。
- AgentRole 可决定默认可见工具集。

不照搬：

- 不让 project skill 绕过 repo trust 和 permission。
- 不在 Phase 1 做过度复杂的 marketplace。

验收测试：

- MCP name collision 有稳定 hash 后缀。
- 超过阈值的 MCP tools 延迟暴露。
- secret dependency 不进入 exportable log。
- phase 禁止的 skill tool 不可见。

### 4.7 Observability / Logging / Privacy

第一参考：

- `tmp/whalecode-refs/codex-cli/codex-rs/rollout-trace/src/raw_event.rs`
- `tmp/whalecode-refs/codex-cli/codex-rs/core/src/compact.rs`

补充参考：

- `tmp/whalecode-refs/pi/packages/coding-agent/src/core/agent-session.ts`
- `tmp/whalecode-refs/opencode/internal/pubsub/events.go`

采用：

- 结构化事件，而不是散落文本日志。
- 每个事件带 schema version、wall time、trace/session/turn ids。
- compaction、tool、permission、model request 都必须有 trace。
- pubsub/live viewer 只是 event store 的消费端。

WhaleCode 改造：

- Redaction 是事件写入前置步骤。
- Multi-agent 统计必须包含 effective agent count、diversity score、cache hit、wasted fan-out、verification pass rate。
- Viewer concern 也是一等事件。

不照搬：

- 不上传用户代码或完整 tool args 到远端遥测。
- 不把本地 debug log 当作 exportable report。

验收测试：

- secret/env/path-sensitive 字段默认 redacted。
- Viewer live event 与 JSONL replay 渲染一致。
- traceId 可贯穿 model -> tool -> patch -> verification。

### 4.8 Workflow / Create-Debug / Multi-Agent Differentiation

第一参考：

- Codex 的 tool/session/context/permission 层作为底座。
- OpenCode/Pi/Claude-like 只补充交互和 session 表达。

WhaleCode 自研原因：

- 目前参考产品没有把 Create 和 Debug 作为强 phase machine 原语。
- 也没有把大量 Flash agent 的 cohort/tournament/evidence-race 作为第一层调度模型。
- WhaleCode 的差异层必须建立在成熟底座之上，而不是替代底座。

采用底座：

- permission、session、tool、patch、context 不自创。
- phase machine 只编排这些成熟能力。

WhaleCode 改造：

- `CreatePhaseMachine`：Clarify -> Design Tournament -> Scaffold -> Patch League -> Review -> Verify -> Confirm。
- `DebugPhaseMachine`：Reproduce -> Hypothesize -> Evidence Race -> Root Cause Judge -> Fix Candidates -> Verify -> Regression Guard。
- `Supervisor` 是唯一可以跨 phase 改权限、改并发、应用 patch 的角色。

验收测试：

- Create scaffold 阶段必须先建 logging/testing/constraints。
- Debug HYPOTHESIZE 必须只读。
- Viewer critical concern 阻止 phase transition。
- tournament 失败时输出可解释 stuck report。

---

## 五、工程落地顺序调整

### Phase 0.5 — Codex Substrate Baseline

在继续写 Whale runtime 之前补一个轻量阶段：

交付：

- 本文档作为 Codex upstream substrate 融合准则。
- `docs/plans/2026-04-27-codex-cli-upstream-substrate-migration-plan.md` 作为执行主计划。
- `docs/plans/2026-04-24-system-architecture.md` 挂载 Substrate Adoption Gate。
- `docs/adr/2026-04-27-codex-cli-upstream-substrate.md` 记录整仓导入决策。

验收：

- 每个成熟基础设施模块都有 Codex upstream 路径和 adoption mode。
- 每个 Codex 不覆盖的问题都有明确二级参考。
- 每个自研差异点都能解释为什么不是成熟产品已有能力。

### Phase 1 — Codex-backed Single Agent Core

先从 Codex substrate 做单 agent 纵切，Whale 只实现 bridge 和 DeepSeek/provider 差异：

1. `third_party/codex-cli`: pinned upstream source.
2. `whalecode-codex-bridge`: permission、exec、patch、session、context、MCP/skills adapter.
3. `whalecode-provider-deepseek`: DeepSeek streaming/tool/thinking adapter.
4. `whalecode-protocol`: Whale event、artifact、primitive、workunit schema.

### Phase 2 — WhaleCode Differentiation

等成熟底座跑通后再做：

- Create/Debug phase machine。
- Swarm runtime。
- Cohort scheduler。
- Tournament / Evidence Race / Patch League。
- Viewer adversarial gate。
- Skill evolution proposal loop。

这样可以避免多 agent 架构建立在脆弱的 permission、patch、session、context 基础上。

---

## 六、不照搬清单

| 来源 | 不照搬 | 原因 |
|------|--------|------|
| Codex CLI | 产品边界、OpenAI 专属 API、ChatGPT 登录、云端任务默认路径 | WhaleCode 是 DeepSeek-first、Multi-Agent-first |
| Codex CLI | 无记录地修改 vendor 目录 | 会破坏未来 upstream sync；必须走 bridge 或 patch queue |
| Claude-like 复刻 | toy permission / shell bypass / 简化安全模型 | 适合理解体验，不适合生产安全 |
| OpenCode | DB-first session 作为 Phase 1 默认 | JSONL 更利于 early debug/replay |
| Pi | TypeScript core | Rust-first 已确定，Pi 只参考 event/session/viewer |
| Claw Code | 实现代码 | license 未明确前只做 parity/audit 参考 |

---

## 七、后续设计文档必须新增的字段

后续每份模块级设计文档统一包含：

```markdown
## Substrate Adoption

| 项 | 内容 |
|----|------|
| Codex upstream commit | ... |
| Codex paths | ... |
| Adoption mode | ... |
| Secondary references | ... |
| Adopted behavior | ... |
| WhaleCode delta | ... |
| Rejected behavior | ... |
| Sync impact | ... |
| Tests | ... |
```

如果某模块无法找到 Codex 对应实现，必须写明：

1. 为什么 Codex 不覆盖。
2. 为什么 Claude/OpenCode/Pi 的实现更合适。
3. WhaleCode 自研部分的最小边界。

---

## 八、外部来源

本设计以本地快照为准，外部链接仅用于项目定位、许可证入口和后续更新追踪：

1. Codex CLI: https://github.com/openai/codex
2. OpenCode: https://github.com/opencode-ai/opencode
3. Pi mono repo: https://github.com/badlogic/pi-mono
4. Claude Code from Scratch: https://github.com/Windy3f3f3f3f/claude-code-from-scratch
5. Model Context Protocol: https://modelcontextprotocol.io/
