# WhaleCode 系统架构设计

---

## 一、项目概述

构建以 DeepSeek V4 模型为核心的终端 AI coding agent，对标 Claude Code / OpenCode / Codex CLI / Pi。

- **技术栈**: Rust-first core + TypeScript Web Viewer
- **目标模型**: `deepseek-v4-flash` + `deepseek-v4-pro`
- **核心定位**: Multi-Agent First + Coding-Native，极致适配 DeepSeek 模型特性

> 技术栈决策已更新为 Rust-first，详见 `docs/plans/2026-04-25-rust-first-technology-architecture.md` 与 `docs/adr/2026-04-25-rust-first-core-runtime.md`。本文早期章节中的 TypeScript 风格代码块保留为结构化伪代码，真正的 MVP 接口以第十八章 Rust 形态和新技术架构文档为准。

---

## 二、DeepSeek V4 极致适配

| 特性 | 数值 | 适配策略 |
|------|------|---------|
| 上下文窗口 | **1M tokens** | 上下文资源池化，按角色分配预算 |
| 最大输出 | **384K tokens** | Implementer 一次输出大量代码，减少通信 |
| 思考链 (Thinking) | `reasoning_content` | 当前按 DeepSeek API 的 thinking 模式设计；工具调用期间必须保留并回传当轮 `reasoning_content` |
| 推理强度 | `thinking` + `reasoning_effort` | 当前 API 支持 thinking toggle；thinking effort 使用 `high` / `max`，由 `ModelCapabilityProbe` 映射 |
| 平行工具调用 | 工具调用 + runtime 并发 | API 返回多个 tool call 时由 WhaleCode runtime 按工具并发安全性调度 |
| 缓存定价 | Flash cache hit 比 miss 便宜 **5 倍** | 共享 Stable Prefix，让多 Agent fan-out 尽量命中 context cache |
| API 兼容 | **OpenAI / Anthropic 格式** | Rust core 直接实现 HTTP/SSE adapter，保留 SDK 兼容但不依赖 SDK 作为核心边界 |
| V4-Flash 定价 | $0.28/M 输出 | 极低成本运行 Scout / Analyst / Implementer 等大量并行 Agent |
| V4-Pro 定价 | $3.48/M 输出 | 只在 Architect / Judge / Viewer / Root-cause 等关键路径使用 |

> **注意**：截至 2026-04-25，DeepSeek 官方 API 文档已公开列出 `deepseek-v4-flash` / `deepseek-v4-pro`，并标注旧 `deepseek-chat` / `deepseek-reasoner` 将于 2026-07-24 废弃。WhaleCode 仍必须通过 `ModelCapabilityProbe` 在运行时确认模型、context、output、thinking、tool-call、pricing 和 429 行为：
> - 实际上下文窗口、最大输出和 thinking/tool-call 兼容性来自 provider capability probe。
> - 缓存命中率和成本节省只作为调度优化，不作为正确性前提。
> - DeepSeek API 会动态限制并发并返回 429；Supervisor 必须用 `ConcurrencyGovernor` 动态调宽/降宽。
> - 定价随 API 版本和区域变化，成本估算必须带 `pricing_source` 与 `observed_at`。

---

## 三、Multi-Agent First 设计原则

传统架构是"先有一个 Agent，再扩展成多 Agent"。WhaleCode 反过来——**Agent 从不独立存在，总是群体中的一员**。单 Agent 模式只是多 Agent 的一个特例（群体大小为 1）。

- 通信协议是原语，不是扩展
- 调度器是核心组件，不是附加功能
- Agent 实例是轻量的（一个 loop + 一个 channel），可大规模创建
- 失败和隔离是设计假设，不是异常情况

多 Agent 群体协同的详细设计见 `docs/plans/2026-04-25-multi-agent-collaboration-architecture.md`。核心思想是用大量 V4-Flash agent 做并行探索、候选生成和局部实现，用少量 V4-Pro agent 做关键裁判、整合和对抗审查，通过 DiversityPolicy、Tournament、Evidence Race、Patch League、EvidenceWeightedConsensus 把数量转化为非冗余证据和可验证质量。

WhaleCode 相对参考项目的其他差异化原语见 `docs/plans/2026-04-25-differentiated-primitives-architecture.md`。证据链 Debug、脚手架先行、参考驱动、独立 Viewer、技能自进化都必须实现为 artifact schema、phase gate、session event 和 replayable state，不能只靠提示词约束。

---

## 四、两类原生任务：Create 与 Debug

WhaleCode 不做通用多 Agent，而是面向 coding 做领域特化。Create 和 Debug 是两种本质上不同的计算模式，作为**架构原语**内建，而不是通过 Skills/Tools 后期添加。

| 维度 | Create | Debug |
|------|--------|-------|
| **本质** | 发散: 一个需求 → 多个文件 | 收敛: 多个症状 → 一个根因 |
| **思维模式** | 构建 (Construction) | 诊断 (Diagnosis) |
| **DAG 拓扑** | 树形扩散 (1→N) | 树形收束 (N→1) |
| **上下文广度** | 全项目范围和约定 | 深入追踪特定代码路径 |
| **成功标准** | 功能完整、风格一致、可编译 | 复现消失、根因消除、无回归 |
| **典型耗时** | 长 (分钟到小时级) | 短 (秒到分钟级) |
| **工具重心** | Write / Edit / Glob | Bash / Grep / Git / Read |
| **Agent 角色组合** | Architect + 多 Implementer ×N + Reviewer | Debugger + Searcher + Implementer + Reviewer |

---

## 五、系统拓扑

```
                    ┌──────────────────────────┐
                    │      User Interface      │
                    │     (CLI / REPL / API)   │
                    └────────────┬─────────────┘
                                 │
                    ┌────────────▼─────────────┐
                    │      Supervisor          │
                    │  (生命周期 + 路由 + 调度)  │
                    │  ─ 非 AI，纯确定性调度     │
                    └──┬────┬────┬────┬────┬──┘
                       │    │    │    │    │
              ┌────────┘    │    │    │    └──────────┐
              │             │    │    │               │
         ┌────▼────┐   ┌───▼───┐│┌───▼───┐     ┌─────▼─────┐
         │Architect│   │Implement│││Reviewer│     │  Searcher  │
         │V4-Pro   │   │V4-Flash│││V4-Pro │     │  V4-Flash  │
         └────┬────┘   └───┬───┘│└───┬───┘     └─────┬─────┘
              │            │    │    │               │
              └────────────┴────┴────┴───────────────┘
                           │
                    ┌──────▼──────────────────────────┐
                    │         Agent Message Bus       │
                    └──────┬──────────────────────────┘
                           │
                    ┌──────▼──────┐
                    │   Viewer    │
                    │  V4-Pro     │ ← 常驻观察者，不归属任何工作流
                    │  只读       │    持续产出批判性意见
                    └─────────────┘

                    [+] Debugger (仅在 Debug 流程激活)
                        V4-Pro
```

### 5.1 角色定义

| 角色 | 用于 | 模型 | 工具权限 | 职责 |
|------|------|------|---------|------|
| **Architect** | Create | V4-Pro | 只读 | 系统设计、任务分解、质量把关 |
| **Debugger** | Debug | V4-Pro | 只读+Bash+Git | 问题分析→生成假设→设计证据计划→评估收敛→根因定位 |
| **Implementer** | 两者 | V4-Flash | 读写+Shell | 按 spec 实现代码，可大规模并行 |
| **Searcher** | 两者 | V4-Flash | 只读 | 搜索代码、路径追踪、收集上下文 |
| **Reviewer** | 两者 | V4-Pro | 只读+测试 | 代码审查、验证完整性、防回归 |
| **Viewer** | 跨工作流常驻 | V4-Pro | 只读+raise_concern | 持续产出对抗性批判、质疑假设、揭露盲区 |
| **Supervisor** | — | (非 AI) | 不直接调用 LLM | 生命周期管理、任务路由、恢复 |

模型选择原则：复杂推理任务（设计、诊断、审查）→ Pro/Reasoner 路线，常规任务（实现、搜索）→ Flash/Chat 路线。推理强度不写死为单一参数，而由 `ModelCapabilityProbe` 映射到当前 provider 支持的字段；上下文预算同样来自模型能力探测，避免把 V4 假设硬编码进 MVP。

### 5.2 Agent 抽象模型

```
┌──────────────────────────────────────┐
│ Agent                               │
│──────────────────────────────────────│
│ id: AgentId                          │
│ role: Architect                      │
│ inbox: Channel<Envelope>             │  ← 消息队列
│ context: AgentContext                │  ← 系统提示词 + 历史
│ workspace: WorkspaceRef              │  ← 文件系统视图
│ tools: ToolSet                       │  ← 可用工具
│ model: ModelConfig                   │  ← 模型选择 (Pro/Flash)
│ state: Idle | Busy | Blocked | Done  │
│──────────────────────────────────────│
│ loop() → 消费 inbox → LLM → 工具     │
│ send() → 发消息到 Message Bus         │
└──────────────────────────────────────┘
```

每个 Agent 的核心循环：

```
while (envelope = inbox.receive()) {
  context.append(envelope.message);
  response = llm.call(context, tools, model);

  if (response.hasToolCalls) {
    executeTools(response.toolCalls);
    continue; // 继续思考
  }

  if (response.isFinal) {
    supervisor.report(id, response);
    state = Done;
    break;
  }
}
```

Agent 是轻量级对象（一个 loop + 一个 channel），上下文懒加载。

---

## 六、系统级工作流

### 6.1 Create 工作流

Create 的核心原则：

1. **参考驱动（Reference-Driven）** — 任何设计决策前必须先研究社区做法，禁止仅凭模型内部知识做决定
2. **脚手架先行（Scaffolding First）** — 在实现功能代码之前，必须先建立日志、测试、开发约束三方面基建

这三项是功能代码的**前置依赖**，在任务 DAG 中表示为不可跳过的节点。

```
用户需求 ("给我加一个用户管理系统")
  │
  ▼
[Classifier] → 判定为 Create
  │
  ▼
┌──────────────────────────────────────────────────────────────┐
│ Phase 1: 理解与参考研究 (并行)                                  │
│  ┌───────────────────┐    ┌───────────────────────────────┐  │
│  │ 1a: Architect      │    │ 1b: Searcher ×N (参考研究)    │  │
│  │ • 理解需求          │    │ • WebSearch: "用户管理系统      │  │
│  │ • 分析项目结构      │    │   最佳实践 Rust CLI agent"     │  │
│  │ • 识别技术栈        │    │ • 技术评估: "ratatui vs       │  │
│  │                     │    │   bubbletea 对比"             │  │
│  │                     │    │ • 竞品分析: "类似项目踩坑记录"  │  │
│  │                     │    │ • 失败案例: "权限系统的常见漏洞" │  │
│  └────────┬──────────┘    └──────────────┬────────────────┘  │
│           └──────────────────┬───────────┘                   │
│                              │                               │
│  架构约束 — Supervisor 等待:                                  │
│  ⏳ Architect 初步理解完成                                    │
│  ⏳ 至少 3 个 Searcher 返回参考结果                            │
└──────────────────────────────┬───────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│ Phase 2: Architect — 设计 (必须引用参考)                      │
│  • 模型: V4-Pro                                                │
│  • 禁止仅凭模型内部知识做设计                                    │
│  • 输入: 需求 + 项目结构 + Searcher 提供的参考结果               │
│  • 输出:                                                     │
│    1. 功能设计文档 (必须包含 Reference 章节，链接到 Searcher 结果)│
│    2. 任务 DAG + 脚手架需求清单                                 │
│    3. 依赖图: 参考任务 → 基建任务 → 功能任务                    │
│  • 上下文: 全项目范围 + 参考结果摘要                             │
│                                                               │
│  Viewer 在此阶段立即介入:                                      │
│  → "设计方案引用了一个 2024 年的库，但该库 2025 年已废弃"       │
│  → "参考的 RBAC 方案涉及 GDPR 合规，设计中没有考虑"            │
│                                                               │
│  架构约束 — 输出必须通过:                                      │
│  ✅ Artifact Contract: 设计文档包含 ≥3 个外部引用               │
│  ✅ Viewer 对设计无 critical-level Concern                      │
└──────────────────────┬───────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│ Phase 3: Scaffolding — 基建先行                               │
│  • 每个基建 Implementer 在选型时也必须做参考研究:               │
│    → "tracing 结构化日志与 JSONL event sink 最佳实践"          │
│    → "cargo test vs nextest 在 workspace 中的取舍"             │
│    → "clippy/rustfmt workspace 约束最佳实践"                   │
│                                                               │
│   ┌────────────────────┬───────────────────┬──────────────┐  │
│   │ Logging            │ Testing           │ Constraints  │  │
│   │ Implementer        │ Implementer       │ Implementer  │  │
│   │ 选型+配置           │ 选型+配置          │ 选型+配置    │  │
│   └────────┬───────────┴────────┬──────────┴──────┬───────┘  │
│            │                    │                  │          │
│   Reviewer 检查: 日志有输出？测试能跑？Lint 通过？              │
└──────────────────────┬───────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│ Phase 4: Implementers — 并行实现 (每段代码参考驱动)            │
│  • 模型: V4-Flash                                               │
│  • 每个 Implementer 在实现前必须:                               │
│    1. 搜索该功能的最佳实践/参考实现                              │
│    2. 引用具体的代码模式（如 "NX 官方 monorepo 推荐的目录结构"）│
│    3. 检查是否有已知的反模式/陷阱                                │
│  • 每个叶子任务领取后，Searcher 并行提供参考上下文                │
│  • 冲突区域声明: 写同一文件的需排队                               │
│  • 并行数: 默认 3-8                                            │
│                                                               │
│     Implementer A         Implementer B         Implementer C  │
│     实现 Model            实现 API Route         实现 View      │
│     + 参考: RBAC 模式     + 参考: RESTful 规范   + 参考: 主流   │
│     + 对应的测试          + 对应的测试            UI 组件库    │
│     + 对应的日志          + 对应的日志           + 对应的测试  │
│                                                               │
│  Viewer 审查每段代码:                                          │
│  → "这个鉴权实现与参考的 RBAC 模式有出入，遗漏了角色继承"        │
│  → "API 路由命名不符合 RESTful 规范，参考的案例用的是复数"      │
└──────────────────┬──────────────────┬──────────────────┬───────┘
                   │                  │                  │
                   └──────────────────┼──────────────────┘
                                      │
                                      ▼
┌──────────────────────────────────────────────────────────────┐
│ Phase 5: Reviewer — 审查与集成                                 │
│  • 模型: V4-Pro                                                 │
│  • 审查维度:                                                   │
│    1. 功能正确性 — 是否满足需求                                  │
│    2. 参考合规 — 实现与引用的最佳实践是否一致                    │
│    3. 基建合规 — 是否使用了已建的日志/测试/约束体系              │
│    4. 边界情况 — 错误处理、空值、并发                            │
│    5. 测试覆盖 — 新增代码有对应测试                              │
│  • 发现问题 → 打回给对应 Implementer + Review Comment            │
│  • 全部通过 → 合并到主工作区                                    │
└──────────────────────┬───────────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────────┐
│ Phase 6: Architect — 最终确认                                  │
│  • 验证整体是否满足需求                                         │
│  • 确认参考被正确吸收                                           │
│  • 输出结果摘要 + 参考来源列表给用户                             │
└──────────────────────────────────────────────────────────────┘
```

#### 参考能力在 DAG 中的表达

```typescript
const CREATE_DAG_RULES: DAGRule[] = [
  // 规则 1: 每个 feature 任务必须依赖至少一个 scaffolding 任务
  // (同上，略)
  
  // 规则 2: 设计任务必须依赖至少一个参考研究任务
  {
    check: (tasks, deps) => {
      const designTasks = tasks.filter(t => t.role === "architect" && t.phase === "design");
      const researchTaskIds = tasks.filter(t => t.category === "research").map(t => t.id);
      for (const dt of designTasks) {
        const ok = deps.some(d => d.to === dt.id && researchTaskIds.includes(d.from));
        if (!ok) return { valid: false, offendingTask: dt.id, 
          message: "Design must reference at least one community research task" };
      }
      return { valid: true };
    },
    errorMessage: "Design task must reference at least one community research task",
  },

  // 规则 3: 实现任务必须参考具体模式或参考实现
  {
    check: (tasks, deps) => {
      const implTasks = tasks.filter(t => t.role === "implementer");
      const refTaskIds = tasks.filter(t => t.category === "reference").map(t => t.id);
      for (const it of implTasks) {
        const ok = deps.some(d => d.to === it.id && refTaskIds.includes(d.from));
        if (!ok) return { valid: false, offendingTask: it.id,
          message: "Implementation must reference a pattern or reference implementation" };
      }
      return { valid: true };
    },
    errorMessage: "Implementation task must reference a known pattern",
  },
];
```

#### Artifact Contract: 设计文档必须包含引用

