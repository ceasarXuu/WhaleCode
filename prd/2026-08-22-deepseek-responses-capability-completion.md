# PRD: DeepSeek Responses 能力完整接入

- Status: Ready for implementation
- Created: 2026-08-22
- Updated: 2026-08-22
- Owner / requester: repository owner
- Source request: 最大限度完整补齐当前 DeepSeek 三模型能力，并在 1 元人民币总测试预算内完成真实验证。
- Product Authority: Confirmed Product Decisions section

## Requester Review Summary

- Key decisions: 当前官方三模型均进入正式产品目录；Responses、thinking、工具、流式事件和视觉输入按官方契约接入；真实验证总费用不得超过 1 CNY。
- Important exceptions: DeepSeek 官方明确不支持或静默忽略的 Responses 字段不得在 Whale 中宣称为已生效能力。
- Must-confirm before implementation: 无。
- Status reason: 模型范围、目标结果和真实测试预算均已由用户直接确认。

## 1. Background And Product Intent

DeepSeek 当前提供 `deepseek-v4-flash`、`deepseek-v4-pro` 和 `deepseek-v4-flash-vision-exp`。Whale 已具备 Responses 基础主链，但视觉模型尚未进入目录，推理档位和若干流式/请求能力与最新官方契约不完整一致。

## 2. Goals And Success Criteria

- 三个模型在 Whale 中具有准确、可选择的静态能力描述。
- Flash/Pro 保持文本 Coding Agent 主链稳定，Vision 能接收用户和工具返回的图片。
- Responses 请求、SSE、thinking、function/custom tool、usage 和错误终态与官方兼容表一致。
- 用户不会看到 DeepSeek 不支持字段被错误描述为有效。
- 离线契约测试通过，并在 1 CNY 总费用硬上限内完成三模型最小真实验证。

## 3. Users And Usage Context

面向通过 Whale CLI/TUI 使用 DeepSeek 进行代码创建、调试、审查、工具调用和视觉代码理解的开发者。

## 4. Scope

### In Scope

- 当前三个官方模型的目录、可见性、上下文、推理档位和输入模态。
- 原生 `POST /responses` 请求与语义 SSE。
- reasoning text、output text、function/custom tool、web search 状态、usage 与终态。
- URL/Base64 图片输入及工具输出图片；Files API `file_id` 在现有协议类型可安全承载时接入。
- 官方支持参数的现有产品入口与请求序列化；不支持字段的准确降级。
- 离线 fixture、缓存门禁和受预算约束的真实 API smoke。

### Out Of Scope

- DeepSeek 官方未支持的 server-side conversation、remote compact、WebSocket、MCP、computer use 和 code interpreter。
- 为本次接入新建通用 Files 管理 UI。
- 超出 1 CNY 总预算的性能或质量 benchmark。

## 5. Core User Journey

用户选择任一 DeepSeek 模型，提交文本或该模型允许的图片，观察 reasoning/正文流式输出；模型可调用 Whale 工具并继续同一轮推理，最终返回结果和准确用量。无效模型能力或输入应在本地明确拒绝或按官方语义降级。

## 6. Interaction And Information Design

- 模型选择器列出三模型并明确 Vision 的实验性、多模态属性。
- 推理选择只展示官方有效语义：关闭、低、高、最大；内部兼容别名不得误导用户。
- 非视觉模型收到图片时不得假装完成视觉理解。

## 7. Product Rules And State Logic

- DeepSeek Responses 是无状态协议；Whale 继续本地维护并回传必要历史。
- thinking tool-use 必须保持同一用户轮次的 reasoning item 和 tool-call 配对。
- `parallel_tool_calls` 由 DeepSeek 始终启用；UI 不承诺该字段能关闭并行。
- `store`、`previous_response_id`、`conversation`、`service_tier`、`prompt_cache_key` 等不支持字段不得被宣传为生效。
- 真实验证无重试；任何错误、usage 缺失或预算风险立即停止后续调用。

