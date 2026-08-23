# Problem P-001: v0.0.6 多 Provider 发布被仓库基线失败阻断
- Status: investigating
- Created: 2026-08-24 04:32 +0800
- Updated: 2026-08-24 06:38 +0800
- Objective: 在不改写已确认产品逻辑的前提下，找到并修复阻断 v0.0.6 multi-provider 发布门禁的仓库基线根因。
- Symptoms:
  - 受影响六 crate 的隔离 nextest 矩阵执行 9284 项，8928 通过、356 失败。
  - `just fmt-check` 和 `just clippy -p codex-tui` 也被 multi-provider 改动外的问题阻断。
- Expected behavior:
  - multi-provider 相关实现与仓库发布门禁一致，六 crate 隔离矩阵、fmt 和 Clippy 可通过。
- Actual behavior:
  - 失败分布于 core 257、app-server 52、TUI 46、protocol 1，涉及 code mode、MCP、plugins、Guardian、status、pets 与 provider/model 兼容测试。
- Impact:
  - v0.0.6 multi-provider 功能定向测试通过，但无法证明 release-ready。
- Reproduction:
  - `python3 scripts/codex-upstream/run_isolated_tests.py -p codex-login -p codex-models-manager -p codex-protocol -p codex-core -p codex-app-server -p codex-tui`
- Environment:
  - Ubuntu 24.04 x86_64；branch `whalecode-alpha`；commit `09d8d4fa1`；Rust stable 1.95 工具链；Asia/Shanghai。
- Known facts:
  - login 197/197 和 models-manager 54/54 通过。
  - 失败集合的 72.2% 在 core，其中 code mode 80 项为最大单一簇。
  - 至少两个 provider/model 名称相关失败使用默认 DeepSeek 配置，但断言 OpenAI 目录或 capability。
- Ruled out:
  - none
- Fix criteria:
  - 确认的根因与修复一一对应；原始失败簇的定向复现通过；六 crate 隔离矩阵、`just fmt-check`、相关 Clippy 和 cache gate 通过；无未授权真实模型请求。
- Current conclusion: 失败明显不是 356 个独立问题，但共享根因的数量和边界尚未通过证据门禁。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
  - H-008
  - H-009
  - H-010
  - H-011
  - H-012
  - H-013
  - H-014
  - H-015
  - H-016
  - H-017
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: legacy 模型目录接口对 OpenAI 调用错误应用 DeepSeek-only 过滤
- Status: confirmed
- Parent: P-001
- Claim: 一批 model、capability、code-mode 和 Guardian 失败由 OpenAI fixture/兼容调用仍使用 legacy `build_available_models`，而该接口默认只保留 `deepseek-*` 引起。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - Whale 产品默认 Provider 已是 DeepSeek，而上游大量测试默认使用 OpenAI 能力；legacy `ModelsManager` 无 route 参数，却默认执行 DeepSeek-only 过滤。
- Falsifiable predictions:
  - If true: 临时关闭 legacy DeepSeek-only 过滤后，OpenAI 代表测试恢复，且不需要改 capability 或 tool 实现。
  - If false: 关闭过滤不改变代表测试失败信号。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败由 effective provider 与断言 provider 不同导致。
  - Signal: 定向测试的 config/provider 事实、失败断言和临时关闭过滤的对照实验。
  - Capture method: 读取 fixture 与 manager 构建路径，做可立即回滚的单点诊断改动。
  - Event name or marker:
    - none
  - Correlation keys:
    - test name
  - Differentiates from:
    - H-003
  - Supports if:
    - 临时关闭 legacy 过滤单独恢复代表性失败断言。
  - Refutes if:
    - 临时关闭 legacy 过滤不改变失败信号。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-005
  - E-006
- Conclusion: 关闭 legacy DeepSeek-only 过滤可单独恢复模型切换失败，且撤销 provider ID 同步后仍通过；该机制已确认，但不解释 Code Mode 与 Guardian。
- Repair design readiness: implemented and verified
- Next step: none for H-001
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 隔离环境缺失资产或工具导致批量基础设施失败
- Status: confirmed
- Parent: P-001
- Claim: MCP、plugins、pets、fmt 和部分 app-server 失败由隔离 runner 未提供测试需要的资产、可执行文件、配置或环境变量引起。
- Layer: environment
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - `dotslash` 明确缺失，且 MCP/plugin/pets 失败具有资产与子进程依赖特征。
- Falsifiable predictions:
  - If true: 失败输出会聚类为 missing executable/file/config/server metadata，补齐隔离环境后多项同时恢复。
  - If false: 失败在资产完整的定向宿主测试中以同样业务断言稳定复现。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败信号是环境/资产缺失而非业务状态错误。
  - Signal: JUnit failure message、定向宿主复现、隔离 runner 复制清单与运行环境。
  - Capture method: 抽取 MCP、plugin、pets 各一个失败的 raw failure，对照宿主定向测试。
  - Event name or marker:
    - none
  - Correlation keys:
    - test name
  - Differentiates from:
    - H-004
  - Supports if:
    - raw failure 指向共享缺失资产/工具且宿主对照改变结果。
  - Refutes if:
    - 宿主与隔离结果一致且断言为稳定业务语义差异。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Related evidence:
  - E-001
  - E-004
  - E-007
  - E-008
- Evidence gate: satisfied
- Conclusion: Code Mode 与 MCP 代表失败分别缺少 `codex-code-mode-host` 和 `test_stdio_server`；前者当前还受 rusty_v8 上游缺失预构建资产阻断。
- Repair design readiness: MCP helper repair implemented and verified; code-mode host dependency remains externally blocked
- Next step: 将 code-mode host 404 作为独立上游依赖阻断，不混入产品回归；继续盘点其他失败簇。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: multi-provider 路由改动破坏了显式 route 的运行时行为
- Status: unverified
- Parent: P-001
- Claim: 至少一部分失败由 route-bound manager、capability 或 transition 实现在显式 route 下返回错误运行时导致。
- Layer: regression-window
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 失败集合包含 model switching、models cache 和 provider capabilities 名称相关测试。
- Falsifiable predictions:
  - If true: 显式 OpenAI/DeepSeek route 的新 contract test 或最小运行时复现也会失败。
  - If false: 新 route-bound contract test 均通过，失败只发生在未迁移的 legacy fixture/caller。
- Diagnostic evidence plan:
  - Prediction or clause under test: 显式 route 行为本身是否失败。
  - Signal: route-bound auth/catalog/transition/history/capability 定向测试结果与失败 caller 的 route 使用方式。
  - Capture method: 重跑最小显式 route 套件，并检查相关失败是否调用 legacy accessor。
  - Event name or marker:
    - none
  - Correlation keys:
    - route and test name
  - Differentiates from:
    - H-001
  - Supports if:
    - 显式 route 套件可复现同一错误。
  - Refutes if:
    - 显式 route 套件通过且失败 caller 未携带 route。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-002
  - E-003
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 复验 route-bound 套件并对照 legacy accessor 失败。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: 快照与断言未跟随 Whale 产品基线演进
- Status: unverified
- Parent: P-001
- Claim: TUI status/Guardian/exec/feedback 以及部分 core 失败是预期文本、模型目录、品牌或布局快照落后于已接受产品行为。
- Layer: regression-window
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - multi-provider 收口时已发现 model popup 快照缺少既有 vision model，更新预期后单测通过。
- Falsifiable predictions:
  - If true: `.snap.new` 与当前渲染的差异将与已接受的模型/品牌/布局变化一致，不显示数据丢失或错误状态。
  - If false: 快照差异暴露未预期的交互、状态或信息丢失。
