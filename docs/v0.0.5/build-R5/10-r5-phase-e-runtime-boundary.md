# R5 Phase E：Runtime 硬边界收敛与验收

> 日期：2026-07-10
> 状态：E0-E5 与后续 G0/G1 门禁完成；R5-F 为下一阶段；未执行用户授权的对抗性审查
> 目标：删除普通请求预算 hard stop 和 runtime 语义决策，只保留状态机、协议、权限、安全、资源硬底线，并保证拒绝反馈忠实进入 Agent 上下文。

## 1. 结论

Phase E 的边界目标已完成：route/profile request count 只观测、不再终止正常采样；Agent completion、sampling interruption、external validation 已拆分；projection、tool output、action-contract 和 `taskspace_control` 活动路径不再合成策略、证据或下一步动作；当前拒绝会以原始机械反馈进入 provider history。

E5 已在不回退上述边界的前提下补齐 shell/pipeline/termination 机械反馈契约。后续 G0/G1
又证明 E4 的 latest-only replacement 会同时删除成功状态工具反馈并破坏缓存前缀，现已改为单
epoch snapshot 加 Agent 原始状态工具 journal。

E0-E3 首次验收样本后来被确认存在 E4 污染：history composer 把每轮 active projection 全部追加到 provider payload，导致旧 running 与新 completed 快照并存。E4 只做机械 latest-only 替换，不压缩、不重写 projection 内容，并把唯一性纳入 pre-wire request scan gate。修复后 `count-call-stack` 的 9 个 provider request 全部只有一份 projection，standard/R5 均 solved；R5 wall ratio 从污染样本的 3.11 降至 1.40，总 input token 从 269093 降至 100365。

## 2. 子阶段完成情况

| 子阶段 | 工程结果 | 验收结果 | 状态 |
|---|---|---|---|
| E0 请求 hard stop 退场 | 删除 profile-count pre-dispatch 拒绝及 grace 控制；请求 profile 只写 trace | 超过旧 profile 后仍完成；无 `TaskSpaceProviderBudgetHardStopV1` | 完成 |
| E0 生命周期拆分 | completion、interruption、external validation 独立提取和报告 | 两侧均为 Agent complete；外部验证不覆盖 completion | 完成 |
| E1 反馈收薄 | tool output 不再生成 semantic summary；recovery 不含策略性 next action | E0-E3 样本禁用标记 0；其旧 scanner 的 replacement 结论废弃，E4 scanner v3 重新验收唯一性 | 完成 |
| E2 硬门禁分类 | 拒绝分类为 state machine、protocol、permission、safety、resource | focused taxonomy/protocol tests 通过 | 完成 |
| E3 fallback 审计 | action-contract 只做显式动作机械映射；不再把 finish 改成 block、自动建 validation/rework 或伪造事实 | fallback 和 failed validation focused tests 通过 | 完成 |
| E4 projection 唯一性 | provider history 只保留最新 active projection，旧快照以稳定 reason 省略；scanner 和 benchmark 强制唯一性 | 9/9 真实请求 `active_projection_count=1`；当前 gate/tool feedback 回归通过 | 完成 |

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
| `cargo test -p codex-core --lib` | FAIL：2161 passed、224 failed、3 ignored；新增 E4 回归通过，失败数与 E0-E3 基线一致；失败集中在已撤销的自动 validation/rework、node-kind 策略 gate、semantic action-contract normalization、ledger/final 约束及少量历史 guardian/session 测试 |
| `cargo test -p codex-features` | PASS，37/37 |
| `cargo test -p codex-tools` | PASS，139/139，1 ignored |
| benchmark metrics/cost/E3-validity/harness selftests | 4/4 PASS |
| `cargo build -p codex-cli --bin whale` | PASS |

说明：Phase E 的活动路径和 focused gate 通过，但仓库全量 core 测试尚未恢复绿色。旧套件仍包含已经退役的策略断言，必须在 Phase F 随死代码一起删除或重写，并复核少量非 TaskSpace 历史失败；不应通过兼容分支让旧断言继续成立。Phase F 未完成前，R5 不能进入最终回归/发布状态。

