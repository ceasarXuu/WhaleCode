# TaskSpace 原生动作协议修复结果

- Date: 2026-08-19
- Issue: R8-I03
- Status: reverted / historical evidence only / I03 remains verifying

> 2026-08-19 设计澄清：`taskspace_exec` 替代的是 Codex 顶层 `exec` 超级工具，不是替代或重命名其内部原生 Tool。
> 本文记录的 `exec_command -> shell action` 候选基于错误的替代层级，commit `3750a3932` 已整体回退。以下实现与运行数据
> 仅作为失败候选的历史证据保留，不代表现行架构或修复方向。

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

本轮坐实了旧协议的结构诱因并完成离线修复。I03 只有在真实运行中证明初始化、后续工作、Patch、验证和结束均持续使用
`taskspace_exec`，且不再生成顶层普通 Function Tool 后才能关闭。

## 真实运行复验

获批计划是 `release-dispatch-repair × map-request × repeat=5`。通用 pair runner 会在物理 left/right 间交替放置逻辑模式，
但本轮错误使用 `RunSide=right`，实际得到 3 次 map-request 和 2 次 Standard；因此不能声称完成五次 TaskSpace 验收。

三次有效 TaskSpace 运行均通过业务、公开验证、隐藏 Oracle 并闭合 Map，共 34 requests、639,189 input、569,856 cached、
69,333 uncached、18,891 output：

1. 顶层 `exec_command` 为 `0/3 runs, 0 calls`，旧同形名称提升未复现；
2. 两轮把新的 `kind=shell` 提升成未声明顶层 `shell`，合计 5 calls；Runtime 全部在副作用前拒绝，Agent 下一请求恢复；
3. 另一轮始终使用 `taskspace_exec`，但有两次普通 schema 错误并在下一请求纠正；
4. 自动把非法顶层 action 包回 Exec 不可接受：两次误用没有 `node_id`，Runtime 无法忠实恢复 Agent 未声明的节点归属。

本轮只证明错误候选会把被提升名称从 `exec_command` 改成 `shell`，没有解决抽象层级逃逸。它不能证明内部原生 Tool
应该被隐藏、改名或替换。I03 保持 `verifying`；原计划中的缓存双臂因行为验收未通过而未执行，不记录为零值结果。

证据：`benchmarks/taskspace/r8/evidence/WAR-20260819-064028-R8-NATIVE-ACTION-R5.json`。
