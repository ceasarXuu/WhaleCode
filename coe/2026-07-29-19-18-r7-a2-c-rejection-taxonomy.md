# Problem P-001: A2-C repair rerun 的零执行拒绝仍占主要请求成本
- Status: validating
- Created: 2026-07-29 19:18
- Updated: 2026-07-31 07:34
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
- Current conclusion: Round 13 分块 closure review 证明 `6e487f057` 的 ordinary exit、state scope、
  ToolSearch sibling 和 retained 298-request/192-artifact raw facts 子门成立，但 GI-005/GI-007 仍被四类
  机械事实缺口阻断：lifecycle violation 重复包装；direct carrier JSON/envelope/success 与普通 Tool 隔离
  不严；provenance bytes 类型强转；合法 L5 carrier 被错误记为 wire identity mismatch，且 freshness 未进入
  eligibility。原 24/24 comparison eligible 声明失效，W0 保持 validating。修复只允许收紧构造、解析、
  分类和证据门，不得增加 Runtime 对 Agent 的语义控制
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
  - H-008
  - H-009
- Resolution basis:
  - latest 24-run matrix
  - raw rollout request reconstruction
- Close reason:
  - blocked by E-023/E-024; repair authorized by diagnostic evidence

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

## Evidence E-011: W0 反例回归全部 fail-closed
- Related hypotheses:
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Direction: supports
- Type: fix-validation
- Source:
  - `scripts/taskspace-benchmark/test-r7-five-layer-trace-analysis.ps1`
  - `scripts/taskspace-benchmark/test-r7-request-observability-report.ps1`
  - `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
  - `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/phase_d_tests.rs`
- Prediction or plan link:
  - H-003 至 H-007 fix criteria
- Matched signal:
  - Standard/TaskSpace 均覆盖 Function、Custom、ToolSearch、LocalShell、MCP call/output
  - malformed/unknown failure、orphan/duplicate/missing output、不完整 receipt 和 request identity
    mismatch 均阻断观察
  - 两个独立同码失败保持独立；只有相同 explicit copy group 且 zero-dispatch 才计 sibling copy
  - ToolSearch pairing=`completed` 与 execution=`failed` 分开，typed cause 不再进入 JSON string
  - rejected complete/reserve 同时返回 canonical=`ready`、evaluated=`completed`
- Correlation keys:
  - commit `e9d705a23558d3f777179ad8696351866e79081a`
  - provider wire schema `provider-chat-wire-trace-v9`
- Raw content:
  ```text
R7 five-layer trace analysis passed
R7 request observability report passed
cargo test -p codex-core --lib: 1926 passed, 0 failed, 3 ignored
  ```
- Interpretation: 首轮 reviewer 给出的确定性反例均已有正向与反向回归，且 parser 不再通过旧 schema
  fallback
- Time: 2026-07-30 04:45

## Evidence E-012: final-commit 24-run 工件来源与 request taxonomy 全部对账
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Direction: supports
- Type: production-trace
- Source:
  - `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/e9d705a23558d3f777179ad8696351866e79081a/20260730-045032-584`
- Prediction or plan link:
  - GI-007 retained artifact close criteria
- Matched signal:
  - 2 个 development sample × 4 arms × repeat 3 = 24/24 run 完成且业务成功
  - artifact provenance=`valid`，findings=0
  - source commit=`e9d705a23558d3f777179ad8696351866e79081a`
  - binary SHA-256=`c04fe7a5ba45d7c0e9898799556779498bd0722eddda5313f1b444d08d84fa02`
  - 24/24 run `classification_reconciled=true`
  - receipt wire role unresolved=0
- Correlation keys:
  - matrix run `20260730-045032-584`
  - Docker image `sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`
- Raw content:
  ```json
  {
    "status": "valid",
    "run_count": 24,
    "findings": []
  }
  ```
- Interpretation: 旧 retained run 的 invalid attestation 和旧脚本污染已被 final-commit 证据替代；
  现存 sequence/state/ordinary failure 仍被分类出来，没有被成功率掩盖
- Time: 2026-07-30 05:00

## Evidence E-013: repeat live 未复现 candidate state 被当作 canonical commit
- Related hypotheses:
  - H-002
  - H-006
  - H-007
- Direction: supports
- Type: fix-validation
- Source:
  - W0 retained matrix `trace-analysis.json`
- Prediction or plan link:
  - H-007 feedback state-scope fix
- Matched signal:
  - 39 个 state-failure request 全部保留 typed violation 或 typed direct control failure
  - 未观察到旧 trace 中“Agent 声明 canonical 已 completed，但实际只在 rejected candidate
    completed”的判断
  - simple `map-request` 有 2 次 Waiting 拒绝后 `read_map`；两次拒绝已直接携带
    canonical/evaluated=`waiting`、未满足前驱与 canonical revision，后续均正确完成前驱并继续
- Correlation keys:
  - sample `single-file-fast-fix`
  - arm `map-request`
  - repeats 1、3
- Raw content:
  ```text
