# R8 已知问题唯一账本

- Created: 2026-07-31
- Updated: 2026-08-12
- Authority: R8 当前问题状态的唯一事实源
- Historical evidence: `docs/v0.0.5/build-R7/47-r7.1-global-issue-register-legacy.md`

> **VA-04A 离线重映射（2026-08-09）**：TaskSpace Exec Phase B4 已完成观测、固定离线门禁和当前源码重映射。
> 此前的 Tool schema 入侵、顶层结构化容器和 sibling 配对路线已封存在
> [`tool-sequence-protocol/`](tool-sequence-protocol/README.md)。I01/I02/I05/I06 的旧根因已由新架构删除，列为静态关闭候选；
> I10 的 Runtime-only capability identity 已完成离线闭环；I03/I04/I07/I08 与 I10 的生产验收必须等待获批的
> Phase B5 真实证据。离线候选不等于问题关闭。

> **VA-02 当前生产证据（2026-08-09）**：首次获批的正式 `map-request` 请求中，模型没有调用顶层
> `taskspace_exec`，而是把内部 `exec_command` 提升为顶层 call；Runtime 在零副作用边界正确拒绝。该事实归入 I03，
> VA-03 已暂停。运行同时暴露 v11 wire producer 与旧 consumer 的 I07 漂移，已由 `cca76e921` 修复并从原始 trace
> 恢复 usage；详见 [`taskspace-exec/24-phase-b5-va02-first-result.md`](taskspace-exec/24-phase-b5-va02-first-result.md)。
> 后续 VA-02R 已参考最新 Codex `exec` 将 outer Tool 操作合同收敛为一份 catalog-owned description，并通过同一示例的
> decoder/preflight 离线验证；真实 Agent 遵循仍待新预算复验，详见
> [`taskspace-exec/25-phase-b5-protocol-authority-repair.md`](taskspace-exec/25-phase-b5-protocol-authority-repair.md)。

> **VA-02 第二轮生产证据（2026-08-10）**：模型已稳定选择顶层 `taskspace_exec`，合法第二响应可初始化
> `root -> inspect -> fix -> verify -> finish` 并原生执行 client Tool；但两轮首响应都在无 Hosted output 时的必填
> `hosted_bindings: []` 邻近位置生成不同的非法 JSON。I03 因首次参数稳定性继续 verifying，VA-03 保持阻断。I07 的
> provider boundary 范围修复已在新 run 中在线结算 2 provider requests、3 local attempts 和完整 usage；request 2+
> cache hit 为 96.20%。详见
> [`taskspace-exec/39-phase-b5-va02-revalidation-result.md`](taskspace-exec/39-phase-b5-va02-revalidation-result.md)。

> **VA-02 零 Hosted 复验（2026-08-10）**：首响应已省略 `hosted_bindings` 并一次完成 Map 初始化与嵌套
> `exec_command`，证明局部合同修复在线成立。第二响应却生成了未声明的顶层 `exec_command`；两次 Provider 请求实际
> 声明的顶层 Tool 均只有 `taskspace_exec + web_search`，因此不是 Runtime 重新暴露普通 Tool。Runtime 在副作用前拒绝
> 符合硬边界。后续对照两次历史 Function Exec 和非法调用来源指纹后，主根因收敛为 TaskSpace 内层 `calls[]` wire 与
> Provider 顶层 Function Call 过于同形：模型把内层 `tool=exec_command` 提升为顶层，并把 wrapper-only `node_id` 扁平写入
> 原生参数。复用 Standard base 是放大因素而非充分根因；同一模型/base/Function outer 使用 JavaScript `tools.*` 内层语法
> 时连续 15 次保持正确 outer `exec`。适配层和 Runtime 没有改写名称，也没有反馈丢失。I03 继续 verifying，VA-03 继续阻断。

