# Problem P-001: 缓存 runner 拒绝完整的 provider usage 证据
- Status: verifying
- Created: 2026-08-02 16:59
- Updated: 2026-08-02 17:17
- Objective: 让缓存回归只使用逐次 provider terminal 证据计算请求数、token 和缓存命中，并拒绝真正缺失或矛盾的 usage。
- Symptoms:
  - 获批运行完成 Standard 样本后，runner 报 `ValueError: input_tokens must be a nonnegative integer`，停止执行 map-request。
- Expected behavior:
  - 5 个 provider 响应均提供完整整数 usage 时，runner 应准确结算 Standard，并继续执行获批的下一 arm。
- Actual behavior:
  - runner 将 rollout 派生摘要中的 `108402.0` 交给整数合同；严格合同拒绝该值，账本 token 与费用被结算为不可用。
- Impact:
  - R8 Tool Sequence MVT-0 未完成；已消费 1 个真实样本和 5 次 provider 请求，但无法形成有效缓存对照。
- Reproduction:
  - 运行记录 `WAR-20260802-165454-CACHE-REGRESSION-2723DE14`，执行 Standard `single-file-fast-fix` repeat 1。
- Environment:
  - Linux；分支 `whalecode-alpha`；subject HEAD `2390597f7`；模型 `deepseek-v4-flash`。
- Known facts:
  - provider wire 有 5 个 `response_completed`，usage 均为非负整数，合计 input 60,617、cached input 48,128、output 810。
  - `request-summary.rollout_trace` 报 9 次请求、input 108,402.0、cached input 83,584.0、output 1,435.0。
  - rollout 在前四个 provider 响应后各出现两份相同的 `last_token_usage` 累计快照；最后一次出现一份，因此被误计为 9 次。
  - `cache_run_analysis.py` 使用 cache summary 的缓存分项，却使用 `request-summary.rollout_trace` 的 token 总量，混合了两套不同权威边界。
- Ruled out:
  - 不是 DeepSeek 漏报 usage；5 个 terminal 记录全部完整。
  - 不是 Agent 业务失败；Standard `business_success` 为 true，公开与隐藏验证均通过。
  - 不是 provider boundary 请求计数缺失；boundary 已完整对账为 5。
- Fix criteria:
  - 缓存 runner 从同一组 provider terminal 记录计算请求数和所有 token 字段，不再依赖重复的 rollout token 快照。
  - 对 5 个现有 terminal 证据的离线分析得到 input 60,617、cached input 48,128、output 810、request 2+ hit rate 0.970812。
  - 缺失、非整数或与 provider boundary 不一致的 terminal usage 继续 fail closed。
  - 新授权下的 Standard + map-request 最小回归能够完整结算；在获批前不得再次发起真实运行。
- Current conclusion: 根因已在提交 `0076e720a` 修复并通过原始 artifact 离线复算；新授权下的双臂真实结算仍是唯一未完成验收。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-001、H-002；E-001 至 E-007；真实双臂验收待预算
- Close reason:
  - not closed

## Hypothesis H-001: runner 混用了 provider cache 证据与 rollout token 摘要
- Status: confirmed
- Parent: P-001
- Claim: `cache_run_analysis.py` 用 provider wire 派生的 cache summary 校验缓存分项，却从 rollout `request-summary` 读取 token 总量，导致同一 observation 的字段来自不一致的计量边界。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-002
- Rationale:
  - 报错字段来自 `request-summary`，而 wire terminal usage 已完整且与 metrics 一致。
