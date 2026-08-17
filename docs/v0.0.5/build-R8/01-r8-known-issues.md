# R8 已知问题唯一账本

- Created: 2026-07-31
- Updated: 2026-08-18
- Authority: R8 当前问题状态的唯一事实源
- Historical evidence: `docs/v0.0.5/build-R7/47-r7.1-global-issue-register-legacy.md`
- Current progress report: [`03-r8-current-progress.md`](03-r8-current-progress.md)

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
> 未再产生 `arguments` wrapper，因此“syntax/contract 分类与 wrapper 诱导”子问题关闭；I05 全局项仍需覆盖其他拒绝入口。
> 随后 Agent 正确读取、修改并通过 3 项测试，且用 canonical handoff 在同批完成
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
> 每项只归属同响应已经执行的一个 Provider ToolSpec，不携带原生 Tool input；Provider 内部 output/action subtype 不单独
> 声明，名称逐字使用原生 ToolSpec 名。修复直接落在最终 Provider-visible variant，未增加提示层、Runtime 推断、
> 对账分支或状态语义。TaskSpace Exec 72/72 通过；该段只记录离线实现，后续真实复验未通过。

> **Hosted action 合同复验（2026-08-13）**：`provider-web-search-probe × map-request × repeat=1` 在 12 个 Provider
> requests 后仍未生成业务文件。最终 schema 已确认进入真实 wire，但 Agent 继续把 Hosted action 写成带 `node_id/input`
> 的 client 执行请求，并在没有对应 Hosted output 的响应中补登记。根因不再表述为单纯“合同文字缺失”，而是统一
> `tools[]` 同时承载执行前 client 请求和执行后 Hosted 归属凭据，两者外观相近而生命周期相反。I03 保持 verifying。详见
> [`taskspace-exec/45-ls09-hosted-contract-revalidation-result.md`](taskspace-exec/45-ls09-hosted-contract-revalidation-result.md)。

> **Hosted 原生 ToolSpec 产品纠偏与离线修复（2026-08-13）**：用户明确一次 `web_search` 是不可拆分的一个 Tool action；
> `search/open_page`、内部失败和重试都不得进入 TaskSpace 协议、Map 或节点绑定。active code 已删除逐 output 数量、顺序、
> Provider ID 和 output index 对账；同一 response scope 的同种 Hosted capability 只形成一个使用 outer call + Tool index
> 身份的 action，Agent 声明一次 `node_ids`。成功 search 加失败 open_page 的 fixture 只产生一个成功 action，TaskSpace Exec
> 73/73 通过。I03 继续 verifying，等待已批准的单次真实复验，不以离线测试宣称关闭。

> **Hosted 原生 ToolSpec 在线复验（2026-08-13）**：`provider-web-search-probe × map-request × repeat=1` 证明内部过程聚合已进入
> 生产路径：同响应 `search succeeded + open_page failed` 只形成一个 `actual=[web_search]`，`find_in_page failed` 加一次
> Agent 声明只返回一个 Hosted result，Provider 内部 ID/index/subtype 均未进入 outer result。端到端仍失败：Agent 在
> 两次真实搜索响应中漏写归属，Runtime 当轮准确拒绝；下一请求补声明时当前响应已无 Hosted 事实，合同没有合法恢复路径，Agent
> 误判为能力注册反复变化。后续原始 reasoning 复核证明 Agent 曾把 Hosted 归属结构直接当成搜索执行入口，因此跨响应恢复被降级为后续候选。当前先验证 Hosted-only 必填 `execution: "already_executed"`、禁止 `input`、正向示例和事实型错误反馈的单变量修复；离线 TaskSpace Exec 74/74 已通过，真实 Provider 行为未复验。

> **Hosted 执行方向在线复验（2026-08-13）**：`provider-web-search-probe × map-request × repeat=1` 业务、公开验证、
> 隐藏 oracle 和 Map 闭合全部通过。Agent 能准确解释 `already_executed` 不调用 Tool，并最终在同一响应中完成真实搜索和
> 一次原生 ToolSpec 归属；未再携带 Hosted `input`，也未拆分 Provider 内部步骤。但首次仍提前登记一次，第一次真实搜索后仍漏登并在
> 下一响应补登失败。该结构从失败闭环改善为成功闭环，但 12 requests 和约 303K input 未下降，故 I03 保持 verifying；不把
> 单次成功扩大为跨响应恢复、自动绑定或默认 Root 的依据。详见
> [`taskspace-exec/47-ls09-hosted-execution-direction-result.md`](taskspace-exec/47-ls09-hosted-execution-direction-result.md)。

