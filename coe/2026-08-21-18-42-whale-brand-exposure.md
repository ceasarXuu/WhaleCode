# Problem P-001: Whale 用户界面仍暴露 Codex/OpenAI 品牌
- Status: fixed
- Created: 2026-08-21 18:42
- Updated: 2026-08-21 21:02
- Objective: 复查 v0.0.5 分发身份修复后，Whale 的用户可达 CLI、TUI、doctor、提示和文档入口是否仍错误暴露 Codex/OpenAI 品牌。
- Symptoms:
  - `whale --help` 多个命令仍描述为 Codex；TUI 和 doctor 仍显示 OpenAI Codex/Codex Doctor。
- Expected behavior:
  - Whale 自有产品界面、命令示例、错误提示、状态标题和导出物统一使用 Whale；上游 provenance 和真实第三方服务名称保持准确。
- Actual behavior:
  - npm/release owner 已恢复为 Whale，但 CLI/TUI/doctor 的通用产品品牌 overlay 未完整重放。
- Impact:
  - 用户会误认为正在运行 OpenAI Codex，复制错误的 `codex ...` 命令；Whale 与 Codex substrate 的产品边界不清晰。
- Reproduction:
  - 运行 `whale --help`、`whale doctor --summary --no-color`；检查现行 TUI snapshots 中的 `OpenAI Codex`。
- Environment:
  - Linux；branch `main`；HEAD `87b664ebc`；workspace `whalecode-d17d6279e0`。
- Known facts:
  - 分发身份门禁通过，`@ceasarxuu/whalecode` 与 v0.0.5 正确。
  - 生产源、实际 CLI 输出和现行快照同时包含错误品牌，不是单纯历史文档或缓存二进制。
  - `ChatGPT`、`OpenAI Curated`、OpenAI 文档链接中有一部分绑定真实外部服务，不能仅做字符串替换。
  - `CODEX_HOME`、`.codex`、crate/module 名称属于兼容性和内部技术标识，需要与纯展示品牌分开决策。
- Ruled out:
  - 只存在于 vendor workflow、migration 文档或历史 evidence。
  - 仅因旧二进制未重编译导致。
  - npm/release owner 再次回退。
- Fix criteria:
  - CLI help、doctor、TUI 标题/占位符/通知/导出、daemon remediation 和用户可见命令统一为 Whale；外部服务入口明确标注为集成或禁用；新增品牌门禁；定向 snapshots 与 runtime smoke 通过。
- Current conclusion: 已恢复 Whale 用户界面、运行时提示、分发路径、模型提示词和 Git 归属 overlay，并新增发布品牌门禁；真实 OpenAI/ChatGPT 兼容入口、Codex upstream provenance 与稳定内部协议标识按分类保留。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-001 由 E-001、E-002、E-003 确认；H-002 由 E-004、E-005 确认分类边界。
- Close reason:
  - 根因修复、实际二进制 smoke、定向 Rust 测试、发布门禁、worktree 隔离门禁和缓存 final-wire 门禁均通过。

## Hypothesis H-001: Whale 的用户界面品牌 overlay 在 vendor 同步后仍不完整
- Status: confirmed
- Parent: P-001
- Claim: 当前生产 CLI、TUI 和 doctor 仍直接渲染 Codex/OpenAI 产品品牌及 `codex` 命令，因为 release 修复只覆盖分发路由，没有覆盖通用用户界面品牌。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 如果只是合法 provenance，实际 `whale` 进程和现行 UI snapshots 不应展示这些字符串。
- Falsifiable predictions:
  - If true: 实际 CLI 输出、生产字符串和现行 snapshots 会一致出现 Codex/OpenAI 品牌。
  - If false: 命中只存在于测试、历史文档、内部 symbol 或旧构建产物。
- Diagnostic evidence plan:
  - Prediction or clause under test: 用户可达性与源码一致性。
  - Signal: `whale --help`、doctor 输出、生产 Rust string 和 snapshot rendered output。
  - Capture method: workspace-isolated runtime smoke、`rg`、snapshot inventory。
  - Event name or marker:
    - whale-brand-runtime-audit
  - Correlation keys:
    - HEAD `87b664ebc`
    - version `0.0.5`
  - Differentiates from:
    - H-002
  - Supports if:
    - 两条独立路径均证明品牌字符串实际可达用户。
  - Refutes if:
    - runtime 输出为 Whale 且命中全部不可达。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: 通用产品品牌 overlay 确实不完整，且范围跨 CLI、TUI、doctor 和 daemon。
