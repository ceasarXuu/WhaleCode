# Problem P-001: I07 请求、用量和 Provider 边界事实被混同
- Status: fixed
- Created: 2026-08-05 07:59
- Updated: 2026-08-05 08:58
- Objective: 让性能观察从唯一身份忠实还原 logical request、local attempt、boundary request、completed response 和 usage
- Symptoms:
  - 8 个 completed Provider responses 被 rollout summary 统计为 15 个请求并近似双计 token
  - 10 个 supervisor boundary requests 与 11 个本地 payload attempts 被判为 upstream mismatch
- Expected behavior:
  - no-ID TokenCount 只作为状态快照，不产生请求或 usage 记录
  - 本地 attempt、实际 boundary request 和 completed response 分开计数并按身份关联
  - 缺失、冲突或过期证据明确不可比较
- Actual behavior:
  - rollout summary 按带 `last_token_usage` 的事件条数聚合
  - boundary verifier 要求全部 `payload_captured` 与 boundary claims 列表严格相等
- Impact:
  - request、token、cache 和成本报告不可复算，可能误导版本与根因判断
- Reproduction:
  - `pwsh -File scripts/taskspace-benchmark/test-i07-characterization.ps1`
- Environment:
  - Linux；branch `whalecode-alpha`；起始 commit `6e3d78a3e`
- Known facts:
  - response-completed TokenCount 带完整 request identity；rate-limit snapshot 无身份但重复携带 last usage
  - ProviderWireTrace 在 `client.stream_request()` 前记录 `payload_captured`
  - Provider supervisor 的 boundary claims 是实际受监督边界证据
- Ruled out:
  - Provider 实际发送了 15 个请求；boundary/final-wire 只证明 8 个 completed requests
  - 必须删除 no-ID TokenCount；该事件仍服务 UI/rate-limit 状态
- Fix criteria:
  - 8/15 fixture 离线重放为 8 completed/usage 和 7 snapshots
  - 10/11 fixture报告 10 boundary、11 attempts、1 local-only failed attempt
  - 所有请求数消费者使用同一规范化事实，身份冲突 fail closed
  - TaskSpace Exec 接入前完成 I07-W0～W8，不运行真实 Whale Agent
- Current conclusion: request/usage、Provider 边界、projection 与 TaskSpace Exec 拒绝均已进入唯一事实消费链；最新真实 trace 离线对账通过
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - `476d60802` 接入默认 metrics，最新三轮真实 trace 对账为 `0/1/2`
- Close reason:
  - 原始漏报症状已由同一批生产 trace 的离线修复验证消除

## Hypothesis H-003: 默认 benchmark 未消费 TaskSpace Exec 拒绝事实
- Status: verified
- Parent: P-001
- Claim: TaskSpace Exec 专用 observer 已能识别拒绝，但默认 `metrics.json` 仍只复制旧 `taskspace_control` 失败字段，导致真实 Exec 拒绝稳定漏报为零
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 最新三轮 rollout 中存在 canonical Exec reject，而同轮默认 metrics 的旧 control/preflight/state 字段全部为零
- Falsifiable predictions:
  - If true: 专用 observer 离线读取同一 rollout 得到非零拒绝，默认 metrics 仍为零
  - If false: 默认 metrics 已直接消费专用 observer，或专用 observer 同样返回零
- Diagnostic evidence plan:
  - Prediction or clause under test: 对比同一 artifact 的 rollout、专用 observer 和默认 metrics
  - Signal: `taskspace_exec rejected`、`rejected_*_call_count`、旧 `control_*_failure_count`
  - Capture method: 静态追踪消费者接线，并离线回放 `WAR-20260820-000226-R8-MULTILINE-SELF-HEAL-R3`
  - Event name or marker:
    - `taskspace_exec rejected:`
  - Correlation keys:
    - outer `call_id`
  - Differentiates from:
    - Runtime 没有产生拒绝
    - observer 分类器无法识别拒绝
  - Supports if:
    - rollout 和专用 observer 非零，但默认 metrics 为零且代码只读取旧 control 字段
  - Refutes if:
    - 三者计数一致或拒绝日志不存在
  - Instrumentation status: permanent-observability
  - Instrumentation lifecycle:
    - 默认 metrics 直接复用唯一 Exec observer，不复制分类逻辑
