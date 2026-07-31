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
| CR-I04 | P0 | 真实 DeepSeek wire、Tool serializer 和 usage decoder 未覆盖 | `codex-api` endpoint/SSE 与 `tools/src/tool_spec.rs` 位于当前 18 个 glob 外 | payload 或缓存指标解释改变时门禁可能静默通过 | open | CR-06、CR-09 至 CR-11 |
| CR-I05 | P0 | 主要上下文和 Tool 选择入口未覆盖 | `session/mod.rs`、`session/turn.rs`、`tools/router.rs` 可改变消息顺序、可见 Tool 和 `tool_choice` | 最容易破坏稳定前缀的变更可能漏报 | open | CR-06、CR-12 至 CR-17 |
| CR-I06 | P1 | model/provider 路由和请求元数据未覆盖 | provider config、模型默认值和 `models.json` 不在当前合同中 | 模型或 wire API 切换可能沿用无效基线 | open | CR-18 |
| CR-I07 | P1 | 一个固定付费样本被赋予过宽证明范围 | runner 固定 Flash、`single-file-fast-fix`、Standard + map-request、repeat=1 | Pro、三种 projection、MCP、Skills、权限、压缩等未执行路径无法被证明 | open | CR-21、CR-22 |
| CR-I08 | P1 | 原始文件字节造成付费误报 | 当前 77 个匹配文件中至少 10 个是显式测试文件；注释和格式也进入 hash | 无缓存语义变化的提交会阻断并要求 API 预算 | open | CR-07、CR-08、CR-20 |
| CR-I09 | P0 | 发布证据没有绑定唯一源码快照 | worktree 枚举忽略 untracked，release 记录 HEAD 却检查 dirty worktree | 报告的 commit 不一定是实际测试对象 | closed | CR-05 |

问题总数：**9**；Open：**5**；Closed：**4**。

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

## 3. 已验证但不属于门禁缺陷的产品现象

首次真实回归发现 map-request request 2+ 缓存命中率为 `35.79%`，Standard 为 `96.62%`。这是当前 TaskSpace
上下文路径的有效 E3 证据，关联 R8-I02、R8-I08，但不是 CR-I01 至 CR-I09 任一门禁缺陷的关闭证据。

门禁修复不得顺带修改 map-request 上下文产品行为。它只应保证后续相关修改能够被可靠、低误报地发现和验证。

## 4. 影响面证据

当前合同没有完整覆盖以下生产入口：

| 影响面 | 已确认入口 | 可能改变的缓存事实 |
|---|---|---|
| 最终 Chat Completions body | `codex-api/src/endpoint/responses.rs::build_chat_completions_body` | body 字段、消息、Tool、`tool_choice` |
| 角色转换 | `codex-api/src/endpoint/chat_completions.rs` | developer/system 角色和消息顺序 |
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