- Falsifiable predictions:
  - If true: analyzer 会读取 `request-summary["rollout_trace"]` 作为 request token；该值与 provider wire terminal 合计不同。
  - If false: analyzer 应直接读取 provider wire terminal usage，或两份合计应完全一致。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对齐 analyzer 输入路径、provider wire 合计和 rollout 摘要合计。
  - Signal: 代码参数来源与三份 artifact 的请求/token 数。
  - Capture method: 静态代码追踪和现有 artifact 的离线 `jq` 聚合。
  - Event name or marker:
    - `response_completed`
    - `input_tokens must be a nonnegative integer`
  - Correlation keys:
    - `WAR-20260802-165454-CACHE-REGRESSION-2723DE14`
  - Differentiates from:
    - provider usage 缺失或严格合同错误。
  - Supports if:
    - analyzer 读取 rollout 摘要，且它与完整 provider terminal 合计冲突。
  - Refutes if:
    - analyzer 已以 provider terminal 为唯一 token 来源。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 修复后持久化 provider terminal usage 来源与对账状态。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-004
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: 取得新预算后执行 Standard + map-request 各一次，不自动重试。
- Blocker:
  - real-run authorization required
- Close reason:
  - not closed

## Hypothesis H-002: rollout token_count 重复快照被误当成独立 provider 请求
- Status: confirmed
- Parent: P-001
- Claim: `New-TaskspaceRolloutRequestTraceSummary` 对每条含 `last_token_usage` 的 `token_count` 事件直接累加，而 Codex 会在工具阶段再次发送相同累计快照，导致请求数和 token 双计。
- Layer: sub-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - rollout 摘要报 9 次，而 provider boundary 与 wire 均为 5 次。
- Falsifiable predictions:
  - If true: 前四个响应的相同 `total_token_usage`/`last_token_usage` 各连续出现两次，最后一次出现一次，形成 9 条 usage 事件。
  - If false: 9 条事件应分别对应 9 个不同 provider terminal 或不同累计 usage。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较 rollout token_count 序列与 provider wire terminal 序列。
  - Signal: 相邻累计 usage 身份、provider terminal 数与各自合计。
  - Capture method: 对现有 JSONL 做只读结构化查询。
  - Event name or marker:
    - `token_count`
    - `response_completed`
  - Correlation keys:
    - rollout session `019fc1af-3568-7f93-bb4e-a1049308daec`
  - Differentiates from:
    - provider 实际发生 9 次请求但 boundary 漏计。
  - Supports if:
    - 四组快照完全重复，且 provider terminal 只有 5 个。
  - Refutes if:
    - 每个 rollout usage 都有唯一 provider terminal 对应。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - rollout 摘要仍可用于诊断，但不能再作为缓存 runner 的 provider 计费真值；其去重另列非阻塞观测修复。
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-003
- Conclusion: confirmed
- Repair design readiness: implemented
- Next step: rollout 摘要保留为诊断附件；其通用统计准确性不再阻塞缓存 runner。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: provider terminal usage 完整且可精确求和
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `benchmarks/cache-regression/evidence/WAR-20260802-165454-CACHE-REGRESSION-2723DE14/WAR-20260802-165454-CACHE-REGRESSION-2723DE14-CACHE-001/usage-reconciliation.json`（原始 source SHA-256 同文件保存）
- Prediction or plan link:
  - H-001 对 provider 真值与 rollout 摘要冲突的预测。
- Matched signal:
  - 5 个 `response_completed` 均含完整整数 usage。
- Correlation keys:
  - `WAR-20260802-165454-CACHE-REGRESSION-2723DE14-CACHE-001`
- Raw content:
  ```text
  count=5 input_tokens=60617 cached_input_tokens=48128 output_tokens=810
  ```
- Interpretation: provider usage 可用，runner 不应以 usage 缺失结算。
- Time: 2026-08-02 16:58

## Evidence E-002: rollout 摘要与 provider terminal 冲突
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: reproduction
- Source: `benchmarks/cache-regression/evidence/WAR-20260802-165454-CACHE-REGRESSION-2723DE14/WAR-20260802-165454-CACHE-REGRESSION-2723DE14-CACHE-001/usage-reconciliation.json`
- Prediction or plan link:
  - H-001/H-002 的不同计量边界预测。
- Matched signal:
  - rollout 为 9 次、108402.0 input；provider boundary/wire 为 5 次、60617 input。
- Correlation keys:
  - `WAR-20260802-165454-CACHE-REGRESSION-2723DE14`
- Raw content:
  ```text
  rollout_trace: requests=9 input=108402.0 cached=83584.0 output=1435.0
  provider_wire: requests=5 input=60617 cached=48128 output=810
  ```
