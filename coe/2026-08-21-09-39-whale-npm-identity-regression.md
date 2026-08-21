# Problem P-001: Whale npm 发布身份在 Codex vendor 同步后回退
- Status: fixed
- Created: 2026-08-21 09:39
- Updated: 2026-08-21 18:30
- Objective: 解释已独立发布的 Whale npm 包为什么在当前仓库中变回 OpenAI Codex 包配置，并审计同类分发面回退。
- Symptoms:
  - 当前 `third_party/codex-cli/codex-cli/package.json` 声明 `@openai/codex`，而 Whale 之前已独立发布 npm 包。
- Expected behavior:
  - Whale 仓库保持 `@ceasarxuu/whalecode`、`whale` bin 和 Whale 自有 repository/release identity。
- Actual behavior:
  - 当前 package、bin launcher 和 vendor release workflow 是 OpenAI Codex 上游身份。
- Impact:
  - v0.0.5 不能沿当前 npm 路径安全发布；直接使用 vendor workflow 可能错误指向 OpenAI 包、R2、WinGet 或网站部署面。
- Reproduction:
  - 对比 `c162013eb`、`720abe529^`、`720abe529` 和当前 main 的 `third_party/codex-cli/codex-cli/package.json`。
- Environment:
  - Linux；branch `main`；诊断 HEAD `7c6e72802`；npm registry 查询时间 2026-08-21。
- Known facts:
  - `94dd5695d` 首先把 npm 包改名为 `whalecode`；`c162013eb` 改为 `@ceasarxuu/whalecode`。
  - npm registry 公开记录 `@ceasarxuu/whalecode@0.0.1-dev`，bin 为 `whale -> bin/whale.js`。
  - `720abe529` 的 0.147 vendor cutover 将 package、launcher 和 release workflow 回退为上游 Codex 版本。
  - 0.149 同步及 main 合并保留了该回退状态。
  - 回退范围还覆盖已编译进 Whale 的自动更新/doctor 路径，以及 vendor 内的安装器、GitHub Release、R2、WinGet、代码签名和 SDK 发布配置。
  - vendor 内 `.github/workflows` 在当前 Whale 仓库不会自动触发，但不能直接作为 Whale 发布工作流启用或复制。
- Ruled out:
  - Whale 独立 npm 包从未发布。
  - 当前仓库另有一套仍有效的 Whale npm package manifest。
- Fix criteria:
  - 用户授权修复后，恢复并验证 Whale 独立 package identity、launcher、平台包构建和只指向 Whale 资产的发布入口；加入同步/发布回归门禁；原始安装 smoke 通过。
- Current conclusion: 这是 0.147 vendor cutover 中未重放 Whale 分发 overlay 导致的系统性回归，不是 npm registry 迁移或 OpenAI 接管；Whale npm identity、launcher、更新/支持路由已恢复，未拥有的 standalone/Desktop/多渠道发布入口已显式禁用或隔离，并由根级 CI 门禁保护。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - Root cause and distribution blast radius diagnosed by H-001/H-003 and E-001 through E-011；H-002/H-004 排除了替代解释和误报范围；E-012 through E-015 验证 Whale-owned 修复、渠道隔离、跨平台门禁和真实 launcher smoke。
- Close reason:
  - 修复标准满足：`@ceasarxuu/whalecode@0.0.5` 可独立 staging，`whale.js` 启动真实 `whale 0.0.5`，活跃分发面不再指向 OpenAI，回归门禁已接入 main CI。

## Hypothesis H-001: 0.147 vendor cutover 覆盖并遗漏 Whale npm overlay
- Status: confirmed
- Parent: P-001
- Claim: `720abe529` 应用大规模 Codex 0.147 vendor 内容时，把已发布的 Whale npm package/launcher/release 配置替换为 OpenAI 版本；后续最小 overlay 恢复范围未包含 npm 发布身份，因此回归持续到 main。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 症状出现在 vendor 内 package 和 workflow，多项文件同时回退，符合一次 substrate cutover 覆盖而非单字段误改。
- Falsifiable predictions:
  - If true: cutover 前 manifest 是 `@ceasarxuu/whalecode`，cutover commit 同时改回 `@openai/codex`、`codex.js` 和上游 repository，之后没有完整恢复 Whale npm overlay。
  - If false: package 在 cutover 前已是 OpenAI，或 cutover 后存在另一 Whale package/release authority。