node_state_invalid:
canonical_state_before_transaction=waiting
evaluated_state_at_violation=waiting
evaluated_unsatisfied_predecessor_ids_at_violation=[explore]
next: read_map
then: complete_node(explore) + action(fix)
  ```
- Interpretation: 原 candidate/canonical 语义扭曲未复现；显式读取发生在 map-request 外部资料模式，
  现有证据不支持让 Runtime 增加建议或自动状态推进
- Time: 2026-07-30 05:03

## Evidence E-014: Round 2 反例与 post-boundary 因果归属已 fail-closed
- Related hypotheses:
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Direction: supports
- Type: fix-validation
- Source:
  - `scripts/taskspace-benchmark/test-r7-five-layer-trace-analysis.ps1`
  - `scripts/taskspace-benchmark/test-r7-request-observability-report.ps1`
  - `scripts/taskspace-benchmark/test-external-wrapper-harness.ps1`
  - `third_party/codex-cli/codex-rs/core/src/tools/sequence_tests.rs`
- Prediction or plan link:
  - W0 Round 2 blocking findings 1-5
- Matched signal:
  - rejected candidate 明确 `committed=false`，canonical 明确 `node_present`
  - provider wire v10 保留 logical request、attempt、HTTP/WS 与所有 terminal
  - ordinary Tool 文本不能伪造 trusted TaskSpace failure；重复 output/supplemental 阻断解析
  - ToolSearch V3 的 `call_id` 与 `affected_call_ids` 必须一致
  - supplemental 即使在 `token_count` 后出现，也由 affected calls 反查 owning request；跨请求、残缺、
    重复 call identity 均 fail-closed
  - 每个正式 run 的 8 份 raw artifact 由 evidence manifest 哈希封存，final report 只接受 clean
    commit、matching binary probe 和 finalized aggregate
- Correlation keys:
  - commits `787070d88`、`79a1e1c96`、`242414c8a`、`f602e6c90`
  - provider wire schema `provider-chat-wire-trace-v10`
- Raw content:
  ```text
