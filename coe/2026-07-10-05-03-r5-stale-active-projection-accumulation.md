# Problem P-001: R5 active projection 未替换旧快照并持续累积
- Status: fixed
- Created: 2026-07-10 05:03
- Updated: 2026-07-10 05:22
- Objective: provider-visible TaskSpace history 在任一请求中只保留最新一份 active projection，同时忠实保留当前工具反馈和状态机拒绝，消除旧状态冲突与重复 token。
- Symptoms:
  - `count-call-stack` R5-E 的 provider payload 从首轮 32870 bytes 增长到末轮 131830 bytes，input 从 8389 增长到 33631 tokens。
  - rollout 中存在 14 份 `ContextProjectionV1 active replacement`，旧 projection 的 running 状态与最新 completed 状态同时进入 history。
  - Agent 在第一次 `finish_node` 成功、最新 projection 已显示 completed 后又判断节点仍 running 并重复 finish。
- Expected behavior:
  - active projection 是状态快照替换，不是追加日志；每个 provider request 最多出现一份且必须是最新 projection。
  - 当前 tool/gate feedback 必须继续成对保留，不能用 projection replacement 吞掉。
  - exact payload scanner 必须验证 projection uniqueness，不能只验证 marker 存在。
- Actual behavior:
  - `compose_provider_visible_history` 找到 latest projection，但只用于 final-readiness 判断。
  - `provider_visible_history_action` 对所有 `ActiveProjection` 返回 `Include`，旧 projection 全部进入 provider payload。
  - `replacement_confirmed=true` 没有检查 active projection 数量。
- Impact:
  - provider payload 和 token 随请求轮次至少近似二次累积。
  - Agent 同时看到多份互相冲突、都自称 active replacement 的状态，造成语义歧义和错误动作。
  - R5-E live sample 的成本及重复 finish 归因被污染，R5-F 在修复前不得开始行为保持型拆分。
- Reproduction:
  - 检查 `target/r5e-phase-e-final-clean/count-call-stack/20260710-043411-389/pair-001/right/artifacts/rollout.jsonl` 中 14 份 active projection。
  - 对照 `provider-request-events.jsonl` 的 13 个 request、payload bytes 和 input tokens。
- Environment:
  - branch `whalecode-alpha`，commit `47d7ee7`，DeepSeek `deepseek-v4-flash`，R5 Phase E。
- Known facts:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
  - E-007
- Ruled out:
  - 不是 state-machine bookkeeping CPU 开销：13 次模型请求占 R5 wall time 78.9%。
  - 不是 Phase E 新增 current gate feedback 的主要成本：`ActiveProjection => Include` 自 2026-06-19 已存在。
  - 不是单纯 Agent 智能错误：provider history 同时存在旧 running 与新 completed 快照。
- Fix criteria:
  - provider-visible history 在 TaskSpace active 状态下恰好保留最新一份 active projection。
  - 旧 projection 的 decision reason 稳定记录为 `stale_active_projection_replaced`。
  - 当前 gate feedback call/output pair 和 protected user/developer input 不被误删。
  - exact payload scan 记录 `active_projection_count`，大于 1 时 `passed=false`。
  - focused tests、core compile、benchmark selftests 和同一样本重跑通过；最终 payload 无 stale projection，成本显著下降或残差有独立解释。
- Current conclusion: fixed。H-001/H-002/H-003 均已修复：provider history 机械省略旧 projection，scanner/benchmark 强制唯一性；修复后 9 个真实请求均只有一份 projection，样本完成且 input、request、wall time 相对污染样本显著下降。实现未压缩或重解释 projection 语义。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-001
  - H-002
  - H-003
  - E-004
  - E-005
  - E-006
  - E-007
- Close reason:
  - fixed

## Hypothesis H-001: provider history composer 将所有 active projection 当成可并存输入
- Status: fixed
- Parent: P-001
- Claim: composer 虽定位 latest active projection，但没有据此排除旧 projection；统一 `Include` 分支使每轮快照持续累积。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 代码行为直接解释 rollout 中 14 份 active projection 和 payload 单调增长。
- Falsifiable predictions:
  - If true: `ActiveProjection` 分类始终返回 Include，loop 中没有 stale active projection omission。
  - If false: composer 应只 push latest projection，历史 decision 应记录旧 projection omitted。
- Diagnostic evidence plan:
  - Prediction or clause under test: 多份 active projection 输入 composer 后均进入 prepared items。
  - Signal: composition decision 和 prepared payload 中 active projection 数量。
  - Capture method: 静态代码检查并新增双 projection 失败单测。
  - Event name or marker: `ContextProjectionV1 active replacement:`
  - Correlation keys: provider history item index
  - Differentiates from: projection renderer 单份内容过大、tool output 过大。
  - Supports if: 两份 active projection 均为 Include。
  - Refutes if: 旧 projection 已被 omit。
  - Instrumentation status: permanent
  - Instrumentation lifecycle: 保留 uniqueness test 和 omission reason。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: fixed by E-005 and validated by E-006/E-007
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed

## Hypothesis H-002: stale projection 重发是 R5-E token 和主要时间放大的核心驱动
- Status: fixed
- Parent: P-001
- Claim: 旧 projection 被每轮重发，并与自然工具历史重复，使 payload/input 单调增长；额外 provider 处理时间构成主要 wall-time 成本。
- Layer: interaction
- Factor relation: all_of
- Depends on:
  - H-001
- Rationale:
  - payload bytes 和 input tokens 同步约 4 倍增长，模型请求占 wall time 78.9%。
- Falsifiable predictions:
  - If true: projection 数量随轮次增长，末轮 payload 和 input 明显高于首轮，修复后同样本应显著降低末轮 input 或增长斜率。
  - If false: 删除旧 projection 后 token 和 wall time基本不变，主要成本来自其他固定前缀或工具执行。
- Diagnostic evidence plan:
  - Prediction or clause under test: stale projection 累积与 request payload/input 增长同向。
  - Signal: active projection count、payload bytes、input/cached/uncached tokens、request latency。
  - Capture method: 现有 artifact 对照修复后同样本。
  - Event name or marker: `taskspace-provider-request-budget-event-v1`
  - Correlation keys: request_id、logical request sequence
  - Differentiates from: pip install 耗时、Agent 额外工具动作、provider queue。
  - Supports if: 修复后 stale count 为 0，末轮 payload/input 和总 token 显著下降。
  - Refutes if: 指标无实质变化。
  - Instrumentation status: permanent
  - Instrumentation lifecycle: benchmark report 保留 projection count 和 uniqueness violation。
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: fixed and benefit prediction validated by E-007
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed

## Hypothesis H-003: replacement observability 未验证唯一性并产生假阳性
- Status: fixed
- Parent: P-001
- Claim: scanner 的 `replacement_confirmed` 只证明旧 marker/large output 等负面项未命中，没有统计 active projection 数量，因此多份 projection 时仍报告 true。
- Layer: observability
- Factor relation: single
- Depends on:
  - H-001
- Rationale:
  - live artifact 同时存在 14 份 projection 和 `replacement_confirmed=true`。
- Falsifiable predictions:
  - If true: exact scan schema 不含 `active_projection_count` 或 uniqueness failure reason。
  - If false: 多份 projection 应使 scan passed=false。
- Diagnostic evidence plan:
  - Prediction or clause under test: 构造双 projection payload 时 scanner 仍通过。
  - Signal: scan event fields 和 failure reasons。
  - Capture method: scanner focused test。
  - Event name or marker: `taskspace-exact-payload-scan-event-v1`
  - Correlation keys: request_id、provider_payload_sha256
  - Differentiates from: benchmark extractor 丢字段。
  - Supports if: 当前 scanner 无 uniqueness 断言。
  - Refutes if: 已有 active projection count gate。
  - Instrumentation status: permanent
  - Instrumentation lifecycle: schema 和 report 字段长期保留。
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: fixed by scanner v3 and benchmark uniqueness gate
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed

## Evidence E-001: composer 从 2026-06-19 起默认包含所有 active projection
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/session/turn.rs:4040-4171`，git commit `bdca6d7`
- Prediction or plan link: H-001 If true
- Matched signal: `latest_active_projection_item` 只用于 recovery 判断；`ActiveProjection` 统一映射到 Include。
- Correlation keys: `ProviderVisibleItemCategory::ActiveProjection`
- Raw content:
  ```text
  ProviderVisibleItemCategory::ActiveProjection => ProviderVisibleHistoryAction::Include
  ```
- Interpretation: replacement 名称与实际 append 行为不一致。
- Time: 2026-07-10 05:03

## Evidence E-002: R5-E live artifact 显示 projection、payload、token 同步增长
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: reproduction
- Source: `target/r5e-phase-e-final-clean/count-call-stack/20260710-043411-389/pair-001/right/artifacts`
- Prediction or plan link: H-002 diagnostic evidence plan
- Matched signal: 14 份 active projection；payload 32870→131830 bytes；input 8389→33631；总 input 269093；13 次模型请求耗时 37052ms，占 R5 wall time 78.9%；同时 `replacement_confirmed=true`。
- Correlation keys: `pair-001/right`、logical request 1..13
- Raw content:
  ```text
  active_projection_messages=14
  provider_payload_bytes=32870..131830
  input_tokens=8389..33631
  total_input_tokens=269093
  model_request_duration_ms=37052
  ```
- Interpretation: stale projection 累积同时造成成本和状态冲突，现有 replacement 指标是假阳性。
- Time: 2026-07-10 05:03

## Evidence E-003: Phase E 未引入 Include 机制，但移除 hard stop 后暴露完整增长
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: regression-window
- Source: git history 与 R5-C/C1/D/E token summaries
- Prediction or plan link: H-001/H-002 alternatives
- Matched signal: `ActiveProjection => Include` blame 到 `bdca6d7`；R5-C/C1/D 已分别出现 961499、164498、161996 input tokens；R5-D 在 7 requests 被截断，R5-E 继续到 13 requests 后为 269093。
- Correlation keys: `bdca6d7`、`a537254`、`50e786a`
- Raw content:
  ```text
  R5-C  input=961499
  R5-C1 input=164498
  R5-D  input=161996, hard stop after request 7
  R5-E  input=269093, complete after request 13
  ```
- Interpretation: 根因是历史 composer 缺陷；Phase E 让旧问题不再被错误 stop 隐藏。
- Time: 2026-07-10 05:03

## Evidence E-004: 双 projection 回归在修复前稳定失败
- Related hypotheses:
  - H-001
- Direction: supports
- Type: failing-test
- Source: `third_party/codex-cli/codex-rs/core/src/session/turn.rs`
- Prediction or plan link: H-001 diagnostic evidence plan
- Matched signal: 输入 stale running projection、当前 gate call/output、latest completed projection 后，修复前 provider-visible text 中 marker count 为 2，期望为 1。
- Correlation keys: `active_context_replacement_keeps_only_latest_projection_and_current_feedback`
- Raw content:
  ```text
  assertion failed: left=2 right=1
  ```
- Interpretation: 测试直接锁定 composer append 行为，不依赖模型输出或 benchmark 推断。
- Time: 2026-07-10 05:08

## Evidence E-005: latest-only composer 和 uniqueness scanner 已落地
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: fix
- Source: commits `c54b510`、`7242fba`
- Prediction or plan link: P-001 Fix criteria
- Matched signal:
  - composer 只保留最后一个 `ActiveProjection`，旧项记录 `stale_active_projection_replaced`。
  - 当前 tool/gate call-output pair 继续 Include；projection 文本本身不改写。
  - scanner v3 记录 `active_projection_count`，数量不等于 1 时返回 `active_projection_not_unique`。
  - benchmark 按 `scan_event_id` 去重，唯一性违规进入 `metrics_taints`。
- Correlation keys: `ProviderVisibleItemCategory::ActiveProjection`、`taskspace-exact-payload-scan-event-v1`
- Raw content:
  ```text
  active_projection_count == 1
  stale_active_projection_replaced
  active_projection_not_unique
  ```
- Interpretation: 修复限定在快照 identity 和机械计数，不做 projection 内容压缩、摘要或语义再组织。
- Time: 2026-07-10 05:14

## Evidence E-006: focused、构建和 benchmark 回归通过
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: fix-validation
- Source: 本地测试输出
- Prediction or plan link: P-001 Fix criteria
- Matched signal:
  - latest-only/current-feedback/scanner/runtime-trace focused tests 全部通过。
  - `cargo test -p codex-core --lib --no-run` 和 `cargo build -p codex-cli --bin whale` 通过。
  - cost、metrics、E3 validity、harness 四组 benchmark selftest 通过。
  - full core 为 `2161 passed / 224 failed / 3 ignored`，失败数与修复前基线一致，新增 E4 测试通过。
- Correlation keys: `R5-E4-focused-20260710`
- Raw content:
  ```text
  focused E4 tests: PASS
  benchmark selftests: 4/4 PASS
  codex-core full: 2161 passed, 224 failed, 3 ignored
  whale build: PASS
  ```
- Interpretation: E4 没有扩大已知旧策略测试失败集合；224 个历史失败仍由 R5-F 清理，不通过兼容分支恢复。
- Time: 2026-07-10 05:19

## Evidence E-007: 同一样本验证 projection 唯一且成本停止二次累积
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: reproduction-after-fix
- Source: `target/r5e4-projection-latest-only/count-call-stack/20260710-051931-572/pair-001`
- Prediction or plan link: H-002 falsifiable prediction、P-001 Fix criteria
- Matched signal:
  - standard/R5 均 solved、Agent complete、external validation passed、engineering clean。
  - R5 9 个 distinct provider request 的 `active_projection_count` 均为 1，uniqueness violation 为 0，replacement confirmed。
  - R5 input 100365，request 9，wall 23649ms；污染样本分别为 269093、13、46971ms。
  - 单请求 input 8021→12297，后段约 12K；污染样本为 8389→33631。
- Correlation keys: `pair-001/right`、logical request 1..9
- Raw content:
  ```text
  active_projection_count=1 for 9/9 requests
  active_projection_uniqueness_violation_count=0
  input_tokens=100365 (-62.7%)
  model_request_count=9 (-30.8%)
  wall_time_ms=23649 (-49.7%)
  ```
- Interpretation: H-001/H-003 直接关闭；H-002 的成本预测成立。剩余 1.40x wall 和 1.56x input 相对 standard 是后续缓存、自然历史和 cadence 问题，不再由 stale projection 解释。
- Time: 2026-07-10 05:21