- Diagnostic evidence plan:
  - Prediction or clause under test: 快照差异是预期演进还是真实行为回归。
  - Signal: 代表性 snapshot diff 及引入变化的 commit/code path。
  - Capture method: 选 status、Guardian、exec/feedback 各一项定向运行并审查 `.snap.new`。
  - Event name or marker:
    - none
  - Correlation keys:
    - snapshot name
  - Differentiates from:
    - H-002
  - Supports if:
    - 差异可与已接受的实现变更直接对应。
  - Refutes if:
    - 差异含不可解释的状态丢失或错误行为。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - remove `.snap.new` after diagnosis
- Evidence gate: pending
- Related evidence:
  - E-001
- Conclusion: Guardian 两份请求布局快照已确认仅落后于既有 Whale 品牌文本；TUI 等其余快照仍需分别核验，不能据此整体判定。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 定向生成代表性 snapshot diff。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-005: Guardian 测试夹具隐式继承 DeepSeek 默认 Provider
- Status: confirmed
- Parent: P-001
- Claim: Guardian 单测用 dummy OpenAI API auth 和 OpenAI mock server 验证 Responses 行为，却只覆盖 `base_url`，因此在 Whale 默认 DeepSeek 后仍要求 `DEEPSEEK_API_KEY` 并批量失败。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 失败直接报告缺少 `DEEPSEEK_API_KEY`，而测试请求、认证和 mock 均属于 OpenAI fixture。
- Falsifiable predictions:
  - If true: 在 fixture 中同时绑定 OpenAI provider ID、registry entry 和 concrete provider 后，失败无需修改 Guardian 生产逻辑即可恢复。
  - If false: 显式绑定 OpenAI 后仍以同样的缺少 DeepSeek key 或 Guardian 业务断言失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: fixture provider 与测试协议不一致。
  - Signal: 原始认证错误、显式 OpenAI 对照运行、剩余失败类型。
  - Capture method: 仅修改测试配置，重跑完整 Guardian 单测簇与三个代表测试。
  - Event name or marker:
    - none
  - Correlation keys:
    - guardian test name
  - Differentiates from:
    - H-003
  - Supports if:
    - 原失败批量恢复，剩余差异可独立归因。
  - Refutes if:
    - 原失败保持不变。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-009
- Conclusion: 显式 OpenAI fixture 将原 15 个 Guardian 单测失败中的 12 个直接恢复；补齐两个手工 fixture 后，错误传播测试恢复，剩余两项仅为品牌快照。
- Repair design readiness: implemented and verified
- Next step: none for H-005
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-006: app-server 默认 capability 用例仍断言上游 OpenAI 默认值
- Status: confirmed
- Parent: P-001
- Claim: `read_default_provider_capabilities` 未显式配置 provider，却仍期待 OpenAI 能力；Whale 的已确认默认 provider 是 DeepSeek，因此测试预期过期。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 配置层已有 `defaults_to_deepseek_flash_responses_provider` 合同，provider 层已有 DeepSeek capability 合同。
- Falsifiable predictions:
  - If true: endpoint 实际返回 DeepSeek 的 false/false/true；显式配置 OpenAI 时仍返回 true/true/true。
  - If false: endpoint 返回值不随配置 provider 改变，或 DeepSeek 返回值与 provider 单测不一致。
- Diagnostic evidence plan:
  - Prediction or clause under test: endpoint 正确读取当前 provider，只有默认用例预期陈旧。
  - Signal: 默认与显式 OpenAI 两个 endpoint 对照测试。
  - Capture method: 保留默认路径并新增显式 OpenAI 覆盖。
  - Event name or marker:
    - none
  - Correlation keys:
    - provider ID
  - Differentiates from:
    - H-003
  - Supports if:
    - 默认返回 DeepSeek 能力，显式 OpenAI 返回 OpenAI 能力。
  - Refutes if:
    - 两个配置返回相同或错误能力。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-012
- Conclusion: 原用例的默认前提已被 Whale 默认 DeepSeek 配置取代；生产 endpoint 与 provider capability 实现一致。
- Repair design readiness: implemented and verified
- Next step: none for H-006
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-007: app-server 插件测试仍依赖已取消的 feature 隐式启用
- Status: confirmed
- Parent: P-001
- Claim: plugin sharing 与 recommended plugins 的正向测试未显式启用完整 feature 依赖，因当前稳定 feature 默认关闭而提前进入禁用或 legacy 路径。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 失败分别稳定报告 `plugin sharing is disabled` 和缺少 `<recommended_plugins>`；feature registry 明确将两者默认设为 false。
- Falsifiable predictions:
  - If true: 只在正向测试夹具显式启用对应 feature 后，分享与推荐簇恢复，显式禁用测试继续通过。
  - If false: 启用 feature 后仍以相同禁用信号失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: 请求未进入被测正向路径是因为 fixture 缺少 opt-in。
  - Signal: feature registry、禁用错误、正反向文件级测试结果。
  - Capture method: 修改共享正向 fixture，运行 plugin_share 与 recommended_plugins 两个完整模块。
  - Event name or marker:
    - none
  - Correlation keys:
    - feature key and test module
  - Differentiates from:
    - H-002
  - Supports if:
    - 正向与显式禁用用例同时通过。
  - Refutes if:
    - 同一禁用错误保留或禁用态语义被破坏。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-014
- Conclusion: 分享簇已证明失败发生在测试配置门禁；推荐路径还要求 `remote_plugin`，首轮只启用 `recommended_plugins` 的对照仍进入 legacy 模式，进一步确认完整依赖必须由 fixture 声明。
- Repair design readiness: implemented and verified
- Next step: none for H-007
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-008: plugin-list remote catalog fixture 未声明正交 feature 组合
- Status: confirmed
- Parent: P-001
- Claim: 14 个 plugin-list 失败由测试 helper/直写配置只启用 `plugins`，却分别期待 remote catalog、sharing catalog 或二者组合引起。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - feature registry 将 `remote_plugin` 与 `plugin_sharing` 独立默认关闭；失败表现为 remote marketplace 为空、请求未发生或错误走入 legacy vertical endpoint。
- Falsifiable predictions:
  - If true: 给每类正向 fixture 声明其最小 feature 组合后，完整 plugin-list 模块恢复；显式 remote disabled 用例仍只请求 sharing 所需 scope。
  - If false: 补齐 feature 后仍出现相同的空 catalog/零请求信号。
- Diagnostic evidence plan:
  - Prediction or clause under test: catalog 生产逻辑未执行是 feature 组合不完整，而非远端映射损坏。
  - Signal: 原全量 JUnit、fixture config、request processor feature gate、完整模块复验。
  - Capture method: 修共享 helper 与四个名称明确的 remote-enabled 直写配置，运行 plugin-list 模块。
  - Event name or marker:
    - none
  - Correlation keys:
    - feature combination and marketplace kind
  - Differentiates from:
    - H-002
  - Supports if:
    - catalog、cache、startup refresh 与显式 kind 用例同时恢复。
  - Refutes if:
    - 原请求计数与 marketplace 断言保持失败。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-016
- Conclusion: remote 与 sharing 是正交 opt-in；测试 helper 仍按旧的聚合默认编写，生产 feature 门禁无需改变。
- Repair design readiness: implemented and verified
- Next step: none for H-008
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-009: 外部配置导入断言仍使用上游品牌文本
- Status: confirmed
- Parent: P-001
- Claim: attribution-only 导入测试期待 `Codex guidance`，但 Whale 已有的导入源归属文本是 `Whale guidance`，失败来自陈旧品牌断言。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 测试验证的是 source 字段仅用于归属而不改变主导入源，不应借此要求旧产品品牌。
- Falsifiable predictions:
  - If true: 只更新品牌预期即可恢复测试，导入来源和文件路径行为不变。
  - If false: 更新品牌后仍有导入路由或内容错误。