- Diagnostic evidence plan:
  - Prediction or clause under test: 精确比较回归窗口前后 package、launcher、workflow，并检查之后的 package 历史和 overlay ledger。
  - Signal: Git object diff、path log、当前 config 和 overlay metadata。
  - Capture method: `git show`、`git diff 720abe529^ 720abe529`、`git log -- <paths>`、`rg`。
  - Event name or marker:
    - npm-identity-regression-window
  - Correlation keys:
    - commits `c162013eb`、`720abe529`、`0044ffee5`、`972252d7a`
  - Differentiates from:
    - H-002
  - Supports if:
    - 单一 cutover commit 将全部 Whale npm identity 改为 OpenAI 且后续没有独立 Whale manifest。
  - Refutes if:
    - 当前存在另一条有效 Whale npm 发布路径，或回退发生在 npm registry 外部。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
  - E-004
  - E-005
- Conclusion: 0.147 vendor cutover 覆盖了 Whale npm/release overlay，发布身份未被纳入后续最小重放和门禁。
- Repair design readiness: implemented and validated
- Next step: 只在建立并审批 Whale 自有 native artifact workflow 后执行正式 npm 发布；当前不得启用 vendor 上游发布模板。
- Blocker:
  - none
- Close reason:
  - repaired by Whale distribution overlay and regression guard

## Hypothesis H-002: Whale npm 配置仍在另一发布入口，当前 OpenAI 文件只是无效 vendor 快照
- Status: refuted
- Parent: P-001
- Claim: 当前 `@openai/codex` package 不影响 Whale 发布，因为仓库其他位置仍有一套有效的 `@ceasarxuu/whalecode` manifest/workflow。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - vendor 快照可能保留上游文件，而产品发布入口可能位于根仓库或独立包装层。
- Falsifiable predictions:
  - If true: 当前树能找到 Whale package name、whale.js launcher 或独立 npm publish workflow。
  - If false: 当前唯一 CLI npm manifest 是 `@openai/codex`，Whale identity 只存在于历史和 registry。
- Diagnostic evidence plan:
  - Prediction or clause under test: 搜索当前 package manifests、workflow、publish 命令和 Whale launcher。
  - Signal: 当前文件清单和内容。
  - Capture method: `rg`、`rg --files`、Git path history。
  - Event name or marker:
    - current-npm-authority-search
  - Correlation keys:
    - main `7c6e72802`
  - Differentiates from:
    - H-001
  - Supports if:
    - 找到完整、可执行且指向 `@ceasarxuu/whalecode` 的当前发布链。
  - Refutes if:
    - 当前树不存在该发布链。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-005
- Conclusion: 当前树不存在仍有效的独立 Whale npm manifest/workflow；上游 vendor 文件已成为实际唯一候选路径。
- Repair design readiness: not applicable
- Next step: closed by evidence
- Blocker:
  - none
- Close reason:
  - refuted by current-tree search and history

## Evidence E-001: 历史提交建立 Whale 独立 scoped package
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `git show 94dd5695d`、`git show c162013eb`
- Prediction or plan link:
  - H-001 cutover 前 manifest 是 Whale identity。
- Matched signal:
  - `c162013eb` 的 name 为 `@ceasarxuu/whalecode`，bin 为 `whale: bin/whale.js`，repository 指向 WhaleCode。
- Correlation keys:
  - commits `94dd5695d`、`c162013eb`
- Raw content:
  ```text
  94dd5695d Rename npm package to whalecode
  c162013eb Publish scoped npm dev package
  "name": "@ceasarxuu/whalecode"
  "whale": "bin/whale.js"
  ```
- Interpretation: Whale npm identity 是明确提交过的产品配置，不是口头或未落地意图。
- Time: 2026-08-21 09:40