```typescript
const DESIGN_ARTIFACT_CONTRACT = {
  requiredSections: ["overview", "architecture", "references"],
  references: {
    minCount: 3,                          // 至少引用 3 个外部来源
    requiredTypes: ["best_practice"],      // 必须包含最佳实践类引用
    format: {
      source: string,                      // URL 或具体来源
      relevance: string,                   // 与本项目的关联
      keyInsight: string,                  // 从中获得的关键洞察
    }
  },
  validate(artifact: DesignArtifact): ValidationResult {
    if (artifact.references.length < this.references.minCount) {
      return { 
        passed: false, 
        issue: `Design must include at least ${this.references.minCount} external references` 
      };
    }
    return { passed: true };
  }
};
```

#### Searcher 的参考研究工具集

Searcher 不再仅限于本地搜索，而是获得外部研究能力：

| 工具 | 用途 | 示例 |
|------|------|------|
| `WebSearch` | 搜索最佳实践、教程、对比 | "RBAC 权限设计最佳实践 2025" |
| `GitHubSearch` | 查找参考实现、项目结构 | "开源 Rust CLI agent 项目" |
| `RegistryLookup` | 包生态评估 | crates.io: ratatui vs alternatives |
| `DocSearch` | 框架官方文档 | Tokio / clap / ratatui 配置指南 |
| `FailureLookup` | 失败案例/反模式 | "权限系统常见漏洞" |

#### 脚手架先行在 DAG 中的表达

```typescript
// Supervisor 验证规则: 基建任务必须排在功能任务之前
const SCREENING_RULES = {
  scaffoldingCategories: ["logging", "testing", "constraints"] as const,

  // 任何功能任务必须至少依赖一个同领域的基建任务
  validate(task: Task, allTasks: Task[]): boolean {
    if (task.category === "feature") {
      return this.scaffoldingCategories.some(cat =>
        task.dependsOn.some(depId =>
          allTasks.find(t => t.id === depId)?.category === `scaffolding:${cat}`
        )
      );
    }
    return true; // 基建任务本身不需要验证
  }
};
```

#### 基建类型与验收标准

| 基建类别 | 典型产出 | 验收标准 |
|---------|---------|---------|
| **Logging** | Logger 配置、分级策略、请求追踪中间件、错误格式化器 | `log.info()` 输出结构化 JSON，支持 debug/info/warn/error 四级，含 requestId |
| **Testing** | Rust 测试框架配置、fixture repo、mock DeepSeek SSE、CI workflow | `cargo test --workspace` 可运行，有示例测试通过，session replay 可验证 |
| **Constraints** | `rustfmt`、`clippy`、workspace lint、目录结构、Web Viewer 前端 lint | `cargo fmt --check` 和 `cargo clippy --workspace --all-targets -- -D warnings` 通过 |

### 6.2 Debug 工作流（基于证据链）

Debug 的核心不是"追踪代码找到问题"，而是**结构化科学推理**：从目标问题出发，生成假设，设计实验寻找证据，通过证实/证伪假设来收敛到根因。

#### 证据链数据模型

```
                   ┌─────────────────────────┐
                   │    Goal (任务目标)       │
                   │  "登录页面500错误"        │
                   └───────────┬─────────────┘
                               │ 驱动
                               ▼
              ┌────────────────────────────────┐
              │  Hypotheses (假设池)            │
              │                                │
              │  H₁: 数据库连接失败              │── 待验证
              │  H₂: 中间件权限校验抛异常       │── 待验证
              │  H₃: Redis session 过期        │── 待验证
              │  H₄: 前端请求格式不对           │── 已排除
              └──────────┬────────────────────┘
                         │ 验证
                         ▼
              ┌────────────────────────────────┐
              │  Evidence (证据链)              │
              │                                │
              │  E₁: pg_isready 输出           │── 支持 H₁
              │  E₂: Nginx 403 日志            │── 支持 H₂，排除 H₄
              │  E₃: Redis ping 正常           │── 排除 H₃
              │  E₄: 调用链追踪到 AuthFilter    │── 确认 H₂
              └────────────────────────────────┘
```

```typescript
type DebugSession = {
  goal: string;                // Bug 描述（任务目标）
  knownInfo: KnownFact[];      // 已知信息（错误信息、堆栈、环境等）
  hypotheses: Hypothesis[];    // 假设池
  evidence: Evidence[];        // 证据链
  status: "active" | "converged" | "stuck";
};

type Hypothesis = {
  id: string;
  description: string;         // 可验证的根因假设
  confidence: number;          // 初始置信度 0-1
  status: "unverified" | "supported" | "refuted" | "inconclusive";
  evidenceIds: string[];       // 关联的证据
};

type Evidence = {
  id: string;
  description: string;         // "检查 UserService.login() 的 null 指针"
  plan: EvidencePlan;          // 如何收集这个证据
  result: EvidenceResult;      // 收集到的结果
  supports: string[];          // 支持的假设 ID
  refutes: string[];           // 证伪的假设 ID
  status: "planned" | "collecting" | "collected" | "failed";
};

type EvidencePlan = {
  action: "run_command" | "read_file" | "trace_code" | "check_log" | "write_test" | "inspect_var";
  detail: string;              // 具体操作描述
  target: string;              // 目标文件/命令
};
```

#### Debug 工作流

```
Bug 报告 ("登录页面报 500 错误")
  │
  ▼
[Classifier] → 判定为 Debug
  │
  ▼
┌──────────────────────────────────────────────────────────────────┐
│ Phase 1: Problem Analysis — 理解问题域                           │
│ Debugger (V4-Pro)                                                │
│                                                                  │
│  1. 解析 Bug 报告 → 提取 Goal + KnownInfo                        │
│  2. 复现问题 → 验证可复现性，收集复现步骤                         │
│  3. 建立问题上下文 → 了解相关代码区域                             │
│                                                                  │
│  输出: DebugSession (goal + knownInfo + 问题上下文)               │
└──────────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────────┐
│ Phase 2: Hypothesis Generation — 生成假设池                      │
│ Debugger (V4-Pro)                                                │
│                                                                  │
│  根据 Goal + KnownInfo 生成 N 个可验证的假设:                     │
│                                                                  │
│    H₁: [置信度: 0.7] 数据库连接池耗尽导致查询超时                │
│      → 证据计划: 检查数据库连接数和超时配置                       │
│                                                                  │
│    H₂: [置信度: 0.5] AuthFilter 中 JWT 解析抛 NPE               │
│      → 证据计划: 检查 AuthFilter 第 42 行的 null 判断            │
│                                                                  │
│    H₃: [置信度: 0.3] Redis session 丢失导致重定向循环           │
│      → 证据计划: 检查 Redis key 是否存在，TTL 是否正常           │
│                                                                  │
│  假设优先级 = f(置信度, 验证成本)                                │
│                                                                  │
│  输出: DebugSession.hypotheses[] (每个假设附带证据计划)          │
└──────────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────────┐
│ Phase 3: Evidence Collection — 并行收集证据                      │
│ Searcher(s) (V4-Flash, 可并行多个)                                │
│                                                                  │
│  Supervisor 将每个假设的证据计划分配给 Searcher:                  │
│                                                                  │
│    Searcher A ← H₁: 运行 SQL 诊断命令，检查连接池配置            │
│    Searcher B ← H₂: 读取 AuthFilter 源码，追踪调用路径           │
│    Searcher C ← H₃: Redis CLI 检查 key 状态，检查 session 配置  │
│                                                                  │
│  每个 Searcher 返回 Evidence { result, supports, refutes }       │
│                                                                  │
│  收集到的证据自动关联到对应假设:                                  │
│    E₁: pg_stat_activity 显示 80 个空闲连接 → H₁ 证伪            │
│    E₂: AuthFilter:42 user==null 未被检查 → H₂ 支持              │
│    E₃: Redis key TTL 正常 → H₃ 证伪                             │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
  │
  ▼
┌──────────────────────────────────────────────────────────────────┐
│ Phase 4: Hypothesis Evaluation — 评估与收敛                      │
│ Debugger (V4-Pro)                                                │
│                                                                  │
│  评估所有证据，更新假设状态:                                      │
│                                                                  │
│    H₁: 证伪 (数据库正常)                                         │
│    H₂: 支持 (null 指针路径确认) → 置信度 0.7→0.9               │
│    H₃: 证伪 (Redis 正常)                                         │
│                                                                  │
│  ┌──────────────────────────────────────────────────────┐        │
│  │ 收敛判断:                                              │        │
│  │  ├─ 唯一假设被证实 → 根因定位 → 进入 Fix Phase        │        │
│  │  ├─ 全部被证伪     → 返回 Phase 2 重新生成假设        │        │
│  │  ├─ 部分支持/部分待定 → 生成第二轮证据计划 → Phase 3 │        │
│  │  └─ 卡住            → 升级到更广上下文或请求用户输入  │        │
│  └──────────────────────────────────────────────────────┘        │
└──────────────────────────────────────────────────────────────────┘
  │ (收敛到根因)
  ▼
┌──────────────────────────────────────────────────────────────────┐
│ Phase 5: Fix — 修复 & 验证                                       │
│                                                                  │
│  5a: Searcher — 影响域分析                                       │
│    • 查找根因位置的调用链、相关测试、类似模式                     │
│    • 输出: 受影响文件 + 相关测试用例                              │
│                                                                  │
│  5b: Implementer — 修复                                          │
│    • V4-Flash                                                    │
│    • 根据 Debugger 的根因分析 + Searcher 的影响域实现修复        │
│                                                                  │
│  5c: Reviewer — 验证                                             │
│    • V4-Pro                                                      │
│    • 检查: 根因是否解决、是否引入回归、修复方式是否最优           │
│    • 运行相关测试                                                 │
│                                                                  │
│  5d: Debugger — 闭环确认                                         │
│    • 确认症状消失 (重新运行复现步骤)                              │
│    • 输出: 根因证据链 + 修复 + 预防建议（如加回归测试）          │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

#### 证据链的可视化反馈（给用户看）

用户在 CLI 中可以实时看到证据链的收敛过程：

```
🔍 Debug Session: "登录页面报 500 错误"
─────────────────────────────────────────
Goal: 登录接口 POST /api/login 返回 500

Hypotheses:
  H₁: 数据库连接池耗尽        ████████░░ 证伪 (E₁)
  H₂: AuthFilter NPE          █████████░ 支持 (E₂, E₄) ← 根因
  H₃: Redis session 丢失     ██████░░░░ 证伪 (E₃)
  H₄: 前端请求格式不对       ████░░░░░░ 证伪 (E₂)

Evidence Chain:
  E₁: pg_stat_activity → 80 空闲连接          [证伪 H₁]
  E₂: AuthFilter:42 user==null 无保护         [支持 H₂, 证伪 H₄]
  E₃: Redis ping pong 正常                     [证伪 H₃]
  E₄: 调用链路追踪 → AuthFilter.doFilter() 抛 NPE [确认 H₂]

Root Cause: AuthFilter.java:42 — user 参数为 null 时未做空判断
Fix: Implementer 已修复 → Reviewer 已通过 → 复现步骤已验证
```

---

## 七、架构约束层：把工作模式编入架构，而非提示词

工作流约束（如"脚手架先行"、"证据链驱动"）如果只放在系统提示词里，LLM 随时可能偏离——上下文膨胀后会"遗忘"、推理路径会抄近路、输出格式会走样。WhaleCode 的做法是**用多层架构约束来兜底**，越靠近核心的约束越硬：

```
                    ┌─────────────────────────────────────┐
                    │  Layer 1: Phase State Machine       │ ← 最硬
                    │  (什么阶段做什么事，由代码决定)       │
                    ├─────────────────────────────────────┤
                    │  Layer 2: Tool Permissions          │
                    │  (每个阶段能用什么工具，由运行时决定)  │
                    ├─────────────────────────────────────┤
                    │  Layer 3: DAG Validation            │
                    │  (任务依赖关系，由调度器强制检查)      │
                    ├─────────────────────────────────────┤
                    │  Layer 4: Artifact Contracts        │
                    │  (产物的结构必须符合 Schema)          │
                    ├─────────────────────────────────────┤
                    │  Layer 5: Context Allocation        │
                    │  (每个角色聚焦到其任务范围内的上下文) │
                    ├─────────────────────────────────────┤
                    │  Layer 6: System Prompt             │ ← 最软
                    │  (仅用于引导"怎么做"，不用于约束"做什么")│
                    └─────────────────────────────────────┘
```

### 7.1 Layer 1: Phase State Machine

这是最硬的约束。工作流是一个确定性的状态机，**阶段转换由 Supervisor 控制，不由 LLM 决定**。

```
Create 状态机:
  IDLE → ANALYZE → DESIGN_DONE → SCAFFOLD → SCAFFOLD_DONE
    → SEARCH → SEARCH_DONE → IMPLEMENT → IMPLEMENT_DONE
    → REVIEW → REVIEW_DONE → CONFIRM → CONFIRM_DONE → RESULT

Debug 状态机:
  IDLE → ANALYZE → ANALYZE_DONE → HYPOTHESIZE → HYPOTHESES_READY
    → COLLECT_EVIDENCE → EVIDENCE_COLLECTED → EVALUATE
    → ├─ CONVERGED → FIX → FIX_DONE → VERIFY → VERIFIED → RESULT
      ├─ REFUTED_ALL → (回到 HYPOTHESIZE)
      └─ INCONCLUSIVE → (回到 COLLECT_EVIDENCE)
```

规则：
- Agent 运行在某个 Phase 内部，它不能自行触发 Phase 转换
- Phase 转换由 Supervisor 在满足 Gate 条件后执行
- Gate 条件由代码检查，不依赖 LLM 判断

```typescript
// Phase Gate 示例: Scaffolding → Implement 的门禁
const GATE_ScaffoldingToImplement: Gate = {
  from: "SCAFFOLD",
  to: "IMPLEMENT",
  
  async check(context: WorkflowContext): Promise<GateResult> {
    const issues: string[] = [];
    
    if (!await context.fileExists("src/utils/logger.ts"))
      issues.push("Logging scaffolding: logger.ts not found");
    if (!await context.hasScript("package.json", "test"))
      issues.push("Testing scaffolding: test script not configured");
    if (!await context.hasScript("package.json", "lint"))
      issues.push("Constraints scaffolding: lint script not configured");
    
    return issues.length === 0 
      ? { passed: true }
      : { passed: false, issues };
  }
};
```

### 7.2 Layer 2: Tool Permissions

每个 Phase 对每个角色开放的工具集不同。这是运行时级别的访问控制。

```typescript
const PHASE_TOOL_PERMISSIONS: Record<string, Record<Role, ToolPermission[]>> = {
  
  "SCAFFOLD": {
    Architect:   ["read", "glob", "grep"],
    Implementer: ["read", "write", "edit", "bash"],
    Reviewer:    ["read", "glob", "grep", "bash"],
    Searcher:    ["read", "glob", "grep"],
  },
  
  "IMPLEMENT": {
    Architect:   ["read", "glob", "grep"],
    Implementer: ["read", "write", "edit", "bash"],
    Reviewer:    ["read", "glob", "grep", "bash"],
    Searcher:    ["read", "glob", "grep"],
  },
  
  "HYPOTHESIZE": {
    Debugger:    ["read", "glob", "grep", "git_read"],
    Implementer: [],
    Searcher:    ["read", "glob", "grep"],
  },
  
  "FIX": {
    Debugger:    ["read", "glob", "grep", "git"],
    Implementer: ["read", "write", "edit", "bash"],
    Reviewer:    ["read", "glob", "grep", "bash"],
  },
};
```

**关键设计**：Debug 的 `HYPOTHESIZE` 阶段中没有任何角色可写文件，也不开放通用 `bash`。`git_read` 是受控只读能力，只允许 `status`、`diff`、`log`、`show`、`grep` 等不会改工作区或索引的命令。这就从架构上保证了"必须先诊断，不能边猜边改"，不需要在提示词里写"请先分析再修改"。

### 7.3 Layer 3: DAG Validation

Supervisor 在接收 Architect/Debugger 的任务 DAG 时，会做结构验证。

```typescript
type DAGRule = {
  check: (tasks: Task[], deps: Dependency[]) => RuleResult;
  errorMessage: string;
};

const CREATE_DAG_RULES: DAGRule[] = [
  // 规则 1: 每个 feature 任务必须依赖至少一个 scaffolding 任务
  {
    check: (tasks, deps) => {
      const featureTasks = tasks.filter(t => t.category === "feature");
      const scaffTaskIds = tasks.filter(t => t.category.startsWith("scaffolding")).map(t => t.id);
      for (const ft of featureTasks) {
        const ok = deps.some(d => d.to === ft.id && scaffTaskIds.includes(d.from));
        if (!ok) return { valid: false, offendingTask: ft.id };
      }
      return { valid: true };
    },
    errorMessage: "Feature task must depend on at least one scaffolding task",
  },
];