- Evidence gate: satisfied
- Related evidence:
  - E-018
- Conclusion: 定向测试仅更新品牌预期后通过，生产逻辑未修改。
- Repair design readiness: implemented and verified
- Next step: none for H-009
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-010: secondary session fixture 偶然依赖目录反解和文件 mtime
- Status: confirmed
- Parent: P-001
- Claim: Cursor 会话端到端夹具未在记录顶层提供 `cwd` 和时间戳，导致临时目录名无法唯一反解时 session summary 被丢弃。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - Cursor session parser支持记录内 `cwd`/`timestamp_ms`；原夹具把时间写在用户文本标签中，并依赖 fallback cwd 才回退到文件 mtime。
- Falsifiable predictions:
  - If true: 补齐记录级 `cwd` 和 `timestamp_ms` 后，Sessions 与 Plugins 两项都会被检测并完成导入。
  - If false: 补齐元数据后 Sessions 仍缺失。
- Evidence gate: satisfied
- Related evidence:
  - E-018
- Conclusion: 补齐真实记录格式的必要元数据后，端到端检测和导入恢复。
- Repair design readiness: implemented and verified
- Next step: none for H-010
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-011: command/exec 测试误把宿主不可见等同于子进程权限错误
- Status: refuted
- Parent: P-001
- Claim: 隔离沙箱允许命令在临时文件系统视图中完成父路径写入，但不会把该写入暴露给宿主；原测试用 shell `!` 要求写入返回错误，误判了隔离合同。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 原失败中命令退出 1，但宿主父文件不存在；移除 `!` 后命令退出 0，宿主父文件仍不存在且工作目录子文件正常持久化。
- Falsifiable predictions:
  - If true: 直接执行两次写入时命令成功，同时只有 workspace root 内的子文件在宿主可见。
  - If false: 父文件泄漏到宿主或子文件也不可见。
- Evidence gate: satisfied
- Related evidence:
  - E-018
  - E-026
  - E-028
- Conclusion: 先前 `/dev/shm` 不可见环境给出了误导性通过；在沙箱可见的 `/var/tmp` 上，父目录写入正确返回只读错误，原 `!` 断言才是权限边界合同。
- Repair design readiness: reverted incorrect repair and verified
- Next step: none for H-011
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-012: 非 OpenAI 子 Agent 的消息正文被 provider 历史投影剥离
- Status: confirmed
- Parent: P-001
- Claim: multi-agent v2 对普通工具调用无条件把消息正文写入 OpenAI 专用 `encrypted_content`；非 OpenAI provider 的历史投影只保留文本内容，因此 DeepSeek 子 Agent 只收到消息头而丢失 payload。
- Layer: runtime
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 冷恢复测试把 matcher 改为稳定的结构化消息匹配后，捕获到 `Payload:` 后正文为空；源码路径同时显示发送端无条件构造 encrypted content，接收端对非 OpenAI 历史删除 encrypted content。
- Falsifiable predictions:
  - If true: 根据目标 provider 选择消息表示后，DeepSeek/mock 子 Agent 能看到完整 payload，OpenAI 路径仍保留 encrypted content。
  - If false: 改为可移植明文信封后冷恢复测试仍缺失正文，或 OpenAI 加密合同被破坏。
- Diagnostic evidence plan:
  - Prediction or clause under test: 正文丢失发生在 encrypted inter-agent content 到 non-OpenAI history projection 的边界。
  - Signal: mock 捕获的请求正文、发送端构造类型、投影后保留的 content 类型。
  - Capture method: 结构化 wiremock matcher、源码路径核对、provider 双路径单测。
  - Event name or marker:
    - `agent_message`
  - Correlation keys:
    - recipient agent path
  - Differentiates from:
    - 仅测试 matcher 超时或 developer instructions 未恢复
  - Supports if:
    - 原请求含消息头但 payload 为空，且 provider-aware 表示恢复正文。
  - Refutes if:
    - 请求未经过 encrypted content，或 provider-aware 表示不改变结果。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-019
  - E-020
- Conclusion: 根因已确认；发送端必须以目标 provider 能消费的表示写入会话，未知 provider 保守使用可移植明文以保证语义完整。
- Repair design readiness: implemented and verified
- Next step: 运行 cache regression gate 并提交。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-013: host-only file-change fixture 被自动环境绑定到不同 cwd
- Status: refuted
- Parent: P-001
- Claim: session approval 持久化测试在宿主 `workspace` 准备文件，却通过 `start_thread` helper 自动选择另一个 local environment cwd；首轮新增文件可见，但第二轮隔离视图无法读取宿主文件，导致补丁失败和测试超时。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 测试已明确声明 apply-patch fixture 只在 host materialize；运行日志同时显示 thread cwd 与自动 environment cwd 不同，第二轮报宿主 README 不存在，而 approval request 确实已被跳过。
- Falsifiable predictions:
  - If true: 对该 host-only 测试禁用 auto environment 并显式启动 thread 后，两轮补丁成功且第二轮不发审批请求。
  - If false: 对齐 cwd 后仍出现审批重复或文件不可见。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败来自 fixture 环境选择，不是 AcceptForSession 状态丢失。
  - Signal: thread/environment cwd、第二轮 approval request 数量、apply_patch 错误路径。
  - Capture method: 隔离测试日志与测试 helper 源码对照。
  - Event name or marker:
    - `FileChangeRequestApproval`
  - Correlation keys:
    - `patch-call-1`
    - `patch-call-2`
  - Differentiates from:
    - session approval 未持久化
  - Supports if:
    - 第二轮没有 approval request，但读取与 host fixture 同名路径失败。
  - Refutes if:
    - 第二轮再次发出 approval request。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-021
  - E-023
- Conclusion: approval 持久化行为已生效，但禁用自动环境后相同路径仍不可见；cwd 分裂不是根因。
- Repair design readiness: not applicable
- Next step: 由 H-015 解释并修复隔离临时根目录可见性。
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-014: legacy attribution 测试输入被误品牌化为当前 Whale 文本
- Status: confirmed
- Parent: P-001
- Claim: 冷恢复测试的 CommitOnly legacy fixture 被品牌提交改成 Whale trailer，但生产迁移器故意只识别旧 Codex trailer，因此该分支不再触发 legacy 替换。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - `git blame` 显示同一品牌提交只修改测试常量；生产 `LEGACY_COMMIT_ATTRIBUTION_INSTRUCTIONS` 保持 Codex，用于真实旧 rollout 兼容。
- Falsifiable predictions:
  - If true: 恢复测试的 Codex legacy trailer 后，commit-only 分支保留一次历史 Codex commit 指令、不产生当前 Whale commit 指令，并追加一次 disabled 指令；unlinked PR 分支保留一次当前 Whale commit 指令并追加一次 disabled 指令。
  - If false: 恢复真实 legacy 输入后 disabled 指令仍缺失，或任一分支发生重复。
- Diagnostic evidence plan:
  - Prediction or clause under test: fixture 不再匹配生产 legacy matcher。
  - Signal: 测试/生产 legacy 常量逐字差异、品牌提交 blame、两个参数化分支结果。
  - Capture method: 源码与 blame 对照、隔离参数化测试。
  - Event name or marker:
    - `<git_attribution>`
  - Correlation keys:
    - `commit_only`
    - `unlinked_pull_request`
  - Differentiates from:
    - workspace attribution policy 请求失败
  - Supports if:
    - 只有 commit_only 失败，完整策略与 unlinked PR 分支通过。
  - Refutes if:
    - 两个 legacy 分支或策略请求同时失败。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-022
  - E-024
  - E-025
