# Problem P-001: VA-02 首轮证据链未能如实结算
- Status: verifying
- Created: 2026-08-10 06:00
- Updated: 2026-08-10 08:18
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
- Current conclusion: 零 Hosted 合同已在 `WAR-20260810-061241-CACHE-REGRESSION-A143B6F0` 首响应在线通过。第二响应失败的主根因已确认：`taskspace_exec.calls[]` 用原生 Tool 名、`arguments` 和 TaskSpace-only `node_id` 描述内层调用，与 Provider 顶层 Function Call 的语义和结构过于同形；DeepSeek 会把整个内层分支提升并扁平化成顶层 `exec_command`。两次非法调用都携带原生 `exec_command` schema 不接受的 `node_id`，直接留下内层 wire 来源指纹。TaskSpace 复用 Standard base instructions 是放大该歧义的已确认诱因，但不是充分根因：历史 Function Exec 使用同一模型、同一 base、同样的 Function 外层，只把内层改为明显不同的 JavaScript `tools.*` 语法，连续 15 次均保持顶层 `exec`。两次 Provider wire 的顶层 Tool 集合始终为 `taskspace_exec + web_search`，适配层和 Runtime 均原样透传 Function name，Runtime 正确 fail closed；不是 Tool 重新暴露、名称改写、反馈丢失、总 schema 大小或 Map 状态机问题。VA-02 仍因 I03 当前生产表现未关闭。
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
  - E-001 至 E-018；H-007 已通过来源指纹与历史对照两条独立证据链确认；H-005 降为促成因素，H-004/H-006/H-008/H-009 已排除
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
- Repair design readiness: yes
- Next step: 单独设计最小候选 wire，并用离线合同先消除同形性；真实稳定性验证需另行预算。
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
