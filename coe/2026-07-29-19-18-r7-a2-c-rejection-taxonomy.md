# Problem P-001: A2-C repair rerun 的零执行拒绝仍占主要请求成本
- Status: open
- Created: 2026-07-29 19:18
- Updated: 2026-07-30 16:20
- Objective: 区分协议遵循、状态反馈和 Agent 普通工具错误，避免继续用汇总 failure code 错判根因
- Symptoms:
  - 24/24 业务成功和 18/18 Map terminal 掩盖了大量零执行拒绝
  - 首请求初始化成功率极低
  - Map 初始化后仍频繁发出不带 control 的 ordinary Tool response
  - `reservation_invalid` 只给 reservation ID，无法直接判断节点状态原因
- Expected behavior:
  - TaskSpace 工作请求稳定使用 control manifest
  - 状态硬门返回完整、机械、无再解释的拒绝事实
  - 汇总器按 request 和根因计数，不把 sibling 复制反馈重复当作独立 violation
- Actual behavior:
  - 277 个 TaskSpace request 中 116 个被流程或状态硬门零执行拒绝
  - baseline 9 个和 rerun 14 个 reservation failure 都被早期汇总错误归到同一 ownership 根因
  - observer 同时输出 82 个 `tool_sequence_protocol_failure_requests`、0 个
    `taskspace_protocol_failure_requests` 和 53 个 `control_failures`，字段边界不直观
- Impact:
  - request、自然历史、失败反馈和 uncached input 被结构性放大
  - 业务成功率无法代表 TaskSpace 协议稳定性
  - 错误分类会诱导错误修复方向
- Fix criteria:
  - 首请求 init + work 和持续 control 遵循度分别设门
  - state failure 输出结构化 violation context
  - observer 输出 request-level 根因，不重复统计 sibling copies
  - observer 字段名和文档明确区分 sequence、control-call、state 和 ordinary failure
  - 不修改 ordinary Tool schema，不让 Runtime 替 Agent 选择节点或动作
- Current conclusion: 当前主要剩余问题不是普通工具执行能力，而是 control 跨调用协议不稳定、revision 反馈歧义和
  reservation 拒绝原因缺失
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - latest 24-run matrix
  - raw rollout request reconstruction
- Close reason:
  - not closed

## Hypothesis H-001: control_required 是持续协议遵循问题，不是拒绝反馈缺失
- Status: confirmed
- Parent: P-001
- Claim: Agent 收到 `taskspace_control_required` 后通常能在下一 response 纠正，但后续又回到 ordinary-only
  response；根因是跨 Tool-call 行动框架没有稳定形成
- Layer: protocol-adherence
- Factor relation: single
- Depends on:
  - none
- Falsifiable predictions:
  - If true: 大多数 control-required 的下一 response 会携带 control，但同一 run 后续再次遗漏
  - If false: Agent 连续重复 ordinary-only，说明错误反馈本身不可理解
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: 59/63 的下一 response 携带 control，但 41 次遗漏发生在 Map 已成功初始化后
- Repair design readiness: needs-design
- Next step: 评估 provider 原生可表达的 response-level Tool 序列合同，不能继续只增加提示词
- Blocker:
  - cross-call sequence 无法由单个 Tool JSON schema 完整表达
- Close reason:
  - not closed

## Evidence E-001: 首请求和持续 response 均存在 control 遗漏
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: A2-C repair rerun `trace-analysis.json`
- Prediction or plan link:
  - H-001
- Matched signal:
  - 18 个首请求：14 ordinary-only，3 solo init mismatch，1 init + work committed
  - 63 个 control-required：22 pre-init，41 post-init
  - 59/63 的下一 response 携带 control
- Correlation keys:
  - run root `target/r7-five-layer-matrix/a2-c-repair/445499582/20260729-0546`
  - arm/sample/repeat/request index
- Raw content:
  ```text
first request: ordinary-only=14, solo-init-without-siblings=3, committed-init-and-work=1
control_required: pre_init=22, post_init=41
next response contains control=59/63
  ```
- Interpretation: Runtime 拒绝信息可被短期理解，但 L1/L2/L4 没有形成稳定的跨 response 默认行为
- Time: 2026-07-29 19:18

## Hypothesis H-002: reservation_invalid 的反馈丢失了机械原因
- Status: confirmed
- Parent: P-001
- Claim: Runtime 内部知道 reservation 目标节点及其 Ready/InFlight/Waiting/Completed 状态，但 model-visible
  failure 只返回 reservation ID
- Layer: feedback-semantics
- Factor relation: single
- Depends on:
  - none
