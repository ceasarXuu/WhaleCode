# Problem P-001: VA-02 首轮证据链未能如实结算
- Status: verifying
- Created: 2026-08-10 06:00
- Updated: 2026-08-11
- Objective: 保留 VA-02 首轮的真实协议、Map 和成本证据，并在不放宽 TaskSpace 硬约束的前提下修复确定的观测缺陷。
- Symptoms:
  - 首次 `taskspace_exec` 参数多出一个右花括号，被严格 JSON 解码拒绝。
  - 第二次 `taskspace_exec` 初始化 Map 并成功执行 `exec_command`，但旧 Map 管理观测器尝试读取不存在的 `node.results[].resultId`。
  - 两次已越过 provider 边界且含完整 usage 的响应，因为第三次本地尝试在代理预算门禁前失败而被整体判为 usage 不可用。
- Expected behavior:
  - 非法 Exec envelope 在执行副作用前被拒绝；合法重试可按原生工具语义执行。
  - benchmark 只消费 R8 canonical Map 模型，不运行已淘汰的结果引用和压缩模型。
  - provider 成本按实际越过边界的请求结算；本地失败尝试不伪造为 provider 成本，也不被隐藏。
- Actual behavior:
  - 协议拒绝和合法重试行为正确，但旧观测器报空 ID，runner 又把完整的两次 provider usage 判为零。
- Impact:
  - VA-02 首轮不能形成可信的成本与缓存 observation，剩余一次授权运行不能直接开始。
- Reproduction:
  - 运行 `WAR-20260810-044303-CACHE-REGRESSION-417B0312`。
- Environment:
  - Linux；分支 `whalecode-alpha`；subject commit `982131f279696ba184fc385ee26ef4afff6ac6b5`；`deepseek-v4-flash`；`map-request`。
- Known facts:
  - 首次参数长度 545，尾部为 `}}`；第二次同形参数长度 544，尾部为 `}`，并成功执行。
  - request facts 记录 3 次本地尝试、2 次 provider boundary、2 次完成响应和 2 份完整 usage。
  - canonical Map 只有 `node.role` 和 `node.actions[]`；旧观测器仍读取 `node.kind` 和 `node.results[]`。
- Ruled out:
  - 不是 DeepSeek 无法调用 `taskspace_exec`；第二次调用已成功完成 Map 初始化和嵌套原生工具执行。
  - 不是 provider usage 缺失；两次已完成响应共含 input 28,060、cached input 7,936、output 588。
- Fix criteria:
  - 已完成 provider 请求可精确离线结算，失败本地尝试仍单独可见。
  - benchmark 不再消费已淘汰的旧 Map result/ref/compaction 模型。
  - 严格 JSON 解码保持不变；剩余一次真实运行只用于观察生产形状与偶发格式错误是否重复。
- Current conclusion: Dedicated TaskSpace base 已消除顶层 client Tool 逃逸，Source 已退役。I05 修复后的在线复验再次出现首请求少一个闭合括号，但新反馈准确说明 syntax、direct `calls` 和零执行，Agent 下一请求立即纠正且未再产生 wrapper 重复。随后正确 patch、3 项测试和完整 Map 均完成，canonical parent handoff 在线可执行。最终自然语言回复所需的第 9 次本地请求被 8-request 授权边界在 Provider 前截断，因此端到端仍为 partial。I03、I04 继续开放；I07 新发现 nested patch 漏计。整体保持 verifying。
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
  - H-010
  - H-011
  - H-012
  - H-013
  - H-014
  - H-015
  - H-016
  - H-017
  - H-018
- Resolution basis:
  - E-001 至 E-031；H-012 确认顶层逃逸消失，H-013 的交接合同已在线走通，H-015 的反馈修复已在线验证；H-016～H-018 分别坐实正式上下文自愈接缝、waiting 事实反馈和 nested patch 观测缺口。
- Close reason:
  - not closed

## Hypothesis H-001: 首次 Exec 失败来自模型生成的非法 outer JSON
- Status: confirmed
- Parent: P-001
- Claim: provider 返回的首次 function arguments 本身不是合法 JSON，错误集中在无 Hosted output 时仍要求填写的 `hosted_bindings: []` 附近；严格解析器因此在副作用前拒绝，而不是 Runtime 截断或扭曲参数。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 首轮首次调用尾部多一个字符；第二轮首次调用把 `hosted_bindings` 错放进尚未闭合的 `calls` 数组；两轮第二响应均自行修正。
- Falsifiable predictions:
  - If true: rollout 原始 `arguments` 首次以 `]}}` 结束，第二次以 `]}` 结束。
  - If false: 两次原始字符串都应是合法 JSON，错误应发生在后续转换层。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较 provider 原始 function-call arguments，不经过 Runtime 重序列化。
  - Signal: 字符串长度、尾部和严格 JSON 解析结果。
  - Capture method: 对原始 rollout JSONL 做只读结构化查询。
  - Event name or marker:
    - `function_call`
  - Correlation keys:
    - `call_00_8rnADvI4W3txyF5MixyK3681`
    - `call_00_3ngwIaUxhzMNAQNphpNZ3971`
  - Differentiates from:
    - Runtime 参数重写或解码器丢失。
  - Supports if:
    - 首次原始参数多一个 `}`，第二次原始参数可解析并成功执行。
  - Refutes if:
    - 原始参数相同且都可解析。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed recurrent；保持 fail-closed，不加入语义容错。共同结构诱因已收敛到无 Hosted output 时的必填空数组。
- Repair design readiness: implemented offline
- Next step: 缓存敏感面门禁通过后，申请新的最小真实预算复验首响应；通过前不启动 VA-03。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Map 观测错误来自已淘汰模型的残留消费者
- Status: confirmed
- Parent: P-001
- Claim: `map-management.ps1` 仍按旧 `node.results/resultId/kind` 模型生成压缩指标，与 R8 canonical `node.actions/actionId/role` 不兼容，且其 retention/compaction 输出已无新产品消费者。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 每个无 `results` 属性的新节点都会被 PowerShell 循环解释为空 result，并传入空 `Id`。
- Falsifiable predictions:
  - If true: 对真实 observability JSON 离线调用该函数会为 3 个节点各报一次空 `Id`。
  - If false: 观测器应直接读取 `actions`，或对缺失 `results` 无错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: 用首轮 canonical Map artifact 单独执行旧观测函数。
  - Signal: `New-TaskspaceManagedItem` 的参数绑定错误及调用行。
  - Capture method: 离线 PowerShell probe 和静态 caller 审计。
  - Event name or marker:
    - `Cannot bind argument to parameter 'Id'`
  - Correlation keys:
    - `WAR-20260810-044303-CACHE-REGRESSION-417B0312-CACHE-001`
  - Differentiates from:
    - canonical Map 自身存在空 node ID。
  - Supports if:
    - Map 的 node ID 均非空，但旧 `results` 循环稳定报错。
  - Refutes if:
    - canonical Map 含空 node ID。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: confirmed；旧 benchmark 管理链已删除，没有把新模型适配回旧语义。
- Repair design readiness: implemented
- Next step: 剩余真实运行确认 runner 不再在 Map 观测后处理中断。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: usage 判零来自把本地失败尝试混入 provider 成本合同
- Status: confirmed
- Parent: P-001
- Claim: usage parser 要求所有本地 wire 尝试均成功，而没有按 provider boundary identity 选择实际产生 provider 成本的请求，导致两份完整 usage 被第三次本地门禁失败整体否定。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - canonical request facts 已明确区分 3 次 local attempt 与 2 次 boundary request。
- Falsifiable predictions:
  - If true: 两个 boundary request ID 都有 `response_completed` 和 usage；唯一 `response_failed` ID 不在 boundary ID 集合中。
  - If false: 失败请求也已越过 boundary，或已完成请求缺失 usage。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐 request facts、provider boundary 和 provider wire terminal identity。
  - Signal: 三组 request ID、boundary status、terminal status 和 usage。
  - Capture method: 离线解析现有 JSONL/JSON artifact。
  - Event name or marker:
    - `payload_captured`
    - `response_completed`
    - `response_failed`
  - Correlation keys:
    - session `019fe844-50b0-7531-9114-462658f4f031`
  - Differentiates from:
    - provider 已收费但错误响应无 usage 的部分成本情况。
  - Supports if:
    - boundary ID 集合恰好等于两条完整 terminal usage 的 ID 集合。
  - Refutes if:
    - 任一 boundary ID 无 terminal usage。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 identity 对账作为缓存 runner 硬合同。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: confirmed；usage 与 cache 聚合已改为 provider boundary identity 范围，本地失败继续保留在 request facts。
- Repair design readiness: implemented
- Next step: 结算首轮账本，并在剩余真实运行复验完整 runner 路径。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 首次原始参数多出一个右花括号
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: 首轮 `rollout.jsonl`
- Prediction or plan link:
  - H-001 原始参数尾部预测。
- Matched signal:
  - 首次长度 545、尾部 `]}}`；第二次长度 544、尾部 `]}`。
- Correlation keys:
  - 两个 outer call ID
- Raw content:
  ```text
  first:  ... "hosted_bindings": []}}
  second: ... "hosted_bindings": []}
  ```
- Interpretation: 非法字符来自模型响应，严格拒绝没有扭曲语义。
- Time: 2026-08-10 06:00

## Evidence E-002: 合法重试完成 Map 初始化和 client tool 执行
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: 首轮 `rollout.jsonl` 的第二个 `taskspace_exec_result`
- Prediction or plan link:
  - H-001 对第二次合法参数可执行的预测。
- Matched signal:
  - `status=completed`、`exec_command outcome=succeeded`、Map revision 2。
- Correlation keys:
  - `call_00_3ngwIaUxhzMNAQNphpNZ3971`
- Raw content:
  ```text
  taskspace_exec_result status=completed client_result_count=1 outcome=succeeded
  ```
- Interpretation: outer Function Call 生产路径已经可用。
- Time: 2026-08-10 06:00

## Evidence E-003: 旧观测函数在三个有效节点上生成三个空 result
- Related hypotheses:
  - H-002
- Direction: supports
- Type: probe
- Source: `Get-TaskspaceMapManagedItems` 对真实 `action-map-observability.json` 的离线调用
- Prediction or plan link:
  - H-002 的稳定空 ID 预测。
- Matched signal:
  - root、inspect、finish ID 均非空；函数在 line 133 连续三次报告 `Id` 为空。
