# TaskSpace 原生动作协议修复结果

- Date: 2026-08-19
- Issue: R8-I03
- Status: offline complete / live verification pending

## 根因

Provider 最终顶层只声明 `taskspace_exec` 和 Provider-hosted Tool，没有声明 `exec_command`。但旧
`taskspace_exec` schema、示例和成功反馈仍把工作表达为：

```json
{"tools":[{"tool":"exec_command","node_id":"inspect","input":{"cmd":"pwd"}}]}
```

这不是单纯的 Runtime 内部复用，而是向 Agent 再次暴露了一个与 Provider 顶层 Function Call 同形的内层 Tool。
历史逃逸调用同时携带原生 `cmd` 和 wrapper 专属 `node_id`，证明模型把该内层分支提升到了顶层。

## 修复

Agent 可见合同改为 TaskSpace 自身的动作语言：

```json
{"actions":[{"kind":"shell","node_id":"inspect","parameters":{"cmd":"pwd"}}]}
```

1. `taskspace_exec` 直接承载 Map 操作和 TaskSpace action，不嵌套普通 client Tool。
2. Runtime catalog 保存 `kind -> 原生 ToolName` 的机械映射；dispatch 继续复用原 Router、handler、权限、hook 和结果转换。
3. 终端、进程输入、Patch、图片读取和 Tool Search 分别显示为 `shell`、`process_input`、`patch`、`inspect_image` 和
   `discover_tools`；其他能力使用不会与顶层 Function Tool 同名的 `client::...` action identity。
4. 成功反馈改为 `action_results[].action`，Map Action 也记录 action identity，不把原生 `exec_command` 重新注入上下文。
5. 旧 `tools[] / tool / input` 直接拒绝，不保留兼容、迁移或双协议分支。
6. Standard 工具声明和原生 Tool 实现未修改。

## 离线验收

- `cargo test -p codex-core taskspace --lib --locked`: 123 passed。
- `cargo test -p codex-core base_instructions_profile --lib --locked`: 6 passed。
- `taskspace_raw_newline_self_heal_replaces_the_item_before_history_is_recorded`: passed。
- 最终 TaskSpace declaration 断言不含 `exec_command`、`write_stdin` 和 `"tool"` 字段。
- 原生 dispatch 测试证明 `kind=shell` 仍机械路由到 `exec_command -> shell_command` handler。

## 证据边界

本轮坐实了旧协议的结构诱因并完成离线修复，但没有使用 Whale Agent 预算。I03 只有在真实运行中证明初始化、后续工作、
Patch、验证和结束均持续使用 `taskspace_exec`，且不再生成顶层普通 Function Tool 后才能关闭。