- Conclusion: 生产规则按历史类型保留一份既有 commit 指令并追加 disabled 覆盖；测试必须区分 Codex legacy 与 Whale 当前指令，不能继续用品牌替换后的同一字符串计数。
- Repair design readiness: implemented and verified
- Next step: none for H-014
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-015: 隔离 runner 的 `/dev/shm` 根目录对受限文件沙箱不可见
- Status: confirmed
- Parent: P-001
- Claim: Codex 隔离 runner 在 Linux 优先把测试临时根放到 `/dev/shm`，但 app-server 的受限 apply-patch 文件沙箱不暴露该挂载，导致宿主创建或首轮创建的 workspace 文件在校验进程中不可见。
- Layer: test-infrastructure
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 将 README 预先写入宿主 workspace 后，首轮 update 仍报文件不存在；同一测试仅将有效临时根改为 `/var/tmp` 即通过。
- Falsifiable predictions:
  - If true: runner 默认优先选择 `/var/tmp` 后，原始 Add→Update approval 用例通过，无需修改产品逻辑或测试环境选择。
  - If false: `/var/tmp` 下仍出现相同文件不可见，或必须改变 approval/session 实现才能通过。
- Diagnostic evidence plan:
  - Prediction or clause under test: `/dev/shm` 挂载边界而非审批状态或 cwd 选择造成不可见。
  - Signal: 实际 runtime cwd、预存文件首轮 update、仅改变 runtime root 的对照结果。
  - Capture method: 隔离 nextest 日志及 `WHALE_CODEX_TEST_TMPDIR=/var/tmp` 对照。
  - Event name or marker:
    - `apply_patch verification failed`
  - Correlation keys:
    - nextest runtime root
    - `patch-call-1`
  - Differentiates from:
    - H-013
    - AcceptForSession 状态丢失
  - Supports if:
    - `/dev/shm` 失败而 `/var/tmp` 通过。
  - Refutes if:
    - 两个根目录结果相同。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-023
  - E-025
- Conclusion: 根因是隔离 runner 的默认临时挂载选择；显式配置无效时静默回退还会掩盖对照，需改为快速失败。
- Repair design readiness: implemented and verified
- Next step: none for H-015
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-016: app-server 隔离回归缺少 code-mode host 预构建与离线 V8 资产绑定
- Status: confirmed
- Parent: P-001
- Claim: app-server 的 Code Mode、ImageGen Code Mode 和相关 analytics 用例依赖 `codex-code-mode-host`，但隔离 runner 未预构建它；直接补构建时，代理被清理的资格化环境又无法自动下载 V8 归档。
- Layer: test-infrastructure
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 完整回归中四项失败共享 host binary 不存在或 cell 为空信号；绑定资格化流程已校验的本地 V8 资产并构建 host 后，四项全部通过。
- Falsifiable predictions:
  - If true: runner 对 app-server/core scope 预构建 host，并自动复用校验通过的候选缓存后，四项用例无需产品改动即可通过。
  - If false: host 存在后仍出现相同 Null image、hasCell=false 或远端 host 启动失败。
- Diagnostic evidence plan:
  - Prediction or clause under test: 失败由缺失 runtime helper 产生。
  - Signal: host spawn 错误、V8 build 输出、绑定缓存后的目标回归。
  - Capture method: 完整 nextest 与五项定向对照。
  - Event name or marker:
    - `failed to spawn code-mode host`
  - Correlation keys:
    - `codex-code-mode-host`
    - nextest test names
  - Differentiates from:
    - ImageGen tool result映射错误
    - analytics correlation 逻辑错误
  - Supports if:
    - 构建 host 后四项同时恢复。
  - Refutes if:
    - 任一原信号保留。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-027
  - E-028
- Conclusion: app-server 测试的运行时依赖未被 runner 声明；应复用既有、带校验的 V8 候选缓存，不开放隔离测试的宿主代理。
- Repair design readiness: implemented and verified
- Next step: none for H-016
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-017: tool spec 规划测试隐式继承 DeepSeek provider 能力
- Status: confirmed
- Parent: P-001
- Claim: `spec_plan_tests` 的通用探针沿用 `make_session_and_context` 的 Whale 默认 DeepSeek provider，但多数用例验证 namespace、deferred tool search 和 hosted tools 等 OpenAI 能力，导致工具被正确过滤后测试误报。
- Layer: test-fixture
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 24 项失败集中表现为 namespace/deferred/hosted tool 缺失；规划器明确按 provider capabilities 门禁，文件中显式 Bedrock 用例反而通过。
- Falsifiable predictions:
  - If true: 通用 probe 显式绑定 OpenAI provider 后，namespace/deferred/hosted 工具断言恢复；显式 Bedrock 用例仍通过。
  - If false: 绑定 OpenAI 后原工具仍缺失或 Bedrock 合同被破坏。
- Diagnostic evidence plan:
  - Prediction or clause under test: 测试 provider 与被测能力合同不一致。
  - Signal: 失败工具类别、provider capabilities gate、完整 spec-plan 文件回归。
  - Capture method: 只修改测试探针 provider，运行 49 项 spec-plan 单测。
  - Event name or marker:
    - none
  - Correlation keys:
    - provider ID
    - tool namespace/exposure
  - Differentiates from:
    - tool registry 生产实现丢失
  - Supports if:
    - 原 24 项失败全部恢复。
  - Refutes if:
    - 同一缺失信号保留。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-030
  - E-031
- Conclusion: 通用 spec-plan fixture 必须显式选择支持其工具合同的 OpenAI provider；provider 专属用例继续自行覆盖。
- Repair design readiness: implemented and verified
- Next step: none for H-017
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 失败按 crate 与模块聚类
- Related hypotheses:
  - H-002
  - H-004
- Direction: supports
- Type: test
- Source: `third_party/codex-cli/codex-rs/target/nextest/local/junit.xml`
- Prediction or plan link:
  - H-002/H-004 的失败应聚类而非均匀分布预测。
- Matched signal:
  - core 257、app-server 52、TUI 46、protocol 1；core code_mode 80、tools 26、RMCP 24；app-server plugin list/share 24；TUI status 14、Guardian 8。
- Correlation keys:
  - nextest UUID `2c80ad31-b935-4c0a-9ea8-bae8f614a1a8`
- Raw content:
  ```text
  tests=9284 failures=356 time=244.568
  codex-core failures=55; codex-core::all failures=202
  codex-app-server::all failures=52
  codex-tui failures=46
  codex-protocol failures=1
  ```
- Interpretation: 少数模块占据大多数失败，值得优先查找共享 fixture、环境或快照根因。
- Time: 2026-08-24 04:32 +0800

## Evidence E-002: 两个 provider 相关失败的 effective default 为 DeepSeek
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: reproduction
- Source: 定向 `cargo test` 与对应 fixture/code path
- Prediction or plan link:
  - H-001 的 provider 前提与断言 provider 不同预测。
- Matched signal:
  - model switching 测试在 legacy/default manager 中找不到注入的 OpenAI 模型；capability read 实际返回 DeepSeek false/false/true，测试期待 OpenAI true/true/true。
- Correlation keys:
  - `model_switch_to_smaller_model_updates_token_context_window`
  - `read_default_provider_capabilities`
- Raw content:
  ```text
  expected test-text-only-model to be available in remote model list
  actual capabilities: namespace_tools=false, image_generation=false, web_search=true
  expected capabilities: namespace_tools=true, image_generation=true, web_search=true
  ```
