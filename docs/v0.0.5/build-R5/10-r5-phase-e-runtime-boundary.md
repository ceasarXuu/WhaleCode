# R5 Phase E：Runtime 硬边界收敛与验收

> 日期：2026-07-10
> 状态：实现与单样本验收完成；未执行用户授权的对抗性审查
> 目标：删除普通请求预算 hard stop 和 runtime 语义决策，只保留状态机、协议、权限、安全、资源硬底线，并保证拒绝反馈忠实进入 Agent 上下文。

## 1. 结论

Phase E 的边界目标已完成：route/profile request count 只观测、不再终止正常采样；Agent completion、sampling interruption、external validation 已拆分；projection、tool output、action-contract 和 `taskspace_control` 活动路径不再合成策略、证据或下一步动作；当前拒绝会以原始机械反馈进入 provider history。

最终 `count-call-stack` 单样本中，R5 在 13 次内部 provider request 后自行完成，map 闭合、Agent final 可见、外部验证通过，未出现 profile hard stop 或 runtime 禁用语义标记。该样本同时显示明显成本回退：R5 wall time 为 standard 的 3.11 倍。Phase E 只证明“不会被错误截断且语义边界更干净”，不证明 TaskSpace 已有成本收益。

## 2. 子阶段完成情况

| 子阶段 | 工程结果 | 验收结果 | 状态 |
|---|---|---|---|
| E0 请求 hard stop 退场 | 删除 profile-count pre-dispatch 拒绝及 grace 控制；请求 profile 只写 trace | 超过旧 profile 后仍完成；无 `TaskSpaceProviderBudgetHardStopV1` | 完成 |
| E0 生命周期拆分 | completion、interruption、external validation 独立提取和报告 | 两侧均为 Agent complete；外部验证不覆盖 completion | 完成 |
| E1 反馈收薄 | tool output 不再生成 semantic summary；recovery 不含策略性 next action | 39 次 exact payload scan 全部通过，禁用标记 0 | 完成 |
| E2 硬门禁分类 | 拒绝分类为 state machine、protocol、permission、safety、resource | focused taxonomy/protocol tests 通过 | 完成 |
| E3 fallback 审计 | action-contract 只做显式动作机械映射；不再把 finish 改成 block、自动建 validation/rework 或伪造事实 | fallback 和 failed validation focused tests 通过 | 完成 |

## 3. 活动路径边界

### 3.1 Runtime 仍负责

- task/map/node/lease 的机械生命周期和状态机转移。
- JSON/参数协议校验、权限、安全和外部资源错误。
- 原始工具输出的有界传递、透明截断和可恢复引用。
- profile/request/tool/control 的观测计数与 trace。
- 状态非法时拒绝动作，并返回 exact reason、gate class 和 state unchanged。

### 3.2 Runtime 不再负责

- 根据请求次数判断 Agent 应停止。
- 根据 node kind、validation failure、coverage 或 fact source 决定 Agent 下一步动作。
- 合成 missing property、schema rename、validation strategy 等工具语义摘要。
- 把 `finish_node` 重解释为 `block_node`，自动创建 rework/validation，或伪造 state commit 事实。
- 把 runtime interruption 或外部 validator success 伪装为 Agent 正常完成。

### 3.3 关键活动路径

| 边界 | 实现位置 | 结果 |
|---|---|---|
| provider-visible 当前拒绝 | `core/src/session/turn.rs` 的 `CurrentTaskspaceRuntimeFeedback` 分类和 call/output 成对保留 | 当前 gate/final rejection 不再被 history replacement 吞掉 |
| preflight 拒绝归档 | `core/src/tools/parallel.rs` 调用 `record_action_map_runtime_feedback` | dispatch 前失败也进入 node/runtime feedback |
| control 原始解析 | `core/src/tools/handlers/taskspace_control.rs` 直接 `parse_arguments` | 生产入口不调用旧 semantic normalizer |
| 硬门禁 taxonomy | `core/src/action_map/runtime.rs` 的 `TaskSpaceHardGateClass` | 拒绝只表达硬边界类型和机械原因 |
| 原始 tool feedback | `core/src/tools/context.rs`、`core/src/tools/mod.rs` | 删除 schema/validation 再解释；只保留透明 truncation/ref/tail sentinel |
| benchmark 计量 | `scripts/taskspace-benchmark/lib/*` | request、control、completion、interruption、validation 分源记录 |

## 4. 测试与构建

| 验证 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | PASS；stable rustfmt 仅提示 workspace nightly 配置不可用 |
| `cargo test -p codex-core --no-run` | PASS；184 个旧策略 dead-code warning 留给 Phase F 物理删除 |
| Phase E focused core tests | PASS：当前拒绝反馈、hard taxonomy、protocol、raw output、无自动 rework、无 action 重解释、Agent 自主并行选择等 |
| `cargo test -p codex-core --lib` | FAIL：2160 passed、224 failed、3 ignored；失败集中在已撤销的自动 validation/rework、node-kind 策略 gate、semantic action-contract normalization、ledger/final 约束及少量历史 guardian/session 测试 |
| `cargo test -p codex-features` | PASS，37/37 |
| `cargo test -p codex-tools` | PASS，139/139，1 ignored |
| benchmark metrics/cost/E3-validity/harness selftests | 4/4 PASS |
| `cargo build -p codex-cli --bin whale` | PASS |