> **I03 离线修复（2026-08-10）**：`calls[]` 已破坏性替换为互斥的 `map` / `client` envelope：Map 使用
> `operation + input`，Client 使用 `name + node_id + input`。旧 `tool + arguments` wire 不再兼容，并有负向测试阻止回流。
> 内部 plan、原生 Tool 输入、Router、Map transaction、Hosted binding 和 Standard 均未改变；TaskSpace Exec 70 项测试通过。
> 该结果证明工程修复完整，不等于目标模型在线稳定性已通过；VA-02 仍需另行批准最小复验，VA-03 继续阻断。

> **I03 map-client wire 在线复验（2026-08-10）**：目标模型连续两次正确生成顶层 `taskspace_exec`，分别执行
> `initialize_map + client(exec_command)` 和后续 `client(exec_command)`；旧顶层 client Tool 提升与旧同形 wire 均未复现。
> 第二请求缓存命中 94.69%，Tool shape 保持稳定。运行未完成业务修复的直接原因是批准的两请求上限：Agent 读取完实现与测试后，
> 第三次请求在 Provider 执行前被预算代理以 429 截止，因此没有 patch。I03 的新 wire 在线可用性已通过，但端到端动作闭环仍待新预算；
> VA-03 继续阻断。详见 [`taskspace-exec/39-phase-b5-va02-revalidation-result.md`](taskspace-exec/39-phase-b5-va02-revalidation-result.md)。

> **VA-02 六请求端到端复验（2026-08-10）**：前三次 outer Exec 完成初始化、读取和失败测试；第 4 次在依赖未满足的
> `fix` 节点 patch，被 DAG 正确拒绝。第 5、6 次都尝试 `update_map + client(apply_patch)`，但 Agent 在 map/client 边界少一个
> `}`，第二次收到解析错误后仍逐字重复。patch 字符串转义正确，Runtime 未改写、补括号或产生副作用。I03 转为 mixed transition
> 协议表现待决策；I04 已获得当前生产观察。6 requests 共 97,584 input、91,392 cached、2,056 output，request 2+ 缓存命中
> 92.68%，排除缓存和预算过早截止为本轮根因。VA-03 继续阻断。

> **TaskSpace 专用合同三臂复验（2026-08-10）**：`standard/structured/source × repeat=3` 已完成。Standard 3/3，
> Structured 0/3，Source 0/3；六次 TaskSpace 运行均只使用顶层 `taskspace_exec`，未再发生 client Tool 逃逸，证明
> Standard/TaskSpace 完整 base 冲突修复在线成立。当前 I03 阻塞转为父子节点交接规则表达不足与 outer arguments 结构
> 不稳定；I07 新确认 Responses 顶层 `instructions` 未被 wire identity scanner 识别，且 observer 不能解码最新两种
> carrier；I08 获得同 commit 三臂成本事实。详见
> [`taskspace-exec/40-va02-source-structured-ab-plan.md`](taskspace-exec/40-va02-source-structured-ab-plan.md) 与账本
> `WAR-20260810-230951-R8-E01-ESCAPE-R3`。

> **Structured 收口修复（2026-08-11）**：用户决定停止 Source，实验 feature、decoder、declaration 分支和专属测试已从
> active code 删除。I03 已在唯一 outer Tool 合同中明确父节点完成后子节点 readiness 由 Map 机械派生，并增加同源
> canonical handoff 反向通过生产 decoder/preflight；Runtime 没有替 Agent 改状态。I07 已对齐当前 `map/client` wire、
> canonical rejection 和 Responses 顶层 `instructions`。两项均完成离线修复，真实 Agent 稳定性仍待新预算复验。

> **Structured 收口在线复验（2026-08-11）**：`standard/map-request × repeat=1` 使用 14 个 Provider 请求；Standard
> 6 请求成功，TaskSpace 8 请求失败。TaskSpace 第一次 outer arguments 少一个闭合括号；随后四次逐字重复合法 JSON 但错误的
> 顶层 `arguments` wrapper。第 6～8 请求恢复正确 `calls`，成功初始化 Map、读取代码并确认根因，但预算在 patch 前截止。
> 这坐实了 I05 的反馈分类缺陷：Runtime 把 JSON 语法错误和合法 JSON 的合同错误都标为 `InvalidJson`，且没有明确顶层恢复
> 形状。当前已离线拆分两类错误并明确零执行/direct-calls 合同；未修补 JSON，也未放宽硬约束。I07 observer 已可从同一
> artifact 同时报告 5 次拒绝和 3 次成功执行。在线效果仍需新预算，详见
> [`taskspace-exec/40-va02-source-structured-ab-plan.md`](taskspace-exec/40-va02-source-structured-ab-plan.md)。