> **同响应双写五轮复验（2026-08-14）**：提交 `806b29780` 明确原生 Provider Tool 与 Exec 归属必须在同一 assistant
> response 成对出现。五轮共 60 requests，Agent 16 次 Hosted 声明中 8 次成功对账，但每轮仍出现协议或序列错误，公开验证
> 仅 2/5。纠偏后主根因是两个独立顶层 response item 的共现关系无法由现有 JSON Schema 结构性表达，只能依赖文字合同和
> Runtime 事后核对；`search/open_page/find_in_page` 只是同一 Web Search 的内部 action，不是三个 Tool，也不分别双写。
> 现有 `update_and_work` 无法表达“先登记本响应 Hosted work，再完成 owner”仍是独立序列缺口。I03 继续 verifying。
> runner 无条件遇业务失败停止 repeat 的行为归入 I07 测量工程，不新增全局问题。
> 详见 [`taskspace-exec/48-ls09-same-response-pairing-repeat5-result.md`](taskspace-exec/48-ls09-same-response-pairing-repeat5-result.md)。

> **不可拆分措辞阶段复验（2026-08-14）**：提交 `ba8198acd` 只把 Hosted 调用与 Exec 归属改写为不可拆分的一对，
> 未改 Runtime、Map 或序列。计划 repeat=5，前三轮均未完成业务闭环；第 3 轮因漏登/补登后的反复搜索达到 947,691
> input 并触发单轮 500,000 观察阈值，后两轮停止。首轮还出现模型直接生成顶层 `web_search` Function Call 的误读，
> 证明更强文字没有给两个独立顶层 item 建立结构性共现约束，并可能强化错误调用形态。I03 保持 verifying；剩余两轮须
> 重新授权，且不得把 Web Search 内部 action 当成独立 Tool 或独立双写。详见
> [`taskspace-exec/49-ls09-indivisible-pairing-partial-result.md`](taskspace-exec/49-ls09-indivisible-pairing-partial-result.md)。

> **原生 Hosted Tool 身份在线验收（2026-08-14）**：提交 `2b92f2345` 删除 TaskSpace 自建 Hosted 公共名称，改为逐字
> 复用当前请求 `ToolSpec::name()`。Standard/map-request smoke 2/2 通过；专项 Web Search trace 确认 Provider 顶层声明、
> 原始 response item 和 Runtime mismatch 均只使用原生 `web_search`，内部 `search/open_page/find_in_page` 没有被提升成 Tool。
> 但专项运行的两个逻辑 Hosted 使用均漏掉同响应 Exec 归属，成功 Hosted 归属为 0；业务、校验和 Map 最终通过不能替代协议验收。
> I03 保持 verifying。I07 需把“原生执行数、成功归属数、mismatch 数”分开观察，避免把 `provider_results=0` 误读为没有执行。
> 详见 [`taskspace-exec/50-native-hosted-identity-live-result.md`](taskspace-exec/50-native-hosted-identity-live-result.md)。

> **Hosted 错误优先级第 1 阶段复验（2026-08-14）**：提交 `a54cae056` 只将 Hosted 实际事实与归属登记的集合核对
> 提前到 client 节点可执行性校验之前。离线合同通过且缓存敏感面不变；真实 `repeat=3` 在首轮业务失败后按停点停止。
> 本轮没有命中“Hosted 漏登 + waiting client”的目标复合分支；Agent 最终完成文件和本地校验，但前序 2 次 Exec 结构错误、
> 1 次提前登记、1 次漏登和 2 次 Map 拒绝耗尽每-run 12 requests 硬上限；前 12 个 Provider 请求均返回 200，第 13 个收尾
> 请求被本地预算代理以 429 拒绝，Map 未闭合。第 1 点不能晋升为在线通过；第 4 点随后以确定性复合门禁完成，
> 第 2 点按单变量计划继续，I03 仍为 verifying。详见
> [`taskspace-exec/51-hosted-error-priority-stage1-result.md`](taskspace-exec/51-hosted-error-priority-stage1-result.md)。

