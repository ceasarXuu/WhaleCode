# WhaleCode 最初设计理念概览

## 范围与结论

本文以 2026-04-24 至 2026-04-27 的首批架构文档、ADR 和 Git 历史为主要证据，回答 WhaleCode 的“最初设计理念”。这里的“最初”不是仅指初始提交（该提交只有一个标题），而是指首轮成体系的产品与架构设计。

WhaleCode 最初不是被定义为聊天机器人，而是被定义为一个以 DeepSeek V4 为中心、直接在真实代码仓库中工作的终端 coding agent。它试图把多 Agent 协作、Create/Debug 工作流、安全执行、证据、门禁和回放都变成运行时能力，而不是依赖提示词自觉。

## 九项核心理念

1. **DeepSeek-first，而非通用模型壳。** 围绕 Flash/Pro 分工、长上下文、长输出、thinking、并行工具调用和缓存前缀做原生优化。当前实现仍默认选择 DeepSeek provider 与 `deepseek-v4-flash`（`S01`），并为 DeepSeek 保留独立压缩策略（`S02`）。
2. **Multi-Agent First。** 单 Agent 只是群体大小为 1 的特例；通信、调度、隔离和失败恢复从一开始就是核心问题，而不是后加功能。
3. **Coding-Native。** 产品服务于真实仓库中的读取、搜索、命令、补丁、测试和验证，而不是泛化聊天。
4. **Create 与 Debug 是两种架构原语。** Create 是从需求向多个实现单元发散；Debug 是从多个症状和假设向单一根因收敛。二者应拥有不同的 DAG、权限、上下文和成功标准。
5. **运行时约束强于提示词。** 关键行为应由 phase state machine、tool permissions、DAG validation、artifact contracts、context allocation 和 deterministic gates 保证。当前 TaskSpace 的 canonical map schema（`S03`）和图不变量验证（`S04`）延续了这一思想。
6. **Create 必须参考驱动、脚手架先行。** 设计前先研究外部最佳实践和失败案例；功能代码前先建立 logging、testing、constraints，并通过依赖图强制顺序。
7. **Debug 必须证据驱动。** 用 Goal → Hypotheses → Evidence 结构先复现、生成可证伪假设、收集证据，再修复；诊断阶段原则上只读，避免“先猜一个补丁再试”。
8. **独立对抗视角与全链路可观测。** Viewer 应独立于执行者和阶段末 Reviewer，读取结构化 artifact、gate result 和 session event；关键动作需要可追踪、可回放。当前 TaskSpace 的 action producer 关闭与排空协议（`S05`）体现了对持久状态完整性的重视。
9. **稳定底座与差异化原语分离。** 差异化能力必须可插拔、可评测、可关闭、可替换。早期先从 TypeScript 转向 Rust-first，随后又迅速改为复用 Codex CLI 整仓 substrate；变的是实现路线，不变的是 DeepSeek-first、Multi-Agent-first、Primitive-driven 的产品身份。

## 演进中的重要修正

- **已替换：从零自研 runtime。** 2026-04-25 的 Rust-first 自研方案在 2026-04-27 被 Codex whole-repo upstream substrate 方案取代。
- **已收敛：角色与竞争机制不是核心对象。** 后续 Multi-Agent 设计采用 Occam-first，核心从角色清单、投票和竞赛概念收敛为 map/node/action/result/gate 等可验证对象。
- **已演进：Action Map 到 TaskSpace。** 当前代码采用 TaskSpace canonical map、严格图不变量和持久状态；这不是放弃 Multi-Agent First，而是把它改造成更小、更可测的运行时控制面。
- **仍属规划或不完整实现：** 完整 Create/Debug PrimitiveModule、独立 Viewer、技能自进化和完整 Web Viewer 不应仅凭设计文档认定为已完成。

## 当前成熟度

仓库处于活跃开发和实验验证阶段。Codex-derived Rust workspace、DeepSeek provider、TaskSpace 协议/状态/执行路径已有代码；不少最初差异化原语仍处于设计、分阶段实现或实验验证状态。设计文档中的“必须”表示产品意图，不等同于当前完成度。

## 阅读顺序

1. `docs/plans/2026-04-24-system-architecture.md`
2. `docs/plans/2026-04-25-differentiated-primitives-architecture.md`
3. `docs/adr/2026-04-25-rust-first-core-runtime.md`
4. `docs/adr/2026-04-27-codex-cli-upstream-substrate.md`
5. `docs/plans/2026-04-25-multi-agent-collaboration-architecture.md`（历史）
6. `docs/plans/2026-05-22-taskspace-runtime-design.md`（后续收敛）

