# Problem P-001: R6 最新投影破坏 provider 前缀缓存
- Status: fixed
- Created: 2026-07-15 09:29
- Updated: 2026-07-15 09:50
- Objective: 在每个上下文 epoch 只保留一份全图基线 projection、并用原始 control journal 忠实表达后续变更的前提下，恢复 DeepSeek provider 可复用的稳定请求前缀。
- Symptoms:
  - Phase C 最新简单样本中，Standard 第 2 次后缓存命中率为 95.64%，R6 为 0.32%。
  - R6 的 9 个后续请求全部未保持上一请求的完整 message prefix。
- Expected behavior:
  - R6 每次 provider 请求只能看见一份当前 epoch 基线 projection；自然历史和原始 control journal 语义完整；动态 map 状态不应破坏稳定前缀缓存。
- Actual behavior:
  - provider 视图先删除上一请求末尾的 projection，再在新增自然历史之后追加新版 projection；相邻请求的首个差异稳定发生在旧 projection 的 message index。
- Impact:
  - R6 简单样本 uncached input 为 83865 tokens，Standard 为 1301 tokens，直接放大成本和延迟。
- Reproduction:
  - 比较 `target/r6-phase-c-final/simple/single-file-fast-fix/20260715-092525-210` 中 Standard/R6 的 provider wire trace 和 cache trace。
- Environment:
  - branch `whalecode-alpha`，commit `bd5daa88a`，DeepSeek `deepseek-v4-flash`，ChatCompletions/native tools。
- Known facts:
  - R6 active projection 每个请求恰好一份，projection 语义最新，canonical history 中不持久化 projection。
  - R6 请求 3-10 tool schema 相同，仍全部没有保持 message prefix。
  - 相邻 R6 请求首个 message 差异位于上一请求末尾 projection 的位置；下一请求该位置已变成新 assistant/tool 历史。
- Ruled out:
  - active projection 重复累积。
  - 后续请求持续变更 tool schema。
  - Standard 与 R6 cached token 统计口径不同。
- Fix criteria:
  - 每个 active R6 provider 请求只有一份当前 epoch 基线 projection，canonical history 不在每轮替换或积累投影。
  - 相邻请求 wire trace 能证明稳定前缀得到保留，且 live R6 cache hit 显著恢复。
  - 简单样本业务结果和外部验证通过；focused tests 覆盖同轮 map 变更、projection 唯一性和 canonical history 无投影。
- Current conclusion: latest-tail replacement 是 cache collapse 的直接根因；改为每个 context epoch 一份基线 projection，并原样追加 control call/output 后，简单与复杂样本均恢复严格消息前缀和高缓存命中。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-001 已由同一次 live run 的 wire LCP 和 provider cache usage 形成直接因果证据。
  - H-002 已由 DeepSeek 官方兼容矩阵和当前 ChatCompletions schema 排除。
  - H-003 已由 R5 G1 三次受控实跑、复杂样本和 R6 control call/output 可回放字段共同满足诊断门禁。
  - R6 live simple/branch-join 的 26 个逻辑请求、78 条精确扫描事件全部通过，active projection 恒为 1。
  - simple/branch-join request 2+ cache 分别为 91.28%/92.47%，同 shape zero-hit 均为 0。
- Close reason:
  - epoch 基线加原始 journal 的实现与两个 Docker 样本修复门禁全部通过。

## Hypothesis H-001: 末尾动态投影使下一请求不再包含上一完整前缀
- Status: confirmed
- Parent: P-001
- Claim: 每轮删除旧 projection 并在新增自然历史后追加新版 projection，使上一请求的完整 messages 不再是下一请求前缀，因而 DeepSeek 无法复用主要 cache prefix unit。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 当前 provider 视图的 projection 是 transient 尾项；自然历史会在下一轮追加到该尾项原位置。
- Falsifiable predictions:
  - If true: 相邻请求的 first changed message 等于旧 projection index，projection 之前结构保持一致；cache prefix_preserved 为 false。
  - If false: 首个差异发生在 projection 之前，或删除/替换 projection 后仍保持完整 prefix 并命中缓存。
- Diagnostic evidence plan:
  - Prediction or clause under test: 相邻请求的首个 wire message 差异归属于旧 projection 槽位。
  - Signal: `TaskSpaceProviderWireStructureV1` 的 message role/hash、LCP index、projection index 和 provider usage。
  - Capture method: 解析 Phase C 最新 retained run 的逐请求 wire/cache trace。
  - Event name or marker:
    - TaskSpaceProviderWireStructureV1
    - TaskSpaceProviderCacheTraceV1
  - Correlation keys:
    - run root
    - logical request id
  - Differentiates from:
    - H-002
  - Supports if:
    - projection 之前保持一致，first changed message 为上一请求的 projection，且 cache hit 接近零。
  - Refutes if:
    - 更早字段先变化或 cache prefix 仍被保留。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留脱敏 hash/LCP 观测，不保留正文。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready；用户已明确授权继续 Phase C 修复
- Next step: none
- Blocker:
  - none
