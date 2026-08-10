# VA-02 Source 与 Structured Carrier A/B 实验计划

- Created: 2026-08-10
- Status: 2-run Provider smoke completed / directional evidence only, selection pending
- Product Authority: [`00-product-contract.md`](00-product-contract.md)
- Active Plan: [`12-phase-b-zero-base-plan.md`](12-phase-b-zero-base-plan.md)
- Applicable Decisions: TaskSpace Exec 只负责合法序列与节点归属；Agent 声明动作；Runtime 机械预检和执行；Standard 零变化

## 1. 要回答的问题

VA-02 的当前 structured carrier 在合法的“完成前置节点 + 执行后续 patch”意图上，连续两次生成了同一份非法 JSON。
历史 source 候选曾被 structured carrier 替换，但旧实验同时混入了 `version`、`capability_id`、`item_id`、旧 Hosted
字段和不完整说明，不能用于回答 carrier 本身是否更适合 DeepSeek。

本实验只回答：**在 TaskSpace 语义、Tool 能力、Map、Runtime、样本和模型均相同的情况下，DeepSeek 使用哪一种
`taskspace_exec` carrier 更稳定、有效且成本合理。**

实验不预设 source 或 structured 胜出，也不把一次成功解释为稳定性结论。

## 2. 唯一实验变量

两臂必须解码为同一个当前 `TaskSpaceExecPlan`：

### Arm A: structured

```json
{
  "calls": [
    {"map":{"operation":"update_map","input":{"node_patches":[]}}},
    {"client":{"name":"apply_patch","node_id":"fix","input":"..."}}
  ]
}
```

Provider 通过 Function parameters 看到 `calls[]`、Map/client variants 和各 Tool 原生输入 schema。

### Arm B: source

```json
{
  "source": "taskspace.plan({\"calls\":[...]});"
}
```

`taskspace.plan(...)` 内必须是与 Arm A 逐值相同的当前 plan JSON。Provider 的 Function parameters 只约束
`source:string`；当前 plan schema 由同一 catalog 机械渲染到 Tool description，Runtime 解包后继续调用与 Arm A 相同的
decoder、preflight、Map transaction、dispatch、persistence 和 result builder。

Arm B 不是通用 JavaScript、不是 Codex V8 执行环境，也不允许变量、条件、循环或动态 Tool 调用。引入这些能力会同时改变
carrier 与执行模型，失去 A/B 可解释性。

## 3. 必须保持一致

| 面 | 控制要求 |
|---|---|
| commit 与生产代码 | 同一 commit；仅编译期实验 feature 选择 carrier |
| TaskSpace 语义 | 同一合法序列、节点绑定、Hosted 核对、单 Patch 和零副作用拒绝规则 |
| typed plan | 两臂成功解码后逐值相同 |
| Runtime | 共用同一 envelope、preflight、Map、Router、Store、结果和错误路径 |
| Agent 上下文 | 同一 base instructions、TaskSpace 协议、projection policy 和自然历史 |
| Tool 能力 | 同一原生 ToolSpec 快照、同一 Hosted 顶层能力、普通 Tool 顶层均不暴露 |
| 样本 | 同一冻结 Docker fixture；每次使用全新工作区 |
| 模型 | `deepseek-v4-flash`，相同模型参数和 Provider route |
| 限额 | 相同 request、token、时间和无自动重试规则 |

允许存在且必须计量的差异只有 carrier 必然带来的 wire 差异：Function parameters、carrier 专用语法说明、序列化字节和
对应的 input/cache 成本。

## 4. 离线实施与门禁