const DEBUG_DAG_RULES: DAGRule[] = [
  // 规则 1: Fix 任务必须依赖至少一个 evidence 任务
  {
    check: (tasks, deps) => {
      const fixTasks = tasks.filter(t => t.category === "fix");
      const evIds = tasks.filter(t => t.category === "evidence").map(t => t.id);
      for (const ft of fixTasks) {
        const ok = deps.some(d => d.to === ft.id && evIds.includes(d.from));
        if (!ok) return { valid: false, offendingTask: ft.id };
      }
      return { valid: true };
    },
    errorMessage: "Fix task must depend on at least one evidence collection task",
  },
];
```

### 7.4 Layer 4: Artifact Contracts

每个角色的输出产物有确定的 Schema。Supervisor 在接收产物时做 Schema 校验，不符合的打回重做。

```
Debugger → Hypothesis Generation 的输出契约:
  ✓ 每个假设必须包含 description + confidence (0-1)
  ✓ 每个假设必须附带 evidencePlan
  Schema:
  {
    hypotheses: [{
      description: string,
      confidence: number,
      evidencePlan: {
        action: "run_command" | "read_file" | "trace_code" | "check_log" | "write_test",
        detail: string,
        target: string
      }
    }]
  }

Architect → Task Plan 的输出契约:
  ✓ 必须包含至少 1 个 scaffolding 类任务
  ✓ 每个 feature 任务必须通过 DAG 依赖到 scaffolding 任务
  ✓ 任务数量不超过上下文预算上限
```

### 7.5 Layer 5: Context Allocation

利用 DeepSeek 1M 上下文窗口的优势，不做激进裁剪，而是按角色职责分配相关上下文：

```
Create Phase 2 (Scaffolding):
  Architect:   系统设计文档 + 项目结构 (全量)
  Implementer: 脚手架相关配置文件 (聚焦)
  Reviewer:    验收标准 + 脚手架产出 (聚焦)

Create Phase 4 (Implement):
  Implementer: 其分配任务相关文件 (聚焦)
               无需知道其他 Implementer 细节

Debug Phase 2 (Hypothesize):
  Debugger: Bug 复现信息 + 相关代码路径 (深度聚焦)

Debug Phase 3 (Collect Evidence):
  Searcher: 仅其被分配的证据目标 (聚焦)
```

### 7.6 Layer 6: System Prompt

经过上面 5 层约束过滤后，System Prompt 只负责"怎么做"，不需要负责"做什么"：

```
✅ 好的提示词: "在设置日志框架时，推荐使用 tracing，配置结构化 JSON 输出，
   包含 timestamp、level、msg、requestId 字段"

❌ 不好的提示词: "请记住，在实现功能代码之前必须先搭建设施"
   这条已被 Layer 3 DAG Validation 强制执行，提示词不必再写
```

### 7.7 约束层对比

| 约束层 | 硬度 | 违反后果 | 实现位置 |
|--------|------|---------|---------|
| Phase State Machine | 最高 | 转换被阻止 | Supervisor |
| Tool Permissions | 高 | 工具调用被拒绝 | Runtime |
| DAG Validation | 高 | 任务 DAG 被拒绝 | Scheduler |
| Artifact Contracts | 中 | 产物被打回重做 | Supervisor |
| Context Allocation | 低 | LLM 上下文不聚焦 | Context Manager |
| System Prompt | 低 | LLM 可能偏离 | — |

### 7.8 Layer 7 (Cross-cutting): Independent Viewer — 对抗性批评层

Viewer 不归属于任何工作流阶段。它是一个**常驻的、独立的对抗性观察者**，持续监听 Message Bus 上的 Agent 产出，**在每个步骤**主动给出批判意见——从 Architect 的设计方案、到 Debugger 的每个假设、到 Implementer 的每一段代码。

#### 与 Reviewer 的区别

```
Reviewer: 仅存在于 Phase 末端的质量门禁
  • 触发: Phase 结束时 Supervisor 调用
  • 频次: Phase 一次，只在收口
  • 视角: 检查产物是否满足验收标准
  • 输出: Pass / Fail + Issues
  • 关注: 功能正确性、完整性、一致性

Viewer: 全流程每个步骤的对冲机制
  • 触发: 事件驱动 (每次 Agent 产出后立即触发)，异步
  • 频次: 每个 Agent 消息，每个小步骤
  • 视角: 质疑假设、揭露盲区、对抗性反驳
  • 输出: Concern (警告/建议/反驳) + 置信度
  • 关注: 过度自信、隐藏假设、遗漏场景、幻觉
```

Viewer **渗透在每个环节**，而非仅在收口：

```
Create 工作流各环节的 Viewer 介入:
  ┌─ Phase 1: Architect 设计
  │   [Architect 输出设计文档] → Viewer 立即分析
  │   → "这个方案假设了所有请求都有用户凭证，但登录请求没有"
  │   → "任务 DAG 中遗漏了错误处理中间件的基建任务"
  │
  ├─ Phase 2: Scaffolding
  │   [Implementer 配置日志框架] → Viewer 检查
  │   → "这个日志配置没有包含 requestId 追踪，未来调试会很困难"
  │
  ├─ Phase 4: Implement
  │   [Implementer 提交代码] → Viewer 每段代码分析
  │   → "这里直接返回了数据库错误信息给前端，会泄露表结构"
  │   → "这个循环没有设置上限，用户输入过长会导致 OOM"
  │
  └─ Phase 5: Reviewer
      [Reviewer 给出审查意见] → Viewer 也审查 Reviewer
      → "Reviewer 遗漏了安全性检查，代码中存在 SQL 注入风险"

Debug 工作流各环节的 Viewer 介入:
  ┌─ Phase 2: Hypothesize
  │   [Debugger 提出假设 H₁-H₄] → Viewer 检查
  │   → "H₂ 的"测试通过"不能证明根因是 NPE，证明方向反了"
  │
  ├─ Phase 3: Collect Evidence
  │   [Searcher 返回证据 E₁] → Viewer 评估
  │   → "这条证据链不完整，没有检查超时配置的默认值"
  │
  └─ Phase 4: Evaluate
      [Debugger 说"收敛到 H₂"] → Viewer 挑战
      → "H₄ 虽然置信度低，但没有被任何证据证伪，不能排除"
```

#### 触发机制

Viewer 通过三种方式被触发：

```
1. 事件驱动 (主要)
   Agent 产出 → Message Bus 发布 → Viewer 收到 → 分析 → 产出 Concern
   
   [Implementer] ──task_result──► [Bus] ──► [Viewer]
                                            │
                                            ├─ Concern: "这段代码忽略了空值情况"
                                            ├─ Concern: "引用的 API 在 v2 已废弃"
                                            └─ Concern: "这个算法在大数据量下会 OOM"

2. 周期巡检 (次要)
   长时间无产出的 Phase 中，Viewer 主动检查当前状态
   → "Phase 3 已经进行了 15 分钟，仍未产生任何任务结果"
   → "Debugger 的假设 H₃ 置信度从 0.3 降到 0.1 但未排除"

3. Supervisor 按需调用
   遇到不确定情况时，Supervisor 主动请求 Viewer 介入
   → "DAG 验证器发现异常模式，请 Viewer 审查 Architect 的设计"
```

#### Concern 模型

```typescript
type Concern = {
  id: string;
  severity: "critical" | "warning" | "suggestion" | "question";
  category: ConcernCategory;
  target: {               // 指向被质疑的对象
    agentId: AgentId;
    artifactId?: string;
    phase?: string;
  };
  claim: string;           // Viewer 认为的问题描述
  evidence: string;        // 支撑的证据（代码引用、逻辑推理）
  counterProposal?: string; // 可选的改进建议
  confidence: number;      // Viewer 自己对此 concern 的自信度 0-1
};

type ConcernCategory =
  | "overconfidence"     // 断言没有足够证据支持
  | "hidden_assumption"  // 隐含假设未明确说明
  | "hallucination"      // 引用不存在的方法/API/文件
  | "missing_edge_case"  // 遗漏了边界场景
  | "inconsistency"      // 与已有代码或设计不一致
  | "security"           // 安全风险
  | "performance"        // 性能问题
  | "architecture"       // 架构偏离
  | "process";           // 流程问题（如跳过了必要步骤）
```

#### 处理流程

```
Agent 产出 → Viewer 分析 → Concern(s)
                            │
                      ┌─────┴──────┐
                      ▼            ▼
               severity=critical    warning/suggestion
                      │                  │
                      ▼                  ▼
              Supervisor 暂停 Phase    Supervisor 记录 Concern
                      │                  │
                      ▼                  ▼
              打回 Agent 修复       Agent 可选择性回应
              + Concern 附加在      + Concern 附加在上下文
              上下文里继续追踪      中作为背景信息
```

**Critical Concern 示例**：
```
Debugger: "根因是数据库连接池耗尽"
Viewer: {
  severity: "critical",
  category: "overconfidence",
  claim: "pg_stat_activity 显示 80 个空闲连接，与连接池耗尽的假设矛盾",
  evidence: "Phase 3 的 Evidence E₁ 明确显示连接池健康",
  confidence: 0.95
}
→ Supervisor 阻止进入 Fix 阶段，要求 Debugger 重新评估
```

#### Viewer 的独立性保障

Viewer 的独立性是架构层面的，不是靠提示词"请独立思考"：

```typescript
const VIEWER_INDEPENDENCE: ArchitecturalGuarantee = {
  // 1. 信息隔离 — Viewer 看不到其他 Agent 的推理过程
  contextAccess: ["output_only"],  // 只能看到产出的结果，看不到 prompt/内部思考

  // 2. 模型隔离 — 使用独立的 model config（防止共享推理偏置）
  modelConfig: {
    model: "deepseek-v4-pro",
    temperature: 0.3,
  },

  // 3. 时间隔离 — Viewer 产出 Concern 不影响 Agent 当前操作
  execution: "async_non_blocking", // Viewer 异步运行，不阻塞工作流

  // 4. 视角隔离 — Viewer 的系统提示词与其他 Agent 不同
  systemPromptTone: "adversarial", // 对抗性立场，鼓励质疑

  // 5. 权限隔离 — Viewer 只读，不能修改任何文件
  tools: ["read", "glob", "grep", "raise_concern"],
  // raise_concern 是 Viewer 独有的工具，用于将 Concern 写入 Message Bus
};
```

#### Viewer 与约束层的关系

```
  6 层约束 (确定性)    +    Viewer (对抗性)     =   完整约束体系
  ────────────────         ───────────             ──────────────
  Phase Machine           质疑阶段合理性            流程不错，方向也对
  Tool Permissions        质疑工具使用是否恰当     能做的不等于该做的
  DAG Validation          质疑依赖关系是否遗漏     结构对了，但假设错了  
  Artifact Contracts      质疑产物的隐含假设       格式对了，但内容有问题
  Context Allocation      质疑上下文盲区           没看到的不等于不存在
  System Prompt           Viewer 本身就是          提示词的建议由
                          对提示词的对冲           Viewer 来挑战
```

---

## 八、能力模型：Skills / Tools / MCP

Create 和 Debug 是 WhaleCode 的原生工作流，但不排斥业界通用能力。Skills、Tools、MCP 构成能力层——让 Agent 可以执行通用任务、复用社区生态、接入外部系统。

### 8.1 能力分层

```
┌─────────────────────────────────────────────┐
│   Workflows (工作流)                          │
│   Create / Debug                             │ ← 原生，面向 coding 特化
├─────────────────────────────────────────────┤
│   Skills (技能)                               │
│   可组合的通用能力单元                          │ ← 可复用、可共享
│   "代码审查"、"重构"、"写测试"等                │
├─────────────────────────────────────────────┤
│   Tools (工具)                                │
│   原子操作：Read / Write / Edit / Bash / ...   │ ← Agent 直接调用的最小单元
├─────────────────────────────────────────────┤
│   MCP (协议)                                  │
│   连接外部工具和数据源的标准化协议               │ ← 生态接入层
└─────────────────────────────────────────────┘
```

### 8.2 Skills — 可组合的技能系统

Skills 是介于 Workflow 和 Tool 之间的可复用能力单元。一个 Skill 封装了多个 Tool 调用和判断逻辑，Agent 可通过 Skill 快速完成常见任务。

#### Skill 定义

```typescript
type Skill = {
  name: string;                    // 唯一标识
  displayName: string;             // 用户可读名称
  description: string;             // 能力描述
  category: SkillCategory;

  parameters: JSONSchema;          // 参数 Schema
  returns: JSONSchema;             // 返回值 Schema

  preconditions?: string[];        // 前置条件
  postconditions?: string[];       // 后置条件

  // 两种定义方式：组合式（步骤序列）或编程式（handler）
  steps?: SkillStep[];
  handler?: (args: unknown, ctx: SkillContext) => Promise<SkillResult>;

  metadata: {
    version: string;
    requires?: string[];           // 依赖的其他 Skill
    cost?: "cheap" | "medium" | "expensive";
  };
};

type SkillStep = {
  type: "tool_call" | "skill_call" | "condition" | "loop" | "parallel";
  payload: unknown;
};

type SkillCategory =
  | "code"       // 重构、格式化、代码生成
  | "test"       // 生成测试、运行测试
  | "review"     // 代码审查、安全检查
  | "deploy"     // 构建、部署、回滚
  | "data"       // 迁移、转换、分析
  | "utility"    // git 操作、文件操作
  | "research";  // 技术调研、文档查找
```

#### Skill 示例

```typescript
const SKILL_writeTests: Skill = {
  name: "write-tests",
  displayName: "生成测试",
  description: "为指定代码生成单元测试，含边界用例",
  category: "test",
  parameters: {
    type: "object",
    properties: {
      target: { type: "string" },
      framework: { enum: ["vitest", "jest"] }
    }
  },
  metadata: { version: "1.0.0", cost: "medium" },
};
```

#### Skill 注册与发现

```typescript
type SkillRegistry = {
  register(skill: Skill): void;
  unregister(name: string): void;
  get(name: string): Skill | undefined;
  search(query: string): Skill[];
  listByCategory(cat: SkillCategory): Skill[];
  saveUserSkill(skill: Skill): Promise<void>;   // 持久化用户自定义 Skill
  loadUserSkills(): Promise<Skill[]>;
};
```

- **内置 Skill**：随 WhaleCode 发布，开箱即用
- **用户自定义**：通过 DSL 或编程方式定义，持久化到项目 `.whalecode/skills/`
- **社区共享**：通过 MCP 或 npm 包导入

### 8.3 Tools — 通用工具系统

工具是 Agent 执行的最小原子操作单元。前文各角色章节已涉及工具权限，这里做全局定义。

#### 工具接口

```typescript
interface Tool {
  name: string;
  description: string;
  parameters: JSONSchema;

  execute(args: unknown, ctx: ToolContext): Promise<ToolResult>;

  // 元数据
  parallelSafe: boolean;       // 能否与其他工具并行
  rateLimit?: RateLimit;
  timeout?: number;

  // 安全检查
  permissions?: ToolPermission[];
  validateInput?(args: unknown): ValidationResult;
}
```

#### 完整工具分类

| 类别 | 工具 | 并行安全 | 说明 |
|------|------|---------|------|
| **文件** | Read, Write, Edit, Glob, Ls | 读 ✅ 写 ❌ | 文件系统操作 |
| **Shell** | Bash, RunScript | ❌ | 命令执行 |
| **搜索** | Grep, FileSearch, TextSearch | ✅ | 代码搜索 |
| **代码** | Diff, Format, Lint | ✅ | 代码分析 |
| **网络** | WebFetch, WebSearch | ✅ | 外部信息获取 |
| **Git** | GitStatus, GitDiff, GitLog | ❌ | 版本控制 |
| **MCP** | McpCallTool, McpReadResource | 依赖实现 | MCP 协议桥接 |
| **AI** | AskUser, RaiseConcern | ✅ | 用户/Viewer 交互 |

#### 第三方工具适配

```typescript
// 任何实现 Tool 接口的模块都可注册
const registry = new ToolRegistry();

// 内置工具
registry.register(new ReadTool());
registry.register(new BashTool());

// 第三方 npm 包
import { MyCustomTool } from "my-whalecode-tools";
registry.register(new MyCustomTool());

// MCP 发现工具自动注册
mcpClient.onToolDiscovered((tool) => {
  registry.register(new McpBridgeTool(tool));
});
```

### 8.4 MCP — Model Context Protocol 支持

MCP 是连接 WhaleCode 与外部世界的标准化协议。通过 MCP，WhaleCode 可以接入任何实现了 MCP 协议的工具、数据源和服务。

#### 架构

```
┌───────────────────────────────────────────┐
│              WhaleCode                     │
│  ┌───────────────────────────────────┐    │
│  │        MCP Client Manager         │    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────┐ │    │
│  │  │ MCP      │ │ MCP      │ │ ... │ │    │
│  │  │ Client 1 │ │ Client 2 │ │     │ │    │
│  │  └────┬────┘ └────┬────┘ └─────┘ │    │
│  └───────┼───────────┼──────────────┘    │
└──────────┼───────────┼───────────────────┘
           │           │
      ┌────▼───┐  ┌───▼──────┐
      │ MCP    │  │ MCP      │
      │Server A│  │Server B  │
      │(Files) │  │(Database)│
      └────────┘  └──────────┘