> **I05 修复在线复验（2026-08-11）**：获批的 `single-file-fast-fix × map-request × repeat=1` 在首请求再次生成
> 少一个闭合括号的 JSON；新反馈明确返回 syntax、direct `calls` 和零执行，Agent 下一请求立即改为正确合同并初始化 Map，
> 未再产生 `arguments` wrapper，I05 关闭。随后 Agent 正确读取、修改并通过 3 项测试，且用 canonical handoff 在同批完成
> `inspect` 后执行 `fix`。第 8 个 Provider 响应已完成 Map；最终自然语言回复所需的第 9 次本地尝试被批准的 8-request
> 边界拦截，因此 runner 仍为 partial，不能晋升端到端基线。I03 的首请求 JSON 稳定性和 I04 的一次 waiting-node 误选继续开放。
> I07 同时发现 nested `apply_patch` 已执行但 patch lifecycle 仍计为 0 的观测缺口。

> **I04 批次合同清晰度修复（2026-08-12）**：确认 waiting 规则并非完全缺失，而是“Map 操作改变状态、client outcome
> 不改变状态、client 可并行、整批预检”分散表达，且 handoff 示例没有突出只能解锁直接子节点。当前 Tool 合同已收敛为一条机械规则：
> 只有批次中排在前面的 Map 操作能解锁后续工作，client outcome 不能在同批继续解锁后代；handoff 明确为 direct-child，
> waiting 拒绝返回同一机械边界。Runtime 未自动选点、改状态或重排动作。TaskSpace Exec 82 项测试通过；同一 final-wire fixture
> 的 Tool 段由 29,578 降至 29,263 bytes。

> **I04 批次合同在线复验（2026-08-12）**：获批的 `single-file-fast-fix × map-request × repeat=2` 在首轮完成业务、
> 公开测试、隐藏 oracle 和 5 节点 Map，使用 9 requests、139,221 input、116,992 cached、22,229 uncached、2,759 output，
> request 2+ cache hit 为 88.95%，估算费用 USD 0.0042121576。Agent 仍有一次把测试绑定到 waiting `verify` 的行为；新反馈后
> 能准确复述“client outcome 不能同批解锁后代”，说明批次边界已被理解，但随后没有重放因整批拒绝而未提交的
> `inspect completed`，又触发一次 `TransitionInvalid`，读取 Map 后才纠正。首轮 input 超过 125,000 单轮观察阈值，runner
> 按授权停止，第二轮未执行。因此 I04 保持 verifying：当前证据支持反馈清晰度改善，不支持 waiting 频率或总成本已下降。

> **I04 两轮确认复验（2026-08-12）**：用户追加批准同配置 repeat=2，两轮均完成业务、验证和 Map，分别使用 7 / 8
> requests、103,483 / 118,841 input、93.64% / 92.63% request 2+ cache hit；平均 input 111,162、平均费用
> USD 0.0018663176、平均 Agent wall 22.91s，分别优于修改前暖缓存基线的 120,306、USD 0.0019944、23.2s。
> 平均 request 2+ cache hit 93.13%，仍比旧基线低 0.91 个百分点；两轮各有一次协议/state 拒绝，说明 waiting/frontier
> 行为尚未关闭，但没有复现首轮的 9-request 放大。当前证据不支持回滚合同修复；首轮较差结果按新前缀冷启动叠加异常动作路径保留，
> 不从账本删除，也不据此宣称缓存已经完全等价。