| ID | 工作 | 验证 | 停止条件 |
|---|---|---|---|
| AB-01 | 增加编译期实验 carrier 选择，默认仍为 structured | 默认 final wire 与当前 accepted fingerprint 相同；Standard exact equality | 需要运行时双协议、普通 Tool 改写或第二套 Runtime 时停止 |
| AB-02 | source 只解包 `taskspace.plan(<current JSON>);` | 空值、Markdown、动态 JS、尾随内容、超限、未知字段均 fail closed | decoder 不能复用当前 plan decoder 时停止 |
| AB-03 | 从同一 catalog 生成两臂声明 | Tool 名、描述、输入和 Map operation 集合逐值同源 | 需要手写第二份 Tool catalog 时停止 |
| AB-04 | 建立 typed-plan 同构矩阵 | 初始化+工作、work list、完成+patch、finish、read、Hosted、Freeform、Namespace 全部逐值相同 | 任一有效输入产生不同 plan/preflight 时停止 |
| AB-05 | 接入现有 Docker runner 的双 binary arm | 记录 binary SHA、feature、final-wire hash、fixture hash 和 ledger ID | 不能证明两臂只差 carrier 时不得真实运行 |
| AB-06 | 运行缓存敏感面门禁 | 默认 structured 和 Standard 通过；source 作为独立候选 fingerprint 记录，不晋升 accepted baseline | 默认 wire 漂移立即阻断 |

实验 feature 不构成产品模式，不写入用户配置、Session、Map 或持久化 schema。完成选型后删除未选 carrier 和 feature，不维护
长期双轨。

## 5. 真实配对实验

真实运行必须另行申请预算并先写 `benchmarks/whale-agent-run-ledger.json`。建议分两级：

1. **Smoke**：`single-file-fast-fix × structured/source × repeat=1`，共 2 个 sample run。任一基础设施、观测或非 carrier
   差异立即停止，不进入比较。
2. **Directional repeat**：Smoke 两臂均可运行后，`single-file-fast-fix × structured/source × repeat=3`，共 6 个
   sample run，采用交错顺序并分别计算每臂首请求与 request 2+ 缓存。

`repeat=3` 只提供方向性证据，不宣称统计显著。若两臂在简单样本上都稳定，再另行决定是否需要复杂样本；不得预先把复杂
样本预算并入本实验。

## 6. 观测指标

| 类别 | 指标 |
|---|---|
| 结果 | business success、公共测试、patch 正确性、最终 Map 是否显式闭合 |
| 结构 | 首次 outer call 合法率、decode reject、preflight reject、顶层 Tool escape、重复非法 payload |
| 路径 | 每 request 的 Map/client/Hosted 动作、单独生命周期请求、工具重复和 patch 数量 |
| Map | 节点/依赖/状态/actions、非法节点选择、Runtime 是否发生未授权推断 |
| 成本 | requests/attempts、input/cached/uncached/output tokens、request 2+ cache hit、费用、耗时 |
| carrier | Tool declaration bytes/tokens、arguments bytes、结构深度、解析错误位置与原始 payload hash |
| 反馈 | Tool 原始结果是否完整进入下一请求，错误语义是否被改写、丢失或重复 |

报告必须逐次列出原始值，同时给出总和、均值和中位数；bad case 不得静默剔除，只能按预先定义的基础设施无效条件单列。

## 7. 判定边界

1. Runtime 语义、Map 所有权、普通 Tool 原生路径或 Standard 出现差异，实验无效，不归因于 carrier。
2. 任一 arm 在结构错误后被 Runtime 修复、补全、重排或自动推进，实验无效。
3. source 的收益必须来自更少的结构/协议失败或更好的任务路径；仅靠隐藏内层 schema、放宽合同或增加重试不算收益。
4. structured 的收益必须体现为更强的实际稳定性或更低成本；“理论上 schema 更强”不能覆盖真实失败。
5. 最终选型由用户基于完整证据决定；实验脚本不得自动晋升、废弃或修改产品合同。

## 8. Execution Contract

- `00-product-contract.md` 仍是产品权威；本实验不自动改变 active carrier 决策。
- 本文件只定义受控验证，不建立第二份长期协议或产品模式。
- 工程证据可以更新实验计划和技术结论，不能自行改写产品权威。
- 每一阶段只审计该阶段是否改变产品语义；出现 material provisional/conflict 时停止并由用户确认。

## 9. 离线实施结果

2026-08-10 已完成 AB-01～AB-04，并确认 AB-05 不需要新 Runner：现有 Docker harness 已接受显式 `WhaleBin`，真实实验前
从同一 commit 分别构建默认 binary 和 `taskspace-exec-source-ab` feature binary，登记各自 SHA 与 attestation 即可。