- Evidence gate: satisfied
- Related evidence:
  - E-020
  - E-021
- Conclusion: confirmed and repaired; default metrics now reports the canonical Exec observation
- Repair design readiness: ready
- Next step: 接入默认 metrics 并离线回放
- Blocker:
  - none
- Close reason:
  - E-022 以原始真实 trace 验证默认 metrics 接线恢复

## Evidence E-022: 默认 metrics 修复后按真实 trace 返回 0/1/2
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `476d60802`；`WAR-20260820-000226-R8-MULTILINE-SELF-HEAL-R3/H011`
- Prediction or plan link:
  - H-003 的修复验证预测
- Matched signal:
  - run-1=0；run-2=1 个 state/preflight；run-3=2 个 state/preflight；unknown=0
- Correlation keys:
  - outer `call_id`
- Raw content:
  ```text
  taskspace_exec_rejected_call_count: 0 / 1 / 2
  taskspace_exec_rejected_state_call_count: 0 / 1 / 2
  taskspace_exec_rejected_unknown_call_count: 0 / 0 / 0
  ```
- Interpretation: 默认 benchmark 不再隐藏已发生的 Exec 状态拒绝
- Time: 2026-08-20 11:40

## Evidence E-020: 最新三轮真实 rollout 与默认 metrics 发生事实冲突
- Related hypotheses:
  - H-003
- Direction: supports
- Type: runtime-trace
- Source: `target/whale-agent-runs/WAR-20260820-000226-R8-MULTILINE-SELF-HEAL-R3/H011`
- Prediction or plan link:
  - H-003 的同一 artifact 对账预测
- Matched signal:
  - run-2 有 1 次 waiting reject；run-3 有 1 次 waiting reject 和 1 次 `TransitionInvalid`
  - 三轮默认 `metrics.json` 的旧 sequence/control/state 失败字段仍全部为 0
- Correlation keys:
  - `call_00_EktJAVBAvXWDJ06BfP0s8995`
  - `call_00_8DUlnhC45qPHmREyahHm2870`
  - `call_00_kJnc0IRcxvY3kugY0sXi6848`
- Raw content:
  ```text
  actual TaskSpace Exec rejects: run-1=0, run-2=1, run-3=2
  default metrics reported legacy failure counts: run-1=0, run-2=0, run-3=0
  ```
- Interpretation: 默认结果漏报，不是拒绝未发生
- Time: 2026-08-20 11:20

## Evidence E-021: 默认 metrics 接线只读取旧 TaskSpace Control 字段
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `scripts/taskspace-benchmark/lib/metrics-extractor.ps1`
- Prediction or plan link:
  - H-003 的消费者接线预测
- Matched signal:
  - 默认 metrics 复制 `taskspace_control_usage.sequence_preflight_rejected_call_count` 与 `control_*_failure_count`
  - `taskspace-exec-observation.ps1` 仅由可选 performance observer 消费
- Correlation keys:
  - `Get-TaskspaceBenchmarkMetrics`
  - `Get-TaskspaceExecObservation`
- Raw content:
  ```text
  default metrics path -> legacy taskspace_control_usage
  optional performance path -> taskspace_exec-observation
  ```
- Interpretation: 漏报来自明确的最终消费者缺口
- Time: 2026-08-20 11:20

## Hypothesis H-001: no-ID 状态快照被按请求完成事件消费
- Status: confirmed
- Parent: P-001
- Claim: `New-TaskspaceRolloutRequestTraceSummary` 忽略 Provider identity，把 rate-limit snapshot 重复携带的 last usage 当成新请求
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 8/15 差值恰好等于前 7 个请求的 paired no-ID snapshots
- Falsifiable predictions:
  - If true: 脱敏 8-request fixture 会被旧 analyzer 计为 15，按完整 identity 聚合后为 8
  - If false: 排除 no-ID snapshots 后仍不是 8，或 snapshots 携带另一请求 identity
