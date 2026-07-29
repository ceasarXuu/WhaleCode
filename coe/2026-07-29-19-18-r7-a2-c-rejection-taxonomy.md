# Problem P-001: A2-C repair rerun 的零执行拒绝仍占主要请求成本
- Status: open
- Created: 2026-07-29 19:18
- Updated: 2026-07-30 03:45
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
  - observer 覆盖生产支持的全部执行型 ResponseItem，畸形或不完整证据 fail-closed
  - sibling copy 只由显式 causal identity 判定，不从相同错误内容推断
  - ToolSearch sibling 保留 typed state facts，不把底层 JSON 再包装为字符串
  - 状态拒绝区分 canonical state 与 rejected transaction 的 evaluated state
  - 不修改 ordinary Tool schema，不让 Runtime 替 Agent 选择节点或动作
- Current conclusion: W0 首轮只修复了 Function/Custom Tool 状态事实和 Function-call 样本观测子集。
  对抗性审查已确认 observer 完整性、sibling 因果身份、ToolSearch carrier、state scope 和 retained
  evidence provenance 仍不满足关闭标准
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
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
- Conclusion: 原始“完全缺失”已修复，但 E-008/E-009 证明 ToolSearch sibling 和状态域仍会扭曲或歧义化
  同一机械原因，因此原关闭结论无效
- Repair design readiness: repair-authorized
- Next step: 按 H-006/H-007 修复所有 model-visible carrier 和状态域合同
- Blocker:
  - none
- Close reason:
  - reopened by W0 adversarial review; E-004 的 request 14 叙述被 E-008 证伪

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
- Conclusion: Function-call live 子集可以对账，但 E-006/E-007/E-010 证明完整生产载体、证据健康度、
  sibling 因果身份和最终 artifact provenance 仍未闭合
- Repair design readiness: repair-authorized
- Next step: 按 H-004/H-005 修复 observer input model、fail-closed 和因果 identity
- Blocker:
  - none
- Close reason:
  - reopened by W0 adversarial review; E-005 只证明单一 Function-call 样本

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

## Evidence E-004: 节点状态拒绝事实已原样进入模型上下文
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source:
  - `core/src/action_map/rooted_dag/phase_d_tests.rs`
  - `core/src/tools/sequence_tests.rs`
  - W0 live rollout
- Prediction or plan link:
  - H-002 的结构化机械事实和无同形重复标准
- Matched signal:
  - `node_state_invalid` 直接返回 `node_id`、`actual_state`、`allowed_states`、
    `unsatisfied_predecessor_ids`
  - request 12 返回 `fix / waiting / [explore]`
  - request 13 返回 `explore / completed / []`
  - request 14 转向合法 `fix` reservation；没有第三次重复前两种状态事实
- Correlation keys:
  - run `target/r7-w0-live-credentialed/subscription-billing-repair/20260730-025642-404`
  - request index 12、13、14
  - node ID
- Raw content:
  ```text
request 12: node_state_invalid node=fix actual=waiting unsatisfied=[explore]
request 13: node_state_invalid node=explore actual=completed unsatisfied=[]
request 14: taskspace_control execute node=fix state_commit=true
  ```
- Interpretation: Runtime 只透传状态机已经知道的事实，没有建议下一动作；Agent 第二次仍选错节点属于可独立观察的
  行为，不是反馈缺失或同形重试
- Time: 2026-07-30 17:15

## Evidence E-005: request taxonomy、receipt/cache 与解析健康度已成为同一观测事实
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source:
  - `scripts/taskspace-benchmark/lib/r7-request-observability.ps1`
  - W0 live provider wire trace
  - performance observer self-test
- Prediction or plan link:
  - H-003 的一级分类对账、carrier/cache 自动关联和 fail-closed 标准
