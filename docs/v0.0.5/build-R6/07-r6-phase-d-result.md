# R6 Phase D 动态图、并发前沿与 Agent 控制结果

## 1. 阶段结论

```text
Status: Complete / Phase E Ready
Runtime candidate: 3e400206a7d500c9b8576b137826c9f0899fed98
Benchmark observability fix: 424921adbb158d3990a335a7eb40e008d39497ce
Runtime semantic graph repair: none
```

Phase D 已补全 Rooted DAG 的动态图硬规则：Agent 可以批量声明节点和边变更、block/unblock、
rework 与带 revision 的并发写；Runtime 只做图不变量、执行因果、状态转换、lease 和原子提交校验。
它不替 Agent 选择边、自动补 join、推断 rework 或合并 stale 意图。

机器结果见 `benchmarks/taskspace/r6/phase-d-result.json`。

## 2. 已完成能力

| 区域 | Phase D 结果 |
|---|---|
| 图事务 | `mutate_graph` 批量 add nodes/add edges/remove edges，候选全量通过后一次提交 |
| 执行因果 | Running/Blocked/Completed Work 的入边事实不可改写；指向未来节点的出边仍可修改 |
| Readiness | 多前驱全部满足；图变更/rework 后支持 Pending 与 Ready 双向机械重算 |
| Rework | 仅 Agent 可发起 Completed -> Ready；保留历史 result/event，不生成替代节点或建议 |
| 并发冲突 | `expected_revision` stale 写机械拒绝，不重放、不合并、不覆盖新图 |
| 活动前沿 | 只包含 Ready/Running Work；Root/Finish 不获得 ordinary tool lease，Finish ready 单独显示 |
| 恢复校验 | snapshot 恢复校验 node/status/lease/holder/owner/current binding 一致性 |
| 协议与 Viewer | snapshot、JSON/TS schema、TUI 均展示真实边、入度/出度和合法 active frontier |
| 拒绝反馈 | 统一 `state_commit=false`、`partial_commit=0` 与稳定 violation code |

`execution_causality_conflict` 只保护已经发生的执行事实，不评价 Agent 的任务语义。rework 若已有
Running/Blocked/Completed 的传递后继则拒绝，因为下游已经消费旧完成事实；否则允许并重算前沿。

## 3. 确定性机制证明

| 门禁 | 结果 |
|---|---|
| fork/join/diamond 与多前驱 readiness | PASS |
| Ready 因新增未满足前驱回退 Pending | PASS |
| started Work 入边改写原子拒绝 | PASS |
| rework 保留 result 并回退下游前沿 | PASS |
| 已消费 result 的 rework 原子拒绝 | PASS |
| 同 revision 双写只保留胜者 | PASS |
| 混合 add/remove 失败零部分提交 | PASS |
| Root/Finish lease 与非法恢复状态拒绝 | PASS |
| diamond 观测 max indegree/outdegree = 2 | PASS |

聚焦回归：`phase_d_tests` 过滤用例 13/13、`action_map::` 66/66、control handler 16/16、
tool schema 3/3、protocol fixture 4/4、Viewer snapshot 3/3；图观测脚本、metrics extractor 与完整
benchmark harness 自测均通过。

## 4. Docker 三臂结果

R5 固定为 `d12818f`，不在 R6 分支伪装重跑。Standard 与 R6 使用相同模型、provider、Docker
substrate 和 validator；每臂只取 1 次快速诊断，因此单次值同时也是总和、均值和中位数，不能用于
统计收益判断。两次 live pair 均为 `valid_pair=True`，两臂均通过公开测试和隐藏 oracle；命令退出 1
仅因 `Repeats=1` 不满足 E2 聚合门槛。

### 4.1 正确性、动作与 Map

| Sample | Arm | 来源 | 结果 | Requests | Runtime tools | Controls | Failed | Map N/E/R | Shape |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| branch-join | Standard | 同轮 | PASS | 9 | 17 | 0 | 1 | 0/0/0 | N/A |
| branch-join | R5 | Phase A 冻结 | PASS | 9 | 12 | 5 | 1 | 4/0/4 | 无边 Map |
| branch-join | R6 | 同轮 | PASS | 8 | 13 | 6 | 1 | 5/4/0 | chain，Map 未闭合 |
| rework opportunity | Standard | 同轮 | PASS | 10 | 16 | 0 | 1 | 0/0/0 | N/A |
| rework opportunity | R5 | S4.2 冻结 | PASS | 13 | 20 | 5 | 0 | 4/0/4 | 无边 Map |
| rework opportunity | R6 | 同轮 | PASS | 13 | 22 | 7 | 1 | 5/4/3 | chain，显式终结 |

