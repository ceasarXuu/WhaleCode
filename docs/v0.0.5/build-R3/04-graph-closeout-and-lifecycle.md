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
