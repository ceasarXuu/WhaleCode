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
| D1 | `deepseek-v4-flash` 保持默认且可见；`deepseek-v4-pro` 正式版发布并完成 Responses API 本地验证后恢复可见。 | 保持 Flash 默认；保留 Flash/Pro 原生 Responses、provider 与 TUI 回归合同。 | 不得因上游 GPT 模型目录改变默认值；不得重新隐藏或删除已验证的 Pro 路径。 | 用户先要求适配完成前隐藏 Pro，随后明确补充 Pro 正式版已发布且支持 Responses API，并批准在本地验证后恢复；U5–U10/U6 已完成该验证。 | 默认模型不再是 Flash；公共目录重新暴露 GPT；Pro 被无依据隐藏或删除。 | user-confirmed-direct: “接受flash 默认，pro继续隐藏……官方适配之后再恢复”；后续“deepseek-v4-pro 正式版已经发布，且支持 response API”及“批准” | active |
| D2 | 本轮工作的唯一目标是当前 `whalecode-codex` 工作空间内的 Codex 主线融合，并保留现有 DeepSeek 适配和 TaskSpace 产品能力。 | 以上游 substrate 为基底，按独立闭环分别重放 DeepSeek 与 TaskSpace；所有操作限定在当前工作空间。 | 不得把其他分支或工作空间纳入范围；不得为了合入而顺手重构、替代或删除 DeepSeek/TaskSpace 业务语义。 | 用户明确收窄了工作空间与业务边界，要求只解决当前分支如何融合 Codex 主线。 | 访问或修改其他工作空间；上游同步单元同时重写 DeepSeek 与 TaskSpace；以“测试变绿”为由改变产品语义。 | user-confirmed-direct: “禁止你离开 whalecode-codex 工作空间做任何越界动作”“你就解决当前分支怎么融合 codex 主线变更” | active |