```

#### MCP 能力模型

```typescript
// MCP Server 暴露的能力
type McpCapabilities = {
  tools: McpTool[];           // 可调用工具
  resources: McpResource[];   // 可读取资源
  prompts: McpPrompt[];       // 提示模板
};

// 自动适配为 WhaleCode Tool
class McpBridgeTool implements Tool {
  constructor(private mcpTool: McpTool, private client: McpClient) {}

  get name() { return `mcp_${this.mcpTool.name}`; }
  get description() { return this.mcpTool.description; }
  get parameters() { return this.mcpTool.inputSchema; }
  get parallelSafe() { return true; }

  async execute(args: unknown, ctx: ToolContext): Promise<ToolResult> {
    await this.checkPermission(ctx);   // 权限检查
    return this.client.callTool(this.mcpTool.name, args);
  }
}
```

#### MCP 配置

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"],
      "transport": "stdio",
      "permissions": ["read", "write"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "transport": "stdio",
      "permissions": ["read"]
    }
  }
}
```

#### MCP 与约束层的集成

```typescript
// MCP 工具同样受 Phase Tool Permissions (Layer 2) 约束
function isMcpToolAllowed(tool: McpBridgeTool, phase: string, role: Role): boolean {
  const phasePerms = PHASE_TOOL_PERMISSIONS[phase]?.[role] ?? [];
  const mapped = mapMcpToToolCategory(tool);
  return phasePerms.includes(mapped);
}
```

### 8.5 与原生工作流的关系

Skills / Tools / MCP 不取代 Create 和 Debug，而是作为能力基础：

```
Create / Debug 工作流
      │
      ├── 使用 Skill 封装重复步骤
      │   └── "写测试" → write-tests Skill → 多 Tool 组合调用
      │
      ├── 调用 Tool 执行原子操作
      │   └── Implementer 用 Write/Edit 写代码
      │
      └── 通过 MCP 接入外部能力
          └── Searcher 通过 MCP-GitHub 搜索参考实现
```

**核心原则**：Create 和 Debug 是"做什么"，Skills/Tools/MCP 是"用什么做"。前者是差异化核心，后者是通用生态基础，互补不冲突。

### 8.6 技能自进化系统 (Self-Evolving Skills)

行业现状：Skill 导入后就是静态资源，不会随使用优化迭代。WhaleCode 的 Skill 是**活的**——从创建起就有专门的进化回路持续监控、分析、优化。

#### 进化回路

```
Skill 创建/导入
     │
     ▼
┌─────────────────────────────────────────────────┐
│ 使用阶段 (持续)                                     │
│                                                   │
│  Agent 调用 Skill ──→ 使用数据采集 ──→ 写入 Skill   │
│  用户给出反馈          状态数据库                   │
│                                                    │
│  数据采集维度:                                       │
│  • 调用成功率 / 失败率                              │
│  • Agent 满意度评分 (自动)                          │
│  • 用户显式反馈 (赞/踩/评论)                        │
│  • 执行耗时分布                                     │
│  • 失败时的上下文 (项目类型、错误信息)                │
│  • 调用参数分布                                     │
└─────────────────────┬───────────────────────────────┘
                      │ 达到触发条件
                      ▼
┌─────────────────────────────────────────────────┐
│ 进化分析                                           │
│  Skill Evolution Agent (V4-Pro)                    │
│                                                    │
│  输入: 使用数据 + 反馈 + Skill 当前定义             │
│  输出: Evolution Proposal(s)                        │
│                                                    │
│  Proposal 包含:                                     │
│  • 问题描述: "write-tests 在 GraphQL 项目中          │
│    生成的测试覆盖不足，仅 40% 的分支覆盖率"           │
│  • 改进方案: 具体修改建议 (加什么参数/改什么步骤)    │
│  • 预期收益: "预计分支覆盖率提升至 70%+"            │
│  • 风险等级: low / medium / high                    │
└─────────────────────┬───────────────────────────────┘
                      │ Proposal 提交
                      ▼
┌─────────────────────────────────────────────────┐
│ 审查与发布                                         │
│  Reviewer (V4-Pro) 审查 Proposal                   │
│  ┌─ 通过 → 创建新版本 → 更新 Skill 注册表          │
│  ├─ 需修改 → 打回 Evolution Agent 调整              │
│  └─ 拒绝   → 记录原因，关闭提案                     │
│                                                    │
│  发布策略:                                          │
│  • patch: 参数调优、bug 修复 (自动发布)              │
│  • minor: 步骤调整、新增参数 (需 Review)             │
│  • major: 行为变更、破坏兼容 (需用户确认)            │
└─────────────────────────────────────────────────────┘
```

#### 数据模型

```typescript
// Skill 版本化定义
type VersionedSkill = Skill & {
  versions: SkillVersion[];
  evolutionMeta: {
    createdAt: string;
    totalCalls: number;
    successRate: number;
    avgDuration: number;
    lastEvolved: string;
    currentVersion: string;
  };
};

type SkillVersion = {
  version: string;               // semver: 1.0.0
  skill: Skill;                  // 该版本的 Skill 定义
  changelog: string;
  releasedAt: string;
  status: "active" | "canary" | "rollback" | "deprecated";
};

// 使用记录
type SkillInvocation = {
  skillName: string;
  version: string;
  caller: AgentId;
  args: unknown;
  result: "success" | "failure" | "partial";
  duration: number;
  agentFeedback?: { score: 1-5; comment?: string };   // Agent 自评
  userFeedback?: { thumbs: "up" | "down"; comment?: string };  // 用户评价
  context: {
    projectType: string;
    taskType: string;
    language?: string;
    framework?: string;
  };
  errors?: string[];
  timestamp: string;
};

// 进化提案
type EvolutionProposal = {
  id: string;
  skillName: string;
  currentVersion: string;
  
  analysis: {
    problem: string;             // 数据分析发现的问题
    severity: "critical" | "minor" | "enhancement";
    evidence: string[];          // 支撑证据（调用记录 ID 列表）
  };
  
  changes: {
    type: "parameter" | "step" | "prompt" | "condition" | "metadata";
    description: string;
    oldValue?: unknown;
    newValue: unknown;
  }[];
  
  expectedImpact: string;
  riskLevel: "low" | "medium" | "high";
  
  suggestedVersion: string;      // semver bump
};
```

#### 触发条件

```typescript
const EVOLUTION_TRIGGERS = {
  // 1. 批量分析 — 每积累 20 次调用或每 7 天触发一次
  batchAnalysis: { minCalls: 20, intervalDays: 7 },

  // 2. 阈值告警 — 成功率低于 80% 立即触发
  thresholdBreach: { successRate: 0.8, windowSize: 10 },

  // 3. 用户主动触发 — 用户对 Skill 点踩或执行 /skill-evolve
  userRequest: true,

  // 4. 模式发现 — 发现新的框架/语言版本时主动适配
  patternDiscovery: true,
};
```

#### 版本管理与回滚

```typescript
type SkillVersionManager = {
  // 版本发布
  publish(skillName: string, version: SkillVersion): Promise<void>;

  // 金丝雀发布 — 新版本仅对 10% 调用生效
  canaryDeploy(skillName: string, version: SkillVersion, percentage: number): Promise<void>;

  // 回滚 — 回退到指定版本
  rollback(skillName: string, toVersion: string): Promise<void>;

  // 版本对比 — 展示两个版本的 diff
  diff(skillName: string, v1: string, v2: string): Promise<SkillDiff>;

  // 调用路由 — 按版本策略分发调用
  resolveVersion(skillName: string, context: InvocationContext): string;
};
```

#### 与现有架构的集成

| 组件 | 角色 |
|------|------|
| **Skill Evolution Agent** | 新角色 (V4-Pro)，专门分析使用数据、生成改进提案。与 Viewer 类似是常驻角色，但不做对抗性批判，做数据分析驱动的优化 |
| **Viewer** | 审查 Evolution Proposal 的质量——"这个改进建议的数据支撑够吗？会不会引入新问题？" |
| **Reviewer** | 在 Proposal 发布前做代码级审查，确保新版本的 Skill 定义正确 |
| **Supervisor** | 调度进化分析任务（避开高峰期），管理版本发布策略 |
| **Message Bus** | 新增 `evolution_proposal`、`version_published`、`rollback` 消息类型 |

```typescript
// 新增消息类型
type EvolutionMessage =
  | { type: "evolution_proposal";  proposal: EvolutionProposal }
  | { type: "version_published";   skillName: string; version: string; changes: string }
  | { type: "rollback_executed";   skillName: string; from: string; to: string; reason: string }
  | { type: "skill_feedback";      invocation: SkillInvocation };  // 实时反馈流
```

#### 对比：静态 Skill vs 自进化 Skill

| 维度 | 静态 Skill | 自进化 Skill |
|------|-----------|-------------|
| 导入后 | 永远不变 | 持续监控使用数据 |
| 适配性 | 用户手动修改 | 自动发现不足并优化 |
| 版本管理 | 无 | semver + 金丝雀 + 回滚 |
| 跨项目 | 表现不稳定 | 随各项目使用数据积累逐步泛化 |
| 社区贡献 | 重新发布新版本 | 进化提案可导出为社区 PR |

---

## 九、通信协议：Agent Message Bus

所有 Agent 通过统一消息总线通信，不直接调用。

### 消息类型

```typescript
type Envelope = {
  from: AgentId;
  to: AgentId | Broadcast;
  traceId: string;           // 追踪链路
  payload: MessagePayload;
};

type MessagePayload =
  | { type: "task_assign";         task: Task; plan: PlanRef }
  | { type: "task_result";         taskId: string; status: ResultStatus; artifacts: Artifact[] }
  | { type: "query";               question: string; context: ContextSelector }
  | { type: "answer";              queryId: string; answer: ContextBlock[] }
  | { type: "review_request";      taskId: string; code: FileRef[] }
  | { type: "review_result";       taskId: string; issues: Issue[] }
  | { type: "coordinate";          topic: string; body: string }
  | { type: "status_update";       state: AgentState; progress?: string }
  | { type: "error";               taskId: string; error: ErrorInfo; recoverable: boolean }
  // Debug 证据链专用消息
  | { type: "hypothesis_propose";  hypotheses: Hypothesis[] }
  | { type: "evidence_plan";       taskId: string; plan: EvidencePlan }
  | { type: "evidence_result";     taskId: string; evidence: Evidence }
  | { type: "converge_report";     session: DebugSessionStatus }
  // Viewer 对抗性批判消息
  | { type: "concern_raised";      concern: Concern }                   // Viewer → Bus
  | { type: "concern_ack";         concernId: string; action: "accept" | "reject" | "address" }  // Agent/Supervisor → Bus
  | { type: "viewer_request";      target: string; reason: string };    // Supervisor → Viewer
```

### 通信模式

| 模式 | 用途 | 示例 |
|------|------|------|
| **Unicast** | Supervisor → 指定 Agent | 分配任务 |
| **Broadcast** | Supervisor → 全体 | 终止、全局通知 |
| **Peer-to-peer** | Agent → Agent | Implementer 间协调文件锁 |
| **Request-Reply** | 查询 → 应答 | 向 Searcher 请求上下文 |

每条消息携带 traceId，用于调试审计、性能分析、死锁检测。

---

## 十、任务模型：DAG + Phase 序列

### 任务 DAG

```typescript
type Plan = {
  tasks: Task[];
  dependencies: Dependency[]; // A → B 表示 B 依赖 A
};

type Task = {
  id: TaskId;
  description: string;
  context: ContextRef[];
  acceptanceCriteria: string;
  agentRole: "architect" | "implementer" | "search" | "review" | "debug";
  modelHint: "flash" | "pro";
};

type Dependency = {
  from: TaskId;
  to: TaskId;
};
```

DAG 特性:
- **无环** — Supervisor 在调度前做环检测
- **并行调度** — 同一层级的无依赖任务并行分配给多个 Implementer
- **关键路径感知** — 最长路径上的任务优先调度
- **动态调整** — Architect 可根据阶段性结果调整后续任务

### Phase 作为执行单元

工作流由 Phase 序列组成，每个 Phase 内部可以包含多个并行 Agent：

```
Phase 序列 (以 Create 为例):
  [Phase1: Architect] → [Phase2: Scaffolding×N] → [Phase3: Searcher×N]
    → [Phase4: Implementer×N] → [Phase5: Reviewer] → [Phase6: Architect 确认]

Phase 内:
  Supervisor 从 DAG 中取出当前可执行的任务 → 分配给空闲 Agent
  Agent 执行 → 报告结果 → Supervisor 更新 DAG 状态
  当该 Phase 的所有任务完成 → 进入下一个 Phase
```

---

## 十一、调度模型

### 协作式调度

不抢占，靠协议。Supervisor 事件驱动管理：

```
[Supervisor]                       [Agent]
     │                                  │
     │──── task_assign ────────────────►│
     │                                  │──── 开始执行
     │◄──── status_update(busy) ────────│
     │                                  │──── 子任务完成/出错/卡住
     │◄──── task_result / error ────────│
     │                                  │
     │  判断结果:
     │  ├─ 成功 → 调度下一个任务
     │  ├─ 可重试 → 重新分配
     │  ├─ 不可恢复 → 拆解 / 升级到 Architect
     │  └─ 阻塞等待 → 标记 blocked
```

### 调度策略

| 策略 | 适用场景 | 行为 |
|------|---------|------|
| **greedy** | 简单多任务 | 所有可执行任务立即分配 |
| **critical-path** | 复杂项目 | 优先调度关键路径任务 |
| **round-robin** | 同等优先级 | 循环分配 |
| **resource-aware** | 资源受限 | 按可用 token/context 预算分配 |

Create 模式偏向 **greedy + critical-path**，Debug 模式偏向**单线程追踪**。

### 失败恢复

| 失败类型 | 检测方式 | 恢复策略 |
|---------|---------|---------|
| Agent 超时 | Supervisor 心跳 | 重试 (最多 3 次) + 换 Agent |
| 工具执行失败 | Agent 报告 error | 重试 → 升级到 Architect |
| 文件冲突 | Git merge 冲突 | Reviewer 裁决 |
| LLM 调用失败 | API error | 指数退避 + 切换模型 |
| 上下文溢出 | Token 预算超限 | 压缩 + 持久化大结果 |
| 死锁 | 消息 DAG 环检测 | Supervisor 中断 + 重规划 |

---

## 十二、工作空间隔离

```
实际仓库: /project/src/
     │
     ├── Executor A: /wks/a/    仓库只读挂载 + 独立临时区
     ├── Executor B: /wks/b/    仓库只读挂载 + 独立临时区
     └── Executor C: /wks/c/    仓库只读挂载 + 独立临时区
```

- **共享层**: 仓库文件只读挂载，所有 Agent 看到相同基线
- **工作层**: 每个 Agent 有自己的临时工作目录，互不可见
- **输出层**: 完成后 artifacts 提交到共享区，Supervisor 协调合并

上下文管理见第十三节。

---

## 十三、上下文管理

上下文管理是 coding agent 的生死线。行业内有两条路线：**Claude Code 的内联分级压缩**（Truncation → Window Sliding → Summarization → Emergency Clean，全部在主 Agent 循环内完成）和 **Codex CLI 的独立轮次压缩**（压缩是独立的 API 调用，生成 LLM 交接摘要，然后替换历史）。WhaleCode 选择 Codex CLI 路线——更简洁、更可预测，且天然适合多 Agent 场景。

### 13.1 架构概览

Codex CLI 的核心洞察：**上下文压缩不应该在主 Agent 循环内发生**。压缩是一个独立的 API 调用（separate turn），生成结构化的 LLM-to-LLM 交接摘要，然后用压缩后的历史**替换**原历史。

```
┌────────────────────────────────────────────────────────────┐
│                    ContextManager (每个 Agent 一个)          │
│                                                             │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Layers (分层组织)                                   │    │
│  │                                                     │    │
│  │  Layer 0: System        ─ 系统提示词 + 工具定义     │    │
│  │  Layer 1: Phase         ─ 当前阶段上下文             │    │
│  │  Layer 2: Task          ─ 当前任务相关文件/数据      │    │
│  │  Layer 3: Conversation  ─ 对话历史 (可变长)          │    │
│  │  Layer 4: Artifacts     ─ 跨阶段持久产物（设计文档、  │    │
│  │                          审查报告、证据链等）        │    │
│  └────────────────────────────────────────────────────┘    │
│                                                             │
│  数据流:                                                    │
│  for_prompt() → 序列化 Layer 0-4 → Token 计数              │
│       → 未超限 → 发送到 LLM                                 │
│       → 接近窗口上限 → 触发独立轮次压缩（separate turn）     │
│       → 超限 → remove_first_item() 紧急回收                │
└────────────────────────────────────────────────────────────┘
```

