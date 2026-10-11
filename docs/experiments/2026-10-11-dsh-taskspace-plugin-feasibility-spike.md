# DSH 插件化 TaskSpace/TaskMap 可行性实验报告

```text
experiment_level: E1
sample_set_id: dsh-taskspace-plugin-spike_deterministic_1_1
sample_names: dsh-plugin-batch-admission, dsh-plugin-parallel-admission, dsh-plugin-mode-switch, dsh-plugin-durable-state, dsh-plugin-taskmap-dag, dsh-plugin-reservations, dsh-plugin-context-projection, dsh-plugin-resume-continuity, dsh-plugin-multi-agent
repeats_per_sample: 1
runner_family: internal-matrix
runner_entrypoint: pnpm vitest run packages/experimental/whale-taskspace-spike/tests
run_root: /home/zhangxu/dsh-harness-spike
score_valid: not_applicable
human_audit_status: not_required
allowed_claim: 在 DeepSeek Harness 0.2.1-alpha.2 上，TaskSpace/TaskMap 的核心思路可由纯插件（零上游文件改动）实现并通过确定性测试
```

- 状态：`passed`
- 日期：2026-10-11（Wave 1：卡点验证；Wave 2：TaskMap 思路复刻）
- 上游：`deepseek-ai/deepseek-harness` @ `d743267388641bc76f17c45ce8b4c231aed1d32c`（release-0.2.1-alpha.2，MIT）
- 源码归档：[`dsh-taskspace-spike/`](./dsh-taskspace-spike/README.md)
- 测试规模：**26 个确定性用例全部通过**；真实模型请求 0；真实费用 0；DSH 上游 tracked 文件改动 0

## 1. 问题

WhaleCode 当前 TaskMap 仍是 demo。真正要验证的不是 demo 细节，而是**思路能否以 DSH 插件复刻**，以及有没有卡点必须改 DSH 本体。

Wave 1 针对三个已知卡点：

| # | 卡点 | 原怀疑 |
|---|---|---|
| B1 | 批级原子准入：control 必须首位、manifest 与同批 sibling 数量/顺序/工具名一致、不匹配整批零派发 | DSH 的 `tools/pre-execute` 是逐调用 waterfall，可能没有整批 hook |
| B2 | 线程内 standard ↔ taskspace 实时切换 | DSH 的 agent preset 首轮前绑定，可能不支持热切换 |
| B3 | canonical map 的持久化、重放与"无插件读取者"兼容 | 出仓库插件无法向 `SessionEventMap` 添加事件 |

Wave 2 复刻 TaskMap 思路：

| # | 能力 |
|---|---|
| V1 | work-node DAG（blockedBy 依赖）+ ready frontier + finish/reopen 门禁 |
| V2 | CAS（`expectedRevision`）与 state 前置条件在整个批次上零派发拒绝 |
| V3 | action reservation（确定性 id）+ deferred mutation（sibling 成功才提交） |
| V4 | 上下文投影策略 on-request / append / always |
| V5 | viewer 客户端视图（projection `wire` + `snapshot()`） |
| V6 | 进程重启后 resume 续写 map |
| V7 | 多 agent：lead 独占 map，spawn 子 agent 工作、结果回流 |

## 2. 实验边界（防止口径误读）

- 这是 E1 机制冒烟：证明"纯插件路径可以执行并产出正确产物"，**不**构成效用、正确率或发布结论。
- LLM 用 `MockAdapter` 脚本化，唯一 mock 是模型边界；agent loop、tool pipeline、session log、projection registry、JSONL 持久化、文件锁、subagent runtime 全部为真实实现。
- 无真实 provider 请求；未测量 DeepSeek 前缀缓存与 token 成本。
- 上游为 developer preview，结论只对上述 commit 有效。

## 3. Wave 1 结果：三个卡点全部为纯插件可解

| # | 用例 | 断言要点 | 结果 |
|---|---|---|---|
| 1 | control 不在首位 → 整批零派发 | probe 执行 0 次；2 条结果均为错误 | ✅ |
| 2 | manifest 与 sibling 不匹配 | probe 0 次；3 条结果全错误；map 未创建 | ✅ |
| 3 | 并行对抗：2 个 `isConcurrencySafe` probe 排在 control 之前 | probe 0 次 | ✅ |
| 4 | `read_map` 与 sibling 同响应 | probe 0 次；结果含 "only call" | ✅ |
| 5 | manifest 匹配 → 整批提交 | probe 1 次；revision 1→2；node `done`；返回 receipt | ✅ |
| 6 | 同进程双线程模式隔离 | standard 无工具/被拒；taskspace 有工具/进入领域逻辑 | ✅ |
| 7 | standard→taskspace→standard 切换 | 请求 schema 依次 无/有/无；standard 直调被拒 | ✅ |
| 8 | 持久化 + 无插件读取者 | 记录 `ignorable`；不装插件的 Context 正常 open/read；fold 重放与在线投影相等 | ✅ |
| 9 | 普通批次不受影响 | 非 TaskSpace 批次正常执行 | ✅ |
| 10 | 工具注册卫生 | 普通 harness 上注册 `probe` 与 `taskspace_control` | ✅ |

