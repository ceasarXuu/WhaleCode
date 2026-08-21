# U7：验证 DeepSeek 原生 Responses 请求与流式事件

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 生产代码变化：0
- 真实模型请求：0

## 1. 实施边界

本单元只验证 DeepSeek V4 Flash/Pro 与 Codex 0.147 原生 Responses 主链的兼容性，不恢复旧 Chat Completions 转换层，也不修改 TaskSpace、模型目录、用量、压缩或缓存策略。

依据 DeepSeek 官方兼容表，本次逐项检查：

- 请求使用 `POST /responses` 和 `stream: true`；
- reasoning 参数可进入原生请求；`reasoning.summary` 即使发送也不会产生摘要；
- `prompt_cache_key`、`store` 等不支持参数由服务端忽略；
- SSE 可承载 reasoning text、output text、function call、custom `apply_patch` 和完成/失败事件；
- API 不支持 `previous_response_id` 或 conversation，本地仍保持无服务端会话依赖。

官方资料：

- [DeepSeek-V4-Pro 正式版上线](https://api-docs.deepseek.com/zh-cn/news/news260813)
- [DeepSeek Responses API 兼容性明细](https://api-docs.deepseek.com/zh-cn/guides/responses_api)

## 2. 验证性改动

只增加两组无网络契约 fixture：

1. `codex-api/tests/clients.rs`：以显式 `deepseek` provider 构造 Pro 请求，断言目标是 `/responses` 而不是 `/chat/completions`，并验证 model、reasoning、parallel tools、store、stream 与 stream options 的序列化结果。
2. `codex-api/src/sse/responses.rs` 的测试模块：使用 DeepSeek 官方事件形状验证 reasoning delta、文本 delta、完整 function call 和不带 token detail 子结构的 completed usage。

fixture 全部直接通过上游 0.147 实现，因此没有增加 provider 条件、事件转换器或生产适配层。官方声明会忽略的字段继续由通用 Responses 请求发送；当前没有证据证明为它们增加 DeepSeek 专用过滤分支有收益。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | passed；仅 stable rustfmt 的已知 unstable-option warning |
| DeepSeek 原生请求定向测试 | 1 passed |
| DeepSeek 原生 SSE 定向测试 | 1 passed |
| `cargo test -p codex-api` | 159 passed（143 lib + 8 clients + 1 models + 6 realtime websocket + 1 SSE e2e） |
| sync replay / metadata 门禁 | 42 tests passed；inventory/replay/metadata checks passed；当前 overlay 18 路径 |
| cache regression index gate | passed；指纹 `bc96d21bae9ee62a15f72a8d686d20b7cc533eab0a4fe27560e97ca8fcff2eca`；最近一次 live 回归仍为失败 |
| 真实网络/API 请求 | 0 |

## 4. 结论与剩余边界

U7 证明当前 0.147 Responses-only 主链无需生产适配即可覆盖 DeepSeek 官方请求和核心流式事件。它是离线 wire-contract 证据，不替代 U10 的 final-wire/cache 门禁，也不提前证明模型目录和 TUI 可见性；Pro 仍按计划在 U8–U10 完成后由 U6 恢复可见。

下一工作单元为 U8：恢复 provider usage、请求预算和 terminal reconciliation。
