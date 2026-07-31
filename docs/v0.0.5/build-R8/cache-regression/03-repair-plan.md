# 缓存命中回归门禁修复计划

- Created: 2026-07-31
- Plan mode: Authoring
- Plan status: in progress（CR-01 至 CR-06 completed）
- Risk: High，涉及发布门、付费验证触发与证据可信性
- Problem register: [02-known-issues.md](02-known-issues.md)

## 1. 问题与目标

当前门禁以手工 glob 的原始源码 SHA 代替最终 DeepSeek 请求语义，既漏掉真实构造入口，又会因注释和测试变化
误触发付费回归；同时存在 bootstrap 通过发布、index/worktree 混读和结果晋升证据不足等控制面缺陷。

目标是建立三段式门禁：

1. **免费源码风险哨兵**只决定是否运行更深的免费检查；
2. **免费生产 final-wire 场景矩阵**判断 provider 实际可见请求或缓存测量合同是否变化；
3. **获批真实回归**只验证受影响的场景，并把结果绑定到精确源码身份。

非目标：本计划不修复 map-request 当前 `35.79%` 的产品缓存退化，不改变 TaskSpace 语义，不建立第二套请求
serializer，不自动调用真实模型，也不承诺抵抗拥有仓库写权限的恶意维护者。

## 2. 目标控制流

```text
HEAD / index / worktree 中出现风险面变化
  -> source sentinel 计算风险类别
  -> 复用生产 Session -> Prompt -> ResponsesApiRequest -> DeepSeek body 链路
  -> 对同一确定性场景生成 request 1 / request 2 final-wire 快照
  -> 比较消息角色、顺序、内容、Tool schema/order、tool_choice、model/provider identity
  -> 未变：免费通过
  -> 已变：阻断，输出场景、首差异、旧/新摘要和建议的最小真实运行矩阵
  -> 用户批准预算后运行真实 provider
  -> 校验业务结果、request 2+ usage、证据摘要和源码身份
  -> 仅晋升本次实际覆盖的场景
```

final-wire fixture 通过本地 mock endpoint 捕获生产 HTTP body。动态路径、时间和 ID 优先在 fixture 输入端固定；不得
用宽泛文本替换掩盖差异。JSON 可生成便于审阅的稳定表示，但数组顺序和字符串内容必须原样保留；同时保留生产
serializer 产生的原始 body SHA，防止可读快照遗漏真实 wire 变化。

