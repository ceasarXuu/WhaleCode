# 项目目标

构建以 DeepSeek V4 为核心的终端 AI coding agent，对标 Claude Code / OpenCode / Codex CLI / Pi。

- **开源**
- **技术栈**: Codex-derived Rust core + TypeScript Web Viewer
- **模型**: `deepseek-v4-flash` + `deepseek-v4-pro`
- **核心定位**: Multi-Agent First + Coding-Native，极致适配 DeepSeek 模型
- **原生任务**: Create（构建发散）和 Debug（诊断收敛）作为架构原语
- **V1 目标**: 先交付主流竞品级通用 coding agent CLI 底座；差异化能力通过 PrimitiveModule 插件化增强

## 核心能力

| 能力 | 说明 |
|------|------|
| **Multi-Agent 群体协同** | 7 角色 + Scout/Analyst/Reviewer/Judge/Verifier cohorts，通过 DiversityPolicy / Tournament / Evidence Race / Patch League / EvidenceWeightedConsensus 用数量换非冗余质量 |
| **Agent Message Bus** | 统一消息总线（unicast/broadcast/p2p/request-reply），traceId 追踪全链路 |
| **证据链 Debug** | Goal → Hypotheses → Evidence 链式推理，假设证伪收敛到根因，HYPOTHESIZE 阶段全员只读 |
| **脚手架先行 Create** | Logging/Testing/Constraints 三基建必须先于功能代码，DAG 验证强制执行 |
| **参考驱动设计** | 任何设计前必须搜索社区最佳实践/失败案例，设计文档必须引用 ≥3 外部来源 |
| **Codex upstream substrate** | 权限、沙箱、工具执行、补丁、会话、上下文、MCP/Skills、日志等成熟基础设施以 Codex CLI 整仓上游底座为主；不足处再参考 Claude Code、OpenCode、Pi |
| **6 层架构约束** | Phase Machine → Tool Permissions → DAG Validation → Artifact Contracts → Context Allocation → System Prompt |
| **独立 Viewer** | 常驻对抗性批判角色（V4-Pro, 只读），全流程渗透每个步骤 |
| **Skills / Tools / MCP** | 业界通用能力层，可组合 Skills、原子 Tools、MCP 协议接入外部生态 |
| **技能自进化** | Skill 从创建起持续监控使用数据，Evolution Agent 自动分析短板并迭代版本 |
| **上下文管理** | 独立轮次压缩 + 历史替换（参考 Codex CLI），片段注入，适配 1M 窗口 |
| **实时可视化** | Web 端 Agent 网络图动画、DAG 进度、统计面板（token/工具调用/缓存命中率） |
| **模型分层** | 复杂推理用 V4-Pro，常规执行用 V4-Flash，按角色自动选择 |

差异化原语（证据链、脚手架先行、参考驱动、独立 Viewer、技能自进化）必须实现为 artifact schema、phase gate、session event 和 replayable state，不能只停留在提示词或愿景描述；同时必须通过 PrimitiveModule contract 可插拔接入，方便验证效用、模块化组装升级或淘汰特化能力。

## DeepSeek V4 极致适配

| 特性 | 适配策略 |
|------|---------|
| 1M 上下文 | 分级压缩管线，阈值提升到 ~755K，短作业零压缩 |
| 思考链 (Thinking) | 实时 streaming 展示 reasoning_content |
| 超长输出 384K | 保留 >50K 输出头寸，分块流式写入 |
| 5x 缓存定价 | 共享 System Prompt 前缀跨 Agent 命中缓存 |
| 平行工具调用 | 工具系统原生支持并行执行 |
| V4-Flash 低成本 | 大量并行 Scout/Analyst/Implementer 候选，具体价格以 provider probe 为准 |
| V4-Pro 高质量 | 关键路径使用（设计、诊断、审查、批判、上下文压缩），具体价格以 provider probe 为准 |

