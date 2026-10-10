# DSH TaskSpace 插件可行性 spike（E1 机制冒烟）

- 状态：`passed`（10/10 确定性测试通过，0 次真实模型请求，0 行 DSH 上游文件修改）
- 日期：2026-10-11
- 上游：`deepseek-ai/deepseek-harness` @ `d743267388641bc76f17c45ce8b4c231aed1d32c`（release-0.2.1-alpha.2，MIT）
- 本目录是 spike 源码归档；完整结论见 `../2026-10-11-dsh-taskspace-plugin-feasibility-spike.md`

## 目的

回答一个问题：TaskSpace 的三个已知卡点能否**只用 DSH 插件**解决，还是必须改 DSH 本体（core）。

## 复现步骤

```sh
git clone --depth 1 https://github.com/deepseek-ai/deepseek-harness.git /tmp/deepseek-harness
cd /tmp/deepseek-harness
pnpm install --frozen-lockfile
pnpm run build:native-system          # JSONL 持久化的文件锁原生插件
mkdir -p packages/experimental/whale-taskspace-spike/{src,tests}
cp <本目录>/src/types.ts   packages/experimental/whale-taskspace-spike/src/types.ts
cp <本目录>/src/index.ts   packages/experimental/whale-taskspace-spike/src/index.ts
cp <本目录>/tests/spike.spec.ts packages/experimental/whale-taskspace-spike/tests/spike.spec.ts
pnpm vitest run packages/experimental/whale-taskspace-spike/tests/spike.spec.ts
```

预期：`Test Files 1 passed (1) / Tests 10 passed (10)`，且 `git status --porcelain` 只有新增的 spike 目录（无 tracked 文件修改）。

## 目录

| 路径 | 内容 |
|---|---|
| `src/types.ts` | canonical map 类型、插件记录（`plugin:whale-taskspace/*`）、批裁决规则、投影定义 |
| `src/index.ts` | Service 插件：control 工具、guard 批准入、per-agent 模式门控、map 提交 |
| `tests/spike.spec.ts` | 10 个确定性用例（真实 agent loop + tool pipeline + session log + projection + JSONL 持久化，LLM 用 MockAdapter） |

## 结论速览

| 卡点 | 结论 | 是否需改 DSH 本体 |
|---|---|---|
| 批级原子准入（control 必须首位、manifest 与 sibling 数量/顺序/名称一致、不匹配整批零派发） | 纯插件可实现，含并行调度对抗场景 | 否 |
| standard ↔ taskspace 实时切换 + 线程隔离 | 纯插件可实现（`agent.ctx.tools.restrict` + guard，不需要 preset 重绑） | 否 |
| canonical map 持久化 + 重放 + 无插件读取者兼容 | 纯插件可实现（`appendPluginRecord` + `sessionProjections`） | 否 |

未覆盖但已识别为需进一步验证的项：真实 DeepSeek 前缀缓存影响、跨会话全局 map 归属、终端 TUI、RPC 暴露、上游覆盖率门禁。
