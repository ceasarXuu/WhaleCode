# Phase B3 EX-05 原生 Client Tool 分发结果

- 日期：2026-08-07
- 状态：离线验证完成
- 代码提交：`3b578b08d`
- 真实 Whale Agent run：0

## 1. 本单元证明了什么

TaskSpace Exec 已能把通过完整预检的内部 client calls 机械还原成 Codex 原生 `ResponseItem`，并复用
`ToolRouter::build_tool_call` 与 `ToolCallRuntime` 执行。它没有复制一套 Tool handler、alias、MCP、Tool Search、
并行锁或错误反馈逻辑，也没有把 `node_id` 写入普通 Tool 参数。

所有内部 client calls 必须先全部成功构造为原生调用，之后才开始执行。因此，任一调用在原生构造阶段失败时，
整批仍保持零 Tool 副作用。进入执行阶段后，Runtime 延续 Standard 的 Tool 并行属性；结果以真实完成顺序产生，
为后续逐项写回 Map 保留最低延迟，不人为等待同批慢工具。

## 2. 验证证据

| 检查 | 结果 |
|---|---|
| Function / Freeform / Namespace / Tool Search 原生还原 | PASS |
| `exec_command` 等原生 alias 解析 | PASS |
| 可并行 Tool 并发执行并按完成顺序返回 | PASS |
| 不可并行 Tool 延续原生串行策略 | PASS |
| Tool 失败保持 Standard 原生失败 payload | PASS |
| `cargo test -p codex-core taskspace_exec --lib --quiet` | 39 passed |
| `cargo test -p codex-core tools::router --lib --quiet` | 7 passed |
| `cargo test -p codex-core action_map --lib --quiet` | 17 passed |
| `cargo check -p codex-core -p codex-state -p codex-cli --tests` | PASS |
| TaskSpace zero-base gate | PASS |
| 缓存回归门禁 | PASS，免费 final-wire 验证，无真实 API 请求 |

## 3. 明确未完成的范围

EX-05 不负责 Map 持久化和结果结算。当前整图 `canonical_json` Store 不会作为逐 Tool 结算的临时生产方案。
后续 MS-01～MS-03 将先建立关系化 canonical Map、细粒度 CAS transaction 和逐项 Action outcome 结算，之后
再处理 Hosted 核对、唯一反馈与生产注册。

因此，本单元只收口“TaskSpace Exec 能否无侵入复用 Standard client Tool 执行链”这一工程边界；答案为是。
