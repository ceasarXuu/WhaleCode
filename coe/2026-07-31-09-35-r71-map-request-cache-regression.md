# Problem P-001: R7.1 map-request 缓存命中率回归
- Status: open
- Created: 2026-07-31 09:35
- Updated: 2026-07-31 09:35
- Objective: 用最多三次用户授权的 Whale Agent sample，证明 map-request request 2+ 缓存命中率从约 92% 降至约 41% 的根因。
- Symptoms:
  - 当前 `subscription-billing-repair` map-request 单次运行共 18 个 provider request，request 2+ 缓存命中率仅 41.00%。
  - 14 个非高命中请求的加权命中率仅 25.70%，第 11、15、18 次请求间歇性恢复到 93% 至 96%。
- Expected behavior:
  - map-request 不自动注入完整 projection；自然历史和稳定合同应尽可能按 provider 可缓存前缀组织。
  - 同一 session 的后续请求不应长期只命中固定基础前缀。
- Actual behavior:
  - 每轮 provider wire 都移除上一轮 Map Handle，再把当前 Handle 放到 request tail，严格消息前缀保持率为 0%。
  - 当前 DeepSeek 缓存多数请求只命中约 6K 至 8K token，少数请求间歇性命中完整公共前缀。
- Impact:
  - 未缓存 input 成本显著增加；当前单次运行 489,490 input token 中 281,362 未命中缓存。
- Reproduction:
  - Docker 中运行 `subscription-billing-repair`、TaskSpace-only、`map-request`、repeat=1。
- Environment:
  - branch `whalecode-alpha`；当前调查起点 `ddd94faa190597b33afcc7b3e44c966eae865769`；provider `deepseek`；model `deepseek-v4-flash`。
- Known facts:
  - 当前 18 轮 base instructions、Tool schema 和 `tool_choice` 各自保持稳定；automatic projection 为 0。
  - request-tail Map Handle 由 `9a0c37cd8f` 引入，但 2026-07-29 `abe2b872b` 的同形态 map-request 复杂样本仍有 91.9% 至 92.5% request 2+ 命中率。
  - 缓存回归首次在现存证据中出现在 `445499582` 后；该提交新增 `TaskSpaceResponseFinalReceiptV1` model-visible developer receipt。
  - DeepSeek 官方缓存为 best-effort；命中依赖已落盘的完整缓存前缀单元，落盘点包括请求边界、公共前缀检测和固定 token 间隔。
- Ruled out:
  - “request-tail Map Handle 单独导致 7 月 29 日缓存回归”已被历史高命中证据否定。
  - 当前单次运行内 Tool schema 切换或 `tool_choice` 切换不是原因。
- Fix criteria:
  - 至少一个根因假设通过相同 provider 条件下的历史边界对照和逐请求 wire/cache 信号证实。
  - 修复前必须能解释回归前高命中、回归后间歇性低命中，以及 map-append 严格前缀下也曾出现间歇失效。
- Current conclusion: 回归窗口已收敛到 `abe2b872b..445499582` 与同期 provider 缓存行为；尚不能区分 Final Receipt 上下文载体、供应商缓存时序或二者交互。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: Final Receipt 上下文载体触发缓存回归
- Status: unverified
- Parent: P-001
- Claim: `445499582` 新增的 model-visible developer Final Receipt 改变 provider 输入和缓存单元边界，是 map-request 缓存回归的必要原因。
- Layer: root-cause
- Factor relation: unknown
- Depends on:
  - none
- Rationale:
  - 现存证据的回归拐点与 Final Receipt 首次进入 provider history 的提交一致。
- Falsifiable predictions:
  - If true: 在当前 provider 条件下，`abe2b872b` 同样本运行应恢复接近历史高命中，而 `445499582` 和 current 应稳定显著更低；低命中应从首个 Final Receipt 进入历史后开始。
  - If false: `abe2b872b` 也出现相同低命中，或者 `445499582` 在 Final Receipt 已出现后仍保持与 pre-receipt 相当的逐请求命中。
