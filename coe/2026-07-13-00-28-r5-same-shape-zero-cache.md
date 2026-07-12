# Problem P-001: R5相同请求形态出现零缓存命中
- Status: validating
- Created: 2026-07-13 00:28
- Updated: 2026-07-13 00:28
- Objective: 解释R5在wire消息前缀和工具形态保持时仍出现`same_shape_zero_hit`的原因，避免把provider落盘时序误判为上下文结构或缓存键缺陷。
- Symptoms:
  - 排除120-request异常配对后，5组R5样本中有3组各出现1次`same_shape_zero_hit`，Standard为0次。
  - 零命中请求前后均保持相同tools hash、tool choice和完整message prefix。
- Expected behavior:
  - 正常业务tool loop应尽可能复用DeepSeek前缀缓存。
  - 观测必须区分前缀破坏、缓存落盘尚未完成和provider best-effort miss。
- Actual behavior:
  - 三个零命中请求分别在上一响应结束后50ms、81ms、88ms发出。
  - 三者均由plain final被Runtime拒绝后自动发起，无用户输入或业务工具动作。
- Impact:
  - 三个请求额外产生9,803、15,471、15,220个uncached input tokens。
  - 其后请求先部分命中再恢复高命中，污染R5缓存收益判断。
- Reproduction:
  - 读取`target/r5-j6-7-7-repeat3-final`下focused repeat-1、complex repeat-1和complex repeat-3的`provider-cache-trace.jsonl`与`provider-request-events.jsonl`。
- Environment:
  - Docker hard boundary，`deepseek-v4-flash`，binary commit `84979fe`。
- Known facts:
  - E-001：三次零命中与三次单次final rejection一一对应。
  - E-002：零命中前wire消息为严格前缀，tools和tool choice不变。
  - E-003：请求间隔仅50-88ms；DeepSeek官方说明缓存构建耗时为秒级且为best-effort。
  - E-004：无final rejection的两个focused R5 repeat均无`same_shape_zero_hit`。
- Ruled out:
  - projection重排、tools schema变化、tool choice变化不是这三次零命中的原因。
  - 不需要通过等待、缓存重试或Runtime语义控制修复provider缓存。
- Fix criteria:
  - 删除final rejection自动follow-up后，不再产生对应的无业务请求。
  - 新focused/complex样本不出现`TaskSpaceFinalAnswerRejectedV1`或`final_rejected`。
  - 若仍有零命中，按最终wire前缀和请求间隔作为新的独立证据重新分类。
- Current conclusion: H-001已确认。缓存异常是final rejection越界循环的次生效应，不是已证实的缓存键或projection缺陷；与final loop同根修复，等待sample验证。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - diagnostic evidence satisfied; fix validation pending
- Close reason:
  - not closed

## Hypothesis H-001: final拒绝后的即时自动请求早于provider缓存落盘
- Status: confirmed
- Parent: P-001
- Claim: Runtime在plain final拒绝后于50-88ms内自动重采样；上一响应的缓存前缀尚未完成秒级落盘，因此该无业务请求得到零命中。
- Layer: interaction
- Factor relation: single
- Depends on:
  - `coe/2026-07-12-23-17-r5-final-rejection-provider-loop.md` H-001
- Rationale:
  - 三个非异常R5零命中位置都紧跟Agent final及Runtime rejection，下一请求恢复部分或完整命中。
- Falsifiable predictions:
  - If true: 每个零命中请求都应紧跟final rejection，且间隔远小于官方所述秒级缓存构建时间。
  - If false: 零命中应出现在普通tool loop，或存在wire前缀、tools、tool choice变化。
- Diagnostic evidence plan:
  - Prediction or clause under test: 零命中与Runtime自动final recovery的时序一一对应。
  - Signal: request index、上一terminal completed时间、下一started时间、final rejection trace、prefix与shape字段。
  - Capture method: join `provider-cache-trace.jsonl`、`provider-request-events.jsonl`和`rollout.jsonl`。
  - Event name or marker:
    - `same_shape_zero_hit`
    - `provider_response_actionability:final_rejected`
  - Correlation keys:
    - focused repeat-1 request 7
    - complex repeat-1 request 15
    - complex repeat-3 request 9
  - Differentiates from:
    - projection变更、tool schema变更、provider transport retry
  - Supports if:
    - 三个请求均在rejection后小于100ms发起且保持wire前缀。
  - Refutes if:
    - 任一请求存在业务动作、前缀破坏或较长静默间隔。
  - Instrumentation status: existing-permanent-observability
  - Instrumentation lifecycle:
    - 保留wire prefix、terminal usage和request lifecycle时间戳。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
- Conclusion: confirmed
- Repair design readiness: no separate cache repair；复用final loop边界修复
- Next step: Docker sample fix validation
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: TaskSpace动态projection破坏了缓存前缀
- Status: refuted
- Parent: P-001
- Claim: projection latest replacement或上下文重排改变了零命中请求的已有消息前缀。
- Layer: context
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - R5存在动态Map状态，需要排除canonical context被替换。
- Falsifiable predictions:
  - If true: 零命中请求的`prefix_preserved`应为false或message LCP短于上一请求。
  - If false: 上一请求所有message均逐项保持，变化只在尾部追加。
