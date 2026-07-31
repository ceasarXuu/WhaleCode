# 缓存命中回归门禁修复计划

- Created: 2026-07-31
- Plan mode: Authoring
- Plan status: in progress（CR-01 至 CR-20、CR-21.1 completed；Phase A、Phase B、Phase C complete；Phase D 进行中，下一单元 CR-21.2）
- Risk: High，涉及发布门、付费验证触发与证据可信性
- Problem register: [02-known-issues.md](02-known-issues.md)

## 1. 问题与目标

当前门禁以手工 glob 的原始源码 SHA 代替最终 DeepSeek 请求语义，既漏掉真实构造入口，又会因注释和测试变化
误触发付费回归；同时存在 bootstrap 通过发布、index/worktree 混读和结果晋升证据不足等控制面缺陷。

目标是建立三段式门禁：

1. **免费源码风险哨兵**只决定是否运行更深的免费检查；
2. **免费生产 final-wire 场景矩阵**判断 provider 实际可见请求或缓存测量合同是否变化；
3. **获批真实 smoke**验证代表性真实缓存表现，并把实际配置和结果绑定到精确源码身份，不扩大为未执行路径的证明。

总体原则是**重发现、强判别、轻处置**。门禁必须严谨判断变化是否触及缓存相关语义：比较最终请求的字段、角色、
顺序、内容、Tool schema/order、`tool_choice`、model/provider identity，以及 usage 测量合同；确定性噪声不得误报，
未知字段或无法可靠比较的状态必须阻断。需要保持轻量的是判别之后的流程：门禁不判断产品变化是否正确，不替人
选择 benchmark，也不建设测试充分性或逐路径覆盖证明系统。

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
  -> 已变：阻断，输出结构化 change report、首差异和旧/新摘要
       -> 非预期变化：修复，不进入付费路径
       -> 经人工确认的有意变化：由人明确选择代表性 smoke 配置
  -> 根据所选 model/sample/arm/repeat 生成 dry-run 预算提案
  -> 用户批准完全相同的预算后运行真实 provider
  -> 忠实记录业务结果、request 2+ usage、证据摘要和源码身份
  -> 晋升已接受的 final-wire 基线，并附上 smoke 的准确边界
