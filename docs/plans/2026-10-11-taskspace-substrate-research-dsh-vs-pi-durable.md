# TaskSpace/TaskMap 底座调研：DSH vs pi-durable

- 状态：调研记录（证据充分，待选型决策）
- 日期：2026-10-11
- 范围：把 WhaleCode TaskSpace/TaskMap 做成"可安装插件/扩展"这一目标下，DeepSeek Harness（DSH）与 pi-durable 两条技术路线的可行性与优劣
- 关联证据：
  - DSH 插件可行性实验（32 用例）：[`docs/experiments/2026-10-11-dsh-taskspace-plugin-feasibility-spike.md`](../experiments/2026-10-11-dsh-taskspace-plugin-feasibility-spike.md)、源码 [`docs/experiments/dsh-taskspace-spike/`](../experiments/dsh-taskspace-spike/README.md)
  - pi-durable 原语实验：[`docs/experiments/pi-durable-probe/`](../experiments/pi-durable-probe/README.md)

## 1. 外部来源（参考驱动）

| # | 来源 | 用途 |
|---|---|---|
| 1 | `deepseek-ai/deepseek-harness` @ `d743267`（release-0.2.1-alpha.2，MIT） | DSH 架构、插件/客户端插件/持久化契约 |
| 2 | `@earendil-works/pi-durable@1.1.0`（npm，MIT）及其 `docs/spec.md`、`test/examples/*` | pi-durable 的 durable 文档/任务/存储契约 |
| 3 | `@earendil-works/pi-coding-agent@1.1.0` docs（`how-pi-works.md`、`rpc-extension-ui.md`、`extensions.md`） | pi 产品自身的会话/扩展/UI 模型，用于区分 pi CLI 与 pi-durable |
| 4 | npm 生态实例：`pi-durable-subagents`、`@groeponline/pi-missions`、`@xzzpig/pi-goal-x` | pi-durable 已被第三方用于多 agent/任务/目标编排的旁证 |
| 5 | `@earendil-works/chord`（replicated state） | pi-durable 的 view/watch/taskGraph 状态复制基础 |

## 2. 结论摘要

| 问题 | 结论 |
|---|---|
| DSH 能否纯插件实现 TaskSpace/TaskMap？ | 能。32 个确定性用例覆盖批级准入、CAS、DAG/frontier、reservation/deferred、上下文策略、resume、跨会话权威、取消完整性、fork 血缘、RPC/命令、**GUI 安装即用** |
| DSH 的唯一硬边界 | `plugin:` 记录是 **ignorable + best-effort**，跨 DSH 格式迁移不保证保留；发布级持久性需上游声明事件或插件侧 `ctx.storage` 双写 |
| pi-durable 能否承载 TaskMap？ | 能，且**持久化与任务原语更强**：typed documents（version + `migrate()`）、durable task（checkpoint/waiting/owner/abort）、task graph、session/conversation/task 三种 scope 都是原生能力 |
| 谁更适合 Whale 的"GUI 安装即用"目标？ | **DSH**：内建 Web GUI + 客户端插件运行时加载 + 侧栏 extension 槽位（已实测）；pi 产品是 TUI-first，Web GUI 需自建前端 |
| 谁更适合 TaskMap 的"核心语义与持久性"？ | **pi-durable**：DSH 侧需要插件手写的状态机/迁移，在 pi-durable 里是库原语 |
| 是否必须二选一？ | 不是。可选：① TaskMap 内核建在 pi-durable、UI 自建；② 留在 DSH 并用 `ctx.storage` 双写补齐持久性；③ 双轨（DSH 做产品壳，pi-durable 做语义参考/移植） |

## 3. DSH 调研记录（摘要）

完整报告见关联文档；要点：

- **架构**：Cordis "everything is a plugin"，profile/bundle/patch 组合；session log 为唯一权威 + session projection（纯 fold，`wire` 客户端视图）；插件可用 `appendPluginRecord` 写 `plugin:` 命名空间的 ignorable 记录。
- **实测通过**：批级原子准入（读已提交 assistant message 重算整批裁决 → guard 零派发）、`agent.ctx.tools.restrict` 模式切换、跨会话权威写入、取消时 reservation 释放、Typert source-mode RPC、fork 血缘、`sidebarRightTabs` extension 类型、`dsh.client` 客户端入口运行时加载。
- **GUI 路径**：Web Plugins 页安装 bundle（registry/git/tarball/path/link）→ bundle patch 装载宿主服务 → 客户端入口编入 `window.__DSH_BOOT__` → 侧栏面板读 `useProjection('whaleTaskspace')`。
- **唯一边界**：上游明文 "a format migration retains records on a best-effort basis, so a release package declares its events instead"。