- Diagnostic evidence plan:
  - Prediction or clause under test: 旧 analyzer 对真实事件形态稳定复现 8/15
  - Signal: `model_request_count` 与 TokenCount identity 分布
  - Capture method: 脱敏 fixture 调用当前 `New-TaskspaceRolloutRequestTraceSummary`
  - Event name or marker:
    - `token_count`
  - Correlation keys:
    - `provider_request_id`
    - `provider_logical_request_id`
    - `provider_attempt_seq`
  - Differentiates from:
    - Provider 实际发出额外请求
  - Supports if:
    - 8 个有身份 completed usage + 7 个无身份 snapshots 被计为 15
  - Refutes if:
    - 当前 analyzer 已按身份去重或 fixture 不复现
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 将 snapshot 排除数保留为 observer 自诊断
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: confirmed by source and real trace
- Repair design readiness: ready
- Next step: I07-W2/W3
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: payload capture 被误解释为 boundary-accepted request
- Status: confirmed
- Parent: P-001
- Claim: Provider wire tracer 在 transport 调用前记录 local attempt，而 verifier 将全部 attempts 与 supervisor boundary claims 强制一一相等
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 10/11 run 的额外 payload attempt 以 response_failed 结束，supervisor 没有对应 claim
- Falsifiable predictions:
  - If true: 旧 verifier 对 10 claims + 11 attempts 报 mismatch；第 11 attempt 不在 boundary 且未 completed
  - If false: record_request 发生在 transport 接收后，或 boundary 中存在第 11 个 claim
- Diagnostic evidence plan:
  - Prediction or clause under test: 对照 record_request/stream_request 顺序并重放 10/11 事件关系
  - Signal: 源码调用顺序、payload digest 集合、terminal status
  - Capture method: 读取 `client.rs` 和 `provider_wire_trace.rs`；运行脱敏 verifier fixture
  - Event name or marker:
    - `provider.chat_wire_request_terminal`
    - `provider_request_claimed`
  - Correlation keys:
    - `request_id`
    - `provider_payload_sha256`
  - Differentiates from:
    - 上游存在未被 supervisor 记录的成功请求
  - Supports if:
    - 第 11 个 attempt 位于 boundary 集合之外且 terminal=response_failed
  - Refutes if:
    - 第 11 个 digest 存在于 boundary 或 terminal=response_completed
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 attempt/boundary/completion 三阶段计数
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-004
- Conclusion: confirmed by source and real trace
- Repair design readiness: ready
- Next step: I07-W2/W4
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 真实 rollout 的 8 个完成事件与 7 个无身份快照
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002/.../rollout.jsonl`
- Prediction or plan link:
  - H-001 的 paired event 数量预测
- Matched signal:
  - 8 full-identity TokenCount + 7 no-ID TokenCount with repeated last usage
- Correlation keys:
  - run `WAR-20260805-063652-R8-NESTED-RESULT-VISIBILITY-002`
- Raw content:
  ```text
completed identity events = 8
no-id last-usage snapshots = 7
legacy rollout request summary = 15
  ```
- Interpretation: 15 来自事件语义混淆，不是 Provider 请求放大
- Time: 2026-08-05 07:59

## Evidence E-002: provider wire 在 transport 调用前记录 payload
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/client.rs:2903-2919`
- Prediction or plan link:
  - H-002 的调用顺序预测
- Matched signal:
  - budget admission -> `record_request()` -> `client.stream_request()`
- Correlation keys:
  - logical request id
  - attempt sequence
- Raw content:
  ```text
provider_request_budget.before_dispatch(...)
provider_wire_trace.record_request(...)
client.stream_request(request, options).await
  ```
- Interpretation: `payload_captured` 是 local attempt 证据，不足以证明 boundary accepted
- Time: 2026-08-05 07:59