## Evidence E-002: npm registry 保留 Whale 独立发布记录
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: config
- Source: `npm view @ceasarxuu/whalecode name version dist-tags time repository bin --json`
- Prediction or plan link:
  - H-002 的替代解释“Whale 包从未真实发布”。
- Matched signal:
  - registry 返回公开包和四个 dev 版本时间记录。
- Correlation keys:
  - package `@ceasarxuu/whalecode`
- Raw content:
  ```text
  name: @ceasarxuu/whalecode
  version/latest: 0.0.1-dev
  created: 2026-04-27T17:08:37.292Z
  bin: whale -> bin/whale.js
  repository: github.com/ceasarXuu/WhaleCode.git
  ```
- Interpretation: 用户关于“之前已经独立发布”的陈述得到 registry 直接证实。
- Time: 2026-08-21 09:41

## Evidence E-003: 0.147 cutover 同一提交完成身份回退
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `git diff 720abe529^ 720abe529 -- third_party/codex-cli/codex-cli/package.json` 及 path status
- Prediction or plan link:
  - H-001 预测 package、launcher 和 workflow 在同一 cutover 回退。
- Matched signal:
  - scoped name、description、bin、files、repository 同时改回 OpenAI；`whale.js` rename 为 `codex.js`；release workflow 和 npm build script 同批更新。
- Correlation keys:
  - commit `720abe529`
- Raw content:
  ```text
  - "name": "@ceasarxuu/whalecode"
  + "name": "@openai/codex"
  - "whale": "bin/whale.js"
  + "codex": "bin/codex.js"
  R061 codex-cli/bin/whale.js codex-cli/bin/codex.js
  M    .github/workflows/rust-release.yml
  D    BUILD_NUMBER
  ```
- Interpretation: 回归机制是可定位的单次 vendor cutover，不是后来 npm 服务端改名。
- Time: 2026-08-21 09:41

## Evidence E-004: 同步计划主动排除了 Whale 发布框架恢复
- Related hypotheses:
  - H-001
- Direction: supports
- Type: config
- Source: `docs/v0.0.5/codex-upstream-sync/plan.md:256` 及当前 overlay replay ledger 搜索
- Prediction or plan link:
  - H-001 预测后续最小 overlay 不包含 npm/release identity。
- Matched signal:
  - 计划写明“不复制或改写 vendor 内的上游 CI/release workflow，也不新增 Whale 发布框架”；当前 replay ledger 没有 CLI package/whale.js/release workflow 条目。
- Correlation keys:
  - Phase E / U19
- Raw content:
  ```text
  不复制或改写 vendor 内的上游 CI/release workflow，也不新增 Whale 发布框架。
  overlay-replay-ledger: no codex-cli/package.json, whale.js, or rust-release.yml entry
  ```
- Interpretation: npm 发布身份被排除在恢复范围之外，也没有门禁确保历史发布合同仍在。
- Time: 2026-08-21 09:42

## Evidence E-005: 当前 main 只剩 OpenAI CLI npm manifest
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: config
- Source: current-tree `package.json`/workflow search and Git path log
- Prediction or plan link:
  - H-001 持续性预测；H-002 当前独立入口预测。
- Matched signal:
  - 当前 CLI manifest 为 `@openai/codex`；720 之后该 path 仅由 0.149 vendor 同步再次修改，仍未恢复 Whale package；根级仅有离线 identity CI，没有 npm publish workflow。
- Correlation keys:
  - main `7c6e72802`
- Raw content:
  ```text
  current name: @openai/codex
  current bin: codex -> bin/codex.js
  0044ffee5 chore(sync): rebase codex vendor to 0.149
  ```
- Interpretation: 上游配置不是无效旁路，而是当前唯一可见 CLI npm package 候选，因此 v0.0.5 发布准备确实缺失 Whale npm 链。
- Time: 2026-08-21 09:42

## Evidence E-006: 请求者确认历史发布预期
- Related hypotheses:
  - H-001
