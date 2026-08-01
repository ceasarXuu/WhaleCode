# R6 Phase C 生产链路纵向切换结果

## 1. 阶段结论

```text
Status: Complete / Phase D Ready
Production candidate: fa6b6ff1885994d994fda616a3cb7a3a1b27b499
Compatibility layer added: no
Legacy production authority retained: no
```

Phase C 已把 TaskSpace 的生产 model、tool schema、handler、Event Store、snapshot、projection 和
observer 纵向切换到同一张 Rooted DAG。Map 本身就是状态机：唯一 `task_root` 是 source，唯一
`finish` 是 sink，所有 Work 位于 Root 到 Finish 的路径上；Root 只在 Agent 显式 `finish_end` 时与
Finish 同一事务闭合。

本阶段没有让 Runtime 补边、选择节点、解释工具结果或生成下一步建议。Runtime 只校验图不变量、
revision、角色状态和事务原子性；Agent 的 goal、工具 call/output、结果和 final summary 保持原样。

机器结果见 `benchmarks/taskspace/r6/phase-c-result.json`。

## 2. 生产切换

| 区域 | Phase C 结果 |
|---|---|
| 领域模型 | Phase B `rooted_dag` 核心进入生产，旧平行 Task/Map 完成权威删除 |
| Tool schema | 初始化必须声明 Root、初始 Work、可选其余 Work、Finish 和完整边；依赖不再默认空 |
| Handler | `initialize_map`、`mutate_graph`、`transition_node`、`finish_end` 统一走候选校验后原子提交 |
| Snapshot | 一等保存 `root_node_id`、`finish_node_id`、role/status、revision、nodes 和 edges |
| Event Store | 保存原始 Agent control call/output 与普通上下文；机械 lifecycle event 单独记录 |
| Projection | 每个 context epoch 一份全图基线；后续状态由原始 append-only control journal 表达 |
| Observer | 输出 Root/Finish、节点、边、结果、开放状态、语义保留和精确 payload 扫描 |
| 旧数据 | 旧 schema/session 返回 `legacy_schema_unsupported`，不迁移、不猜测、不 fallback |

生产 schema 中 `create_node`、`finish_then_end`、terminal-list 和可空 dependency 已不可表达。静态扫描
只在负向测试中保留这些字符串，用于证明旧输入被拒绝，不是兼容入口。

## 3. 修复的生产阻断

纵向切换初次 live 运行暴露了三个工具链问题，均按“先检查语义传递，再考虑 Runtime 约束”的原则
修复并关闭：

1. 旧 bootstrap 合同允许把 Root 当作 current Work，随后机械 Bind 必然拒绝；拒绝回执还把候选
   revision 当作已提交 revision。新合同只接受明确 Work，回执报告真实 pre-state。
2. 初始 Work 与 Work 列表字段语义重叠，Agent 持续重复声明同一节点；preflight 又把 typed 错误
   字符串化。新字段集合互斥，无效参数由原 handler 返回单层 typed JSON。
3. 每轮删除旧末尾 projection、再在新增历史后追加新版 projection，破坏 DeepSeek 严格前缀缓存。
   现改为 epoch 基线加原始 control journal，不积累 stale projection，也不由 Runtime 生成语义 delta。

对应 COE：

- `coe/2026-07-15-08-26-r6-transition-reject-loop.md`
- `coe/2026-07-15-08-55-r6-bootstrap-node-duplication-loop.md`
- `coe/2026-07-15-09-29-r6-latest-projection-cache-prefix.md`

## 4. Docker 三臂结果

R5 使用 Phase A 已冻结的 commit `d12818f055494e510c6bd1d34f5b7b5154536471` 单次结果；Standard 与
R6 是本阶段同轮 Docker pair。R5 不在当前分支伪装重跑，表中明确标记为冻结参考。每臂均为 1 次，
因此总和、均值和中位数相同；这里只做 correctness 快速门禁，不宣称统计显著性。

### 4.1 正确性、动作和 Map

| Sample | Arm | 来源 | 结果 | Requests | Ordinary tools | Controls | Failed | Map N/E/R |
|---|---|---|---|---:|---:|---:|---:|---:|
| simple | Standard | Phase C 同轮 | PASS | 6 | 8 | 0 | 0 | 0/0/0 |
| simple | R5 | Phase A 冻结 | PASS | 7 | 7 | 4 | 0 | 3/0/3 |
| simple | R6 | Phase C 同轮 | PASS | 13 | 8 | 10 | 0 | 5/4/3 |
| branch-join | Standard | Phase C 同轮 | PASS | 14 | 24 | 0 | 1 | 0/0/0 |
| branch-join | R5 | Phase A 冻结 | PASS | 9 | 12 | 5 | 1 | 4/0/4 |
| branch-join | R6 | Phase C 同轮 | PASS | 13 | 21 | 7 | 1 | 4/3/2 |