- Correlation keys:
  - Map `map-019fe844-50b0-7531-9114-462658f4f031`
- Raw content:
  ```text
  Cannot bind argument to parameter 'Id' because it is an empty string.
  ```
- Interpretation: 失败来自旧消费者，不是 canonical Map 数据损坏。
- Time: 2026-08-10 06:00

## Evidence E-004: provider 边界与完整 usage 集合一致
- Related hypotheses:
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: `request-facts.json` 与 `provider-boundary-evidence.json`
- Prediction or plan link:
  - H-003 的 identity 集合预测。
- Matched signal:
  - boundary=2、completed=2、usage=2；第三次 local-only attempt 为 `not_observed/response_failed`。
- Correlation keys:
  - `WAR-20260810-044303-CACHE-REGRESSION-417B0312-CACHE-001`
- Raw content:
  ```text
  local_attempts=3 boundary_requests=2 completed_responses=2 usage_records=2
  input=28060 cached_input=7936 output=588
  ```
- Interpretation: 两次 provider 成本可完整结算，第三次本地尝试必须保留但不能把成本归零。
- Time: 2026-08-10 06:00

## Evidence E-005: 旧 Map 管理链删除后离线 benchmark 回归通过
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: benchmark harness、performance observation、release decision self-tests
- Prediction or plan link:
  - P-001 不再消费旧 Map result/ref/compaction 模型的验收条件。
- Matched signal:
  - 旧模块及所有当前脚本 caller 静态扫描为零；三组 PowerShell 自测通过。
- Correlation keys:
  - none
- Raw content:
  ```text
  TaskSpace benchmark harness self-test: PASS
  performance observation self-test passed
  Release decision self-test: PASS
  ```
- Interpretation: 删除的是无效 benchmark 旁路，不影响 canonical Map 和当前 runner 的其他职责。
- Time: 2026-08-10 06:12

## Evidence E-006: 原始首轮 artifact 可按 provider boundary 精确复算
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: 修复后的 usage/cache analyzer 对首轮 immutable raw trace 的离线复算
- Prediction or plan link:
  - P-001 的成本与缓存复算条件。
- Matched signal:
  - provider cache 只含 2 个 boundary 请求；3 次 local attempt 仍保留；usage 无缺失。
- Correlation keys:
  - `WAR-20260810-044303-CACHE-REGRESSION-417B0312-CACHE-001`
- Raw content:
  ```text
  provider_requests=2 provider_attempts=3 input=28060 cached=7936 uncached=20124 output=588
  request_2_plus_hit_rate=0.560056 cache_usage_missing_count=0 business_success=false
  ```
- Interpretation: 成本证据已恢复且没有改变业务失败结论。
- Time: 2026-08-10 06:12

## Evidence E-007: 第二轮首次响应再次生成非法 outer JSON
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `WAR-20260810-051702-CACHE-REGRESSION-EEF1DDF4` 的 `rollout.jsonl`
- Prediction or plan link:
  - H-001 剩余授权复验。
- Matched signal:
  - 首次 arguments 长度 751，在 column 589 失败；`hosted_bindings` 被写入尚未闭合的 `calls` 数组。第二次 arguments 长度 736，可严格解析。
- Correlation keys:
  - `call_00_cS3WNAmnJy1pTSAWAiEm8551`
  - `call_00_NJT7tJg2D59W7x39b9w39693`
- Raw content:
  ```text
  first: ... "finish": {...}}}, "hosted_bindings": []}, {"tool": "exec_command", ...
  error: expected `,` or `]` at line 1 column 589
  second: root keys = [calls, hosted_bindings], calls = 2
  ```
- Interpretation: 两轮不同 JSON 错误排除固定截断，但都与机械空 Hosted 字段相邻；首次稳定性未通过。
- Time: 2026-08-10 05:22

## Evidence E-008: 第二轮合法响应完成生产执行与 Map 持久化
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: fix-validation
- Source: 第二轮 `taskspace_exec` events、outer result 和 canonical Map Store
- Prediction or plan link:
  - CP-13 后生产路径复验。
- Matched signal:
  - preflight accepted、candidate revision 2、client result succeeded、final revision 3；Map 为 `root -> inspect -> fix -> verify -> finish`。
- Correlation keys:
  - Map `map-019fe863-2bc5-7110-9ce4-4f6e260d03f1`
- Raw content:
  ```text
  taskspace.exec.completed client_result_count=1 success=true map_revision=3
  ```
- Interpretation: outer Exec、Map 和 client dispatch 生产链可用；旧 Map management consumer 不再中断 runner。
- Time: 2026-08-10 05:22

## Evidence E-009: 第二轮在线成本结算正确区分边界与本地尝试
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: 第二轮 result、request facts 和全局 ledger
- Prediction or plan link:
  - H-003 新生产 run 验收。
- Matched signal:
  - provider requests=2、local attempts=3、usage records=2；账本直接结算 input 28131、cached 27520、output 633。
- Correlation keys:
  - `WAR-20260810-051702-CACHE-REGRESSION-EEF1DDF4`
- Raw content:
  ```text
  request_2_plus_hit_rate=0.962 cache_usage_missing_count=0
  ```
- Interpretation: provider-boundary 范围修复已获在线生产证据，第三次 local-only 429 不再污染成本。
- Time: 2026-08-10 05:22

## Evidence E-010: 零 Hosted 合同已完成最小离线修复
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `taskspace_exec` schema、decoder、canonical examples 与原有 Hosted preflight
- Prediction or plan link:
  - H-001 的最小协议修复。
- Matched signal:
  - 无 Hosted 示例省略 `hosted_bindings` 后可直接解码和通过 preflight；存在 Hosted output 但省略绑定仍返回 `HostedCountMismatch`。
- Correlation keys:
  - none
- Raw content:
  ```text
  taskspace_exec unit tests: 69 passed
  cache gate: PASS candidate_transition=true
  candidate surface: e49cc5ff2184b34e08872ebaccf9c7d9bb92b947072befec0e2b467005a91a56
  ```
- Interpretation: 修复只移除无意义的空字段填写成本，没有放宽 Hosted 事实的完整绑定硬约束。
- Time: 2026-08-10

## Hypothesis H-004: 第二请求重新暴露了顶层 client Tool
- Status: refuted
- Parent: P-001
- Claim: Runtime 在 Map 初始化后改变 Provider Tool 集合，把 `exec_command` 重新加入第二请求顶层声明，Agent 因而合法选择它。
- Layer: alternative-cause
- Factor relation: competing
- Depends on:
  - none
- Rationale:
  - 如果顶层能力集合发生变化，第二响应不能归因于 Agent 违反静态合同。
- Falsifiable predictions:
  - If true: 第二请求 `tools_hash`、`tools_count` 或 exact final-wire Tool name 集合应包含 `exec_command`，并与第一请求不同。
  - If false: 两次请求的 Tool identity 完全相同，且都只有 `taskspace_exec` 与 Hosted Tool。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐两次 provider-wire identity、candidate final-wire 和 Router TaskSpace 可见性代码。
  - Signal: `tools_hash`、`tools_count`、`tool_choice`、Tool names。
  - Capture method: 只读生产 trace、候选 final-wire report 和源码调用链。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
    - `provider.chat_wire_prefix_broken`
  - Correlation keys:
    - `provider-wire:019fe896-1e71-7321-9930-bc5d0921294b:0:logical-1:attempt-1`
    - `provider-wire:019fe896-1e71-7321-9930-bc5d0921294b:0:logical-2:attempt-1`
  - Differentiates from:
    - 模型把内层能力名提升为未声明顶层调用。
  - Supports if:
    - 第二请求 Tool 集合新增 `exec_command`。
  - Refutes if:
    - 两次 Tool 集合和 identity 完全一致且不含 `exec_command`。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-011
- Conclusion: refuted；两次 Provider wire 的 Tool identity 完全相同，均只有 `taskspace_exec + web_search`，第二请求没有新增 `exec_command`。
- Repair design readiness: no
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-005: Agent 可见合同同时鼓励顶层直调和 outer Exec
- Status: confirmed
- Parent: P-001
- Claim: TaskSpace 复用了 Standard 基础指令，其中明确描述直接终端、补丁和 `update_plan` Function Call；与此同时 `taskspace_exec` schema、示例和结果又以 `tool: exec_command` 等原生名称暴露内层能力。两个高显著性合同对同一能力给出不同层级，模型在成功首轮后按熟悉的顶层 Tool prior 提升了内层名称。
- Layer: contributing-factor
- Factor relation: contributing
- Depends on:
  - H-004 refuted
- Rationale:
  - 当前症状不是参数错误，而是 outer/inner 层级错误；重复出现的名称恰好来自 Standard 指令与 outer 内层 catalog。
- Falsifiable predictions:
  - If true: TaskSpace 实际 base profile 文本与 Standard 完全相同并描述直接工具调用；outer declaration 和首轮历史多次出现精确字符串 `exec_command`；原始第二响应直接命名 `exec_command`，不存在 Runtime 改名步骤。
  - If false: TaskSpace base 已明确统一 outer Exec 工作方式，或 `exec_command` 在 Agent 可见 schema/history 中不存在，或原始响应实际仍命名 `taskspace_exec`。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐实际 session base、base resolver、candidate declaration、第一轮 call/result 和第二轮原始 response item。
  - Signal: 精确文本、Tool name、call ID、出现顺序和层级。
  - Capture method: 只读源码、final-wire、rollout；用离线 fixture 固定可见合同矛盾。
  - Event name or marker:
    - `session_meta`
    - `function_call`
    - `function_call_output`
  - Correlation keys:
    - `call_00_JShrk8S4aS5DgNLGsELi8006`
    - `call_00_4ssfdJl0vRuVI5nkcRfr9385`
  - Differentiates from:
    - Provider Tool 集合变化、Runtime Function 名转换、反馈丢失。
  - Supports if:
    - Standard base 与 outer nested catalog 的冲突同时存在，且泄漏名称直接来自模型原始输出。
  - Refutes if:
    - 任一必要链条不存在，或存在其他层把合法 outer call 改名。
  - Instrumentation status: diagnostic-offline
  - Instrumentation lifecycle:
    - 若确认，转为 base/profile 与 final-wire 的永久一致性门禁。
- Evidence gate: satisfied
- Related evidence:
  - E-012
  - E-013
  - E-014
  - E-015
