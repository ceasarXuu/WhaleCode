# R6 Phase E Finish 终结边界实施结果

## 1. 结论

```text
Finish / verify 分离：PASS
Phase E 整体：IN PROGRESS
Runtime candidate: fa505477dd32e69a57480ea7fb6d31bbd64bfdf0
```

Finish 已从工具合同到领域状态完整移除工作目标：初始化只接受 `finish: {node_id}`，验证只能归属
Work。Runtime 不识别 `verify`、`test` 等关键词，不规定 Agent 必须拆出独立验证节点，也不替 Agent
重排图。

机器结果见 `benchmarks/taskspace/r6/phase-e-finish-boundary-result.json`。

## 2. 工程结果

| 层 | 结果 |
|---|---|
| Tool schema | Finish strict object 只有必填 `node_id`，无 `goal` |
| Typed args | Finish 使用独立类型；旧 `finish.goal` 因 unknown field 明确拒绝 |
| Handler/runtime | 独立 Finish input 不再携带 goal |
| Rooted DAG | `MapNode::finish()` 固定空 goal；非空状态返回 `finish_goal_not_empty` |
| Snapshot/projection | 通用节点 wire 字段保留，但 Finish goal 固定为空，不被重新解释 |
| 日志 | 参数拒绝沿用 `taskspace.control_arguments_rejected`；领域拒绝保留稳定 violation code |
| 兼容 | 无 migration、adapter、silent drop 或旧字段 fallback |

该约束没有强迫一个连贯修复拆成多个 Work。Agent 仍可把相关多文件修改放在同一 Work 中；硬边界
只禁止唯一终点承载尚待执行的工作。

## 3. 确定性验证

| Gate | 结果 |
|---|---|
| `codex-tools` 完整 crate | 141 passed / 1 ignored |
| `taskspace_control` | 17/17 passed |
| `action_map::` | 67/67 passed |
| Rooted DAG | 26/26 passed |
| sequence/carrier | 13/13 passed |
| 关键 session、projection、replay | passed |
| Guardian（加载 `.env.local`） | 79/79 passed |
| scoped `just fix` + `just fmt` | completed |
| `cargo build -p codex-cli --bin whale --locked` | passed |

`codex-core` 全量测试第一次执行为 1847 passed / 12 failed / 3 ignored。8 个 Guardian 失败在加载
`.env.local` 后全部通过；一个 provider budget 用例使用了已删除的旧 projection marker，修正为
R6 marker 后通过。其余 3 个单线程仍可复现的失败是两个 file watcher 状态断言和一个 provider
refresh 调用计数，不经过本次 Finish/TaskSpace 变更路径，本主题不扩散修复。

## 4. Docker 单样本

样本：`subscription-billing-repair`，Standard/TaskSpace 同模型、同 Docker substrate，各 1 次。
两臂业务验证、public tests 和 hidden oracle 均通过；pair 有效，但因 repeats=1 只作为诊断证据，
不进入聚合收益判断。

| 指标 | Standard | TaskSpace | T/S |
|---|---:|---:|---:|
| Requests | 17 | 15 | 0.88x |
| Runtime tools | 25 | 23 | 0.92x |
| Wall | 58.90s | 63.59s | 1.08x |
| Input | 225,391 | 201,939 | 0.90x |
| Uncached input | 6,511 | 11,859 | 1.82x |
| Output | 6,371 | 7,357 | 1.15x |
| Request 2+ cache hit | 97.06% | 96.29% | -0.77pp |

TaskSpace 有 6 次 control，0 次 protocol/state/nested failure。单样本不证明性能收益；它只证明
本次 schema 切换没有阻碍业务完成。

## 5. Raw trace 验收

Agent 实际初始化输入为：

```json
{
  "finish": {"node_id": "finish"},
  "work": [
    ["explore_project", "Explore project structure, README rules, and failing tests"],
    ["fix_regressions", "Fix regression bugs and align tests with README rules"],
    ["verify", "Run tests and verify all fixes"]
  ]
}
```

因此本轮不是 Runtime 丢弃了 `finish.goal`，而是 Agent 按新 schema 从源头没有生成该字段，并把
验证自然放入 Work。随后 5 次 transition 均提交成功，revision 从 2 推进到 7；最终 control state
为所有 Work completed、无 current/pending/ready/running Work、Finish ready。

## 6. 新暴露问题

### R6-E-TERM-01：缺少显式终结

Agent 在 revision 7 后输出“全部测试通过”的最终回答，没有调用 `finish_end`。因此业务任务成功，
但 canonical Map 的 Root 仍 OPEN、Finish 仍 READY、`complete=false`。这说明 Finish 工作语义已经
收干净，但 Phase E 的“唯一显式终结”尚未完成。

后续应继续从 tool/prompt/runtime 的有机合同检查为什么 final answer 可以绕过 `finish_end`；不得让
Runtime 自动终结，也不得通过解析最终回答语义来补闭合。

### R6-E-OBS-01：observer 状态重放错误

performance observer 报告仍显示 `explore_project=running`、后续 Work pending；raw control outputs
和 `graph_revision_committed` 明确已到 revision 7。当前 artifact 有 2 个 checkpoint 和 72 个 delta，
observer 没有反映后续 delta。报告的成本、请求和 control 数可用，但其最终 Map 状态不可作为本轮
权威；修复前必须以 raw rollout 的 committed revision 为准。

## 7. 阶段判断

本主题退出门禁通过，可以作为 Phase E 的终结边界基线。Phase E 不能标记完成，下一步应先解决：

1. Agent 最终回答绕过 `finish_end` 的显式终结缺口；
2. benchmark observer 对 snapshot delta 的最终状态重放缺口；
3. 再进入 resume/fork/crash 原子性矩阵。

两项缺口的系统修复设计已写入 `10-r6-terminal-replay-convergence-design.md`。方案不新增 terminal tool 或
平行 Map 状态：Finish READY 复用现有 named `taskspace_control` hard-state selection；observer 通过
canonical Rust replay 获取最终 snapshot。当前失败没有 Hook 根因证据，本专项不修改通用 Hook，也不让
`taskspace_control` 新接入 Hook。