> **最新根因收敛与计划（2026-08-11）**：用户确认对唯一可证明的单闭合符号缺失执行 Runtime 机械自愈，且修正版必须在
> `OutputItemDone` 落账前替换原 FunctionCall，成为 history、rollout、RawResponseItem 和 dispatch 共用的唯一正式事实；只在
> handler/decoder 中修补会保留错误上下文，已判定为错误实现方向。I04 中 Agent 并未“调用 waiting”，而是把 `apply_patch`
> 绑定到仍依赖未完成 `inspect` 的 `fix`；DAG 拒绝正确，当前反馈缺口是只输出内部状态枚举、没有展开未完成父节点。I07 的
> patch 专项消费者仍只识别顶层 Tool 和旧 `taskspace_control`，没有消费当前 `calls[].client`。修复已作为 SR-01～SR-03、
> WF-01、OB-03 写入唯一 Phase B5 计划；未启动新的真实运行。

> **SR/WF/OB 在线复验（2026-08-11）**：获批的 `single-file-fast-fix × map-request × repeat=1` 完整结算
> 10 个 Provider 请求，缓存从第 2 请求起命中 91.20%，usage 和 Patch parse-failure 观测完整，但业务失败且没有 patch。
> 首请求是可由单个 `}` 修复的中文 arguments，自愈却未触发：`serde_json` 的错误列号是 UTF-8 byte column，SR-01
> 错按 Unicode 字符序号换算，候选窗口因中文内容偏离。后续 syntax reject 又无条件附加 no-wrapper 提示，Agent 被错误反馈
> 带向一次真实 `arguments` wrapper。两项确定工程缺口已纳入并完成 SR-04、FF-01 离线修复。其余 mixed batch 同时破坏
> map/client call envelope，超出单闭合符自愈边界，继续归 I03 在线稳定性；本轮未进入 waiting preflight，不能作为 I04 在线验收。

> **SR-04 / FF-01 预算包复验（2026-08-11）**：3 次有效 `single-file-fast-fix × map-request` 运行全部完成业务、
> 公开验证、隐藏 oracle、Map 闭合和最终答复；共 21 requests，344,635 input / 323,200 cached / 21,435 uncached /
> 6,555 output，估算 USD 0.00574126。三轮均无 syntax、wrapper 或顶层 client 逃逸，但也都未自然触发自愈，故只证明
> 修复后生产路径稳定、不证明在线自愈分支命中。第三轮两次 waiting 子节点误选被零副作用拒绝，反馈准确列出直接父节点，
> Agent 下一请求完成父节点并继续；WF-01 在线成立，I04 的 Agent 行为仍开放。详见
> [`taskspace-exec/41-phase-b5-sr04-ff01-revalidation-result.md`](taskspace-exec/41-phase-b5-sr04-ff01-revalidation-result.md)。

> **I03/I04 表达模型重评（2026-08-12）**：最新静态盘点确认，当前 `calls[]` 要求 Agent 自行组合 Map
> 边界、状态转移和 client work，而 schema 本身没有把“完成前置并继续”等合法场景表达为一等类型。
> 用户已确认转向闭集合法顺序。七个核心场景均已关联真实 trace，或确定性能力加已确认产品需要；Provider Tool 进入统一
> Tool action，纯 `update_map` 保留，Ready 工作机械进入 InFlight，无正向运行证据的 `blocked` 将被删除。该方向同时是
> I03 的直接修复路径和 I04 的上游重评前置；当前完成产品合同，不等于两问题已修复。见
> [`taskspace-exec/43-closed-legal-sequence-design.md`](taskspace-exec/43-closed-legal-sequence-design.md)。

> **LS-09 闭集序列在线验收（2026-08-12）**：Run A/B 均完成业务、隐藏验证和 Map 闭合，且没有普通 client Tool
> 顶层逃逸。复杂 Run B 使用 12 requests；一次嵌套 `apply_patch` 参数含裸换行而成为非法 JSON，两次 L2 `work` 选择
> Waiting 后继并被零副作用拒绝，下一请求才改用 L4 完成父节点后继续。最终 Map 是 5 节点线性链，没有覆盖预定的
> fork/join 与 Map 调整。该证据不否定 Runtime DAG 硬边界，但证明闭集的分支适用条件尚未让 Agent 稳定选对序列，I03/I04
> 继续 verifying。Run B 请求 3+ cache hit 92.28%，Tool schema 与 `tool_choice` 无变化，缓存结构回归已排除。按批准停点
> 暂停 provider-hosted Run C，不新增问题编号。

