# U17：Codex 0.147 主线融合发布收口

- 日期：2026-08-14
- 状态：`verified-with-explicit-deferrals`
- 上游 substrate：`rust-v0.147.0` / `be6e8eac029b183056b7e4402879f15d2c85f61b`
- 工作空间：仅 `/home/zhangxu/whalecode-codex`
- 真实模型请求：0
- live cache baseline：未晋升，保留最近一次失败状态

## 1. 收口结论

0.147 substrate、Whale identity/home/auth、DeepSeek 原生 Responses、缓存合同和 TaskSpace extension 主链已经形成可构建、可追溯的单一 vendor tree。U17 没有新增生产架构或发布框架，只修正上游测试夹具中仍假设 `codex` 二进制、`CODEX_HOME`、OpenAI 默认 provider 和 `0.0.0` 服务版本的部分，并完成机械工件、Linux 构建、离线 smoke 与分层回归。

2026-08-15 后续收尾修复了 16 项仍硬编码 GPT 模型/effort/tier 的子 Agent 测试夹具，并为当前 vendor 增加宿主隔离回归入口。隔离入口只清理 proxy、ambient sandbox 和共享临时根污染，不修改产品逻辑；原环境噪声定向集合 6/6 通过。最新证据见[子 Agent DeepSeek 夹具修复](../../v0.0.5/codex-upstream-sync/evidence/u17-closure-4f4f5d4c5/subagent-deepseek-fixture-fix.md)与[宿主隔离回归入口](../../v0.0.5/codex-upstream-sync/evidence/u17-closure-4f4f5d4c5/host-isolated-core-tests.md)。剩余 21 个 core lib 与 23 个 core integration 失败保持既有产品延期分类。

当前 overlay inventory 相对固定上游树共 185 条路径：48 added、133 modified、4 deleted。replay ledger 同为 185 条路径，其中 155 `adapt-semantically`、6 `reapply-exact`、20 `regenerate`、4 `defer`。这些数字包含测试和生成物，不代表 185 个手写产品补丁。

全 workspace 测试没有被表述为全绿。剩余失败均保留真实签名，并按已批准产品差异、平台/制品限制、宿主资源限制或上游测试波动登记；U17 没有为了绿色数字启用 OpenAI hosted、Bedrock、remote plugin/sharing 等 Whale 明确关闭或延期的能力。

## 2. 本阶段变更

### 2.1 测试夹具适配

- vendor 测试子进程统一使用 `WHALE_HOME`，避免读取真实 `~/.whale`；
- 测试查找 Whale CLI 时使用 `whale` 二进制，并同步 CLI help/version 断言；
- 以本地 mock OpenAI Responses server 为目标的 core/exec 夹具显式指定 `model_provider="openai"`，不改变 Whale 的 DeepSeek 默认值；
- MCP initialize 断言读取当前 Cargo 构建版本，不再硬编码开发版本 `0.0.0`；
- app-server 少量旧夹具显式声明其 OpenAI auth 或 DeepSeek 模型前提。

对应实现提交：`a3da33068`、`6820bf661`、`394aac900`、`a02c6eb71`。全部是测试/证据适配，没有新增生产业务逻辑。

### 2.2 来源与机械工件

- `third_party/codex-cli/UPSTREAM.md` 已从 U4 时点更新到当前完整 Whale overlay；
- overlay inventory 与 replay ledger 已刷新到 185 条路径；
- stable/experimental app-server schema 通过 Python wrapper 重生成，工作树无 schema 差异；
- Cargo 执行产生的 workspace package `0.147.0` lock 噪声已恢复为仓库约定的 `0.0.0`，lockfile 无差异；
- `validate_sync_metadata.py` 通过。

## 3. 通过矩阵

| 验证 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过；仅 stable rustfmt 对 nightly-only 配置的已知警告 |
| `cargo check --workspace --all-targets` | 通过；使用已校验的官方 sandbox V8 archive/binding |
| stable schema 重生成 | 通过，生成物 clean |
| experimental schema 重生成 | 通过，生成物 clean |
| `cargo build -p codex-cli --bin whale` | 通过 |
| `whale --version` | `whale 0.147.0` |
| `whale --help`、`whale features list` | 离线 smoke 通过 |
| `codex-exec --test all` | 73 passed |
| `codex-mcp-server --test all` | 4 passed |
| `codex-skills-extension --lib` | 隔离 HOME 后 87 passed |
| `codex-secrets --lib` | 清除测试产生的空 `/tmp/.git`、`/tmp/.codex` 后 9 passed |
| core CLI stream | 9 passed |
| 完整免费 cache contract | 7/7 passed |
| cache contract Python tests | 已由 U16 固定为 20 passed |
| sync metadata validator | 通过 |
| 真实网络/API 请求 | 0 |

