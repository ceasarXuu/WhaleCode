# R8-I07 性能观测可信性修复计划

- Created: 2026-08-05
- Status: Planned
- Risk depth: Full
- Scope: request identity、usage 聚合、Provider 边界对账、证据可用性与可复算性
- Global issue: [`../01-r8-known-issues.md`](../01-r8-known-issues.md) 中的 `R8-I07`
- TaskSpace Exec mapping: `TX-00`（独立前置）与 `TX-11`（新协议接入）
- Changes issue state: No

## 1. 产品问题

性能观察工具当前可能把同一请求计算两次，也可能把“本地已经准备或尝试发送”误写成“上游已经收到”。因此报告中的
请求数、token、缓存命中和失败归因可能彼此矛盾，团队无法可靠判断一次修复到底提高还是降低了产品表现。

I07 的目标不是让报表尽量给出一个数字，而是让每个数字都能回答三个问题：

1. 统计对象是什么：本地 Agent 轮次、采样尝试、实际越过 Provider 边界的请求，还是成功完成的响应；
2. 凭什么认定：唯一身份和原始事件分别是什么；
3. 证据不足时发生什么：明确报告 unavailable/incomparable，而不是按事件条数猜测。

## 2. 已确认事实与根因

### 2.1 同一 usage 被统计两次

证据 `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002` 中，Provider boundary 与 final wire 均证明真实完成 8 次
请求，但 Harness 报告 15 次。前 7 次请求各产生两条 `TokenCount`：

- response-completed 路径发送一条带完整 `provider_request_id`、`provider_logical_request_id` 和
  `provider_attempt_seq` 的请求完成事件；
- `Session::update_rate_limits()` 随后发送一条不带请求身份、但再次携带同一 `last_token_usage` 的状态快照；
- `New-TaskspaceRolloutRequestTraceSummary` 只判断 `token_count + last_token_usage`，将两者都当成新请求并累加 usage。

根因不是 Runtime 多发了一次 Provider 请求，而是同一种事件外壳承载了“请求完成事实”和“当前累计状态快照”，消费方
忽略身份与语义，按事件条数聚合。删除无 ID 状态事件会损害 UI/rate-limit 状态更新，正确修复点是消费语义。

### 2.2 “已构造请求”被误认为“上游已收到请求”

证据 `WAR-20260801-222316-R8-I01-W9-MA-1B64DB37` 中，Provider supervisor 实际接收 10 次请求，而
`provider-wire-trace.jsonl` 有 11 个 `payload_captured`。第 11 个事件随后以 `response_failed` 结束，没有越过受监督的
Provider 边界。

源码路径证明：`ProviderWireTrace::record_request()` 位于请求体构造和预算检查之后、`client.stream_request()` 之前。
所以 `payload_captured` 只证明一次本地 sampling/dispatch attempt 已形成，不证明 Provider 边界已经收到请求。当前
`verify_provider_boundary.py` 却要求 boundary payload hash 列表与全部 `payload_captured` 严格相等，因而把合法的本地
失败判成 upstream mismatch。

### 2.3 相同歧义已经扩散到多个消费者

当前至少存在以下消费方式，不能只修一个汇总函数后宣称 I07 完成：

| 消费方 | 当前口径 | 风险 |
|---|---|---|
| `cost-instrumentation.ps1` rollout summary | 每条带 usage 的 `TokenCount` 都是请求 | 请求与 token 双计 |
| `verify_provider_boundary.py` | 全部 `payload_captured` 必须等于 boundary requests | 本地失败被误判为上游不一致 |
| `performance-observation.ps1` | fallback 按 `payload_captured` 计 `provider_requests` | 把尝试数命名为真实请求数 |
| `r7-five-layer-evidence-freshness.ps1` | 按 `payload_captured` 汇总 Provider 请求 | 旧结果可被错误判新鲜或错误判不一致 |
| `provider-section-cost.ps1` | payload shape 与 terminal usage 合并后仍称 provider request | 结构分析分母和成本分母可能混用 |
| cache regression parser | 只接受所有 payload attempt 均成功完成 | 作为缓存基线门禁可能合理，但不能被复用为通用请求事实口径；需由 W1 明确职责 |

这张表是实施前的已知影响面下限。I07-W1 必须完成全量静态 inventory，不能假设本表已经穷尽。