> **LS-09 分支适用合同补全（2026-08-13）**：Run B 证明八条序列虽然已被 Runtime 完整实现，但各 `anyOf` 分支此前
> 没有自包含地说明选择条件。当前 L1～L8 均已补齐分支级适用合同；L2/L4 直接表达 Tool outcome 不完成节点、只有前置
> Map update 能解锁本批 owner、同批 Tool outcome 不解锁后继。TaskSpace Exec 72/72、zero-base 与缓存敏感面门禁通过，
> 且未改变状态机或拒绝语义。I03/I04 继续 verifying：这是离线修复完成，不是目标模型行为已复验。

> **LS-09 Run C 真实验收（2026-08-13）**：`provider-web-search-probe × map-request × repeat=1` 未通过正式闭环。
> Map 初始化后 Agent 正确使用 L4/L2，未复现 Waiting 后继误选；业务文件、公开验证和隐藏 oracle 均通过。但初始化前
> Agent 连续 7 次试探 Hosted `web_search` 的 Exec 声明方式，12 次请求上限在 Patch 成功后耗尽，Map 未闭合。Runtime
> 每次均忠实、零副作用拒绝；Agent 可见合同缺少“Hosted action 是同响应 output 的逐项归属声明、参数不在 Exec 内、
> 失败 output 也必须声明”的操作语义。该缺口归入 I03，不新增问题；I04 保持 verifying，不能由单次无 Waiting 拒绝关闭。
> 详见 [`taskspace-exec/44-ls09-run-c-result.md`](taskspace-exec/44-ls09-run-c-result.md)。

> **Hosted action 合同针对性修复（2026-08-13）**：统一 `tools[]` 的 Hosted schema 已补回旧结构迁移时丢失的操作合同：
> 每项只归属同响应已经执行的一个 Provider output，不携带原生 Tool input；全部 output 按序逐项声明，失败项和 action
> subtype 也不例外，并始终使用公开 Tool 名。修复直接落在最终 Provider-visible variant，未增加提示层、Runtime 推断、
> 对账分支或状态语义。TaskSpace Exec 72/72 通过；该段只记录离线实现，后续真实复验未通过。

> **Hosted action 合同复验（2026-08-13）**：`provider-web-search-probe × map-request × repeat=1` 在 12 个 Provider
> requests 后仍未生成业务文件。最终 schema 已确认进入真实 wire，但 Agent 继续把 Hosted action 写成带 `node_id/input`
> 的 client 执行请求，并在没有对应 Hosted output 的响应中补登记。根因不再表述为单纯“合同文字缺失”，而是统一
> `tools[]` 同时承载执行前 client 请求和执行后 Hosted 归属凭据，两者外观相近而生命周期相反。I03 保持 verifying；
> 在用户确认逐 output item 或逻辑 Provider Tool 调用粒度之前，不继续增加提示或 Runtime 语义。详见
> [`taskspace-exec/45-ls09-hosted-contract-revalidation-result.md`](taskspace-exec/45-ls09-hosted-contract-revalidation-result.md)。

TaskSpace Exec Phase B4 已完成正式生产链、可靠 Action 结算、跨层观测、缓存/性能消费和固定离线验收。该结果证明工程
不变量成立，但尚未证明目标 Provider 下的 Agent 行为、三种 projection 的效果和不可约成本；最终关闭仍按
VA-04B 使用 Phase B5 当前 trace 重评。

TaskSpace Exec 与全局问题的处理边界统一记录在
[`taskspace-exec/03-global-issue-prerequisite-review.md`](taskspace-exec/03-global-issue-prerequisite-review.md)：I07 已确认的
请求/usage 双计子问题前置为 TX-00；I10、I06、I01/I02/I05 分别融入新方案对应单元；I03/I04/I08 等生产接入后
重评。该映射不改变本表任何问题状态，也不新增 I07 子问题编号。