## 8. Edge Cases, Errors, And Recovery

- 图片类型、角色、大小或来源非法时返回可理解错误，不丢弃为普通文本后继续声称视觉成功。
- incomplete/failed/context overflow 必须成为明确终态。
- function/custom tool 的增量与最终事件不得导致重复执行。
- Vision 实验模型下线或服务端拒绝时保留离线能力证据，并明确报告真实验证失败。

## 9. Content And Terminology

- 使用官方模型 ID；展示名称可补充版本日期，但调用仍使用稳定别名。
- `none/low/high/max` 分别表示关闭、低、高、最大 thinking effort。

## 10. Acceptance Criteria

- Given 模型选择器，when 用户查看 DeepSeek 模型，then 三个当前模型均出现且 Vision 标明图片能力。
- Given Flash 或 Pro，when 用户提交图片，then Whale 不会把模型描述为具备视觉理解。
- Given Vision，when 用户或工具提交合规图片，then `/responses` 请求保留 `input_image`。
- Given 任一模型，when 选择 thinking effort，then wire 值符合官方 Responses 契约。
- Given reasoning/text/tool/usage SSE fixture，when 解析事件，then 不丢失内容、不重复执行工具并产生正确终态。
- Given 真实验证，when 任何累计费用可能超过 1 CNY，then 在发出下一请求前停止。
- Given 所有实现改动，when 运行相关测试和缓存敏感门禁，then 全部通过后才允许提交。

## 11. Review Checklist And Sign-off Questions

- 三模型目录、协议、视觉和 thinking 是否均由测试覆盖？
- 是否明确区分官方支持、静默忽略和 Whale 本地能力？
- 真实验证是否有 planned/settled 账本记录并严格小于等于 1 CNY？

## Confirmed Product Decisions

> PROTECTED USER-AUTHORITY SECTION
> Rows in this section MUST NOT be created, modified, deleted, reinterpreted,
> or superseded without explicit user approval for that specific decision change.
> Agent self-approval is forbidden.

| ID | Confirmed Decision | Must Do | Must Not Do | Rationale | Violation Signal | Confirmation | Status |
|---|---|---|---|---|---|---|---|
| PD1 | 最大限度完整补齐当前 DeepSeek 能力支持 | 以当前三模型和官方 Responses 契约为完成边界 | 不得只允许透传模型名却宣称完整支持 | 用户要求完整补齐 | Vision 无法保留图片或模型目录缺失 | user-confirmed-direct: “最大限度完整补齐deepseek 能力支持” | active |
| PD2 | 真实测试采用 1 CNY 总预算包 | 所有真实请求合计费用硬上限 1 CNY，启动前登记，结束后结算 | 不得超额、拆分规避或先跑后补账 | 用户明确提供付费验证授权 | 累计预计或实际费用超过 1 CNY | user-confirmed-direct: “批准1元的总测试预算包用于验证” | active |

## 12. Open Questions And Risks

- Vision 为实验模型，服务端能力可能快速变化；静态目录必须由契约测试保护并在后续官方更新时刷新。
- 官方 `/models` 文档示例暂未展示 Vision，不能仅依赖该示例作为目录权威。
- 当前环境未直接暴露 `DEEPSEEK_API_KEY`；真实验证前需使用仓库既有安全凭据路径，绝不输出密钥。

## 13. Implementation Notes

- 官方事实来源：DeepSeek 当前模型页、2026-07-31/08-13/08-21 更新日志、Responses API、Thinking Mode 与 Vision 指南。
- 生产手写代码增量保持在本阶段 500 行以内；测试和 fixture 不计入该限制。
- 真实验证计划：三个模型各一次最小 smoke，最多 3 个 sample、无重试；预计不超过 12 个 provider 请求、总输入不超过 30K tokens、总输出不超过 6K tokens、最长 10 分钟，费用硬上限 1 CNY；任一失败、usage 缺失或预算风险即停止。
