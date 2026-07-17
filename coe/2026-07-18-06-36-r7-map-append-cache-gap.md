# Problem P-001: R7 map-append 缓存命中显著低于自然聊天
- Status: open
- Created: 2026-07-18 06:36
- Updated: 2026-07-18 06:39
- Objective: 解释并证明 R7 `map-append` 在消息前缀保持时仍只有 46.51%/69.36% 缓存命中的直接原因。
- Symptoms:
  - Simple 的 request 2+ cache hit 为 46.51%，Complex 为 69.36%，显著低于同轮 Standard 的 96.10%/95.66%。
  - 本地 observer 同时报告 message prefix preserved 为 81.82%/92.59%。
- Expected behavior:
  - 除首轮与 tool choice 形态切换外，真正按 provider token 前缀追加的 `map-append` 应接近自然多轮聊天的缓存表现。
- Actual behavior:
  - 同一 auto tool shape 下，每次新增 revision snapshot 都出现 0% 至 11.5% 的低命中；同 revision 的后续请求恢复到 94.6% 至 99.2%。
- Impact:
  - R7 Phase C 成本结论可能错误归类为 `map-append` 固有特征，影响三策略产品判断。
- Reproduction:
  - 检查 `target/r7-phase-c/current/*/20260718-052006-254/provider-cache-trace.jsonl` 的逐请求 cache、projection bytes 与 prefix 字段。
- Environment:
  - Linux，commit `54fffb17a`；有效 binary source commit `e753ea864`；DeepSeek official ChatCompletions，`deepseek-v4-flash`，Docker hard boundary。
- Known facts:
  - 低命中与新增 revision snapshot 一一对应；same revision 请求命中恢复。
  - R7 snapshot 在 canonical history 中是尾部 `developer` message，DeepSeek ChatCompletions adapter 将 `developer` 转为 `system`。
  - DeepSeek 当前缓存以完整 cache prefix unit 匹配，缓存构建需要数秒且属于 best effort。
  - 等待 5 秒的官方 API 受控探针中，普通 user 追加首次扩展命中 99.22%，interleaved system 追加首次扩展命中 0%，相同 system 扩展重放恢复到 99.17%。
- Ruled out:
  - 不是同 revision snapshot 重复追加：exact scanner 的 duplicate 与 order violation 均为零。
- Fix criteria:
  - 通过最终 provider wire 或受控 API probe 证明首个差异机制；修复后新增 snapshot 的下一请求命中应接近同条件自然追加，且 projection 语义与 revision 门禁不回退。
- Current conclusion: 根因已确认。R7 只在内部 ResponseItem 层实现追加；snapshot 的 `developer` role 在 DeepSeek ChatCompletions wire 上转换为 interleaved `system`，首次新 system 扩展不能复用自然会话 cache prefix unit。当前 prefix observer 证明的是 JSON message 追加，不是 DeepSeek cache-unit 等价。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: 追加 developer snapshot 在 DeepSeek wire 中成为 system 消息并破坏缓存前缀单元
- Status: confirmed
- Parent: P-001
- Claim: Whale 将尾部 `developer` snapshot 转为 `system` 后，DeepSeek 对 system 消息的 prompt 预处理使新增 snapshot 不能像普通 user/tool 历史一样复用既有 cache prefix unit。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 低命中只在 active projection bytes 增长时出现；adapter 明确执行 `developer -> system`。
- Falsifiable predictions:
  - If true: 相同长前缀和等待时间下，普通消息扩展首次请求高命中，而追加 system snapshot 的首次请求显著低命中；后者完全重复后可重新命中。
  - If false: 普通消息与 system snapshot 扩展首次请求命中相近。