- Conclusion: confirmed as contributing factor, not sufficient root cause。TaskSpace 实际输入同时包含 Standard 顶层 Function Call 工作方式和 outer Exec 内层工具目录，确实放大层级歧义；但历史 Function Exec 在同一模型、同一 base 和 Function 外层下连续 15 次保持正确 outer `exec`，证明共享 base 不能单独解释本故障。
- Repair design readiness: no
- Next step: 修复设计必须优先消除内外层 wire 同形性；base 只做与实际能力一致的宏观校正，不得承载详细 Tool wire。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-006: Provider 或 Runtime 把合法 outer call 改名为内层 Tool
- Status: refuted
- Parent: P-001
- Claim: 模型原始响应仍是 `taskspace_exec`，但 Responses API 适配、反序列化或 Runtime reconciliation 根据嵌套 `tool` 字段把它错误转换为顶层 `exec_command`。
- Layer: alternative-cause
- Factor relation: competing
- Depends on:
  - none
- Rationale:
  - 如果适配层改名，则修复 Agent 协议不会解决问题。
- Falsifiable predictions:
  - If true: Provider 原始 response item 与 rollout/runtime item 的 Function name 不同，或转换代码从 arguments 读取 `tool` 并覆盖 outer name。
  - If false: 原始 rollout 已直接记录顶层 `name=exec_command`，转换链只透传 Provider Function name，reconciliation 随后拒绝。
- Diagnostic evidence plan:
  - Prediction or clause under test: 追踪 Responses output item 到 `ResponseItem::FunctionCall` 和 response scope 的 name 数据流。
  - Signal: Function name 的唯一赋值点与生产原始 item。
  - Capture method: 源码静态数据流、现有 provider response/rollout evidence、离线 parser fixture。
  - Event name or marker:
    - `response_item`
    - `taskspace.exec.response_finalized`
  - Correlation keys:
    - Provider response `4ed8d4e9-92b7-42fb-8378-0eb5023ab37c`
  - Differentiates from:
    - Agent 直接生成未声明 Function name。
  - Supports if:
    - 发现 name 覆盖或原始/运行时 name 不一致。
  - Refutes if:
    - name 逐层原样透传且原始 item 已为 `exec_command`。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-014
- Conclusion: refuted；DeepSeek Chat Completions delta 的 `function.name` 被直接保存为 `ResponseItem::FunctionCall.name`，TaskSpace response scope 读取同一字段。生产 rollout 在进入 reconciliation 前已经是 `name=exec_command`，没有 arguments 驱动的重命名路径。
- Repair design readiness: no
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-011: 两次请求的顶层 Tool 集合完全相同
- Related hypotheses:
  - H-004
- Direction: refutes
- Type: diagnostic-log
- Source: `WAR-20260810-061241-CACHE-REGRESSION-A143B6F0` provider wire 与候选 final-wire report
- Prediction or plan link:
  - H-004 的 Tool identity 变化预测。
- Matched signal:
  - 两次请求均为 `tools_count=2`、相同 `tools_hash`、`tool_choice=auto`；候选 payload 的精确顶层名称只有 `taskspace_exec` 和 `web_search`。
- Correlation keys:
  - logical request 1、2
- Raw content:
  ```text
  ["taskspace_exec","web_search"]
  ```
- Interpretation: 第二响应的 `exec_command` 不是 Runtime 重新暴露后的合法选择。
- Time: 2026-08-10 07:20

## Evidence E-012: TaskSpace 实际复用 Standard base instructions
- Related hypotheses:
  - H-005
- Direction: supports
- Type: static-dataflow
- Source: `base_instructions_profile.rs`、生产 `session_meta`、`whalecode_standard.md`
- Prediction or plan link:
  - H-005 的 base profile 一致性预测。
- Matched signal:
  - `Standard` 与 `TaskSpace` 均解析为 `standard_instructions`；源码文件和生产 session 文本 SHA-256 同为 `5e1178bd781d3be2cb2c4d5ead76ba074b3349954b7832333d86b6c454cc7382`。
- Correlation keys:
  - session `019fe896-1e71-7321-9930-bc5d0921294b`
- Raw content:
  ```text
  Emit function calls to run terminal commands and apply patches.
  You have access to an `update_plan` tool...
  ```
- Interpretation: TaskSpace 顶层实际不暴露普通 client Tool 和 `update_plan`，但最高层工作说明仍按 Standard 的顶层调用模型描述能力。
- Time: 2026-08-10 07:22

## Evidence E-013: outer 合同和反馈反复暴露内层原生 Tool 名
- Related hypotheses:
  - H-005
- Direction: supports
- Type: contract-inspection
- Source: 候选 `taskspace_exec` final-wire declaration 与首响应 outer result
- Prediction or plan link:
  - H-005 的内层名称显著性预测。
- Matched signal:
  - declaration 明确给出 `{"tool":"exec_command",...}` 示例，17 个 `calls[]` union 分支和结果合同继续列出原生 Tool identity；首响应结果再次返回 `tool=exec_command`。
- Correlation keys:
  - outer call `call_00_JShrk8S4aS5DgNLGsELi8006`
- Raw content:
  ```text
  taskspace_exec_result.client_results[0].tool=exec_command
  ```
- Interpretation: 内层能力名必须可见，但在缺少一致宏观调用层级说明时，会与 Standard 的顶层 Function Call prior 组合成歧义。
- Time: 2026-08-10 07:25

## Evidence E-014: Function name 从 Provider delta 到 Runtime 原样透传
- Related hypotheses:
  - H-005
  - H-006
- Direction: supports H-005 / refutes H-006
- Type: static-dataflow
- Source: Chat Completions SSE adapter、TaskSpace response scope、生产 rollout
- Prediction or plan link:
  - H-006 的名称覆盖预测。
- Matched signal:
  - adapter 在 delta 到达时执行 `state.name = Some(name)`，结束时直接构造 `ResponseItem::FunctionCall { name, ... }`；response scope 直接按该 `name` 判断 forbidden top-level client Tool。生产 response item 已记录 `name=exec_command`。
- Correlation keys:
  - `call_00_4ssfdJl0vRuVI5nkcRfr9385`
  - Provider response `4ed8d4e9-92b7-42fb-8378-0eb5023ab37c`
- Raw content:
  ```text
  {"type":"function_call","name":"exec_command",...}
  ```
- Interpretation: 非法层级由模型响应产生；Provider 适配和 Runtime 没有根据嵌套参数改名，也没有丢失合法 outer call。
- Time: 2026-08-10 07:28

## Evidence E-015: inner-name 顶层提升跨协议版本重复出现
- Related hypotheses:
  - H-005
- Direction: supports
- Type: historical-reproduction
- Source: 现有四次 VA-02 生产 rollout 的离线枚举
- Prediction or plan link:
  - H-005 的历史一致性预测。
- Matched signal:
  - 首次旧 description 运行直接生成 `exec_command`；协议增强后的零 Hosted 运行先成功生成 `taskspace_exec`，下一请求又生成 `exec_command`。另外两次运行生成 outer Exec，但因独立 JSON 结构错误终止。
- Correlation keys:
  - `WAR-20260809-195732-CACHE-REGRESSION-4E4DA2D5`
  - `WAR-20260810-061241-CACHE-REGRESSION-A143B6F0`
- Raw content:
  ```text
  run 1: exec_command
  run 4: taskspace_exec,exec_command
  ```
- Interpretation: 这不是单次传输损坏。Tool description 增强改善了首次 outer 选择，但当前 Agent 输入仍不能稳定维持 outer/inner 层级。
- Time: 2026-08-10 07:31

## Hypothesis H-007: TaskSpace 内层调用 wire 被提升并扁平化为 Provider 顶层调用
- Status: confirmed
- Parent: P-001
- Claim: `taskspace_exec.calls[]` 以 `tool` 原生名称 discriminator、`arguments` 和外层 `node_id` 表达内层调用，与 Provider 顶层 Function Call 形状过于接近；DeepSeek 因而会把内层分支提升为顶层 Function Call，并把 wrapper metadata 扁平写入原生 Tool arguments。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-004 refuted
  - H-006 refuted
- Rationale:
  - 两次非法调用不仅名称等于内层 branch，还携带只存在于 TaskSpace wrapper 的 `node_id`；这比通用“模型不遵循”更能定位错误信息来源。
- Falsifiable predictions:
  - If true: 非法顶层调用的 name 应等于 `calls[]` branch 的 `tool` enum，arguments 应混入 wrapper-only `node_id`；明显区分内外语法的 Function Exec 在相同 base/model 下不应发生同类提升。
  - If false: 非法调用只含原生 Tool 参数，或语法区分后的 Function Exec 同样频繁把 `tools.exec_command` 提升为顶层。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐两次非法调用、正式 inner branch schema，以及历史 Function Exec rollout。
  - Signal: 顶层 name、arguments keys、outer call name、内层语法和连续调用次数。
  - Capture method: 只读 rollout、候选 final-wire、历史获批实验 artifact。
  - Event name or marker:
    - `function_call`
  - Correlation keys:
    - `call_00_bHZUdurJshkGaum9XOPF0818`
    - `call_00_4ssfdJl0vRuVI5nkcRfr9385`
  - Differentiates from:
    - Standard base 单独致错、outer Function Tool 本身不兼容、结果反馈诱导、声明总长度过大。
  - Supports if:
    - 两次非法调用均携带 `node_id`，且历史 Function Exec 15 次均保持 outer `exec`。
  - Refutes if:
    - 任一来源指纹不存在，或对照实验出现同类 inner-name 顶层提升。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-016
  - E-017
  - E-018
- Conclusion: confirmed。错误输出逐字段保留了 TaskSpace inner branch 的身份和归属语义；相同模型/base/Function outer 的语法分离对照没有发生提升。当前证据能确认到“内层结构化调用表达未形成稳健层级边界”，但尚不能把微观原因进一步唯一归结为字段名 `tool`、17 分支 union 或 Map 操作复杂度中的某一个。
- Repair design readiness: implemented offline
- Next step: 缓存门禁通过后另行申请最小真实预算，验证目标模型是否稳定保持 outer `taskspace_exec`。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-008: 顶层 Tool declaration 总长度过大导致越界调用
- Status: refuted
- Parent: P-001
- Claim: 模型因为 TaskSpace Tool declaration 总 token 过大而忽略 outer 层级，生成未声明顶层 Tool。
- Layer: alternative-cause
- Factor relation: competing
- Depends on:
  - none