## 3. 设计原则

1. **事实分层，不互相冒充。** 本地动作、Provider 尝试、边界接收、响应完成和状态快照是不同事实。
2. **身份优先于顺序和数量。** 聚合和对账以稳定 ID/哈希集合为主，不能用“第 N 条”猜配两个来源。
3. **usage 只归属于完成响应。** 没有完整请求身份或没有完成响应的状态快照不得产生 token/cost 记录。
4. **状态事件继续保留。** I07 不删除 Runtime/UI 需要的 `TokenCount` 或 rate-limit 广播，只限制统计消费者的解释。
5. **忠实暴露失败。** 本地拒绝不伪装成 Provider 失败；Provider 失败也不因没有 usage 而消失。
6. **缺证据时 fail closed。** 缺失、部分身份、同 ID 冲突、来源过期或无法关联时输出明确可用性，不回退成估算值。
7. **一套原始事实，多种派生视图。** 不新增平行 observer 服务；共享 parser/identity classifier 供成本、缓存和报告复用。
8. **Standard 与 TaskSpace 使用同一请求事实合同。** TaskSpace Exec 只新增内部 item/node 关联，不另造 Provider 计数规则。
9. **观测不反向控制 Agent。** 日志和报表只记录、校验机械事实，不根据失败类别给 Agent 注入建议或改变语义动作。

## 4. 权威身份和事件模型

### 4.1 四类不可混同的计数

| 产品事实 | 建议字段/身份 | 成立条件 | 可用于什么 |
|---|---|---|---|
| Agent model turn | `logical_request_id` | Runtime 开始一次逻辑采样，重试共享该 ID | 行动路径、重试分析 |
| Local provider attempt | `provider_request_id = logical + attempt_seq` | dispatch admission 已通过并捕获 Provider payload，尚未证明 transport 已发送 | payload shape、前缀与本地失败诊断 |
| Boundary-accepted request | boundary event + payload digest，并关联 attempt ID | 受监督 HTTP 边界实际收到请求 | 真实 API 请求数、预算与费用边界 |
| Completed provider response | 完整 attempt identity + `response_completed` | Provider 流完成；usage 可有或缺 | 成功请求数、token/cache/cost |

`TokenCount` 中没有任何 Provider identity 的记录属于 `state_snapshot`，不属于以上四种计数。

### 4.2 规范化记录

共享 analyzer 应把原始事件规范化为不可变的机械记录，而不是让每个报表重新解释：

```json
{
  "logical_request_id": "provider-request:...:logical-7",
  "provider_request_id": "provider-request:...:logical-7:attempt-1",
  "attempt_seq": 1,
  "payload_sha256": "...",
  "attempt_status": "response_completed",
  "boundary_status": "accepted",
  "usage_status": "measured",
  "input_tokens": 1234,
  "cached_input_tokens": 900,
  "output_tokens": 120
}
```

允许的 `boundary_status` 至少为 `accepted`、`not_observed`、`evidence_unavailable`；`not_observed` 只有在本地 attempt
存在而 boundary 不存在时成立，不能自动解释成 Provider 拒绝。允许的 `usage_status` 至少为 `measured`、
`provider_omitted`、`not_applicable`、`evidence_unavailable`。

### 4.3 对账规则

- 同一完整 `provider_request_id` 的重复完成事件 usage 完全一致：幂等去重并记录 duplicate count；
- 同一 ID 的 identity、terminal status 或 usage 冲突：`identity_conflict`，整组不可比较；
- 只有部分 identity：`identity_incomplete`，不得按顺序补齐；
- no-ID `TokenCount`：保留为 snapshot count，不进入 request/usage 集合；
- boundary 中出现本地 attempt 集合不存在的 digest：真正的 `upstream_unattributed_request`，阻断；
- 本地 attempt 未出现在 boundary：单列 `attempt_not_observed_at_boundary`；若其 terminal 是 `response_completed`，
  则证据互相矛盾并阻断；若为本地失败/取消，不判 upstream mismatch；
- 重试按 physical attempt 分开计数，按 `logical_request_id` 汇总逻辑请求，两个值都保留；
- usage 总和只来自 `response_completed + measured usage`，不能从累计 snapshot 相减或按最后一条推算。

## 5. 输出合同与可用性

请求汇总升级后至少显式提供：