- Direction: supports
- Type: user-feedback
- Source: current conversation
- Prediction or plan link:
  - H-001 的 expected behavior 与实际产品历史一致。
- Matched signal:
  - 请求者指出 WhaleCode 已在自己的 npm 独立发布。
- Correlation keys:
  - user report 2026-08-21
- Raw content:
  ```text
  whalecode 之前已经在自己的npm 独立发布过了，怎么现在跑到openAI去了
  ```
- Interpretation: 用户预期与 Git/registry 证据一致，并暴露了发布准备盘点中的错误假设。
- Time: 2026-08-21 09:39

## Hypothesis H-003: 分发身份回退不止 npm manifest，并已进入 Whale 可执行运行时
- Status: confirmed
- Parent: P-001
- Claim: 同一未重放的 Whale 分发 overlay 还使自动更新、doctor、安装器和多渠道发布配置保留 OpenAI Codex 目标，其中部分路径已编译进 Whale CLI。
- Layer: contributing
- Factor relation: amplifying
- Depends on:
  - H-001
- Rationale:
  - package manifest、更新检测、安装命令和发布流水线共享 package/repository/artifact identity，vendor cutover 可能同时覆盖这些面。
- Falsifiable predictions:
  - If true: 生产 Rust 路径会查询或执行 OpenAI npm、GitHub、Homebrew、ChatGPT installer；vendor 发布链会指向 OpenAI npm scope、R2、WinGet 或签名标识。
  - If false: OpenAI 配置仅存在于测试、文档或不可执行的 upstream provenance 中。
- Diagnostic evidence plan:
  - Prediction or clause under test: 区分编译进 Whale 的生产路径、当前根工作流和仅保留的 vendor 发布模板。
  - Signal: 非测试源中的 URL/包名/命令、GitHub workflow 位置和根入口引用。
  - Capture method: `rg`、目标文件阅读、根 `.github/workflows` 枚举、现有发布门禁执行。
  - Event name or marker:
    - distribution-openai-target-audit
  - Correlation keys:
    - main `715f91909`
  - Differentiates from:
    - H-004
  - Supports if:
    - 存在实际生产代码或可发布模板把 Whale 用户/制品导向 OpenAI 渠道。
  - Refutes if:
    - 所有命中都仅是合法 upstream 来源或测试夹具。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-007
  - E-008
  - E-009
  - E-010
  - E-011
- Conclusion: 回退横跨运行时更新、npm 构建、安装器和 vendor 多渠道发布模板；最高优先级是已编译进 Whale 的更新/doctor 路径。
- Repair design readiness: implemented and validated
- Next step: 保持 standalone、Desktop、R2、WinGet、Homebrew、签名和网站渠道禁用，直到各自具备 Whale-owned 资产与凭据合同。
- Blocker:
  - none
- Close reason:
  - active runtime routes repaired; unowned channels quarantined

## Hypothesis H-004: 仓库内所有 OpenAI 分发相关字样都必须改成 Whale
- Status: refuted
- Parent: P-001
- Claim: 所有 `openai/codex`、OpenAI URL 或 Codex artifact 引用都属于 Whale 分发错误。
- Layer: root-cause
- Factor relation: competing
- Depends on:
  - none
- Rationale:
  - 仓库以 Codex upstream 为 substrate，合法的来源追踪、同步资格验证、测试夹具和未启用 vendor 快照也会保留上游身份。
- Falsifiable predictions:
  - If true: 每个命中都会被 Whale 运行时或发布入口消费。
  - If false: 能找到仅用于 upstream provenance、同步验证或历史证据的命中。
- Diagnostic evidence plan:
  - Prediction or clause under test: 按生产可达性和发布入口分类命中。
  - Signal: 根工作流引用关系、文件职责、测试/证据目录边界。
  - Capture method: `find .github/workflows`、`rg`、同步计划和 runbook 阅读。
  - Event name or marker:
    - openai-reference-classification
  - Correlation keys:
    - Codex substrate `rust-v0.149.0`
  - Differentiates from:
    - H-003
  - Supports if:
    - 所有命中均可达 Whale 分发。
  - Refutes if:
    - 部分命中是保留上游可追溯性的必要配置或不可执行证据。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-009
