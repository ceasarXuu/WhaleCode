# R7 连续动作合同回归修复

- Created: 2026-07-21
- Updated: 2026-07-22
- Version: 3.1
- Status: `active_verified`
- Phase: FLA-3.5
- Scope: TaskSpace 非终态生命周期交接、真实动作 Tool schema、执行顺序与事实反馈
- Compatibility: 不保留 `required_next_call` 或旧 session 兼容路径

## 1. 实施决策

FLA-3.5 直接在现有 Tool 构建和执行链上恢复连续动作，不建立候选晋级体系。

此前文档中的 CA-0 至 CA-6、candidate namespace、immutable anchor、prepare/reserve/promotion、三臂晋级和
专用 rollback toolchain 已撤销，不再是 R7 的实现要求或完成门禁。原因是这些机制远超 H-003 的问题边界，
增加了新的架构层次，却没有提高连续动作本身的正确性。

本阶段的结构合同收敛为三个产品改动：

1. TaskSpace 普通动作 Tool 必须携带 `taskspace_action`，不再把字段缺失解释成“继续当前节点”。
2. `taskspace_action=continue_current` 显式声明本动作继续服务当前节点；Runtime 只机械核对 Map revision 和
   当前绑定，不改变 Map，也不判断任务语义。
3. 初始化、绑定或完成后继续仍在同一个 Tool call 内先提交 Agent 明确选择的生命周期动作，再把原始参数交给
   原有 Tool 执行链。

Standard 模式、普通 Tool handler、权限、sandbox、approval、hook、MCP 和业务结果格式均不建立平行实现。

## 2. 产品合同

连续动作是 Tool 结构合同，不是 Runtime 对 Agent 意图的推断：

| 状态动作 | 承载方式 | 可否作为独立 `taskspace_control` |
|---|---|---|
| `continue_current` | 仍服务当前节点的普通动作 `taskspace_action` | 否 |
| `initialize_map` | 第一个真实动作的 `taskspace_action` | 否 |
| `bind_node` | 该节点第一个真实动作的 `taskspace_action` | 否 |
| `complete_then_continue` | 后继节点第一个真实动作的 `taskspace_action` | 否 |
| `complete_then_end` | 终态 `taskspace_control` | 是 |
| `close_ready_finish` | 仅在 Finish 已 Ready 且没有 active Work 时使用的终态 `taskspace_control` | 是 |

`mutate_graph`、`block_node`、`unblock_node`、`rework_node`、`expand_nodes`、`read_map` 和
`read_output_ref` 仍是独立 Map 操作。Runtime 不根据命令、Patch、reasoning 或任务内容选择节点、补动作或改写交接。

## 3. 工程实现

字段必填解决的是“Agent 是否明确声明这次动作属于哪个状态”这一结构歧义。Agent 仍可明确选择
`continue_current`，即使这个选择在任务语义上并不理想；Runtime 不据此纠正 Agent。

### 3.1 Schema

- 共享 registry 始终构造原始 Tool schema；只在 TaskSpace provider 可见性投影边界应用共享 decorator，增加必填
  `taskspace_action` 判别联合。Standard 通过同一 registry 取得未经装饰的原始 schema。
- Function、动态 MCP Function、freeform `apply_patch` 和 code-mode `exec` 共用这一机械投影能力。
- `ToolSearch` 同样暴露该字段；未经过 decorator 的原生自由格式调用不会被要求提供一个不可见字段。
- `taskspace_control` 不暴露三个非终态 lifecycle action；即使模型绕过 schema 直接提交，也由参数合同拒绝。
- `required_next_call` 和跨 sibling 的 `TASKSPACE_REQUIRED_SIBLING_MISSING` preflight 已删除。
- Standard Tool schema 不变。

### 3.2 Runtime

一次 carrier 调用按以下顺序处理：

1. 从普通 Tool 参数中提取并删除保留字段 `taskspace_action`。
2. 字段缺失或参数无效时停止 dispatch，返回准确协议失败。
3. `continue_current` 只核对 `expected_revision` 与 `current_node_id`；通过后不提交状态事件。
4. 生命周期动作先校验并提交 Agent 明确给出的状态交接；失败时停止本次普通 Tool dispatch。
5. 校验或交接成功后，把剥离后的原始参数交给现有 router/handler。
6. 普通 Tool 的成功或失败不改写已提交的状态事实，也不由 Runtime 自动回滚。

生命周期 carrier 是 sequence barrier，避免同一响应中的依赖动作越过状态交接；`continue_current` 不是 barrier，
同节点内相互独立的普通动作仍可使用原有并行执行能力。

### 3.3 反馈

发生生命周期提交时，同一个 provider call id 返回一个短 `TaskSpaceCarrierResultV2` 头和未经改写的原 Tool 输出；
action 拒绝时普通 Tool 没有执行，只返回一份 carrier envelope：

```text
TaskSpaceCarrierResultV2.action_result = TaskSpaceControlResultV2 | TaskSpaceActionValidationResultV1
TaskSpaceCarrierResultV2.tool_dispatched = true | false
<original tool output follows unchanged when dispatched>
```