- Diagnostic evidence plan:
  - Prediction or clause under test: pre-receipt 与 post-receipt 在相同 provider、模型、Docker 和复杂样本下产生可重复的缓存分界。
  - Signal: request 2+ hit rate、逐请求 cached/uncached token、receipt 首次出现请求、LCP、first diff、section cost。
  - Capture method: 分别用 `abe2b872b`、`445499582` 历史二进制执行一次相同复杂 sample。
  - Event name or marker:
    - `TaskSpaceResponseFinalReceiptV1`
    - `provider.chat_wire_prefix_broken`
  - Correlation keys:
    - subject commit
    - model_request_index
    - provider request id
  - Differentiates from:
    - H-002
  - Supports if:
    - pre-receipt 明确恢复高命中，post-receipt 明确复现低命中，且下降开始于 receipt 进入下一请求。
  - Refutes if:
    - pre/post 两臂在当前 provider 下表现相同，或差异不能和 receipt 边界对应。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 复用现有 provider wire、cache 和 section trace。
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-002
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 构建并运行 pre/post 历史边界对照。
- Blocker:
  - none

## Hypothesis H-002: DeepSeek 缓存落盘行为变化是主要原因
- Status: unverified
- Parent: P-001
- Claim: 仓内 wire 风险长期存在，但 DeepSeek 当前缓存单元落盘或 best-effort 命中行为变化，使此前可高命中的公共前缀现在只能间歇命中。
- Layer: environment
- Factor relation: unknown
- Depends on:
  - none
- Rationale:
  - 同样严格前缀破坏形态在 7 月 29 日前可达到约 92%；回归后 map-append 在消息前缀保持 100% 时也出现过间歇性低命中。
- Falsifiable predictions:
  - If true: 在当前 provider 条件下，`abe2b872b` 也会显著低于其历史约 92% 结果；三个版本都会呈现类似的低低高缓存锯齿或落盘延迟。
  - If false: pre-receipt 历史版本稳定恢复高命中，而 post-receipt/current 才下降。
- Diagnostic evidence plan:
  - Prediction or clause under test: 当前 provider 对历史 pre-receipt wire 也不再复现历史缓存表现。
  - Signal: 历史版本 fresh run 与同版本 retained artifact 的逐请求 cache 差异。
  - Capture method: 将 `abe2b872b` fresh run 与其 2026-07-29 retained trace 对照。
  - Event name or marker:
    - `prompt_cache_hit_tokens`
  - Correlation keys:
    - subject commit
    - model_request_index
  - Differentiates from:
    - H-001
  - Supports if:
    - pre-receipt fresh run 同样低命中，且 wire 结构与 retained 高命中运行相同。
  - Refutes if:
    - pre-receipt fresh run 恢复历史高命中，并与 post-receipt 形成清晰边界。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 provider 返回的原始 cache usage 与 wire identity。
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-003
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 运行历史 pre-receipt 对照。
- Blocker:
  - none

## Hypothesis H-003: 当前额外反馈与拒绝请求只放大而不触发回归
- Status: unverified
- Parent: P-001
- Claim: 当前更大的 control feedback、协议拒绝和状态拒绝增加未缓存 token 与请求数，但不是缓存率从约 92% 降至约 41% 的首要触发因素。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - H-001
  - H-002
- Rationale:
  - 当前运行有 7 个协议或状态拒绝相关请求，TaskSpace control feedback 平均约 4,321 estimated token/request。
- Falsifiable predictions:
  - If true: 排除拒绝请求后，正常请求仍呈现相同低命中；不同版本的下降应先由 wire/receipt 或 provider 行为解释。
  - If false: 低命中仅集中于拒绝恢复链，生产性请求维持高命中。
- Diagnostic evidence plan:
  - Prediction or clause under test: request outcome 分类不能单独解释缓存锯齿。
  - Signal: 每请求 outcome、receipt 状态、cache hit、input 增量和 LCP。
  - Capture method: 对三次 fresh run 做逐请求 join，不新增 Agent 运行。
  - Event name or marker:
    - `taskspace_response_final_receipt_emitted`
    - `provider.chat_wire_prefix_broken`
  - Correlation keys:
    - provider request id
    - control call id
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - 正常请求与拒绝恢复请求都存在低命中。
  - Refutes if:
    - 去除拒绝恢复请求后缓存恢复到接近历史基线。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 复用现有 request reason、phase、receipt 与 cache trace。
