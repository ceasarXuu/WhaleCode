# MVT-1 原生 Tool Router 复用结果

- Date: 2026-08-02
- Status: Completed
- Commit: `228c68ff8`
- Cost: 仅本地 Rust 测试，0 次真实 Whale Agent run，0 次外部 provider 请求

## 1. 结论

H1 通过：未来 Tool 序列容器中的普通 Function/Freeform Tool 可以先机械构造成原生 `ToolCall`，再交给现有
`ToolRouter -> ToolRegistry -> handler/hook` 路径执行。该路线不要求第二套 Tool handler，不要求修改普通 Tool
schema，也不要求把 node、Map 或 TaskSpace 参数写入普通 Tool。

本阶段只证明“容器项可以回到原 Router”。尚未实现序列容器、执行归属、hosted adapter 或生产 CLI 接入。

## 2. 实现边界

新增 `core/src/tools/nested_call.rs`，职责只有两项：

1. 根据已有 `ToolSpec` 区分 Function 与 Freeform 输入形态；
2. 保留 Tool 名称和 call id，构造原生 `ToolCall`。

Code Mode 原有的 payload 构造逻辑改为复用该入口。MCP 的名称解析和 payload 仍由原路径负责；实际执行继续进入
`ToolCallRuntime` 和 `ToolRouter`。没有新增执行器、fallback、TaskSpace 特判或普通 Tool 参数 decoration。

## 3. 验证证据

| 检查 | 结果 | 证明内容 |
|---|---:|---|
| 构造器单测 | 1/1 通过 | Function/Freeform 的身份与 payload 类型保持正确 |
| Router 测试 | 7/7 通过 | 两项记录型 Tool 依次穿过真实 Router，名称、输入和顺序不变 |
| Dispatch trace | 3/3 通过 | 原分派生命周期记录没有被旁路 |
| 缓存敏感面门禁 | PASS | 免费 final-wire 验证通过，指纹 `a0e06b82dc2c7eab23ecbf4a07b980fd913971e54780dadce4e2af6154faf84c` |
| 真实 Agent/provider | 未运行 | MVT-1 不需要付费运行，未消费预算 |

记录型 Router 测试的唯一事实顺序为：

```text
inspect / {"path":"README.md"}
patch   / patch body
```

两项调用共用同一个原生记录型 handler，证明不同 payload 类型不需要复制执行实现。

## 4. 已知邻接缺口

完整 `code_mode` 筛选执行 39 项，其中 38 项通过；
`code_mode_notify_injects_additional_exec_tool_output_into_active_context` 因下一请求缺少 notify marker 失败。

同一测试已在改动前基线 `b3913f965` 的临时 detached worktree 中以相同症状复现，因此它不是 MVT-1 引入的回归。
本结果不把该项记为通过，也不在 H1 中借用其证据；后续若单独修复 notify，应建立独立问题和验证范围。

## 5. 下一步

进入 MVT-2：在测试范围内为序列项声明 `Client/ProviderHosted` 执行归属，复用现有序列调度器验证
`local-A -> hosted-B -> local-C` 的唯一顺序和结果配对。若需要修改普通 Tool handler、复制 scheduler 或引入第二份
顺序事实，应立即停止并重新讨论基础路线。
