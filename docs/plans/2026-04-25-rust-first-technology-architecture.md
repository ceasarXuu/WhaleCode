# WhaleCode Rust-first 技术架构规划

---

## 一、结论

WhaleCode 的主技术栈应从早期的 TypeScript / Node / Bun 调整为 **Rust-first core + TypeScript web viewer**。

多 Agent 群体协同的运行时设计见 `docs/plans/2026-04-25-multi-agent-collaboration-architecture.md`。本文的 Rust workspace 和 Phase 2 规划以该文档为准扩展 `whalecode-swarm`。

核心判断：

- WhaleCode 的长期难点是本地执行内核，而不是普通 API wrapper。
- CLI、TUI、Agent Supervisor、工具调度、权限、沙箱、补丁合并、会话恢复和并发控制应放在 Rust 核心中。
- Web Viewer、可视化面板、部分插件开发体验可以继续用 TypeScript / React / Vite。
- Skills、Tools、MCP 要通过协议边界隔离语言，而不是被单一语言生态绑定。
- Rust core 的成熟基础设施模块必须先通过 `docs/plans/2026-04-25-codex-first-reference-audit.md` 定义的 Codex-first Reference Audit Gate，不能在 permission、sandbox、exec、patch、session、context、MCP/skills、observability 上从 0 自创方案。

最终推荐：

| 层级 | 推荐选型 |
|------|----------|
| Core runtime | Rust stable |
| Async runtime | Tokio |
| CLI | clap |
| TUI | ratatui + crossterm |
| Serialization | serde / serde_json / serde_yaml |
| HTTP / SSE | reqwest + stream parser |
| Event / logging | tracing + JSONL event sink |
| Session store | JSONL first, SQLite index later |
| Patch / diff | Rust PatchArtifact + unified diff parser |
| MCP | Phase 1 stdio JSON-RPC adapter, Phase 2 评估官方 Rust SDK |
| Web Viewer | TypeScript + React + Vite |
| Web bridge | Rust SSE / WebSocket read-only event server |
| Release | GitHub Releases + platform binaries; later evaluate installer tooling |

---

## 二、决策驱动

### 2.1 WhaleCode 的真实问题域

WhaleCode 不是普通的聊天 CLI。它的核心问题是：

1. 多 Agent 生命周期调度。
2. 工具调用的并发安全和权限边界。
3. 文件系统写入、补丁生成、补丁合并和冲突恢复。
4. shell / git / test / build 命令的受控执行。
5. 长时间会话的事件日志、replay、fork 和恢复。
6. DeepSeek thinking + tool-call streaming 的结构化处理。
7. TUI 和 Web Viewer 对同一事件流的实时消费。
8. 开源分发时的一致安装体验和最小运行时依赖。

这些约束更接近 Codex CLI 的系统工具问题域，而不是 Web 应用或脚本工具问题域。

### 2.2 选型原则

| 原则 | 含义 |
|------|------|
| Runtime safety first | 本地 agent 会执行命令和写文件，核心必须尽量减少状态和并发错误 |
| Deterministic orchestration | Supervisor、Phase Machine、Permission、Patch Apply 必须可测试、可 replay |
| Protocol over embedding | MCP、Skills、Viewer 都通过协议/事件边界接入，不把核心绑死到某语言 |
| Single binary UX | 终端工具应优先给用户一个二进制，而不是要求 Node/Bun 运行时 |
| Observable by default | 所有核心动作必须结构化记录，方便 Debug 和自进化 |
| Web only where web wins | 可视化、图谱、交互面板用 TypeScript；本地执行核心不用 TypeScript |
| Reference implementation first | 成熟 coding-agent 基础设施先审计 Codex CLI，不足处再用 Claude/OpenCode/Pi 补充 |

---

## 三、候选方案评估

### 3.1 纯 TypeScript / Node / Bun

优点：

- 开发速度快。
- LLM SDK、MCP SDK、Web UI 生态好。
- Pi 和 cc-from-scratch 的参考代码更容易迁移。
- Bun 可生成单文件可执行文件。

问题：