> **Hosted 错误优先级确定性门禁（2026-08-14）**：按用户决定提前执行原计划第 4 点。新增复合用例在同一合法输入中
> 同时构造“当前响应真实 `web_search` 漏登归属”和“client Tool 绑定 waiting 节点”，确定性证明 Runtime 首先返回
> `HostedToolSetMismatch`，且 Map 与 client Tool 均无副作用；现有 handler 合同已覆盖该枚举到 Agent 可见事实反馈的映射。
> 该门禁补齐了上一轮真实 trace 未自然命中目标分支的证据缺口，但不证明 Agent 在线行为改善，不关闭 I03，也未消耗真实运行预算。

> **首轮 client/Hosted 合并示例第 2 阶段（2026-08-14）**：提交 `7f7a1e7b9` 将首次 client work 与
> `already_executed` 归属合并进同一个 `initialize_and_work` 示例，并删除独立 Hosted JSON，Tool description 反而减少
> 126 bytes，离线 75/75 通过。真实 `repeat=3` 在首轮失败后停止：Agent 同时生成 Exec 登记和携带 `queries` 的顶层
> `function_call(name=web_search)`，而不是 Provider 原生 `web_search_call`。Runtime 正确以顶层 client Tool 逃逸拒绝，
> Map/client 均零副作用。该“完整示例”只能展示双写的一半，形成结构性误导，候选不晋升缓存基线且不进入第 3 点。
> 详见 [`taskspace-exec/52-hosted-first-turn-example-stage2-result.md`](taskspace-exec/52-hosted-first-turn-example-stage2-result.md)。

> **Hosted 原生调用合同纠正（2026-08-14）**：提交 `b77663e43` 删除首轮 Hosted 预填示例和“Agent 主动 emit
> 原生 Provider item”的错误指令，并明确 `web_search` 是原生 ToolSpec、不是 Function Tool；`already_executed` 只记录
> 当前原生结果的节点归属。真实 `repeat=1` 中不再出现顶层 `FunctionCall(name=web_search)`。第一响应的原生
> `search + open_page` 正确聚合为一个逻辑 `web_search`，但 Exec 漏登后整批零副作用拒绝；第二响应原生搜索与一条
> `already_executed` 归属同响应出现并成功对账，随后业务、验证和 5 节点 Map 全部闭合。明确误导已修复，但首次双写仍不稳定，
> I03 继续 verifying，候选不晋升缓存基线。详见
> [`taskspace-exec/53-hosted-native-contract-fix-result.md`](taskspace-exec/53-hosted-native-contract-fix-result.md)。

> **Provider 待归属队列真实验收（2026-08-14）**：`provider-web-search-probe × map-request × repeat=3` 中，6 个
> Provider Action 均在后续请求由 Agent 按稳定 ID 归属并原子出队，三轮结束时 pending 均为 0；没有同响应双写、漏绑、
> 错绑或默认 Root。业务和 Map 闭合 2/3；第三轮在写文件后耗尽 12 请求，尚未校验和 finish。Trace 同时坐实 active base
> instructions 残留旧同响应双写文字，提交 `ce69cbb13` 已离线删除；性能观察器的 OutputReference 误报由 `4584ad05d` 修复。
> 当前 I03 的 Provider 归属子问题已通过，整体仍 verifying。PA-08 已按完整 Provider 请求统一检查 Provider facts 与 Exec
> client actions，离线 77/77 通过；真实 `repeat=1` 的第 4 个请求已同时出现原生 `web_search` 和 Exec，但 Agent 仍主动加入
> `pwd`。前三个真正空的初始化请求正确拒绝，后续还出现一次纯 pending 归属序列误选和一次 Waiting 节点拒绝，最终在
> 12 requests 内未完成业务。因此不能把失败归因于 Provider/Exec 不共现，也不能宣称 Provider-only 分支已在线通过。详见
> [`taskspace-exec/56-response-level-work-validation-result.md`](taskspace-exec/56-response-level-work-validation-result.md)。
> 详见 [`taskspace-exec/55-pending-provider-attribution-live-result.md`](taskspace-exec/55-pending-provider-attribution-live-result.md)。

