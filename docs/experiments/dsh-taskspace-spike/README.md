# DSH TaskMap/TaskSpace 插件 spike（E1 机制冒烟）

- 状态：`passed`（32/32 确定性测试通过，0 次真实模型请求，0 行 DSH 上游文件修改）
- 日期：2026-10-11（Wave 1 卡点 → Wave 2 TaskMap 复刻 → Wave 3 剩余边界 → Wave 4 GUI 激活路径）
- 上游：`deepseek-ai/deepseek-harness` @ `d743267388641bc76f17c45ce8b4c231aed1d32c`（release-0.2.1-alpha.2，MIT）
- 本目录是 spike 源码归档；完整结论见 `../2026-10-11-dsh-taskspace-plugin-feasibility-spike.md`

## 目的

在 DSH 上以**纯插件**复刻 WhaleCode TaskMap/TaskSpace 的核心思路（不照抄 demo 细节），并找出真正必须改 DSH 本体的边界：

1. 批级原子准入（control 首位、manifest 与 sibling 一致、不匹配零派发、CAS）；
2. 线程内 standard ↔ taskspace 实时切换与隔离；
3. canonical map 持久化、重放、无插件读取者兼容；
4. work-node DAG、ready frontier、finish/reopen 门禁；
5. action reservation + deferred mutation（sibling 成功才生效、取消不泄漏）；
6. 上下文投影策略（on-request / append / always）；
7. viewer 客户端视图 + 变更通知；
8. 进程重启后 resume 续写 map；
9. 多 agent：lead/child 协调、**跨会话权威写入**、fork 血缘；
10. 外部暴露：Typert source-mode RPC、slash 命令；
11. GUI 激活路径：安装 bundle 即出现 TaskMap 面板（客户端插件 + 侧栏扩展位 + projection 数据面）。

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

预期：`Test Files 6 passed (6) / Tests 32 passed (32)`，且 `git status --porcelain` 只有新增的 spike 目录（无 tracked 文件修改）。

## 目录

| 路径 | 内容 |
|---|---|
| `src/domain.ts` | work-node DAG、mutation 应用、frontier、有界渲染 |
| `src/batch.ts` | 批级准入校验（control 首位 / manifest 对齐 / CAS 字段） |
| `src/records.ts` | 插件记录、投影（含 wire 客户端视图与 binding）、消息 source 声明 |
| `src/index.ts` | Service 插件：control 工具、guard、模式门控、reservation、上下文策略、跨会话权威、`@Remote` 读接口、命令插件 |
| `tests/harness.ts` | 共享测试夹具（真实 agent loop + tool pipeline + session + projection + JSONL + commands） |
| `tests/spike.spec.ts` | Wave 1：三个卡点回归（10 用例） |
| `tests/taskmap-v2.spec.ts` | Wave 2：DAG/CAS/门禁/reservation/上下文策略/viewer（14 用例） |
| `tests/lifecycle.spec.ts` | Wave 2：resume 续写 + 多 agent 协调（2 用例） |
| `tests/authority.spec.ts` | Wave 3：跨会话权威写入 + 取消时 reservation 完整性（2 用例） |
| `tests/exposure.spec.ts` | Wave 3：Typert source-mode RPC + 变更通知 + slash 命令 + fork 血缘（2 用例） |
| `tests/gui.spec.ts` | Wave 4：安装包的客户端入口进入 Web boot graph 并被 `/plugins` 服务；第三方 tab 类型以 extension band 抢占地址（2 用例） |
| `src/client/index.tsx` | GUI 客户端入口（构建为 `lib/client.js`）：注册 `taskmap` 侧栏 tab 类型 + body/title 槽位，用 `useProjection('whaleTaskspace')` 读宿主投影 |

## 结论速览