- Diagnostic evidence plan:
  - Prediction or clause under test: 角色是区分 cache hit 的唯一实验变量。
  - Signal: DeepSeek usage 中 `prompt_cache_hit_tokens`、`prompt_cache_miss_tokens` 与 hit rate。
  - Capture method: 运行 `probe-deepseek-appended-system-cache.ps1` 的 natural/system 两臂，base 后等待 5 秒，再发送 extension 和 identical replay。
  - Event name or marker:
    - `deepseek.appended_system_cache_probe`
  - Correlation keys:
    - probe id
    - arm
    - request position
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - natural extension 首次命中高，system extension 首次命中低，system identical replay 恢复。
  - Refutes if:
    - 两臂 extension 首次命中无显著差异。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 保留为 provider contract probe
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-002
  - E-004
- Conclusion: confirmed；角色是区分首次扩展 cache hit 的实验变量。
- Repair design readiness: ready
- Next step: 经用户确认后设计不产生 interleaved system 的 append carrier。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: Cache prefix unit 尚未落盘导致紧邻请求低命中
- Status: refuted
- Parent: P-001
- Claim: revision commit 后下一次 provider request 距上一响应只有约 50 至 180 ms，早于 DeepSeek 数秒级缓存构建，因而不能命中最近的完整 prefix unit。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 官方文档明确缓存构建需要数秒；coding loop 的连续请求间隔远短于该时间。
- Falsifiable predictions:
  - If true: natural 与 system 两臂在无等待时都可能低命中；等待 5 秒后两者均恢复。
  - If false: 等待相同时间后，system 扩展仍显著低于 natural 扩展。
- Diagnostic evidence plan:
  - Prediction or clause under test: 充分等待可消除角色间差异。
  - Signal: base completion 到 extension start 的等待时间及 provider cache hit rate。
  - Capture method: 探针固定等待 5 秒，并记录 delay；必要时补无等待对照。
  - Event name or marker:
    - `deepseek.appended_system_cache_probe`
  - Correlation keys:
    - probe id
    - delay_ms
  - Differentiates from:
    - H-001
  - Supports if:
    - 两臂等待后首次 extension 均高命中。
  - Refutes if:
    - natural 高而 system 低。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 保留为 provider contract probe
- Evidence gate: pending
- Related evidence:
  - E-003
  - E-004
- Conclusion: refuted为主根因；固定等待 5 秒后 system extension 仍为 0%，而自然追加为 99.22%。实际快速 loop 的落盘延迟可能影响最佳命中量，但不能解释角色间差异。
- Repair design readiness: not applicable
- Next step: closed
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: 本地 prefix observer 比较的不是最终 ChatCompletions token 序列
- Status: confirmed
- Parent: P-001
- Claim: observer 的 message-level prefix 可为 true，但 DeepSeek adapter 的角色转换或服务端 prompt 模板仍可能使最终 token prefix 不同，因此当前 `message_prefix_preserved` 被过度解释。
- Layer: diagnostic
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - trace 在 cache 为 0 时仍报告 message prefix true，证明该指标本身不能推出 provider cache unit 命中。
- Falsifiable predictions:
  - If true: source history prefix 保持，但最终 role 序列包含新增 interleaved system；cache 事实与 source-level 指标分离。
  - If false: observer 已对最终 provider token 序列做了完整等价比较。
- Diagnostic evidence plan:
  - Prediction or clause under test: prefix 指标的输入层级早于服务端 prompt 编码。
  - Signal: observer 代码位置、ChatCompletions adapter 输出和 `first_diff_path` 定义。
  - Capture method: 静态追踪 `ResponseItem -> chat_messages -> cache trace` 数据流并核对 wire section。
  - Event name or marker:
    - `provider.chat_wire_prefix_preserved`
  - Correlation keys:
    - model_request_index
  - Differentiates from:
    - H-001
  - Supports if:
    - 指标只证明 JSON messages 追加，不证明 DeepSeek 内部 cache prefix unit。
  - Refutes if:
    - 指标能证明 DeepSeek 最终 token prompt 全前缀相同。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 根据诊断结果修正指标命名或增加 role/cache-unit 风险字段
- Evidence gate: pending
- Related evidence:
  - E-001
  - E-002
  - E-004
