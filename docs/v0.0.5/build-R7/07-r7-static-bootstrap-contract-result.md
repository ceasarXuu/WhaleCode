# R7 静态 Bootstrap 合同实验结果

- Created: 2026-07-19
- Updated: 2026-07-19
- Status: Complete / Phase D Ready
- Production Commit: `2b24886ae`
- Machine Result: `benchmarks/taskspace/r7/phase-c-static-bootstrap-contract-result.json`

## 1. 目标与边界

上一轮移除生命周期动态命名 `tool_choice` 后，TaskSpace 最终初始化和任务正确性为 `6/6`，但首请求
初始化为 `0/6`：Agent 总是先调用普通工具，收到 `no_task_path` 后才在第二次请求初始化 Map。

本轮不恢复动态 `tool_choice`，不隐藏普通工具，也不让 Runtime 创建或补写 Map。只强化三个静态合同：

1. `taskspace_control` 永久位于 TaskSpace 工具列表第一位；
2. 固定 tool schema 明确 `bootstrap_required=true` 时首个顶层调用必须是
   `taskspace_control.initialize_map`，普通动作放入初始化 continuation；
3. bootstrap projection 忠实暴露 `bootstrap_control_action`、`ordinary_tools_allowed=false` 和
   `ordinary_tool_failure=no_task_path` 三项状态机硬事实。

这些内容在所有生命周期请求中保持同一工具顺序和 schema。Runtime 没有增加任务建议、节点优先级、
下一步推断或失败后的自动动作。

## 2. 工程验证

| 合同 | 结果 |
|---|---|
| `tool_choice` | 所有 TaskSpace 请求均为 `auto` |
| tool surface | 13 tools，`taskspace_control` 永久第一 |
| tool hash | 64/64 payload 使用同一 hash |
| lifecycle shape transition | 0 |
| thinking | 6/6 首请求均产生 reasoning token |
| bootstrap projection | 三项机械硬状态字段稳定存在 |
| empty-map hard gate | 保留，未放松 |
| terminal hard contract | 保留，非法普通 final 仍不闭合任务 |

## 3. Docker 自然运行

模型为 `deepseek-v4-flash`，reasoning effort 为 `max`，projection policy 为 `map-append`。简单、复杂
样本各运行 3 对 Standard/TaskSpace 双臂，执行与验证均使用 Docker hard boundary。

12 个 side 均为 `complete` 且结果可比较。复杂样本第 1 次 TaskSpace 的通用 observer 对一条 control
参数报告解析告警，因此该行的 control 细分为 `partial_with_parse_errors`；首个 provider 调用、payload、
result、请求数、token 和缓存证据仍完整，不影响本轮 bootstrap 与成本结论。

| Sample | Repeat | TaskSpace result | 首个顶层调用 | Init 次数 | Requests | 首请求 reasoning | Request 2+ cache |
|---|---:|---|---|---:|---:|---:|---:|
| simple | 1 | solved | `initialize_map` | 1 | 11 | 52 | 92.04% |
| simple | 2 | solved | `initialize_map` | 1 | 10 | 53 | 91.56% |
| simple | 3 | solved | `initialize_map` | 1 | 10 | 63 | 91.46% |
| complex | 1 | solved | `initialize_map` | 1 | 12 | 117 | 91.30% |
| complex | 2 | solved | `initialize_map` | 1 | 13 | 99 | 91.97% |
| complex | 3 | solved | `initialize_map` | 1 | 8 | 48 | 89.18% |

结果：

- 首请求初始化 `6/6`，从上一轮 `0/6` 提升到 `6/6`；
- 初始化均一次提交成功，没有重复初始化；
- 初始化前普通工具调用为 0，`no_task_path` 工具失败为 0；
- correctness、公开验证、隐藏验证和 Map 闭合均为 `6/6`；
- 64 个 TaskSpace provider payload 全部为 `auto + 13 tools + 同一 tool hash`；
- message prefix preservation 为 100%，choice/shape transition 和 same-shape zero 均为 0；
- 后续工作中出现 5 次普通 control protocol failure，均与 bootstrap 无关，失败反馈保留后 Agent 自行纠正；
- 通用 observer 的上述单条解析告警保留为观测覆盖缺口，没有按 0 处理；
- observer 在复杂第 3 次报告的 multi-patch 是一次包含多个文件的单一 `apply_patch` 调用，不是同一
  request 连续执行多个 patch tool。