- Close reason:
  - root cause repaired and validated by E-006/E-007

## Hypothesis H-002: provider 存在可直接承载动态投影的显式缓存边界
- Status: refuted
- Parent: P-001
- Claim: DeepSeek 当前 ChatCompletions 接口或仓库适配层提供显式 cache-control 能力，可让动态 projection 位于非缓存段而稳定复用其前缀。
- Layer: diagnostic
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 若 provider 有官方显式边界，则无需通过上下文重排推测缓存行为。
- Falsifiable predictions:
  - If true: DeepSeek 官方文档和当前请求 schema 同时支持可控 cache breakpoint，并有 usage 信号可验证。
  - If false: DeepSeek 仅提供自动前缀缓存，当前 ChatCompletions request schema 没有可用 cache-control 字段。
- Diagnostic evidence plan:
  - Prediction or clause under test: 当前 provider 路径存在官方、可调用、可观测的显式缓存控制。
  - Signal: DeepSeek 官方 API 文档、仓库请求类型和最终 wire body。
  - Capture method: 官方文档审计和静态调用链审计。
  - Event name or marker:
    - none
  - Correlation keys:
    - provider wire API
  - Differentiates from:
    - H-001
  - Supports if:
    - 官方字段存在且当前 wire path 可无损传递。
  - Refutes if:
    - 官方仅描述自动 prefix caching，仓库也没有对应字段。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: refuted；DeepSeek 自动缓存不要求也不接受显式边界，Anthropic 兼容层明确忽略 `cache_control`
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - provider capability claim refuted

## Hypothesis H-003: epoch 基线投影加原始 control journal 可同时保持语义与前缀
- Status: confirmed
- Parent: P-001
- Claim: 每个 context epoch 只持久化一份带 revision 的全图基线投影，后续原样保留 Agent 的 `taskspace_control` call/output 和普通工具历史；当前状态可由同一 canonical Map 基线与原始变更日志无损重建，同时保持 append-only wire prefix。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-001
  - H-002
- Rationale:
  - 动态 latest-only 视图与严格前缀不可同时成立；R5 G1 已用 epoch snapshot + natural journal 验证过同一 provider 行为。
- Falsifiable predictions:
  - If true: 无 compaction 时 request N 的最终 wire messages 是 request N+1 的严格前缀；初始化、图变更、节点变更和终结可由原始 control 参数与结果按 revision 重放。
  - If false: control history 缺少恢复 topology/status 所需字段，或后续请求仍替换/重排 epoch snapshot。
- Diagnostic evidence plan:
  - Prediction or clause under test: R6 control journal 是否包含完整初始化图、原子 mutation、节点 transition、revision、commit 结果和当前机械状态。
  - Signal: tool schema/handler 输出字段、R5 G1 retained live metrics、R6 focused append-only test。
  - Capture method: 静态字段审计并复用已有 R5 G1 live 证据；修复后增加 R6 focused test 和 Docker live run。
  - Event name or marker:
    - TaskSpaceMapEpochSnapshotR6V1
    - provider.chat_wire_prefix_preserved
  - Correlation keys:
    - map_id
    - revision
    - request_id
  - Differentiates from:
    - 每轮 latest-only replacement
    - stale projection accumulation
  - Supports if:
    - epoch 只有一份基线；control call/output 原样追加；R6 相邻同 shape 请求保持严格前缀。
  - Refutes if:
    - 需要 Runtime 生成语义 delta/摘要才能恢复当前状态，或 wire 仍在旧 snapshot 位置断裂。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 snapshot 数量、revision、wire LCP 和 cache usage，不记录正文。
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-005
- Conclusion: confirmed
- Repair design readiness: ready；用户已授权 Phase C 继续修复
- Next step: none
- Blocker:
  - none
- Close reason:
  - implemented and validated

## Evidence E-001: 最新三臂简单样本显示 R6 缓存坍缩
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: `target/r6-phase-c-final/simple/single-file-fast-fix/20260715-092525-210`
- Prediction or plan link:
  - H-001 的 cache 行为预测
- Matched signal:
  - Standard 后续 cache hit rate 0.956367；R6 后续 cache hit rate 0.003213。
- Correlation keys:
  - run root `20260715-092525-210`
- Raw content:
  ```text
  standard: requests=5 input=33045 cached=31744 uncached=1301 wall=14.857s
  r6:       requests=10 input=84121 cached=256 uncached=83865 wall=27.516s
  r6 prefix_preserved=0/9, zero_cache_hit=9
  ```
- Interpretation: R6 的成本差异主要来自缓存失效，不是单纯请求数放大。
- Time: 2026-07-15 09:29

## Evidence E-002: 首个 wire 差异稳定落在上一轮 projection 位置
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: 同一 run 的 provider wire structure trace
- Prediction or plan link:
  - H-001 的 first changed message 预测
- Matched signal:
  - request 2 的末尾 message 7 是 system projection；request 3 的 message 7 是新增 assistant，自身 projection 移到 message 11；后续请求重复同一模式。
- Correlation keys:
  - logical requests 2-10
