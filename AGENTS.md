# 项目目标

构建以 DeepSeek V4 为核心的终端 AI coding agent，对标 Claude Code / OpenCode / Codex CLI / Pi。

- **开源**
- **技术栈**: Rust-first core + TypeScript Web Viewer
- **模型**: `deepseek-v4-flash` + `deepseek-v4-pro`
- **核心定位**: Multi-Agent First + Coding-Native，极致适配 DeepSeek 模型
- **原生任务**: Create（构建发散）和 Debug（诊断收敛）作为架构原语

## 核心能力

| 能力 | 说明 |
|------|------|
| **Multi-Agent 网格** | 7 角色（Architect/Debugger/Implementer/Searcher/Reviewer/Viewer/Supervisor），角色网格协作，Agent 从不独立存在 |
| **Agent Message Bus** | 统一消息总线（unicast/broadcast/p2p/request-reply），traceId 追踪全链路 |
| **证据链 Debug** | Goal → Hypotheses → Evidence 链式推理，假设证伪收敛到根因，HYPOTHESIZE 阶段全员只读 |
| **脚手架先行 Create** | Logging/Testing/Constraints 三基建必须先于功能代码，DAG 验证强制执行 |
| **参考驱动设计** | 任何设计前必须搜索社区最佳实践/失败案例，设计文档必须引用 ≥3 外部来源 |
| **6 层架构约束** | Phase Machine → Tool Permissions → DAG Validation → Artifact Contracts → Context Allocation → System Prompt |
| **独立 Viewer** | 常驻对抗性批判角色（V4-Pro, 只读），全流程渗透每个步骤 |
| **Skills / Tools / MCP** | 业界通用能力层，可组合 Skills、原子 Tools、MCP 协议接入外部生态 |
| **技能自进化** | Skill 从创建起持续监控使用数据，Evolution Agent 自动分析短板并迭代版本 |
| **上下文管理** | 独立轮次压缩 + 历史替换（参考 Codex CLI），片段注入，适配 1M 窗口 |
| **实时可视化** | Web 端 Agent 网络图动画、DAG 进度、统计面板（token/工具调用/缓存命中率） |
| **模型分层** | 复杂推理用 V4-Pro，常规执行用 V4-Flash，按角色自动选择 |

## DeepSeek V4 极致适配

| 特性 | 适配策略 |
|------|---------|
| 1M 上下文 | 分级压缩管线，阈值提升到 ~755K，短作业零压缩 |
| 思考链 (Thinking) | 实时 streaming 展示 reasoning_content |
| 超长输出 384K | 保留 >50K 输出头寸，分块流式写入 |
| 5x 缓存定价 | 共享 System Prompt 前缀跨 Agent 命中缓存 |
| 平行工具调用 | 工具系统原生支持并行执行 |
| V4-Flash ¥2/M | 低成本执行 Implementer/Searcher 任务 |
| V4-Pro ¥24/M | 关键路径使用（设计、诊断、审查、批判、上下文压缩）|

# 工程约束
- 本项目是开源项目，注意管理好隐私数据禁止泄露，经常更新和优化 gitignore
- 禁止未经允许新开分支，如有必要向用户申请确认
- 最小化提交原则：每次有小主题改动就积极 commit 并 push到远端，增强安全性，无需用户确认
- repo中所有改动都要提交，不要有未提交的改动，所有代码都是你改的，不要甩锅给用户
