# DSH TaskMap/TaskSpace 插件 spike（E1 机制冒烟）

- 状态：`passed`（26/26 确定性测试通过，0 次真实模型请求，0 行 DSH 上游文件修改）
- 日期：2026-10-11（Wave 2 扩展）
- 上游：`deepseek-ai/deepseek-harness` @ `d743267388641bc76f17c45ce8b4c231aed1d32c`（release-0.2.1-alpha.2，MIT）
- 本目录是 spike 源码归档；完整结论见 `../2026-10-11-dsh-taskspace-plugin-feasibility-spike.md`

## 目的

在 DSH 上以**纯插件**复刻 WhaleCode TaskMap/TaskSpace 的核心思路（不照抄 demo 细节），验证：

1. 批级原子准入（control 首位、manifest 与 sibling 一致、不匹配零派发、CAS）；
2. 线程内 standard ↔ taskspace 实时切换与隔离；
3. canonical map 持久化、重放、无插件读取者兼容；
4. work-node DAG、ready frontier、finish/reopen 门禁；
5. action reservation + deferred mutation（sibling 成功才生效）；
6. 上下文投影策略（on-request / append / always）；
7. viewer 客户端视图（projection wire snapshot）；
8. 进程重启后 resume 续写 map；
9. 多 agent：lead 独占 map，spawn 子 agent 工作、结果回流。

## 复现步骤

```sh
mkdir -p /home/zhangxu/dsh-harness-spike && cd /home/zhangxu/dsh-harness-spike
git init -q && git remote add origin https://github.com/deepseek-ai/deepseek-harness.git
git fetch --depth 1 origin d743267388641bc76f17c45ce8b4c231aed1d32c && git checkout -q FETCH_HEAD
pnpm install --frozen-lockfile
pnpm run build:native-system          # JSONL 持久化的文件锁原生插件
mkdir -p packages/experimental/whale-taskspace-spike/{src,tests}
cp <本目录>/src/*.ts   packages/experimental/whale-taskspace-spike/src/
cp <本目录>/tests/*.ts packages/experimental/whale-taskspace-spike/tests/
pnpm vitest run packages/experimental/whale-taskspace-spike/tests
```

预期：`Test Files 3 passed (3) / Tests 26 passed (26)`，且 `git status --porcelain` 只有新增的 spike 目录（无 tracked 文件修改）。

## 目录

| 路径 | 内容 |
|---|---|
| `src/domain.ts` | work-node DAG、mutation 应用、frontier、有界渲染 |
| `src/batch.ts` | 批级准入校验（control 首位 / manifest 对齐 / CAS 字段） |
| `src/records.ts` | 插件记录、投影（含 wire 客户端视图）、消息 source 声明 |
| `src/index.ts` | Service 插件：control 工具、guard、模式门控、reservation、上下文策略 |
| `tests/harness.ts` | 共享测试夹具（真实 agent loop + tool pipeline + session + projection + JSONL） |
| `tests/spike.spec.ts` | Wave 1：三个卡点回归（10 用例） |
| `tests/taskmap-v2.spec.ts` | Wave 2：DAG/CAS/门禁/reservation/上下文策略/viewer（14 用例） |
| `tests/lifecycle.spec.ts` | Wave 2：resume 续写 + 多 agent 协调（2 用例） |

## 结论速览

| 能力 | 结论 | 是否需改 DSH 本体 |
|---|---|---|
| 批级原子准入 + CAS（零派发） | 纯插件可实现，含并行调度对抗 | 否 |
| standard ↔ taskspace 切换 + 线程隔离 | 纯插件可实现（`agent.ctx.tools.restrict` + guard，无需 preset 重绑） | 否 |
| canonical map 持久化/重放/兼容 | 纯插件可实现（`appendPluginRecord` + `sessionProjections`） | 否 |
| DAG + frontier + finish/reopen 门禁 | 纯插件可实现 | 否 |
| action reservation + deferred mutation | 纯插件可实现（`tools/result` 归因） | 否 |
| 上下文投影策略（on-request/append/always） | 纯插件可实现（`agent.inject` / `systemPrompt.context` / `section`） | 否 |
| viewer 客户端视图 | 纯插件可实现（projection `wire` + `snapshot()`） | 否 |
| 进程重启 resume 续写 | 纯插件可实现（JSONL + projection 重放） | 否 |
| 多 agent 协调（lead 独占 map） | 纯插件可实现（subagent spawn + parent delivery） | 否 |

自带测试覆盖率（DSH 原生 gate 要求 per-file 100%）：本 spike 当前 statements 88.0% / branches 77.4% / lines 95.9%。差额主要是错误分支用例，属机械补齐；作为 out-of-tree 插件由我们自定门槛，若要合入上游需补到 100%。