| 检查 | Structured | Source |
|---|---:|---:|
| `cargo test -p codex-core taskspace_exec --lib` | 73 PASS | N/A |
| `cargo test -p codex-core taskspace_exec --lib --features taskspace-exec-source-ab` | N/A | 73 PASS；补充完整 outer example 后定向 2 PASS |
| 完整 CLI compile | 当前基线已有 | `cargo check -p codex-cli --bin whale --features taskspace-exec-source-ab` PASS |
| mixed Map transition + Freeform patch 同构 | PASS | PASS，解码结果逐值相同 |
| Function / Namespace / Hosted / canonical examples 同构 | PASS | PASS，解码结果逐值相同 |
| dynamic JS、旧字段、尾随语句、Markdown、错误 wrapper | 原 structured 合同拒绝 | source decoder 拒绝 |
| persisted Map + native Tool + rollout + provider preparation | PASS | PASS，使用相同生产链 |

实现只新增编译期实验 feature，不增加用户配置、Session 字段、Map 状态、Runtime 自动修复或长期双协议协商。默认 build
仍选择 structured。

AB-06 最终 staged cache-sensitive gate PASS，敏感面 SHA 为
`cfd2da6cc0a83b0c9105d1c931e92f65193473167a542ab273389b9e483ac55c`。Standard final-wire 前后均为
`3e6f1602ab3a1a2349420afafca95570b25ce7eca1faad07de1d7c649a956c08`。默认 TaskSpace final-wire 为
`deb51ae4615d3f9c5076d6713cae574c7c21b0473c9f510d928ee2115b8b7a49`，与本实验前已提交的
`2026-08-10-r8-b5-va02-e2e-revalidation.json` 相同；它相对更早 accepted snapshot `8e58e2...` 的既有差异继续保持发布
阻断，但本实验没有再次改变默认 wire。

## 10. 真实 Smoke 结果

2026-08-10 用户批准 `single-file-fast-fix × structured/source × repeat=1`。两臂从同一产品 commit
`2bae4f063b191679b0328d81e137d39e25419449` 构建，均使用 `deepseek-v4-flash`、`map-request`、右侧 TaskSpace、
最多 6 个 Provider 请求且不自动重试。账本记录为 `WAR-20260810-193648-R8-VA02-CARRIER-AB`。

最初运行目录包含 `taskspace/structured` 等处理标签，被中立路径门禁在 Provider 前拒绝；确认零 API 请求后改为
`target/r8-va02-ab/runs/a1|a2`。该次 pre-provider harness 尝试不是模型样本，也不计入两臂结果。

### 10.1 汇总

| 指标 | Structured | Source |
|---|---:|---:|
| binary SHA-256 | `f0a43b7f...bd6f6` | `0c9832ba...ad0e` |
| Provider 已完成请求 | 2 | 6 |
| 本地 logical request | 2 | 7（第 7 次被 hard limit 在 Provider 前阻断） |
| 任务文件是否正确修改 | 否 | 是，`src/tax_calc.py` |
| public / hidden validator | FAIL / FAIL | PASS / PASS |
| Map | revision 3；root active；work 未闭合 | revision 13；root/work/finish 全部 completed |
| Agent 最终状态 | interrupted | interrupted（缺最终自然语言总结） |
| harness business success | false | false |
| Agent wall time | 5,006 ms | 19,146 ms |
| sample 总时长 | 9,224 ms | 23,456 ms |
| input / cached / uncached | 29,370 / 28,416 / 954 | 117,196 / 92,544 / 24,652 |
| output | 423 | 1,785 |
| 全请求 cache hit | 96.75% | 78.97% |
| request 2+ cache hit | 94.31% | 88.71% |
| 稳态参考 | 仅 request 2：94.31% | request 3+：93.82% |
| Tool section 首请求 | 30,627 bytes / 7,657 estimated tokens | 57,882 bytes / 14,471 estimated tokens |
| 按冻结单价估算成本 | USD 0.0003316 | USD 0.0042102 |