每个 Agent 拥有独立的 ContextManager，由 Supervisor 统一协调上下文预算。上下文不是"尽可能塞"，而是**按层分级管理，由独立压缩轮次和片段注入系统动态调控**。

### 13.2 上下文窗口模型

不同于 Claude Code 的多级百分比阈值（60% truncation / 80% compaction / 95% emergency），Codex CLI 采用更简洁的模型：**一个预算上限 + 两种回收机制**。

```typescript
type ContextWindowConfig = {
  // 模型原始上下文窗口 (DeepSeek V4 = 1,048,576)
  modelWindow: number;

  // 有效窗口 — 保留头部给 system prompt 和输出头寸
  // DeepSeek V4: 1,048,576 * 0.90 = ~943,718 tokens
  effectiveWindow: number;

  // 压缩触发阈值 — 达到此值触发独立轮次压缩
  // DeepSeek V4: ~755K tokens（大部分短会话零压缩）
  compactionTrigger: number;

  // 用户消息保留预算 — 压缩时保留最近多少 tokens 的用户消息
  // 参考 Codex CLI: COMPACT_USER_MESSAGE_MAX_TOKENS = 20_000
  userMessageBudget: number;  // 20_000

  // 每 token 估算字节数 (不依赖 tokenizer)
  bytesPerToken: number;      // 4
};
```

核心区别：

| | Claude Code 路线 | Codex CLI 路线（WhaleCode 采用） |
|---|---|---|
| 压缩位置 | 主 Agent 循环内，内联执行 | 独立 API 调用，separate turn |
| 压缩层级 | 4 级管道 (L1-L4) | 1 种压缩 + 1 种紧急回收 |
| 紧急回收 | 丢弃所有非关键上下文，阻断 Agent | `remove_first_item()` 移除最旧消息 |
| 压缩产物 | 内联总结注入对话 | LLM-to-LLM 交接摘要替换历史 |
| 上下文注入 | 隐性（靠 LLM 自行判断） | 显性片段注入（类型化 marker） |

利用 DeepSeek 大窗口优势：WhaleCode 的压缩触发阈值是 ~755K tokens，远高于 Claude（~180K）。大部分短会话根本不需要压缩。

### 13.3 核心机制：独立轮次压缩（Separate-Turn Compaction）

参考 Codex CLI 的 `run_compact_task_inner_impl()`：压缩不作为 Agent 主循环内的一个步骤，而是一个**独立的 API 调用轮次**。这保证了压缩质量——压缩 LLM 可以专注于总结任务，不受当前 Agent 任务的上下文污染。

#### 触发条件

```typescript
type CompactionTrigger =
  | { type: "auto"; reason: "token_budget_nearing" }   // Token 接近窗口上限
  | { type: "manual"; reason: "user_requested" }        // 用户主动触发 /compact
  | { type: "phase_boundary"; reason: "phase_changed" } // Phase 转换（最佳时机）
  | { type: "preemptive"; reason: "supervisor_predicted" }; // Supervisor 预测到即将超限
```

#### 压缩流程

```
Agent 主循环运行中
  │
  ├─ Token 使用率达到 compactionTrigger (~755K)
  │  (或 Phase 转换、用户手动触发)
  │
  ▼
┌────────────────────────────────────────────────────────────┐
│ Step 1: 克隆当前历史（不影响 Agent 正在使用的上下文）        │
│                                                             │
│   const history = sess.cloneHistory();                      │
│   // 参考 Codex CLI: sess.clone_history()                   │
└────────────────────────┬───────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│ Step 2: 分割历史 — 保留最近用户消息                          │
│                                                             │
│   const USER_MSG_BUDGET = 20_000;  // tokens                │
│   const { recentUserMsgs, oldHistory } = splitHistory(      │
│     history,                                                │
│     USER_MSG_BUDGET                                         │
│   );                                                        │
│   // 参考 Codex CLI: COMPACT_USER_MESSAGE_MAX_TOKENS        │
│   // 保留最近用户消息（最多 20K tokens），其余全部压缩       │
└────────────────────────┬───────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│ Step 3: 独立轮次 API 调用 — 生成 LLM 交接摘要                │
│                                                             │
│   这是一个全新的 API 调用，不属于 Agent 主循环。              │
│   压缩模型: V4-Flash（日常）/ V4-Pro（关键路径）             │
│                                                             │
│   const summary = await compactAPI.call({                   │
│     system: COMPACT_SYSTEM_PROMPT,                          │
│     messages: oldHistory,                                   │
│     maxTokens: 4_000,                                       │
│   });                                                       │
│                                                             │
│   参考 Codex CLI 的 compact prompt:                          │
│   "You are performing a CONTEXT CHECKPOINT COMPACTION.      │
│    Create a handoff summary for another LLM that will       │
│    resume the task."                                        │
└────────────────────────┬───────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│ Step 4: 构建压缩历史                                        │
│                                                             │
│   const compactedHistory = buildCompactedHistory({          │
│     summary,             // LLM 交接摘要                    │
│     recentUserMsgs,      // 保留的最近用户消息              │
│     artifacts,           // 持久 artifacts 引用            │
│   });                                                      │
│                                                             │
│   // 参考 Codex CLI: build_compacted_history()              │
│   // 结构: [摘要(作为最后一条消息), ...保留的用户消息]       │
│   // 摘要前缀: "Another language model started to solve     │
│   //   this problem... use this to build on the work        │
│   //   that has already been done"                          │
└────────────────────────┬───────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────────┐
│ Step 5: 替换历史 + 重新注入初始上下文                        │
│                                                             │
│   sess.replaceCompactedHistory(compactedHistory);           │
│   // 参考 Codex CLI: sess.replace_compacted_history()       │
│                                                             │
│   // 重新注入初始上下文到正确位置                             │
│   insertInitialContextBeforeLastRealUserOrSummary(           │
│     compactedHistory,                                       │
│     initialContext                                          │
│   );                                                        │
│   // 参考 Codex CLI: insert_initial_context_before_         │
│   //   last_real_user_or_summary()                          │
└────────────────────────┬───────────────────────────────────┘
                         │
                         ▼
Agent 主循环继续，使用压缩后的历史
```

#### 压缩摘要格式（LLM 交接）

参考 Codex CLI 的 compact prompt 和 summary_prefix，WhaleCode 的压缩摘要格式化为**面向下一个 LLM 的交接文档**：

```
<context_checkpoint version="1" role="architect" phase="design">
<handoff_note>
  另一个 LLM 已经开始解决这个问题。以下是当前进度、关键决策和待办事项。
  请基于已完成的工作继续。
</handoff_note>
<summary>
  已完成用户管理系统的方案设计，输出包含 5 个任务、3 个参考引用。
  Architect 选择了 RBAC 权限模型，参考了 axum tower middleware 和 CASL 的策略表达。
  需要 Implementer 实现 User/ Role/ Permission 三个核心模块。
</summary>
<key_decisions>
  - 权限模型: RBAC (参考: CASL docs, section 3.2)
  - 认证方案: JWT + refresh token (参考: axum/tower middleware pattern)
  - 数据库: Prisma + PostgreSQL，已有 schema
</key_decisions>
<open_items>
  - Review 时需要确认 RBAC 的角色继承实现是否完整
  - 前端权限组件的参考实现待补充
</open_items>
<artifacts>
  - design_doc.md (ref: artifact_0032)
  - task_dag.json (ref: artifact_0033)
</artifacts>
</context_checkpoint>
```

**关键设计决策**：摘要使用 `<handoff_note>` 明确告知后续 LLM 这是交接场景。这比 Claude Code 的隐式总结注入更透明——后续 LLM 知道自己在"接棒"，会主动查阅摘要中的决策和待办事项。

#### 压缩分析追踪

参考 Codex CLI 的 `CompactionAnalyticsAttempt`，每次压缩都记录详细的分析数据：

```typescript
type CompactionAnalytics = {
  id: string;
  agentId: string;
  trigger: "auto" | "manual" | "phase_boundary" | "preemptive";
  phase: string;                    // 压缩发生在哪个 Phase
  beforeTokens: number;
  afterTokens: number;
  compressionRatio: number;         // 压缩比
  duration: number;                 // 压缩耗时 (ms)
  model: "v4-flash" | "v4-pro";    // 使用的压缩模型
  status: "success" | "failed" | "partial";
  errorType?: "context_window_exceeded" | "api_error" | "timeout";
  recoveryAction?: "remove_first_item" | "retry" | "user_notified";
};
```

### 13.4 工具输出截断（始终开启）

工具输出在写入 ContextManager 前始终做 middle-cut 截断，零额外成本：

```typescript
type TruncationPolicy = {
  strategy: "middle-cut";
  maxTokens: number;
  budgetRatio: number;    // 序列化开销余量, default 1.2
  showLines: boolean;
};

function truncateOutput(text: string, policy: TruncationPolicy): string {
  const budget = Math.floor(policy.maxTokens / policy.budgetRatio);
  if (approxTokenCount(text) <= budget) return text;

  const halfBudget = Math.floor(budget / 2);
  const prefix = takeChars(text, halfBudget * 4);
  const suffix = lastChars(text, halfBudget * 4);

  const truncated = approxTokenCount(text) - budget;
  return `${prefix}\n...${truncated} tokens truncated...\n${suffix}`;
}
```

| 工具 | 默认截断上限 | 说明 |
|------|------------|------|
| Bash stdout | 10,000 tokens | 命令输出 |
| Read 文件内容 | 8,000 tokens | 大文件自动截断 |
| WebFetch | 6,000 tokens | 网页内容 |
| Grep 结果 | 4,000 tokens | 搜索匹配行 |
| Glob 结果 | 2,000 tokens | 文件列表 |

### 13.5 紧急回收：remove_first_item()

当上下文窗口被超出时（如 API 返回 `ContextWindowExceeded` 错误），WhaleCode 采用 Codex CLI 的简洁方案——直接移除最旧的历史条目：

```typescript
function emergencyReclaim(history: ConversationItem[]): ConversationItem[] {
  // 参考 Codex CLI: history.remove_first_item()
  // 移除最旧的非系统消息，直到回到窗口限制内
  while (estimateTokens(history) > effectiveWindow && history.length > 0) {
    const removed = history.shift();
    if (removed?.role === "user") {
      // 用户消息被移除前，对其上文的 assistant 消息做微型摘要
    }
  }
  return history;
}
```

对比 Claude Code 的 L4 Emergency Clean（丢弃所有非关键上下文、阻断 Agent、强制新 Phase），Codex CLI 的 `remove_first_item()` 更轻量、更可预测——它只是丢弃最旧的消息，不会突然打断 Agent 的工作流。

### 13.6 上下文片段注入（Context Fragment Injection）

参考 Codex CLI 的 `ContextualUserFragment` trait 和 20+ 种片段类型，WhaleCode 采用**类型化片段注入**机制来管理上下文。这比 Claude Code 的隐性上下文组织更结构化——每个上下文片段都有明确的类型、位置和生命周期。

```typescript
// 参考 Codex CLI: ContextualUserFragment trait
interface ContextualFragment {
  type: FragmentType;
  marker: {
    start: string;  // START_MARKER
    end: string;    // END_MARKER
  };
  content: string;
  priority: number;        // 注入优先级
  position: "before_tasks" | "after_system" | "before_recent" | "append";
  ttl: number;             // 片段有效期（轮次）
}

type FragmentType =
  | "project_structure"    // 项目结构概览
  | "phase_context"        // 当前阶段说明
  | "task_brief"           // 当前任务摘要
  | "artifact_ref"         // 产物引用
  | "concern_active"       // 活跃的 Viewer Concern
  | "model_instructions"   // 模型特定指令
  | "tool_output_truncated"// 被截断的工具输出标记
  | "compaction_checkpoint"// 压缩检查点
  | "reference_citation"   // 参考来源引用
  // ...可扩展至 20+ 种类型
```

**注入位置策略**：

```
完整上下文结构:
  ┌─────────────────────────────────────┐
  │ System Prompt                       │  ← Layer 0
  ├─────────────────────────────────────┤
  │ [Fragment: project_structure]       │  ← 注入: after_system
  │ [Fragment: model_instructions]       │
  ├─────────────────────────────────────┤
  │ Phase Context                       │  ← Layer 1
  │ [Fragment: phase_context]            │  ← 注入: before_tasks
  ├─────────────────────────────────────┤
  │ Task Context                        │  ← Layer 2
  │ [Fragment: task_brief]              │
  │ [Fragment: artifact_ref]            │
  │ [Fragment: reference_citation]      │
  ├─────────────────────────────────────┤
  │ Conversation History                │  ← Layer 3
  │ ... (最近用户消息保留区)             │
  │ [Fragment: compaction_checkpoint]   │  ← 注入: before_recent
  │ [Fragment: concern_active]          │
  ├─────────────────────────────────────┤
  │ Output Headroom (reserved)          │  ← 保留给模型输出
  └─────────────────────────────────────┘
```

```typescript
class FragmentManager {
  private fragments: Map<string, ContextualFragment> = new Map();

  register(fragment: ContextualFragment): void {
    this.fragments.set(fragment.type, fragment);
  }

  inject(context: PromptContext): PromptContext {
    const result = { ...context };
    for (const frag of this.activeFragments()) {
      result = this.injectAt(frag, result, frag.position);
    }
    return result;
  }

  tick(): void {
    for (const [key, frag] of this.fragments) {
      frag.ttl -= 1;
      if (frag.ttl <= 0) this.fragments.delete(key);
    }
  }
}
```

### 13.7 多 Agent 上下文预算管理

WhaleCode 的上下文管理不仅是单 Agent 的，还是全局的。Supervisor 统一监控所有 Agent 的上下文使用率，在必要时触发独立轮次压缩或 `remove_first_item()`：

```
Supervisor Context Budget Manager
     │
     ├── Agent A (Architect)  ── 754K / 943K  ● 正常
     ├── Agent B (Impl A)     ── 823K / 943K  ◉ 接近阈值 → 预触发压缩
     ├── Agent C (Impl B)     ── 450K / 943K  ○ 充裕
     ├── Agent D (Searcher)   ── 120K / 943K  ○ 充裕
     └── Agent E (Viewer)     ── 920K / 943K  ● 超限 → remove_first_item()
                                                          │
                                                          ▼
                                                  触发 Agent B 的独立轮次压缩
                                                  触发 Agent E 的紧急回收
```

**上下文预算路由**：

```typescript
type ContextBudget = {
  totalPool: number;            // 总上下文池 (所有 Agent 合计)
  perAgentLimit: number;        // 单个 Agent 上限
  reservedForOutput: number;    // 为模型输出保留的头寸
};

class ContextBudgetManager {
  private agents: Map<AgentId, ContextManager>;

  // 预压缩调度 — 在 Agent 实际超限前主动触发独立轮次压缩
  async preemptiveCompact(): Promise<void> {
    for (const [id, cm] of this.agents) {
      const ratio = cm.currentTokenUsage / cm.effectiveWindow;
      if (ratio > 0.65 && !cm.isCompactInProgress) {
        // 预测到即将超限，提前触发独立轮次压缩（separate turn）
        this.scheduleSeparateTurnCompact(id, cm.predictNextTurnGrowth());
      }
    }
  }

  // 上下文借贷 — 短时间内允许 Agent 超出预算
  borrow(agentId: AgentId, tokens: number): boolean {
    if (this.totalBorrowed + tokens < this.maxBorrowable) {
      this.totalBorrowed += tokens;
      return true;
    }
    return false;
  }
}
```

### 13.8 Agent 上下文隔离

借鉴 Codex CLI 的 `remove_first_item()` + Agent 消息总线设计：

```
Agent A 的上下文 (不可见 Agent B)
┌─────────────────────────────────────────┐
│ System: "你是 Architect，负责系统设计..."  │
│ Phase: 当前在 DESIGN 阶段                  │
│ Task: 用户管理系统设计                     │
│ History: [用户需求] [你的分析] [参考研究]  │
│ Artifacts: [设计文档] [任务 DAG]          │
└─────────────────────────────────────────┘

Agent B 的上下文 (不可见 Agent A)
┌─────────────────────────────────────────┐
│ System: "你是 Implementer，负责实现...    │
│ Phase: 当前在 SCAFFOLD 阶段              │
│ Task: 配置 logging 框架                   │
│ History: [任务分配] [你的配置过程]        │
│ Artifacts: [logger.ts]                  │
└─────────────────────────────────────────┘

共享上下文 (跨 Agent 只读)
┌─────────────────────────────────────────┐
│ 项目结构 (只读挂载)                      │
│ 全局 System Prompt 前缀                  │
│ (命中 DeepSeek 缓存，5x 便宜)            │
└─────────────────────────────────────────┘
```