六臂均由 Agent 正常完成并通过外部验证。R6 的两个 Map 都只有一个 Root、一个 Finish、零开放节点，
且全部节点位于 Root 到 Finish 的路径上：

```text
simple:      root -> explore -> fix -> run_tests -> finish
branch-join: root -> explore_project -> fix_issues -> finish
```

branch-join 的自然轨迹本次形成 chain，没有形成多入边 join。这不影响 Phase C 的唯一 source/sink
生产门禁；自然 fork/join、mutation 和并发 frontier 属于 Phase D，Runtime 不应替 Agent 自动造出 join。

### 4.2 Token、缓存与时间

| Sample | Arm | Wall | Input | Cached | Uncached | Output | Req2+ hit |
|---|---|---:|---:|---:|---:|---:|---:|
| simple | Standard | 19.544s | 41,993 | 40,448 | 1,545 | 1,897 | 96.04% |
| simple | R5 frozen | 20.464s | 50,746 | 48,256 | 2,490 | 1,746 | 94.67% |
| simple | R6 | 30.427s | 111,170 | 97,408 | 13,762 | 2,569 | 91.28% |
| branch-join | Standard | 76.212s | 184,211 | 174,848 | 9,363 | 8,349 | 94.77% |
| branch-join | R5 frozen | 50.140s | 100,110 | 93,440 | 6,670 | 6,028 | 93.23% |
| branch-join | R6 | 55.775s | 162,854 | 150,784 | 12,070 | 5,737 | 92.47% |

| Sample | R6/Standard Req | Wall | Input | Uncached | Output | Cache delta |
|---|---:|---:|---:|---:|---:|---:|
| simple | 2.17x | 1.56x | 2.65x | 8.91x | 1.35x | -4.76pp |
| branch-join | 0.93x | 0.73x | 0.88x | 1.29x | 0.69x | -2.30pp |

simple 仍有明显 request 和 input 放大，branch-join 单次轨迹则低于 Standard。两者说明成本仍受 Agent
动作路径影响，不能从各 1 次样本得出总体性能结论。该问题进入 Phase G 的三次轮换重基线和逐 request
section 分解，不在 Phase C 用 Runtime 行为约束掩盖。

## 5. Projection 与缓存修复证据

修复前简单样本的 R6 request 2+ cache 只有 `0.32%`，message prefix 为 `0/9`。修复后：

| Sample | Logical requests | Exact scan | Active projection | Message prefix | Same-shape zero | Req2+ hit |
|---|---:|---:|---:|---:|---:|---:|
| simple | 13 | 39/39 PASS | 恒为 1 | 12/12 | 0 | 91.28% |
| branch-join | 13 | 39/39 PASS | 恒为 1 | 12/12 | 0 | 92.47% |

两个样本的 Map 语义 retention/salience 都是 `100%`，semantic replacement、protected miss 和
compaction 都是 0。branch-join 的 full-shape prefix 为 `11/12`，唯一差异来自一次 bootstrap
tool choice/schema shape 转换；消息前缀仍是 `12/12`，同 shape 没有零命中。

## 6. 工程门禁

| Gate | 结果 |
|---|---|
| `codex-tools` 与 control/schema 定向回归 | PASS |
| `codex-core` rooted DAG/action_map/control/sequence/reconstruction 定向回归 | PASS |
| epoch snapshot、compaction 和 provider payload scanner 回归 | PASS |
| observer、benchmark harness、cost instrumentation 自测 | PASS |
| `cargo build -p codex-cli --bin whale --locked` | PASS |
| binary attestation 与候选 commit 对齐 | PASS |
| `just fix -p codex-core`、`just fix -p codex-tools`、`just fmt` | PASS |
| simple + branch-join Docker 外部验证 | PASS |

未执行完整 workspace/release suite；当前计划把全量矩阵保留在 Phase H。`just fix`/`just fmt` 后按
vendor `AGENTS.md` 约束没有再次运行测试，之后只用格式化后的 attested binary 完成了两组 Docker
业务与精确 payload 门禁。

构建经验：从 `third_party/codex-cli/codex-rs` 调用仓库脚本需要使用 `../../../scripts/...`，不是
`../../scripts/...`。attestation 文件位于
`third_party/codex-cli/codex-rs/target/debug/whale.build-attestation.json`。

## 7. 退出判断

| Phase C Gate | 结果 |
|---|---|
| 初始化后恰好一个 Root/Finish | PASS |
| 全节点均位于 Root -> Finish 路径 | PASS |
| completion 只有图内派生来源 | PASS |
| 旧 create/terminal/zero-edge schema 不可表达 | PASS |
| 旧 session 明确 fatal，不兼容迁移 | PASS |
| simple + branch-join 三臂参考均通过外部验证 | PASS |
| 精确 projection、前缀缓存与反馈真实性门禁 | PASS |
| 提交已推送且工作区 clean | PASS |

Phase C 完成。下一阶段是 Phase D：动态图、并发 frontier、多前置依赖与 Agent 声明的原子图变更。