## Evidence E-003: W0 usage characterization fixture 复现旧错误
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `scripts/taskspace-benchmark/test-i07-characterization.ps1`
- Prediction or plan link:
  - H-001 的旧 analyzer 复现预测
- Matched signal:
  - legacy analyzer returned `model_request_count=15` for 8 identified completions
- Correlation keys:
  - `usage-double-count-rollout.jsonl`
- Raw content:
  ```text
  I07 characterization: PASS (legacy 8/15 and 10/11 defects reproduced)
  usage model_request_count = 15
  ```
- Interpretation: 脱敏最小输入独立复现同一双计机制，H-001 证据门保持 satisfied
- Time: 2026-08-05 07:59

## Evidence E-004: W0 boundary characterization fixture 复现旧错误
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: `scripts/taskspace-benchmark/test-i07-characterization.ps1`
- Prediction or plan link:
  - H-002 的 10/11 mismatch 预测
- Matched signal:
  - verifier returned exit 3 with boundary=10、wire attempts=11 and `provider_dispatch_trace_mismatch`
- Correlation keys:
  - `attempt-boundary-events.jsonl`
  - `attempt-boundary-wire.jsonl`
- Raw content:
  ```text
  I07 characterization: PASS (legacy 8/15 and 10/11 defects reproduced)
  boundary_request_count = 10
  wire_request_count = 11
  errors = [provider_dispatch_trace_mismatch]
  ```
- Interpretation: 脱敏最小输入独立复现 attempt/boundary 阶段混淆，H-002 证据门保持 satisfied
- Time: 2026-08-05 07:59

## Evidence E-005: W1 消费面 inventory 门禁
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/check-request-fact-consumers.py`
- Prediction or plan link:
  - I07-W1 全量消费面分类
- Matched signal:
  - 10 个生产 reader/producer 均登记其原始来源和目标事实
  - 新增未登记 reader 的负例被拒绝
  - 测试 support 不触发误报
- Correlation keys:
  - `whalecode-request-fact-consumers-v1`
- Raw content:
  ```text
request fact consumer gate: PASS
Ran 3 tests ... OK
  ```
- Interpretation: 后续 W2-W7 的迁移范围可由机器清单约束，不再依赖手工记忆
- Time: 2026-08-05 08:15

## Evidence E-006: W2 单一 request facts 合同
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/test_request_facts.py`
- Prediction or plan link:
  - I07-W2 规范化事实生成器
- Matched signal:
  - 8/15 fixture 归一为 8 completed、8 usage、7 snapshots
  - 10/11 fixture 归一为 11 attempts、10 boundary、10 completed、1 local-only failure
  - partial identity、冲突重复、completed-without-boundary 和未知 boundary 均 fail closed
  - 相同输入重复生成 byte-identical artifact
- Correlation keys:
  - `whalecode-request-facts-v1`
  - analyzer `i07-w2-v1`
- Raw content:
  ```text
Ran 10 tests ... OK
request fact consumer gate: PASS
cmp request-facts-a.json request-facts-b.json: equal
  ```
- Interpretation: W3-W7 可从一份规范化 artifact 派生各自视图，不再复制事件语义判断
- Time: 2026-08-05 08:25

## Evidence E-007: W3 request/usage 双计修复
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/test-i07-characterization.ps1`
- Prediction or plan link:
  - I07-W3 只统计完整请求身份的 completed usage
- Matched signal:
  - 历史形态 fixture 从 15 修正为 8
  - 7 个 no-ID 状态快照仍被保留并单独计数
  - malformed rollout 的请求数 fail closed，不从剩余行估算
  - cost、metrics 和 benchmark harness 均通过
- Correlation keys:
  - `taskspace-rollout-request-trace-v2`
  - `request_facts_completed_usage`
- Raw content:
  ```text