- Conclusion: 不能全局替换 OpenAI/Codex；必须只修 Whale 产品运行时和 Whale 发布入口，保留 upstream provenance 与资格验证身份。
- Repair design readiness: not applicable
- Next step: closed by evidence
- Blocker:
  - none
- Close reason:
  - refuted by reachability and provenance classification

## Evidence E-007: Whale 生产代码中的更新动作仍指向 OpenAI
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `codex-rs/tui/src/update_action.rs`、`updates.rs`、`npm_registry.rs`、`cli/src/doctor/updates.rs`、`cli/src/doctor.rs`、`app-server-daemon/src/update_loop.rs`
- Prediction or plan link:
  - H-003 预测生产更新/doctor 路径会消费 OpenAI 渠道。
- Matched signal:
  - 实际命令执行 `npm|bun|pnpm ... @openai/codex`、`brew upgrade --cask codex` 或 `chatgpt.com/codex/install.*`；版本检测查询 OpenAI GitHub/npm/Homebrew/桌面 appcast，doctor 固定 npm 路径 `@openai/codex` 和 Windows identity `OpenAI.Codex`；`whale app` 的平台实现会下载/打开 OpenAI Codex Desktop。
- Correlation keys:
  - npm package `@openai/codex`
  - GitHub repo `openai/codex`
- Raw content:
  ```text
  update_action.rs:44 npm install -g @openai/codex
  update_action.rs:52 https://chatgpt.com/codex/install.sh
  updates.rs:62 https://api.github.com/repos/openai/codex/releases/latest
  npm_registry.rs:5 https://registry.npmjs.org/@openai%2fcodex
  doctor/updates.rs:341 package_identity != "OpenAI.Codex"
  doctor.rs:1085 npm_root.join("@openai").join("codex")
  desktop_app/mac.rs:9 https://persistent.oaistatic.com/codex-app-prod/Codex.dmg
  desktop_app/windows.rs:7 https://get.microsoft.com/installer/download/9PLM9XGG6VKS
  ```
- Interpretation: 这是用户运行 Whale 时可触达的错误更新路径，不只是待启用的发布模板。
- Time: 2026-08-21 10:08

## Evidence E-008: npm 构建、归档下载和 standalone installer 构成完整 OpenAI 链
- Related hypotheses:
  - H-003
- Direction: supports
- Type: config
- Source: `codex-cli/package.json`、`bin/codex.js`、`build_npm_package.py`、`scripts/stage_npm_packages.py`、`scripts/install/install.sh`、`install.ps1`
- Prediction or plan link:
  - H-003 预测 package 之外仍有同身份的构建与安装配置。
- Matched signal:
  - 六个平台包、launcher、stager、release asset 名和 Unix/Windows installer 全部使用 `@openai/codex`、`openai/codex`、`rust-v*` 或 `releases.openai.com/codex`。
- Correlation keys:
  - package/artifact family `codex-*`
- Raw content:
  ```text
  CODEX_NPM_NAME = "@openai/codex"
  GITHUB_REPO = "openai/codex"
  RELEASES_BASE_URL = "https://releases.openai.com/codex"
  ```
- Interpretation: 直接沿用这些脚本会下载、打包或卸载 Codex，不能发布 Whale v0.0.5。
- Time: 2026-08-21 10:10

## Evidence E-009: vendor 多渠道发布模板保留 OpenAI 专属目标，但当前不会自动触发
- Related hypotheses:
  - H-003
  - H-004
- Direction: supports
- Type: config
- Source: `third_party/codex-cli/.github/workflows/rust-release.yml`、`r2-release.yml`、`.github/scripts/publish_r2_release.py`、根 `.github/workflows` 枚举
- Prediction or plan link:
  - H-003 的多渠道预测；H-004 的可达性分类。
