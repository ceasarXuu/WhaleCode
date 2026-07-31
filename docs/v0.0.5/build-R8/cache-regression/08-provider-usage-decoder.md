# CR-10：provider usage 解码合同

- Verified: 2026-07-31
- Status: completed
- Code evidence: `01e4cc915`
- Contract version: `whalecode-provider-usage-v1`
- Real Whale Agent runs: 0

## 1. 单一冻结 fixture

`third_party/codex-cli/codex-rs/codex-api/tests/fixtures/provider_usage_contract.json` 同时保存 Chat Completions 与
Responses API 的四类 provider payload：

1. 缓存命中；
2. 明确未命中；
3. usage details 缺失；
4. cached token 字段类型错误。

每个有效 case 都给出统一 `TokenUsage` 期望值。fixture 带独立 schema 版本，CR-11 的 Python 聚合必须直接复用
这份数据，不能另写第二份数字口径。

## 2. 已冻结的生产行为

| 输入 | Chat Completions | Responses API |
|---|---|---|
| cached tokens > 0 | 映射到 `cached_input_tokens` | 映射到 `cached_input_tokens` |
| cached tokens = 0 | 明确 cache miss | 明确 cache miss |
| details 整体缺失 | `cached_input_tokens=0` | `cached_input_tokens=0` |
| cached tokens 类型错误 | usage chunk 无法反序列化，最终表现为 usage 缺失 | `process_responses_event` 返回 stream error |

最后一行存在 wire 行为差异。CR-11 必须把“usage 缺失”和“usage 解码错误”都归类为不可比较，不能按 0 命中率或
正常 miss 继续晋升。

## 3. 验证

```bash
cd third_party/codex-cli/codex-rs
cargo test -p codex-api
```

结果：所有 test target 共 134 个测试通过，0 失败。未调用真实 provider。

## 4. 边界

CR-10 只冻结 Rust decoder 的真实行为。Python 对 request 2+ 的聚合、缺证据拒绝和跨语言一致性属于 CR-11。
CR-I04 在 CR-11 完成前继续保持 open。
