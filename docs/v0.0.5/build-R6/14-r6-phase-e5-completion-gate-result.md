# R6 Phase E5 Completion Gate 实施结果

- Created: 2026-07-16
- Updated: 2026-07-16
- Status: Completed
- Scope: Phase E5 only
- Related Design: `10-r6-terminal-replay-convergence-design.md`
- Prerequisite: `13-r6-phase-e4-terminal-hard-state-result.md`

## 1. 结论

Phase E5 已通过独立退出门禁，可以进入 E6。TaskSpace 的成功终结现在只接受已提交
`taskspace_control.finish_end` 返回的 terminal carrier；provider 普通 assistant text 不再能绕过 Map 事务形成
final answer 或成功 TurnComplete。

Runtime 没有解析文本、自动补 `finish_end`、生成纠错提示或追加 recovery request。provider 文本仍按原字节写入
history/rollout，并以 nonterminal Commentary 发布；Map 保持 OPEN，turn 以稳定协议错误结束。

## 2. 实现链路

```text
provider response
  -> buffer TaskSpace assistant text
  -> execute declared tool sequence
       -> committed finish_end returns {map_id, revision, exact summary}
  -> completion gate
       -> carrier present: release exact summary as sole FinalAnswer
       -> carrier absent + follow-up: continue ordinary tool loop
       -> carrier absent + stop attempt: publish exact text as Commentary,
          emit taskspace_terminal_protocol_violation, no retry
```

terminal carrier 是 turn-local 已提交事务凭证，不持久化第二份状态。它从 handler 经 registry、tool runtime 和
sequence 逐层透传；只有成功 `finish_end` 可以构造，失败输出和普通工具无法伪造。

## 3. 反馈与边界

1. provider assistant text 在 Map 未闭合时不改写内容，只把 UI phase 机械降为 Commentary。
2. 成功 carrier 存在时，provider 同响应中的普通文本被抑制，最终回答只来自 Agent 传入
   `finish_end.final_summary` 的原文。
3. completion gate 使用请求开始时冻结的 control mode、Map id 和 revision 作为 response contract，不重读或
   推断语义。
4. protocol violation 是非重试错误；不自动提交 Map mutation，不增加 provider request。
5. Standard 路径没有 response contract，继续沿用原有 streaming、final answer 和 TurnComplete 行为。
6. Hook 未修改，也不参与终结判断。

## 4. 实施中发现并修复的问题

E5 集成 fixture 首次执行时，bootstrap 顶层 hard-state 只暴露 `taskspace_control`，但 nested preflight 错误复用
了“provider 顶层可见工具”集合。code-mode 中 `exec_command` 等原子工具虽然被合法编入 control schema，却会被
判为不可见，导致初始化原子失败。

修复后两个概念明确分离：

- provider 顶层可见性继续由 hard-state 控制，bootstrap/terminal 仍只有 `taskspace_control`；
- control nested action 按构造 control schema 的注册工具集合校验，继续排除 `taskspace_control` 和
  `update_plan`，不放宽到任意工具。

现有 `taskspace.control_batch_preflight_failed` 日志继续记录 nested capability 拒绝；新增回归证明
code-mode nested `exec_command` 可执行，而控制工具和线性 plan 工具不能嵌套调用。

## 5. 日志建设

| 事件 | 级别 | 机械事实 |
|---|---|---|
| `taskspace_agent_final_staged` | info | tool sequence 收到 committed carrier |
| `taskspace_agent_final_released` | info | exact summary 已发布为 FinalAnswer |
| `taskspace_provider_assistant_text_suppressed_by_terminal_carrier` | info | carrier 优先，普通 provider text 未发布 |
| `taskspace_terminal_protocol_violation` | error | Map 未闭合时 provider 试图结束；含 control mode/map/revision |
| `taskspace.control_batch_preflight_failed` | warn | nested action 不属于合法 control capability 集合 |

日志只说明请求、提交和发布边界，不包含 Agent 下一步建议。

## 6. 回归结果

| 验证 | 结果 | 覆盖重点 |
|---|---:|---|
| `cargo test -p codex-core taskspace_terminal_contract --test all -- --nocapture` | 2 passed | carrier sole-final、plain final 无泄漏/无 retry |
| `cargo test -p codex-core session::turn::active_context_replacement_tests --lib` | 14 passed | response contract、Standard/TaskSpace prompt 边界 |
| `cargo test -p codex-core tools::sequence::tests --lib` | 11 passed | carrier 透传、barrier 与批处理回归 |
| `cargo test -p codex-core tools::handlers::taskspace_control::tests --lib` | 6 passed | terminal identity、summary 原样保留 |
| `cargo test -p codex-core tools::router::tests --lib` | 6 passed | code-mode nested capability 与 alias |
| `cargo test -p codex-core action_map::runtime::phase_d_tests --lib` | 7 passed | terminal frontier 与 canonical Map 回归 |
| `just fix -p codex-core` | passed | Clippy；仅有既有 warnings |
| `just fmt` | passed | Rust 格式化 |

未执行全 workspace test；本阶段按改动边界执行 targeted regression，完整 workspace 测试依项目规则需要用户
单独授权。

## 7. 阶段判定

| Gate | 判定 |
|---|---|
| 无 carrier 时不发布 TaskSpace FinalAnswer | PASS |
| plain provider text 原样进入非终态上下文 | PASS |
| protocol violation 不产生额外 provider request | PASS |
| 成功 final 只来自 committed `finish_end` summary | PASS |
| Root/Finish/revision 不被普通文本改写 | PASS |
| Standard 与 Hook 路径未被接管 | PASS |

E6 将完成故障原子性矩阵，并在 Docker 中执行 simple/complex 的 Standard、冻结 R5、当前 R6 各 3 次 live
门禁；该阶段才对自然 `finish_end` 采用率、请求、token、缓存、耗时、Map 和 replay proof 形成正式结论。
