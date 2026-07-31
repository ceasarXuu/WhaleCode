# CR-12：Standard 两请求 final-wire 基准

- Verified: 2026-07-31
- Status: completed
- Code evidence: `31f92729e`
- Real Whale Agent runs: 0

## 1. 场景

同一个 Standard Session 连续提交两个确定性用户 turn，生产 provider 通过本地 Chat Completions endpoint 各响应
一次。测试捕获 request 1 和 request 2 的完整最终 body，而不是 Prompt 中间对象。

已验证：

- request 1 的完整消息序列是 request 2 的严格前缀；
- 两次请求的完整 `tools` 数组逐值相同；
- 两次请求的 `tool_choice` 相同；
- 模型、thinking、stream 配置及完整消息/Tool 内容进入固定快照；
- 插入一条已知 developer 消息会改变结构化 evidence。

快照：

`third_party/codex-cli/codex-rs/core/tests/suite/snapshots/all__suite__cache_payload_contract__standard_two_request_final_wire.snap`

## 2. 确定性处理

fixture 输入 cwd 固定为 `/tmp`。当前生产 TurnContext 没有测试时钟注入点，因此快照仅对以下动态输入做精确替换：

- `<current_date>` 标签值；
- `<timezone>` 标签值；
- 本次测试 `$CODEX_HOME` 绝对前缀；
- checkout source root 绝对前缀。

Skill 名称、描述、相对路径、顺序和完整上下文文本均保留，没有折叠 Skills 或权限内容。快照连续两次非更新复验
稳定，且不包含本机用户名或临时目录。

## 3. 验证

```bash
cd third_party/codex-cli/codex-rs
cargo test -p codex-core --test all standard_request_pair_preserves -- --nocapture
```

结果：`1 passed; 0 failed`。本地 mock 共接收 2 个请求，未调用真实 provider。

## 4. 边界

CR-12 只建立 Standard 基准。三种 TaskSpace projection 策略由 CR-13 分别覆盖，CR-I05 继续保持 open。