- Interpretation: 证明了存在 legacy fixture/provider 前提冲突；尚未证明该机制能解释多少失败。
- Time: 2026-08-24 04:32 +0800

## Evidence E-003: multi-provider 显式 route 定向套件通过
- Related hypotheses:
  - H-003
- Direction: refutes
- Type: test
- Source: Phase 1–4 定向测试记录与 `docs/releases/v0.0.6/multi-provider/plan.md`
- Prediction or plan link:
  - H-003 的显式 route 行为也会失败预测。
- Matched signal:
  - route-bound auth/catalog/cache/transition/history/lifecycle/TUI selection/login recovery 定向套件均通过。
- Correlation keys:
  - `openai/chatgpt`
  - `openai/api-key`
  - `deepseek/api-key`
- Raw content:
  ```text
  codex-login 197/197 passed
  codex-models-manager 54/54 passed
  provider transition, history projection, lifecycle and TUI routed selection focused tests passed
  ```
- Interpretation: 降低了显式 route 实现普遍回归的可能性，但需要在当前 HEAD 复验关键套件才能关闭 H-003。
- Time: 2026-08-24 04:32 +0800

## Evidence E-004: fmt 与 Clippy 阻断来自环境和未触及模块
- Related hypotheses:
  - H-002
- Direction: supports
- Type: environment
- Source: `just fmt-check`、`cargo fmt --all -- --check`、`just clippy -p codex-tui`
- Prediction or plan link:
  - H-002 的工具/环境缺失会独立阻断门禁预测。
- Matched signal:
  - `cargo fmt --all -- --check` 通过；`just fmt-check` 因 `dotslash` 不存在且 stable rustfmt 不支持 nightly import granularity 而失败；Clippy 在未触及的 codex-state `expect_used` 失败。
- Correlation keys:
  - none
- Raw content:
  ```text
  [Errno 2] No such file or directory: 'dotslash'
  error: used expect() on an Option value
  state/src/runtime/taskspace_action_settlements.rs:35:52
  ```
- Interpretation: 至少静态门禁不能作为 multi-provider 生产逻辑回归证据；工具链和独立 lint 需分开修复。
- Time: 2026-08-24 04:32 +0800

## Evidence E-005: 同步 fixture provider ID 未改变三项代表失败
- Related hypotheses:
  - H-001
- Direction: refutes
- Type: diagnostic
- Source: `core/tests/common/test_codex.rs` 单行诊断改动与三项定向 `cargo test`
- Prediction or plan link:
  - 原 H-001 中“仅 provider ID 未同步”的预测。
- Matched signal:
  - fixture 将 `model_provider_id` 同步为 `openai` 后，Code Mode 仍只发出一次请求、模型切换仍找不到远端模型、Guardian 仍收不到 assessment。
- Correlation keys:
  - `code_mode_can_return_exec_command_output`
  - `model_switch_to_smaller_model_updates_token_context_window`
  - `guardian_review_uses_preferred_review_model_without_model_catalog_override`
- Raw content:
  ```text
  expected two output items, got one
  expected test-text-only-model to be available in remote model list
  expected guardian assessment
  ```
- Interpretation: provider ID 不一致不是充分根因；代码检查显示 legacy manager 即使由 OpenAI provider 创建，仍默认执行 DeepSeek-only 目录过滤，需单独验证该机制。
- Time: 2026-08-24 04:51 +0800

## Evidence E-006: 关闭 legacy DeepSeek-only 过滤单独恢复模型切换
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic
- Source: `models-manager/src/manager.rs` 单行可回滚诊断改动与定向 nextest
- Prediction or plan link:
  - H-001 的过滤机制预测。
- Matched signal:
  - 临时令 legacy `restrict_to_whale_models` 返回 false 后，`model_switch_to_smaller_model_updates_token_context_window` 从稳定失败转为通过；撤销 fixture provider ID 同步后仍通过。Code Mode 与 Guardian 仍失败，划清了机制边界。
- Correlation keys:
  - nextest run `1860845b-48c4-4634-952a-8eba6d0dbba9`
  - nextest run `406ca047-1334-49f4-8a8f-d31e97379711`
- Raw content:
  ```text
  PASS suite::model_switching::model_switch_to_smaller_model_updates_token_context_window
  FAIL suite::code_mode::code_mode_can_return_exec_command_output
  FAIL guardian::tests::guardian_review_uses_preferred_review_model_without_model_catalog_override
  ```
- Interpretation: H-001 已满足证据门禁；修复应只让 OpenAI legacy manager 不过滤，不能全局关闭 DeepSeek-only 视图。
- Time: 2026-08-24 04:55 +0800

## Evidence E-007: Code Mode 与 MCP 失败缺少运行时 helper
- Related hypotheses:
  - H-002
- Direction: supports
- Type: environment
- Source: 定向测试、helper 查找路径、`cargo build` 与 rusty_v8 官方发布资产
- Prediction or plan link:
  - H-002 的失败由缺失 executable 而非业务状态引起预测。
- Matched signal:
  - Code Mode 原始响应为 `unsupported custom tool call: exec`；`effective_tool_mode` 在 host 不可用时退回 Direct。MCP raw failure明确找不到 `test_stdio_server`。两者均不在六 crate 选择集的构建产物中。
- Correlation keys:
  - `code_mode_can_return_exec_command_output`
  - `test_stdio_server`
  - rusty_v8 `v150.4.0`
- Raw content:
  ```text
  unsupported custom tool call: exec
  could not locate binary "test_stdio_server"
  Failed to download .../v150.4.0/librusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.a.gz (HTTP 404)
  ```
- Interpretation: H-002 已满足证据门禁。Code Mode 不是 provider 行为回归；当前 host 构建还命中 OpenAI Codex 上游已报告的 rusty_v8 sandbox asset 404，不能用修改业务断言掩盖。
- Time: 2026-08-24 05:13 +0800

## Evidence E-008: 隔离 runner 预构建 MCP helper 后代表测试通过
- Related hypotheses:
  - H-002
- Direction: supports
- Type: verification
- Source: `scripts/codex-upstream/run_isolated_tests.py` 与定向隔离 nextest
- Prediction or plan link:
  - H-002 的补齐 helper 后原失败恢复预测。
- Matched signal:
  - runner 在包含 `codex-core` 的选择范围内先构建 `test_stdio_server`；脚本单测 7/7 通过，原 MCP 代表测试在隔离环境中通过。
- Correlation keys:
  - nextest run `e35e1c48-ffaa-458c-b87f-afae89e06110`
- Raw content:
  ```text
  Ran 7 tests ... OK
  PASS suite::mcp_refresh_cleanup::refresh_keeps_superseded_mcp_server_alive_for_in_flight_calls
  ```
- Interpretation: MCP helper 缺失的 runner 工程缺口已修复；Code Mode host 仍需等待或绕开上游 rusty_v8 资产问题。
- Time: 2026-08-24 05:25 +0800

## Evidence E-009: Guardian 显式 OpenAI fixture 恢复原失败簇
- Related hypotheses:
  - H-005
- Direction: supports
- Type: diagnostic
- Source: `core/src/guardian/tests.rs` 与隔离 Guardian 定向 nextest
- Prediction or plan link:
  - H-005 的显式绑定 OpenAI 后无需修改生产逻辑即可恢复预测。
- Matched signal:
  - helper 同步 provider ID、registry entry 和 concrete provider 后，原 15 个 Guardian 单测失败只剩 3 个；补齐两个独立手工 fixture 后，Responses API 错误传播测试通过，剩余两项为快照文本差异。
