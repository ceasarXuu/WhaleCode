# CR-06：生产 final-wire 捕获边界

- Verified: 2026-07-31
- Status: completed
- Code evidence: `d04aab5fb`
- Real Whale Agent runs: 0

## 1. 结论

DeepSeek Chat Completions 请求的权威捕获边界已经找到并由本地 mock 测试证明。测试从生产 Session 提交用户输入，
经过生产 Prompt、Tool 选择、请求对象和 Chat Completions serializer，最后在本地 HTTP endpoint 捕获实际请求体。

后续 final-wire fixture 应复用这条生产路径和现有 serializer，不得建立测试专用的第二套请求构造或序列化逻辑。

## 2. 已确认调用链

| 顺序 | 生产位置 | 职责 | 传递对象 |
|---|---|---|---|
| 1 | `core/src/session/turn.rs::build_prompt` | 从会话状态构造本轮 Prompt，并确定可见 Tool | `Prompt` |
| 2 | `core/src/client.rs::build_responses_request` | 合并 instructions、输入、Tool、`tool_choice` 和模型信息 | `ResponsesApiRequest` |
| 3 | `core/src/client.rs::stream_with_provider_request_budget` | 按 provider 的 `WireApi` 选择发送路径 | `WireApi::ChatCompletions` |
| 4 | `core/src/client.rs::stream_responses_api` | 记录 provider wire trace，并交给 API client | `ResponsesApiRequest` |
| 5 | `codex-api/src/endpoint/responses.rs::build_chat_completions_body` | 将生产请求转换为最终 Chat Completions JSON body | HTTP body |
| 6 | `codex-api/src/endpoint/responses.rs::stream_chat_completions_request` | 向 `chat/completions` 发送该 body | HTTP POST |

`core/src/provider_wire_trace.rs::record_request` 与实际 endpoint 都调用
`build_chat_completions_body`。因此现有 provider wire trace 和本地 mock endpoint 观察的是同一个生产 serializer，
不存在 trace 自行重建请求语义的平行路径。

## 3. 本地证明

测试：`third_party/codex-cli/codex-rs/core/tests/suite/cache_final_wire.rs`

测试执行以下过程：

1. 启动本地 wiremock，并只接受一次 `POST /v1/chat/completions`；
2. 创建使用 `WireApi::ChatCompletions` 和 `deepseek-v4-flash` 的生产 Session；
3. 通过 `Op::UserInput` 提交用户消息；
4. 等待生产流完成，然后读取 mock 捕获的原始请求 body；
5. 断言最终 body 中的 system/user 消息、生产 Tool 数组、`tool_choice` 和模型身份。

验证命令：

```bash
cd third_party/codex-cli/codex-rs
cargo test -p codex-core --test all cache_final_wire -- --nocapture
```

结果：`1 passed; 0 failed`。该测试只访问本地 mock，没有使用 DeepSeek API，也没有产生 Whale Agent 费用。

## 4. 后续边界

CR-06 只确认捕获边界，不实现 payload 快照、规范化、差异分类或场景矩阵。

- CR-07：在该生产边界保存原始 body SHA 和可审阅结构化证据；
- CR-08：定义哪些 final-wire 差异属于缓存相关变化；
- CR-09：覆盖普通 Tool 与 `taskspace_control` 的生产 wire schema；
- CR-10、CR-11：单独验证 usage decoder 和报告聚合口径。

因此 CR-I04、CR-I05 仍保持开放。仅当上述后续覆盖和验收全部完成后，才能关闭对应问题。