> **Provider 归属路线撤销（2026-08-15）**：用户明确当前阶段不再要求 Agent 管理 Provider-hosted Tool。active code 删除
> 同响应双写、跨响应 pending Store、上下文暴露、Exec 归属字段及结束硬门。Runtime 只在真实调用发生后按原生 Tool 名，
> 在 Root 下按需建立 Completed 聚合节点并追加机械 Action；无 Map 或同名 Agent 节点冲突时允许 escape 并记录诊断。
> 因此 I03 不再包含“Agent 稳定完成 Provider 归属”这一验收目标，旧 PA 结果仅为历史证据。

> **initialize_map 参数类型五轮复验（2026-08-15）**：同一 subject、二进制、模型、sample 与 map-request 合同下，首个
> `initialize_map` 为 object 4/5、JSON string 1/5；唯一错误在下一请求改回 object 并成功初始化。原始 Provider Function
> Call 已包含 string，Runtime 只做零副作用类型拒绝，没有扭曲或补写。五轮首请求 Tool schema、Base Instructions、system
> section、cache shape、tool_choice 和 payload bytes 相同，排除轮间合同切换；schema 体积或嵌套深度仍只是待检验风险因素。
> 五轮代码修复、公开验证、隐藏 oracle 和 Map 闭合均通过，Agent 生命周期 3/5 complete；另两轮在第 10 请求完成 Map 后因
> 无第 11 请求生成最终回复而 interrupted。I03 保持 verifying。详见
> [`taskspace-exec/58-initialize-map-type-repeat5-result.md`](taskspace-exec/58-initialize-map-type-repeat5-result.md)。

> **initialize_map 四候选单变量实验（2026-08-16）**：C1 准确反馈 5 轮未自然触发 string，不能评价恢复收益；C2 将
> `initialize_map` schema 内联后 5/5 首发 object、合法序列、Agent complete 和外部验证通过，但未改 schema 的 C1 同样
> 0/5 string，因果仍不足；C3 删除完整首次示例后虽然 5/5 object，却只有 2/5 首次合法初始化，明确回归；C4 只打开
> DeepSeek strict 时在聚焦 Catalog 中至少发现 7 个合同不兼容点，未启动真实运行。I03 继续 verifying，当前代码停留在 C2 候选，尚未晋升缓存
> 基线。详见 [`taskspace-exec/63-initialize-map-candidate-comparison-report.md`](taskspace-exec/63-initialize-map-candidate-comparison-report.md)。

> **C2 十轮扩大复验（2026-08-16）**：当前 C2 在 10 个独立 `map-request` 观测中首发 `initialize_map` 为
> object 10/10；连同原五轮累计为 15/15。新批次只有 6/10 首请求同时携带 work，另 4 次被零副作用拒绝后下一请求修正；
> 9/10 业务和外部验证通过。唯一失败是在成功初始化后把 `exec_command` 提升为未暴露的顶层 client Tool，与 C1 的唯一失败
> 同形，继续归 I03。Request 2+ 加权缓存命中率 92.71%，没有 Tool shape 或 `tool_choice` 切换。C2 保留，H-003 与 I03
> 仍为 verifying，不把累计 0/15 string 解释为因果坐实。详见
> [`taskspace-exec/64-initialize-map-candidate2-repeat10-result.md`](taskspace-exec/64-initialize-map-candidate2-repeat10-result.md)。

> **client work 结构前置条件恢复（2026-08-16）**：追溯确认 `e4e7fc874` 将工作序列的非空 `tools[]` 放宽为
> Provider/client 响应级 OR；`682164844` 随后撤销 Provider 的 Agent 归属，却没有同步撤销该 OR gate，导致 C2 十轮中
> 4/10 首次只建 Map、不做 client work。当前实现已恢复四类工作序列必须声明非空 client `tools[]`，删除
> `has_provider_work` 对 Exec 合法性的替代作用，同时保留 Provider Root 机械归纳。聚焦离线测试 67/67 通过；真实 Agent 与
> 缓存验收尚未执行。详见
> [`taskspace-exec/65-client-work-structural-restoration.md`](taskspace-exec/65-client-work-structural-restoration.md)。