## 1. 使用规则

本账本迁移 R7.1 已观测到的问题，不迁移旧根因和旧修复方案。`Source` 只用于追溯历史证据。

新增问题必须满足以下之一：

- 当前源码或确定性测试证明独立缺陷；
- 当前有效 trace 证明新的用户可见或 Agent 可见异常；
- 当前问题深挖后发现无法归入既有问题的独立根因。

不得把一次失败中的多个日志表现重复登记为多个问题，也不得把计划、验收或发布步骤登记为产品问题。

## 2. 影响分层

| 层级 | 责任面 | 该层失败的影响 | 对应问题 |
|---:|---|---|---|
| F0 | canonical Map Store | Runtime 读取到非法或错误事实，所有上层判断失去基础 | I09 |
| F1 | Runtime 事务与 revision | 同一动作出现竞争状态或提交身份，导致 stale、重复提交和错误恢复 | I01 |
| F2 | Tool admission 与 dispatch | TaskSpace 硬约束可被入口绕过，出现未绑定或多 Patch 真实执行 | I06 |
| F3 | Tool feedback 与 context | Agent 收到丢失、重复、歧义事实，缓存前缀也可能被破坏 | I05、I02 |
| F4 | capability 与观测身份 | 可见能力和证据边界不稳定，行为与成本结论无法准确归因 | I10、I07 |
| F5 | Agent 协议行为 | Agent 生成低效或非法动作，但底层仍应守住正确性 | I03、I04 |
| F6 | 成本与晋升 | 衡量修复后的不可约产品成本，不能反向决定底层语义 | I08 |

## 3. 当前全集与优先级

`P0` 表示 canonical 正确性或不可绕过边界；`P1` 表示语义、能力身份或证据可信性；`P2` 表示 Agent 行为；
`P3` 表示修复后的成本和发布验收。执行序优先处理更底层责任面。

本表只描述产品问题，不在问题名称中预设技术根因。具体机制、证据和修复方案进入各问题专项文档。