## 4. pi-durable 调研记录

### 4.1 架构事实

- 定位：**durable agent harness 库**（不是 pi CLI 的运行时；pi CLI 用自己的 JSONL 会话树）。pi-durable 由 pi 团队维护，标注 Experimental，依赖 `pi-ai`（模型）+ `chord`（复制状态）。
- 核心对象：
  - **Harness**：一次打开一个 storage；所有变更走**一条原子提交线**，且"committed before anything is shown"。
  - **Conversation**：不可变 entry 的 transcript；支持 fork、reset/handoff、compaction、busy 时的 steer/follow-up/write 与 `requestId` 幂等。
  - **Document**：与 transcript 同 commit 的 typed JSON 状态；scope 为 `session | conversation | task`，支持 family（多实例）、`version + migrate()`、`history: latest | rewindable`、`fork: initial | current | asOf`。
  - **Task**：durable 状态机，每步 checkpoint；`pending/running/waiting/completing/terminal`；owner 为 conversation 或父 task；`failFast/allSettled` join；abort 自底向上；`migrate()` 支持跨版本。
  - **Extension**：tools / system prompt sections / hooks（`beforeRequest`、`afterResponse`、`onYield`、`afterTools`、`beforeTool`、`afterTool`）/ wraps / tasks；registry 支持运行时 install/replace/uninstall；每会话通过 `configure()` 选择扩展。
  - **观测面**：`viewState()`、`watch()`（Chord ops）、`watchEvents()`（agent 事件）、`taskGraph()/watchTaskGraph()`、`watchDoc()`。
  - **存储**：memory / JSONL（可选 fsync）/ SQLite（WAL，`synchronous=NORMAL`）/ Cloudflare Durable Object；单进程独占（与 DSH 相同），但 DO 后端给出远程/边缘存储路径。
  - **缓存**：provider session identity 持久化并转发给 pi-ai（prompt-cache/session affinity）；系统提示只发送变更部分。

### 4.2 实验证据（`docs/experiments/pi-durable-probe/`）

| 验证项 | 结果 |
|---|---|
| session 级文档跨会话共享 | ✅ |
| entry + 文档原子提交 | ✅ |
| 文档 live watch | ✅（revision 帧） |
| **文档 v1→v2 跨进程迁移** | ✅ `migrate()` 生效 |
| durable 父/子任务 waiting + failFast + owner | ✅ 父 `waiting on 2`，子 `owner: 14`，最终 `completed` |
| task graph 状态帧 | ✅ waiting → 子收束 → running |

### 4.3 与 TaskMap 的映射

| TaskMap 需求 | pi-durable 原生对应 |
|---|---|
| canonical map + revision | session-scoped `defineDoc`（含 history/fork 语义） |
| CAS / 幂等 | 原子 commit + `requestId` 幂等 + 文档单写者 |
| 版本演进 | `version + migrate()`（原生） |
| action reservation / deferred | 父任务 + 子任务 + `waiting`/`failFast` + checkpoint（官方支付示例同构） |
| 任务面板 | `taskGraph()` / `watchTaskGraph()` |
| 模式切换 | `configure({ extensions: { add/remove } })`（官方 `27-plan-mode.ts`） |
| 多 agent | conversation ownership + subagent 工具示例 + background 任务 |
| 工具批级准入 | 未原生：`beforeTool` 按调用、`afterTools` 在轮次后；整批零派发需自建（与 DSH 同类问题） |

## 5. 对比