> **client work 恢复真实复验（2026-08-16）**：`single-file-fast-fix × map-request × repeat=3` 中，三轮首请求均一次生成并
> 执行 `initialize_and_work + client exec_command`，Map-only 空初始化 0/3；Agent complete、业务、公开验证、隐藏 oracle
> 和 Map 闭环均 3/3。共 23 requests、338,069 input、292,608 cached、6,817 output，Request 2+ 加权缓存命中 93.09%。
> Run 1 仍有 waiting 节点误选和一次 JSON parse 拒绝，Run 3 仍有两次冗余 `in_flight` 转换拒绝；均零副作用后修正。
> 因此本次恢复子问题在线通过，I03 因其他动作组织异常继续 verifying。详见
> [`taskspace-exec/66-client-work-restoration-repeat3-result.md`](taskspace-exec/66-client-work-restoration-repeat3-result.md)。

> **repeat=3 独立异常登记（2026-08-16）**：本轮把残余拒绝拆成三个稳定观测项，不新增重复的全局问题编号：
> `I03-ARG-SYNTAX` 为 Run 1 一次 Function arguments JSON 分隔错误；Runtime 准确返回 syntax、零副作用，下一请求纠正，
> 因此它不是 I05 反馈分类回归。`I04-FRONTIER-EARLY` 为 Run 1 一次在父节点未完成时选择 Waiting 子节点；
> `I04-REDUNDANT-INFLIGHT` 为 Run 3 两次同时提交 Tool 和显式 `in_flight`，与 Runtime 的 Ready -> InFlight 机械启动重复。
> 后两者反馈均准确且下一请求纠正，但分别浪费一次请求。详见
> [`taskspace-exec/67-repeat3-independent-anomaly-register.md`](taskspace-exec/67-repeat3-independent-anomaly-register.md)。

> **client work 恢复十轮扩大复验（2026-08-16）**：十轮首请求均一次生成并执行
> `initialize_and_work + client exec_command`，Map-only 空初始化和顶层 client Tool 逃逸均为 0/10；Agent complete、业务、
> 外部验证和 5 节点 Map 闭环均 10/10。共 75 requests、1,105,478 input、1,027,840 cached、21,917 output，Request 2+
> 加权缓存命中 92.21%，无 Provider retry、zero-cache 或 Tool shape 切换。独立异常为 `I03-ARG-SYNTAX` 1 次、
> `I04-FRONTIER-EARLY` 4 次、`I04-REDUNDANT-INFLIGHT` 2 次；均在副作用前准确拒绝并由 Agent 修正。工作序列非空 client work
> 子问题通过扩大验收，I03/I04 因可重复的独立异常继续 verifying。详见
> [`taskspace-exec/68-client-work-restoration-repeat10-result.md`](taskspace-exec/68-client-work-restoration-repeat10-result.md)。

> **owner state 反馈单变量复验（2026-08-16）**：候选在每条 client Tool 成功结果中机械返回 canonical owner state，不改
> schema 分支、Base、DAG 或拒绝逻辑。五轮中四轮到达 patch-to-verify 边界，Run 1/3 即使逐字收到
> `owner_state_after=in_flight`，仍生成 `work@verify`，frontier 误选为 2/4，未优于 4/10 基线；因此状态省略不是充分根因，
> 候选未晋升并已回退。Run 5 独立复发一次未声明顶层 `exec_command`，被 response contract 在执行前终止，继续归 I03。
> 详见 [`taskspace-exec/69-owner-state-feedback-repeat5-result.md`](taskspace-exec/69-owner-state-feedback-repeat5-result.md)。

