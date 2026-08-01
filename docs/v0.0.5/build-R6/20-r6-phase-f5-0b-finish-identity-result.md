# R6 Phase F5.0b Finish Identity Wire Shape 实验结果

- 日期：2026-07-17
- 状态：Complete / H-011 Refuted / H-012 Confirmed / F5.0c Pending
- 范围：仅 Finish identity provider 合同因果实验
- 代码提交：`3e8c87968`、`63470f124`
- 计划：`18-r6-phase-f5-cost-regression-repair-plan.md`
- COE：`coe/2026-07-16-18-52-r6-phase-f-context-cost.md`

## 1. 结论

F5.0b 已完成。错误不是由 Finish 使用对象类型本身造成，而是由当前 `finish: { node_id }` 与普通图节点共享
命名束造成。保持对象语义、只改为 `finish_identity: { id }` 后，DeepSeek 6/6 生成合法参数；标量
`finish_identity: string` 为 5/6，另一次错误地生成空对象。

因此 F5.0c 冻结 E 臂，不采用 F 臂：E 对现有领域模型和 parser 的改动更小，正确性更高，schema 成本与 D
基本相同。Runtime 不需要增加纠错、字段修复或语义提示。

## 2. 实验合同

三臂都从生产 builder 的同一 bootstrap-only schema 机械派生，tool description、`$defs`、ordinary tools、prompt、
模型、`temperature=0`、named tool choice 和 `thinking=disabled` 完全相同：

| Arm | Finish identity | 变化目的 | Schema bytes |
|---|---|---|---:|
| D | `finish: { node_id }` | 当前生产基线 | 4,406 |
| E | `finish_identity: { id }` | 改变 identity 命名束，保留对象 | 4,414 |
| F | `finish_identity: string` | 在 E 基础上只改变对象/标量类型 | 4,247 |

simple/complex 每臂各 3 次，共 18 次；轮换顺序为 D/E/F、E/F/D、F/D/E。结果文件直接记录实际 required、
properties、identity type 和 schema hash，不能只依赖人工 arm 标签。

原始证据：

- `target/r6-f5-finish-identity-ab/20260717-live-01/provider-capability.json`
- `target/r6-f5-finish-identity-ab/20260717-live-01/probe-events.jsonl`
- `target/r6-f5-finish-identity-ab/20260717-live-01/analysis.json`

## 3. 正式结果

| Arm | HTTP/parsed | Valid | Identity errors | Common errors | Input total/mean/median | Uncached total/mean/median | Duration ms total/mean/median | Request 2+ cache |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| D | 6/6 | 1/6 | 5/6 | 0/6 | 8,622 / 1,437 / 1,437 | 174 / 29 / 29 | 16,005 / 2,667.5 / 2,576 | 97.98% |
| E | 6/6 | 6/6 | 0/6 | 0/6 | 8,634 / 1,439 / 1,439 | 1,722 / 287 / 41 | 17,577 / 2,929.5 / 2,637 | 97.85% |
| F | 6/6 | 5/6 | 1/6 | 0/6 | 8,388 / 1,398 / 1,398 | 964 / 160.67 / 128 | 15,395 / 2,565.83 / 2,556 | 91.56% |

D 的五次错误均为 `unexpected:finish.goal`。E 六次均严格为 `finish_identity: { id }`。F 的一次错误为
`type:finish_identity:not_string`，模型生成了空对象。三臂 Root、initial Work、additional Work、edges、continuation
和 nested action 的公共字段错误均为 0。

E 相对 D 仅增加 8 schema bytes，六次 input 总量增加 12 tokens，约 0.14%；request-2+ cache 只差 0.14pp。
uncached 总量受各 schema 首次出现和轮换顺序影响，不能横向解释为 E 的持续成本。F 虽然 schema 更小，但正确性低于
E，不能以 token 体积覆盖合同稳定性。

## 4. 根因判定

| Hypothesis | Verdict | Evidence |
|---|---|---|
| H-011：Finish 使用对象类型导致 `goal` 泛化 | Refuted | E 仍为对象，但 6/6 合法 |
| H-012：`finish`/`node_id` 与普通节点共享命名束触发字段泛化 | Confirmed | D 5/6 错误，E 0/6；除命名束外保持对象和请求合同不变 |
| 标量是必要条件 | Refuted | E 不使用标量已经 6/6 通过；F 反而有 1/6 类型错误 |

F5.0 已排除 schema breadth 和 description；F5.0b 又排除对象类型。当前证据将根因收敛到 wire contract 的
identity 命名相似性，不涉及 projection、历史丢失、Runtime reject 或 Agent 状态决策。

## 5. 验证

| Gate | Result |
|---|---:|
| scalar-supported fixture | PASS |
| naming-supported fixture | PASS |
| no-candidate + common regression fixture | PASS |
| F5.0 bootstrap probe regression | PASS |
| D/E/F actual schema structure assertions | PASS |
| secret、目标正文、reasoning 原文不落盘 | PASS |
| live provider | 18/18 HTTP 200，18/18 单 control call，18/18 可解析 |

## 6. 下一步

进入 F5.0c，只实施 E 臂 wire contract：`finish: { node_id }` 一次性切换为
`finish_identity: { id }`，同步修改生产 schema、typed parser、mapping、event/replay 和 observer，不保留旧字段兼容。
完成 deterministic 回归及 simple/complex 各一次后暂停，再进入独立的 F5.1 hard-state 工具面。
