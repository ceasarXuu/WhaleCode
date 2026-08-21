# 核心理念到实现的逻辑映射

## DeepSeek-first 模型路径

- 文件：`third_party/codex-cli/codex-rs/core/src/config/mod.rs`、`third_party/codex-cli/codex-rs/core/src/compact.rs`
- 角色：选择默认 provider/model，并为 DeepSeek 分派专有上下文压缩行为。
- 输入：用户配置、profile、provider 能力、当前模型。
- 输出：有效 provider/model 与压缩策略。
- 不变量：模型层优化不能成为正确性前提；provider 能力和配置仍需显式解析。
- 证据：`S01`、`S02`
- 意义：证明 DeepSeek-first 已从营销定位进入实际运行路径，但最初文档中的价格、窗口和能力数值仍需独立 probe，不能视为永久事实。

## TaskSpace 结构化工作状态

- 文件：`third_party/codex-cli/codex-rs/protocol/src/taskspace.rs`
- 角色：定义可序列化、带版本的 canonical map，明确 root、work nodes、finish、revision 与 action outcome。
- 输入：用户目标被分解后的工作图和工具行动结果。
- 输出：可持久化、可投影、可验证的任务状态。
- 不变量：未知字段被拒绝；canonical schema 不应混入派生 children/edges 或历史遗留字段。
- 证据：`S03`
- 意义：它是“工作流不能只存在于提示词”这一原始理念的当前主要实现形态。

## 确定性 DAG 门禁

- 文件：`third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/invariants.rs`
- 角色：检查 schema、身份、父子关系、环、可达性、状态、action 和 revision 等机械不变量。
- 输入：TaskSpace map。
- 输出：稳定排序的 violation 列表。
- 不变量：root 必须是唯一源，finish 必须是唯一汇；每个节点必须从 root 可达并能到达 finish。
- 证据：`S04`
- 意义：LLM 负责语义分解，runtime 只强制机械正确性，体现“提示词负责引导，代码负责约束”。

## Session 收口与可观测性

- 文件：`third_party/codex-cli/codex-rs/core/src/session/taskspace_store/producer.rs`
- 角色：追踪异步 action producer，在 session 收口时停止接纳新 producer，并等待已有 producer 排空。
- 输入：异步 TaskSpace action settlement 任务。
- 输出：完成或拒绝的 producer 生命周期，以及结构化 tracing event。
- 不变量：关闭 admission 后不能继续产生新持久化动作；退出前必须排空已接纳任务。
- 证据：`S05`
- 意义：避免“最终回答已经结束但状态仍在后台漂移”，延续最初的 session event、replay 和可观测性诉求。

## 风险与未完成边界

- 最初文档描绘的完整 Create/Debug 状态机，不应与当前 TaskSpace 通用控制面混为一谈。
- PrimitiveModule contract 在设计文档中很完整，但是否已形成完整 registry、kill switch、replay reducer 和 eval gate，需要按具体模块逐项审计。
- Viewer、技能自进化和 Web Viewer 在产品目标中重要，但当前成熟度不能从目录名或设计稿推断。
- `third_party/codex-cli/` 既是 upstream substrate，也含 Whale 修改；维护者需要结合 patch、migration 和 git history 判断所有权边界。