- 本地执行权限、shell、PTY、进程信号、文件锁、补丁应用和并发调度会越来越重。
- Node permission model 不能作为恶意代码或任意 shell 的安全边界。
- Deno 的权限模型更好，但 coding agent 一旦需要 `allow-run`，子进程权限仍然会成为核心风险点。
- Bun 单文件分发体验好，但作为长期本地 agent 内核仍要承担运行时兼容性和生态差异。
- TypeScript 类型不能像 Rust ownership 那样约束并发状态、资源生命周期和工具锁。

适合范围：

- Web Viewer。
- Plugin SDK。
- MCP server examples。
- 非核心实验和快速原型。

结论：不适合作为 WhaleCode 长期核心运行时。

### 3.2 纯 Go

优点：

- 单二进制分发体验好。
- goroutine/channel 很适合 Agent bus 和 tool execution。
- OpenCode 已证明 Go 可以承载成熟 coding agent。
- Bubble Tea / Lipgloss TUI 生态成熟。

问题：

- 类型系统对复杂状态机、权限组合和 artifact contract 的约束弱于 Rust。
- 资源生命周期和并发共享数据更依赖工程纪律。
- 对 WhaleCode 这种要长期扩展多 Agent DAG、权限、replay、patch contract 的系统，Rust 更利于把错误提前到编译期。

适合范围：

- 如果团队 Rust 能力不足，Go 是务实备选。
- 也可参考 OpenCode 的 session service、permission request、file edit safety。

结论：第二选择，不是首选。

### 3.3 纯 Rust

优点：

- 单二进制分发自然。
- Tokio 适合 streaming、subprocess、timeout、Message Bus 和长连接。
- ownership/type system 更适合表达 tool lock、permission decision、phase gate、patch ownership。
- ratatui/clap/serde/tracing/sqlx 等生态足够覆盖 CLI agent 核心。
- Codex CLI 的 Rust 架构可作为高质量参照。

问题：

- Web UI、插件生态和 LLM SDK 迁移速度不如 TypeScript。
- Rust 开发门槛更高，Phase 1 速度会慢于 TS。
- DeepSeek/OpenAI-compatible 的 Rust SDK 不一定覆盖 DeepSeek thinking 的细节，可能需要手写 adapter。

结论：适合作为核心，但 Web Viewer 和插件体验不应强行 Rust 化。

### 3.4 Rust core + TypeScript web/plugin

优点：

- 核心执行安全和并发模型交给 Rust。
- Web 可视化继续使用 TypeScript / React 生态。
- MCP/Skills 通过 stdio、JSON-RPC、SSE/WebSocket 接入，语言边界清晰。
- 长期可演进为多进程架构：Rust core 是权威 runtime，TS 是外围体验层。

问题：

- 需要维护跨语言事件 schema。
- Web Viewer 需要从一开始只读消费事件，不能反向写核心状态。
- 插件 SDK 需要版本化协议，避免运行时耦合。

结论：首选。

---

## 四、总体架构

```text
whalecode
  │
  ├─ Rust Core Binary
  │   ├─ CLI / TUI
  │   ├─ Supervisor
  │   ├─ Agent Runtime
  │   ├─ Swarm Runtime
  │   ├─ Cohort Scheduler
  │   ├─ Concurrency Governor
  │   ├─ Message Bus
  │   ├─ Workflow Phase Machine
  │   ├─ Tool Runtime
  │   ├─ Permission Engine
  │   ├─ Patch Engine
  │   ├─ Session Store
  │   ├─ Context Manager
  │   ├─ DeepSeek Adapter
  │   ├─ MCP Host
  │   └─ Observability Event Sink
  │
  ├─ Web Viewer
  │   ├─ React + Vite
  │   ├─ Agent graph
  │   ├─ DAG progress
  │   ├─ Evidence chain
  │   └─ Token/tool/cache metrics
  │
  └─ Plugin / Skill Processes
      ├─ MCP stdio servers
      ├─ TypeScript plugin host
      ├─ Python tools
      └─ Project-local skills
```

核心规则：

- Rust core 是唯一状态权威。
- Web Viewer 默认只读，不直接修改 session state。
- Skills 和 MCP 不能绕过 Permission Engine。
- 所有跨边界数据必须是版本化 JSON schema。
- 所有核心状态变化必须写入 SessionStore。

