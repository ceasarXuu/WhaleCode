# I03 多 outer Exec 复发与 fatal 恢复缺口

- Date: 2026-08-20
- Commit: `5ce2cfaf8`
- Ledger: `WAR-20260820-214337-R8-MAP-REQUEST-R3`
- Run: `release-dispatch-repair × map-request × TaskSpace-only`
- Result: 第 1/3 轮失败即停；2 个 Provider 请求；零 Tool 副作用

## 逐请求事实

| Request | Agent 动作 | Runtime 结果 |
|---:|---|---|
| 1 | 一个 `initialize_and_work`，建立 `root -> inspect -> fix -> verify -> finish`，在 `inspect` 执行目录发现 | 接受；命令成功；`inspect=in_flight` |
| 2 | 同一 Provider response 并列两个 `taskspace_exec(type=work)`；二者都绑定 `inspect`，分别读取实现文件和测试 | response scope 识别 `exec_call_count=2`，整批零执行；turn 以 fatal 结束 |

请求 2 的两个调用各自都有合法 `type`、`tools[]`、`node_id=inspect` 和完整 `exec_command.input`。这说明任务语义、节点归属和
Tool 输入没有丢失或扭曲；错误仅发生在 outer 调用基数：Agent 把两个可以放入同一 `tools[]` 的读取动作拆成两个同级 outer Exec。

## 两层根因

### Agent 生成层

唯一顶层 Tool 名称不能由 Function schema 表达“同一响应最多调用一次”。Base `3.0.8` 和 Tool description 已明确一响应一个
outer Exec，历史 7 次 TaskSpace / 59 个响应未复发，但本轮再次出现，证明文字合同降低了频率，尚未形成结构性保证。

本轮没有上下文缺失证据：Request 2 收到了 Request 1 的 outer call、成功 output、完整 owner 状态和目录结果；两个错误调用也
保留同一正确 owner。不得把根因改写成 Map、Tool 结果或 node 状态丢失。

### Runtime 恢复层

`TaskSpaceExecResponseScope::finalize` 正确在任何 Tool 执行前拒绝 `exec_call_count > 1`，但 `turn.rs` 只把顶层 client Tool
escape 标成 recoverable。多 outer 错误直接映射为 `CodexErr::Fatal`，发生在 drain 两个 pending Tool call 之前，因此：

- 两个 outer call 都没有收到对应的失败 output；
- Agent 没有下一请求读取拒绝原因并纠正；
- observer 只能报告 `exec_result_missing` 和 `trace_outer_call_missing`；
- runner 将业务结果记为 interrupted，并按停止条件结束批次。

这是明确的反馈/恢复链路缺口，不是要求 Runtime 合并两个调用。Runtime 应继续拒绝整批、保持零副作用，同时为两个原始
`call_id` 各返回同一 response-level 合同错误，让 Agent 在下一请求自行重新组织动作。Runtime 不应选择、合并、重排或执行其中
任何一个 outer call。

## 成本与停止

| Runs | Requests | Input | Cached | Uncached | Output | Agent wall | CNY |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1/3 | 2 | 26,722 | 19,584 | 7,138 | 655 | 7.282s | 0.00883968 |

两个 preflight 中止分别由缺少 binary attestation 和当前 shell 未加载 `.env.local` 引起，均发生在 Provider 请求前、费用为 0；
二者不计为 Agent sample。真实第 1 轮触发 `StopOnAnySideFailure` 后没有重试或补跑。

## 后续边界

后续修复应只把“多个 outer Exec”从 session-fatal 调整为 response-level、逐 call-id、零副作用、可继续的合同拒绝，并补齐
observer 对该失败的可比计数。是否进一步改变 Agent 生成合同需单变量证据；不得自动合并 outer calls，也不得修改 client Tool
原生 schema、Map 状态或 Tool 执行结果。

该修复已完成离线验证，结果见
[`94-i03-multi-outer-recovery-result.md`](94-i03-multi-outer-recovery-result.md)。本文件保留原始根因与修复前证据；自然在线恢复
仍待后续获批样本验证。