| 执行序 | ID | 层级 | 严重度 | 产品问题 | 产品应有表现 | VA-04A 离线结论 | 状态 | Source |
|---:|---|---:|---:|---|---|---|---|---|
| 1 | R8-I09 | F0 | P0 | 恢复旧任务时可能接受内部关系损坏的任务地图 | 只恢复结构完整的地图；损坏时停止且不改变当前事实 | 当前关系 Store、hydrate 校验和 State 回归继续成立 | [closed](I09/01-i09-store-hydrate-repair-result.md) | GI-009 |
| 2 | R8-I01 | F1 | P0 | 一轮工作后 Agent 可能收到互相竞争的新旧进度 | 每轮只有一个可继续使用的结果，revision 不成为 Agent 填表负担 | 旧 receipt/control 双版本链已删除；Exec 只返回一个 outer 结果，request revision 由 Runtime 内部维护。静态关闭候选，待 E3 排除 stale 重试 | [verifying](I01/00-i01-response-final-revision-repair-plan.md) | GI-001 |
| 3 | R8-I06 | F2 | P0 | 组合工具内部动作可能绕过归属和单 Patch 硬门 | 所有 TaskSpace client 动作先过同一请求级预检，普通 Tool 保持原生 | 生产顶层仅 Exec+Hosted；完整 plan 在副作用前校验，顶层绕过和多 Patch 有确定性拒绝。静态关闭候选 | verifying | GI-006 |
| 4 | R8-I05 | F3 | P1 | 拒绝原因可能重复或混淆错误发生的协议层级 | 忠实返回一次失败，并准确区分语法、合同、预检与执行错误；未提交候选不得表现为已保存状态 | 只有实际顶层 `arguments` 才返回 wrapper 事实；修复后简单样本 3/3 无 wrapper，但未自然进入 syntax reject 分支，保持验证态 | verifying | GI-005 |
| 5 | R8-I02 | F3 | P1 | Tool 事实可能被另造高优先级消息重复包装 | 原 Tool/outer Tool 反馈只进入上下文一次，不建立 system/developer 副本 | 旧 carrier 与专属 Event Store 已由 zero-base 删除；Exec 源码不存在额外 developer 注入。静态关闭候选，待 final-wire trace 复核 | verifying | GI-002 |
| 6 | R8-I10 | F4 | P1 | 工具能力变化没有跨执行、缓存和报告共用的身份 | 实际工具集合变化才切换身份，各消费面引用同一值 | 同一 Catalog 快照机械生成 Runtime-only SHA-256，并由 dispatch、request scope、Provider/Exec trace 和性能报告共用；缺失或冲突时报告不可比较。离线实现已验证，待当前生产 trace 验收 | [verifying](I10/00-i10-capability-identity-repair-plan.md) | GI-010 |
| 7 | R8-I07 | F4 | P1 | 观察工具可能漏计、重复计数或使用过期证据 | 请求和失败逐身份计一次；协议拒绝与证据损坏分开表达，身份不一致时才不可比较 | 最新三轮 request/usage/cache/Exec/client/Patch/Map 均可复算；第三轮正确计为 2 次 patch 声明、1 次 preflight reject、1 次执行结果。完整跨模式验收仍未执行 | [verifying](I07/00-i07-observability-trust-repair-plan.md) | GI-007 |
| 8 | R8-I03 | F5 | P2 | Agent 不能稳定组织 Map 与工作动作的同轮提交 | 稳定生成初始化并执行、完成并继续、完成并结束等合法组合 | Run D 证明 Hosted 文字合同已送达但不足以收敛行为；统一 `tools[]` 同时表达执行前 client 请求与执行后 Hosted 凭据，生命周期冲突待产品决策；正式验收未通过 | [verifying](taskspace-exec/45-ls09-hosted-contract-revalidation-result.md) | GI-003 |
| 9 | R8-I04 | F5 | P2 | Agent 可能选择依赖未满足或已完成的节点 | Agent 准确使用可执行 frontier；Runtime 只守硬规则 | Run C 未复现 Waiting 误选，支持分支适用合同已生效；单次未闭环样本不足以关闭问题 | verifying | GI-004 |
| 10 | R8-I08 | F6 | P3 | TaskSpace 的请求、输入、时间和未缓存成本可能高于 Standard | 额外成本可解释、稳定并与产品收益匹配 | 最新三次 TaskSpace-only 有效运行共 21 requests、344,635 input、93.78% 全量 cache、62.093s Agent wall；没有 Standard 臂，不形成相对成本结论 | queued | GI-008 |

问题总数：**10**；Open：**9**；Closed：**1**。当前专题：**TaskSpace Exec Phase B6 闭集合法顺序实施**。

## 4. VA-04A 证据边界

| 分类 | 问题 | 当前可下结论 | 当前不能下结论 |
|---|---|---|---|
| 确定性关闭 | I09 | 关系 Store hydrate 仍拒绝非法图，State 134 项通过 | 无 |
| 静态关闭候选 | I01、I02、I06 | 旧根因和旧生产路径为零，新 Exec 的内部 revision、零副作用预检和不可绕过入口有确定性测试 | 目标模型是否仍产生 stale 重试或非法组合 |
| 工程修复待生产验收 | I05 | syntax 与顶层 `arguments` 已由 typed error 分流，纯 syntax 不再附带 wrapper 恢复提示；零副作用合同不变 | 当前修复是否在线消除错误反馈诱发的 wrapper 行为 |
| 工程完成待生产验收 | I10 | catalog、dispatch、request scope、Provider/Exec trace 和报告共用同一 Runtime-only identity；Standard request 不变 | 当前 Provider trace 是否完整携带且逐 request 一致 |
| 工程修复待生产验收 | I07 | 当前生产 trace 已原生完整计量 Exec、Map、client 和拒绝 | nested patch lifecycle 仍漏计，不能宣称完整关闭 |
| outer wire 与 handoff 在线观察 | I03 | 旧顶层提升未复现；单闭合符自愈的 UTF-8 坐标缺口已确定性修复 | mixed map/client envelope 的在线稳定性，以及自愈后的真实首请求表现 |
| 当前行为已观察 | I04 | 新 waiting 反馈已在线命中；追加两轮请求、input、费用和时间均不差于旧暖缓存基线，Runtime 保持零副作用 | 每轮仍有一次协议/state 拒绝；缓存平均低 0.91pp，尚不能关闭 frontier 行为问题 |
| 成本待验证 | I08 | 最新 TaskSpace request-2+ 缓存为 91.20%，排除缓存失效 | 去除已确认 syntax/feedback 放大后的不可约请求、token 和时间成本；VA-03 尚未开始 |