说明：Phase E 的活动路径和 focused gate 通过，但仓库全量 core 测试尚未恢复绿色。旧套件仍包含已经退役的策略断言，必须在 Phase F 随死代码一起删除或重写，并复核少量非 TaskSpace 历史失败；不应通过兼容分支让旧断言继续成立。Phase F 未完成前，R5 不能进入最终回归/发布状态。

## 5. 单样本横向证据

最终运行：

```text
target/r5e-phase-e-final-clean/count-call-stack/20260710-043411-389
```

| 版本 | 证据来源 | outcome | Agent completion | interruption | external validation | wall time | 普通 tools | provider requests | control calls |
|---|---|---|---|---|---|---:|---:|---:|---:|
| standard 当前 | 同次 left | solved | complete | false | passed | 15098ms | 10 | unavailable | 0 |
| R4 历史 | `target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136` | solved | 旧口径未拆分 | 旧口径未拆分 | passed | 154525ms | 11 | unavailable | 旧口径 |
| R5 Phase E | 同次 right | solved | complete | false | passed | 46971ms | 12 | 13，rollout trace | 2，rollout trace |

证据限制：R4 是历史文档基线，不是本轮同机运行；standard 没有 rollout telemetry，因此 request count 必须为 unavailable。单次样本只用于阶段诊断，不进入 utility aggregate，也不构成成功率或成本统计结论。

## 6. 样本行为分析

R5 比 standard 多做了两类普通动作：pytest 缺失后尝试联网安装 pytest，以及额外执行一次直接函数验证。它还在首次 `finish_node` 成功、projection 已显示 `current_node: none` 和 `status=completed` 后错误判断“node is still running”，再次调用 `finish_node`。runtime 只按硬状态机拒绝第二次调用，并返回 `no_current_node_binding`；下一轮 Agent 读取 projection 和拒绝后自行纠正并完成。

这个重复动作不是反馈丢失：第一次 finish 的成功输出和 completed projection 都已进入 provider history。它是 Agent 对可见状态的错误理解。按照 R5 原则，runtime 不应为此增加策略约束；当前状态机拒绝只守住不可重复完成无绑定节点的硬底线，边界正确。

成本放大的主要可见因素：

- R5 普通工具 12 次，对 standard 10 次；另有 2 次状态机 control。
- R5 provider input 从首轮 8389 tokens 增长到末轮 33631 tokens，13 次请求累计 input 269093 tokens。
- projection 虽已去除策略字段，但每轮仍重复携带增长中的 node event excerpts 和 result refs。
- standard 缺少同口径 provider request telemetry，不能计算可信 request ratio。

## 7. Phase E 收益判定

| 假设 | 结果 | 证据 |
|---|---|---|
| 正常执行不再被 profile 截断 | PASS | R5 超过旧 6-request profile 后继续到 13 并完成；hard-stop 标记 0 |
| 中断不再伪装成 Agent completion | PASS | completion/interruption/validation 独立字段和 harness fixtures 通过 |
| 不通过新语义策略控制成本 | PASS | forbidden marker 0；action-contract/control/tool feedback 活动路径无语义合成 |
| feedback 拒绝能进入下一轮上下文 | PASS | final/gate feedback pair test；live 第二次 finish rejection 后 Agent 自行纠正 |
| 简单任务成本不回退 | FAIL，非 Phase E 退出门禁 | wall ratio 3.11；输入上下文逐轮增长；进入 Phase F/G 分析 |

## 8. 遗留项

1. Phase F 删除 184 个 warning 所暴露的旧 semantic normalizer、validation/rework policy、context compiler 和相关历史测试，并拆分 map/event/gate/projection 模块；把当前 `2160 passed / 224 failed / 3 ignored` 恢复为新边界下的全量绿色基线，不做兼容。
2. Phase G 对 projection 的重复 event/ref 体积和 request cadence 做同口径收益审计，优先保证可恢复语义，再做机械去重、引用和渐进暴露。
3. 为 standard 补齐真实 rollout/request telemetry 后再比较 request ratio；当前禁止用 token record 或 outer exec 猜测。
4. 本次尚未执行用户授权的对抗性审查；执行前不把它记为已完成 gate。

## 9. 退出决定

Phase E 的活动路径退出门禁通过，可以进入 Phase F；但全量 core suite 尚未绿色，R5 整体不能跳过 Phase F 进入最终回归。禁止因当前成本回退或旧测试失败恢复 route/profile hard stop、node-kind 策略 gate、semantic summary 或 runtime 自动 rework；后续优化首先检查重复上下文、引用策略和 feedback 效率。
