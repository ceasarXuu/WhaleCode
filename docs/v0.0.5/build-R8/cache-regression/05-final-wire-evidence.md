# CR-07：可复算 final-wire 证据

- Verified: 2026-07-31
- Status: completed
- Code evidence: `11d5b2bdd`
- Real Whale Agent runs: 0

## 1. 已实现合同

`core/tests/common/cache_payload.rs::FinalWireEvidence` 直接接收本地 mock endpoint 捕获的原始 HTTP body，并生成：

1. `raw_body_sha256`：对生产 serializer 实际输出字节计算 SHA-256；
2. `structured_body`：完整解析后的 JSON 值，用于审阅字段、消息和数组顺序；
3. `render()`：同时包含上述两部分的稳定、可读 JSON 表示。

该构造器不删除字段、不裁剪字符串、不重排数组，也不替换路径、时间或 ID。后续场景必须在 fixture 输入端消除
非确定性，不能通过输出端宽泛归一化隐藏生产变化。

## 2. 已验证行为

| 用例 | 预期 | 结果 |
|---|---|---|
| 相同原始 body 重复构造 | SHA 和结构化证据完全相同 | passed |
| 只有 JSON 空白不同 | 原始 SHA 不同，结构化 body 相同 | passed |
| 字段值改变 | 结构化 body 不同 | passed |
| 数组顺序改变 | 结构化 body 不同 | passed |
| 非法 JSON | 明确失败，不生成残缺证据 | passed |
| 生产 Session 到本地 Chat Completions endpoint | 可从实际捕获 body 构造并重复复算证据 | passed |

验证命令：

```bash
cd third_party/codex-cli/codex-rs
cargo test -p core_test_support --lib cache_payload -- --nocapture
cargo test -p codex-core --test all cache_final_wire -- --nocapture
```

结果分别为 `3 passed; 0 failed` 和 `1 passed; 0 failed`。测试只使用本地 mock。

## 3. 边界

CR-07 提供证据表示，不决定变化的产品含义：

- 哪些字段属于缓存相关保护面由 CR-08 定义；
- 完整 Tool schema 生产场景由 CR-09 覆盖；
- Standard 和 TaskSpace request pair 快照由 CR-12、CR-13 建立；
- 当前 v1 发布门尚未接入该证据，CR-I08 保持 open。