免费缓存 7 项为 `build_mcp_cache_helper`、`prompt_caching`、`prompt_cache_key`、`mcp_tool_cache`、`responses_request_contract`、`deepseek_standard_final_wire`、`deepseek_taskspace_final_wire`。

## 4. 非全绿项与归因

### 4.1 已批准的产品差异

对抗性修复提交 `4f4f5d4c55bb527fb842fa4076117ae79badf79d` 上，app-server 为 1122 run、1089 passed（1 flaky）、33 failed、1 skipped。33 条失败中：

- Bedrock/default account、Bedrock static catalog：Whale 当前不交付 Bedrock 模型市场；
- OpenAI model list/remote catalog：Whale 公共模型列表按产品合同保持 DeepSeek-only；
- remote plugin、plugin share、recommended plugins：这些 OpenAI hosted 能力默认关闭；
- 本轮没有 watcher 失败；三条约 30 秒的失败均在等待被关闭的 remote plugin refresh。

core lib 为 2178 run、2154 passed、24 failed；core integration 为 1123 run、1086 passed（1 flaky）、37 failed、8 skipped。94 个失败名、命令、环境、脱敏原始输出和逐项映射见[收口失败清单](../../v0.0.5/codex-upstream-sync/evidence/u17-closure-4f4f5d4c5/failure-manifest.md)。93 项属于明确不在本轮发布合同的 Guardian、OpenAI remote model、Multi-Agent GPT 型号/effort、remote plugin、独立 image generation 或 Bedrock 前提；唯一宿主代理环境项在清除代理变量后精确复跑 1/1 通过。没有 TaskSpace 失败或未分类失败。

对抗性审查发现并已修复两个 TaskSpace 边界：旧版合法 `canonical_json="null"` 记录现在可按 inactive 语义读取并在首次 CAS 时原地激活；普通 app-server `thread/fork` 现在通过可信 lifecycle lineage 继承父 map 并持久化 `Fork` relation。真实 SQLite migration、extension relation 以及 process-level app-server 回归均已通过；后者在同一用例中依次验证 Standard request、typed mode/read RPC、普通 fork、shutdown/restart/resume，以及带 `taskspace_control` 和 canonical map projection 的 Responses final-wire。

CLI help 中仍有部分上游 `Codex` / `~/.codex` 说明文字。U17 只验证二进制身份和命令可用性；剩余品牌清理继续作为独立产品文案单元，避免在发布核验阶段扩大修改面。

### 4.2 环境或制品限制

| 项目 | 结果 | 归因 |
| --- | --- | --- |
| `codex-linux-sandbox --test all` | 27 passed、1 failed | `wget` 在本机受限网络中 10 秒超时，而测试只接受立即 denied；其他 curl/nc/ssh/ping/写隔离均通过 |
| `codex-v8-poc --lib` | 5 passed、1 failed | 本机只缓存官方 `ptrcomp_sandbox` V8，crate 当前期望 non-sandbox feature；是验证制品变体不匹配 |
| `codex-model-provider --lib` | 58 passed、1 failed | 唯一失败要求 Bedrock GPT static catalog，与 DeepSeek-only 产品合同冲突 |
| `codex-exec-server --lib` | 215 passed、1 failed | trace-context 用例在全集受全局 tracing 状态影响；单独 exact 重跑通过 |
| Skills/Secrets 在 workspace 并发矩阵 | target failed | 测试在 `/tmp` 根生成空 `.git/.codex` 并读取宿主目录；清理后隔离全量分别通过 |

测试创建的空 `/tmp/.git` 和 `/tmp/.codex` 已确认无内容并删除，未触碰任何用户数据。

### 4.3 明确延期

- TaskSpace TUI 已登记夹具问题；完整 workspace 测试进入 TUI 后长期无进展，因此安全中止，未声明 TUI 全绿；
- Windows runner 与 Windows 实机终端 smoke 继续延期；
- 缺少匹配 non-sandbox feature 的 V8 官方制品时，不声明 `v8-poc` 全绿；
- OpenAI/ChatGPT 登录产品 UI、OpenAI hosted remote plugins/sharing/recommendations、Bedrock catalog 不属于当前 Whale 0.147 发布合同；
- accepted live cache baseline 不晋升；若未来确需真实回归，必须另行申请预算并登记全局账本。

## 5. 发布判断

U17 的结论是“0.147 主线融合工程计划已收口”，不是“所有上游产品和平台测试全部通过”。Linux 上的 Whale CLI、DeepSeek Responses、缓存合同、TaskSpace state/extension/RPC/final-wire 与同步元数据已经有免费、可重复的通过证据；已知延期项没有被隐藏或误报。

后续工作应按各自产品或平台单元处理，不再回到本计划追加重叠 Phase：

1. TaskSpace TUI fixture；
2. Windows runner/terminal；
3. 需要时的品牌文案清理；
4. 未来若启用 OpenAI hosted/Bedrock 能力，再恢复对应上游测试；
5. 获得预算后才可进行 live cache baseline 回归与晋升。