> **I03 单闭合符号自愈边界补全（2026-08-17）**：历史 Run 发现一次 `apply_patch` action 尾部多出 `}`；删除任一相邻
> 多余符号得到同一个合法参数，且与 Agent 下一请求的成功重试逐字一致。SR-01 已从“只插入一个闭合符号”收敛为“插入或删除
> 一个 `}`/`]` 的统一候选集”，仍要求唯一候选通过当前 Catalog 完整解码。历史 13 次 syntax reject 复扫还发现一次同类
> 多余 `]`；复合字符串转义错误和多个结构错误均不满足唯一机械修复条件，保持原样拒绝。该修复缩短明确序列化小错的恢复路径，
> 不代表 I03 的 Agent 参数稳定性整体关闭。详见
> [`taskspace-exec/75-single-closing-delimiter-self-heal-result.md`](taskspace-exec/75-single-closing-delimiter-self-heal-result.md)。

> **单闭合符号自愈五轮复验（2026-08-17）**：`single-file-fast-fix × map-request × repeat=5` 全部完成业务、公开验证、
> 隐藏 oracle 和 5 节点 Map 闭合。Run 4 自然生成缺少一个 `}` 的参数，Runtime 在落账前插入后于同一请求执行 patch；正式
> rollout 摘要与修复摘要一致，错误版未进入历史。五轮共 34 requests、528,450 input、491,136 cached、37,314 uncached、
> 11,356 output，零 syntax reject、协议/状态拒绝或重试。新增的“删除多余闭合符号”分支未自然触发，继续以历史坏例和确定性
> 测试为证；I03 整体仍保持 verifying。详见
> [`taskspace-exec/76-single-closing-delimiter-self-heal-repeat5-result.md`](taskspace-exec/76-single-closing-delimiter-self-heal-repeat5-result.md)。

> **I03/I04 IC-09 异常离线修复（2026-08-17）**：自愈器新增唯一裸 LF 转义候选，继续要求当前 Catalog 完整解码、全局唯一
> 修复并在历史落账前替换；多个裸换行和复合错误保持拒绝。TaskSpace Base 升级为 `3.0.5`，明确父节点先完成、Runtime 再
> 派生依赖节点 Ready，并允许同响应提交刚解锁子节点的 Tool，不允许 Agent 显式把 waiting 子节点改为 `in_flight`。没有修改
> Tool schema、状态机或拒绝语义。TaskSpace Exec 74 项、Base 3 项和正式历史替换测试通过；尚未执行真实 Agent 复验，I03/I04
> 均保持 verifying。详见 [`I08/08-ic09-feedback-compaction-repeat5-result.md`](I08/08-ic09-feedback-compaction-repeat5-result.md)。

> **R8-E3 当前生产版本双臂 Repeat 3（2026-08-17）**：Standard 3/3 完成，TaskSpace 2/3 完成。失败轮首次
> `initialize_and_work` 正常，第二响应却生成被禁止的顶层 `exec_command`；Runtime 在零副作用下阻止执行，但以 Fatal
> 终止会话，没有给 Agent 纠正机会。另一轮 `update_and_finish` 同时出现缺闭合符和字段误嵌套，超出唯一机械自愈边界，
> 下一请求自行纠正。两次到达交接的 TaskSpace 运行均无 Waiting frontier 错误。34 个请求身份完整、无 retry/duplicate，
> 两次成功 TaskSpace Patch 均为 declaration/result 1/1；同时暴露 observer 错报 Map 已闭合、usage 展示口径不一致及 runner
> 未执行声明停止条件。I03 明确未通过，I05/I07 保持开放，I04 获得有限正向证据；成功 Pair 的 TaskSpace 请求数/input/
> 平均每请求 input 分别为 Standard 的 `1.17x/1.40x/1.20x`。详见
> [`I08/10-r8-e3-current-production-repeat3-result.md`](I08/10-r8-e3-current-production-repeat3-result.md)。
> `R8-E3` 为实验标签，runner 正式 evidence target 是 E2，不能替代 I01/I07 关闭合同要求的正式 E3。