- Diagnostic evidence plan:
  - Prediction or clause under test: message级最终wire前缀是否破坏。
  - Signal: `message_prefix_preserved`、`lcp_message_count`、`message_count`。
  - Capture method: provider final wire trace。
  - Event name or marker:
    - `provider.chat_wire_prefix_preserved`
  - Correlation keys:
    - 三个零命中request id
  - Differentiates from:
    - H-001缓存落盘时序
  - Supports if:
    - prefix broken。
  - Refutes if:
    - prefix preserved且LCP覆盖上一请求全部消息。
  - Instrumentation status: existing-permanent-observability
  - Instrumentation lifecycle:
    - 保留。
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - refuted

## Hypothesis H-003: tools或tool choice改变导致缓存形态切换
- Status: refuted
- Parent: P-001
- Claim: blank-map到active-map的tools schema或named到auto切换发生在零命中请求上。
- Layer: transport
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 每个R5 run初始化后确有一次预期的tool shape切换。
- Falsifiable predictions:
  - If true: 零命中行应有`tool_choice_changed=true`或不同`tools_hash/cache_shape_hash`。
  - If false: 零命中及相邻请求shape完全一致。
- Diagnostic evidence plan:
  - Prediction or clause under test: 零命中是否与初始化shape transition重合。
  - Signal: tools hash、cache shape hash、tool choice kind。
  - Capture method: provider cache trace。
  - Event name or marker:
    - `provider.chat_wire_shape_recorded`
  - Correlation keys:
    - 三个零命中request id
  - Differentiates from:
    - H-001缓存落盘时序
  - Supports if:
    - shape changed。
  - Refutes if:
    - shape相同且transition只发生在初始化请求。
  - Instrumentation status: existing-permanent-observability
  - Instrumentation lifecycle:
    - 保留。
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: refuted
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - refuted

## Evidence E-001: 三个非异常R5零命中均紧跟final rejection
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: `target/r5-j6-7-7-repeat3-final` provider cache trace与rollout
- Prediction or plan link:
  - H-001一一对应预测
- Matched signal:
  - focused repeat-1 request 7、complex repeat-1 request 15、complex repeat-3 request 9均为各自唯一零命中，并紧随各自唯一logical final rejection。
- Correlation keys:
  - request 7/15/9
- Raw content:
  ```text
  final_rejection logical counts: 1, 1, 1
  same_shape_zero_hit counts:      1, 1, 1
  ```
- Interpretation: 零命中并非散布在普通tool loop，而是集中在同一越界控制路径。
- Time: 2026-07-13 00:24

## Evidence E-002: 零命中请求保持最终wire消息前缀和工具形态
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: diagnostic-log
- Source: 三组`provider-cache-trace.jsonl`
- Prediction or plan link:
  - H-002、H-003的反证条件
- Matched signal:
  - 三组均为`prefix_preserved=true`、相同`tools_hash=7ed0...359f`、相同auto tool choice和相同`cache_shape_hash=3f9c...976d`。
- Correlation keys:
  - request 7/15/9
- Raw content:
  ```text
  request 7:  lcp_messages=23, previous_message_count=23
  request 15: lcp_messages=40, previous_message_count=40
  request 9:  lcp_messages=31, previous_message_count=31
  ```
- Interpretation: 当前可观测的最终provider payload结构没有发生中部改写或shape切换。
- Time: 2026-07-13 00:24

## Evidence E-003: 自动请求间隔显著短于官方缓存构建时间
- Related hypotheses:
  - H-001
- Direction: supports
- Type: runtime-timing-and-primary-documentation
- Source: `provider-request-events.jsonl`；DeepSeek API官方Context Caching文档
- Prediction or plan link:
  - H-001小于100ms的时序预测
- Matched signal:
  - 上一响应completed到零命中请求started分别为50ms、81ms、88ms。
  - 官方文档说明缓存构建耗时为秒级，且缓存系统为best-effort、不保证100%命中。
- Correlation keys:
  - focused r1 request 6->7
  - complex r1 request 14->15
  - complex r3 request 8->9
- Raw content:
  ```text
  1783868434189 - 1783868434139 = 50 ms
  1783868525034 - 1783868524953 = 81 ms
  1783869118112 - 1783869118024 = 88 ms
  ```
- Interpretation: Runtime即时重采样到达时，上一输出对应的缓存单元没有合理的落盘时间。
- Time: 2026-07-13 00:25

## Evidence E-004: 无final rejection的R5 repeat无同形零命中
- Related hypotheses:
  - H-001
- Direction: supports
- Type: negative-control
- Source: focused repeat-2与repeat-3 cache trace及rollout
- Prediction or plan link:
  - H-001关于触发条件的预测
- Matched signal:
  - 两组均`logical final rejection=0`且`same_shape_zero_hit=0`。
- Correlation keys:
  - focused repeat-2/repeat-3
- Raw content:
  ```text
  focused r2: rejection=0, same_shape_zero=0
  focused r3: rejection=0, same_shape_zero=0
  ```
- Interpretation: 负对照支持final rejection即时请求是本轮可见零命中的共同触发条件。
- Time: 2026-07-13 00:26
