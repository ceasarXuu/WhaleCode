# R7 TaskSpace tool-choice 消融结果

- Created: 2026-07-19
- Updated: 2026-07-19
- Status: Complete / Bootstrap behavior decision required
- Production Commit: `e95b5e262`
- Machine Result: `benchmarks/taskspace/r7/phase-c-tool-choice-ablation-result.json`

## 1. 实验目标

本轮同时移除两个隐式机制，观察 Agent 是否仍能自然、稳定地初始化 Map：

1. TaskSpace 不再在 bootstrap 和 terminal 阶段把 `tool_choice` 切换为命名的
   `taskspace_control`；所有生命周期请求统一使用 `auto`。
2. Chat Completions 不再因为 `required` 或命名 tool choice 自动关闭 thinking；请求保留调用方
   明确配置的 reasoning effort。

Map 状态机、空 Map 硬约束、终态硬校验、`taskspace_control` schema 和普通工具集合均未放松或分叉。

## 2. 工程结果

| 合同 | 结果 |
|---|---|
| bootstrap/work/terminal `tool_choice` | 全部统一为 `auto` |
| TaskSpace lifecycle state | 保留，只用于状态校验与结构化日志 |
| 命名/required tool choice 对 thinking 的副作用 | 已删除 |
| 空 Map 下普通工具调用 | 继续以 `no_task_path` 硬拒绝 |
| terminal 非法普通 final | 继续硬拒绝，不自动推断或重试 |
| provider tool schema | 生命周期内保持同一份 13-tool schema |

定向测试证明三种生命周期请求均为 `auto`，命名 tool choice 也不会改写显式 reasoning effort；
TaskSpace terminal fixture 继续证明最终结束必须由合法 `finish_end` 提交。

## 3. Docker 自然运行

模型统一为 `deepseek-v4-flash`，reasoning effort 为 `max`，projection policy 为 `map-append`，
执行与验证均在 Docker hard boundary 中。简单、复杂样本各运行 3 次。

| Sample | Repeat | Solved | Requests | 首轮普通动作 | `initialize_map` 次数 | 初始化所在请求 | 首请求 reasoning token | Request 2+ cache |
|---|---:|---|---:|---:|---:|---:|---:|---:|
| simple | 1 | yes | 15 | 2 | 1 | 2 | 32 | 92.99% |
| simple | 2 | yes | 14 | 1 | 1 | 2 | 24 | 92.79% |
| simple | 3 | yes | 15 | 2 | 1 | 2 | 29 | 92.47% |
| complex | 1 | yes | 15 | 1 | 1 | 2 | 26 | 93.81% |
| complex | 2 | yes | 20 | 1 | 1 | 2 | 54 | 94.27% |
| complex | 3 | yes | 13 | 1 | 1 | 2 | 25 | 92.53% |

汇总结果：

- correctness、公开验证、隐藏验证和 Map 闭合均为 `6/6`；
- `initialize_map` 最终一次成功为 `6/6`，重复初始化为 `0/6`；
- 首请求直接初始化为 `0/6`；
- 6 次运行都先调用 1 至 2 个普通工具，收到忠实的 `no_task_path` 反馈后，第二次请求才初始化；
- 92 个 provider request 的 payload capture 均为 `tool_choice=auto`，choice transition 为 0；
- 6 个首请求均产生非零 reasoning token，证明 thinking 没有在 bootstrap 被关闭；
- 简单样本 3 次合计 44 request、726,242 input token，request 2+ cache 为 92.74%；
- 复杂样本 3 次合计 48 request、967,666 input token，request 2+ cache 为 93.72%。

运行证据：

```text
target/r7-phase-c/auto-control-ablation/simple/single-file-fast-fix/20260719-021930-018
target/r7-phase-c/auto-control-ablation/simple-supplement/single-file-fast-fix/20260719-022322-602
target/r7-phase-c/auto-control-ablation/complex/subscription-billing-repair/20260719-022518-908
```

首次简单运行使用 `-RunSide right`，该参数选择盲化后的物理侧而非固定 TaskSpace 逻辑臂，因此只得到
2 次 TaskSpace 和 1 次 Standard。随后用一对完整双臂补足第 3 次 TaskSpace；最终统计只按
`logical-mode-map.json` 识别逻辑模式。

## 4. 结论

如果“稳定初始化”指最终建立正确 Map、只初始化一次并完成任务，结果是 `6/6`，可以稳定恢复。
如果它指空 Map 时首动作必须是初始化，则结果是 `0/6`，当前不能认为通过。

这不是工具反馈丢失或扭曲：空 Map projection 明确进入首请求，普通工具失败也以结构化
`no_task_path` 忠实进入下一轮上下文，Agent 随即纠正。问题发生在首轮动作选择上：

- bootstrap projection 只机械声明 `map: none` 和 `bootstrap_required: true`；
- `taskspace_control` schema 描述它是 mandatory lifecycle tool，并声明 `initialize_map` 先于 continuation；
- 但在 13 个同时可选工具和 `tool_choice=auto` 下，这两处信息不足以让模型首轮优先选择控制工具。

因此，旧命名 tool choice 不只是请求形状开关，也确实承担了 bootstrap 动作选择的强制作用。移除后
Runtime 硬门禁可以阻止越过空 Map，但代价是稳定增加一次失败反馈和一次 provider request。

## 5. 后续边界

不应恢复“命名 tool choice 同时关闭 thinking”的隐式耦合，也不应让 Runtime 推断并代替 Agent 创建
Map。若产品要求首请求初始化，后续方案应只围绕 bootstrap 时 `taskspace_control` 的工具操作合同与
可见动作面做最小设计，并继续保持 provider 请求形态、thinking 配置和状态机反馈彼此独立。

在明确接受“稳定晚一轮”还是要求“首轮初始化”之前，Phase D 不应把本现象当作已解决问题继续掩盖。

## 6. 验证

```text
just fmt                                                                 PASS
just fix -p codex-core                                                   PASS (existing warnings)
cargo test -p codex-api                                                  PASS
cargo test -p codex-core taskspace_control_modes_preserve_state_without_changing_tool_contract --lib
                                                                         PASS
cargo test -p codex-core named_tool_choice_preserves_requested_chat_reasoning --lib
                                                                         PASS
cargo test -p codex-core --test all taskspace_terminal_contract -- --test-threads=1
                                                                         2 passed
cargo build -p codex-cli --bin whale --locked                            PASS
Docker simple TaskSpace                                                  3/3 solved
Docker complex Standard / TaskSpace                                      3/3 / 3/3 solved
```
