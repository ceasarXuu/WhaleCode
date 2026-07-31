# Problem P-001: R7.1 map-request 缓存命中率回归
- Status: open
- Created: 2026-07-31 09:35
- Updated: 2026-07-31 10:35
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
  - 同一 provider 时间窗口内，pre-receipt、post-receipt、current 三臂均业务验证通过，request 2+ 命中率依次为 86.51%、57.62%、33.61%。
  - post-receipt 臂每次新增 developer Final Receipt 后，下一请求命中率均落在 16.4% 至 34.6%；未新增 receipt 的稳定段恢复到 67.6% 至 94.9%。
  - current 臂几乎每轮新增 Final Receipt；DeepSeek wire 将对话中段的 developer receipt 表达为 system message，后续请求持续只命中约 6K token。
- Ruled out:
  - “request-tail Map Handle 单独导致 7 月 29 日缓存回归”已被历史高命中证据否定。
  - 当前单次运行内 Tool schema 切换或 `tool_choice` 切换不是原因。
- Fix criteria:
  - 至少一个根因假设通过相同 provider 条件下的历史边界对照和逐请求 wire/cache 信号证实。
  - 修复前必须能解释回归前高命中、回归后间歇性低命中，以及 map-append 严格前缀下也曾出现间歇失效。
- Current conclusion: 完整根因是反馈层的 out-of-band dynamic factual carrier。TaskSpace 已通过原生 Tool result
  返回的动态事实，又被 `factual_message` 或独立 Final Receipt 复制成 developer 消息；DeepSeek wire
  将它们表达为对话中段 system，从该边界开始只能命中较短缓存前缀。Final Receipt、sequence preflight
  failure 和 state rejection 不是三个独立缓存根因，而是同一错误 carrier 抽象的三个实例。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - 根因证据门已满足；生产修复和修复后复验尚未执行，因此 P-001 保持 open。
- Close reason:
  - not closed

## Hypothesis H-001: Final Receipt 上下文载体触发缓存回归
- Status: confirmed
- Parent: P-001
- Claim: `445499582` 新增的 model-visible developer Final Receipt 改变 provider 输入和缓存单元边界，是 map-request 缓存回归的独立因果因素。
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
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-005
  - E-006
  - E-007
  - E-008
  - E-010
  - E-011
- Conclusion: confirmed as a causal factor, not as the sole cause。同版本、同二进制 A/B 中，仅停止独立 receipt carrier 后，wire receipt count 从最终累计 6 降为 0，request 2+ cache 提升 26.91 个百分点；剩余低点由 failure developer/system carrier 解释。
- Repair design readiness: ready after R71-10 establishes the unique native control-result revision carrier
- Next step: 先完成 R71-10，再由 R71-11 删除独立 receipt；并通过 R71-01/R71-09 去除失败结果的额外 developer/system 副本。
- Blocker:
  - none

## Hypothesis H-002: DeepSeek 缓存落盘行为变化是主要原因
- Status: refuted
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
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
  - E-005
  - E-006
- Conclusion: refuted as primary cause。pre-receipt fresh run 达到 86.51%，明显恢复且接近历史 91.9% 至 92.5%；provider best-effort 行为只构成剩余方差。
- Repair design readiness: not applicable
- Next step: 保留 provider cache usage 观测，不以供应商波动替代仓内 carrier 修复。
- Blocker:
  - none

## Hypothesis H-003: 当前额外反馈与拒绝请求只放大而不触发回归
- Status: refuted
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
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-006
  - E-007
- Conclusion: refuted in its broad form。拒绝动作会放大请求数，但其结果被额外复制成 developer/system
  carrier 时，也会直接触发缓存下降；原生 Tool result 本身不是该缓存触发器。
- Repair design readiness: superseded by H-004
- Next step: 将失败反馈与 Final Receipt 统一按 H-004 的 carrier 边界审视，但保留 R71-01、R71-09、
  R71-11 的原子实施边界。
- Blocker:
  - none

## Hypothesis H-004: out-of-band dynamic factual carrier 是完整缓存根因
- Status: confirmed
- Parent: P-001
- Claim: 任何已经存在于原生 Tool result 的 TaskSpace 动态事实，只要再以独立 developer message
  插入自然历史，就会在 DeepSeek wire 中形成中途 system 边界并破坏后续缓存；动态事实的具体 schema
  不是决定因素。
