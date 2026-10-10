# DSH 插件化 TaskSpace 可行性实验报告

```text
experiment_level: E1
sample_set_id: dsh-taskspace-plugin-spike_deterministic_1_1
sample_names: dsh-plugin-batch-admission, dsh-plugin-parallel-admission, dsh-plugin-mode-switch, dsh-plugin-durable-state
repeats_per_sample: 1
runner_family: internal-matrix
runner_entrypoint: pnpm vitest run packages/experimental/whale-taskspace-spike/tests/spike.spec.ts
run_root: /tmp/deepseek-harness
score_valid: not_applicable
human_audit_status: not_required
allowed_claim: 在 DeepSeek Harness 0.2.1-alpha.2 上，TaskSpace 的三个已知卡点可由纯插件（零上游文件改动）实现并通过确定性测试
```

- 状态：`passed`
- 日期：2026-10-11
- 上游：`deepseek-ai/deepseek-harness` @ `d743267388641bc76f17c45ce8b4c231aed1d32c`（release-0.2.1-alpha.2，MIT）
- 源码归档：[`dsh-taskspace-spike/`](./dsh-taskspace-spike/README.md)
- 真实模型请求：0；真实费用：0；DSH 上游 tracked 文件改动：0

## 1. 问题

上一轮分析识别出把 TaskSpace 做成 DSH agent 预制模式插件的 3 个疑似卡点：

| # | 卡点 | 原怀疑 |
|---|---|---|
| B1 | 批级原子准入：`taskspace_control` 必须首位、manifest 与同批 sibling 的数量/顺序/工具名一致、不匹配时整批零派发 | DSH 的 `tools/pre-execute` 是逐调用 waterfall，可能没有整批 hook，需要改 core |
| B2 | 线程内 standard ↔ taskspace 实时切换 | DSH 的 agent preset 在会话首轮前绑定、已存在 Agent 保留旧 revision，可能不支持热切换 |
| B3 | canonical map 的持久化、CAS、重放与"无插件读取者"兼容 | 出仓库插件无法向 `SessionEventMap`/`KNOWN_SESSION_EVENT_TYPES` 增加事件，可能需要改 core |

本实验用一个**只新增、不修改**的上游目录 `packages/experimental/whale-taskspace-spike/` 逐条验证。

## 2. 实验边界（防止口径误读）

- 这是 E1 机制冒烟：证明"纯插件路径可以执行并产出正确产物"，**不**构成任何效用、正确率或发布结论。
- LLM 用 `MockAdapter` 脚本化，唯一 mock 是模型边界；agent loop、tool pipeline、session log、projection registry、JSONL 持久化、文件锁全部为真实实现。
- 无真实 provider 请求，因此不触发真实运行预算门禁；也未测量 DeepSeek 前缀缓存与 token 成本。
- 上游为 developer preview（官方声明会有 breaking changes），本结论只对上述 commit 有效。

## 3. 结果

`Test Files 1 passed (1)` / `Tests 10 passed (10)` / `Duration ~0.9s`（`pnpm vitest run`）。

| # | 用例 | 断言要点 | 结果 |
|---|---|---|---|
| 1 | control 不在首位 → 整批零派发 | probe 执行 0 次；2 条结果均为错误且带拒绝原因 | ✅ |
| 2 | manifest 与 sibling 不匹配（1 声明 vs 2 sibling） | probe 执行 0 次；3 条结果全错误；map 未创建 | ✅ |
| 3 | **并行对抗**：2 个 `isConcurrencySafe` probe 排在 control 之前 | probe 执行 0 次；3 条结果全错误 | ✅ |
| 4 | `read_map` 与 sibling 同响应 | probe 0 次；2 条结果均含 "only call" | ✅ |
| 5 | manifest 匹配 → 整批提交 | probe 执行 1 次；map revision 1→2；node 状态 `done`；返回 receipt | ✅ |
| 6 | 同进程双线程模式隔离 | standard 线程请求无 control 工具、调用被拒；taskspace 线程请求含 control 工具、调用进入领域逻辑 | ✅ |
| 7 | 单线程 standard→taskspace→standard 切换 | 三个请求的 `tools` schema 依次为 无/有/无；standard 下调用被 guard 拒绝 | ✅ |
| 8 | 持久化 + 无插件读取者 | 记录全部 `ignorable===true`；`KNOWN_SESSION_EVENT_TYPES` 不含该类型；不装插件的 Context 能正常 open/read；纯 fold 重放结果与在线投影逐字段相等 | ✅ |
| 9 | 普通批次不受影响 | 非 TaskSpace 批次正常执行、无错误 | ✅ |
| 10 | 工具注册卫生 | 插件在普通 harness 上注册 `probe` 与 `taskspace_control` | ✅ |

零上游改动证据（实验后 `git status --porcelain`）：

```text
?? packages/experimental/whale-taskspace-spike/
```

`git diff --name-only HEAD` 为空，即没有任何 tracked 文件被修改。

## 4. 关键机制（卡点如何被化解）

### B1 批级原子准入

- 不依赖 per-call pre-execute 的时序，而是在 **guard 中读取当前 session log 的最后一条 `assistant/message`**，取出该批次的全部 tool call，重算整批裁决。
- 因为 assistant message 在工具派发前就已 commit，裁决对所有 sibling 是**同一个既定事实**；每个 sibling 在各自的 `tools/pre-execute` 后由 `ctx.tools.guard()` 独立拒绝，滚动调度器无法让任何一个先跑（用例 3 专门用并行安全工具 + control 后置构造了对抗顺序）。
- guard 是单调拒绝（`ToolGuard` 返回 reason 即 deny，后续 listener 无法翻案），符合"不匹配整批关闭"的语义；拒绝原因是模型可见的 `Error: ...` 内容。
- 同一机制可原样扩展到 `finish_map` / `reopen_map` / action reservation 的批规则。