| 字段 | 含义 |
|---|---|
| `logical_request_count` | Agent 逻辑采样轮数 |
| `provider_attempt_count` | 本地形成并尝试分派的 physical attempts |
| `boundary_request_count` | 有边界证据的实际请求数 |
| `completed_response_count` | 成功完成的 Provider 响应数 |
| `failed_or_cancelled_attempt_count` | 未完成 attempts，不与成功请求混算 |
| `usage_record_count` | 有完整 identity 且 usage measured 的完成响应数 |
| `state_snapshot_count` | 被排除在请求聚合外的 no-ID 状态事件数 |
| `availability` | `measured`、`partial`、`unavailable` 或 `incomparable` |
| `findings` | 稳定 reason code、身份和证据路径，不包含敏感 payload |

“部分可用”不等于所有指标都可比较。报告应按指标暴露 availability：例如没有 supervisor 时，完成响应及 usage 可以
`measured`，但 `boundary_request_count` 必须是 `unavailable`，不能复制 attempt count 填充。

## 6. 实施单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| I07-W0 | 固化两类真实反例 | evidence/test | `scripts/taskspace-benchmark/fixtures/i07/`、专题结果文档 | 8/15 paired TokenCount 与 10/11 boundary mismatch 的脱敏最小 fixture | 从现有原始证据提取最小事件形状，保留身份、terminal、usage 和 digest 关系，不复制 prompt/命令/用户正文 | 后续修复可离线重放两个根因 | 零 API 成本证明旧行为和新行为差异 | 增加少量受版本管理 fixture；不改变生产 | 旧 analyzer 分别复现 15 和 mismatch；fixture SHA 写入结果 | 无法由 fixture 复现现有报告时暂停，先修正根因描述 | completed (`test-i07-characterization.ps1`) |
| I07-W1 | 建立唯一消费面 inventory | discovery | `scripts/taskspace-benchmark/`、`scripts/cache-regression/`、Runtime trace consumers | `TokenCount`、`payload_captured`、terminal 和 boundary 的全部 reader | 用静态检查列出每个消费者所需的是 shape、attempt、boundary、completion 还是 usage，并形成机器可检查 allowlist | 后续不会只修一处而留下同义错误 | 防止报表间继续出现不同请求数 | 新增一份 inventory/检查脚本，不改变数据 | `rg` 结果全部被分类；未知新消费者使检查失败 | 遇到无法判定语义的消费者停止并单独调查，不先改名掩盖 | completed (`request-fact-consumers.json`) |
| I07-W2 | 提取唯一请求事实生成器 | architecture | `scripts/taskspace-benchmark/request_facts.py`、`build-request-facts.py` | raw rollout/wire/boundary -> `request-facts.json` | Python library 解析完整 identity、去重、冲突、attempt/terminal/boundary 和 per-metric availability；薄 CLI 接收三个可选来源并一次生成规范化 rows 与 summary；PowerShell 只读取产物，不复制分类算法 | 所有报告从同一版本化事实 artifact 派生 | 消除 PowerShell/Python 多套口径继续漂移 | Harness 已依赖 Python；新增一个不超过 500 行的 library 和薄 CLI，不改变 Runtime | contract fixtures 覆盖 no-ID、partial ID、duplicate equal/conflict、retry、failed/cancelled、missing terminal；CLI 输出确定性 | 若单文件将超 500 行则按 parser/reconcile 拆模块；任一消费者要求复制算法时暂停 | completed (`request_facts.py`) |
| I07-W3 | 修复 request/usage 双计 | observability | `cost-instrumentation.ps1`、`test-harness.ps1`、`test-cost-instrumentation.ps1` | `New-TaskspaceRolloutRequestTraceSummary`、`request-facts.json` 与 request summary schema | 由 W2 生成器消费 rollout；仅完整 identity 的 completed usage 建立记录；no-ID snapshot 排除；相同 ID 去重、冲突 fail closed；PowerShell summary 机械读取同一产物 | 8 个请求报告为 8，token/cache/output 各只累加一次 | TX-00 完成，后续成本结论可复算 | 既有错误数字会变化；UI/Runtime 事件不变；增加一个本地 Python 子进程 | 合成 pair 1->1；历史 8/15 fixture -> 8；缺失/冲突输出不可比较 | 不允许回退按事件条数；W3 通过即可结算 TX-00，但不关闭完整 I07 | completed (`request-facts.json`) |
| I07-W4 | 修正 attempt 与 boundary 对账 | observability | `request_facts.py`、`docker/verify_provider_boundary.py`、provider boundary tests | ordered attempt facts 与 boundary claimed facts | 让现有 verifier 成为 W2 classifier 的薄调用方；由“两个完整列表必须相等”改为按 payload digest/identity 的阶段关系核对；合法本地未越界 attempt 单列，未知上游请求、重排、冲突和 completed-without-boundary 仍 fail closed | 10 次边界请求 + 1 次本地失败不再伪装 upstream mismatch | 保留严格性，同时忠实表达失败发生在哪一层 | boundary evidence schema 升级为 v2；缓存严格合同同步识别 v2，usage 分母迁移仍留给 W6 | 10/11 fixture => reconciled with one local-only attempt；未知 boundary digest 仍阻断 | 无法证明额外 attempt 未越界时标记 unavailable，不猜原因或放宽为 pass | completed (`whalecode-provider-boundary-evidence-v2`) |
| I07-W5 | 迁移性能请求计数消费者 | compatibility | `performance-observation.ps1`、`r7-five-layer-trace-analysis.ps1`、相关 tests | `provider_requests`、logical/attempt/completed counts | 删除按 `payload_captured` 或预分配行数代填 Provider 请求的 fallback，改读 W2 summary；报告并列展示 logical、attempt、boundary、completed；wire detail/latency reader 只保留 canonical facts 尚未承载的独特字段，数量不一致即阻断 | 性能表中的请求数不再随消费者不同而变化 | 先修直接影响版本对比的主报表 | 性能报告 schema 增列，旧报告只读 | 同一 fixture 在 performance/five-layer/request summary 中四类计数一致 | 任一主报表仍只有无来源 `provider_requests` 时停止迁移 | completed (`request_facts_boundary` / `request_facts_completion`) |
| I07-W6 | 迁移缓存与 section-cost 消费者 | compatibility | `provider-section-cost.ps1`、`cache_usage_contract.py`、cache tests | payload shape rows、completed usage rows、cache denominator | payload shape 继续服务前缀/section 分析但明确命名 attempt；cache/token 分母只选 completed measured rows；缓存基线仍可要求全部完成，但从 W2 facts 判断门禁失败，不能反向定义通用事实 | 结构诊断保留，缓存命中率不被无 usage 尝试污染 | 避免为修请求口径丢失 payload shape 价值，也不放松缓存发布门禁 | cache summary schema 与 baseline validator 需按门禁流程更新 | completed fixture 正常聚合；failed/missing/retry fixture 的通用 facts 正确且缓存基线按既有严格合同阻断；生产 payload 0-diff | 门禁若发现 Provider payload 变化则暂停并先说明，I07 不应主动改变 payload | completed (`request_facts_completed_usage`) |
| I07-W7 | 接通证据新鲜度和来源完整性 | evidence governance | `r7-artifact-provenance.ps1`、`r7-five-layer-evidence-freshness.ps1`、release/report gate | `request-facts.json` 与其来源 | 将 rollout、wire、boundary、binary/commit、analyzer version 和 SHA 封入 manifest；重算时验证来源未变；按指标输出 availability | 旧证据、当前代码和当前报告不会被混为一次运行 | 报告可复算，过期证据不能关闭问题 | manifest 增加事实 artifact 和 analyzer identity；不要求重写历史产物 | 篡改、缺文件、commit/二进制不匹配、旧 analyzer 均使比较 fail closed | 不为了让旧数据通过而做兼容推测；历史证据可保留但标记 legacy/incomparable | completed (`r7-artifact-evidence-manifest` v2) |
| I07-W8 | 建立 observer 自观测和诊断 | logging | `request_facts.py` 与报告输出 | stable findings/reconciliation summary | 记录各阶段计数、去重数、snapshot 排除数、冲突 reason code 和证据路径；禁止 payload、prompt、Tool 正文 | 任何数字变化都能定位到分类或数据来源 | 降低再次出现 8/15 时的调查成本 | 增加小型 JSON summary，不进入 Agent context | 日志 contract tests；敏感字段负向扫描；总数可由 normalized rows 复算 | 日志无法解释汇总差值则不进入验收 | planned |
| I07-W9 | 集成 TaskSpace Exec 身份 | integration | `taskspace_exec` TX-11 trace、`request_facts.py` | outer exec、internal item/node、provider facts 与 request identity | 在不改变 Provider 计数合同的前提下，给 TaskSpace Exec 内部 item 关联 outer call、node、capability 和 provider fact；本地 preflight reject 明确 provider delta=0 | 新协议失败可定位到生成、预检、执行、绑定或反馈层 | I07 不因新超级工具形成第二套观测规则 | 依赖 TX-03～TX-10 的最终数据结构；不在旧 sibling 路径提前实现 | 合成合法/拒绝/mixed hosted trace 可逐 ID 重建；Standard provider facts 0-diff | TaskSpace 字段需要修改普通 Tool payload或 Provider identity 时暂停 TX-11 | planned |
| I07-W10 | 结算当前协议的离线观测基础 | verification/docs | I07 fixtures、全部聚焦测试、R8/TaskSpace Exec 文档 | W0-W8 当前协议结果 | 重放真实形态 fixture，运行 Python/PowerShell tests 和 consumer inventory gate，记录提交与 schema 变化 | 不运行 Whale Agent 即可证明已确认双计、边界误判和消费漂移被修复 | 恢复后续评测可信度，不产生 API 成本 | 测试时间增加；无缓存敏感生产变更预期 | 两个反例通过、负例仍阻断、所有消费者同口径；I07 保持 queued 等待新协议接入 | 任一 consumer 数字不可解释则不结算当前协议基础 | planned |
| I07-W11 | 新协议生产验收并结算 I07 | E3/governance | TaskSpace Exec TX-14/TX-15、全局 run ledger、I07 result doc | Standard/TaskSpace 当前生产 trace | 确定性测试全绿后另行申请最小真实预算；验证 request/attempt/boundary/completion/usage 可复算，再更新唯一问题账本 | I07 在真实新协议路径完成闭环 | 后续成本与行为比较有可信分母 | 产生经授权 API 成本；不得把业务成功当作观测通过 | 预算内样本逐 ID 对账；本地 reject fixture provider delta=0；证据 manifest sealed | 未获预算不运行；任一身份冲突保持 I07 verifying/queued，不自动重试 | planned |

