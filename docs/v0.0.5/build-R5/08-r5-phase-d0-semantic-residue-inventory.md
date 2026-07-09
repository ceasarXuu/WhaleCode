# R5 Phase D0 Provider-visible 语义残留清点与首轮清理

Version: v0.0.5 build-R5
Status: D0 landed
Date: 2026-07-09
Owner: Codex

## 1. 目标

D0 的目标不是继续增强 runtime 约束，而是先把 provider-visible 链路里残留的语义控制层清出来：

1. projection/recovery/action-contract 只保留忠实状态、工具反馈和硬底线事实。
2. 不再输出 allowed-actions、next-valid-actions、action-space-source、runtime-selected strategy。
3. 遇到 Agent 低级失败时，优先怀疑上下文传递、失败反馈和裁剪，而不是让 runtime 继续加语义干预。

## 2. 本轮清理范围

| Area | Before | After |
|---|---|---|
| bootstrap / no active task / no node | `state_machine_allowed_actions: start_task/route/create_node/...` | `hard_state` + ordinary tool 机械边界 |
| validation recovery | `validation_needs_test`、`validation_command_source`、`action_space_source` | `hard_state` + `tool_feedback_facts` + raw output |
| duplicate read / patch-only recovery | 说明 action space 来源或暗示实现策略 | 只保留 previous result、target artifact、repair contract copy、raw feedback |
| `TaskSpaceGateRecoveryV1` | JSON 内含 `next_valid_actions` | 新生成 payload 只含 `allowed/reason/blocking_items/missing_evidence` |
| forced inspect transition | gate recovery 可触发 runtime 自动 finish inspect 并创建 implementation | 测试改为确认不会自动推进 Agent-owned transition |
| inspect node edit gate | runtime 语义拒绝 inspect 下 edit 并提示 implement node | 测试改为确认只要符合硬约束，runtime 不因节点语义纠正 Agent |
| active projection omission audit | 暴露 `projection.next_valid_actions` 旧标签 | 删除该旧标签 |

## 3. 分类结果

| Residue | Classification | D0 Action |
|---|---|---|
| `state_machine_allowed_actions` in provider text | semantic residue | 删除或改为 `hard_state` |
| `rejected_by_state_baseline` + tool list | semantic residue | 保持测试反向约束，生产路径不再输出 |
| `validation_needs_test` | semantic residue | marker 改为 validation-node feedback，不再作为策略状态输出 |
| `next_valid_actions` in gate payload | semantic residue | 新 payload 删除；旧 parser/sync 仅保留为 legacy/internal 读取 |
| `action_space_source` / `Action-space source` | semantic residue | 删除，改成 ordinary tool boundary 或 raw facts |
| `validation_command_source` | semantic residue | 删除，改成 derived command hard-state fact |
| `projection.next_valid_actions` omission label | provider-visible old semantic label | 删除 |
| `problem_ledger/cognitive_state` storage and omission labels | D2 scope | 暂不拆结构；D2 从 active projection/gate 移出 |
| `projection_next_valid_actions` test helper | debug/test-only residue | D0 不作为生产输出；R5-F 清死代码时处理 |

## 4. 保留的硬底线

允许保留并可 model-visible：

- no active task/path/map/node/lease
- invalid node state or lifecycle target mismatch
- protocol/schema parse failure
- permission/sandbox/security/resource limit
- output ref/crop explanation
- ordinary tool attribution and lease boundary

不允许输出：

- 告诉 Agent 下一步 read/search/edit/test/final
- 用 validation/coverage/fact-source 推断修复策略
- runtime 自动创建/关闭语义节点来替 Agent 推进
- 把局部任务文本提升为 canonical truth

## 5. 验证

代码验证：

```text
cargo fmt --all
cargo test -p codex-core taskspace_action_contract -- --nocapture
cargo test -p codex-core gate_recovery -- --nocapture
cargo test -p codex-core active_projection -- --nocapture
cargo test -p codex-core validation_needs_test_recovery_blocks_discovery_loop -- --nocapture
cargo test -p codex-core node_contract_allows_edit_inside_inspect_node_after_boundary_reduction -- --nocapture
cargo test -p codex-core gate_recovery_message_omits_next_valid_actions_from_visible_payload -- --nocapture
cargo check -p codex-core
cargo build -p codex-cli --bin whale
```

结果：

```text
所有上述测试/构建通过。
rustfmt 仍输出既有 nightly-only imports_granularity 配置警告，不影响格式化完成。
```

样本验证：

```text
scenario: count-call-stack
run: target/r5d0-semantic-residue-clean/count-call-stack/20260709-232508-447
pair report: target/r5d0-semantic-residue-clean/count-call-stack/20260709-232508-447/pair-001/pair-report.md
standard: solved
taskspace: solved
failure_taxonomy: none
engineering_unclean: False
standard model_request_count: 1
taskspace model_request_count: 1
standard tool_call_count: 14
taskspace tool_call_count: 12
taskspace_tool_call_ratio: 0.86
taskspace_wall_time_ratio: 0.5
forbidden scan: no matches
```

本机未找到同场景 `count-call-stack` 的 R4 历史 pair report，因此本轮只记录 standard/R5 当前对照；不把它伪装成完整 standard/R4/R5 三向结论。

## 6. 仍需后续处理

1. R5-D1：`start_task initial_*` 不再自动提升为 canonical truth。
2. R5-D2：`problem_ledger/cognitive_state` 从 active projection/gate 移出，保留为 Agent-authored note/event。
3. R5-E：剩余 gate 建立 hard-baseline classifier，所有保留拒绝都要可归类为状态机、协议、权限、安全或资源底线。
4. R5-F：删除 legacy/test-only `next_valid_actions` helper/sync 字段或移入纯调试结构。
5. 样本 telemetry 中 `ordinary_before_binding=True` 需要后续核对口径：当前任务已通过，但该字段可能把机械空 map 初始化期算作 ordinary-before-binding。

## 7. 结论

D0 已完成首轮 provider-visible 语义残留清理：明显越界的动作菜单、下一步建议、action-space 来源说明和 forced transition 测试都已移除或反向约束。Phase D 可以继续进入 D1/D2，重点拆 `initial_*`、ledger、cognitive state 的 active canonical truth。
