# Problem P-001: VA-02 首轮证据链未能如实结算
- Status: verifying
- Created: 2026-08-10 06:00
- Updated: 2026-08-10 06:12
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
- Current conclusion: 两项工程根因已修复并通过离线回归及原始 artifact 复算；首次多余花括号属于模型输出事实，目前没有证据支持 Runtime 容错，剩余一次真实运行用于稳定性验证。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - E-001 至 E-006；真实稳定性验证待完成
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
- Repair design readiness: decision required
- Next step: 用户决定是否允许无 Hosted output 时省略 `hosted_bindings`；若批准，先离线修改同源 schema/example/decoder，再申请新的最小真实预算。
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
