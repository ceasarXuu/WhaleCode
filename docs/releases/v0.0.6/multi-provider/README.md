# v0.0.6 子主题：多 Provider 支持

- 状态：需求盘点完成，产品规则待确认
- 产品权威：[多 Provider 切换 PRD](../../../../prd/2026-08-23-v0.0.6-multi-provider.md)
- 代码盘点：[当前实现与差距清单](current-state-inventory.md)

## 目标

在 Whale TUI 中提供 `/provider` 命令，让用户能够选择：

1. OpenAI 订阅；
2. OpenAI API；
3. DeepSeek。

OpenAI 订阅与 OpenAI API 复用 Codex 原生认证路径；选择 DeepSeek 时提供 API Key 录入流程。切换必须同时处理 Provider、认证、模型目录、系统提示词、工具集合、命令集合、上下文压缩和同一 session 中历史上下文的兼容迁移。

## 当前阶段结论

现有代码已具备同一 Provider 内按 turn 切换模型、重建工具集合和处理模型上下文差异的基础，但不支持活跃 session 原子切换 Provider。实现前必须先确认 PRD 中的切换边界、凭据生命周期、持久化默认值和跨 Provider 历史处理规则。

本阶段只建立产品主题与代码事实基线，不授权进入实现。
