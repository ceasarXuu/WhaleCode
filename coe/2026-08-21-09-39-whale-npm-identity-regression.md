# Problem P-001: Whale npm 发布身份在 Codex vendor 同步后回退
- Status: diagnosed
- Created: 2026-08-21 09:39
- Updated: 2026-08-21 09:43
- Objective: 解释已独立发布的 Whale npm 包为什么在当前仓库中变回 OpenAI Codex 包配置。
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
- Ruled out:
  - Whale 独立 npm 包从未发布。
  - 当前仓库另有一套仍有效的 Whale npm package manifest。
- Fix criteria:
  - 用户授权修复后，恢复并验证 Whale 独立 package identity、launcher、平台包构建和只指向 Whale 资产的发布入口；加入同步/发布回归门禁；原始安装 smoke 通过。
- Current conclusion: 这是 0.147 vendor cutover 中未重放 npm/release overlay 导致的确定性回归，不是 npm registry 迁移或 OpenAI 接管。
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - Root cause diagnosed by H-001 and E-001 through E-006；尚未授权或实施修复。
- Close reason:
  - not closed

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
- Repair design readiness: ready after user confirmation
- Next step: 请求用户确认是否恢复 Whale 独立 npm 发布链并纳入 v0.0.5 release preparation。
- Blocker:
  - 修复尚未获得用户授权。
- Close reason:
  - not closed

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