- Layer: root-cause
- Factor relation: unifies
- Depends on:
  - H-001
  - H-003
- Rationale:
  - pre-receipt 90%+ 版本与当前版本的固定 Base、Protocol 和 Tools 大小接近，结构性差异集中在动态
    developer/system carrier。
- Falsifiable predictions:
  - If true: 没有动态 developer/system 插入的请求段应恢复 90%+；每次插入后下一请求应立即下降；
    system 内容应与已存在的 Tool output 内容 hash 相同。
  - If false: 固定 system/tool schema 差异足以解释下降，或没有新增 developer/system 的当前请求仍
    持续低命中。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较 pre-receipt 90%+ 与当前 receipt-ablation 的逐请求 role
    sequence、固定 section 大小、content hash 和 cache hit。
  - Signal: system count、developer event、Tool/system content SHA-256、request 2+ hit。
  - Capture method: 复用 retained provider wire 和 rollout，不新增 Agent run。
  - Event name or marker:
    - `TaskSpaceResponseFinalReceiptV1`
    - `ToolSequencePreflightResultV3`
    - `TaskSpaceResponseCommitFailureV3`
  - Correlation keys:
    - model request index
    - message index
    - content SHA-256
  - Differentiates from:
    - H-002
    - 固定 L1/L2/Tool schema 成本
  - Supports if:
    - 历史臂 system 始终为 2；当前无新增 carrier 的请求恢复 90%+；新增 system 与 Tool output
      content hash 精确相同。
  - Refutes if:
    - 当前无 carrier 请求仍低于历史臂，或新增 system 不是 Tool 事实副本。
  - Instrumentation status: existing
  - Instrumentation lifecycle:
    - 永久保留 provider wire role、message hash 和 section trace。
- Evidence gate: satisfied
- Related evidence:
  - E-006
  - E-007
  - E-010
  - E-011
  - E-012
- Conclusion: confirmed。固定 prompt 增量不足 1.3%，无法解释缓存下降；动态 out-of-band carrier 的
  插入与恢复逐请求一一对应，并且 failure system 内容 hash 与先前 Tool outputs 精确相同。
- Repair design readiness: ready for user-confirmed design
- Next step: 设计统一 carrier 修复，确保事实只保留在原生 Tool result；R71-10 先保证 canonical final
  revision 不丢失，再按 R71-01/R71-09/R71-11 分块实施。
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

## Evidence E-005: pre-receipt fresh run 在当前 provider 下恢复高命中
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports H-001; refutes H-002
- Type: controlled-experiment
- Source: `target/r71-cache-root-cause/runs/pre/subscription-billing-repair/20260731-094545-239`
- Prediction or plan link:
  - H-001/H-002 的当前 provider 历史边界对照。
- Matched signal:
  - commit `abe2b872b`；11 requests；request 2+ cached 157,440、uncached 24,542、hit 86.5141%；业务与隐藏 oracle 通过。
- Correlation keys:
  - binary SHA-256 `7cdee712b5e163eb7078b132b427436e65bf5a5982eee8d18f0d52faef28db16`
  - run `20260731-094545-239`
- Raw content:
  ```text
  req2+ hit: 0.865141
  request count: 11
  prefix_preserved_rate: 0
  business_success: true
  ```
- Interpretation: request-tail Handle 和严格消息前缀破坏长期存在，但不足以造成当前持续低命中；provider 行为变化不是主要回归原因。
- Time: 2026-07-31 09:46

## Evidence E-006: post-receipt 新增 system carrier 与缓存塌缩逐次对应
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports H-001 and H-003; refutes H-002
- Type: controlled-experiment
- Source: `target/r71-cache-root-cause/runs/post/subscription-billing-repair/20260731-094655-203`
- Prediction or plan link:
  - H-001 的“下降开始于 receipt 进入下一请求”条款。
- Matched signal:
  - commit `445499582`；15 requests；request 2+ hit 57.6226%。
  - system message 数新增的请求为 4、5、6、8、14、15，对应 hit 25.3%、34.6%、33.4%、26.6%、16.4%、16.5%。
  - system message 数不变的稳定段请求 7、9、10、11、12、13，对应 hit 84.0%、67.6%、81.4%、94.9%、88.8%、91.6%。
  - rollout 中确认 6 条原始 role=`developer` 的 `TaskSpaceResponseFinalReceiptV1`。