- Falsifiable predictions:
  - If true: raw failure 无节点状态和前置条件，Agent 需要 read_map 或重复猜测
  - If false: failure 已结构化返回目标节点、实际状态和允许状态
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: 14 个 failure 中 12 个是 waiting 后继节点、2 个是完成节点 ownership；反馈均只含 reservation ID，
  并产生 4 次同形重复。baseline 也已有 8 个 waiting-node failure，因此不是本轮新引入
- Repair design readiness: ready
- Next step: 在 Rejection 中保留 node ID、actual state、allowed states 和 unsatisfied predecessors，并由 response
  原样结构化透传
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-002: reservation failure 根因与公开 detail 不一致
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source:
  - repair rerun raw rollout
  - `core/src/action_map/rooted_dag/events.rs`
  - `core/src/tools/sequence.rs`
- Prediction or plan link:
  - H-002
- Matched signal:
  - `reserve_action` 用 derived node state 判断 Ready/InFlight
  - replay rejection 只保存 `reservation_invalid` 和 reservation ID
  - sequence 又把 rejection JSON 作为字符串放进 `error.detail`
- Correlation keys:
  - reservation ID
  - node ID
  - request index
- Raw content:
  ```text
{"code":"reservation_invalid","subjects":["map-...:reservation:..."]}
  ```
- Interpretation: 硬门本身符合状态机底线，反馈层却没有忠实保留 Runtime 已知的机械失败原因
- Time: 2026-07-29 19:18

## Hypothesis H-003: Observer 缺少 request-level 唯一主分类和 receipt/cache 一等关联
- Status: confirmed
- Parent: P-001
- Claim: 当前矩阵先按 call-level `failure_class` 分别扫描多个集合，再按“request 内存在该类 call”计数；
  同一零执行拒绝复制到 sibling Tool 后会产生重复 call 事实，且 request 没有唯一主分类。receipt、wire role
  和下一请求 cache 也没有进入同一 request row，只能依赖事后脚本临时关联
- Layer: observability
- Factor relation: single
- Depends on:
  - none
- Falsifiable predictions:
  - If true: request row 只有 `calls[].failure_class` 和 `failure_codes`，没有唯一
    `primary_failure_class`、复制计数或 receipt-before cache 字段
  - If false: 每个 provider request 已被唯一分类，分类总和可与 provider request 对账，且 receipt carrier/cache
    已自动输出
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查 trace reconstruction、matrix aggregation 和 provider wire terminal 的数据连接
  - Signal: request row 字段、分类计数算法、request_id/index、receipt marker、input/cached input
  - Capture method: `r7-five-layer-trace-analysis.ps1`、`report-r7-five-layer-matrix.ps1` 和聚焦 fixture
  - Correlation keys:
    - run
    - provider request index
    - call ID
    - provider wire request ID
  - Supports if:
    - report 通过四个独立 `Where-Object` 集合计数，且 receipt/cache 不在 request row
  - Refutes if:
    - request 已有互斥主分类和自动 carrier/cache 关联
  - Instrumentation status: permanent-observer-change-required
  - Instrumentation lifecycle:
    - 新 request taxonomy、sibling copy count 和 receipt/cache facts 永久保留
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: 生产 observer 当前按 call 集合派生多个 request 计数，缺少唯一主分类和自动 cache carrier 归因
- Repair design readiness: ready
- Next step: 先在 request reconstruction 生成唯一主分类和 secondary tags，再由 matrix report 只聚合 request facts
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-003: 当前矩阵分类集合可重叠且 receipt/cache 不在 request row
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source:
  - `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1`
  - `scripts/taskspace-benchmark/report-r7-five-layer-matrix.ps1`
- Prediction or plan link:
  - H-003 的 request row 与分类算法预测
- Matched signal:
  - `Complete-R7RequestRows` 只写 `failure_codes`，不写 request-level 主分类
  - report 分别扫描 `tool_sequence_protocol`、`taskspace_protocol`、`taskspace_state_machine` 和
    `ordinary_tool` call，四个集合没有互斥合同
  - provider wire terminal 已有 request-level input/cached input，但没有与 receipt marker 自动对齐
- Correlation keys:
  - request index
  - call ID
  - provider wire request ID
- Raw content:
  ```text
request row: calls[], action_kind, failure_codes
report: four independent requestPath Where-Object scans
wire terminal: request_id + input_tokens + cached_input_tokens
  ```
- Interpretation: Observer 有足够原始事实，但构造层没有把它们归一为可对账的一等 request 事实
- Time: 2026-07-30 16:20