两臂合计 8 个 Provider 请求、146,566 input、120,960 cached、25,606 uncached、2,208 output，估算成本
USD 0.0045418，未超过批准上限。

Source 的首请求与 request 2 缓存率受到新 Tool shape 首次出现影响，而 Structured shape 在本次之前已有历史运行，不能把
78.97% 与 96.75% 直接解释为稳态 carrier 差异。Source request 3+ 为 93.82%，与 Structured request 2 的 94.31% 接近。
但是 Source 将完整 inner schema 作为 description 字符串传输，JSON 转义使首请求 Tool section 实际增加 27,255 bytes；这是
当前 Source arm 的明确静态 wire 成本，不能用缓存冷启动完全解释。

### 10.2 逐请求路径

| Arm | Request | Agent 动作 | Runtime / 结果 | Input | Cached | Output |
|---|---:|---|---|---:|---:|---:|
| Structured | 1 | `initialize_map + exec_command(ls/find)` | 合法；Map 初始化和 inspect action 成功 | 14,305 | 14,208 | 292 |
| Structured | 2 | 顶层直接调用 `exec_command` | 响应合同拒绝；该普通 Tool 不在顶层声明中，零执行 | 15,065 | 14,208 | 131 |
| Source | 1 | `taskspace.plan({...`，遗漏结尾 `);` | source envelope 忠实拒绝；Map 未创建 | 18,067 | 4,608 | 250 |
| Source | 2 | 修正为 `taskspace.plan({...});`，初始化 + ls/find | 合法；Map 和 inspect action 成功 | 18,361 | 12,160 | 262 |
| Source | 3 | 读取 README、源码和测试 | 合法；读取结果完整返回 | 19,089 | 17,920 | 169 |
| Source | 4 | 完成 inspect + patch fix | 合法；patch 成功 | 19,929 | 18,176 | 565 |
| Source | 5 | 在 fix 未完成时直接执行 waiting verify，并预声明完成/finish | preflight 忠实拒绝，零副作用 | 20,738 | 19,840 | 233 |
| Source | 6 | 完成 fix + 执行 verify + 完成 verify + `finish_map` | 合法；pytest 3 PASS；Map 全闭合 | 21,012 | 19,840 | 306 |
| Source | 7 | 读取 request 6 结果后准备最终响应 | Provider hard limit 前阻断，无 API/usage | N/A | N/A | N/A |

### 10.3 当前结论

1. 本轮不是“两臂都失败所以没有结论”。Structured 在第 2 请求发生顶层 Tool escape，未读取任务内容也未修改文件；Source
   虽有一次 wrapper 语法错误和一次 DAG 顺序错误，但均收到准确反馈并自行纠正，最终完成代码、验证和 Map 闭合。
2. Source 对本样本表现出明显更好的实际可用性；Structured 的 schema 约束没有阻止模型在后续请求生成未声明的顶层普通
   Tool call。Runtime 的拒绝符合产品合同，没有代替 Agent 修复或执行。
3. Source 当前仍不能直接晋升：repeat 1 不证明稳定性；首请求仍漏写 wrapper 结尾；完整 schema 嵌入 description 带来明显
   静态 wire 膨胀；第 5 请求仍出现可由 Agent 纠正的 DAG 顺序错误。
4. 两臂的 `business_success=false` 原因不同。Structured 没有完成业务工作；Source 已完成业务结果和 Map，只因 6-request
   上限不允许第 7 次最终自然语言总结而被 lifecycle gate 标记 interrupted。
5. 现有 performance observer 只按 Structured outer arguments 解析 `taskspace_exec`，因此把全部 Source calls 误报为
   `exec_arguments_invalid`/`exec_result_invalid`。真实判断以 canonical request facts、原始 rollout、Runtime event 和持久化
   Map 为准；该派生报告不参与本轮 carrier 结论。

本轮只提供方向性证据，不自动选择 carrier。是否执行两臂各 repeat 3、是否先优化 Source wire/observer，均需新的用户决策和
独立预算。
