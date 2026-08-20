# R8 I05 / I07 收敛审查

## Review Control Contract

- 状态：进行中
- 轮次预算：最多 2 轮；首轮为完整审查，只有已接受 blocker 需要闭合时才启动第二轮。
- 基线与回滚点：`91da7eff8`
- 原始目标：修复 I07 request-fact consumer inventory 漂移；修复 I05 顶层 client Tool 逃逸导致 TaskSpace 会话 Fatal、Agent 无法纠正的问题。
- 验收条件：
  - I07 仍只有 canonical request facts 作为统一请求事实模型，清单与实际消费者一致。
  - TaskSpace 顶层 client Tool 逃逸保持零副作用，不被自动包装或执行。
  - 原始错误调用及与其 `call_id` 配对的准确失败反馈进入正式历史，下一次 provider 请求可纠正。
  - 仅该已知可纠正协议错误降级；响应身份、Map 快照、持久化和多 Exec 等完整性错误仍为硬失败。
  - Standard 模式和合法 `taskspace_exec` 路径不退化。
- 明确非目标：不新增提示词约束，不修改原生 client Tool schema，不自动推断 node 绑定，不引入并行 carrier，不执行真实 Whale Agent run。
- 允许改动：响应级校验分类、顶层 TaskSpace 直调入口的机械拒绝与反馈、本地回归测试、I07 inventory 元数据与门禁测试。
- 禁止或需审批：新的产品状态/Map 语义、provider 协议变化、真实 API 运行、范围外重构。
- 权威依据：用户确认的 Runtime 边界与真实运行故障（E0/E2）、`AGENTS.md`（E1）、现有 TaskSpace Exec 合同与测试（E1/E2）。
- 预期收益：协议错误不再终止会话，Agent 获得忠实反馈后可自行纠正；观测门禁恢复可信。
- 可接受副作用：发生真实逃逸时增加一次纠正请求；合法路径不得增加请求或输入。

## Round 1 Launch

- 轮次：initial
- reviewer：fresh internal subagent，`fork_context=false`
- 目标文件：
  - `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/taskspace_exec/response_scope.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/taskspace_exec_handler_tests.rs`
  - `scripts/taskspace-benchmark/request-fact-consumers.json`
  - `scripts/taskspace-benchmark/test_request_fact_consumers.py`
- 已有验证：request-fact Python 测试 19/19；TaskSpace Exec Rust 测试 75/75；`cargo fmt --check` 通过。
- 重点攻击：错误是否可能在判定前产生副作用；反馈是否真正进入下一请求；可恢复分类是否过宽；合法 Exec/hosted/Standard 是否受影响；inventory 是否只是掩盖第二套事实源。
- reviewer id：`01a0109c-3219-7580-89d3-5b65a2e66734`
- freshness：`fork_context=false`，只提供中性导航包。

## Reviewer Output

1. `P1 / E2`：`r7-request-observability.ps1` 仍从 raw wire terminal 重建 request identity、attempt、terminal 和 usage，
   生产 report 使用该模型重新生成 summary；consumer inventory 仅靠词法分类把它标成 canonical，I07 实际仍有第二套请求事实。
2. `P1 / E4`：response scope 先返回 recoverable client escape，再检查 `exec_call_count > 1`；同响应 escape + 多 Exec
   会把完整性错误错误降级。
3. `P1 / E4`：顶层 client guard 位于 `ToolRouter::build_tool_call` 之后；malformed escaped call 会先在原生参数/alias
   解析失败，反馈可能失去原 `call_id`，既有 runtime 单测没有覆盖正式 history/build 路径。

## Main-Agent Disposition

1. `accept`：提交 `7a4346156` 删除 raw terminal/usage 重建；生产 report 直接传 canonical facts，raw wire 只保留
   shape/LCP/transport/final-control identity。canonical facts 新增重复 wire attempt/terminal fail-closed。
2. `accept`：`validate_finalized` 先检查 Fatal 和结构完整性，再返回 recoverable escape；新增复合反例测试。
3. `accept`：在 `handle_output_item_done` 的原生参数解析前按原 ResponseItem 生成同 `call_id` 失败 output，并保留 runtime
   dispatch 防御；新增正式 history/build 路径测试。I05 修复提交为 `e596d2f27`。

修复后验证：request facts 22/22、五层 trace、Provider token identity、完整 observability report、performance self-test、
TaskSpace Exec 77/77、聚焦 session/router tests 和 `cargo fmt --check` 全部通过。

## Round 2 Launch

- 轮次：closure
- reviewer id：`01a010af-8ef2-7001-929c-4385e94d2e6f`
- freshness：`fork_context=false`，只允许检查上述三个已接受 blocker 及其直接相邻回归。
- 当前 HEAD：`e596d2f27`
- 状态：completed

## Round 2 Reviewer Output

- P0/P1/P2：无 finding；未发现修复直接引入的相邻破坏。
- blocker 1（I07 单一请求事实模型）：`closed`。
- blocker 2（escape 不得遮蔽多 Exec 完整性错误）：`closed`。
- blocker 3（解析前配对反馈、零执行、Hosted 保留）：`closed`。
- closure：`yes`。
- reviewer 指出的非 blocker 测试缺口：重复 wire attempt 直接用例、真正非法 JSON 的解析前拒绝 fixture、完整
  provider-stream 复合测试。

## Residual Gap Disposition

1. `accept / resolved`：新增 equal duplicate wire attempt Python contract test；request-fact tests 22/22。
2. `accept / resolved`：session fixture 改为真正未闭合 JSON，确认原 `call_id` 反馈仍在解析前生成。
3. `defer / non-blocking`：完整 provider-stream 复合场景由下一次获批真实验收覆盖；现有 response-scope、session、router 和
   生产链测试已分别锁定硬不变量，不为离线闭环新增重型 harness。

## Governor And Closure

- 当前收敛状态：converged
- 关闭结论：三个首轮 blocker 均闭合；第二轮无新 blocker，允许进入真实验收。
- 第二轮：仅在首轮存在已接受 blocker 时使用。