### B2 模式切换与隔离

- 实际不需要 agent preset：插件用 `agent.ctx.tools.restrict({ deny: [CONTROL_TOOL] })` 对**单个 agent scope** 隐藏控制工具，模式翻转时 dispose 该 restriction 即可；下一个请求的 tool schema 立即变化（用例 7）。
- 模式本身是可重放状态：`plugin:whale-taskspace/mode` 记录 → 投影 → `agent/created` 时按投影恢复，因此 resume 后模式不丢。
- guard 额外拦截标准模式下对 control 的直接调用（模型幻觉调用名也不放过），并且限制只在目标 agent scope 生效（用例 6 双线程隔离）。
- 结论修正：preset 的"首轮前绑定"限制**不是**卡点，preset 仍可用于初始组合（工具/提示词/技能），热切换由 scope restriction 承担。

### B3 持久化、重放与兼容

- 出仓库插件用官方渠道 `appendPluginRecord(session, type, data)` 写 `plugin:` 命名空间的记录；语义上是 `ignorable`，永不进入模型可见 surface，fork/resume 随日志携带，不认识它的读取者安全跳过而不是拒绝重建。
- `ctx.sessionProjections.register(ProjectionDefinition)` 注册纯同步 fold（whole-value 快照、`stateVersion`、`stateOf(session, key)`）；本 spike 的 map 状态由 `mapSchema.safeParse` 校验后整体替换。
- 因此不需要第二个状态库，也不需要 CAS：单一 session log 天然是单写者；跨读一致性由投影的 `asOfSeq` 快照提供（用例 8 验证了持久化重放等价）。
- 若将来需要跨线程共享同一张 map，DSH 的既有模式是"lead session 拥有权威记录 + 子会话引用"（`agent-team` 正是如此：Team 事件只写 Lead session），或使用 `ctx.storage` domain KV；两者都是插件层能力。

## 5. 判定

| 卡点 | 判定 | 依据 |
|---|---|---|
| B1 批级原子准入 | **纯插件可解决** | 用例 1–5 |
| B2 模式热切换与隔离 | **纯插件可解决**（且不需要 preset 重绑） | 用例 6–7 |
| B3 canonical 状态持久化/重放/兼容 | **纯插件可解决** | 用例 8, 9 |

在本实验覆盖范围内，**没有出现"必须改 DSH 本体"的卡点**。

## 6. 未覆盖与残余风险

| 项 | 状态 | 说明 |
|---|---|---|
| DeepSeek 前缀缓存 / token 成本 | 未验证 | 切换模式会改变 tool schema；DSH 有 `toolUpdate`/`systemPromptUpdate` 路由能力与 `request/header` series 记账，但实际缓存影响需真实 provider 对照实验（受预算门禁约束） |
| 跨会话全局 map 与并发写 | 未验证 | 本 spike 采用 session 内权威（DSH grain）；多线程共享需按 lead-session 或 `ctx.storage` 模式另做设计验证 |
| 终端 TUI | 未验证 | DSH 自带 web/desktop 客户端；终端 TUI 需作为客户端插件新建，与 TaskSpace 语义无关但是产品工作量 |
| app-server/RPC 暴露 read/mode | 未验证 | 需用远程服务/typert 模式把插件服务暴露给 SDK/ACP 客户端；属插件层但未在本 spike 覆盖 |
| 上游门禁适配 | 未验证 | 把 spike 合入 DSH 仓需过 per-file 100% 覆盖率、doc-sync、产品包策略等门禁；作为 out-of-tree 插件则由我们自定 CI 门槛 |
| 上游 API 稳定性 | 风险 | DSH 官方声明 developer preview 会有 breaking changes；`PluginRecordMap`/`tools.restrict`/projection 契约需随版本迁移 |

## 7. DSH 中的摩擦点（实证，非阻塞）

1. JSONL 持久化依赖 `native/system` 原生插件，首次跑持久化用例前必须 `pnpm run build:native-system`。
2. `MockAdapter` 没有多 tool-call 单响应辅助函数，需要自写约 15 行构造器。
3. 插件状态暴露给测试需要 `Service` 子类（普通对象 `apply(ctx)` 拿到的是 fork 后的子 context，父 context 上的 WeakMap 查不到）。
4. Context 销毁 API 是 `ctx.fiber.dispose()`，没有 `ctx.dispose()`。
5. 出仓库插件真正的安装路径是 `dsh plugin --profile <name> add <pkg>`；本 spike 放在 `packages/experimental/` 是为了复用仓库内 tsconfig paths 解析。

## 8. 复现

见 [`dsh-taskspace-spike/README.md`](./dsh-taskspace-spike/README.md)，包含 clone、安装、构建原生插件、拷贝源码、运行命令的完整步骤。

## 9. 对底座决策的含义

- "TaskSpace 作为 DSH 插件"在**技术可行性**上已通过机制验证：不需要改 DSH 本体，差异代码全部位于 upstream 之外，这正是可维护性相对 Codex fork overlay（56 路径侵入、每次上游 sync 重放）的结构性差异。
- 该结论**不自动支持**整体切换底座：缓存/成本、TUI、RPC、跨会话状态、上游 pre-stable 风险仍是待验证项，且 Codex 线路上的既有资产（U 系列迁移、R4–R8、cache 基线）不会因为插件可行而自动迁移。
- 建议按序推进：① 用同一套 spike 模式补 B4/B5（跨会话 map、RPC/viewer）；② 在预算批准后用同 corpus 做 DeepSeek 缓存对照；③ 通过后再评估底座级迁移方案与重基线计划。
