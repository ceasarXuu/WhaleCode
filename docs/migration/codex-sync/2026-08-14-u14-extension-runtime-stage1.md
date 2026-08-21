# U14 extension runtime：第一原子段与 seam spike

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- U14 状态：`in-progress`
- 本原子段：`verified`
- 真实模型请求：0

## 1. 本段完成内容

本段只恢复不依赖 response-batch preflight 的安全能力：

- 在 `ext/taskspace` 定义窄 `TaskSpaceStore` capability，由 `codex-state::StateRuntime` 实现，避免形成 `state ↔ extension` Cargo 环依赖；
- app-server 使用现有 extension install seam 注入 store，不新增 TaskSpace 数据库或 core/session/provider 专用分支；
- thread start/resume 与 turn start 从 U13 唯一 store rehydrate canonical Map；
- subagent `ThreadSpawn` 继承 parent Map，并只增加 `Child` binding；AgentGraphStore 不保存 Map；
- 已绑定线程按需暴露只读 `taskspace_control(action=read_map)`；未绑定 Standard 线程不暴露工具；
- canonical Map 通过 extension-owned WorldState section 发布，snapshot 未变化时不重复输出；
- U13 的公开 state 类型移动到 TaskSpace crate，并由 state 保持兼容 re-export；持久化实现与表结构不变。

## 2. seam spike 结论

Codex 0.147 已覆盖单个 Tool、Tool lifecycle、Thread/Turn lifecycle、WorldState 和 event sink，但没有覆盖 TaskSpace 既有 action manifest 所需的“整次模型响应内所有并行 Tool calls 在 dispatch 前统一校验”。当前 core 在流式收到每个 tool item 后立即排队执行；`ToolLifecycleContributor::on_tool_start`：

- 只能观察单个已接受调用；
- 不能拒绝或延迟 dispatch；
- 不携带完整 sibling batch 或稳定 response-call index；
- 因而不能保证 `initialize_and_execute/execute` manifest 与普通 sibling tools 原子匹配。

本段没有恢复旧 core handler、nested dispatcher 或无门禁并行执行。U14 后续要恢复写工具，必须先选择并批准一个产品/host seam：

1. 推荐：增加窄的 response tool-batch preflight contributor，由 host 在 dispatch 前提供完整有序 call batch；保持原 action manifest 和并行原生工具语义，但会对 0.147 tool stream/dispatch 增加一个明确扩展门；
2. 改为串行两轮协议：先提交 TaskSpace transaction，再在下一模型响应执行普通工具；无需新增 core seam，但增加请求与延迟，并改变既有产品合同；
3. 不接受：在 TaskSpace tool 内嵌套分发普通工具，或仅用非阻断 lifecycle observer 做事后归属；前者破坏原生工具权限/身份，后者无法阻止未归属动作。

## 3. 验证

| 验证 | 结果 |
| --- | --- |
| `cargo check --offline -p codex-taskspace-extension -p codex-state -p codex-app-server` | passed |
| `cargo test --offline -p codex-taskspace-extension --lib` | 36 passed |
| `cargo test --offline -p codex-state --lib` | 177 passed |
| 三 crate Clippy `-D warnings` | passed |
| cache regression index gate | passed；指纹 `d71b01586c4d3ef7e57493bba31e1ecab5b81dcf2e6177c244593560f5527ba4`；免费 final-wire 通过，live baseline 未晋升 |
| parent→child binding、read tool、WorldState diff | passed |
| 未绑定 Standard surface | no tool、no TaskSpace WorldState |
| 手写生产改动 | 438 行新增；小于 500 行门禁；Codex 原生大文件仅做窄 install 改动 |

## 4. 未完成边界

U14 尚未完成：Map 初始化/execute/reopen/finish、reservation release、tool lifecycle attribution、terminal gate 与 extension event emission 仍待 response preflight 决策。U15 RPC 和 U16 TUI/final-wire 未开始。