- Matched signal:
  - vendor workflow 配置 npm scope `@openai`、签名 ID `com.openai.codex.*`、developers.openai.com deploy hook、WinGet `OpenAI.Codex`/`openai-oss-forks`、CODEX_R2 secrets 和 `releases.openai.com/codex`；同时保留 `@openai/codex-sdk`、`@openai/codex-responses-api-proxy`、`openai-codex` 和 `openai-codex-cli-bin` 发布身份；根工作流只有 `release-identity.yml`，未调用这些 vendor workflows。
- Correlation keys:
  - vendor workflow path boundary
- Raw content:
  ```text
  scope: "@openai"
  identifier: OpenAI.Codex
  fork-user: openai-oss-forks
  REPOSITORY = "openai/codex"
  https://releases.openai.com/codex/releases/...
  root workflow: .github/workflows/release-identity.yml only
  ```
- Interpretation: 这些配置当前是“休眠但危险”的上游模板；不能误称已在 Whale 仓库自动发布，但任何启用/复制都会发错渠道。upstream 来源追踪本身应保留。
- Time: 2026-08-21 10:13

## Evidence E-010: 现有发布门禁不能阻止上述回归
- Related hypotheses:
  - H-003
- Direction: supports
- Type: test-output
- Source: `scripts/check-codex-collision-risk.ps1`、`scripts/check-build-profile-policy.ps1`、`scripts/release/check_release_identity.py`
- Prediction or plan link:
  - H-003 的回归持续性与控制缺口。
- Matched signal:
  - release identity v0.0.5/Codex 0.149 检查和 5 个单测通过，但只覆盖版本身份；collision guard 默认模式在 Linux 因 `USERPROFILE`/安装路径隔离检查提前失败，以 `-SkipCliPathCheck` 运行时能检出 npm identity 回退；build profile guard 因 `tui/build.rs`/`BUILD_NUMBER` 已被同步删除而失败；根 CI 只运行 release identity 检查。
- Correlation keys:
  - v0.0.5 preflight
- Raw content:
  ```text
  release identity check OK: WhaleCode v0.0.5; Codex substrate rust-v0.149.0
  Ran 5 tests ... OK
  check-cli-isolation.ps1: USERPROFILE is null
  check-codex-collision-risk.ps1 -SkipCliPathCheck: npm CLI package must be named @ceasarxuu/whalecode
  check-build-profile-policy.ps1: tui/build.rs does not exist
  ```
- Interpretation: 新增的版本号门禁正确区分了 0.0.5 与 0.149，但尚未覆盖分发 owner/package/channel identity；旧 Whale guard 仍含有有价值的 npm 断言，但默认可运行性、覆盖范围和 CI 接线均不足。
- Time: 2026-08-21 10:16

## Evidence E-011: TUI 远程公告和用户反馈仍由 OpenAI 渠道控制
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `codex-rs/tui/src/tooltips.rs`、`announcement_tip.toml`、`history_cell/notices.rs`、`bottom_pane/feedback_view.rs`
- Prediction or plan link:
  - H-003 的用户可达分发/引导面预测。
- Matched signal:
  - TUI 从 `raw.githubusercontent.com/openai/codex/main/announcement_tip.toml` 获取公告，展示 Codex Desktop/ChatGPT 安装引导、OpenAI Codex release notes，并把外部反馈送到 `openai/codex` issue。
- Correlation keys:
  - user-facing routing
- Raw content:
  ```text
  tooltips.rs:7 raw.githubusercontent.com/openai/codex/main/announcement_tip.toml
  tooltips.rs:12 https://chatgpt.com/codex?app-landing-page=true
  notices.rs:46 https://github.com/openai/codex/releases/latest
  feedback_view.rs:34 https://github.com/openai/codex/issues/new?template=3-cli.yml
  ```
- Interpretation: 这不是制品上传目标，但会把 Whale 用户引导到 OpenAI 的安装、公告、release 和支持渠道；其中远程公告还允许上游在 Whale TUI 中动态改变文案，应作为独立产品边界修复。
- Time: 2026-08-21 10:21

## Evidence E-012: Whale npm identity 与真实 launcher smoke 已恢复
- Related hypotheses:
  - H-001
  - H-003