## 5. 单样本横向证据

最终运行：

```text
target/r5e4-projection-latest-only/count-call-stack/20260710-051931-572
```

| 版本 | 证据来源 | outcome | Agent completion | interruption | external validation | wall time | 普通 tools | provider requests | control calls |
|---|---|---|---|---|---|---:|---:|---:|---:|
| standard 当前 | 同次 left | solved | complete | false | passed | 16906ms | 10 | unavailable | 0 |
| R4 历史 | `target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136` | solved | 旧口径未拆分 | 旧口径未拆分 | passed | 154525ms | 11 | unavailable | 旧口径 |
| R5 Phase E4 | 同次 right | solved | complete | false | passed | 23649ms | 9 | 9，rollout trace | 1，rollout trace |

证据限制：R4 是历史文档基线，不是本轮同机运行；standard 没有 rollout telemetry，因此 request count 必须为 unavailable。单次样本只用于阶段诊断，不进入 utility aggregate，也不构成成功率或成本统计结论。

## 6. 样本行为分析

E0-E3 样本中的重复 `finish_node` 不能归因于 Agent 已看到唯一、清晰状态后仍理解错误。第一次 finish 的成功输出和最新 completed projection 确实存在，但 composer 同时保留了此前所有 running projection；Agent 面对的是相互冲突的多个“active replacement”。根因属于上下文语义未正确替换，不应通过 runtime 增加动作约束修复。

E4 在 history item 层按顺序只保留最新 projection，旧项记录 `stale_active_projection_replaced`；当前 tool/gate call-output pair、用户输入和 projection 原文均不改写。真实样本的 9 个请求全部通过 uniqueness scan，未再出现重复 finish，R5 以 9 次普通工具和 1 次 control 完成，少于 standard 的 10 次普通工具。

成本变化：

- R5 总 input 从污染样本 269093 降至 100365，下降 62.7%；请求数从 13 降至 9，下降 30.8%。
- R5 wall time 从 46971ms 降至 23649ms，下降 49.7%；相对同轮 standard 为 1.40 倍。
- 单请求 input 从 8021 增长到 12297，后段约 12K 平台；修复前从 8389 增长到 33631。
- R5 cached input 仅 9728、uncached input 90637，缓存与自然历史增长仍是 Phase G 成本审计项。
- standard 缺少同口径 provider request telemetry，仍不能计算可信 request ratio。

### 6.1 E4 后续结构审计修正

E4 的 latest-only 修复解决了旧、新 projection 同时进入一次请求的问题，但后续审计证明
“唯一性”不等于“缓存结构正确”：

```text
request 1: H0 + P1
request 2: H0 + A1/T1 + P2
```

上一轮已经发送的 `P1` 被删除，新的自然对话项插入其位置后，`P2` 再追加到末尾。
因此 request 2 不包含 request 1 的完整 input/output prefix；standard history 则持续 append-only。
DeepSeek 的 cache 依赖从 token 0 开始的公共前缀，这一非单调布局会丢失上一轮 input/model-output
边界的直接复用。controlled 的 TaskSpace/standard/TaskSpace 三次 right-side 运行中，两次 TaskSpace
cache hit 约 14.0%/9.9%，中间 standard 约 90.7%，已排除单纯 warm-up/运行顺序解释。

同时，当前 `taskspace-exact-payload-scan-event-v1` 在 core 层序列化的是
`ResponsesApiRequest`；最终 `build_chat_completions_body` 仍在下游。因此它可以继续作为
projection 唯一性/禁用 marker scanner，但不能再被当作最终 Chat wire message layout 证明。
实际 wire role/hash/LCP trace 纳入 R5-G0。

该结论不回退 E4：重复 projection 污染确已修复，E4 仍完成；但 E4 不能再被表述为缓存问题已解决。

