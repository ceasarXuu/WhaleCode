# R6 Phase B 纯领域核心实施结果

## 1. 阶段结论

```text
Status: Complete / Phase C Ready
Candidate: b3d45ec94655b3f31277d349c29b623a9f190864
Production behavior changed: no
Compatibility layer added: no
```

Phase B 已完成 Rooted DAG 的纯 Rust 领域核心，但整个模块只通过 `#[cfg(test)]` 暴露。它没有接入
provider tool schema、handler、projection、snapshot 或旧 runtime，也没有双写或兼容分支。Phase C
之前，CLI 的 TaskSpace 生产行为仍然是 R5。

机器结果见 `benchmarks/taskspace/r6/phase-b-result.json`。

## 2. 实现内容

新增 `core/src/action_map/rooted_dag/`，拆为 9 个不超过 500 行的文件：

| 模块 | 责任 |
|---|---|
| `model.rs` | Root/Work/Finish、状态、边、revision、canonical state hash |
| `invariants.rs` | 单 Root/Finish、单 source/sink、cycle、正反可达、角色状态校验 |
| `transitions.rs` | Agent 声明的状态转换、Root-open readiness、join 全前驱满足 |
| `transactions.rs` | initialize/mutate/transition/finish 的原子事务与机械拒绝 |
| `events.rs` | canonical lifecycle events、batch、reducer、replay 与 corruption 拒绝 |
| `fixture_tests.rs` | Phase A 的 14 个图 fixture、21 组 role/status、稳定 hash |
| `property_tests.rs` | 固定种子的合法/任意图、cycle 和顺序确定性生成测试 |
| `replay_tests.rs` | fork/join、原子 mutation、拒绝零提交、终结和 20-cycle replay |

事务使用 `clone -> event -> reduce -> full validate -> commit`。Runtime 只检查机械合同，不推断
goal、拓扑或下一步。Root/Work/Finish goal、source refs 的顺序与重复项、最终总结均原样保存；拒绝
只携带稳定 code、subjects、当前 revision 和 `state_commit=false`。

## 3. 正确性门禁

| Gate | 结果 | 证据 |
|---|---:|---|
| Cargo 定向测试 | PASS | 18 passed / 0 failed |
| Phase A 图 fixture | PASS | 14/14 合法性匹配 |
| role/status 矩阵 | PASS | 21/21 组合覆盖 |
| property tests | PASS | 4 项 x 256 例，固定 seed |
| reject 原子性 | PASS | state hash/revision 不变，partial commit=0 |
| fork/join readiness | PASS | join 等待全部普通前驱 completed |
| replay | PASS | 20 work cycles，revision 42，逐字段/hash 一致 |
| event corruption | PASS | 错 event ID、空 event batch 明确拒绝 |
| Bazel 定向分片 | PASS | `--test_arg=action_map::rooted_dag`，8 shards |
| production build | PASS | attested Whale binary，commit 与 source 对齐 |
| Cargo/Bazel lock | PASS | lock 已更新并通过 lock check |
| contract/observer self-test | PASS | 14 fixtures；performance observer 自测通过 |

`cargo check -p codex-core --lib --locked` 通过；`petgraph` 与 `proptest` 只作为 `codex-core`
dev-dependency，Rooted DAG 模块没有生产 re-export。`just fix -p codex-core` 和 `just fmt` 已执行，
Clippy 对旧 core 的既有 warning 不作为本阶段新增失败。

## 4. Docker 身份快速臂

样本为 `single-file-fast-fix`，模型为 `deepseek-v4-flash`，两侧均在 Docker hard boundary 中执行
1 次。单次 quick arm 的总和、均值和中位数相同，只用于正确性和生产身份诊断。

| Arm | 结果 | Req | Ordinary | Control | Failed | Wall | Input | Cached | Uncached | Output | Total | Req2+ cache |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | PASS | 6 | 8 | 0 | 0 | 13.263s | 39,276 | 37,760 | 1,516 | 1,220 | 40,496 | 95.81% |
| R6-B identity | PASS | 7 | 9 | 1 | 1 | 20.024s | 53,317 | 50,432 | 2,885 | 2,121 | 55,438 | 94.14% |
| Frozen R5 reference | PASS | 7 | 7 | 4 | 0 | 20.464s | 50,746 | 48,256 | 2,490 | 1,746 | 52,492 | 94.67% |