---

## 五、Workspace 结构

推荐初始结构：

```text
crates/
  whalecode-protocol/     # Shared event/message/tool/session/swarm schema
  whalecode-core/         # Supervisor, AgentRuntime, SwarmRuntime, MessageBus
  whalecode-model/        # Model routes, DeepSeek adapter, capability probe
  whalecode-context/      # ContextManager, compaction, fragments
  whalecode-tools/        # Built-in read/search/edit/write/git/shell tools
  whalecode-permission/   # Permission profiles, grants, ask/deny decisions
  whalecode-patch/        # PatchArtifact, diff, ownership, apply engine
  whalecode-session/      # JSONL store, replay, fork, SQLite index later
  whalecode-workflow/     # Create/Debug phase machines and gates
  whalecode-swarm/        # CohortScheduler, WorkUnit, Tournament, EvidenceRace
  whalecode-mcp/          # stdio JSON-RPC client, MCP tool mapping
  whalecode-observe/      # tracing bridge, redaction, event sinks
  whalecode-cli/          # clap commands and non-interactive mode
  whalecode-tui/          # ratatui interactive UI

apps/
  viewer/                 # React + Vite Web Viewer

docs/
  plans/
  adr/

fixtures/
  repos/                  # e2e fixture repositories

tests/
  e2e/                    # black-box CLI tests
```

包边界原则：

- `whalecode-protocol` 不依赖其他业务 crate，避免循环依赖。
- `whalecode-core` 只依赖抽象 trait，不直接依赖具体工具实现。
- `whalecode-swarm` 不执行工具、不写文件，只负责 cohort、work unit、DiversityPolicy、budget、concurrency 和 evidence-weighted consensus 编排。
- `whalecode-tools` 不决定权限，只声明工具 metadata 和执行能力。
- `whalecode-permission` 不执行工具，只返回 allow/deny/ask。
- `whalecode-session` 不理解业务逻辑，只 append/replay event。
- `whalecode-tui` 和 `apps/viewer` 都消费同一事件模型。

---

## 六、核心接口形态

### 6.1 Agent Runtime

```rust
pub struct AgentRuntime {
    pub id: AgentId,
    pub role: AgentRole,
    pub state: AgentState,
}

#[async_trait::async_trait]
pub trait AgentLoop {
    async fn start(&mut self, task: TaskAssignment) -> Result<TaskResult, AgentError>;
    async fn interrupt(&mut self, reason: InterruptReason) -> Result<(), AgentError>;
    async fn close(&mut self) -> Result<(), AgentError>;
}
```

约束：

- AgentLoop 只负责 LLM/tool loop。
- 不在 AgentLoop 内做 phase transition。
- 每个 assistant message 结束后解析 tool calls。
- DeepSeek thinking + tool-call sub-turn 必须保留当轮 `reasoning_content`。
- 新用户输入通过 steering/follow-up queue 进入，不直接改写历史。

### 6.2 Message Bus

```rust
pub struct BusEnvelope<T> {
    pub id: EventId,
    pub seq: u64,
    pub trace_id: TraceId,
    pub causality: Vec<EventId>,
    pub from: ActorId,
    pub to: BusTarget,
    pub payload: T,
    pub created_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait MessageBus {
    async fn publish(&self, envelope: DraftEnvelope) -> Result<BusEnvelope<EventPayload>, BusError>;
    async fn subscribe(&self, filter: BusFilter) -> Result<BusSubscription, BusError>;
    async fn drain(&self, agent_id: AgentId) -> Result<Vec<BusEnvelope<EventPayload>>, BusError>;
}
```

MVP 采用单进程内存 bus：

- `seq` 单调递增。
- publish 成功后同步 append 到 SessionStore。
- subscriber 异常不得击穿 bus。
- Phase 2 可替换为 SQLite/Redis-backed bus，但 trait 不变。

### 6.3 Tool Runtime