- Correlation keys:
  - 66-test Guardian unit cluster
  - `guardian_review_uses_preferred_review_model_without_model_catalog_override`
  - `guardian_review_surfaces_responses_api_errors_in_rejection_reason`
- Raw content:
  ```text
  before: 15 Guardian unit failures
  after shared fixture binding: 63 passed, 3 failed
  focused follow-up: API error test passed; two request-layout snapshots differed only by Codex -> Whale branding
  ```
- Interpretation: Guardian 生产路径无需修改；根因是上游测试夹具未声明其 OpenAI provider 前提。
- Time: 2026-08-24 06:02 +0800

## Evidence E-010: Guardian 请求快照仅包含既有品牌替换
- Related hypotheses:
  - H-004
- Direction: supports
- Type: snapshot
- Source: 两份 Guardian request-layout `.snap.new` 对照
- Prediction or plan link:
  - H-004 的快照差异应与已接受品牌演进一致且不丢失状态预测。
- Matched signal:
  - 所有内容差异均为 `Codex agent/session` 改为 `Whale agent/session`；请求、历史、cache key、审批动作和 prior rationale 均保持不变。
- Correlation keys:
  - `guardian_review_request_layout_matches_model_visible_request_snapshot`
  - `guardian_reuses_prompt_cache_key_and_appends_prior_reviews`
- Raw content:
  ```text
  The following is the Codex agent history -> The following is the Whale agent history
  Reviewed Codex session id -> Reviewed Whale session id
  The Codex agent has requested -> The Whale agent has requested
  ```
- Interpretation: 这两份快照可以安全更新；证据不外推到尚未审查的 TUI 快照。
- Time: 2026-08-24 06:02 +0800

## Evidence E-011: Guardian 完整单测簇恢复
- Related hypotheses:
  - H-004
  - H-005
- Direction: supports
- Type: verification
- Source: 隔离 nextest Guardian unit filter
- Prediction or plan link:
  - H-005 修复与 H-004 Guardian 子集快照更新后应完整恢复。
- Matched signal:
  - 66 项 Guardian 单测全部通过，无 `.snap.new` 遗留。
- Correlation keys:
  - nextest run `2f2b942a-ae8e-49fc-9bdc-98332a7ceed7`
- Raw content:
  ```text
  Summary [1.012s] 66 tests run: 66 passed, 3685 skipped
  ```
- Interpretation: Guardian 单测失败簇已由 fixture 与预期基线修复闭环，未修改生产运行时。
- Time: 2026-08-24 06:05 +0800

## Evidence E-012: capability endpoint 默认返回 DeepSeek 合同值
- Related hypotheses:
  - H-006
- Direction: supports
- Type: reproduction
- Source: app-server 隔离定向 nextest、core config contract、model-provider unit contract
- Prediction or plan link:
  - H-006 的默认用例前提过期预测。
- Matched signal:
  - 默认 endpoint 连续两次稳定返回 namespace=false、image=false、web=true；core 明确断言默认 `model_provider_id=deepseek`；provider 单测断言相同 capability。
- Correlation keys:
  - nextest run `0c670d26-97b8-4587-a47a-4f1b34c7d424`
- Raw content:
  ```text
  actual default: namespace_tools=false, image_generation=false, web_search=true
  config default: model_provider_id="deepseek", model="deepseek-v4-flash"
  ```
- Interpretation: 这是单一陈旧测试前提，不是 provider runtime 回归；修复需同时保留显式 OpenAI endpoint 覆盖。
- Time: 2026-08-24 06:12 +0800

## Evidence E-013: DeepSeek 默认与显式 OpenAI capability endpoint 均通过
- Related hypotheses:
  - H-006
- Direction: supports
- Type: verification
- Source: app-server capability 文件隔离 nextest
- Prediction or plan link:
  - H-006 的 endpoint 应随显式 provider 正确返回能力预测。
- Matched signal:
  - 默认 DeepSeek、显式 OpenAI、Bedrock 与 Bedrock Runtime 四项全部通过。
- Correlation keys:
  - nextest run `3fca70c9-5d92-43d7-a149-7642f49d93f1`
- Raw content:
  ```text
  Summary [0.217s] 4 tests run: 4 passed, 1237 skipped
  ```
- Interpretation: 更新后的测试同时覆盖 Whale 默认值和 OpenAI 支持，避免以弱化断言换取通过。
- Time: 2026-08-24 06:15 +0800

## Evidence E-014: 插件正向用例提前命中默认关闭门禁
- Related hypotheses:
  - H-007
- Direction: supports
- Type: reproduction
- Source: app-server 全量隔离 nextest、feature registry 与测试 fixture
- Prediction or plan link:
  - H-007 的缺少显式 feature opt-in 预测。
- Matched signal:
  - plugin_share 正向夹具仅写 `plugins=true`，10 个原失败中多数返回 `plugin sharing is disabled`；推荐测试在 tool_suggest=true 分支不写 `recommended_plugins=true`，两项均缺少推荐上下文。
- Correlation keys:
  - nextest run `809e6bfc-92e6-4b2f-9e1b-6d5d1e0fe660`
- Raw content:
  ```text
  Feature::RemotePlugin default_enabled=false
  Feature::PluginSharing default_enabled=false
  Feature::RecommendedPlugins default_enabled=false
  recommended_plugins_mode_for_config requires plugins_enabled && remote_plugin_enabled && ChatGPT auth
  ```
- Interpretation: 测试应声明其 feature 前提；不应为恢复上游测试而改变 Whale 产品默认。
- Time: 2026-08-24 06:27 +0800

## Evidence E-015: 插件正向 fixture 补齐 feature 依赖后模块恢复
- Related hypotheses:
  - H-007
- Direction: supports
- Type: verification
- Source: app-server plugin_share 与 recommended_plugins 隔离 nextest
- Prediction or plan link:
  - H-007 的正向和显式禁用测试应同时通过预测。
- Matched signal:
  - plugin_share 14/14 通过（含两项显式禁用合同）；recommended_plugins 在补齐 `remote_plugin` 后 2/2 通过。
- Correlation keys:
  - nextest run `e8b0d648-5453-4366-a3dd-12ca29c00b95`
  - nextest run `f4b10337-d6aa-4e33-9a76-de5b48b88586`
- Raw content:
  ```text
  plugin_share: 14 passed
  recommended_plugins: 2 passed
  ```
- Interpretation: 12 个原全量失败已闭环；产品默认关闭与不可用错误行为保持不变。
- Time: 2026-08-24 06:31 +0800

## Evidence E-016: plugin-list 失败与缺失 feature 组合一一对应
- Related hypotheses:
  - H-008
- Direction: supports
- Type: diagnostic
- Source: app-server 全量 JUnit、plugin-list fixtures 与 request processor gates
- Prediction or plan link:
  - H-008 的未进入 remote/sharing 路径预测。
- Matched signal:
  - 14 项失败中 remote marketplace 用例均未显式启用 `remote_plugin`；shared-with-me 用例未启用 `plugin_sharing`；startup cache 用例等待从未启动的 remote 请求直至超时。
- Correlation keys:
  - nextest run `809e6bfc-92e6-4b2f-9e1b-6d5d1e0fe660`
- Raw content:
  ```text
  plugin_list failures=14
  representative signals: expected remote marketplace; actual plugin count=0; deadline elapsed waiting for /ps/plugins/list
  ```
- Interpretation: 失败共享 fixture 前提，且与 plugin share 已验证的独立 feature 行为一致。
- Time: 2026-08-24 06:38 +0800

## Evidence E-017: plugin-list 完整模块恢复
- Related hypotheses:
  - H-008