## 7. 阶段、依赖与停点

### Phase A：冻结事实与影响面

- Units: I07-W0～W1。
- Exit: 两类根因可离线复现，所有已知消费者都有明确语义分类。
- Stop: 若 8/15 或 10/11 无法由最小 fixture 复现，停止实施并修正文档，不迎合预期输出。

### Phase B：建立单一事实层

- Units: I07-W2～W4。
- Dependency: W2 在 W3/W4 前；W3 对应 TaskSpace Exec `TX-00`。
- Exit: 请求完成/usage 不双计，attempt/boundary 差异不再被错误解释，真正冲突仍 fail closed。
- Stop: 若需要两套分类器或按序号猜配才能通过，暂停并重新设计身份来源。

### Phase C：迁移消费者与证据门禁

- Units: I07-W5～W8。
- Exit: 成本、缓存、性能、freshness 和 boundary 报告使用同一事实合同；证据可复算。
- Stop: 任一同名字段仍有两个语义，或 null 被静默替换成估算值，不进入下一阶段。

### Phase D：TaskSpace Exec 接入与关闭

- Units: I07-W9～W11。
- Dependency: W9 对应 TaskSpace Exec `TX-11`，只能在其 typed plan/result identity 稳定后接入。
- Exit: Standard 与 TaskSpace 共用 Provider 事实，TaskSpace 只增加 exec/item/node 关联；经授权 E3 后再决定 I07 状态。
- Stop: 新协议要求观测层修改普通 Tool schema、反馈内容或 Agent 行为时立即暂停。

