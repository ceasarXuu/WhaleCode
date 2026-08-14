# U16：锁定 TaskSpace final-wire 与免费缓存合同

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 真实模型请求：0

## 1. 实施边界

本段只补 TaskSpace 请求进入 DeepSeek Responses 最终 HTTP body 的免费 mock 证据，并把该用例纳入既有缓存门禁。没有修改运行时代码、accepted/live baseline、provider 路由或 TaskSpace 状态模型，也没有恢复旧 `provider_wire_trace`。

测试通过 core 已有的 extension registry 注入只读内存 TaskSpace store。在同一线程内先发送 Standard 请求，再激活一张 canonical Map 并发送 TaskSpace 请求，由本地 wiremock 捕获两个真实序列化后的 request body。

## 2. 锁定的合同

- Standard 与 TaskSpace 均使用 `deepseek-v4-flash` 和 `reasoning.effort=standard`；
- 切换模式不改变 `instructions` 与 `prompt_cache_key`；
- 第二次请求完整保留第一次请求的 conversation input 前缀；
- TaskSpace 只新增 `taskspace_control`，移除该扩展工具后，公共工具的内容和顺序与 Standard 完全一致；
- Standard body 不含 `<taskspace_map>`；TaskSpace body 含 canonical `taskspace-canonical-map-v2` world-state；
- 动态 Map 仍位于 extension WorldState，不复制进固定 system instructions。

工具注册表按统一优先级放置扩展工具，因此合同不要求 `taskspace_control` 必须位于工具数组末尾；这种位置要求不是缓存语义，也不是 0.147 extension seam 的保证。

## 3. 免费门禁

缓存 surface contract 新增两条明确命令，分别运行：

- U10 的 Standard/compaction DeepSeek final-wire；
- U16 的 Standard→TaskSpace final-wire 与缓存前缀测试。

完整免费缓存矩阵结果：

| 验证 | 结果 |
| --- | --- |
| cache contract Python tests | 20 passed |
| `build_mcp_cache_helper` | passed |
| `prompt_caching` | passed |
| `prompt_cache_key` | passed |
| `mcp_tool_cache` | passed |
| `responses_request_contract` | passed |
| `deepseek_standard_final_wire` | passed |
| `deepseek_taskspace_final_wire` | passed |
| `codex-core --tests` Clippy | passed with `-D warnings` |
| 真实网络/API 请求 | 0 |

## 4. 结论

U16 的 slash routing、localhost viewer 和 TaskSpace final-wire/cache 三段均已完成。Phase D 的 U11–U16 至此全部 verified；旧 TUI fixture 与 Windows 验证继续按既有决策延期，live cache baseline 仍保持失败状态且未晋升。