- Raw content:
  ```text
  request 2 -> 3: first_changed_message=messages[7].message
  request 3 -> 4: first_changed_message=messages[11].message
  request 4 -> 5: first_changed_message=messages[14].message
  same-shape prefix preserved: 0/7
  ```
- Interpretation: latest-only 尾项替换破坏了相邻请求的 append-only 关系，直接解释 prefix cache 失效。
- Time: 2026-07-15 09:29

## Evidence E-003: 官方显式 cache-control 能力尚待确认
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: external-review
- Source: DeepSeek 官方 `Context Caching`、`Anthropic API` 文档及当前 ChatCompletions request schema
- Prediction or plan link:
  - H-002 官方/provider schema 审计
- Matched signal:
  - DeepSeek 缓存默认自动启用，命中要求完整复用已持久化 prefix unit；Anthropic 兼容矩阵明确标记 message/tool 的 `cache_control` 为 ignored。
- Correlation keys:
  - none
- Raw content:
  ```text
  DeepSeek wire_api = ChatCompletions
  DeepSeek Anthropic API: cache_control = Ignored
  build_chat_completions_body: no cache_control field
  ```
- Interpretation: 当前 provider 没有可用显式缓存边界，不能用 provider-specific 字段解决 latest-only replacement。
- Time: 2026-07-15 09:42

## Evidence E-004: R5 G1 已验证 epoch snapshot 与自然 journal 的 cache 行为
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `docs/v0.0.5/build-R5/11-r5-feedback-cache-priority-plan.md`
- Prediction or plan link:
  - H-003 append-only 和 cache 预测
- Matched signal:
  - 三次受控 R5 均 solved、strict prefix 100%，request-2+ cache hit 分别 97.01%、98.03%、97.66%；复杂样本 24/24 strict prefix、98.14% cache hit。
- Correlation keys:
  - `target/r5-g1-repeats/count-call-stack/20260710-210444-351`
- Raw content:
  ```text
  epoch start: append one faithful map snapshot
  map change: preserve exact taskspace_control call and tool output
  controlled R5 prefix: 12/12, 20/20, 16/16
  ```
- Interpretation: 同一 DeepSeek/provider substrate 上，该布局已实证恢复缓存且没有依赖 Runtime 语义 delta。
- Time: 2026-07-15 09:42

## Evidence E-005: R6 control history 足以机械重建图变更
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `taskspace_control_args.rs`、`taskspace_control.rs`、`taskspace_control_output.rs`
- Prediction or plan link:
  - H-003 journal 可重放预测
- Matched signal:
  - initialize call 保存完整 root/work/finish/nodes/edges；mutation 保存 add/remove；transition 保存 node/revision；结果保存 state_commit、map identity、revision、机械状态集合。
- Correlation keys:
  - call_id
  - map_id
  - revision
- Raw content:
  ```text
  initialize_map(root, initial_work_node, finish, additional_work_nodes, edges)
  mutate_graph(expected_revision, add_nodes, add_edges, remove_edges)
  transition_node(expected_revision, node_id, transition)
  result(state_commit, map_state.revision, root_node_id, finish_node_id, status sets)
  ```
- Interpretation: R6 不需要 Runtime 生成摘要或推断性 delta；原始 Agent call/output 本身就是忠实状态变更日志。
- Time: 2026-07-15 09:42

## Evidence E-006: simple 样本恢复稳定消息前缀和缓存
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `target/r6-phase-c-epoch/simple/single-file-fast-fix/20260715-094309-889`
- Prediction or plan link:
  - P-001 wire prefix 与 cache fix criteria
- Matched signal:
  - 13 个逻辑请求的 39 条精确扫描全部通过，active projection 恒为 1；全部 12 个后续请求保持 message prefix
- Correlation keys:
  - run `20260715-094309-889`
- Raw content:
  ```text
  request2+ cache: 91.28%
  message prefix: 12/12
  same-shape zero-hit: 0
  projection_count: 1
  exact scan failures: 0
  ```
- Interpretation: 修复前同样本 0/9 前缀与 0.32% 后续命中已消失，且没有通过积累旧 projection 规避问题。
- Time: 2026-07-15 09:50

## Evidence E-007: branch-join 样本独立复验 epoch 布局
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `target/r6-phase-c-epoch/branch-join/multi-file-order-pipeline/20260715-094519-735`
- Prediction or plan link:
  - P-001 复杂样本修复泛化
- Matched signal:
  - 13 个逻辑请求的 39 条精确扫描全部通过，active projection 恒为 1；唯一前缀差异是一次 bootstrap tool shape 转换
- Correlation keys:
  - run `20260715-094519-735`
- Raw content:
  ```text
  request2+ cache: 92.47%
  message prefix: 12/12
  full-shape prefix: 11/12
  same-shape zero-hit: 0
  projection_count: 1
  exact scan failures: 0
  ```
- Interpretation: epoch 基线与原始 control journal 在多文件任务上仍保持 append-only message 历史；shape 转换被单独观测，没有伪装为 projection 失败。
- Time: 2026-07-15 09:50