- Conclusion: confirmed；`message_prefix_preserved` 是本地 message-level 结构事实，不能命名或解释成 DeepSeek cache-unit 等价。
- Repair design readiness: ready
- Next step: 修复时同步收窄指标命名和报告解释。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 低命中只发生在 projection 增长请求
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: `target/r7-phase-c/current/single-file-fast-fix/20260718-052006-254/provider-cache-trace.jsonl`
- Prediction or plan link:
  - H-001：新增 snapshot 是 cache 下降触发器。
- Matched signal:
  - request 5/6/8/11 的 active projection bytes 增长且 hit rate 分别为 0%、4.12%、6.62%、11.50%；request 3/4/7/9/10 projection 不变且为 94.59%-99.16%。
- Correlation keys:
  - model_request_index 3-11
- Raw content:
  ```text
  req3 projection=1335B hit=95.00%
  req4 projection=1335B hit=94.59%
  req5 projection=2881B hit=0.00%
  req6 projection=5602B hit=4.12%
  req7 projection=5602B hit=97.62%
  req8 projection=7363B hit=6.62%
  req9 projection=7363B hit=98.05%
  req10 projection=7363B hit=99.16%
  req11 projection=10085B hit=11.50%
  ```
- Interpretation: provider 波动不能解释与 revision snapshot 增长的逐次对应关系。
- Time: 2026-07-18 06:36

## Evidence E-002: DeepSeek adapter 将每个 developer snapshot 转为 system role
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/codex-api/src/endpoint/chat_completions.rs:38`
- Prediction or plan link:
  - H-001：snapshot 的最终 ChatCompletions role 不是自然聊天中的 user/tool，而是 system。
- Matched signal:
  - `let role = if role == "developer" { "system" } else { role };`
- Correlation keys:
  - projection_kind=revision_snapshot
- Raw content:
  ```text
  ResponseItem::Message { role, content, .. } => {
      pending_assistant.flush_into(&mut messages, require_tool_reasoning_field);
      let role = if role == "developer" { "system" } else { role };
  }
  ```
- Interpretation: source history 的尾部 developer append 在 DeepSeek wire 上表现为会话中间新增 system message。
- Time: 2026-07-18 06:36

## Evidence E-003: DeepSeek 当前按完整 cache prefix unit 命中且构建需要数秒
- Related hypotheses:
  - H-002
- Direction: supports
- Type: external-review
- Source: `https://api-docs.deepseek.com/guides/kv_cache`
- Prediction or plan link:
  - H-002：紧邻请求可能早于 cache unit 落盘。
- Matched signal:
  - 官方文档声明 cache unit 必须完整匹配、请求边界产生 prefix unit、缓存构建需要数秒且为 best effort。
- Correlation keys:
  - none
- Raw content:
  ```text
  A subsequent request can only hit the cache if it fully matches a cache prefix unit.
  Cache construction takes seconds.
  ```
- Interpretation: source-level JSON 前缀相同不是充分条件，探针必须固定等待时间。
- Time: 2026-07-18 06:36

## Evidence E-004: 受控探针证明 interleaved system 首次扩展独立失去缓存
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: experiment
- Source: `target/r7-phase-c-cache-probe/r7-cache-1784327903288/result.json`
- Prediction or plan link:
  - H-001：角色是唯一变量时，system extension 首次低命中、自然 extension 高命中、system replay 恢复。
  - H-002：5 秒等待不能消除 system 与 natural 差异。
- Matched signal:
  - natural first extension 99.2248%；system first extension 0%；system identical replay 99.1707%。
- Correlation keys:
  - probe_id=r7-cache-1784327903288
  - persistence_delay_ms=5000
- Raw content:
  ```text
  natural base: input=8604 hit=0
  natural first extension: input=8643 cached=8576 hit=0.992248
  natural identical replay: input=8643 cached=8576 hit=0.992248
  system base: input=9124 hit=0
  system first extension: input=9164 cached=0 hit=0
  system identical replay: input=9164 cached=9088 hit=0.991707
  ```
- Interpretation: `map-append` 当前使用的 interleaved system carrier 不是 DeepSeek 缓存意义上的自然线性追加；Phase C 的低命中不能归为 append 固有成本。
- Time: 2026-07-18 06:39
