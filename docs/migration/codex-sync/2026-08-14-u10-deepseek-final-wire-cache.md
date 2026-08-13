# U10：锁定 DeepSeek Standard final-wire 与免费缓存合同

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 真实模型请求：0

## 1. 实施边界

本单元只补足 DeepSeek Standard 请求到最终 HTTP body 的确定性证据，并执行既有免费缓存门禁。没有修改运行时代码、缓存策略、accepted/live baseline 或 TaskSpace。

历史 `provider_wire_trace` 同时编码 TaskSpace projection、control feedback、receipt 和协议版本身份，不是纯 DeepSeek 适配。U10 不恢复这套强耦合观测层；其 TaskSpace 部分必须在 Phase D 随 TaskSpace 状态与协议一起审查。

## 2. Final-wire 合同

通过本地 wiremock 捕获 core 实际发出的 Responses body，锁定：

- 普通请求模型为 `deepseek-v4-flash`，compaction 请求模型为 `deepseek-v4-pro`；
- 默认 reasoning 为 `standard`；
- 不发送 DeepSeek 接受但不产出内容的 `reasoning.summary`，也不发送关联 `stream_options`；
- 不发送 unsupported verbosity `text`；
- 保留 `parallel_tool_calls=true`、`stream=true` 和 `store=false`；
- `apply_patch` 使用 DeepSeek Responses 支持的 custom tool 形态。

`prompt_cache_key`、`store` 等被 DeepSeek 忽略的字段不作为本单元新增 provider 分支的理由；缓存命中继续依赖服务端自动前缀缓存，客户端用既有前缀稳定性合同防止请求结构漂移。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| DeepSeek Standard + compaction final-wire | passed；1 个 mock 集成测试，0 外部网络 |
| `build_mcp_cache_helper` | passed |
| `prompt_caching` | passed |
| `prompt_cache_key` | passed |
| `mcp_tool_cache` | passed |
| `responses_request_contract` | passed |
| sync replay / metadata 门禁 | 42 tests passed；inventory/replay/metadata checks passed；当前 overlay 27 路径 |
| cache regression index gate | passed；指纹 `dfc5237fbd21b8abb0e4e35f13a7bf874b57b9f89c767da50538e547e9aca1a3`；当前指纹未变，最近一次 live 回归仍为失败 |
| 真实网络/API 请求 | 0 |

## 4. 结论

U10 以测试和证据完成，不新增 provider/cache 状态。下一工作单元为 U6：把 Flash 保持默认并恢复可见，同时在已完成 provider/final-wire 验证的前提下恢复 Pro 可见性，并完成模型选择器/TUI 回归。