- Repair design readiness: implemented
- Next step: closed by verified repair
- Blocker:
  - none
- Close reason:
  - repaired and verified by E-006 and E-007

## Hypothesis H-002: 所有 OpenAI/Codex 命中都能直接替换为 Whale
- Status: refuted
- Parent: P-001
- Claim: 仓库内所有 Codex/OpenAI/ChatGPT 字符串都是同一种品牌错误，可以全局替换。
- Layer: contributing
- Factor relation: competing
- Depends on:
  - none
- Rationale:
  - 项目保留 Codex substrate、ChatGPT/OpenAI 服务集成、兼容性环境变量和历史证据。
- Falsifiable predictions:
  - If true: 每个命中都只是 Whale 自称 Codex 的展示文案。
  - If false: 存在真实外部服务名、upstream provenance、兼容性 key、内部 symbol 或测试 fixture。
- Diagnostic evidence plan:
  - Prediction or clause under test: 按运行时角色和所有权分类命中。
  - Signal: URL 目标、配置 key、路径边界、调用位置和文档职责。
  - Capture method: source inspection 与路径分类。
  - Event name or marker:
    - whale-brand-classification
  - Correlation keys:
    - Codex substrate `rust-v0.149.0`
  - Differentiates from:
    - H-001
  - Supports if:
    - 没有服务绑定或兼容性/provenance 命中。
  - Refutes if:
    - 找到必须保留原名或需产品决策的类别。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-005
- Conclusion: 不能全局替换。纯展示品牌应修复；ChatGPT/OpenAI Curated/Cloud 等服务入口需确认保留还是禁用；`CODEX_HOME`/`.codex` 与内部 substrate 标识需兼容性方案。
- Repair design readiness: not applicable
- Next step: closed by evidence
- Blocker:
  - none
- Close reason:
  - refuted by service ownership and compatibility classification

## Evidence E-001: 实际 Whale CLI help 仍自称 Codex
- Related hypotheses:
  - H-001
- Direction: supports
- Type: runtime-state
- Source: workspace-isolated `target/debug/whale --help` 与 `whale doctor --help`
- Prediction or plan link:
  - H-001 的实际用户可达性预测。
- Matched signal:
  - `exec`、`mcp`、`plugin`、`mcp-server`、`update`、`doctor`、`sandbox`、`apply`、`cloud` 和 `--strict-config` 均显示 Codex；配置帮助显示 `~/.codex/config.toml`。
- Correlation keys:
  - binary version `0.0.5`
- Raw content:
  ```text
  Run Codex non-interactively
  Update Codex to the latest version
  Diagnose local Codex installation, config, auth, and runtime health
  Browse tasks from Codex Cloud
  ```
- Interpretation: 这是当前 Whale 二进制的直接输出，属于确定的错误产品品牌露出。
- Time: 2026-08-21 18:40

## Evidence E-002: Whale doctor 运行时标题和修复命令仍为 Codex
- Related hypotheses:
  - H-001
- Direction: supports
- Type: runtime-state
- Source: workspace-isolated `whale doctor --summary --no-color`
- Prediction or plan link:
  - H-001 的 doctor 用户可达性预测。
- Matched signal:
  - 标题为 `Codex Doctor v0.0.5`，末尾要求运行 `codex doctor`；同一报告中的 DeepSeek/Whale 文案证明这是混合品牌而非纯上游工具。
- Correlation keys:
  - version `0.0.5`
- Raw content:
  ```text
  Codex Doctor v0.0.5 · linux-x86_64
  Set DEEPSEEK_API_KEY to a DeepSeek API key before starting Whale.
  Run codex doctor without --summary for detailed diagnostics.
  ```
- Interpretation: doctor 是生产可达的混合身份，且会给用户错误命令。
- Time: 2026-08-21 18:41

