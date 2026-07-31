# 缓存命中回归门禁已知问题

- Created: 2026-07-31
- Updated: 2026-07-31
- Authority: R8 缓存门禁工程问题的唯一清单
- Source review: [`vs_review/2026-07-31-cache-regression-surface-review.md`](../../../../vs_review/2026-07-31-cache-regression-surface-review.md)

## 1. 当前结论

当前 v1 的根本缺陷不是少写了几个 glob，而是把“相关源码的原始字节发生变化”直接当成“最终 provider 请求发生
变化”。这个代理关系不可靠：

- **漏报**：真实请求构造链位于 glob 外时，最终 payload 已改变而源码指纹不变；
- **误报**：测试、注释、格式或等价重构改变文件字节，但最终 payload 完全不变；
- **过度证明**：固定一个 Flash 样本通过后，结果却可能被解释为整个缓存敏感面已验证；
- **证据错配**：worktree、index、HEAD、合同和结果文件可能不属于同一源码快照。

因此当前门禁只可用于诊断和保守阻断，不具备发布权威性。修复方向是以生产 final wire payload 的确定性变化作为
付费验证触发依据，而不是继续扩大原始文件 glob。

## 2. 唯一问题清单

| ID | 严重度 | 问题 | 已确认表现 | 影响 | 状态 | 修复单元 |
|---|---:|---|---|---|---|---|
| CR-I01 | P0 | 发布门接受未做真实验证的 bootstrap 状态 | `--require-live-baseline` 同时接受 `structural_bootstrap` 和 `live_verified` | 没有 provider 证据也可能通过发布检查 | closed | CR-01 |
| CR-I02 | P0 | index 检查读取了 worktree 合同 | 已复现：index 暂存空 `surface_rules`，worktree 保留旧合同，门禁返回 0 | 实际提交内容与被检查内容不同 | closed | CR-02 |
| CR-I03 | P0 | 晋升与控制面证据可自我授权 | 晋升只检查结果状态、hash 和 `actual_sample_runs <= 2`，没有重验 arm、阈值、证据摘要和 subject identity | 手工或错配结果可能被晋升为可信基线 | closed | CR-03 至 CR-05 |
| CR-I04 | P0 | 真实 DeepSeek wire、Tool serializer 和 usage decoder 未覆盖 | `codex-api` endpoint/SSE 与 `tools/src/tool_spec.rs` 位于当前 18 个 glob 外 | payload 或缓存指标解释改变时门禁可能静默通过 | closed | CR-06、CR-09 至 CR-11 |
| CR-I05 | P0 | 主要上下文和 Tool 选择入口未覆盖 | `session/mod.rs`、`session/turn.rs`、`tools/router.rs` 可改变消息顺序、可见 Tool 和 `tool_choice` | 最容易破坏稳定前缀的变更可能漏报 | closed | CR-06、CR-12 至 CR-17 |
| CR-I06 | P1 | model/provider 路由和请求元数据未覆盖 | provider config、模型默认值和 `models.json` 不在当前合同中 | 模型或 wire API 切换可能沿用无效基线 | closed | CR-18 |
| CR-I07 | P1 | 一个固定付费样本被赋予过宽证明范围 | runner 固定 Flash、`single-file-fast-fix`、Standard + map-request、repeat=1 | Pro、三种 projection、MCP、Skills、权限、压缩等未执行路径无法被证明 | open | CR-21、CR-22 |
| CR-I08 | P1 | 原始文件字节造成付费误报 | 当前 77 个匹配文件中至少 10 个是显式测试文件；注释和格式也进入 hash | 无缓存语义变化的提交会阻断并要求 API 预算 | open | CR-07、CR-08、CR-20 |
| CR-I09 | P0 | 发布证据没有绑定唯一源码快照 | worktree 枚举忽略 untracked，release 记录 HEAD 却检查 dirty worktree | 报告的 commit 不一定是实际测试对象 | closed | CR-05 |

问题总数：**9**；Open：**2**；Closed：**7**。

CR-I01 关闭证据：提交 `6a44bf0f1` 删除 bootstrap 的 release 放行语义；6 个 gate tests 通过，当前普通开发门
保持通过，`--require-live-baseline` 对非 `live_verified` 基线返回退出码 20。未运行真实 Whale Agent。

CR-I02 关闭证据：提交 `0a5866c05` 让 HEAD、index 和 worktree 从各自源码快照读取合同与受检内容，并明确拒绝
合同部分暂存；8 个 gate tests、8 个分析测试和三 source 当前仓库检查通过。未运行真实 Whale Agent。

CR-I03 当前进展：提交 `38fc62830` 完成 CR-03，将门禁政策改动与基线晋升、缓存敏感产品改动强制分开；11 个
gate tests 和 8 个分析测试通过，release 继续阻断。