Agent 间通过 Message Bus 传递引用，不复制上下文：
- `Implementer` 完成代码 → 发送 `task_result` 消息（含 artifact 引用）
- `Reviewer` 通过引用读取 artifact，不用加载 Implementer 的完整上下文
- `Architect` 的设计文档作为引用存入 Message Bus

### 13.9 DeepSeek V4 特有优化

| 特性 | 适配策略 |
|------|---------|
| **1M 上下文窗口** | 压缩触发阈值提升到 ~755K，大部分短作业零压缩 |
| **缓存定价优势** | Layer 0 (System Prompt) 跨 Agent 共享前缀，cache hit 比例最大化；具体折扣来自 provider pricing probe |
| **超长输出 384K** | 保留足够输出头寸（always reserve 50K+ for output） |
| **V4-Flash 低成本** | 日常独立轮次压缩、Scout、Analyst、Implementer 候选优先走 V4-Flash |
| **V4-Pro 高质量** | 关键路径（Architect/Debugger/Viewer）的压缩和上下文重建走 V4-Pro，保证质量 |

> **注意**：DeepSeek V4 的具体模型能力和价格仍以 provider capability probe 与官方 pricing source 为准。即使官方文档已列出 V4 Flash/Pro，也不能在 runtime 中写死 context、output、thinking、tool-call、并发和价格参数。

缓存优化示例：

```typescript
// 所有 Agent 的 System Prompt 共享稳定前缀 → 提升 DeepSeek cache hit 概率
const SYSTEM_PROMPT_PREFIX = `你是 WhaleCode 的 {role}。你的职责是 {description}。
工作流阶段: {phase}。项目类型: {projectType}。语言: {language}。`;

// 只有后半部分（角色名、阶段等）变化
// 前半部分在所有 Agent 间相同 → cache hit
```

### 13.10 与约束层的关系

上下文管理本身也是约束层的一部分——

```
Phase Machine     ← 控制何时可以清理上下文（Phase 边界是最佳时机）
Tool Permissions ← 控制工具输出大小上限
DAG Validation   ← 任务数量上限间接控制上下文总量
Artifact Contracts ← 产物最小/最大大小约束
Context Manager  ← ← 本节描述的完整上下文管理体系
System Prompt    ← 被压缩时 checkpoint 保留关键信息
```

最佳压缩时机：
- **Phase 边界**：旧阶段的上下文不再需要，是独立轮次压缩的最佳窗口
- **Agent 空闲**：等待其他 Agent 完成时，预压缩自身上下文
- **Supervisor 调度间隙**：Supervisor 可在分配新任务前检查并触发全局压缩

---

## 十四、可观测性：可视化与监控

WhaleCode 不是黑盒。用户应当能实时看到系统内部——Agent 在做什么、消息如何流转、任务 DAG 的推进状态、Token 消耗了多少。可视化是 WhaleCode 透明性的基础保障，也是用户理解、调试、信任系统的窗口。

### 14.1 设计原则

| 原则 | 说明 |
|------|------|
| **实时但不阻塞** | 可视化是独立子系统，不影响核心 Agent 执行性能 |
| **分层展示** | 概览 → Agent 网络 → 消息流 → 细节日志，用户可逐层下钻 |
| **可回溯** | 会话历史可回放，支持事后分析 |
| **可交互** | 用户可暂停、聚焦特定 Agent、查看详情 |

### 14.2 数据采集层

可视化由 Message Bus 上的事件驱动，不额外侵入核心流程：

```typescript
// 可视化的数据来源 — 全部从现有消息总线监听
type ObservabilityEvent =
  // Agent 生命周期
  | { type: "agent_spawned";     agent: AgentInfo }
  | { type: "agent_state_change"; agentId: AgentId; from: AgentState; to: AgentState }
  | { type: "agent_completed";   agentId: AgentId; result: TaskResult }

  // 消息通信
  | { type: "message_sent";      envelope: Envelope }
  | { type: "message_delivered"; envelopeId: string; to: AgentId }

  // 任务 DAG
  | { type: "task_created";      task: Task }
  | { type: "task_assigned";     taskId: string; agentId: AgentId }
  | { type: "task_progress";     taskId: string; progress: number; detail?: string }
  | { type: "task_completed";    taskId: string; status: "success" | "failure" }

  // 工具调用
  | { type: "tool_call_start";   tool: string; args: unknown; agentId: AgentId; timestamp: number }
  | { type: "tool_call_end";     tool: string; agentId: AgentId; duration: number; success: boolean }

  // Token 用量
  | { type: "token_usage";       agentId: AgentId; model: string; input: number; output: number; cached: number }

  // Phase 转换
  | { type: "phase_transition";  from: string; to: string; workflow: "create" | "debug" }

  // Viewer Concern
  | { type: "concern_raised";    concern: Concern }
  | { type: "concern_resolved";  concernId: string };

// 事件采集器 — 作为 Message Bus 的监听者
class ObservabilityCollector {
  constructor(private bus: MessageBus) {
    // 订阅所有消息类型
    bus.subscribe("*", this.onEvent.bind(this));
  }

  private onEvent(envelope: Envelope) {
    // 转换为 ObservabilityEvent
    // 写入环形缓冲区（内存）+ 定期持久化
    this.buffer.push(this.transform(envelope));
    this.broadcast(envelope); // 推送至 WebSocket
  }
}
```

### 14.3 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                   WhaleCode Core                         │
│  ┌──────────┐  ┌────────────┐  ┌───────────────────┐   │
│  │ Supervisor│  │ Message Bus│  │ Agent Pool        │   │
│  └────┬─────┘  └─────┬──────┘  └────────┬──────────┘   │
│       │              │                  │              │
│       └──────────────┼──────────────────┘              │
│                      │                                  │
│              ┌───────▼──────────┐                       │
│              │ Observability    │  ← 事件监听，零侵入   │
│              │ Collector        │                       │
│              └───────┬──────────┘                       │
└──────────────────────┼──────────────────────────────────┘
                       │ WebSocket / SSE
                       ▼
┌─────────────────────────────────────────────────────────┐
│                Web 可视化服务 (独立进程)                   │
│  ┌────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │ Event Store │  │ State Manager│  │ WebSocket      │  │
│  │ (环形缓冲区) │  │ (当前快照)   │  │ Server         │  │
│  └────────────┘  └──────────────┘  └───────┬────────┘  │
│                                            │           │
│  ┌─────────────────────────────────────────▼────────┐  │
│  │               Web UI (React + Vite)               │  │
│  │  ┌───────────┐ ┌──────────┐ ┌────────────────┐   │  │
│  │  │ Agent     │ │ DAG      │ │ Stats Panel    │   │  │
│  │  │ 网络图    │ │ 进度视图  │ │ (Token/工具/耗时)│  │  │
│  │  └───────────┘ └──────────┘ └────────────────┘   │  │
│  │  ┌───────────┐ ┌────────────────────────┐        │  │
│  │  │ 消息流    │ │ 时间线 + 日志回放       │        │  │
│  │  └───────────┘ └────────────────────────┘        │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 14.4 可视化面板

#### 1. Agent 网络图 (核心视图)

实时动画展示 Agent 群体状态：

```
                        ┌──────────┐
           ┌────────────│ Viewer   │────────────┐
           │            │ ● 常驻    │            │
           │            └──────────┘            │
           │                                    │
     ┌─────▼──────┐                    ┌────────▼───────┐
     │ Architect  │ ◄─── design ───►  │   Searcher ×3  │
     │ ● Design   │                    │ ● Researching  │
     └────────────┘                    └────────────────┘
           │                                   │
           │ task_assign                       │ context
           ▼                                   ▼
     ┌─────────────────────────────────────────────┐
     │         Implementer Pool                     │
     │  ┌──────────┐ ┌──────────┐ ┌──────────┐    │
     │  │ Impl A   │ │ Impl B   │ │ Impl C   │    │
     │  │ ● 写Model│ │ ● API    │ │ ● 空闲   │    │
     │  └──────────┘ └──────────┘ └──────────┘    │
     └─────────────────────────────────────────────┘
```

每个 Agent 节点展示：
- **角色图标** + 名称
- **当前状态**：颜色编码（Idle=灰色, Busy=绿色, Blocked=橙色, Done=蓝色）
- **当前任务描述**（滚动摘要）
- **活跃度指示**：脉冲动画表示正在调用 LLM 或执行工具
- **连线动画**：消息在 Agent 间流动的光点动画

#### 2. 任务 DAG 进度视图

```
  ┌──────────┐      ┌──────────┐      ┌──────────┐
  │ 脚手架    │ ──►  │  Implement│ ──►  │ Review   │
  │ ✅ 完成   │      │ ● 3/5    │      │ ⏳ 等待  │
  └──────────┘      └──────────┘      └──────────┘
       │                  │
       ▼                  ▼
  ┌──────────┐      ┌──────────┐
  │ Logging  │      │ Model    │
  │ ✅ 完成   │      │ ● 进行中 │
  └──────────┘      └──────────┘
```

- 节点颜色依状态变化（待定→进行中→完成→失败）
- 关键路径高亮
- 鼠标悬停显示任务详情

#### 3. 统计面板

| 指标 | 实时值 | 说明 |
|------|-------|------|
| Active Agents | 5 / 8 | 当前活跃 Agent 数 / 总数 |
| Tasks | 12 完成 / 3 进行中 / 2 等待 | 任务 DAG 进度 |
| Tool Calls | 47 次 | 累计工具调用次数 |
| Token (Input) | 124,532 | 累计输入 Token |
| Token (Output) | 38,210 | 累计输出 Token |
| Cache Hit Rate | 67% | 缓存命中率（直接影响成本） |
| Session Duration | 12m 34s | 当前会话已运行时间 |
| Estimated Cost | $0.42 | 估算 API 费用 |

#### 4. 时间线与日志

```
12:30:01  [Phase]     Create: DESIGN → SCAFFOLD
12:30:03  [Architect]  输出任务 DAG (5 tasks)
12:30:05  [Supervisor] 分配 Scaffold Logging → Implementer A
12:30:06  [Supervisor] 分配 Scaffold Test → Implementer B
12:30:08  [Impl A]    Tool: glob("src/**/*.ts")  ✓ 12ms
12:30:10  [Viewer]    Concern: "日志配置缺少 requestId 追踪"
12:30:12  [Impl A]    Tool: write("src/utils/logger.ts") ✓ 45ms
12:30:15  [Impl A]    Tool: bash("cargo add tracing")    ✓ 2.3s
12:30:18  [Token]     Impl A: ↑2,341 ↓512 cache:62%
```

- 可筛选（按 Agent、事件类型、关键词）
- 可暂停/恢复实时流
- 支持回溯和搜索历史
- 每个工具调用可展开查看入参和结果

#### 5. Debug 证据链可视化 (Debug 工作流专用)

Debug 模式下，Agent 网络图切换为**证据链视图**：

```
┌─────────────────────────────────────────────────────┐
│  🔍 Debug Session: "登录页面500错误"                   │
│                                                     │
│  Goal: POST /api/login 返回 500                      │
│                                                     │
│  假设状态:                                            │
│  ┌─ H₁: 数据库连接池耗尽     ████████░░░░  0.7→证伪  │
│  ├─ H₂: AuthFilter NPE       ████████████  0.5→确认  │◄ 根因
│  ├─ H₃: Redis session 丢失  ██████░░░░░░  0.3→证伪  │
│  └─ H₄: 前端请求格式不对    ████░░░░░░░░  0.2→证伪  │
│                                                     │
│  证据链:                                             │
│  E₁: pg_stat → 80 空闲连接         [证伪 H₁] ●      │
│  E₂: AuthFilter:42 null 未检查      [支持 H₂] ●─► ● │
│  E₃: Redis ping 正常                [证伪 H₃] ●     │
│  E₄: 调用链路 → NPE at AuthFilter  [确认 H₂] ●─► ● │
│                                                     │
│  收敛状态: ✅ 已找到根因 → Fix Phase                 │
└─────────────────────────────────────────────────────┘
```

### 14.5 与约束层的关系

可视化本身也是一种约束——**阳光是最好的消毒剂**：

```
约束层(做正确的事)  +  可视化(看到正在做的事)  =  完整可信系统
      │                        │
      │ Agent 不能跳过阶段     │ 用户看到 Phase 转换
      │ Agent 不能越权用工具  │ 用户看到每次工具调用
      │ DAG 不能违反依赖      │ 用户看到任务拓扑
      │ 产物必须通过验证      │ 用户看到审查结果
```

### 14.6 技术选型

| 组件 | 选择 | 原因 |
|------|------|------|
| 前端框架 | React + Vite | 生态丰富，组件化 |
| 实时通信 | WebSocket | 双向实时，低延迟 |
| 图形可视化 | D3.js + vis-network | 力导向图，动画支持好 |
| 样式 | Tailwind CSS | 快速构建 UI |
| 状态管理 | Zustand | 轻量，适合实时数据流 |
| 历史存储 | SQLite (via better-sqlite3) | 会话持久化，零配置 |

---

## 十五、与对标项目对比

| 维度 | Codex CLI | OpenCode | Claude Code | Pi | **WhaleCode** |
|------|-----------|----------|-------------|-----|---------------|
| 多 Agent 定位 | 附加特性 | 附加特性 | 附加特性 | 无 | **核心原语** |
| Agent 关系 | 单Agent+SDK | 会话树 | 主从 fork-return | 单 Agent | **角色网格** |
| 通信 | Mailbox 异步 | 内部 pubsub/session events | 结果合并 | EventBus + steering/followUp | **Message Bus** |
| 任务模型 | 无显式 DAG | 线性 | 线性分解 | 无 | **DAG 工作流** |
| 调度器 | AgentControl | 无 | 无 | steering/followUp | **Supervisor** |
| 角色分工 | 隐式 | 隐式 | 显式(子Agent类型) | 无 | **显式 7 角色** |
| 模型分层 | 用户配置 | 用户配置 | 用户配置 | 用户配置 | **按角色自动选** |
| 对抗性审查 | 无 | 无 | 无 | 无 | **Viewer 持续批判** |
| 任务原生性 | 通用 | 通用 | 通用 | 通用 | **Create/Debug** |
| 失败恢复 | 手动 | 有限 | 有限 | 无 | **分级自动** |
| 上下文隔离 | AgentPath 继承 | Session 树 | fork 复制 | 无 | **按需分配** |
| 技能体系 | 静态 | 静态 | 静态 (CLI Skills) | AgentTool execute | **自进化 + 版本管理** |
| 可视化 | 无 | CLI 日志 | CLI 日志 | 无 | **实时 Web 网络图 + 统计** |

---

## 十六、关键设计决策

本节把原开放问题收敛成第一版实现决策。后续如果实测证明某项决策不可行，再以 ADR 形式变更，不在实现中隐式漂移。

| 问题 | 决策 | 理由 | 变更条件 |
|------|------|------|----------|
| Classifier 信任度 | 自动分类 + 用户显式覆盖。CLI 支持 `/create`、`/debug`、`/mode`，自动分类低置信度时进入 `clarify` 阶段 | coding 任务常混合；显式命令可避免误判 | 误判率连续 20 个任务 > 15% 时调整分类器 |
| Architect 持续介入 | Architect 在设计阶段产出 DAG；每个 Phase Gate 只做轻量检查；重大偏离由 Supervisor 重新唤醒 Architect | 避免 Architect 常驻消耗 Pro 预算 | Gate fail 或 Viewer critical concern |
| 混合任务 | 先拆成 Debug 子 DAG 和 Create 子 DAG。默认 Debug 先行，除非 Create 是复现/诊断前置条件 | 修复根因比堆功能更安全 | 用户明确要求先做功能 |
| Implementer 冲突 | Phase 1 禁止并行写同一文件；Phase 2 引入 `PatchArtifact` + ownership + 三方合并 | 文件冲突是多 Agent 最大不确定性来源 | 文件 ownership 可以被 DAG 静态证明时放开 |
| 并行度上限 | 默认由 `SwarmMode` 决定：economy 少量 agent，balanced 中等 fan-out，swarm 扩大 Scout/Analyst/Implementer 候选；共享工作区仍保持 `maxWorkspaceWriters=1` | 让系统能用数量换质量，同时守住写入安全 | 429、延迟、token、CPU、file ownership、cache hit 反馈触发动态调宽/降宽 |
| 会话持久化 | 使用 JSONL event-sourced session 作为 Phase 1；SQLite 作为 Phase 2 索引/查询层 | JSONL 易审计、易回放、易调试 | 会话列表/检索性能成为瓶颈 |
| 证据链收敛超时 | 每轮最多 5 个假设，最多 3 轮证据收集；仍未收敛则输出 `stuck_report` 请求用户补充 | 防止 Debug DAG 无限循环 | 用户选择继续深挖 |
| 假设生成数量 | 初始 3-5 个；第二轮允许最多 8 个，但必须引用新增证据或被排除假设 | 保持诊断可解释 | 复杂系统故障可手动提升 |
| Viewer 触发频率 | 默认只审查 Phase Gate、写入 artifact、权限升级、失败恢复、Reviewer 结论；不监听每个 token/普通消息 | 控制成本和 429 风险 | strict-review 模式开启 |
| DeepSeek V4 参数 | 官方已列出 V4 Flash/Pro，但 runtime 仍用 `ModelCapabilityProbe` 决定 context、output、thinking、tool-call、pricing、429 行为 | 官方 API 能力和价格会变动 | probe 结果持久化并带版本戳 |
| 成熟基础设施设计 | 采用 Codex-first Reference Audit Gate：权限、沙箱、工具执行、补丁、会话、上下文、MCP/Skills、日志先审计 Codex CLI；不足处再参考 Claude/OpenCode/Pi | 这些方向已经被成熟 coding 产品反复验证，WhaleCode 不应从 0 自创底座 | Codex 没覆盖或与 DeepSeek/Multi-Agent/Create-Debug 差异冲突时，用 ADR 记录替代方案 |