```rust
pub enum ToolExecutionMode {
    ParallelSafe,
    Sequential,
    ExclusiveWrite,
}

pub struct ToolSpec {
    pub name: ToolName,
    pub description: String,
    pub input_schema: JsonSchema,
    pub permissions: Vec<ToolPermission>,
    pub execution_mode: ToolExecutionMode,
    pub output_policy: TruncationPolicy,
}

#[async_trait::async_trait]
pub trait ToolRuntime {
    async fn list(&self, role: AgentRole, phase: WorkflowPhase) -> Vec<ToolSpec>;
    async fn execute(&self, call: ToolCall, ctx: ToolContext) -> Result<ToolResult, ToolError>;
}
```

并发规则：

- `ParallelSafe`：read/glob/grep 等只读工具可并发。
- `Sequential`：shell、git、test 等按序执行。
- `ExclusiveWrite`：edit/write/apply_patch 获得全局 write lock。
- 同一 batch 里只要出现写工具，整个 batch 按写锁串行化。
- shell 默认不是并行安全工具，除非被 `ReadonlyCommandPolicy` 证明。

### 6.4 Permission Engine

```rust
pub enum PermissionDecision {
    Allow { source: PermissionSource },
    Deny { reason: String },
    Ask { prompt: PermissionPrompt },
}

#[async_trait::async_trait]
pub trait PermissionEngine {
    async fn decide(
        &self,
        request: PermissionRequest,
        ctx: PermissionContext,
    ) -> Result<PermissionDecision, PermissionError>;

    async fn grant(&self, request: PermissionRequest, scope: GrantScope) -> Result<(), PermissionError>;
    async fn deny(&self, request: PermissionRequest, reason: String) -> Result<(), PermissionError>;
}
```

优先级：

1. deny profile
2. workflow phase permission
3. role permission
4. file ownership claim
5. session/project grant
6. hook decision
7. user ask

### 6.5 Session Store

```rust
#[async_trait::async_trait]
pub trait SessionStore {
    async fn append(&self, entry: SessionEntry) -> Result<(), SessionError>;
    async fn read(&self, session_id: SessionId) -> Result<SessionStream, SessionError>;
    async fn replay(&self, session_id: SessionId) -> Result<ReplaySnapshot, SessionError>;
    async fn fork(&self, session_id: SessionId, from_seq: u64) -> Result<SessionId, SessionError>;
}
```

MVP 存储格式：

```text
~/.whalecode/sessions/<session-id>.jsonl
```

要求：

- 第一行必须是 `session_started`。
- 每行是一个带 schema version 的 JSON object。
- tool args、env、path、secret 做 redaction 后再写盘。
- replay 必须能重建消息、工具调用、phase transition、patch artifact、verification result。

---

## 七、DeepSeek Adapter

DeepSeek adapter 不依赖 OpenAI SDK 作为核心边界，而是用 Rust 直接实现 OpenAI-compatible HTTP/SSE 适配层。

原因：

- DeepSeek thinking + tool call 对 `reasoning_content` 有特殊历史处理要求。
- V4 模型名、上下文、最大输出、Flash/Pro 路由和价格都必须由 `ModelCapabilityProbe` 实测或配置确认。
- SDK 抽象可能无法及时覆盖 DeepSeek 当前 API 差异。

核心模块：

```text
whalecode-model/
  capability.rs      # ModelCapabilityProbe
  deepseek.rs        # DeepSeek HTTP/SSE adapter
  route.rs           # Flash/Pro/Reasoner route policy
  stream.rs          # LlmEvent normalization
  usage.rs           # token/cache/pricing metrics
```

硬约束：

- 默认 base URL 是 `https://api.deepseek.com`，允许 project/user config 覆盖。
- thinking 通过 DeepSeek 当前 API 支持的 `thinking` 参数表达。
- tool-use sub-turn 内必须把当前 `reasoning_content` 回传给 API。
- 新用户 turn 开始前必须清理旧 turn 的 reasoning 内容。
- `parallel_tool_calls` 不作为 provider 必选参数；并发由 ToolRuntime 控制。
- context/output/pricing/cache 全部带 `observed_at` 和 `pricing_source`。

---

## 八、权限与沙箱

WhaleCode 的安全边界不应依赖语言运行时权限模型。

安全模型分三层：

1. **Deterministic permission**：Phase、Role、FileOwnership、Grant、Hook。
2. **Command policy**：readonly allowlist、dangerous pattern denylist、timeout、cwd、env redaction。
3. **OS sandbox**：按平台逐步接入 macOS sandbox-exec / Linux namespace / Windows restricted token。