- Correlation keys:
  - binary SHA-256 `e6035994ced408cb3110baf66512e569abdc12121fb30842bb22460bfcc3b812`
  - run `20260731-094655-203`
- Raw content:
  ```text
  receipt-added successor hit range: 16.4%..34.6% (6/6)
  no-new-receipt stable hit range: 67.6%..94.9% (6/6)
  ```
- Interpretation: 同一运行内的逐事件对应排除了模型任务难度、provider 时间窗口和静态 Tool schema；动态 Final Receipt carrier 是缓存塌缩触发器。
- Time: 2026-07-31 09:49

## Evidence E-007: current 每轮 receipt 使低命中成为持续状态
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: controlled-experiment
- Source: `target/r71-cache-root-cause/runs/current/subscription-billing-repair/20260731-094928-336`
- Prediction or plan link:
  - H-001 的 current 持续低命中预测。
- Matched signal:
  - 9 requests；request 2+ cached 50,304、uncached 99,379、hit 33.6070%；业务与隐藏 oracle 通过。
  - 请求 2 至 9 每轮只命中 6,016 至 6,784 token；Final Receipt 从 1 条增长到 6 条，wire system message 从 3 条增长到 10 条。
  - 唯一 state rejection 出现在低命中已持续之后，不能解释此前下降。
- Correlation keys:
  - binary SHA-256 `734915a520afd5b597ee18962bb151e38504189d958ed104a99f12a56b31110a`
  - run `20260731-094928-336`
- Raw content:
  ```text
  req2..9 cached: 6016, 6016, 6016, 6272, 6272, 6400, 6528, 6784
  req2+ hit: 0.336070
  ```
- Interpretation: 后续 R7.1 请求合并减少了 request 数，但未修复 carrier 造成的每请求缓存损失。
- Time: 2026-07-31 09:50