- Matched signal:
  - 21 个请求唯一归类为 `none=8`、`tool_sequence_protocol=8`、
    `taskspace_state_machine=2`、`ordinary_tool=3`
  - 一级分类总和为 21，未知分类为 0
  - sibling failure copies 为 9
  - 9 个 receipt-before request 均解析出原始 `developer`、wire `system`、精确 revision 和 cache
  - 共享普通 Tool 失败解析器从同一 live rollout 对账出 Standard 2 次、TaskSpace 3 次失败
  - 畸形 control 参数使 cadence availability 变为 `partial_with_parse_errors`
- Correlation keys:
  - provider request index
  - provider wire request ID
  - control call ID hash
- Raw content:
  ```text
classification_reconciled=true
ordinary failures: standard=2 taskspace=3
receipt-before cache=23.7208%
no-receipt-before cache=86.4687%
receipt wire role unresolved=0
  ```
- Interpretation: W0 只建立可信事实口径；低 receipt-before cache 和大量 sequence failure 仍分别归属
  R71-GI-002、003，不能因观测器完成而视为已修复
- Time: 2026-07-30 17:15

## Hypothesis H-004: Observer 输入模型和健康门窄于生产事件模型
- Status: confirmed
- Parent: P-001
- Claim: trace analyzer 只消费 Function call/output，且 failure/receipt/provenance 解析失败不参与
  comparison eligibility，所以不完整证据仍可形成表面完整的 request 对账
- Layer: observability
- Factor relation: single
- Depends on:
  - none
- Falsifiable predictions:
  - If true: event codec 支持的 Custom、ToolSearch、LocalShell 形态没有 observer 分支；畸形
    TaskSpace failure 仍归为已知 ordinary Tool
  - If false: 所有执行型 ResponseItem 都有 typed parser，任一 parse/schema/identity 缺失都会让报告失效
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较生产 event codec 与两个模式的 trace parser，并注入 malformed failure
  - Signal: event type 支持集合、per-call parse health、顶层 `classification_reconciled`
  - Capture method: 代码路径审计和内存 PowerShell probe
  - Supports if: 支持集合不一致，或 malformed failure 仍 reconciled
  - Refutes if: observer 完整覆盖且 probe fail-closed
  - Instrumentation status: permanent-observer-change-required
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-007
- Conclusion: 两条独立证据路径均支持；这是 GI-007 的生产实现缺口
- Repair design readiness: repair-authorized
- Next step: 建立 typed observed-call model、parse/schema health 和 observation eligibility
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-005: sibling copy 使用内容签名错误推断因果身份
- Status: confirmed
- Parent: P-001
- Claim: `class|code|violations` 相同只能证明内容相同，不能证明失败是同一零执行拒绝的复制；
  当前算法因此合并独立失败
- Layer: observability
- Factor relation: single
- Depends on:
  - none
- Falsifiable predictions:
  - If true: 两个独立 `shell_exit_1` call 会得到 `sibling_failure_copy_count=1`
  - If false: 只有携带相同 explicit cause identity 的 derivative outputs 才计为 copy
- Diagnostic evidence plan:
  - Prediction or clause under test: 构造两个独立同码失败和一组 response-scoped derivative failures
  - Signal: `failed_call_count`、`sibling_failure_copy_count`、cause identity
  - Capture method: in-memory observer probe
  - Supports if: 独立失败被合并
  - Refutes if: 独立失败保持两份且真正复制可由 identity 关联
  - Instrumentation status: permanent-causal-identity-required
- Evidence gate: satisfied
- Related evidence:
  - E-007
- Conclusion: 反例稳定复现，signature heuristic 不能保留
- Repair design readiness: repair-authorized
- Next step: 从 response-scoped zero-dispatch failure 生成 stable cause/copy identity
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-006: ToolSearch sibling 的专用载体重新扭曲 typed state failure
- Status: confirmed
- Parent: P-001
- Claim: ToolSearch pairing 不能直接承载失败，因此 supplemental message 把原始
  `TaskSpaceResponseCommitFailureV1` JSON 放进 `error.message` 字符串，恢复了嵌套 JSON，并与
  `status=completed` pairing 形成冲突信号
