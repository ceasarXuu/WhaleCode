# U6：恢复 DeepSeek 模型目录与可见性

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 真实模型请求：0

## 1. 实施边界

本单元只恢复模型目录的用户可见行为：`deepseek-v4-flash` 保持默认并可见，`deepseek-v4-pro` 在正式版及 Responses 主链已验证后恢复可见。没有新增 Whale 专用 TUI 路由、provider wire 分支、TaskSpace 状态或真实 API probe。

外部事实依据为 DeepSeek 官方的 [V4 Pro 正式版公告](https://api-docs.deepseek.com/zh-cn/news/news260813) 与 [Responses API 兼容说明](https://api-docs.deepseek.com/zh-cn/guides/responses_api)。本地开放条件由 U7 的原生 Responses fixture、U9 的 Pro compaction 请求和 U10 的 final-wire/cache 证据满足。

## 2. 实现结果

- bundled catalog 中 Flash 与 Pro 均为 `list`；
- 公共模型列表只保留 `deepseek-*`，避免上游或远端目录把非 Whale 产品模型暴露给用户；
- `deepseek-v4-flash` 无论目录优先级如何始终是默认模型；
- Flash 与 Pro 均保持 `standard` 默认 reasoning，并支持 `standard`、`high`、`max`；
- TUI 继续使用 Codex 0.147 原生 `show_in_picker` 机制，同时展示 Flash/Pro 并过滤隐藏项。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| `codex-models-manager` | 50 passed |
| TUI model picker | 1 passed；快照确认 Pro 可见、隐藏项不可见 |
| sync replay / metadata | 42 tests passed；inventory/replay/metadata checks passed；当前 overlay 31 路径 |
| cache regression index gate | passed；指纹 `64a753dc368395160c04f81223fbeb8c3b93fad081b524ddf040ce8c24b408ae`；免费 final-wire 验证通过，最近一次 live 回归仍为失败且未晋升 |
| 真实网络/API 请求 | 0 |

## 4. 结论

U6 完成 D1 的恢复条件，Phase C 的 U5–U10 至此全部闭环。下一步不是直接实施 TaskSpace，而是先执行 Phase D Pre-Phase Plan Rebase Gate，按当前 0.147 seam 重新确认 U11–U16 的最小工作边界。
