# U14 extension runtime：response tool-batch preflight

- 日期：2026-08-14
- 状态：`verified`
- 范围：U14 第二原子段，仅建立完整响应级工具门禁及当前只读 TaskSpace 规则
- 真实模型请求：0
- 手写生产代码：229 行新增、1 行删除；单个新增模块均小于 500 行

## 结果

Codex 0.147 extension API 现在提供通用、默认空操作的 `ToolBatchPreflightContributor`。Core 只在收到真实 `response.completed` 且该响应包含工具调用时，按原始顺序把完整批次交给已注册 contributor；所有 contributor 通过后才轮询既有工具 future。任一 contributor 拒绝时，原工具 future 全部丢弃，每个 provider call 都收到失败配对输出，整批保持零 dispatch。

TaskSpace extension 注册了首个实现：仅在线程已绑定 canonical Map 且响应包含 `taskspace_control` 时生效，当前验证单个 control、control 必须位于首位、函数参数合法，以及 `read_map` 不得携带 sibling 工具。未绑定 Standard 线程不改变工具执行路径；不完整流、流错误和邮箱抢占也不冒充完整响应进入该门禁。

## 最小设计边界

- 扩展 API 只暴露稳定的 call id、tool name、payload 和三层 extension store，不暴露 `Session`、`TurnContext` 或 core router 类型。
- 不恢复旧 `tools/sequence`、session TaskSpace 分支、provider wire trace 或 nested dispatcher。
- 本段不恢复 `initialize_and_execute`、`execute`、`reopen_map`、`finish_map`，也不提交 reservation；action manifest 的业务校验和事务准备留在 U14 下一原子段。
- 无 contributor 时 registry 循环为空；TaskSpace contributor 在线程未绑定时直接放行，因此 Standard 行为不受影响。

## 验证

- `cargo check -p codex-extension-api -p codex-core -p codex-taskspace-extension`
- `cargo test -p codex-extension-api --test registry`：6 passed
- `cargo test -p codex-taskspace-extension runtime_extension_tests`：3 passed
- `RUST_MIN_STACK=33554432 cargo test -p codex-core --test all tool_batch_preflight::rejected_complete_response_closes_call_without_dispatch`：1 passed；拒绝后 executor dispatch 计数为 0，模型收到带稳定错误码的配对输出
- `cargo fmt --all`

首次以默认测试线程栈运行新增 core integration test 时发生测试 worker stack overflow；固定 `RUST_MIN_STACK=33554432` 后进入真实断言，并发现空工具响应也会调用 contributor。实现随后收紧为“完整且非空工具批次”才调用，最终用例通过。这不是产品 fallback，也未降低断言。

## 下一步

U14 下一原子段在本 seam 上实现 action manifest 的完整顺序匹配、canonical transaction prepare/reservation、普通 sibling 的 node attribution 与完成/释放。完成这些语义前，现有 `taskspace_control` 仍只公开 `read_map`。