## 3. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|
| CR-01 | 发布只接受真实基线 | release policy | `scripts/cache-regression/check_cache_regression_gate.py` | `live_status_accepted` | 删除 `structural_bootstrap` 的 release 接受分支 | `--require-live-baseline` 只接受 `live_verified` | 防止没有付费证据的版本被误放行 | 新增 bootstrap rejection fixture；现有 failed baseline 继续失败 | 单独回退该提交；发布仍保持阻断 | planned |
| CR-02 | 每种 source 使用同一快照合同 | source identity | `cache_surface.py`、`check_cache_regression_gate.py` | contract loader 与 `surface_snapshot` | 为 HEAD/index/worktree 分别从对应 Git 对象读取合同和内容；控制面文件 index/worktree 不一致时拒绝 | staged 检查不再混读 worktree，也不会执行一份规则却提交另一份 | 开发者提交的就是实际被检查的内容 | 复现旧 partial-stage case 必须失败；三 source fixture | 若失败，保留 CR-01 并恢复保守阻断 | planned |
| CR-03 | 控制面变更不能静默自授权 | control-plane policy | `.githooks/pre-commit`、`scripts/cache-regression/**`、non-agent gate、contract | policy-change classifier | 由变更前 subject 识别 hook、checker、contract、promoter 和结果 schema 变化，要求独立 policy-change 证据，禁止同一证据同时改验收规则并晋升基线 | 门禁自身变化被显式标记并与性能晋升分离 | 防止日常误操作让规则修改自行放行 | 对每类控制面文件做 staged mutation；同批 promotion 必须失败 | 远端 trusted-base 未确认前 release 保持 fail closed | planned |
| CR-04 | 晋升结果不可缺字段或扩大范围 | evidence integrity | `promote_cache_baseline.py`、result JSON schema | promotion validator | 强制实际 arms、repeat、sample 数和阈值与获批运行计划完全一致，并校验 trace coverage、subject SHA、surface/payload digest 和证据摘要 | 手工残缺、超计划或错配结果不能晋升 | 避免错误证据被当成性能基线，同时不把当前 2-sample 规模固化成长期架构 | fabricated、缺 arm、实际规模偏离计划、摘要不符 fixture 均失败 | 晋升保持禁用，不影响免费诊断 | planned |
| CR-05 | 发布证据绑定精确 commit | release identity | `build-v005-non-agent-gates.ps1`、gate result schema | release source selector | release 只检查显式 commit tree；相关 dirty/untracked 文件直接拒绝，结果记录同一 SHA | 检查对象、构建对象和证据对象一致 | 任何缓存结论都可复算到唯一源码 | exact-commit、dirty relevant、untracked relevant fixtures | 无法确定身份时 fail closed，不运行 API | planned |
| CR-06 | 找到生产 final-wire 捕获边界 | discovery | `core/src/session/turn.rs`、`core/src/client.rs`、`codex-api/src/endpoint/*.rs` | `Prompt` 到 HTTP body 的调用链 | 绘制并用测试证明 DeepSeek 请求从上下文、Tool 选择到 serializer 的唯一生产路径 | 后续 fixture 有明确权威入口 | 避免另建与生产漂移的测试 serializer | 调用链文档、函数引用和一个本地 mock 捕获 spike | 只提交调查与测试 spike；未证明前不进入 CR-07 | completed（`d04aab5fb`） |
| CR-07 | 保存可复算的 final-wire 证据 | cache contract | `core/tests/common/responses.rs`、新 `core/tests/common/cache_payload.rs` | mock request capture | 同时输出原始 body SHA 和结构化快照；只固定 fixture 输入，不宽泛重写输出 | 字节变化与可读语义差异均可定位 | 缓存变化不再靠源码路径猜测 | 相同输入重复执行摘要一致；字段/数组顺序突变可被 fixture 捕获 | 保留现有 v1 gate，新增能力未接门禁 | planned |
| CR-08 | 明确哪些差异需要付费验证 | comparison policy | `benchmarks/cache-regression/` 新 payload contract | cache-relevant fields | 固化消息角色/顺序/内容、Tool schema/order、`tool_choice`、model/provider identity 的比较规则；每个允许忽略字段逐项说明 | 门禁区分真实请求变化与确定性噪声 | 降低误报且不牺牲语义保真 | mutation tests 对每个受保护字段均报警；忽略字段反例测试 | 默认不忽略未知字段，规则不明时保守阻断 | planned |
| CR-09 | Tool schema 使用生产 serializer | tool wire | `tools/src/tool_spec.rs`、`core/tests/suite/cache_payload_contract.rs` | provider-visible tools array | 从实际 Session 请求捕获普通 Tool 和 `taskspace_control` 的名称、顺序、描述和参数 schema | Tool 变化进入 final payload 合同 | TaskSpace 改动不会再漏掉普通 Tool 的缓存影响 | 修改 fixture Tool 字段时对应场景快照失败 | 失败时只阻断相关变更，不改 Tool 产品逻辑 | planned |
| CR-10 | 冻结 provider usage 解码合同 | observability | `codex-api/src/sse/chat_completions.rs`、`sse/responses.rs` | cached token decoder | 用冻结 provider SSE/JSON fixture 覆盖 hit、miss、缺字段和错误类型，并给测量合同独立版本 | decoder 变化由离线 Rust 测试判断 | 防止 provider 字段解释变化被误判成缓存性能变化 | 两种 wire API 的 decoder fixtures | decoder 未通过时结果标记不可比较，不晋升 | planned |
| CR-11 | 验证分析器与 decoder 口径一致 | observability | `run_cache_hit_regression.py`、trace analyzer tests | Python usage aggregation | 让 Python 读取 CR-10 的统一归一化 fixture，断言 request 2+ 与总量计算一致 | Rust 解码和报告聚合不再形成两个口径 | 性能报告可以从原始证据复算，不需要 API 自证 | cross-language golden fixture 和缺证据失败测试 | 不一致时只阻断报告，不触发付费运行 | planned |
| CR-12 | 建立 Standard 两请求基准场景 | scenario matrix | 新 `core/tests/suite/cache_payload_contract.rs` 及 snapshots | Standard request 1/2 fixture | 通过生产 Session 和 mock endpoint 生成连续两次请求，保留完整前缀结构 | Standard 追加路径有确定性基线 | 为 TaskSpace 差异提供可信对照 | snapshot repeat 稳定；已知消息插入 mutation 被发现 | 该场景可独立提交和回退 | planned |
| CR-13 | 覆盖三种 TaskSpace projection 策略 | scenario matrix | 同 CR-12 | map-always、map-append、map-request request 1/2 | 每种策略使用同一任务事实生成两请求 final-wire 快照 | 三种模式分别拥有实际请求合同 | 不再用 map-request 代表全部 TaskSpace | 每臂快照、前缀差异摘要和 Tool 集合断言 | 任一策略 fixture 不稳定时只暂停该策略接门 | planned |
| CR-14 | 覆盖权限上下文 | scenario matrix | `core/tests/suite/cache_payload_contract.rs`、permissions fixtures | permission developer message | 增加一次能触发真实权限消息构造的 request pair | 权限变化映射到独立 payload 场景 | 权限提示变化不会被默认样本漏掉 | 与 Standard 默认场景的差异只来自权限输入 | 场景可独立移除，不影响其他矩阵 | planned |
| CR-15 | 覆盖 Skill 上下文 | scenario matrix | `core/tests/suite/cache_payload_contract.rs`、skills fixtures | selected skill injection | 增加显式选择内置 Skill 的 request pair，并固定 Skill snapshot identity | Skill 内容和插入位置进入 payload 合同 | 内置 Skill 变化不再靠目录 glob 推测 | 无 Skill/有 Skill 对照与快照身份断言 | Skill fixture 不稳定时暂停该场景 | planned |
| CR-16 | 覆盖 Apps 与 Plugins 能力 | scenario matrix | `core/tests/suite/cache_payload_contract.rs`、现有 apps/plugins fixtures | app/plugin-provided context and tools | 各用一个最小 fixture 触发生产能力注入路径 | Apps/Plugins 造成的消息或 Tool 变化可定位 | 动态能力不再被固定 Tool 样本掩盖 | 默认、App、Plugin 三者差异来源断言 | 任一能力可单独暂停，不合并失败原因 | planned |
| CR-17 | 覆盖 MCP Tool 集合 | scenario matrix | `core/tests/suite/cache_payload_contract.rs`、MCP test server | MCP provider-visible tools | 用本地 MCP fixture 增删一个 Tool 并捕获 final-wire | MCP Tool 集合与顺序进入合同 | 外部 Tool 变化不会静默破坏前缀 | MCP off/on request pair 与 Tool order mutation | 不连接真实 MCP 服务；失败时停在本地 fixture | planned |
| CR-18 | 覆盖模型与 provider 路由 | scenario matrix | `model-provider-info`、`core/src/config/mod.rs`、`models-manager/models.json` | Flash/Pro identity 与 wire API | 建立路由元数据和最终请求身份快照；若两模型共享路径则以证据合并，不机械复制场景 | 路由变化能定位到模型/provider 身份 | 防止沿用不适用于当前模型或 wire API 的基线 | route matrix assertions 和错误模型反例 | 发现未知路由时标记 blocked-on-discovery | planned |
| CR-19 | 覆盖压缩后的请求结构 | scenario matrix | `core/tests/suite/compact*.rs`、cache payload fixtures | pre/post compaction request pair | 复用生产压缩路径生成压缩前后 final-wire 快照 | 长历史重写对稳定前缀的影响可见 | 日常长会话不会成为门禁盲区 | compact request pair、重复稳定性和首差异摘要 | 不设置真实超长 token；使用有代表性的确定性历史 | planned |
| CR-20 | 源码变化先触发免费语义测试 | gate orchestration | `cache_surface.py`、pre-commit、non-agent gate | source sentinel 和 payload runner | 将宽生产 crate、依赖配置和控制面列为免费测试触发面；只有 final-wire/measurement contract diff 才输出付费候选 | 注释、测试和等价重构可免费通过；真实 payload 变化阻断 | 同时减少漏报和不必要预算申请 | test/comment-only、生产字段变化、依赖变化、控制面变化 fixtures | 免费 runner 不稳定或过慢时保持 release 阻断，先优化 fixture | planned |
| CR-21 | 真实运行按受影响场景申请 | paid validation | `run_cache_hit_regression.py/.ps1`、contract schema | scenario router 与 budget proposal | 把固定 sample 改为场景到最小 arm/model/sample 的显式映射，运行前打印数量、请求/token/费用上限和停止条件 | 真实运行只覆盖且只声明受影响范围 | 用最少 API 成本取得可解释证据 | dry-run 不发请求并生成预算单；矩阵计数与账本 planned 记录一致 | 未获批永不执行；未知场景不得 fallback 到默认样本 | planned |
| CR-22 | 基线只按实际覆盖范围晋升 | baseline model | cache contract 与 promotion script | scoped baseline records | 用 commit + scenario + arm + model + payload digest 作为基线身份，迁移后删除全局单 hash 权威语义 | 一次运行不能替未执行场景背书 | 防止 `live_verified` 过度证明 | 不同场景结果不能互相晋升；旧 v1 结果只保留历史证据 | 新模型未验证前继续 fail closed，不保留双权威路径 | planned |
| CR-23 | 恢复门禁发布权威性 | release/review | pre-commit、non-agent gate、文档 | final gate integration | 跑全套免费测试并启动新的空白对抗性审查；只有 blocking 全关闭才替换 v1 | 发布门使用三段式证据链 | 团队可依赖门禁而不频繁误付费 | 单元/集成测试、门禁自测、fresh review；真实 run 另行申请预算 | 审查失败则保持 v1 诊断状态并回滚接线 | planned |

