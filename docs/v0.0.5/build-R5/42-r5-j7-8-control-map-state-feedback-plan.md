# R5-J7.8 Control Map 状态反馈收敛计划

- Date: 2026-07-13
- Status: COMPLETE
- Scope: H-027；J7.5 第二次复验前置门
- Related: `38-r5-j7-phase5-docker-benefit-result.md`、`41-r5-j7-7-terminal-finish-chain-plan.md`

## 1. 问题定义

J7.7 billing R5 的业务修改和外部验证均正确，但首次 terminal 提交留下了 4 次可避免的状态拒绝：

1. Agent 已实际执行初始测试，却从 `inspect_tests` 直接切到 `fix_implementation`，漏闭合
   `run_tests_initial`；最后一次成功 finish 回执只表达局部 transition，未表达 Map 中仍有哪些 open node。
2. 原子 terminal chain 因 open node 被拒绝后，回执只有 `state_machine_failed` 和错误正文；Agent 随后错误认为
   `verify_tests` 已部分提交，造成 lease/current/bind 三次恢复错误。

原始 init、普通工具结果、success/failure control output 均已进入 provider 上下文，错误正文也未丢失或扭曲。
缺口不是 Runtime 应替 Agent 排序，而是状态工具的结果没有完整表达本次提交范围和提交后的最小 Map 状态。

## 2. 设计边界

只扩展 `taskspace_control` 的机械结果：

```json
{
  "schema_version": "TaskSpaceControlResultV2",
  "status": "committed",
  "success": true,
  "state_commit": "full",
  "map_state": {
    "task_id": "task-1",
    "task_status": "active",
    "map_id": "map-1",
    "map_status": "active",
    "current_node_id": "verify_tests",
    "pending_node_ids": [],
    "open_node_ids": ["run_tests_initial", "verify_tests"],
    "blocked_node_ids": [],
    "completed_node_count": 4,
    "total_node_count": 6
  },
  "steps": []
}
```

约束：

1. `map_state` 直接读取 canonical Action Map，不推断任务语义，不生成 next action、建议或优先级。
2. ID 排序确定；pending、open（`ready/running`）、blocked 分列；不复制 goal、工具正文或历史 result。
3. terminal chain 失败必须返回 `state_commit=none` 和失败后的真实 current/open 状态，明确全链零提交。
4. 普通 `finish_nodes` 保留现有逐步提交语义；若前序成功、后序失败，返回 `state_commit=partial`，不得谎报原子。
5. 成功初始化、非终态 finish、create/bind/block 返回 `state_commit=full`；纯读取不附加状态快照。
6. 不增加 Runtime 自动 finish、bind、create、dedupe、排序或语义恢复。

## 3. 实施阶段

### J7.8-A：Canonical Control State

- 在 Action Map runtime 中构造只读 `ActionMapControlState`。
- Session 暴露按 active/hinted map 读取的内部方法。
- 单测覆盖 active、terminal completed、open/blocked 排序。

### J7.8-B：V2 Result Contract

- `format_state_batch` 增加 `state_commit` 和 `map_state`。
- 所有 mutation action 使用同一 JSON 结果，不再保留 create/bind/block 的弱纯文本成功回执。
- terminal 失败读取未变 Map，返回 `none`；nonterminal batch 按成功 step 数返回 `none/partial/full`。

### J7.8-C：日志与观测

- 记录 `taskspace.control_map_state_exposed`，仅含计数、commit class 和 current 是否存在，不记录正文。
- performance observer 增加 `map_state_missing`、`terminal_failure_nonzero_commit`、`open_node_visibility`。

### J7.8-D：验证

- focused parser/handler/runtime/session tests、Action Map scenario、observer selftests、locked build。
- Docker 重跑 order、billing Standard/R5 pair。
- 逐 request 检查最后一次成功 transition 后 open nodes 可见，terminal 原子失败不会被误读为部分提交。

## 4. 退出门禁

```text
所有 mutation success/failure 的 map_state coverage = 100%。
terminal failure state_commit = none，且 before/after Map hash 相同。
nonterminal partial failure 必须如实标记 partial。
两组 R5 protocol/state failure = 0。
success identity missing = 0；committed repeat finish = 0。
R5 Map open = 0；task/map completed；外部 validator 通过。
无 Runtime semantic decision、工具反馈正文丢失或 cache prefix 回退。
```

若 live Agent 在完整 `map_state` 下仍做错排序，记录为 Agent 能力波动；不得继续扩大 Runtime 状态规则来追求
样本通过。

## 5. 实施结果

J7.8 A-D 已完成：canonical `ActionMapControlState`、`state_commit=full|partial|none`、mutation V2 output、
`taskspace.control_map_state_exposed` 日志及 observer 指标全部落地。Runtime 只读取并回传 canonical Map 机械事实，
未增加自动 finish/bind/create/dedupe/order/recovery。

工程验证通过：`codex-core taskspace_control` 23项、terminal chain 4项、Action Map scenario 9项、event store/
redaction 2项、`codex-tools taskspace` 4项，以及 observer、cost instrumentation、metrics harness、skill validation、
fmt、locked `whale` build 和 binary attestation。

最终 Docker 复验中，order/billing R5 的 `map_state present/missing` 分别为 `2/0`、`6/0`，open-node visibility
分别为1、5；protocol/state failure、identity missing、committed repeat finish、terminal bad commit 和 Map open 均为0，
外部 validator 通过。J7.8 退出门禁和 J7.5 14/14 门禁均关闭。完整证据见
`38-r5-j7-phase5-docker-benefit-result.md` 第9节。