### 16.1 Debug 只读边界

Debug 的 `HYPOTHESIZE` 阶段只开放确定性只读能力：

```typescript
const GIT_READ_ALLOWLIST = [
  ["git", "status", "--short"],
  ["git", "diff", "--no-ext-diff"],
  ["git", "diff", "--cached", "--no-ext-diff"],
  ["git", "log"],
  ["git", "show"],
  ["git", "grep"],
] as const;

type ShellCapability =
  | { kind: "git_read"; argv: readonly string[] }
  | { kind: "readonly_exec"; argv: readonly string[]; cwd: string }
  | { kind: "write_exec"; argv: readonly string[]; cwd: string };
```

`readonly_exec` 仅用于测试可复现性时的读型命令，例如 `cargo test --no-run` 或会写 `target/` 的测试命令默认不在 HYPOTHESIZE 开放。需要写缓存的测试命令进入 `COLLECT_EVIDENCE`，并在临时 sandbox 中执行。

### 16.2 并行写入策略

第一版不让多个 Implementer 同时写共享工作区。并行 Implementer 只能产出 `PatchArtifact`，由 Supervisor 统一应用：

```typescript
type PatchArtifact = {
  id: string;
  taskId: string;
  agentId: AgentId;
  baseCommit: string;
  touchedFiles: string[];
  ownership: FileOwnershipClaim[];
  diff: string;
  testsRun: VerificationResult[];
  createdAt: string;
};

type FileOwnershipClaim = {
  path: string;
  mode: "exclusive" | "append_only" | "read_only";
  reason: string;
};

type PatchApplyResult =
  | { status: "applied"; commitSha?: string }
  | { status: "conflict"; files: string[]; reason: string }
  | { status: "rejected"; issues: string[] };
```

Supervisor 应用 patch 的门禁：

1. `baseCommit` 必须等于当前集成基线，或可 clean rebase。
2. `exclusive` ownership 不得重叠。
3. patch 应用后必须运行 task 级测试和全局 smoke。
4. 冲突不能交给 LLM 猜测合并，必须生成 `conflict_report` 并由 Architect/Reviewer 裁决。

### 16.3 Viewer 成本门禁

Viewer 是高价值机制，但不能默认全量监听。MVP 的 `ViewerPolicy`：

```typescript
type ViewerTrigger =
  | "phase_gate"
  | "artifact_written"
  | "permission_escalation"
  | "failed_recovery"
  | "review_completed"
  | "manual_request";

type ViewerPolicy = {
  enabled: boolean;
  triggers: ViewerTrigger[];
  maxConcernsPerPhase: number;
  maxTokensPerReview: number;
  severityGate: "critical_only" | "warning_and_above" | "all";
};
```

默认值：`enabled=true`、`triggers=["phase_gate","artifact_written","permission_escalation","review_completed"]`、`maxConcernsPerPhase=5`、`severityGate="warning_and_above"`。

### 16.4 隐私与日志边界

日志驱动是项目原则，但开源 coding agent 默认会接触用户代码、环境变量、API key、私有路径和命令输出。所有可观测性事件必须经过 redaction：

```typescript
type RedactionRule = {
  id: string;
  pattern: RegExp;
  replacement: string;
  category: "secret" | "path" | "token" | "email" | "env" | "custom";
};

type LogEventEnvelope<T> = {
  traceId: string;
  event: T;
  redaction: {
    appliedRuleIds: string[];
    hasSensitiveData: boolean;
  };
  storage: "local_only" | "exportable";
};
```

默认策略：
- `.env*`、SSH key、API key、token、cookie、Authorization header 不进入模型上下文和持久日志。
- `tool_call_start.args` 默认只记录 schema-valid 摘要，完整参数仅本地 debug 模式保存。
- Web 可视化默认读取本地 session，不上传。
- 导出报告必须经过 `redactForExport()`，并把 redaction 摘要写入报告头。

---

## 十七、参考实现映射

参考目录位于 `tmp/whalecode-refs/`，只作为设计证据，不直接复制代码。复制代码前必须单独做 license 审查和归属标注。

详细的 Codex-first 审计准则见 `docs/plans/2026-04-25-codex-first-reference-audit.md`。后续所有成熟基础设施模块都必须补齐 `reference_source`、`borrowed_behavior`、`whalecode_delta`、`rejected_behavior`、`license_boundary`、`acceptance_tests`，没有完成审计不得进入实现。

参考优先级：

1. Codex CLI：Rust core、permission、sandbox、unified exec、apply patch、context compaction、session trace、MCP/skills。
2. Claude Code 语义：plan mode、subagent、permission modes、skills 体验；通过公开复刻项目观察，不作为生产安全标准。
3. OpenCode：permission request UX、read-before-write、diff metadata、LSP diagnostics、session service。
4. Pi：JSONL session tree、event bus、web UI/runtime presentation。

| 参考项目 | 本地快照 | 上游 | 许可证 | WhaleCode 借鉴点 | 不借鉴点 |
|----------|----------|------|--------|------------------|----------|
| Codex CLI | `tmp/whalecode-refs/codex-cli` @ `c10f95dda` | https://github.com/openai/codex | Apache-2.0 | Rust CLI/core/tool/context/permission/session 设计、权限/沙箱、工具调度、context compaction、context fragment、agent mailbox、thread history 重建 | 不直接 fork 产品边界；不照搬其 Bazel/多 crate 复杂度和 OpenAI/Codex 专属假设 |
| OpenCode | `tmp/whalecode-refs/opencode` @ `73ee493` | https://github.com/opencode-ai/opencode | MIT | 文件编辑安全、permission request、session service、pubsub、LSP diagnostics、task session | Go 技术栈和 DB-first 架构不直接迁移 |
| Pi | `tmp/whalecode-refs/pi` @ `c0675041` | https://github.com/badlogic/pi-mono | MIT | TypeScript monorepo、agent loop、streaming event、tool sequential/parallel mode、JSONL session、web-ui 组件化 | 通用 Agent 定位，不直接继承其单 Agent 产品边界 |
| Claude Code from Scratch | `tmp/whalecode-refs/cc-from-scratch` @ `e5ce492` | https://github.com/Windy3f3f3f3f/claude-code-from-scratch | MIT | 最小 Agent、Tool、Subagent、Skill、MCP 教学实现，适合 MVP 骨架对照 | 安全和并发模型过轻，不作为生产标准 |
| Claw Code | `tmp/whalecode-refs/claw-code` @ `a389f8d` | https://github.com/ultraworkers/claw-code | 未在根目录发现标准 LICENSE | Claude Code 兼容面、parity audit、命令/工具快照 | license 未明确前不复制实现 |

### 17.1 模块到参考实现

| WhaleCode 模块 | 主要参考 | 本地证据 | 补全设计 |
|----------------|----------|----------|----------|
| `AgentLoop` | Codex + Pi | `codex-rs/core`、`packages/agent/src/agent-loop.ts` | Rust core 采用事件流 + tool loop；支持 steering/follow-up；不在 loop 内混入 workflow phase |
| `ToolRuntime` | Codex + Pi | `core/src/tools/parallel.rs`、`agent-loop.ts` | Rust 工具带 `executionMode`；只读工具可并发，写工具串行；所有工具输出进入 truncation |
| `PermissionEngine` | Codex + OpenCode | `config/permissions.rs`、`permission.go` | profile 编译 + permission request；deny 优先；session grant 与 persistent grant 分开 |
| `FileEditTools` | OpenCode + Codex apply-patch | `write.go`、`edit.go`、`apply-patch` crate | read-before-write、mtime 检查、diff metadata、patch artifact、LSP diagnostics |
| `SessionStore` | Codex + Pi + OpenCode | `thread_history.rs`、`session-manager.ts`、`session.go` | Phase 1 JSONL event log；Phase 2 SQLite index；支持 parent session 和 replay |
| `MessageBus` | Codex mailbox + local pubsub | `agent/mailbox.rs`、`pubsub/broker.go` | 单进程内存 bus 起步；事件带 seq、traceId、causality；后续可换 Redis |
| `ContextManager` | Codex | `compact.rs`、`context_manager/history.rs`、`context/fragment.rs` | separate-turn compaction、replacement history、fragment marker、remove-first emergency |
| `Skills` | cc-from-scratch + Codex | `skills.ts`、`core/src/skills.rs` | frontmatter + project/user priority；Phase 1 静态，self-evolving 延后 |
| `MCP` | Codex + cc-from-scratch | `codex-mcp`、`rmcp-client`、`mcp.ts` | Rust stdio MCP client + tool prefix；权限映射后才能暴露给 Agent |
| `Observability` | Codex thread history + Pi events | `thread_history.rs`、`agent-loop.ts` event sink | 所有核心动作 emit event；Web UI 只读消费 event store |

### 17.2 不直接照搬的点

1. 不直接 fork Codex CLI；WhaleCode 采用 Rust-first core，但按自身 Multi-Agent First、Create/Debug、DeepSeek 路由和 Viewer 约束重新实现。
2. 不照搬 OpenCode 的 DB-first session；早期 JSONL 更利于 debug、回放和用户手动检查。
3. 不照搬 Pi 的通用 Agent 产品形态；WhaleCode 的差异点仍是 Create/Debug 原语和多 Agent DAG。
4. 不照搬 `cc-from-scratch` 的通用 `run_shell` 权限；它适合作最小教学实现，不足以覆盖开源 agent 的安全边界。
5. 不在 Claw Code license 未明确前复制任何实现代码，只参考其 parity/audit 思路。

---

## 十八、MVP 接口草案

接口草案用于约束第一版工程落地。技术栈决策已在 `docs/plans/2026-04-25-rust-first-technology-architecture.md` 中更新为 Rust-first core，本章保留核心语义并改为 Rust 形态。实际代码可以调整命名，但语义不能弱化。

### 18.1 包结构

```text
crates/
  whalecode-protocol/     # Shared event/message/tool/session schema
  whalecode-core/         # AgentLoop, Supervisor, MessageBus
  whalecode-model/        # DeepSeek adapter, capability probe, model routing
  whalecode-context/      # ContextManager, compaction, fragments
  whalecode-tools/        # Read/Edit/Write/Shell/Git/Web/MCP built-in tools
  whalecode-permission/   # Permission profiles, grants, ask/deny decisions
  whalecode-patch/        # PatchArtifact, diff, ownership, apply engine
  whalecode-session/      # JSONL store, replay, fork, SQLite index later
  whalecode-workflow/     # Create/Debug phase machines + DAG validators
  whalecode-mcp/          # stdio JSON-RPC client, MCP tool mapping
  whalecode-observe/      # Event schemas, redaction, tracing/JSONL bridge
  whalecode-cli/          # clap commands, REPL, slash commands
  whalecode-tui/          # ratatui interactive UI

apps/
  viewer/                 # React + Vite visualization UI
```

### 18.2 Agent Loop

```rust
pub enum AgentRole {
    Architect,
    Debugger,
    Implementer,
    Searcher,
    Reviewer,
    Viewer,
    Supervisor,
}

pub enum AgentState {
    Idle,
    Busy,
    Blocked,
    Done,
    Failed,
}

pub struct AgentLoopConfig {
    pub role: AgentRole,
    pub model: ModelRoute,
    pub tools: Vec<ToolSpec>,
    pub context: Arc<dyn ContextManager>,
    pub bus: Arc<dyn MessageBus>,
    pub permission: Arc<dyn PermissionEngine>,
    pub events: Arc<dyn ObservabilitySink>,
}

#[async_trait::async_trait]
pub trait AgentLoop {
    fn id(&self) -> AgentId;
    fn state(&self) -> AgentState;
    async fn start(&mut self, task: TaskAssignment) -> Result<TaskResult, AgentError>;
    async fn interrupt(&mut self, reason: InterruptReason) -> Result<(), AgentError>;
    async fn close(&mut self) -> Result<(), AgentError>;
}
```

设计要求：
- Agent Loop 只负责“LLM ↔ tool loop”，不负责 phase 转换。
- 每个 assistant message 结束后提取 tool calls，交给 `ToolRuntime`。
- tool result 作为结构化消息回填；DeepSeek thinking tool-use 期间必须保留当前 sub-turn 的 `reasoning_content`。
- 用户新输入走 steering/follow-up 队列，不直接改写 Agent 内部状态。

### 18.3 DeepSeek Adapter

```rust
pub struct ModelCapability {
    pub provider: ProviderId,
    pub model: String,
    pub observed_at: DateTime<Utc>,
    pub context_window: usize,
    pub max_output_tokens: usize,
    pub supports_thinking: bool,
    pub supports_tool_calls: bool,
    pub supports_thinking_tool_calls: bool,
    pub supports_strict_tool_schema: bool,
    pub pricing: Option<ModelPricing>,
}

#[async_trait::async_trait]
pub trait ModelCapabilityProbe {
    async fn probe(&self, model: &str) -> Result<ModelCapability, ModelError>;
    fn cached(&self, model: &str) -> Option<ModelCapability>;
}

#[async_trait::async_trait]
pub trait LlmClient {
    async fn stream(&self, input: LlmRequest) -> Result<LlmStream, ModelError>;
}
```

DeepSeek adapter 的硬约束：
- `baseURL` 默认 `https://api.deepseek.com`，可通过配置覆盖。
- thinking 参数通过 DeepSeek 当前 API 支持的 request body 字段表达；Rust core 不把 OpenAI SDK 作为核心依赖。
- thinking + tool calls 的 sub-turn 保留 `reasoning_content`，新用户 turn 前清理旧 reasoning。
- `parallel_tool_calls` 不作为 DeepSeek 必选参数；并发由 `ToolRuntime` 自己决定。
- V4 模型名、1M context、384K output、Flash/Pro 价格只有 probe 通过后才进入 runtime。

### 18.4 Message Bus

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

MVP 用内存 bus，并满足：
- `seq` 单调递增，便于 replay 和 UI 排序。
- handler 异常不得击穿 bus。
- bus event 同步写入 `SessionStore`，避免进程崩溃后丢主线。
- Phase 2 再增加 Redis/SQLite-backed bus，不改变接口。

### 18.5 Tool Runtime

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
- `parallel_safe` 可并发执行，例如 read/glob/grep/web fetch。
- 任意 batch 中出现 `sequential` 时整个 batch 串行。
- `exclusive_write` 获得全局 write lock，并必须产出 diff metadata。
- shell 默认 `sequential`，除非命令被 `ReadonlyCommandPolicy` 证明安全。

### 18.6 Permission Engine

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
2. phase tool permission
3. role permission
4. file ownership
5. session/project grant
6. hook decision
7. user ask

### 18.7 Session Store

```rust
pub enum SessionEntry {
    SessionStarted(SessionStarted),
    Message(BusEnvelope<EventPayload>),
    ToolCall { call: ToolCall, redacted_args: serde_json::Value },
    ToolResult(ToolResult),
    PhaseTransition { from: WorkflowPhase, to: WorkflowPhase },
    PatchArtifact(PatchArtifact),
    Compaction(CompactionEntry),
    Verification(VerificationResult),
}

#[async_trait::async_trait]
pub trait SessionStore {
    async fn append(&self, entry: SessionEntry) -> Result<(), SessionError>;
    async fn read(&self, session_id: SessionId) -> Result<SessionStream, SessionError>;
    async fn replay(&self, session_id: SessionId) -> Result<ReplaySnapshot, SessionError>;
    async fn fork(&self, session_id: SessionId, from_seq: u64) -> Result<SessionId, SessionError>;
}
```

Phase 1 存储格式：`~/.whalecode/sessions/<session-id>.jsonl`。每行一个 JSON object；第一行必须是 `session_started`；所有 entry 必须带 schema version。

### 18.8 Workflow Phase Machine