## Evidence E-003: 现行 TUI 快照固定渲染 OpenAI Codex
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test-output
- Source: `tui/src/**/snapshots/*.snap` inventory 与对应生产源。
- Prediction or plan link:
  - H-001 的 TUI rendered output 预测。
- Matched signal:
  - 37 个现行 snapshot 文件包含 `OpenAI Codex`、Codex app/version、Codex approval 或 Codex agent 文案；生产源还包含 `Ask Codex to do anything`、Codex 通知和 `# Codex conversation` 导出标题。
- Correlation keys:
  - current main snapshots
- Raw content:
  ```text
  >_ OpenAI Codex (v<VERSION>)
  Ask Codex to do anything
  # Codex conversation
  ```
- Interpretation: TUI 品牌泄漏被现有快照当成期望行为固定下来，需要修改源和基线，而不只是新增字符串扫描。
- Time: 2026-08-21 18:41

## Evidence E-004: 分发 owner 门禁仍通过
- Related hypotheses:
  - H-002
- Direction: neutral
- Type: test-output
- Source: `python3 scripts/release/check_distribution_identity.py`
- Prediction or plan link:
  - 区分分发身份回退和通用品牌 overlay 缺失。
- Matched signal:
  - `distribution identity check OK: all active routes are Whale-owned`。
- Correlation keys:
  - package `@ceasarxuu/whalecode`
- Raw content:
  ```text
  distribution identity check OK: all active routes are Whale-owned
  ```
- Interpretation: npm/release 路由修复有效；当前问题是门禁覆盖范围之外的用户界面品牌，不应回滚前一修复结论。
- Time: 2026-08-21 18:41

## Evidence E-005: 部分命中属于真实服务或兼容性/provenance
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: code-location
- Source: TUI auth/plugin/status 路径、`CODEX_HOME`/`.codex` 配置帮助、migration/ADR/upstream scripts。
- Prediction or plan link:
  - H-002 的全局替换预测。
- Matched signal:
  - ChatGPT 登录、用量页、OpenAI Curated marketplace 和 Codex Cloud 是外部服务绑定；`CODEX_HOME`/`.codex` 是持久状态兼容标识；migration/ADR 和 upstream qualification 明确记录来源。
- Correlation keys:
  - upstream `rust-v0.149.0`
- Raw content:
  ```text
  Sign in with ChatGPT
  OpenAI Curated
  CODEX_HOME
  ~/.codex/config.toml
  ```
- Interpretation: 纯展示可直接 Whale 化；服务入口需要产品决策；兼容性和 provenance 必须保留或迁移，不能机械替换。
- Time: 2026-08-21 18:42

## Evidence E-006: Whale 品牌与分发门禁覆盖修复面并通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test-output
- Source: release identity、distribution identity、brand identity、workspace reference gates 与 release gate unit tests。
- Prediction or plan link:
  - P-001 fix criteria。
- Matched signal:
  - WhaleCode v0.0.5 与 Codex substrate rust-v0.149.0 被分别登记；活动分发路由均归属 Whale；用户可见源码通过品牌扫描；14 个门禁单测通过；workspace reference gate 通过。
- Correlation keys:
  - branch `main`
  - workspace `whalecode-d17d6279e0`
- Interpretation: 发布身份、用户品牌和 worktree 隔离均有自动化回归保护。
- Time: 2026-08-21 21:02

## Evidence E-007: 实际 Whale 二进制与缓存 final-wire 验证通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: runtime-state
- Source: workspace-isolated `target/debug/whale` smoke 与 staged-index cache regression gate。
- Prediction or plan link:
  - P-001 fix criteria。
- Matched signal:
  - `whale 0.0.5`；根帮助、plugin/marketplace、login、doctor 均使用 Whale/DeepSeek 路径；Cloud 明确禁用；缓存门禁以指纹 `9c817b9f59426efa097be43988d4731a2e7ba412bad63bdf69a466fcbcaaaced` 完成免费 final-wire 验证。
- Correlation keys:
  - version `0.0.5`
  - cache fingerprint `9c817b9f59426efa097be43988d4731a2e7ba412bad63bdf69a466fcbcaaaced`
- Interpretation: 修复已进入实际运行时，且模型提示词变化未破坏 provider 缓存前缀合同。
- Time: 2026-08-21 21:02
