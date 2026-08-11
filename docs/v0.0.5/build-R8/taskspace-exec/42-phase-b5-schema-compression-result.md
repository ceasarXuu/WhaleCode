# Phase B5 TaskSpace Exec 单变量压缩记录

- Status: Complete / SC-01 retained / SC-02 and SC-03 rejected
- Budget package: `R8-EXEC-SCHEMA-USD100-20260811`
- Subject commit: `95579125c805be645a64b5f938a63bcba647f177`

## 1. 执行边界

本专题只压缩 `taskspace_exec` 的模型可见固定合同。每轮只改变一个因素，先通过离线合同、final-wire 和缓存敏感面门禁，再运行
`single-file-fast-fix × map-request × repeat=1`。业务、结构、usage 或成本证据不完整时不得叠加下一变化。

## 2. SC-01：移除完整 outer result TypeScript 展开

唯一变化是取消 Tool description 中完整 `TaskSpaceExecResult` TypeScript 合同，改为一条机械反馈保证。以下能力保持不变：

- 内部 typed result 与 capability identity；
- Agent 实际收到的完整 JSON、原生 client 结果及错误；
- 输入 schema、Map 操作、序列、node binding、Hosted 对账；
- Router、Runtime、preflight、自愈、协议示例和 Standard 路径。

离线验证：TaskSpace Exec `82/82`、production final-wire fixture、Standard final-wire 和缓存敏感面候选检查通过。生产 wire 的 Tool
段由上一有效 7-request 基线的 `30,522 bytes/request` 降至 `25,773 bytes/request`，减少 `4,749 bytes`（`15.56%`）。

| Run | 结果 | Requests | Input | Cached | Uncached | Output | Req 2+ hit | Agent wall | 估算费用 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 旧合同 R03 | PASS | 7 | 114,793 | 107,904 | 6,889 | 2,096 | 93.33% | 19.3s | $0.001853 |
| SC-01 首轮 | PASS | 7 | 102,358 | 87,808 | 14,550 | 1,989 | 92.59% | 18.5s | $0.002840 |
| SC-01 暖缓存 | PASS | 8 | 120,306 | 113,792 | 6,514 | 2,728 | 94.04% | 23.2s | $0.001994 |

同为 7 requests 时，SC-01 input 减少 `12,435`（`10.83%`），首请求从 `13,850` 降至 `12,500`。首轮费用上升来自新 Tool
前缀第一次出现的冷缓存，不是 input 反弹；第二轮暖缓存已恢复到 94% 以上且费用低于旧 R03。

两轮都完成相同 5-node 线性 Map、一个最终 patch、公开测试和隐藏 oracle。没有 syntax、wrapper、顶层逃逸、结果缺失或 usage 缺口。
暖缓存轮的两次 preflight reject 均为 Agent 过早选择 waiting 子节点，Runtime 忠实返回未完成父节点后下一请求纠正；该既有行为与删除结果合同无
语义关联。

结论：`SC-01` 保留并作为下一因素的实验基线。两轮有效运行累计估算 `$0.0048342`，USD 1.00 包剩余约 `$0.9951658`。

## 3. 门禁边界

现有基线晋升器只允许自动替换 Standard semantic snapshot，`taskspace_production_tool_wire` 尚不在其受保护替换集合中，因此本轮没有为了
推进实验而扩大门禁策略。下一变量使用本轮 gate report 保存的完整 SC-01 candidate payload 做直接静态差分，正式缓存门禁仍保持发布阻断，最终
候选确定后再单独处理基线晋升范围。

启动期间有两个 Provider-zero 事件：第一次相对 `RunRoot` 在账本创建前失败；第二次证据目录名含 `taskspace`，触发 neutral-cwd 门禁并以
`provider_boundary_requests_minimum=0` 结算。二者均未消耗 API token，不计入有效样本和预算费用。

## 4. SC-02：协议示例去重

`SC-02` 只处理协议示例重复：保留首次 `initialize_map + client work` 与父节点完成后交接两个高价值示例，评估删除可由输入 schema 和硬规则直接
表达的 `read_map`、最终 finish 两个示例。不得同时修改协议规则、schema、Map、Runtime 或结果反馈。

静态 candidate 只减少 `439 bytes/request`，约 `110 token/request`，不足当前简单样本单请求 input 的 1%。其中最终示例是历史上用于表达
`update_map + finish_map` 合批的直接证据，移除后的潜在请求放大或单独 finish 回归会轻易抵消全部收益。该候选在付费运行前按风险收益比证伪，代码已
恢复到 SC-01 基线，预算消耗为零。

## 5. 下一因素

`SC-03` 只移除 11 个 client variant 中重复的 `node_id` 字段描述。字段仍是 required string，outer 协议仍保留唯一 ownership 说明；其他
schema、协议、Map 和 Runtime 均未改变。静态 candidate 减少 `473 bytes/request`，约 `118 token/request`。

在线运行在 2 个 Provider 请求后业务失败：首请求正确生成 `initialize_map + exec_command`，`node_id=inspect` 绑定和执行均成功；第二请求把
`exec_command` 提升为非法顶层 Tool，未继续修改代码。该失败不是漏填 `node_id`，单次证据不足以证明字段描述删除与顶层逃逸存在因果；但相对于
不足 0.5% 的 wire 收益，结构失败使继续复验的风险收益不成立。commit `a07dfd11e` 已由 `a58666eb1` 整体回退，不保留部分实现。