## 8. 验收矩阵

| 场景 | Attempt | Boundary | Completed | Usage | 预期 |
|---|---:|---:|---:|---:|---|
| completed + no-ID snapshot | 1 | 依证据 | 1 | 1 | 请求与 usage 均只计一次，snapshot_count=1 |
| 同 ID 相同 completed 重放 | 1 | 依证据 | 1 | 1 | 幂等去重并报告 duplicate |
| 同 ID usage 冲突 | 1 | 依证据 | 不可比较 | 不可比较 | `identity_conflict` 阻断 |
| partial identity TokenCount | 不推断 | 不推断 | 不推断 | 不累计 | `identity_incomplete` 阻断该指标 |
| retry 后成功 | 2 | 2 或按边界证据 | 1 | 1 | logical=1、attempt=2、completed=1 |
| 本地失败未越过边界 | 1 | 0 | 0 | 0 | local-only attempt，不是 upstream mismatch |
| completed 但 boundary 不存在 | 1 | 0 | 1 | 1 | 来源矛盾，整体 incomparable |
| boundary 存在未知 digest | 0 | 1 | 未知 | 未知 | upstream unattributed，阻断 |
| 没有 supervisor 证据 | N | unavailable | M | K | completion/usage 可测，boundary 指标 null |
| fixture 或 analyzer identity 过期 | 不输出比较值 | 不输出比较值 | 不输出比较值 | 不输出比较值 | stale/incomparable |