- Rationale:
  - 大 schema 可能降低局部约束显著性，但必须解释为何相近或更大声明仍可稳定使用 outer Exec。
- Falsifiable predictions:
  - If true: 成功 Function Exec 的 tools section 应明显小于失败 TaskSpace。
  - If false: 两者大小相近，甚至 Function Exec 不更小。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较现有 provider wire 首请求 tools section。
  - Signal: bytes 与 estimated tokens。
  - Capture method: 只读 provider-wire trace。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - 两个历史 run identity
  - Differentiates from:
    - schema 内部结构和语法同形性。
  - Supports if:
    - TaskSpace 显著更大。
  - Refutes if:
    - 大小相近。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-016
- Conclusion: refuted as primary cause。Function Exec tools section 为 28,012 bytes / 7,003 estimated tokens，TaskSpace 为 29,062 bytes / 7,266 estimated tokens，差异不足以解释 15 次正确 outer 调用与两次层级提升。
- Repair design readiness: no
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-009: 首轮 outer result 反馈诱导第二轮提升
- Status: refuted
- Parent: P-001
- Claim: `taskspace_exec_result.client_results[].tool=exec_command` 是模型在下一轮改为顶层 `exec_command` 的必要诱因。
- Layer: alternative-cause
- Factor relation: competing
- Depends on:
  - none
- Rationale:
  - 当前零 Hosted 运行的越界发生在合法 outer result 之后，但必须解释首次 VA-02 为何在没有任何 outer result 时已同样越界。
- Falsifiable predictions:
  - If true: 首请求无历史 outer result 时不应发生同类提升。
  - If false: 首次请求即可生成带 `node_id` 的顶层 `exec_command`。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查首次 VA-02 的第一响应及其前置历史。
  - Signal: response 序号、function name 与 arguments。
  - Capture method: 只读 rollout。
  - Event name or marker:
    - `function_call`
  - Correlation keys:
    - `WAR-20260809-195732-CACHE-REGRESSION-4E4DA2D5`
  - Differentiates from:
    - inner schema 自身已足以诱发提升。
  - Supports if:
    - 只在 result 后发生。
  - Refutes if:
    - 第一响应已发生。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-017
- Conclusion: refuted as necessary cause。结构化 outer result 可能继续增强内层名称显著性，但首次请求在没有任何 TaskSpace result 时已经生成同类顶层调用。
- Repair design readiness: no
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-016: 相同模型和 base 的 Function Exec 连续保持正确 outer 层级
- Related hypotheses:
  - H-005
  - H-007
  - H-008
- Direction: refutes H-005/H-008 as sufficient causes / supports H-007
- Type: historical-controlled-comparison
- Source: 两次已获批 Function Exec 真实实验及原始 rollout/provider wire
- Prediction or plan link:
  - H-007 的语法分离对照预测。
- Matched signal:
  - 两次实验的生产 session base SHA-256 均为当前 Standard hash；outer 同为普通 Function Tool。第一轮 7 个、第二轮 8 个顶层调用全部命名 `exec`，内层通过 `tools.exec_command`/`tools.apply_patch` JavaScript 表达，没有一次顶层 inner-name 提升。Function Exec tools section 28,012 bytes，与 TaskSpace 29,062 bytes 接近。
- Correlation keys:
  - `WAR-20260805-055746-R8-DEEPSEEK-FUNCTION-EXEC-CORRECTED-001`
  - `WAR-20260805-061947-R8-FUNCTION-EXEC-CONTRACT-FIX-001`
- Raw content:
  ```text
  function exec: 15/15 top-level name=exec
  base sha256: 5e1178bd781d3be2cb2c4d5ead76ba074b3349954b7832333d86b6c454cc7382
  ```
- Interpretation: 共享 Standard base、Function outer 和大 Tool description 都不足以产生当前故障；明显不同的内层 JS 语法能够让 DeepSeek 保持层级。
- Time: 2026-08-10 08:04

## Evidence E-017: 两次非法调用都携带 TaskSpace-only node_id
- Related hypotheses:
  - H-007
  - H-009
- Direction: supports H-007 / refutes H-009
- Type: output-fingerprint
- Source: 首次与零 Hosted VA-02 rollout、正式 TaskSpace Exec schema
- Prediction or plan link:
  - H-007 的 wrapper metadata 来源预测。
- Matched signal:
  - 两个原始顶层 `exec_command` arguments 分别为 `{cmd,node_id:"root"}` 和 `{cmd,node_id:"inspect"}`；原生 `exec_command` schema 不含 `node_id`，该字段只属于 `taskspace_exec.calls[]` client wrapper。
- Correlation keys:
  - `call_00_bHZUdurJshkGaum9XOPF0818`
  - `call_00_4ssfdJl0vRuVI5nkcRfr9385`
- Raw content:
  ```text
  top-level exec_command({cmd, node_id})
  inner branch requires {tool, node_id, arguments}
  ```
- Interpretation: 模型在提升内层 Tool 名的同时扁平带出了节点归属，错误直接来源于 TaskSpace inner wire，而不是单纯回忆 Standard 原生 Tool。
- Time: 2026-08-10 08:08

## Evidence E-018: Function Exec 的旧 prior 只污染参数，没有破坏 outer 层级
- Related hypotheses:
  - H-005
  - H-007
- Direction: supports
- Type: historical-contrast
- Source: Function Exec 首轮真实实验 action path 与修复复验
- Prediction or plan link:
  - 区分通用 base prior 与层级同形性的具体表现。
- Matched signal:
  - Function Exec 首次曾生成正确顶层 `exec` 但错误使用 `{cmd}`，修正 carrier description 后 8 次均使用 `{source}`；整个过程没有调用顶层 `exec_command`。TaskSpace 的错误则同时改变顶层 name 并混入 `node_id`。
- Correlation keys:
  - `WAR-20260805-055746-R8-DEEPSEEK-FUNCTION-EXEC-CORRECTED-001`
  - `WAR-20260805-061947-R8-FUNCTION-EXEC-CONTRACT-FIX-001`
- Raw content:
  ```text
  Function Exec: exec({cmd}) -> exec({source})
  TaskSpace: exec_command({cmd,node_id})
  ```
- Interpretation: Standard prior 能解释旧参数名泄漏，但不能解释 TaskSpace 的 outer name 提升；后者需要 inner wire 作为信息来源。
- Time: 2026-08-10 08:12

## Evidence E-019: 新 wire 已在离线合同中消除 inner/outer 同形性
- Related hypotheses:
  - H-007
- Direction: fix-validation
- Type: code-and-test
- Source: TaskSpace Exec catalog、decoder、protocol 和定向测试
- Prediction or plan link:
  - H-007 的最小修复必须删除 Agent-visible `tool + arguments` 内层形状，同时不改内部 dispatch 和 Standard。
- Matched signal:
  - `calls[]` schema 的每个分支现在只有一个 `map` 或 `client` envelope；Map 使用 `operation + input`，Client 使用 `name + node_id + input`。decoder 明确拒绝旧 Map/Client wire；70 项 TaskSpace Exec 测试覆盖 Map、Function、Freeform、Namespace、Hosted、持久化和零副作用失败路径。生产改动局限于 TaskSpace Exec catalog/decoder/protocol，Standard 路径没有条件分支或 schema 修改。
- Correlation keys:
  - `taskspace_exec_catalog_tests::decoder_rejects_the_provider_shaped_legacy_inner_wire`
  - `cargo test -p codex-core taskspace_exec --lib --quiet`
- Raw content:
  ```text
  calls[].map    = {operation, input}
  calls[].client = {name, node_id, input}
  70 passed; 0 failed
  ```
- Interpretation: 根因对应的协议结构已完成离线修复，且没有通过 Runtime 推断、兼容解析或修改普通 Tool 来规避问题。该证据不能替代目标 Provider 的 Agent 行为复验。
- Time: 2026-08-10 09:05

## Evidence E-020: 新 map-client wire 连续保持正确 outer 层级
- Related hypotheses:
  - H-007
- Direction: supports
- Type: fix-validation
- Source: `WAR-20260810-174818-CACHE-REGRESSION-0EF76553` 与 `WAR-20260810-180151-CACHE-REGRESSION-7E11A055` rollout
- Prediction or plan link:
  - H-007 修复后不再把 inner client name 提升到 Provider 顶层。
- Matched signal:
  - 两轮共 8 个已完成 Function Call 全部命名 `taskspace_exec`；Agent 直接生成 `map/client`，Runtime 未包装或改名。
- Correlation keys:
  - 两个 WAR record ID
- Raw content:
  ```text
  completed outer calls: taskspace_exec × 8
  top-level exec_command: 0
  ```
- Interpretation: 旧 inner/outer 同形根因对应的协议修复在线成立；后续 mixed batch JSON 错误是不同表现，不能回写成旧顶层提升仍存在。
- Time: 2026-08-10 18:10

## Hypothesis H-010: 当前 patch 阻塞来自 mixed batch 的 map envelope 少一个闭合花括号
- Status: confirmed
- Parent: P-001
- Claim: 两次 `update_map + apply_patch` arguments 在 map call 外层少一个 `}`，导致整个 outer Function arguments 不是合法 JSON；Freeform patch 内容和转义不是失败原因。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 错误稳定发生在 map/client 边界，且两个失败 arguments 字节完全一致。
- Falsifiable predictions:
  - If true: `node_patches`、中文 content 和 patch 的 `\n` 均合法；在 `}, {"client"` 前只关闭 input 与 map，没有关闭 call wrapper。
  - If false: 补齐 map wrapper 后仍不能解析，或错误点位于 patch 字符串转义。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对原始 `.payload.arguments` 做严格 `fromjson` 并分段检查 map/client 边界。
  - Signal: 原始长度、解析错误、边界字符和重复参数身份。
  - Capture method: 只读 rollout 结构化查询，不修改输入。
  - Event name or marker:
    - `function_call`
  - Correlation keys:
    - `call_00_zzRV83dFip0rdFZLH6NV6981`
    - `call_00_qzT6ULnKUawDrIJ1B1H19727`
  - Differentiates from:
    - Freeform 多行输入未转义、Runtime 参数重写、Provider 截断。
  - Supports if:
    - 两次参数相同且只缺 map call wrapper 的一个 `}`。
  - Refutes if:
    - 原始参数可解析或 patch 字符串包含未转义换行。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-021