R6-B identity 相对本轮 Standard：requests `1.17x`、wall `1.51x`、input `1.36x`、uncached
`1.90x`、total `1.37x`，request 2+ cache 低 `1.66pp`。TaskSpace 的一次 failed tool 是修复前
pytest 按预期暴露缺陷，随后 patch 并复测通过，不是 control/runtime 失败。

这些数值不能用于评价 R6 Rooted DAG，因为新核心尚未接入生产。R6-B identity 使用的仍是 R5
生产实现，Frozen R5 也只是另一条单次随机轨迹；这里只能确认候选二进制可运行、两侧业务正确性通过。

## 5. Map 观察

| Arm | Map | Nodes | Edges | Results | Open leaves |
|---|---:|---:|---:|---:|---:|
| Standard | 0 | 0 | 0 | 0 | 0 |
| R6-B identity | 1 | 3 | 0 | 0 | 3 |
| Frozen R5 reference | 1 | 3 | 0 | 3 | 0 |

本轮 Agent 只调用一次旧 `taskspace_control` 初始化 Map，之后直接给出最终答复；旧 runtime 允许业务
任务完成时仍留下三个未闭合零边节点。Phase A 的同代码 R5 轨迹则闭合了三个节点。差异来自 Agent
随机动作路径和旧模型可表达性，不是 Phase B 新核心回归。Phase C 会通过唯一 Root/Finish、强图不变量
和显式 `finish_end` 纵向替换这套旧权威，不在 Phase B 给旧 runtime 增加补丁。

## 6. 工具链与日志经验

1. Bazel 的 `--test_filter` 没有转发为 Rust harness 名称过滤，第一次意外执行完整 core target。
   Rooted DAG 18 项均通过，但既有 config/file-watcher/Guardian/session 测试失败，其中 Guardian 明确
   缺少 Bazel test 环境内的 `DEEPSEEK_API_KEY`。改用 `--test_arg=action_map::rooted_dag` 后定向门禁通过。
2. BuildBuddy 匿名 BEP 上传在结果完成后给出弃用警告并长时间不退出；本地测试结果已经落盘，上传
   警告不影响门禁。后续 Bazel 运行应优先关闭不需要的匿名远端上传或配置正式凭据。
3. benchmark runner 不自动加载仓库 `.env.local`。第一次只在 credential preflight 失败，未发出
   provider 请求；显式导出环境变量后有效运行通过。后续 runner 封装应统一负责安全加载本地 env。
4. Docker artifacts 完整包含 rollout、provider/cache、Map、container lifecycle、validation 和 oracle
   日志；本阶段结果可从固定 run root 重建。

## 7. 延后项

以下不是遗漏，而是继续保留在原计划的责任边界：

- Phase C：tool schema、handler、snapshot、projection 和 runtime 一次纵向切换，删除旧权威；
- Phase D：运行中节点相关 mutation 因果规则、并发 frontier、lease/owner；
- Phase E：持久化 corruption、crash injection、resume/fork 完整矩阵；
- Phase G：`node_detail_expanded` 与详情折叠/展开；
- 完整 workspace/release suite 和对抗性审查仍需用户授权，不以本次定向门禁替代。

## 8. 退出判断

| Phase B Gate | 结果 |
|---|---|
| 合法 DAG 全接受、非法矩阵全拒绝 | PASS |
| reject 前后 state hash/revision 不变 | PASS |
| 20-cycle replay 与直接状态一致 | PASS |
| property tests 无 panic、漏环、顺序漂移 | PASS |
| 新核心 production 不可达、无双写 | PASS |
| Standard/当前 TaskSpace simple 均完成外部验证 | PASS |
| 代码提交推送且工作区 clean | PASS |

Phase B 可以收口，下一阶段是 Phase C 一次纵向生产切换。本次按用户要求停在 Phase C 前。