本轮 SC-03 使用 2 requests、25,382 input / 17,024 cached / 8,358 uncached / 363 output，估算 `$0.0013194`。压缩预算包累计
消耗 `$0.0061536`，剩余约 `$0.9938464`。

## 6. 收口结论

当前可证明的高价值、低风险压缩只有 SC-01。继续减少固定 Tool wire 主要只有四条路径，均不进入实现：

1. 用 `$defs`/`$ref` 或条件 schema 合并 `calls[].anyOf`：会改变 Provider schema 兼容面和模型对每个 native input 的直接可见性；
2. 删除 client Tool 原生 description/input schema：TaskSpace 顶层不再暴露这些 Tool，会直接丢失能力合同；
3. 按任务语义隐藏 `spawn_agent` 等大 Tool：需要 Runtime 代替 Agent 判断能力需求；
4. 继续删初始化、handoff 或 finish 规则与示例：历史失败已经证明它们承担真实行为约束，节省量不足以覆盖请求放大风险。

因此本轮停止继续付费迭代，不为用完预算制造新候选。最终生产代码为 SC-01；缓存正式基线仍按第 3 节保持发布阻断，后续应作为门禁能力范围的独立
主题处理，不和 Tool 合同压缩混做。

## 7. Waiting 批次合同清晰度修复

2026-08-12 针对最新暖缓存轮的两次 waiting 拒绝完成最小合同修复，不改变 DAG、状态迁移、preflight 原子性、client 原生并行或
Standard 路径：

1. `calls` 直接说明只有排在前面的 Map 操作会改变后续调用可执行性；client outcome 不改变节点状态，也不能在同批解锁后代；
2. handoff 示例明确为父节点完成后执行 direct-child，不再使用含糊的 dependent-node 标题；
3. state schema 明确 `waiting/ready` 由 parents 派生，不能绕过依赖；保留全部既有合法生命周期能力；
4. waiting 拒绝除准确列出未完成直接父节点外，返回同一机械批次边界，不规定 Agent 应选择什么工作。

TaskSpace Exec `82/82` 通过。同一 production final-wire fixture 的紧凑 Tool JSON 从 `29,578` 降到 `29,263 bytes`，减少
`315 bytes`（`1.06%`）；其中 description 减少 `309 bytes`。因此没有以固定上下文膨胀换取显著性。本轮未运行真实 Whale
Agent，不能宣称 waiting 触发频率已经下降；I04 保持验证态。

## 8. Waiting 批次合同在线复验

本轮授权范围为 `deepseek-v4-flash`、`single-file-fast-fix × map-request × repeat=2`，总计最多 20 requests、250,000 input、
8,000 output、USD 0.05，禁止自动重试。正式运行前发生两个 Provider-zero harness 事件：首次构建后的二进制 attestation
未更新；补齐后，runner 因相对 `run-root` 无法转换为仓库相对路径而在账本 claim 前退出。两者都没有启动 Agent 或消耗 API
预算，保留原始证据但不计入有效 sample。

首个有效 sample 完成业务、公开测试、隐藏 oracle 和 5 节点 / 4 边 Map：

| 结果 | Requests | Input | Cached | Uncached | Output | Req 2+ hit | Agent wall | 估算费用 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| PASS | 9 | 139,221 | 116,992 | 22,229 | 2,759 | 88.95% | 26.98s | $0.0042121576 |

动作路径为：初始化 Map 并查看目录；读取需求、测试和实现；尝试 `inspect completed + patch fix + test verify`。第三步因
`verify` 仍依赖未完成的 `fix` 被整批零副作用拒绝。Agent 随后准确复述“client Tool outcome 不改变节点状态，也不能在同批
解锁后代”，说明新增批次边界已进入上下文并被理解；但它没有重放同一拒绝批次中未提交的 `inspect completed`，直接尝试把仍为
waiting 的 `fix` 改为 `in_flight`，触发一次 `TransitionInvalid`。读取 Map 后，Agent 依次完成 `inspect + patch`、
`fix completed + pytest`、`verify completed + finish_map`，最终成功。

与同一提交前最近一次暖缓存 run 相比，请求从 8 增至 9，input 从 120,306 增至 139,221，request 2+ cache hit 从 94.04%
降至 88.95%；两次运行均有 2 次 preflight/state 失败，只是本轮由“两次 waiting”变为“一次 waiting + 一次未重放原子回滚的
状态迁移”。单样本存在随机性，不能据此归因成本回归，但也不能宣称本次文字修复已降低请求数。

首轮 input 超过提案的 125,000 单轮观察阈值，专用 runner 按 `after_any_budget_observation_exceeded` 停止，第二轮未执行。
全局账本记录为 `WAR-20260812-042203-CACHE-REGRESSION-D7FE72F8`，状态 `partial`，usage 完整。I04 继续保持 verifying；后续若
再调整，应优先检查拒绝反馈对“整批 Map 也未提交”的显著性，不增加 Runtime 选点、自动重放或状态推断。
