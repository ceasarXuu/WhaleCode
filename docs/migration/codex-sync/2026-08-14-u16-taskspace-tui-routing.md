# U16 TaskSpace TUI slash 与 typed RPC 路由

## 结论

U16 第一原子段已恢复 `/taskspace` 与 `/task-show` 的最窄用户入口，并接到 U15 的 typed App Server RPC：

- `/taskspace` 顺序提交 `MapRuntimeMode::Taskspace` 和 snapshot read；
- `/task-show` 只读取当前 snapshot，不改变 mode；
- Standard 仍是默认值，没有自动启用 TaskSpace；
- snapshot 直接来自 canonical service/read seam，没有 TUI 本地状态副本。

在 browser viewer 接入前，read 结果以 Map ID、revision 和 work node 数量组成文本摘要；无 Map 时明确显示当前 mode。下一原子段会把相同 read RPC 接到 localhost browser viewer，不改变命令和 mode 语义。

## 兼容收口

U15 新增事件使 TUI 与 MCP server 的 exhaustive match 出现编译缺口，本段一并做最小收口：

- TUI 将 `thread/taskspace/updated` 正确归属到对应 thread；当前不额外渲染消息，viewer 后续通过 canonical read 刷新。
- MCP tool runner 继续把该事件作为已转发、无需专门终止处理的事件。

## 验证

- `cargo check -p codex-tui`：通过。
- slash parse/availability：1 passed。
- `/taskspace` mode→read 顺序与 `/task-show` read-only：2 passed。
- 本段手写生产新增 113 行，未发送真实模型请求。
