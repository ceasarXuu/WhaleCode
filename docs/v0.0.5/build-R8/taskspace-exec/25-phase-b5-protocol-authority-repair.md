# Phase B5 Outer Exec 协议单一权威修复

- Date: 2026-08-09
- Status: Implemented and verified offline / Provider revalidation pending
- Unit: VA-02R
- Scope: `taskspace_exec` 模型可见操作合同，不改变执行语义

## 1. 触发证据

VA-02 的首个真实响应把内部 `exec_command` 提升为顶层 Function Call，同时把 `node_id` 混入原生参数。模型已经看到
内部能力名称和部分结构，但没有形成以下完整调用模型：

```text
top-level taskspace_exec
  -> calls[]
     -> map operation {tool, arguments}
     -> client invocation {tool, node_id, arguments|input}
  -> hosted_bindings[]
```

Runtime 的零副作用拒绝正确，Router 也没有泄漏顶层 client Tool。缺口位于 Agent 可见的 outer Tool affordance：旧
description 只有五句字段摘要，完整序列规则只存在于 schema、preflight 和产品文档中，模型没有得到一份自包含的操作合同。

## 2. 最新 Codex 主线结论

本轮使用 OpenAI Codex 官方仓库 `main` 的 `646f7c0a91b8e327d263335da68ae8ef212895ce`（2026-08-09）作为对照，
不是 Whale 当前 vendor 基线。

Codex `exec` 的完整生效链不是单独依赖 prompt：

1. `code-mode-protocol/src/description.rs` 集中维护 `exec` 的调用方式、运行环境、helper、示例和嵌套 Tool 声明。
2. `core/src/tools/spec_plan.rs` 从同一有效 `ToolRegistry` 同时选择嵌套能力、生成 model-visible declaration，并注册
   `exec` handler；没有另一份手写 Tool inventory。
3. `core/src/tools/code_mode/delegate.rs` 把嵌套调用交回同一 turn 的 `ToolCallRuntime` 和原 Router。
4. `tools/src/tool_output.rs` 从原 Tool output 统一转换嵌套结果，不另造反馈语义。
5. model metadata 显式声明 `tool_mode`；不支持 Code Mode 的模型会收到兼容性告警。因此上游实现证明 runtime 合同完整，
   但不能替代 DeepSeek 的真实遵循验证。

最新主线近期还删除了旧 code-mode metadata inventory，并让 effective exposure、名称冲突选择、prompt declaration 和
dispatch 共用同一 registry identity。这进一步说明正确方向是收敛协议权威，而不是继续向多个 prompt 层补提醒。

## 3. 本轮设计

Whale 不照搬 JavaScript Freeform wire，只复用上游的信息架构：

| 位置 | 唯一职责 |
|---|---|
| Base instructions | 通用 coding agent 工作方式；不包含 TaskSpace JSON 或合法序列 |
| `taskspace_exec` description | 唯一模型可见 outer 操作合同：入口、调用包装、节点归属、Hosted 对位、序列规则、最小示例 |
| Catalog schema | 从原 ToolSpec 和 Map operations 机械生成精确字段与参数结构 |
| Decoder/preflight | 只执行结构与硬底线校验，不补全、重排、修复或解释 Agent 动作 |
| Router/Tool output | 复用 Standard 原生执行与结果语义 |

description 只在 catalog 实际包含 `exec_command` 时加入对应首次示例，避免描述不存在的能力。Hosted 类型列表同样来自
当前 catalog。普通 Tool 的参数说明仍只来自原 ToolSpec，没有第二份手写清单。

首次示例不是不可验证的提示词片段。生产代码构造同一个 JSON value，再由测试送回正式 catalog decoder 和完整 preflight；
任何字段、序列或 Map 规则漂移都会使离线测试失败。

## 4. 实施边界

新增 `taskspace_exec/protocol.rs`，由 `catalog.rs` 构建最终 Function declaration 时调用。没有修改：

- Whale Standard 或 TaskSpace base instructions；
- 普通 Function/Freeform/Namespace/MCP Tool schema；
- Provider tool choice；
- Map 状态机、合法序列定义或节点状态；
- Router、权限、sandbox、hook、dispatch 或结果反馈；
- Runtime 对非法顶层 client Tool 的拒绝行为。

因此本轮只修复协议暴露缺口，不用 prompt 惩罚、强制 tool choice 或 Runtime 自动包装掩盖模型输出。

## 5. 离线验证

```text
cargo test -p codex-core taskspace_exec --lib
60 passed; 0 failed
```

新增验收覆盖：

1. outer description 明确唯一顶层入口、`node_id` 位置和 Runtime 非推断边界；
2. 首次初始化并执行示例可被同一 catalog decoder 解码；
3. 同一示例通过正式 preflight，生成 canonical Map 并接纳一个 `exec_command` client call；
4. declaration 仍按 catalog 确定性生成，递归 Tool、Runtime identity 与 revision 不进入 Agent 参数；
5. 原 TaskSpace Exec 路由、Hosted、dispatch、持久化和反馈测试继续通过。

缓存敏感面门禁随后通过：

```text
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
PASS 13697c67477cb0b1bdb6411b66f0e7aa9195d50788f1ea5bb68ce0f736df9ff6
```

门禁确认 Standard final-wire 未被本轮 TaskSpace declaration 变更污染，并保持真实发布复验阻断。Base instructions 的
负向测试同时禁止出现 `taskspace_exec`、`initialize_map` 和 `hosted_bindings`，防止后续再形成 prompt 层平行合同。

## 6. 剩余边界

离线通过不能证明 DeepSeek 已稳定采用 outer Exec。下一步必须先通过缓存敏感面门禁；门禁若要求真实回归，则按全局规则
说明 declaration 变化并申请专项预算。之后另行申请 VA-02 单样本复验预算，首个结构失败仍立即停止，不自动重试。

VA-02 复验通过前不得启动 VA-03 四臂测量，也不得关闭 I03。

## 7. 上游资料

- [Codex exec 协议渲染](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/code-mode-protocol/src/description.rs)
- [Codex 单一 Tool registry 与 code-mode 注册](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/core/src/tools/spec_plan.rs)
- [Codex 嵌套调用委托](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/core/src/tools/code_mode/delegate.rs)
- [Codex 模型 Tool Mode 元数据](https://github.com/openai/codex/blob/646f7c0a91b8e327d263335da68ae8ef212895ce/codex-rs/models-manager/models.json)
- [删除旧 code-mode metadata inventory](https://github.com/openai/codex/commit/8e4b10446eed7bafb39d8a469f9be25a41f4864f)