- Conclusion: confirmed。严格解码和零副作用拒绝正确；不得由 Runtime 补括号或猜测 Agent 意图。
- Repair design readiness: blocked pending H-011 and product decision
- Next step: 比较“补充同源 mixed transition 示例”与“进一步扁平化 call discriminator”的最小性、稳定性和旧根因回归风险。
- Blocker:
  - 需要用户确认 Agent-visible 协议修复方向。
- Close reason:
  - not closed

## Evidence E-021: 两次失败参数相同且只缺 map call wrapper
- Related hypotheses:
  - H-010
- Direction: supports
- Type: diagnostic-log
- Source: 完整 VA-02 `rollout.jsonl`
- Prediction or plan link:
  - H-010 的边界字符预测。
- Matched signal:
  - 两次 arguments 长度均为 533，严格解析均失败；边界为 `..."content":"修复..."}]}}, {"client":...`，完整合法结构应在逗号前再有一个 `}`。patch 内部换行以 `\n` 表达。
- Correlation keys:
  - 两个失败 call ID
- Raw content:
  ```text
  actual:   ... node_patches:[...]}}, {"client":...}}
  required: ... node_patches:[...]}}}, {"client":...}}
  ```
- Interpretation: 失败是 Agent 输出的 outer JSON 结构错误；Freeform Tool 本身未进入 decoder 或执行路径。
- Time: 2026-08-10 18:10

## Hypothesis H-011: 缺少同源 mixed transition 示例使 Agent 只能手工拼装高嵌套边界
- Status: verified
- Parent: P-001
- Claim: 当前 Tool description 有初始化+client 和 update+finish 示例，但没有“完成前置节点 + 后续 client work”的 canonical 示例；增加由同一类型生成并由 decoder/preflight 反向验证的最小示例，可能在不改 wire 和 Runtime 语义的情况下消除当前重复括号错误。
- Layer: interaction
- Factor relation: unknown
- Depends on:
  - H-010 confirmed
- Rationale:
  - Agent 已理解需要先完成 inspect 再 patch，但两次在未覆盖的 mixed 组合上复制同一结构错误；首轮初始化 mixed 示例对应的调用则一次合法。
- Falsifiable predictions:
  - If true: 静态协议可加入不绑定具体业务路径的 canonical mixed 示例，且同一示例通过 decoder/preflight；后续真实复验不再在该边界缺括号。
  - If false: 示例无法保持通用、显著增加歧义/成本，或复验仍产生同一错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: 先完成静态示例设计、schema 成本和旧根因回归审查；只有用户批准后才实施并申请真实复验。
  - Signal: 示例来源唯一性、反向解析、description 增量、旧 inner-name 禁止扫描和缓存门禁。
  - Capture method: 离线设计对比和定向测试；真实行为另行预算。
  - Event name or marker:
    - `canonical_transition_example`
  - Correlation keys:
    - none
  - Differentiates from:
    - 直接改 wire、Runtime JSON 修复、单纯增加请求预算。
  - Supports if:
    - 最小示例不复制协议、不引入具体路径/命令，且覆盖 exact mixed shape。
  - Refutes if:
    - 必须引入第二套说明或仍无法降低结构歧义。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-022
- Conclusion: confirmed and refined by H-013。canonical handoff 已由同一 typed contract 生成并通过生产 decoder/preflight；真实 Agent 效果仍待预算复验。
- Repair design readiness: implemented offline
- Next step: 在新预算下只复验 Structured 当前生产路径。
- Blocker:
  - 真实 Provider 预算。
- Close reason:
  - verified offline

## Evidence E-022: 完整运行成本、缓存和 DAG 行为已可信结算
- Related hypotheses:
  - H-010
  - H-011
- Direction: neutral
- Type: observation
- Source: `WAR-20260810-180151-CACHE-REGRESSION-7E11A055` result、ledger、Map Store 和 provider cache trace
- Prediction or plan link:
  - 区分协议失败与预算、缓存、反馈或 Map 持久化异常。
- Matched signal:
  - 6 requests；input 97,584、cached 91,392、output 2,056、request 2+ hit 92.68%、USD 0.0016984576。第一次 patch 因 `fix=waiting` 被 DAG 拒绝；两次非法 mixed batch 均零提交、零 patch，Map revision 7 保持完整。
- Correlation keys:
  - `WAR-20260810-180151-CACHE-REGRESSION-7E11A055`
- Raw content:
  ```text
  provider_requests=6 business_success=false cache_hit_2_plus=0.926835
  first patch: ClientNodeNotExecutable(fix, Waiting)
  next two batches: InvalidJson; git diff empty
  ```
- Interpretation: 本轮不是缓存坍缩、usage 缺失、Runtime 自动状态推进或预算过早终止；继续增加请求不能替代协议修复。
- Time: 2026-08-10 18:10

## Hypothesis H-012: Dedicated TaskSpace base 已消除顶层 client Tool 逃逸
- Status: verified
- Parent: P-001
- Claim: 移除与 TaskSpace Exec 冲突的 Standard Tool 使用说明后，Agent 不再绕过 `taskspace_exec` 直接调用顶层 client Tool。
- Layer: interaction
- Factor relation: single
- Depends on:
  - H-009
- Rationale:
  - Structured 与 Source 共 6 次当前版本运行均未出现顶层 client Tool call。
- Falsifiable predictions:
  - If true: 当前版本 TaskSpace 运行只暴露并调用 `taskspace_exec` 与 provider-hosted Tool。
  - If false: rollout 中再次出现顶层 `exec_command`、`apply_patch` 或其他 client Tool。
- Diagnostic evidence plan:
  - Prediction or clause under test: 扫描 3x2 TaskSpace rollout 的所有顶层 Tool call。
  - Signal: 顶层 Tool name。
  - Capture method: benchmark rollout 结构化查询。
  - Event name or marker:
    - `function_call`
  - Correlation keys:
    - `target/r8-e01/runs/a0/single-file-fast-fix/r1/pair-001`
  - Differentiates from:
    - TaskSpace Exec 内部 client call。
  - Supports if:
    - 6 次运行均为零逃逸。
  - Refutes if:
    - 任一运行出现顶层 client Tool。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - benchmark evidence
- Evidence gate: satisfied
- Related evidence:
  - E-023
- Conclusion: confirmed。逃逸修复成立，不解释后续状态交接失败。
- Repair design readiness: not applicable
- Next step: 保留静态 Tool 暴露门禁。
- Blocker:
  - none
- Close reason:
  - verified

## Evidence E-023: 当前 TaskSpace 六次运行均无顶层 client Tool 逃逸
- Related hypotheses:
  - H-012
- Direction: supports
- Type: benchmark
- Source: 3-arm x repeat-3 matrix 的 Structured/Source rollout
- Prediction or plan link:
  - H-012 顶层 Tool name 扫描。
- Matched signal:
  - 6 次 TaskSpace rollout 中，顶层 client Tool call 计数为 0。
- Correlation keys:
  - `target/r8-e01/runs/a0/single-file-fast-fix/r1/pair-001`
- Raw content:
  ```text
  taskspace_runs=6 top_level_client_tool_escape=0
  ```
- Interpretation: Dedicated TaskSpace base 的目标修复已生效；本轮失败来自 Exec 参数和状态交接，不是顶层逃逸复发。
- Time: 2026-08-11

## Hypothesis H-013: 父节点完成后的子节点交接合同表达缺失
- Status: verified
- Parent: P-001
- Claim: 当前协议只要求 complete 后携带后续工作，却没有明确说明子节点 readiness 会在父节点更新后机械派生；Agent 因而额外把 `waiting` 子节点直接 patch 为 `in_flight`，触发正确的硬约束拒绝。
- Layer: interaction
- Factor relation: single
- Depends on:
  - H-012 confirmed
- Rationale:
  - 两次 Structured 运行都先正确识别父子依赖，随后生成同一种多余状态跳转；preflight 已按 call 顺序先应用父节点完成，再检查后续 client work。
- Falsifiable predictions:
  - If true: 仅 patch 父节点为 completed，并把绑定子节点的 client work 放在同批后续位置，可通过真实 decoder/preflight；无需 Runtime 推断或修改 Agent 声明。
  - If false: 子节点在后续 client work 检查时仍为 waiting，或必须由 Agent 显式 patch 状态。
- Diagnostic evidence plan:
  - Prediction or clause under test: 用 canonical handoff 经生产 decoder 和 preflight 验证 candidate Map。
  - Signal: 父节点 completed、子节点 mechanically ready、client call admitted。
  - Capture method: Rust 定向测试。
  - Event name or marker:
    - `rendered_parent_handoff_example_derives_child_readiness_before_work`
  - Correlation keys:
    - none
  - Differentiates from:
    - 放宽 `waiting -> in_flight`、Runtime 自动绑定、Runtime 修复 Agent batch。
  - Supports if:
    - canonical batch 在不显式 patch 子节点状态时通过。
  - Refutes if:
    - preflight 拒绝或 candidate 子节点不为 ready。
  - Instrumentation status: implemented pending test
  - Instrumentation lifecycle:
    - permanent regression test
- Evidence gate: satisfied
- Related evidence:
  - E-024
- Conclusion: confirmed。canonical handoff 不显式 patch 子节点状态即可通过生产 decoder/preflight，candidate 中父节点 completed、子节点 ready。
- Repair design readiness: implemented offline
- Next step: 在新预算下验证目标模型遵循。
- Blocker:
  - none
- Close reason:
  - verified offline

## Evidence E-024: Structured 重复生成 waiting 子节点的非法显式跃迁
- Related hypotheses:
  - H-013
- Direction: supports
- Type: diagnostic-log
- Source: Structured repeat-3 rollout
- Prediction or plan link:
  - H-013 的重复行为与生产 preflight 顺序核对。
- Matched signal:
  - pair-001 与 pair-002 均先遇到 `ClientNodeNotExecutable Waiting`；后续 batch 同时完成 inspect、显式将 fix 设为 in_flight 并调用 patch，因 `TransitionInvalid ["fix"]` 被拒绝。生产 preflight 会先应用 update_map，并在该操作内部派生 readiness，然后才检查后续 client call。
- Correlation keys:
  - `pair-001/right`
  - `pair-002/right`
- Raw content:
  ```text
  update_map: inspect -> completed, fix -> in_flight
  preflight: TransitionInvalid ["fix"]
  ```
- Interpretation: 硬约束行为正确；缺口在 Agent-visible Tool 合同，没有明确告诉 Agent 不要重复声明机械派生状态。
- Time: 2026-08-11

