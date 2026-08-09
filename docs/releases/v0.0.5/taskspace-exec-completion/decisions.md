# TaskSpace Exec 产品决策基线

> PROTECTED USER-AUTHORITY ARTIFACT
> 本文件中的决策不得由 Agent 自行创建、修改、删除、重释或替代。
> 任何具体决策变化都必须获得用户明确批准；实现、测试、审查、既有文档或用户未反对均不构成批准。

- Authority: User
- Write Gate: Explicit user approval required
- Agent Self-Approval: Forbidden
- Release Version: v0.0.5
- Topic: TaskSpace Exec 完整协议与生产闭环
- Plan: ./plan.md

| ID | Confirmed Decision | Must Do | Must Not Do | Rationale | Violation Signal | Confirmation | Status |
|---|---|---|---|---|---|---|---|
| D1 | TaskSpace 使用 Function Call 形态的单一顶层 `taskspace_exec`；普通 client Tool 和 Map 操作都由 Agent 在其结构化参数内声明。 | TaskSpace 请求只暴露一个 client/map 入口；Runtime 解析后交回原生执行路径。 | 不把它退化成 sibling manifest、独立序列声明或自然语言容器。 | 在 Provider 固定顶层 Tool Call 结构内前置约束合法批次。 | TaskSpace 顶层再次出现普通 client Tool，或 Runtime 事后拼装 Agent 未声明的批次。 | user-confirmed-direct: “建立参考codex exec…但类型为function call 的超级工具，其他工具都通过该工具内部声明给agent” | active |
| D2 | `taskspace_exec` 只承担合法序列和节点绑定；Agent 决定动作、参数、节点和 Map 推进。 | Runtime 只做结构解析、硬规则预检和机械执行。 | 不推断、补全、重排、修复或语义判断 Agent 动作。 | 状态机是 Agent 支配但不可绕过的工具，只负责底线。 | Runtime 自动选择节点、补动作、重写参数或按任务语义拒绝。 | user-confirmed-direct: “exec 承担这两个主要任务（1）taskspace 合法序列（2）node绑定” | active |
| D3 | 普通 Tool 的原生结构和执行合同对 TaskSpace 无感。 | 从同一原生 Tool 定义机械派生内层能力；节点归属位于外层调用 metadata。 | 不给普通 Tool schema 增加 `node_id`，不分叉 handler、权限、sandbox、hook 或结果语义。 | 保持行业标准、Standard 共用和低耦合。 | 普通 Tool 参数中出现 TaskSpace 字段，或 TaskSpace 使用专属 Tool handler。 | user-confirmed-direct: “tool call 执行本身对node_id 完全无感” | active |
| D4 | `taskspace_control`/Map 操作在结构和执行地位上与普通 Tool 平级，只依赖 Exec 外层合同保证边界。 | Map 操作作为 `calls[]` 中的普通内部 variant 暴露和执行。 | 不建设独立 control 通道、特殊 Agent 动作层或高于 Tool 的语义层。 | Map 本身就是状态机，不是 Runtime 编排器。 | Map 操作绕过 Exec，或 Runtime 用 Map 控制 Agent 的上限行为。 | user-confirmed-direct: “taskspace_control 本身也是普通的tool…在结构和实现方式上不应与普通tool有本质差异” | active |
| D5 | Exec 的顺序合同只保证 Map 边界；普通 work calls 之间的依赖由 Map DAG 表达。 | 校验 initialize/reopen、finish、complete+next 等边界位置；无结果依赖的 work 可并行。 | 不把 `calls[]` 数组顺序解释成任意 client Tool 的业务依赖或强制串行。 | 避免重复表达依赖并保留低延迟并行。 | Runtime 因 B 在 C 前而推断 B→C，或忽略 Map DAG 另建序列状态机。 | user-confirmed-direct: “tools容器其实无需声明或保障 B-->C，只需声明A在第一个，B/C由map中的节点依赖声明” | active |
| D6 | Provider-hosted Tool 保持 Provider 原生执行；Agent 在同一响应的 Exec 中声明其节点归属。 | Runtime 用真实 Provider item 身份和顺序逐项核对，允许一个 hosted fact 归属多个 Work node。 | 不重执行 hosted Tool，不按内容猜配，不因 Tool 完成而自动推进节点。 | 已发生的 Provider 事实与 Agent 声明的 Map 归属必须同时保真。 | hosted 结果被重跑、默认归 Root、按文本相似度绑定或改变节点生命周期。 | user-confirmed-direct: “执行后的 provider tools 及其绑定关系…runtime…双写进行校验（错绑、漏绑）”及“一项…多个节点都用到了”澄清 | active |
| D7 | 未绑定、漏绑、错绑不是可接受的降级状态。 | 在无法唯一机械核对时拒绝 TaskSpace 响应，保留 Provider 原始事实作为失败证据。 | 不以 `unbound`、默认 Root 或默认节点继续落 Map。 | Map 中 action 归属必须来自 Agent 明确声明。 | 响应在归属缺失时仍被接受或自动选择 owner。 | user-confirmed-direct: “未绑定属于不接受的错误情况” | active |
| D8 | 连续动作是明确产品能力。 | initialize/reopen 与真实工作同批；非终态完成应携带后续工作；一次响应可执行多个无结果依赖动作。 | 不允许单独非终态 complete 成为正常路径，不因修复其他问题退化为每次只做一个动作。 | 减少无效 Provider request，并保持 Map 连续推进。 | 单独 complete 回归，或 TaskSpace 无理由把可并行动作拆成多轮。 | user-confirmed-direct: “连续动作这个设计是有明确收益的，要严格保障实施” | active |
| D9 | 一个 Agent response 最多实际执行一个 Patch。 | 在任何副作用前对完整 Exec 批次执行硬预检。 | 不以执行后惩罚替代预检，不侵入 Patch 原生 freeform 合同。 | 防止连续 Patch 破坏开发规范，同时保持 Tool 原生能力。 | 同批执行多个 Patch，或 Patch schema 被 TaskSpace 改写。 | user-confirmed-direct: “需要做工具层面的升级”及后续 one-patch 约束确认 | active |
| D10 | 内层能力、模型可见合同、Runtime dispatch 必须共用一个确定性事实源。 | 从原生 ToolSpec/Registry 的同一有效快照生成输入、结果、命名和身份；相同能力集合生成稳定声明。 | 不维护第二份手写 Tool 清单，不重复暴露顶层与内层 schema，不把运行时 Map 数据写进 schema。 | 避免协议漂移、缓存破坏和“提示词说一套、Runtime 做一套”。 | 能力只在 prompt/catalog/router 之一更新，或 schema 随 revision/node 变化。 | user-confirmed-direct: “尽可能的共享基建工具…避免架构分叉”及“schema本来就是静态的” | active |
| D11 | Tool 反馈和上下文必须忠实保留语义；TaskSpace 不建设额外结果引用体系。 | 内层结果复用 Standard 的原生结果转换、裁剪、压缩和持久化；Map 只存必要 action 事实和 Agent 写入内容。 | 不把原生成功变失败、丢失输出、注入建议、重新摘要或复制到 Map 专属 ref。 | Agent 低级错误首先应排查语义传递，而不是增加 Runtime 控制。 | Agent 看不到真实结果，或 TaskSpace outer envelope 用传输细节替代原生结果。 | user-confirmed-direct: “语义透传…不得扭曲、注入、再解释、重要上下文裁剪丢失” | active |
| D12 | 旧入侵、容器和 sibling 方案不与 TaskSpace Exec 双轨保留。 | 不符合新方案、失效或无生产消费者的代码直接删除。 | 不加兼容、adapter、fallback、改名残留或 dormant 分支。 | 实验产品无数据兼容负担，旧抽象会持续污染实现。 | 同一能力存在新旧 parser/schema/handler，或用“暂留”保留死代码。 | user-confirmed-direct: “大胆拆掉旧方案的影响…对标从0做起” | active |
| D13 | 正式实施前必须完整参考最新 Codex Exec 链路并先把方案定清楚。 | 明确可复用职责、不适用机制、当前缺口、验证和停止条件后再改生产代码。 | 不只复制一段 description，不盲目照搬 JavaScript/V8/host 特性，也不在方案不清晰时继续实现。 | Codex 的收益来自注册、暴露、分发、结果和模型能力闭环，不是单点提示词。 | 未完成职责映射即进入生产改动，或照搬与结构化 Function Exec 无关的 runtime。 | user-confirmed-direct: “先把方案进行完整的参考codex完善，不要在不清晰的方案上盲目执行” | active |