**关键机制**：guard 读取已提交的 `assistant/message` 重算整批裁决 —— 裁决在派发前是既定事实，逐调用拒绝即可整批零派发；模式用 `agent.ctx.tools.restrict()` 做 per-agent 隐藏，**不需要 preset 重绑**；持久化用官方 `appendPluginRecord`（自动 `ignorable`）+ `sessionProjections` 纯 fold。

## 4. Wave 2 结果：TaskMap 思路可完整复刻

| # | 用例 | 断言要点 | 结果 |
|---|---|---|---|
| 11 | DAG 初始化 + frontier | blockedBy 图 + goal 保存；完成 n1 后 frontier 变为 `['n2']` | ✅ |
| 12 | 初始化即拒绝环 | 依赖环被拒，map 保持 null | ✅ |
| 13 | `set_blocked_by` 造成环 | 拒绝且无部分提交（revision 不变） | ✅ |
| 14 | CAS：过期 `expectedRevision` | probe 0 次；两条结果均为 `revision conflict`；revision 不变 | ✅ |
| 15 | finish/reopen 门禁 | 未完成节点时 finish 被整批拒绝；terminal 后 execute 被整批拒绝；reopen 后恢复 | ✅ |
| 16 | deferred mutation 归因 | 失败的 sibling 不提交完成；成功的 sibling 才把 node 置 done；reservation id 确定性 | ✅ |
| 17 | 上下文策略 | append 注入提交摘要；on-request 不注入；always 无变更也渲染；standard 下 guidance 消失 | ✅ |
| 18 | viewer 视图 | `snapshot(session, ['whaleTaskspace'])` 返回 mapId/revision/counts/nodes + `asOfSeq` | ✅ |
| 19 | 渲染有界 | 40 节点 & maxChars 320：输出 ≤320 且带省略提示 | ✅ |
| 20 | 未知 control action | 整批拒绝，零派发 | ✅ |
| 21 | `read_map` 回读 | 返回 canonical map + action ledger | ✅ |
| 22 | 重复 node id / 重复添加 | 拒绝且无部分提交 | ✅ |
| 23 | 未就绪激活 / 块态迁移 | `activate_node` 未满足依赖被拒；unblock/block 路径覆盖 | ✅ |
| 24 | 冗余 finish/reopen | `already terminal` / `not terminal` 整批拒绝 | ✅ |
| 25 | **进程重启 resume** | flush+dispose 后 `ctx.agents.resume`：map revision/mode/policy 恢复，继续 CAS 提交到 revision 3，模式仍生效 | ✅ |
| 26 | **多 agent 协调** | lead 建图后 spawn 子 agent；子会话**不含任何 map 记录**（lead 独占写入）；子结果作为上下文回流；lead 提交完成节点 | ✅ |

### 思路 → DSH seam 映射（Wave 2 实证）

| TaskMap 思路 | 本 spike 实现（纯插件） | DSH 提供的机制 |
|---|---|---|
| canonical map + revision CAS | `plugin:whale-taskspace/map` 全量快照记录 | `appendPluginRecord` + `ignorable` 语义 |
| 唯一写路径 | `taskspace_control` 工具 + 整批 guard | `tools.register` / `tools.guard` / `ToolRuntime` |
| DAG 校验与 frontier | 插件域函数（纯函数，可重放） | 无本体依赖 |
| action reservation + 归因 | `plugin:whale-taskspace/actions` 全量账本；`tools/result` 监听释放 | `tools/result` observe-only 事件 |
| deferred mutation | 成功才 `applyMutations` 并追加 map 记录 | 事件溯源写入 |
| 上下文投影策略 | append → `agent.inject()`；always → `systemPrompt.context()`；guidance → `systemPrompt.section()` | 注入/动态上下文/提示段三种原生 seam |
| viewer / RPC 读模型 | projection `wire` + `snapshot(session)` | session-projection 客户端视图 |
| 进程重启续写 | session log 重放（JSONL） | `ctx.agents.resume` + projection 懒重放 |
| 多 agent 协调 | lead 会话独占 map 记录；子 agent 通过结果回流 | `ctx.subagents.startActivation`（spawn + parent delivery） |

## 5. 判定

在 Wave 1 + Wave 2 的 26 个确定性用例范围内，**没有出现"必须改 DSH 本体"的卡点**：

| 能力 | 判定 |
|---|---|
| B1 批级原子准入、B2 模式热切换、B3 canonical 状态 | 纯插件可解 |
| V1–V7（DAG/frontier、CAS 门禁、reservation、上下文策略、viewer、resume、多 agent） | 纯插件可解 |

## 6. 实验过程中被测试抓出的设计错误（证据价值）