MVP 最小要求：

- `HYPOTHESIZE` 阶段没有写权限，没有任意 shell。
- `git_read` 只能执行 allowlisted read-only git 命令。
- write/edit/apply_patch 必须 read-before-write。
- 写入必须产出 diff metadata。
- shell tool 默认需要 permission decision。
- 所有命令必须有 timeout、output limit、redaction。

---

## 九、Patch 与 Workspace 策略

多 Agent 写入不能直接并发写共享工作区。

推荐策略：

```text
Implementer
  -> PatchArtifact
  -> Supervisor ownership check
  -> PatchEngine dry-run
  -> PermissionEngine
  -> apply to workspace
  -> run verification
  -> SessionStore append result
```

核心数据：

```rust
pub struct PatchArtifact {
    pub id: PatchId,
    pub base_commit: String,
    pub author_agent: AgentId,
    pub files: Vec<PatchFile>,
    pub declared_ownership: Vec<FileOwnershipClaim>,
    pub required_checks: Vec<VerificationCommand>,
}
```

合并门禁：

- base commit 必须匹配或可重放。
- file ownership 不得冲突。
- dry-run apply 必须成功。
- 写入后必须记录 diff metadata。
- 相关测试必须通过或明确记录 test gap。
- Viewer critical concern 可阻止 phase transition。

---

## 十、MCP 与 Skills

### 10.1 MCP

Phase 1：

- 实现 stdio JSON-RPC MCP client。
- 支持 initialize、list_tools、call_tool。
- MCP tool 暴露前必须映射到 WhaleCode ToolSpec。
- MCP tool 同样进入 PermissionEngine。
- MCP output 同样进入 truncation/redaction。

Phase 2：

- 评估官方 Rust SDK 的成熟度和 spec 覆盖。
- 增加 Streamable HTTP transport。
- 增加 MCP server health、timeout、capability cache。

### 10.2 Skills

Skills 是 prompt + metadata + optional tools，不是任意代码的快捷入口。

MVP：

- 支持 project/user/global 三层 skill discovery。
- skill frontmatter 必须包含 name、description、version。
- skill 只能影响 prompt/tool exposure，不能绕过权限。

后续：

- Skill usage events 进入 SessionStore。
- Evolution Agent 读取匿名/本地指标生成改进 proposal。
- Viewer 审查 evolution proposal。

---

## 十一、TUI 与 Web Viewer

### 11.1 TUI

Rust TUI 用于主交互：

- REPL 输入。
- streaming answer。
- tool call progress。
- patch diff preview。
- permission prompt。
- phase/DAG compact status。

技术：

- ratatui 负责布局和绘制。
- crossterm 负责 terminal event。
- tui 只消费 core event，不直接改 core state。

### 11.2 Web Viewer

Web Viewer 用于可视化和审计：

- Agent network graph。
- Create/Debug DAG progress。
- Evidence chain。
- Viewer concerns。
- token/tool/cache metrics。
- session replay。

技术：

- React + Vite。
- Rust core 提供只读 SSE/WebSocket。
- Web 端不持久化权威状态。
- 所有交互型命令必须回到 CLI/TUI permission flow，不能从浏览器直接执行 shell。

---

## 十二、测试策略

### 12.1 Rust core

| 类型 | 覆盖 |
|------|------|
| Unit | phase transition、permission priority、tool metadata、patch parser、redaction |
| Integration | mock DeepSeek SSE、tool runtime、JSONL replay、MCP stdio fixture |
| Golden | session replay snapshot、diff metadata、tool truncation output |
| Concurrency | read parallel/write exclusive、bus seq monotonic、timeout cancellation |
| E2E | fixture repo 中执行 read/search/edit/verify 流程 |

基础命令：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### 12.2 Web Viewer

| 类型 | 覆盖 |
|------|------|
| Unit | event reducer、graph layout input normalization |
| Component | DAG progress、concern list、tool timeline |
| E2E | 从 fixture JSONL replay 渲染 Viewer |

基础命令：