- Evidence gate: pending
- Related evidence:
  - E-004
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 在三次运行完成后离线关联请求级证据。
- Blocker:
  - none

## Evidence E-001: request-tail Handle 早于缓存回归
- Related hypotheses:
  - H-001
  - H-002
- Direction: refutes
- Type: code-location
- Source: `git blame`、`provider-cache-trace.jsonl` retained artifacts
- Prediction or plan link:
  - 否定“Handle 尾部移动单独解释回归”。
- Matched signal:
  - `9a0c37cd8f` 引入 tail refresh；`abe2b872b` 同样 prefix preserved 0%，复杂样本 request 2+ hit 91.9% 至 92.5%。
- Correlation keys:
  - commit `9a0c37cd8f`
  - commit `abe2b872b`
- Raw content:
  ```text
  9a0c37cd8f 2026-07-21 fix(taskspace): refresh map-request handle at request tail
  abe2b872b complex map-request retained runs: 0.924513, 0.922125, 0.918746
  ```
- Interpretation: Handle 尾部移动是缓存脆弱性的必要背景，但不是 7 月 29 日性能回归的充分原因。
- Time: 2026-07-31 09:35

## Evidence E-002: 回归首次出现在 Final Receipt 提交边界
- Related hypotheses:
  - H-001
- Direction: supports
- Type: observation
- Source: retained benchmark artifacts and `git show 445499582`
- Prediction or plan link:
  - H-001 回归窗口预测。
- Matched signal:
  - `445499582` 首次新增 model-visible developer Final Receipt；其后 map-request complex retained runs 降至 46.5% 至 59.8%。
- Correlation keys:
  - commit `445499582`
- Raw content:
  ```text
  outputs.push(ResponseInputItem::Message {
      role: "developer".to_string(),
      content: TaskSpaceResponseFinalReceiptV1
  })
  post-commit complex map-request request 2+ hit: 0.465562, 0.597505, 0.469752
  ```
- Interpretation: 时间相关性强，但尚未证明 receipt 是必要原因；需要在当前 provider 下重跑历史边界。
- Time: 2026-07-31 09:35

## Evidence E-003: 当前缓存呈现间歇性前缀单元命中
- Related hypotheses:
  - H-002
- Direction: supports
- Type: diagnostic-log
- Source: `target/r71-01-closure-live/.../provider-cache-trace.jsonl`
- Prediction or plan link:
  - H-002 缓存单元落盘预测。
- Matched signal:
  - 普通低命中请求仅缓存 6,016 至 7,936 token；第 11、15、18 请求突然缓存 30,080、32,768、36,864 token。
- Correlation keys:
  - model request 11
  - model request 15
  - model request 18
- Raw content:
  ```text
  req11 31194 input / 30080 cached
  req15 35088 input / 32768 cached
  req18 38102 input / 36864 cached
  ```
- Interpretation: provider 并非完全无法复用长公共前缀，而是只在部分落盘单元可用时命中。
- Time: 2026-07-31 09:35

## Evidence E-004: 当前拒绝恢复链放大成本
- Related hypotheses:
  - H-003
- Direction: neutral
- Type: observation
- Source: 当前 performance observation 与 request trace
- Prediction or plan link:
  - H-003 请求分类预测。
- Matched signal:
  - 18 次请求中有 7 次涉及协议或状态拒绝后的恢复；尚未完成逐请求 outcome/cache join。
- Correlation keys:
  - current live run
- Raw content:
  ```text
  protocol-rejected request indexes: 1,2,5,8,13
  state-rejected request indexes: 9,16
  ```
- Interpretation: 已证明存在成本放大器，尚未证明它决定缓存率。
- Time: 2026-07-31 09:35