I07 characterization: PASS (usage 8/15 fixed; legacy boundary 10/11 reproduced)
cost instrumentation selftest passed
TaskSpace metrics extractor harness self-test: PASS
TaskSpace benchmark harness self-test: PASS
  ```
- Interpretation: H-001 根因已在唯一 classifier 层修复；Runtime 状态广播和 Agent 上下文均未改变
- Time: 2026-08-05 08:30

## Evidence E-008: W4 attempt/boundary 分层对账
- Related hypotheses:
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/test-i07-characterization.ps1`
- Prediction or plan link:
  - I07-W4 按阶段关系核对，不要求 local attempt 与 boundary 列表无条件相等
- Matched signal:
  - 11 local attempts、10 boundary requests、10 completed、1 local-only failure 被判 reconciled
  - completed-without-boundary 与未知 boundary digest 仍阻断
  - 219 个 cache regression tests 在 boundary evidence v2 下通过
- Correlation keys:
  - `whalecode-provider-boundary-evidence-v2`
  - analyzer `i07-w2-v1`
- Raw content:
  ```text
I07 characterization: PASS (usage 8/15 and boundary 10/11 fixed)
Ran 12 tests ... OK
Ran 219 tests ... OK
  ```
- Interpretation: H-002 的错误来自 observer 阶段混淆；现在本地失败和上游不一致具有不同、可复算的事实表达
- Time: 2026-08-05 08:40

## Evidence E-009: W5 性能请求计数迁移
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/test-performance-observation.ps1`
- Prediction or plan link:
  - I07-W5 主报表统一读取 canonical facts
- Matched signal:
  - performance observation 不再读取 `payload_captured`
  - logical、local attempt、boundary、completed 和 failed/cancelled 并列暴露
  - 五层 trace 动态创建请求行，由 canonical rollout line identity 结算，不按预期数量预分配
  - request consumer gate 不再登记这两个旧 raw reader
- Correlation keys:
  - `request_facts_boundary`
  - `rollout_line_number`
- Raw content:
  ```text
R7 five-layer trace analysis passed.
R7 supplemental failure evidence passed.
performance observation self-test passed
TaskSpace benchmark harness self-test: PASS
request fact consumer gate: PASS
  ```
- Interpretation: 横向报告的请求分母已统一；保留的 wire/lifecycle reader 只提供独特详情，并受 canonical count 对账约束
- Time: 2026-08-05 08:55

## Evidence E-010: W6 缓存与 section-cost 消费者迁移
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/cache-regression/cache_usage_contract.py`
- Prediction or plan link:
  - I07-W6 缓存分母只使用 completed measured usage，shape 仍按 local attempt 观测
- Matched signal:
  - cache usage strict contract 改由 canonical facts 判断全部 attempt 是否完成且有 usage
  - provider cache trace 的 shape 继续来自 final-wire attempt，status/usage 只从 canonical rows 取得
  - 缺少 wire identity 时不再从 budget event 合成缓存请求，明确输出 unavailable source
  - failed、missing terminal 和 retry 的通用 facts 保真，严格缓存基线仍按原合同阻断
- Correlation keys:
  - `request_facts_completed_usage`
  - `TaskSpaceProviderCacheTraceSummaryV4`
  - analyzer `i07-w2-v1`
- Raw content:
  ```text
Ran 219 tests ... OK
cost instrumentation selftest passed
TaskSpace benchmark harness self-test: PASS
performance observation self-test passed
request fact consumer gate: PASS
  ```
- Interpretation: 缓存分母与 shape 观察已拆开但共用一份请求事实；W6 未修改 Provider payload、Agent 上下文或 Runtime 行为
- Time: 2026-08-05 08:43

## Evidence E-011: W7 来源封存与新鲜度门禁
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/test-r7-request-facts-provenance.ps1`
- Prediction or plan link:
  - I07-W7 请求事实、原始来源和 analyzer 必须属于同一次可复算证据
- Matched signal:
  - `request-facts.json` 记录 rollout、wire、boundary 的读取状态与 SHA-256
  - run evidence manifest v2 封存 request facts、来源文件、analyzer 版本和四个 analyzer 文件的组合哈希
  - freshness 从 canonical boundary/completion facts 取得请求数，raw wire 只保留形状与协议身份检查
  - 修改原始 trace 但不改变规范化计数时，旧 facts 仍以 `request_facts_stale` 阻断
- Correlation keys:
  - `r7-artifact-evidence-manifest` v2
  - `request_facts_stale`
  - analyzer `i07-w2-v1`
- Raw content:
  ```text