提交 `c46ebfa05` 完成 CR-04：v2 结果使用唯一运行计划，晋升时交叉复算合同、HEAD、源码面、两臂 artifact、
阈值、证据摘要和账本授权。10 个晋升测试、8 个分析测试和 11 个 gate tests 通过。

提交 `ea8d4a25a` 补齐未来新增控制面 helper 的分类，提交 `b51664a8e` 完成 CR-05：release 固定检查 clean HEAD，
拒绝相关 tracked/untracked 偏差并在结果中记录同一 SHA。17 个 gate tests、10 个晋升测试、8 个分析测试及
PowerShell builder self-test 通过。CR-I03、CR-I09 关闭；未运行真实 Whale Agent。

提交 `d04aab5fb` 完成 CR-06：本地 mock 测试从生产 Session 贯穿到最终 Chat Completions HTTP body，确认
`codex-api/src/endpoint/responses.rs::build_chat_completions_body` 是 endpoint 与 provider wire trace 共用的生产
serializer。定向测试 `1 passed; 0 failed`，未运行真实 Whale Agent。CR-I04、CR-I05 仍需 CR-07 至 CR-17 的
证据覆盖，因此保持 open。

提交 `11d5b2bdd` 完成 CR-07：生产 final-wire body 现在可同时生成原始字节 SHA-256 和完整结构化 JSON 证据；
相同输入、格式变化、字段变化、数组顺序变化及非法 JSON 均有离线测试。公共辅助测试 `3 passed; 0 failed`，生产
捕获测试 `1 passed; 0 failed`，未运行真实 Whale Agent。CR-I08 仍需 CR-08、CR-20 才能关闭。

提交 `2dc401d50` 完成 CR-08：final-wire 比较合同精确保护消息、Tool、`tool_choice`、模型、provider 路由和
未知字段，当前不允许忽略字段；原始 SHA 单独作为完整性证据，避免 JSON 格式变化误报为语义变化。10 个合同
mutation tests 与全部 45 个缓存门禁离线测试通过，未运行真实 Whale Agent。CR-I08 仍需 CR-20 接线后关闭。

提交 `45284b5de` 完成 CR-09：从生产 Chat Completions body 冻结 TaskSpace Tool 顺序、`taskspace_control` 和
普通 `exec_command` 的完整 wire 定义，并断言普通 Tool 在 Standard/TaskSpace 中逐值相同。定向测试
`2 passed; 0 failed`，未运行真实 Whale Agent。CR-I04、CR-I05 仍需后续 decoder 与场景覆盖，因此保持 open。

提交 `01e4cc915` 完成 CR-10：同一版本化 fixture 冻结 Chat Completions 和 Responses API 的 cache hit、miss、
details 缺失及类型错误解码行为。完整 `codex-api` 共 134 个测试通过，未运行真实 Whale Agent。两种 wire 的错误
表现不同，CR-11 必须统一按不可比较处理；CR-I04 暂不关闭。

提交 `c008cab58` 完成 CR-11：Python 直接复用 CR-10 fixture，校验全程与 request 2+ token 恒等式，缺失、错误或
矛盾证据 fail closed；合同版本和 request 2+ token 明细进入 arm 并由晋升器重算。全部 50 个缓存门禁离线测试
通过，未运行真实 Whale Agent。CR-I04 的既定修复单元全部完成，现关闭。

提交 `31f92729e` 完成 CR-12：生产 Standard Session 的连续 request 1/2 完整 final-wire 进入稳定快照，消息前缀、
Tool 集合与 `tool_choice` 有直接断言，已知消息插入可被发现。定向测试通过，未运行真实 Whale Agent。CR-I05
仍需 CR-13 至 CR-17 的 TaskSpace 与条件入口场景，因此保持 open。

提交 `2dd70fe75` 完成 CR-13：同一确定性任务分别通过 map-always、map-append、map-request 的生产 Session 生成
两次完整 final-wire 请求。对应请求的 Tool 集合和 `tool_choice` 必须逐值一致；map-always 每次只携带当前
projection，map-append 的第二次请求保留两版 projection，map-request 默认不注入 projection。三臂快照连续复跑
稳定，定向测试 `4 passed; 0 failed`，未运行真实 Whale Agent。CR-I05 仍需 CR-14 至 CR-17 的条件入口场景。

提交 `7da38b2ed` 完成 CR-14：同一 Standard 两请求任务分别使用默认权限和只读/按需批准权限，完整生产
final-wire 进入独立快照。两组请求各仅包含一个权限区块；精确替换该区块后，两组 wire 逐值完全一致，证明普通
Tool schema 等其他上下文没有被权限 fixture 意外改变。连续两轮稳定复跑及全部 5 个缓存合同测试通过，未运行
真实 Whale Agent。CR-I05 仍需 CR-15 至 CR-17 的 Skill、Apps/Plugins 与 MCP 条件入口场景。

