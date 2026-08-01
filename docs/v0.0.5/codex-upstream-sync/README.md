# Codex CLI 上游追赶差异分析与合并策略

- 文档状态：分析完成，待实施决策
- 分析日期：2026-08-01
- 适用版本：WhaleCode v0.0.5
- 当前仓库提交：`c539cbe18030727ae9c48e27246c0439ad246390`
- 当前 Codex vendor 基线：`fed0a8f4faa58db3138488cca77628c1d54a2cd8`
- 建议追赶目标：Codex CLI `rust-v0.146.0` / `e363b08c9175ac1cbe5893615dd2cb9ddf95043b`
- 范围：只读差异审计、合并风险分类和追赶顺序；不代表已经实施上游同步

## 1. 执行摘要

当前不适合把官方 Codex `main` 直接合入 WhaleCode，也不适合继续长期只做零散 backport。

建议采用两段式路线：

1. 在现有基线上优先回移少量已经验证可干净应用的安全和通用修复；
2. 以官方稳定版 `rust-v0.146.0` 建立新的 vendor 候选快照，按明确的 Whale overlay 清单分组重放品牌、DeepSeek、缓存和 TaskSpace 改造。

主要依据：

- 当前 vendor 基线到稳定版相差 2,790 个提交、4,209 个文件；
- Whale 相对基线修改了 723 个路径，官方和 Whale 同时修改 496 个路径，其中 495 个最终内容仍然不同；
- Codex 是通过 codeload tarball 导入子目录，仓库根历史与官方 Git 历史没有 merge-base，普通 merge/cherry-pick 不是可靠同步机制；
- DeepSeek 官方 Codex 接入当前要求 Responses API、仅支持 `deepseek-v4-flash`，并将最低 Codex 客户端版本标为 `0.144.0`；Whale 的 provider 方向已经对齐，但 substrate 基线仍停留在约 0.125 时代；
- TaskSpace 目前不是薄插件，而是贯穿协议、状态、session、provider payload、tool sequence、app-server 和 TUI 的纵向改造，必须进行语义级重放。

## 2. 审计范围与方法

### 2.1 对比边界

| 边界 | 提交或版本 | 用途 |
| --- | --- | --- |
| Whale 当前 HEAD | `c539cbe18030727ae9c48e27246c0439ad246390` | 本次本地事实边界 |
| Whale vendor 固定基线 | `fed0a8f4faa58db3138488cca77628c1d54a2cd8` | 识别 Whale 自有修改 |
| 官方稳定目标 | `rust-v0.146.0` / `e363b08c9175ac1cbe5893615dd2cb9ddf95043b` | 建议的同步版本 |
| 官方 main | `ee0247f95a6fe2b094ba2253d82cae2a2b4c2dff` | 观察稳定版之后的演进，不作为本轮目标 |

### 2.2 使用的方法

- 比较 vendor 基线、Whale 当前 vendor 和官方源码三棵文件树；
- 统计提交、文件和行级变化；
- 检查 Whale 从 vendor import 之后的提交与同步记录；
- 对候选上游提交执行只读 `git apply --check --directory=third_party/codex-cli`；
- 检查 DeepSeek provider、model catalog、streaming、compaction 和缓存合同；
- 检查 TaskSpace canonical map、event store、tool sequence、session replay、state store、app-server 和 TUI 边界；
- 未执行真实 Whale Agent run，未产生模型费用。

## 3. 差异规模

### 3.1 官方稳定版差距

`fed0a8f4..e363b08c`：

| 指标 | 结果 |
| --- | ---: |
| 官方提交数 | 2,790 |
| 变化文件数 | 4,209 |
| 新增行 | 762,347 |
| 删除行 | 231,665 |

### 3.2 官方 main 差距

`fed0a8f4..ee0247f9`：

| 指标 | 结果 |
| --- | ---: |
| 官方提交数 | 2,982 |
| 变化文件数 | 4,380 |
| 新增行 | 824,134 |
| 删除行 | 247,364 |

三方内容路径比较：

| 路径集合 | 数量 | 含义 |
| --- | ---: | --- |
| Whale 相对基线变化 | 723 | Whale vendor overlay 的实际表面 |
| 官方相对基线变化 | 4,533 | 含新增、修改和删除路径 |
| 双方同时修改 | 496 | 潜在合并热点 |
| 双方最终内容相同 | 1 | 已自然收敛 |
| 双方最终内容不同 | 495 | 需要冲突判断或语义迁移 |
| 仅官方变化 | 4,037 | 仍需检查依赖，不能自动等同于可直接合并 |
| 仅 Whale 变化 | 227 | 主要是 TaskSpace、DeepSeek 和 Whale 产品层 |