| 维度 | DSH | pi-durable | 更优 |
|---|---|---|---|
| 定位 | 完整 agent harness（Web GUI/CLI/SDK/桌面） | durable runtime 库（用于构建 agent） | 取决于目标 |
| 状态原语 | session log + projection；插件记录 ignorable/best-effort | typed documents（scope/version/migrate/history/fork）+ 原子 commit | **pi-durable** |
| 任务原语 | 无内建 durable task；reservation/deferred 需插件自建 | 内建 durable task + join + owner + abort + task graph | **pi-durable** |
| 跨版本持久性 | 唯一边界（需上游声明或 `ctx.storage` 双写） | 原生 `migrate()`，实验已证 | **pi-durable** |
| 多 agent | subagent + agent-team（experimental）；跨会话写需插件绑定（已验证可行） | conversation ownership + subagent/background 示例；session doc 天然共享 | **pi-durable**（语义），DSH（已验证插件路径） |
| 模式切换 | `agent.ctx.tools.restrict` + guard（已验证） | `configure()` 提交式切换（官方示例） | 平手 |
| 扩展模型 | Cordis 插件 + bundle/profile + 客户端插件 | extension（tools/sections/hooks/tasks）+ npm pi packages | 平手（DSH 客户端面更强） |
| **GUI** | **内建 Web GUI + 槽位 + 运行时客户端插件（已验证"安装即用"）** | pi 产品 TUI-first；RPC/SDK 自建前端；pi-durable 提供 view/watch 数据面 | **DSH** |
| DeepSeek 适配 | 第一方 wire（`dsh_session_log`、cache-aware prompt surface） | pi-ai 多 provider；DeepSeek 适配与缓存行为未验证 | **DSH** |
| 存储 | JSONL + SQLite state DB；插件可扩展 | memory/JSONL/SQLite/Cloudflare DO + conformance 测试套件 | **pi-durable** |
| 成熟度/风险 | developer preview，2 个月，官方 breaking 预警 | pi-durable 标 Experimental；pi 组织与生态更久，已有第三方包 | pi-durable（生态） |
| 生态 | 新兴插件生态（DeepSeek 官方） | npm 已有 `pi-durable-subagents`、`pi-missions`、`pi-goal-x` 等 | **pi-durable** |
| 跨进程共享 | 单进程/单写者（storage 锁） | 单进程/单写者；DO 后端可远程 | 平手（DO 略优） |

## 6. 对 Whale 的选型含义

1. **TaskMap 语义内核**：如果目标是把 TaskSpace 做成"发布级、可迁移、带任务面板"的一等能力，pi-durable 的原语更贴近，能省掉我们在 DSH 上手工实现的任务状态机与迁移边界。
2. **产品形态（GUI 安装即用）**：用户已明确期望 GUI 安装插件即生效；这条 DSH 已实测打通（内建 Web GUI + 客户端插件运行时加载 + extension 槽位）。pi-durable 侧要自建 Web 前端（pi 的 RPC/SDK 只提供数据面与受限 UI 子协议）。
3. **DeepSeek 极致适配**：DSH 是 DeepSeek 官方 harness，会话日志 wire 扩展与缓存策略是第一方；pi-ai 的 DeepSeek 路径需要另行验证缓存/成本。
4. **可行组合**：
   - **A（推荐先做）**：留在 DSH 交付 GUI 产品；用 `ctx.storage` 双写把持久性边界收进纯插件（工作量为一个 storage provider 插件）。
   - **B**：以 pi-durable 为内核另起 Whale 运行时，自建 Web 前端（可参考 DSH 的 projection/槽位设计），换取原生 durable task 与文档迁移。
   - **C**：双轨——DSH 交付，pi-durable 作为 TaskMap 语义实现参考，等 pi-durable 的 GUI 生态或 pi "pico-v5" 演进明朗后再评估迁移。
5. **未决问题**（选型前应补）：
   - pi-durable 工具批级准入能否在不改库的前提下实现整轮零派发；
   - pi-ai 的 DeepSeek provider 缓存行为与 DSH 第一方 wire 的差距；
   - pi-durable 的 "Experimental" 版本节奏与升级成本（是否有迁移指南）；
   - pi CLI 下一代（`pico-v5-handoff`）是否会统一到 pi-durable，从而带来现成 TUI/前端。

## 7. 证据索引

| 证据 | 路径 |
|---|---|
| DSH 可行性报告（32 用例） | `docs/experiments/2026-10-11-dsh-taskspace-plugin-feasibility-spike.md` |
| DSH spike 源码 | `docs/experiments/dsh-taskspace-spike/` |
| pi-durable 原语实验 | `docs/experiments/pi-durable-probe/`（`probe.mjs` + `probe-output.txt`） |
| DSH 上游 | `deepseek-ai/deepseek-harness` @ `d743267388641bc76f17c45ce8b4c231aed1d32c` |
| pi-durable 上游 | `@earendil-works/pi-durable@1.1.0`（含 `docs/spec.md` 规范） |