成功的 `continue_current` 不注入重复结果头，只保留原 Tool 输出；准确的校验事实写入日志。这样既保持反馈忠实，
又避免每个普通动作反复污染上下文。`tool_dispatched=false` 时不存在普通 Tool 事实，carrier envelope 会替换内部失败
占位 body，不能把同一个 action failure 再附加一遍。

新增事实日志：

- `taskspace.carrier_transition_committed`
- `taskspace.carrier_transition_rejected`
- `taskspace.carrier_continue_validated`
- `taskspace.carrier_action_rejected`

## 4. 边界

本阶段明确没有实现：

- 第二套 Tool router、handler 或权限链；
- Runtime 语义判断、自动选后继、自动重试或自动纠错；
- prepare/reservation/promotion/candidate 架构；
- provider 不稳定假设或针对模型行为的特殊分支；
- FLA-4 的完整 Tool 文案/schema 正式化；
- FLA-5 的全载体结果代数验收；
- FLA-7 的 resume/fork/compaction 生命周期等价；
- FLA-8 的正式多臂收益评估。

## 5. 验证结果

### 5.1 工程验证

| 验证 | 结果 |
|---|---|
| `cargo test -p codex-tools taskspace --no-fail-fast` | 9 passed |
| `cargo test -p codex-core taskspace --no-default-features --no-fail-fast` | 92 lib + 4 integration passed |
| `cargo test -p codex-core taskspace_terminal_contract --test all -- --nocapture` | 2 passed |
| FLA-3.5 contract gate | passed |
| `cargo build -p codex-cli --bin whale --locked` | passed |

定向测试覆盖 schema 暴露、参数剥离、standalone lifecycle 拒绝、transition 成功/失败、原 Tool 输出保留、
sequence barrier、terminal closure、compact/resume 回归、Standard schema 隔离和拒绝反馈单一表达。

### 5.2 Docker 三组配对样本

运行目录：

`target/r7-action-scope-fix/single-file-fast-fix/20260722-212237-506`

| 模式 | 结果 | Requests | Runtime tools | Input | Request 2+ cache | Wall |
|---|---|---:|---:|---:|---:|---:|
| Standard | 3/3 solved | 18 | 25 | 219,904 | 97.47% | 53.55s |
| TaskSpace map-request | 3/3 solved | 22 | 25 | 528,471 | 94.93% | 81.39s |

三次 TaskSpace trace 中：

- 初始化均由第一个真实 `exec_command` 携带；
- Patch 均携带 `complete_then_continue(explore -> fix)`，测试均携带
  `complete_then_continue(fix -> verify)`；
- 静默 Patch/Map 生命周期漂移为 0/3；
- 最终 Map 均为 5 nodes / 4 edges / 0 open leaves，公开与隐藏验证全部通过。

三次 Standard trace 使用同一原始 schema identity，普通 Tool 中 `taskspace_action` 和
`TASKSPACE_ACTION_*` 出现次数均为 0，证明 carrier 不再污染 Standard。TaskSpace Tool schema 当前每请求约
60,747 bytes，Standard 为 21,669 bytes；本阶段证明正确性修复，不宣称成本收益。

### 5.3 独立残留观测

三次 TaskSpace 中有两次在 `verify` 仍为 Running 时先选择当时名为 `finish_end` 的专用动作，被现有硬状态约束准确拒绝后改用
`complete_then_end`；另一次直接选择正确动作。此时 Patch、测试和 Map revision 已全程同步，因此该现象不再能
由 H-003 的静默漂移解释。它属于终态 Tool action 选择的独立问题，不在本次修复中通过 Runtime 猜测或自动改写。

## 6. 完成结论

H-003、H-004 和 H-005 的结构根因已经消除：每个 TaskSpace 普通动作都显式选择 continuation 或生命周期动作，
状态与真实动作由同一个 Tool schema 和 call id 承载；Standard 保持原始 schema；未 dispatch 的拒绝只反馈一次。
实现复用现有工具链，没有新增 router 或 handler 分叉，FLA-3.5 状态为 `active_verified`。

FLA-4 可以从当前 carrier 基线继续，但不得重新引入 sibling、中央 nested action carrier 或 Runtime 语义控制。

## 7. H-006 终态动作合同修复

FLA-3.5 验证暴露出一个独立的 L2/L4 合同歧义：最终 Work 仍为 Running 时，Agent 经常把原先的
`finish_end` 当成通用结束动作，而不是选择能够同时完成当前 Work 的 `complete_then_end`。工具结果完整且状态机拒绝
准确，问题不在反馈丢失，也不应由 Runtime 替 Agent 选择动作。

修复保持既有状态机不变，只收敛 Agent 可见合同：

- 将只适用于“Finish 已 Ready 且没有 active Work”的专用动作改名为 `close_ready_finish`；旧名称不保留兼容入口；
- `complete_then_end` 的 schema 明确其适用于最终 Work 仍为 Running 的状态，并要求 `current_node_id`；
- L2 `taskspace-core-v2.4` 给出同一状态判别规则，避免宏观协议与 Tool 合同相互脱节；
- Runtime 继续只校验 canonical state，不动态删减 schema、不自动代选动作、不根据任务语义改写调用；
- 终态 operation、回放识别和性能观测统一使用 `close_ready_finish`，使误选与合法专用闭合可直接审计。