提交 `d43941d2f` 完成 CR-15：显式选择隔离测试 home 中安装的 bundled `skill-creator`，实际 `SKILL.md` 内容和
归一化路径进入两次生产 final-wire 请求。选择 Skill 会新增一条独立 user message，并在后续请求中按自然历史
保留；移除该明确消息后，有/无 Skill 两组 wire 完全一致。连续两轮稳定复跑及全部 6 个缓存合同测试通过，未运行
真实 Whale Agent。CR-I05 仍需 CR-16、CR-17 的 Apps/Plugins 与 MCP 条件入口场景。

提交 `1e5b5c0ba`、`3e0a36aba`、`128b47d88` 和 `d229ac0aa` 完成 CR-15A 至 CR-15C：DeepSeek 内置路径、默认
模型及既有缓存场景已统一到真实 DeepSeek Flash Responses provider；所有请求只命中 `/v1/responses`，完整
final-wire 快照连续两轮稳定。未运行真实 Whale Agent。

提交 `60c8744ef` 完成 CR-16：Default、显式 App 与显式 Plugin 分别生成两次生产 Responses 请求。App 使用
Responses 原生 namespace；Plugin 只增加对应 Skill/Plugin 上下文，普通 Tool 集合与 Default 完全一致。CR-I05
只剩 CR-17 的普通 MCP 条件入口，因此继续保持 open。

提交 `e8a810a0d` 完成 CR-17：本地 `rmcp` fixture 的两次生产 Responses 请求同时冻结 3 个 MCP 资源访问工具和
包含 `echo` 的 `mcp__rmcp__` namespace。MCP on/off 对照证明全部原有 Tool 逐值不变，除新增 MCP Tool 集合外
其他请求字段完全一致；连续两轮定向测试及 8 项缓存场景回归通过。CR-I05 的既定覆盖单元已全部完成，现关闭。

提交 `f4cc55d28` 完成 CR-18：所有缓存场景的 provider identity 改为从实际运行 `Config` 派生，不再使用手写
标签；fixture 明确绑定 `model_provider_id=deepseek`，最终 body 继续冻结 `deepseek-v4-flash`。默认配置、DeepSeek
provider 路由、Flash 可见性及 Pro 无法被远端目录重新启用的定向测试全部通过。CR-I06 现关闭。

## 3. 已验证但不属于门禁缺陷的产品现象

首次真实回归发现 map-request request 2+ 缓存命中率为 `35.79%`，Standard 为 `96.62%`。这是当前 TaskSpace
上下文路径的有效 E3 证据，关联 R8-I02、R8-I08，但不是 CR-I01 至 CR-I09 任一门禁缺陷的关闭证据。

门禁修复不得顺带修改 map-request 上下文产品行为。它只应保证后续相关修改能够被可靠、低误报地发现和验证。

## 4. 影响面证据

当前合同没有完整覆盖以下生产入口：

| 影响面 | 已确认入口 | 可能改变的缓存事实 |
|---|---|---|
| 最终 Responses body | `codex-api/src/endpoint/responses.rs` | body 字段、输入、Tool、`tool_choice` |
| Responses 输入角色与顺序 | `codex-api/src/endpoint/responses.rs` | developer/user 输入和顺序 |
| 缓存 usage 解码 | `codex-api/src/sse/chat_completions.rs`、`codex-api/src/sse/responses.rs` | cached token 观测口径 |
| 上下文组装 | `core/src/session/mod.rs` | developer/user/projection 的位置和重复表达 |
| 请求与 Tool 选择 | `core/src/session/turn.rs` | Prompt、可见 Tool、`tool_choice` |
| 动态 Tool 路由 | `core/src/tools/router.rs` | Tool 集合和顺序 |
| Tool wire schema | `codex-rs/tools/src/tool_spec.rs` | 每次请求携带的 schema |
| provider/model identity | `model-provider-info/src/lib.rs`、`core/src/config/mod.rs`、`models-manager/models.json` | 模型、wire API、能力元数据 |

## 5. 验收边界

门禁问题只有在以下证据同时成立时才能关闭：

1. `index`、`worktree`、`HEAD` 各自检查同一来源的合同和内容，不发生快照混用；
2. release 只接受真实 `live_verified`，且证据绑定精确 commit、场景、模型和 arm；
3. 已知生产入口的变化会自动运行免费 final-payload 场景；
4. 测试、注释和不改变 final payload 的重构不会要求付费运行；
5. payload 变化会指出首个差异、受影响场景和最小建议预算；
6. usage decoder 变化由离线 provider fixture 验证，不用真实 API 掩盖观测错误；
7. 新的空白对抗性审查未发现 blocking 漏报、自授权或高频误报路径。