- Layer: feedback-semantics
- Factor relation: single
- Depends on:
  - H-002
- Falsifiable predictions:
  - If true: state rejection + ToolSearch 产生空 completed pairing 和嵌套 failure message
  - If false: supplemental failure 直接保留 typed cause object，且 carrier 明确区分 pairing 与执行失败
- Diagnostic evidence plan:
  - Prediction or clause under test: 调用 `invalid_call_responses` 的 ToolSearch 分支
  - Signal: ResponseInputItem 类型、pairing status、supplemental JSON shape
  - Capture method: 代码路径审计与聚焦 Rust 测试
  - Supports if: original JSON 出现在 JSON string 字段
  - Refutes if: typed cause 保留且无 JSON-in-string
  - Instrumentation status: permanent-carrier-shape-telemetry-required
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-009
- Conclusion: sequence 将同一 state rejection 发送到每个 sibling，ToolSearch 分支确定性重包装
- Repair design readiness: repair-authorized
- Next step: 让 supplemental failure 直接携带 parsed mechanical cause，并记录 derivative identity
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-007: `actual_state` 没有声明 rejected transaction 的状态域
- Status: confirmed
- Parent: P-001
- Claim: transaction 先应用 node mutation 再校验 reservation，violation 的 `actual_state` 来自临时
  candidate；事务拒绝后 canonical Map 回滚，但反馈字段没有区分两者
- Layer: feedback-semantics
- Factor relation: single
- Depends on:
  - H-002
- Falsifiable predictions:
  - If true: 同事务 `complete_node(explore) + reserve(explore)` 返回
    `actual_state=completed,state_commit=false`，随后 read_map 显示 canonical `explore=ready`
  - If false: feedback 明确输出 canonical state 和 evaluated-at-violation state，Agent 无需猜测
- Diagnostic evidence plan:
  - Prediction or clause under test: 重放 live request 13 的事务顺序
  - Signal: fact application order、rejection state、canonical read、Agent 下一动作
  - Capture method: rooted DAG 代码路径和 retained rollout
  - Supports if: candidate/canonical 不同且字段未标明 scope
  - Refutes if: 两个机械状态域均明确可见
  - Instrumentation status: permanent-feedback-contract-change-required
- Evidence gate: satisfied
- Related evidence:
  - E-008
  - E-009
- Conclusion: live Agent 已将 candidate state 误解为 canonical state，因果链完整
- Repair design readiness: repair-authorized
- Next step: violation 同时携带 `canonical_state_before_transaction` 与
  `evaluated_state_at_violation`，不添加建议
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-006: fresh implementation-completeness review 推翻 W0 关闭
- Related hypotheses:
  - H-002
  - H-003
  - H-004
  - H-006
- Direction: supports
- Type: independent-review
- Source:
  - `vs_review/2026-07-30-r7-1-w0-factual-foundation-review.md`
- Prediction or plan link:
  - W0 close criteria
- Matched signal:
  - 非 Function event shapes 未进入 request taxonomy
  - malformed failure 和不完整 receipt 可 fail-open
  - ToolSearch sibling 恢复 nested JSON
  - retained run attestation 和 final implementation provenance 不合格
- Correlation keys:
  - reviewer session `019faf52-85fa-7260-897c-55634799451c`
  - reviewed commits `1d086fd7e^..a9264f5ff`
- Raw content:
  ```text
closure is not supported; reopen R71-GI-005 and R71-GI-007
  ```
- Interpretation: 独立路径找到多个能直接违反 W0 关闭标准的生产反例
- Time: 2026-07-30 03:30

## Evidence E-007: malformed failure 和独立同码失败反例
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Direction: supports
- Type: reproduction
- Source:
  - in-memory PowerShell observer probe
- Prediction or plan link:
  - H-004 malformed evidence
  - H-005 independent same-code failures
