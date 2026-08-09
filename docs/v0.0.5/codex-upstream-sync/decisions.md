# Codex 主线融合产品决策基线

> PROTECTED USER-AUTHORITY ARTIFACT
> 本文件中的决策只能由用户针对具体决策显式批准后新增、修改、删除、重新解释或取代。Agent 推断、实现、测试、审查、既有文档或用户未反对，均不构成批准。

- Authority: User
- Write Gate: Explicit user approval required
- Agent Self-Approval: Forbidden
- Release Version: WhaleCode v0.0.5
- Topic: Codex CLI upstream sync
- Plan: [./plan.md](plan.md)

| ID | Confirmed Decision | Must Do | Must Not Do | Rationale | Violation Signal | Confirmation | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| D1 | `deepseek-v4-flash` 保持默认且可见；`deepseek-v4-pro` 在 DeepSeek 官方 Responses API 适配完成前继续隐藏，完成并验证后再恢复。 | 主线融合期间保持 Flash 默认和 Pro 隐藏门禁；恢复 Pro 前重新核验官方能力并完成 provider/TUI 回归。 | 不得因上游模型目录、测试快照或默认值变化提前展示 Pro；也不得把 Pro 永久删除。 | DeepSeek 官方 Responses API 仍在适配 Pro，当前产品行为已由用户直接确认。 | 默认模型不再是 Flash；Pro 在恢复条件未满足时可见；Pro 代码路径被不可逆删除。 | user-confirmed-direct: “接受flash 默认，pro继续隐藏……官方适配之后再恢复” | active |
| D2 | 本轮工作的唯一目标是当前 `whalecode-codex` 工作空间内的 Codex 主线融合，并保留现有 DeepSeek 适配和 TaskSpace 产品能力。 | 以上游 substrate 为基底，按独立闭环分别重放 DeepSeek 与 TaskSpace；所有操作限定在当前工作空间。 | 不得把其他分支或工作空间纳入范围；不得为了合入而顺手重构、替代或删除 DeepSeek/TaskSpace 业务语义。 | 用户明确收窄了工作空间与业务边界，要求只解决当前分支如何融合 Codex 主线。 | 访问或修改其他工作空间；上游同步单元同时重写 DeepSeek 与 TaskSpace；以“测试变绿”为由改变产品语义。 | user-confirmed-direct: “禁止你离开 whalecode-codex 工作空间做任何越界动作”“你就解决当前分支怎么融合 codex 主线变更” | active |