运行证据：

```text
target/r7-phase-c/static-bootstrap-contract/simple/single-file-fast-fix/20260719-030052-065
target/r7-phase-c/static-bootstrap-contract/complex/subscription-billing-repair/20260719-030418-675
```

## 4. 与未强化版本对照

下表只比较相同模型、Docker、`map-append` 和每样本 3 次的 TaskSpace 聚合。上一轮基线来自
`06-r7-tool-choice-ablation-result.md`。

| Sample | Version | First-request init | Requests | Wall | Input | Uncached | Output | Request 2+ cache |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| simple | auto without static contract | 0/3 | 44 | 114.52s | 726,242 | 51,426 | 10,300 | 92.74% |
| simple | static bootstrap contract | 3/3 | 31 | 82.12s | 426,148 | 40,868 | 7,731 | 91.70% |
| complex | auto without static contract | 0/3 | 48 | 167.70s | 967,666 | 59,890 | 20,228 | 93.72% |
| complex | static bootstrap contract | 3/3 | 33 | 158.09s | 643,989 | 61,205 | 20,180 | 91.16% |

静态合同同时减少了失败恢复轮次：简单 request 减少 29.5%，复杂减少 31.3%；总 input 分别减少
41.3% 和 33.5%。复杂样本未缓存 input 只增加 1,315 token（2.2%），但 request 2+ 命中率下降
2.56 个百分点，原因不是 cache shape 失效，而是接近相同的工具工作被压缩到更少请求中，每个新请求
前新增的尚未缓存工具反馈更大。工具 hash、message prefix、choice/shape transition 和 same-shape zero
证据均排除了生命周期缓存断裂。

## 5. 当前 Standard 对照

| Sample | Mode | Solved | Requests | Wall | Input | Uncached | Request 2+ cache |
|---|---|---:|---:|---:|---:|---:|---:|
| simple | Standard | 3/3 | 21 | 53.81s | 152,951 | 6,007 | 95.82% |
| simple | TaskSpace | 3/3 | 31 | 82.12s | 426,148 | 40,868 | 91.70% |
| complex | Standard | 3/3 | 37 | 143.67s | 412,229 | 18,629 | 95.32% |
| complex | TaskSpace | 3/3 | 33 | 158.09s | 643,989 | 61,205 | 91.16% |

简单任务仍体现 TaskSpace 固定机制成本。复杂任务中 TaskSpace request 已少于 Standard，但总 input 和
未缓存 input 仍高，继续属于 `map-append` 累积 projection 与 TaskSpace tool/control 上下文成本，不是
本轮 bootstrap 修复要掩盖的问题。

## 6. 结论

静态强化方案通过：它在不恢复动态 `tool_choice`、不关闭 thinking、不中断缓存形状的前提下，把首请求
Map 初始化从 `0/6` 提升到 `6/6`。这属于工具能力与状态机硬规则的清晰暴露，不是 Runtime 替 Agent
决策。

本轮样本支持继续 Phase D，不需要进入“全程命名 taskspace_control carrier”的更强方案。后者仍保留
为静态方案未来大样本不稳定时的备选，不在当前证据充分的情况下增加架构复杂度。

## 7. 测试

```text
just fmt                                                                 PASS
just fix -p codex-tools                                                  PASS (existing warning)
just fix -p codex-core                                                   PASS (existing warnings)
cargo test -p codex-tools --lib                                          141 passed / 1 ignored
cargo test -p codex-core provider_composer_injects_one_blank_map_projection --lib
                                                                         PASS
cargo test -p codex-core provider_payload_scan_validates_canonical_projection_shape --lib
                                                                         PASS
cargo test -p codex-core --test all taskspace_terminal_contract -- --test-threads=1
                                                                         2 passed
cargo build -p codex-cli --bin whale --locked                            PASS
Docker simple Standard / TaskSpace                                       3/3 / 3/3 solved
Docker complex Standard / TaskSpace                                      3/3 / 3/3 solved
```