# 工程约束
- 本项目是开源项目，注意管理好隐私数据禁止泄露，经常更新和优化 gitignore
- 禁止未经允许新开分支，如有必要向用户申请确认
- 最小化提交原则：每次有小主题改动就积极 commit 并 push到远端，增强安全性，无需用户确认
- repo中所有改动都要提交，不要有未提交的改动，所有代码都是你改的，不要甩锅给用户
- GitHub Actions 默认不得由 push、pull request、schedule 等事件自动触发；根 workflow 只保留 `workflow_dispatch`，日常验证默认在当前隔离 worktree 本地执行。确需远端验证时按需手动触发；新增自动触发器必须先取得用户明确批准。
- 真实 Whale Agent 运行实行成本授权门禁：未经用户明确允许，禁止启动单次计划执行总数超过 3 个 sample 的真实 Whale Agent run。单次命令、脚本或矩阵中的 `sample × arm × repeat` 均累计计数，串行、并行、重试和包装脚本不得拆分或换名绕过；API Key 可用、阶段执行授权、测试或审查要求均不等同于付费运行授权。判断确有必要执行超过 3 个 sample 的大规模运行时，必须在启动前主动向用户申请专项预算，不得先运行后补报；预算申请至少说明模型、sample/arm/repeat 数量、预计 API 请求数、input/output token 与费用上限、最长耗时、停止条件及允许的重试范围，获得明确批准后方可执行。
- 每次真实 Whale Agent run 都必须登记到全局账本 `benchmarks/whale-agent-run-ledger.json`：启动前先创建 `planned` 记录并写明时间、理由、模型、规模和预算，结束、失败或取消后立即结算实际请求数、input/cached/uncached/output token、费用、耗时、状态及证据路径。超过 3 个 sample 的记录必须关联用户预算批准；重试必须新建记录，不得覆盖或删除历史运行。
- 缓存敏感面变更必须通过 `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`。门禁阻断时，先向用户说明具体变更路径、可能破坏的 provider 前缀结构和验证理由，再申请专用真实回归预算；不得使用 `--no-verify` 绕过。获批后只允许使用专用 runner，真实结果通过且与当前敏感面指纹一致时方可晋升基线。
- `third_party/codex-cli/` 未来作为 Codex upstream vendor 快照时，应尽量保持上游原样；上游文件可保留原始长度和结构，不受本项目普通单文件 500 行限制。Whale 自有代码仍遵守 500 行原则。
- 严禁为自然语言用户输入设置本地固定答复、寒暄模板、关键词答复或绕过模型的“智能回复”；所有自然语言输入必须进入 Agent/Model 路径，由 Agent 生成回答。CLI/slash 命令只能输出明确的机械状态、错误、路径或配置结果，不能伪装成 Agent 回答。

# 工作空间开工门禁

- 新clone、worktree或切换branch后，在执行安装、Whale运行、cache regression、TaskSpace benchmark等workspace敏感命令前，必须先运行`python3 scripts/workspace-safety/workspace_context.py bootstrap plan --json`。
- `bootstrap plan`只读；检查输出后，只能把该次计划的精确`fingerprint`传给`bootstrap apply --expect <fingerprint>`。不得跳过确认、复用过期fingerprint或手工创建marker。
- 初次bootstrap按`plan → apply → bash scripts/install-whale-local.sh --scope workspace → doctor --require-binary`执行。安装后人工开发统一从目标worktree内使用全局`whale-dev`，由dispatcher选择当前worktree的隔离binary与runtime home；全局`whale`只用于release。日常开工至少运行`require-ready`；自动化需要隔离运行时环境时可通过`workspace_context.py exec -- <command>`启动。
- 门禁失败必须先按稳定诊断码恢复；禁止fallback到PATH上的全局`whale`，禁止复制或迁移legacy `~/.whale`、凭据、history、sessions、plugins或skills。
- 真实模型运行的账本与预算批准仍是开发流程约束，不得把它实现成Whale产品逻辑或自然语言运行时授权协议。
- 在本机对当前 Codex vendor 执行完整 crate 回归时，使用 `python3 scripts/codex-upstream/run_isolated_tests.py <nextest 参数>`；不得用宿主代理或共享临时目录产生的失败判断产品回归。定向测试仍可按上游 `just test` 运行。
- VS Code用户使用`Workspace: Bootstrap Plan`、`Workspace: Bootstrap Apply`、`Workspace: Doctor`任务；权威合同仍是workspace-safety CLI。完整流程见`runbooks/local-workspace-safety.md`。
