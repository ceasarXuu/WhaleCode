# Phase R3-D. Graph Closeout and Lifecycle Convergence

## D.1 目标

解决 build-R2 Phase H 中的 graph blocker：

```text
open_leaf_nodes = 1
```

上下文模块可以让 Agent 更清楚当前节点和下一步动作，但不能单独保证 graph 收口。
R3-D 要修的是 TaskSpace lifecycle policy。

## D.2 当前问题

B-tier smoke 已经业务成功：

```text
TaskSpace public_validation_exit_code = 0
TaskSpace hidden_oracle_exit_code = 0
business_success = true
```

但 graph 仍留下 open leaf，说明 runtime 没有把成功验证、criteria satisfied、node finish、
final closeout 串成稳定闭环。

## D.3 设计方向

| Area | Required Behavior |
|---|---|
| Success evidence adoption | 成功 test/validator 必须能被 state_commit 或等价流程采纳 |
| Validation node closeout | smoke/regression node 有成功 validator 后应可 finish |
| Criteria satisfaction | success criteria 与 validator evidence 建立引用 |
| Main path leaf close | 用户可回答前，主路径 leaf 必须 completed 或 explicitly blocked |
| Final answer gate | 不因为隐藏 TaskSpace 术语要求创建无意义 final_synthesis |

## D.4 Lifecycle policy

推荐 closeout 条件：

```text
If current node kind is smoke_test or regression_test
and latest validator result exit_code == 0
and hidden oracle exit_code == 0 when present
then runtime should guide or require:
  state_commit(result_validities, success_criteria)
  finish_node(outcome=success)
  no new leaf unless unresolved work remains
```

对于小任务：

```text
Do not require final_synthesis only for summary.
After accepted validation evidence and no blockers, user answer may close the session.
```

## D.5 实施任务

| Task | Production Code Path | Expected Behavior |
|---|---|---|
| graph health reason fields | runtime graph health | open leaf has reason and owning node |
| validation closeout hint | runtime gate/recovery | successful validator suggests state_commit + finish |
| closeout enforcement | release decision / runtime | release-like claim blocked if open leaf remains |
| final answer path | session/turn and runtime | answer allowed only after leaf close or explicit blocker |
| tests for small task closeout | action_map/runtime tests | single-file fix closes without final_synthesis |

## D.6 完成证据矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Graph health reason | open leaves explainable | runtime graph health | benchmark artifact | runtime tests | graph-health.json | none | planned |
| Validation closeout | successful tests close leaf | runtime/taskspace_control | validator result | lifecycle tests | map snapshot | none | planned |
| Release gate | open leaf blocks release | release decision | release closeout | release fixture | release-decision.json | none | planned |

## D.7 测试和收益验证

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | successful validation closes node | unit test | node status completed |
| Correctness | unresolved blocker leaves explicit blocked node | unit test | open leaf reason recorded |
| Correctness | final answer not forced into final_synthesis | small-task fixture | no unnecessary final_synthesis |
| Benefit | B-tier graph hygiene | B-tier smoke | open_leaf_nodes=0 |
| Observability | graph health reason | artifact inspection | every open leaf has reason |

## D.8 Risks and fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| over-eager closeout | task closes before real validation | hidden oracle failure | require validator evidence refs | block final answer |
| forced state_commit loop | walltime increases | repeated recovery events | combine state_commit and finish guidance | allow explicit blocked closeout |
| final_synthesis regression | small task creates extra node | node count grows after validation | small-task tests | direct answer gate |

## D.9 Exit criteria

```text
B-tier graph health has open_leaf_nodes=0.
Release fixtures still block open_leaf_nodes>0.
Small single-file task does not create summary-only final_synthesis.
Graph health artifact explains any diagnostic open node before formal E3.
```

## D.10 当前实现状态

已落地第一处 lifecycle 收口修复：当当前 `implement_solution` / `smoke_test` / `regression_test` 节点已经有成功的必需动作证据时，`final_answer` 不再阻断 runtime 先生成 `finish_node`。这修复的是“业务已经成功，但终端回答绕过节点关闭，导致 `open_leaf_nodes=1`”这一类失败路径。

已验证：

```text
cargo test -p codex-core active_context_replacement --lib
82 passed

cargo test -p codex-core taskspace --lib
92 passed
```

新增回归测试：

```text
taskspace_final_answer_does_not_block_successful_required_action_auto_finish
```

尚未完成的真实收益证明：需要重新跑 B-tier / targeted diagnostic，确认真实运行 artifact 中 `open_leaf_nodes=0`，并检查没有引入多余 `final_synthesis` 节点。

## D.11 2026-06-27 closeout blocker 修复

B-tier smoke `target\phase-r3-btier-smoke-20260627-003813` 暴露出第二类 graph closeout blocker：

```text
business_success = true
open_leaf_nodes = 1
node-3 kind = smoke_test
node-3 status = running
```

根因不是模型没有发出 closeout，也不是 `taskspace_control` handler 拒绝了 `finish_node`。实际链路是：

```text
assistant text emitted valid taskspace_control(action=finish_node)
session/turn.rs detected successful validation on smoke_test
runtime rewrote that explicit finish_node into final_answer
taskspace_control tool call was never synthesized
node-3 stayed running
```

修复原则：

```text
显式 lifecycle action 必须先落 runtime state，不能被 final_answer 替换。
final_answer 只能用于真正的 terminal response，不能绕过 finish_node/block_node。
```

已落地：

```text
removed should_answer_after_successful_validation_finish_node rewrite branch
added taskspace_action_contract_finish_node_on_validation_node_remains_lifecycle_tool
```

已验证：

```text
cargo test -p codex-core active_context_replacement --lib
83 passed

cargo test -p codex-core taskspace --lib
93 passed
```

仍需真实收益证明：

```text
重建 whale.exe
重跑 B-tier single-file-fast-fix
确认 open_leaf_nodes=0
确认没有新增 summary-only final_synthesis
```

## D.12 2026-06-27 closed-graph final answer 修复

第二轮 B-tier `target\phase-r3-btier-smoke-20260627-012652` 证明 D.11 的直接收益已经出现：

```text
open_leaf_nodes = 0
model_request_duration_ms = 503738
provider lifecycle timing source = rollout.jsonl
public_validation_exit_code = 0
hidden_oracle_exit_code = 0
```

但该 run 仍然失败：

```text
TaskSpace exec_exit_code = 1
business_success = false
```

根因是 graph 已经全部关闭后，runtime 进入 `node_id=null / node_kind=unknown`，action-contract prompt 没有把“已有 task 但无 active node”定义为 final-answer 状态，模型开始重复 `create_node`，最后发出 `list_files node_id=null` 并被 policy 拒绝。

修复：

```text
TaskSpaceActionContractStateV1:
  existing task + no active bound node => if work is complete, return final_answer

Runtime guard:
  no active node + accepted successful validation result + non-terminal work action
  => synthesize final_answer("Validation passed; final result is ready.")
```

已验证：

```text
cargo test -p codex-core active_context_replacement --lib
84 passed

cargo test -p codex-core taskspace --lib
94 passed
```

仍需真实收益证明：重跑 B-tier，要求 `business_success=true`、`exec_exit_code=0`、`open_leaf_nodes=0` 同时成立。