## Hypothesis H-014: 当前观测器仍消费旧 Exec wire，Responses base 扫描字段错误
- Status: verified
- Parent: P-001
- Claim: benchmark 观测器仍读取扁平 `call.tool/call.node_id`，而生产 wire 已是互斥 `call.map/call.client`；provider wire trace 又只从 Responses `input` 消息寻找 base，漏掉顶层 `instructions`。
- Layer: observability
- Factor relation: independent
- Depends on:
  - none
- Rationale:
  - 代码静态核对可直接坐实字段错位；两者均不改变 Agent 或 Runtime 行为，只会扭曲诊断数据。
- Falsifiable predictions:
  - If true: 当前 fixture 改为真实 Structured wire 后，旧解析器会把 map/client 误计为普通 client；Responses identity 会报告 unrecognized。
  - If false: 解析器已读取嵌套 discriminator，或 Responses base 实际位于 input。
- Diagnostic evidence plan:
  - Prediction or clause under test: 更新 fixture 后执行 PowerShell 测试；增加 Responses 顶层 instructions 单测。
  - Signal: map/client/node/failure 计数与 base profile。
  - Capture method: 离线单元测试。
  - Event name or marker:
    - `test-taskspace-exec-observation.ps1`
    - `responses_base_identity_uses_top_level_instructions`
  - Correlation keys:
    - none
  - Differentiates from:
    - Provider usage 缺失、真实缓存未命中、TaskSpace 执行失败。
  - Supports if:
    - 修复后的测试准确消费当前 wire 并识别 top-level instructions。
  - Refutes if:
    - 生产 wire 与静态模型不一致。
  - Instrumentation status: implemented pending test
  - Instrumentation lifecycle:
    - permanent regression tests
- Evidence gate: satisfied
- Related evidence:
  - E-025
- Conclusion: 原两个观测缺陷已修复，当前 Structured fixture、canonical rejection 和 Responses 顶层 instructions 测试通过；E-028 又坐实 patch 专项消费者仍未读取 Exec 内部 `client.name=apply_patch`，因此整体观测问题继续开放。
- Repair design readiness: implemented offline
- Next step: 在 I07 内修复 nested patch lifecycle 消费，不改 Exec、Router 或 Agent 协议。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-025: 生产 wire 与观测代码字段逐项不一致
- Related hypotheses:
  - H-014
- Direction: supports
- Type: code-inspection
- Source: `taskspace_exec/catalog.rs`、`taskspace-exec-observation.ps1`、`provider_wire_trace.rs`、`codex-api/common.rs`
- Prediction or plan link:
  - H-014 的静态字段核对。
- Matched signal:
  - 生产 schema 使用 `map.operation/map.input` 与 `client.name/client.node_id/client.input`；观测器读取 `tool/node_id`。Responses request 将 base 放在顶层 `instructions`，identity 函数仅扫描 `input` 中 developer/system message。
- Correlation keys:
  - none
- Raw content:
  ```text
  production: calls[].map | calls[].client
  observer:   calls[].tool
  responses:  instructions + input[]
  scanner:    input[].role in {developer, system}
  ```
- Interpretation: 这是确定的观测实现漂移，不应据此推导 Agent 语义或缓存行为。
- Time: 2026-08-11

## Hypothesis H-015: 解析反馈把语法错误和顶层合同错误扭曲成同一种错误
- Status: verified
- Parent: P-001
- Claim: `TaskSpaceExecPlan::decode` 直接反序列化 `RawPlan`，使非法 JSON 与合法 JSON 的错误顶层字段都进入 `InvalidJson`；反馈又只给 parser 文本，没有明确 Function 参数已位于顶层，诱发 Agent 把完整计划重复包进 `arguments`。
- Layer: feedback
- Factor relation: single
- Depends on:
  - H-014 verified
- Rationale:
  - 最新 run 的 Request 1 是语法错误；Request 2～5 是可被 JSON parser 接受的同一个 `{"arguments":"..."}`，却收到同类 `PlanDecode(InvalidJson)` 文本并逐字重复。
- Falsifiable predictions:
  - If true: 两类输入可由严格两阶段 parser 确定区分，且 handler 能在零副作用下分别返回 syntax/contract 与 direct-calls 恢复合同。
  - If false: wrapper 本身也不是合法 JSON，或错误来自 Provider/Runtime 的参数改写。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对原始 arguments 独立 JSON parse、hash，并用生产 handler fixture 验证两类反馈与零副作用。
  - Signal: parse verdict、top-level keys、arguments SHA、handler message、Map/client side effects。
  - Capture method: rollout 只读分析 + Rust 定向测试。
  - Event name or marker:
    - `malformed_json_feedback_preserves_the_error_and_direct_shape`
    - `wrapped_arguments_feedback_distinguishes_contract_from_json_syntax`
  - Correlation keys:
    - `WAR-20260811-014315-CACHE-REGRESSION-99053528`
  - Differentiates from:
    - JSON 自动修复、schema 放宽、Agent 无视准确反馈、Map 状态错误。
  - Supports if:
    - Request 1 仅 syntax parse 失败；Request 2～5 JSON 合法且顶层只有 `arguments`；修复后两类反馈不同并均零执行。
  - Refutes if:
    - 原始 arguments 被 Runtime 改写或两类无法机械区分。
  - Instrumentation status: implemented
  - Instrumentation lifecycle:
    - permanent regression tests
- Evidence gate: satisfied
- Related evidence:
  - E-026
- Conclusion: confirmed。parser 现先解析通用 JSON，再校验 `RawPlan`；handler 忠实区分 syntax 与 top-level contract，明确 direct `calls`、禁止 `arguments` wrapper 和零执行。未增加容错执行。
- Repair design readiness: implemented offline
- Next step: 在线目标已满足；后续 syntax 生成稳定性归 I03，不再以 I05 名义重复验证。
- Blocker:
  - none
- Close reason:
  - verified online

## Evidence E-026: 最新 Structured run 的五次协议拒绝由两类错误组成
- Related hypotheses:
  - H-011
  - H-013
  - H-014
  - H-015
- Direction: supports
- Type: benchmark
- Source: `WAR-20260811-014315-CACHE-REGRESSION-99053528` result、rollout、provider boundary 与 sanitized diagnosis
- Prediction or plan link:
  - 验证 canonical handoff 后的 Structured 生产表现，并区分生成、反馈、预算和观测问题。
- Matched signal:
  - Standard 6 请求成功；TaskSpace 8 请求失败。TaskSpace Request 1 syntax invalid；Request 2～5 的 arguments SHA 相同且均为合法 JSON `arguments` wrapper；Request 6～8 正确执行初始化、读取和测试。无 patch，业务根因已由 Agent 准确识别。
- Correlation keys:
  - `WAR-20260811-014315-CACHE-REGRESSION-99053528`
- Raw content:
  ```text
  standard: requests=6 input=75195 request2+cache=97.37% success=true
  taskspace: requests=8 input=124614 request2+cache=88.75% success=false
  taskspace_exec: rejected=5 executed_map=1 executed_client=3 patch=0
  ```
- Interpretation: 本轮没有抵达 handoff，不能验证或否定 H-013。首个非 strict Function arguments 语法稳定性仍属 I03；四次后续放大由 H-015 的反馈分类缺陷解释。请求上限是终止点，不是首因。
- Time: 2026-08-11

## Evidence E-027: 新反馈使 Agent 在下一请求一次恢复并完成工作
- Related hypotheses:
  - H-013
  - H-015
- Direction: supports
- Type: benchmark
- Source: `WAR-20260811-042531-CACHE-REGRESSION-4BB46AE7` result、rollout、Map export 和 oracle
- Prediction or plan link:
  - H-015 的在线恢复预测；H-013 的 canonical handoff 预测。
- Matched signal:
  - Request 1 syntax reject 后，Request 2 立即使用 direct `calls` 并成功初始化；没有 `arguments` wrapper 重复。
  - Request 6 只完成父节点 `inspect`，随后同批 `apply_patch@fix` 成功，无显式子节点状态跳转。
  - Request 7 测试 `3 passed`；Request 8 完成 `finish_map`；最终 Map revision 15 全部 completed，hidden oracle passed。
- Correlation keys:
  - `WAR-20260811-042531-CACHE-REGRESSION-4BB46AE7`
- Raw content:
  ```text
  provider requests=8, input=132555, cached=125312, output=2835
  request2+ cache=94.02%
  request1=syntax reject; request2=corrected initialize+inspect
  request6=complete inspect + apply_patch@fix
  request7=3 passed; request8=finish_map
  ```
- Interpretation: I05 的反馈扭曲已修复并在线验证；I03 的首次 JSON 生成错误仍独立存在。最终回复缺失由第 9 次本地请求超过授权边界造成，不能把该 run 晋升为端到端成功。
- Time: 2026-08-11

## Evidence E-028: 当前 observer 仍漏计 Exec 内部 patch 生命周期
- Related hypotheses:
  - H-014
- Direction: supports
- Type: artifact-replay
- Source: 同一 run 的 performance observation、rollout 和 Exec trace
- Prediction or plan link:
  - I07 当前生产 trace 完整性检查。
- Matched signal:
  - TaskSpace Exec 表正确计量 8 outer Exec、5 Map operations、6 client actions 和 3 failures。
  - rollout 明确包含两次 `client.name=apply_patch`：一次 preflight reject、一次 succeeded；性能报告 Patch lifecycle 却显示 declarations=0。
- Correlation keys:
  - `WAR-20260811-042531-CACHE-REGRESSION-4BB46AE7-CACHE-001`
- Raw content:
  ```text
  exec nested apply_patch declarations=2
  executed nested apply_patch=1
  performance patch declarations=0
  ```
- Interpretation: 请求、Exec 和 client 主计数已可信，但 patch 专项消费者仍只识别旧/顶层载体。该缺口归入既有 I07，不新增全局问题。
- Time: 2026-08-11

## Hypothesis H-016: 只在 Exec decoder 中修补参数会保留错误的正式上下文
- Status: confirmed
- Parent: P-001
- Claim: 当前 `OutputItemDone` 链先把同一个原始 FunctionCall 交给 response scope，再由 `handle_output_item_done` 写入会话与 rollout；若只在 `TaskSpaceExecPlan::decode` 或 handler 内修补缺失闭合符号，执行可使用修正版，但后续 Agent context 和恢复仍保留错误版。
- Layer: context-fidelity
- Factor relation: single
- Depends on:
  - H-001 confirmed