```bash
npm --prefix apps/viewer test
npm --prefix apps/viewer run build
```

---

## 十三、可观测性

所有核心动作都必须产生结构化事件：

- agent_started / agent_finished
- llm_request_started / llm_stream_delta / llm_request_finished
- tool_call_started / tool_call_finished
- permission_decision
- phase_transition
- patch_artifact_created / patch_apply_result
- verification_started / verification_finished
- compaction_started / compaction_finished
- viewer_concern_raised

事件要求：

- 必须有 `trace_id`。
- 必须有 `session_id`。
- 必须有 schema version。
- secret/env/path-sensitive 字段必须 redacted。
- 写入 JSONL 后再通知 Viewer，避免 UI 看到无法 replay 的瞬时状态。

---

## 十四、分阶段落地

### Phase 0 — 技术基线

交付：

- 完成 Codex-first Reference Audit Baseline，并把审计结果挂到系统架构和 ADR。
- 新建 Rust workspace。
- 新建 `apps/viewer` React/Vite skeleton。
- 建立 CI：fmt、clippy、test、viewer build。
- 建立 `docs/adr/`。
- 写入 Rust-first ADR。

验收：

- `cargo test --workspace` 通过。
- `cargo clippy --workspace --all-targets -- -D warnings` 通过。
- `npm --prefix apps/viewer run build` 通过。
- permission、exec、patch、session、context、MCP/skills、observability 设计均有 Codex 首选参考路径。
- repo 中没有未提交生成物。

### Phase 1 — 单 Agent 纵切

交付：

- `whalecode-protocol`
- `whalecode-model`
- `whalecode-core`
- `whalecode-tools`
- `whalecode-permission`
- `whalecode-session`
- `whalecode-cli`

验收：

- CLI 能在真实 repo 中 read/search/edit 一个文件。
- write/edit 必须 read-before-write。
- DeepSeek mock SSE 覆盖 thinking + tool-call sub-turn。
- JSONL session 可 replay。
- Permission deny 优先级测试通过。

### Phase 2 — 群体协同 + Create/Debug 工作流

交付：

- `whalecode-workflow`
- `whalecode-swarm`
- `SwarmSpec`、`CohortSpec`、`WorkUnit`
- Scout / Analyst / Implementer / Reviewer / Judge / Verifier cohorts
- DiversityPolicy、effective_agent_count、Anti-ConformityProtocol
- Tournament、Evidence Race、Patch League、EvidenceWeightedConsensus
- ConcurrencyGovernor 和 SwarmBudget
- Create phase machine。
- Debug phase machine。
- PatchArtifact ownership gate。
- Reviewer verification gate。

验收：

- Create 能完成 plan tournament -> scaffold -> patch league / sharded implement -> review -> confirm。
- Debug 能完成 hypothesis cohort -> evidence race -> root-cause judge -> fix candidates -> verify。
- 8 个只读 Scout 可并行产出结构化 Finding。
- 同一 work unit 可产 2-4 个 PatchArtifact 候选，共享工作区只应用最终 patch。
- 429 mock、延迟和 token budget 能触发 ConcurrencyGovernor 降宽。
- HYPOTHESIZE 阶段没有写权限。
- Viewer critical concern 可阻止 phase transition。

### Phase 3 — TUI

交付：

- ratatui interactive UI。
- tool progress。
- permission prompt。
- patch preview。
- session status。

验收：

- 非交互 CLI 和 TUI 共享同一 core。
- TUI 关闭不破坏 session replay。
- permission prompt 可在 TUI 中完成。

### Phase 4 — Web Viewer

交付：

- read-only event bridge。
- Agent graph。
- DAG progress。
- evidence chain。
- metrics dashboard。

验收：

- Viewer 可从 live event 和 JSONL replay 两种来源渲染。
- Web Viewer 无 shell/write 入口。
- 断线重连后可按 seq 补事件。

### Phase 5 — Skills / MCP / Evolution

交付：

- MCP stdio client。
- Skills discovery。
- Skill usage telemetry。
- Evolution proposal workflow。

验收：

- MCP tools 经过 PermissionEngine。
- Skills 不能绕过 phase permission。
- Evolution proposal 需要 Viewer review。

---