```

final-wire fixture 通过本地 mock endpoint 捕获生产 HTTP body。动态路径、时间和 ID 优先在 fixture 输入端固定；不得
用宽泛文本替换掩盖差异。JSON 可生成便于审阅的稳定表示，但数组顺序和字符串内容必须原样保留；同时保留生产
serializer 产生的原始 body SHA，防止可读快照遗漏真实 wire 变化。

## 3. 工作单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|
| CR-01 | 发布只接受真实基线 | release policy | `scripts/cache-regression/check_cache_regression_gate.py` | `live_status_accepted` | 删除 `structural_bootstrap` 的 release 接受分支 | `--require-live-baseline` 只接受 `live_verified` | 防止没有付费证据的版本被误放行 | 新增 bootstrap rejection fixture；现有 failed baseline 继续失败 | 单独回退该提交；发布仍保持阻断 | completed（`6a44bf0f1`） |
| CR-02 | 每种 source 使用同一快照合同 | source identity | `cache_surface.py`、`check_cache_regression_gate.py` | contract loader 与 `surface_snapshot` | 为 HEAD/index/worktree 分别从对应 Git 对象读取合同和内容；控制面文件 index/worktree 不一致时拒绝 | staged 检查不再混读 worktree，也不会执行一份规则却提交另一份 | 开发者提交的就是实际被检查的内容 | 复现旧 partial-stage case 必须失败；三 source fixture | 若失败，保留 CR-01 并恢复保守阻断 | completed（`0a5866c05`） |
| CR-03 | 控制面变更不能静默自授权 | control-plane policy | `.githooks/pre-commit`、`scripts/cache-regression/**`、non-agent gate、contract | policy-change classifier | 由变更前 subject 识别 hook、checker、contract、promoter 和结果 schema 变化，要求独立 policy-change 证据，禁止同一证据同时改验收规则并晋升基线 | 门禁自身变化被显式标记并与性能晋升分离 | 防止日常误操作让规则修改自行放行 | 对每类控制面文件做 staged mutation；同批 promotion 必须失败 | 远端 trusted-base 未确认前 release 保持 fail closed | completed（`38fc62830`） |
| CR-04 | 晋升结果不可缺字段或扩大范围 | evidence integrity | `promote_cache_baseline.py`、result JSON schema | promotion validator | 强制实际 arms、repeat、sample 数和阈值与获批运行计划完全一致，并校验 trace coverage、subject SHA、surface/payload digest 和证据摘要 | 手工残缺、超计划或错配结果不能晋升 | 避免错误证据被当成性能基线，同时不把当前 2-sample 规模固化成长期架构 | fabricated、缺 arm、实际规模偏离计划、摘要不符 fixture 均失败 | 晋升保持禁用，不影响免费诊断 | completed（`c46ebfa05`） |
| CR-05 | 发布证据绑定精确 commit | release identity | `build-v005-non-agent-gates.ps1`、gate result schema | release source selector | release 只检查显式 commit tree；相关 dirty/untracked 文件直接拒绝，结果记录同一 SHA | 检查对象、构建对象和证据对象一致 | 任何缓存结论都可复算到唯一源码 | exact-commit、dirty relevant、untracked relevant fixtures | 无法确定身份时 fail closed，不运行 API | completed（`b51664a8e`） |
| CR-06 | 找到生产 final-wire 捕获边界 | discovery | `core/src/session/turn.rs`、`core/src/client.rs`、`codex-api/src/endpoint/*.rs` | `Prompt` 到 HTTP body 的调用链 | 绘制并用测试证明 DeepSeek 请求从上下文、Tool 选择到 serializer 的唯一生产路径 | 后续 fixture 有明确权威入口 | 避免另建与生产漂移的测试 serializer | 调用链文档、函数引用和一个本地 mock 捕获 spike | 只提交调查与测试 spike；未证明前不进入 CR-07 | completed（`d04aab5fb`） |
| CR-07 | 保存可复算的 final-wire 证据 | cache contract | `core/tests/common/responses.rs`、新 `core/tests/common/cache_payload.rs` | mock request capture | 同时输出原始 body SHA 和结构化快照；只固定 fixture 输入，不宽泛重写输出 | 字节变化与可读语义差异均可定位 | 缓存变化不再靠源码路径猜测 | 相同输入重复执行摘要一致；字段/数组顺序突变可被 fixture 捕获 | 保留现有 v1 gate，新增能力未接门禁 | completed（`11d5b2bdd`） |
| CR-08 | 明确哪些差异需要付费验证 | comparison policy | `benchmarks/cache-regression/` 新 payload contract | cache-relevant fields | 固化消息角色/顺序/内容、Tool schema/order、`tool_choice`、model/provider identity 的比较规则；每个允许忽略字段逐项说明 | 门禁区分真实请求变化与确定性噪声 | 降低误报且不牺牲语义保真 | mutation tests 对每个受保护字段均报警；忽略字段反例测试 | 默认不忽略未知字段，规则不明时保守阻断 | completed（`2dc401d50`） |
| CR-09 | Tool schema 使用生产 serializer | tool wire | `tools/src/tool_spec.rs`、`core/tests/suite/cache_payload_contract.rs` | provider-visible tools array | 从实际 Session 请求捕获普通 Tool 和 `taskspace_control` 的名称、顺序、描述和参数 schema | Tool 变化进入 final payload 合同 | TaskSpace 改动不会再漏掉普通 Tool 的缓存影响 | 修改 fixture Tool 字段时对应场景快照失败 | 失败时只阻断相关变更，不改 Tool 产品逻辑 | completed（`45284b5de`） |
| CR-10 | 冻结 provider usage 解码合同 | observability | `codex-api/src/sse/chat_completions.rs`、`sse/responses.rs` | cached token decoder | 用冻结 provider SSE/JSON fixture 覆盖 hit、miss、缺字段和错误类型，并给测量合同独立版本 | decoder 变化由离线 Rust 测试判断 | 防止 provider 字段解释变化被误判成缓存性能变化 | 两种 wire API 的 decoder fixtures | decoder 未通过时结果标记不可比较，不晋升 | completed（`01e4cc915`） |
| CR-11 | 验证分析器与 decoder 口径一致 | observability | `run_cache_hit_regression.py`、trace analyzer tests | Python usage aggregation | 让 Python 读取 CR-10 的统一归一化 fixture，断言 request 2+ 与总量计算一致 | Rust 解码和报告聚合不再形成两个口径 | 性能报告可以从原始证据复算，不需要 API 自证 | cross-language golden fixture 和缺证据失败测试 | 不一致时只阻断报告，不触发付费运行 | completed（`c008cab58`） |
| CR-12 | 建立 Standard 两请求基准场景 | scenario matrix | 新 `core/tests/suite/cache_payload_contract.rs` 及 snapshots | Standard request 1/2 fixture | 通过生产 Session 和 mock endpoint 生成连续两次请求，保留完整前缀结构 | Standard 追加路径有确定性基线 | 为 TaskSpace 差异提供可信对照 | snapshot repeat 稳定；已知消息插入 mutation 被发现 | 该场景可独立提交和回退 | completed（`31f92729e`） |
| CR-13 | 覆盖三种 TaskSpace projection 策略 | scenario matrix | 同 CR-12 | map-always、map-append、map-request request 1/2 | 每种策略使用同一任务事实生成两请求 final-wire 快照 | 三种模式分别拥有实际请求合同 | 不再用 map-request 代表全部 TaskSpace | 每臂快照、前缀差异摘要和 Tool 集合断言 | 任一策略 fixture 不稳定时只暂停该策略接门 | completed（`2dd70fe75`） |
| CR-14 | 覆盖权限上下文 | scenario matrix | `core/tests/suite/cache_payload_contract.rs`、permissions fixtures | permission developer message | 增加一次能触发真实权限消息构造的 request pair | 权限变化映射到独立 payload 场景 | 权限提示变化不会被默认样本漏掉 | 与 Standard 默认场景的差异只来自权限输入 | 场景可独立移除，不影响其他矩阵 | completed（`7da38b2ed`） |
| CR-15 | 覆盖 Skill 上下文 | scenario matrix | `core/tests/suite/cache_payload_contract.rs`、skills fixtures | selected skill injection | 增加显式选择内置 Skill 的 request pair，并固定 Skill snapshot identity | Skill 内容和插入位置进入 payload 合同 | 内置 Skill 变化不再靠目录 glob 推测 | 无 Skill/有 Skill 对照与快照身份断言 | Skill fixture 不稳定时暂停该场景 | completed（`d43941d2f`） |
| CR-15A | 使用 DeepSeek 官方 Codex 协议 | provider route | `model-provider-info/src/lib.rs` | `create_deepseek_provider()` | 将内置 DeepSeek provider 的 `wire_api` 从 Chat Completions 改为 Responses，不增加第二 provider 或 Chat namespace 兼容层 | WhaleCode 通过 DeepSeek 原生 Responses API 发送 Codex 请求 | 恢复 Codex 原生 Tool 表达，避免维护私有协议翻译分支 | provider 单测、Responses endpoint 本地 mock、Chat endpoint 不得收到请求 | 单独回退 provider 提交；CR-16/17 保持阻塞 | completed（`1e5b5c0ba`） |
| CR-15B | 只暴露当前官方支持的 Codex 模型 | model catalog | `models-manager/models.json`、`models-manager/src/manager.rs` | Flash/Pro preset 与默认模型 | 将默认模型改为 Flash，按官方 Codex 元数据更新 Flash；在 Pro 官方支持前从选择列表隐藏且不得作为默认压缩模型 | 新会话默认使用可工作的 Flash Responses；用户不会误选尚未支持 Codex 的 Pro | 产品能力声明与 provider 实际支持一致 | catalog/default/selection/compact model 单测；Pro 不出现在可选列表 | 若隐藏语义无法覆盖所有入口则暂停，不做静默 fallback | completed（`3e0a36aba`） |
| CR-15C | 重建 Responses final-wire 基线 | cache contract | `core/tests/suite/cache_final_wire.rs`、`cache_payload_*` 与 snapshots | CR-12 至 CR-15 request pairs | 将既有 DeepSeek Chat fixture 改为生产 Responses endpoint，保留相同场景事实并重新生成快照 | Standard、三种 TaskSpace、权限、Skill 的权威基线与当前生产协议一致 | 后续缓存门禁不再保护已经退出的旧协议载荷 | 每个场景重复两次稳定；断言只命中 `/v1/responses` | 任一场景不稳定时停在对应 fixture，不进入 CR-16 | completed（`128b47d88`、`d229ac0aa`） |
| CR-16 | 覆盖 Apps 与 Plugins 能力 | scenario matrix | `core/tests/suite/cache_payload_capabilities_contract.rs`、现有 apps/plugins fixtures | app/plugin-provided context and tools | 各用一个最小 fixture 触发生产能力注入路径 | Apps/Plugins 造成的消息或 Tool 变化可定位 | 动态能力不再被固定 Tool 样本掩盖 | 默认、App、Plugin 三者差异来源断言 | 任一能力可单独暂停，不合并失败原因 | completed（`60c8744ef`） |
| CR-17 | 覆盖 MCP Tool 集合 | scenario matrix | `core/tests/suite/cache_payload_mcp_contract.rs`、MCP test server | MCP provider-visible tools | 用本地 MCP fixture 增删一个 Tool 并捕获 final-wire | MCP Tool 集合与顺序进入合同 | 外部 Tool 变化不会静默破坏前缀 | MCP off/on request pair 与 Tool order mutation | 不连接真实 MCP 服务；失败时停在本地 fixture | completed（`e8a810a0d`） |
| CR-18 | 覆盖模型与 provider 路由 | scenario matrix | `model-provider-info`、`core/src/config/mod.rs`、`models-manager/models.json` | Flash/Pro identity 与 wire API | 建立路由元数据和最终请求身份快照；若两模型共享路径则以证据合并，不机械复制场景 | 路由变化能定位到模型/provider 身份 | 防止沿用不适用于当前模型或 wire API 的基线 | route matrix assertions 和错误模型反例 | 发现未知路由时标记 blocked-on-discovery | completed（`f4cc55d28`） |
| CR-19 | 覆盖压缩后的请求结构 | scenario matrix | `core/tests/suite/compact*.rs`、cache payload fixtures | pre/post compaction request pair | 复用生产压缩路径生成压缩前后 final-wire 快照 | 长历史重写对稳定前缀的影响可见 | 日常长会话不会成为门禁盲区 | compact request pair、重复稳定性和首差异摘要 | 不设置真实超长 token；使用有代表性的确定性历史 | completed（`55900ac18`） |
| CR-20 | 源码变化先触发免费语义测试 | gate orchestration | `cache_surface.py`、pre-commit、non-agent gate | source sentinel 和 payload runner | 将宽生产 crate、依赖配置和控制面列为免费测试触发面；只有 final-wire/measurement contract diff 才输出付费候选 | 注释、测试和等价重构可免费通过；真实 payload 变化阻断 | 同时减少漏报和不必要预算申请 | test/comment-only、生产字段变化、依赖变化、控制面变化 fixtures | 免费 runner 不稳定或过慢时保持 release 阻断，先优化 fixture | completed（`71c8f0cf0`、`3b7d7b4fa`） |
| CR-21 | 输出判别结果并交接获批 smoke | validation handoff | `scripts/cache-regression/`、cache contract | change report、budget proposal、authorized run plan | 免费合同严格判别缓存相关语义是否变化并输出可复算差异；有意变化由人选择 smoke 配置，工具只计算预算并保证授权、命令和账本一致 | 门禁重发现、强判别、轻处置，不决定产品正确性或哪个 benchmark 足以证明变更 | 防止未知上下文变化静默进入，同时避免建设主观 coverage 推理系统 | protected-field mutation 精确报警，确定性噪声不报警；dry-run 零 API/零账本副作用 | 未获批永不执行；门禁不得自动选择或扩大 sample/arm | in progress（CR-21.1 completed） |
| CR-22 | 忠实晋升已接受基线 | baseline identity | cache contract 与 promotion script | accepted final-wire baseline、smoke evidence reference | 记录精确 commit、已接受 payload digest、实际 model/sample/arm/repeat 和真实结果；明确 smoke 仅代表其运行配置，删除全局“所有路径已验证”语义 | release 能识别当前 final-wire 已经人工接受且有代表性真实 smoke，但不会把证据扩大到未执行路径 | 保留回归门禁价值，同时消除虚假覆盖声明和复杂逐路径基线 | 错 commit、错 digest、错授权或篡改结果拒绝；报告中必须保留证据边界 | 新模型未验证前继续 fail closed，不保留双权威路径 | planned |
| CR-23 | 恢复门禁发布权威性 | release/review | pre-commit、non-agent gate、文档 | final gate integration | 跑全套免费测试并启动新的空白对抗性审查；只有 blocking 全关闭才替换 v1 | 发布门使用三段式证据链 | 团队可依赖门禁而不频繁误付费 | 单元/集成测试、门禁自测、fresh review；真实 run 另行申请预算 | 审查失败则保持 v1 诊断状态并回滚接线 | planned |

### 3.1 CR-21 执行拆分与架构边界

CR-21 不扩建 `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`，也不向通用 benchmark 注入缓存门禁、
预算、晋升或场景判定逻辑。通用 benchmark 继续只负责按显式参数执行 sample/arm 并产出事实；人决定是否运行
以及选择哪份代表性 smoke，缓存回归侧只负责预算、授权一致性和结果记录。

| 子单元 | Change Location | Concrete Action | Resulting Behavior | Benefit | Verification | Status |
|---|---|---|---|---|---|---|
| CR-21.1 | 免费 final-wire 合同与 runner | 对版本化保护字段执行严格语义比较，并为每个确定性 fixture 输出 scenario ID、比较对象、首差异和新旧 payload digest | 免费结果能可靠区分语义未变、语义变化和不可比较三种状态 | 强化未知变化发现与判别，同时把产品接受和测试选择留给人 | 每个受保护字段 mutation 命中对应 fixture/字段；允许的确定性噪声不报警；不可比较状态阻断 | completed（`e35cf681b`；[结果](13-cr21-1-change-report-result.md)） |
| CR-21.2 | 新 budget proposal CLI | 接受人工明确提供的 model/sample/arm/repeat 和停止条件，机械计算最大 sample run、预计请求、token、费用、耗时及重试上限 | `--dry-run` 不读取 API Key、不启动 Whale、不修改全局运行账本，也不推荐测试配置 | 用户能在发生费用前审阅完整预算，工具不替人决定测试充分性 | 无 Key 环境运行；零文件副作用；预算乘法和价格计算测试 | planned；当前 Next Best Intervention |
| CR-21.3 | `run_cache_hit_regression.py/.ps1` | 只接受已保存预算提案和完全匹配的用户授权；启动前才创建 `planned` 账本记录；拒绝扩大矩阵和自动重试 | 获批内容、实际命令和账本计划使用同一不可变计划 | 防止 dry-run 变相运行、授权与执行错配或包装脚本扩大成本 | mock subprocess、授权错配、计划篡改、账本一致性测试；不调用 provider | planned |
| CR-21.4 | gate 输出与结果 schema | 免费失败只输出 change report 和下一步选项；真实结果只声明实际配置、指标和证据路径，禁止生成未执行路径的覆盖结论 | 发现、人工决策、预算和执行结果边界清晰 | 保持门禁简单可审计，避免结果被过度解释 | unintended、accepted、authorized、result-boundary 四类 fixture | planned |

### 3.2 Dev Loop 约束

- **决策边界**：从缓存相关改动完成，到开发者获得“免费通过、应修复，或需要申请哪一份预算”之一的可信结论；
  不把后续真实运行耗时混入普通 edit-to-feedback 指标。
- **已有基线**：CR-20 免费矩阵热状态约 `4.5s`、发生增量重编译时约 `29.4s`；真实 provider 路径成本高且需要
  用户授权，不能作为普通提交的默认反馈步骤。
- **P0 finding**：固定 smoke 曾被解释为未执行路径的证明，属于验证结果不可信；release 当前保持阻断，因此问题已被
  containment，但在 CR-21/CR-22 完成前不能恢复发布权威性。
- **当前唯一 Next Best Intervention**：只实施 CR-21.2。CR-21.1 已提供忠实的三态变化事实；下一步只把人明确
  选择的 smoke 配置机械换算为预算提案，不并行修改执行器、账本或 baseline 模型。
- **证据分层**：final-wire contract 负责严格判断“请求缓存语义是否变化或不可比较”；真实 benchmark 负责“真实
  provider 缓存和业务结果”；人工负责判断变化是否符合产品预期及选择代表性 smoke。三层不得互相替代。
- **验证升级**：CR-21.1 已先运行目标 mutation 与 runner 单测，再运行全部免费缓存控制面测试。验证复用了现有
  Cargo 增量状态，没有清缓存、重建 Docker 或运行 Whale Agent。
- **反馈门禁**：CR-21.1 只有在 affected mutation 被发现、unaffected case 明确报告 `unchanged` 且无差异、失败
  位置可操作、热态耗时没有实质扩大时才保留；当前验收已满足。
- **停止条件**：当免费失败能精确定位 fixture/字段且不添加语义分类时进入下一单元；若精确定位必须侵入
  通用 benchmark 或依赖真实 API，则暂停 CR-21，不能扩大架构。
- **长期 guardrail**：任何新增阻断检查必须声明触发面、独立证据、预期热/冷耗时、失败升级路径和移除条件；
  不允许用“更全面”作为无条件扩大 E2E 矩阵的理由。

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

### Phase C：迁移官方协议并逐类补齐免费场景

- Entry condition：Phase B 已证明无第二 serializer。
- Work units：CR-12 至 CR-15、CR-15A 至 CR-15C、CR-16 至 CR-19。
- Phase-local evidence：Standard、三种 TaskSpace 策略和条件入口均有独立、可复算的 request pair。
- Next-phase condition：审查列出的 wire、context、Tool、routing、compaction 入口均映射到至少一个场景。

### Phase D：替换触发与晋升语义

- Entry condition：Phase C 场景稳定，免费运行成本可接受。
- Work units：CR-20、CR-21、CR-22。
- Phase-local evidence：语义不变的源码改动不申请预算；payload 变化输出结构化事实；只有人工确认并明确选择 smoke
  配置后才生成 dry-run 预算单；结果不产生未执行路径结论。
- Next-phase condition：新的 accepted final-wire baseline 和准确边界的 smoke 引用完全替代 v1 过度证明语义。

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
| 降低 API 成本 | 固定两臂，不论是否存在 wire 变化 | 只有 final-wire 或测量合同变化才允许形成预算申请；配置由人明确选择，工具不得自行扩展 |
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
2. 三种 TaskSpace projection 均保留免费 fixture 用于发现变化；真实 smoke 运行哪些模式由用户在预算批准时明确
   指定，结果不得扩展到未运行模式。
3. 任何新的真实 Whale Agent 运行不包含在本计划授权中。到 CR-21/CR-23 时必须提交独立预算申请。