本轮 B4 证据为：TaskSpace Exec 57、settlement/recovery 11、State 134、Core 1856/3、CLI 5、Viewer 4、App Server
Protocol 183、workspace、zero-base 和 cache gate 全部通过，详见
[`22-phase-b4-offline-acceptance.md`](taskspace-exec/22-phase-b4-offline-acceptance.md)。OB-01/OB-02 的身份链和报告消费见
[`19-phase-b4-observability-audit.md`](taskspace-exec/19-phase-b4-observability-audit.md) 与
[`21-phase-b4-performance-observer-result.md`](taskspace-exec/21-phase-b4-performance-observer-result.md)。完整重映射结论见
[`23-phase-b4-issue-remap-result.md`](taskspace-exec/23-phase-b4-issue-remap-result.md)。

I10 后续离线补证为 TaskSpace Exec 58、Core 1857/3、workspace、zero-base、性能观察 fixture 和缓存门禁通过，见
[`I10 修复计划与结果`](I10/00-i10-capability-identity-repair-plan.md)。

旧 control/sibling 真实运行、旧 developer carrier 缓存结果和旧请求放大数字只保留在各专题历史文档中，不再作为当前
问题的产品证据。VA-04B 只使用最终生产入口的获批 trace 更新状态。

## 5. 依赖与重评关系

| 上游问题 | 关闭后必须重评 | 原因 |
|---|---|---|
| I09 旧任务恢复可信性 | I01、I04 | 先确保恢复出的任务地图可信，才有资格评价后续进度版本和节点选择 |
| I01/I02/I05/I06 静态候选 | I03、I04、I08 | Phase B5 同一 trace 同时确认旧根因未复现，再评价 Agent 行为与成本 |
| I10 稳定的工具能力版本 | I07、I08 | 性能报告必须能区分“工具变了”和“任务变了” |
| I03 稳定的动作组合 | I04 | 先解决通用动作组织问题，再判断节点顺序错误是否仍是独立问题 |
| I01～I07、I09～I10 | I08 | 成本是最终验收，不作为底层设计的先验优化目标 |

I07 不作为所有问题的整体前置。其 request/usage 双计、local attempt/boundary 对账和 Exec 动作身份已完成离线修复；
后续只负责用当前生产 trace 验收，不再扩展为长期 Observer 专项。

## 6. 已知但不作为独立问题迁移

| 事项 | R8 处理方式 |
|---|---|
| R7.1 的 20 个 Phase | 不迁移；其中包含调查、实现、评测和发布动作，不是问题全集 |
| 五层架构 | 不作为预设答案；相关职责边界按 R8 全局约束重新验证 |
| 三种 projection 的固有差异 | 保留为产品模式，不把已声明差异当成缺陷 |
| 旧 candidate 的完成度与晋升门 | 不迁移；R8 重新建立自己的实现与验收证据 |
| 历史兼容与旧 Map 数据 | 无保留价值，不建立兼容工作 |
| 未证明的“Agent 智能不足” | 不登记；优先检查上下文和 Tool 反馈 |

## 7. 关闭要求

每个问题关闭时必须在本表更新状态，并链接一份问题结果文档。结果文档必须包含：

- 实际根因和被否定的假设；
- 修改与删除的代码路径；
- 确定性测试、日志和回归结果；
- 对 Standard、连续动作、普通 Tool、Map Store、缓存和成本的影响；
- 若使用真实 Agent，关联全局运行账本；
- 对全局约束逐项检查的结论。