| 能力 | 结论 | 是否需改 DSH 本体 |
|---|---|---|
| 批级原子准入 + CAS（零派发） | 纯插件可实现，含并行调度对抗 | 否 |
| standard ↔ taskspace 切换 + 线程隔离 | 纯插件可实现（`agent.ctx.tools.restrict` + guard） | 否 |
| canonical map 持久化/重放/兼容 | 纯插件可实现（`appendPluginRecord` + `sessionProjections`） | 否 |
| DAG + frontier + finish/reopen 门禁 | 纯插件可实现 | 否 |
| action reservation + deferred mutation | 纯插件可实现；取消时通过 `tools/result` 正常释放，无泄漏 | 否 |
| 上下文投影策略（on-request/append/always） | 纯插件可实现（`agent.inject` / `systemPrompt.context` / `section`） | 否 |
| viewer 视图 + 变更通知 | 纯插件可实现（projection `wire` + `snapshot()` + `onChanged`） | 否 |
| 进程重启 resume 续写 | 纯插件可实现（JSONL + projection 重放） | 否 |
| 跨会话权威写入（child 写 lead 的 log） | 纯插件可实现（`ctx.sessions.get` + `appendPluginRecord`） | 否 |
| fork 血缘（继承 map、分支自己的 lineage） | 纯插件可实现（fork provider 种子携带 ignorable 记录） | 否 |
| RPC 读接口 | 纯插件可实现（`TypertRemoteService` + `@Remote`，gateway 运行时 source-mode 发现） | 否 |
| slash 命令（`/taskspace`、`/map-projection`） | 纯插件可实现（独立 commands 角色包） | 否 |
| GUI 安装即用（客户端入口加载 + 面板注册 + 数据面） | 纯插件可实现 | 否 |
| **跨 DSH 版本格式迁移的持久性保证** | **不在纯插件边界内** | **需要上游声明事件**（或插件侧双写 `ctx.storage`） |

## GUI 激活路径（Wave 4）

用户预期是"在 DSH GUI 里安装插件就能用上 TaskMap"，该路径已确认可行，且每一步都有 DSH 原生契约支撑：

| 步骤 | 机制 | 验证 |
|---|---|---|
| 安装 | Web 侧栏 **Plugins 页** 的 bundle 管理：支持 registry / git / tarball / 绝对路径 / `link:`；安装后默认启用 bundle，HMR 生效或提示重启 | 上游文档（`packages/boot/plugin-manager/README.md`） |
| 宿主插件 | bundle 的 `cordis.patch.yml` 插入宿主服务行（本 spike 的 `WhaleTaskspaceSpike`） | 本 spike 全量测试 |
| 客户端入口 | 包声明 `dsh.client: { platform: 'web' }` + `exports["./client"]`；宿主 client-module 扫描把入口编入 `window.__DSH_BOOT__` 并在 `/plugins` 下服务 —— **无需重建前端** | `tests/gui.spec.ts` 用例 1 |
| GUI 面板 | `ctx.sidebarRightTabs.register({ kind: 'taskmap', priority: 'extension', patterns: ['dsh-resource://taskmap/**'] })` + `sidebar.right.pane.tab` / `.title` 槽位；extension band 优先于内置 viewer | `tests/gui.spec.ts` 用例 2 |
| 数据面 | 宿主 projection 的 `wire` 值随 `session/projection` 推送；客户端用 `useProjection('whaleTaskspace')` 读取（`ui-goal` 是同一模式的现成先例）—— **不需要生成式 Remote 客户端** | 宿主侧见 `exposure.spec.ts`；客户端模式见 `src/client/index.tsx` |

bundle 的包清单形状（本 spike 未构建客户端 bundle，真实发布需 `tsdown` 构建）：

```json
{
  "name": "@whale/dsh-taskmap",
  "exports": { ".": { "default": "./lib/index.js" }, "./client": { "default": "./lib/client.js" } },
  "dsh": {
    "bundle": { "patch": "./cordis.patch.yml" },
    "client": { "platform": "web", "inject": ["@deepseek-ai/dsh-client-ui-renderer"] }
  }
}
```

未在本 spike 覆盖：浏览器内实际渲染（DSH 自带 `test:web` 浏览器快照门禁可覆盖）、客户端 bundle 的构建产物。

## 唯一确认的边界：格式迁移持久性

`appendPluginRecord` 写入的是 `plugin:` 命名空间的 **ignorable** 记录：

- 优点：读者不装插件也能安全跳过、fork/resume 携带、不进模型可见面。
- 限制（上游明文）：*"Experimental packages keep state there that their owners can lose: a format migration retains records on a best-effort basis, so a release package declares its events instead."*（`.agents/notes/implemented/architecture/2026-08-30-retain-ignorable-external-session-events.md`，另见 `docs/experimental-persistence-catalog.md`）

因此：

- **今天即可用**：map 状态、CAS、重放、resume、fork 血缘全部验证通过。
- **若要"发布级、跨 DSH 格式迁移不丢"的保证**：需要把 map 事件升格为 `SessionEventMap` 成员（仓库内/上游声明），或插件侧把 canonical 状态镜像到 `ctx.storage` 并可从日志重建。前者是对 DSH 的贡献，后者仍是纯插件。

自带测试覆盖率（DSH 原生 gate 要求 per-file 100%）：statements 88.0% / branches 77.1% / lines 96.3%。差额集中在错误分支，属机械补齐；作为 out-of-tree 插件由我们自定门槛。