## 4. Phase 顺序

### Phase A：先修复控制面可信性

- Entry condition：当前 release 保持阻断，不晋升新基线。
- Work units：CR-01 至 CR-05。
- Phase-local evidence：bootstrap、partial-stage、fabricated result、dirty/untracked 和 exact-commit fixtures。
- Next-phase condition：免费检查的输入、合同和输出能够绑定同一源码身份。

### Phase B：建立生产 final-wire 权威证据

- Entry condition：Phase A 证据通过。
- Work units：CR-06 至 CR-11。
- Phase-local evidence：本地 mock 捕获生产 body，重复执行稳定，受保护字段 mutation 全部被发现。
- Next-phase condition：无需真实 API 即可判断最终请求或 usage 测量合同是否变化。

### Phase C：逐类补齐免费场景

- Entry condition：Phase B 已证明无第二 serializer。
- Work units：CR-12 至 CR-19。
- Phase-local evidence：Standard、三种 TaskSpace 策略和条件入口均有独立、可复算的 request pair。
- Next-phase condition：审查列出的 wire、context、Tool、routing、compaction 入口均映射到至少一个场景。

### Phase D：替换触发与晋升语义

- Entry condition：Phase C 场景稳定，免费运行成本可接受。
- Work units：CR-20、CR-21、CR-22。
- Phase-local evidence：语义不变的源码改动不申请预算；payload 变化准确输出受影响场景和 dry-run 预算单。
- Next-phase condition：新 scoped baseline 模型可以完全替代 v1 全局 hash，不保留两个权威来源。

