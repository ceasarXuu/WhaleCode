# v0.0.6 子主题：多 Provider 支持

- 状态：Phase 0 已验证；Phase 1 实施中
- 产品权威：[多 Provider 切换 PRD](../../../../prd/2026-08-23-v0.0.6-multi-provider.md)
- 代码盘点：[当前实现与差距清单](current-state-inventory.md)
- 工程计划：[多 Provider 工程实施计划](plan.md)

## 目标

在 Whale TUI 中提供 `/provider` 命令，让用户能够选择：

1. OpenAI 订阅；
2. OpenAI API；
3. DeepSeek。

OpenAI 订阅与 OpenAI API 复用 Codex 原生认证路径；选择 DeepSeek 时提供 API Key 录入流程。切换必须同时处理 Provider、认证、模型目录、系统提示词、工具集合、命令集合、上下文压缩和同一 session 中历史上下文的兼容迁移。

`/model` 是跨 Provider 的统一模型入口：按 `OpenAI 订阅`、`OpenAI API`、`DeepSeek` 分组展示所有模型并允许直接选择，选择其他分组的模型时同步切换 Provider/访问方式，不要求用户先执行 `/provider`。同名 OpenAI 模型分别归入两个 OpenAI 组，分组明确决定认证与计费路径。

## 当前阶段结论

现有代码已具备同一 Provider 内按 turn 切换模型、重建工具集合和处理模型上下文差异的基础，但不支持活跃 session 原子切换 Provider。已确认 active turn 期间允许 UI 选择、下一 turn 生效，并确认凭据、模型 fallback、历史投影、命令禁用和 logout 规则。

OpenAI 官方与当前源码均确认 API key 可直接登录使用。产品规则已确认扩展为订阅 token 与 API key 双凭据安全共存，同时保持 Codex 原生登录流程；这是对当前单激活认证存储的明确扩展需求。

本主题已建立产品权威、代码事实基线和分阶段工程计划，并已完成 Phase 0 最小协议实现与离线验证。

## 实施进度

Phase 0 已完成非敏感 route identity、OpenAI 双凭据存储/刷新可行性、active-turn next-turn 边界和 wire-copy 历史投影方向验证。证据表明现有 `AuthDotJson` 已可承载 OpenAI 双槽。D1 已获用户批准：保留现有 flat 字段，仅新增 DeepSeek 独立槽并把登录/登出改为字段级操作；Phase 1 已进入实施。

Phase 1 已完成认证基础与 route-scoped 模型缓存：三槽凭据可在 file/keyring 中共存，`AuthManager` 可按 route 分别解析 OpenAI 订阅、OpenAI API 和 DeepSeek API，DeepSeek 提供独立登录入口，route logout 只清除目标槽；不同 route 的模型目录缓存不会互相命中。三组 catalog 与 account/TUI/runtime 接线仍在后续批次。