- Direction: supports
- Type: verification
- Source: app-server plugin-list 隔离 nextest
- Prediction or plan link:
  - H-008 的各 feature 组合应同时恢复预测。
- Matched signal:
  - remote enabled/disabled、sharing enabled/disabled、cache TTL、startup refresh、force refetch 与 vertical kind 共 48 项全部通过。
- Correlation keys:
  - nextest run `786b6239-03cb-4d4d-a94c-e82e2be2564c`
- Raw content:
  ```text
  Summary [4.546s] 48 tests run: 48 passed, 1193 skipped
  ```
- Interpretation: 14 个原全量失败已闭环，正交 feature 的正反向合同均保留。
- Time: 2026-08-24 06:42 +0800

## Evidence E-018: 三项 app-server 测试合同修正后通过
- Related hypotheses:
  - H-009
  - H-010
  - H-011
- Direction: supports
- Type: verification
- Source: app-server 三项隔离定向 nextest
- Prediction or plan link:
  - H-009 至 H-011 的最小测试夹具修正预测。
- Matched signal:
  - attribution-only 用例保留导入行为并接受 Whale 品牌；secondary source 同时检测 Sessions 与 Plugins；command/exec 命令成功、子文件持久化且父文件不泄漏。
- Correlation keys:
  - nextest run `ed6bf35e-f7a4-413f-b38c-653c1fe36e90`
- Raw content:
  ```text
  Summary [0.229s] 3 tests run: 3 passed, 1238 skipped
  ```
- Interpretation: 三项均为测试合同或夹具偏差，未发现需要新增产品决策的运行时缺口。
- Time: 2026-08-24 05:17 +0800

## Evidence E-019: 非 OpenAI 投影可稳定复现 inter-agent payload 丢失
- Related hypotheses:
  - H-012
- Direction: supports
- Type: diagnostic
- Source: app-server 冷恢复请求捕获与 core 消息构造、provider projection 源码
- Prediction or plan link:
  - H-012 的 encrypted content 在 non-OpenAI 投影边界丢失预测。
- Matched signal:
  - 稳定 matcher 捕获到目标 `agent_message` 及正确 recipient，但 `Payload:` 后为空；发送端使用 `new_encrypted`，而非 OpenAI 投影仅保留 `InputText`。
- Correlation keys:
  - `/root/worker`
  - `/root/worker/nested`
- Raw content:
  ```text
  Message Type: NEW_TASK
  Task name: <recipient>
  Sender: <sender>
  Payload:
  <empty>
  ```
- Interpretation: 超时表象不是 developer instructions 恢复失败；它暴露了跨 provider 会话消息正文的真实语义丢失。
- Time: 2026-08-24 05:32 +0800

## Evidence E-020: provider-aware 消息表示恢复正文并保留 OpenAI 加密合同
- Related hypotheses:
  - H-012
- Direction: supports
- Type: fix-validation
- Source: codex-core 与 app-server 隔离 nextest
- Prediction or plan link:
  - H-012 的 DeepSeek 明文、OpenAI 加密双路径预测。
- Matched signal:
  - 纯单测验证 OpenAI 目标仍使用 encrypted content、非 OpenAI 目标使用完整结构化文本；三项核心消息测试、两项冷恢复测试及 view-image 子 Agent 测试全部通过；扩大 multi-agent 集合 102/102 通过。
- Correlation keys:
  - nextest run `a59019a0-6f74-4b7e-8e2e-59f41befc296`
  - nextest run `389c61d1-7da7-44e3-a5bf-dcd6db148d31`
  - multi-agent regression filter on final implementation
- Raw content:
  ```text
  focused provider/message tests: 5 passed
  cold resume variants: 2 passed
  expanded multi-agent regression: 102 passed
  ```
- Interpretation: 修复解决了原始 payload 丢失，并用双路径合同防止以禁用 OpenAI 加密为代价换取通过。
- Time: 2026-08-24 05:32 +0800

## Evidence E-021: session approval 已跳过但自动环境中缺少宿主文件
- Related hypotheses:
  - H-013
- Direction: supports
- Type: diagnostic
- Source: app-server file-change approval 隔离 nextest 与 TestAppServer helper
- Prediction or plan link:
  - H-013 的 cwd/environment fixture 分裂预测。
- Matched signal:
  - thread/turn cwd 是 host `workspace`，自动 environment cwd 是另一个临时目录；第二轮没有收到 approval request，但 apply_patch 报 host `workspace/README.md` 不存在并导致 completion 等待超时。
- Correlation keys:
  - nextest run `be5d9def-8a02-4866-aaab-ee8827a4a653`
  - `patch-call-2`
- Raw content:
  ```text
  apply_patch verification failed: Failed to read file to update .../workspace/README.md: No such file or directory
  Error: deadline has elapsed
  ```
- Interpretation: AcceptForSession 已生效；失败位于 host-only fixture 与自动 environment 的文件视图边界。
- Time: 2026-08-24 05:41 +0800

## Evidence E-022: 只有被误品牌化的 commit-only legacy 分支失败
- Related hypotheses:
  - H-014
- Direction: supports
- Type: diagnostic
- Source: app-server git-attribution 隔离 nextest、生产 matcher 与 git blame
- Prediction or plan link:
  - H-014 的测试常量不再匹配真实 legacy 文本预测。
- Matched signal:
  - 完整认证工作区策略和 unlinked PR legacy 分支通过；commit-only 分支 disabled 指令计数为 0。生产常量为 Codex trailer，测试常量由品牌提交单独改成 Whale trailer。
- Correlation keys:
  - nextest run `be5d9def-8a02-4866-aaab-ee8827a4a653`
  - commit `875277b54d`
- Raw content:
  ```text
  commit_only: expected disabled attribution count 1, actual 0
  unlinked_pull_request: PASS
  git_attribution_follows_authenticated_workspace_policy: PASS
  ```
- Interpretation: 当前 Whale attribution 产品行为无需修改；应让 migration fixture 继续代表真实旧 Codex rollout。
- Time: 2026-08-24 05:41 +0800

## Evidence E-023: 文件不可见由 runtime root 决定而非自动环境 cwd
- Related hypotheses:
  - H-013
  - H-015
- Direction: refutes H-013; supports H-015
- Type: diagnostic
- Source: app-server file-change approval 隔离 nextest 对照
- Prediction or plan link:
  - H-013 的禁用自动环境预测与 H-015 的 runtime root 预测。
- Matched signal:
  - 禁用 auto environment 后，在 `/dev/shm` 预先创建 README，首轮 update 仍报文件不存在；使用通过安全检查的 `/var/tmp` 作为唯一变量后，同一诊断用例通过。
- Correlation keys:
  - nextest run `0c2e1cf0-740a-40c0-ac9f-2044f2e26b7b`
  - nextest run `cbce9cee-6e56-4905-a680-34294343a88e`
- Raw content:
  ```text
  /dev/shm: apply_patch verification failed ... README.md: No such file or directory
  /var/tmp: PASS [1.350s]
  ```
- Interpretation: auto environment 不是必要条件；受限文件校验进程看不到 `/dev/shm` 下的测试 workspace，`/var/tmp` 可见。
- Time: 2026-08-24 05:52 +0800

## Evidence E-024: 品牌拆分后 legacy attribution 必须按历史类型分别计数
- Related hypotheses:
  - H-014
- Direction: supports
- Type: diagnostic
- Source: app-server git-attribution 隔离 nextest与品牌前测试合同
- Prediction or plan link:
  - H-014 的真实 Codex legacy fixture 预测。