### Phase E：接线与对抗性验收

- Entry condition：Phase D 没有兼容双路径。
- Work units：CR-23。
- Phase-local evidence：全套免费门禁、release fixture、控制面篡改 fixture 和 fresh adversarial review。
- Next-phase condition：只有用户另行批准最小真实运行预算后，才可建立新的 live baseline。

## 5. 验证与收益判定

| 目标 | 基线 | 验收方式 |
|---|---|---|
| 消除已知漏报 | 当前遗漏 wire/context/tool/routing 入口 | 对每个已知入口做 mutation，至少一个免费场景必须失败并指出首差异 |
| 消除已知付费误报 | 原始字节变化即阻断 | 测试、注释、格式和 final-wire 等价重构只运行免费检查且最终通过 |
| 保证证据身份 | HEAD、index、worktree 可能混用 | 每份结果包含唯一 subject SHA 和证据摘要，错配 fixture 必须拒绝 |
| 降低 API 成本 | 固定两臂，不论变更类型 | 只有 final-wire 或测量合同变化才形成预算申请，且只选择受影响场景 |
| 保持发现能力 | 已发现 96.62% vs 35.79% | 新 runner 能继续读取 provider request 2+ usage；真实复验需另行授权 |

免费门禁耗时先记录冷/热两种实测值，再决定是否把完整矩阵放入 pre-commit；不得为了追求本地速度删减共享
release 覆盖。可以让 pre-commit 调用受影响场景子集，但 non-agent/CI 必须运行由同一合同计算出的完整必要集合。

## 6. 外部依据与设计取舍

- [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/) 说明缓存依赖已持久化且完整匹配的请求
  前缀，并通过 usage 返回 hit/miss token。因此最终 provider 请求和 request 2+ usage 才是权威证据。
- [Git diff-index](https://git-scm.com/docs/git-diff-index.html) 明确 index 与 worktree 是不同比较对象。因此
  `--source index` 必须读取 index 中的合同和内容，不能混读工作区。
- [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html) 给出可哈希 JSON 的确定性表示原则。本项目只借用稳定对象
  表示思想；缓存敏感的数组顺序和字符串内容保持原样，且额外保存生产原始 body SHA。
- [GitHub Required Status Checks](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
  要求检查绑定最新 commit SHA。远端 CI 接入时应对精确 subject SHA 运行同一免费门禁，而不是接受旧提交结果。

## 7. 待确认边界

1. 远端仓库是否已启用可作为 trusted base 的 required check/branch protection，当前尚未证明。CR-04 先保证本地和
   non-agent artifact 的精确快照；远端控制面信任需要在接 CI 前单独确认。
2. 三种 TaskSpace projection 是否全部属于当前 release 支持面。计划默认都生成免费 fixture；真实付费矩阵只覆盖
   用户确认要发布的模式。
3. 任何新的真实 Whale Agent 运行不包含在本计划授权中。到 CR-21/CR-23 时必须提交独立预算申请。
