# R6 Phase E4 Terminal Hard-State 实施结果

- Created: 2026-07-16
- Updated: 2026-07-16
- Status: Completed
- Scope: Phase E4 only
- Related Design: `10-r6-terminal-replay-convergence-design.md`
- Prerequisite: `12-r6-phase-e3-observer-convergence-result.md`

## 1. 结论

Phase E4 已通过独立退出门禁，可以进入 E5。provider action surface 现在由 canonical
`ActionMapControlState` 机械派生为三种 control mode：空 Map 必须初始化，工作态保持 Agent 自由，终局前沿必须
调用现有 `taskspace_control`。

Runtime 只约束当前状态下可调用的工具，不替 Agent 选择 `finish_end`、返工、改图或读取 Map。未新增 terminal
flag、terminal tool、恢复请求、projection 提示或 Hook 行为。

## 2. 状态与能力面

| Canonical 状态 | Provider tool choice | 可见工具 | Agent 决策权 |
|---|---|---|---|
| Map 尚未初始化 | named `taskspace_control` | bootstrap control only | Agent 声明完整初始图 |
| Work active | `auto` | ordinary tools + active control | Agent 自主工作、改图和状态迁移 |
| Finish READY 且无 Work frontier | named `taskspace_control` | active control only | Agent 自主 finish、rework、扩图或读取证据 |

终局 mode 只依赖以下机械事实：Map 未 complete、Finish READY、没有 current node，且
pending/ready/running/blocked Work 集合均为空。该状态由 Map 即时派生，不持久化第二份 lifecycle 状态。

## 3. 语义边界

1. named tool choice 只要求 Agent 进入 `taskspace_control`，不强制具体 variant。
2. active control schema 继续暴露 `finish_end`、`transition_node`、`mutate_graph`、`expand_nodes` 和
   `read_output_ref`；terminal frontier 没有专用 schema 分叉。
3. ordinary tools 在无 Work lease 的终局前沿不可执行，因此从该请求隐藏；这属于 hard capability
   projection，不是 Runtime 的语义判断。
4. 现有 tool schema 已声明 `finish_end` 原样释放 Agent 的 `final_summary`。本阶段没有再向基础 prompt 或
   projection 复制同一协议，避免三重暴露和上下文污染。
5. Hook 未修改，也不参与 terminal hard-state selection。

## 4. 日志建设

每个 TaskSpace provider request 新增 `taskspace_provider_control_mode_selected` 结构化事件，记录：

- `control_mode`：`bootstrap_required | work_active | terminal_control_required`；
- canonical `map_id`；
- canonical `revision`。

日志只说明 Runtime 选择了哪种机械能力面，不记录行动建议或完成度判断。

## 5. 回归结果

| 验证 | 结果 | 覆盖重点 |
|---|---:|---|
| `cargo test -p codex-core taskspace_control_modes_align_named_choice_visibility_and_schema --lib` | 1 passed | 三种 mode、tool choice、schema visibility |
| `cargo test -p codex-core control_state_exposes_only_work_nodes_as_the_active_frontier --lib` | 1 passed | terminal predicate 的正反边界 |
| `cargo test -p codex-core session::turn::active_context_replacement_tests --lib` | 12 passed | provider prompt/tool surface 回归 |
| `cargo test -p codex-core action_map::runtime::phase_d_tests --lib` | 7 passed | canonical frontier 回归 |
| `cargo test -p codex-tools taskspace --lib` | 4 passed | bootstrap/active control schema 回归 |
| `just fix -p codex-core` | passed | Clippy；仅有既有测试警告 |
| `just fmt` | passed | Rust 格式化 |

未执行全 workspace test；本阶段按改动边界执行 targeted regression，完整 workspace 测试依项目规则需要用户
单独授权。

## 6. 阶段判定

| Gate | 判定 |
|---|---|
| Finish READY 请求使用 named `taskspace_control` | PASS |
| terminal frontier 不暴露 ordinary tools | PASS |
| Agent 仍可选择 finish、rework、改图和读取 | PASS |
| control mode 只从 canonical Map 派生 | PASS |
| 无新增语义提示、持久状态或 Hook 分叉 | PASS |

E5 将处理响应发布和 TurnComplete 边界：只有已经提交的 `finish_end` carrier 可以发布 TaskSpace final；provider
违反 named tool contract 时明确结束为协议错误，不自动闭合 Map，也不产生 recovery request。
