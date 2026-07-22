# R7 连续动作合同回归修复

- Created: 2026-07-21
- Updated: 2026-07-22
- Version: 2.0
- Status: `active_verified`
- Phase: FLA-3.5
- Scope: TaskSpace 非终态生命周期交接、真实动作 Tool schema、执行顺序与事实反馈
- Compatibility: 不保留 `required_next_call` 或旧 session 兼容路径

## 1. 实施决策

FLA-3.5 直接在现有 Tool 构建和执行链上恢复连续动作，不建立候选晋级体系。

此前文档中的 CA-0 至 CA-6、candidate namespace、immutable anchor、prepare/reserve/promotion、三臂晋级和
专用 rollback toolchain 已撤销，不再是 R7 的实现要求或完成门禁。原因是这些机制远超 H-003 的问题边界，
增加了新的架构层次，却没有提高连续动作本身的正确性。

本阶段只有两个产品改动：

1. 普通动作 Tool 在 TaskSpace 模式下可携带轻量 `taskspace_transition`。
2. Runtime 在同一个 Tool call 内先执行 Agent 明确提交的状态交接，再把原始参数交给原有 Tool 执行链。

Standard 模式、普通 Tool handler、权限、sandbox、approval、hook、MCP 和业务结果格式均不建立平行实现。

## 2. 产品合同

连续动作是 Tool 结构合同，不是 Runtime 对 Agent 意图的推断：

| 状态动作 | 承载方式 | 可否作为独立 `taskspace_control` |
|---|---|---|
| `initialize_map` | 第一个真实动作的 `taskspace_transition` | 否 |
| `bind_node` | 该节点第一个真实动作的 `taskspace_transition` | 否 |
| `complete_then_continue` | 后继节点第一个真实动作的 `taskspace_transition` | 否 |
| `complete_then_end` | 终态 `taskspace_control` | 是 |
| `finish_end` | 终态 `taskspace_control` | 是 |

`mutate_graph`、`block_node`、`unblock_node`、`rework_node`、`expand_nodes`、`read_map` 和
`read_output_ref` 仍是独立 Map 操作。Runtime 不根据命令、Patch、reasoning 或任务内容选择节点、补动作或改写交接。

## 3. 工程实现

### 3.1 Schema

- 共享 decorator 只在 TaskSpace 普通动作 Tool 上增加可选 `taskspace_transition`。
- Function、动态 MCP Function、freeform `apply_patch` 和 code-mode `exec` 共用这一机械投影能力。
- `taskspace_control` 不暴露三个非终态 lifecycle action；即使模型绕过 schema 直接提交，也由参数合同拒绝。
- `required_next_call` 和跨 sibling 的 `TASKSPACE_REQUIRED_SIBLING_MISSING` preflight 已删除。
- Standard Tool schema 不变。

### 3.2 Runtime

一次 carrier 调用按以下顺序处理：

1. 从普通 Tool 参数中提取并删除保留字段 `taskspace_transition`。
2. 校验并提交 Agent 明确给出的状态交接。
3. 状态交接失败时停止本次普通 Tool dispatch，返回准确失败事实。
4. 状态交接成功时，把剥离后的原始参数交给现有 router/handler。
5. 普通 Tool 的成功或失败不改写已提交的状态事实，也不由 Runtime 自动回滚。

carrier 是 sequence barrier，避免同一响应中的依赖动作越过状态交接；独立普通动作仍可使用原有并行执行能力。

### 3.3 反馈

同一个 provider call id 返回一个短 `TaskSpaceCarrierResultV1` 头和未经改写的原 Tool 输出：

```text
TaskSpaceCarrierResultV1.transition = TaskSpaceControlResultV2
TaskSpaceCarrierResultV1.tool_dispatched = true | false
<original tool output follows unchanged when dispatched>
```

因此 Agent 能分别判断“状态是否提交”和“真实工具是否执行/成功”，不会把两个事实压成一个整体成功或失败。

新增事实日志：

- `taskspace.carrier_transition_committed`
- `taskspace.carrier_transition_rejected`

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
| `cargo test -p codex-tools taskspace --no-fail-fast` | 8 passed |
| `cargo test -p codex-core taskspace --no-default-features --no-fail-fast` | 87 lib + 4 integration passed |
| `cargo test -p codex-core taskspace_terminal_contract --test all -- --nocapture` | 2 passed |
| FLA-3.5 contract gate | passed |
| `cargo build -p codex-cli --bin whale --locked` | passed |

定向测试覆盖 schema 暴露、参数剥离、standalone lifecycle 拒绝、transition 成功/失败、原 Tool 输出保留、
sequence barrier、terminal closure、compact/resume 回归和 Standard 不变。

### 5.2 Docker 单样本

运行目录：

`target/r7-fla35-minimal/single-file-fast-fix/20260722-174521-239`

| 模式 | 结果 | Requests | Runtime tools | Input | Request 2+ cache | Wall |
|---|---|---:|---:|---:|---:|---:|
| Standard | solved | 7 | 9 | 142,734 | 98.26% | 14.56s |
| TaskSpace map-request | solved | 10 | 10 | 225,412 | 91.86% | 27.20s |

TaskSpace trace 中：

- `initialize_map` 与 `ls` 使用同一个 `exec_command` call；
- 两次 `complete_then_continue` 均与后续 pytest 使用同一个 `exec_command` call；
- 每次反馈同时包含已提交 transition 和原始 pytest 输出；
- 未出现 `required_next_call`、missing-sibling rejection 或 lifecycle sibling call；
- 最终 Map 为 5 nodes / 4 edges / 0 open leaves，公开与隐藏验证均通过。

该单样本只验证生产接线和 H-003 路径，不用于证明总体性能收益。额外请求来自首次动作漏带初始化，以及 Agent
过早尝试终态闭合后主动 `read_map`；二者不是旧 sibling 合同回归，也不在 FLA-3.5 内扩展修复。

## 6. 完成结论

H-003 的结构根因已经消除：非终态生命周期交接不再依赖另一个顶层 sibling call，状态与真实动作由同一个
Tool schema 和 call id 承载。实现复用现有工具链，没有新增架构分叉，FLA-3.5 状态为 `active_verified`。

FLA-4 可以从当前 carrier 基线继续，但不得重新引入 sibling、中央 nested action carrier 或 Runtime 语义控制。