路径不重叠只说明没有文本级重叠，不能证明提交可独立编译。若提交依赖此前的 crate 拆分、类型变化或 permission profile 迁移，仍然需要整批迁移。

## 4. 已经完成的选择性 backport

2026-05-01 已经从上游选择性吸收：

- stateful streaming `apply_patch` parser；
- Windows sandbox、process 和环境变量修复；
- MCP tool output 持久化前截断；
- MCP client shutdown/drain。

这些变更不应在新批次中重复回移。详细记录见[选择性上游回移记录](../../migration/codex-sync/2026-05-01-selective-upstream-backports.md)。

## 5. 可以快速回移的提交

下列提交已通过只读 patch apply 检查；实际实施时仍需每个主题单独提交，并运行对应 crate 测试。

| 优先级 | 官方提交 | 内容 | 主要验证 |
| --- | --- | --- | --- |
| P0 | [`2e598df6`](https://github.com/openai/codex/commit/2e598df6fcd30717cfdcd2a898746a84d365ca23) | 禁止错误自动批准 `git -C ...` | `codex-shell-command` 定向测试与审批回归 |
| P1 | [`6ec8c4a6`](https://github.com/openai/codex/commit/6ec8c4a6ecb17bc3ab10d0c5edf75494b50cab7e) | Git 元数据读取忽略 repository fsmonitor 配置 | `git-utils` 测试 |
| P1 | [`36912ce3`](https://github.com/openai/codex/commit/36912ce3de1c039f7faaddd509d0465ff644e6c1) | 修复 Windows paste burst interval | TUI 定向测试与 Windows smoke |
| P1 | [`5d7e6a25`](https://github.com/openai/codex/commit/5d7e6a2503fc71f09cea71bfca9e193e0c3fd215) | 修复 TUI borrowed slice wrapping | TUI wrapping 测试 |
| P1 | [`c86b1be3`](https://github.com/openai/codex/commit/c86b1be3cdbe12307843bcc9e7a44c1904ddcdf1) | 减少 TUI diff render clone | TUI diff render 测试 |
| P1，可选 | [`3afb185a`](https://github.com/openai/codex/commit/3afb185a4f02dab00927ad597996f3e5528cea45) | 收紧 managed network proxy bypass 默认值 | 先确认 Whale 是否启用该 runtime 路径 |
| P1，可选 | [`2dbde94a`](https://github.com/openai/codex/commit/2dbde94aa9e645715d14fff0d8d00143e236019b) | 规范化 network proxy host matching | network proxy 测试 |

不应把 `git apply --check` 通过解释为已经完成兼容性验证。它只证明补丁能应用，不证明编译、行为和安全合同成立。

## 6. 需要分批迁移的上游架构

| 上游架构变化 | 预期收益 | Whale 当前冲突 | 处理方式 |
| --- | --- | --- | --- |
| `message-history` 独立 crate | 从 core 移出 history 责任 | TaskSpace projection、replay 和 context 仍依赖 core history | 采用上游结构，增加 TaskSpace adapter |
| `prompts`、`context-fragments` | 收敛 prompt 和 context 拼装 | Whale 双 base instructions、TaskSpace manifest、cache prefix | 先冻结输出合同，再迁移内容来源 |
| shared `http-client` | 统一代理、TLS、重试和连接池 | DeepSeek、MCP、auth、provider trace 各有接入点 | 迁移 transport，保留 provider-specific policy |
| `app-server-transport` | 降低 app-server 主模块耦合 | Whale 增加 TaskSpace RPC 与 TUI adapter | 先迁 transport，再重放 RPC |
| permission profiles | 新版权限与 sandbox 统一模型 | Whale 仍保留旧 `SandboxPolicy` 接口；2026-05-01 已明确延期 | 独立迁移，不混入小型 bug backport |
| SQLite/thread-store/分页 history | 提升 resume、fork、search 和持久化性能 | TaskSpace canonical store、migration、rollout replay | 统一 transaction 和 ownership 边界后迁移 |
| MultiAgent V2 / AgentGraphStore / WorldState | 更成熟的 agent 生命周期和上下文模型 | 可能和 TaskSpace Event Store 形成双权威状态 | 先做权威状态 ADR，再接入 adapter |
| MCP 2026、Skills、Plugins、Code Mode | 能力、性能和安全提升 | tool registry 和 provider-visible tool 集合影响缓存前缀 | 按能力分批，逐批冻结 final wire |

值得作为架构迁移起点的上游提交包括：

- `2004173c`：从 core 提取 message history；
- `ba2b67f9`：集中 prompts；
- `ac67905f`：提取 context fragments；
- `9acfe896`：共享 HTTP transport；
- `41e171fc`：提取 app-server transport。

这些提交描述目标架构，不适合作为五个孤立补丁直接应用到旧基线。

## 7. DeepSeek 适配边界

### 7.1 当前已经对齐的方向

Whale 当前内置 DeepSeek provider 已经：

- 使用 `https://api.deepseek.com`；
- 使用 Responses API；
- 默认选择 `deepseek-v4-flash`；
- 暂时隐藏不支持 Codex 的 `deepseek-v4-pro`；
- 禁用 Responses WebSocket；
- 保留 1M context、755K auto compact、parallel tool calls 和 reasoning 能力。

这与 DeepSeek 官方 Codex 接入说明一致：当前只有 Flash 支持 Codex，Codex 使用 DeepSeek 原生 Responses API，官方模型配置要求最低 Codex 客户端 `0.144.0`。

### 7.2 必须保留的 Whale 产品合同

| 合同 | 当前位置 | 同步要求 |
| --- | --- | --- |
| provider identity/auth | `model-provider-info/src/lib.rs` | 保留 `deepseek`、`DEEPSEEK_API_KEY`、非 OpenAI auth |
| Whale home 隔离 | `utils/home-dir/src/lib.rs` | 保留 `WHALE_HOME`、`~/.whale`，禁止与 `.codex` 共址 |
| model catalog | `models-manager/models.json`、`models-manager/src` | 保留 Flash 默认、Pro 隐藏和 Whale 能力值 |
| Chat Completions 兼容层 | `codex-api/src/endpoint/chat_completions.rs`、`sse/chat_completions.rs` | 内置 DeepSeek 不再依赖，但自定义 Chat provider 仍可能需要 |
| reasoning/tool-call stream | `codex-api` | 不得丢失 `reasoning_content` 和 streamed tool call 组装 |
| compaction | `core/src/compact*.rs` | 保留 Flash compact 与 Whale 状态保留合同 |
| final wire/cache trace | `core/src/provider_wire_*` | 适配上游 request type，不能删除证据链 |
| request budget/usage | `core/src/client.rs` | 保留 hard limit、provider usage 和 terminal 对账 |

### 7.3 最高冲突文件

- `core/src/client.rs`；
- `core/src/session/mod.rs`；
- `core/src/session/turn.rs`；
- `core/src/session/rollout_reconstruction.rs`；
- `core/src/compact*.rs`；
- `codex-api/src/endpoint/**`；
- `codex-api/src/sse/**`；
- `model-provider-info/src/**`；
- `models-manager/**`；
- `core/src/config/**`；
- `protocol/src/**`。

这些路径同时承载 DeepSeek、TaskSpace 和缓存语义，禁止整文件选择 upstream 或 Whale 一侧。

## 8. TaskSpace 边界

### 8.1 当前实现规模

TaskSpace 当前约有 145 个 Rust/SQL 生产文件直接引用 TaskSpace/ActionMap，专属代码约 15,446 行，其中 `core/src/action_map/` 约 8,478 行。

它已经覆盖：

- canonical map schema；
- rooted DAG runtime 和 invariant；
- event store 与 replay；
- SQLite CAS store；
- TaskSpace tool schema 和 handler；
- response-level tool sequence preflight；
- session/turn terminal carrier；
- provider-visible projection；
- app-server RPC；
- TUI Action Map viewer。

因此 TaskSpace 目前不是可单独摘挂的 `PrimitiveModule`。

### 8.2 可整体保留、只适配接口的模块

- `core/src/action_map/**`；
- `core/src/tools/handlers/taskspace_control*.rs`；
- `core/src/session/taskspace_*.rs`；
- `protocol/src/taskspace.rs`；
- `tools/src/taskspace_tool*.rs`；
- `state/src/model/taskspace_map.rs`；
- `state/src/runtime/taskspace_map*.rs`；
- `tui/src/app/action_map_viewer.rs`；
- TaskSpace prompt、manifest 和 skill assets。

### 8.3 必须语义重放的宿主挂点

- `core/src/client.rs`；
- `core/src/session/turn.rs`；
- `core/src/tools/sequence*.rs`；
- tool registry/context；
- rollout reconstruction；
- protocol/app-server schema；
- state migration registry/runtime；
- TUI routing 与 app-server adapter；
- base instructions 和 compaction。

### 8.4 Multi-Agent 权威状态决策

TaskSpace 已经移除旧的 multi-agent lease/node 直绑链，当前通过通用 tool call identity 消费 `spawn_agent`、`wait` 等能力。因此可以继续吸收上游 MultiAgent V2，但必须保持：

- 稳定的 `call_id`、tool name 和 response call index；
- `AgentPath` 和 parent/child identity；
- spawn、wait、completion 事件；
- fork history 语义；
- tool output 成功、失败和 terminal carrier 扩展点。

建议的唯一权威关系：

```text
TaskSpace Event Store（任务状态唯一权威）
    -> projection / adapter
Upstream AgentGraphStore + WorldState + ThreadManager
```

不得让 TaskSpace Event Store 和 upstream AgentGraphStore 并列持久化两套“真实任务状态”。如果未来决定由 AgentGraphStore 取代部分 TaskSpace，必须先定义迁移、回放和回滚合同。

## 9. Create / Debug Primitive 状态

当前代码中未发现完整实现的：

- `PrimitiveModule` / `PrimitiveRegistry`；
- `ScaffoldArtifact` / `ScaffoldVerification`；
- `DebugCase` / `Hypothesis` / `EvidenceRecord`；
- `RootCauseDecision`；
- `EvidenceRace` / `PatchLeague`；
- Create/Debug phase machine。

这些能力目前主要存在于架构设计文档中，Rust 侧只有少量 compaction retention 文案。因此：

- 当前没有需要阻止上游同步的 Create/Debug 生产代码；
- 后续实现不应复制 TaskSpace 对 session、tool runtime 和 store 的纵向侵入；
- 在继续开发 Create/Debug 前，应先建立真正的 PrimitiveModule host seam。

## 10. 缓存回归门禁

当前缓存敏感面基线状态为 `live_regression_failed`，见[缓存敏感面合同](../../../benchmarks/cache-regression/cache-surface-contract.json)。

凡是触及以下任一范围，都不能以“编译通过”作为合并完成标准：

- base instructions/context construction；
- TaskSpace projection/control feedback；
- provider payload/protocol；
- tool declarations；
- MCP、Apps、Plugins、Skills；
- model/provider routing；
- compaction。

最低验证顺序：

1. 对应 crate 的 unit/integration tests；
2. free final-wire 和 cache payload contracts；
3. `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`；
4. 若门禁要求真实回归，按全局 Whale Agent run ledger 和专项预算流程申请授权；
5. 不得使用 `--no-verify` 绕过。

## 11. 推荐实施波次

### Wave 0：建立可追溯基线

- 修正 `UPSTREAM.md` 中失真的 local patch count；
- 生成机器可读 Whale overlay inventory；
- 按 brand/home、provider/model、wire/SSE、cache observability、TaskSpace domain、TaskSpace host hooks 分组；
- 记录 `rust-v0.146.0` 的 commit、release date 和 license。

### Wave 1：快速安全回移

- 每个 P0/P1 小主题独立提交；
- 运行对应 crate 测试、smoke 和平台专项验证；
- 不触及 DeepSeek/provider payload/TaskSpace/cache 敏感面。

### Wave 2：准备 0.146 substrate

- 建立干净的官方稳定版候选快照；
- 采用 upstream 的 message history、prompts、context fragments、HTTP transport、permission profiles 和 thread store；
- 先保证纯 upstream substrate 自身测试通过；
- 不在此阶段混入 Whale 业务语义。

### Wave 3：重放 Whale 基础 overlay

- Whale brand 和二进制命名；
- `WHALE_HOME`、secret/keyring 和安装隔离；
- DeepSeek Responses provider、Flash model catalog、reasoning 和 usage；
- provider wire trace、request budget 和 cache contracts；
- 逐组运行缓存门禁。

### Wave 4：TaskSpace 与 Multi-Agent 收敛

- 先形成 AgentGraphStore/WorldState/TaskSpace 权威状态 ADR；
- 保留 TaskSpace domain/runtime/store 独立模块；
- 将宿主侵入重写为 adapter、hook 或 extension point；
- 接入上游 MultiAgent V2 生命周期、分页 history、fork 和错误传播；
- 验证 TaskSpace replay、resume、fork、terminal 和 TUI viewer。

### Wave 5：生成物与发布闭环

- 统一生成 app-server JSON/TypeScript schema；
- 执行 workspace build、Rust 回归、CLI smoke、TaskSpace contracts、缓存 gate；
- 更新 `UPSTREAM.md` 和 codex-sync log；
- 明确 adopted、adapted、disabled 和 deferred 上游能力。

## 12. 暂缓或拒绝直接吸收的能力

以下内容即使文本上无冲突，也不属于当前优先追赶收益：

- OpenAI 专属 login、ChatGPT plan、rate limit 和 account UI；
- Codex desktop app 自更新和远程控制产品面；
- OpenAI hosted Apps、remote plugin service 的专属路由；
- audio/image/realtime 等 DeepSeek 当前未声明兼容的能力；
- Bedrock 专属模型和 marketplace 产品逻辑；
- 0.147 alpha 的未稳定接口。

处理原则是保留 upstream 内部接口兼容性，但通过 Whale feature/product policy 禁用或延后，不在同步时删除其底层通用能力。

## 13. 实施验收标准

完成一次稳定版上游追赶至少需要满足：

- vendor 来源、commit、时间和 license 可追溯；
- Whale overlay inventory 与实际 diff 一致；
- 不存在未经记录的 vendor 直改；
- DeepSeek Flash 是默认且可见模型，Pro 的可见性符合当时官方能力；
- `WHALE_HOME`、auth 和 keyring 不与 Codex 混用；
- Standard 与 TaskSpace 的 final wire 合同均通过；
- TaskSpace canonical map、CAS store、resume/fork/replay/terminal tests 通过；
- Multi-Agent 与 TaskSpace 只有一套任务状态权威；
- app-server schema 由生成流程刷新，不保留手工漂移；
- cache regression gate 通过，或按规则记录明确 blocker；
- 没有未提交改动，所有主题按最小提交原则推送。

## 14. 外部资料

1. [OpenAI Codex 官方仓库](https://github.com/openai/codex)
2. [Codex CLI 0.146.0 Release](https://github.com/openai/codex/releases/tag/rust-v0.146.0)
3. [Codex App Server v2 文档](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
4. [Codex 配置 schema](https://github.com/openai/codex/blob/main/codex-rs/core/config.schema.json)
5. [DeepSeek Codex 接入文档](https://api-docs.deepseek.com/quick_start/agent_integrations/codex/)
6. [DeepSeek Responses API 指南](https://api-docs.deepseek.com/guides/responses_api/)
7. [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache)

## 15. 本地证据索引

- [Codex vendor 固定基线](../../../third_party/codex-cli/UPSTREAM.md)
- [Codex upstream substrate ADR](../../adr/2026-04-27-codex-cli-upstream-substrate.md)
- [初始导入记录](../../migration/codex-sync/2026-04-27-initial-import.md)
- [Whale 品牌与 DeepSeek overlay](../../migration/codex-sync/2026-04-27-whale-brand-deepseek.md)
- [选择性上游回移记录](../../migration/codex-sync/2026-05-01-selective-upstream-backports.md)
- [DeepSeek Responses 迁移决策](../build-R8/cache-regression/11-deepseek-responses-migration.md)
- [缓存敏感面合同](../../../benchmarks/cache-regression/cache-surface-contract.json)
- [TaskSpace Map Store 运行手册](../../runbooks/r7-taskspace-map-store.md)

## 16. 已知未决策项

| 决策 | 当前建议 | 决策时点 |
| --- | --- | --- |
| 是否先落快速 backport | 是，先落 P0 和低风险 P1 | 开始实施前 |
| 是否以 0.146.0 替换 vendor | 是 | overlay inventory 完成后 |
| 是否启用 upstream network proxy | 先核实现有 runtime 使用情况 | network proxy backport 前 |
| AgentGraphStore 与 TaskSpace 的权威关系 | TaskSpace 为任务状态权威，上游 graph 作为 projection/adapter | Wave 4 ADR |
| 是否继续保留通用 Chat Completions provider | 保留，但不作为内置 DeepSeek 主路径 | DeepSeek overlay 重放时 |
| Pro 何时重新开放 | 以 DeepSeek 官方 Codex 支持声明和 provider probe 为准 | 每次模型目录更新时 |