R7 request facts provenance self-test passed.
R7 five-layer evidence freshness self-test passed.
Ran 10 tests ... OK
request fact consumer gate: PASS
  ```
- Interpretation: 当前报告不能再把旧 raw source、新 analyzer 和新汇总拼成一次新鲜运行；没有 boundary 时保持 unavailable，不复制 attempt count
- Time: 2026-08-05 08:52

## Evidence E-012: W8 observer 自观测合同
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/test_request_facts.py`
- Prediction or plan link:
  - I07-W8 汇总变化必须可由 observer 自身诊断，不记录业务正文
- Matched signal:
  - diagnostics 记录三个来源的原始事件数和 normalized row/attempt/boundary/completion/usage 数
  - snapshot 排除、幂等去重、boundary matched/unattributed/local-only 均有独立守恒计数
  - findings 按稳定 code/source 聚合，汇总可由 normalized rows 重算
  - 负向扫描确认 diagnostics 不包含 prompt、command、arguments、Tool output 或 content
- Correlation keys:
  - `whalecode-request-facts-diagnostics-v1`
  - analyzer `i07-w8-v1`
- Raw content:
  ```text
Ran 11 tests ... OK
request fact consumer gate: PASS
Ran 219 tests ... OK
cost instrumentation selftest passed
TaskSpace benchmark harness self-test: PASS
performance observation self-test passed
R7 request facts provenance self-test passed.
R7 five-layer evidence freshness self-test passed.
I07 characterization: PASS (usage 8/15 and boundary 10/11 fixed)
  ```
- Interpretation: observer 数字变化现在可以定位到输入、排除、对账或 finding 分类；诊断产物不进入 Agent context，也不记录敏感业务语义
- Time: 2026-08-05 08:55

## Evidence E-013: W10 独立修复离线结算
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: test
- Source: `scripts/taskspace-benchmark/test-r7-request-observability-report.ps1`
- Prediction or plan link:
  - I07-W10 在零 API 成本下结算当前协议的观测基础
- Matched signal:
  - 两个历史反例与所有负例保持预期
  - 24-run 本地确定性矩阵逐 run 封存 request facts、sources、analyzer 和 commit provenance
  - 完整 report 可在 clean worktree 从 sealed artifacts 重建并通过
- Correlation keys:
  - commit `63b0336d3`
  - `r7-artifact-evidence-manifest` v2
- Raw content:
  ```text
R7 request observability report passed.
git status --short: empty
real Whale Agent runs: 0
  ```
- Interpretation: 当前协议下可独立修复的 I07 缺陷已完成；W9/W11 仍依赖 TaskSpace Exec，不因本结果自动关闭
- Time: 2026-08-05 08:58

## Evidence E-014: R7 observer 第二套请求事实模型被删除
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: deterministic-test
- Source: commit `7a4346156`
- Prediction or plan link:
  - 请求身份、attempt、terminal 和 usage 必须只有 canonical `request-facts.json` 一个权威模型
- Matched signal:
  - 生产 report 将已封存的 request facts 直接传入 observer
  - observer 不再解析 raw terminal identity/status/usage，只读取 wire 独有的 shape、LCP、transport 和 final-control identity
  - 重复 wire attempt/terminal 由 canonical facts 标记为 incomparable；重复 rollout 状态快照仍保持幂等
  - consumer inventory 的代码合同禁止 observer 重新读取 terminal 身份字段
- Raw content:
  ```text
  Python request facts: 22/22 passed
  R7 five-layer trace analysis passed
  R7 provider token identity passed
  R7 request observability report passed
  performance observation self-test passed
  ```
- Interpretation: I07 的生产消费者不再维护平行请求事实模型；本结果仍是离线证据，真实运行产物和停止结算待获批验收
- Time: 2026-08-18