写 Wave 2 时，真实事件流两次推翻了实现假设，说明这套 harness 能抓设计错误而不只是"跑通"：

1. **revision 单调性**：`finish_map` / `reopen_map` 最初只翻转 `terminal` 而不递增 revision，CAS 计数出现空洞；测试追踪请求序列后修正为一次提交 +1。
2. **节点就绪语义**：resume 用例里对未完成依赖的节点做 `activate_node` 被域规则正确拒绝，暴露了测试脚本的业务假设错误（而非实现错误），说明 Domain 不变式在真实路径上生效。

## 7. DSH 摩擦点（实证，非阻塞）

1. JSONL 持久化依赖 `native/system` 原生插件，首次跑持久化用例前必须 `pnpm run build:native-system`。
2. `MockAdapter` 没有多 tool-call 单响应辅助函数，需要自写约 20 行构造器。
3. 插件状态暴露给测试/其他插件需要 `Service` 子类（普通对象 `apply(ctx)` 拿到 fork 后的子 context）。
4. Context 销毁 API 是 `ctx.fiber.dispose()`，没有 `ctx.dispose()`。
5. 出仓库插件真正安装路径是 `dsh plugin --profile <name> add <pkg>`；本 spike 放在 `packages/experimental/` 只为复用仓库内 tsconfig paths。
6. `pre-step` 期间注入时序、dynamic context 与注入消息在 prefix 上的行为需要按真实 provider 验证（见残余风险 1）。

## 8. 未覆盖与残余风险

| 项 | 状态 | 说明 |
|---|---|---|
| DeepSeek 前缀缓存 / token 成本 | 未验证 | 模式切换与 `always` 策略会改变 prompt；DSH 有 `toolUpdate`/`systemPromptUpdate` 路由能力与 request-series 记账，实际缓存影响需真实 provider 对照（受预算门禁） |
| 跨会话全局 map 与并发写 | 未验证 | 本 spike 采用 DSH 的 session 内权威 + lead 独占；多 lead 共享一张图的 CAS 需按 lead-session 或 `ctx.storage` 模式另做设计验证 |
| 终端 TUI | 未验证 | DSH 自带 web/desktop 客户端；终端 TUI 需作为客户端插件新建，与 TaskMap 语义无关但是产品工作量 |
| RPC / app-server 暴露 | 未验证 | `wire` 视图已验证数据面；把服务暴露给 SDK/ACP 客户端仍需 typert 远程服务模式 |
| 上游 API 稳定性 | 风险 | 官方声明 developer preview 会有 breaking changes；`PluginRecordMap`/`tools.restrict`/`systemPrompt.context` 契约需随版本迁移 |
| 大图性能 | 未验证 | 40 节点渲染有界；未测数百节点 + 长会话的投影 fold 成本 |

## 9. 覆盖率与上游门禁

DSH 仓库自带 per-file 100% 覆盖率门禁。本 spike（out-of-tree）当前：

| 文件 | Stmts | Branch | Lines |
|---|---|---|---|
| 全部 src | 88.0% | 77.4% | 95.9% |
| `batch.ts` | 73.6% | 72.2% | 88.2% |
| `domain.ts` | 94.3% | 82.5% | 98.8% |
| `index.ts` | 90.0% | 81.1% | 97.8% |
| `records.ts` | 94.1% | 64.7% | 93.8% |

差额集中在错误分支（无效 manifest 形态、非法 payload、异常 mutation 等），属机械补齐；作为 out-of-tree 插件由我们自定门槛，若要合入上游需补到 100%。命令：
`pnpm vitest run packages/experimental/whale-taskspace-spike/tests --coverage --coverage.include='packages/experimental/whale-taskspace-spike/src/**/*.ts'`

## 10. 复现

见 [`dsh-taskspace-spike/README.md`](./dsh-taskspace-spike/README.md)：clone 固定 commit、安装、构建原生插件、拷贝源码、运行测试。

## 11. 对底座决策的含义

- **思路复刻成立**：TaskSpace/TaskMap 的核心（唯一写路径、批级准入、CAS、DAG、reservation/归因、上下文投影、重放、多 agent 独占）全部能在 upstream 之外以插件实现，且能被确定性测试完整覆盖。这正是相对 Codex fork overlay（56 路径侵入、每次上游 sync 重放）的结构性维护性差异。
- **结论不自动支持整体切换底座**：缓存/成本、TUI、RPC、跨会话状态、上游 pre-stable 风险仍待验证；Codex 线路上的既有资产（U 系列迁移、R4–R8、cache 基线）不会自动迁移。
- 建议下一步按序推进：① 补 B4/B5（跨会话 map、RPC/viewer 远程暴露）；② 预算批准后用同 corpus 做 DeepSeek 缓存对照（尤其验证 `append`/`always` 两种策略的前缀影响）；③ 通过后再评估底座级迁移方案与重基线计划。