- Matched signal:
  - 两个独立 `shell_exit_1` 得到 `reported_sibling_copies=1`
  - malformed `TaskSpaceResponseCommitFailureV1` 得到
    `ordinary_tool/tool_failed_unclassified`
  - 顶层 `classification_reconciled=true`
- Correlation keys:
  - synthetic call IDs `independent-a`、`independent-b`
- Raw content:
  ```json
  {
    "independent_failed_calls": 2,
    "reported_sibling_copies": 1,
    "malformed_failure_class": "ordinary_tool",
    "malformed_failure_code": "tool_failed_unclassified",
    "classification_reconciled": true
  }
  ```
- Interpretation: 旧测试通过不能证明 observer 正确，两个原始症状均可确定性复现
- Time: 2026-07-30 03:32

## Evidence E-008: live Agent 将 candidate completed 误认为 canonical completed
- Related hypotheses:
  - H-002
  - H-007
- Direction: supports
- Type: production-trace
- Source:
  - `target/r7-w0-live-credentialed/subscription-billing-repair/20260730-025642-404/pair-001/right/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-007 state scope
- Matched signal:
  - request 13 提交 `complete_node(explore) + action(explore)`
  - rejection 返回 `state_commit=false,actual_state=completed,canonical_revision=17`
  - Agent 明确判断 “explore node is already completed”
  - request 14 `read_map` 显示 revision 17 的 `explore=ready`
- Correlation keys:
  - call `call_00_mTwXeScYxbbhWRfwhdp09577`
  - request 13、14
  - node `explore`
- Raw content:
  ```text
rejection: state_commit=false actual_state=completed canonical_revision=17
agent: "explore node is already completed"
read_map: revision=17 explore state=ready
  ```
- Interpretation: 这是反馈状态域歧义导致 Agent 额外读取，不是 Agent 无视完整事实
- Time: 2026-07-30 03:34

## Evidence E-009: 代码路径确定 candidate state 与 ToolSearch 重包装机制
- Related hypotheses:
  - H-006
  - H-007
- Direction: supports
- Type: code-location
- Source:
  - `core/src/action_map/rooted_dag/transactions.rs`
  - `core/src/action_map/rooted_dag/events.rs`
  - `core/src/tools/parallel.rs`
  - `core/src/tools/sequence.rs`
- Prediction or plan link:
  - H-006/H-007
- Matched signal:
  - transaction facts 先加入 node mutations，后加入 reservations
  - `apply_batch` 在 mutable candidate 上顺序执行 facts
  - reservation rejection 从 candidate `derive_node_state`
  - ToolSearch supplemental 把 model-visible failure 字符串放入 `error.message`
- Correlation keys:
  - event batch revision
  - Tool call ID
- Raw content:
  ```text
facts.extend(node_mutations); facts.extend(reservations)
actual_state = derive_node_state(candidate, reservation.node_id)
ToolSearchFailureV1.error.message = function_call_error_model_visible_message(error)
  ```
- Interpretation: 两个反馈问题均由确定的生产路径产生，不依赖模型随机性
- Time: 2026-07-30 03:36

## Evidence E-010: retained run 缺最终实现和二进制有效溯源
- Related hypotheses:
  - H-003
  - H-004
- Direction: supports
- Type: artifact-health
- Source:
  - W0 `whale-binary-preflight-health.json`
  - W0 `run-status.json`
  - artifact mtimes
- Prediction or plan link:
  - H-004 provenance health
- Matched signal:
  - build attestation 为 `binary_sha_mismatch,codex_source_commit_mismatch`
  - `final_aggregate_ready=false`，suite/source provenance 为空
  - performance/request-observability artifact 早于最终 metrics commit `4a136ca32`
- Correlation keys:
  - run `20260730-025642-404`
  - implementation commit `4a136ca32`
- Raw content:
  ```text
build_attestation_status=invalid
final_aggregate_ready=false
artifact generated before final implementation commit
  ```
- Interpretation: retained run 可以作为诊断原始数据，不能作为最终候选关闭证据
- Time: 2026-07-30 03:37