## Evidence E-008: 代码路径将机械 receipt 写成对话中段 developer/system 消息
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/tools/sequence.rs`、DeepSeek Context Caching 官方文档
- Prediction or plan link:
  - H-001 的载体机制。
- Matched signal:
  - `execute_prepared_taskspace_siblings` 在普通 Tool outputs 后额外 push `ResponseInputItem::Message { role: "developer" }`。
  - provider wire 将这些消息表达为 system role；内容含每轮变化的 call id、revision 和计数。
  - DeepSeek 只命中从第 0 token 开始完整匹配且已落盘的缓存前缀单元；缓存为 best-effort。
- Correlation keys:
  - commit `445499582`
  - schema `TaskSpaceResponseFinalReceiptV1`
- Raw content:
  ```text
  outputs.push(ResponseInputItem::Message {
      role: "developer".to_string(),
      content: receipt.model_visible_result(),
  });
  ```
- Interpretation: receipt 是必要机械事实，却被放进了错误的自然历史角色和独立 carrier；这既违反反馈层“原生 Tool 结果忠实透传”的边界，也使 map-request 的动态尾部前持续出现新 system 边界。
- Time: 2026-07-31 10:08

## Evidence E-009: 共享 Cargo target 可污染历史臂构建指纹
- Related hypotheses:
  - H-001
  - H-002
- Direction: neutral
- Type: environment-observation
- Source: 三臂构建预检
- Prediction or plan link:
  - 确保历史边界二进制身份可信。
- Matched signal:
  - 多 worktree 共用 target 后，current 首次构建错误复用了历史 `codex-protocol` 产物；强制 protocol 重新指纹化后当前 HEAD 构建通过。
  - 三臂运行前均以各自源码 commit、binary SHA-256 和 build attestation 通过健康检查。
- Correlation keys:
  - `whale_binary_attestation`
- Raw content:
  ```text
  initial: TokenCountEvent has no field provider_request_id
  after protocol rebuild: Finished dev profile
  ```
- Interpretation: 这是 benchmark 构建隔离缺口，不影响三次已证明二进制的 API 结果；后续历史对照不得无证明共用 Cargo 产物。
- Time: 2026-07-31 09:45

## Evidence E-010: 同版本 carrier A/B 确认因果贡献
- Related hypotheses:
  - H-001
- Direction: supports
- Type: controlled-experiment
- Source: `target/r71-receipt-ablation`
- Prediction or plan link:
  - H-001 的同版本单变量消融证据门。
- Matched signal:
  - 两臂使用同一 binary SHA-256、模型、样本、projection policy 和 runner 参数。
  - control：9 requests；request 2+ hit 33.6031%；cached 63,872；uncached 100,575；最终累计 6 条 receipt。
  - ablation：10 requests；request 2+ hit 60.5156%；cached 130,688；uncached 76,915；receipt count 0。
  - 两臂 public validation 与 hidden oracle 均通过。
- Correlation keys:
  - binary SHA-256 `1e485f3c2a7abb776bae830d157985f79306a6ffa28d223694776012ca2d0383`
  - control run `20260731-101503-705`
  - ablation run `20260731-101630-243`
- Raw content:
  ```text
  request 2+ cache: 33.6031% -> 60.5156% (+26.9125 pp)
  uncached input: 100575 -> 76915 (-23.52%)
  final receipt count: 6 -> 0
  business_success: true / true
  ```
- Interpretation: 独立 Final Receipt carrier 对缓存下降具有因果贡献；结果不支持把全部剩余低命中只归因于该 carrier。
- Time: 2026-07-31 10:18

## Evidence E-011: 消融后的剩余 system carrier 来自失败结果副本
- Related hypotheses:
  - H-001
  - H-003
- Direction: qualifies H-001; supports H-003
- Type: diagnostic-log
- Source: ablation rollout 与 provider wire trace
- Prediction or plan link:
  - 解释为何 receipt count 为 0 后仍未恢复 pre-receipt 86.51%。
- Matched signal:
  - 消融臂 system message 从基础 2 条增加到 6 条。
  - 4 条新增 developer/system message 分别复制 1 个 `ToolSequencePreflightResultV3` 和 3 个
    `TaskSpaceResponseCommitFailureV3`。
  - 新增 failure carrier 后的请求命中率为 38.76%、35.65%、28.91%、23.50%；无新增 carrier 的后段
    请求恢复到 95.32% 和 92.63%。
- Correlation keys:
  - rollout task event 16、25、42、57
  - provider request index 3、4、6、8
- Raw content:
  ```text
  response_final_receipt_count: 0
  sequence_preflight_rejected_call_count: 2
  control_failure_count: 3
  final system message count: 6
  ```
- Interpretation: 失败结果已作为原生 Tool output 出现，又被复制成独立 developer/system message；这是
  R71-01/R71-09 已登记问题的缓存成本，不应通过 projection 或 provider 波动解释。
- Time: 2026-07-31 10:22

## Evidence E-012: 90%+ 与当前 prompt 结构对比确认共同 carrier 根因
- Related hypotheses:
  - H-003
  - H-004
- Direction: refutes H-003; supports H-004
- Type: retained-trace-comparison
- Source: pre-receipt 与 current carrier-ablation provider wire/rollout
- Prediction or plan link:
  - H-004 的 role sequence、固定 section 和 content hash 预测。
- Matched signal:
  - 两臂 Base Instructions 完全相同：version `2.0.1`、4,792 estimated tokens、相同 SHA-256。
  - Core Protocol 从 818 增到 904 estimated tokens，Tools 从 6,706 增到 6,792，增量均约 86
    tokens，不足 1.3%。
  - pre-receipt 的 11 次请求始终只有 2 条 system；request 3～11 命中率为 83.25%～95.14%。
  - current ablation 在没有新增 system 的 request 2、9、10 分别命中 91.57%、95.32%、92.63%。
  - current 的 request 3、4、6、8 分别新增第 1～4 条动态 system，命中率立即降到
    38.76%、35.65%、28.91%、23.50%。
  - 每条新增 system 的 content SHA-256 与同一事实此前所有受影响 Tool outputs 完全相同。
- Correlation keys:
  - pre run `20260731-094545-239`
  - ablation run `20260731-101630-243`
  - commit `787070d887`
- Raw content:
  ```text
  pre roles: system,system,(user/assistant/tool natural append only)
  current failure roles: ...,tool[,tool],system,user
  current stable cache: req2 91.57%, req9 95.32%, req10 92.63%
  current carrier cache: req3 38.76%, req4 35.65%, req6 28.91%, req8 23.50%
  ```
- Interpretation: prompt 的固定内容变化不是根因。回归由动态 Tool 事实被重复提升为 developer/system
  carrier 引起；`787070d887` 将该模式扩展到 provider-response failure，`445499582` 将同一模式用于
  Final Receipt。
- Time: 2026-07-31 10:35