两个 R6 Map 都满足单 source、单 sink、全节点位于 Root -> Finish 路径、无环；max depth 均为 4，
max indegree/outdegree 均为 1。Agent 两次都自然选择 chain，没有调用 `mutate_graph` 或 `rework`。
因此 live 证据只证明工具链未阻碍任务完成，不能宣称 Agent 已自然利用 fork/join/rework；这些机制由
确定性 fixture 证明。Runtime 没有为了测试目标自动补图。

branch-join 中 Agent 在业务验证通过后结束，但 Map 仍有一个 Running Work、Root/Finish 未闭合。
这次没有发生 control reject 或 hard stop。显式终结、完成权威与 resume 后终结属于 Phase E，本阶段
保留该事实，不把外部业务成功改写成 Map 完成。

### 4.2 Token、缓存与时间

| Sample | Arm | Wall | Input | Cached | Uncached | Output | Req2+ hit |
|---|---:|---:|---:|---:|---:|---:|---:|
| branch-join | Standard | 56.361s | 98,070 | 93,184 | 4,886 | 6,036 | 94.74% |
| branch-join | R5 frozen | 50.140s | 100,110 | 93,440 | 6,670 | 6,028 | 93.23% |
| branch-join | R6 | 68.650s | 98,088 | 86,784 | 11,304 | 7,359 | 87.94% |
| rework opportunity | Standard | 42.976s | 99,948 | 93,184 | 6,764 | 4,983 | 92.88% |
| rework opportunity | R5 frozen | 52.866s | 150,678 | 144,512 | 6,166 | 6,033 | 95.84% |
| rework opportunity | R6 | 73.288s | 187,505 | 180,608 | 6,897 | 8,658 | 96.25% |

| Sample | R6/Standard Requests | Wall | Input | Uncached | Output | Cache delta |
|---|---:|---:|---:|---:|---:|---:|
| branch-join | 0.89x | 1.22x | 1.00x | 2.31x | 1.22x | -6.80pp |
| rework opportunity | 1.30x | 1.71x | 1.88x | 1.02x | 1.74x | +3.37pp |

branch-join 的 requests 少于 Standard，但时间更高；复杂样本 requests/input/output 都更高。两次均为
单样本随机轨迹，Phase D 不下性能收益结论。正式三次轮换、provider/tool 时间拆解和 projection 成本
重基线仍留给 Phase G。

## 5. 运行中发现并修复的观测缺口

复杂样本第一次运行时，R6 Agent 修改代码后自行创建 Git commit。validator 读取最终文件并通过，
但 harness 只执行 `git diff`，把已提交变更错误报告成 `changed_paths=none`、空 patch。

修复后，样本初始化时保存 `refs/taskspace-benchmark/baseline`，采集统一比较 baseline commit 到最终
工作树；新增 `workspace-change-baseline.json` 记录 baseline、最终 HEAD、是否推进 commit、工作树状态和
diff 字节数。回归测试覆盖“已提交源码变更 + 未提交新文件”，完整 harness 自测与 Docker 复跑均通过。
这是一处反馈/观测层修复，没有改变 Runtime 或 Agent 行为。

## 6. 工程门禁

| Gate | 结果 |
|---|---|
| Phase D 聚焦 Rust、schema、Viewer 与观测回归 | PASS |
| `just fix -p codex-core/codex-tools/codex-protocol/codex-tui` | PASS |
| `just fmt` | PASS |
| `cargo build -p codex-cli --bin whale --locked` | PASS |
| binary SHA/源码提交 attestation | PASS |
| branch-join + rework opportunity Docker 外部验证 | PASS |
| benchmark commit 变更观测回归与真实复跑 | PASS |

按 vendor 约束，最终 `just fmt` 后没有重复 Rust 测试；Docker 使用格式化后的 attested binary。
完整 workspace/release suite 未执行，按计划保留到 Phase H。

## 7. 退出判断

| Phase D Gate | 结果 |
|---|---|
| fork/join/diamond/rework fixture 状态推进正确 | PASS |
| 所有 mutation reject 零状态/部分提交 | PASS |
| stale revision 不覆盖新图、不自动合并意图 | PASS |
| active frontier 与 lease 只指向合法 Work | PASS |
| branch-join 与 rework 三臂各 1 次 | PASS，live adoption 未观察到 |
| Agent 未建理想图时 Runtime 不补图 | PASS |
| 结果、限制、成本、缓存和自然动作路径进入报告 | PASS |
| 提交已推送、工作区 clean | 待本结果提交完成后确认 |

Phase D 完成。下一阶段是 Phase E：显式 Finish 终结、event replay、resume/fork 与故障原子性。