- Rationale:
  - 用户要求自愈后的输出成为后续唯一正式上下文；当前 decoder 位于正式 ResponseItem 落账链之后，无法满足该合同。
- Falsifiable predictions:
  - If true: `record_completed_response_item` 接收 handler 尚未修补的原始 `item`。
  - If false: decoder 返回的修正版会反向替换 response scope、history 和 rollout 中的 FunctionCall。
- Evidence gate: satisfied
- Related evidence:
  - E-029
- Conclusion: confirmed。唯一正确接缝是 `session/turn` 收到完成 ResponseItem 后、response scope/history/rollout/dispatch 之前；Provider raw wire 可以保留原始 transport 证据，但不得成为 Agent 后续正式上下文。
- Repair design readiness: implemented and verified offline as SR-01～SR-03
- Next step: 只在获得新预算后复验 Provider 行为；不得用离线修复宣称 Agent 已稳定。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-029: FunctionCall 原文先于 Exec decoder 进入正式落账链
- Related hypotheses:
  - H-016
- Direction: supports
- Type: code-inspection
- Source: `session/turn.rs`、`stream_events_utils.rs`、`taskspace_exec/handler.rs`
- Prediction or plan link:
  - SR-02 canonical response replacement。
- Matched signal:
  - `OutputItemDone(item)` 先调用 response scope；`handle_output_item_done` 在 Tool dispatch 前调用 `record_completed_response_item(&item)`；TaskSpace plan decode 随后才在 handler 中发生。
- Correlation keys:
  - none
- Raw content:
  ```text
  OutputItemDone -> response_scope -> handle_output_item_done
                 -> record_completed_response_item(original item)
                 -> TaskSpaceExecHandler::decode_outer_call(arguments)
  ```
- Interpretation: handler-only 修补会制造执行事实与上下文事实分叉，必须在最早公共 ResponseItem 边界替换。
- Time: 2026-08-11

## Hypothesis H-017: waiting 失败来自 Agent 未先闭合父节点，而不是调用了 waiting 动作
- Status: confirmed
- Parent: P-001
- Claim: `waiting` 是 Map 根据未完成 parents 机械派生的节点状态，不是 Agent 可调用动作；本轮 Agent 在语义检查完成后直接提交 `apply_patch@fix`，但 Map 中 `inspect` 尚未被显式标为 completed，因此 `fix` 仍不可执行。
- Layer: agent-protocol-behavior
- Factor relation: single
- Depends on:
  - H-013 verified
- Rationale:
  - 节点生命周期只由 Agent 的显式 Map operation 改变，Tool 完成不自动完成节点；Runtime 不应替 Agent 推断 inspection 已完成。
- Falsifiable predictions:
  - If true: 拒绝前 Map 中 `fix.parents=[inspect]`、`inspect!=completed`、`fix=waiting`，且后续“complete inspect + apply_patch@fix”同批通过。
  - If false: `inspect` 已 completed，或 Agent 实际调用了名为 waiting 的 Tool/Map operation。
- Evidence gate: satisfied
- Related evidence:
  - E-027
  - E-030
- Conclusion: confirmed。DAG 和零副作用拒绝正确；明确工程缺口是 `ClientNodeNotExecutable` 只以 Debug 枚举返回状态，没有机械列出未完成父节点，增加了 Agent 重新推断成本。
- Repair design readiness: implemented and verified offline as WF-01
- Next step: 只在获得新预算后观察 Agent 是否减少一次 waiting 误用；不再扩大 Runtime 状态控制。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-030: 同一 Map 的父节点闭合后 fix 立即可执行
- Related hypotheses:
  - H-017
- Direction: supports
- Type: artifact-replay
- Source: `WAR-20260811-042531-CACHE-REGRESSION-4BB46AE7` diagnosis、Map export 和 outer results
- Prediction or plan link:
  - WF-01 waiting rejection fidelity。
- Matched signal:
  - Request 5 的 `apply_patch@fix` 在 `fix=waiting` 时零副作用拒绝；Request 6 先将 `inspect` completed，再在同批执行同一节点的 patch 并成功。
- Correlation keys:
  - `WAR-20260811-042531-CACHE-REGRESSION-4BB46AE7`
- Raw content:
  ```text
  request 5: apply_patch@fix -> ClientNodeNotExecutable Waiting
  request 6: complete inspect -> apply_patch@fix -> succeeded
  ```
- Interpretation: 不是 Tool 结果丢失、Map 推导错误或 Runtime 过度拒绝，而是 Agent 声明的节点生命周期落后于它已经完成的实际工作。
- Time: 2026-08-11

## Hypothesis H-018: patch 生命周期漏计来自专项消费者仍解析旧载体
- Status: confirmed
- Parent: P-001
- Claim: `patch-observability.ps1` 只把顶层 `apply_patch` 和旧 `taskspace_control` continuation 展开为 patch 声明，没有解析当前 `taskspace_exec.arguments.calls[].client`；因此主 Exec observer 能计量 client action，patch 专项却稳定报零。
- Layer: observability
- Factor relation: single
- Depends on:
  - H-014 verified
- Rationale:
  - 同一 rollout 中当前 wire 的两个 `client.name=apply_patch` 对主 observer 可见，对 patch 专项的两个旧分支均不可见。
- Falsifiable predictions:
  - If true: 静态代码只有 `provider_top_level` 和 `taskspace_control` 两种声明源；当前 artifact 离线复算补入 Exec client 后得到 2 次声明。
  - If false: patch 专项已经消费 `calls[].client`，零计数来自 rollout 缺失。
- Evidence gate: satisfied
- Related evidence:
  - E-028
  - E-031
- Conclusion: confirmed。缺口只在 benchmark consumer，不在 Runtime 执行、Map action 或 outer result；修复应复用当前 Exec action 解码事实，不增加第二 Runtime 事件源。
- Repair design readiness: implemented and verified offline as OB-03
- Next step: 完整 I07 仍随下一次生产验收结算；Patch 专项缺口不再阻塞离线报告。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-031: patch 专项代码没有当前 Exec client 分支
- Related hypotheses:
  - H-018
- Direction: supports
- Type: code-inspection
- Source: `scripts/taskspace-benchmark/lib/patch-observability.ps1` 与 `taskspace-exec-observation.ps1`
- Prediction or plan link:
  - OB-03 current-protocol patch observation。
- Matched signal:
  - patch 专项只展开 `name=taskspace_control` 的 legacy actions，其余按 provider 顶层 name 计数；主 Exec observer 已独立读取 `arguments.calls[].client.name`。
- Correlation keys:
  - none
- Raw content:
  ```text
  patch observer: provider top-level | taskspace_control continuation
  exec observer:  taskspace_exec -> calls[].client.name
  ```
- Interpretation: 两个消费者对同一当前协议使用了不同解析口径，导致 patch 指标与 Exec 主计数矛盾。
- Time: 2026-08-11

## Evidence E-032: 自愈后的 FunctionCall 在正式落账前替换原项
- Related hypotheses:
  - H-016
- Direction: supports
- Type: deterministic-test
- Source: `taskspace_exec/self_heal.rs`、`session/turn.rs`、`session/tests.rs`
- Matched signal:
  - 单个缺失 `}` 或 `]` 只有唯一候选通过严格 JSON 与当前 Catalog 时才改写；缺多个闭合符、合法输入和非 Exec 输入保持原样。
  - session test 证明 history 只记录修正版，错误参数串不进入正式历史。
- Interpretation: 执行、Map response scope 和后续上下文使用同一个修正版；自愈未下沉到 handler 形成双重事实。
- Time: 2026-08-11

## Evidence E-033: waiting 拒绝只增加机械父节点事实
- Related hypotheses:
  - H-017
- Direction: supports
- Type: deterministic-test
- Source: `taskspace_exec/preflight.rs`、`handler.rs`、preflight/handler tests
- Matched signal:
  - `ClientNodeNotExecutable` 携带 waiting 节点的未完成直接父节点；反馈同时声明整批 Map/client calls 零执行。
  - 80 个 TaskSpace Exec tests 全部通过，节点状态仍只由 Agent 的显式 Map operation 改变。
- Interpretation: Runtime 没有自动完成父节点、替 Agent 选点或注入下一步建议，只忠实暴露硬规则拒绝事实。
- Time: 2026-08-11

## Evidence E-034: 原始 VA-02 rollout 的 patch 专项复算与执行事实一致
- Related hypotheses:
  - H-018
- Direction: supports
- Type: artifact-replay
- Source: `target/cache-hit-regression/WAR-20260811-042531-CACHE-REGRESSION-4BB46AE7/.../artifacts/rollout.jsonl`
- Matched signal:
  - 修复后的 observer 输出 `declarations=2`、`preflight_rejects=1`、`dispatch_results=1`、`parse_failures=1`。
  - 三个 observer self-test 通过；当前 Exec 主 observer 与 patch 专项共用 canonical calls decoder。
- Interpretation: 历史零 patch 是消费者漏计，不是 Runtime 未执行；坏 JSON 单列为不可解析，不再静默混入零声明。
- Time: 2026-08-11

## Hypothesis H-019: 中文 arguments 使单闭合符自愈候选窗口偏离真实错误
- Status: confirmed
- Parent: P-001
- Claim: `serde_json` 的错误 `column` 是 UTF-8 字节列号，而 SR-01 用 Unicode 字符序号换算 byte offset；中文 Map content 使偏差超过 24-byte 候选窗口，因此原本唯一可修复的缺 `}` 没有进入 Catalog decode。
- Layer: context-fidelity
- Factor relation: single
- Depends on:
  - H-016 confirmed
- Rationale:
  - 最新 production 首请求与既有 ASCII fixtures 的结构相同，差异是错误点之前存在中文节点目标和内容。
- Falsifiable predictions:
  - If true: Rust parser column 与 UTF-8 byte offset 一致、与 Unicode character offset 不一致；改为 byte-column 后同形中文 fixture 唯一修复，ASCII 与拒绝边界不变。
  - If false: 候选窗口已覆盖缺符号位置，失败应来自多个合法 Catalog plan 或 hook 未进入。
- Evidence gate: satisfied
- Related evidence:
  - E-035
  - E-037
- Conclusion: confirmed。缺口只在 parser 坐标换算，不在 response hook、Catalog 或 DAG；修复不得扩大到逗号、开括号、字段或动作猜测。
- Repair design readiness: implemented and verified offline as SR-04
- Next step: 新真实预算获批前不得宣称在线通过。
- Blocker:
  - provider revalidation requires separate approval