> **I05/I07 离线收敛（2026-08-18）**：`e596d2f27` 在原生参数解析前把 forbidden 顶层 client Tool 逃逸转换为
> 同 `call_id`、零执行、可继续请求的正式失败反馈；多 Exec、响应身份与 Map 快照等完整性错误仍保持 Fatal，同响应已完成
> Hosted action 不丢失。`7a4346156` 删除 R7 observer 内平行的 terminal/usage 解析，生产报告统一消费 canonical
> `request-facts.json`，raw wire 只提供独有 shape/LCP 元数据，重复 wire attempt/terminal fail closed。TaskSpace Exec 77/77、
> request-fact 22/22 及完整本地报告链通过；尚未运行 Whale Agent，两项继续 `verifying`。

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
| 2 | R8-I01 | F1 | P0 | 一轮工作后 Agent 可能收到互相竞争的新旧进度 | 每轮只有一个可继续使用的结果，revision 不成为 Agent 填表负担 | 当前双臂 34 个请求无 retry/duplicate；旧双版本链仍为零。只覆盖 `map-request`，三 projection 验收未完成 | [verifying](I01/00-i01-response-final-revision-repair-plan.md) | GI-001 |
| 3 | R8-I06 | F2 | P0 | 组合工具内部动作可能绕过归属和单 Patch 硬门 | 所有 TaskSpace client 动作先过同一请求级预检，普通 Tool 保持原生 | 顶层 client 旁路在零副作用边界拒绝；完整计划统一预检；成功路径每请求最多一个 Patch 并走原生 Router | [closed](taskspace-exec/79-i06-tool-boundary-closure.md) | GI-006 |
| 4 | R8-I05 | F3 | P1 | 拒绝原因可能重复或混淆错误发生的协议层级 | 忠实返回一次失败，并准确区分语法、合同、预检与执行错误；未提交候选不得表现为已保存状态 | `e596d2f27` 完成同 `call_id`、零执行、可继续反馈；repeat=3 正常路径无回归，但未自然触发逃逸恢复分支 | verifying | GI-005 |
| 5 | R8-I02 | F3 | P1 | Tool 事实可能被另造高优先级消息重复包装 | 原 Tool/outer Tool 反馈只进入上下文一次，不建立 system/developer 副本 | 成功、拒绝和内部失败只返回一个 outer output；最新三次生产运行 `18 calls = 18 outputs`，无副本或 orphan | [closed](taskspace-exec/80-i02-single-feedback-closure.md) | GI-002 |
| 6 | R8-I10 | F4 | P1 | 工具能力变化没有跨执行、缓存和报告共用的身份 | 实际工具集合变化才切换身份，各消费面引用同一值 | 同一 Catalog 身份沿 dispatch/request/wire/trace/report 传播；最新 21 个 TaskSpace wire 请求身份一致且无冲突 | [closed](I10/01-i10-capability-identity-closure.md) | GI-010 |
| 7 | R8-I07 | F4 | P1 | 观察工具可能漏计、重复计数或使用过期证据 | 请求和失败逐身份计一次；协议拒绝与证据损坏分开表达，身份不一致时才不可比较 | 真实矩阵 41 logical/boundary/completed/usage 完全一致，无孤儿、重复、重试或 finding；Map、停止参数和账本同链可复算 | [closed](I07/01-i07-independent-repair-result.md) | GI-007 |
| 8 | R8-I03 | F5 | P2 | Agent 不能稳定组织 Map 与 client 动作 | 稳定生成初始化并执行、完成并继续、完成并结束；Provider-hosted Tool 当前不参与 Agent 归属协议 | 最新简单样本 repeat=3 全部完成且无 escape/JSON reject；历史异常未在更复杂动作面复验，继续观察 | verifying | GI-003 |
| 9 | R8-I04 | F5 | P2 | Agent 可能选择依赖未满足或已完成的节点 | Agent 准确使用可执行 frontier；Runtime 只守硬规则 | 最新 3 次 TaskSpace 均无 Waiting/frontier 拒绝，全部节点闭合；简单样本正向证据不足以外推关闭 | verifying | GI-004 |
| 10 | R8-I08 | F6 | P3 | TaskSpace 的请求、输入、时间和未缓存成本可能高于 Standard | 额外成本可解释、稳定并与产品收益匹配 | 最新 repeat=3 的请求/input/平均每请求 input/Agent wall 为 `1.05x/1.32x/1.25x/1.33x`；无异常重试，复杂样本外推未执行 | [investigating](I08/10-r8-e3-current-production-repeat3-result.md) | GI-008 |