- Matched signal:
  - 恢复 Codex legacy 输入后 disabled 指令出现；commit-only 请求保留一次 Codex 历史指令、Whale 当前指令为零。品牌前 `COMMIT_ATTRIBUTION` 本就与 Codex legacy 文本相同，因此旧断言计数的是历史指令，不是迁移成新品牌指令。
- Correlation keys:
  - nextest commit-only branch
  - commit `875277b54d`
- Raw content:
  ```text
  commit_only: Whale current count 0; Codex legacy count 1; disabled count 1
  pre-brand test: COMMIT_ATTRIBUTION == Codex legacy trailer
  ```
- Interpretation: 不应修改生产迁移器；测试需恢复历史 fixture，并显式区分历史 Codex 与当前 Whale 指令。
- Time: 2026-08-24 05:54 +0800

## Evidence E-025: runner 与 app-server 目标集合完成回归
- Related hypotheses:
  - H-014
  - H-015
- Direction: supports
- Type: verification
- Source: Python runner 单测与 Codex app-server 隔离 nextest
- Prediction or plan link:
  - H-014/H-015 修复后的目标回归。
- Matched signal:
  - runner 单测覆盖显式无效目录快速失败和 `/var/tmp` 默认选择；原始 file-change approval 用例未改动即恢复，git-attribution 两个 legacy 分支同时通过。
- Correlation keys:
  - nextest run `0e8b37a6-11d5-46a7-9043-fbdeae8469ed`
- Raw content:
  ```text
  runner unittest: 8 passed
  app-server target set: 7 passed, 1234 skipped
  ```
- Interpretation: 修复位于正确工程层，没有通过改写 session approval 产品行为或删除 legacy 覆盖来换取通过。
- Time: 2026-08-24 05:55 +0800

## Evidence E-026: 沙箱可见根目录证伪 command-exec 的隔离写入假设
- Related hypotheses:
  - H-011
  - H-015
- Direction: refutes H-011; supports H-015
- Type: diagnostic
- Source: 完整 app-server 隔离 nextest 与 command-exec 定向对照
- Prediction or plan link:
  - H-011 的父目录写入会成功但不持久化预测。
- Matched signal:
  - runner 切换到 `/var/tmp` 后，去掉 `!` 的测试稳定返回 `Read-only file system` 和 exit 2；这正是原测试要求捕获的权限拒绝。
- Correlation keys:
  - full app-server run
  - nextest run `654520e3-f954-494b-ab82-ee4a38aebe66`
- Raw content:
  ```text
  sh: 1: cannot create ../parent.txt: Read-only file system
  actual exitCode: 2
  ```
- Interpretation: 先前修正受 `/dev/shm` 文件视图异常污染，应恢复原来的负向权限断言。
- Time: 2026-08-24 06:02 +0800

## Evidence E-027: 四项 Code Mode 失败由缺失 host helper 共同解释
- Related hypotheses:
  - H-016
- Direction: supports
- Type: diagnostic
- Source: 完整 app-server nextest、V8 构建日志与带本地资产的目标对照
- Prediction or plan link:
  - H-016 的 runtime helper 缺失预测。
- Matched signal:
  - 完整回归报告 `codex-code-mode-host` 不存在，ImageGen 输出为 Null、analytics 为 hasCell=false；使用资格化缓存中的 V8 archive/binding 构建 host 后，两个 remote-host、ImageGen 和 analytics 四项同时通过。
- Correlation keys:
  - full app-server summary `1235 passed, 5 failed, 1 skipped`
  - nextest run `654520e3-f954-494b-ab82-ee4a38aebe66`
- Raw content:
  ```text
  failed to spawn code-mode host .../target/debug/codex-code-mode-host: No such file
  V8-bound target rerun: four Code Mode dependent tests PASS
  ```
- Interpretation: 四项不是独立产品回归；runner 需要像 MCP helper 一样声明 host，并只接受哈希校验通过的既有 V8 缓存。
- Time: 2026-08-24 06:03 +0800

## Evidence E-028: helper 与权限负向合同的目标回归通过
- Related hypotheses:
  - H-011
  - H-016
- Direction: supports
- Type: verification
- Source: runner Python 单测与 app-server 五项定向 nextest
- Prediction or plan link:
  - H-011 原权限合同恢复与 H-016 helper 修复。
- Matched signal:
  - runner 校验 V8 archive/binding 成对哈希、为 app-server 构建 host；原 command-exec 负向写入、两个 remote host、ImageGen 和 analytics 全部通过。
- Correlation keys:
  - nextest run `6a6c081a-28ec-4c93-96df-ca1408c31872`
- Raw content:
  ```text
  runner unittest: 11 passed
  app-server target set: 5 passed, 1236 skipped
  ```
- Interpretation: 五项失败均在测试基础设施/错误测试改动层闭环，未修改 Code Mode、ImageGen、analytics 或权限生产逻辑。
- Time: 2026-08-24 06:06 +0800

## Evidence E-029: app-server 完整隔离回归清零
- Related hypotheses:
  - H-005
  - H-006
  - H-007
  - H-008
  - H-009
  - H-010
  - H-011
  - H-012
  - H-013
  - H-014
  - H-015
  - H-016
- Direction: supports
- Type: verification
- Source: Codex app-server 完整隔离 nextest
- Prediction or plan link:
  - app-server 原始 52 项失败簇的完整回归。
- Matched signal:
  - 修复 runner 临时根、runtime helper、fixture feature/provider/品牌合同及跨 provider 消息表示后，app-server 全量无失败。
- Correlation keys:
  - full app-server rerun
- Raw content:
  ```text
  Summary [176.399s] 1240 tests run: 1240 passed, 1 skipped
  ```
- Interpretation: app-server 模块已从 52 项基线失败恢复为全绿；剩余发布门禁应继续在 core/TUI/protocol 等模块收敛。
- Time: 2026-08-24 06:10 +0800

## Evidence E-030: core 完整回归将剩余失败聚类到 61 项
- Related hypotheses:
  - H-017
- Direction: supports
- Type: diagnostic
- Source: Codex core 完整隔离 nextest
- Prediction or plan link:
  - app-server 清零后对最大初始失败模块重新基线。
- Matched signal:
  - Code Mode 大簇已恢复；剩余 61 项中 24 项集中于 `tools::spec_plan::tests`，共同缺失 namespace、deferred search、dynamic/hosted 工具。
- Correlation keys:
  - nextest run `0a33cf05-d9ae-404f-bc9e-52bea9bda1fa`
- Raw content:
  ```text
  core: 3743 tests run; 3681 passed; 61 failed; 1 timed out; 9 skipped
  spec_plan: 24 failures
  ```
- Interpretation: runner helper 已消除初始 Code Mode 基础设施失败，最大剩余簇是 provider-sensitive 测试探针。
- Time: 2026-08-24 06:16 +0800

## Evidence E-031: 显式 OpenAI spec-plan fixture 恢复完整模块
- Related hypotheses:
  - H-017
- Direction: supports
- Type: verification
- Source: core spec-plan 隔离 nextest
- Prediction or plan link:
  - H-017 的显式 provider 对照。
- Matched signal:
  - 通用 probe 与三个直接构造 turn 的用例绑定 OpenAI provider 后，原 24 项失败全部恢复，显式 Bedrock 测试仍通过。
- Correlation keys:
  - `tools::spec_plan::tests` filtered rerun
- Raw content:
  ```text
  Summary [0.682s] 49 tests run: 49 passed, 3703 skipped
  ```
- Interpretation: 生产工具规划能力门禁正确；测试不能依赖全局默认 provider。
- Time: 2026-08-24 06:19 +0800