- Interpretation: 两份 artifact 不是可互换的 token 事实源。
- Time: 2026-08-02 16:58

## Evidence E-003: rollout 中存在四组重复累计快照
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: `benchmarks/cache-regression/evidence/WAR-20260802-165454-CACHE-REGRESSION-2723DE14/WAR-20260802-165454-CACHE-REGRESSION-2723DE14-CACHE-001/usage-reconciliation.json`（原始 rollout SHA-256 同文件保存）
- Prediction or plan link:
  - H-002 的重复快照预测。
- Matched signal:
  - 累计 input 11042、22764、35071、47785 各连续出现两次，60617 出现一次。
- Correlation keys:
  - rollout session `019fc1af-3568-7f93-bb4e-a1049308daec`
- Raw content:
  ```text
  total input sequence: 11042,11042,22764,22764,35071,35071,47785,47785,60617
  ```
- Interpretation: 9 条 token_count 事件只代表 5 次 provider 请求，当前直接累加必然双计。
- Time: 2026-08-02 16:58

## Evidence E-004: analyzer 明确选择 rollout 摘要作为 token 输入
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `scripts/cache-regression/cache_run_analysis.py`
- Prediction or plan link:
  - H-001 的 analyzer 数据路径预测。
- Matched signal:
  - `analyze_artifacts` 执行 `request = read_json(request_path)["rollout_trace"]`，随后交给 `validate_cache_artifacts(cache, request)`。
- Correlation keys:
  - `analyze_artifacts`
- Raw content:
  ```text
  request = read_json(request_path)["rollout_trace"]
  usage = validate_cache_artifacts(cache, request)
  ```
- Interpretation: 失败机制由生产代码与运行 artifact 共同闭合，不是推测。
- Time: 2026-08-02 16:59

## Evidence E-005: provider terminal 成为唯一 usage 事实源
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `scripts/cache-regression/cache_usage_contract.py`、`cache_run_analysis.py`；提交 `0076e720a`
- Prediction or plan link:
  - H-001 的修复验收。
- Matched signal:
  - analyzer 解析 `provider-wire-trace.jsonl`，并按 request ID、payload hash 与 provider boundary 严格对账。
- Correlation keys:
  - `parse_provider_wire_usage`
- Raw content:
  ```text
  usage = validate_cache_artifacts(cache, provider_usage)
  provider terminal usage does not match provider boundary evidence
  ```
- Interpretation: token、缓存分项和请求边界不再混用 rollout 事实。
- Time: 2026-08-02 17:10

## Evidence E-006: 原始失败 artifact 离线复算成功
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: fix-validation
- Source: `analyze_arm()` 对运行 `WAR-20260802-165454-CACHE-REGRESSION-2723DE14-CACHE-001` 的只读重算
- Prediction or plan link:
  - P-001 的原始症状复验。
- Matched signal:
  - 不再出现整数合同错误，得到完整 provider 数值。
- Correlation keys:
  - `WAR-20260802-165454-CACHE-REGRESSION-2723DE14-CACHE-001`
- Raw content:
  ```text
  requests=5 input=60617 cached=48128 uncached=12489 output=810 hit_rate=0.970812 business_success=true
  ```
- Interpretation: 修复直接消除了原始证据汇总失败，未通过重新运行或类型强转掩盖问题。
- Time: 2026-08-02 17:13

## Evidence E-007: 离线回归和缓存门禁通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: Python cache-regression suite、Ruff、cache regression gate
- Prediction or plan link:
  - P-001 的 fail-closed 与回归验收。
- Matched signal:
  - 219 项测试通过；Ruff 通过；staged index 门禁通过且发布仍保持待真实验证阻断。
- Correlation keys:
  - commit `0076e720a`
- Raw content:
  ```text
  Ran 219 tests: OK
  cache regression gate: PASS 204978af... (pending live verification; release blocked)
  ```
- Interpretation: 修复没有放宽缺失/矛盾 usage，也没有改变当前 provider 上下文指纹。
- Time: 2026-08-02 17:16