### 6.2 复杂样本 map 坍缩

L3 `subscription-billing-repair` 暴露了独立的 map 结构问题：Agent 通过四次 `update_plan`
维护 6 阶段语义计划，但 TaskSpace 最终只有一个 runtime mechanical init 创建的
`Blank inspect node`。35 次模型请求和 36 个普通工具结果中的 read/edit/test 全部归入 node-1，
task objective 最终仍是 `Agent-authored objective pending`。

根因不是 Agent 没有拆解，而是状态面分裂：通用 `update_plan` 构成 map 旁路；compact
`taskspace_control` 没有原子填充 blank task objective/map/nodes 的动作；已绑定的 blank inspect
node 又允许普通工作持续推进。修复归入 R5-G2，原则是提供 Agent-authored 的机械批量 map
初始化并消除双状态源，不让 runtime 从计划文本推断节点或自动拆任务。

G2 后续已在 commit `137766c`、`006e9c5`、`17adc5b` 收敛：TaskSpace provider 隐藏
`update_plan`，runtime 创建零节点机械空 map，Agent 用 `initialize_map` 原子提交图；
`finish_node.node_id` 可机械解析当前绑定；旧 `start_task/route_task` 从 Agent schema、handler
和 action-contract 移除，不做兼容。复验 `20260710-074551-730` 最终只有 1 个 map、4 nodes、
3 edges，24 requests 后 Agent complete，未再出现 final rejection loop。该复验不改变 Phase E
收益结论，只关闭后续 G2；34.2MB rollout 再次触发 extractor 32MB skip，仍归 G3。

## 7. Phase E 收益判定

| 假设 | 结果 | 证据 |
|---|---|---|
| 正常执行不再被 profile 截断 | PASS | R5 超过旧 6-request profile 后继续到 13 并完成；hard-stop 标记 0 |
| 中断不再伪装成 Agent completion | PASS | completion/interruption/validation 独立字段和 harness fixtures 通过 |
| 不通过新语义策略控制成本 | PASS | forbidden marker 0；action-contract/control/tool feedback 活动路径无语义合成 |
| feedback 拒绝能进入下一轮上下文 | PASS | final/gate feedback pair focused tests；E4 composer 回归证明当前 call/output 不被 stale projection replacement 误删 |
| stale projection 累积不再放大成本 | PASS | 9/9 projection 唯一；input -62.7%，requests -30.8%，wall -49.7%（相对污染样本）；不代表 cache prefix 已修复 |
| 简单任务达到 standard 成本 parity | 未证明，非 Phase E 退出门禁 | 单样本 wall ratio 1.40、input ratio 1.56；进入 Phase G 聚合验证 |

## 8. 遗留项

1. Phase F 删除旧 semantic normalizer、validation/rework policy、context compiler、action-contract 和相关历史测试，并拆分 map/event/gate/projection 模块；恢复新边界下的全量绿色基线，不做兼容。
2. Phase G2 已完成：mechanical blank map、`update_plan` 旁路、compact map 初始化能力和旧 `start_task/route_task` 双协议均已收敛。
3. Phase G3 继续用另一个复杂依赖样本验证 Agent-authored map，并收敛剩余 request cadence 放大。
4. complex sample 公共 validator 的 Miniconda pytest 缺失属于 harness 环境残余；hidden oracle 与 Agent lifecycle 必须分项记录。
5. 本次尚未执行对抗性审查；执行前不把它记为已完成 gate。

## 9. 退出决定

Phase E 的 E0-E5 和后续 G0/G1 退出门禁均已通过，下一步为 `F -> G3/H`。详细证据见
`docs/v0.0.5/build-R5/11-r5-feedback-cache-priority-plan.md`。禁止因剩余请求轮数差距或旧测试失败
恢复 route/profile hard stop、node-kind 策略 gate、semantic summary 或 runtime 自动 rework；反馈层
只补机械事实，缓存层只保留单 epoch snapshot 与自然状态工具历史。