- Direction: supports
- Type: test-output
- Source: `build_npm_package.py --package whalecode --release-version 0.0.5`；staged `bin/whale.js --version`
- Prediction or plan link:
  - Fix criteria 的 package identity、launcher 和原始进程 smoke。
- Matched signal:
  - staged manifest 为 `@ceasarxuu/whalecode@0.0.5`，bin 为 `whale -> bin/whale.js`；launcher 从 Whale vendor layout 启动本地编译二进制并返回 `whale 0.0.5`。
- Correlation keys:
  - package `@ceasarxuu/whalecode`
  - stage `/tmp/whale-npm-smoke.U1xW4H/package`
- Raw content:
  ```text
  Staged version 0.0.5 for release
  whale 0.0.5
  ```
- Interpretation: npm 元包身份和跨平台 launcher 合同已重新落到 Whale，不再依赖 `@openai/codex` 或 `codex.js`。
- Time: 2026-08-21 18:12

## Evidence E-013: 活跃分发面和根 CI 门禁通过
- Related hypotheses:
  - H-003
- Direction: supports
- Type: test-output
- Source: `scripts/release/check_distribution_identity.py`、PowerShell compatibility guards、`.github/workflows/release-identity.yml`
- Prediction or plan link:
  - Fix criteria 的 Whale-only 发布入口和同步/发布回归门禁。
- Matched signal:
  - 门禁验证 manifest、launcher、六个平台 alias、更新/doctor/公告/支持路径、standalone/Desktop 禁用合同、vendor workflow 隔离；根 workflow 在 push/PR 执行门禁及单测。
- Correlation keys:
  - CI workflow `release-identity`
- Raw content:
  ```text
  distribution identity check OK: all active routes are Whale-owned
  Codex collision risk check OK
  Build profile policy check OK
  ```
- Interpretation: 同类 vendor 同步回退会在合并前失败，不再只依赖人工审计。
- Time: 2026-08-21 18:25

## Evidence E-014: 分发门禁负向与发布身份测试通过
- Related hypotheses:
  - H-003
- Direction: supports
- Type: test-output
- Source: `python3 -m unittest discover -s scripts/release/tests -p 'test_*.py'`
- Prediction or plan link:
  - 门禁必须拒绝 OpenAI runtime target、vendor release workflow 激活和 `codex.js` launcher，同时保留 Whale 0.0.5 / Codex substrate 0.149 双版本语义。
- Matched signal:
  - 10 个 release tests 全部通过（含六个平台包 staging）；release identity 输出 WhaleCode v0.0.5 与 Codex substrate rust-v0.149.0。
- Correlation keys:
  - Whale release `v0.0.5`
  - substrate `rust-v0.149.0`
- Raw content:
  ```text
  release identity check OK: WhaleCode v0.0.5; Codex substrate rust-v0.149.0
  Ran 10 tests ... OK
  ```
- Interpretation: 149 不会再被登记为 Whale 版本，分发 owner 回退也有可证伪测试。
- Time: 2026-08-21 18:25

## Evidence E-015: 受影响 Rust 和 installer 行为验证通过
- Related hypotheses:
  - H-003
- Direction: supports
- Type: test-output
- Source: targeted Cargo tests、release-mode update prompt test、`test_install_sh.py`
- Prediction or plan link:
  - 更新动作、公告/支持文案、doctor 和未拥有 installer 的安全行为保持可编译可测试。
- Matched signal:
  - update action 2、tooltips 14、history 1、feedback 16、doctor updates 2 项测试通过；release-mode update prompt 1 项通过；installer refusal 1 项通过；Cargo fmt 通过。
- Correlation keys:
  - Rust crates `codex-tui`、`codex-cli`
- Raw content:
  ```text
  update_action: 2 passed
  tooltips: 14 passed
  feedback: 16 passed
  doctor::updates: 2 passed
  update_prompt --release: 1 passed
  installer: 1 passed
  ```
- Interpretation: 修复覆盖 debug 与 release 条件编译面，未拥有渠道会明确拒绝，而不是回退到 OpenAI。
- Time: 2026-08-21 18:30