R7 five-layer trace analysis passed
R7 request observability report passed
TaskSpace external wrapper self-test: PASS
R7 five-layer evidence freshness self-test passed
  ```
- Interpretation: observer 不再用事件遍历位置猜测失败属于哪个 provider request；严格集合相等门保留，
  没有为适配真实顺序而放宽 provenance
- Time: 2026-07-30 06:39

## Evidence E-015: current-commit 24-run 正式矩阵完成 W0 事实链复验
- Related hypotheses:
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Direction: supports
- Type: production-trace
- Source:
  - `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/f602e6c90203016c38970af3b1c359f6bb1bceac/20260730-064119-500`
- Prediction or plan link:
  - GI-005/GI-007 fresh closure review
- Matched signal:
  - 24/24 business success、24/24 comparison eligible、24/24 classification reconciled
  - artifact provenance=`valid`、findings=0、final aggregate=`finalized`
  - commit=`f602e6c90203016c38970af3b1c359f6bb1bceac`
  - binary SHA-256=`2e8dfd644a3c6f12120cb968844a4055d8b894b3c12fa48919695911a5894218`
  - 281 个 TaskSpace request 对账为 149 none、83 sequence、1 TaskSpace protocol、26 state、
    22 ordinary；138 份 sibling copy 有 explicit causal identity
  - 7 个 `node_state_invalid` 均保留 canonical/evaluated/unsatisfied facts；3 次直接纠正，4 次
    `map-request` 先 `read_map` 再纠正
- Correlation keys:
  - matrix run `20260730-064119-500`
  - Docker image `sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`
- Raw content:
  ```json
  {
    "status": "valid",
    "run_count": 24,
    "findings": [],
    "final_aggregate_ready": true
  }
  ```
- Interpretation: W0 的事实链和 fail-closed observer 已在当前 committed candidate 上成立；4 次保守
  `read_map` 是否表示反馈显著性缺口仍需 fresh reviewer 裁决，但不能据此增加 Runtime 建议或自动推进
- Time: 2026-07-30 06:51

## Evidence E-016: Round 3 blocker 修复与 replacement-review 候选矩阵
- Related hypotheses:
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Direction: supports
- Type: fix-validation
- Source:
  - `third_party/codex-cli/codex-rs/core/src/tools/failure_provenance.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/sequence_tests.rs`
  - `scripts/taskspace-benchmark/test-r7-five-layer-trace-analysis.ps1`
  - `scripts/taskspace-benchmark/test-metrics-extractor-harness.ps1`
  - `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/1196b4e99ca507d5cb3bcb619343053463cf752c/20260730-073211-169`
- Prediction or plan link:
  - Round 3 blocking findings 1-3
- Matched signal:
  - 生产 `ToolSearchFailureV3` 使用 `scope=tool_execution`、self cause、精确单 call set 和
    `zero_dispatch=false`；错误 skip scope 被拒绝
  - 真实状态机 complete/reserve 冲突与 ToolSearch sibling 组合路径得到 canonical=`ready`、
    candidate=`completed`、完整 affected call set、零 dispatch 和零 state commit
  - Standard/TaskSpace 和 metrics fixture 均使用生产 namespaced Function MCP 形态
  - cross-request、affected-call subset、duplicate affected call 和重复 supplemental 均 fail-closed
  - 24/24 run 完成且可比较，artifact provenance=`valid`，final aggregate=`finalized`
- Correlation keys:
  - fix commit `1196b4e99ca507d5cb3bcb619343053463cf752c`
  - matrix run `20260730-073211-169`
  - binary SHA-256 `6ad84803fade763843a2dce2297460f5f9e75384997b62d85fe3ad02a8fc5298`
- Raw content:
  ```text
  cargo test -p codex-core --lib: 1931 passed, 0 failed, 3 ignored
  PowerShell gates: 14/14 passed
  matrix: 24/24, provenance valid, final aggregate finalized
  ```
- Interpretation: Round 3 的生产合同漂移和组合测试缺口已在同一候选提交闭合。正式矩阵没有自然产生
  ToolSearch/MCP live call，因此这些载体的行为保证来自生产路径确定性测试和静态接线；replacement
  reviewer 必须判断这是否满足 W0 关闭标准，不能用矩阵未触发来伪造 live 收益
- Time: 2026-07-30 07:47

## Evidence E-017: Round 4 证明 supplemental、typed origin、生产入口和人工统计仍有缺口
- Related hypotheses:
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Direction: challenges
- Type: adversarial-review
- Source:
  - reviewer `Ampere`，session `019fb049-4437-7331-be0e-104759654f9c`
  - `scripts/taskspace-benchmark/lib/r7-supplemental-failure.ps1`
  - `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs`
  - `third_party/codex-cli/codex-rs/core/src/tools/sequence.rs`
  - W0 retained matrix `1196b4e99c/20260730-073211-169`
- Prediction or plan link:
  - GI-005/GI-007 fresh closure review
- Matched signal:
  - malformed/unknown/incomplete supplemental 仍有跳过路径
  - state rejection + ToolSearch 回归没有穿过异步生产入口
  - ToolSearch execution origin 仍可由嵌套错误文本中的 scope 影响
  - 文档声称 7 个状态拒绝、4 个后续 `read_map`，与工件的 13 个、3 个不一致
- Correlation keys:
  - reviewed commit `3b0831208`
- Interpretation: Round 3 修复没有完全闭合 producer/observer/production-entry/documentation 四条证据链；
  四项均接受为 W0 blocker。Reviewer 对 C-10/C-13 的失败判断属于已公开的 GI-006/GI-008，不改变 W0
  范围，但不能从报告中删除。
- Time: 2026-07-30 08:00

## Evidence E-018: Round 4 blocker 修复与 current-commit 正式矩阵
- Related hypotheses:
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Direction: supports
- Type: fix-validation
- Source:
  - commits `2c6d65ddb`、`92cccd7e0`、`50c2b77d1`
  - `scripts/taskspace-benchmark/test-r7-supplemental-failure-evidence.ps1`
  - `third_party/codex-cli/codex-rs/core/src/tools/sequence_taskspace_rejection_tests.rs`
  - `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/50c2b77d199cfa41615f03e97f8dc07e72cd4c74/20260730-083136-712`
- Prediction or plan link:
  - Round 4 blocking findings 1-4
- Matched signal:
  - malformed/unknown/incomplete/duplicate supplemental、非布尔失败和错误 provenance 全部 fail-closed
  - provider-response rejection 与 ToolSearch execution failure 使用不同的 typed 生产分支
  - 异步 `execute_response_tool_sequence` 回归证明 complete/reserve 拒绝发生在 ToolSearch dispatch 前，
    canonical Map、revision、reservations 和 result 均未提交
  - 自动汇总区分 request 与 violation，current matrix 输出 11/11；状态对包含
    `ready->completed`、`absent->waiting`，只有 2 个下一请求 `read_map`
  - 24/24 run 完成且可比较，artifact provenance=`valid`，final aggregate=`finalized`
- Correlation keys:
  - source commit `50c2b77d199cfa41615f03e97f8dc07e72cd4c74`
  - binary SHA-256 `aa9042dbc049ab68560dec219cd03becbeb95534f07bd8137477f5a129d2d660`
  - Docker image `sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`
- Raw content:
  ```text
  cargo test -p codex-core --lib: 1933 passed, 0 failed, 3 ignored
  PowerShell gates: 15/15 passed
  matrix: 24/24, provenance valid, final aggregate finalized
  TaskSpace requests: 292, classification unreconciled runs: 0
  ```
- Interpretation: Round 4 的四个 W0 blocker 已形成代码、负向门禁和 current-commit live artifact 三层证据；
  GI-005/GI-007 仍需由新的无上下文 reviewer 关闭，不能由主线程自证完成。
- Time: 2026-07-30 08:43

## Evidence E-019: Round 5 复现 supplemental 类型 fail-open 与 violation signature 合并
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Direction: challenges
- Type: adversarial-review
- Source:
  - reviewer `Franklin`，session `019fb07d-cd31-7f92-9aa3-db1901e1a5f1`
  - `scripts/taskspace-benchmark/lib/r7-supplemental-failure.ps1`
  - `scripts/taskspace-benchmark/lib/r7-call-evidence.ps1`
  - `scripts/taskspace-benchmark/lib/r7-state-rejection-summary.ps1`
- Prediction or plan link:
  - GI-007 fresh closure review
- Matched signal:
  - malformed payload 在 `schema_version` 不位于首字段时被静默忽略
  - `affected_call_ids` 字符串被 PowerShell 隐式提升为单元素数组并接受
  - 同 node/canonical/candidate state、但 subjects/allowed/predecessors 不同的两条 violation 被汇总为一条
- Correlation keys:
  - reviewed HEAD `ddab5f12c`
  - production code `50c2b77d1`
- Raw content:
  ```json
  {
    "malformed_reordered": {
      "classification_reconciled": true,
      "evidence_health": "valid"
    },
    "scalar_affected_call_ids": {
      "supplemental_count": 1,
      "evidence_valid": true
    },
    "input_violation_count": 2,
    "reported_violation_count": 1
  }
  ```
- Interpretation: 前一轮只覆盖了首字段截断和不同状态对，没有证明字段顺序、JSON 类型与同状态对独立事实；
  两项均违反 C-03/C-14，接受为 GI-007 blocker。
- Time: 2026-07-30 09:08

## Evidence E-020: 严格 supplemental 合同、显式 violation copy identity 与新正式矩阵
- Related hypotheses:
  - H-003
  - H-004
  - H-005
- Direction: supports
- Type: fix-validation
- Source:
  - commit `e72637d070764c2f2de03a978761a6739780f37b`
  - `scripts/taskspace-benchmark/test-r7-supplemental-failure-evidence.ps1`
  - `scripts/taskspace-benchmark/test-r7-request-observability-report.ps1`
  - `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/e72637d070764c2f2de03a978761a6739780f37b/20260730-091713-589`
- Prediction or plan link:
  - Round 5 blocking findings 1-2
- Matched signal:
  - 保留 schema family 的 malformed 识别不依赖字段顺序
  - status/success/provenance/error/call identity 使用严格 object/array/string/boolean 与值合同
  - scalar/object/boolean/array 混淆、错误 status/class、未知/残缺/重复 payload 全部 fail-closed
  - 状态汇总只按显式 copy group/affected calls 合并 sibling，完整保留 subjects、allowed states、
    canonical/candidate predecessor sets 和原始 ordinal
  - 负向 fixture 中同状态对的 3 条独立 violation 全部保留，两个 sibling carrier 只计一次
  - 24/24 run finalized，23/24 business success 原样保留，24/24 comparison eligible；
    289 个 TaskSpace request 全部对账
- Correlation keys:
  - matrix run `20260730-091713-589`
  - binary SHA-256 `aa9042dbc049ab68560dec219cd03becbeb95534f07bd8137477f5a129d2d660`
  - Docker image `sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`
- Raw content:
  ```text
  PowerShell gates: 15/15 passed
  matrix: 24/24 finalized, provenance valid
  TaskSpace taxonomy: none=152, sequence=82, state=34, ordinary=21
  node_state_invalid: 11 requests, 11 violations, 3 next-request read_map
  ```
- Interpretation: 新 observer 在不修改 Rust Runtime、Agent prompt、Tool schema 或 projection 的前提下收紧证据
  完整性；真实 Agent 失败没有被删除。GI-007 仍需新的无上下文 reviewer 关闭。
- Time: 2026-07-30 09:28

## Evidence E-021: Round 6 发现四类证据身份缺口
- Related hypotheses:
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Direction: challenges
- Type: adversarial-review
- Source:
  - reviewer `Curie`，session `019fb0a6-6602-7c90-b655-9d4048351510`
  - `scripts/taskspace-benchmark/lib/r7-supplemental-failure.ps1`
  - `scripts/taskspace-benchmark/lib/r7-request-observability.ps1`
  - `scripts/taskspace-benchmark/lib/r7-artifact-provenance.ps1`
- Prediction or plan link:
  - GI-007 fresh closure review
- Matched signal:
  - schema/root array 与 unicode escaped malformed supplemental 可以绕过保留 family gate
  - provider、skip、bound provenance 没有绑定精确 producer identity
  - request path 丢弃 output/reasoning/total token，无法证明 join 与聚合守恒
  - resolved manifest 只验证封存哈希，没有验证四臂内容身份
  - GI-005 的 11 个状态拒绝被独立重算为机械事实完整，3 次 `read_map` 均发生在正确理解之后
- Correlation keys:
  - reviewed HEAD `905f8adf3`
  - production code `e72637d07`
- Interpretation: GI-005 的反馈语义未发现新缺口，但 GI-007 仍可能允许错误证据进入完整报告；四项均接受为
  C-01/C-03/C-14 blocker。
- Time: 2026-07-30 09:55

## Evidence E-022: 证据身份修复与 current-commit 正式矩阵
- Related hypotheses:
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
- Direction: supports
- Type: fix-validation
- Source:
  - commit `2005442d34416820a888dc7395ac8ba1b3812635`
  - `scripts/taskspace-benchmark/test-r7-supplemental-failure-evidence.ps1`
  - `scripts/taskspace-benchmark/test-r7-provider-token-identity.ps1`
  - `scripts/taskspace-benchmark/test-r7-resolved-manifest-identity.ps1`
  - `target/r7-five-layer-matrix/r7-five-layer-evaluation-contract-v1/2005442d34416820a888dc7395ac8ba1b3812635/20260730-101516-783`
- Prediction or plan link:
  - Round 6 blocking findings 1-4
- Matched signal:
  - escaped-key malformed、root/schema array、非 object root 与严格 schema 类型全部 fail-closed
  - provider、ToolSearch、skip、bound 的 copy/cause/call/reservation/scope/zero-dispatch identity 精确校验
  - 347 个 request 的 input/cached/output/reasoning/total token 通过类型、算术和 run aggregate 对账
  - 24 个 resolved manifest 逐项通过 sample/repeat/side/mode/policy/model/binary/provider/capability 校验；
    每个 sample-repeat 恰有四臂，且 prompt/fixture/model/image/binary 相同
  - 24/24 finalized、24/24 business success、artifact provenance=`valid`、findings=0
  - 5 个状态拒绝请求、5 个 violation、2 个下一请求 `read_map` 由报告自动生成
- Correlation keys:
  - matrix run `20260730-101516-783`
  - binary SHA-256 `cff8ad0235c41b778e3ba8638339567357d6b31923cc496be92509420cf46c55`
  - Docker image `sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`
- Raw content:
  ```text
  cargo test -p codex-core --lib: 1933 passed, 0 failed, 3 ignored
  PowerShell gates: 14/14 scripts passed
  matrix: 24/24 finalized, provenance valid, findings=0
  request tokens: input=7947276 cached=3519488 output=135380 reasoning=54288 total=8082656
  ```
- Interpretation: 修复只收紧 observer、artifact identity 和日志完整性，没有改动 Runtime 决策、Tool schema、
  Agent prompt 或 projection；W0 仍需 Curie 之外的 fresh reviewer 关闭。
- Time: 2026-07-30 10:28

## Hypothesis H-008: W0 证据链仍会接受未绑定 output、错误 token 类型或脏工作区候选
- Status: confirmed
- Parent: P-001
- Claim: 当前 producer/observer/artifact authority 只校验了部分身份字段，没有证明 provider supplemental
  就是对应 Tool output、所有 token 都保持精确整数身份，也没有证明当前工作区及评估脚本字节与候选提交一致
- Layer: evidence-authority
- Factor relation: combined
- Depends on:
  - H-003
  - H-004
  - H-005
- Falsifiable predictions:
  - If true: supplemental 可先于 output 或覆盖成功 output，字符串零 token 可成为 complete observation，
    当前 dirty worktree 或提交外脚本字节不会让 provenance 失败
  - If false: 三类构造都会稳定产生 invalid/rejected，且不能进入 finalized report
- Diagnostic evidence plan:
  - Prediction or clause under test: provider output 顺序与内容、token CLR 类型、current Git identity
  - Signal: reconstructed call 的 output/supplemental 次序和原文、observation status/eligibility、provenance findings
  - Capture method: 最小 PowerShell 构造、生产代码路径检查和 fresh reviewer 独立复现
  - Correlation keys:
    - request index
    - call ID
    - metric path
    - candidate commit
  - Supports if:
    - supplemental-before-output 或 successful-output-after-supplemental 仍 evidence_valid
    - string `"0"` token 返回 complete/eligible
    - current worktree clean 不参与候选一致性判断
  - Refutes if:
    - 任一构造被既有合同 fail-closed
  - Instrumentation status: permanent-observer-change-required
  - Instrumentation lifecycle:
    - 稳定错误码、精确 output/token identity 和 Git blob identity 永久保留
- Evidence gate: satisfied
- Related evidence:
  - E-023
  - E-024
- Conclusion: 三项预测均被 reviewer 与本地独立路径支持；根因不是 Agent 智能或 Runtime 状态机，
  而是评估证据构造器没有完成精确身份绑定
- Repair design readiness: repair-authorized
- Next step: 分别收紧 output、token 和 candidate authority，并以负向 fixture 与正式矩阵验证
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-023: Round 8 fresh review 发现三类真实 fail-open
- Related hypotheses:
  - H-003
  - H-004
  - H-005
  - H-008
- Direction: challenges
- Type: adversarial-review
- Source:
  - reviewer `Jason`，session `019fb136-da88-7031-bbd4-a723998d9aff`
  - reviewed HEAD `f167616ff`
  - formal matrix `a677374782ff4ac15ad3242af19e1a681fec7e08/20260730-114506-875`
- Prediction or plan link:
  - GI-005/GI-007 fresh closure review
- Matched signal:
  - provider supplemental 可在 output 前写入并覆盖后续成功 output
  - string `"0"` 的 cached/uncached/output token 被归一成 null，但 observation 仍 complete/eligible
  - current dirty worktree 未进入 binary/provenance 一致性条件，报告与 authority 文件未绑定候选提交 blob
  - reviewer 独立重算 24/24 run、360 request、token 汇总和 192/192 seal 均与报告一致
  - GI-005 被独立判定 PASS；GI-007 因上述证据边界保持 BLOCKED
- Correlation keys:
  - reviewer session
  - candidate commit
  - matrix run
- Interpretation: 当前正式矩阵本身可以复算，但生成同类“有效”工件的入口仍允许错误证据；三项接受为
  C-01/C-03/C-14 blocker。Reviewer 要求所有 business-incomplete run 不得 finalized 的建议不接受，
  因为 C-14 要求忠实保留 Agent 失败，finalized 表示证据已封存而非任务全部成功
- Time: 2026-07-30 12:08

## Evidence E-024: 主线程最小构造复现 output 与 token 缺口，并确认 Git 条件遗漏
- Related hypotheses:
  - H-008
- Direction: supports
- Type: reproduction
- Source:
  - `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1`
  - `scripts/taskspace-benchmark/lib/performance-observation.ps1`
  - `scripts/taskspace-benchmark/lib/harness-health.ps1`
  - `scripts/taskspace-benchmark/lib/r7-artifact-provenance.ps1`
- Prediction or plan link:
  - H-008 的三条支持条件
- Matched signal:
  - 两个 call 先应用 provider supplemental、再应用成功 output 后，均得到
    `success=false`、`output_count=1`、`supplemental_count=1`、`evidence_valid=true`
  - input token 为整数 100、其他 token 为字符串 `"0"` 时，返回
    `observation_status=complete`、`comparison_eligible=true`，三个字符串 token 为 null
  - binary attestation 的 `gitMatches` 只比较 marker 中的 clean/head/tree，没有比较当前
    `gitIdentity.worktree_clean`；artifact provenance 也只取 current HEAD
  - inner run 的 `final_aggregate_ready=false` 是外层矩阵禁用通用 aggregate 的结果，外层
    `matrix-final-status.final_aggregate_ready=true` 是独立封存状态；已有测试有意保留 Agent incomplete row
- Correlation keys:
  - call ID
  - temporary metric path
  - candidate commit
- Raw content:
  ```text
  provider repro: output_count=1 supplemental_count=1 evidence_valid=true success=false
  token repro: input=100 cached=null uncached=null output=null status=complete eligible=true
  git condition: current worktree_clean absent from gitMatches
  ```
- Interpretation: H-008 达到修复证据门。修复必须只收紧证据完整性，不能把 Agent 业务失败误判为
  基础设施失败，也不能改动 Runtime/Tool/Map 语义
- Time: 2026-07-30 12:18

## Evidence E-025: 正式矩阵证明 serialized Tool output 的显式 success 事实未被 observer 读取
- Related hypotheses:
  - H-008
- Direction: supports
- Type: fix-validation
- Source:
  - candidate commit `f2dea47652e635543c305cd1905a964c7c840a81`
  - matrix run `20260730-124902-146`
  - `scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1`
- Prediction or plan link:
  - H-008 provider output 精确绑定的 current-commit 验证
- Matched signal:
  - 24/24 raw run 完成，但 report 在 call
    `call_00_HICyAiW9DnYHU3uPTy4I5404` 的 exact output gate fail-closed
  - 三个 Function output 与 developer supplemental 的 JSON 原文逐字相同
  - JSON payload 内含显式布尔 `success:false`
  - `Get-R7ResponseItemOutcome` 只对 object output 读取 `success`；生产 rollout 的 output 是 serialized
    JSON string，因此回退到 Shell/apply_patch 文本启发式并误判 `tool_success=true`
- Correlation keys:
  - matrix commit
  - matrix run
  - call ID
- Raw content:
  ```text
  output: {"schema_version":"ToolSequencePreflightResultV3",...,"success":false,...}
  supplemental: same ordinal JSON text
  report: Provider-response failure does not match its Tool output
  ```
- Interpretation: exact-output 合同本身正确，不应放宽；缺口位于 observer 对生产 serialized structured
  output 的机械解码。修复只读取显式 bool `success`，不得从任意文本再解释成功语义
- Time: 2026-07-30 13:02

## Evidence E-026: Round 9 复算通过但四类事实入口仍可 fail-open
- Related hypotheses:
  - H-008
- Direction: supports
- Type: adversarial-review
- Source:
  - reviewer `Mencius`，session `019fb16d-c10a-7082-b81a-7c328f19e9da`
  - candidate `ebaedcf6c6c641a0d4361d68ab9c99fc0c594f22`
  - matrix run `20260730-125939-756`
- Prediction or plan link:
  - GI-005/GI-007 Round 9 closure review
- Matched signal:
  - retained matrix 的 24 run、385 request、token 守恒和 192 raw seal 独立复算一致
  - 缺失 state violations 的 typed failure 仍被标记 valid
  - count 可接受 fraction/string 并在 `2^53` 以上丢失精度，缺失值在 report 默认成 0
  - 唯一问题清单和 W0 结果仍引用上一份 `a677374` 矩阵
  - affected call IDs 交换顺序后仍可通过 supplemental provenance
- Correlation keys:
  - reviewer session
  - candidate commit
  - matrix run
- Interpretation: 单个 sealed 工件自洽不等于同类事实入口可靠。四项均接受为
  C-01/C-03/C-09/C-12/C-13/C-14 blocker；根因仍位于事实构造与验证链，不应通过 Runtime 约束
  Agent 修复
- Time: 2026-07-30 13:35

## Evidence E-027: Round 9 四项 blocker 的确定性反例已转为回归门
- Related hypotheses:
  - H-008
- Direction: challenges
- Type: fix-validation
- Source:
  - commit `ea6f27b1b807d12041a736d28ec1573db32d3ffe`
  - `test-r7-state-failure-contract.ps1`
  - `test-r7-supplemental-failure-evidence.ps1`
  - `test-performance-observation.ps1`
- Matched signal:
  - state rejection 缺失 violations 或 candidate allowed states 时 evidence invalid
  - supplemental affected call 顺序交换时 fail-closed
  - string/fraction/负数/越界 count 被拒绝，缺失必需 count 不再默认零
  - invalid count 产生 `performance_count_identity_invalid` 稳定事件
- Interpretation: 修复只收紧机械事实结构、类型与顺序，没有增加 Runtime 对 Agent 语义或状态推进的干预；
  仍需 current-commit 正式矩阵和 fresh replacement reviewer 才能关闭 H-008
- Time: 2026-07-30 13:36

## Evidence E-028: post-repair 正式汇总发现普通 Tool 机械退出码形状遗漏
- Related hypotheses:
  - H-003
  - H-008
- Direction: supports
- Type: live-failure-and-fix
- Source:
  - failed report attempt
    `905d7fdd5883856945ffd83fd3cde6864fdbcb24/20260730-134259-002`
  - commit `ab2595847238443a33795652174a744ef2dfd093`
  - `test-r7-five-layer-trace-analysis.ps1`
- Matched signal:
  - 真实 `apply_patch` 失败返回
    `metadata.execution_outcome=exited` 与 `metadata.shell_exit_code=1`
  - observer 只识别 `exit_code`，将完整普通 Tool 失败标为
    `evidence_unclassified/tool_failed_unclassified`
  - report 因 classification 不守恒 fail-closed，没有封存错误矩阵
  - 修复后同一 payload 机械分类为 `ordinary_tool/shell_exit_1`
- Correlation keys:
  - request 18
  - call `call_01_R5aAkNcd0eajnXG5ixYk7508`
  - failed candidate matrix
- Interpretation: 根因是反馈观测器遗漏显式运行时字段，不是 Agent 幻觉，也不是 Runtime 状态约束不足。
  修复不解释 Patch 文本，只读取既有退出码
- Time: 2026-07-30 14:04

## Evidence E-029: current-candidate 正式矩阵通过严格事实链
- Related hypotheses:
  - H-003
  - H-004
  - H-005
  - H-008
- Direction: challenges
- Type: fix-validation
- Source:
  - candidate `ab2595847238443a33795652174a744ef2dfd093`
  - matrix run `20260730-135544-601`
- Matched signal:
  - 24/24 observation complete、business success、comparison eligible
  - 354 provider request 的一级分类全部守恒
  - input `8,294,969`、cached `3,609,344`、uncached `4,685,625`、
    output `143,899`、reasoning `61,774`、total `8,438,868`
  - 16 个 `node_state_invalid` request 完整保留 16 份 violation，2 个下一请求为 `read_map`
  - 192 个证据工件逐文件 hash 复算无偏差，final aggregate finalized
- Correlation keys:
  - candidate commit
  - matrix run
  - artifact manifest
- Interpretation: 当前候选已具备进入 fresh closure review 的生产、observer、report 和 seal 证据；
  该结果不自行关闭 H-008，也不证明 request、缓存或 Agent 行为问题已解决
- Time: 2026-07-30 14:12

## Evidence E-030: Round 10 复算通过但四个事实入口仍可丢失或放宽
- Related hypotheses:
  - H-008
- Direction: supports
- Type: adversarial-review
- Source:
  - reviewer `Euclid`，session `019fb1a2-b8cd-7fe2-9430-91e90a6a1a10`
  - candidate `ab2595847238443a33795652174a744ef2dfd093`
  - matrix run `20260730-135544-601`
- Matched signal:
  - retained matrix 的 24 run、354 request、token 守恒和 192 raw seal 独立复算一致
  - direct Runtime rejection 未复用完整 state violation serializer
  - structured ordinary exit 可从错误 scope、fraction/string/duplicate/malformed JSON 进入
  - direct `TaskSpaceControlResultV2` 可用残缺 non-state failure envelope 通过
  - 普通 count 经 double 求和后无法区分 `2^53` 与 `2^53+1`
- Interpretation: sealed 工件自洽不代表同类输入均被忠实构造和验证。四项接受为
  C-01/C-03/C-09/C-12/C-13/C-14 blocker；修复目标是机械事实链，不是增加 Runtime 语义控制
- Time: 2026-07-30 14:35

## Evidence E-031: Round 10 修复及 current-candidate 矩阵通过
- Related hypotheses:
  - H-003
  - H-004
  - H-005
  - H-008
- Direction: challenges
- Type: fix-validation
- Source:
  - candidate `6e487f0578be834afc178a7a377393383346c296`
  - matrix run `20260730-145945-966`
  - `test-r7-ordinary-tool-outcome.ps1`
  - `test-r7-state-failure-contract.ps1`
  - `test-r7-exact-counts.ps1`
- Matched signal:
  - direct Runtime/lifecycle 共用完整 model-visible violation serializer
  - ordinary exit 仅接受完整 JSON 的顶层唯一正 Int64 `metadata.shell_exit_code`
  - direct control failure 必须满足完整 typed envelope 和 failure 语义
  - request/report/provenance/duplication count 使用 BigInt 中间和 Int64 边界，`2^53+1` 精确保留
  - Rust 1934 passed、0 failed、3 ignored；workspace check 与 locked build 通过
  - 24/24 observation complete、business success、comparison eligible
  - 298 provider request 分类守恒；input `6,322,711`、cached `2,970,624`、uncached `3,352,087`、
    output `127,666`、reasoning `58,076`、total `6,450,377`
  - 12 个 state rejection request 完整保留 12 份 violation，2 个下一请求为 `read_map`
  - 192 个 raw artifact hash 独立复算无偏差，final aggregate finalized
- Interpretation: Round 10 的确定性反例均已转为回归门，且 current candidate 具备 fresh replacement
  review 条件；该证据不自行关闭 H-008，也不证明 GI-002/GI-006/GI-008 的行为和成本问题已解决
- Time: 2026-07-30 15:20

## Evidence E-032: Round 13 分块 review 使通过子门与剩余 blocker 独立可归因
- Related hypotheses:
  - H-006
  - H-007
  - H-008
- Direction: supports
- Type: adversarial-review
- Source:
  - Shard A `Hume`：`019fb2ae-8c6d-7401-bdb4-2fbd0c5065cc`
  - Shard B `Huygens`：`019fb2ae-c0c5-78f1-a560-3eed1b9201d6`
  - Shard C `Pascal`：`019fb2ae-fb1a-7bc2-8299-dcb6149e5276`
  - Shard D `Gauss`：`019fb2b6-afaf-7550-9614-4d36af63196a`
  - Shard H `Tesla`：`019fb2c7-c7e6-7523-ac9c-2ac46355d8f1`
  - candidate `6e487f0578be834afc178a7a377393383346c296`
  - matrix run `20260730-145945-966`
- Matched signal:
  - A：typed state facts、canonical/evaluated scope、ToolSearch 和 2 次 read 归因通过；lifecycle
    `actual.violations` 与 `actual.condition.violations` 重复同一事实
  - B：ordinary exit 全部正反门通过；direct JSON 重复属性、非法 action/shape、outer success 覆盖 inner
    failure 和普通 Tool schema 误分类可复现
  - C：shared exact integer helper 通过；provenance file bytes 仍接受 string/fraction/zero-byte missing
  - D：24 run、298 request、token taxonomy、12 state rejection 和 192 raw hashes 独立复算无偏差
  - H：226/238 TaskSpace request 的合法 L5 projection/control feedback 被误计为第三个静态 system；
    freshness 能发现 mismatch，但 retained observation/report 没有消费该 gate
- Interpretation: 当前 retained artifact 内容自洽与事实入口完备是两个独立验收面。前者通过不能覆盖后者；
  GI-005/GI-007 继续开放，但修复不得回退 ordinary exit、state scope、ToolSearch 或 raw seal 子门
- Time: 2026-07-30 19:37

## Hypothesis H-009: direct failure adapter 的宽松解析破坏单一 carrier 合同
- Status: confirmed
- Parent: P-001
- Claim: `Get-R7CallOutcome` 在同一个 direct failure 入口中依次存在 last-wins JSON、外层 success
  早返回、任意 schema 进入 TaskSpace 分支，以及 envelope 跨字段校验不足；四者共同使 observer
  无法忠实区分有效 TaskSpace failure、矛盾 carrier 和普通 Tool 失败。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-008
- Rationale:
  - Round 13 Shard B 已给出四个最小反例，当前代码仍对应相同控制流。
- Falsifiable predictions:
  - If true: duplicate direct JSON、非法 action、outer success + inner failure 会被标记为有效，普通 Tool
    自有 schema 会被误记为 TaskSpace untrusted carrier。
  - If false: 四类输入均能由当前入口正确拒绝或隔离。
- Diagnostic evidence plan:
  - Prediction or clause under test: 在当前 HEAD 直接调用 `Get-R7CallOutcome` 重放四类最小输入。
  - Signal: `success`、`evidence_valid`、`parse_status`、`failure_class`、`failure_code`、`state_commit`。
  - Capture method: 内存 PowerShell probe，并运行现有 trace/state/supplemental 三组回归。
  - Event name or marker:
    - `r71_direct_failure_evidence`
  - Correlation keys:
    - case name
  - Differentiates from:
    - Runtime output 缺失
    - ordinary Tool 执行失败
  - Supports if:
    - 四个 probe 结果与预测一致，且旧回归仍全部通过。
  - Refutes if:
    - 任一反例已被严格分类，或 carrier 原文没有进入 observer。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 将稳定 `carrier_schema/parse_status/reason_code` 保留为 R71-01 Phase evidence。
- Evidence gate: satisfied
- Related evidence:
  - E-033
  - E-034
- Conclusion: 四项表现来自同一个 observer contract boundary，R71-01 可按一个原子 Phase 修复。
- Repair design readiness: repair-authorized
- Next step: 严格 JSON、carrier identity、outer/inner 一致性和 envelope validator 一次收敛。
- Blocker:
  - none
- Close reason:
  - user authorized R71-01 implementation after evidence-backed reproduction

## Evidence E-033: 当前 direct failure 调用链保留四个宽松入口
- Related hypotheses:
  - H-009
- Direction: supports
- Type: code-location
- Source:
  - `scripts/taskspace-benchmark/lib/r7-call-evidence.ps1`
  - `scripts/taskspace-benchmark/lib/r7-state-failure-contract.ps1`
  - [Microsoft ConvertFrom-Json 文档](https://learn.microsoft.com/powershell/module/microsoft.powershell.utility/convertfrom-json)
- Prediction or plan link:
  - H-009 的控制流预测
- Matched signal:
  - `ConvertFrom-Json` 对重复属性使用最后一个值；
  - `ToolSuccess` 在 structured payload 校验前直接返回；
  - 任意非空 `schema_version` 进入 TaskSpace carrier 分支；
  - action 只要求非空，`actual/expected` 只检查属性存在。
- Correlation keys:
  - `Get-R7CallOutcome`
  - `Get-R7ControlFailureEnvelopeShapeError`
- Raw content:
  ```text
  ConvertFrom-Json: duplicate keys keep only the last key
  Get-R7CallOutcome: if ($ToolSuccess) { return success }
  schema gate: if (-not IsNullOrWhiteSpace($schemaVersion)) { ... }
  action gate: null or non-empty string
  ```
- Interpretation: 代码路径足以解释四个反例，但仍需当前 HEAD 的直接运行证据。
- Time: 2026-07-31 07:34

## Evidence E-034: 当前 HEAD 确定性复现四类 direct carrier 缺口
- Related hypotheses:
  - H-009
- Direction: supports
- Type: reproduction
- Source:
  - 当前提交 `3f466a989`
  - 内存 PowerShell probe
  - `test-r7-five-layer-trace-analysis.ps1`
  - `test-r7-state-failure-contract.ps1`
  - `test-r7-supplemental-failure-evidence.ps1`
- Prediction or plan link:
  - H-009 四条支持条件
- Matched signal:
  - duplicate JSON：`evidence_valid=true/structured_failure`；
  - 非法 action：`evidence_valid=true/structured_failure`；
  - outer success + inner failure：`success=true/evidence_valid=true/state_commit=false`；
  - ordinary schema：`taskspace_failure_untrusted_carrier`，未进入 ordinary exit；
  - 三组旧回归均 exit 0。
- Correlation keys:
  - `duplicate_json`
  - `invalid_action`
  - `outer_success_inner_failure`
  - `ordinary_schema`
- Raw content:
  ```text
  duplicate_json: false / true / structured_failure
  invalid_action: false / true / structured_failure
  outer_success_inner_failure: true / true / success / state_commit=false
  ordinary_schema: false / false / untrusted_structured_failure_carrier
  existing tests: 3/3 PASS
  ```
- Interpretation: H-009 达到修复门；旧测试通过证明当前缺口尚未被持久化回归覆盖。
- Time: 2026-07-31 07:34
