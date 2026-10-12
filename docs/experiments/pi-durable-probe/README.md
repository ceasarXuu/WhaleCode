# pi-durable TaskMap 原语实验（E1 机制冒烟）

- 状态：`passed`（0 次真实模型请求；仅使用 memory/JSONL 本地存储）
- 日期：2026-10-11
- 上游：`@earendil-works/pi-durable@1.1.0`（npm，MIT，"Experimental. The API changes without notice between releases."）
- 依赖：`@earendil-works/pi-ai`、`@earendil-works/chord`

## 目的

验证"TaskMap 建在 pi-durable 上"所需的原语是否原生存在，特别是 DSH 侧唯一超出插件边界的**跨版本持久性**问题。

## 复现

```sh
mkdir -p /home/zhangxu/pi-durable-probe && cd /home/zhangxu/pi-durable-probe
npm init -y && npm i @earendil-works/pi-durable @earendil-works/pi-ai @earendil-works/chord
cp <本目录>/probe.mjs .
node probe.mjs
```

预期输出见 `probe-output.txt`。

## 验证到的原语（probe-output.txt 为原始证据）

| 能力 | 结果 |
|---|---|
| session 级 canonical 文档（跨会话共享权威） | ✅ 第二个 conversation 读到同一份 map（`shared.otherConversationSees`） |
| 原子提交（transcript entry + 文档同一 commit） | ✅ `root.commit(tx => { tx.doc(...) ... })` 后快照一致 |
| 文档 live watch（UI 数据面） | ✅ 提交后收到 `revision: 2` 帧 |
| 文档版本迁移（v1 → v2，跨进程重开） | ✅ `migrate()` 后快照 `{revision:2, nodes:[...], status:"open"}` |
| durable task 父子等待（waiting + failFast + owner） | ✅ 父任务 `waiting on 2`、两个子任务 `running` 且 `owner: 14`；最终父任务 `completed`，子结果 `["completed","completed"]` |
| task graph 可观察（任务面板数据面） | ✅ `harness.taskGraph()` + `subscribe` 观察到 等待→收束→运行 的状态帧序列 |

## 对 TaskMap 的直接含义

- **canonical map 状态**：`defineDoc`（scope: session/conversation/task，version + `migrate()`，history: latest/rewindable，fork: initial/current/asOf）是原生一等公民，不需要像 DSH 那样依赖 best-effort 插件记录。
- **action reservation / deferred mutation**：可由"父任务 + 子任务 + waiting/failFast/allSettled + checkpoint"直接表达（官方 `24-child-tasks.ts` 的支付示例就是该模式）。
- **task 面板**：`taskGraph` / `watchTaskGraph` 直接提供 UI 数据面。
- **模式切换**：官方 `27-plan-mode.ts` 示例用 `configure({ extensions: { add/remove } })` + 自有文档实现只读模式。
- **未覆盖**：批级工具准入（per-call `beforeTool` + 轮次级 `afterTools`，整轮零派发需要插件自建，与 DSH 同类问题）、浏览器 GUI（pi 产品为 TUI-first，RPC/SDK 供自建前端）、真实模型与缓存行为。