## 9. 已知副作用与非目标

### 已知副作用

- 修复后历史错误报告中的 request/token/cache 总量会下降，这是口径纠正，不是性能变化；
- 请求汇总和 Provider boundary evidence 需要 schema 升级，当前消费者必须原子迁移；
- 失败运行会出现更多精确的 `partial/unavailable/incomparable`，不会再总有一个可比较数字；
- attempt、boundary、completion 三个计数同时暴露，报告列会增加，但每列语义更窄且可复算。

### 非目标

- 不修改 Agent prompt、Tool schema、Map 状态机或工具执行顺序；
- 不删除 rate-limit/TokenCount 状态广播；
- 不用 observer 自动修复、重试或阻止 Agent 动作；
- 不重写历史 artifact，也不为无身份旧数据增加推测性兼容；
- 不把 I07 扩大为通用 telemetry 平台建设。

## 10. 外部依据

1. [W3C Trace Context](https://www.w3.org/TR/trace-context/) 将 trace identity 与当前 operation 的 parent/span identity
   分开，支持本计划区分 logical request 与 physical attempt，并要求无效身份不能被继续解释。
2. [OpenTelemetry Logs Data Model](https://opentelemetry.io/docs/specs/otel/logs/data-model/) 强调日志映射必须保持原始语义，
   并将 event type、TraceId、SpanId 与 attributes 分开；这支持“状态快照不能仅因携带 usage 就改解释为请求”。
3. [OpenTelemetry GenAI semantic attributes](https://opentelemetry.io/docs/specs/semconv/registry/attributes/gen-ai/)
   分别定义 response identity、input/output/cache usage 和 Tool call identity，支持按完成响应身份归属 usage。
4. [Codex protocol v1](https://github.com/openai/codex/blob/main/codex-rs/docs/protocol_v1.md) 将一次 Turn 的模型请求、
   Provider `response.completed` 和后续 Tool 执行区分为不同阶段，支持本计划不把本地 Tool/preflight 事实混入 Provider 请求。
5. [Codex app-server protocol](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md) 的
   `rawResponse/completed` 为每次上游完成事件携带 `responseId` 和精确 usage，进一步支持 usage 以完成响应为归属边界。

这些资料只用于约束身份和事件语义，不要求 WhaleCode 引入 OpenTelemetry SDK 或复制 Codex app-server 协议。

## 11. 完成定义

I07 只有同时满足以下条件才能从唯一全局问题表关闭：

1. 8/15 双计反例离线重放为 8，所有 token/cache/output 总量逐 ID 可复算；
2. 10/11 反例忠实报告 10 次 boundary request、11 次 local attempt 和 1 次 local-only failure，不再误报 upstream mismatch；
3. 所有已知消费者通过 inventory gate，`provider_request_count` 不再由 `payload_captured` 或 no-ID snapshot 代填；
4. 缺失、冲突、部分身份和过期证据均 fail closed，报告不输出伪精确比较值；
5. TaskSpace Exec 的本地 preflight reject 明确 `provider delta=0`，Standard 与 TaskSpace 使用同一 Provider 事实合同；
6. 确定性测试和必要的授权 E3 均有提交、二进制、模型、原始证据和账本记录；
7. 结果文档记录被否定假设、已知副作用和仍不可观测的边界，不能用“基本完成”代替证据。
