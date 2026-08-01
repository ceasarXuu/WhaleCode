# Problem P-001: R7 map-append 缓存命中显著低于自然聊天
- Status: fixed
- Created: 2026-07-18 06:36
- Updated: 2026-07-18 07:45
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
  - 用户确认产品合同不是“revision commit 后附带 snapshot”，而是“每轮 provider request 构造时，机械地把当时最新完整 projection 持久追加为上下文末项”。
- Ruled out:
  - 不是同 revision snapshot 重复追加：exact scanner 的 duplicate 与 order violation 均为零。
- Fix criteria:
  - 最终 provider wire 证明每轮 request 以最新 projection 收尾；相同 request shape 不再出现由 projection carrier 引发的零命中；projection identity、revision 与反馈门禁不回退。
- Current conclusion: 根因已确认并修复。`map-append` 现在在每轮 provider request 构造时持久追加最新完整 projection，使用自然历史兼容的 `user` carrier，形成 `A+P1 -> A+P1+B+P2`。31/31 个 Docker request 的 projection 均为消息末项且 identity 对齐，same-shape zero hit 为零；缓存从旧实现的 46.51%/69.36% 提升至 78.95%/87.35%。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-001 confirmed
  - E-006 request-tail 回归
  - E-007 31/31 Docker exact scans
  - E-008 cache fix validation
- Close reason:
  - 原始缓存症状和错误触发合同均已在修复后复现路径中消失

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
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-004
  - E-005
  - E-006
  - E-007
  - E-008
- Conclusion: confirmed；角色是区分首次扩展 cache hit 的实验变量。
- Repair design readiness: ready
- Next step: closed
- Blocker:
  - none
- Close reason:
  - request-tail 与 cache carrier 缺陷均已通过定向测试、完整回归和 Docker trace 验证修复

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
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: refuted为主根因；固定等待 5 秒后 system extension 仍为 0%，而自然追加为 99.22%。实际快速 loop 的落盘延迟可能影响最佳命中量，但不能解释角色间差异。
- Repair design readiness: not applicable
- Next step: closed
- Blocker:
  - none
- Close reason:
  - 落盘延迟已被受控 5 秒探针排除为主根因

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
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-004
- Conclusion: confirmed；`message_prefix_preserved` 是本地 message-level 结构事实，不能命名或解释成 DeepSeek cache-unit 等价。
- Repair design readiness: ready
- Next step: closed
- Blocker:
  - none
- Close reason:
  - 报告已明确 message-level prefix 不等于 provider cache-unit 命中

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

## Evidence E-005: 用户确认 map-append 是每轮 request 的持久尾部全景
- Related hypotheses:
  - H-001
- Direction: supports
- Type: user-feedback
- Source: 用户在 2026-07-18 说明目标行为。
- Prediction or plan link:
  - H-001 修复方向必须保持 provider 请求之间的自然前缀追加，不能改成 tool result carrier。
- Matched signal:
  - 每轮 request 最后都携带当时最新 Map projection；构造 context 时机械追加到末尾。
- Correlation keys:
  - R7 Phase C map-append
- Raw content:
  ```text
  一轮request 最后都带上最新的map projection，不是要依赖tool返回，只要构造context的时候机械追加到最后。
  ```
- Interpretation: Phase C 的 `RevisionCommit -> AppendRevision` 触发模型与产品目标不一致；同 revision 可跨 request 重复，末项而非最高唯一 revision 决定当前状态。
- Time: 2026-07-18 06:46

## Evidence E-006: request-tail 合同进入生产与回归测试
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: test-result
- Source: commits `4366b95ec`、`f04b5573d` 及本地回归输出。
- Prediction or plan link:
  - H-001 修复后，emission 只能由 provider request 触发，projection 必须是最终 message。
- Matched signal:
  - projection 定向测试 24/24、codex-tools TaskSpace 测试 4/4、policy contract、cost instrumentation、performance observer 与 benchmark harness 全部通过。
- Correlation keys:
  - production_commit=4366b95ec
  - observer_commit=f04b5573d
- Raw content:
  ```text
  cargo test -p codex-core projection --lib: 24 passed
  cargo test -p codex-tools taskspace --lib: 4 passed
  cargo test -p codex-core --lib: 1891 passed / 25 baseline failures / 3 ignored
  ```
- Interpretation: 新增路径没有引入 projection 或工具合同回归；完整 suite 的 25 项失败与冻结基线一致。
- Time: 2026-07-18 07:33

## Evidence E-007: 每个 Docker provider request 都以最新 projection 收尾
- Related hypotheses:
  - H-001
- Direction: supports
- Type: experiment
- Source: `target/r7-phase-c/request-tail/` 下 simple 与 complex 两组有效运行。
- Prediction or plan link:
  - 用户确认的 `A+P1 -> A+P1+B+P2` 产品合同。
- Matched signal:
  - Simple 11/11、Complex 20/20 个唯一 request scan 全部通过；`projection_is_message_tail=true`、identity confirmed，revision 无回退。
- Correlation keys:
  - simple_run=20260718-072520-561
  - complex_run=20260718-072634-693
- Raw content:
  ```text
  simple revisions: bootstrap, 2,2,3,4,5,6,7,7,8,9
  complex revisions: bootstrap, 2x4,3x2,4,5x5,6x4,7,8,9
  failed scans=0; tail violations=0; identity failures=0
  ```
- Interpretation: Map 未变化时同 revision 仍随下一轮新历史再次追加，证明实现不依赖 revision commit 或 tool result。
- Time: 2026-07-18 07:29

## Evidence E-008: 自然 append 缓存恢复且剩余低点可归因到 request shape
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: 两组有效运行的 `provider-cache-trace.jsonl` 与 summary。
- Prediction or plan link:
  - 修复 interleaved system 后，相同 request shape 的连续追加不应再出现零命中。
- Matched signal:
  - Simple request 2+ 从 46.51% 提升至 78.95%，Complex 从 69.36% 提升至 87.35%；两组 same-shape zero hit 均为 0。
- Correlation keys:
  - tool_choice_transition_count=2 per TaskSpace run
- Raw content:
  ```text
  simple: 114,560 cached / 30,550 uncached after request 1
  complex: 393,216 cached / 56,960 uncached after request 1
  main auto loop request hit rates: 84%-97%
  ```
- Interpretation: 旧 carrier 缺陷已消失。Simple 的一次零命中属于初始化后 `named_function -> auto` 形状切换；两组最终收口也切回 named function。低于 Standard 的剩余差异不能再归因于 projection 中部替换，但旧 projection 累积和额外 request 仍造成总 input 成本。
- Time: 2026-07-18 07:29