## 十五、风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| Rust 开发速度慢于 TS | Phase 1 变慢 | 只做纵切，不过早做完整多 Agent；Web/插件仍用 TS |
| DeepSeek API 变化 | adapter 失效 | capability probe + mock SSE fixtures + provider version stamp |
| MCP Rust SDK 变化 | 集成成本高 | Phase 1 先实现最小 stdio JSON-RPC adapter |
| Web/Core schema 漂移 | Viewer 渲染错误 | `whalecode-protocol` 生成 JSON schema，Web 端从 schema 生成类型 |
| TUI 复杂度上升 | 交互不稳定 | CLI non-interactive 先行；TUI 只消费 core event |
| OS sandbox 跨平台差异 | 安全能力不一致 | Permission/CommandPolicy 先行，OS sandbox 按平台 feature flag |
| 多 Agent 写冲突 | 用户工作区损坏 | PatchArtifact + ownership + dry-run + write lock |
| 日志泄露隐私 | 开源用户不信任 | redaction 默认开启，敏感字段 opt-in 才显示 |

---

## 十六、参考来源

外部来源：

1. Rust 官方站点：可靠、高效、内存安全的软件构建定位。https://www.rust-lang.org/
2. Tokio 官方文档：Rust async runtime，覆盖 I/O、timer、filesystem、sync、scheduling。https://tokio.rs/
3. ratatui 官方站点：Rust terminal UI library。https://ratatui.rs/
4. clap docs：Rust command line argument parser。https://docs.rs/clap
5. serde docs：Rust serialization / deserialization framework。https://docs.rs/serde/latest
6. tracing docs：structured diagnostics、spans、causality。https://docs.rs/tracing/
7. Node.js Permission Model docs：Node 不把自身定位为恶意代码 sandbox。https://nodejs.org/api/permissions.html
8. Node.js Single Executable Applications docs。https://nodejs.org/api/single-executable-applications.html
9. Bun Single-file Executable docs。https://bun.sh/docs/bundler/executables
10. Deno Security and Permissions docs：`allow-run` 子进程权限风险。https://docs.deno.com/runtime/fundamentals/security/
11. DeepSeek API Models & Pricing。https://api-docs.deepseek.com/quick_start/pricing
12. DeepSeek Thinking Mode。https://api-docs.deepseek.com/guides/thinking_mode
13. DeepSeek Tool Calls。https://api-docs.deepseek.com/guides/tool_calls
14. MCP Rust SDK。https://github.com/modelcontextprotocol/rust-sdk
15. MCP build server docs。https://modelcontextprotocol.io/docs/develop/build-server

本地参考：

| 参考项目 | 本地路径 | 用途 |
|----------|----------|------|
| Codex CLI | `tmp/whalecode-refs/codex-cli` | Rust CLI/core/tool/context/permission/session 参考 |
| OpenCode | `tmp/whalecode-refs/opencode` | Go permission/session/file edit safety 参考 |
| Pi | `tmp/whalecode-refs/pi` | TypeScript agent loop/event/web-ui 参考 |
| Claude Code from Scratch | `tmp/whalecode-refs/cc-from-scratch` | 最小工具/skill/MCP 概念参考 |

参考审计：

- Codex-first Reference Audit: `docs/plans/2026-04-25-codex-first-reference-audit.md`
- 任何 Rust core 模块设计都必须记录 Codex 路径、采用行为、WhaleCode 差异、不采用边界和测试。

---

## 十七、执行结论

后续所有工程规划默认按以下边界推进：

1. WhaleCode 主体不是 TypeScript CLI，而是 Rust CLI/TUI/core。
2. TypeScript 只承担 Web Viewer、插件 SDK 示例和外围生态。
3. DeepSeek adapter 在 Rust 中实现，不把 OpenAI SDK 作为核心依赖。
4. MCP 和 Skills 通过协议接入，不能绕过 Rust PermissionEngine。
5. Codex CLI 更值得深入参考，但不直接 fork 成 WhaleCode 主线。
6. 成熟基础设施按 Codex-first 审计后实现，WhaleCode 自研只集中在 DeepSeek 适配、Create/Debug 原语、Swarm、Viewer 和群体协同策略。