问题总数：**10**；Open：**5**；Closed：**5**。当前停点：**Provider-hosted Runtime 机械归纳的生产验收预算**。
I07 已由 41 个真实请求关闭；I05 的 Fatal 恢复缺口已离线修复且正常路径无回归，不在未实现清单中；I08 的小型等价压缩按用户决定暂缓，
不回删 Map、合法序列或状态机硬合同。

## 4. VA-04A 证据边界

| 分类 | 问题 | 当前可下结论 | 当前不能下结论 |
|---|---|---|---|
| 确定性关闭 | I09 | 关系 Store hydrate 仍拒绝非法图，State 134 项通过 | 无 |
| 静态关闭候选 | I01 | 旧双版本链为零，新 Exec 内部 revision 有确定性测试 | 三 projection 的最终一致性尚未验收 |
| 已关闭 | I02、I06 | 单次 outer 反馈、统一预检、零副作用旁路拒绝和单 Patch 边界均有确定性与生产证据 | Agent 仍可能生成非法动作，但不再构成底层边界缺口 |
| 工程修复、正常路径已验收 | I05 | JSON/schema reject 与 forbidden 顶层 client Tool 均有准确、同调用身份、零副作用反馈；repeat=3 正常路径全部通过 | 本轮未自然触发顶层 client escape，尚无新的恢复分支在线命中证据 |
| 已关闭 | I10 | 最新 21 个 TaskSpace wire 请求身份一致；Catalog、dispatch、wire、trace 与 observer 共用同一值 | Projection 不参与能力身份计算，其三臂验收归入 I01/I08 |
| 已关闭 | I07 | 41 个真实请求的 logical/boundary/completed/usage 完全一致；Map 完成、显式停止参数、实际 artifact 与账本结算同链可复算 | 本轮无失败，未刻意在线触发停止分支；该分支由确定性 runner 测试覆盖 |
| 当前简单样本稳定 | I03 | 最新 repeat=3 全部完成 Map 初始化、工作、验证和 Finish，无 escape、JSON/schema reject 或重试 | 尚未用复杂动作样本证明历史不稳定已普遍消失 |
| 当前行为已观察 | I04 | 最新 3 次 TaskSpace 均无 Waiting/frontier 拒绝，全部工作节点闭合 | 简单线性样本不足以覆盖复杂依赖 frontier |
| 成本已定位、机械反馈已收敛 | I08 | 最新 repeat=3 的请求/input/平均每请求 input/Agent wall 为 `1.05x/1.32x/1.25x/1.33x`，无异常重试 | 复杂样本外推未执行；TaskSpace 未缓存 input 仍明显高于 Standard |

本轮 B4 证据为：TaskSpace Exec 57、settlement/recovery 11、State 134、Core 1856/3、CLI 5、Viewer 4、App Server
Protocol 183、workspace、zero-base 和 cache gate 全部通过，详见
[`22-phase-b4-offline-acceptance.md`](taskspace-exec/22-phase-b4-offline-acceptance.md)。OB-01/OB-02 的身份链和报告消费见
[`19-phase-b4-observability-audit.md`](taskspace-exec/19-phase-b4-observability-audit.md) 与
[`21-phase-b4-performance-observer-result.md`](taskspace-exec/21-phase-b4-performance-observer-result.md)。完整重映射结论见
[`23-phase-b4-issue-remap-result.md`](taskspace-exec/23-phase-b4-issue-remap-result.md)。

I10 后续离线补证为 TaskSpace Exec、Core、workspace、zero-base、性能观察 fixture 和缓存门禁通过，生产 trace 的 21 个
TaskSpace wire 请求身份一致，见 [`I10 关闭结算`](I10/01-i10-capability-identity-closure.md)。

旧 control/sibling 真实运行、旧 developer carrier 缓存结果和旧请求放大数字只保留在各专题历史文档中，不再作为当前
问题的产品证据。VA-04B 只使用最终生产入口的获批 trace 更新状态。

## 5. 依赖与重评关系

| 上游问题 | 关闭后必须重评 | 原因 |
|---|---|---|
| I09 旧任务恢复可信性 | I01、I04 | 先确保恢复出的任务地图可信，才有资格评价后续进度版本和节点选择 |
| I01/I05 | I03、I04、I08 | 后续复杂 trace 同时确认旧根因未复现，再评价 Agent 行为与成本 |
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