```rust
pub enum WorkflowKind {
    Create,
    Debug,
}

pub enum WorkflowPhase {
    Idle,
    Analyze,
    Design,
    Scaffold,
    Implement,
    Review,
    Confirm,
    Hypothesize,
    CollectEvidence,
    Evaluate,
    Fix,
    Verify,
    Result,
}

pub struct PhaseGate {
    pub from: WorkflowPhase,
    pub to: WorkflowPhase,
    pub checks: Vec<GateCheck>,
}

#[async_trait::async_trait]
pub trait WorkflowMachine {
    fn current(&self) -> WorkflowPhase;
    async fn can_transition(&self, to: WorkflowPhase, ctx: WorkflowContext) -> GateResult;
    async fn transition(&mut self, to: WorkflowPhase, ctx: WorkflowContext) -> Result<(), WorkflowError>;
}
```

Create 和 Debug 各自拥有独立 gates；gates 只读检查，不执行修复。

---

## 十九、技术栈

详细技术架构规划见 `docs/plans/2026-04-25-rust-first-technology-architecture.md`。本章只保留主选型摘要。

| 组件 | 选择 | 原因 | 参考 |
|------|------|------|------|
| 核心运行时 | Rust stable | 本地执行安全、并发状态、单二进制分发、长期可维护性更适合 coding agent core | Codex CLI |
| Async runtime | Tokio | streaming、subprocess、timeout、bus、Web bridge 都是 async I/O 问题 | Codex CLI / Tokio |
| CLI | clap | Rust CLI 标准选择，适合子命令、completion、配置参数 | Codex CLI |
| TUI | ratatui + crossterm | 终端交互、diff preview、permission prompt、tool progress | Codex CLI |
| HTTP / SSE | reqwest + stream parser | DeepSeek OpenAI-compatible API 需要手写 thinking/tool-call 兼容层 | DeepSeek official docs |
| Schema / serialization | serde / serde_json / serde_yaml | 事件、session、tool schema、配置都需要版本化结构化数据 | Rust 生态 |
| 测试 | cargo test + integration fixtures | Rust core 需要覆盖权限、phase、tool runtime、session replay | Codex CLI |
| 格式/Lint | cargo fmt + clippy | Rust workspace 基线质量门禁 | Rust 生态 |
| Git/Patch | 自研 `PatchArtifact` + unified diff parser | 先生成 artifact，再由 Supervisor 应用，避免多 Agent 直接写共享区 | OpenCode + Codex apply-patch |
| Shell | Rust subprocess wrapper | timeout、env redaction、stream output、exit metadata、permission gate | Codex unified_exec 思路 |
| MCP | stdio JSON-RPC adapter first；Phase 2 评估 Rust SDK | MCP 工具必须经过 PermissionEngine 和 truncation/redaction | cc-from-scratch + MCP docs |
| 会话存储 | JSONL first, SQLite index later | 可回放、可审计、易恢复，后续增加索引 | Pi + OpenCode |
| Web Viewer | TypeScript + React + Vite | 可视化前端仍用 Web 生态 | Pi web-ui 思路 |
| 实时通信 | Rust SSE / WebSocket | 只读推送 session events，断线后按 seq 补事件 | 可观测性层 |

---

## 二十、分阶段落地计划

### 20.1 Phase 0 — 文档与工程基线

目标：保证后续实现不从空白开始摸索。

交付：
- `docs/plans/2026-04-24-system-architecture.md` 补齐参考映射、接口、MVP 验收和引用。
- `docs/plans/2026-04-25-rust-first-technology-architecture.md` 固化 Rust-first 技术架构。
- `docs/plans/2026-04-25-multi-agent-collaboration-architecture.md` 固化多 Agent 群体协同、Tournament、Evidence Race、Patch League、ConcurrencyGovernor 设计。
- `docs/plans/2026-04-25-codex-first-reference-audit.md` 固化成熟基础设施的 Codex-first 审计门禁。
- `docs/plans/2026-04-25-differentiated-primitives-architecture.md` 固化证据链、脚手架先行、参考驱动、Viewer、技能自进化的运行时原语设计。
- `docs/adr/2026-04-25-rust-first-core-runtime.md` 记录主栈决策。
- `.gitignore` 忽略 `tmp/`、`.DS_Store`、`node_modules/`、`dist/`、`coverage/`、`.env*`。
- 选定 Rust workspace + TypeScript Web Viewer。
- 建立 `docs/adr/`，记录关键设计变更。

验收：
- 文档有 ≥3 外部引用。
- 每个核心模块有明确参考来源和不借鉴边界。
- 每个成熟基础设施模块都有 Codex-first audit 字段。
- 每个差异化原语都有 artifact、gate、event 和 replay 验收。
- 没有未决开放问题阻塞 Phase 1。

### 20.2 Phase 1 — 单 Agent 可运行纵切

目标：先做可运行的 coding agent，不做完整多 Agent 网格。

交付范围：
1. `crates/whalecode-protocol`: event/message/tool/session schema。
2. `crates/whalecode-core`: `AgentLoop`、`Supervisor`、`MessageBus` 基础实现。
3. `crates/whalecode-model`: DeepSeek streaming client、thinking support、capability probe。
4. `crates/whalecode-tools`: `read`、`glob`、`grep`、`edit`、`write`、`git_read`、受控 `shell`。
5. `crates/whalecode-permission`: permission profiles、grant、ask/deny。
6. `crates/whalecode-session`: JSONL event log、redaction、session replay。
7. `crates/whalecode-cli`: 基础 REPL、`/create`、`/debug`、`/compact`、`/status`。
8. 差异化原语 schema skeleton：`ReferenceDecision`、`ScaffoldArtifact`、`DebugCase`、`EvidenceRecord`、`ViewerConcern`、`SkillInvocationEvent`。

验收：
- 能在真实仓库内读取、搜索、编辑一个文件。
- 写文件前必须 read-before-write，写后返回 diff metadata。
- DeepSeek thinking + tool call 的多 sub-turn 测试通过。
- 所有 tool result 截断策略可测试。
- Session JSONL 可 replay 出最终消息、工具调用、patch 和 phase transitions。
- fixture JSONL 可 replay 出 reference、scaffold、debug、viewer concern 和 skill telemetry 的最小状态。

### 20.3 Phase 2 — 多 Agent 群体协同 + Create/Debug DAG

目标：引入可证明安全的群体协同，把数量转化为质量。详细设计见 `docs/plans/2026-04-25-multi-agent-collaboration-architecture.md`。

交付范围：
1. Phase 2A：`SwarmSpec`、`CohortSpec`、`WorkUnit`、Scout Cohort、Artifact store、ConcurrencyGovernor MVP。
2. Phase 2B：Analyst Cohort、DiversityPolicy、CandidateScore、EvidenceWeightedConsensus、Pro Judge gate、independence/effective-agent score。
3. Phase 2C：Implementer Cohort、Patch League、dry-run apply、Reviewer scorecard、Verifier matrix。
4. Phase 2D：HypothesisSet、EvidencePlan、Evidence Race、falsification rules、RootCauseDecision。
5. Create workflow: analyze → design tournament → scaffold → patch league / sharded implement → review → confirm。
6. Debug workflow: analyze → hypothesis cohort → evidence race → root-cause judge → fix candidates → verify。

验收：
- 同一任务可并行启动 8 个只读 Scout，且 Searcher/Scout 不会写文件。
- 同一需求能生成至少 3 个 plan candidate，并由 Judge 选择、合成或拒绝。
- 同一 work unit 可产 2-4 个 PatchArtifact 候选，共享工作区只应用最终 patch。
- 两个 Implementer 同时触碰同一文件时，Supervisor 阻止并输出 conflict report。
- Debug HYPOTHESIZE 阶段无法执行写命令。
- 至少 3 个 Debug 假设可并行收集证据，被证伪假设停止继续消耗。
- Debug 修复必须带复现消失证据和回归测试。
- 429 mock、延迟和 token budget 能触发 ConcurrencyGovernor 降宽。

### 20.4 Phase 3 — Viewer 与可视化

目标：把可观测性变成产品能力，不影响核心执行。

交付范围：
1. Viewer 按 `ViewerPolicy` 审查 phase gate 和 artifacts。
2. WebSocket/SSE 推送 session events。
3. Web UI 显示 Agent 状态、DAG、工具调用、token/cost、证据链。
4. Strict-review 模式可手动开启。

验收：
- Viewer critical concern 能阻止 phase transition。
- 可视化断开不影响 Agent 执行。
- session replay 和实时视图展示一致。

### 20.5 Phase 4 — Skills / MCP / Self-Evolution

目标：生态能力后置，避免早期架构被插件系统拖慢。

交付范围：
1. 静态 Skills：project/user priority、frontmatter、allowed tools。
2. MCP stdio client：tool prefix、permission mapping、timeout。
3. Skill invocation telemetry。
4. Self-evolving proposal 只生成建议，不自动发布。

验收：
- MCP 工具默认不可越过 phase permissions。
- Skill 修改需要版本号、发布时间、发布者、变更内容。
- Evolution proposal 必须引用使用数据，不允许凭空改 skill。

---

## 二十一、Phase 1 验收清单

### 21.1 功能验收

- `whalecode` 可在一个 TS 项目中启动 REPL。
- 支持输入自然语言任务并进入单 Agent loop。
- 支持 `read/glob/grep/edit/write/git_read`。
- 支持 DeepSeek streaming 文本、thinking、tool call。
- 支持中断当前 turn。
- 支持 `/status` 查看 session id、model、token、cost、phase。
- 支持 `/compact` 触发 separate-turn compaction。
- 支持 `/debug` 进入 Debug phase machine 的单 Agent 版本。

### 21.2 安全验收

- `.env*` 默认不可读，除非用户显式授权。
- `HYPOTHESIZE` 不能写文件、不能执行通用 shell。
- shell 命令带 timeout、cwd、env allowlist。
- 写文件必须先读；mtime 变化时拒绝写。
- 所有写操作产出 diff metadata。
- destructive command 默认 deny 或 ask，不自动执行。
- 日志经过 redaction，默认 local-only。

### 21.3 测试验收

- `cargo test --workspace` 通过。
- `cargo fmt --check` 通过。
- `cargo clippy --workspace --all-targets -- -D warnings` 通过。
- `npm --prefix apps/viewer run build` 通过（Phase 3 后启用）。
- DeepSeek adapter 有 mock streaming 测试，覆盖 thinking + tool call。
- ToolRuntime 有并发/串行测试。
- PermissionEngine 有 deny > phase > role > grant 的优先级测试。
- SessionStore 有 append/replay/fork 测试。
- ContextManager 有 compaction replacement 和 remove-first 测试。

### 21.4 可观测性验收

- 每个 tool call 记录 start/end/error。
- 每次 phase transition 记录 gate result。
- 每次 compaction 记录 before/after tokens。
- 每次 write 记录 redacted diff summary。
- session replay 能重建用户可见 transcript。

---

## 二十二、工程风险与应对

| 风险 | 影响 | 应对 |
|------|------|------|
| DeepSeek V4 API 参数与假设不一致 | 模型适配失败 | 所有 V4 能力走 `ModelCapabilityProbe` 和 feature flag |
| API 429 / 并发限制 | 多 Agent 吞吐不可控 | RateLimiter + dynamic maxAgents + retry backoff |
| thinking + tool call 历史格式错误 | API 400 | DeepSeek adapter 单测覆盖 reasoning_content 保留/清理 |
| 并行写入冲突 | 代码损坏 | Phase 1 单写；Phase 2 PatchArtifact + ownership |
| 日志泄露隐私 | 开源项目高风险 | redaction first，export 需二次脱敏 |
| 工具输出撑爆上下文 | Agent 失控 | 所有 tool output 入库前截断，原始输出只本地临时保存 |
| Viewer 成本过高 | 响应慢、成本高 | 事件分级触发，默认只审查关键节点 |
| Skills 自进化误改行为 | 用户信任受损 | Phase 4 只生成 proposal，不自动发布 major/minor |

---

## 二十三、参考来源

### 23.1 外部来源

1. DeepSeek API Docs — Your First API Call: https://api-docs.deepseek.com/
   用途：确认 `deepseek-v4-flash` / `deepseek-v4-pro`、OpenAI/Anthropic compatible base URL，以及旧模型 deprecation 信息。
2. DeepSeek API Docs — Models & Pricing: https://api-docs.deepseek.com/quick_start/pricing
   用途：确认当前官方 API 的 V4 Flash/Pro 模型名、1M context、384K max output、tool-call 支持和价格表；runtime 仍通过 provider probe 记录能力与价格版本。
3. DeepSeek API Docs — Thinking Mode: https://api-docs.deepseek.com/guides/thinking_mode
   用途：确认 `reasoning_content`、thinking tool calls、多轮对话中 reasoning 的保留/清理要求。
4. DeepSeek API Docs — Tool Calls: https://api-docs.deepseek.com/guides/tool_calls
   用途：确认当前 API 的 function/tool call 形态，并把并行执行责任放在 WhaleCode runtime。
5. DeepSeek API Docs — Context Caching: https://api-docs.deepseek.com/guides/kv_cache/
   用途：确认缓存是默认服务能力、64-token 单元和 best-effort 属性；缓存不作为调度正确性前提。
6. DeepSeek API Docs — Rate Limit: https://api-docs.deepseek.com/quick_start/rate_limit/
   用途：确认动态并发限制、HTTP 429、SSE keep-alive 和长时间未开始推理的连接关闭行为。
7. DeepSeek API Docs — Create Chat Completion: https://api-docs.deepseek.com/api/create-chat-completion
   用途：确认 `deepseek-v4-flash` / `deepseek-v4-pro` model enum、thinking 参数和 usage 中 cache/reasoning token 字段。
8. OpenAI Codex repository: https://github.com/openai/codex
   用途：参考 compaction、context fragment、permission/sandbox、tool orchestration、mailbox 和 thread history。
9. OpenCode repository: https://github.com/opencode-ai/opencode
   用途：参考 file edit safety、permission request、session service、pubsub、LSP diagnostics。
10. Pi monorepo: https://github.com/badlogic/pi-mono
   用途：参考 agent loop、tool execution mode、JSONL session、web-ui 组件化；Rust core 只迁移语义，不迁移 TS runtime。
11. Claude Code from Scratch repository: https://github.com/Windy3f3f3f3f/claude-code-from-scratch
   用途：参考最小 Agent/Tool/Skill/MCP/Subagent 实现边界。
12. Rust official site: https://www.rust-lang.org/
   用途：确认 Rust-first core 的可靠性、性能和内存安全定位。
13. Tokio official site: https://tokio.rs/
   用途：确认 async runtime 覆盖 I/O、timer、filesystem、sync 和 scheduling。
14. ratatui official site: https://ratatui.rs/
   用途：确认 Rust TUI 生态可覆盖终端交互。
15. Node.js Permission Model docs: https://nodejs.org/api/permissions.html
   用途：确认 Node permission model 不应作为 coding agent 的核心安全边界。
16. Deno Security and Permissions docs: https://docs.deno.com/runtime/fundamentals/security/
   用途：确认 `allow-run` 子进程权限会削弱 runtime sandbox，shell 安全必须由 WhaleCode 自己控制。

### 23.2 本地源码证据

| 证据 | 路径 |
|------|------|
| Codex separate-turn compaction | `tmp/whalecode-refs/codex-cli/codex-rs/core/src/compact.rs` |
| Codex ContextManager history replacement | `tmp/whalecode-refs/codex-cli/codex-rs/core/src/context_manager/history.rs` |
| Codex contextual fragment | `tmp/whalecode-refs/codex-cli/codex-rs/core/src/context/fragment.rs` |
| Codex tool parallel gate | `tmp/whalecode-refs/codex-cli/codex-rs/core/src/tools/parallel.rs` |
| Codex permission profile compiler | `tmp/whalecode-refs/codex-cli/codex-rs/core/src/config/permissions.rs` |
| OpenCode permission request | `tmp/whalecode-refs/opencode/internal/permission/permission.go` |
| OpenCode write/edit safety | `tmp/whalecode-refs/opencode/internal/llm/tools/write.go`、`tmp/whalecode-refs/opencode/internal/llm/tools/edit.go` |
| OpenCode pubsub/session service | `tmp/whalecode-refs/opencode/internal/pubsub/broker.go`、`tmp/whalecode-refs/opencode/internal/session/session.go` |
| Pi agent loop and tool execution | `tmp/whalecode-refs/pi/packages/agent/src/agent-loop.ts` |
| Pi event bus and JSONL session | `tmp/whalecode-refs/pi/packages/coding-agent/src/core/event-bus.ts`、`tmp/whalecode-refs/pi/packages/coding-agent/src/core/session-manager.ts` |
| cc-from-scratch minimal tools/subagent/skills/MCP | `tmp/whalecode-refs/cc-from-scratch/src/tools.ts`、`src/subagent.ts`、`src/skills.ts`、`src/mcp.ts` |

### 23.3 引用合规

- 本文档只引用架构思想、接口边界和实现模式，不复制第三方源码。
- 后续如需复制或改写具体实现，必须在对应文件头或 NOTICE 中标注来源与许可证。
- `tmp/whalecode-refs/` 属于本地研究材料，不进入仓库提交。