- Close reason:
  - not closed

## Evidence E-035: 最新中文 payload 的 Rust/Python 错误位置差异符合 UTF-8 byte column
- Related hypotheses:
  - H-019
- Direction: supports
- Type: production-trace-analysis
- Source: `WAR-20260811-052713-CACHE-REGRESSION-AD3C808C` rollout requests 1 and 4
- Prediction or plan link:
  - SR-04 byte-column coordinate。
- Matched signal:
  - Request 1 Runtime 报 column 426，而 Unicode character position 为 351；Request 4 报 column 513，而 character position 为 462。偏差来自错误点前的中文 UTF-8 多字节内容。
  - 整轮没有 `taskspace.exec.arguments_self_healed`，首请求保持 parser reject。
- Correlation keys:
  - `WAR-20260811-052713-CACHE-REGRESSION-AD3C808C`
- Interpretation: ASCII 测试无法覆盖该坐标语义；候选搜索逻辑本身没有被 production 调用绕过。
- Time: 2026-08-11

## Hypothesis H-020: syntax reject 的无条件 wrapper 提示扭曲了实际失败语义
- Status: confirmed
- Parent: P-001
- Claim: `render_envelope_rejection` 对所有 `InvalidJson` 都追加“不要包 arguments”，即使输入没有 wrapper；Agent 把该无关提示当成主修复方向，并在第 9 请求实际生成 `{"arguments":"..."}`。
- Layer: tool-feedback
- Factor relation: single
- Depends on:
  - H-015 verified
- Rationale:
  - 错误反馈应忠实区分 syntax 与合法 JSON contract，不应给纯 parser failure 注入另一类错误的解释。
- Falsifiable predictions:
  - If true: Requests 1/4～8 均无顶层 `arguments` 却收到 no-wrapper 提示；Request 9 首次引入 wrapper，并被 typed envelope reject。
  - If false: wrapper 在提示前已存在，或提示只在 decoder 确认该字段后出现。
- Evidence gate: satisfied
- Related evidence:
  - E-036
  - E-037
- Conclusion: confirmed。反馈层造成了语义注入和错误方向强化；修复只做 typed error 分流，不提供 Agent 下一步建议。
- Repair design readiness: implemented and verified offline as FF-01
- Next step: 后续获批 trace 只观察是否仍出现 wrapper，不自动扩大 Runtime 约束。
- Blocker:
  - provider revalidation requires separate approval
- Close reason:
  - not closed

## Evidence E-036: Agent 在错误提示后由无 wrapper 转为真实 wrapper
- Related hypotheses:
  - H-020
- Direction: supports
- Type: production-trace-analysis
- Source: `WAR-20260811-052713-CACHE-REGRESSION-AD3C808C` rollout requests 4～9
- Prediction or plan link:
  - FF-01 feedback fidelity。
- Matched signal:
  - Requests 4～8 的原始 arguments 没有顶层 `arguments`，反馈却每次附带 no-wrapper 文本；reasoning 持续围绕 wrapper 自我纠正。
  - Request 9 首次提交合法 JSON 的顶层 `arguments` 字段，随后才收到与实际错误匹配的 contract reject。
- Correlation keys:
  - `WAR-20260811-052713-CACHE-REGRESSION-AD3C808C`
- Interpretation: 这是反馈层把另一类错误注入当前上下文后的可观测行为放大，不应归因于缓存、waiting 或 patch Tool。
- Time: 2026-08-11

## Evidence E-037: UTF-8 坐标与 typed feedback 修复通过确定性回归
- Related hypotheses:
  - H-019
  - H-020
- Direction: supports
- Type: deterministic-test
- Source: `self_heal.rs`、`plan.rs`、`handler.rs`、TaskSpace Exec tests
- Prediction or plan link:
  - SR-04、FF-01。
- Matched signal:
  - 中文 content 后缺一个 call-envelope `}` 的 fixture 被修复为原合法 arguments；多缺符号、非法 plan、合法输入边界保持不变。
  - syntax reject 不再包含 direct-calls/no-wrapper；只有实际顶层 `arguments` 字段触发该机械事实。
  - `cargo test -p codex-core --lib taskspace_exec --locked`：81 passed。
- Interpretation: 两个工程机制均已离线闭合；当前 production business failure 仍不能标记为在线修复。
- Time: 2026-08-11

## Evidence E-038: SR-04 / FF-01 后简单样本三次端到端通过
- Related hypotheses:
  - H-019
  - H-020
- Direction: supports
- Type: production-revalidation
- Source: `WAR-20260811-061236-R8-SELFHEAL-PKG-R01`、`WAR-20260811-061753-R8-SELFHEAL-PKG-R03`、`WAR-20260811-061935-R8-SELFHEAL-PKG-R04`
- Prediction or plan link:
  - SR-04 / FF-01 修复后生产路径不再放大为 syntax/wrapper 死循环。
- Matched signal:
  - 三次有效运行均完成正确 patch、公开验证、隐藏 oracle、Map 闭合和最终答复。
  - 共 21 个 Provider 请求；没有 syntax reject、顶层 `arguments` wrapper、顶层 client Tool 逃逸或错误 patch。
  - 三次均没有 `taskspace.exec.arguments_self_healed`，因为模型直接生成了合法 arguments。
- Correlation keys:
  - `R8-SELFHEAL-USD050-20260811`
- Interpretation: 修复后生产路径稳定性获得 3/3 支持；由于真实输入没有触发自愈条件，本证据对“自愈 hook 在线命中”保持中性，不能用未发生的事件反向宣称已动态验证。
- Time: 2026-08-11

## Evidence E-039: waiting 父节点反馈在线支持一次请求恢复
- Related hypotheses:
  - H-017
- Direction: supports
- Type: production-revalidation
- Source: `WAR-20260811-061935-R8-SELFHEAL-PKG-R04` requests 4～7
- Prediction or plan link:
  - WF-01 waiting rejection fidelity。
- Matched signal:
  - `apply_patch@fix` 和 `exec_command@verify` 分别在直接父节点 `explore`、`fix` 未完成时被零副作用拒绝。
  - 两次反馈均返回目标节点、`waiting` 状态、未完成直接父节点和整批零执行范围。
  - Agent 均在下一请求先完成父节点，再执行原子节点工作并成功；最终 Map 5 节点、4 边全部闭合。
- Correlation keys:
  - `WAR-20260811-061935-R8-SELFHEAL-PKG-R04`
- Interpretation: WF-01 的反馈缺口已在线闭合；Agent 仍可能误选 waiting 节点，属于 I04 行为观察，不应通过 Runtime 自动完成、选点或放宽 DAG 来掩盖。
- Time: 2026-08-11

## Hypothesis H-021: response-local Hosted 对账在 Agent 当轮漏声明后没有合法恢复路径
- Status: confirmed
- Parent: P-001
- Claim: Runtime 只允许 `taskspace_exec` 认领同一 Provider 响应内已经发生的 Hosted capability；如果 Agent 在该响应漏写归属，当轮会被正确拒绝，但下一响应再声明时旧事实已经离开 response scope，因此同样会被拒绝。反馈准确但合同不可恢复，诱发 Agent 把时序差异误判为能力注册变化。
- Layer: agent-protocol-behavior
- Factor relation: single
- Depends on:
  - H-013 verified
- Rationale:
  - Hosted 原始结果已经忠实进入自然上下文；缺口不是结果丢失，而是 Agent 无法在后续合法表达对前一响应事实的节点归属。
- Falsifiable predictions:
  - If true: 有 Hosted output 但漏声明的响应返回 `actual=[web_search], declared=[]`；紧接着只补声明的响应返回 `actual=[], declared=[web_search]`，且 capability identity 不变。
  - If false: 下一响应仍可认领前一事实，或请求间实际 capability schema/identity 发生变化。
- Evidence gate: satisfied
- Related evidence:
  - E-040
  - E-041
- Conclusion: confirmed。Runtime 的单次拒绝语义均准确，但 response-local 生命周期使第一次遗漏不可纠正。不能用额外同义提示、自动绑定或默认 Root 归属掩盖；跨响应待归属事实是否进入产品模型需用户决策。
- Repair design readiness: blocked on product decision
- Next step: 在“严格同响应”与“Agent 后续显式认领待归属事实”之间确认产品边界，再制定最小工程方案。
- Blocker:
  - product lifecycle decision required
- Close reason:
  - not closed

## Evidence E-040: 多个 web search 内部输出在生产路径只形成一个逻辑 capability
- Related hypotheses:
  - H-021
- Direction: neutral
- Type: production-revalidation
- Source: `WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE` request 5 and request 9
- Prediction or plan link:
  - LS-09 logical Hosted Tool aggregation。
- Matched signal:
  - Request 5 同一响应包含成功 `search` 和失败 `open_page`，preflight 只报告 `actual=[web_search]`，没有按内部项拆成两个动作。
  - Request 9 的失败 `find_in_page` 加一次 Agent `web_search` 声明通过，只返回一个 `hosted_results[0]`，且没有 Provider ID、output index 或 action subtype。
- Correlation keys:
  - `WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE`
- Interpretation: 用户指定的“web_search 内部过程不可拆分”修复在线成立；端到端失败必须从剩余时序合同解释，不能回退逐 output 模型。
- Time: 2026-08-13

## Evidence E-041: 同一 Hosted 事实的漏声明与下一轮补声明形成互逆拒绝
- Related hypotheses:
  - H-021
- Direction: supports
- Type: production-trace-analysis
- Source: `WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE` requests 5/6 and 10/11
- Prediction or plan link:
  - response-local ownership recovery boundary。
- Matched signal:
  - Requests 5、10 在真实搜索已发生但 Exec 漏声明时均返回 `actual=[web_search], declared=[]`。
  - Requests 6、11 在下一响应补声明时均返回 `actual=[], declared=[web_search]`。
  - 12 个请求的 capability identity 固定为 `18d7af7230501496c3a4011605f80ff00d8fb6e0cd32d73cc959174fb6665cf7`，`tool_choice` 无变化。
- Correlation keys:
  - `WAR-20260813-085518-CACHE-REGRESSION-CC73D9BE`
- Interpretation: Agent 所称“工具集状态切换”是对 response-local 事实的错误解释；实际能力面稳定。当前合同只报告错误，没有给 Agent 一个合法的后续认领动作。
- Time: 2026-08-13
