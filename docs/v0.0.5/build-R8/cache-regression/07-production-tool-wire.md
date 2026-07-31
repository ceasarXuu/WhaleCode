# CR-09：生产 Tool wire 合同

- Verified: 2026-07-31
- Status: completed
- Code evidence: `45284b5de`
- Real Whale Agent runs: 0

## 1. 已验证事实

本地 Chat Completions endpoint 分别捕获 Standard 与 TaskSpace 生产 Session 的最终请求体。TaskSpace 请求中：

- `taskspace_control` 是第一个 Tool；
- 原生线性 `update_plan` 不可见；
- Tool 总顺序固定进入快照；
- `taskspace_control` 的完整 description 和 parameters schema 固定进入快照；
- 普通 `exec_command` 的完整 wire 定义固定进入快照；
- Standard 与 TaskSpace 的 `exec_command` 逐值相同，TaskSpace 未改写普通 Tool schema。

快照位于：

`third_party/codex-cli/codex-rs/core/tests/suite/snapshots/all__suite__cache_final_wire__taskspace_production_tool_wire.snap`

快照直接来自生产 Session、生产 Tool serializer 和生产 Chat Completions serializer，不存在测试专用 Tool 构造器。

## 2. 验证

```bash
cd third_party/codex-cli/codex-rs
cargo test -p codex-core --test all cache_final_wire -- --nocapture
```

非更新模式复验结果：`2 passed; 0 failed`。测试只使用本地 mock。

快照包含 13 个有序 Tool 名称及两份完整 Tool 定义，没有会话 ID、文件路径、时间戳等动态字段。Tool 名称、顺序、
描述或参数 schema 发生变化时测试会失败并显示差异。

## 3. 边界

CR-09 不评价现有 Tool schema 的产品质量，也不修改 Tool 设计。它只保证相关生产变化可见。

- usage decoder 仍由 CR-10、CR-11 处理；
- Standard 与三种 TaskSpace 策略的 request pair 仍由 CR-12、CR-13 建立；
- MCP、Skills、Apps、Plugins 等条件 Tool 集合由 CR-15 至 CR-17 覆盖。

因此 CR-I04、CR-I05 继续保持 open。
