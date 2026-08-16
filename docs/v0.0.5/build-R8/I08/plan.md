# R8-I08 Input 成本根因定位计划

- Status: In Progress / IC-04 complete
- Created: 2026-08-17
- Product Authority: [`../taskspace-exec/00-product-contract.md`](../taskspace-exec/00-product-contract.md#confirmed-product-decisions)
- Applicable Decisions: PD1、PD2、PD3、PD4、PD6、PD7、PD9、PD10
- Global Engineering Constraints: [`../02-r8-global-constraints.md`](../02-r8-global-constraints.md)
- Issue: R8-I08
- Paid Budget Authorization: `R8-I08-INPUT-COST-CNY3-20260817`
- Paid Whale Agent runs executed by this plan: 0

## 1. 目标

定位当前 TaskSpace `input_tokens` 高于 Standard 的直接来源，并把总差值拆成可复算、互斥的工程组成。测试先回答“成本在哪里”，
不在根因未坐实前压缩提示词、Tool schema、Map、反馈或历史。

本计划区分三个容易混淆的量：

1. **Input 体量**：Provider 实际报告的 `input_tokens`；这是本专题的主指标。
2. **缓存计费结构**：`cached/uncached input_tokens`；它影响费用，但不改变 input 总量。
3. **请求次数**：每增加一次请求，当前全部上下文都会再次计入 input；它既是独立成本来源，也会放大每请求固定内容和历史累积。

## 2. 当前证据与未知项

### 2.1 已知事实

- 最新 `single-file-fast-fix × map-request × repeat=5` 共 34 requests、528,450 input，平均 6.8 requests 和
  105,690 input/run；Request 2+ 加权缓存为 92.07%。
- 一条 7-request 成功 trace 的 Provider payload 从 52,862 bytes 增长到 70,033 bytes；其中当前观察器报告：
  - `tools` 固定为 26,688 bytes/request；
  - `system_messages` 固定为 5,049 bytes/request；
  - `natural_history` 从 729 增长到 17,878 bytes；
  - `other_payload` 约为 20,376～20,398 bytes/request；
  - `base_instructions_identity.message_bytes` 为 20,019 bytes。
- `other_payload` 包含 Responses 顶层 `instructions` 的可能性很高；`base_instructions_identity` 是该字段的身份观察，不能在未核对前
  把 20,019 bytes 再加一次。
- 当前 `natural_history` 把 Assistant、outer `taskspace_exec` call/output、Map read 结果和 nested Tool result 混在一起；
  `ordinary_tool_feedback=0` 不能证明没有 Tool 反馈成本。
- TaskSpace Base 源文件比 Standard Base 源文件小约 950 bytes，因此“TaskSpace Base 本身更长”不是当前证据支持的先验根因。
- 历史 SC-01 已证明删除没有实际消费价值的完整 outer result TypeScript 展开，可将 Tool wire 每请求减少 4,749 bytes，
  同为 7 requests 时 input 降低 10.83%；继续删除高价值示例或字段说明曾产生行为风险，不能直接复用为本轮结论。

### 2.2 尚未证明

1. 当前 Standard 与 TaskSpace 在同一能力集合下的 Tool schema 实际差值。
2. `taskspace_exec` 外层容器是否让原生 Tool 合同发生结构性重复，而不是单次迁移。
3. TaskSpace outer result 是否把 nested Tool 原始结果复制成两份自然历史。
4. `read_map`、Map projection 或 Map 反馈在累计历史中的真实面积。
5. 请求次数放大与单请求体量放大各解释多少 Provider input 差值。
6. JSON、waiting、Provider-hosted 等异常请求剔除后，正常 TaskSpace 工作流的不可约成本。

## 3. 根因假设

| ID | 假设 | 可观察预测 | 证伪条件 |
|---|---|---|---|
| H1 | 主要差值来自请求次数放大 | TaskSpace 每请求体量接近 Standard，但多出的请求重复携带完整上下文 | 请求数对齐后仍有显著 input 差值 |
| H2 | `taskspace_exec` 固定 Tool wire 额外过大 | 第一请求和所有请求的 `tools` bytes 显著高于 Standard；差值可定位到 TaskSpace 外层序列/metadata，而非原生 Tool 合同 | 同能力集合下 TaskSpace tools 与 Standard 相近，或差值主要来自历史 |
| H3 | outer call/result 使自然历史重复保存 Tool 事实 | 等价一次 Tool 动作在 TaskSpace 历史增量明显大于 Standard，且同一 nested result body/identity 被多个结构承载 | 每个原生结果只出现一次，额外历史只来自必要 Map/序列 metadata |
| H4 | Map read/projection 是主要增量 | Map 内容在多个请求中占据显著累计面积，或同一 revision 被无必要重复携带/读取 | `map-request` 中 Map 相关 bytes 很小且无重复 revision |
| H5 | 固定协议在 Base、developer/system message 和 Tool description 中重复 | 三层中可证明存在同一操作合同或 schema 的重复承载，且 bytes 占比显著 | Base 只保留宏观工作模型，Tool 是唯一具体合同，system/developer 不复制协议 |
| H6 | 异常拒绝和恢复放大总 input | 高成本 run 的额外请求与 syntax/waiting/protocol reject 一一对应，干净 run 明显下降 | 无拒绝 run 仍稳定维持相同放大比例 |
| H7 | Provider usage 或观察器归因错误 | payload bytes、request identity 或 terminal usage 无法逐请求对账 | 每个请求 identity 唯一、section bytes 精确闭合、Provider usage 与 rollout 一致 |

这些假设允许同时成立。测试必须给出每项贡献边界，不以“找到一个问题”提前停止其他主要差值的核算。

## 4. 测量模型

### 4.1 Provider 精确指标

每个真实请求只使用 Provider terminal usage 作为 token 权威值：

```text
input_total = sum(request.input_tokens)
cached_total = sum(request.cached_input_tokens)
uncached_total = input_total - cached_total
```

section token 不能由 bytes/4 冒充精确 Provider token。结构分析使用精确 JSON bytes；只有单变量真实 A/B 的 Provider usage 差值，
才能升级为该变量的精确 token 收益证据。

### 4.2 Input 放大拆分

报告至少同时给出：

```text
request_amplification = taskspace_requests / standard_requests
per_request_input_amplification = taskspace_input_mean / standard_input_mean
total_input_amplification = taskspace_input_total / standard_input_total
```

并把 wire bytes 按请求累计为“面积”：

```text
section_area(kind) = sum(request.section_bytes[kind])
```

固定内容即使每请求逐字相同，也必须计入 input 面积；缓存只改变该面积的计费单价，不把它从 input 中删除。

### 4.3 归因边界

- 请求数差值是行为事实，不自动解释为 Runtime 缺陷。
- bytes 是结构体量证据，不自动等价于 token 数。
- 相似文本审计只能发现候选重复；删除任何语义内容必须另行证明对正确性无负收益。
- Standard 与 TaskSpace 必须使用同一 commit、模型、Skills、样本、容器和有效能力集合。

## 5. 测试工作单元

| ID | 目标 | 变更位置/对象 | 单一动作 | 产出与收益 | 副作用 | 验证 | 停止条件 | 状态 |
|---|---|---|---|---|---|---|---|---|
| IC-01 | 修正 section 成本口径 | `provider_wire_sections.rs`、wire trace fixture | 将 Responses `instructions` 从 `other_payload` 独立为 `base_instructions`；保持每个 payload byte 只归属一次 | 消除 Base 重复相加风险，得到真实固定前缀面积 | 仅观察 schema 增加一个 section kind；Provider payload 零变化 | Chat/Responses fixture；各 section bytes 之和逐字等于 payload bytes；缓存门禁 source 检查 | 需要保存原始提示词内容或改变 payload 时停止 | complete；见 [`01-ic01-section-attribution-result.md`](01-ic01-section-attribution-result.md) |
| IC-02 | 拆分自然历史结构 | 同一 observer、`ResponseItem` 结构分类 | 按结构类型拆分 user、assistant、client call/output、`taskspace_exec` call/output、Provider-hosted item 和 projection；只记录 count/bytes/hash | 判断 outer 历史、Map 反馈和普通 Tool 结果各占多少，不读取 reasoning 语义 | 增加结构化观测字段，不增加 Agent context 或 Runtime 决策 | 合成 mixed history 每 item 恰好归类一次；原文不进入 trace；总 bytes 闭合 | 需要启发式解析自然语言或记录敏感 body 时停止 | complete；见 [`02-ic02-history-breakdown-result.md`](02-ic02-history-breakdown-result.md) |
| IC-03 | 拆分 Tool declaration | TaskSpace Catalog/final-wire fixture | 结构化统计顶层每个 Tool、`taskspace_exec` 外层序列、Map schema、TaskSpace metadata、原生 client Tool 合同和 transport wrapper bytes | 判断 26,688 bytes 中哪些是 Standard 共用合同、哪些是 TaskSpace 独有成本 | 只读 analyzer；不改变 schema 序列化 | 分项之和等于 `tools` section；同一原生 ToolSpec identity 只统计一次 | 需要改 schema 才能测量时停止 | complete；见 [`03-ic03-tool-breakdown-result.md`](03-ic03-tool-breakdown-result.md) |
| IC-04 | 建立免费静态线材对照 | final-wire payload builder fixtures | 用生产请求构造路径捕获 Standard/TaskSpace 首请求；多轮历史改由 IC-05/IC-06 的真实 trace 承担，不手造迎合结论的 transcript | 隔离首请求固定 Tool/Base 面积，限定固定结构可解释的成本上界 | 现有两夹具的 system/history 不同，只能对 Tool 与总面积建立边界 | Provider payload bytes、section closure、Tool 子结构逐项表格 | 需要改写产品 payload 或接受无关旧快照时停止 | complete-with-scope；见 [`04-ic04-static-wire-result.md`](04-ic04-static-wire-result.md) |
| IC-05 | 离线复算最近真实 trace | 最新干净 repeat=5 和历史异常批次 | 用新 observer 能表达的旧字段先建立 v1 边界；新细分无法追溯时明确 unavailable，不伪造 | 量化固定面积、历史增长、异常请求面积，并挑选真实 A/B 的观察重点 | 旧 trace 无原始 payload，部分细分必然 unavailable | 5 轮逐请求表；干净/异常分层；Provider usage、request identity、section total 对账 | 需要从 hash 反推内容或用估计补缺时停止 | planned |
| IC-06 | 首轮真实双臂定位 | Docker benchmark；`single-file-fast-fix` | 当前 commit 下 Standard 与 `map-request` 各 repeat=1；不改协议 | 获得同版本总 input、请求数、每请求体量、section area 和动作路径 | 2 个付费 sample；模型随机性不足以给稳定频率 | 两臂业务/oracle 通过；无协议异常；能力 identity 一致；逐请求成本表 | 未获预算、任一业务/usage/identity 异常立即停止 | blocked-on-budget |
| IC-07 | 扩大简单样本置信度 | 与 IC-06 完全相同 | 仅在 IC-06 可比较且根因仍受随机性影响时扩到每臂累计 repeat=3 | 区分结构差值和单轮动作波动 | 最多再增加 4 个 sample；属于大规模运行，必须专项预算 | 总和/均值/中位数；逐 run 异常不被均值隐藏 | IC-06 已足够定位、出现新问题或预算未批准则不执行 | deferred |
| IC-08 | 复杂样本外推 | `subscription-billing-repair` 或届时最小充分复杂样本 | Standard 与 `map-request` 各 repeat=1，保持 IC-06 全部变量 | 判断固定成本结论是否被长历史、复杂 Map 或更多 Tool 结果改变 | 2 个付费 sample；不用于修改 Map 产品设计 | 同业务/oracle、section area、Map、请求路径和成本明细 | 简单样本根因未定位或复杂样本不具代表性时停止 | deferred |
| IC-09 | 单变量因果验证 | 由 IC-01～IC-08 选出的唯一主贡献项 | 每次只修改一个可准确命名的结构因素；先免费静态测量，再申请简单+复杂真实 A/B | 将“结构相关”升级为 Provider token 因果证据 | 可能改变缓存指纹或 Agent 行为；每个候选独立 commit 并可整体回退 | 业务、动作、request、input/cache/output/time/cost 全量比较 | 预测收益低于 5%、削弱语义、改变 Runtime 决策或出现行为回归即回退 | deferred |

## 6. 测试顺序

```text
IC-01 精确固定字段
  -> IC-02 历史结构
  -> IC-03 Tool declaration
  -> IC-04 免费 Standard/TaskSpace 静态对照
  -> IC-05 历史 trace 复算
  -> 汇报第一轮根因候选和真实预算
  -> IC-06 双臂 repeat=1
  -> 按证据决定 IC-07 / IC-08 / IC-09
```

不得先运行四臂或 repeat=10。`map-always`、`map-append` 会引入 projection policy 变量，本轮首先定位 Standard 与最省
projection 的 `map-request` 之间的基础差值；该差值解释清楚后，才允许把其他两种模式加入后续产品测量。

## 7. 首轮真实预算草案

IC-06 尚未获得授权，以下仅用于后续申请：

| 项目 | 上限 |
|---|---:|
| Model | `deepseek-v4-flash` |
| Sample | `single-file-fast-fix` |
| Arms | Standard 1 + map-request 1 |
| Repeat | 每臂 1；共 2 sample runs |
| Provider requests | 24 |
| Input tokens | 260,000 |
| Output tokens | 10,000 |
| 正常预期费用 | 约 CNY 0.03；按最近干净 TaskSpace 五轮均值保守假设两臂成本相同 |
| 费用观察停止线 | CNY 0.12；覆盖约 70% cache hit 下的 260K input + 10K output |
| 协议极端上限 | CNY 0.28；按全部 260K input 未命中 + 10K output 计算，实际授权必须覆盖或进一步收紧 token 上限 |
| 最长耗时 | 20 分钟 |
| 自动重试 | 0 |

如果任一 arm 出现业务失败、JSON/waiting/provider 异常、能力身份不一致、usage 缺失、缓存同形零命中或请求超限，本轮只作为异常
证据，不进入 input 根因比较，也不自动补跑。

价格快照采用 2026-08-17 最近真实运行账本中的 DeepSeek 官方价格：cached input `CNY 0.02/M`、uncached input
`CNY 1.00/M`、output `CNY 2.00/M`。运行前必须重新冻结当日价格；价格变化时只重算预算，不改变测试变量。

### 7.1 后续可选阶段预算边界

下表用于说明总风险，不代表提前授权。IC-07、IC-08 只有在前序证据要求时才分别申请；IC-09 的变量尚未知，不能预支预算。

| 阶段 | 新增 sample runs | Request 上限 | Input / Output 上限 | 正常预期 | 观察停止线 | 全部 input 未命中的协议极端上限 |
|---|---:|---:|---:|---:|---:|---:|
| IC-A 免费测量 | 0 | 0 | 0 / 0 | CNY 0 | CNY 0 | CNY 0 |
| IC-B 首轮双臂 | 2 | 24 | 260K / 10K | CNY 0.028 | CNY 0.12 | CNY 0.28 |
| IC-07 追加到每臂累计 repeat=3 | 4 | 48 | 520K / 20K | CNY 0.056 | CNY 0.24 | CNY 0.56 |
| IC-08 复杂样本双臂 | 2 | 30 | 600K / 30K | CNY 0.089 | CNY 0.25 | CNY 0.66 |
| **IC-A～IC-08 最大合计** | **8** | **102** | **1.38M / 60K** | **CNY 0.173** | **CNY 0.61** | **CNY 1.50** |

正常预期中，简单样本按最近干净 TaskSpace 五轮均值 `CNY 0.013969744/run` 并保守假设 Standard 同价；复杂样本按最近
`subscription-billing-repair` TaskSpace trace 以当前价格重算为约 `CNY 0.04452/run`，并同样保守假设 Standard 同价。
这些是容量估计，不是已观测的当前 Standard 成本。

### 7.2 已批准总包

用户于 2026-08-17 在收到第 7.1 节预算拆分后明确批准总包 CNY 3.00，并允许在根因修复后额外执行一轮测试。授权按下列
最大范围解释；阶段证据不要求执行的 run 不得为消耗预算而启动：

| 项目 | 授权上限 |
|---|---:|
| Model | `deepseek-v4-flash` |
| 基线与定位 | IC-B 2 runs；必要时 IC-07 4 runs；必要时 IC-08 2 runs |
| 额外修复复验 | 仅一个已坐实因素；简单样本 Standard 1 + map-request 1，共 2 runs |
| Sample runs | 10 |
| Provider requests | 126 |
| Input / Output | 1.64M / 70K |
| 费用硬上限 | CNY 3.00 |
| 预计正常费用 | 约 CNY 0.20 |
| 按当前 token 上限全部 input 未命中 | 约 CNY 1.78 |
| 最长累计耗时 | 120 分钟 |
| 自动重试 | 0 |

每个实际 run 仍须在启动前建立独立 `planned` 账本记录，结束后立即结算。IC-B 只在 IC-A 通过后激活；IC-07、IC-08
分别由前序证据决定；额外修复复验必须先有一个明确根因、一个单变量改动、免费测试和缓存敏感面门禁。业务、协议、usage、
能力身份或证据异常时立即停止当前阶段，不用总包余额自动补跑。

## 8. 根因定位通过条件

完成 IC-06 后，只有同时满足以下条件才称为“根因已定位”：

1. Provider 请求、terminal usage 和 payload section bytes 全部逐身份对账；没有未知请求或重复 usage。
2. 每个 Provider payload byte 只进入一个结构 section；`other_payload` 不得继续承载超过 payload 5% 的无法解释内容。
3. Standard/TaskSpace 的固定 Tool/Base 差值、累计历史差值、Map 相关差值和额外请求成本分别报告，不互相重复计算。
4. 至少 90% 的 wire byte 差值可落到明确结构；Provider token 差值只报告真实 usage，不用 bytes/4 替代。
5. 根因结论同时说明正常路径与异常请求的贡献，不用一次 reject 解释所有稳定成本。
6. 尚未通过单变量真实 A/B 的候选只能标为“结构相关”，不能写成已坐实 token 根因。

## 9. 非目标

- 本阶段不修改 TaskSpace Base、Tool schema、合法序列、Map 投影、状态机或反馈。
- 不把缓存命中率下降解释为 input 总量增加；两者分别报告。
- 不以删除协议、示例、Tool description 或 Map 全局信息作为默认优化方向。
- 不引入新的 tokenizer、监控服务、数据库、语义分析器或大模型评审器。
- 不运行 `map-always/map-append`，不执行 R8 最终四臂晋升。

## 10. Execution Contract

- Product Authority 中 active 的用户决策是本计划唯一产品权威；Agent 不得自行修改或重新解释。
- 工程证据可以修订本计划，不得静默改写 Product Authority。
- 新的产品行为选择必须 deferred、provisional 或经用户明确确认；本计划当前没有 provisional 产品选择。
- 每个 material phase 完成后只审计该阶段引入的 Product Decision Delta。
- 每个 material phase 开始前，根据当前代码、测试和证据重排剩余计划；Gate 为 `pending` 或
  `blocked-on-plan-approval` 时不得开始。
- material Plan Delta 必须记录并取得用户明确批准；Agent 不得自我批准。

## 11. Phase Gates

### Phase IC-A：免费测量可信性

- Units: IC-01～IC-05
- Pre-Phase Plan Rebase Gate: ready
- Material plan delta: none
- User approval: not-required
- Exit: section bytes 精确闭合；结构分类无原文；免费静态对照和历史 trace 复算完成。
- Product Decision Delta: engineering-only

### Phase IC-B：最小真实双臂

- Units: IC-06
- Pre-Phase Plan Rebase Gate: pending
- Material plan delta: pending IC-A result
- User approval: user-approved-budget-direct: R8-I08-INPUT-COST-CNY3-20260817；仍受 IC-A rebase gate 阻断
- Exit: 两臂可比较且逐 request 成本结构完整，或以明确异常停止。
- Product Decision Delta: engineering-only

### Phase IC-C：置信度、复杂样本与单变量验证

- Units: IC-07～IC-09，按 IC-B 证据选择，不默认全部执行
- Pre-Phase Plan Rebase Gate: pending
- Material plan delta: pending IC-B result
- User approval: paid matrix covered by R8-I08-INPUT-COST-CNY3-20260817；material plan change 和 cache-sensitive
  production change 仍须按各自门禁处理
- Exit: 主贡献项通过单变量因果验证，或候选被证伪并停止。
- Product Decision Delta: any behavior-affecting optimization requires separate user confirmation
