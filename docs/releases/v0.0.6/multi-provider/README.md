# v0.0.6 子主题：多 Provider 支持

- 状态：主要产品规则已确认，双凭据与同名模型路由待确认
- 产品权威：[多 Provider 切换 PRD](../../../../prd/2026-08-23-v0.0.6-multi-provider.md)
- 代码盘点：[当前实现与差距清单](current-state-inventory.md)

## 目标

在 Whale TUI 中提供 `/provider` 命令，让用户能够选择：

1. OpenAI 订阅；
2. OpenAI API；
3. DeepSeek。

OpenAI 订阅与 OpenAI API 复用 Codex 原生认证路径；选择 DeepSeek 时提供 API Key 录入流程。切换必须同时处理 Provider、认证、模型目录、系统提示词、工具集合、命令集合、上下文压缩和同一 session 中历史上下文的兼容迁移。

`/model` 是跨 Provider 的统一模型入口：展示所有 Provider 模型并允许直接选择，选择其他 Provider 模型时同步切换 Provider/访问方式，不要求用户先执行 `/provider`。

## 当前阶段结论

现有代码已具备同一 Provider 内按 turn 切换模型、重建工具集合和处理模型上下文差异的基础，但不支持活跃 session 原子切换 Provider。已确认 active turn 期间允许 UI 选择、下一 turn 生效，并确认凭据、模型 fallback、历史投影、命令禁用和 logout 规则。

OpenAI 官方与当前源码均确认 API key 可直接登录使用；但原生认证只保留一个当前认证记录，不会同时保存订阅 token 和 API key。实现前还需确认是否扩展为双凭据槽，以及 `/model` 中同名 OpenAI 模型的访问方式消歧规则。

本阶段只建立产品主题与代码事实基线，不授权进入实现。
